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
#the shared function
from functions.shared.shared_functions import *



def lambda_handler(event, context):

    print("Event:" + str(event))

    jsonResult = ''
    resultJson = {}
    resultJson['statusAPI'] = 'SUCCESS'
    resultJson['statusAPICODE'] = 'SUCCESS'
    resultJson['errorMessage'] = ''

    try:
        print('Hello World')
        
        if event["body"]["statusCode"] != 'SUCCESS':
            if event["body"]["statusCode"] == 'UNKNOWNRESSOURCE':
                raise LambdaCustomException404('exception message UNKNOWNRESSOURCE')
            if event["body"]["statusCode"] == 'OPERATIONFAIL':
                raise LambdaCustomException400('exception message OPERATIONFAIL')
            else:
                raise Exception('This is a standart error message')
        
        resultJson['message'] = 'Hello World!'
        print('Finalize success')
        jsonResult = json.dumps(resultJson, default=str)
        return jsonResult

    except LambdaCustomException404 as myCustoException:
        print(myCustoException)
        print("ISSUE DURING THE PROCESS: 404")
        resultJson['statusAPI'] = 'ERROR'
        resultJson['errorMessage'] = str(myCustoException)
        resultJson['statusAPICODE'] = 'UNKNOWNRESSOURCE'
        jsonResult = json.dumps(resultJson, default=str)
        raise Exception(jsonResult)
    #
    except LambdaCustomException400 as myCustoException:
        print(myCustoException)
        print("ISSUE DURING THE PROCESS: 404")
        resultJson['statusAPI'] = 'ERROR'
        resultJson['errorMessage'] = str(myCustoException)
        resultJson['statusAPICODE'] = 'OPERATIONFAIL'
        jsonResult = json.dumps(resultJson, default=str)
        raise Exception(jsonResult)
    #
    except Exception as e:
        print(e)
        print("ISSUE DURING THE PROCESS")
        resultJson['statusAPI'] = 'ERROR'
        resultJson['errorMessage'] = str(e)
        resultJson['statusAPICODE'] = 'CRASH'
        print('la')
        jsonResult = json.dumps(resultJson, default=str)
        raise Exception(jsonResult)