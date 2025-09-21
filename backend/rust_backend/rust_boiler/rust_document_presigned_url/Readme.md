# Document Presigned URL Lambda

This Lambda function generates secure, time-limited presigned URLs for accessing documents stored in S3. It includes authentication via Cognito tokens and database validation to ensure proper access control.

## Functionality

1. **Authentication**:

   - Validates AWS Cognito JWT tokens from the Authorization header
   - Extracts user email from token claims
   - Ensures proper authorization before generating URLs

2. **Processing Flow**:

   - Receives HTTP requests with document UUID and optional expiration time
   - Validates the request format and authentication
   - Queries the database to verify document existence and retrieve location
   - Generates a presigned URL for secure S3 object access
   - Returns the URL along with document metadata

3. **Features**:
   - Configurable URL expiration time (default: 15 minutes)
   - Secure database connections with TLS
   - Comprehensive error handling and logging
   - AWS Cognito integration for authentication
   - Custom response formatting

## Configuration

The Lambda function requires the following environment variables:

- `DATABASE_CONECTION_STRING`: Secret name for database connection string
- `S3BUCKET_IMPORT_FOLDER`: Name of the S3 bucket containing documents
- `COGNITO_REGION`: AWS region for Cognito
- `COGNITO_USER_POOL_ID`: Cognito user pool ID
- `COGNITO_CLIENT_ID`: Cognito client ID

## API Request Format

```json
{
    "document_uuid": "string",
    "expiration": number  // Optional, defaults to 900 seconds (15 minutes)
}
```

## API Response Format

Success Response (200):

```json
{
    "status": "success",
    "presigned_url": "string",
    "document_name": "string",
    "expiration": number
}
```

Error Responses:

- 400: Bad Request (Invalid JSON)
- 401: Unauthorized (Invalid token)
- 404: Document Not Found
- 500: Server Error (Database/S3 issues)

## Database Schema

The function interacts with the following database table:

`document_library.documents`:

- `document_uuid`: Unique identifier for the document
- `document_name`: Name of the document
- `document_location`: S3 object key/location

## Security Features

1. **Authentication**:

   - JWT token validation
   - Cognito user pool integration
   - Token claims verification

2. **Database Security**:

   - TLS encrypted connections
   - Secrets manager for credentials
   - Parameterized queries

3. **S3 Security**:
   - Time-limited presigned URLs
   - No direct S3 bucket access
   - Secure URL generation

## Error Handling

The function includes comprehensive error handling for:

- Invalid/missing authentication tokens
- Malformed request bodies
- Database connection issues
- Document not found scenarios
- S3 presigning failures

All errors are logged with appropriate HTTP status codes and descriptive messages.

## Dependencies

Main dependencies include:

- `lambda_http`: AWS Lambda HTTP runtime
- `jsonwebtokens-cognito`: Cognito JWT validation
- `aws-sdk-s3`: S3 operations and presigning
- `aws-sdk-secretsmanager`: Secrets management
- `tokio-postgres`: PostgreSQL database client
- `serde`: JSON serialization/deserialization

## Deployment

The service is configured in the `serverless.yaml` file. Deploy using:

```bash
# Build the Lambda function
cargo lambda build --release

# Deploy using serverless framework
serverless deploy --stage dev
```
