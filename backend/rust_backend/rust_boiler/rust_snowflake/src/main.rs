use tracing_subscriber::filter::{EnvFilter, LevelFilter};use lambda_http::{run, service_fn, Body, Error, Request, Response};
use snowflake_connector_rs::SnowflakeClient;
use snowflake_connector_rs::SnowflakeClientConfig;
use snowflake_connector_rs::SnowflakeAuthMethod;

struct SnowflakeClientCredentials {
    username: String,
    password: String,
    account: String,
    role: String,
    warehouse: String,
    database: String,
    schema: String,
}
/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Extract some useful information from the request
    // we print the event
    println!("---------------------------------------- EVENT ------------------------------------------------");
    println!("{:?}",event);
    println!("---------------------------------------- END OF EVENT -----------------------------------------");
    
    // Let's start the madness
    //Declare the snowflake client
    let snowflake_credential = SnowflakeClientCredentials {
        username: "bfoucque".to_string(),
        password: "".to_string(),
        account: "QD77191.eu-west-1".to_string(),
        role: "SYSADMIN".to_string(),
        warehouse: "INNOVATION".to_string(),
        database: "SUSTAINABILITY_PLATFORM".to_string(),
        schema: "ESRS".to_string(),
    };

    /*
    let client = SnowflakeClient::new(
        "USERNAME",
        SnowflakeAuthMethod::Password("PASSWORD".to_string()),
        SnowflakeClientConfig {
            account: "ACCOUNT".to_string(),
            role: Some("ROLE".to_string()),
            warehouse: Some("WAREHOUSE".to_string()),
            database: Some("DATABASE".to_string()),
            schema: Some("SCHEMA".to_string()),
        },
    )?;
     */
    // Create the snowflake client
    let client = SnowflakeClient::new(
        &snowflake_credential.username.to_string(),
        SnowflakeAuthMethod::Password(snowflake_credential.password.to_string()),
        SnowflakeClientConfig {
            account: snowflake_credential.account.to_string(),
            role: Some(snowflake_credential.role.to_string()),
            warehouse: Some(snowflake_credential.warehouse.to_string()),
            database: Some(snowflake_credential.database.to_string()),
            schema: Some(snowflake_credential.schema.to_string()),
        },
    )?;
    
    let session = client.create_session().await?;
    
    let query = "SELECT qesrs.ESRS_PARAGRAPH AS ESRS_PARAGRAPH FROM SUSTAINABILITY_PLATFORM.ESRS.ESRS_QUESTION qesrs LIMIT 5;";
    let rows = session.query(query).await?;
    
    for my_row in &rows {
        //println!("{:?}",my_row);
        println!("{:?}",my_row.get::<String>("ESRS_PARAGRAPH")?);
    }
    
    /* 
    assert_eq!(rows[0].get::<i64>("ID")?, 1);
    assert_eq!(rows[0].get::<String>("VALUE")?, "hello");
    */

    // Return something that implements IntoResponse.
    let message = "Hello, Rust SNOWFLAKE !";
    // It will be serialized to the right response event automatically by the runtime
    let resp = Response::builder()
        .status(200)
        .header("content-type", "text/html")
        .body(message.into())
        .map_err(Box::new)?;
    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        // disable printing the name of the module in every log line.
        .with_target(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}
