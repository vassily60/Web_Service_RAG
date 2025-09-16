import os
import json
import requests
import pytest
from dotenv import load_dotenv
load_dotenv()
from shared_function import cognito_connexion

# @pytest.mark.integration
def test_rust_dynamo():
    user_name = os.getenv('USER_NAME')
    password = os.getenv('PASSWORD')
    api_key = os.getenv('API_KEY')
    api_endpoint_root = os.getenv('API_ENDPOINT_ROOT')
    api_debug_endpoint_root = os.getenv('API_DEBUG_ENDPOINT_ROOT')
    api_stage = os.getenv('API_STAGE')
    local_debug_flag = os.getenv('LOCAL_DEBUG_FLAG', 'false').lower() == 'true'

    conn_result = cognito_connexion(user_name, password)
    assert conn_result['success'] is True, 'Cognito authentication failed'

    api_end_point_service = 'rust_dynamo'
    if local_debug_flag:
        post_address = api_debug_endpoint_root + '/' + api_end_point_service
    else:
        post_address = api_endpoint_root + '/' + api_stage + '/' + api_end_point_service

    token = conn_result['data']['id_token']
    post_data = {'client_id': 22}
    response = requests.post(
        post_address,
        data=json.dumps(post_data),
        headers={
            "Content-Type": "application/json",
            "Authorization": "Bearer " + token,
            "x-api-key": api_key
        }
    )
    assert response.status_code == 200, f"POST failed: {response.status_code} {response.reason} {response.text}"