import boto3
import hmac
import hashlib
import base64
import os
from dotenv import load_dotenv
load_dotenv()



def get_secret_hash(username):
    msg = username + os.getenv('CLIENT_ID')
    dig = hmac.new(str(os.getenv('CLIENT_SECRET')).encode('utf-8'),
    msg = str(msg).encode('utf-8'), digestmod=hashlib.sha256).digest()
    d2 = base64.b64encode(dig).decode()
    return d2

def initiate_auth(client, username, password):
    secret_hash_flag = os.getenv('SECRET_HASH_FLAG', 'true').lower() == 'true'
    auth_params = {
        'USERNAME': username,
        'PASSWORD': password,
    }
    if secret_hash_flag:
        secret_hash = get_secret_hash(username) # or os.getenv('SECRET_HASH')  
        auth_params['SECRET_HASH'] = secret_hash
    try:
        resp = client.initiate_auth(
                    #UserPoolId=USER_POOL_ID, #perhaps only when the secret is not mandatory
                    ClientId=os.getenv('CLIENT_ID'),
                    AuthFlow='USER_PASSWORD_AUTH',
                    AuthParameters=auth_params,
                ClientMetadata={
                    'username': username,
                    'password': password,
                })
        # print(resp)
    except client.exceptions.NotAuthorizedException:
        return None, "The username or password is incorrect"
    except client.exceptions.UserNotConfirmedException:
        return None, "User is not confirmed"
    except Exception as e:
        return None, e.__str__()
    return resp, None

def cognito_connexion(username, password):
    client = boto3.client('cognito-idp')

    '''for field in ["username", "password"]:
        if event.get(field) is None:
        return  {"error": True, 
                "success": False, 
                "message": f"{field} is required", 
                "data": None}'''

    resp, msg = initiate_auth(client, username, password)
    if msg != None:
        return {'message': msg, 
                "error": True, "success": False, "data": None}

    if resp.get("AuthenticationResult"):
        return {'message': "success", 
                "error": False, 
                "success": True, 
                "data": {
                "id_token": resp["AuthenticationResult"]["IdToken"],
                "refresh_token": resp["AuthenticationResult"]["RefreshToken"],
                "access_token": resp["AuthenticationResult"]["AccessToken"],
                "expires_in": resp["AuthenticationResult"]["ExpiresIn"],
                "token_type": resp["AuthenticationResult"]["TokenType"]
                        }
                }
    else:
        return {"error": True, 
            "success": False, 
            "data": None, "message": None}