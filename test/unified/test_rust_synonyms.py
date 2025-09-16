import json
import requests
import os
import pytest
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


# Test module for GET synonym operations
class TestGetSynonyms:
    """Test class for GET operations on synonyms"""
    
    def test_get_synonyms(self, api_credentials):
        """Test GET operation for synonyms"""
        endpoint = api_credentials['get_endpoint']('rust_get_synonym')
        print("GET API Endpoint:", endpoint)

        # Make the GET request to fetch all synonyms
        get_response = requests.post(
            endpoint,
            headers={
                "Content-Type": "application/json", 
                "Authorization": f"Bearer {api_credentials['token']}",
                "x-api-key": api_credentials['api_key']
            }
        )

        print(f'GET STATUS: {get_response.status_code} REASON: {get_response.reason}')
        print(f'GET RESPONSE: {get_response.text}')
        
        assert get_response.status_code == 200


# Test module for ADD synonym operations
class TestAddSynonym:
    """Test class for ADD operations on synonyms"""
    
    def test_add_synonym(self, api_credentials):
        """Test ADD operation for a synonym"""
        endpoint = api_credentials['get_endpoint']('rust_add_synonym')
        print("ADD API Endpoint:", endpoint)

        # Prepare data for creating a new synonym
        add_data = {
            "synonym_name": "test_synonym",
            "synonym_value": "test_value",
            "comments": "Test synonym created by automated test"
        }

        # Make the PUT request to add a new synonym
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
        assert "synonym" in add_result, "Response should contain synonym data"
        assert "synonym_uuid" in add_result["synonym"], "Response should contain synonym_uuid"
        
        synonym_uuid = add_result["synonym"]["synonym_uuid"]
        print("New synonym UUID:", synonym_uuid)
        
        # # Cleanup: Delete the synonym after test is done
        # delete_endpoint = api_credentials['get_endpoint']('rust_delete_synonym')
        # delete_data = {"synonym_uuid": synonym_uuid}
        
        # delete_response = requests.delete(
        #     delete_endpoint,
        #     data=json.dumps(delete_data),
        #     headers={
        #         "Content-Type": "application/json",
        #         "Authorization": f"Bearer {api_credentials['token']}",
        #         "x-api-key": api_credentials['api_key']
        #     }
        # )
        
        # print(f'Cleanup DELETE STATUS: {delete_response.status_code} REASON: {delete_response.reason}')
        # assert delete_response.status_code == 200


# Test module for UPDATE synonym operations
class TestUpdateSynonym:
    """Test class for UPDATE operations on synonyms"""
    
    def test_update_synonym(self, api_credentials):
        """Test UPDATE operation for a synonym"""
        # First, create a synonym to update
        add_endpoint = api_credentials['get_endpoint']('rust_add_synonym')
        add_data = {
            "synonym_name": "synonym_to_update",
            "synonym_value": "original_value",
            "comments": "Temporary synonym for update test"
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
        synonym_uuid = add_result["synonym"]["synonym_uuid"]
        print("Created synonym UUID for update:", synonym_uuid)
        
        # Now update it
        endpoint = api_credentials['get_endpoint']('rust_update_synonym')
        print("UPDATE API Endpoint:", endpoint)

        # Prepare data for updating the synonym
        update_data = {
            "synonym_uuid": synonym_uuid,
            "synonym_name": "updated_test_synonym",
            "synonym_value": "updated_test_value",
            "comments": "Updated test synonym by automated test"
        }

        # Make the PUT request to update the synonym
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
        assert "synonym" in update_result
        assert update_result["synonym"]["synonym_name"] == "updated_test_synonym"
        
        # # Cleanup: Delete the synonym after test is done
        # delete_endpoint = api_credentials['get_endpoint']('rust_delete_synonym')
        # delete_data = {"synonym_uuid": synonym_uuid}
        
        # delete_response = requests.delete(
        #     delete_endpoint,
        #     data=json.dumps(delete_data),
        #     headers={
        #         "Content-Type": "application/json",
        #         "Authorization": f"Bearer {api_credentials['token']}",
        #         "x-api-key": api_credentials['api_key']
        #     }
        # )
        
        # print(f'Cleanup DELETE STATUS: {delete_response.status_code} REASON: {delete_response.reason}')
        # assert delete_response.status_code == 200


# Test module for DELETE synonym operations
class TestDeleteSynonym:
    """Test class for DELETE operations on synonyms"""
    
    def test_delete_synonym(self, api_credentials):
        """Test DELETE operation for a synonym"""
        # First create a synonym to delete
        add_endpoint = api_credentials['get_endpoint']('rust_add_synonym')
        add_data = {
            "synonym_name": "synonym_to_delete",
            "synonym_value": "delete_value",
            "comments": "Temporary synonym for deletion test"
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
        synonym_uuid = add_result["synonym"]["synonym_uuid"]
        print("Created synonym UUID for deletion:", synonym_uuid)
        
        # Now delete it
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
        
        print(f'DELETE STATUS: {delete_response.status_code} REASON: {delete_response.reason}')
        print(f'DELETE RESPONSE: {delete_response.text}')
        
        assert delete_response.status_code == 200

if __name__ == "__main__":
    # You can run this file directly to test all tests
    # Or specify a specific test class to run with pytest
    # Example: pytest rust_synonyms.py::TestAddSynonym -v
    # Example: pytest rust_synonyms.py::TestGetSynonyms -v
    # Example: pytest rust_synonyms.py::TestUpdateSynonym -v
    # Example: pytest rust_synonyms.py::TestDeleteSynonym -v
    pytest.main(['-xvs', __file__])