import base64
import json
import boto3
import time
import urllib.request
import jose
from jose import jwt, jwk
import typing

class LambdaCustomException404(Exception):
    pass

class LambdaCustomException400(Exception):
    pass

def get_secret(secret_name,region_name) -> str:
    # Create a Secrets Manager client
    session = boto3.session.Session()
    client = session.client(
        service_name='secretsmanager',
        region_name=region_name
    )

    # In this sample we only handle the specific exceptions for the 'GetSecretValue' API.
    # See https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_GetSecretValue.html
    # We rethrow the exception by default.
    secret =  '{"EMPTY":"empty secret"}'
    try:
        get_secret_value_response = client.get_secret_value(
            SecretId=secret_name
        )
        print(get_secret_value_response)
        # Decrypts secret using the associated KMS CMK.
        # Depending on whether the secret is a string or binary, one of these fields will be populated.
        if 'SecretString' in get_secret_value_response:
            secret = get_secret_value_response['SecretString']
            return secret
        else:
            decoded_binary_secret = base64.b64decode(get_secret_value_response['SecretBinary'])
            return decoded_binary_secret

    except Exception as e:
        print(e)
    
    return secret

def decode_token(event,myRegion,myUserPoolId,myAppClientId):
    jsonResult = ''
    resultJson = {}

    try:
        print('REGION: ' + str(myRegion))
        print('USER_POOL_ID: ' + str(myUserPoolId))
        keys_url = 'https://cognito-idp.{}.amazonaws.com/{}/.well-known/jwks.json'.format(myRegion, myUserPoolId)
        # instead of re-downloading the public keys every time
        # we download them only on cold start
        # https://aws.amazon.com/blogs/compute/container-reuse-in-lambda/
        print(keys_url)
        with urllib.request.urlopen(keys_url) as f:
            response = f.read()
        keys = json.loads(response.decode('utf-8'))['keys']

        token = event['headers']['Authorization'].replace('Bearer ','')
        # get the kid from the headers prior to verification
        headers = jwt.get_unverified_headers(token)
        kid = headers['kid']
        # search for the kid in the downloaded public keys
        key_index = -1

        resultJson['message'] = ''
        for i in range(len(keys)):
            if kid == keys[i]['kid']:
                key_index = i
                break
        if key_index == -1:
            print('Public key not found in jwks.json')
            resultJson['message'] = 'Public key not found in jwks.json'
            return False
        
        # construct the public key
        public_key = jwk.construct(keys[key_index])
        # get the last two sections of the token,
        # message and signature (encoded in base64)
        message, encoded_signature = str(token).rsplit('.', 1)
        # decode the signature
        decoded_signature = jose.utils.base64url_decode(encoded_signature.encode('utf-8'))
        # verify the signature
        if not public_key.verify(message.encode("utf8"), decoded_signature):
            print('Signature verification failed')
            return False
        print('Signature successfully verified')
        # since we passed the verification, we can now safely
        # use the unverified claims
        claims = jose.jwt.get_unverified_claims(token)
        # additionally we can verify the token expiration
        if time.time() > claims['exp']:
            print('Token is expired')
            return False
        # and the Audience  (use claims['client_id'] if verifying an access token)
        if claims['aud'] != myAppClientId:
            print('Token was not issued for this audience')
            return False
        # now we can use the claims
        print(claims)
        
        resultJson['message'] = 'Token successfully decoded in dev . See logs.'
        resultJson['claims'] = claims

    except Exception as e:
        print(e)
        print("ISSUE DURING THE DECODING OF THE TOKEN")
        resultJson['message'] ="ERROR"

    finally:
        print("END DECODE TOKEN")
        return resultJson