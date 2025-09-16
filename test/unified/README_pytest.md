# Running Unified Tests with Pytest

## Prerequisites
- Python 3.7+
- Install dependencies: `pip install -r requirements.txt` (ensure `pytest` and `python-dotenv` are included)
- Ensure `.env` file is present in the test folder with all required variables

## Running Tests

### Run all tests
```bash
pytest
```

### Run tests in parallel (requires pytest-xdist)
```bash
pytest -n auto
```

### Run tests with markers
```bash
pytest -m <marker>
```

### Run a specific test file
```bash
pytest path/to/test_file.py
```

### Run with environment variables
Pytest will automatically load variables from `.env` via `python-dotenv`.

## Best Practices
- Use `assert` statements to validate all test results
- Structure test files with clear setup, execution, and assertion sections
- Use pytest fixtures for setup/teardown
- Add tests to cover all edge and error cases
- Ensure all tests are discoverable (test file and function names should start with `test_`)

## Example Test Structure
```python
import os
from dotenv import load_dotenv
import pytest

load_dotenv()

def test_example():
    api_key = os.getenv('API_KEY')
    assert api_key is not None
```
