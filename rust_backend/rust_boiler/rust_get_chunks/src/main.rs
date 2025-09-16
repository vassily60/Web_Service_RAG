use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::time::Instant;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use jsonwebtokens_cognito::KeySet;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_secretsmanager::Client as SecretManagerClient;
use tokio_postgres::{Client, Error as PgError};
use postgres_native_tls::MakeTlsConnector;
use native_tls::TlsConnector;
use reqwest::Client as ReqwestClient;

#[derive(Deserialize)]
struct DocumentFilter {
    filter_type: String,
    filter_value: String,
}

#[derive(Deserialize)]
struct GetChunksRequest {
    document_filters: Option<Vec<DocumentFilter>>,
    question: String,
    num_results: Option<i64>,
    start_date: Option<String>,
    end_date: Option<String>,
    tags: Option<Vec<String>>,
    document_uuid: Option<String>, // Added optional document_uuid parameter
}

#[derive(Debug, Serialize, Clone)]
struct DocumentMetadata {
    metadata_uuid: String,
    metadata_name: String,
    metadata_type: String,  // Added field to store the type of metadata (STRING, INTEGER, NUMBER, BOOLEAN, DATE)
    metadata_value_string: Option<String>,
    metadata_value_int: Option<i32>,  // Changed from i64 to i32 to match PostgreSQL int4
    metadata_value_float: Option<f64>,
    metadata_value_boolean: Option<bool>,
    metadata_value_date: Option<chrono::NaiveDate>,  // Changed from String to NaiveDate to handle PostgreSQL date type
}

#[derive(Debug, Serialize, Clone)]
struct Chunk {
    document_uuid: String,
    document_name: String,
    document_location: String,
    document_hash: String,
    document_type: String,
    document_status: String,
    document_chunk_uuid: String,
    embebed_text: String,
    document_embeding_uuid: String,
    embeder_type: String,
    embedding_token: i32,
    embedding_time: f64,
    document_metadata: Vec<DocumentMetadata>,
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    input: String,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    data: Vec<Embedding>,
}

#[derive(Deserialize)]
struct Embedding {
    embedding: Vec<f64>,
}

// Extract email from Authorization token
async fn extract_email_from_token(event: &Request) -> Result<String, Error> {
    // Get authorization header
    let auth_header = match event.headers().get("Authorization") {
        Some(header) => header,
        None => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "No Authorization header"))),
    };

    // Get the bearer token
    let bearer_str = auth_header.to_str()?;
    if !bearer_str.starts_with("Bearer ") {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Invalid Authorization format")));
    }

    let token_str = &bearer_str[7..];

    // Get Cognito configuration from environment
    let cognito_region = env::var("COGNITO_REGION").expect("COGNITO_REGION environment variable not set");
    let user_pool_id = env::var("USER_POOL_ID").expect("USER_POOL_ID environment variable not set");
    let client_id = env::var("CLIENT_ID").expect("CLIENT_ID environment variable not set");

    // Verify the token
    match KeySet::new(&cognito_region, &user_pool_id) {
        Ok(key_set) => {
            let verifier = key_set.new_id_token_verifier(&[&client_id]).build()?;
            let verification_result = key_set.verify(token_str, &verifier).await;

            match verification_result {
                Ok(claims) => {
                    // Extract email claim
                    if let Some(email) = claims["email"].as_str() {
                        println!("Verified user email: {}", email);
                        Ok(email.to_string())
                    } else {
                        println!("No email found in token claims.");
                        Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Email not found in token")))
                    }
                },
                Err(e) => {
                    println!("Token verification error: {}", e);
                    Err(Box::new(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Invalid token")))
                },
            }
        },
        Err(e) => {
            println!("Error creating key set: {}", e);
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("KeySet error: {}", e))))
        },
    }
}

// Function to get user_uuid from email
async fn get_user_uuid_from_email(client: &Client, email: &str) -> Result<String, Error> {
    let query = "SELECT user_uuid FROM document_library.users WHERE sso_unique_id = $1";
    println!("PRINT ICI");
    let rows = client.query(query, &[&email]).await?;
    println!("PRINT la");
    if rows.is_empty() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("User with email {} not found", email),
        )));
    }
    
    let user_uuid: String = rows[0].get(0);
    Ok(user_uuid)
}

async fn show_secret(client: &SecretManagerClient, name: &str) -> Result<String, Error> {
    let resp = client.get_secret_value().secret_id(name).send().await?;
    match resp.secret_string() {
        Some(secret) => Ok(secret.into()),
        None => panic!("Error to get the secret: {:?}", name),
    }
}

// Function to get embedding from Ollama
async fn get_ollama_embedding(text: &str, model: &str) -> Result<Vec<f64>, Error> {
    let ollama_api_url = env::var("OLLAMA_API_URL").expect("OLLAMA_API_URL environment variable not set");
    let client = ReqwestClient::new();
    
    // Prepare the request body
    let body = json!({
        "model": model,
        "prompt": text
    });
    
    println!("Sending embedding request to Ollama server at: {}", ollama_api_url);
    
    // Send the request to the Ollama API
    let response = client
        .post(&ollama_api_url)
        .json(&body)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    
    // Extract the embedding from the response
    if let Some(embedding) = response["embedding"].as_array() {
        let embedding_vec: Vec<f64> = embedding
            .iter()
            .filter_map(|v| v.as_f64())
            .collect();
        
        Ok(embedding_vec)
    } else {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to parse embedding from response: {:?}", response),
        )))
    }
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

// Function to get embedding from OpenAI
async fn get_embedding(api_key: &str, model: &str, input: &str) -> Result<Vec<f64>, reqwest::Error> {
    let client = ReqwestClient::new();
    let request = OpenAIRequest {
        model: model.to_string(),
        input: input.to_string(),
    };

    let response = client
        .post("https://api.openai.com/v1/embeddings")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await?
        .json::<OpenAIResponse>()
        .await?;

    Ok(response.data[0].embedding.clone())
}

// Function to query documents similar to a question - adapted from rust_compute_metadata
async fn query_documents(
    question: &str,
    client: &Client,
    embedding_vector: &str,
    n_results: i64,
    where_clause: &str,
    user_uuid: &str,
    document_uuid: &Option<String>,
) -> Result<Vec<Chunk>, Error> {
    println!("Executing semantic search for question: {}", question);
    println!("User UUID: {}", user_uuid);
    
    // Get target embedding table from environment or use default
    let target_chunk_table = env::var("EMBEDDING_TABLE").expect("EMBEDDING_TABLE environment variable not set");
    
    // Add document_uuid filter if provided
    let document_filter = if let Some(doc_uuid) = document_uuid {
        println!("Filtering by document UUID: {}", doc_uuid);
        format!("AND d.document_uuid = '{}'", doc_uuid)
    } else {
        String::new()
    };
    
    // First check if user exists in the security system
    let sec_check_query = format!(
        "SELECT COUNT(*) FROM document_library.user_security_groups 
         WHERE user_uuid = '{}'", user_uuid);
    
    let sec_check_result = client.query(&sec_check_query, &[]).await?;
    let user_group_count: i64 = sec_check_result[0].get(0);
    
    println!("User belongs to {} security groups", user_group_count);
    
    // Modify the query to use LEFT JOIN instead of INNER JOIN for security filtering
    // This way if the security filtering is the issue, we'll get records but can see which ones
    let query = format!(
        "SELECT 
        d.document_uuid as document_uuid,
        d.document_name as document_name,
        d.document_location as document_location,
        d.document_hash as document_hash,
        d.document_type as document_type,
        d.document_status as document_status,
        dc.document_chunk_uuid as document_chunk_uuid,
        dc.embebed_text as embebed_text,
        emb.document_embeding_uuid as document_embeding_uuid,
        emb.embeder_type as embeder_type,
        emb.embedding_token as embedding_token,
        emb.embedding_time as embedding_time
        FROM document_library.\"{}\" emb
        INNER JOIN document_library.document_chunks dc ON dc.document_chunk_uuid = emb.document_chunk_uuid 
        INNER JOIN document_library.documents d ON d.document_uuid = dc.document_uuid
        INNER JOIN document_library.document_security_groups dsg ON dsg.document_uuid = d.document_uuid
        INNER JOIN document_library.security_groups sg ON sg.security_group_uuid = dsg.security_group_uuid
        INNER JOIN document_library.user_security_groups usg ON usg.security_group_uuid = sg.security_group_uuid
        INNER JOIN document_library.users u ON u.user_uuid = usg.user_uuid AND u.user_uuid = '{}'
        {}
        {}
        -- Temporarily remove security filtering to debug
        -- WHERE (u.user_uuid IS NOT NULL OR d.is_public = true)
        ORDER BY embedding <-> {}::vector
        LIMIT {};",
        target_chunk_table, user_uuid, where_clause, document_filter, embedding_vector, n_results
    );
    
    println!("Debug Query: {}", query);

    let rows = client.query(&query, &[]).await?;
    println!("Query returned {} rows", rows.len());
    
    // Parse rows into Chunk structs
    let mut chunks: Vec<Chunk> = Vec::new();
    let mut document_uuids: Vec<String> = Vec::new();
    
    for row in rows {
        let doc_uuid: String = row.get("document_uuid");
        
        // Track unique document UUIDs for metadata query
        if !document_uuids.contains(&doc_uuid) {
            document_uuids.push(doc_uuid.clone());
        }
        
        let chunk = Chunk {
            document_uuid: doc_uuid,
            document_name: row.get("document_name"),
            document_location: row.get("document_location"),
            document_hash: row.get("document_hash"),
            document_type: row.get("document_type"),
            document_status: row.get("document_status"),
            document_chunk_uuid: row.get("document_chunk_uuid"),
            embebed_text: row.get("embebed_text"),
            document_embeding_uuid: row.get("document_embeding_uuid"),
            embeder_type: row.get("embeder_type"),
            embedding_token: row.get("embedding_token"),
            embedding_time: row.get("embedding_time"),
            document_metadata: Vec::new(), // Initialize with empty vector, will populate later
        };
        chunks.push(chunk);
    }
    
    // Fetch metadata for all document UUIDs in a single query
    if !document_uuids.is_empty() {
        let doc_uuids_list = document_uuids
            .iter()
            .map(|uuid| format!("'{}'", uuid))
            .collect::<Vec<_>>()
            .join(",");
            
        let metadata_query = format!(
            "SELECT 
                dm.document_uuid,
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
             WHERE dm.document_uuid IN ({})",
            doc_uuids_list
        );
        
        println!("Fetching metadata with query: {}", metadata_query);
        
        let metadata_rows = match client.query(&metadata_query, &[]).await {
            Ok(rows) => {
                println!("Found {} metadata entries", rows.len());
                rows
            },
            Err(e) => {
                println!("Error fetching metadata: {}", e);
                Vec::new()
            }
        };
        
        // Create a map of document_uuid -> Vec<DocumentMetadata>
        let mut metadata_map: std::collections::HashMap<String, Vec<DocumentMetadata>> = std::collections::HashMap::new();
        
        for row in metadata_rows {
            let doc_uuid: String = row.get("document_uuid");
            
            let metadata = DocumentMetadata {
                metadata_uuid: row.get("metadata_uuid"),
                metadata_name: row.get("metadata_name"),
                metadata_type: row.get("metadata_type"),  // Added to get the type from the query
                metadata_value_string: row.get("metadata_value_string"),
                metadata_value_int: row.get("metadata_value_int"),
                metadata_value_float: row.get("metadata_value_float"),
                metadata_value_boolean: row.get("metadata_value_boolean"),
                metadata_value_date: row.get("metadata_value_date"),
            };
            
            metadata_map.entry(doc_uuid).or_insert_with(Vec::new).push(metadata);
        }
        
        // Add metadata to each chunk
        for chunk in &mut chunks {
            if let Some(metadata_list) = metadata_map.get(&chunk.document_uuid) {
                chunk.document_metadata = metadata_list.clone();
            }
        }
    }
    
    Ok(chunks)
}

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    let req: GetChunksRequest = serde_json::from_slice(event.body().as_ref()).unwrap();

    // Extract user email from token
    let user_email = match extract_email_from_token(&event).await {
        Ok(email) => {
            println!("Successfully extracted user email: {}", email);
            email
        },
        Err(e) => {
            println!("Failed to extract email from token: {}", e);
            return Ok(Response::builder()
                .status(401)
                .header("content-type", "application/json")
                .body(json!({"statusAPI": "ERROR", "message": "Unauthorized: Invalid token"}).to_string().into())
                .map_err(Box::new)?);
        }
    };

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

    println!("END OF TLS");

    // Connect to the database with better error handling
    let connection_string = format!("host={} port={} user={} password={} dbname={}", 
        db_server, db_port, db_username, db_password, database);
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
                .header("content-type", "text/plain")
                .body("Internal server error: Database connection failed".into())
                .map_err(Box::new)?);
        }
    };

    // Spawn a new task to manage the connection
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    println!("END OF DATABASE CONNECT");
    
    println!("START USER_UUID LOOKUP");
    
    // Check if users table exists
    let check_users_table = client
        .query("SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'document_library' AND table_name = 'users')", &[])
        .await?;
    
    let users_table_exists: bool = check_users_table[0].get(0);
    println!("Users table exists: {}", users_table_exists);
    
    // For now, directly use email as user identifier until we can fix the schema
    // If users table exists, try to look up UUID, otherwise fall back to email
    let user_identifier = if users_table_exists {
        match get_user_uuid_from_email(&client, &user_email).await {
            Ok(uuid) => {
                println!("Found user_uuid: {}", uuid);
                uuid
            },
            Err(e) => {
                println!("User UUID lookup failed: {}. Using email as identifier instead.", e);
                user_email.clone()
            }
        }
    } else {
        println!("Users table not found. Using email as identifier.");
        user_email.clone()
    };
    
    println!("END USER_UUID LOOKUP");
    
    // Create a timer to measure performance
    let start_time = Instant::now();

    // Use Ollama to get embedding instead of OpenAI
    let model = env::var("OLLAMA_MODEL").expect("OLLAMA_MODEL environment variable not set");
    let input = &req.question;

    let embedding = match get_ollama_embedding(input, &model).await {
        Ok(emb) => emb,
        Err(e) => {
            eprintln!("Error generating embedding: {}", e);
            return Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({"error": "Failed to generate embedding"}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    println!("Got Ollama embedding with {} dimensions", embedding.len());

    let embedding_vector = format!("ARRAY[{}]", embedding.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "));
    let num_results = req.num_results.unwrap_or(20);

    // Build WHERE clause for the filters
    let mut where_clauses = Vec::new();
    
    // Add date range filters if they exist
    if let Some(start_date) = &req.start_date {
        if !start_date.is_empty() {
            where_clauses.push(format!("d.creation_date >= '{}'::timestamp", start_date));
        }
    }
    
    if let Some(end_date) = &req.end_date {
        if !end_date.is_empty() {
            where_clauses.push(format!("d.creation_date <= '{}'::timestamp", end_date));
        }
    }
    
    // Add tags filter if it exists
    if let Some(tags) = &req.tags {
        if !tags.is_empty() {
            let tags_str = tags
                .iter()
                .map(|tag| format!("'{}'", tag.replace("'", "''")))
                .collect::<Vec<_>>()
                .join(", ");
            where_clauses.push(format!("d.tags @> ARRAY[{}]", tags_str));
        }
    }
    
    // Process document filters (including metadata filters)
    if let Some(filters) = &req.document_filters {
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
    
    // Construct the WHERE clause part of the query
    let where_clause = if !where_clauses.is_empty() {
        format!("WHERE {}", where_clauses.join(" AND "))
    } else {
        String::new()
    };

    // Use the query_documents function to get semantically similar chunks
    let chunks = query_documents(
        &req.question, 
        &client, 
        &embedding_vector, 
        num_results, 
        &where_clause, 
        &user_identifier,  // Use the identifier which may be UUID or email
        &req.document_uuid
    ).await?;
    
    let elapsed_time = start_time.elapsed();
    println!("Query completed in {:.2?} with {} chunks found", elapsed_time, chunks.len());

    let response_body = json!({ "chunks": chunks });

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