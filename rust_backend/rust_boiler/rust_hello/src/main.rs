use lambda_http::{run, service_fn, Body, Error, Request, RequestExt, Response};
use std::env;

// this include is needed to use the current_platform macro
use current_platform::{COMPILED_ON, CURRENT_PLATFORM};

/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Extract some useful information from the request
    let _who = event
        .query_string_parameters_ref()
        .and_then(|params| params.first("name"))
        .unwrap_or("world");

    // Access environment variables
    //const my_variable: &str = env::var("BEN_VARIABLE");

    let message = format!("Hello, world from {}! I was compiled on {}.", CURRENT_PLATFORM, COMPILED_ON);

    println!("Hello, world from {}! I was compiled on {}.", CURRENT_PLATFORM, COMPILED_ON);


    // Access environment variables
    //const my_variable: &str = env::var("BEN_VARIABLE");


    


    // Return something that implements IntoResponse.
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
