
use std::env;
use aws_sdk_secretsmanager::Client as SecretManagerClient;
use aws_config::Region;
use lambda_http::{run, service_fn, Body, Error, Request, Response};
use tracing_subscriber;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use reqwest::Client as ReqwestClient;
use chrono::NaiveDate;


#[derive(Deserialize)]
struct OpenAIAnswerRequest {
    question: String,
    chunks: Vec<Chunk>,
}

#[derive(Debug, Deserialize)]
struct Chunk {
    document_uuid: String,
    document_name: String,
    document_location: String,
    document_hash: String,

    document_type: String,

    document_status: String,
    
    /// Unique identifier for this specific chunk
    document_chunk_uuid: String,
    
    /// The actual text content extracted from the document
    embebed_text: String,
    
    /// Unique identifier for the embedding of this chunk
    document_embeding_uuid: String,
    
    /// Type of model used for embedding (openai, etc.)
    embeder_type: String,
    
    /// Number of tokens used in the embedding
    embedding_token: i32,
    
    /// Time taken to generate the embedding in seconds
    embedding_time: f64,
    
    /// Collection of metadata properties associated with the document
    document_metadata: Vec<DocumentMetadata>,
}

/// Structure for document metadata with various possible data types
///
/// Metadata can be stored in different formats (string, integer, float, boolean, date)
/// and this structure allows for any of those types to be used flexibly.
#[derive(Debug, Deserialize)]
struct DocumentMetadata {
    /// Unique identifier for this metadata entry
    metadata_uuid: String,
    
    /// Name/key of the metadata property
    metadata_name: String,
    
    /// Value as a string (if applicable)
    metadata_value_string: Option<String>,
    
    /// Value as an integer (if applicable)
    metadata_value_int: Option<i32>,
    
    /// Value as a float (if applicable)
    metadata_value_float: Option<f64>,
    
    /// Value as a boolean (if applicable)
    metadata_value_boolean: Option<bool>,
    
    /// Value as a date (if applicable)
    #[serde(with = "naive_date_format")]
    metadata_value_date: Option<chrono::NaiveDate>,
}

/// Custom serializer for handling NaiveDate formats in JSON
///
/// Provides custom deserialization logic for date strings in the format YYYY-MM-DD
/// into Rust's NaiveDate type, with proper error handling.
mod naive_date_format {
    use chrono::NaiveDate;
    use serde::{self, Deserialize, Deserializer};

    /// Deserialize an Option<String> into an Option<NaiveDate>
    ///
    /// # Arguments
    /// * `deserializer` - The deserializer to use
    ///
    /// # Returns
    /// * `Result<Option<NaiveDate>, D::Error>` - The deserialized date or an error
    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Option<NaiveDate>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        if let Some(s) = opt {
            return Ok(Some(NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                .map_err(serde::de::Error::custom)?));
        }
        Ok(None)
    }
}

/// Request structure for OpenAI Chat API
///
/// Contains all parameters needed to make a request to OpenAI's chat completion API.
#[derive(Serialize)]
struct OpenAIRequest {
    /// Model identifier (e.g., "gpt-4o-mini")
    model: String,
    
    /// Array of message objects containing the conversation history
    messages: Vec<Message>,
    
    /// Controls randomness: 0 is deterministic, higher values are more random
    temperature: f32,
    
    /// Maximum number of tokens to generate in the completion
    max_tokens: u32,
}

/// Message structure for OpenAI Chat API
///
/// Represents a single message in the conversation context
#[derive(Serialize)]
struct Message {
    /// The role of the message sender (system, user, assistant)
    role: String,
    
    /// The actual text content of the message
    content: String,
}

/// Response structure from OpenAI Chat API
///
/// Contains the response data returned by OpenAI's API
#[derive(Deserialize)]
struct OpenAIResponse {
    /// Array of generated completions (usually just one)
    choices: Vec<Choice>,
    
    /// Token usage statistics if provided
    usage: Option<Usage>,
}

/// Choice structure from OpenAI Chat API response
///
/// Contains a single completion choice from the API response
#[derive(Deserialize)]
struct Choice {
    /// The message content of the completion
    message: ChatMessage,
}

/// Message content from OpenAI Chat API response
///
/// The actual content of the generated completion
#[derive(Deserialize)]
struct ChatMessage {
    /// The text content of the message
    content: String,
}

/// Token usage statistics from OpenAI API
///
/// Tracks token usage for billing and rate limiting purposes
#[derive(Debug, Deserialize, Serialize)]
struct Usage {
    /// Number of tokens in the prompt (input)
    prompt_tokens: u32,
    
    /// Number of tokens in the completion (output)
    completion_tokens: u32,
    
    /// Total tokens used in the request
    total_tokens: u32,
}

/// Simplified response structure for chat completions
///
/// Our own simplified structure for passing the API response around
#[derive(Debug, Serialize)]
struct ChatResponse {
    /// The generated text content
    content: String,
    
    /// Token usage statistics if available
    usage: Option<Usage>,
}

async fn show_secret(client: &SecretManagerClient, name: &str) -> Result<String, Error> {
    // Get the secret value from AWS Secrets Manager
    let resp = match client.get_secret_value().secret_id(name).send().await {
        Ok(response) => response,
        Err(e) => return Err(format!("Failed to retrieve secret '{}': {}", name, e).into()),
    };
    
    // Extract the secret string from the response
    match resp.secret_string() {
        Some(secret) => Ok(secret.into()),
        None => Err(format!("Secret string not found in response for '{}'", name).into())
    }
}


async fn call_openai_chat(
    api_key: &str, 
    model: &str, 
    system_prompt: &str, 
    question: &str, 
    context: &str
) -> Result<ChatResponse, reqwest::Error> {
    let client = ReqwestClient::new();
    
    // Format the user's input with context and question
    let user_content = format!("Context: {}\n\nQuestion: {}", context, question);
    
    // Build the message array for the API request
    let messages = vec![
        Message {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        },
        Message {
            role: "user".to_string(),
            content: user_content,
        },
    ];
    
    let request = OpenAIRequest {
        model: model.to_string(),
        messages,
        temperature: 0.7,
        max_tokens: 2000,
    };

    // Send the request to OpenAI
    let response_result = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await;
    
    // Handle the HTTP response
    let response = match response_result {
        Ok(resp) => resp,
        Err(e) => return Err(e),
    };
    
    // Parse the JSON response
    let parsed_response = match response.json::<OpenAIResponse>().await {
        Ok(parsed) => parsed,
        Err(e) => return Err(e),
    };

    // Extract the generated content
    let content = if parsed_response.choices.is_empty() {
        "No answer generated.".to_string()
    } else {
        parsed_response.choices[0].message.content.clone()
    };

    Ok(ChatResponse {
        content,
        usage: parsed_response.usage,
    })
}

/// Lambda function handler that processes incoming API requests
///
/// Handles the API gateway request, extracts the question and document chunks,
/// calls OpenAI API to generate an answer, and returns a formatted response.
///
/// # Arguments
/// * `event` - The Lambda request event from API Gateway
///
/// # Returns
/// * `Result<Response<Body>, Error>` - HTTP response or error
async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Parse the incoming request
    let req: OpenAIAnswerRequest = match serde_json::from_slice(event.body().as_ref()) {
        Ok(r) => r,
        Err(e) => return Err(Box::new(e)),
    };

    // Get region from environment variable
    let region_name = env::var("REGION").expect("REGION environment variable not set");
    let region = Region::new(region_name.clone());
    
    // Initialize AWS SDK configuration
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(region)
        .load()
        .await;
    let client_secret = SecretManagerClient::new(&config);

    // Get database connection string from environment
    let db_secret_name = env::var("DATABASE_CONECTION_STRING")
        .expect("DATABASE_CONECTION_STRING environment variable not set");
    
    // Retrieve the secret from AWS Secrets Manager
    let db_secret = match show_secret(&client_secret, &db_secret_name).await {
        Ok(secret) => secret,
        Err(e) => return Err(e),
    };
    
    // Parse the secret JSON
    let db_credentials: serde_json::Value = match serde_json::from_str(&db_secret) {
        Ok(creds) => creds,
        Err(e) => return Err(Box::new(e)),
    };

    // Extract OpenAI API key from secrets
    let openai_api_key = match db_credentials["OPENAI_API_KEY"].as_str() {
        Some(key) => key,
        None => return Err("Failed to extract OpenAI API key from secrets".into()),
    };

    // Prepare the context by combining all chunks' text and metadata
    let context = req.chunks
        .iter()
        .map(|chunk| {
            // Convert metadata to string representation
            let metadata_str = chunk.document_metadata
                .iter()
                .map(|meta| {
                    let value = if let Some(s) = &meta.metadata_value_string {
                        s.clone()
                    } else if let Some(i) = meta.metadata_value_int {
                        i.to_string()
                    } else if let Some(f) = meta.metadata_value_float {
                        f.to_string()
                    } else if let Some(b) = meta.metadata_value_boolean {
                        b.to_string()
                    } else if let Some(d) = meta.metadata_value_date {
                        d.format("%Y-%m-%d").to_string()
                    } else {
                        "N/A".to_string()
                    };
                    format!("{}: {}", meta.metadata_name, value)
                })
                .collect::<Vec<String>>()
                .join("\n");
            
            format!(
                "Document: {}\nMetadata:\n{}\nContent: {}", 
                chunk.document_name,
                metadata_str,
                chunk.embebed_text
            )
        })
        .collect::<Vec<String>>()
        .join("\n\n");
    
    // Define the system prompt
    let system_prompt = "You are a helpful assistant that answers questions based on the provided document context and metadata. \
    Provide clear and concise answers based on both the content and metadata of the provided documents. \
    If the answer cannot be found in the context or metadata, state that you don't have enough information to answer.";
    
    // Call OpenAI to get the answer
    let model = "gpt-4o-mini"; // or another appropriate model
    
    let answer = match call_openai_chat(&openai_api_key, model, system_prompt, &req.question, &context).await {
        Ok(response) => response,
        Err(e) => return Err(Box::new(e)),
    };

    // Format the response with token usage information
    let token_usage = match answer.usage {
        Some(usage) => json!({
            "input_tokens": usage.prompt_tokens,
            "output_tokens": usage.completion_tokens,
            "total_tokens": usage.total_tokens
        }),
        None => json!(null),
    };

    let response_body = json!({
        "answer": answer.content,
        "token_usage": token_usage
    });

    // Build and return the HTTP response
    let resp = Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(response_body.to_string().into())
        .map_err(|e| Box::new(e))?;
    
    Ok(resp)
}

/// Entry point for the Lambda function
///
/// Sets up tracing and starts the Lambda runtime with our function handler
#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing for better observability
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or(tracing_subscriber::EnvFilter::new("INFO")),
        )
        .with_target(false)
        .without_time()
        .init();

    // Start the Lambda runtime with our function handler
    run(service_fn(function_handler)).await
}