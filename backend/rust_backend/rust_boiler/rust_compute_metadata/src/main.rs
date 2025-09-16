use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use std::env;
use std::time::{Instant, Duration};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use jsonwebtokens_cognito::KeySet;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_secretsmanager::Client as SecretManagerClient;

use tokio_postgres::{Client, Error as PostgresError};
use postgres_native_tls::MakeTlsConnector;
use native_tls::TlsConnector;

// Request struct to deserialize incoming data
#[derive(Debug, Serialize, Deserialize)]
struct ComputeMetadataRequest {
    metadata_uuid: Option<String>,
    document_uuid: Option<String>,
    llm_provider: Option<String>,  // Add this field to select between "openai" and "ollama"
}

// Response structs for API
#[derive(Debug, Serialize, Deserialize)]
struct ComputeMetadataResponse {
    statusAPI: String,
    message: String,
    results: Vec<MetadataResult>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MetadataResult {
    document_uuid: String,
    metadata_uuid: String,
    metadata_name: String,
    raw_value: String,
    processed_value: Option<String>,
    confidence: f64,
    processing_time: f64,
}

// OllamaRequest struct for sending requests to Ollama server
#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
}

#[derive(Debug, Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

// OllamaResponse struct for parsing Ollama server responses
#[derive(Debug, Deserialize)]
struct OllamaResponse {
    model: String,
    message: OllamaResponseMessage,
}

#[derive(Debug, Deserialize)]
struct OllamaResponseMessage {
    role: String,
    content: String,
}

// OpenAI request struct for sending requests to OpenAI API
#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

// OpenAI response structs for parsing OpenAI API responses
#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    index: u32,
    message: OpenAIMessage,
    finish_reason: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

async fn show_secret(client: &SecretManagerClient, name: &str) -> Result<String, Error> {
    let resp = client.get_secret_value().secret_id(name).send().await?;
    match resp.secret_string() {
        Some(secret) => Ok(secret.into()),
        None => panic!("Error getting the secret: {:?}", name),
    }
}

// Extract email from Authorization token
async fn extract_email_from_token(event: &Request) -> Result<String, Error> {
    let key_to_check = "Authorization";
    if !event.headers().contains_key(key_to_check) {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "No Authorization header found")));
    }

    let bearer_str = event.headers()[key_to_check].to_str()?;
    if (!bearer_str.starts_with("Bearer ")) {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid Authorization format")));
    }

    let token_str = &bearer_str[7..];
    
    // Get AWS region and user pool ID from environment variables or use defaults
    let region = env::var("COGNITO_REGION").expect("COGNITO_REGION environment variable not set");
    let user_pool_id = env::var("COGNITO_USER_POOL_ID").expect("COGNITO_USER_POOL_ID environment variable not set");
    let client_id = env::var("COGNITO_CLIENT_ID").expect("COGNITO_CLIENT_ID environment variable not set");

    let keyset = KeySet::new(&region, &user_pool_id);
    
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

// Generate embedding using Ollama API
async fn get_ollama_embedding(text: &str, model: &str) -> Result<Vec<f64>, Error> {
    let ollama_api_url = env::var("OLLAMA_API_URL").expect("OLLAMA_API_URL environment variable not set");
    let client = reqwest::Client::new();
    
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
        .json::<Value>()
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

// Function to query documents similar to a question
async fn query_documents(
    question: &str,
    client: &Client,
    username: &str,
    n_results: i32,
    model: &str,
    document_uuid: Option<&str>,
    query_ledger_uuid: &str,
) -> Result<Vec<serde_json::Value>, Error> {
    println!("Getting embedding for question: {}", question);
    
    // For now, we will mock the embedding since we can't call Ollama directly
    let target_chunk_table = "document_embeding_mistral_generic"; // Fixed table for now
    
    println!("Building SQL query for document chunks...");
    
    // Base query with all fields
    let mut query = String::from(r#"
        SELECT 
            d.document_uuid as document_uuid,
            d.document_name as document_name,
            d.document_location as document_location,
            d.document_hash as document_hash,
            d.document_type as document_type,
            d.document_status as document_status,
            d.tags as document_tags,
            dc.document_chunk_uuid as document_chunk_uuid,
            dc.embebed_text as embebed_text,
            emb.document_embeding_uuid as document_embeding_uuid,
            emb.embeder_type as embeder_type,
            emb.embedding_token as embedding_token,
            emb.embedding_time as embedding_time
    "#);
    
    query += &format!(r#"
        FROM document_library."{}" emb
        INNER JOIN document_library.document_chunks dc ON dc.document_chunk_uuid = emb.document_chunk_uuid 
        INNER JOIN document_library.documents d ON d.document_uuid = dc.document_uuid
        INNER JOIN document_library.document_security_groups dsg on dsg.document_uuid = d.document_uuid
        INNER JOIN document_library.security_groups sg on sg.security_group_uuid = dsg.security_group_uuid
        INNER JOIN document_library.user_security_groups usg on usg.security_group_uuid = sg.security_group_uuid
        INNER JOIN document_library.users u on u.user_uuid = usg.user_uuid and u.sso_unique_id = $1
    "#, target_chunk_table);
    
    // Add document filter if UUID is provided
    if let Some(doc_uuid) = document_uuid {
        query += r#"
            WHERE d.document_uuid = $2
        "#;
    }
    
    // Add limit
    if let Some(_) = document_uuid {
        query += &format!("\nLIMIT $3");
    } else {
        query += &format!("\nLIMIT $2");
    }
    
    println!("Executing query for document chunks...");
    
    // Convert i32 to i64 for PostgreSQL bigint compatibility
    let n_results_i64: i64 = n_results as i64;
    
    let rows = if let Some(doc_uuid) = document_uuid {
        client.query(&query, &[&username, &doc_uuid, &n_results_i64]).await?
    } else {
        client.query(&query, &[&username, &n_results_i64]).await?
    };
    
    println!("Retrieved {} chunks", rows.len());
    
    // Convert rows to JSON objects
    let mut results: Vec<serde_json::Value> = Vec::new();
    for row in rows {
        let document_uuid: String = row.get("document_uuid");
        let document_chunk_uuid: String = row.get("document_chunk_uuid");
        let embebed_text: String = row.get("embebed_text");
        let document_name: String = row.get("document_name");
        
        // Create a JSON object for each row
        let result = json!({
            "document_uuid": document_uuid,
            "document_chunk_uuid": document_chunk_uuid,
            "embebed_text": embebed_text,
            "document_name": document_name,
            // Include other fields as needed
        });
        
        results.push(result);
        
        // Insert into query_answer_chunks table
        let insert_query = format!(r#"
            INSERT INTO document_library.query_answer_chunks
            (query_answer_chunk_uuid, query_answer_uuid, query_ledger_uuid, chunk_uuid, creation_date, created_by, updated_date, updated_by, "comments")
            VALUES ($1, 'to_be_managed_if_needed', $2, $3, now(), 'system', now(), 'system', 'chunk used for the answer')
        "#);
        
        let query_answer_chunk_uuid = Uuid::new_v4().to_string();
        match client.execute(&insert_query, &[&query_answer_chunk_uuid, &query_ledger_uuid, &document_chunk_uuid]).await {
            Ok(_) => println!("Inserted chunk record into query_answer_chunks"),
            Err(e) => println!("Error inserting into query_answer_chunks: {:?}", e),
        }
    }
    
    Ok(results)
}

// Function to retrieve metadata fields from the database
async fn get_metadata_fields(client: &Client, target_metadata_uuid: Option<&str>) -> Result<serde_json::Value, Error> {
    println!("Retrieving metadata fields...");
    
    let mut query = String::from(r#"
        SELECT 
            metadata_uuid,
            metadata_name,
            metadata_description,
            metadata_type
        FROM document_library.metadatas
    "#);
    
    if let Some(metadata_uuid) = target_metadata_uuid {
        query += " WHERE metadata_uuid = $1";
    }
    
    let rows = if let Some(metadata_uuid) = target_metadata_uuid {
        client.query(&query, &[&metadata_uuid]).await?
    } else {
        client.query(&query, &[]).await?
    };
    
    println!("Retrieved {} metadata field(s)", rows.len());
    
    // Convert rows to a map of metadata fields
    let mut metadata_fields = json!({});
    for row in rows {
        let metadata_uuid: String = row.get("metadata_uuid");
        let metadata_name: String = row.get("metadata_name");
        let metadata_description: String = row.get("metadata_description");
        let metadata_type: String = row.get("metadata_type");
        
        // Determine appropriate output format based on metadata type
        let output_format = match metadata_type.as_str() {
            "DATE" => "date in DD/MM/YYYY format",
            "NUMBER" => "number (no currency symbol)",
            "INTEGER" => "number as an integer",
            "BOOLEAN" => "'true' or 'false'",
            _ => "value"
        };
        
        // Map database metadata_type to the appropriate value_type
        let value_type = match metadata_type.to_uppercase().as_str() {
            "STRING" => "string",
            "NUMBER" => "float",
            "INTEGER" => "int",
            "DATE" => "date",
            "BOOLEAN" => "boolean",
            _ => "string"
        };
        
        // Create a prompt template based on the metadata description
        let prompt_template = format!(
            "You are an expert document analyzer. Extract the {} from this text. Return ONLY the {} or 'Not found' if it cannot be determined. Text: {{text}}",
            metadata_description,
            output_format
        );
        
        // Add this metadata field to the results
        let metadata_field = json!({
            "name": metadata_name,
            "prompt": prompt_template,
            "confidence_threshold": 0.7,
            "value_type": value_type
        });
        
        // Add this metadata field to the overall metadata_fields map
        metadata_fields[&metadata_uuid] = metadata_field;
    }
    
    Ok(metadata_fields)
}

// Function to extract metadata using Ollama or OpenAI
async fn extract_metadata_ollama(
    document_uuid: &str,
    client: &Client,
    username: &str,
    target_metadata_uuid: Option<&str>,
    openai_api_key: &str,
) -> Result<Vec<MetadataResult>, Error> {
    println!("Starting metadata extraction for document: {}", document_uuid);
    
    // Get metadata fields
    let metadata_fields = get_metadata_fields(&client, target_metadata_uuid.clone()).await?;
    
    let mut results: Vec<MetadataResult> = Vec::new();
    
    // Process each metadata field
    for (metadata_uuid, field_config) in metadata_fields.as_object().unwrap() {
        let query_ledger_uuid = Uuid::new_v4().to_string();
        let field_name = field_config["name"].as_str().unwrap();
        let prompt_template = field_config["prompt"].as_str().unwrap();
        
        println!("Processing metadata field: {} (UUID: {})", field_name, metadata_uuid);
        
        // Insert query ledger entry
        let query_ledger_sql = format!(r#"
            INSERT INTO document_library.query_ledgers_extended
            (query_ledger_uuid, query_type, query_content, metadata_uuid, user_uuid, query_tags, 
             query_start_document_date, query_end_document_date, query_answer, creation_date, 
             created_by, updated_date, updated_by, "comments")
            VALUES ($1, 'metadata_extraction', $2, $3, NULL, 'metadata_extraction', 
                   now(), now(), 'to_be_managed_if_needed', now(), 'system', now(), 'system', 'metadata extraction')
        "#);
        
        match client.execute(
            &query_ledger_sql, 
            &[&query_ledger_uuid, &prompt_template.replace("'", "''"), &metadata_uuid]
        ).await {
            Ok(_) => println!("Inserted query ledger record"),
            Err(e) => println!("Error inserting query ledger: {:?}", e),
        }
        
        let field_start_time = Instant::now();
        
        // Get document chunks for the document
        let chunks = query_documents(
            prompt_template, 
            &client, 
            username,
            3, // n_results_per_document
            "nomic-embed-text", // default model
            Some(document_uuid),
            &query_ledger_uuid
        ).await?;
        
        if chunks.is_empty() {
            println!("No chunks found for document UUID: {}", document_uuid);
            continue;
        }
        
        // Concatenate all chunks text
        let mut all_text = String::new();
        for chunk in chunks.iter() {
            all_text.push_str(&format!("\n\n---SECTION---\n\n{}", chunk["embebed_text"].as_str().unwrap_or("")));
        }
        
        println!("Retrieved {} chunks with total length: {} characters", chunks.len(), all_text.len());
        
        // Format the prompt with the document text
        let prompt = prompt_template.replace("{text}", &all_text);
        
        // Use the OpenAI API key from database credentials
        let openai_api_url = env::var("OPENAI_API_URL").expect("OPENAI_API_URL environment variable not set");
        let openai_model = env::var("OPENAI_MODEL").expect("OPENAI_MODEL environment variable not set");
        let model = env::var("OPENAI_MODEL").expect("OPENAI_MODEL environment variable not set");
        let client_http = reqwest::Client::new();

        // Create OpenAI request
        let openai_request = OpenAIRequest {
            model: openai_model.clone(),
            messages: vec![
                OpenAIMessage {
                    role: "user".to_string(),
                    content: prompt,
                }
            ],
            temperature: 0.0, // Use lower temperature for more deterministic responses
        };
        
        println!("Sending request to OpenAI API at: {}", openai_api_url);

        // Send the request to OpenAI
        let response = match client_http
            .post(&openai_api_url)
            .header("Authorization", format!("Bearer {}", openai_api_key))
            .header("Content-Type", "application/json")
            .json(&openai_request)
            .send()
            .await {
                Ok(resp) => resp,
                Err(e) => {
                    println!("Error sending request to OpenAI: {:?}", e);
                    return Err(Box::new(e));
                }
            };

        // Check if the response status is successful
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unable to read error response".to_string());
            println!("OpenAI API error: {} - {}", status, error_text);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("OpenAI API error: {}", status)
            )));
        }

        let openai_response: OpenAIResponse = match response.json().await {
            Ok(resp) => resp,
            Err(e) => {
                println!("Error parsing OpenAI response: {:?}", e);
                return Err(Box::new(e));
            }
        };

        // Extract content from the response
        let raw_value = if !openai_response.choices.is_empty() {
            openai_response.choices[0].message.content.trim().to_string()
        } else {
            "Not found".to_string()
        };

        // Use a higher confidence for OpenAI compared to Ollama
        let confidence = 0.95; // Default confidence for OpenAI

        /* 
        // Use Ollama to extract metadata
        let ollama_api_url = env::var("OLLAMA_API_URL").expect("OLLAMA_API_URL environment variable not set");
        let model = env::var("OLLAMA_MODEL").expect("OLLAMA_MODEL environment variable not set");
        let client_http = reqwest::Client::new();
        
        let ollama_request = OllamaRequest {
            model: model.clone(),
            messages: vec![
                OllamaMessage {
                    role: "system".to_string(),
                    content: prompt,
                }
            ],
        };

        println!("Sending request to Ollama server at: {}", ollama_api_url);
        
        
        // Send the request to Ollama
        let response = match client_http
            .post(&ollama_api_url)
            .json(&ollama_request)
            .send()
            .await {
                Ok(resp) => resp,
                Err(e) => {
                    println!("Error sending request to Ollama: {:?}", e);
                    return Err(Box::new(e));
                }
            };
        
        let ollama_response: OllamaResponse = match response.json().await {
            Ok(resp) => resp,
            Err(e) => {
                println!("Error parsing Ollama response: {:?}", e);
                return Err(Box::new(e));
            }
        };
        
        let raw_value = ollama_response.message.content.trim().to_string();
        let confidence = 0.85; // Default confidence for Ollama
        */
        
        // Log the raw LLM response and value type for debugging
        let value_type = field_config["value_type"].as_str().unwrap();
        println!("LLM Response for field '{}': '{}', Value Type: '{}'", field_name, raw_value, value_type);
        
        // Process the value based on the expected type with validation
        let processed_value = match value_type {
            "float" => {
                if raw_value.to_lowercase() == "not found" || raw_value == "-1" {
                    None
                } else {
                    // Extract numeric characters and decimal point
                    let numeric_str: String = raw_value.chars()
                        .filter(|c| c.is_digit(10) || *c == '.' || *c == '-')
                        .collect();
                    
                    match numeric_str.parse::<f64>() {
                        Ok(v) => Some(v.to_string()),
                        Err(e) => {
                            println!("Error parsing float value '{}': {}", raw_value, e);
                            None
                        }
                    }
                }
            },
            "int" => {
                if raw_value.to_lowercase() == "not found" || raw_value == "-1" {
                    None
                } else {
                    // Extract numeric characters and minus sign
                    let numeric_str: String = raw_value.chars()
                        .filter(|c| c.is_digit(10) || *c == '-')
                        .collect();
                    
                    match numeric_str.parse::<i32>() {
                        Ok(v) => Some(v.to_string()),
                        Err(e) => {
                            println!("Error parsing integer value '{}': {}", raw_value, e);
                            None
                        }
                    }
                }
            },
            "date" => {
                if raw_value.to_lowercase() == "not found" {
                    None
                } else {
                    // Try to parse the date as DD/MM/YYYY format
                    let date_parts: Vec<&str> = raw_value.split('/').collect();
                    if date_parts.len() == 3 {
                        // Basic validation - check if parts are numeric and within reasonable ranges
                        let day = date_parts[0].trim().parse::<u32>();
                        let month = date_parts[1].trim().parse::<u32>();
                        let year = date_parts[2].trim().parse::<i32>();
                        
                        if let (Ok(d), Ok(m), Ok(y)) = (day, month, year) {
                            // Simple range check
                            if d >= 1 && d <= 31 && m >= 1 && m <= 12 && y >= 1900 && y <= 2100 {
                                // Convert to chrono::NaiveDate object that can be directly used with PostgreSQL
                                match chrono::NaiveDate::from_ymd_opt(y, m, d) {
                                    Some(_) => Some(raw_value.clone()),
                                    None => {
                                        println!("Invalid date combination: {}/{}/{}", d, m, y);
                                        None
                                    }
                                }
                            } else {
                                println!("Invalid date range in '{}'. Day: {}, Month: {}, Year: {}", raw_value, d, m, y);
                                None
                            }
                        } else {
                            println!("Error parsing date components in '{}'", raw_value);
                            None
                        }
                    } else {
                        println!("Invalid date format in '{}'. Expected DD/MM/YYYY format.", raw_value);
                        None
                    }
                }
            },
            "boolean" => {
                // Convert to proper boolean
                let lowercase_val = raw_value.to_lowercase();
                if lowercase_val == "true" || lowercase_val == "yes" || lowercase_val == "1" {
                    Some("true".to_string())
                } else if lowercase_val == "false" || lowercase_val == "no" || lowercase_val == "0" {
                    Some("false".to_string())
                } else {
                    println!("Invalid boolean value '{}'. Expected true/false, yes/no, or 1/0.", raw_value);
                    Some("false".to_string()) // Default to false for invalid values
                }
            },
            _ => { // string
                if raw_value.to_lowercase() == "not found" {
                    None
                } else {
                    // Verify this is valid UTF-8
                    match std::str::from_utf8(raw_value.as_bytes()) {
                        Ok(_) => Some(raw_value.clone()),
                        Err(e) => {
                            println!("Invalid UTF-8 string '{}': {}", raw_value, e);
                            None
                        }
                    }
                }
            }
        };
        
        let processing_time = field_start_time.elapsed().as_secs_f64();
        
        // Check if metadata already exists for this document and field
        let check_query = format!(r#"
            SELECT document_metadata_uuid 
            FROM document_library.document_metadatas 
            WHERE document_uuid = $1 
            AND metadata_uuid = $2
        "#);
        
        let existing_metadata_rows = client.query(&check_query, &[&document_uuid, &metadata_uuid]).await?;
        
        if let Some(processed_val) = &processed_value {
            // For boolean values, we need to convert the string to an actual boolean for PostgreSQL
            if value_type == "boolean" {
                // Convert string boolean representation to actual boolean for PostgreSQL
                let bool_value = processed_val.to_lowercase() == "true";
                
                let comments = format!("Extracted by model: {}. Confidence: {:.2}", model, confidence);
                
                if !existing_metadata_rows.is_empty() {
                    // Update existing record
                    let document_metadata_uuid: String = existing_metadata_rows[0].get("document_metadata_uuid");
                    
                    let update_query = format!(r#"
                        UPDATE document_library.document_metadatas 
                        SET metadata_value_boolean = $1, updated_date = now(), updated_by = $2, query_ledger_uuid = $3, comments = $4
                        WHERE document_metadata_uuid = $5
                    "#);
                    
                    match client.execute(
                        &update_query, 
                        &[&bool_value, &"SYSTEM", &query_ledger_uuid, &comments, &document_metadata_uuid]
                    ).await {
                        Ok(_) => println!("Updated boolean metadata value for field {}: {}", field_name, bool_value),
                        Err(e) => {
                            println!("Error updating boolean metadata: {:?}", e);
                            // We'll continue execution but log the error - this prevents the function from crashing
                        }
                    }
                } else {
                    // Insert new record
                    let document_metadata_uuid = Uuid::new_v4().to_string();
                    
                    let insert_query = format!(r#"
                        INSERT INTO document_library.document_metadatas 
                        (document_metadata_uuid, document_uuid, metadata_uuid, metadata_value_boolean, 
                         creation_date, created_by, updated_date, updated_by, 
                         query_ledger_uuid, comments)
                        VALUES ($1, $2, $3, $4, now(), $5, now(), $6, $7, $8)
                    "#);
                    
                    match client.execute(
                        &insert_query, 
                        &[&document_metadata_uuid, &document_uuid, &metadata_uuid, &bool_value, 
                          &"SYSTEM", &"SYSTEM", &query_ledger_uuid, &comments]
                    ).await {
                        Ok(_) => println!("Inserted new boolean metadata value for field {}: {}", field_name, bool_value),
                        Err(e) => {
                            println!("Error inserting boolean metadata: {:?}", e);
                            // We'll continue execution but log the error - this prevents the function from crashing
                        }
                    }
                }
            } else {
                // Handle non-boolean types
                let column_name = match value_type {
                    "float" => "metadata_value_float",
                    "int" => "metadata_value_int",
                    "date" => "metadata_value_date",
                    _ => "metadata_value_string",
                };
                
                let comments = format!("Extracted by model: {}. Confidence: {:.2}", model, confidence);
            
                if !existing_metadata_rows.is_empty() {
                    // Update existing record
                    let document_metadata_uuid: String = existing_metadata_rows[0].get("document_metadata_uuid");
                    
                    let update_query = format!(r#"
                        UPDATE document_library.document_metadatas 
                        SET {} = $1, updated_date = now(), updated_by = $2, query_ledger_uuid = $3, comments = $4
                        WHERE document_metadata_uuid = $5
                    "#, column_name);
                    
                    // Execute update with properly typed parameters
                    let update_result = match value_type {
                        "float" => {
                            // Convert string to float
                            match processed_val.parse::<f64>() {
                                Ok(float_val) => {
                                    client.execute(
                                        &update_query,
                                        &[&float_val, &"SYSTEM", &query_ledger_uuid, &comments, &document_metadata_uuid]
                                    ).await
                                },
                                Err(e) => {
                                    println!("Error parsing float for DB update: {} - {}", processed_val, e);
                                    continue;
                                }
                            }
                        },
                        "int" => {
                            // Convert string to integer
                            match processed_val.parse::<i32>() {
                                Ok(int_val) => {
                                    client.execute(
                                        &update_query,
                                        &[&int_val, &"SYSTEM", &query_ledger_uuid, &comments, &document_metadata_uuid]
                                    ).await
                                },
                                Err(e) => {
                                    println!("Error parsing integer for DB update: {} - {}", processed_val, e);
                                    continue;
                                }
                            }
                        },
                        "date" => {
                            // Parse date string from DD/MM/YYYY format to chrono::NaiveDate
                            let date_parts: Vec<&str> = processed_val.split('/').collect();
                            if date_parts.len() == 3 {
                                if let (Ok(day), Ok(month), Ok(year)) = (
                                    date_parts[0].trim().parse::<u32>(),
                                    date_parts[1].trim().parse::<u32>(),
                                    date_parts[2].trim().parse::<i32>()
                                ) {
                                    // Create the NaiveDate object from the parsed components
                                    if let Some(date_obj) = chrono::NaiveDate::from_ymd_opt(year, month, day) {
                                        client.execute(
                                            &update_query,
                                            &[&date_obj, &"SYSTEM", &query_ledger_uuid, &comments, &document_metadata_uuid]
                                        ).await
                                    } else {
                                        println!("Error creating date object from {}/{}/{}", day, month, year);
                                        continue;
                                    }
                                } else {
                                    println!("Error parsing date components in '{}'", processed_val);
                                    continue;
                                }
                            } else {
                                println!("Invalid date format in '{}'. Expected DD/MM/YYYY format.", processed_val);
                                continue;
                            }
                        },
                        _ => {
                            // For string, use as is
                            client.execute(
                                &update_query,
                                &[&processed_val, &"SYSTEM", &query_ledger_uuid, &comments, &document_metadata_uuid]
                            ).await
                        }
                    };
                    
                    match update_result {
                        Ok(_) => println!("Updated metadata value for field {}: {}", field_name, processed_val),
                        Err(e) => {
                            println!("Error updating metadata: {:?}", e);
                            // We'll continue execution but log the error - this prevents the function from crashing
                        }
                    }
                } else {
                    // Insert new record
                    let document_metadata_uuid = Uuid::new_v4().to_string();
                    
                    let insert_query = format!(r#"
                        INSERT INTO document_library.document_metadatas 
                        (document_metadata_uuid, document_uuid, metadata_uuid, {}, 
                         creation_date, created_by, updated_date, updated_by, 
                         query_ledger_uuid, comments)
                        VALUES ($1, $2, $3, $4, now(), $5, now(), $6, $7, $8)
                    "#, column_name);
                    
                    // Execute insert with properly typed parameters
                    let insert_result = match value_type {
                        "float" => {
                            // Convert string to float
                            match processed_val.parse::<f64>() {
                                Ok(float_val) => {
                                    client.execute(
                                        &insert_query,
                                        &[&document_metadata_uuid, &document_uuid, &metadata_uuid, &float_val,
                                          &"SYSTEM", &"SYSTEM", &query_ledger_uuid, &comments]
                                    ).await
                                },
                                Err(e) => {
                                    println!("Error parsing float for DB insert: {} - {}", processed_val, e);
                                    continue;
                                }
                            }
                        },
                        "int" => {
                            // Convert string to integer
                            match processed_val.parse::<i32>() {
                                Ok(int_val) => {
                                    client.execute(
                                        &insert_query,
                                        &[&document_metadata_uuid, &document_uuid, &metadata_uuid, &int_val,
                                          &"SYSTEM", &"SYSTEM", &query_ledger_uuid, &comments]
                                    ).await
                                },
                                Err(e) => {
                                    println!("Error parsing integer for DB insert: {} - {}", processed_val, e);
                                    continue;
                                }
                            }
                        },
                        "date" => {
                            // Parse date string from DD/MM/YYYY format to chrono::NaiveDate
                            let date_parts: Vec<&str> = processed_val.split('/').collect();
                            if date_parts.len() == 3 {
                                if let (Ok(day), Ok(month), Ok(year)) = (
                                    date_parts[0].trim().parse::<u32>(),
                                    date_parts[1].trim().parse::<u32>(),
                                    date_parts[2].trim().parse::<i32>()
                                ) {
                                    // Create the NaiveDate object from the parsed components
                                    if let Some(date_obj) = chrono::NaiveDate::from_ymd_opt(year, month, day) {
                                        client.execute(
                                            &insert_query,
                                            &[&document_metadata_uuid, &document_uuid, &metadata_uuid, &date_obj,
                                              &"SYSTEM", &"SYSTEM", &query_ledger_uuid, &comments]
                                        ).await
                                    } else {
                                        println!("Error creating date object from {}/{}/{}", day, month, year);
                                        continue;
                                    }
                                } else {
                                    println!("Error parsing date components in '{}'", processed_val);
                                    continue;
                                }
                            } else {
                                println!("Invalid date format in '{}'. Expected DD/MM/YYYY format.", processed_val);
                                continue;
                            }
                        },
                        _ => {
                            // For string, use as is
                            client.execute(
                                &insert_query,
                                &[&document_metadata_uuid, &document_uuid, &metadata_uuid, &processed_val,
                                  &"SYSTEM", &"SYSTEM", &query_ledger_uuid, &comments]
                            ).await
                        }
                    };
                    
                    match insert_result {
                        Ok(_) => println!("Inserted new metadata value for field {}: {}", field_name, processed_val),
                        Err(e) => {
                            println!("Error inserting metadata: {:?}", e);
                            // We'll continue execution but log the error - this prevents the function from crashing
                        }
                    }
                }
            }
        } else {
            // If processed_value is None, we had a validation failure
            println!("Validation failed for metadata field '{}' with value '{}'. No update performed.", field_name, raw_value);
        }
        
        // Record the result
        results.push(MetadataResult {
            document_uuid: document_uuid.to_string(),
            metadata_uuid: metadata_uuid.to_string(),
            metadata_name: field_name.to_string(),
            raw_value: raw_value.clone(),
            processed_value: processed_value,
            confidence,
            processing_time,
        });
    }
    
    Ok(results)
}

// Main function handler
async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    println!("Request received: {:?}", event);
    
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

    // Parse request body
    let body = event.body();
    let request_data: ComputeMetadataRequest = match serde_json::from_slice(body.as_ref()) {
        Ok(data) => data,
        Err(e) => {
            println!("Failed to parse request body: {}", e);
            return Ok(Response::builder()
                .status(400)
                .header("content-type", "application/json")
                .body(json!({"statusAPI": "ERROR", "message": "Invalid request format"}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    // Determine which LLM provider to use
    let llm_provider = request_data.llm_provider.as_deref().unwrap_or("ollama");
    if llm_provider != "ollama" {
        return Ok(Response::builder()
            .status(400)
            .header("content-type", "application/json")
            .body(json!({"statusAPI": "ERROR", "message": "Unsupported LLM provider. Only 'ollama' is supported."}).to_string().into())
            .map_err(Box::new)?);
    }
    println!("Using LLM provider: {}", llm_provider);
    
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

    // Extract OpenAI API key from secrets
    let openai_api_key = db_credentials["OPENAI_API_KEY"].as_str()
        .ok_or_else(|| Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "OpenAI API key not found in database credentials")))?;

    let db_server = db_credentials["DB_HOST"].as_str().unwrap();
    let database = db_credentials["DB_NAME"].as_str().unwrap();
    let db_username = db_credentials["DB_USER"].as_str().unwrap();
    let db_password = db_credentials["DB_PASSWORD"].as_str().unwrap();
    let db_port = db_credentials["DB_PORT"].as_str().unwrap();

    let tls_connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true) // Disable certificate validation for development
        .build();
    let tls = MakeTlsConnector::new(tls_connector.expect("Failed to build TLS connector"));

    // Connect to the database
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

    // Process the request based on parameters
    let start_time = Instant::now();
    let mut all_results: Vec<MetadataResult> = Vec::new();
    
    if let Some(document_uuid) = &request_data.document_uuid {
        // Process a single document
        println!("Processing document with UUID: {}", document_uuid);
        
        let results = extract_metadata_ollama(
            document_uuid,
            &client,
            &user_email,
            request_data.metadata_uuid.as_deref(),
            openai_api_key
        ).await;
        
        match results {
            Ok(res) => all_results.extend(res),
            Err(e) => {
                eprintln!("Error extracting metadata: {}", e);
                return Ok(Response::builder()
                    .status(500)
                    .header("content-type", "application/json")
                    .body(json!({"statusAPI": "ERROR", "message": format!("Metadata extraction failed: {}", e)}).to_string().into())
                    .map_err(Box::new)?);
            }
        };
    } else {
        // Process all documents
        println!("Processing all documents");
        
        // Get all document UUIDs
        let documents_query = "SELECT document_uuid, document_name FROM document_library.documents ORDER BY document_name";
        let document_rows = match client.query(documents_query, &[]).await {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!("Error querying documents: {}", e);
                return Ok(Response::builder()
                    .status(500)
                    .header("content-type", "application/json")
                    .body(json!({"statusAPI": "ERROR", "message": "Failed to retrieve documents"}).to_string().into())
                    .map_err(Box::new)?);
            }
        };
        
        println!("Found {} documents to process", document_rows.len());
        
        // Process each document
        for row in document_rows {
            let document_uuid: String = row.get("document_uuid");
            let document_name: String = row.get("document_name");
            
            println!("Processing document: {} ({})", document_name, document_uuid);
            
            let results = extract_metadata_ollama(
                &document_uuid,
                &client,
                &user_email,
                request_data.metadata_uuid.as_deref(),
                openai_api_key
            ).await;
            
            match results {
                Ok(res) => all_results.extend(res),
                Err(e) => {
                    eprintln!("Error extracting metadata for document {}: {}", document_uuid, e);
                    // Continue with the next document
                    continue;
                }
            };
        }
    }
    
    let total_time = start_time.elapsed();
    println!("Total processing time: {:.2} seconds", total_time.as_secs_f64());
    
    // Create the response
    let response_body = ComputeMetadataResponse {
        statusAPI: "OK".to_string(),
        message: format!("Successfully processed {} metadata entries using {} provider", all_results.len(), llm_provider),
        results: all_results,
    };
    
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