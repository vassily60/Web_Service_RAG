use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use std::env;
use chrono::{DateTime, Utc};
use jsonwebtokens_cognito::KeySet;

use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_secretsmanager::Client as SecretManagerClient;

use tokio_postgres::{Client, Error as OtherError};
use postgres_native_tls::MakeTlsConnector;
use native_tls::TlsConnector;

// Document struct to map query results
#[derive(Debug, Serialize, Deserialize)]
struct DocumentMetadata {
    metadata_uuid: String,
    metadata_name: String,
    metadata_type: String,  // New field to store the type of metadata (STRING, INTEGER, NUMBER, BOOLEAN, DATE)
    metadata_value_string: Option<String>,
    metadata_value_int: Option<i32>,
    metadata_value_float: Option<f64>,
    metadata_value_boolean: Option<bool>,
    metadata_value_date: Option<chrono::NaiveDate>,
}

// Document struct to map query results
#[derive(Debug, Serialize, Deserialize)]
struct DocumentChunkCount {
    document_uuid: String,
    created_by: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    creation_date: DateTime<Utc>,
    document_name: String,
    document_location: String,
    lenght_chunk: i64,
    tags: Vec<String>,
    document_metadata: Option<Vec<DocumentMetadata>>,
}

// Request struct to parse incoming parameters
#[derive(Debug, Serialize, Deserialize)]
struct DocumentFilter {
    filter_type: String,
    filter_value: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DocumentListRequest {
    client_id: Option<i32>,
    tags: Option<Vec<String>>,
    document_filters: Option<Vec<DocumentFilter>>,
}

async fn show_secret(client: &SecretManagerClient, name: &str) -> Result<String, Error> {
    let resp = client.get_secret_value().secret_id(name).send().await?;
    match resp.secret_string() {
        Some(secret) => Ok(secret.into()),
        None => panic!("Error to get the secret: {:?}", name),
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
    
    // Get region from environment variable
    let region_name = env::var("REGION").expect("REGION environment variable not set");
    println!("Using region: {}", region_name);
    
    // Initialize AWS SDK configuration
    let region = Region::new(region_name.clone());
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(region)
        .load()
        .await;
    let client_secret = SecretManagerClient::new(&config);
    println!("AWS SDK initialized!");
    
    // Get Cognito configuration from AWS Secrets Manager
    let cognito_secret_name = env::var("COGNITO_SECRET").expect("COGNITO_SECRET environment variable not set");
    let secret_content = show_secret(&client_secret, &cognito_secret_name).await?;
    let cognito_credentials: Value = serde_json::from_str(&secret_content)?;
    
    // Extract user_pool_id and client_id from the secret
    let user_pool_id = cognito_credentials["USER_POOL_ID"].as_str()
        .ok_or_else(|| Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "USER_POOL_ID not found in secret")))?;
    let client_id = cognito_credentials["APP_CLIENT_ID"].as_str()
        .ok_or_else(|| Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "APP_CLIENT_ID not found in secret")))?;
    let cognito_region_name = cognito_credentials["REGION"].as_str()
        .unwrap_or(&region_name); // Default to the same region if not specified in the secret
    
    println!("Retrieved Cognito configuration from Secrets Manager!");

    // Create a KeySet for AWS Cognito
    let keyset = KeySet::new(cognito_region_name, user_pool_id);
    
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
    
    // Parse request body to get tags if present
    let body = event.body();
    let request_data: DocumentListRequest = match serde_json::from_slice(body.as_ref()) {
        Ok(data) => data,
        Err(e) => {
            println!("Failed to parse request body: {}", e);
            // Use default empty request if parsing fails
            DocumentListRequest {
                client_id: None,
                tags: None,
                document_filters: None,
            }
        }
    };
    
    println!("Request data parsed: {:?}", request_data);
    
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
    println!("Server: {}",db_server);
    let database = db_credentials["DB_NAME"].as_str().unwrap();
    println!("Database: {}",database);
    let db_username = db_credentials["DB_USER"].as_str().unwrap();
    let db_password = db_credentials["DB_PASSWORD"].as_str().unwrap();
    let db_port = db_credentials["DB_PORT"].as_str().unwrap();
    println!("End of get info from secret!");


    let tls_connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true) // Disable certificate validation
        .build();
    let tls = MakeTlsConnector::new(tls_connector.expect("REASON"));

    println!("END OF TLS");

    // Connect to the database with better error handling
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
                .header("content-type", "text/plain")
                .body("Internal server error: Database connection failed".into())
                .map_err(Box::new)?);
        }
    };

    println!("END OF DATABASE CONNECT");
    
    // Spawn a new task to manage the connection
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    // Build the query with optional tag filtering
    let mut query = String::from("select 
        dc.document_uuid as document_uuid,
        d.created_by as created_by,
        d.creation_date as creation_date,
        d.document_name as document_name,
        d.document_location as document_location,
        sum(dc.chunck_lenght) as lenght_chunk,
        d.tags as tags
        from document_library.document_chunks dc
        inner join document_library.documents d on d.document_uuid = dc.document_uuid
        INNER JOIN document_library.document_security_groups dsg on dsg.document_uuid = d.document_uuid
        INNER JOIN document_library.security_groups sg on sg.security_group_uuid = dsg.security_group_uuid
        INNER JOIN document_library.user_security_groups usg on usg.security_group_uuid = sg.security_group_uuid
        INNER JOIN document_library.users u on u.user_uuid = usg.user_uuid and u.sso_unique_id = $1");
    
    // Add tag filtering if tags are provided
    let mut where_clauses = Vec::new();
    
    if let Some(tags) = &request_data.tags {
        if !tags.is_empty() {
            // Convert tags to a PostgreSQL array literal
            let tags_array = tags
                .iter()
                .map(|tag| format!("'{}'", tag.replace("'", "''"))) // Escape single quotes
                .collect::<Vec<_>>()
                .join(", ");
            
            where_clauses.push(format!("d.tags && ARRAY[{}]", tags_array));
            println!("Filtering by tags: {:?}", tags);
        }
    }
    
    // Process document filters (including metadata filters)
    if let Some(filters) = &request_data.document_filters {
        for filter in filters {
            match filter.filter_type.as_str() {
                "metadata" => {
                    // Parse the metadata filter JSON
                    if let Ok(metadata_filter) = serde_json::from_str::<serde_json::Value>(&filter.filter_value) {
                        // Check if the required fields exist in the metadata filter
                        if let (Some(metadata_uuid), Some(operator)) = (
                            metadata_filter["metadata_uuid"].as_str(),
                            metadata_filter["operator"].as_str()
                        ) {
                            // Get the value from the metadata filter
                            let value = &metadata_filter["value"];
                            
                            // First, get the metadata type from the database
                            match get_metadata_type(&client, metadata_uuid).await {
                                Ok(metadata_type) => {
                                    println!("Retrieved metadata type for {}: {}", metadata_uuid, metadata_type);
                                    
                                    // Create the proper filter condition based on the metadata type
                                    if let Some(condition) = create_metadata_filter_condition(&metadata_type, operator, value) {
                                        // Add the subquery to filter documents by metadata
                                        where_clauses.push(format!(
                                            "EXISTS (SELECT 1 FROM document_library.document_metadatas dm WHERE dm.document_uuid = d.document_uuid AND dm.metadata_uuid = '{}' AND {})",
                                            metadata_uuid, condition
                                        ));
                                    } else {
                                        println!("Warning: Could not create filter condition for metadata {} with type {}, operator {}, and value {:?}",
                                            metadata_uuid, metadata_type, operator, value);
                                    }
                                },
                                Err(e) => {
                                    println!("Error fetching metadata type for {}: {}", metadata_uuid, e);
                                    // Skip this filter due to error
                                    continue;
                                }
                            }
                        }
                    }
                },
                _ => continue, // Skip unsupported filter types
            }
        }
    }
    
    // Construct the query with WHERE clause
    if !where_clauses.is_empty() {
        query.push_str(" WHERE ");
        query.push_str(&where_clauses.join(" AND "));
    }
    
    // Add group by clause
    query.push_str(" group by 
        dc.document_uuid,
        d.created_by,
        d.creation_date,
        d.document_name,
        d.document_location,
        d.tags;");
    
    println!("Executing query: {}", query);
    let rows = client.query(&query, &[&user_email]).await?;
    
    // Parse rows into DocumentChunkCount structs and fetch metadata for each document
    let mut documents: Vec<DocumentChunkCount> = Vec::new();
    
    for row in rows {
        let document_uuid: String = row.get("document_uuid");
        let tags: Vec<String> = match row.try_get("tags") {
            Ok(tags) => tags,
            Err(_) => Vec::new(), // Default to empty vector if NULL
        };

        // Create the document without metadata first
        let mut document = DocumentChunkCount {
            document_uuid: document_uuid.clone(),
            created_by: row.get("created_by"),
            creation_date: row.get("creation_date"),
            document_name: row.get("document_name"),
            document_location: row.get("document_location"),
            lenght_chunk: row.get("lenght_chunk"),
            tags,
            document_metadata: None,
        };
        
        /*
        // Fetch metadata for this document
        let metadata_query = "SELECT
            dm.metadata_uuid,
            m.metadata_name,
            m.metadata_type,
            dm.metadata_value_string,
            dm.metadata_value_int,
            dm.metadata_value_float,
            dm.metadata_value_boolean,
            dm.metadata_value_date
        FROM document_library.document_metadatas dm
        JOIN document_library.metadatas m ON dm.metadata_uuid = m.metadata_uuid
        WHERE dm.document_uuid = $1";
        
        let metadata_rows = match client.query(metadata_query, &[&document_uuid]).await {
            Ok(rows) => rows,
            Err(e) => {
                println!("Error fetching metadata for document {}: {}", document_uuid, e);
                Vec::new()
            }
        };
        
        if !metadata_rows.is_empty() {
            let mut metadata_list: Vec<DocumentMetadata> = Vec::new();
            
            for meta_row in metadata_rows {
                // Get the metadata type from the database
                let metadata_type = meta_row.get::<_, String>("metadata_type");
                let metadata_uuid: String = meta_row.get("metadata_uuid");
                let metadata_name: String = meta_row.get("metadata_name");
                
                // Log the metadata information
                println!("Processing metadata: UUID={}, Name={}, Type={}", 
                    metadata_uuid, metadata_name, metadata_type);
                
                // Handle different metadata types to ensure proper value extraction
                let value_for_log = match metadata_type.as_str() {
                    "INTEGER" => {
                        if let Ok(value) = meta_row.try_get::<_, i32>("metadata_value_int") {
                            println!("  INTEGER value: {}", value);
                        }
                        "INTEGER value"
                    },
                    "NUMBER" => {
                        if let Ok(value) = meta_row.try_get::<_, f64>("metadata_value_float") {
                            println!("  NUMBER value: {}", value);
                        }
                        "NUMBER value"
                    },
                    "STRING" => {
                        if let Ok(value) = meta_row.try_get::<_, String>("metadata_value_string") {
                            println!("  STRING value: {}", value);
                        }
                        "STRING value"
                    },
                    "BOOLEAN" => {
                        if let Ok(value) = meta_row.try_get::<_, bool>("metadata_value_boolean") {
                            println!("  BOOLEAN value: {}", value);
                        }
                        "BOOLEAN value"
                    },
                    "DATE" => {
                        if let Ok(value) = meta_row.try_get::<_, chrono::NaiveDate>("metadata_value_date") {
                            println!("  DATE value: {}", value);
                        }
                        "DATE value"
                    },
                    _ => "Unknown value type"
                };
                
                let metadata = DocumentMetadata {
                    metadata_uuid,
                    metadata_name,
                    metadata_type: metadata_type.clone(), // Store the type for later use
                    metadata_value_string: meta_row.try_get("metadata_value_string").ok(),
                    metadata_value_int: meta_row.try_get("metadata_value_int").ok(),
                    metadata_value_float: meta_row.try_get("metadata_value_float").ok(),
                    metadata_value_boolean: meta_row.try_get("metadata_value_boolean").ok(),
                    metadata_value_date: meta_row.try_get("metadata_value_date").ok(),
                };
                
                metadata_list.push(metadata);
            }
            
            document.document_metadata = Some(metadata_list);
        }*/

        println!("Document UUID: {}, Document Name: {}, Created By: {}, Creation Date: {}, Length Chunk: {}, Tags: {:?}", 
                document.document_uuid, document.document_name, document.created_by, document.creation_date, document.lenght_chunk, document.tags);
        documents.push(document);
    }
    
    // You can now work with the strongly-typed documents collection
    println!("Total documents: {}", documents.len());

    // Generate JSON response
    let response_body = json!({ "documents": documents });
    
    let resp = Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(response_body.to_string().into())
        .map_err(Box::new)?;
    Ok(resp)
}

// Helper function to get metadata type from database
async fn get_metadata_type(client: &Client, metadata_uuid: &str) -> Result<String, Error> {
    let query = "SELECT metadata_type FROM document_library.metadatas WHERE metadata_uuid = $1";
    
    match client.query_one(query, &[&metadata_uuid]).await {
        Ok(row) => {
            let metadata_type: String = row.get("metadata_type");
            println!("Retrieved metadata type for {}: {}", metadata_uuid, metadata_type);
            Ok(metadata_type)
        },
        Err(e) => {
            eprintln!("Error fetching metadata type for {}: {}", metadata_uuid, e);
            // Default to STRING if we can't determine the type
            Ok("STRING".to_string())
        }
    }
}

// Helper function to create filter condition based on metadata type and value
fn create_metadata_filter_condition(
    metadata_type: &str,
    operator: &str,
    value: &serde_json::Value
) -> Option<String> {
    match metadata_type {
        "STRING" => {
            // String operations
            if let Some(string_value) = value.as_str() {
                let escaped_value = string_value.replace("'", "''");
                match operator {
                    "eq" => Some(format!("dm.metadata_value_string = '{}'", escaped_value)),
                    "neq" => Some(format!("dm.metadata_value_string != '{}'", escaped_value)),
                    "contains" => Some(format!("dm.metadata_value_string ILIKE '%{}%'", escaped_value)),
                    "not_contains" => Some(format!("dm.metadata_value_string NOT ILIKE '%{}%'", escaped_value)),
                    _ => None // Unsupported operator for strings
                }
            } else {
                None // Not a string value
            }
        },
        "INTEGER" => {
            // Integer operations
            if let Some(num) = value.as_i64() {
                match operator {
                    "eq" => Some(format!("dm.metadata_value_int = {}", num)),
                    "neq" => Some(format!("dm.metadata_value_int != {}", num)),
                    "gt" => Some(format!("dm.metadata_value_int > {}", num)),
                    "lt" => Some(format!("dm.metadata_value_int < {}", num)),
                    "gte" => Some(format!("dm.metadata_value_int >= {}", num)),
                    "lte" => Some(format!("dm.metadata_value_int <= {}", num)),
                    _ => None // Unsupported operator for integers
                }
            } else if let Some(string_value) = value.as_str() {
                // Try parsing the string as an integer
                if let Ok(num) = string_value.parse::<i32>() {
                    match operator {
                        "eq" => Some(format!("dm.metadata_value_int = {}", num)),
                        "neq" => Some(format!("dm.metadata_value_int != {}", num)),
                        "gt" => Some(format!("dm.metadata_value_int > {}", num)),
                        "lt" => Some(format!("dm.metadata_value_int < {}", num)),
                        "gte" => Some(format!("dm.metadata_value_int >= {}", num)),
                        "lte" => Some(format!("dm.metadata_value_int <= {}", num)),
                        _ => None // Unsupported operator for integers
                    }
                } else {
                    println!("Failed to parse '{}' as INTEGER", string_value);
                    None // Couldn't parse as integer
                }
            } else {
                None // Not an integer value
            }
        },
        "NUMBER" => {
            // Float operations
            if let Some(num) = value.as_f64() {
                match operator {
                    "eq" => Some(format!("dm.metadata_value_float = {}", num)),
                    "neq" => Some(format!("dm.metadata_value_float != {}", num)),
                    "gt" => Some(format!("dm.metadata_value_float > {}", num)),
                    "lt" => Some(format!("dm.metadata_value_float < {}", num)),
                    "gte" => Some(format!("dm.metadata_value_float >= {}", num)),
                    "lte" => Some(format!("dm.metadata_value_float <= {}", num)),
                    _ => None // Unsupported operator for floats
                }
            } else if let Some(string_value) = value.as_str() {
                // Try parsing the string as a float
                if let Ok(num) = string_value.parse::<f64>() {
                    match operator {
                        "eq" => Some(format!("dm.metadata_value_float = {}", num)),
                        "neq" => Some(format!("dm.metadata_value_float != {}", num)),
                        "gt" => Some(format!("dm.metadata_value_float > {}", num)),
                        "lt" => Some(format!("dm.metadata_value_float < {}", num)),
                        "gte" => Some(format!("dm.metadata_value_float >= {}", num)),
                        "lte" => Some(format!("dm.metadata_value_float <= {}", num)),
                        _ => None // Unsupported operator for floats
                    }
                } else {
                    println!("Failed to parse '{}' as NUMBER", string_value);
                    None // Couldn't parse as float
                }
            } else {
                None // Not a float value
            }
        },
        "DATE" => {
            // Date operations
            if let Some(string_value) = value.as_str() {
                // Validate the date format (YYYY-MM-DD)
                if string_value.matches('-').count() == 2 && string_value.len() >= 8 {
                    match operator {
                        "eq" => Some(format!("dm.metadata_value_date = '{}'::date", string_value)),
                        "neq" => Some(format!("dm.metadata_value_date != '{}'::date", string_value)),
                        "gt" => Some(format!("dm.metadata_value_date > '{}'::date", string_value)),
                        "lt" => Some(format!("dm.metadata_value_date < '{}'::date", string_value)),
                        "gte" => Some(format!("dm.metadata_value_date >= '{}'::date", string_value)),
                        "lte" => Some(format!("dm.metadata_value_date <= '{}'::date", string_value)),
                        _ => None // Unsupported operator for dates
                    }
                } else {
                    println!("Invalid date format: {}", string_value);
                    None // Invalid date format
                }
            } else {
                None // Not a string date
            }
        },
        "BOOLEAN" => {
            // Boolean operations
            if let Some(bool_val) = value.as_bool() {
                match operator {
                    "eq" => Some(format!("dm.metadata_value_boolean = {}", bool_val)),
                    "neq" => Some(format!("dm.metadata_value_boolean != {}", bool_val)),
                    _ => None // Unsupported operator for booleans
                }
            } else if let Some(string_value) = value.as_str() {
                // Try parsing the string as a boolean
                let lower_val = string_value.to_lowercase();
                if lower_val == "true" || lower_val == "yes" || lower_val == "1" {
                    match operator {
                        "eq" => Some(format!("dm.metadata_value_boolean = true")),
                        "neq" => Some(format!("dm.metadata_value_boolean != true")),
                        _ => None
                    }
                } else if lower_val == "false" || lower_val == "no" || lower_val == "0" {
                    match operator {
                        "eq" => Some(format!("dm.metadata_value_boolean = false")),
                        "neq" => Some(format!("dm.metadata_value_boolean != false")),
                        _ => None
                    }
                } else {
                    println!("Failed to parse '{}' as BOOLEAN", string_value);
                    None // Couldn't parse as boolean
                }
            } else {
                None // Not a boolean value
            }
        },
        _ => {
            println!("Unsupported metadata type: {}", metadata_type);
            None // Unknown metadata type
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