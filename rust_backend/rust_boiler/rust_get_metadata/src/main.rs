use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde::{Serialize, Deserialize};
use serde_json::json;
use std::env;
use chrono::{DateTime, Utc};

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_secretsmanager::Client as SecretManagerClient;

use tokio_postgres::{Client, Error as OtherError};
use postgres_native_tls::MakeTlsConnector;
use native_tls::TlsConnector;

// Metadata struct to map query results
#[derive(Debug, Serialize, Deserialize)]
struct Metadata {
    metadata_uuid: String,
    metadata_name: String,
    metadata_description: Option<String>,
    metadata_type: String,
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

async fn function_handler(_event: Request) -> Result<Response<Body>, Error> {
    
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
    println!("Server: {}", db_server);
    let database = db_credentials["DB_NAME"].as_str().unwrap();
    println!("Database: {}", database);
    let db_username = db_credentials["DB_USER"].as_str().unwrap();
    let db_password = db_credentials["DB_PASSWORD"].as_str().unwrap();
    let db_port = db_credentials["DB_PORT"].as_str().unwrap();
    println!("End of get info from secret!");

    let tls_connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true) // Disable certificate validation for development
        .build();
    let tls = MakeTlsConnector::new(tls_connector.expect("Failed to create TLS connector"));

    println!("TLS connector created");

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
                .body(json!({"statusAPI": "CRASH", "error": "Database connection failed"}).to_string().into())
                .map_err(Box::new)?);
        }
    };

    // Spawn a new task to manage the connection
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    // Simple query to fetch all metadata records from the metadatas table
    let query = "SELECT * FROM document_library.metadatas";
    
    // Execute the query
    let rows = client.query(query, &[]).await?;
    
    // Parse rows into Metadata structs
    let mut metadatas: Vec<Metadata> = Vec::new();
    for row in rows {
        let metadata = Metadata {
            metadata_uuid: row.get("metadata_uuid"),
            metadata_name: row.get("metadata_name"),
            metadata_description: row.try_get("metadata_description").unwrap_or(None),
            metadata_type: row.get("metadata_type"),
            creation_date: row.try_get("creation_date").unwrap_or(None),
            created_by: row.try_get("created_by").unwrap_or(None),
            updated_date: row.try_get("updated_date").unwrap_or(None),
            updated_by: row.try_get("updated_by").unwrap_or(None),
            comments: row.try_get("comments").unwrap_or(None),
        };
        
        println!("Metadata UUID: {}, Name: {}, Type: {}", 
                metadata.metadata_uuid, metadata.metadata_name, metadata.metadata_type);
        metadatas.push(metadata);
    }
    
    println!("Total metadatas: {}", metadatas.len());

    // Generate JSON response
    let response_body = json!({
        "statusAPI": "OK",
        "metadatas": metadatas
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