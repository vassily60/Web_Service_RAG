use lambda_http::{run, service_fn, Body, Error, Request, Response};
use jsonwebtokens_cognito::{self, KeySet};
use serde_json::json;
use serde_json::Value;
use std::env;
use aws_sdk_secretsmanager::Client as SecretManagerClient;
use aws_sdk_secretsmanager::config::Region;
use aws_config;

// Function to retrieve secrets from AWS Secrets Manager
async fn show_secret(client: &SecretManagerClient, name: &str) -> Result<String, Error> {
    let resp = client.get_secret_value().secret_id(name).send().await?;
    match resp.secret_string() {
        Some(secret) => Ok(secret.into()),
        None => panic!("Error getting the secret: {:?}", name),
    }
}


/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // we print the event
    println!("{:?}", event);

    //we declare the variables
    let mut my_verif_str: String = "No Token".to_string();
 
    // we get the authorization
    let key_to_check = "Authorization";
    
    println!("The key '{}' is in the table.", key_to_check);

    // Check if the Authorization header exists
    if !event.headers().contains_key(key_to_check) {
        return Ok(Response::builder()
            .status(401)
            .header("content-type", "application/json")
            .body(json!({"message": "Missing Authorization header"}).to_string().into())
            .map_err(Box::new)?);
    }

    let my_bearer_strslice: &str = event.headers()[key_to_check].to_str()?;
    println!("The Content: {:?} ", my_bearer_strslice);

    // Check if token has correct format (Bearer prefix)
    if !my_bearer_strslice.starts_with("Bearer ") || my_bearer_strslice.len() <= 7 {
        return Ok(Response::builder()
            .status(401)
            .header("content-type", "application/json")
            .body(json!({"message": "Invalid Authorization format"}).to_string().into())
            .map_err(Box::new)?);
    }

    let my_token_strslice: &str = &my_bearer_strslice[7..];
    println!("The token: {:?} ", my_token_strslice);
    
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

    match my_verif_result {
        Ok(my_verif_parsed) => {
            // Parse the token claims into a serde_json::Value
            // This ensures we return a proper JSON object, not a string
            match serde_json::from_str::<serde_json::Value>(&my_verif_parsed.to_string()) {
                Ok(json_value) => {
                    Ok(Response::builder()
                        .status(200)
                        .header("content-type", "application/json")
                        .body(json_value.to_string().into())
                        .map_err(Box::new)?)
                },
                Err(e) => {
                    // If we can't parse the token claims as JSON, return an error
                    Ok(Response::builder()
                        .status(500)
                        .header("content-type", "application/json")
                        .body(json!({"message": format!("Failed to parse token claims: {}", e)}).to_string().into())
                        .map_err(Box::new)?)
                }
            }
        },
        Err(e) => {
            Ok(Response::builder()
                .status(401)
                .header("content-type", "application/json")
                .body(json!({"message": format!("Failed to verify token: {}", e)}).to_string().into())
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
        // disable printing the name of the module in every log line.
        .with_target(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();
    

    run(service_fn(function_handler)).await
}
