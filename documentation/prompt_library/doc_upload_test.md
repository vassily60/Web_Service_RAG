# Doc upload test



We want to test the document upload chain functionality in our Rust backend. 
I am going to describe the chain of events when a document is uploaded, and then we will write tests to ensure everything works as expected.

The upload process involves several steps, including generating a URL for the upload, uploading the document, and then processing it. Upload endpoint in rust_s3_upload_url is responsible for generating a pre-signed URL that the client can use to upload the document to a storage service like S3. The url is then sent back for put operation (upload). Once the the document is uploaded in the first bucket the file rust_pdf_file_integration is responsible for processing the document and moving it to the second bucket (by changing prefix). Processing includes extracting text and splitting the text into chunks from the PDF and storing it in a database. Once it arrives to the second bucket, the file rust_file_vectorization is responsible for vectorizing the chunks and storing the vectors in a database. All these steps are part of the document upload chain.

I want you to create a test that will ensure that the document upload chain works correctly. In the directory test/unified create test file that will make sure that the document upload chain works correctly. The test should include the following steps:
1. Generate a pre-signed URL for uploading a document.
2. Upload a sample document to the pre-signed URL. For the TestDocumentUpload class can you Make sure you retrieve the pdf document from the folder documents under test/unified. It has to be a real pdf document not a made up one. The endpoint will take care of parsing the pdf, just retrieve it and upload it.
3. Trigger the processing of the document by creating a new json s3 event on the model of:
```json
{
  "Records": [
    {
      "eventVersion": "2.1",
      "eventSource": "aws:s3",
      "awsRegion": "ap-southeast-1",
      "eventTime": "2025-07-18T12:34:56.000Z",
      "eventName": "ObjectCreated:Put",
      "userIdentity": {
        "principalId": "EXAMPLE"
      },
      "requestParameters": {
        "sourceIPAddress": "127.0.0.1"
      },
      "responseElements": {
        "x-amz-request-id": "EXAMPLE123456789",
        "x-amz-id-2": "EXAMPLE123/5678abcdefghijklambdaisawesome/mnopqrstuvwxyzABCDEFGH"
      },
      "s3": {
        "s3SchemaVersion": "1.0",
        "configurationId": "testConfigRule",
        "bucket": {
          "name": "paloit-cve",
          "ownerIdentity": {
            "principalId": "EXAMPLE"
          },
          "arn": "arn:aws:s3:::paloit-cve"
        },
        "object": {
          "key": "indexed/93c6a0fc-f689-48e4-ac79-55fafd16c873-_EEEEreceipt.pdf",
          "size": 102400,
          "eTag": "0123456789abcdef0123456789abcdef",
          "sequencer": "0A1B2C3D4E5F678901"
        }
      }
    }
  ]
}
```
Call the rust_pdf_file_integration function with the above event to simulate the processing of the document (http webservice call). This should process the document and move it to the second bucket. Create the s3 event in the event folder with the same name as the uploaded document.  Please ensure the name of the file is the same in the bucket and in the event file. 

4. Next test, verify that the document is processed correctly (i.e., it is moved to the second bucket, text is extracted, and chunks are created). You can call the file rust_get_document_list to check that the file was indeed moved to the database. 

5. For the last test I want you to take the document moved to the second bucket and trigger the lambda function rust_file_vectorization. It should be triggered by a json event following this model:
```json 
{
  "Records": [
    {
      "eventVersion": "2.1",
      "eventSource": "aws:s3",
      "awsRegion": "ap-southeast-1",
      "eventTime": "2025-07-22T14:25:36.000Z",
      "eventName": "ObjectCreated:Put",
      "userIdentity": {
        "principalId": "EXAMPLE"
      },
      "requestParameters": {
        "sourceIPAddress": "127.0.0.1"
      },
      "responseElements": {
        "x-amz-request-id": "EXAMPLE123456789",
        "x-amz-id-2": "EXAMPLE123/5678abcdefghijklambdaisawesome/mnopqrstuvwxyzABCDEFGH"
      },
      "s3": {
        "s3SchemaVersion": "1.0",
        "configurationId": "testConfigRule",
        "bucket": {
          "name": "paloit-cve",
          "ownerIdentity": {
            "principalId": "EXAMPLE"
          },
          "arn": "arn:aws:s3:::paloit-cve"
        },
        "object": {
          "key": "indexed/93c6a0fc-f689-48e4-ac79-55fafd16c873-_EEEEreceipt.pdf",
          "size": 102400,
          "eTag": "0123456789abcdef0123456789abcdef",
          "sequencer": "0A1B2C3D4E5F678901"
        }
      }
    }
  ]
}
```
To retrieve the document in the second bucket, you should have a global variable that stores the processed document name (the one that was moved to the second bucket). The processed key must be retrieve from the upload_details dictionary that was created in the first step of the test.
Create the s3 event in the event2 folder with the same name as the processed document. Ensure the name of the file is the same in the bucket and in the event file.
Call the rust_file_vectorization function with the above event to simulate the vectorization of the document. Verify that the chunks are vectorized and stored in the database. 
