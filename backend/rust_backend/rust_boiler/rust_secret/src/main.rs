
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use lambda_http::{run, service_fn, Body, Error, Request, Response};
use aws_sdk_secretsmanager::{Client as SecretManagerClient};
use std::env;
use serde_json::{json, Value};
use aws_sdk_secretsmanager::config::Region;
use dotenv::dotenv;

/// Retrieves a secret value from AWS Secrets Manager
///
/// # Arguments
///
/// * `client` - The AWS Secrets Manager client
/// * `name` - The name of the secret to retrieve
///
/// # Returns
///
/// The secret value as a string, or an error if the secret cannot be retrieved
async fn show_secret(client: &SecretManagerClient, name: &str) -> Result<String, Error> {
    println!("Retrieving secret: {}", name);
    
    let resp = client.get_secret_value().secret_id(name).send().await?;

    match resp.secret_string() {
        Some(secret) => {
            println!("Successfully retrieved secret of length: {}", secret.len());
            
            // Check if the retrieved secret is valid JSON (for logging purposes only)
            match serde_json::from_str::<Value>(&secret) {
                Ok(_) => println!("Secret appears to be valid JSON"),
                Err(e) => println!("Secret is not valid JSON: {}", e)
            }
            
            Ok(secret.into())
        },
        None => {
            // Check if there's binary data instead
            if let Some(_binary_data) = resp.secret_binary() {
                let error_msg = format!("Secret '{}' contains binary data, not string data", name);
                println!("Error: {}", error_msg);
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error_msg)));
            }
            
            let error_msg = format!("No string or binary value found for secret: {}", name);
            println!("Error: {}", error_msg);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, error_msg)));
        }
    }
}

/// Lambda function handler
///
/// Processes incoming API Gateway requests, retrieves secrets from AWS Secrets Manager,
/// and returns the appropriate response.
async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Log request information
    println!("---------------------------------------- EVENT ------------------------------------------------");
    println!("{:?}", event);
    println!("---------------------------------------- END OF EVENT -----------------------------------------");
    
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
    
    // Get the secret name from environment variable
    let default_secret_name = env::var("TEST_SECRET").expect("TEST_SECRET environment variable not set");
    println!("Secret name from env: {}", default_secret_name);
    
    println!("---------------------------------------- EXTRACT SECRET -----------------------------------------");
    println!("Retrieving secret: {}", default_secret_name);
    
    // Retrieve the secret
    match show_secret(&client_secret, &default_secret_name).await {
        Ok(secret_value) => {
            println!("Secret retrieved successfully");
            
            // Try to parse the secret value as JSON
            match serde_json::from_str::<Value>(&secret_value) {
                Ok(json_value) => {
                    println!("Secret parsed as valid JSON");
                    
                    // Return success response with the secret as JSON
                    let resp = Response::builder()
                        .status(200)
                        .header("content-type", "application/json")
                        .body(json!({
                            "statusAPI": "OK", 
                            "message": "The secret has been decoded and parsed as JSON",
                            "data": json_value
                        }).to_string().into())
                        .map_err(Box::new)?;
                    Ok(resp)
                },
                Err(parse_err) => {
                    println!("Secret is not valid JSON: {}", parse_err);
                    
                    // Return success but indicate the value is not JSON
                    let resp = Response::builder()
                        .status(200)
                        .header("content-type", "application/json")
                        .body(json!({
                            "statusAPI": "OK", 
                            "message": "The secret has been decoded (non-JSON value)",
                            "is_json": false,
                            "raw_length": secret_value.len()
                        }).to_string().into())
                        .map_err(Box::new)?;
                    Ok(resp)
                }
            }
        },
        Err(e) => {
            println!("Error retrieving secret: {:?}", e);
            
            // Return error response
            let resp = Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({
                    "statusAPI": "ERROR", 
                    "message": format!("Failed to retrieve secret: {}", e)
                }).to_string().into())
                .map_err(Box::new)?;
            Ok(resp)
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