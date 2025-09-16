"""
Test module for Synonym functionality.

This module tests the end-to-end compute synonym operations, including:
1. Testing query computation with no synonyms
2. Testing query computation with existing synonyms
3. Testing error handling for invalid requests
4. Verifying synonyms are applied correctly in query processing

The tests ensure that the synonym replacement functionality works correctly
and handles edge cases appropriately.
"""
import json
import requests
import os
import pytest
import uuid
from dotenv import load_dotenv
from shared_function import cognito_connexion

# Load environment variables from .env file
load_dotenv()

@pytest.fixture(scope="module")
def api_credentials():
    """
    Fixture for API credentials and endpoints.
    
    Returns:
        dict: Dictionary containing authentication token, API key, and endpoint function.
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
def test_synonym(api_credentials):
    """
    Create a test synonym, yield it for tests, then delete it after tests.
    
    This fixture:
    1. Creates a temporary test synonym with unique name/value
    2. Yields the synonym details for test usage
    3. Cleans up by deleting the test synonym after tests complete
    
    Args:
        api_credentials: Fixture providing API authentication and endpoint details
        
    Returns:
        dict: Dictionary containing the test synonym's UUID, name, and value
    """
    # Generate a unique test synonym name and value with UUID
    test_name = f"test_synonym_{uuid.uuid4().hex[:8]}"
    test_value = f"test_synonym_value_{uuid.uuid4().hex[:8]}"
    
    # Create a synonym
    add_endpoint = api_credentials['get_endpoint']('rust_add_synonym')
    add_data = {
        "synonym_name": test_name,
        "synonym_value": test_value,
        "comments": "Test synonym created for automated testing"
    }
    
    add_response = requests.put(
        add_endpoint,
        data=json.dumps(add_data),
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_credentials['token']}",
            "x-api-key": api_credentials['api_key']
        }
    )
    
    assert add_response.status_code == 200, f"Failed to create test synonym: {add_response.text}"
    add_result = json.loads(add_response.text)
    synonym_uuid = add_result["synonym"]["synonym_uuid"]
    
    print(f"Created test synonym for testing: {test_name} -> {test_value} (UUID: {synonym_uuid})")
    
    # Yield the synonym info for test use
    yield {
        "uuid": synonym_uuid,
        "name": test_name,
        "value": test_value
    }
    
    # Clean up - Delete the synonym after tests
    delete_endpoint = api_credentials['get_endpoint']('rust_delete_synonym')
    delete_data = {"synonym_uuid": synonym_uuid}
    
    delete_response = requests.delete(
        delete_endpoint,
        data=json.dumps(delete_data),
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_credentials['token']}",
            "x-api-key": api_credentials['api_key']
        }
    )
    
    assert delete_response.status_code == 200, f"Failed to clean up test synonym: {delete_response.text}"
    print(f"Cleanup: Deleted test synonym {test_name} (Status: {delete_response.status_code})")


class TestComputeSynonym:
    """
    Test class for compute synonym operations.
    
    This class contains tests for the synonym computation service, which
    transforms queries by replacing known synonym names with their values.
    """
    
    def _make_compute_request(self, api_credentials, data, expected_status=200):
        """
        Helper method to make a request to the compute_synonym API.
        
        Args:
            api_credentials: API credentials fixture
            data: Request data to send
            expected_status: Expected HTTP status code(s)
            
        Returns:
            tuple: (response object, parsed response data if available)
        """
        endpoint = api_credentials['get_endpoint']('rust_compute_synonym')
        
        # Prepare headers
        headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_credentials['token']}",
            "x-api-key": api_credentials['api_key']
        }
        
        # Convert data to JSON string if it's not already a string
        if not isinstance(data, str):
            data = json.dumps(data)
        
        # Make the POST request
        response = requests.post(
            endpoint,
            data=data,
            headers=headers
        )
        
        # Log response details for debugging
        print(f'STATUS: {response.status_code} REASON: {response.reason}')
        print(f'RESPONSE: {response.text}')
        
        # Check if status code matches expected
        if isinstance(expected_status, list):
            assert response.status_code in expected_status, f"Expected status code in {expected_status}, got {response.status_code}"
        else:
            assert response.status_code == expected_status, f"Expected status code {expected_status}, got {response.status_code}"
        
        # Parse response data if possible
        response_data = None
        if response.text:
            try:
                response_data = json.loads(response.text)
            except json.JSONDecodeError:
                pass
                
        return response, response_data
    
    def test_basic_query_no_synonyms(self, api_credentials):
        """
        Test compute synonym with a query that doesn't contain any synonyms.
        
        This test verifies that queries without any matching synonyms remain unchanged.
        """
        # Test with a query that doesn't contain any known synonyms
        test_data = {
            "query": "This is a test query with no known keywords"
        }

        # Make the request and get response data
        _, response_data = self._make_compute_request(api_credentials, test_data)
        
        # Verify response structure
        assert response_data.get('statusAPI') == 'OK', f"API did not return OK: {response_data}"
        assert 'original_query' in response_data, "Response missing 'original_query' field"
        assert 'processed_query' in response_data, "Response missing 'processed_query' field"
        
        # Original query should match the processed query (no synonyms applied)
        original = response_data.get('original_query')
        processed = response_data.get('processed_query')
        assert original == processed, "Query should remain unchanged when no synonyms match"
    
    def test_query_with_synonyms(self, api_credentials, test_synonym):
        """
        Test compute synonym with a query containing the test synonym.
        
        This test verifies that a query containing a known synonym name is properly
        transformed to include the synonym value.
        """
        # Create a query using the test synonym name
        test_data = {
            "query": f"This is a test query about {test_synonym['name']} and other topics"
        }
        
        # Make the request and get response data
        _, response_data = self._make_compute_request(api_credentials, test_data)
        
        # Verify response structure
        assert response_data.get('statusAPI') == 'OK', f"API did not return OK: {response_data}"
        assert 'original_query' in response_data, "Response missing 'original_query' field"
        assert 'processed_query' in response_data, "Response missing 'processed_query' field"
        
        # Check that the synonym was properly applied
        original = response_data.get('original_query')
        processed = response_data.get('processed_query')
        
        assert original != processed, "Synonym replacement should have happened"
        assert test_synonym['value'] in processed, f"Processed query should contain the synonym value '{test_synonym['value']}'"
        expected_format = f"{test_synonym['name']} or {test_synonym['value']}"
        assert expected_format in processed, f"Processed query should contain the formatted replacement '{expected_format}'"
    
    def test_invalid_request_format(self, api_credentials):
        """
        Test compute synonym with invalid request format.
        
        This test verifies that the API properly handles malformed JSON input.
        """
        # Invalid JSON data
        test_data = "This is not valid JSON"
        
        # Make the request - expect 400 or 500 status code for invalid input
        self._make_compute_request(api_credentials, test_data, expected_status=[400, 500])
        
    def test_missing_query(self, api_credentials):
        """
        Test compute synonym with missing query parameter.
        
        This test verifies that the API properly handles requests missing the required
        'query' parameter.
        """
        # Missing required query parameter
        test_data = {}
        
        # Make the request - expect 400 or 500 status code for missing required param
        self._make_compute_request(api_credentials, test_data, expected_status=[400, 500])
        
    def test_empty_query(self, api_credentials):
        """
        Test compute synonym with an empty query string.
        
        This test verifies that the API properly handles requests with an empty
        'query' parameter.
        """
        # Empty query parameter
        test_data = {"query": ""}
        
        # Make the request - expect 400 or 500 status code for empty query
        self._make_compute_request(api_credentials, test_data, expected_status=[400, 500])
        
    def test_multiple_synonyms_in_query(self, api_credentials):
        """
        Test compute synonym with multiple synonyms in a single query.
        
        This test creates two temporary synonyms and verifies that both are
        properly replaced in a single query.
        """
        # Create two temporary synonyms for this test
        add_endpoint = api_credentials['get_endpoint']('rust_add_synonym')
        
        # First synonym
        first_name = f"first_test_{uuid.uuid4().hex[:6]}"
        first_value = f"first_value_{uuid.uuid4().hex[:6]}"
        
        # Second synonym
        second_name = f"second_test_{uuid.uuid4().hex[:6]}"
        second_value = f"second_value_{uuid.uuid4().hex[:6]}"
        
        created_uuids = []
        
        try:
            # Create first synonym
            add_data1 = {
                "synonym_name": first_name,
                "synonym_value": first_value,
                "comments": "First test synonym for multiple synonym test"
            }
            
            add_response1 = requests.put(
                add_endpoint,
                data=json.dumps(add_data1),
                headers={
                    "Content-Type": "application/json",
                    "Authorization": f"Bearer {api_credentials['token']}",
                    "x-api-key": api_credentials['api_key']
                }
            )
            
            assert add_response1.status_code == 200, f"Failed to create first test synonym: {add_response1.text}"
            add_result1 = json.loads(add_response1.text)
            first_uuid = add_result1["synonym"]["synonym_uuid"]
            created_uuids.append(first_uuid)
            
            # Create second synonym
            add_data2 = {
                "synonym_name": second_name,
                "synonym_value": second_value,
                "comments": "Second test synonym for multiple synonym test"
            }
            
            add_response2 = requests.put(
                add_endpoint,
                data=json.dumps(add_data2),
                headers={
                    "Content-Type": "application/json",
                    "Authorization": f"Bearer {api_credentials['token']}",
                    "x-api-key": api_credentials['api_key']
                }
            )
            
            assert add_response2.status_code == 200, f"Failed to create second test synonym: {add_response2.text}"
            add_result2 = json.loads(add_response2.text)
            second_uuid = add_result2["synonym"]["synonym_uuid"]
            created_uuids.append(second_uuid)
            
            # Test a query with both synonyms
            test_query = f"A query with {first_name} and {second_name} in it"
            test_data = {"query": test_query}
            
            # Make the compute request
            _, response_data = self._make_compute_request(api_credentials, test_data)
            
            # Verify both synonyms were replaced
            processed = response_data.get('processed_query')
            assert f"{first_name} or {first_value}" in processed, f"First synonym not properly replaced in: {processed}"
            assert f"{second_name} or {second_value}" in processed, f"Second synonym not properly replaced in: {processed}"
            
        finally:
            # Clean up - Delete both test synonyms
            delete_endpoint = api_credentials['get_endpoint']('rust_delete_synonym')
            
            for uuid_to_delete in created_uuids:
                delete_data = {"synonym_uuid": uuid_to_delete}
                
                delete_response = requests.delete(
                    delete_endpoint,
                    data=json.dumps(delete_data),
                    headers={
                        "Content-Type": "application/json",
                        "Authorization": f"Bearer {api_credentials['token']}",
                        "x-api-key": api_credentials['api_key']
                    }
                )
                
                print(f"Cleanup: Deleted test synonym with UUID {uuid_to_delete} (Status: {delete_response.status_code})")


if __name__ == "__main__":
    """
    Main entry point for running the tests directly.
    
    Examples:
        # Run all tests in this file:
        python rust_compute_synonym.py
        
        # Run a specific test:
        pytest rust_compute_synonym.py::TestComputeSynonym::test_query_with_synonyms -v
        
        # Run with coverage:
        pytest rust_compute_synonym.py --cov=.
    """
    print("Running rust_compute_synonym tests...")
    pytest.main(['-xvs', __file__])