import boto3
import sys
import json
import requests
from config_local import *
# the shared function
import os
# sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))
from shared_function import *

# First, authenticate with Cognito to get a token
connResult = cognito_connexion(USER_NAME, PASSWORD)

# ----------------------------------------------------------
# Now call the API GATEWAY
# ----------------------------------------------------------
if connResult['success'] == True:
    API_END_POINT_SERVICE = 'rust_update_metadata'

    # API_ENDPOINT_ROOT from config file
    if LOCAL_DEBUG_FLAG == True:
        PostAddressUpdateMetadata = API_DEBUG_ENDPOINT_ROOT + '/' + API_END_POINT_SERVICE
    else:
        PostAddressUpdateMetadata = API_ENDPOINT_ROOT + '/' + API_STAGE + '/' + API_END_POINT_SERVICE

    print("API Endpoint:", PostAddressUpdateMetadata)

    # Get the token from Cognito authentication
    token = connResult['data']['id_token']
    
    # Prepare data for updating an existing metadata
    # Note: Replace this with an actual existing metadata_uuid
    update_data = {
        "metadata_uuid": "abcd1234-e89b-12d3-a456-426614174000",  # Replace with actual UUID
        "metadata_name": "updated_document_category",
        "metadata_description": "Updated category description",
        "metadata_type": "string"
    }

    # Make the PUT request to update metadata
    response = requests.put(
        PostAddressUpdateMetadata, 
        data=json.dumps(update_data), 
        headers={
            "Content-Type": "application/json", 
            "Authorization": "Bearer " + token, 
            "x-api-key": API_KEY
        }
    )

    print('UPDATE STATUS: ' + str(response.status_code) + ' REASON: ' + str(response.reason))
    print('UPDATE RESPONSE: ' + response.text)
else:
    print('Error connecting to Cognito')