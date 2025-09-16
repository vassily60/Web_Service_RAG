use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde::{Serialize, Deserialize};
use serde_json::json;
use std::env;
use std::collections::HashMap;
use chrono::{DateTime, Utc, NaiveDate};

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_secretsmanager::Client as SecretManagerClient;

use tokio_postgres::{Client, Error as OtherError};
use postgres_native_tls::MakeTlsConnector;
use native_tls::TlsConnector;

// Document metadata struct to map query results
#[derive(Debug, Serialize, Deserialize)]
struct DocumentMetadata {
    document_metadata_uuid: String,
    document_uuid: String,
    metadata_uuid: String,
    metadata_name: String,
    metadata_type: String,
    metadata_value_string: Option<String>,
    metadata_value_int: Option<i32>,
    metadata_value_float: Option<f64>,
    metadata_value_date: Option<NaiveDate>, // Changed to NaiveDate to match PostgreSQL date type
    metadata_value_boolean: Option<bool>,
    #[serde(with = "chrono::serde::ts_seconds_option", skip_serializing_if = "Option::is_none")]
    creation_date: Option<DateTime<Utc>>,
    created_by: Option<String>,
}

// Response struct for returning multiple document metadatas grouped by document
#[derive(Debug, Serialize, Deserialize)]
struct DocumentMetadatasResponse {
    document_uuid: String,
    metadata_values: HashMap<String, serde_json::Value>,
}

async fn show_secret(client: &SecretManagerClient, name: &str) -> Result<String, Error> {
    let resp = client.get_secret_value().secret_id(name).send().await?;
    match resp.secret_string() {
        Some(secret) => Ok(secret.into()),
        None => panic!("Error to get the secret: {:?}", name),
    }
}

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    println!("Request received: {:?}", event);
    // Access environment variables
    let region_provider = RegionProviderChain::default_provider().or_else("ap-southeast-1");
    let config = aws_config::from_env().region(region_provider).load().await;
    let client_secret = aws_sdk_secretsmanager::Client::new(&config);
    println!("Access environment variables!");

    // Parse request body if available to get specific document_uuid
    let body_str = match event.body() {
        Body::Text(s) => Some(s.to_string()),
        Body::Binary(b) => Some(String::from_utf8_lossy(b).to_string()),
        _ => None,
    };
    
    // Extract document_uuid from request body if present
    let document_uuid_filter = if let Some(body) = body_str {
        match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(json_body) => {
                if let Some(doc_uuid) = json_body.get("document_uuid").and_then(|v| v.as_str()) {
                    println!("Filtering by document_uuid: {}", doc_uuid);
                    Some(doc_uuid.to_string())
                } else {
                    None
                }
            },
            Err(_) => None,
        }
    } else {
        None
    };

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

    // Query to fetch document metadata values joined with metadata information
    let query = if document_uuid_filter.is_some() {
        // Query with document_uuid filter
        "SELECT 
            dm.document_metadata_uuid,
            dm.document_uuid,
            dm.metadata_uuid,
            m.metadata_name,
            m.metadata_type,
            dm.metadata_value_string,
            dm.metadata_value_int,
            dm.metadata_value_float,
            dm.metadata_value_date,
            dm.metadata_value_boolean,
            dm.creation_date,
            dm.created_by
         FROM document_library.document_metadatas dm
         JOIN document_library.metadatas m ON dm.metadata_uuid = m.metadata_uuid
         WHERE dm.document_uuid = $1"
    } else {
        // Query without filter to get all document metadatas
        "SELECT 
            dm.document_metadata_uuid,
            dm.document_uuid,
            dm.metadata_uuid,
            m.metadata_name,
            m.metadata_type,
            dm.metadata_value_string,
            dm.metadata_value_int,
            dm.metadata_value_float,
            dm.metadata_value_date,
            dm.metadata_value_boolean,
            dm.creation_date,
            dm.created_by
         FROM document_library.document_metadatas dm
         JOIN document_library.metadatas m ON dm.metadata_uuid = m.metadata_uuid"
    };
    
    // Execute the query with the document_uuid if provided
    let rows = if let Some(doc_uuid) = document_uuid_filter {
        client.query(query, &[&doc_uuid]).await?
    } else {
        client.query(query, &[]).await?
    };
    
    // Parse rows into DocumentMetadata structs
    let mut document_metadatas: Vec<DocumentMetadata> = Vec::new();
    for row in rows {
        let metadata_type = row.get::<_, String>("metadata_type");
        let metadata_name = row.get::<_, String>("metadata_name");
        
        // Log the metadata values based on type
        match metadata_type.as_str() {
            "INTEGER" => {
                if let Ok(value) = row.try_get::<_, i32>("metadata_value_int") {
                    println!("Found INTEGER metadata '{}' with value: {}", metadata_name, value);
                }
            },
            "NUMBER" => {
                if let Ok(value) = row.try_get::<_, f64>("metadata_value_float") {
                    println!("Found NUMBER metadata '{}' with value: {}", metadata_name, value);
                }
            },
            "DATE" => {
                if let Ok(value) = row.try_get::<_, NaiveDate>("metadata_value_date") {
                    println!("Found DATE metadata '{}' with value: {} (as string: {})", 
                             metadata_name, value, value.format("%Y-%m-%d").to_string());
                } else {
                    println!("DATE metadata '{}' found but value could not be retrieved", metadata_name);
                }
            },
            _ => {}
        }
        
        let metadata = DocumentMetadata {
            document_metadata_uuid: row.get("document_metadata_uuid"),
            document_uuid: row.get("document_uuid"),
            metadata_uuid: row.get("metadata_uuid"),
            metadata_name,
            metadata_type,
            metadata_value_string: row.try_get("metadata_value_string").unwrap_or(None),
            metadata_value_int: row.try_get("metadata_value_int").unwrap_or(None),
            metadata_value_float: row.try_get("metadata_value_float").unwrap_or(None),
            metadata_value_date: row.try_get("metadata_value_date").unwrap_or(None),
            metadata_value_boolean: row.try_get("metadata_value_boolean").unwrap_or(None),
            creation_date: row.try_get("creation_date").unwrap_or(None),
            created_by: row.try_get("created_by").unwrap_or(None),
        };
        
        document_metadatas.push(metadata);
    }
    
    println!("Total document metadatas: {}", document_metadatas.len());

    // Group metadata by document_uuid and create a response structure
    let mut documents_map: HashMap<String, HashMap<String, serde_json::Value>> = HashMap::new();
    
    for metadata in document_metadatas {
        let metadata_values = documents_map.entry(metadata.document_uuid).or_insert_with(HashMap::new);
        
        // Determine which value to use based on metadata_type
        let value = match metadata.metadata_type.as_str() {
            "STRING" => {
                if let Some(value) = metadata.metadata_value_string {
                    serde_json::Value::String(value)
                } else {
                    serde_json::Value::Null
                }
            },
            "INTEGER" => {
                if let Some(value) = metadata.metadata_value_int {
                    serde_json::Value::Number(serde_json::Number::from(value))
                } else {
                    serde_json::Value::Null
                }
            },
            "NUMBER" => {
                if let Some(value) = metadata.metadata_value_float {
                    // Convert f64 to serde_json::Number
                    match serde_json::Number::from_f64(value) {
                        Some(num) => serde_json::Value::Number(num),
                        None => serde_json::Value::Null
                    }
                } else {
                    serde_json::Value::Null
                }
            },
            "DATE" => {
                if let Some(date) = metadata.metadata_value_date {
                    // For debugging, show the date value in different formats
                    println!("Serializing DATE metadata '{}' with value: {}", metadata.metadata_name, date);
                    
                    // Format date as string in YYYY-MM-DD format
                    let formatted_date = date.format("%Y-%m-%d").to_string();
                    println!("DATE '{}' as formatted string: {}", metadata.metadata_name, formatted_date);
                    
                    // Send the date as a string in ISO format (YYYY-MM-DD)
                    serde_json::Value::String(formatted_date)
                    
                    // Alternative: If you prefer to send as timestamp, convert NaiveDate to timestamp
                    // let timestamp = date.and_hms_opt(0, 0, 0).unwrap().timestamp();
                    // println!("DATE '{}' as timestamp: {}", metadata.metadata_name, timestamp);
                    // serde_json::Value::Number(serde_json::Number::from(timestamp))
                } else {
                    println!("DATE metadata '{}' has null value", metadata.metadata_name);
                    serde_json::Value::Null
                }
            },
            "BOOLEAN" => {
                if let Some(value) = metadata.metadata_value_boolean {
                    serde_json::Value::Bool(value)
                } else {
                    serde_json::Value::Null
                }
            },
            _ => serde_json::Value::Null,
        };
        
        // Only insert non-null values
        if value != serde_json::Value::Null {
            metadata_values.insert(metadata.metadata_name, value);
        }
    }
    
    // Convert map to response format
    let response_data: Vec<DocumentMetadatasResponse> = documents_map
        .into_iter()
        .map(|(document_uuid, metadata_values)| {
            DocumentMetadatasResponse {
                document_uuid,
                metadata_values,
            }
        })
        .collect();
    
    // Debug: count date metadata in response
    let mut date_count = 0;
    for doc in &response_data {
        for (key, value) in &doc.metadata_values {
            if key.to_lowercase().contains("date") {
                date_count += 1;
                println!("Response includes date field: {}, value: {:?}", key, value);
            }
        }
    }
    println!("Total date fields in response: {}", date_count);

    // Generate JSON response
    let response_body = json!({
        "statusAPI": "OK",
        "document_metadatas": response_data 
    });
    
    // Debug: Also check the serialized JSON for date values
    let json_str = response_body.to_string();
    println!("JSON response size: {} bytes", json_str.len());
    
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
