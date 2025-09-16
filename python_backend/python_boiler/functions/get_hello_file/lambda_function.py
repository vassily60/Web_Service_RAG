#-*- coding: utf-8 -*-
"""
.. module:: get_hello_cognito.lambda_function
   :synopsis:
.. moduleauthor:: bfoucque
# """
# """
# Created on 02/08/2022
# @author: Benoit
# """

import os
import json
import boto3
from ast import literal_eval
from botocore.exceptions import ClientError


def lambda_handler(event, context):

    print("Event:" + str(event))

    

    try:
        jsonResult = ''
        resultJson = {}
        BUCKET = 'nicomatic-demo-blochchain'
        OBJECT = 'qrc_0ad85ae7-dd6f-4055-9b86-8cf77d23375f.png'
        print('Hello Cognito')
        
        s3_client = boto3.client('s3')
        url = s3_client.generate_presigned_url('get_object',    Params={'Bucket': BUCKET, 'Key': OBJECT},    ExpiresIn=300)
        resultJson['file'] = url
        jsonResult = json.dumps(resultJson, default=str)

    except Exception as e:
        print(e)
        print("ISSUE DURING THE PROCESS")
        jsonResult = json.dumps("INVALID_PARAMETERS", default=str)

    finally:
        print("FINALLY CLOSE EVERYTHING")
        return jsonResult
