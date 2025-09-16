import json
import requests
import os
from dotenv import load_dotenv
import pytest
from shared_function import *

load_dotenv()

def test_get_document_metadatas():
    connResult = cognito_connexion(os.getenv('USER_NAME'), os.getenv('PASSWORD'))
    assert connResult['success'] is True, 'Cognito authentication failed'

    API_END_POINT_SERVICE = 'rust_get_document_metadatas'
    local_debug_flag = os.getenv('LOCAL_DEBUG_FLAG', 'false').lower() == 'true'
    if local_debug_flag:
        PostAddressGetInfoElement = os.getenv('API_DEBUG_ENDPOINT_ROOT') + '/' + API_END_POINT_SERVICE
    else:
        PostAddressGetInfoElement = os.getenv('API_ENDPOINT_ROOT') + '/' + os.getenv('API_STAGE') + '/' + API_END_POINT_SERVICE
    PostData = {
        "document_uuid": os.getenv('TEST_DOCUMENT_UUID', 'dummy-uuid')
    }
    token = connResult['data']['id_token']
    response = requests.post(
        PostAddressGetInfoElement,
        data=json.dumps(PostData),
        headers={
            "Content-Type": "application/json",
            "Authorization": "Bearer " + token,
            "x-api-key": os.getenv('API_KEY')
        }
    )
    assert response.status_code == 200, f"POST failed: {response.status_code} {response.reason} {response.text}"
    try:
        response_data = json.loads(response.text)
        assert response_data.get('statusAPI') == 'OK', f"API did not return OK: {response_data}"
    except json.JSONDecodeError:
        pytest.fail("Failed to parse response JSON")
    """
    # Test case 2: Get all document metadatas
    print("\n--- TEST CASE 2: Get all document metadatas ---")
    PostData = {
        "get_all": True
    }

    response = requests.post(
        PostAddressGetInfoElement, 
        data=json.dumps(PostData), 
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + token, "x-api-key": API_KEY}
    )

    print('POST STATUS: ' + str(response.status_code) + ' REASON: ' + str(response.reason))
    print('POST ANSWER: ' + response.text)

    # Test case 3: Get metadata by category
    print("\n--- TEST CASE 3: Get metadata by category ---")
    PostData = {
        "category": "ESRS"  # Replace with an actual category in your system
    }

    response = requests.post(
        PostAddressGetInfoElement, 
        data=json.dumps(PostData), 
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + token, "x-api-key": API_KEY}
    )

    print('POST STATUS: ' + str(response.status_code) + ' REASON: ' + str(response.reason))
    print('POST ANSWER: ' + response.text)
    """
# else:
#     print('Error connecting to Cognito authentication service')