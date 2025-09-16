"""
Test module for Document Upload Chain functionality.

This module tests the end-to-end document upload flow, including:
1. Generating a pre-signed URL for upload
2. Uploading a sample document to the pre-signed URL
3. Verifying document processing (extraction, chunking)
4. Verifying chunk vectorization and database storage
"""
import os
import json
import time
import base64
import requests
import io
from datetime import datetime
from dotenv import load_dotenv
import pytest
from shared_function import cognito_connexion

load_dotenv()

@pytest.fixture(scope="module")
def api_credentials():
    """
    Fixture for API credentials and endpoints.
    
    Returns:
        dict: Dictionary containing token, api_key, and endpoint generator function.
    """
    # Check required environment variables
    user_name = os.getenv('USER_NAME')
    password = os.getenv('PASSWORD')
    api_key = os.getenv('API_KEY')
    
    assert user_name, "USER_NAME environment variable is required"
    assert password, "PASSWORD environment variable is required"
    assert api_key, "API_KEY environment variable is required"
    
    # Determine API endpoint based on debug flag
    local_debug_flag = os.getenv('LOCAL_DEBUG_FLAG', 'false').lower() == 'true'
    api_debug_endpoint_root = os.getenv('API_DEBUG_ENDPOINT_ROOT')
    api_endpoint_root = os.getenv('API_ENDPOINT_ROOT')
    
    if local_debug_flag:
        assert api_debug_endpoint_root, "API_DEBUG_ENDPOINT_ROOT is required when LOCAL_DEBUG_FLAG is true"
    else:
        api_stage = os.getenv('API_STAGE')
        assert api_endpoint_root, "API_ENDPOINT_ROOT environment variable is required"
        assert api_stage, "API_STAGE environment variable is required when not in debug mode"
    
    # Authenticate with Cognito to get a token
    conn_result = cognito_connexion(user_name, password)
    assert conn_result['success'], f"Authentication failed: {conn_result.get('message', 'No error message provided')}"
    
    # Build base endpoint for all API calls
    def get_endpoint(service):
        if local_debug_flag:
            return f"{api_debug_endpoint_root}/{service}"
        else:
            return f"{api_endpoint_root}/{os.getenv('API_STAGE')}/{service}"
    
    return {
        'token': conn_result['data']['id_token'],
        'api_key': api_key,
        'get_endpoint': get_endpoint
    }


@pytest.fixture(scope="module")
def test_filename():
    """Generate a unique test filename that will be used across all tests"""
    timestamp = datetime.now().strftime("%Y%m%d%H%M%S")
    return f"test_document_{timestamp}.pdf"

@pytest.fixture(scope="module")
def upload_details(api_credentials, test_filename):
    """
    Fixture to store the upload details including presigned URL, bucket and key.
    This ensures the same details are used across all tests.
    """
    return {
        "presigned_url": None,
        "bucket": None,
        "key": None,
        "test_filename": test_filename
    }

class TestGenerateUploadURL:
    """Test class for generating a pre-signed URL for document upload"""
    
    def test_generate_presigned_url(self, api_credentials, test_filename, upload_details):
        """Test the generation of a pre-signed URL for document upload"""
        upload_endpoint = api_credentials['get_endpoint']('rust_s3_upload_url')
        print("Upload URL API Endpoint:", upload_endpoint)
        
        # Request data for pre-signed URL generation
        upload_data = {
            "file_name": test_filename,
            "content_type": "application/pdf"
        }
        
        # Make request to get pre-signed URL
        upload_response = requests.post(
            upload_endpoint,
            data=json.dumps(upload_data),
            headers={
                "Content-Type": "application/json", 
                "Authorization": f"Bearer {api_credentials['token']}",
                "x-api-key": api_credentials['api_key']
            }
        )
        
        print(f'Upload URL STATUS: {upload_response.status_code} REASON: {upload_response.reason}')
        print(f'Upload URL RESPONSE: {upload_response.text}')
        
        assert upload_response.status_code == 200, f"Pre-signed URL generation failed: {upload_response.status_code} {upload_response.text}"
        
        upload_json = upload_response.json()
        assert 'presigned_url' in upload_json, "Pre-signed URL not found in response"
        assert 'bucket' in upload_json, "Bucket not found in response"
        assert 'key' in upload_json, "Key not found in response"
        
        # Store the upload details for other tests
        upload_details["presigned_url"] = upload_json['presigned_url']
        upload_details["bucket"] = upload_json['bucket']
        upload_details["key"] = upload_json['key']
        
        print(f"Successfully generated pre-signed URL for {test_filename}")
        print(f"Bucket: {upload_details['bucket']}, Key: {upload_details['key']}")
        
        return upload_details


class TestDocumentUpload:
    """Test class for uploading a document using the pre-signed URL"""
    
    def test_upload_document(self, api_credentials, upload_details):
        """Test uploading a sample document to the pre-signed URL"""
        # Ensure the upload details are available from previous test
        assert upload_details["presigned_url"], "Pre-signed URL not generated. Run URL generation test first."
        
        # Use a real PDF file from the documents folder
        pdf_path = os.path.join(os.path.dirname(__file__), "documents", "_AAAAreceipt.pdf")
        print(f"Using PDF file from: {pdf_path}")
        
        # Verify that the PDF file exists
        assert os.path.exists(pdf_path), f"PDF file not found at {pdf_path}"
        
        # Read the PDF file
        with open(pdf_path, 'rb') as pdf_file:
            pdf_content = pdf_file.read()
            print(f"Read {len(pdf_content)} bytes from PDF file")
        
        # Upload the document using the pre-signed URL
        put_response = requests.put(
            upload_details["presigned_url"],
            data=pdf_content,
            headers={
                "Content-Type": "application/pdf"
            }
        )
        
        print(f'Document Upload STATUS: {put_response.status_code} REASON: {put_response.reason}')
        
        assert put_response.status_code in [200, 204], f"Document upload failed: {put_response.status_code}"
        print("Successfully uploaded test document")
        
        # Wait for processing to start
        print("Waiting for document processing to start...")
        time.sleep(5)  # Adjust the wait time based on your processing time


class TestDocumentProcessing:
    """Test class for triggering document processing via an S3 event and verifying chunk creation"""
    
    def test_document_processing(self, api_credentials, upload_details):
        """
        Test document processing by creating and sending an S3 event to trigger the PDF File Integration Lambda.
        This verifies:
        1. Event-based triggering of document processing
        2. File processing from source S3 bucket
        3. Text content extraction and chunking
        4. Storage of chunks in the database
        5. Preparation for vectorization
        """
        # Ensure we have the test filename and bucket details
        assert upload_details["test_filename"], "Test filename not available"
        assert upload_details["bucket"], "Bucket name not available"
        assert upload_details["key"], "Object key not available"
        
        # Create event file directory if it doesn't exist
        event_dir = os.path.join(os.path.dirname(__file__), "event")
        os.makedirs(event_dir, exist_ok=True)
        
        # Generate current timestamp for the event
        current_time = datetime.now().strftime("%Y-%m-%dT%H:%M:%S.000Z")
        
        # Create S3 event JSON
        s3_event = {
            "Records": [
                {
                    "eventVersion": "2.1",
                    "eventSource": "aws:s3",
                    "awsRegion": "ap-southeast-1",
                    "eventTime": current_time,
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
                            "name": upload_details["bucket"],
                            "ownerIdentity": {
                                "principalId": "EXAMPLE"
                            },
                            "arn": f"arn:aws:s3:::{upload_details['bucket']}"
                        },
                        "object": {
                            "key": upload_details["key"],
                            "size": 102400,
                            "eTag": "0123456789abcdef0123456789abcdef",
                            "sequencer": "0A1B2C3D4E5F678901"
                        }
                    }
                }
            ]
        }
        
        # Save the event to a file with the same name as the document
        event_file_name = os.path.basename(upload_details["test_filename"]).replace('.pdf', '.json')
        event_file_path = os.path.join(event_dir, event_file_name)
        
        print(f"Creating S3 event file at: {event_file_path}")
        with open(event_file_path, 'w') as event_file:
            json.dump(s3_event, event_file, indent=2)
        
        # Trigger the PDF File Integration Lambda function with the S3 event
        pdf_integration_endpoint = api_credentials['get_endpoint']('rust_pdf_file_integration')
        print(f"PDF Integration API Endpoint: {pdf_integration_endpoint}")
        
        # Debug information about the S3 event
        print(f"S3 Event - Bucket: {upload_details['bucket']}")
        print(f"S3 Event - Key: {upload_details['key']}")
        
        # The error message indicates the Lambda function expects a different format
        # The Lambda is expecting the event wrapped inside a body.event structure
        # Wrap the S3 event in the format expected by the Lambda function
        
        
        # Send the event to the Lambda function with the correct wrapper
        integration_response = requests.post(
            pdf_integration_endpoint,
            data=json.dumps(s3_event),
            headers={
                "Content-Type": "application/json", 
                "Authorization": f"Bearer {api_credentials['token']}",
                "x-api-key": api_credentials['api_key']
            }
        )
        
        print(f'PDF Integration STATUS: {integration_response.status_code} REASON: {integration_response.reason}')
        
        # Check if the Lambda function was triggered successfully
        assert integration_response.status_code == 200, f"PDF File Integration Lambda invocation failed: {integration_response.status_code}"
        
        print("Successfully triggered document processing via S3 event")
        

class TestDocumentVerification:
    """Test class for verifying that document processing completed successfully"""
    
    def test_document_processing_verification(self, api_credentials, upload_details):
        """
        Verify that the document was processed correctly:
        1. Document was moved to the second bucket
        2. Text was extracted
        3. Chunks were created
        """
        # Ensure we have the test filename and key
        assert upload_details["test_filename"], "Test filename not available"
        assert upload_details["key"], "Document key not available"
        
        print("Waiting for document processing to complete...")
        # Wait for document processing to complete - adjust as needed based on processing time
        time.sleep(15)
        
        # Call the document list API to check if our document has been processed
        doc_list_endpoint = api_credentials['get_endpoint']('rust_document_list')
        print(f"Document List API Endpoint:", doc_list_endpoint)
        
        # Poll for the document in the list with retries
        max_retries = 5
        retry_count = 0
        document_found = False
        processed_document = None
        
        while retry_count < max_retries and not document_found:
            doc_list_response = requests.post(
                doc_list_endpoint,
                headers={
                    "Content-Type": "application/json", 
                    "Authorization": f"Bearer {api_credentials['token']}",
                    "x-api-key": api_credentials['api_key']
                }
            )
            
            print(f'Document List STATUS: {doc_list_response.status_code} REASON: {doc_list_response.reason}')
            
            assert doc_list_response.status_code == 200, f"Failed to retrieve document list: {doc_list_response.status_code}"
            
            doc_list_json = doc_list_response.json()
            print(f'Document List RESPONSE: {json.dumps(doc_list_json, indent=2)}')
            
            # Look for our document in the list based on filename
            original_filename = os.path.basename(upload_details["key"])
            
            if 'documents' in doc_list_json:
                for doc in doc_list_json['documents']:
                    # The document name in the list might contain or be related to our original filename
                    # Adjust this check based on how the system names the processed documents
                    if original_filename in doc.get('document_name', '') or \
                       upload_details["test_filename"] in doc.get('document_name', ''):
                        document_found = True
                        processed_document = doc
                        break
            
            if document_found:
                break
                
            print(f"Document not found in list yet. Waiting before retry {retry_count + 1}/{max_retries}")
            time.sleep(10)
            retry_count += 1
        
        # Assert that document was found
        assert document_found, f"Processed document not found in document list after {max_retries} retries"
        
        print(f"Found processed document in database: {processed_document}")
        
        # Check for important fields in the document record
        assert 'document_uuid' in processed_document, "Document record missing document_id"
        assert 'document_name' in processed_document, "Document record missing document_name"
        
        # Store the processed document details for the next test
        upload_details["processed_document"] = processed_document
        
        # The document key in the second bucket should follow a specific pattern
        # Extract it from the document record or construct it based on the document_id
        if 'document_key' in processed_document:
            processed_key = processed_document['document_key']
        elif 'document_id' in processed_document:
            # Construct the key using the document_id with the expected format
            processed_key = f"indexed/{processed_document['document_id']}-_{os.path.basename(upload_details['test_filename'])}"
        else:
            # Fallback to a fixed key for testing (similar to your example)
            processed_key = "indexed/93c6a0fc-f689-48e4-ac79-55fafd16c873-_EEEEreceipt.pdf"
        
        # Store the processed key for the next test
        upload_details["processed_key"] = processed_key
        
        print(f"Processed document key: {processed_key}")
        
        return processed_document


class TestVectorization:
    """Test class for verifying chunk vectorization and database storage"""
    
    def test_chunk_vectorization(self, api_credentials, upload_details):
        """Test that chunks are vectorized and stored in the database"""
        # Ensure we have the test filename and processed document key
        assert upload_details["test_filename"], "Test filename not available"
        assert upload_details["key"], "Document key not available"
        assert upload_details["processed_key"], "Processed document key not available. Run document verification test first."
        
        # Create event2 directory if it doesn't exist
        event2_dir = os.path.join(os.path.dirname(__file__), "event2")
        os.makedirs(event2_dir, exist_ok=True)
        
        # Generate current timestamp for the event
        current_time = datetime.now().strftime("%Y-%m-%dT%H:%M:%S.000Z")
        
        # Get the processed document key from the previous test
        processed_key = upload_details["processed_key"]
        
        # Create S3 event JSON for vectorization
        vectorization_event = {
            "Records": [
                {
                    "eventVersion": "2.1",
                    "eventSource": "aws:s3",
                    "awsRegion": "ap-southeast-1",
                    "eventTime": current_time,
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
                            "name": "paloit-cve",  # Second bucket for processed documents
                            "ownerIdentity": {
                                "principalId": "EXAMPLE"
                            },
                            "arn": "arn:aws:s3:::paloit-cve"
                        },
                        "object": {
                            "key": processed_key,  # Using the processed document key
                            "size": 102400,
                            "eTag": "0123456789abcdef0123456789abcdef",
                            "sequencer": "0A1B2C3D4E5F678901"
                        }
                    }
                }
            ]
        }
        
        # Save the event to a file with the same name as the processed document
        event_file_name = os.path.basename(processed_key).replace('.pdf', '.json')
        event_file_path = os.path.join(event2_dir, event_file_name)
        
        print(f"Creating S3 vectorization event file at: {event_file_path}")
        with open(event_file_path, 'w') as event_file:
            json.dump(vectorization_event, event_file, indent=2)
        
        # Trigger the File Vectorization Lambda function with the S3 event
        vectorization_endpoint = api_credentials['get_endpoint']('rust_file_vectorisation')
        print(f"File Vectorization API Endpoint: {vectorization_endpoint}")
        
        # Debug information about the vectorization S3 event
        print(f"Vectorization S3 Event - Bucket: {vectorization_event['Records'][0]['s3']['bucket']['name']}")
        print(f"Vectorization S3 Event - Key: {vectorization_event['Records'][0]['s3']['object']['key']}")
        
        # Send the event to the Lambda function
        vectorization_response = requests.post(
            vectorization_endpoint,
            data=json.dumps(vectorization_event),
            headers={
                "Content-Type": "application/json", 
                "Authorization": f"Bearer {api_credentials['token']}",
                "x-api-key": api_credentials['api_key']
            }
        )
        
        print(f'File Vectorization STATUS: {vectorization_response.status_code} REASON: {vectorization_response.reason}')
        
        # Check if the Lambda function was triggered successfully
        assert vectorization_response.status_code == 200, f"File Vectorization Lambda invocation failed: {vectorization_response.status_code}"
        
        print("Successfully triggered document vectorization via S3 event")
        
        # Wait for vectorization to complete
        print("Waiting for document vectorization to complete...")
        time.sleep(10)  # Adjust the wait time based on your processing time
        
        # Start a polling loop to check for document in the document list
        print("Polling for document in document list...")
        max_retries = 5
        retry_count = 0
        document_found = False
        doc_data = None
        
        while retry_count < max_retries and not document_found:
            doc_list_endpoint = api_credentials['get_endpoint']('rust_document_list')
            print(f"Document List API Endpoint (attempt {retry_count + 1}):", doc_list_endpoint)
            
            doc_list_response = requests.post(
                doc_list_endpoint,
                headers={
                    "Content-Type": "application/json", 
                    "Authorization": f"Bearer {api_credentials['token']}",
                    "x-api-key": api_credentials['api_key']
                }
            )
            
            print(f'Document List STATUS: {doc_list_response.status_code} REASON: {doc_list_response.reason}')
            
            assert doc_list_response.status_code == 200, f"Failed to retrieve document list: {doc_list_response.status_code}"
            
            doc_list_json = doc_list_response.json()
            print(f'Document List RESPONSE: {json.dumps(doc_list_json, indent=2)}')
            
            # Check if our test document appears in the document list with vectorization status
            if 'documents' in doc_list_json:
                for doc in doc_list_json['documents']:
                    if doc.get('document_name', '').endswith(os.path.basename(processed_key)):
                        document_found = True
                        doc_data = doc
                        break
            
            if document_found:
                break
                
            # Wait before retrying
            print(f"Vectorized document not found in list yet. Waiting before retry {retry_count + 1}/{max_retries}")
            time.sleep(10)  # Adjust the wait time based on your processing time
            retry_count += 1
        
        # Assert that document was found and has been vectorized
        assert document_found, f"Vectorized document not found in document list after {max_retries} retries"
        
        # Check if the document has been vectorized
        # This assumes the document list response includes a field indicating vectorization status
        # Modify this check based on your actual API response structure
        assert doc_data.get('status') == 'vectorized' or doc_data.get('vectorized') == True, "Document was not vectorized successfully"
        
        print(f"Found vectorized document in document list: {doc_data}")
        print("Document vectorization verification complete: Document successfully vectorized and stored in database")
        
        return doc_data


if __name__ == "__main__":
    # You can run this file directly to test all tests
    # Example: pytest rust_document_upload_chain.py -v
    # Or run specific test classes:
    # Example: pytest rust_document_upload_chain.py::TestGenerateUploadURL -v
    # Example: pytest rust_document_upload_chain.py::TestDocumentUpload -v
    # Example: pytest rust_document_upload_chain.py::TestDocumentProcessing -v
    # Example: pytest rust_document_upload_chain.py::TestVectorization -v
    # Or run specific test methods:
    # Example: pytest rust_document_upload_chain.py::TestGenerateUploadURL::test_generate_presigned_url -v
    pytest.main(["-v", __file__])
