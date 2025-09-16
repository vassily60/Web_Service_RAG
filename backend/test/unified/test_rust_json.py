"""
Test module for JSON schema validation API endpoints.

This module tests the functionality of the rust_json service
which validates JSON data against a predefined schema.
"""
import os
import json
import requests
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


class TestJsonValidation:
    """Test class for JSON validation operations"""
    
    def test_valid_json_schema(self, api_credentials):
        """Test valid JSON data against schema"""
        endpoint = api_credentials['get_endpoint']('rust_json')
        print("JSON API Endpoint:", endpoint)
        
        # Valid test data
        test_client_id = int(os.getenv('TEST_CLIENT_ID', '22'))
        valid_data = {'client_id': test_client_id, 'name': 'Test User'}
        
        # Make the request with valid data
        response = requests.post(
            endpoint,
            data=json.dumps(valid_data),
            headers={
                "Content-Type": "application/json", 
                "Authorization": f"Bearer {api_credentials['token']}",
                "x-api-key": api_credentials['api_key']
            }
        )
        
        print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
        print(f'POST RESPONSE: {response.text}')
        
        # Validate response
        assert response.status_code == 200, f"Valid JSON test failed: {response.status_code} {response.text}"
        response_json = response.json()
        assert response_json['status'] == 'success', "Expected success status for valid JSON"
        assert response_json['schema_validation'] is True, "Schema validation should be true for valid JSON"
    
    def test_invalid_json_schema(self, api_credentials):
        """Test invalid JSON data against schema"""
        endpoint = api_credentials['get_endpoint']('rust_json')
        
        # Invalid test data - missing required 'name' field according to schema
        invalid_data = {'client_id': int(os.getenv('TEST_CLIENT_ID', '22')), 'age': 'thirty'}
        
        # Make the request with invalid data
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
        
        # Even though the schema validation fails, the API should still return 200
        # but indicate the failure in the response body
        assert response.status_code == 200, f"Invalid JSON test unexpected status: {response.status_code}"
        response_json = response.json()
        assert response_json['schema_validation'] is False, "Schema validation should be false for invalid JSON"


if __name__ == "__main__":
    # You can run this file directly to test all tests
    # Example: pytest rust_json.py -v
    # Or run specific test methods:
    # Example: pytest rust_json.py::TestJsonValidation::test_valid_json_schema -v
    pytest.main(["-v", __file__])