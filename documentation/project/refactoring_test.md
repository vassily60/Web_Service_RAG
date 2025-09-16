# Refactoring test

- [] rust_add_metadata.py
- [x] rust_cognito.py
- [] rust_compute_metadata.py
- [?] rust_compute_synonym.py
- [] rust_delete_metadata.py
- [] rust_document_list.py
- [] rust_document_presigned_url.py
- [x] rust_dynamo.py
- [] rust_get_chunks.py
- [] rust_get_document_metadatas.py
- [] rust_get_metadata.py
- [x] rust_hello.py
- [x] rust_json.py
- [] rust_metadata.py
- [] rust_openai_answer.py
- [] rust_recurrent_query.py
- [] rust_s3_upload_url.py
- [x] rust_secret.py
- [] rust_snowflake.py
- [x] rust_synonyms.py
- [] rust_update_metadata.py
- [] rust_update_tags.py
- [x] shared_function.py

## Refactoring rust_secret

We want to refactor the code and the test for rust_secret.py and rust_secretto improve readability and maintainability. This includes:

1. **Code Organization**: Grouping related functions and classes together, and separating concerns where possible.
2. **Naming Conventions**: Ensuring that all functions, variables, and classes follow a consistent naming convention.
3. **Test Coverage**: Adding tests for any new functionality and ensuring existing tests still pass.
4. **Documentation**: Updating docstrings and comments to accurately describe the code's behavior.

Please use the code_rules provided in the project documentation to guide your refactoring process. The goal is to make the code cleaner and easier to understand while maintaining its functionality.

## Refactoring rust_dynamo
We want to refactor the code and the test for rust_dynamo.py to improve readability and maintainability. This includes:
1. **Code Organization**: Grouping related functions and classes together, and separating concerns where possible.
2. **Naming Conventions**: Ensuring that all functions, variables, and classes follow a consistent naming convention.
3. **Test Coverage**: Adding tests for any new functionality and ensuring existing tests still pass.
4. **Documentation**: Updating docstrings and comments to accurately describe the code's behavior.  

Please add a new key in the .env named TEST_DYNAMO_TABLE_NAME and use it in the code.

Please use the code_rules provided in the project documentation to guide your refactoring process. The goal is to make the code cleaner and easier to understand while maintaining its functionality.

## Refactoring rust_json

We want to refactor the code and the test for rust_secret.py and rust_secretto improve readability and maintainability. This includes:

1. **Code Organization**: Grouping related functions and classes together, and separating concerns where possible.
2. **Naming Conventions**: Ensuring that all functions, variables, and classes follow a consistent naming convention.
3. **Test Coverage**: Adding tests for any new functionality and ensuring existing tests still pass.
4. **Documentation**: Updating docstrings and comments to accurately describe the code's behavior.

Please use the code_rules provided in the project documentation to guide your refactoring process. The goal is to make the code cleaner and easier to understand while maintaining its functionality.


## Refactoring rust_compute_synonym

I want to refactor the code and the test for rust_compute_synonym.py to improve readability and maintainability. This includes:
1. **Code Organization**: Grouping related functions and classes together, and separating concerns where possible.
2. **Naming Conventions**: Ensuring that all functions, variables, and classes follow a consistent naming convention.
3. **Test Coverage**: Adding tests for any new functionality and ensuring existing tests still pass.
4. **Documentation**: Updating docstrings and comments to accurately describe
    the code's behavior.   

Please use the code_rules provided in the project documentation to guide your refactoring process. The goal is to make the code cleaner and easier to understand while maintaining its functionality.

For the test rust_compute_synonym.py, please ensure that it covers the following scenarios:
- you need to create a new synonym using the create_synonym method 
- call the compute_snonym method with a string that must be replace by the compute sysnonym
- verify that the string is replaced by the compute synonym created by the create synonym method in the first step.
- delete the synonym created previously using the delete_synonym method
  


Here is what the lambda function does: 
Main Components
Request Handling:

The Lambda function receives HTTP requests with a JSON body containing a query string
It validates that the request contains a non-empty query
Database Connection:

Retrieves database credentials from AWS Secrets Manager
Establishes a secure connection to a PostgreSQL database using TLS
Synonym Processing:

Fetches a list of synonyms from the document_library.synonyms table
Each synonym has a synonym_name (the original word) and a synonym_value (its replacement)
Uses regular expressions to find exact word matches in the query
Transforms the query by replacing matched words with a format like: original_word or synonym_value
Response Generation:

Returns a JSON response containing:
statusAPI: "OK" or "ERROR"
original_query: The unmodified input query
processed_query: The query after synonym replacement


