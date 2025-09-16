use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde::{Serialize, Deserialize};
use serde_json::json;
use std::env;
use regex::Regex;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_secretsmanager::Client as SecretManagerClient;

use tokio_postgres::{Client, Error as PostgresError};
use postgres_native_tls::MakeTlsConnector;
use native_tls::TlsConnector;

// Request struct to deserialize incoming data
#[derive(Debug, Serialize, Deserialize)]
struct ComputeSynonymRequest {
    query: String,
}

// Response struct for API
#[derive(Debug, Serialize, Deserialize)]
struct ComputeSynonymResponse {
    statusAPI: String,
    original_query: String,
    processed_query: String,
}

// Synonym struct to map query results
#[derive(Debug, Serialize, Deserialize)]
struct Synonym {
    synonym_name: String,
    synonym_value: String,
}

async fn show_secret(client: &SecretManagerClient, name: &str) -> Result<String, Error> {
    let resp = client.get_secret_value().secret_id(name).send().await?;
    match resp.secret_string() {
        Some(secret) => Ok(secret.into()),
        None => panic!("Error getting the secret: {:?}", name),
    }
}

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Parse request body to get the query string
    let body = event.body();
    let request_data: ComputeSynonymRequest = match serde_json::from_slice(body) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to parse request body: {}", e);
            return Ok(Response::builder()
                .status(400)
                .header("content-type", "application/json")
                .body(json!({"statusAPI": "ERROR", "message": "Invalid request format"}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    // Validate required fields
    if request_data.query.is_empty() {
        return Ok(Response::builder()
            .status(400)
            .header("content-type", "application/json")
            .body(json!({"statusAPI": "ERROR", "message": "Missing required query parameter"}).to_string().into())
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

    // Get all synonyms from the database
    let query = "SELECT synonym_name, synonym_value FROM document_library.synonyms";
    let rows = client.query(query, &[]).await?;
    
    // Parse rows into Synonym structs
    let mut synonyms: Vec<Synonym> = Vec::new();
    for row in rows {
        let synonym = Synonym {
            synonym_name: row.get("synonym_name"),
            synonym_value: row.get("synonym_value"),
        };
        
        println!("Synonym Name: {}, Value: {}", synonym.synonym_name, synonym.synonym_value);
        synonyms.push(synonym);
    }
    
    println!("Total synonyms: {}", synonyms.len());

    // Process the query string by replacing keywords with their synonyms
    let mut processed_query = request_data.query.clone();
    
    for synonym in synonyms {
        // Create a pattern that matches the whole word only
        let pattern = format!(r"\b{}\b", regex::escape(&synonym.synonym_name));
        let re = Regex::new(&pattern).unwrap();
        
        // Replace whole word matches with keyword + OR + synonym value
        if re.is_match(&processed_query) {
            let replacement = format!("{} or {}", synonym.synonym_name, synonym.synonym_value);
            processed_query = re.replace_all(&processed_query, replacement).to_string();
        }
    }

    // Create response
    let response_body = ComputeSynonymResponse {
        statusAPI: "OK".to_string(),
        original_query: request_data.query,
        processed_query,
    };
    
    // Return successful response
    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(json!(response_body).to_string().into())
        .map_err(Box::new)?)
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