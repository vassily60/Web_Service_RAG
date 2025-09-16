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
import csv
from functions.shared.shared_functions import *
from functions.shared.shared_snowflake import *


# ------------------  GLOBAL ---------------------------------------------------------------------------
#my globals: UGLY TENDER, ULY TRUE :-)
s3_client = boto3.client('s3')
ses = boto3.client('ses')
resourceS3 = boto3.resource('s3')
# ------------------ END GLOBAL ------------------------------------------------------------------------

# ------------------  SECRET (TO REUSE BEFORE RECYCLING) -----------------------------------------------

secret_name = os.environ['DATABASE_CONECTION_STRING']
print('secret_name: ' + str(secret_name))
region_name = os.environ['REGION']
snowFlakeDict = json.loads(get_secret(secret_name,region_name))
# ------------------  END SECRET   ---------------------------------------------------------------------

def lambda_handler(event, context):

    print("Event:" + str(event))

    jsonResult = ''
    resultJson = {}

    try:
        #set up connection
        ctx = generateSnowConnection(snowFlakeDict)
        cursor = ctx.cursor(DictCursor)
        
        myDataBase = os.environ['DATABASE']
        print("myDataBase: " + str(myDataBase))
        cursor.execute("USE DATABASE " + myDataBase)

        myRequest = """SELECT 
                        UUID_STRING() AS SESSION_UUID
                        ;"""
        myRequest = """SELECT qesrs.ESRS_PARAGRAPH AS ESRS_PARAGRAPH FROM SUSTAINABILITY_PLATFORM.ESRS.ESRS_QUESTION qesrs LIMIT 5;"""


        print(myRequest)
        cursor.execute(myRequest)
        resultRows = cursor.fetchall()

        resultJson['question_list'] = []
        
        for resultRow in resultRows:
            questionItem = {}
            questionItem['SESSION_UUID']= resultRow['SESSION_UUID']
            resultJson['question_list'].append(questionItem)
        
        #we answer
        jsonResult = json.dumps(resultJson, default=str)

    except Exception as e:
        print(e)
        print("ISSUE DURING THE PROCESS GET ANSWER FOR ONE QUESTION")
        jsonResult = json.dumps("INVALID_PARAMETERS", default=str)

    finally:
        print("FINALLY CLOSE EVERYTHING")
        return jsonResult
