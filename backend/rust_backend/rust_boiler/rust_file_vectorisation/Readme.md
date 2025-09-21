# File Vectorization Lambda

This Lambda function processes document chunks and generates embeddings using the Ollama API. It is designed to work as part of a document processing pipeline, where it handles the vectorization of text chunks that have been previously extracted from documents.

## Functionality

1. **Trigger**:

   - S3 events when a file is uploaded to the source bucket
   - HTTP API Gateway events that contain S3 event information

2. **Processing Flow**:

   - Detects and parses the incoming event type (Direct S3 or HTTP API Gateway)
   - Locates the corresponding document in the database by:
     - First attempting to match by file name
     - Then attempting to match by file location (path)
   - Retrieves all document chunks associated with the document
   - For each chunk:
     - Verifies if the chunk has text content
     - Checks if an embedding already exists
     - Generates embeddings using the Ollama API if needed
     - Stores the embeddings in the database

3. **Features**:
   - Custom PostgreSQL vector type implementation for storing embeddings
   - Automatic skipping of empty chunks and already processed chunks
   - Performance tracking for embedding generation
   - Detailed logging and error handling
   - Support for both direct S3 events and API Gateway events

## Configuration

The Lambda function requires the following environment variables:

- `DATABASE_CONECTION_STRING`: Secret name for the database connection string
- `OLLAMA_API_URL`: URL for the Ollama API endpoint
- `SOURCE_PREFIX`: Prefix for source files in the source bucket

## Database Schema

The function interacts with the following database tables:

1. `document_library.documents`:

   - Stores document metadata
   - Key fields: document_uuid, document_name, document_location

2. `document_library.document_chunks`:

   - Stores text chunks from documents
   - Key fields: document_chunk_uuid, document_uuid, embebed_text

3. `document_library.document_embeding_mistral_generic`:
   - Stores embeddings for document chunks
   - Key fields: document_embeding_uuid, document_chunk_uuid, embedding

## Deployment

The service is configured in the `serverless.yaml` file. Deploy using:

```bash
# Build the Lambda function
cargo lambda build --release

# Deploy using serverless framework
serverless deploy --stage dev
```

## Response Format

The Lambda function returns a JSON response with the following structure:

```json
{
    "statusCode": 200,
    "body": {
        "message": "Vectorization process completed",
        "document_uuid": "uuid",
        "processed_chunks": number,
        "skipped_chunks": number,
        "total_chunks": number,
        "processing_time_seconds": number
    }
}
```

## Error Handling

The function includes comprehensive error handling for:

- Invalid event formats
- Database connection issues
- Missing documents
- Ollama API failures
- Embedding storage failures

All errors are logged with detailed messages for troubleshooting.

## Performance

The function tracks:

- Total processing time
- Individual embedding generation time
- Number of chunks processed vs skipped
- Database operation timing

## Dependencies

Main dependencies include:

- `lambda_runtime`: AWS Lambda runtime support
- `tokio`: Async runtime
- `aws-sdk`: AWS service integrations
- `tokio-postgres`: PostgreSQL database client
- `reqwest`: HTTP client for Ollama API
- `serde`: JSON serialization/deserialization
