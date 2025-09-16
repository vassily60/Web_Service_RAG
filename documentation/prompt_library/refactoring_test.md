# REFACTORING TEST

## REFACTORING EXISTING TESTS

We want to refactor the originals tests. The tests are located in the test/individual folder.

We want to:

- remove the config_local.py and replace it by a .env file.
- align the way that the tests are called using: pytest and the command line:
- align the internal structure of test files
- systematically use an assert function to validate the results of the tests.
- test the S3 event properly, so fix the bug of the 500 error.
- store all the test at the root of the folder, and 1 sub folder per service triggered by S3 event to store the json example.
- align swagger with test files.
- remove all the unused import

---

### Remove the config_local.py and replace it by a .env file

We want to:

- remove the config_local.py file.
- create a .env file with the same variables as in config_local.py.
- use the python-dotenv package to load the .env file in the tests.
- update all the test files in the unified folder to use the .env file instead of config_local.py.

---

### Align the way that the tests are called using: pytest and the command line

We want to:

- ensure that all tests can be run using the pytest command.
- ensure that the tests can be run from the command line without any issues.
- update the test files to use the pytest framework.
- ensure that the tests are discoverable by pytest.
- ensure that the tests can be run in parallel if needed.
- ensure that the tests can be run with specific markers if needed.
- ensure that the tests can be run with specific options if needed.
- ensure that the tests can be run with specific configurations if needed.
- ensure that the tests can be run with specific environment variables if needed.

update all the test files in the unified folder to use the pytest framework. Please properly describe all the steps to run the tests using pytest in a dedicated document .md.
Do not forget to systematically use an assert function to validate the results of the tests. 
Add tests to cover all the cases.
Please analyse the existing tests and ensure that they are properly aligned with the pytest framework. 

---

## IMPROVE WS Code

We want to:

- remove all the default env variable in the code of the webservices
- remove all the unused import
- clean all the warning
