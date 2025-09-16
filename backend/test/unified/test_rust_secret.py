"""
Test module for the rust_secret Lambda function.

This module contains tests for validating the AWS Secrets Manager functionality 
provided by the rust_secret Lambda.
"""
import json
import requests
import os
import pytest
from dotenv import load_dotenv
from shared_function import cognito_connexion

load_dotenv()

@pytest.fixture(scope="module")
def api_credentials():
    """
    Fixture providing authentication credentials and API endpoints.
    
    Uses environment variables to configure API connection details and authenticates
    with Cognito to obtain a valid token for API requests.
    
    Returns:
        dict: Contains the authentication token, API key and endpoint generator function.
    """
    # Access environment variables with validation
    user_name = os.getenv('USER_NAME')
    assert user_name, "USER_NAME environment variable is required"
    
    password = os.getenv('PASSWORD')
    assert password, "PASSWORD environment variable is required"
    
    api_key = os.getenv('API_KEY')
    assert api_key, "API_KEY environment variable is required"
    
    # Convert string boolean to actual boolean
    local_debug_flag = os.getenv('LOCAL_DEBUG_FLAG', 'false').lower() == 'true'
    
    api_endpoint_root = os.getenv('API_ENDPOINT_ROOT')
    assert api_endpoint_root, "API_ENDPOINT_ROOT environment variable is required"
    
    api_debug_endpoint_root = os.getenv('API_DEBUG_ENDPOINT_ROOT')
    if local_debug_flag:
        assert api_debug_endpoint_root, "API_DEBUG_ENDPOINT_ROOT is required when LOCAL_DEBUG_FLAG is true"
    
    api_stage = os.getenv('API_STAGE')
    if not local_debug_flag:
        assert api_stage, "API_STAGE environment variable is required when not in debug mode"
    
    # Authenticate with Cognito to get a token
    conn_result = cognito_connexion(user_name, password)
    assert conn_result['success'], f"Authentication failed: {conn_result.get('message', 'No error message provided')}"
    
    # Build base endpoint for all API calls
    def get_endpoint(service):
        """
        Generate the appropriate endpoint URL based on environment settings.
        
        Args:
            service (str): The API service endpoint name.
            
        Returns:
            str: The complete URL for the specified service.
        """
        if local_debug_flag:
            return f"{api_debug_endpoint_root}/{service}"
        return f"{api_endpoint_root}/{api_stage}/{service}"
    
    return {
        'token': conn_result['data']['id_token'],
        'api_key': api_key,
        'get_endpoint': get_endpoint
    }


class TestSecretManager:
    """Test class for AWS Secrets Manager operations via rust_secret Lambda."""
    
    def test_secret_without_auth(self, api_credentials):
        """
        Test the rust_secret endpoint without authentication token.
        
        This test verifies that the Lambda properly rejects requests without 
        valid authentication. This test is skipped in local debug mode as it's
        only relevant for production API testing.
        
        Args:
            api_credentials: Fixture providing authentication and API endpoint details.
        """
        # Skip test if in local debug mode
        local_debug_flag = os.getenv('LOCAL_DEBUG_FLAG', 'false').lower() == 'true'
        if local_debug_flag:
            pytest.skip("Test skipped in local debug mode")
        
        # Prepare endpoint and test data
        endpoint = api_credentials['get_endpoint']('rust_secret')
        post_data = {'client_id': 22}
        
        # Make the API request without auth token
        response = requests.post(
            endpoint,
            data=json.dumps(post_data),
            headers={
                "Content-Type": "application/json",
                "x-api-key": api_credentials['api_key']
            },
            timeout=30
        )
        
        # Log response details for debugging
        print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
        
        # Verify unauthorized access is rejected
        assert response.status_code in [401, 403], f"Expected unauthorized status code, got {response.status_code}"
    
    def test_secret_with_invalid_data(self, api_credentials):
        """
        Test the rust_secret endpoint with invalid data format.
        
        This test verifies that the Lambda properly handles invalid request data.
        
        Args:
            api_credentials: Fixture providing authentication and API endpoint details.
        """
        # Prepare endpoint and test data (invalid format)
        endpoint = api_credentials['get_endpoint']('rust_secret')
        post_data = "This is not valid JSON"
        
        # Make the API request with invalid data
        response = requests.post(
            endpoint,
            data=post_data,
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {api_credentials['token']}",
                "x-api-key": api_credentials['api_key']
            },
            timeout=30
        )
        
        # Log response details for debugging
        print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
        print(f'POST RESPONSE: {response.text}')
        
        # Either the request is rejected with 4xx or accepted but returns an error message
        if response.status_code == 200:
            try:
                response_json = response.json()
                assert "statusAPI" in response_json, "Response missing 'statusAPI' field"
                if response_json["statusAPI"] == "ERROR":
                    assert "message" in response_json, "Error response missing 'message' field"
                    print(f"Error message: {response_json['message']}")
            except ValueError as e:
                assert False, f"Response should be valid JSON even for invalid inputs: {e}"
        else:
            assert response.status_code >= 400, f"Expected error status code for invalid data, got {response.status_code}"
            
    def test_json_response_format(self, api_credentials):
        """
        Test that the rust_secret endpoint returns properly formatted JSON.
        
        This test verifies that the Lambda returns responses with the correct
        JSON structure and content type.
        
        Args:
            api_credentials: Fixture providing authentication and API endpoint details.
        """
        # Prepare endpoint and test data
        endpoint = api_credentials['get_endpoint']('rust_secret')
        post_data = {}  # Empty request body, should use default secret name
        
        # Make the API request
        response = requests.post(
            endpoint,
            data=json.dumps(post_data),
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {api_credentials['token']}",
                "x-api-key": api_credentials['api_key']
            },
            timeout=30
        )
        
        # Log response details for debugging
        print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
        print(f'POST RESPONSE: {response.text}')
        
        # Verify response code
        assert response.status_code == 200, f"Expected status code 200, got {response.status_code}"
        
        # Verify content type header
        assert "application/json" in response.headers.get("content-type", ""), "Content-Type header should be application/json"
        
        # Verify response is valid JSON with expected structure
        response_json = response.json()
        assert "statusAPI" in response_json, "Response missing 'statusAPI' field"
        assert "message" in response_json, "Response missing 'message' field"
        
        # If the secret was valid JSON, we should have a 'data' field
        if "data" in response_json:
            print("Secret was parsed as JSON and included in response")
            assert isinstance(response_json["data"], dict) or isinstance(response_json["data"], list), "'data' field should be a JSON object or array"
        # If not JSON, we should have an 'is_json' field set to false
        elif "is_json" in response_json:
            assert response_json["is_json"] is False, "Secret wasn't JSON but 'is_json' isn't false"
            assert "raw_length" in response_json, "Non-JSON response should include 'raw_length' field"
            print(f"Secret was not JSON, length: {response_json['raw_length']}")


if __name__ == "__main__":
    pytest.main()