use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use std::env;
use std::time::Duration;
use jsonwebtokens_cognito::KeySet;
use uuid::Uuid;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_secretsmanager::Client as SecretManagerClient;
use aws_sdk_secretsmanager::config::Region;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::primitives::ByteStream;

// Request structure for generating a presigned URL for uploading
#[derive(Debug, Serialize, Deserialize)]
struct UploadUrlRequest {
    file_name: String,
    content_type: String,
    #[serde(default = "default_expiration")]
    expiration: u64,
}

fn default_expiration() -> u64 {
    900 // Default to 15 minutes (900 seconds)
}

// Response structure
#[derive(Debug, Serialize, Deserialize)]
struct UploadUrlResponse {
    status: String,
    presigned_url: String,
    file_name: String,
    expiration: u64,
    bucket: String,
    key: String,
}

async fn show_secret(client: &SecretManagerClient, name: &str) -> Result<String, Error> {
    let resp = client.get_secret_value().secret_id(name).send().await?;
    match resp.secret_string() {
        Some(secret) => Ok(secret.into()),
        None => {
            eprintln!("Error retrieving secret: {}", name);
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Error retrieving secret: {}", name))))
        }
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



    // Get Cognito configuration from AWS Secrets Manager
    let cognito_secret_name = env::var("COGNITO_SECRET").expect("COGNITO_SECRET environment variable not set");
    
    // Get region from environment variable
    let region_name = env::var("REGION").expect("REGION environment variable not set");
    
    // Initialize AWS SDK configuration
    let region = Region::new(region_name.clone());
    let config = aws_config::from_env().region(region).load().await;
    let client_secret = SecretManagerClient::new(&config);
    println!("AWS SDK initialized!");
    
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

    // Use the region name string instead of Region struct for KeySet creation
    let keyset = KeySet::new(cognito_region_name, user_pool_id);
    
    match keyset {
        Ok(key_set) => {
            let verifier = key_set.new_id_token_verifier(&[&client_id]).build()?;
            let verification_result = key_set.verify(token_str, &verifier).await;
            
            match verification_result {
                Ok(claims) => {
                    // Extract email from claims
                    if let Some(email) = claims.get("email").and_then(|v| v.as_str()) {
                        return Ok(email.to_string());
                    } else {
                        println!("No email found in token claims.");
                        return Ok("unknown@user.com".to_string());
                    }
                },
                Err(e) => {
                    println!("Token verification error: {}", e);
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Invalid token")));
                }
            }
        },
        Err(e) => {
            println!("KeySet creation error: {}", e);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("KeySet creation error: {}", e))));
        }
    }
}

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    println!("Received event: {:?}", event);

    // Get S3 bucket region from environment variable or default to ap-southeast-1
    let s3_region = env::var("S3BUCKET_REGION").unwrap_or_else(|_| {
        println!("S3BUCKET_REGION environment variable not set, using default region");
        "ap-southeast-1".to_string()
    });
    println!("Using S3 bucket region: {}", s3_region);

    // Set up region provider to use the same region as the S3 bucket
    let region_provider = RegionProviderChain::first_try(Region::new(s3_region.clone()))
        .or_default_provider();

    // Load AWS configuration
    let shared_config = aws_config::from_env().region(region_provider).load().await;

    // Extract user email from token for auditing
    let user_email = match extract_email_from_token(&event).await {
        Ok(email) => email,
        Err(e) => {
            println!("Failed to extract email from token: {:?}", e);
            return Ok(Response::builder()
                .status(401)
                .header("Content-Type", "application/json")
                .body(Body::from(json!({
                    "status": "error",
                    "message": "Authentication failed",
                    "error": format!("{}", e)
                }).to_string()))?);
        }
    };
    println!("Request from user: {}", user_email);

    // Parse the request body
    let body = event.body();
    let upload_request: UploadUrlRequest = match serde_json::from_slice(body) {
        Ok(req) => req,
        Err(e) => {
            println!("Failed to parse request body: {:?}", e);
            return Ok(Response::builder()
                .status(400)
                .header("Content-Type", "application/json")
                .body(Body::from(json!({
                    "status": "error",
                    "message": "Invalid request body",
                    "error": format!("{}", e)
                }).to_string()))?);
        }
    };

    // Get S3 bucket name from environment variable
    let bucket_name = match env::var("S3BUCKET_IMPORT_FOLDER") {
        Ok(name) => name,
        Err(_) => {
            println!("S3BUCKET_IMPORT_FOLDER environment variable not set");
            return Ok(Response::builder()
                .status(500)
                .header("Content-Type", "application/json")
                .body(Body::from(json!({
                    "status": "error",
                    "message": "S3 bucket name not configured",
                    "error": "S3BUCKET_IMPORT_FOLDER environment variable not set"
                }).to_string()))?);
        }
    };

    // Generate a unique key for the file using UUID
    let file_extension = upload_request.file_name.split('.').last().unwrap_or("");
    let file_key = format!("uploads/{}-{}.{}", 
        Uuid::new_v4().to_string(), 
        upload_request.file_name.replace(&format!(".{}", file_extension), ""),
        file_extension
    );

    // Create S3 client
    let s3_client = S3Client::new(&shared_config);

    // Create presigning config with the specified expiration
    let presign_config = match PresigningConfig::expires_in(Duration::from_secs(upload_request.expiration)) {
        Ok(config) => config,
        Err(e) => {
            println!("Failed to create presigning config: {:?}", e);
            return Ok(Response::builder()
                .status(500)
                .header("Content-Type", "application/json")
                .body(Body::from(json!({
                    "status": "error",
                    "message": "Failed to create presigning config",
                    "error": format!("{}", e)
                }).to_string()))?);
        }
    };

    // Generate the presigned URL for PUT operation (upload)
    let presigned_request = s3_client
        .put_object()
        .bucket(&bucket_name)
        .key(&file_key)
        .content_type(&upload_request.content_type)
        .presigned(presign_config)
        .await;

    match presigned_request {
        Ok(presigned_req) => {
            // Convert the presigned request to a URL string
            let presigned_url = presigned_req.uri().to_string();

            println!("Generated presigned URL for upload: {}", presigned_url);
            
            // Return success response with the URL and details
            let response = UploadUrlResponse {
                status: "success".to_string(),
                presigned_url,
                file_name: upload_request.file_name,
                expiration: upload_request.expiration,
                bucket: bucket_name,
                key: file_key,
            };

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&response)?))?)
        },
        Err(e) => {
            println!("Failed to generate presigned URL: {:?}", e);
            Ok(Response::builder()
                .status(500)
                .header("Content-Type", "application/json")
                .body(Body::from(json!({
                    "status": "error",
                    "message": "Failed to generate presigned URL",
                    "error": format!("{}", e)
                }).to_string()))?)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .init();

    run(service_fn(function_handler)).await
}
