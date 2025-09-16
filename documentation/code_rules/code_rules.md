# Code Rules Documentation

## Rust Backend Code Rules

## Environment Variables

No environment variable mus have any default value. The template must be
```rust
let db_secret_name = env::var("DATABASE_CONECTION_STRING").expect("DATABASE_CONECTION_STRING environment variable not set");
```

### use this template for the region management

```rust
// Get region from environment variable
    let region_name = env::var("REGION").expect("REGION environment variable not set");
```

### use this template for Postgresl management

```rust
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
```

### use the template for the management of the cognito token

```rust
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
                        // we can do our code logic here: for exemple just get the token ready to be use after verification
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
```

### use the template for the secrets manager management

```rust
// Initialize AWS SDK configuration
let region = Region::new(region_name.clone());
let config = aws_config::from_env().region(region).load().await;
let client_secret = SecretManagerClient::new(&config);
println!("AWS SDK initialized!");
```


## TEST SPECIFICATION

- We must verify that the answer is a properly formed JSON
- We must verify that the test are properly written with pytest
- please ensure that all the boolean values are set to true or false, and when loaded via env variable that the string are transformed in boolean

- please use the template below with class for each crud operation.
- I need to be able to run the test individually with the command pytest filename.py::mehtod

```python
# Test module for GET synonym operations
class TestGetSynonyms:
    """Test class for GET operations on synonyms"""
    
    def test_get_synonyms(self, api_credentials):
        """Test GET operation for synonyms"""
        endpoint = api_credentials['get_endpoint']('rust_get_synonym')
        print("GET API Endpoint:", endpoint)

        # Make the GET request to fetch all synonyms
        get_response = requests.post(
            endpoint,
            headers={
                "Content-Type": "application/json", 
                "Authorization": f"Bearer {api_credentials['token']}",
                "x-api-key": api_credentials['api_key']
            }
        )

        print(f'GET STATUS: {get_response.status_code} REASON: {get_response.reason}')
        print(f'GET RESPONSE: {get_response.text}')
        
        assert get_response.status_code == 200
```

