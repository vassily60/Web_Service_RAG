import json
import requests
import os
import pytest
import time
from dotenv import load_dotenv
from shared_function import cognito_connexion

load_dotenv()

@pytest.fixture(scope="module")
def api_credentials():
    """Fixture for API credentials and endpoints"""
    # Access environment variables
    user_name = os.getenv('USER_NAME')
    password = os.getenv('PASSWORD')
    api_key = os.getenv('API_KEY')
    local_debug_flag = os.getenv('LOCAL_DEBUG_FLAG', 'false').lower() == 'true'
    api_endpoint_root = os.getenv('API_ENDPOINT_ROOT')
    api_debug_endpoint_root = os.getenv('API_DEBUG_ENDPOINT_ROOT')
    api_stage = os.getenv('API_STAGE')
    
    # First, authenticate with Cognito to get a token
    conn_result = cognito_connexion(user_name, password)
    assert conn_result['success'], "Authentication failed"
    
    # Build base endpoint for all API calls
    def get_endpoint(service):
        if local_debug_flag:
            return api_debug_endpoint_root + '/' + service
        return api_endpoint_root + '/' + api_stage + '/' + service
    
    return {
        'token': conn_result['data']['id_token'],
        'api_key': api_key,
        'get_endpoint': get_endpoint
    }


# Test module for GET metadata operations
class TestGetMetadata:
    """Test class for GET operations on metadata"""
    
    def test_get_metadata(self, api_credentials):
        """Test GET operation for metadata"""
        endpoint = api_credentials['get_endpoint']('rust_get_metadata')
        print("GET API Endpoint:", endpoint)

        # Make the GET request to fetch all metadata
        get_response = requests.post(
            endpoint,
            headers={
                "Content-Type": "application/json", 
                "Authorization": f"Bearer {api_credentials['token']}",
                "x-api-key": api_credentials['api_key']
            })
        

        print(f'GET STATUS: {get_response.status_code} REASON: {get_response.reason}')
        print(f'GET RESPONSE: {get_response.text}')
        
        assert get_response.status_code == 200


# Test module for ADD metadata operations
class TestAddMetadata:
    """Test class for ADD operations on metadata"""
    
    def test_add_metadata(self, api_credentials):
        """Test ADD operation for metadata"""
        endpoint = api_credentials['get_endpoint']('rust_add_metadata')
        print("ADD API Endpoint:", endpoint)

        # Prepare data for creating a new metadata
        add_data = {
            "metadata_name": "test_metadata",
            "metadata_value": "test_value",
            "comments": "Test metadata created by automated test"
        }

        # Make the PUT request to add a new metadata
        add_response = requests.put(
            endpoint,
            data=json.dumps(add_data),
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {api_credentials['token']}",
                "x-api-key": api_credentials['api_key']
            }
        )

        print(f'ADD STATUS: {add_response.status_code} REASON: {add_response.reason}')
        print(f'ADD RESPONSE: {add_response.text}')
        
        assert add_response.status_code == 200
        
        add_result = json.loads(add_response.text)
        assert "metadata" in add_result, "Response should contain metadata data"
        assert "metadata_uuid" in add_result["metadata"], "Response should contain metadata_uuid"
        
        metadata_uuid = add_result["metadata"]["metadata_uuid"]
        print("New metadata UUID:", metadata_uuid)


# Test module for UPDATE metadata operations
class TestUpdateMetadata:
    """Test class for UPDATE operations on metadata"""
    
    def test_update_metadata(self, api_credentials):
        """Test UPDATE operation for metadata"""
        # First, create a metadata to update
        add_endpoint = api_credentials['get_endpoint']('rust_add_metadata')
        add_data = {
            "metadata_name": "metadata_to_update",
            "metadata_value": "original_value",
            "comments": "Temporary metadata for update test"
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
        
        assert add_response.status_code == 200
        add_result = json.loads(add_response.text)
        metadata_uuid = add_result["metadata"]["metadata_uuid"]
        print("Created metadata UUID for update:", metadata_uuid)
        
        # Wait a moment to avoid throttling
        time.sleep(1)
        
        # Now update it
        endpoint = api_credentials['get_endpoint']('rust_update_metadata')
        print("UPDATE API Endpoint:", endpoint)

        # Prepare data for updating the metadata
        update_data = {
            "metadata_uuid": metadata_uuid,
            "metadata_name": "updated_test_metadata",
            "metadata_value": "updated_test_value",
            "comments": "Updated test metadata by automated test"
        }

        # Make the PUT request to update the metadata
        update_response = requests.put(
            endpoint,
            data=json.dumps(update_data),
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {api_credentials['token']}",
                "x-api-key": api_credentials['api_key']
            }
        )

        print(f'UPDATE STATUS: {update_response.status_code} REASON: {update_response.reason}')
        print(f'UPDATE RESPONSE: {update_response.text}')
        
        assert update_response.status_code == 200
        update_result = json.loads(update_response.text)
        assert "metadata" in update_result
        assert update_result["metadata"]["metadata_name"] == "updated_test_metadata"


# Test module for DELETE metadata operations
class TestDeleteMetadata:
    """Test class for DELETE operations on metadata"""
    
    def test_delete_metadata(self, api_credentials):
        """Test DELETE operation for metadata"""
        # First create a metadata to delete
        add_endpoint = api_credentials['get_endpoint']('rust_add_metadata')
        add_data = {
            "metadata_name": "metadata_to_delete",
            "metadata_value": "delete_value",
            "comments": "Temporary metadata for deletion test"
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
        
        assert add_response.status_code == 200
        add_result = json.loads(add_response.text)
        metadata_uuid = add_result["metadata"]["metadata_uuid"]
        print("Created metadata UUID for deletion:", metadata_uuid)
        
        # Wait a moment to avoid throttling
        time.sleep(1)
        
        # Now delete it
        delete_endpoint = api_credentials['get_endpoint']('rust_delete_metadata')
        delete_data = {"metadata_uuid": metadata_uuid}
        
        delete_response = requests.delete(
            delete_endpoint,
            data=json.dumps(delete_data),
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {api_credentials['token']}",
                "x-api-key": api_credentials['api_key']
            }
        )
        
        print(f'DELETE STATUS: {delete_response.status_code} REASON: {delete_response.reason}')
        print(f'DELETE RESPONSE: {delete_response.text}')
        
        assert delete_response.status_code == 200

if __name__ == "__main__":
    # You can run this file directly to test all tests
    # Or specify a specific test class to run with pytest
    # Example: pytest rust_metadata.py::TestAddMetadata -v
    # Example: pytest rust_metadata.py::TestGetMetadata -v
    # Example: pytest rust_metadata.py::TestUpdateMetadata -v
    # Example: pytest rust_metadata.py::TestDeleteMetadata -v
    pytest.main(['-xvs', __file__])
