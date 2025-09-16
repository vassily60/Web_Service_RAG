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


# Test module for GET recurrent query operations
class TestGetRecurrentQuery:
    """Test class for GET operations on recurrent queries"""
    
    def test_get_recurrent_query(self, api_credentials):
        """Test GET operation for recurrent queries"""
        endpoint = api_credentials['get_endpoint']('rust_get_recurrent_query')
        print("GET API Endpoint:", endpoint)

        # Make the GET request to fetch all recurrent queries
        get_response = requests.post(
            endpoint,
            data=json.dumps({}),  # Empty data to fetch all recurrent queries
            headers={
                "Content-Type": "application/json", 
                "Authorization": f"Bearer {api_credentials['token']}",
                "x-api-key": api_credentials['api_key']
            })
        
        print(f'GET STATUS: {get_response.status_code} REASON: {get_response.reason}')
        print(f'GET RESPONSE: {get_response.text}')
        
        assert get_response.status_code == 200


# Test module for ADD recurrent query operations
class TestAddRecurrentQuery:
    """Test class for ADD operations on recurrent queries"""
    
    def test_add_recurrent_query(self, api_credentials):
        """Test ADD operation for recurrent queries"""
        endpoint = api_credentials['get_endpoint']('rust_add_recurrent_query')
        print("ADD API Endpoint:", endpoint)

        # Prepare data for creating a new recurrent query
        add_data = {
            "recurrent_query_name": "Test Monthly Reports",
            "query_type": "document_search",
            "query_content": "monthly report financial",
            "query_tags": "monthly,report,financial",
            "query_start_document_date": "2025-01-01",
            "query_end_document_date": "2025-12-31",
            "comments": "Test recurrent query created by automated test"
        }

        # Make the POST request to add a new recurrent query
        add_response = requests.post(
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
        assert "statusAPI" in add_result, "Response should contain statusAPI"
        assert add_result["statusAPI"] == "OK", "Status should be OK"
        
        # Check if the response contains the recurrent query data
        if "recurrent_query" in add_result:
            recurrent_query_uuid = add_result["recurrent_query"]["recurrent_query_uuid"]
            print("New recurrent query UUID:", recurrent_query_uuid)


# Test module for UPDATE recurrent query operations
class TestUpdateRecurrentQuery:
    """Test class for UPDATE operations on recurrent queries"""
    
    def test_update_recurrent_query(self, api_credentials):
        """Test UPDATE operation for recurrent queries"""
        # First, create a recurrent query to update
        add_endpoint = api_credentials['get_endpoint']('rust_add_recurrent_query')
        add_data = {
            "recurrent_query_name": "Query to Update",
            "query_type": "document_search",
            "query_content": "original search terms",
            "query_tags": "original,tags",
            "query_start_document_date": "2025-01-01",
            "query_end_document_date": "2025-12-31",
            "comments": "Temporary query for update test"
        }
        
        add_response = requests.post(
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
        print(add_result)
        # Extract the UUID of the newly created recurrent query
        recurrent_query_uuid = None
        if "recurrent_query_uuid" in add_result:
            recurrent_query_uuid = add_result["recurrent_query_uuid"]
        else:
            pytest.skip("Could not extract recurrent_query_uuid from response. Skipping update test.")
            
        print("Created recurrent query UUID for update:", recurrent_query_uuid)
        
        # Wait a moment to avoid throttling
        time.sleep(1)
        
        # Now update it
        endpoint = api_credentials['get_endpoint']('rust_update_recurrent_query')
        print("UPDATE API Endpoint:", endpoint)

        # Prepare data for updating the recurrent query
        update_data = {
            "recurrent_query_uuid": recurrent_query_uuid,
            "recurrent_query_name": "Updated Query Name",
            "query_type": "document_search",
            "query_content": "updated search terms",
            "query_tags": "updated,tags",
            "query_start_document_date": "2025-02-01",
            "query_end_document_date": "2025-11-30",
            "comments": "Updated test recurrent query by automated test"
        }

        # Make the PUT request to update the recurrent query
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
        assert "statusAPI" in update_result
        assert update_result["statusAPI"] == "OK"


# Test module for DELETE recurrent query operations
class TestDeleteRecurrentQuery:
    """Test class for DELETE operations on recurrent queries"""
    
    def test_delete_recurrent_query(self, api_credentials):
        """Test DELETE operation for recurrent queries"""
        # First create a recurrent query to delete
        add_endpoint = api_credentials['get_endpoint']('rust_add_recurrent_query')
        add_data = {
            "recurrent_query_name": "Query to Delete",
            "query_type": "document_search",
            "query_content": "delete test search",
            "query_tags": "delete,test",
            "query_start_document_date": "2025-01-01",
            "query_end_document_date": "2025-12-31",
            "comments": "Temporary query for deletion test"
        }
        
        add_response = requests.post(
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
        
        # Extract the UUID of the newly created recurrent query
        recurrent_query_uuid = None
        if "recurrent_query_uuid" in add_result:
            recurrent_query_uuid = add_result["recurrent_query_uuid"]
        else:
            pytest.skip("Could not extract recurrent_query_uuid from response. Skipping deletion test.")
            
        print("Created recurrent query UUID for deletion:", recurrent_query_uuid)
        
        # Wait a moment to avoid throttling
        time.sleep(1)
        
        # Now delete it
        delete_endpoint = api_credentials['get_endpoint']('rust_delete_recurrent_query')
        delete_data = {"recurrent_query_uuid": recurrent_query_uuid}
        
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
    # Example: pytest rust_recurrent_query.py::TestAddRecurrentQuery -v
    # Example: pytest rust_recurrent_query.py::TestGetRecurrentQuery -v
    # Example: pytest rust_recurrent_query.py::TestUpdateRecurrentQuery -v
    # Example: pytest rust_recurrent_query.py::TestDeleteRecurrentQuery -v
    pytest.main(['-xvs', __file__])
