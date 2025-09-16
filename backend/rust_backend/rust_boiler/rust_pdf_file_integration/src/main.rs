use aws_config::meta::region::RegionProviderChain;
use aws_lambda_events::s3::S3Event;
use lambda_runtime::Error;
use aws_sdk_s3::{Client as S3Client, config::Region};
use aws_sdk_secretsmanager::Client as SecretManagerClient;
use md5::{Digest, Md5};
use std::env;
use chrono::Utc;
use tokio_postgres::Client;
use postgres_native_tls::MakeTlsConnector;
use native_tls::TlsConnector;
use uuid::Uuid;
use serde_json::{json, Value};
use pdf_extract; // PDF text extraction library
use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC}; // For URL encoding/decoding
use http::{Response, StatusCode, header};
use lambda_http::{Body, Request, run, service_fn};

const CHUNK_SIZE: usize = 1000;
const CHUNK_OVERLAP: usize = 100;

#[derive(Debug)]
struct Document {
    document_uuid: String,
    document_name: String,
    document_location: String,
    document_hash: String,
    document_type: String,
    document_length: i32,
    document_size: f64,
    document_status: String,
}

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

async fn connect_to_db() -> Result<Client, Error> {
    // Get DB connection string from environment variable
    let secret_name = env::var("DATABASE_CONECTION_STRING")?;
    
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

async fn check_document_exists(client: &Client, doc_hash: &str) -> Result<Option<Document>, Error> {
    let query = "SELECT document_uuid, document_name, document_location, document_hash, document_type, document_lenght, document_size, document_status FROM document_library.documents WHERE document_hash = $1";
    
    let rows = client.query(query, &[&doc_hash]).await.map_err(|e| {
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
        document_hash: row.get(3),
        document_type: row.get(4),
        document_length: row.get(5),
        document_size: row.get(6),
        document_status: row.get(7),
    };
    
    Ok(Some(doc))
}

async fn update_document(client: &Client, doc_uuid: &str, doc_name: &str) -> Result<(), Error> {
    let now = Utc::now();
    let query = "UPDATE document_library.documents SET document_status = 'duplicate', updated_date = $1, updated_by = 'system', document_name = $2 WHERE document_uuid = $3";
    
    client.execute(query, &[&now, &doc_name, &doc_uuid]).await.map_err(|e| {
        println!("Database update error: {}", e);
        format!("Database update error: {}", e)
    })?;
    
    Ok(())
}

async fn insert_document(client: &Client, doc: &Document) -> Result<String, Error> {
    let doc_uuid = Uuid::new_v4().to_string();
    let now = Utc::now();
    
    let query = "INSERT INTO document_library.documents (document_uuid, document_name, document_location, document_hash, document_type, document_lenght, document_size, document_status, chunk_time, creation_date, created_by, updated_date, updated_by, comments, tags) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NULL, NULL)";
    
    client.execute(
        query, 
        &[
            &doc_uuid, 
            &doc.document_name, 
            &doc.document_location, 
            &doc.document_hash,
            &doc.document_type,
            &doc.document_length,
            &doc.document_size,
            &"new",
            &0.0,  // chunk_time
            &now,  // creation_date
            &"system", // created_by
            &now,  // updated_date
            &"system", // updated_by
        ]
    ).await.map_err(|e| {
        println!("Database insert error: {}", e);
        format!("Database insert error: {}", e)
    })?;
    
    Ok(doc_uuid)
}

async fn insert_chunk(
    client: &Client, 
    document_uuid: &str, 
    chunk_text: &str, 
    chunk_length: i32, 
    chunk_overlap: i32, 
    chunk_hash: &str
) -> Result<String, Error> {
    let chunk_uuid = Uuid::new_v4().to_string();
    let now = Utc::now();
    
    let query = "INSERT INTO document_library.document_chunks (document_chunk_uuid, document_uuid, chunck_lenght, chunck_overlap, chunck_hash, embebed_text, creation_date, created_by, updated_date, updated_by, comments) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NULL)";
    
    client.execute(
        query, 
        &[
            &chunk_uuid, 
            &document_uuid, 
            &chunk_length, 
            &chunk_overlap, 
            &chunk_hash,
            &chunk_text,
            &now,  // creation_date
            &"system", // created_by
            &now,  // updated_date
            &"system", // updated_by
        ]
    ).await.map_err(|e| {
        println!("Database insert chunk error: {}", e);
        format!("Database insert chunk error: {}", e)
    })?;
    
    Ok(chunk_uuid)
}

async fn prepare_embedding_entry(
    client: &Client, 
    chunk_uuid: &str
) -> Result<(), Error> {
    println!("Preparing embedding entry for chunk: {}", chunk_uuid);
    let embedding_uuid = Uuid::new_v4().to_string();
    let now = Utc::now();
    
    let query = "INSERT INTO document_library.document_embeding_mistral_generic (document_embeding_uuid, document_chunk_uuid, embeder_type, embedding_token, embedding_time, embedding, creation_date, created_by, updated_date, updated_by, comments) VALUES ($1, $2, $3, NULL, NULL, NULL, $4, $5, $6, $7, NULL)";
    
    println!("Executing embedding insertion SQL query for chunk: {}", chunk_uuid);
    
    match client.execute(
        query, 
        &[
            &embedding_uuid, 
            &chunk_uuid, 
            &"mistral_generic",
            &now,  // creation_date
            &"system", // created_by
            &now,  // updated_date
            &"system", // updated_by
        ]
    ).await {
        Ok(rows_affected) => {
            println!("Successfully inserted embedding entry for chunk: {}. Rows affected: {}", chunk_uuid, rows_affected);
            Ok(())
        },
        Err(e) => {
            println!("Database insert embedding entry error for chunk {}: {}", chunk_uuid, e);
            Err(format!("Database insert embedding entry error: {}", e).into())
        }
    }
}

async fn insert_document_security_group(
    client: &Client,
    document_uuid: &str
) -> Result<(), Error> {
    println!("Inserting document security group for document: {}", document_uuid);
    
    // Read security group UUID from environment variable
    let security_group_uuid = match env::var("ALL_DOCUMENT_SECURITY_GROUP") {
        Ok(uuid) => uuid,
        Err(e) => {
            println!("Failed to read ALL_DOCUMENT_SECURITY_GROUP environment variable: {}", e);
            return Err(format!("Environment variable error: {}", e).into());
        }
    };
    
    let document_security_group_uuid = Uuid::new_v4().to_string();
    let now = Utc::now();
    
    let query = "INSERT INTO document_library.document_security_groups (
        document_security_group_uuid, 
        security_group_uuid, 
        document_uuid, 
        creation_date, 
        created_by, 
        updated_date, 
        updated_by
    ) VALUES ($1, $2, $3, $4, $5, $6, $7)";
    
    match client.execute(
        query, 
        &[
            &document_security_group_uuid,
            &security_group_uuid,
            &document_uuid,
            &now,  // creation_date
            &"system", // created_by
            &now,  // updated_date
            &"system", // updated_by
        ]
    ).await {
        Ok(rows_affected) => {
            println!("Successfully inserted document security group for document: {}. Rows affected: {}", document_uuid, rows_affected);
            Ok(())
        },
        Err(e) => {
            println!("Database insert document security group error for document {}: {}", document_uuid, e);
            Err(format!("Database insert document security group error: {}", e).into())
        }
    }
}

fn chunk_text(text: &str, chunk_size: usize, chunk_overlap: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let text_len = text.len();
    
    if text_len <= chunk_size {
        chunks.push(text.to_string());
        return chunks;
    }
    
    let mut start = 0;
    while start < text_len {
        let end = if start + chunk_size >= text_len {
            text_len
        } else {
            start + chunk_size
        };
        
        chunks.push(text[start..end].to_string());
        
        if end == text_len {
            break;
        }
        
        start = end - chunk_overlap;
    }
    
    chunks
}

async fn process_pdf_content(
    s3_client: &S3Client,
    db_client: &Client,
    bucket: &str,
    key: &str,
    document_uuid: &str
) -> Result<(), Error> {
    println!("Processing PDF content from bucket: {}, key: {}", bucket, key);
    
    // Get the object from S3
    let resp = s3_client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;
    
    // Read the PDF content into a buffer
    let mut buffer = Vec::new();
    let mut byte_stream = resp.body.into_async_read();
    
    use tokio::io::AsyncReadExt;
    byte_stream.read_to_end(&mut buffer).await?;
    
    // Create a temporary file to use with pdf-extract
    use std::fs::File;
    use std::io::Write;
    let temp_path = format!("/tmp/{}.pdf", Uuid::new_v4());
    let mut temp_file = File::create(&temp_path).map_err(|e| {
        println!("Failed to create temporary file: {}", e);
        format!("File creation error: {}", e)
    })?;
    
    temp_file.write_all(&buffer).map_err(|e| {
        println!("Failed to write to temporary file: {}", e);
        format!("File write error: {}", e)
    })?;
    
    // Extract text from the PDF using pdf-extract
    println!("Extracting text from PDF using pdf-extract");
    let text = match pdf_extract::extract_text(&temp_path) {
        Ok(content) => {
            println!("PDF text extraction successful");
            content
        },
        Err(e) => {
            println!("PDF text extraction failed: {}", e);
            
            // Fallback to raw text conversion if pdf-extract fails
            println!("Falling back to raw text conversion");
            String::from_utf8_lossy(&buffer).to_string()
        }
    };
    
    // Clean up the temporary file
    if let Err(e) = std::fs::remove_file(&temp_path) {
        println!("Warning: Failed to remove temporary file {}: {}", temp_path, e);
    }
    
    // Start timing for chunk processing
    let start_time = std::time::Instant::now();
    
    // Process the text in chunks
    let chunks = chunk_text(&text, CHUNK_SIZE, CHUNK_OVERLAP);
    let chunk_count = chunks.len();
    
    // Insert each chunk into the database
    println!("Starting chunk insertion for document {}, {} chunks to process", document_uuid, chunks.len());
    let mut processed_chunks = 0;
    
    for chunk in &chunks {
        let chunk_hash = format!("{:x}", Md5::digest(chunk.as_bytes()));
        let chunk_uuid = insert_chunk(
            db_client, 
            document_uuid, 
            chunk, 
            chunk.len() as i32,
            CHUNK_OVERLAP as i32, 
            &chunk_hash
        ).await?;
        
        processed_chunks += 1;
        if processed_chunks % 10 == 0 || processed_chunks == chunks.len() {
            println!("Processed {}/{} chunks", processed_chunks, chunks.len());
        }
        
        // Prepare embedding entry for this chunk
        println!("Starting embedding preparation for chunk {}", chunk_uuid);
        prepare_embedding_entry(db_client, &chunk_uuid).await?;
    }
    
    println!("All {} chunks have been inserted for document {}", chunks.len(), document_uuid);
    println!("All embedding entries have been prepared for document {}", document_uuid);
    
    // // Insert document security group after all embedding entries have been created
    // println!("Inserting document security group for document {}", document_uuid);
    // insert_document_security_group(db_client, document_uuid).await?;
    
    // Calculate the time spent for chunking
    let chunk_time = start_time.elapsed().as_secs_f64();
    
    // Update document with chunk time
    let update_query = "UPDATE document_library.documents SET chunk_time = $1 WHERE document_uuid = $2";
    db_client.execute(update_query, &[&chunk_time, &document_uuid]).await.map_err(|e| {
        println!("Failed to update document with chunk time: {}", e);
        format!("Database update error: {}", e)
    })?;
    
    println!("Successfully processed {} chunks for document {}", chunk_count, document_uuid);
    
    Ok(())
}

async fn copy_to_destination(
    s3_client: &S3Client,
    source_bucket: &str,
    source_key: &str,
    dest_bucket: &str,
    dest_key: &str
) -> Result<(), Error> {
    println!("Copying file from {}/{} to {}/{}", source_bucket, source_key, dest_bucket, dest_key);
    
    // Format copy source with proper URL encoding for S3
    // Special handling for + character: We need to replace it before encoding
    let source_key_fixed = source_key.replace("+", "%2B");
    let encoded_source_key = utf8_percent_encode(&source_key_fixed, NON_ALPHANUMERIC).to_string();
    let copy_source = format!("{}/{}", source_bucket, encoded_source_key);
    println!("Using copy source: {}", copy_source);
    
    match s3_client
        .copy_object()
        .copy_source(copy_source)
        .bucket(dest_bucket)
        .key(dest_key)
        .send()
        .await
    {
        Ok(_) => {
            println!("File copied successfully from {}/{} to {}/{}", 
                    source_bucket, source_key, dest_bucket, dest_key);
            Ok(())
        },
        Err(e) => {
            println!("Error copying file from {}/{} to {}/{}: {}", 
                    source_bucket, source_key, dest_bucket, dest_key, e);
            
            // Check for specific error types
            if e.to_string().contains("AccessDenied") || e.to_string().contains("access denied") {
                println!("Access denied error - check S3 bucket permissions");
            } else if e.to_string().contains("NoSuchBucket") {
                println!("Bucket does not exist: {}", dest_bucket);
            } else if e.to_string().contains("InvalidRequest") {
                println!("Invalid request - check source and destination paths");
            }
            
            Err(format!("S3 copy operation failed: {}", e).into())
        }
    }
}

async fn delete_from_source(
    s3_client: &S3Client,
    bucket: &str,
    key: &str
) -> Result<(), Error> {
    println!("Deleting file from source bucket: {}/{}", bucket, key);
    
    // Check if the key exists before attempting to delete
    // Make sure the key is properly encoded for S3 API
    let head_result = s3_client
        .head_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await;
        
    if let Err(e) = head_result {
        if e.to_string().contains("NotFound") {
            println!("WARNING: File not found in source bucket: {}/{}", bucket, key);
            println!("This could be due to the file being deleted already or the key being incorrect");
            // We'll return Ok here since the file is already gone
            return Ok(());
        }
    }
    
    match s3_client
        .delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
    {
        Ok(_) => {
            println!("File deleted successfully from {}/{}", bucket, key);
            Ok(())
        },
        Err(e) => {
            println!("Error deleting file from {}/{}: {}", bucket, key, e);
            
            // Check for specific error types
            if e.to_string().contains("AccessDenied") || e.to_string().contains("access denied") {
                println!("Access denied error - check S3 bucket permissions for delete operation");
            } else if e.to_string().contains("NoSuchKey") {
                println!("File does not exist: {}/{}", bucket, key);
            }
            
            Err(format!("S3 delete operation failed: {}", e).into())
        }
    }
}

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Log the raw event JSON
    let payload = match event.body() {
        Body::Empty => Value::Null,
        Body::Text(text) => serde_json::from_str(text).unwrap_or_else(|_| Value::Null),
        Body::Binary(data) => serde_json::from_slice(data).unwrap_or_else(|_| Value::Null),
    };
    let raw_json = serde_json::to_string(&payload).unwrap_or_else(|_| "Failed to serialize payload".to_string());
    println!("Received raw JSON: {}", raw_json);
    
    // Check if this is an HTTP event (Records in body) or direct S3 event (Records at root)
    let s3_event_result = if payload.get("body").is_some() {
        // This might be an HTTP event with the S3 event in the body
        println!("Detected HTTP event structure, checking body for Records");
        
        // Check if body is base64 encoded
        let is_base64 = payload.get("isBase64Encoded")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
            
        // The body could be a string (possibly JSON or base64) or already parsed as an object
        match payload.get("body") {
            Some(body) if body.is_string() => {
                let body_str = body.as_str().unwrap();
                
                // Handle base64 encoded body
                let decoded_body = if is_base64 {
                    use base64::{Engine as _, engine::general_purpose};
                    match general_purpose::STANDARD.decode(body_str) {
                        Ok(decoded) => match String::from_utf8(decoded) {
                            Ok(s) => s,
                            Err(e) => {
                                println!("Failed to convert base64 decoded bytes to string: {}", e);
                                body_str.to_string()
                            }
                        },
                        Err(e) => {
                            println!("Failed to decode base64 body: {}", e);
                            body_str.to_string()
                        }
                    }
                } else {
                    body_str.to_string()
                };
                
                // Try to parse as JSON
                match serde_json::from_str::<Value>(&decoded_body) {
                    Ok(body_json) => serde_json::from_value::<S3Event>(body_json),
                    Err(e) => {
                        println!("Failed to parse body as JSON: {}", e);
                        serde_json::from_value::<S3Event>(payload.clone())
                    }
                }
            },
            Some(body) => {
                // Body is already parsed as JSON
                serde_json::from_value::<S3Event>(body.clone())
            },
            None => {
                // No body field, try the root
                serde_json::from_value::<S3Event>(payload.clone())
            }
        }
    } else {
        // Try to deserialize from root
        println!("Attempting to deserialize S3Event from root payload");
        serde_json::from_value::<S3Event>(payload.clone())
    };
    
    // Process the S3 event
    match s3_event_result {
        Ok(s3_event) => {
            // Return early if no records in the event
            if s3_event.records.is_empty() {
                let no_records_body = json!({"message": "No records in event"});
                let body_string = serde_json::to_string(&no_records_body).unwrap_or_default();
                
                let response_body = body_string;
                
                println!("No records in event, returning early with 200 status code");
                println!("Response: {}", response_body);
                return Ok(Response::builder()
                    .status(200)
                    .header("Content-Type", "application/json")
                    .body(Body::from(response_body))?);
            }
            
            // Get the S3 record
            let record = &s3_event.records[0];
            let bucket = record.s3.bucket.name.as_ref()
                .ok_or("Missing bucket name")?;
            
            // Get encoded key and decode it
            let encoded_key = record.s3.object.key.as_ref()
                .ok_or("Missing object key")?;
                
            // Decode URL-encoded characters in the key
            // Special handling for + character: replace + with %2B before decoding to prevent it from being interpreted as a space
            let encoded_key_fixed = encoded_key.replace("+", "%2B");
            let key = match percent_decode_str(&encoded_key_fixed).decode_utf8() {
                Ok(decoded) => {
                    let decoded_str = decoded.to_string();
                    println!("Successfully decoded key from '{}' to '{}'", encoded_key, decoded_str);
                    decoded_str
                },
                Err(e) => {
                    println!("Failed to decode key '{}': {}", encoded_key, e);
                    // Fall back to the original key if decoding fails
                    println!("Using original encoded key");
                    encoded_key.to_string()
                }
            };
    
            println!("Processing S3 event for bucket: {}, key: {}", bucket, key);
    
    // Get environment variables
    let destination_bucket = env::var("S3BUCKET_EXPORT_FOLDER")?;
    let source_prefix = env::var("SOURCE_PREFIX")?;
    let destination_prefix = env::var("DESTINATION_PREFIX")?;
    
    // Get the bucket region from environment variable, or default to Lambda's region
    let bucket_region = env::var("S3BUCKET_REGION")?;
    
    println!("Using S3 bucket region: {}", bucket_region);
    
    // Initialize AWS S3 client with the specific region
    let config = aws_config::from_env()
        .region(Region::new(bucket_region))
        .load()
        .await;
    let s3_client = S3Client::new(&config);
    
    // Get the file content from S3
    println!("Attempting to get file from S3: s3://{}/{}", bucket, key);
    
    // Make sure the key is properly encoded for S3 API
    // S3 needs specific handling for the '+' character and other special characters
    let resp = match s3_client
        .get_object()
        .bucket(bucket)
        .key(&key)
        .send()
        .await {
            Ok(response) => {
                println!("Successfully retrieved file from S3: s3://{}/{}", bucket, key);
                response
            },
            Err(e) => {
                // Log specific error message based on the error type
                if e.to_string().contains("NoSuchKey") {
                    println!("ERROR: File not found in S3: s3://{}/{}", bucket, key);
                    println!("This could be due to the file being deleted or the key being incorrectly decoded");
                    return Err(format!("File not found in S3 bucket: s3://{}/{}", bucket, key).into());
                } else if e.to_string().contains("Access Denied") || e.to_string().contains("AccessDenied") {
                    println!("ERROR: Access denied to S3: s3://{}/{}", bucket, key);
                    println!("Check IAM permissions for the Lambda function");
                    return Err(format!("Access denied to S3 bucket: s3://{}/{}", bucket, key).into());
                } else {
                    println!("Error retrieving file from S3: {}", e);
                    println!("Error details: {:?}", e);
                    println!("Original key from S3 event: {}", encoded_key);
                    println!("Decoded key used for access: {}", key);
                    println!("Bucket: {}", bucket);
                    return Err(format!("S3 get_object operation failed: {}", e).into());
                }
            }
        };
        
    // Get file metadata
    let file_size = resp.content_length as f64;
    let file_name = key.split('/').last().unwrap_or(&key).to_string();
    
    // Read the file content
    let mut buffer = Vec::new();
    let mut byte_stream = resp.body.into_async_read();
    
    use tokio::io::AsyncReadExt;
    byte_stream.read_to_end(&mut buffer).await?;
    
    // Compute MD5 hash of the file
    let file_hash = format!("{:x}", Md5::digest(&buffer));
    
    println!("File hash computed: {}", file_hash);
    
    // Connect to the database
    let db_client = connect_to_db().await?;
    
    // Check if document already exists
    let existing_doc = check_document_exists(&db_client, &file_hash).await?;
    
    // Destination path in the processed bucket
    let destination_key = if key.starts_with(&source_prefix) {
        // Replace source prefix with destination prefix
        format!("{}{}", destination_prefix, &key[source_prefix.len()..])
    } else {
        // Just add destination prefix if source prefix isn't present
        format!("{}{}", destination_prefix, key)
    };
    
    println!("Destination key set to: {}", destination_key);
    
    let document_uuid = match existing_doc {
        Some(doc) => {
            println!("Document with hash {} already exists with UUID {}", file_hash, doc.document_uuid);
            // Update existing document
            update_document(&db_client, &doc.document_uuid, &file_name).await?;
            doc.document_uuid
        },
        None => {
            // Create a new document record
            let document = Document {
                document_uuid: String::new(), // Will be generated in insert_document
                document_name: file_name.clone(),
                document_location: format!("s3://{}/{}", destination_bucket, destination_key),
                document_hash: file_hash,
                document_type: "PDF".to_string(),
                document_length: buffer.len() as i32,
                document_size: file_size,
                document_status: "new".to_string(),
            };
            
            println!("Inserting new document: {}", document.document_name);
            
            // Insert document and get the UUID
            let doc_uuid = insert_document(&db_client, &document).await?;
            
            // Process PDF and create chunks
            process_pdf_content(&s3_client, &db_client, bucket, &key, &doc_uuid).await?;
            
            // Insert document security group
            insert_document_security_group(&db_client, &doc_uuid).await?;
            
            doc_uuid
        }
    };
    
    // Copy file to destination bucket (for both new and duplicate files) with retry logic
    println!("Starting copy operation to destination bucket");
    
    // Implement a basic retry mechanism
    let max_retries = 3;
    let mut retry_count = 0;
    let mut last_error = None;
    
    while retry_count < max_retries {
        match copy_to_destination(&s3_client, bucket, &key, &destination_bucket, &destination_key).await {
            Ok(_) => {
                println!("Copy to destination completed successfully");
                last_error = None;
                break;
            },
            Err(e) => {
                retry_count += 1;
                println!("Error during copy to destination (attempt {}/{}): {}", 
                         retry_count, max_retries, e);
                
                last_error = Some(e);
                
                if retry_count < max_retries {
                    println!("Retrying copy operation in 1 second...");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
    
    // If we exhausted all retries and still have an error, return it
    if let Some(e) = last_error {
        println!("Copy operation failed after {} attempts", max_retries);
        return Err(e);
    }

    // Delete the file from the source bucket
    println!("Starting delete operation from source bucket");
    let delete_max_retries = 3;
    let mut delete_retry_count = 0;
    let mut delete_last_error = None;

    while delete_retry_count < delete_max_retries {
        match delete_from_source(&s3_client, bucket, &key).await {
            Ok(_) => {
                println!("Delete from source completed successfully");
                delete_last_error = None;
                break;
            },
            Err(e) => {
                delete_retry_count += 1;
                println!("Error during delete from source (attempt {}/{}): {}", 
                       delete_retry_count, delete_max_retries, e);
                
                delete_last_error = Some(e);
                
                if delete_retry_count < delete_max_retries {
                    println!("Retrying delete operation in 1 second...");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    // Log warning if delete failed but don't fail the whole function
    if let Some(e) = delete_last_error {
        println!("Warning: Delete operation failed after {} attempts: {}", delete_max_retries, e);
        // We continue processing even if the delete fails
    }
    
    println!("Creating response JSON");
    let response_body = json!({
        "message": "File processed successfully",
        "documentUuid": document_uuid,
        "sourceLocation": format!("s3://{}/{}", bucket, key),
        "destinationLocation": format!("s3://{}/{}", destination_bucket, destination_key)
    });
    
    // Convert the response body to a string
    let response_body_string = serde_json::to_string(&response_body).unwrap_or_default();
    
    println!("Lambda function completed successfully, returning response");
    println!("Response: {}", response_body_string);
    
    // Return using the Response builder pattern
    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(response_body_string))?)
        },
        Err(e) => {
            // Log the error and return the raw JSON for debugging
            println!("Failed to deserialize S3Event: {}. Raw JSON was: {}", e, raw_json);
            
            // Create error details
            let error_body = json!({
                "error": format!("Failed to parse event: {}", e),
                "rawJson": raw_json
            });
            
            let error_body_string = serde_json::to_string(&error_body).unwrap_or_default();
            
            println!("Returning error response with 200 status code");
            println!("Response: {}", error_body_string);
            
            // Return using the Response build
            Ok(Response::builder()
                .status(500)
                .header("Content-Type", "application/json")
                .body(Body::from(error_body_string))?)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Wrap the lambda execution in a catch-all error handler
    match lambda_http::run(service_fn(function_handler)).await {
        Ok(_) => {
            println!("Lambda runtime completed successfully");
            Ok(())
        },
        Err(e) => {
            // Log the detailed error information
            println!("Lambda runtime error: {:?}", e);
            println!("Error cause: {:?}", e.to_string());
            
            // Return the error to AWS Lambda
            Err(e)
        }
    }
}
