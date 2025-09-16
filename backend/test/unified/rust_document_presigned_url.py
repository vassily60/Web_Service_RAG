import os
import json
import requests
from dotenv import load_dotenv
import pytest
from shared_function import cognito_connexion

load_dotenv()

@pytest.mark.integration
def test_rust_document_presigned_url():
    connResult = cognito_connexion(os.getenv('USER_NAME'), os.getenv('PASSWORD'))
    assert connResult['success'] is True, 'Cognito authentication failed'

    API_END_POINT_SERVICE = 'rust_document_presigned_url'
    local_debug_flag = os.getenv('LOCAL_DEBUG_FLAG', 'false').lower() == 'true'
    if local_debug_flag:
        post_address = os.getenv('API_DEBUG_ENDPOINT_ROOT') + '/' + API_END_POINT_SERVICE
    else:
        post_address = os.getenv('API_ENDPOINT_ROOT') + '/' + os.getenv('API_STAGE') + '/' + API_END_POINT_SERVICE

    token = connResult['data']['id_token']

    # Test case 1: Valid document UUID and default expiration
    test_doc_uuid = os.getenv('TEST_DOCUMENT_UUID', '00000000-0000-0000-0000-000000000000')
    payload1 = {
        "document_uuid": test_doc_uuid
    }
    response1 = requests.post(
        post_address,
        data=json.dumps(payload1),
        headers={
            "Content-Type": "application/json",
            "Authorization": "Bearer " + token,
            "x-api-key": os.getenv('API_KEY')
        }
    )
    assert response1.status_code == 200, f"Test 1 failed: {response1.status_code} {response1.text}"

    # Test case 2: Valid document UUID and custom expiration
    payload2 = {
        "document_uuid": test_doc_uuid,
        "expiration": 3600
    }
    response2 = requests.post(
        post_address,
        data=json.dumps(payload2),
        headers={
            "Content-Type": "application/json",
            "Authorization": "Bearer " + token,
            "x-api-key": os.getenv('API_KEY')
        }
    )
    assert response2.status_code == 200, f"Test 2 failed: {response2.status_code} {response2.text}"

    # Test case 3: Invalid document UUID
    payload3 = {
        "document_uuid": "00000000-0000-0000-0000-000000000000"
    }
    response3 = requests.post(
        post_address,
        data=json.dumps(payload3),
        headers={
            "Content-Type": "application/json",
            "Authorization": "Bearer " + token,
            "x-api-key": os.getenv('API_KEY')
        }
    )
    # Accept either 200 or 404 for invalid UUID, depending on API behavior
    assert response3.status_code in [200, 404], f"Test 3 failed: {response3.status_code} {response3.text}"
