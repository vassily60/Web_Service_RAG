# Lambda Function URL Local Testing Scripts

These scripts help you test AWS Lambda functions locally when they're expecting S3 events but being invoked through Function URLs.

## Problem

When invoking a Lambda function locally using `curl` against a Lambda Function URL, the event structure is different from what the Lambda receives when triggered directly by S3. This can lead to deserialization errors like:

```json
{"errorType":"&lambda_runtime::deserializer::DeserializeError","errorMessage":"failed to deserialize the incoming data into the function's payload type: missing field `Records` at line 1 column 1394\n"}
```

## Solution

These scripts transform your S3 event JSON into the format expected by Lambda Function URLs.

## Available Scripts

1. `create_function_url_payload.js` - Transforms your S3 event into a Lambda Function URL event
2. `invoke_lambda_local.sh` - Simple script to invoke the Lambda with the formatted event
3. `test_lambda_formats_all.sh` - Comprehensive script that tries multiple event formats to find which one works

## How to Use

1. First, ensure your Lambda function is running locally:

   ```bash
   # In your Lambda project directory
   cargo lambda watch
   # OR
   cargo lambda start -p 9000
   ```

2. Run the wrapper script to create the formatted event:

   ```bash
   node create_function_url_payload.js
   ```

3. Invoke your Lambda with the formatted event:

   ```bash
   ./invoke_lambda_local.sh
   ```

4. If you're still having issues, try the comprehensive test script:

   ```bash
   ./test_lambda_formats_all.sh
   ```

   This will try multiple formats and show you which one works.

## Event Format Reference

The Lambda Function URL event format looks like this:

```json
{
  "version": "2.0",
  "routeKey": "$default",
  "rawPath": "/path",
  "headers": {
    "content-type": "application/json"
  },
  "requestContext": {
    "accountId": "123456789012",
    "apiId": "api-id",
    "domainName": "localhost:9000",
    "http": {
      "method": "POST",
      "path": "/path",
      "protocol": "HTTP/1.1",
      "sourceIp": "127.0.0.1"
    },
    "requestId": "request-id",
    "stage": "$default",
    "time": "2023-07-18T15:13:33.055Z",
    "timeEpoch": 1689699213055
  },
  "body": "{\n  \"Records\": [...] }",
  "isBase64Encoded": false
}
```

## Modifying Your Lambda Code

If you'd prefer to modify your Lambda code to handle both S3 events and Function URL events, check out the `updated_lambda_handler.rs` file for an example implementation.
