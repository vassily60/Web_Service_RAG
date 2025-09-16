#!/usr/bin/env python3
# This is a test script for the rust_pdf_file_integration Lambda function

import boto3
import json
import os
import time
from datetime import datetime
import uuid
import argparse

def parse_args():
    parser = argparse.ArgumentParser(description='Test the PDF file integration Lambda')
    parser.add_argument('--bucket', type=str, default='paloit-cve',
                       help='The name of the source S3 bucket')
    parser.add_argument('--file', type=str, required=True,
                       help='The local path to a PDF file to upload')
    parser.add_argument('--prefix', type=str, default='imported/',
                       help='The prefix to use in the source bucket')
    parser.add_argument('--region', type=str, default='ap-southeast-1',
                       help='AWS region')
    return parser.parse_args()

def upload_file_to_s3(bucket_name, file_path, prefix, region):
    """
    Uploads a file to S3 bucket to trigger the Lambda function
    """
    s3_client = boto3.client('s3', region_name=region)
    
    # Generate a unique key using the current timestamp and a UUID
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    unique_id = str(uuid.uuid4())[:8]
    
    # Extract filename from path and create S3 key
    filename = os.path.basename(file_path)
    s3_key = f"{prefix}{timestamp}_{unique_id}_{filename}"
    
    print(f"Uploading {file_path} to s3://{bucket_name}/{s3_key}...")
    
    # Upload file to S3
    try:
        s3_client.upload_file(file_path, bucket_name, s3_key)
        print(f"Successfully uploaded file to s3://{bucket_name}/{s3_key}")
        return s3_key
    except Exception as e:
        print(f"Error uploading file: {e}")
        return None

def check_document_in_db(file_hash, region):
    """
    Checks if the document exists in the database by connecting to the backend API
    This is a simplified placeholder function - in a real test you would need to query the database
    """
    print(f"Checking if document with hash {file_hash} exists in database...")
    # In a real test, you would query your backend API or directly connect to the database
    
    # Example: Sleep for 5 seconds to simulate waiting for Lambda to complete processing
    time.sleep(5)
    print("Document should have been processed by Lambda at this point.")
    
    return True

def main():
    """
    Main function to test the PDF file integration Lambda
    """
    args = parse_args()
    
    # Validate file exists and is a PDF
    if not os.path.isfile(args.file):
        print(f"Error: File {args.file} does not exist!")
        return 1
    
    if not args.file.lower().endswith('.pdf'):
        print(f"Error: File {args.file} is not a PDF!")
        return 1
    
    # Upload file to S3
    s3_key = upload_file_to_s3(args.bucket, args.file, args.prefix, args.region)
    if not s3_key:
        return 1
    
    print("Waiting for Lambda to process the file...")
    print("This test script doesn't actually check the database.")
    print("To verify proper processing, check:")
    print("1. CloudWatch logs for your Lambda function")
    print(f"2. The destination bucket for the processed file")
    print("3. The database for the document record and chunks")
    
    return 0

if __name__ == "__main__":
    exit(main())
