"""
Test module for OpenAI answer generation API endpoints.

This module tests the functionality of the rust_openai_answer service,
which uses OpenAI to generate answers from document chunks with metadata.
"""
import json
import os
import pytest
import requests

from dotenv import load_dotenv
from shared_function import cognito_connexion

# Load environment variables at module level
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

class TestOpenAIAnswer:
    """Test class for OpenAI answer generation operations"""
    
    def test_generate_answer(self, api_credentials):
        """
        Test OpenAI answer generation with document chunks
        
        This test sends document chunks with metadata along with a question
        to the OpenAI answer API and validates that it returns a proper response
        with an answer and token usage statistics.
        """
        endpoint = api_credentials['get_endpoint']('rust_openai_answer')
        print(f"OpenAI Answer API Endpoint: {endpoint}")
    
    
        # Example chunks based on the Chunk structure in the Rust service
        post_data = {
            "question": "What are the key points in this document?",
            "chunks": [
                {
                "document_uuid": "123e4567-e89b-12d3-a456-426614174000",
                "document_name": "Test Document.pdf",
                "document_location": "s3://documents/test.pdf",
                "document_hash": "abc123",
                "document_type": "pdf",
                "document_status": "active",
                "document_chunk_uuid": "chunk-uuid-123",
                "embebed_text": "This is a test document containing important information about sustainability. The company has reduced carbon emissions by 15% in the past year.",
                "document_embeding_uuid": "embedding-uuid-123",
                "embeder_type": "openai",
                "embedding_token": 128,
                "embedding_time": 0.75,
                "document_metadata": [
                    {
                        "metadata_uuid": "meta-123",
                        "metadata_name": "document_type",
                        "metadata_value_string": "report",
                        "metadata_value_int": None,
                        "metadata_value_float": None,
                        "metadata_value_boolean": None,
                        "metadata_value_date": None
                    },
                    {
                        "metadata_uuid": "meta-124",
                        "metadata_name": "department",
                        "metadata_value_string": "sustainability",
                        "metadata_value_int": None,
                        "metadata_value_float": None,
                        "metadata_value_boolean": None,
                        "metadata_value_date": None
                    }
                ]
            },
            {
                "document_uuid": "223e4567-e89b-12d3-a456-426614174001",
                "document_name": "Test Document.pdf",
                "document_location": "s3://documents/test.pdf",
                "document_hash": "abc123",
                "document_type": "pdf",
                "document_status": "active",
                "document_chunk_uuid": "chunk-uuid-124",
                "embebed_text": "The company has implemented new sustainability initiatives such as renewable energy sources and waste reduction programs.",
                "document_embeding_uuid": "embedding-uuid-124",
                "embeder_type": "openai",
                "embedding_token": 132,
                "embedding_time": 0.78,
                "document_metadata": [
                    {
                        "metadata_uuid": "meta-125",
                        "metadata_name": "document_type",
                        "metadata_value_string": "report",
                        "metadata_value_int": None,
                        "metadata_value_float": None,
                        "metadata_value_boolean": None,
                        "metadata_value_date": None
                    },
                    {
                        "metadata_uuid": "meta-126",
                        "metadata_name": "year",
                        "metadata_value_string": None,
                        "metadata_value_int": 2023,
                        "metadata_value_float": None,
                        "metadata_value_boolean": None,
                        "metadata_value_date": None
                    }
                ]
            }
        ]
        }
        
        # Make the POST request
        response = requests.post(
            endpoint,
            data=json.dumps(post_data),
            headers={
                "Content-Type": "application/json", 
                "Authorization": f"Bearer {api_credentials['token']}", 
                "x-api-key": api_credentials['api_key']
            }
        )
        print(f"Response object: {response}")
        
        print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
        print(f'POST RESPONSE: {response.text}')
    
        # Verify response status code is successful
        assert response.status_code == 200, f"Expected status code 200, got {response.status_code}: {response.reason}"
        
        try:
            response_data = response.json()
            # Verify answer exists in response
            assert 'answer' in response_data, "No answer found in response"
            
            print('OPENAI ANSWER:')
            print(response_data['answer'])
            
            # Extract and verify token usage information
            if 'token_usage' in response_data:
                token_usage = response_data['token_usage']
                print('\nTOKEN USAGE:')
                if 'input_tokens' in token_usage:
                    print(f"Input tokens: {token_usage['input_tokens']}")
                if 'output_tokens' in token_usage:
                    print(f"Output tokens: {token_usage['output_tokens']}")
                if 'total_tokens' in token_usage:
                    print(f"Total tokens: {token_usage['total_tokens']}")
                
                # Add optional assertions for token usage if needed
                # assert 'input_tokens' in token_usage, "No input_tokens in token_usage"
                # assert 'output_tokens' in token_usage, "No output_tokens in token_usage"
                # assert 'total_tokens' in token_usage, "No total_tokens in token_usage"
            else:
                print('No token usage information found in response')

        except json.JSONDecodeError:
            pytest.fail("Failed to parse response as JSON")

    def test_generate_answer_with_invalid_chunks(self, api_credentials):
        """
        Test OpenAI answer generation with invalid document chunks
        
        This test sends an invalid request (no chunks) to the OpenAI answer API
        and validates that it returns a proper error response.
        """
        endpoint = api_credentials['get_endpoint']('rust_openai_answer')
        print(f"OpenAI Answer API Endpoint: {endpoint}")
        
        # Invalid request with missing chunks
        invalid_data = {
            "question": "What are the key points in this document?"
            # Missing 'chunks' field
        }
        
        # Make the POST request
        response = requests.post(
            endpoint,
            data=json.dumps(invalid_data),
            headers={
                "Content-Type": "application/json", 
                "Authorization": f"Bearer {api_credentials['token']}", 
                "x-api-key": api_credentials['api_key']
            }
        )
        
        print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
        print(f'POST RESPONSE: {response.text}')
        
        # We expect a 400 Bad Request or similar error status
        assert response.status_code != 200, "Expected an error status code for invalid request"

    def test_generate_answer_without_auth(self, api_credentials):
        """
        Test OpenAI answer generation without authentication
        
        This test tries to access the API without proper authentication
        to validate security requirements.
        """
        endpoint = api_credentials['get_endpoint']('rust_openai_answer')
        
        # Example chunks based on the Chunk structure in the Rust service
        post_data = {
            "question": "What are the key points in this document?",
            "chunks": []  # Empty chunks for simplicity
        }
        
        # Make the POST request without authentication
        response = requests.post(
            endpoint,
            data=json.dumps(post_data),
            headers={
                "Content-Type": "application/json"
                # Missing authorization headers
            }
        )
        
        print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
        
        # We expect a 401 Unauthorized or 403 Forbidden
        assert response.status_code in [401, 403], f"Expected 401 or 403 status code, got {response.status_code}"



if __name__ == "__main__":
    # This allows the test to be run directly as a script
    # Examples:
    # - Run all tests:
    #   pytest rust_openai_answer.py -v
    # - Run specific test:
    #   pytest rust_openai_answer.py::TestOpenAIAnswer::test_generate_answer -v
    pytest.main(["-v", __file__])
