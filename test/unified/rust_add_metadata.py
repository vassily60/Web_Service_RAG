import os
import json
import requests
from dotenv import load_dotenv
import pytest
from shared_function import cognito_connexion

load_dotenv()
print("Environment variables loaded.")

def test_add_metadata():
    print('1hi')
    connResult = cognito_connexion(os.getenv('USER_NAME'), os.getenv('PASSWORD'))
    print('hi')
    assert connResult['success'] is True, 'Cognito authentication failed'

    API_END_POINT_SERVICE = 'rust_add_metadata'
    local_debug_flag = os.getenv('LOCAL_DEBUG_FLAG', 'false').lower() == 'true'
    if local_debug_flag:
        PostAddressGetInfoElement = os.getenv('API_DEBUG_ENDPOINT_ROOT') + '/' + API_END_POINT_SERVICE
    else:
        PostAddressGetInfoElement = os.getenv('API_ENDPOINT_ROOT') + '/' + os.getenv('API_STAGE') + '/' + API_END_POINT_SERVICE
    print(f"PostAddressGetInfoElement: {PostAddressGetInfoElement}")
    token = connResult['data']['id_token']
    PostData = {
        "metadata_name": os.getenv('TEST_METADATA_NAME', 'test_metadata2'),
        "metadata_description": os.getenv('TEST_METADATA_DESCRIPTION', 'Test metadata description created by automated test'),
        "metadata_type": os.getenv('TEST_METADATA_TYPE', 'string')
    }
    response = requests.post(
        PostAddressGetInfoElement,
        data=json.dumps(PostData),
        headers={
            "Content-Type": "application/json",
            "Authorization": "Bearer " + token,
            "x-api-key": os.getenv('API_KEY')
        }
    )
    print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
    print(f'POST RESPONSE: {response.text}')
    assert response.status_code == 200, f"POST failed: {response.status_code} {response.reason} {response.text}"
    try:
        response_data = json.loads(response.text)
        assert 'metadata_uuid' in response_data, "metadata_uuid not in response"
    except json.JSONDecodeError:
        pytest.fail("Failed to parse response JSON")

if __name__ == "__main__":
    test_add_metadata()