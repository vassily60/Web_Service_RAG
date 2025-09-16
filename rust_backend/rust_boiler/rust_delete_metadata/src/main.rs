use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde::{Serialize, Deserialize};
use serde_json::json;
use std::env;
use chrono::Utc;
use jsonwebtokens_cognito::KeySet;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_secretsmanager::Client as SecretManagerClient;

use tokio_postgres::Client;
use postgres_native_tls::MakeTlsConnector;
use native_tls::TlsConnector;

// Request struct to deserialize incoming data
#[derive(Debug, Serialize, Deserialize)]
struct MetadataRequest {
    metadata_uuid: String,
}

// Response struct for API
#[derive(Debug, Serialize, Deserialize)]
struct MetadataResponse {
    statusAPI: String,
    message: String,
}

async fn show_secret(client: &SecretManagerClient, name: &str) -> Result<String, Error> {
    let resp = client.get_secret_value().secret_id(name).send().await?;
    match resp.secret_string() {
        Some(secret) => Ok(secret.into()),
        None => panic!("Error getting the secret: {:?}", name),
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
                    match claims.get("email") {
                        Some(email_value) => {
                            if let Some(email_str) = email_value.as_str() {
                                return Ok(email_str.to_string());
                            }
                        }
                        None => {}
                    }
                    
                    // Fallback if email not found in primary location
                    if let Some(email) = claims.get("email").and_then(|v| v.as_str()) {
                        Ok(email.to_string())
                    } else {
                        println!("No email found in token claims.");
                        Ok("unknown@user.com".to_string())
                    }
                },
                Err(e) => {
                    println!("Token verification error: {}", e);
                    Err(Box::new(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Invalid token")))
                }
            }
        },
        Err(e) => {
            println!("KeySet creation error: {}", e);
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Failed to create KeySet")))
        }
    }
}

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    println!("Request received: {:?}", event);
    
    // Extract user email from token
    let user_email = match extract_email_from_token(&event).await {
        Ok(email) => email,
        Err(e) => {
            println!("Failed to extract email from token: {}", e);
            return Ok(Response::builder()
                .status(401)
                .header("content-type", "application/json")
                .body(json!({"statusAPI": "ERROR", "message": "Unauthorized: Invalid token"}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    println!("User email extracted: {}", user_email);

    // Parse request body
    let body = event.body();
    let request_data: MetadataRequest = match serde_json::from_slice(body.as_ref()) {
        Ok(data) => data,
        Err(e) => {
            println!("Failed to parse request body: {}", e);
            return Ok(Response::builder()
                .status(400)
                .header("content-type", "application/json")
                .body(json!({"statusAPI": "ERROR", "message": "Invalid request format"}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    // Validate required fields
    if request_data.metadata_uuid.is_empty() {
        return Ok(Response::builder()
            .status(400)
            .header("content-type", "application/json")
            .body(json!({"statusAPI": "ERROR", "message": "Missing required metadata_uuid"}).to_string().into())
            .map_err(Box::new)?);
    }

    // Access environment variables
    let region_provider = RegionProviderChain::default_provider().or_else("ap-southeast-1");
    let config = aws_config::from_env().region(region_provider).load().await;
    let client_secret = aws_sdk_secretsmanager::Client::new(&config);
    println!("Access environment variables");

    // Decode secret
    let db_secret_name = env::var("DATABASE_CONECTION_STRING").expect("DATABASE_CONECTION_STRING environment variable not set");
    let db_secret = show_secret(&client_secret, &db_secret_name).await?;
    let db_credentials: serde_json::Value = serde_json::from_str(&db_secret)?;
    println!("Decoded secret");

    let db_server = db_credentials["DB_HOST"].as_str().unwrap();
    let database = db_credentials["DB_NAME"].as_str().unwrap();
    let db_username = db_credentials["DB_USER"].as_str().unwrap();
    let db_password = db_credentials["DB_PASSWORD"].as_str().unwrap();
    let db_port = db_credentials["DB_PORT"].as_str().unwrap();

    let tls_connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true) // Disable certificate validation for development
        .build();
    let tls = MakeTlsConnector::new(tls_connector.expect("Failed to build TLS connector"));

    // Connect to the database with better error handling
    let connection_string = format!(
        "host={} port={} user={} password={} dbname={}", 
        db_server, db_port, db_username, db_password, database
    );
    println!("Attempting to connect to database...");
    
    let connection_result = tokio_postgres::connect(&connection_string, tls).await;

    let (client, connection) = match connection_result {
        Ok((client, connection)) => {
            println!("Successfully connected to database");
            (client, connection)
        },
        Err(e) => {
            eprintln!("Failed to connect to database: {}", e);
            return Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({"statusAPI": "ERROR", "message": "Database connection failed"}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    // Spawn a new task to manage the connection
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    // Check if metadata exists before trying to delete it
    let check_query = "SELECT COUNT(*) FROM document_library.metadatas WHERE metadata_uuid = $1";
    match client.query_one(check_query, &[&request_data.metadata_uuid]).await {
        Ok(row) => {
            let count: i64 = row.get(0);
            if count == 0 {
                return Ok(Response::builder()
                    .status(404)
                    .header("content-type", "application/json")
                    .body(json!({"statusAPI": "ERROR", "message": "Metadata not found"}).to_string().into())
                    .map_err(Box::new)?);
            }
        },
        Err(e) => {
            eprintln!("Failed to check if metadata exists: {}", e);
            return Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({"statusAPI": "ERROR", "message": format!("Database error: {}", e)}).to_string().into())
                .map_err(Box::new)?);
        }
    }

    // First delete the document_metadatas
    let delete_document_metadatas_query = "DELETE FROM document_library.document_metadatas WHERE metadata_uuid = $1";
    let document_metadatas_result = client.execute(delete_document_metadatas_query, &[&request_data.metadata_uuid]).await;
    
    match document_metadatas_result {
        Ok(count) => {
            println!("Successfully deleted {} document_metadatas records", count);
        },
        Err(e) => {
            eprintln!("Failed to delete document_metadatas: {}", e);
            return Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({"statusAPI": "ERROR", "message": format!("Failed to delete document_metadatas: {}", e)}).to_string().into())
                .map_err(Box::new)?);
        }
    }

    // Then delete the metadata itself
    let delete_metadata_query = "DELETE FROM document_library.metadatas WHERE metadata_uuid = $1";
    let metadata_result = client.execute(delete_metadata_query, &[&request_data.metadata_uuid]).await;
    
    match metadata_result {
        Ok(count) => {
            if count > 0 {
                // Success response
                let response_body = MetadataResponse {
                    statusAPI: "OK".to_string(),
                    message: "Metadata deleted successfully".to_string(),
                };
                
                Ok(Response::builder()
                    .status(200)
                    .header("content-type", "application/json")
                    .body(json!(response_body).to_string().into())
                    .map_err(Box::new)?)
            } else {
                // This shouldn't happen because we already checked if the metadata exists
                Ok(Response::builder()
                    .status(404)
                    .header("content-type", "application/json")
                    .body(json!({"statusAPI": "ERROR", "message": "Metadata not found"}).to_string().into())
                    .map_err(Box::new)?)
            }
        },
        Err(e) => {
            eprintln!("Failed to delete metadata: {}", e);
            Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({"statusAPI": "ERROR", "message": format!("Failed to delete metadata: {}", e)}).to_string().into())
                .map_err(Box::new)?)
        }
    }
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