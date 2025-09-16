import os
import json
import requests
from dotenv import load_dotenv
import pytest
from shared_function import cognito_connexion

load_dotenv()

@pytest.mark.integration
def test_delete_metadata():
    connResult = cognito_connexion(os.getenv('USER_NAME'), os.getenv('PASSWORD'))
    assert connResult['success'] is True, 'Cognito authentication failed'

    API_END_POINT_SERVICE = 'rust_delete_metadata'
    local_debug_flag = os.getenv('LOCAL_DEBUG_FLAG', 'false').lower() == 'true'
    if local_debug_flag:
        PostAddressGetInfoElement = os.getenv('API_DEBUG_ENDPOINT_ROOT') + '/' + API_END_POINT_SERVICE
    else:
        PostAddressGetInfoElement = os.getenv('API_ENDPOINT_ROOT') + '/' + os.getenv('API_STAGE') + '/' + API_END_POINT_SERVICE

    token = connResult['data']['id_token']
    PostData = {
        "metadata_uuid": os.getenv('TEST_METADATA_UUID', 'dummy-uuid')
    }
    response = requests.delete(
        PostAddressGetInfoElement,
        data=json.dumps(PostData),
        headers={
            "Content-Type": "application/json",
            "Authorization": "Bearer " + token,
            "x-api-key": os.getenv('API_KEY')
        }
    )
    assert response.status_code == 200, f"DELETE failed: {response.status_code} {response.reason} {response.text}"
    try:
        response_data = json.loads(response.text)
        assert response_data.get('statusAPI') == 'OK', f"API did not return OK: {response_data}"
    except json.JSONDecodeError:
        pytest.fail("Failed to parse response JSON")