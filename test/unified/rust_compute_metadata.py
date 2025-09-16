
import os
import json
import requests
from dotenv import load_dotenv
import pytest
from shared_function import cognito_connexion

load_dotenv()

@pytest.mark.integration
def test_rust_compute_metadata():
    connResult = cognito_connexion(os.getenv('USER_NAME'), os.getenv('PASSWORD'))
    assert connResult['success'] is True, 'Cognito authentication failed'

    API_END_POINT_SERVICE = 'rust_compute_metadata'
    local_debug_flag = os.getenv('LOCAL_DEBUG_FLAG', 'false').lower() == 'true'
    if local_debug_flag:
        PostAddressGetInfoElement = os.getenv('API_DEBUG_ENDPOINT_ROOT') + '/' + API_END_POINT_SERVICE
    else:
        PostAddressGetInfoElement = os.getenv('API_ENDPOINT_ROOT') + '/' + os.getenv('API_STAGE') + '/' + API_END_POINT_SERVICE

    token = connResult['data']['id_token']

    # Test case 1: Process metadata for a specific document
    PostData = {"document_uuid": os.getenv('TEST_DOCUMENT_UUID', '26ca4b89-7af4-4527-961f-b8db48ec9e95')}
    response = requests.post(
        PostAddressGetInfoElement,
        data=json.dumps(PostData),
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + token, "x-api-key": os.getenv('API_KEY')}
    )
    assert response.status_code == 200, f"Test case 1 failed: {response.status_code} {response.text}"

    # Test case 2: Process a specific metadata field for all documents
    PostData = {"metadata_uuid": os.getenv('TEST_METADATA_UUID', '650ffea0-2c50-457b-8b24-ab2aeae4a13a')}
    response = requests.post(
        PostAddressGetInfoElement,
        data=json.dumps(PostData),
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + token, "x-api-key": os.getenv('API_KEY')}
    )
    assert response.status_code == 200, f"Test case 2 failed: {response.status_code} {response.text}"

    # Test case 3: Process a specific metadata field for a specific document
    PostData = {
        "document_uuid": os.getenv('TEST_DOCUMENT_UUID', '26ca4b89-7af4-4527-961f-b8db48ec9e95'),
        "metadata_uuid": os.getenv('TEST_METADATA_UUID', '650ffea0-2c50-457b-8b24-ab2aeae4a13a')
    }
    response = requests.post(
        PostAddressGetInfoElement,
        data=json.dumps(PostData),
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + token, "x-api-key": os.getenv('API_KEY')}
    )
    assert response.status_code == 200, f"Test case 3 failed: {response.status_code} {response.text}"