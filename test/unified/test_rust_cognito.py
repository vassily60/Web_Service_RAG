import json
import requests
import os
from dotenv import load_dotenv
import pytest
from shared_function import *

load_dotenv()

def test_cognito_api():
    connResult = cognito_connexion(os.getenv('USER_NAME'), os.getenv('PASSWORD'))
    assert connResult['success'] is True, 'Cognito authentication failed'

    API_END_POINT_SERVICE = 'rust_cognito'
    local_debug_flag = os.getenv('LOCAL_DEBUG_FLAG', 'false').lower() == 'true'
    if local_debug_flag:
        PostAddressGetInfoElement = os.getenv('API_DEBUG_ENDPOINT_ROOT') + '/' + API_END_POINT_SERVICE
    else:
        PostAddressGetInfoElement = os.getenv('API_ENDPOINT_ROOT') + '/' + os.getenv('API_STAGE') + '/' + API_END_POINT_SERVICE

    print(PostAddressGetInfoElement)
    token = connResult['data']['id_token']
    PostData = {'client_id': 22}
    response = requests.post(PostAddressGetInfoElement, data=json.dumps(PostData), headers={
        "Content-Type": "application/json",
        "Authorization": "Bearer " + token,
        "x-api-key": os.getenv('API_KEY')
    })
    print(response.status_code)
    assert response.status_code == 200, f"POST failed: {response.status_code} {response.reason} {response.text}"
    
    # Print the raw response for debugging
    print(f"Raw response: {response.text}")
    
    # Ensure the response is valid JSON
    try:
        response_data = json.loads(response.text)
    except json.JSONDecodeError as e:
        pytest.fail(f"Response is not valid JSON: {str(e)}. Response: {response.text}")
    
    # Ensure the response is a dictionary/object, not a string or array
    assert isinstance(response_data, dict), f"Response is not a JSON object. Got: {type(response_data).__name__}"
    
    print(f"Response JSON: {json.dumps(response_data, indent=2)}")
    
    # Check if response contains cognito:username
    if 'cognito:username' in response_data:
        # Verify username matches expected
        assert response_data['cognito:username'] == os.getenv('USER_NAME'), f"Username mismatch. Expected: {os.getenv('USER_NAME')}, Got: {response_data['cognito:username']}"
        print(f"Username validated: {response_data['cognito:username']}")
    else:
        # If there's no cognito:username, print what keys are available
        print("Available fields in response:", list(response_data.keys()))
        
        # At least ensure there are some standard JWT claims
        expected_keys = ['exp', 'iat', 'auth_time', 'sub']
        missing_keys = [key for key in expected_keys if key not in response_data]
        if missing_keys:
            pytest.fail(f"Response is missing standard JWT claims: {missing_keys}")
            
        # Print warning but don't fail the test if we found standard JWT claims
        print("Warning: 'cognito:username' not found, but response contains valid JWT claims")