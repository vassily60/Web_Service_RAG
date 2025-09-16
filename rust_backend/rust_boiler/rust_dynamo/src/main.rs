use lambda_http::{run, service_fn, Body, Error, Request, Response};
use rusoto_core::Region;
use rusoto_dynamodb::{DynamoDb, DynamoDbClient, ScanInput};
use serde_json;
use serde_json::Value;



/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
async fn function_handler(_event: Request) -> Result<Response<Body>, Error> {
    // Extract some useful information from the request
    
    let table_name_str: &str = "hello_dynamo";
    let table_name_str_string: String = table_name_str.to_string(); // or String::from(my_str)

    // Create a client to access DynamoDB
    let client = DynamoDbClient::new(Region::ApSoutheast1);
    let scan_input = ScanInput {
        table_name: table_name_str_string,
        ..Default::default()
    };

    let my_return: String;
    match client.scan(scan_input).await {
        Ok(output) => {
            
            let json_string = serde_json::to_string(&output.items).expect("Failed to convert to JSON");
            my_return = json_string.clone();
            println!("JSON string: {}", json_string);
            let v: Value = serde_json::from_str(&json_string).unwrap();
            
            // we print in the logs all the elements
            for elem in v.as_array().unwrap() {
                println!("{:?}",elem["ID_UUID"]["S"].to_string());
            }
        }
        Err(error) => {
            println!("Error: {:?}", error);
            my_return="Error".to_string();
        }
    } ;

    let message = my_return;
    // we replace
    let resp = Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(message.into())
        .map_err(Box::new)?;
    Ok(resp)

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
