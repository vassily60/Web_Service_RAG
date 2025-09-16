# PDF File Integration Lambda

This Lambda function processes PDF files uploaded to an S3 bucket. It is triggered by S3 events when a new PDF file is uploaded to the source bucket.

## Functionality

1. **Trigger**: S3 event when a PDF is uploaded to the source bucket with the configured prefix.
2. **Processing**:
   - Reads the file from the S3 bucket
   - Computes the MD5 hash of the file
   - Checks if the file already exists in the database by comparing the hash
   - If the file is new:
     - Inserts file metadata into the `documents` table
     - Chunks the file text content (1000 characters with 100 character overlap)
     - Inserts each chunk into the `document_chunks` table
     - Prepares entries for vectorization in the `document_embeding_mistral_generic` table
   - If the file is a duplicate:
     - Updates the existing document record with status 'duplicate'
   - Copies the file to the destination bucket

## Configuration

The Lambda function uses the following environment variables:

- `S3BUCKET_IMPORT_FOLDER`: Source S3 bucket name
- `S3BUCKET_EXPORT_FOLDER`: Destination S3 bucket name
- `SOURCE_PREFIX`: Prefix for source files in the source bucket (default: 'imported/')
- `DESTINATION_PREFIX`: Prefix for processed files in the destination bucket (default: 'processed/')
- `DATABASE_CONECTION_STRING`: Secret name for database connection string

## Testing

You can test this Lambda function using the provided `test_integration.py` script:

```bash
python test_integration.py --bucket your-source-bucket --file path/to/test.pdf
```

## Deployment

The service is configured in the `serverless.yaml` file and can be deployed using:

```bash
# Build the Lambda function
cargo lambda build --release

# Deploy using serverless framework
serverless deploy --stage dev
```

## Tables Schema

The Lambda function interacts with the following database tables:

1. `document_library.documents`: Stores document metadata
2. `document_library.document_chunks`: Stores text chunks from the documents
3. `document_library.document_embeding_mistral_generic`: Stores references to chunks for vectorization
