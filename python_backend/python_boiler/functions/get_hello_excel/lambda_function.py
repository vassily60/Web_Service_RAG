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
from os import listdir
from os.path import isfile, join
import xlsxwriter
import uuid


def lambda_handler(event, context):

    print("Event:" + str(event))

    try:

        #we clean the tmp
        onlyfiles = [f for f in listdir('/tmp/') if isfile(join('/tmp', f))]
        #print('file to remove')
        #print(onlyfiles)
        for toberemoved in onlyfiles:
            print("Remove: " + str(toberemoved))
            os.remove('/tmp/'+toberemoved)
        
        jsonResult = ''
        resultJson = {}
        BUCKET = os.environ['S3BUCKET_EXPORT_FOLDER']
        
        OBJECT = 'export' + str(uuid.uuid4()) + '.xlsx';'qrc_0ad85ae7-dd6f-4055-9b86-8cf77d23375f.xlsx'
        #targetFile = 'template_en_gr491_full.xlsx'
        workbook = xlsxwriter.Workbook('/tmp/' + OBJECT)
        worksheet = workbook.add_worksheet('hello')
        worksheet.write('A1', 'Hello world')
        workbook.close()
        
        print('Now we ')
        s3 = boto3.resource("s3")
        s3.meta.client.upload_file('/tmp/' + OBJECT, BUCKET, OBJECT)
        
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
