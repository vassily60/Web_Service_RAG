use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde::{Serialize, Deserialize};
use serde_json::json;
use std::env;
use std::time::Duration;
use jsonwebtokens_cognito::KeySet;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_secretsmanager::Client as SecretManagerClient;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client as S3Client;

use tokio_postgres::{Client, NoTls};
use postgres_native_tls::MakeTlsConnector;
use native_tls::TlsConnector;

// Request structure for getting a presigned URL
#[derive(Debug, Serialize, Deserialize)]
struct PresignedUrlRequest {
    document_uuid: String,
    #[serde(default = "default_expiration")]
    expiration: u64,
}

fn default_expiration() -> u64 {
    900 // Default to 15 minutes (900 seconds)
}

// Document structure returned from database
#[derive(Debug, Serialize, Deserialize)]
struct Document {
    document_uuid: String,
    document_name: String,
    document_location: String,
}

// Response structure
#[derive(Debug, Serialize, Deserialize)]
struct PresignedUrlResponse {
    status: String,
    presigned_url: String,
    document_name: String,
    expiration: u64,
}

async fn show_secret(client: &SecretManagerClient, name: &str) -> Result<String, Error> {
    let resp = client.get_secret_value().secret_id(name).send().await?;
    match resp.secret_string() {
        Some(secret) => Ok(secret.into()),
        None => {
            eprintln!("Error retrieving secret: {}", name);
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Error retrieving secret: {}", name))))
        }
    }
}

// Extract email from Authorization token
async fn extract_email_from_token(event: &Request) -> Result<String, Error> {
    let key_to_check = "Authorization";
    if !event.headers().contains_key(key_to_check) {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "No Authorization header found")));
    }

    let bearer_str = event.headers()[key_to_check].to_str()?;
    if !bearer_str.starts_with("Bearer ") {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid Authorization format")));
    }

    let token_str = &bearer_str[7..];
    
    // Get AWS region and user pool ID from environment variables or use defaults
    let region = env::var("COGNITO_REGION").expect("COGNITO_REGION environment variable not set");
    let user_pool_id = env::var("COGNITO_USER_POOL_ID").expect("COGNITO_USER_POOL_ID environment variable not set");
    let client_id = env::var("COGNITO_CLIENT_ID").expect("COGNITO_CLIENT_ID environment variable not set");

    let keyset = KeySet::new(&region, &user_pool_id);
    
    match keyset {
        Ok(key_set) => {
            let verifier = key_set.new_id_token_verifier(&[&client_id]).build()?;
            let verification_result = key_set.verify(token_str, &verifier).await;
            
            match verification_result {
                Ok(claims) => {
                    // Extract email from claims
                    if let Some(email) = claims.get("email").and_then(|v| v.as_str()) {
                        return Ok(email.to_string());
                    } else {
                        println!("No email found in token claims.");
                        return Ok("unknown@user.com".to_string());
                    }
                },
                Err(e) => {
                    println!("Token verification error: {}", e);
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Invalid token")));
                }
            }
        },
        Err(e) => {
            println!("KeySet creation error: {}", e);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Failed to create KeySet")));
        }
    }
}

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Extract user email from token
    let user_email = match extract_email_from_token(&event).await {
        Ok(email) => email,
        Err(e) => {
            println!("Failed to extract email from token: {}", e);
            return Ok(Response::builder()
                .status(401)
                .header("content-type", "application/json")
                .body(json!({"status": "error", "message": "Unauthorized: Invalid token"}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    println!("User email extracted: {}", user_email);
    
    // Parse request body to get document UUID and expiration
    let body = event.body();
    let request_data: PresignedUrlRequest = match serde_json::from_slice(body.as_ref()) {
        Ok(data) => data,
        Err(e) => {
            println!("Failed to parse request body: {}", e);
            return Ok(Response::builder()
                .status(400)
                .header("content-type", "application/json")
                .body(json!({"status": "error", "message": "Bad request: Invalid JSON"}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    println!("Request data parsed: Document UUID: {}, Expiration: {} seconds", request_data.document_uuid, request_data.expiration);
    
    // Access environment variables and AWS resources
    let region_provider = RegionProviderChain::default_provider().or_else("ap-southeast-1");
    let config = aws_config::from_env().region(region_provider).load().await;
    let client_secret = aws_sdk_secretsmanager::Client::new(&config);
    
    // Get bucket name from environment variable with default
    let s3_bucket_name = env::var("S3BUCKET_IMPORT_FOLDER").expect("S3BUCKET_IMPORT_FOLDER environment variable not set");
    
    println!("S3 bucket name: {}", s3_bucket_name);

    // Decode database secret
    let db_secret_name = env::var("DATABASE_CONECTION_STRING").expect("DATABASE_CONECTION_STRING environment variable not set");
    let db_secret = match show_secret(&client_secret, &db_secret_name).await {
        Ok(secret) => secret,
        Err(e) => {
            println!("Error getting database secret: {}", e);
            return Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({"status": "error", "message": "Server configuration error: Database secret not available"}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    let db_credentials: serde_json::Value = serde_json::from_str(&db_secret).map_err(|e| {
        println!("Error parsing database credentials: {}", e);
        Box::new(e) as Error
    })?;

    let db_server = db_credentials["DB_HOST"].as_str().unwrap();
    let database = db_credentials["DB_NAME"].as_str().unwrap();
    let db_username = db_credentials["DB_USER"].as_str().unwrap();
    let db_password = db_credentials["DB_PASSWORD"].as_str().unwrap();
    let db_port = db_credentials["DB_PORT"].as_str().unwrap();
    
    // Setup TLS connection
    let tls_connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true) // Disable certificate validation for development
        .build();
    let tls = MakeTlsConnector::new(tls_connector.expect("Failed to build TLS connector"));

    // Connect to the database
    println!("Connecting to database...");
    let connection_result = tokio_postgres::connect(
        &format!("host={} port={} user={} password={} dbname={}", 
        db_server, db_port, db_username, db_password, database),
        tls
    ).await;

    let (pg_client, connection) = match connection_result {
        Ok((client, connection)) => {
            println!("Successfully connected to database");
            (client, connection)
        },
        Err(e) => {
            eprintln!("Failed to connect to database: {}", e);
            return Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({"status": "error", "message": "Database connection failed"}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    // Spawn the connection task
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Database connection error: {}", e);
        }
    });

    // Query to get the document information
    let query = "SELECT document_uuid, document_name, document_location 
                 FROM document_library.documents 
                 WHERE document_uuid = $1";
    
    // Execute the query
    let row = match pg_client.query_one(query, &[&request_data.document_uuid]).await {
        Ok(row) => row,
        Err(e) => {
            println!("Error querying document: {}", e);
            return Ok(Response::builder()
                .status(404)
                .header("content-type", "application/json")
                .body(json!({"status": "error", "message": "Document not found"}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    // Extract document data
    let document = Document {
        document_uuid: row.get("document_uuid"),
        document_name: row.get("document_name"),
        document_location: row.get("document_location"),
    };
    
    println!("Document found: {}, Location: {}", document.document_name, document.document_location);
    
    // Create S3 client
    let s3_client = S3Client::new(&config);
    
    // Create presigning configuration with specified expiration
    let presign_config = match PresigningConfig::expires_in(Duration::from_secs(request_data.expiration)) {
        Ok(config) => config,
        Err(e) => {
            println!("Error creating presigning config: {}", e);
            return Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({"status": "error", "message": "Failed to create presigned URL configuration"}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    // Generate presigned URL
    let presigned_req = match s3_client
        .get_object()
        .bucket(s3_bucket_name)
        .key(&document.document_location)
        .presigned(presign_config)
        .await {
        Ok(req) => req,
        Err(e) => {
            println!("Error generating presigned URL: {}", e);
            return Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({"status": "error", "message": "Failed to generate presigned URL"}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    // Extract the presigned URL as a string
    let presigned_url = presigned_req.uri().to_string();
    println!("Presigned URL generated: {}", presigned_url);
    
    // Create the response
    let response = PresignedUrlResponse {
        status: "success".to_string(),
        presigned_url,
        document_name: document.document_name,
        expiration: request_data.expiration,
    };
    
    let resp = Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(serde_json::to_string(&response)?.into())
        .map_err(Box::new)?;
    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or(tracing_subscriber::EnvFilter::new("INFO")),
        )
        .with_target(false)
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}
