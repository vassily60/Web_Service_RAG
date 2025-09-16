import os
import json
import requests
from dotenv import load_dotenv

import os
import json
import requests
from dotenv import load_dotenv
import pytest
from shared_function import cognito_connexion

load_dotenv()

@pytest.mark.integration
def test_rust_document_list():
    connResult = cognito_connexion(os.getenv('USER_NAME'), os.getenv('PASSWORD'))
    assert connResult['success'] is True, 'Cognito authentication failed'

    API_END_POINT_SERVICE = 'rust_document_list'
    local_debug_flag = os.getenv('LOCAL_DEBUG_FLAG', 'false').lower() == 'true'
    if local_debug_flag:
        PostAddressGetInfoElement = os.getenv('API_DEBUG_ENDPOINT_ROOT') + '/' + API_END_POINT_SERVICE
    else:
        PostAddressGetInfoElement = os.getenv('API_ENDPOINT_ROOT') + '/' + os.getenv('API_STAGE') + '/' + API_END_POINT_SERVICE

    token = connResult['data']['id_token']

    # Test case 1: Basic request without filters
    PostData = {}
    response = requests.post(
        PostAddressGetInfoElement,
        data=json.dumps(PostData),
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + token, "x-api-key": os.getenv('API_KEY')}
    )
    assert response.status_code == 200, f"Test case 1 failed: {response.status_code} {response.text}"

    # Test case 2: Request with tags
    PostData = {"tags": os.getenv('TEST_TAGS', 'example_tag')}
    response = requests.post(
        PostAddressGetInfoElement,
        data=json.dumps(PostData),
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + token, "x-api-key": os.getenv('API_KEY')}
    )
    assert response.status_code == 200, f"Test case 2 failed: {response.status_code} {response.text}"

    # Test case 3: Request with metadata filter
    PostData = {"metadata_uuid": os.getenv('TEST_METADATA_UUID', 'dummy-metadata-uuid')}
    response = requests.post(
        PostAddressGetInfoElement,
        data=json.dumps(PostData),
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + token, "x-api-key": os.getenv('API_KEY')}
    )
    assert response.status_code == 200, f"Test case 3 failed: {response.status_code} {response.text}"

    # Test case 4: Request with both tags and metadata filter
    PostData = {
        "tags": os.getenv('TEST_TAGS', 'example_tag'),
        "metadata_uuid": os.getenv('TEST_METADATA_UUID', 'dummy-metadata-uuid')
    }
    response = requests.post(
        PostAddressGetInfoElement,
        data=json.dumps(PostData),
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + token, "x-api-key": os.getenv('API_KEY')}
    )
    assert response.status_code == 200, f"Test case 4 failed: {response.status_code} {response.text}"

