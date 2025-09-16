#-*- coding: utf-8 -*-
"""
.. module:: get_decode_token.lambda_function
   :synopsis: this function decode token
.. moduleauthor:: bfoucque
"""
# """
# Created on 02/08/2022
# @author: Benoit
# """

import json
import boto3
from botocore.exceptions import ClientError
import socket as s
import os
#the shared function
from functions.shared.shared_functions import *

#credit for database
'''
try:
    credsDicCognito
except:
'''    
secret_name = os.environ['COGNITO_SECRET']
#endpoint_url = ""
region_name = os.environ['REGION']

credsDicCognito = json.loads(get_secret(secret_name,region_name))
print(credsDicCognito)
#credsDicCognito = literal_eval(get_secret(secret_name,endpoint_url,region_name))


def lambda_handler(event, context):
    #we always print the event first
    print("Event:" + str(event))

    
    try:
        resultJson= decode_token(event,credsDicCognito['REGION'],credsDicCognito['USER_POOL_ID'],credsDicCognito['APP_CLIENT_ID'])
        print(resultJson)
        jsonResult = json.dumps(resultJson, default=str)

    except Exception as e:
        print(e)
        print("ISSUE DURING THE PROCESS")
        jsonResult = json.dumps("INVALID_PARAMETERS", default=str)

    finally:
        print("FINALLY CLOSE EVERYTHING")
        return jsonResult
