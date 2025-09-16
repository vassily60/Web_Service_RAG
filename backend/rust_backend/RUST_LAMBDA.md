# OBJECTIVES
Deploy end to end process to AWS Lambda using Rust.

# Pre requisites
- AWS account
- AWS CLI
- Serverless framework
- Rust
- Cargo lambda


# Installation

## Cross compilation from M1 to 
https://github.com/messense/homebrew-macos-cross-toolchains

## Serverless Rust
```bash
brew install zig
brew tap cargo-lambda/cargo-lambda
brew install cargo-lambda
```

# Documentation

Cargo Lambda:
https://www.cargo-lambda.info/guide/getting-started.html


# Steps
- Create a new project
- Add the gitignore file
- Create a new lambda using cargo lambda
- Add serverless.yml file at the root of the project
- Add a Cargo.toml file at the root of the project
- Test locally
- Deploy the lambda using serverless framework
- Modify the Lambda and make the loop

# Detailed Steps

## Create a new project
Create a new directory in the folder of your choice


## Add the gitignore file
```.gitignore
node_modules/
target/
```

## Prepare path
export PATH="$HOME/.cargo/bin:$PATH" 

CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-unknown-linux-gnu-gcc cargo build --target=x86_64-unknown-linux-gnu

## Create a new lambda using cargo lambda
```bash
cargo lambda new hello
```

## Add serverless.yml file at the root of the project
```yaml
#
# Defines the project architecture for deployment ("Infrastructure as Code").
#

# The name of the service.
service: rust-lambda



# To automate building and deployment, we'll use a plugin specifically made for Rust.
plugins:


custom:
    myStage: ${opt:stage, self:provider.stage}
    myEnvironment:
        COGNITO_SECRET:
            prod: "cognito_sustainable_platform_dev"
            dev: "cognito_sustainable_platform_dev"
        COGNITO_ARN:
            prod: arn:aws:cognito-idp:ap-southeast-1:781857564217:userpool/ap-southeast-1_AgosmHdU4
            dev: arn:aws:cognito-idp:ap-southeast-1:781857564217:userpool/ap-southeast-1_AgosmHdU4
        ROLE_ARN:
            prod: arn:aws:iam::781857564217:role/aws_lambda_role_api_gateway
            dev: arn:aws:iam::781857564217:role/aws_lambda_role_api_gateway
        DATABASE:
            prod: "INNOVATION"
            dev: "INNOVATION"
        APIKEY:
            prod: "TINY-RUST-BACKEND-PROD"
            dev: "TINY-RUST-BACKEND-DEV"
        S3BUCKET_IMPORT_FOLDER:
            prod: "palo-hk-nicomatic-demo-blockchain-prod"
            dev: "palo-hk-nicomatic-demo-blockchain-dev"
        S3BUCKET_EXPORT_FOLDER:
            prod: "nicomatic-demo-blochchain"
            dev: "nicomatic-demo-blochchain"
        DATABASE_CONECTION_STRING:
            prod: demo_connexion_nicomatic_secret_manager_prod
            dev: demo_connexion_nicomatic_secret_manager_dev
        REGION:
            prod: ap-southeast-1
            dev: ap-southeast-1
        RUST_LOG:
            prod: "debug  prod"
            dev: "debug dev"


# This tells the framework to package each function separately with its own container.
package:
    individually: true


# We want to host the project on AWS using Rust.
provider:
    name: aws
    runtime: provided.al2023
    stage: dev
    region: ${self:custom.myEnvironment.REGION.${self:custom.myStage}}
    #versionFunctions: false #https://github.com/serverless/serverless/issues/3696
    environment:
      DATABASE_CONECTION_STRING: ${self:custom.myEnvironment.DATABASE_CONECTION_STRING.${self:custom.myStage}}
      DATABASE: ${self:custom.myEnvironment.DATABASE.${self:custom.myStage}}
      REGION: ${self:custom.myEnvironment.REGION.${self:custom.myStage}}
      COGNITO_SECRET: ${self:custom.myEnvironment.COGNITO_SECRET.${self:custom.myStage}}
      S3BUCKET_IMPORT_FOLDER: ${self:custom.myEnvironment.S3BUCKET_IMPORT_FOLDER.${self:custom.myStage}}
      S3BUCKET_EXPORT_FOLDER: ${self:custom.myEnvironment.S3BUCKET_EXPORT_FOLDER.${self:custom.myStage}}
      PREFIX_ENVIRONMENT_LAMBDA: ${self:service}-${self:custom.myStage}-
      RUST_LOG: ${self:custom.myEnvironment.RUST_LOG.${self:custom.myStage}}
    apiGateway:
        apiKeys:
        - ${self:custom.myEnvironment.APIKEY.${self:custom.myStage}}
        usagePlan:
        #quota:
            #limit: 100000
            #offset: 0
            #period: MONTH
        # throttle:
        #   burstLimit: 500
        #   rateLimit: 400


# Here is where we define all our functions (each living in a separate Cargo crate).
functions:
    # In this project, we only have one lambda function called `lucky_numbers`.
    hello:
        # The name of the handler must match the name of the crate (not the actual function defined within).
        handler: hello
        
        role: ${self:custom.myEnvironment.ROLE_ARN.${self:custom.myStage}}
        package:
            # build with `cargo lambda build --output-format zip` and then fill in the build path. This is the default for `src/bin/hello_world.rs`
            artifact: target/lambda/hello/hello_bootstrap.zip
        # This tells AWS when to trigger our function.
        events:
            # The event we're interested in is an incoming HTTP request.
            - http:
                  # The function will be called for every POST request that is made
                  # onto the `/lucky_numbers` endpoint.
                  path: /hello
                  method: POST
```

## Add a Cargo.toml file at the root of the project
```toml
[workspace]
members = ["hello"]
```

## Test locally
Call the lambda listener
```bash
cargo lambda watch
cargo lambda invoke hello --data-example apigw-request
```

## Deploy the lambda using serverless framework
```bash
sls deploy
```
You could need to force Open SSL to be vendored: so in the Cargo.toml file of the lambda, add:
```toml
openssl = { version = "0.10", features = ["vendored"] }
```

## Modify the Lambda and make the loop
```bash
cargo add current_platform
```

In the main:

```rust
use current_platform::{COMPILED_ON, CURRENT_PLATFORM};

// then in the main: 
println!("Hello, world from {}! I was compiled on {}.", CURRENT_PLATFORM, COMPILED_ON);

// you can also update the return
let message = format!("Hello, world from {}! I was compiled on {}.", CURRENT_PLATFORM, COMPILED_ON);
```

## Call the lambda
```bash
curl -X POST \
     -H "Content-Type: application/json" \
     -d '{"name":"SilentByte","count": 10}' \
     https://<your-lambda-url>/hello
```

## Call the in the browser or python or ....
```python
    PostAddressGetInfoElement = 'http://localhost:9000/lambda-url/<lambda_name>'

```

# DEPLOY ON AWS

## COMPILE
```shell
cargo lambda build --output-format zip
```
or in release mode
```shell
cargo lambda build --release --output-format zip
```

## RUN THE PRE DEPLOYMENT PROCESS
We must rename the bootstrap files accordingly with the lambda function name.
Run the python scipt named predeployement.py
```shell
python3  predeployement.py
```

## AND THEN DEPLOY
```shell
sls deploy
```
or
```shell
sls deploy function --function <function_name>
```

## SNOWFLAKE AND OTHER PACKAGES

https://crates.io/crates/snowflake-connector-rs


## TEST
There are automated test or manual debug capabilities. Use the right directory to run the test.

You can switch to loacl or remote mode by changing the flag in the config.py file.


### TEST MANUAL
Use the associated python function to test the lambda function.


### TEST AUTOMATION
```shell
pytest

or

pytest test_rust_dynamo.py --log-cli-level=DEBUG
```

### Regular update
At the root of your project, run the following command to update the dependencies:
```shell
cargo update
```

# Compilation for AWS Lambda before deploying in AWS

## Overview
The AWS Lambda runtime environment is based on Amazon Linux 2, which is a Linux distribution that is not compatible with the macOS M1 architecture. This means that the Rust code must be compiled for the correct architecture before it can be deployed to AWS Lambda.

Some library are missing on Mac (see linker issue for PostgreSQL) so we need to compile as a Linux binary.

## Strategy
Create a Docker container that will compile the Rust code for the correct architecture. This container will be based on the Amazon Linux 2 image, which is the same environment that AWS Lambda uses.

## Steps
1. Create a Dockerfile that will build the Rust code.
2. Build the Docker image.
3. Run the Docker container to compile the Rust code.
4. Retrieve the compiled binary from the Docker container.

### 1. Create a Dockerfile
Create a file named `Dockerfile` in the root of the project with the following content:

```Dockerfile
# Use the Amazon Linux 2 image as the base image
FROM ubuntu:22.04

# Prevents interactive installation dialogs
ENV DEBIAN_FRONTEND=noninteractive

# Install essential packages and dependencies
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        curl \
        pkg-config \
        python3 \
        python3-pip \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install Rust using rustup
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y \
    && . "$HOME/.cargo/env"

# Add Rust to PATH
ENV PATH="/root/.cargo/bin:${PATH}"

# Install cargo-lambda
RUN cargo install cargo-lambda

# Set working directory inside container
WORKDIR /rust_back_end

# Default command launch bash shell
CMD ["bash"]
```

### 2. Build the Docker image
Run the following command to build the Docker image:

Find the libpq.so file on the host machine
```shell
find / -name libpq.so
```
Hack like a pig the linker
```shell
cp /usr/lib/x86_64-linux-gnu/libpq.so* /root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/
```

```shell
podman machine start
podman build --platform linux/amd64 -t rust-lambda .
```

### 3. Run the Docker container
Run the following command to start the Docker container:

```shell
podman run -it \
    --mount type=bind,source="$(pwd)/rust_boiler",target=/rust_back_end \
    rust-lambda
```