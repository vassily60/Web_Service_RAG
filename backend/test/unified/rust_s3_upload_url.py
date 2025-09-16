import json
import requests
import os
import pytest
from dotenv import load_dotenv
from shared_function import cognito_connexion

load_dotenv()

def run_test_case(token, post_address, post_data, test_name):
    """Helper function to run a test case and print results"""
    print(f"\n=== {test_name} ===")
    
    response = requests.post(
        post_address, 
        data=json.dumps(post_data), 
        headers={
            "Content-Type": "application/json", 
            "Authorization": f"Bearer {token}", 
            "x-api-key": os.getenv('API_KEY')
        }
    )

    print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
    print(f'POST ANSWER: {response.text}')
    
    if response.status_code == 200:
        response_json = response.json()
        print(f"Presigned URL generated: {response_json.get('presigned_url')}")
        print(f"File name: {response_json.get('file_name')}")
        print(f"Bucket: {response_json.get('bucket')}")
        print(f"Key: {response_json.get('key')}")
        print(f"Expiration: {response_json.get('expiration')} seconds")
    else:
        print("Failed to generate presigned URL")
        
    return response.json() if response.status_code == 200 else None
@pytest.fixture(scope="module")
def api_credentials():
    """Fixture to get API credentials and endpoint"""
    conn_result = cognito_connexion(os.getenv('USER_NAME'), os.getenv('PASSWORD'))
    assert conn_result['success'], "Authentication failed"
    
    api_end_point_service = 'rust_s3_upload_url'
    local_debug_flag = os.getenv('LOCAL_DEBUG_FLAG', 'false').lower() == 'true'
    if local_debug_flag:
        post_address = os.getenv('API_DEBUG_ENDPOINT_ROOT') + '/' + api_end_point_service
    else:
        post_address = os.getenv('API_ENDPOINT_ROOT') + '/' + os.getenv('API_STAGE') + '/' + api_end_point_service
    
    return {
        'token': conn_result['data']['id_token'],
        'endpoint': post_address
    }

def test_s3_upload_event(api_credentials):
    """Test Case 1: Basic S3 upload event"""
    post_data = {"bucket": "test-bucket", "key": "test-key"}
    response = requests.post(
        api_credentials['endpoint'],
        data=json.dumps(post_data),
        headers={
            "Content-Type": "application/json", 
            "Authorization": f"Bearer {api_credentials['token']}", 
            "x-api-key": os.getenv('API_KEY')
        }
    )
    
    print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
    print(f'POST ANSWER: {response.text}')
    
    assert response.status_code == 200
    
    response_json = response.json()
    assert 'presigned_url' in response_json
    assert 'bucket' in response_json
    assert 'key' in response_json
    

# Do not accept .png files only pdf
# def test_image_file_upload(api_credentials):
#     """Test Case 2: Image file upload"""
#     post_data = {
#         "file_name": "test_image.png",
#         "content_type": "image/png"
#     }
    
#     response = requests.post(
#         api_credentials['endpoint'],
#         data=json.dumps(post_data),
#         headers={
#             "Content-Type": "application/json", 
#             "Authorization": f"Bearer {api_credentials['token']}", 
#             "x-api-key": os.getenv('API_KEY')
#         }
#     )
    
#     print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
#     print(f'POST ANSWER: {response.text}')
    
#     assert response.status_code == 200
    
#     response_json = response.json()
#     assert 'presigned_url' in response_json
#     assert 'file_name' in response_json
#     assert response_json.get('file_name') == "test_image.png"
#     assert 'content_type' in response_json
#     assert response_json.get('content_type') == "image/png"

# def test_custom_expiration_time(api_credentials):
#     """Test Case 3: Custom expiration time"""
#     post_data = {
#         "file_name": "test_expiration.pdf",
#         "content_type": "application/pdf",
#         "expiration": 3600  # 1 hour
#     }
    
#     response = requests.post(
#         api_credentials['endpoint'],
#         data=json.dumps(post_data),
#         headers={
#             "Content-Type": "application/json", 
#             "Authorization": f"Bearer {api_credentials['token']}", 
#             "x-api-key": os.getenv('API_KEY')
#         }
#     )
    
#     print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
#     print(f'POST ANSWER: {response.text}')
    
#     assert response.status_code == 200
    
#     response_json = response.json()
#     assert 'presigned_url' in response_json
#     assert 'file_name' in response_json
#     assert response_json.get('file_name') == "test_expiration.pdf"
#     assert 'content_type' in response_json
#     assert response_json.get('content_type') == "application/pdf"
#     assert 'expiration' in response_json
#     assert response_json.get('expiration') == 3600


if __name__ == "__main__":
    pytest.main()
