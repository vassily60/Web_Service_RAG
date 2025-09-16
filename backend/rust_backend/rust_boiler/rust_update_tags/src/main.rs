use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde::{Serialize, Deserialize};
use serde_json::json;
use std::env;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_secretsmanager::Client as SecretManagerClient;

use tokio_postgres::{Client, Error as OtherError};
use postgres_native_tls::MakeTlsConnector;
use native_tls::TlsConnector;

// Request structure for updating tags
#[derive(Debug, Serialize, Deserialize)]
struct UpdateTagsRequest {
    document_uuid: String,
    tags: Vec<String>,
}

// Response structure for the update operation
#[derive(Debug, Serialize, Deserialize)]
struct UpdateTagsResponse {
    success: bool,
    message: String,
    document_uuid: String,
}

async fn show_secret(client: &SecretManagerClient, name: &str) -> Result<String, Error> {
    let resp = client.get_secret_value().secret_id(name).send().await?;
    match resp.secret_string() {
        Some(secret) => Ok(secret.into()),
        None => panic!("Error to get the secret: {:?}", name),
    }
}

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    println!("Processing update tags request");
    
    // Parse the request body to extract document_uuid and tags
    let body = event.body();
    let parse_result: Result<UpdateTagsRequest, _> = serde_json::from_slice(body);
    
    // Handle parsing errors
    let update_request = match parse_result {
        Ok(request) => request,
        Err(e) => {
            eprintln!("Failed to parse request: {}", e);
            return Ok(Response::builder()
                .status(400)
                .header("content-type", "application/json")
                .body(json!({
                    "success": false,
                    "message": format!("Invalid request format: {}", e)
                }).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    // Validate request
    if update_request.document_uuid.is_empty() {
        return Ok(Response::builder()
            .status(400)
            .header("content-type", "application/json")
            .body(json!({
                "success": false,
                "message": "Document UUID cannot be empty"
            }).to_string().into())
            .map_err(Box::new)?);
    }
    
    println!("Update request for document: {}, tags: {:?}", update_request.document_uuid, update_request.tags);
    
    // Access environment variables
    let region_provider = RegionProviderChain::default_provider().or_else("ap-southeast-1");
    let config = aws_config::from_env().region(region_provider).load().await;
    let client_secret = aws_sdk_secretsmanager::Client::new(&config);
    println!("Access environment variables!");

    // Decode secret
    let db_secret_name = env::var("DATABASE_CONECTION_STRING").expect("DATABASE_CONECTION_STRING environment variable not set");
    let db_secret = show_secret(&client_secret, &db_secret_name).await.unwrap();
    let db_credentials: serde_json::Value = serde_json::from_str(&db_secret).unwrap();
    println!("Decoded secret!");

    let db_server = db_credentials["DB_HOST"].as_str().unwrap();
    let database = db_credentials["DB_NAME"].as_str().unwrap();
    let db_username = db_credentials["DB_USER"].as_str().unwrap();
    let db_password = db_credentials["DB_PASSWORD"].as_str().unwrap();
    let db_port = db_credentials["DB_PORT"].as_str().unwrap();
    println!("End of get info from secret!");

    let tls_connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true) // Disable certificate validation
        .build();
    let tls = MakeTlsConnector::new(tls_connector.expect("Failed to create TLS connector"));

    // Connect to the database
    let connection_string = format!("host={} port={} user={} dbname={}", 
        db_server, db_port, db_username, database);
    println!("Attempting to connect to database...");
    
    let connection_result = tokio_postgres::connect(
        &format!("host={} port={} user={} password={} dbname={}", 
        db_server, db_port, db_username, db_password, database),
        tls
    ).await;

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
                .body(json!({
                    "success": false,
                    "message": "Internal server error: Database connection failed"
                }).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    // Spawn a new task to manage the connection
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    // First, check if the document exists
    let check_query = "SELECT document_uuid FROM document_library.documents WHERE document_uuid = $1";
    let rows = client.query(check_query, &[&update_request.document_uuid]).await?;
    
    if rows.is_empty() {
        return Ok(Response::builder()
            .status(404)
            .header("content-type", "application/json")
            .body(json!({
                "success": false,
                "message": format!("Document with UUID {} not found", update_request.document_uuid)
            }).to_string().into())
            .map_err(Box::new)?);
    }
    
    // Update the document tags
    let update_query = "UPDATE document_library.documents SET tags = $1 WHERE document_uuid = $2";
    
    match client.execute(update_query, &[&update_request.tags, &update_request.document_uuid]).await {
        Ok(rows_affected) => {
            if rows_affected == 0 {
                return Ok(Response::builder()
                    .status(500)
                    .header("content-type", "application/json")
                    .body(json!({
                        "success": false,
                        "message": "Failed to update document tags: No rows affected"
                    }).to_string().into())
                    .map_err(Box::new)?);
            }
            
            // Successfully updated
            println!("Successfully updated tags for document: {}", update_request.document_uuid);
            
            let response = UpdateTagsResponse {
                success: true,
                message: "Document tags updated successfully".to_string(),
                document_uuid: update_request.document_uuid,
            };
            
            return Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(serde_json::to_string(&response).unwrap().into())
                .map_err(Box::new)?);
        },
        Err(e) => {
            eprintln!("Database error: {}", e);
            return Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({
                    "success": false,
                    "message": format!("Failed to update document tags: {}", e)
                }).to_string().into())
                .map_err(Box::new)?);
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