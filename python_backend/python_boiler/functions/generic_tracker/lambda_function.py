#-*- coding: utf-8 -*-
"""
.. module:: get_hello_world.lambda_function
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
import uuid
from datetime import datetime


def lambda_handler(event, context):

    print("Event:" + str(event))

    jsonResult = ''
    resultJson = {}

    try:
        #we get the parameter
        myEventUUID = str(uuid.uuid4())
        
                
        myKey = event['body']['event_key']
        myValue = event['body']['event_value']
        myadditionalElementJson = event['body']['event_json']
        myTracked = event['body']['event_tracked']

        table = boto3.resource('dynamodb').Table('generic_tracker')
        
        #we insert
        table.put_item(
            Item={
                'event_uuid': str(myEventUUID),
                'event_date':str(datetime.utcnow()),
                'event_key':str(myKey),
                'event_value':str(myValue),
                'event_tracked':str(myTracked),
                'event_json':str(myadditionalElementJson)
                }
        )
        
        
        resultJson['event_uuid'] = str(myEventUUID)
        print(resultJson)
        jsonResult = json.dumps(resultJson, default=str)
        

    except Exception as e:
        print(e)
        print("ISSUE DURING THE PROCESS")
        jsonResult = json.dumps("INVALID_PARAMETERS", default=str)

    finally:
        print("FINALLY CLOSE EVERYTHING")
        return jsonResult
