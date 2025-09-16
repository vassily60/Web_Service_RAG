use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use std::env;
use chrono::Utc;

use aws_sdk_secretsmanager::Client as SecretManagerClient;
use aws_sdk_secretsmanager::config::Region;

use tokio_postgres::Client;
use postgres_native_tls::MakeTlsConnector;
use native_tls::TlsConnector;
use jsonwebtokens_cognito::KeySet;

// Input struct for the synonym update data
#[derive(Debug, Serialize, Deserialize)]
struct SynonymUpdateInput {
    synonym_uuid: String, // UUID of the synonym to update
    synonym_name: Option<String>,
    synonym_value: Option<String>,
    comments: Option<String>,
}

// Output struct for the response
#[derive(Debug, Serialize, Deserialize)]
struct SynonymOutput {
    synonym_uuid: String,
    synonym_name: String,
    synonym_value: String,
    updated_by: String,
}

async fn show_secret(client: &SecretManagerClient, name: &str) -> Result<String, Error> {
    let resp = client.get_secret_value().secret_id(name).send().await?;
    match resp.secret_string() {
        Some(secret) => Ok(secret.into()),
        None => panic!("Error to get the secret: {:?}", name),
    }
}

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Get region from environment variable
    let region_name = env::var("REGION").expect("REGION environment variable not set");
    
    // Initialize AWS SDK configuration
    let region = Region::new(region_name.clone());
    let config = aws_config::from_env().region(region).load().await;
    let client_secret = SecretManagerClient::new(&config);
    println!("AWS SDK initialized!");

    // Get Cognito configuration from AWS Secrets Manager
    let cognito_secret_name = env::var("COGNITO_SECRET").expect("COGNITO_SECRET environment variable not set");
    let secret_content = show_secret(&client_secret, &cognito_secret_name).await?;
    let cognito_credentials: Value = serde_json::from_str(&secret_content)?;
    
    // Extract user_pool_id and client_id from the secret
    let user_pool_id = cognito_credentials["USER_POOL_ID"].as_str()
        .ok_or("USER_POOL_ID not found in secret")?;
    let client_id = cognito_credentials["APP_CLIENT_ID"].as_str()
        .ok_or("APP_CLIENT_ID not found in secret")?;
    let cognito_region_name = cognito_credentials["REGION"].as_str()
        .ok_or("REGION not found in secret")?;
    
    println!("Retrieved Cognito configuration from Secrets Manager!");
    
    // Extract token from Authorization header
    let my_token_strslice = match event.headers().get("Authorization") {
        Some(auth_header) => {
            let auth_header_str = auth_header.to_str()?;
            if auth_header_str.len() > 7 {
                &auth_header_str[7..] // Skip "Bearer "
            } else {
                return Ok(Response::builder()
                    .status(401)
                    .header("content-type", "application/json")
                    .body(json!({"message": "Invalid authorization header format"}).to_string().into())
                    .map_err(Box::new)?);
            }
        },
        None => {
            return Ok(Response::builder()
                .status(401)
                .header("content-type", "application/json")
                .body(json!({"message": "Authorization header not provided"}).to_string().into())
                .map_err(Box::new)?);
        }
    };

    // Create a KeySet for AWS Cognito with proper error handling
    let key_set = match KeySet::new(cognito_region_name.clone(), user_pool_id.to_string()) {
        Ok(ks) => ks,
        Err(e) => {
            return Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({"message": format!("Failed to create KeySet: {}", e)}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    // Prefetch the JWKs from Cognito - this is necessary before verification
    match key_set.prefetch_jwks().await {
        Ok(_) => println!("Successfully fetched JWKs from Cognito"),
        Err(e) => {
            return Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({"message": format!("Failed to fetch JWKs: {}", e)}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    let verifier = match key_set.new_id_token_verifier(&[client_id]).build() {
        Ok(v) => v,
        Err(e) => {
            return Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({"message": format!("Failed to create verifier: {}", e)}).to_string().into())
                .map_err(Box::new)?);
        }
    };
            
    // More detailed logging for debugging
    println!("Token to verify: {}", my_token_strslice);
    println!("User Pool ID: {}", user_pool_id);
    println!("Client ID: {}", client_id);
    println!("Region: {}", cognito_region_name);
    
    let my_verif_result = key_set.verify(my_token_strslice, &verifier).await;
    println!("The verifier result: {:?} ", my_verif_result);

    // Extract email from verified token or return error
    let user_email = match my_verif_result {
        Ok(my_verif_parsed) => {
            // Parse the token claims into a serde_json::Value
            match serde_json::from_str::<serde_json::Value>(&my_verif_parsed.to_string()) {
                Ok(json_value) => {
                    // Extract email from token claims
                    json_value["email"].as_str()
                        .unwrap_or("unknown_user@example.com")
                        .to_string()
                },
                Err(e) => {
                    // If we can't parse the token claims as JSON, return an error
                    return Ok(Response::builder()
                        .status(500)
                        .header("content-type", "application/json")
                        .body(json!({"message": format!("Failed to parse token claims: {}", e)}).to_string().into())
                        .map_err(Box::new)?);
                }
            }
        },
        Err(e) => {
            return Ok(Response::builder()
                .status(401)
                .header("content-type", "application/json")
                .body(json!({"message": format!("Failed to verify token: {}", e)}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    println!("User email: {}", user_email);
    
    // Parse request body to get synonym update data
    let body = event.body();
    let synonym_input: SynonymUpdateInput = match serde_json::from_slice(body) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to parse request body: {}", e);
            return Ok(Response::builder()
                .status(400)
                .header("content-type", "application/json")
                .body(json!({"statusAPI": "ERROR", "message": "Invalid request body"}).to_string().into())
                .map_err(Box::new)?);
        }
    };

    // Get PostgreSQL configuration from environment variables
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

    // Get the current time for the update timestamp
    let current_time = Utc::now();
    
    // Build the update query dynamically based on which fields are provided
    let mut update_parts = Vec::new();
    let mut params = Vec::new();
    
    // Always add the updated_date and updated_by fields
    update_parts.push("updated_date = $1".to_string());
    params.push(&current_time as &(dyn tokio_postgres::types::ToSql + Sync));
    
    update_parts.push("updated_by = $2".to_string());
    params.push(&user_email as &(dyn tokio_postgres::types::ToSql + Sync));
    
    let mut param_index = 3; // Start from 3 because we already used $1 and $2
    
    // Add optional fields if they are provided
    if let Some(synonym_name) = &synonym_input.synonym_name {
        update_parts.push(format!("synonym_name = ${}", param_index));
        params.push(synonym_name as &(dyn tokio_postgres::types::ToSql + Sync));
        param_index += 1;
    }
    
    if let Some(synonym_value) = &synonym_input.synonym_value {
        update_parts.push(format!("synonym_value = ${}", param_index));
        params.push(synonym_value as &(dyn tokio_postgres::types::ToSql + Sync));
        param_index += 1;
    }
    
    if let Some(comments) = &synonym_input.comments {
        update_parts.push(format!("comments = ${}", param_index));
        params.push(comments as &(dyn tokio_postgres::types::ToSql + Sync));
        param_index += 1;
    }
    
    // Add the UUID condition
    params.push(&synonym_input.synonym_uuid as &(dyn tokio_postgres::types::ToSql + Sync));
    
    // Construct the final update query
    let update_query = format!(
        "UPDATE document_library.synonyms SET {} WHERE synonym_uuid = ${} RETURNING synonym_uuid, synonym_name, synonym_value",
        update_parts.join(", "),
        param_index
    );
    
    println!("Update query: {}", update_query);
    
    // Execute the update query
    match client.query_one(&update_query, &params[..]).await {
        Ok(row) => {
            println!("Successfully updated synonym: {}", synonym_input.synonym_uuid);
            
            // Extract updated data from the returned row
            let updated_synonym = SynonymOutput {
                synonym_uuid: row.get("synonym_uuid"),
                synonym_name: row.get("synonym_name"),
                synonym_value: row.get("synonym_value"),
                updated_by: user_email,
            };
            
            let response_body = json!({
                "statusAPI": "OK",
                "synonym": updated_synonym
            });
            
            let resp = Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(response_body.to_string().into())
                .map_err(Box::new)?;
            Ok(resp)
        },
        Err(e) => {
            eprintln!("Failed to update synonym: {}", e);
            Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({
                    "statusAPI": "ERROR", 
                    "message": format!("Failed to update synonym: {}", e)
                }).to_string().into())
                .map_err(Box::new)?)
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
