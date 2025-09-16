use aws_config::meta::region::RegionProviderChain;
use aws_lambda_events::s3::S3Event;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use aws_sdk_secretsmanager::Client as SecretManagerClient;
use std::env;
use chrono::Utc;
use tokio_postgres::Client;
use tokio_postgres::types::{Type, ToSql, IsNull};
use postgres_native_tls::MakeTlsConnector;
use native_tls::TlsConnector;
use uuid::Uuid;
use serde_json::{json, Value};
use reqwest;
use std::sync::Arc;
use bytes::BytesMut;

// Define data structures for representing database entities
#[derive(Debug)]
struct Document {
    document_uuid: String,
    document_name: Option<String>,
    document_location: Option<String>,
}

#[derive(Debug)]
struct DocumentChunk {
    document_chunk_uuid: String,
    document_uuid: String,
    embebed_text: Option<String>,
}

// Custom Vector type for PostgreSQL compatibility
#[derive(Debug, Clone)]
struct PgVector(Vec<f32>);

// Implementation of PostgreSQL's ToSql trait for our custom vector type
impl ToSql for PgVector {
    fn to_sql(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        // Ensure the SQL type is 'vector'
        if ty.name() != "vector" {
            return Err(format!("Expected vector type, got {}", ty.name()).into());
        }
        
        // Write the dimension as a 2-byte big-endian integer
        let dim = self.0.len() as u16;
        out.extend_from_slice(&dim.to_be_bytes());
        
        // Write unused 2 bytes (for flags/future use in pgvector)
        out.extend_from_slice(&[0, 0]);
        
        // Write each float as a 4-byte representation
        for val in &self.0 {
            out.extend_from_slice(&val.to_bits().to_be_bytes());
        }
        
        Ok(IsNull::No)
    }

    fn accepts(ty: &Type) -> bool {
        ty.name() == "vector"
    }

    fn to_sql_checked(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        self.to_sql(ty, out)
    }
}

// Function to retrieve secrets from AWS Secrets Manager
async fn show_secret(secret_name: &str) -> Result<String, Error> {
    let region_provider = RegionProviderChain::default_provider();
    let config = aws_config::from_env().region(region_provider).load().await;
    let client = SecretManagerClient::new(&config);
    
    let resp = client.get_secret_value().secret_id(secret_name).send().await?;
    match resp.secret_string() {
        Some(secret) => Ok(secret.into()),
        None => {
            println!("Error retrieving secret: {}", secret_name);
            Err("Error retrieving secret".into())
        }
    }
}

// Function to establish connection to the database
async fn connect_to_db() -> Result<Client, Error> {
    // Get DB connection string from environment variable
    let secret_name = env::var("DATABASE_CONECTION_STRING").expect("DATABASE_CONECTION_STRING environment variable not set");
    
    // Get the secret from AWS Secrets Manager
    let db_secret = show_secret(&secret_name).await?;
    
    // Parse the JSON secret
    let db_credentials: serde_json::Value = match serde_json::from_str(&db_secret) {
        Ok(creds) => creds,
        Err(e) => {
            println!("Failed to parse database credentials JSON: {}", e);
            return Err(format!("JSON parsing error: {}", e).into());
        }
    };
    
    // Extract connection parameters
    let db_server = db_credentials["DB_HOST"].as_str()
        .ok_or("DB_HOST not found in secret")?;
    let db_port = db_credentials["DB_PORT"].as_str()
        .ok_or("DB_PORT not found in secret")?;
    let database = db_credentials["DB_NAME"].as_str()
        .ok_or("DB_NAME not found in secret")?;
    let db_username = db_credentials["DB_USER"].as_str()
        .ok_or("DB_USER not found in secret")?;
    let db_password = db_credentials["DB_PASSWORD"].as_str()
        .ok_or("DB_PASSWORD not found in secret")?;
    
    println!("Database connection parameters retrieved successfully");
    
    // Set up TLS for PostgreSQL connection
    let tls_connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| {
            println!("Failed to create TLS connector: {}", e);
            format!("TLS error: {}", e)
        })?;
        
    let postgres_tls = MakeTlsConnector::new(tls_connector);
    
    // Create a properly formatted connection string
    let connection_string = format!(
        "host={} port={} user={} password={} dbname={}",
        db_server, db_port, db_username, db_password, database
    );
    
    println!("Attempting to connect to database...");
    
    // Connect to PostgreSQL
    let (client, connection) = tokio_postgres::connect(&connection_string, postgres_tls)
        .await
        .map_err(|e| {
            println!("Failed to connect to database: {}", e);
            format!("Database connection error: {}", e)
        })?;
        
    // The connection object performs the actual communication with the database,
    // so spawn it off to run on its own
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            println!("Database connection error: {}", e);
        }
    });
    
    Ok(client)
}

// Function to find document by name
async fn find_document_by_name(client: &Client, name: &str) -> Result<Option<Document>, Error> {
    let query = "SELECT document_uuid, document_name, document_location FROM document_library.documents WHERE document_name = $1";
    
    let rows = client.query(query, &[&name]).await.map_err(|e| {
        println!("Database query error: {}", e);
        format!("Database query error: {}", e)
    })?;
    
    if rows.is_empty() {
        return Ok(None);
    }
    
    let row = &rows[0];
    let doc = Document {
        document_uuid: row.get(0),
        document_name: row.get(1),
        document_location: row.get(2),
    };
    
    Ok(Some(doc))
}

// Function to find document by location
async fn find_document_by_location(client: &Client, location: &str) -> Result<Option<Document>, Error> {
    let query = "SELECT document_uuid, document_name, document_location FROM document_library.documents WHERE document_location = $1";
    
    let rows = client.query(query, &[&location]).await.map_err(|e| {
        println!("Database query error: {}", e);
        format!("Database query error: {}", e)
    })?;
    
    if rows.is_empty() {
        return Ok(None);
    }
    
    let row = &rows[0];
    let doc = Document {
        document_uuid: row.get(0),
        document_name: row.get(1),
        document_location: row.get(2),
    };
    
    Ok(Some(doc))
}

// Function to get document chunks for a document
async fn get_document_chunks(client: &Client, document_uuid: &str) -> Result<Vec<DocumentChunk>, Error> {
    let query = "SELECT document_chunk_uuid, document_uuid, embebed_text FROM document_library.document_chunks WHERE document_uuid = $1";
    
    let rows = client.query(query, &[&document_uuid]).await.map_err(|e| {
        println!("Database query error: {}", e);
        format!("Database query error: {}", e)
    })?;
    
    let chunks: Vec<DocumentChunk> = rows.iter().map(|row| {
        DocumentChunk {
            document_chunk_uuid: row.get(0),
            document_uuid: row.get(1),
            embebed_text: row.get(2),
        }
    }).collect();
    
    Ok(chunks)
}

// Function to check if embedding exists for a document chunk
async fn check_embedding_exists(client: &Client, document_chunk_uuid: &str) -> Result<bool, Error> {
    let query = "SELECT count(*) FROM document_library.document_embeding_mistral_generic WHERE document_chunk_uuid = $1 AND embedding IS NOT NULL";
    
    let rows = client.query(query, &[&document_chunk_uuid]).await.map_err(|e| {
        println!("Database query error: {}", e);
        format!("Database query error: {}", e)
    })?;
    
    let count: i64 = rows[0].get(0);
    
    Ok(count > 0)
}

// Function to generate embedding using Ollama API
async fn generate_embedding(text: &str) -> Result<(Vec<f32>, f64), Error> {
    let ollama_api_url = env::var("OLLAMA_API_URL").expect("OLLAMA_API_URL environment variable not set");
    
    let start_time = std::time::Instant::now();
    
    let client = reqwest::Client::new();
    let response = client.post(&ollama_api_url)
        .json(&json!({
            "model": "nomic-embed-text",
            "prompt": text
        }))
        .send()
        .await
        .map_err(|e| {
            println!("Failed to make request to Ollama API: {}", e);
            format!("Ollama API request error: {}", e)
        })?;
    
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        println!("Ollama API returned error: {}", error_text);
        return Err(format!("Ollama API error: {}", error_text).into());
    }
    
    let embedding_result = response.json::<Value>().await.map_err(|e| {
        println!("Failed to parse Ollama API response: {}", e);
        format!("Ollama API response parsing error: {}", e)
    })?;
    
    let embedding = match embedding_result["embedding"].as_array() {
        Some(arr) => arr.iter()
                      .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                      .collect::<Vec<f32>>(),
        None => {
            println!("Embedding not found in response");
            return Err("Embedding not found in response".into());
        }
    };
    
    let elapsed = start_time.elapsed();
    let elapsed_seconds = elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9;
    
    Ok((embedding, elapsed_seconds))
}

// Function to insert embedding into the database
async fn insert_or_update_embedding(
    client: &Client,
    document_chunk_uuid: &str,
    embedding: &Vec<f32>,
    embedding_time: f64
) -> Result<(), Error> {
    // Check if embedding already exists for this chunk
    let check_query = "SELECT document_embeding_uuid FROM document_library.document_embeding_mistral_generic WHERE document_chunk_uuid = $1";
    let rows = client.query(check_query, &[&document_chunk_uuid]).await.map_err(|e| {
        println!("Database query error: {}", e);
        format!("Database query error: {}", e)
    })?;
    
    let now = Utc::now();
    
    // Convert the Vec<f32> to our custom PgVector type that implements ToSql
    let pg_vector = PgVector(embedding.clone());
    
    if rows.is_empty() {
        // Insert new embedding
        let document_embeding_uuid = Uuid::new_v4().to_string();
        let query = "INSERT INTO document_library.document_embeding_mistral_generic (document_embeding_uuid, document_chunk_uuid, embeder_type, embedding_token, embedding_time, embedding, creation_date, created_by, updated_date, updated_by) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)";
        
        client.execute(
            query, 
            &[
                &document_embeding_uuid,
                &document_chunk_uuid,
                &"nomic-embed-text",
                &(-1), // Ollama doesn't provide token count
                &embedding_time,
                &pg_vector, // Use our custom PgVector type
                &now,
                &"system",
                &now,
                &"system"
            ]
        ).await.map_err(|e| {
            println!("Database insert error: {}", e);
            format!("Database insert error: {}", e)
        })?;
    } else {
        // Update existing embedding
        let document_embeding_uuid = rows[0].get::<_, String>(0);
        let query = "UPDATE document_library.document_embeding_mistral_generic SET embedding = $1, embedding_time = $2, updated_date = $3, updated_by = $4 WHERE document_embeding_uuid = $5";
        
        client.execute(
            query, 
            &[
                &pg_vector, // Use our custom PgVector type
                &embedding_time,
                &now,
                &"system",
                &document_embeding_uuid
            ]
        ).await.map_err(|e| {
            println!("Database update error: {}", e);
            format!("Database update error: {}", e)
        })?;
    }
    
    Ok(())
}

async fn handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    println!("Received event: {:?}", event.payload);
    let start_time = std::time::Instant::now();
    
    // Initialize the database connection
    let client = connect_to_db().await?;
    
    // Detect event type (HTTP API Gateway event or direct S3 event)
    let event_type = detect_event_type(&event.payload);
    println!("Detected event type: {}", event_type);
    
    // Parse the event based on its type
    let s3_event: S3Event = match parse_s3_event(event.payload.clone(), event_type) {
        Ok(s3_event) => s3_event,
        Err(e) => {
            println!("Failed to parse S3 event: {}", e);
            return Ok(json!({
                "statusCode": 400,
                "body": format!("Failed to parse S3 event: {}", e)
            }));
        }
    };

    if s3_event.records.is_empty() {
        return Ok(json!({
            "statusCode": 400,
            "body": "No S3 records in event"
        }));
    }
    
    let record = &s3_event.records[0];
    let bucket_name = record.s3.bucket.name.clone().unwrap_or_else(|| "unknown".to_string());
    let object_key = match record.s3.object.key.clone() {
        Some(key) => key,
        None => {
            return Ok(json!({
                "statusCode": 400,
                "body": "S3 object key is missing"
            }));
        }
    };
    
    // Get the source prefix from environment variable
    let source_prefix = env::var("SOURCE_PREFIX").expect("SOURCE_PREFIX environment variable not set");
    
    println!("Processing S3 event for bucket: {}, key: {}", bucket_name, object_key);
    
    // Extract the filename from the object key
    let file_name = match object_key.rsplit('/').next() {
        Some(name) => name,
        None => &object_key,
    };
    
    println!("Extracted file name: {}", file_name);
    
    // First try to find document by name
    let document = match find_document_by_name(&client, &file_name).await? {
        Some(doc) => {
            println!("Found document by name: {}", file_name);
            doc
        },
        None => {
            // If not found by name, try with the full path but remove prefix if exists
            println!("Document not found by name, trying by location");
            let cleaned_object_key = if object_key.starts_with(&source_prefix) {
                object_key[source_prefix.len()..].to_string()
            } else {
                object_key.clone()
            };
            
            println!("Searching for document with location: {}", cleaned_object_key);
            
            match find_document_by_location(&client, &cleaned_object_key).await? {
                Some(doc) => doc,
                None => {
                    println!("Document not found in database for location: {}", cleaned_object_key);
                    return Ok(json!({
                        "statusCode": 404,
                        "body": format!("Document not found in database for name '{}' or location '{}'", file_name, cleaned_object_key)
                    }));
                }
            }
        }
    };
    
    println!("Found document with UUID: {}", document.document_uuid);
    
    // Get all document chunks for this document
    let chunks = get_document_chunks(&client, &document.document_uuid).await?;
    
    println!("Found {} document chunks", chunks.len());
    
    // Process each chunk
    let mut processed_chunks = 0;
    let mut skipped_chunks = 0;
    
    for chunk in &chunks {
        // Skip chunks without text
        if chunk.embebed_text.is_none() || chunk.embebed_text.as_ref().unwrap().is_empty() {
            println!("Skipping chunk {} with empty text", chunk.document_chunk_uuid);
            continue;
        }
        
        // Check if embedding already exists
        let embedding_exists = check_embedding_exists(&client, &chunk.document_chunk_uuid).await?;
        if embedding_exists {
            println!("Skipping chunk {} as embedding already exists", chunk.document_chunk_uuid);
            skipped_chunks += 1;
            continue;
        }
        
        // Generate embedding
        let (embedding, embedding_time) = match generate_embedding(chunk.embebed_text.as_ref().unwrap()).await {
            Ok(result) => result,
            Err(e) => {
                println!("Failed to generate embedding for chunk {}: {}", chunk.document_chunk_uuid, e);
                continue;
            }
        };
        
        // Store embedding in the database
        match insert_or_update_embedding(&client, &chunk.document_chunk_uuid, &embedding, embedding_time).await {
            Ok(_) => {
                println!("Successfully processed chunk {}", chunk.document_chunk_uuid);
                processed_chunks += 1;
            },
            Err(e) => {
                println!("Failed to insert embedding for chunk {}: {}", chunk.document_chunk_uuid, e);
            }
        }
    }
    
    let elapsed = start_time.elapsed();
    let elapsed_seconds = elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9;
    
    println!("Vectorization process completed. Processed: {}, Skipped: {}, Total time: {} seconds", 
          processed_chunks, skipped_chunks, elapsed_seconds);
    
    Ok(json!({
        "statusCode": 200,
        "body": json!({
            "message": "Vectorization process completed",
            "document_uuid": document.document_uuid,
            "processed_chunks": processed_chunks,
            "skipped_chunks": skipped_chunks,
            "total_chunks": chunks.len(),
            "processing_time_seconds": elapsed_seconds
        })
    }))
}

// Enum to represent the different event types
#[derive(Debug)]
enum EventType {
    DirectS3,
    HttpApiGateway,
    Unknown,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::DirectS3 => write!(f, "DirectS3"),
            EventType::HttpApiGateway => write!(f, "HttpApiGateway"),
            EventType::Unknown => write!(f, "Unknown"),
        }
    }
}

// Function to detect the type of event
fn detect_event_type(payload: &Value) -> EventType {
    // Check if it's a direct S3 event (has Records array with s3 object)
    if let Some(records) = payload.get("Records") {
        if records.is_array() && !records.as_array().unwrap().is_empty() {
            if records.as_array().unwrap()[0].get("s3").is_some() {
                return EventType::DirectS3;
            }
        }
    }
    
    // Check if it's an HTTP API Gateway event (has body, headers, etc.)
    if payload.get("body").is_some() && 
       (payload.get("headers").is_some() || 
        payload.get("requestContext").is_some() || 
        payload.get("pathParameters").is_some() || 
        payload.get("queryStringParameters").is_some()) {
        return EventType::HttpApiGateway;
    }
    
    EventType::Unknown
}

fn parse_s3_event(payload: Value, event_type: EventType) -> Result<S3Event, Error> {
    match event_type {
        EventType::DirectS3 => {
            // Direct S3 event, try to parse directly
            match serde_json::from_value::<S3Event>(payload.clone()) {
                Ok(s3_event) => Ok(s3_event),
                Err(e) => {
                    println!("Failed to parse direct S3 event: {}", e);
                    Err(format!("Failed to parse direct S3 event: {}", e).into())
                }
            }
        },
        EventType::HttpApiGateway => {
            // HTTP API Gateway event, extract S3 event from body
            if let Some(body) = payload.get("body") {
                // Try parsing the body as a string JSON
                if let Some(body_str) = body.as_str() {
                    // Try parsing as S3Event directly
                    match serde_json::from_str::<S3Event>(body_str) {
                        Ok(s3_event) => return Ok(s3_event),
                        Err(_) => {
                            // If it's not a valid S3Event JSON string, try parsing it as a generic JSON value first
                            match serde_json::from_str::<Value>(body_str) {
                                Ok(body_json) => match serde_json::from_value::<S3Event>(body_json) {
                                    Ok(s3_event) => return Ok(s3_event),
                                    Err(e) => {
                                        println!("Failed to parse S3 event from body JSON: {}", e);
                                    }
                                },
                                Err(e) => {
                                    println!("Failed to parse body as JSON: {}", e);
                                }
                            }
                        }
                    }
                } else {
                    // Body is not a string, try parsing as JSON object
                    match serde_json::from_value::<S3Event>(body.clone()) {
                        Ok(s3_event) => return Ok(s3_event),
                        Err(e) => {
                            println!("Failed to parse S3 event from body object: {}", e);
                        }
                    }
                }
            }
            
            Err("Could not extract S3 event from HTTP API Gateway event".into())
        },
        EventType::Unknown => {
            // Unknown event type, try various parsing strategies
            
            // Try to parse the payload as an S3Event directly
            let s3_event: Result<S3Event, _> = serde_json::from_value(payload.clone());
            if let Ok(event) = s3_event {
                return Ok(event);
            }
            
            // If direct parsing fails, try to handle different event formats
            if let Some(body) = payload.get("body") {
                // Try parsing the body as a string JSON
                if let Some(body_str) = body.as_str() {
                    match serde_json::from_str::<S3Event>(body_str) {
                        Ok(s3_event) => return Ok(s3_event),
                        Err(_) => {
                            // If it's not a valid JSON string, try parsing it as a JSON value
                            match serde_json::from_str::<Value>(body_str) {
                                Ok(body_json) => match serde_json::from_value::<S3Event>(body_json) {
                                    Ok(s3_event) => return Ok(s3_event),
                                    Err(_) => {}
                                },
                                Err(_) => {}
                            }
                        }
                    }
                }
                
                // Try parsing the body as a JSON object
                match serde_json::from_value::<S3Event>(body.clone()) {
                    Ok(s3_event) => return Ok(s3_event),
                    Err(_) => {}
                }
            }
            
            // Try parsing from "Records" directly if it exists
            if let Some(records) = payload.get("Records") {
                let constructed_event = json!({ "Records": records });
                match serde_json::from_value::<S3Event>(constructed_event) {
                    Ok(s3_event) => return Ok(s3_event),
                    Err(_) => {}
                }
            }
            
            // Last resort - log detailed information and report failure
            let raw_json = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "Unable to serialize payload".to_string());
            println!("Failed to parse S3 event from unknown event type. Raw JSON was: {}", raw_json);
            Err("Failed to parse S3 event from unknown event type".into())
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();
        
    // Start the Lambda runtime
    lambda_runtime::run(service_fn(handler)).await?;
    
    Ok(())
}
