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


def lambda_handler(event, context):

    print("Event:" + str(event))

    jsonResult = ''
    resultJson = {}

    try:
        print('Hello World')
        jsonResult = json.dumps("'hello_world P!!!!!'", default=str)

    except Exception as e:
        print(e)
        print("ISSUE DURING THE PROCESS")
        jsonResult = json.dumps("INVALID_PARAMETERS", default=str)

    finally:
        print("FINALLY CLOSE EVERYTHING")
        return jsonResult
