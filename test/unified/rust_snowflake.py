import os
import json
import requests
from dotenv import load_dotenv
import pytest
from shared_function import cognito_connexion

load_dotenv()

@pytest.mark.integration
def test_rust_snowflake():
    connResult = cognito_connexion(os.getenv('USER_NAME'), os.getenv('PASSWORD'))
    assert connResult['success'] is True, 'Cognito authentication failed'

    API_END_POINT_SERVICE = 'rust_snowflake'
    local_debug_flag = os.getenv('LOCAL_DEBUG_FLAG', 'false').lower() == 'true'
    if local_debug_flag:
        PostAddressGetInfoElement = os.getenv('API_DEBUG_ENDPOINT_ROOT') + '/' + API_END_POINT_SERVICE
    else:
        PostAddressGetInfoElement = os.getenv('API_ENDPOINT_ROOT') + '/' + os.getenv('API_STAGE') + '/' + API_END_POINT_SERVICE

    token = connResult['data']['id_token']
    PostData = {'client_id': int(os.getenv('TEST_CLIENT_ID', '22'))}
    response = requests.post(
        PostAddressGetInfoElement,
        data=json.dumps(PostData),
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + token, "x-api-key": os.getenv('API_KEY')}
    )
    assert response.status_code == 200, f"Test failed: {response.status_code} {response.text}"