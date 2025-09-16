use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use std::env;
use chrono::{DateTime, Utc};

use aws_sdk_secretsmanager::Client as SecretManagerClient;
use aws_sdk_secretsmanager::config::Region;

use tokio_postgres::Client;
use postgres_native_tls::MakeTlsConnector;
use native_tls::TlsConnector;
use jsonwebtokens_cognito::KeySet;

// Synonym struct to map query results
#[derive(Debug, Serialize, Deserialize)]
struct Synonym {
    synonym_uuid: String,
    synonym_name: String,
    synonym_value: String,
    #[serde(with = "chrono::serde::ts_seconds_option", skip_serializing_if = "Option::is_none")]
    creation_date: Option<DateTime<Utc>>,
    created_by: Option<String>,
    #[serde(with = "chrono::serde::ts_seconds_option", skip_serializing_if = "Option::is_none")]
    updated_date: Option<DateTime<Utc>>,
    updated_by: Option<String>,
    comments: Option<String>,
}

async fn show_secret(client: &SecretManagerClient, name: &str) -> Result<String, Error> {
    let resp = client.get_secret_value().secret_id(name).send().await?;
    match resp.secret_string() {
        Some(secret) => Ok(secret.into()),
        None => panic!("Error to get the secret: {:?}", name),
    }
}

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Get region from environment variable
    let region_name = env::var("REGION").expect("REGION environment variable not set");
    
    // Initialize AWS SDK configuration
    let region = Region::new(region_name.clone());
    let config = aws_config::from_env().region(region).load().await;
    let client_secret = SecretManagerClient::new(&config);
    println!("AWS SDK initialized!");

    // Get Cognito configuration from AWS Secrets Manager
    let cognito_secret_name = env::var("COGNITO_SECRET").expect("COGNITO_SECRET environment variable not set");
    let secret_content = show_secret(&client_secret, &cognito_secret_name).await?;
    let cognito_credentials: Value = serde_json::from_str(&secret_content)?;
    
    // Extract user_pool_id and client_id from the secret
    let user_pool_id = cognito_credentials["USER_POOL_ID"].as_str()
        .ok_or("USER_POOL_ID not found in secret")?;
    let client_id = cognito_credentials["APP_CLIENT_ID"].as_str()
        .ok_or("APP_CLIENT_ID not found in secret")?;
    let cognito_region_name = cognito_credentials["REGION"].as_str()
        .ok_or("REGION not found in secret")?;
    
    println!("Retrieved Cognito configuration from Secrets Manager!");
    
    // Extract token from Authorization header
    let my_token_strslice = match event.headers().get("Authorization") {
        Some(auth_header) => {
            let auth_header_str = auth_header.to_str()?;
            if auth_header_str.len() > 7 {
                &auth_header_str[7..] // Skip "Bearer "
            } else {
                return Ok(Response::builder()
                    .status(401)
                    .header("content-type", "application/json")
                    .body(json!({"message": "Invalid authorization header format"}).to_string().into())
                    .map_err(Box::new)?);
            }
        },
        None => {
            return Ok(Response::builder()
                .status(401)
                .header("content-type", "application/json")
                .body(json!({"message": "Authorization header not provided"}).to_string().into())
                .map_err(Box::new)?);
        }
    };

    // Create a KeySet for AWS Cognito with proper error handling
    let key_set = match KeySet::new(cognito_region_name.clone(), user_pool_id.to_string()) {
        Ok(ks) => ks,
        Err(e) => {
            return Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({"message": format!("Failed to create KeySet: {}", e)}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    // Prefetch the JWKs from Cognito - this is necessary before verification
    match key_set.prefetch_jwks().await {
        Ok(_) => println!("Successfully fetched JWKs from Cognito"),
        Err(e) => {
            return Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({"message": format!("Failed to fetch JWKs: {}", e)}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    let verifier = match key_set.new_id_token_verifier(&[client_id]).build() {
        Ok(v) => v,
        Err(e) => {
            return Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({"message": format!("Failed to create verifier: {}", e)}).to_string().into())
                .map_err(Box::new)?);
        }
    };
            
    // More detailed logging for debugging
    println!("Token to verify: {}", my_token_strslice);
    println!("User Pool ID: {}", user_pool_id);
    println!("Client ID: {}", client_id);
    println!("Region: {}", cognito_region_name);
    
    let my_verif_result = key_set.verify(my_token_strslice, &verifier).await;
    println!("The verifier result: {:?} ", my_verif_result);

    // Extract email from verified token or return error
    let user_email = match my_verif_result {
        Ok(my_verif_parsed) => {
            // Parse the token claims into a serde_json::Value
            match serde_json::from_str::<serde_json::Value>(&my_verif_parsed.to_string()) {
                Ok(json_value) => {
                    // Extract email from token claims
                    json_value["email"].as_str()
                        .unwrap_or("unknown_user@example.com")
                        .to_string()
                },
                Err(e) => {
                    // If we can't parse the token claims as JSON, return an error
                    return Ok(Response::builder()
                        .status(500)
                        .header("content-type", "application/json")
                        .body(json!({"message": format!("Failed to parse token claims: {}", e)}).to_string().into())
                        .map_err(Box::new)?);
                }
            }
        },
        Err(e) => {
            return Ok(Response::builder()
                .status(401)
                .header("content-type", "application/json")
                .body(json!({"message": format!("Failed to verify token: {}", e)}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    println!("User email: {}", user_email);

    // Get PostgreSQL configuration from environment variables
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

    // Simple query to fetch all synonym records from the synonyms table
    let query = "SELECT * FROM document_library.synonyms";
    
    // Execute the query
    let rows = client.query(query, &[]).await?;
    
    // Parse rows into Synonym structs
    let mut synonyms: Vec<Synonym> = Vec::new();
    for row in rows {
        let synonym = Synonym {
            synonym_uuid: row.get("synonym_uuid"),
            synonym_name: row.get("synonym_name"),
            synonym_value: row.get("synonym_value"),
            creation_date: row.try_get("creation_date").unwrap_or(None),
            created_by: row.try_get("created_by").unwrap_or(None),
            updated_date: row.try_get("updated_date").unwrap_or(None),
            updated_by: row.try_get("updated_by").unwrap_or(None),
            comments: row.try_get("comments").unwrap_or(None),
        };
        
        println!("Synonym UUID: {}, Name: {}, Value: {}", 
                synonym.synonym_uuid, synonym.synonym_name, synonym.synonym_value);
        synonyms.push(synonym);
    }
    
    println!("Total synonyms: {}", synonyms.len());

    // Generate JSON response
    let response_body = json!({
        "statusAPI": "OK",
        "synonyms": synonyms
    });
    
    let resp = Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(response_body.to_string().into())
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
