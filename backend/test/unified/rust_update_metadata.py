
import os
import json
import requests
import pytest
from dotenv import load_dotenv
from shared_function import cognito_connexion

load_dotenv()

@pytest.fixture(scope="module")
def api_credentials():
    """Fixture to get API credentials and endpoint"""
    conn_result = cognito_connexion(os.getenv('USER_NAME'), os.getenv('PASSWORD'))
    assert conn_result['success'], "Authentication failed"
    
    api_end_point_service = 'rust_update_metadata'
    local_debug_flag = os.getenv('LOCAL_DEBUG_FLAG', 'false').lower() == 'true'
    if local_debug_flag:
        post_address = os.getenv('API_DEBUG_ENDPOINT_ROOT') + '/' + api_end_point_service
    else:
        post_address = os.getenv('API_ENDPOINT_ROOT') + '/' + os.getenv('API_STAGE') + '/' + api_end_point_service
    
    return {
        'token': conn_result['data']['id_token'],
        'endpoint': post_address
    }

def test_update_metadata(api_credentials):
    """Test updating an existing metadata"""
    print("API Endpoint:", api_credentials['endpoint'])
    
    # Prepare data for updating an existing metadata
    update_data = {
        "metadata_uuid": "6aefaf50-c5f7-41a7-8b94-7f90a97cfb8d",
        "metadata_name": "updated_document_category",
        "metadata_description": "Updated category description",
        "metadata_type": "string"
    }
    
    response = requests.put(
        api_credentials['endpoint'],
        data=json.dumps(update_data),
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_credentials['token']}",
            "x-api-key": os.getenv('API_KEY')
        }
    )
    
    print(f'UPDATE STATUS: {response.status_code} REASON: {response.reason}')
    print(f'UPDATE RESPONSE: {response.text}')
    
    assert response.status_code == 200
    
    if response.text:
        response_json = json.loads(response.text)
        assert response_json is not None, "Response should not be empty"

if __name__ == "__main__":
    pytest.main()