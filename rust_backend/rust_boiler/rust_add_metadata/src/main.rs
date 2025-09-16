use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde::{Serialize, Deserialize};
use serde_json::json;
use std::env;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use jsonwebtokens_cognito::KeySet;
use dotenv::dotenv;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_secretsmanager::Client as SecretManagerClient;

use tokio_postgres::{Client, Error as PostgresError, Transaction, Statement};
use postgres_native_tls::MakeTlsConnector;
use native_tls::TlsConnector;
use std::time::{Instant, Duration};

// Request struct to deserialize incoming data
#[derive(Debug, Serialize, Deserialize)]
struct MetadataRequest {
    metadata_name: String,
    metadata_description: String,
    metadata_type: String,
}

// Response struct for API
#[derive(Debug, Serialize, Deserialize)]
struct MetadataResponse {
    statusAPI: String,
    message: String,
    metadata_uuid: String,
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
    if !bearer_str.starts_with("Bearer ") {
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

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    let function_start_time = Instant::now();
    println!("\n====================================================================================");
    println!("==================== FUNCTION HANDLER START ({}) ====================", chrono::Utc::now());
    println!("====================================================================================");
    println!("🔍 Request headers: {:?}", event.headers());
    println!("🔍 Request method: {:?}", event.method());
    println!("🔍 Request URI: {:?}", event.uri());
    
    // Extract user email from token
    println!("\n📝 [STEP 1] Extracting user email from authorization token...");
    let auth_start_time = Instant::now();
    let user_email = match extract_email_from_token(&event).await {
        Ok(email) => {
            println!("✅ User email extracted successfully: {}", email);
            println!("⏱️  Token extraction took: {:?}", auth_start_time.elapsed());
            email
        },
        Err(e) => {
            println!("❌ Failed to extract email from token: {}", e);
            println!("⏱️  Failed token extraction attempt took: {:?}", auth_start_time.elapsed());
            return Ok(Response::builder()
                .status(401)
                .header("content-type", "application/json")
                .body(json!({"statusAPI": "ERROR", "message": format!("Unauthorized: Invalid token - {}", e)}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    // Parse request body
    println!("\n📝 [STEP 2] Parsing request body...");
    let parse_start_time = Instant::now();
    let body = event.body();
    let body_string = String::from_utf8_lossy(body);
    println!("📄 Raw request body: {}", body_string);
    
    let request_data: MetadataRequest = match serde_json::from_slice(body.as_ref()) {
        Ok(data) => {
            println!("✅ Request body parsed successfully: {:?}", data);
            println!("⏱️  Body parsing took: {:?}", parse_start_time.elapsed());
            data
        },
        Err(e) => {
            println!("❌ Failed to parse request body: {}", e);
            println!("⏱️  Failed body parsing took: {:?}", parse_start_time.elapsed());
            return Ok(Response::builder()
                .status(400)
                .header("content-type", "application/json")
                .body(json!({"statusAPI": "ERROR", "message": format!("Invalid request format: {}", e)}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    // Validate required fields
    println!("\n📝 [STEP 3] Validating required fields...");
    if request_data.metadata_name.is_empty() || request_data.metadata_description.is_empty() || request_data.metadata_type.is_empty() {
        println!("❌ Validation failed: One or more required fields are empty");
        println!("   - metadata_name: {}", if request_data.metadata_name.is_empty() { "MISSING ❌" } else { "OK ✅" });
        println!("   - metadata_description: {}", if request_data.metadata_description.is_empty() { "MISSING ❌" } else { "OK ✅" });
        println!("   - metadata_type: {}", if request_data.metadata_type.is_empty() { "MISSING ❌" } else { "OK ✅" });
        
        return Ok(Response::builder()
            .status(400)
            .header("content-type", "application/json")
            .body(json!({"statusAPI": "ERROR", "message": "Missing required fields"}).to_string().into())
            .map_err(Box::new)?);
    }
    println!("✅ All required fields are present and valid");

    // Load environment variables for database connection
    println!("\n📝 [STEP 4] Loading database connection parameters from environment...");
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
    
    println!("✅ Database connection parameters loaded:");
    println!("   - DB_HOST: {}", db_server);
    println!("   - DB_NAME: {}", database);
    println!("   - DB_USER: {}", db_username);
    println!("   - DB_PORT: {}", db_port);

    // Configure TLS for database connection
    println!("\n📝 [STEP 5] Setting up TLS connector for database connection...");
    let tls_start_time = Instant::now();
    let tls_connector_result = TlsConnector::builder()
        .danger_accept_invalid_certs(true) // Disable certificate validation for development
        .build();
    
    let tls = match tls_connector_result {
        Ok(connector) => {
            println!("✅ TLS connector built successfully");
            MakeTlsConnector::new(connector)
        },
        Err(e) => {
            println!("❌ Failed to build TLS connector: {}", e);
            return Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({"statusAPI": "ERROR", "message": format!("Failed to build TLS connector: {}", e)}).to_string().into())
                .map_err(Box::new)?);
        }
    };
    println!("⏱️  TLS setup took: {:?}", tls_start_time.elapsed());

    // Connect to the database
    println!("\n📝 [STEP 6] Connecting to PostgreSQL database...");
    let db_connect_start_time = Instant::now();
    let connection_string = format!(
        "host={} port={} user={} password={} dbname={} application_name=rust_add_metadata_lambda", 
        db_server, db_port, db_username, db_password, database
    );
    println!("🔗 Using connection string: host={} port={} user={} dbname={} application_name=rust_add_metadata_lambda", 
        db_server, db_port, db_username, database);
    
    let connection_result = tokio_postgres::connect(&connection_string, tls).await;
    let (client, connection) = match connection_result {
        Ok((client, connection)) => {
            println!("✅ Successfully connected to database");
            println!("⏱️  Database connection took: {:?}", db_connect_start_time.elapsed());
            (client, connection)
        },
        Err(e) => {
            println!("❌ Failed to connect to database: {}", e);
            println!("⏱️  Failed connection attempt took: {:?}", db_connect_start_time.elapsed());
            
            // Try to get more details about the connection error
            let error_details = match e.source() {
                Some(source) => format!(" (Source: {})", source),
                None => "".to_string()
            };
            
            return Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({
                    "statusAPI": "ERROR", 
                    "message": format!("Database connection failed: {}{}", e, error_details)
                }).to_string().into())
                .map_err(Box::new)?);
        }
    };
    
    // Spawn a task to manage the connection
    tokio::spawn(async move {
        println!("📝 Background task: Managing database connection...");
        if let Err(e) = connection.await {
            eprintln!("❌ CONNECTION ERROR in background task: {}", e);
        }
    });

    // Begin transaction
    println!("\n📝 [STEP 7] Starting database transaction...");
    let transaction_start_time = Instant::now();
    let transaction_result = client.transaction().await;
    let transaction = match transaction_result {
        Ok(txn) => {
            println!("✅ Transaction started successfully");
            println!("⏱️  Transaction setup took: {:?}", transaction_start_time.elapsed());
            txn
        },
        Err(e) => {
            println!("❌ Failed to start transaction: {}", e);
            println!("⏱️  Failed transaction attempt took: {:?}", transaction_start_time.elapsed());
            return Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({"statusAPI": "ERROR", "message": format!("Failed to start database transaction: {}", e)}).to_string().into())
                .map_err(Box::new)?);
        }
    };

    // Generate a new UUID and prepare timestamp for the metadata
    let metadata_uuid = Uuid::new_v4().to_string();
    let current_time = Utc::now();
    println!("\n📝 [STEP 8] Preparing data for insertion...");
    println!("🔑 Generated metadata UUID: {}", metadata_uuid);
    println!("⏰ Current timestamp: {}", current_time);

    // First check if schema and table exist
    println!("\n📝 [STEP 9] Verifying database schema and table existence...");
    let schema_check_query = "SELECT EXISTS (SELECT 1 FROM information_schema.schemata WHERE schema_name = 'document_library')";
    let table_check_query = "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'document_library' AND table_name = 'metadatas')";
    
    let schema_exists = match transaction.query_one(schema_check_query, &[]).await {
        Ok(row) => {
            let exists: bool = row.get(0);
            println!("🔍 Schema 'document_library' exists: {}", exists);
            exists
        },
        Err(e) => {
            println!("❌ Error checking schema existence: {}", e);
            false
        }
    };
    
    let table_exists = match transaction.query_one(table_check_query, &[]).await {
        Ok(row) => {
            let exists: bool = row.get(0);
            println!("🔍 Table 'document_library.metadatas' exists: {}", exists);
            exists
        },
        Err(e) => {
            println!("❌ Error checking table existence: {}", e);
            false
        }
    };
    
    if !schema_exists || !table_exists {
        println!("❌ Required database objects don't exist!");
        let missing = if !schema_exists { "schema 'document_library'" } else { "table 'document_library.metadatas'" };
        
        return Ok(Response::builder()
            .status(500)
            .header("content-type", "application/json")
            .body(json!({"statusAPI": "ERROR", "message": format!("Database missing required {}", missing)}).to_string().into())
            .map_err(Box::new)?);
    }

    // Insert new metadata record into the database
    println!("\n📝 [STEP 10] Inserting metadata record into database...");
    let insert_start_time = Instant::now();
    let insert_query = "INSERT INTO document_library.metadatas 
        (metadata_uuid, metadata_name, metadata_description, metadata_type, creation_date, created_by) 
        VALUES ($1, $2, $3, $4, $5, $6)";
    
    println!("🔍 Insert query: {}", insert_query);
    println!("🔍 Parameters:");
    println!("   - metadata_uuid: {}", metadata_uuid);
    println!("   - metadata_name: {}", request_data.metadata_name);
    println!("   - metadata_description: {}", request_data.metadata_description);
    println!("   - metadata_type: {}", request_data.metadata_type);
    println!("   - creation_date: {}", current_time);
    println!("   - created_by: {}", user_email);
    
    let insert_result = transaction.execute(
        insert_query, 
        &[
            &metadata_uuid, 
            &request_data.metadata_name, 
            &request_data.metadata_description, 
            &request_data.metadata_type,
            &current_time,
            &user_email
        ]
    ).await;
    
    match insert_result {
        Ok(rows) => {
            println!("✅ INSERT SUCCESS: {} row(s) affected", rows);
            println!("⏱️  Insert operation took: {:?}", insert_start_time.elapsed());
            
            // Verify the insert worked by querying the database
            println!("\n📝 [STEP 11] Verifying successful insertion with SELECT query...");
            let verify_start_time = Instant::now();
            let verify_query = "SELECT * FROM document_library.metadatas WHERE metadata_uuid = $1";
            
            match transaction.query_one(verify_query, &[&metadata_uuid]).await {
                Ok(row) => {
                    let name: &str = row.get("metadata_name");
                    let description: &str = row.get("metadata_description");
                    let metadata_type: &str = row.get("metadata_type");
                    let created_by: &str = row.get("created_by");
                    
                    println!("✅ VERIFICATION SUCCESS: Found inserted record with data:");
                    println!("   - metadata_uuid: {}", metadata_uuid);
                    println!("   - metadata_name: {}", name);
                    println!("   - metadata_description: {}", description);
                    println!("   - metadata_type: {}", metadata_type);
                    println!("   - created_by: {}", created_by);
                    println!("⏱️  Verification query took: {:?}", verify_start_time.elapsed());
                },
                Err(e) => {
                    println!("❌ VERIFICATION FAILED: Could not find inserted record: {}", e);
                    println!("⏱️  Failed verification attempt took: {:?}", verify_start_time.elapsed());
                }
            }
            
            // SQL query to add missing metadata to document_metadatas
            println!("\n📝 [STEP 12] Adding missing metadata entries to document_metadatas...");
            let add_missing_start_time = Instant::now();
            
            // First check if document_metadatas table exists
            println!("🔍 Checking if document_metadatas table exists...");
            let table_check_query = "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'document_library' AND table_name = 'document_metadatas')";
            
            let dm_table_exists = match transaction.query_one(table_check_query, &[]).await {
                Ok(row) => {
                    let exists: bool = row.get(0);
                    println!("🔍 Table 'document_library.document_metadatas' exists: {}", exists);
                    exists
                },
                Err(e) => {
                    println!("⚠️ Error checking document_metadatas table existence: {}", e);
                    false
                }
            };
            
            // Check if documents table exists
            println!("🔍 Checking if documents table exists...");
            let doc_table_check_query = "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'document_library' AND table_name = 'documents')";
            
            let doc_table_exists = match transaction.query_one(doc_table_check_query, &[]).await {
                Ok(row) => {
                    let exists: bool = row.get(0);
                    println!("🔍 Table 'document_library.documents' exists: {}", exists);
                    exists
                },
                Err(e) => {
                    println!("⚠️ Error checking documents table existence: {}", e);
                    false
                }
            };
            
            if dm_table_exists && doc_table_exists {
                // Count documents in the documents table
                let count_docs_query = "SELECT COUNT(*) FROM document_library.documents";
                let doc_count = match transaction.query_one(count_docs_query, &[]).await {
                    Ok(row) => {
                        let count: i64 = row.get(0);
                        println!("📊 Number of documents in document_library.documents: {}", count);
                        count
                    },
                    Err(e) => {
                        println!("⚠️ Error counting documents: {}", e);
                        0
                    }
                };
                
                // Only proceed if there are documents
                if doc_count > 0 {
                    let add_missing_metadata_query = "
                        INSERT INTO document_library.document_metadatas
                        (document_metadata_uuid, document_uuid, metadata_uuid, creation_date, created_by, \"comments\")
                        select 
                        cast(gen_random_uuid() as text), -- document_metadata_uuid
                        cartesian_metadata.document_uuid, -- document_uuid
                        cartesian_metadata.metadata_uuid, -- metadata_uuid
                        NOW(),
                        $1, -- created_by
                        'missing metadata' -- comments
                        from (select distinct
                        d.document_uuid as document_uuid,
                        m.metadata_uuid as metadata_uuid
                        FROM document_library.documents d, 
                        document_library.metadatas m WHERE m.metadata_uuid = $2) as cartesian_metadata
                        left join document_library.document_metadatas dm on dm.document_uuid = cartesian_metadata.document_uuid and dm.metadata_uuid = cartesian_metadata.metadata_uuid 
                        where dm.metadata_uuid is null;
                    ";
                    
                    println!("🔍 Add missing metadata query prepared");
                    println!("🔍 Query: {}", add_missing_metadata_query);
                    println!("🔍 Using parameters:");
                    println!("   - created_by: {}", user_email);
                    println!("   - metadata_uuid: {}", metadata_uuid);
                    
                    match transaction.execute(add_missing_metadata_query, &[&user_email, &metadata_uuid]).await {
                        Ok(rows_affected) => {
                            println!("✅ Successfully added {} missing metadata entries to document_metadatas", rows_affected);
                        },
                        Err(e) => {
                            println!("⚠️ Failed to add missing metadata entries: {}", e);
                            println!("⚠️ This is not critical - continuing with commit");
                            
                            // Try to get additional database error info
                            let pg_error = match e.as_db_error() {
                                Some(db_err) => format!("PostgreSQL error: Code={}, Detail={}", 
                                                       db_err.code(), db_err.detail().unwrap_or("No details")),
                                None => "Not a database error".to_string()
                            };
                            println!("⚠️ Error details: {}", pg_error);
                        }
                    };
                } else {
                    println!("ℹ️ No documents found - skipping adding missing metadata entries");
                }
            } else {
                println!("ℹ️ Required tables for adding missing metadata don't exist - skipping this step");
            }
            
            println!("⏱️ Missing metadata operation took: {:?}", add_missing_start_time.elapsed());
            
            // Commit the transaction
            println!("\n📝 [STEP 13] Committing the database transaction...");
            let commit_start_time = Instant::now();
            match transaction.commit().await {
                Ok(_) => {
                    println!("✅ Transaction committed successfully");
                    println!("⏱️ Commit operation took: {:?}", commit_start_time.elapsed());
                    
                    // Success response
                    let response_body = MetadataResponse {
                        statusAPI: "OK".to_string(),
                        message: "Metadata added successfully".to_string(),
                        metadata_uuid: metadata_uuid,
                    };
                    
                    println!("\n✅ SUCCESS: Metadata added with UUID: {}", metadata_uuid);
                    println!("⏱️ Total function execution time: {:?}", function_start_time.elapsed());
                    println!("====================================================================================");
                    println!("==================== FUNCTION HANDLER END (SUCCESS) ====================");
                    println!("====================================================================================\n");
                    
                    Ok(Response::builder()
                        .status(200)
                        .header("content-type", "application/json")
                        .body(json!(response_body).to_string().into())
                        .map_err(Box::new)?)
                },
                Err(e) => {
                    println!("❌ Failed to commit transaction: {}", e);
                    println!("⏱️ Failed commit attempt took: {:?}", commit_start_time.elapsed());
                    
                    // Get detailed error information
                    let pg_error = match e.as_db_error() {
                        Some(db_err) => format!("PostgreSQL error: Code={}, Detail={}", 
                                               db_err.code(), db_err.detail().unwrap_or("No details")),
                        None => "Not a database error".to_string()
                    };
                    println!("❌ Error details: {}", pg_error);
                    
                    println!("\n❌ ERROR: Failed to commit transaction");
                    println!("⏱️ Total function execution time: {:?}", function_start_time.elapsed());
                    println!("====================================================================================");
                    println!("==================== FUNCTION HANDLER END (ERROR) ====================");
                    println!("====================================================================================\n");
                    
                    Ok(Response::builder()
                        .status(500)
                        .header("content-type", "application/json")
                        .body(json!({
                            "statusAPI": "ERROR", 
                            "message": format!("Failed to commit metadata: {}", e)
                        }).to_string().into())
                        .map_err(Box::new)?)
                }
            }
        },
        Err(e) => {
            println!("❌ Failed to insert metadata: {}", e);
            
            // Get detailed error information
            let pg_error = match e.as_db_error() {
                Some(db_err) => {
                    println!("❌ PostgreSQL error code: {}", db_err.code());
                    println!("❌ PostgreSQL error message: {}", db_err.message());
                    println!("❌ PostgreSQL error detail: {}", db_err.detail().unwrap_or("No details"));
                    println!("❌ PostgreSQL error hint: {}", db_err.hint().unwrap_or("No hint"));
                    println!("❌ PostgreSQL error position: {}", db_err.position().unwrap_or(-1));
                    
                    format!("PostgreSQL error: Code={}, Message={}, Detail={}", 
                           db_err.code(), db_err.message(), db_err.detail().unwrap_or("No details"))
                },
                None => {
                    println!("❌ Not a database error");
                    "Not a database error".to_string()
                }
            };
            
            // Try to rollback the transaction
            println!("\n📝 [EMERGENCY] Attempting to roll back the transaction...");
            if let Err(rollback_err) = transaction.rollback().await {
                println!("❌ Failed to roll back transaction: {}", rollback_err);
            } else {
                println!("✅ Transaction rolled back successfully");
            }
            
            println!("\n❌ ERROR: Failed to insert metadata");
            println!("⏱️ Total function execution time: {:?}", function_start_time.elapsed());
            println!("====================================================================================");
            println!("==================== FUNCTION HANDLER END (ERROR) ====================");
            println!("====================================================================================\n");
            
            Ok(Response::builder()
                .status(500)
                .header("content-type", "application/json")
                .body(json!({
                    "statusAPI": "ERROR", 
                    "message": format!("Failed to insert metadata: {}", e),
                    "error_details": pg_error
                }).to_string().into())
                .map_err(Box::new)?)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    println!("🚀 Starting rust_add_metadata Lambda function");
    
    // Load environment variables from .env file if it exists
    match dotenv() {
        Ok(_) => println!("✅ Successfully loaded environment variables from .env file"),
        Err(e) => println!("ℹ️ No .env file loaded: {} (This is normal in production)", e),
    }
    
    // Print some environment variables for debugging (hiding sensitive values)
    println!("🔧 Environment configuration:");
    println!("   - DB_HOST: {}", env::var("DB_HOST").expect("DB_HOST environment variable not set"));
    println!("   - DB_NAME: {}", env::var("DB_NAME").expect("DB_NAME environment variable not set"));
    println!("   - DB_USER: {}", env::var("DB_USER").expect("DB_USER environment variable not set"));
    println!("   - DB_PORT: {}", env::var("DB_PORT").expect("DB_PORT environment variable not set"));
    println!("   - DB_PASSWORD: {}", "********"); // Password value hidden for security
    println!("   - COGNITO_REGION: {}", env::var("COGNITO_REGION").expect("COGNITO_REGION environment variable not set"));
    println!("   - COGNITO_USER_POOL_ID: {}", env::var("COGNITO_USER_POOL_ID").expect("COGNITO_USER_POOL_ID environment variable not set"));
    println!("   - COGNITO_CLIENT_ID: {}", env::var("COGNITO_CLIENT_ID").expect("COGNITO_CLIENT_ID environment variable not set"));
    
    // Initialize the tracing subscriber for structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or(tracing_subscriber::EnvFilter::new("INFO")),
        )
        .with_target(false)
        .with_ansi(false)  // Disable ANSI colors for better log readability in CloudWatch
        .init();

    println!("✅ Logger initialized");
    println!("✅ Starting Lambda runtime...");
    
    // Run the Lambda function handler
    run(service_fn(function_handler)).await
}