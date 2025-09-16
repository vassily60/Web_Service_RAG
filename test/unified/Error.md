# Test with error

rust_compute_metadata.py : error with env variable document uuid

rust_delete_metadata.py : error with env variable metadata uuid

rust_delete_recurrent_query.py : error with env variable recurrent query uuid ``

rust_document_list.py : error with env variable tags    

rust_document_presigned_url.py : error with env variable document uuid

rust_dynamo.py : no error test, all test good but found this in lambda watch Error: Service(ResourceNotFound("Requested resource    notfound"))

rust_get_chunks.py : error with env variable document uuid and test seems weird

rust_get_document_metadatas.py: error with env variable document uuid

rust_s3_upload_url.py: FAILED rust_s3_upload_url.py::test_s3_upload_event - assert 400 == 200
FAILED rust_s3_upload_url.py::test_image_file_upload - AssertionError: assert 'content_type' in {'bucket': 'paloit-cve', 'expiration': 900, 'file_name': 'test_imag...
FAILED rust_s3_upload_url.py::test_custom_expiration_time - AssertionError: assert 'content_type' in {'bucket': 'paloit-cve', 'expiration': 3600, 'file_name': 'test_exp...

rust_snowflake.py: error with env TEST_CLIENT_ID

rust_update_metadata.py : error with env variable metadata uuid

rust_update_recurrent_query.py : an actual recurrent_query_uuid from your database

rust_update_tags.py : error with env variable tags TEST_DOCUMENT_UUID


