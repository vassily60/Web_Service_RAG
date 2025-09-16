use lambda_http::{run, service_fn, tracing, Body, Error, Request, RequestExt, Response};
// JSON Schema Validation
use jsonschema::JSONSchema;
use serde_json::{json, Value};
// Standard library imports
use std::env;
use std::fs::File;
use std::io::Read;


/// Load JSON schema from file and compile it
///
/// This function loads the JSON schema defined in payload_schema.json
/// and compiles it into a JSONSchema object for validation.
///
/// # Returns
/// A compiled JSONSchema object ready for validating JSON data
fn load_schema() -> JSONSchema {
    let schema_str = include_str!("payload_schema.json");
    let schema: Value = serde_json::from_str(schema_str).unwrap();
    JSONSchema::compile(&schema).unwrap()
}

/// Validate JSON data against schema
///
/// # Arguments
/// * `schema` - The JSONSchema to validate against
/// * `data` - The JSON data to validate
///
/// # Returns
/// Boolean indicating if the data is valid according to the schema
fn validate_json(schema: &JSONSchema, data: &Value) -> bool {
    schema.validate(data).is_ok()
}



/// Lambda function handler for JSON validation
///
/// This function processes incoming requests to validate JSON data against a schema.
/// It extracts client_id from the request body and validates it against our schema.
/// 
/// # Arguments
/// * `event` - The Lambda request event
///
/// # Returns
/// A response indicating whether the JSON is valid or not
async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Load schema for validation
    let schema = load_schema();
    
    // Parse the request body as JSON
    let body = event.body();
    let parsed_body: Result<Value, _> = serde_json::from_slice(body);
    
    match parsed_body {
        Ok(json_data) => {
            println!("Received JSON data: {}", json_data);
            
            // Validate the JSON against our schema
            let is_valid = validate_json(&schema, &json_data);
            
            // Check for required client_id field
            let client_id = json_data.get("client_id")
                .and_then(|id| id.as_i64())
                .or_else(|| json_data.get("client_id").and_then(|id| id.as_u64()).map(|id| id as i64));
                
            if client_id.is_none() {
                return Ok(Response::builder()
                    .status(400)
                    .header("content-type", "application/json")
                    .body(json!({"status": "error", "message": "client_id is required and must be a number"}).to_string().into())
                    .map_err(Box::new)?);
            }
            
            // Return validation result
            let response_body = json!({
                "status": if is_valid { "success" } else { "error" },
                "message": if is_valid { "JSON data is valid" } else { "JSON data is invalid" },
                "client_id": client_id,
                "schema_validation": is_valid
            });
            
            Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(response_body.to_string().into())
                .map_err(Box::new)?)
        },
        Err(e) => {
            // Return error for invalid JSON
            Ok(Response::builder()
                .status(400)
                .header("content-type", "application/json")
                .body(json!({"status": "error", "message": format!("Invalid JSON: {}", e)}).to_string().into())
                .map_err(Box::new)?)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    run(service_fn(function_handler)).await
}
