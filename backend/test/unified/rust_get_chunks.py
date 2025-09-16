import json
import requests
import os
from dotenv import load_dotenv
import pytest
from shared_function import cognito_connexion

load_dotenv()

@pytest.fixture(scope="module")
def api_credentials():
    """Fixture for API credentials and endpoints"""
    # Access environment variables
    user_name = os.getenv('USER_NAME')
    password = os.getenv('PASSWORD')
    api_key = os.getenv('API_KEY')
    local_debug_flag = os.getenv('LOCAL_DEBUG_FLAG', 'false').lower() == 'true'
    api_endpoint_root = os.getenv('API_ENDPOINT_ROOT')
    api_debug_endpoint_root = os.getenv('API_DEBUG_ENDPOINT_ROOT')
    api_stage = os.getenv('API_STAGE')
    
    # First, authenticate with Cognito to get a token
    conn_result = cognito_connexion(user_name, password)
    assert conn_result['success'], "Authentication failed"
    
    # Build base endpoint for all API calls
    def get_endpoint(service):
        if local_debug_flag:
            return api_debug_endpoint_root + '/' + service
        return api_endpoint_root + '/' + api_stage + '/' + service
    
    return {
        'token': conn_result['data']['id_token'],
        'api_key': api_key,
        'get_endpoint': get_endpoint
    }


def display_chunk_results(response):
    """Helper function to display chunk results"""
    if response.status_code != 200:
        print(f"Request failed with status code: {response.status_code}, reason: {response.reason}")
        print(f"Response: {response.text}")
        return None
        
    try:
        response_data = response.json()
    except json.JSONDecodeError:
        print("Failed to parse response JSON")
        return None

    print(f'Response Status: {response.status_code}')
    
    if 'chunks' in response_data:
        if len(response_data['chunks']) > 0:
            print(f"Found {len(response_data['chunks'])} chunks")
            for curr_chunk in response_data['chunks']:
                embebed_text = curr_chunk['embebed_text']
                doc_uuid = curr_chunk['document_uuid']
                print(f'DOCUMENT UUID: {doc_uuid}')
                print('EMBEDDED TEXT: ' + embebed_text[:100] + '...')  # Show only first 100 chars
                
                # Display metadata if it exists
                if 'document_metadata' in curr_chunk and curr_chunk['document_metadata']:
                    print(f"METADATA (count: {len(curr_chunk['document_metadata'])}):")
                    for meta in curr_chunk['document_metadata'][:3]:  # Show only first 3 metadata items
                        meta_name = meta.get('metadata_name', 'Unknown')
                        
                        # Find the non-null value
                        value = None
                        for key in ['metadata_value_string', 'metadata_value_int', 'metadata_value_float', 
                                  'metadata_value_boolean', 'metadata_value_date']:
                            if key in meta and meta[key] is not None:
                                value = meta[key]
                                break
                                
                        print(f"  - {meta_name}: {value}")
                    if len(curr_chunk['document_metadata']) > 3:
                        print(f"  ... and {len(curr_chunk['document_metadata']) - 3} more metadata items")
                else:
                    print("No metadata available for this document")
                print("---")
        else:
            print("No chunks returned in the response")
    else:
        print("No 'chunks' key in response JSON")
    
    return response_data


class TestGetChunks:
    """Test class for the rust_get_chunks API"""
    
    def test_simple_query(self, api_credentials):
        """Test 1: Simple query with no filters"""
        endpoint = api_credentials['get_endpoint']('rust_get_chunks')
        print("\n=== TEST CASE 1: Simple query only ===")
        print("API Endpoint:", endpoint)
        
        # Prepare test data
        test_data = {
            'question': "Redige moi une clause de session",
            'num_results': 10,
            'document_filters': []  # Include an empty document_filters array
        }
        
        # Make the request
        response = requests.post(
            endpoint,
            data=json.dumps(test_data),
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {api_credentials['token']}",
                "x-api-key": api_credentials['api_key']
            }
        )
        
        result = display_chunk_results(response)
        assert response.status_code == 200, f"POST failed: {response.status_code} {response.reason} {response.text}"
        try:
            assert result.get('statusAPI') == 'OK', f"API did not return OK: {result}"
        except (AttributeError, KeyError):
            pytest.fail("Response format is not as expected")
    
    def test_query_with_metadata_filter(self, api_credentials):
        """Test 2: Query with metadata filter"""
        endpoint = api_credentials['get_endpoint']('rust_get_chunks')
        print("\n=== TEST CASE 2: Query + metadata filter ===")
        print("API Endpoint:", endpoint)
        
        # Prepare test data
        test_data = {
            'question': "Redige moi une clause de session",
            'num_results': 10,
            'document_filters': [
                {"filter_type": "metadata", "filter_value": json.dumps({
                    "metadata_uuid": "a9312a58-0290-460e-9938-606d7179df50",
                    "operator": "eq", 
                    "value": "service agreement"
                })}
            ]
        }
        
        # Make the request
        response = requests.post(
            endpoint,
            data=json.dumps(test_data),
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {api_credentials['token']}",
                "x-api-key": api_credentials['api_key']
            }
        )
        
        result = display_chunk_results(response)
        assert response.status_code == 200, f"POST failed: {response.status_code} {response.reason} {response.text}"
        try:
            assert result.get('statusAPI') == 'OK', f"API did not return OK: {result}"
        except (AttributeError, KeyError):
            pytest.fail("Response format is not as expected")
    
    def test_query_with_metadata_and_tag_filter_1(self, api_credentials):
        """Test 3: Query with metadata and tag filter"""
        endpoint = api_credentials['get_endpoint']('rust_get_chunks')
        print("\n=== TEST CASE 3: Query + metadata filter + tag filter ===")
        print("API Endpoint:", endpoint)
        
        # Prepare test data
        test_data = {
            'question': "Redige moi une clause de session",
            'num_results': 10,
            'document_filters': [
                {"filter_type": "metadata", "filter_value": json.dumps({
                    "metadata_uuid": "a9312a58-0290-460e-9938-606d7179df50",
                    "operator": "eq", 
                    "value": "service agreement"
                })}
            ],
            'tags': ["tag1"]
        }
        
        # Make the request
        response = requests.post(
            endpoint,
            data=json.dumps(test_data),
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {api_credentials['token']}",
                "x-api-key": api_credentials['api_key']
            }
        )
        
        result = display_chunk_results(response)
        assert response.status_code == 200, f"POST failed: {response.status_code} {response.reason} {response.text}"
        try:
            assert result.get('statusAPI') == 'OK', f"API did not return OK: {result}"
        except (AttributeError, KeyError):
            pytest.fail("Response format is not as expected")
    
    def test_query_with_metadata_and_tag_filter_2(self, api_credentials):
        """Test 4: Alternative metadata and tag filter"""
        endpoint = api_credentials['get_endpoint']('rust_get_chunks')
        print("\n=== TEST CASE 4: Query + alternative metadata filter + tag filter ===")
        print("API Endpoint:", endpoint)
        
        # Prepare test data
        test_data = {
            'question': "Redige moi une clause de session",
            'num_results': 10,
            'document_filters': [
                {"filter_type": "metadata", "filter_value": json.dumps({
                    "metadata_uuid": "type",
                    "operator": "eq", 
                    "value": "contract"
                })}
            ],
            'tags': ["tag1"]
        }
        
        # Make the request
        response = requests.post(
            endpoint,
            data=json.dumps(test_data),
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {api_credentials['token']}",
                "x-api-key": api_credentials['api_key']
            }
        )
        
        result = display_chunk_results(response)
        assert response.status_code == 200, f"POST failed: {response.status_code} {response.reason} {response.text}"
        try:
            assert result.get('statusAPI') == 'OK', f"API did not return OK: {result}"
        except (AttributeError, KeyError):
            pytest.fail("Response format is not as expected")

if __name__ == "__main__":
    # You can run this file directly with pytest
    # Example: pytest rust_get_chunks.py::TestGetChunks::test_simple_query -v
    # Example: pytest rust_get_chunks.py::TestGetChunks -v
    pytest.main(['-xvs', __file__])
