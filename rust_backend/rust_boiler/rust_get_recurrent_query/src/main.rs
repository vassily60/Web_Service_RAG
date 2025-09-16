use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde::{Serialize, Deserialize};
use serde_json::json;
use std::env;
use chrono::{DateTime, Utc};

use aws_sdk_secretsmanager::config::Region;
use aws_sdk_secretsmanager::Client as SecretManagerClient;

use tokio_postgres::types::ToSql;
use postgres_native_tls::MakeTlsConnector;
use native_tls::TlsConnector;

// RecurrentQuery struct to map query results
#[derive(Debug, Serialize, Deserialize)]
struct RecurrentQuery {
    recurrent_query_uuid: String,
    recurrent_query_name: String,
    query_type: String,
    query_content: String,
    user_uuid: String,
    query_tags: Option<String>,
    query_start_document_date: Option<chrono::NaiveDate>,
    query_end_document_date: Option<chrono::NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    creation_date: Option<DateTime<Utc>>,
    created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_date: Option<DateTime<Utc>>,
    updated_by: Option<String>,
    comments: Option<String>,
}

// Request struct to deserialize incoming data
#[derive(Debug, Serialize, Deserialize)]
struct GetRecurrentQueryRequest {
    recurrent_query_uuid: Option<String>,
    user_uuid: Option<String>,
}

async fn show_secret(client: &SecretManagerClient, name: &str) -> Result<String, Error> {
    let resp = client.get_secret_value().secret_id(name).send().await?;
    match resp.secret_string() {
        Some(secret) => Ok(secret.into()),
        None => panic!("Error to get the secret: {:?}", name),
    }
}

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    
    // Parse request body if any
    let mut recurrent_query_uuid_filter: Option<String> = None;
    let mut user_uuid_filter: Option<String> = None;
    
    if !event.body().is_empty() {
        let body = event.body();
        if let Ok(request_data) = serde_json::from_slice::<GetRecurrentQueryRequest>(body) {
            recurrent_query_uuid_filter = request_data.recurrent_query_uuid;
            user_uuid_filter = request_data.user_uuid;
        }
    }
    
    // Get region from environment variable
    let region_name = env::var("REGION").expect("REGION environment variable not set");
    
    // Initialize AWS SDK configuration
    let region = Region::new(region_name.clone());
    let config = aws_config::from_env().region(region).load().await;
    let client_secret = SecretManagerClient::new(&config);
    println!("AWS SDK initialized!");

    // Get PostgreSQL configuration from environment variables
    // Decode secret
    let db_secret_name = env::var("DATABASE_CONECTION_STRING").expect("DATABASE_CONECTION_STRING environment variable not set");
    let db_secret = show_secret(&client_secret, &db_secret_name).await?;
    let db_credentials: serde_json::Value = serde_json::from_str(&db_secret)?;
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

    let (client, connection): (tokio_postgres::Client, _) = match connection_result {
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

    // Build query for recurrent queries based on filters
    let mut query = "SELECT * FROM document_library.recurrent_queries_extended".to_string();
    let mut filters = Vec::new();
    let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
    
    if let Some(recurrent_query_uuid) = &recurrent_query_uuid_filter {
        filters.push(format!("recurrent_query_uuid = ${}", params.len() + 1));
        params.push(recurrent_query_uuid);
    }
    
    if let Some(user_uuid) = &user_uuid_filter {
        filters.push(format!("user_uuid = ${}", params.len() + 1));
        params.push(user_uuid);
    }
    
    if !filters.is_empty() {
        query = format!("{} WHERE {}", query, filters.join(" AND "));
    }
    
    // Execute the query
    println!("Executing query: {}", query);
    let rows_result = client.query(query.as_str(), &params[..]).await;
    
    let rows = match rows_result {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("Query error: {}", e);
            return Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({"statusAPI": "ERROR", "error": "Query execution failed"}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    // Parse rows into RecurrentQuery structs
    let mut recurrent_queries: Vec<RecurrentQuery> = Vec::new();
    for row in rows {
        let recurrent_query = RecurrentQuery {
            recurrent_query_uuid: row.get("recurrent_query_uuid"),
            recurrent_query_name: row.get("recurrent_query_name"),
            query_type: row.get("query_type"),
            query_content: row.get("query_content"),
            user_uuid: row.get("user_uuid"),
            query_tags: row.try_get("query_tags").unwrap_or(None),
            query_start_document_date: row.try_get("query_start_document_date").unwrap_or(None),
            query_end_document_date: row.try_get("query_end_document_date").unwrap_or(None),
            creation_date: row.try_get("creation_date").unwrap_or(None),
            created_by: row.try_get("created_by").unwrap_or(None),
            updated_date: row.try_get("updated_date").unwrap_or(None),
            updated_by: row.try_get("updated_by").unwrap_or(None),
            comments: row.try_get("comments").unwrap_or(None),
        };
        
        println!("RecurrentQuery UUID: {}, Name: {}, Type: {}", 
                recurrent_query.recurrent_query_uuid, recurrent_query.recurrent_query_name, recurrent_query.query_type);
        recurrent_queries.push(recurrent_query);
    }
    
    println!("Total recurrent queries: {}", recurrent_queries.len());

    // Generate JSON response
    let response_body = json!({
        "statusAPI": "OK",
        "recurrent_queries": recurrent_queries
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