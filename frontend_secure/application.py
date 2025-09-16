from os import environ, path, urandom
import platform
import datetime
import flask
import requests
import json
import urllib.parse
from dotenv import load_dotenv
from flask import Flask, jsonify, redirect, render_template, request, session, url_for

from flask_cognito_lib import CognitoAuth
from flask_cognito_lib.decorators import (
    auth_required,
    cognito_login,
    cognito_login_callback,
    cognito_logout,
    cognito_refresh_callback,
    get_token_from_cookie
)
from flask_cognito_lib.exceptions import (
    AuthorisationRequiredError,
    CognitoGroupRequiredError,
)

# Load variables from .env
basedir = path.abspath(path.dirname(__file__))
load_dotenv(path.join(basedir, ".env"))


class Config:
    """Set Flask configuration vars from .env file."""

    # General Config
    SECRET_KEY = environ.get("SECRET_KEY", urandom(32))
    FLASK_application = "TEST_application"
    FLASK_ENV = "TESTING"

    # Cognito config
    # AWS_COGNITO_DISABLED = True  # Can set to turn off auth (e.g. for local testing)
    AWS_REGION = environ["AWS_REGION"]
    AWS_COGNITO_USER_POOL_ID = environ["AWS_COGNITO_USER_POOL_ID"]
    AWS_COGNITO_DOMAIN = environ["AWS_COGNITO_DOMAIN"]
    AWS_COGNITO_USER_POOL_CLIENT_ID = environ["AWS_COGNITO_USER_POOL_CLIENT_ID"]
    AWS_COGNITO_USER_POOL_CLIENT_SECRET = environ["AWS_COGNITO_USER_POOL_CLIENT_SECRET"]
    AWS_COGNITO_REDIRECT_URL = environ["AWS_COGNITO_REDIRECT_URL"]
    AWS_COGNITO_LOGOUT_URL = environ["AWS_COGNITO_LOGOUT_URL"]

    # BI Dashboard URL
    SUPERSET_URL = environ.get("SUPERSET_URL", "http://superset.palo-it.hk:8080/")

    # API Configuration
    API_KEY = environ.get("API_KEY", "")
    API_ENDPOINT_ROOT = environ.get("API_ENDPOINT_ROOT", "")
    API_STAGE = environ.get("API_STAGE", "dev")

    # Optional
    # AWS_COGNITO_COOKIE_AGE_SECONDS = environ["AWS_COGNITO_COOKIE_AGE_SECONDS"]
    # AWS_COGNITO_EXPIRATION_LEEWAY = environ["AWS_COGNITO_EXPIRATION_LEEWAY]
    # AWS_COGNITO_SCOPES = ["openid", "phone", "email"]
    # AWS_COGNITO_REFRESH_FLOW_ENABLED = environ["AWS_COGNITO_REFRESH_FLOW_ENABLED"]
    # AWS_COGNITO_REFRESH_COOKIE_ENCRYPTED = environ["AWS_COGNITO_REFRESH_COOKIE_ENCRYPTED"]
    # AWS_COGNITO_REFRESH_COOKIE_AGE_SECONDS = environ["AWS_COGNITO_REFRESH_COOKIE_AGE_SECONDS"]

    # configuration for local debug
    LOCAL_DEBUG = environ.get("LOCAL_DEBUG", False)
    LOCAL_DEBUG_PORT = environ.get("LOCAL_DEBUG_PORT", 5001)



application = Flask(__name__)
application.config.from_object(Config)
auth = CognitoAuth(application)


@application.route("/")
def home():
    return render_template("home.html")


@application.route("/login")
@cognito_login
def login():
    # A simple route that will redirect to the Cognito Hosted UI.
    # No logic is required as the decorator handles the redirect to the Cognito
    # hosted UI for the user to sign in.
    # An optional "state" value can be set in the current session which will
    # be passed and then used in the postlogin route (after the user has logged
    # into the Cognito hosted UI); this could be used for dynamic redirects,
    # for example, set `session['state'] = "some_custom_value"` before passing
    # the user to this route
    pass


@application.route("/postlogin")
@cognito_login_callback
def postlogin():
    return redirect(url_for("welcomepage"))


@application.route("/refresh", methods=["POST"])
@cognito_refresh_callback
def refresh():
    # A route to handle the token refresh with Cognito.
    # The decorator will exchange the refresh token for new access and refresh tokens.
    # The new validated access token will be stored in an HTTP only secure cookie.
    # The refresh token will be symmetrically encrypted(by default)
    # and stored in an HTTP only secure cookie.
    # The user claims and info are stored in the Flask session:
    # session["claims"] and session["user_info"].
    # Do anything after the user has refreshed access token here, e.g. a redirect
    # or perform logic based on the `session["user_info"]`.
    pass



@application.errorhandler(AuthorisationRequiredError)
def auth_error_handler(err):
    # Register an error handler if the user hits an "@auth_required" route
    # but is not logged in to redirect them to the Cognito UI
    return redirect(url_for("login"))


@application.errorhandler(CognitoGroupRequiredError)
def missing_group_error_handler(err):
    # Register an error handler if the user hits an "@auth_required" route
    # but is not in all of groups specified
    return render_template("missinggroup.html"), 403


@application.route("/logout")
@cognito_logout
def logout():
    # Logout of the Cognito User pool and delete the cookies that were set
    # on login.
    # Revokes the refresh token to not be used again and removes the cookie.
    # No logic is required here as it simply redirects to Cognito.
    pass


@application.route("/postlogout")
def postlogout():
    # This is the endpoint Cognito redirects to after a user has logged out,
    # handle any logic here, like returning to the homepage.
    # This route must be set as one of the User Pool client's Sign Out URLs.
    return redirect(url_for("home"))


@application.route("/welcome")
@auth_required()
def welcomepage():
    return render_template("welcome.html")


@application.route("/search", methods=["GET", "POST"])
@auth_required()
def search():
    if request.method == "POST":
        # Process JSON request for AJAX searches
        if request.is_json:
            json_data = request.get_json()
            search_query = json_data.get("query", "")
            tags = json_data.get("tags", [])
            limit = json_data.get("limit", 5)
            metadata_filters = json_data.get("metadata_filters", [])
            
            # Get the access token from auth instance
            access_token = get_token_from_cookie('cognito_access_token')
            id_token = get_token_from_cookie('cognito_id_token')
            
            # Using the rust_get_chunks API endpoint
            API_END_POINT_SERVICE = 'rust_get_chunks'
            
            # Construct API endpoint - check if API_STAGE is empty
            if application.config['API_STAGE']:
                api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
            else:
                api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
            
            print(f"Calling API endpoint: {api_endpoint}")
            
            # Prepare headers for the request
            headers = {
                "Content-Type": "application/json",
                "Authorization": f"Bearer {id_token}",
                "x-api-key": application.config.get('API_KEY', '')
            }
            
            # Prepare document filters
            document_filters = []
            
            # Add metadata filters to document_filters
            for mf in metadata_filters:
                # Create a document filter for each metadata filter
                filter_type = "metadata"
                filter_value = json.dumps({
                    "metadata_uuid": mf.get("metadata_uuid"),
                    "operator": mf.get("operator"),
                    "value": mf.get("value")
                })
                document_filters.append({
                    "filter_type": filter_type,
                    "filter_value": filter_value
                })
            
            # Prepare the request payload
            payload = {
                "document_filters": document_filters,
                "question": search_query,
                "num_results": limit,
                "tags": tags if tags else None
            }
            
            print(f"Search payload: {json.dumps(payload)}")
            
            try:
                # Make the request to the API
                response = requests.post(
                    api_endpoint,
                    headers=headers,
                    json=payload
                )
                
                # Print the results to the console for debugging
                print(f'Search API response STATUS: {response.status_code}')
                
                if response.status_code == 200:
                    api_response = response.json()
                    
                    # Here you would normally call an LLM to process the chunks
                    # For now, we'll just return the chunks directly
                    
                    # Return the search results as JSON
                    return jsonify({
                        "llm_response": f"Here are the results for: {search_query}",
                        "chunks": api_response.get("chunks", [])
                    })
                else:
                    # Try to get a structured error message from the response
                    error_message = "Unknown error occurred"
                    try:
                        error_data = response.json()
                        if isinstance(error_data, dict):
                            error_message = error_data.get("error", "Unknown error")
                            details = error_data.get("message", "")
                            if details:
                                error_message += f": {details}"
                    except:
                        error_message = response.text or f"Error {response.status_code}"
                    
                    print(f'Error response: {error_message}')
                    return jsonify({
                        "error": "Search failed",
                        "message": error_message
                    }), response.status_code
                    
            except Exception as e:
                print(f"Error calling API: {str(e)}")
                return jsonify({
                    "error": "Error calling search API",
                    "message": str(e)
                }), 500
        else:
            # Handle traditional form submission for non-AJAX requests
            search_query = request.form.get("query", "")
            tags = request.form.get("tags", "")
            limit = request.form.get("limit", "5")
    
    # For GET requests or after processing POST, render the search template
    return render_template("search.html")


@application.route("/admin-documents")
@auth_required(groups=["admin"])
def admin_documents():
    return render_template("admin_documents.html", superset_url=application.config["SUPERSET_URL"])


@application.route("/settings")
@auth_required(groups=["admin"])
def settings():
    return render_template("settings.html")


@application.route("/recurrent-query")
@auth_required()
def recurrent_query():
    """Page to manage recurrent queries"""
    return render_template("recurrent_query.html")


@application.route("/api/recurrent-query", methods=["GET"])
@auth_required()
def get_recurrent_queries():
    """API endpoint to get all recurrent queries"""
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Using the rust_get_recurrent_query API endpoint
    API_END_POINT_SERVICE = 'rust_get_recurrent_query'
    
    # Construct API endpoint - check if API_STAGE is empty
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    
    # Prepare the headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.post(
            api_endpoint,
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'GET STATUS: {response.status_code} REASON: {response.reason}')
        
        # Return the API response directly to the frontend
        return response.json()
        
    except Exception as e:
        print(f"Error calling API: {str(e)}")
        return jsonify({
            "message": "Error retrieving recurrent queries",
            "error": str(e)
        }), 500


@application.route("/api/recurrent-query/<query_uuid>", methods=["GET"])
@auth_required()
def get_recurrent_query(query_uuid):
    """API endpoint to get a specific recurrent query"""
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Using the rust_get_recurrent_query API endpoint
    API_END_POINT_SERVICE = 'rust_get_recurrent_query'
    
    # Construct API endpoint - check if API_STAGE is empty
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}/{query_uuid}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}/{query_uuid}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    
    # Prepare the headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.post(
            api_endpoint,
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'GET STATUS: {response.status_code} REASON: {response.reason}')
        
        if response.status_code != 200:
            return jsonify({
                "message": f"Failed to retrieve recurrent query: {response.reason}",
                "error": response.text
            }), response.status_code
        
        # Return the API response directly to the frontend
        return response.json()
        
    except Exception as e:
        print(f"Error calling API: {str(e)}")
        return jsonify({
            "message": "Error retrieving recurrent query",
            "error": str(e)
        }), 500


@application.route("/api/recurrent-query", methods=["POST"])
@auth_required()
def add_recurrent_query():
    """API endpoint to add a new recurrent query"""
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Get the request data
    request_data = request.get_json()
    if not request_data or 'recurrent_query_name' not in request_data or 'query_type' not in request_data or 'query_content' not in request_data:
        return jsonify({
            "success": False,
            "message": "Missing required fields: recurrent_query_name, query_type, and query_content"
        }), 400
    
    # Add user information from the session
    request_data['user_uuid'] = session.get('claims', {}).get('sub')
    
    # Using the rust_add_recurrent_query API endpoint
    API_END_POINT_SERVICE = 'rust_add_recurrent_query'
    
    # Construct API endpoint - check if API_STAGE is empty
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    
    # Prepare the headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.post(
            api_endpoint,
            data=json.dumps(request_data),
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
        
        if response.status_code != 200:
            return jsonify({
                "success": False,
                "message": f"Failed to add recurrent query: {response.reason}",
                "details": response.text
            }), response.status_code
        
        # Return the API response to the frontend
        return response.json()
        
    except Exception as e:
        print(f"Error calling API: {str(e)}")
        return jsonify({
            "success": False,
            "message": "Error adding recurrent query",
            "error": str(e)
        }), 500


@application.route("/api/recurrent-query/<query_uuid>", methods=["PUT"])
@auth_required()
def update_recurrent_query(query_uuid):
    """API endpoint to update a specific recurrent query"""
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Get the request data
    request_data = request.get_json()
    if not request_data or 'recurrent_query_name' not in request_data or 'query_type' not in request_data or 'query_content' not in request_data:
        return jsonify({
            "success": False,
            "message": "Missing required fields: recurrent_query_name, query_type, and query_content"
        }), 400
    
    # Add the query UUID to the request data
    request_data['recurrent_query_uuid'] = query_uuid
    
    # Add user information for updated_by field
    request_data['updated_by'] = session.get('claims', {}).get('sub')
    
    # Using the rust_update_recurrent_query API endpoint
    API_END_POINT_SERVICE = 'rust_update_recurrent_query'
    
    # Construct API endpoint - check if API_STAGE is empty
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    print(f"Updating recurrent query: {query_uuid}")
    print(f"Request data: {request_data}")
    
    # Prepare the headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.put(
            api_endpoint,
            data=json.dumps(request_data),
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'PUT STATUS: {response.status_code} REASON: {response.reason}')
        print(f'PUT RESPONSE: {response.text}')
        
        if response.status_code != 200:
            return jsonify({
                "success": False,
                "message": f"Failed to update recurrent query: {response.reason}",
                "details": response.text
            }), response.status_code
        
        # Return success response
        return jsonify({
            "success": True,
            "message": "Recurrent query updated successfully",
            "data": response.json() if response.content else {}
        })
        
    except Exception as e:
        print(f"Error updating recurrent query: {str(e)}")
        return jsonify({
            "success": False,
            "message": "Error updating recurrent query",
            "error": str(e)
        }), 500


@application.route("/api/recurrent-query/<query_uuid>", methods=["DELETE"])
@auth_required()
def delete_recurrent_query(query_uuid):
    """API endpoint to delete a specific recurrent query"""
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Using the rust_delete_recurrent_query API endpoint
    API_END_POINT_SERVICE = 'rust_delete_recurrent_query'
    
    # Construct API endpoint - check if API_STAGE is empty
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    print(f"Deleting recurrent query: {query_uuid}")
    
    # Prepare data and headers for the request
    request_data = {
        'recurrent_query_uuid': query_uuid
    }
    
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.delete(
            api_endpoint,
            data=json.dumps(request_data),
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'DELETE STATUS: {response.status_code} REASON: {response.reason}')
        print(f'DELETE RESPONSE: {response.text}')
        
        if response.status_code != 200:
            return jsonify({
                "success": False,
                "message": f"Failed to delete recurrent query: {response.reason}",
                "details": response.text
            }), response.status_code
        
        # Return success response
        return jsonify({
            "success": True,
            "message": "Recurrent query deleted successfully"
        })
        
    except Exception as e:
        print(f"Error deleting recurrent query: {str(e)}")
        return jsonify({
            "success": False,
            "message": "Error deleting recurrent query",
            "error": str(e)
        }), 500


@application.route("/hello-world-debug")
@auth_required(groups=["admin"])
def hello_world_debug():
    """Debug endpoint to display basic environment and configuration info"""
    # Collect basic environment info
    env_info = {
        "python_version": platform.python_version(),
        "platform": platform.platform(),
        "flask_version": flask.__version__,
        "time": datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
        "user_agent": request.user_agent.string
    }
    
    # Collect request headers (excluding sensitive ones)
    headers = {k: v for k, v in request.headers.items() 
              if k.lower() not in ('authorization', 'cookie')}
    
    # Collect non-sensitive configuration info
    config_info = {
        "debug": application.debug,
        "testing": application.testing,
        "secret_key_set": bool(application.config.get("SECRET_KEY")),
        "static_folder": application.static_folder,
        "template_folder": application.template_folder
    }
    
    return render_template("hello_world_debug.html", 
                          env_info=env_info,
                          headers=headers,
                          config_info=config_info)

@application.route("/api-gateway-call")
@auth_required(groups=["admin"])
def api_gateway_call():
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Using the rust_hello.py as a model
    API_END_POINT_SERVICE = 'rust_hello'
    
    # Check if API_STAGE is empty and build endpoint accordingly
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    
    # Prepare the data and headers for the request
    post_data = {
        'client_id': 22
    }
    
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.post(
            api_endpoint,
            data=json.dumps(post_data),
            headers=headers
        )
        
        # Print the results to the console
        print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
        print(f'POST ANSWER: {response.text}')
        
        # Return response to the frontend
        return jsonify({
            "message": "API Gateway call completed",
            "status_code": response.status_code,
            "reason": response.reason,
            "response": response.text
        })
        
    except Exception as e:
        print(f"Error calling API: {str(e)}")
        return jsonify({
            "message": "Error calling API",
            "error": str(e)
        }), 500

@application.route("/token-debug")
@auth_required(groups=["admin"])  # Restrict to admin users for security
def token_debug():
    """Debug endpoint to inspect token information"""
    token_info = {
        # Check if tokens exist in cookies
        "access_token_exists": bool(get_token_from_cookie('cognito_access_token')),
        "refresh_token_exists": 'refresh_token_cookie' in request.cookies,
        # Get claims from session (already decoded by flask-cognito-lib)
        "token_claims": session.get("claims", {}),
        # User info from session
        "user_info": session.get("user_info", {}),
        
        # Token expiration info
        "token_exp": session.get("claims", {}).get("exp", None),
        # Only include a few characters of the actual token for verification
        "access_token_preview": f"{get_token_from_cookie('cognito_access_token')[:10]}...{get_token_from_cookie('cognito_access_token')[-10:]}" if get_token_from_cookie('cognito_access_token') else None,
        # Cookie info
        "cookies_present": list(request.cookies.keys())
    }
    
    return jsonify(token_info)

@application.route("/bi-dashboard")
@auth_required(groups=["admin"])
def bi_dashboard():
    return render_template("bi_dashboard.html", message="BI Dashboard - Under Development")

@application.route("/fetch-documents", methods=["GET", "POST"])
@auth_required(groups=["admin"])
def fetch_documents():
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Using the rust_document_list API endpoint
    API_END_POINT_SERVICE = 'rust_document_list'
    
    # Construct API endpoint - check if API_STAGE is empty
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    
    # Get request data if it's a POST with JSON body
    if request.method == 'POST' and request.is_json:
        request_data = request.get_json() or {}
        post_data = {
            'client_id': 22
        }
        
        # Log received data for debugging
        print(f"Received fetch-documents request data: {json.dumps(request_data)}")
        
        # Handle document_filters if provided
        if 'document_filters' in request_data:
            post_data['document_filters'] = request_data['document_filters']
            print(f"Document filters received: {json.dumps(request_data['document_filters'])}")
        
        # For backward compatibility: process metadata_filters into document_filters format
        if 'metadata_filters' in request_data and 'document_filters' not in request_data:
            document_filters = []
            
            for mf in request_data.get('metadata_filters', []):
                filter_type = "metadata"
                filter_value = json.dumps({
                    "metadata_uuid": mf.get("metadata_uuid"),
                    "operator": mf.get("operator"),
                    "value": mf.get("value")
                })
                document_filters.append({
                    "filter_type": filter_type,
                    "filter_value": filter_value
                })
            
            if document_filters:
                post_data['document_filters'] = document_filters
                print(f"Converted metadata_filters to document_filters: {json.dumps(document_filters)}")
    else:
        # Default payload for GET requests
        post_data = {
            'client_id': 22
        }
    
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.post(
            api_endpoint,
            data=json.dumps(post_data),
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
        print(f'POST ANSWER: {response.text}')
        
        # Return the API response directly to the frontend
        return response.text
        
    except Exception as e:
        print(f"Error calling API: {str(e)}")
        return jsonify({
            "message": "Error fetching documents",
            "error": str(e)
        }), 500

@application.route("/api/get-chunks", methods=["POST"])
@auth_required()
def api_get_chunks():
    """API endpoint to call the rust_get_chunks service"""
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Get the request data
    request_data = request.get_json()
    
    # Using the rust_get_chunks API endpoint
    API_END_POINT_SERVICE = 'rust_get_chunks'
    
    # Construct API endpoint - check if API_STAGE is empty
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    
    # Prepare the headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.post(
            api_endpoint,
            data=json.dumps(request_data),
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
        
        # Return the API response directly to the frontend
        return response.json()
        
    except Exception as e:
        print(f"Error calling API: {str(e)}")
        return jsonify({
            "message": "Error retrieving chunks",
            "error": str(e)
        }), 500

@application.route("/update-document-tags", methods=["PUT"])
@auth_required(groups=["admin"])
def update_document_tags():
    """API endpoint to update document tags via the rust_update_tags service"""
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Get the request data
    request_data = request.get_json()
    if not request_data or 'document_uuid' not in request_data or 'tags' not in request_data:
        return jsonify({
            "success": False,
            "message": "Missing required fields: document_uuid and tags"
        }), 400
    
    # Using the rust_update_tags API endpoint
    API_END_POINT_SERVICE = 'rust_update_tags'
    
    # Construct API endpoint
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    print(f"Updating tags for document: {request_data['document_uuid']}")
    print(f"New tags: {request_data['tags']}")
    
    # Prepare headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.put(
            api_endpoint,
            data=json.dumps(request_data),
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'PUT STATUS: {response.status_code} REASON: {response.reason}')
        print(f'PUT RESPONSE: {response.text}')
        
        if response.status_code != 200:
            return jsonify({
                "success": False,
                "message": f"Failed to update tags: {response.reason}",
                "details": response.text
            }), response.status_code
        
        # Return the API response to the frontend
        return response.text
        
    except Exception as e:
        print(f"Error updating tags: {str(e)}")
        return jsonify({
            "success": False,
            "message": "Error updating document tags",
            "error": str(e)
        }), 500

@application.route("/api/get-openai-answer", methods=["POST"])
@auth_required()
def api_get_openai_answer():
    """API endpoint to call the rust_openai_answer service"""
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Get the request data
    request_data = request.get_json()
    
    # Using the rust_openai_answer API endpoint
    API_END_POINT_SERVICE = 'rust_openai_answer'
    
    # Construct API endpoint - check if API_STAGE is empty
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling OpenAI Answer API endpoint: {api_endpoint}")
    
    # Prepare the headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.post(
            api_endpoint,
            data=json.dumps(request_data),
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
        
        # Return the API response directly to the frontend
        return response.json()
        
    except Exception as e:
        print(f"Error calling OpenAI Answer API: {str(e)}")
        return jsonify({
            "message": "Error retrieving OpenAI answer",
            "error": str(e)
        }), 500

@application.route("/metadata")
@auth_required(groups=["admin"])
def metadata():
    return render_template("metadata.html")

@application.route("/api/metadata")
@auth_required()
def get_metadata():
    # Make available to all users (not just admins)
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Using the rust_get_metadata API endpoint
    API_END_POINT_SERVICE = 'rust_get_metadata'
    
    # Construct API endpoint - check if API_STAGE is empty
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    
    # Prepare headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.post(
            api_endpoint,
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'GET STATUS: {response.status_code} REASON: {response.reason}')
        print(f'GET ANSWER: {response.text}')
        
        # Return the API response directly to the frontend
        return response.text
        
    except Exception as e:
        print(f"Error calling API: {str(e)}")
        return jsonify({
            "message": "Error fetching metadata",
            "error": str(e)
        }), 500

@application.route("/api/metadata", methods=["POST"])
@auth_required(groups=["admin"])
def add_metadata():
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Get the request data
    request_data = request.get_json()
    if not request_data or 'metadata_name' not in request_data or 'metadata_description' not in request_data or 'metadata_type' not in request_data:
        return jsonify({
            "success": False,
            "message": "Missing required fields: metadata_name, metadata_description, and metadata_type"
        }), 400
    
    # Using the rust_add_metadata API endpoint
    API_END_POINT_SERVICE = 'rust_add_metadata'
    
    # Construct API endpoint
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    
    # Prepare headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.post(
            api_endpoint,
            data=json.dumps(request_data),
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
        print(f'POST ANSWER: {response.text}')
        
        # Return the API response directly to the frontend
        return response.text
        
    except Exception as e:
        print(f"Error calling API: {str(e)}")
        return jsonify({
            "message": "Error fetching metadata",
            "error": str(e)
        }), 500

@application.route("/api/metadata/<metadata_uuid>", methods=["PUT"])
@auth_required(groups=["admin"])
def update_metadata(metadata_uuid):
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Get the request data
    request_data = request.get_json()
    if not request_data or 'metadata_name' not in request_data or 'metadata_description' not in request_data or 'metadata_type' not in request_data:
        return jsonify({
            "success": False,
            "message": "Missing required fields: metadata_name, metadata_description, and metadata_type"
        }), 400
    
    # Add the metadata_uuid to the request data
    request_data['metadata_uuid'] = metadata_uuid
    
    # Using the rust_update_metadata API endpoint
    API_END_POINT_SERVICE = 'rust_update_metadata'
    
    # Construct API endpoint
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    print(f"Updating metadata: {metadata_uuid}")
    
    # Prepare headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.put(
            api_endpoint,
            data=json.dumps(request_data),
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'PUT STATUS: {response.status_code} REASON: {response.reason}')
        print(f'PUT RESPONSE: {response.text}')
        
        if response.status_code != 200:
            return jsonify({
                "success": False,
                "message": f"Failed to update metadata: {response.reason}",
                "details": response.text
            }), response.status_code
        
        # Return the API response to the frontend
        return response.text
        
    except Exception as e:
        print(f"Error updating metadata: {str(e)}")
        return jsonify({
            "success": False,
            "message": "Error updating metadata",
            "error": str(e)
        }), 500

@application.route("/api/metadata/<metadata_uuid>", methods=["DELETE"])
@auth_required(groups=["admin"])
def delete_metadata(metadata_uuid):
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Using the rust_delete_metadata API endpoint
    API_END_POINT_SERVICE = 'rust_delete_metadata'
    
    # Construct API endpoint
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    print(f"Deleting metadata: {metadata_uuid}")
    
    # Prepare request data and headers
    request_data = {
        'metadata_uuid': metadata_uuid
    }
    
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.delete(
            api_endpoint,
            data=json.dumps(request_data),
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'DELETE STATUS: {response.status_code} REASON: {response.reason}')
        print(f'DELETE RESPONSE: {response.text}')
        
        if response.status_code != 200:
            return jsonify({
                "success": False,
                "message": f"Failed to delete metadata: {response.reason}",
                "details": response.text
            }), response.status_code
        
        # Return the API response to the frontend
        return jsonify({
            "success": True,
            "message": "Metadata deleted successfully"
        })
        
    except Exception as e:
        print(f"Error deleting metadata: {str(e)}")
        return jsonify({
            "success": False,
            "message": "Error deleting metadata",
            "error": str(e)
        }), 500
    
@application.route("/api/document-metadatas", methods=["POST"])
@auth_required(groups=["admin"])
def get_document_metadatas():
    """API endpoint to call the rust_get_document_metadatas service"""
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Get the request data (optional document_uuid filter)
    request_data = request.get_json()
    
    # Using the rust_get_document_metadatas API endpoint
    API_END_POINT_SERVICE = 'rust_get_document_metadatas'
    
    # Construct API endpoint
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    
    # Prepare headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.post(
            api_endpoint,
            data=json.dumps(request_data) if request_data else "{}",
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
        
        # Return the API response directly to the frontend
        return response.text
        
    except Exception as e:
        print(f"Error calling API: {str(e)}")
        return jsonify({
            "message": "Error retrieving document metadatas",
            "error": str(e)
        }), 500

@application.route("/api/compute-document-metadata", methods=["POST"])
@auth_required(groups=["admin"])
def compute_document_metadata():
    """API endpoint to trigger metadata computation for a specific document"""
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Get the request data with document UUID
    request_data = request.get_json()
    if not request_data or 'document_uuid' not in request_data:
        return jsonify({
            "success": False,
            "message": "Missing required field: document_uuid"
        }), 400
    
    # Using the rust_compute_metadata API endpoint
    API_END_POINT_SERVICE = 'rust_compute_metadata'
    
    # Construct API endpoint
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    print(f"Computing metadata for document: {request_data['document_uuid']}")
    
    # Prepare headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.post(
            api_endpoint,
            data=json.dumps(request_data),
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
        print(f'POST RESPONSE: {response.text}')
        
        if response.status_code != 200:
            return jsonify({
                "success": False,
                "message": f"Failed to compute metadata: {response.reason}",
                "details": response.text
            }), response.status_code
        
        try:
            # Try to parse the response as JSON
            result = response.json()
            result["success"] = True
            return jsonify(result)
        except ValueError:
            # If not JSON, return a success response with the text
            return jsonify({
                "success": True,
                "message": "Metadata computation completed",
                "response": response.text
            })
        
    except Exception as e:
        print(f"Error computing metadata: {str(e)}")
        return jsonify({
            "success": False,
            "message": "Error computing document metadata",
            "error": str(e)
        }), 500

@application.route("/document-search")
@auth_required()
def document_search():
    """Page for searching across all documents"""
    return render_template("document_search.html")

@application.route("/api/document-list", methods=["POST"])
@auth_required()  # Allow both regular users and admin users to access this endpoint
def api_document_list():
    """API endpoint to call the rust_document_list service for the document search page and admin documents page"""
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Get the request data
    request_data = request.get_json() or {}
    
    # Log received data for debugging
    print(f"Received document-list request data: {json.dumps(request_data)}")
    
    # For backward compatibility: check if metadata_filters exists and convert to document_filters
    if 'metadata_filters' in request_data and 'document_filters' not in request_data:
        # Create document_filters array if it doesn't exist
        document_filters = []
        
        # Process metadata filters into document_filters format
        for mf in request_data.get('metadata_filters', []):
            filter_type = "metadata"
            filter_value = json.dumps({
                "metadata_uuid": mf.get("metadata_uuid"),
                "operator": mf.get("operator"),
                "value": mf.get("value")
            })
            document_filters.append({
                "filter_type": filter_type,
                "filter_value": filter_value
            })
        
        # Add document_filters to request data
        if document_filters:
            request_data['document_filters'] = document_filters
    
    # Using the rust_document_list API endpoint
    API_END_POINT_SERVICE = 'rust_document_list'
    
    # Construct API endpoint - check if API_STAGE is empty
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    
    # Prepare the headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.post(
            api_endpoint,
            data=json.dumps(request_data),
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
        
        # Parse the response and return it as JSON
        try:
            return response.json()
        except ValueError:
            # If response is not valid JSON, try to parse it
            if response.text:
                try:
                    # Attempt to parse the response as JSON
                    return json.loads(response.text)
                except:
                    # If all parsing fails, return the raw text
                    return jsonify({
                        "documents": [],
                        "error": f"Invalid response format: {response.text[:100]}..."
                    }), 500
            else:
                return jsonify({
                    "documents": [],
                    "message": "No documents found or empty response"
                })
        
    except Exception as e:
        print(f"Error calling document list API: {str(e)}")
        return jsonify({
            "documents": [],
            "message": "Error retrieving document list",
            "error": str(e)
        }), 500

@application.route("/synonym")
@auth_required(groups=["admin"])
def synonym():
    """Page to manage synonyms"""
    return render_template("synonym.html")


@application.route("/api/synonyms", methods=["GET"])
@auth_required(groups=["admin"])
def get_synonyms():
    """API endpoint to get all synonyms"""
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Using the rust_get_synonyms API endpoint
    API_END_POINT_SERVICE = 'rust_get_synonym'
    
    # Construct API endpoint - check if API_STAGE is empty
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    
    # Prepare the headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.post(
            api_endpoint,
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'GET STATUS: {response.status_code} REASON: {response.reason}')
        
        # Parse the response
        try:
            return response.json()
        except ValueError:
            # If response is not JSON, return an empty array
            return jsonify([])
        
    except Exception as e:
        print(f"Error calling API: {str(e)}")
        return jsonify([]), 500


@application.route("/api/synonyms", methods=["POST"])
@auth_required(groups=["admin"])
def add_synonym():
    """API endpoint to add a new synonym"""
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Get the request data
    request_data = request.get_json()
    if not request_data or 'name' not in request_data or 'keyword' not in request_data or 'synonyms' not in request_data:
        return jsonify({
            "success": False,
            "message": "Missing required fields: name, keyword, and synonyms"
        }), 400
    
    # Using the rust_add_synonym API endpoint
    API_END_POINT_SERVICE = 'rust_add_synonym'
    
    # Construct API endpoint
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    
    # Prepare headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.post(
            api_endpoint,
            data=json.dumps(request_data),
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
        print(f'POST RESPONSE: {response.text}')
        
        if response.status_code != 200:
            return jsonify({
                "success": False,
                "message": f"Failed to add synonym: {response.reason}",
                "details": response.text
            }), response.status_code
        
        # Parse the response
        try:
            # Return the original JSON response directly without reformatting
            return response.text, 200, {'Content-Type': 'application/json'}
        except Exception as e:
            print(f"Error parsing response: {str(e)}")
            # If parsing fails, return a formatted response
            return jsonify({
                "statusAPI": "OK",
                "synonym": request_data
            })
        
    except Exception as e:
        print(f"Error adding synonym: {str(e)}")
        return jsonify({
            "success": False,
            "message": "Error adding synonym",
            "error": str(e)
        }), 500


@application.route("/api/synonyms/<synonym_id>", methods=["PUT"])
@auth_required(groups=["admin"])
def update_synonym(synonym_id):
    """API endpoint to update a specific synonym"""
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Get the request data
    request_data = request.get_json()
    if not request_data or 'name' not in request_data or 'keyword' not in request_data or 'synonyms' not in request_data:
        return jsonify({
            "success": False,
            "message": "Missing required fields: name, keyword, and synonyms"
        }), 400
    
    # Add the synonym ID to the request data
    request_data['synonym_id'] = synonym_id
    
    # Using the rust_update_synonym API endpoint
    API_END_POINT_SERVICE = 'rust_update_synonym'
    
    # Construct API endpoint
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    print(f"Updating synonym: {synonym_id}")
    
    # Prepare headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.put(
            api_endpoint,
            data=json.dumps(request_data),
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'PUT STATUS: {response.status_code} REASON: {response.reason}')
        
        if response.status_code != 200:
            return jsonify({
                "success": False,
                "message": f"Failed to update synonym: {response.reason}",
                "details": response.text
            }), response.status_code
        
        # Return success response
        return jsonify({
            "success": True,
            "message": "Synonym updated successfully",
            "data": response.json() if response.content else {}
        })
        
    except Exception as e:
        print(f"Error updating synonym: {str(e)}")
        return jsonify({
            "success": False,
            "message": "Error updating synonym",
            "error": str(e)
        }), 500


@application.route("/api/synonyms/<synonym_id>", methods=["DELETE"])
@auth_required(groups=["admin"])
def delete_synonym(synonym_id):
    """API endpoint to delete a specific synonym"""
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Using the rust_delete_synonym API endpoint
    API_END_POINT_SERVICE = 'rust_delete_synonym'
    
    # Construct API endpoint
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    print(f"Deleting synonym: {synonym_id}")
    
    # Prepare request data and headers
    request_data = {
        'synonym_id': synonym_id
    }
    
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.post(
            api_endpoint,
            data=json.dumps(request_data),
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'DELETE STATUS: {response.status_code} REASON: {response.reason}')
        
        if response.status_code != 200:
            return jsonify({
                "success": False,
                "message": f"Failed to delete synonym: {response.reason}",
                "details": response.text
            }), response.status_code
        
        # Return success response
        return jsonify({
            "success": True,
            "message": "Synonym deleted successfully"
        })
        
    except Exception as e:
        print(f"Error deleting synonym: {str(e)}")
        return jsonify({
            "success": False,
            "message": "Error deleting synonym",
            "error": str(e)
        }), 500

@application.route("/api/rust_update_synonym", methods=["POST"])
@auth_required(groups=["admin"])
def api_update_synonym():
    """API endpoint to update a synonym via the rust_update_synonym service"""
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Get the request data
    request_data = request.get_json()
    if not request_data or 'synonym_uuid' not in request_data or 'synonym_name' not in request_data or 'synonym_value' not in request_data:
        return jsonify({
            "success": False,
            "message": "Missing required fields: synonym_uuid, synonym_name, and synonym_value"
        }), 400
    
    # Using the rust_update_synonym API endpoint
    API_END_POINT_SERVICE = 'rust_update_synonym'
    
    # Construct API endpoint - check if API_STAGE is empty
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    print(f"Updating synonym: {request_data['synonym_uuid']}")
    
    # Prepare the headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API - using PUT method for update
        response = requests.put(
            api_endpoint,
            data=json.dumps(request_data),
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'PUT STATUS: {response.status_code} REASON: {response.reason}')
        print(f'PUT RESPONSE: {response.text}')
        
        if response.status_code != 200:
            return jsonify({
                "success": False,
                "message": f"Failed to update synonym: {response.reason}",
                "details": response.text
            }), response.status_code
        
        # Return the original JSON response directly without reformatting
        return response.text, 200, {'Content-Type': 'application/json'}
        
    except Exception as e:
        print(f"Error updating synonym: {str(e)}")
        return jsonify({
            "success": False,
            "message": "Error updating synonym",
            "error": str(e)
        }), 500

@application.route("/api/rust_add_synonym", methods=["POST"])
@auth_required(groups=["admin"])
def api_add_synonym():
    """API endpoint to add a new synonym via the rust_add_synonym service"""
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Get the request data
    request_data = request.get_json()
    if not request_data or 'synonym_name' not in request_data or 'synonym_value' not in request_data:
        return jsonify({
            "success": False,
            "message": "Missing required fields: synonym_name and synonym_value"
        }), 400
    
    # Using the rust_add_synonym API endpoint
    API_END_POINT_SERVICE = 'rust_add_synonym'
    
    # Construct API endpoint - check if API_STAGE is empty
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    
    # Prepare the headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.post(
            api_endpoint,
            data=json.dumps(request_data),
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
        print(f'POST RESPONSE: {response.text}')
        
        if response.status_code != 200:
            return jsonify({
                "success": False,
                "message": f"Failed to add synonym: {response.reason}",
                "details": response.text
            }), response.status_code
        
        # Parse the response
        try:
            # Return the original JSON response directly without reformatting
            return response.text, 200, {'Content-Type': 'application/json'}
        except Exception as e:
            print(f"Error parsing response: {str(e)}")
            # If parsing fails, return a formatted response
            return jsonify({
                "statusAPI": "OK",
                "synonym": request_data
            })
        
    except Exception as e:
        print(f"Error adding synonym: {str(e)}")
        return jsonify({
            "success": False,
            "message": "Error adding synonym",
            "error": str(e)
        }), 500

@application.route("/api/rust_delete_synonym", methods=["POST"])
@auth_required(groups=["admin"])
def api_delete_synonym():
    """API endpoint to delete a synonym via the rust_delete_synonym service"""
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Get the request data
    request_data = request.get_json()
    if not request_data or 'synonym_uuid' not in request_data:
        return jsonify({
            "success": False,
            "message": "Missing required field: synonym_uuid"
        }), 400
    
    # Using the rust_delete_synonym API endpoint
    API_END_POINT_SERVICE = 'rust_delete_synonym'
    
    # Construct API endpoint - check if API_STAGE is empty
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    print(f"Deleting synonym: {request_data['synonym_uuid']}")
    
    # Prepare the headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API - using DELETE method for deleting
        response = requests.delete(
            api_endpoint,
            data=json.dumps(request_data),
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'DELETE STATUS: {response.status_code} REASON: {response.reason}')
        print(f'DELETE RESPONSE: {response.text}')
        
        if response.status_code != 200:
            return jsonify({
                "success": False,
                "message": f"Failed to delete synonym: {response.reason}",
                "details": response.text
            }), response.status_code
        
        # Return the original JSON response directly without reformatting
        return response.text, 200, {'Content-Type': 'application/json'}
        
    except Exception as e:
        print(f"Error deleting synonym: {str(e)}")
        return jsonify({
            "success": False,
            "message": "Error deleting synonym",
            "error": str(e)
        }), 500

@application.route("/api/apply-synonym", methods=["POST"])
@auth_required()
def api_apply_synonym():
    """API endpoint to compute synonyms for a text and return expanded text"""
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Get the request data
    request_data = request.get_json()
    if not request_data or 'query' not in request_data:
        return jsonify({
            "success": False,
            "message": "Missing required field: query"
        }), 400
    
    # Using the rust_compute_synonym API endpoint
    API_END_POINT_SERVICE = 'rust_compute_synonym'
    
    # Construct API endpoint - check if API_STAGE is empty
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    
    # Prepare the headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API with the correct parameter name 'query'
        response = requests.post(
            api_endpoint,
            data=json.dumps({
                "query": request_data["query"]
            }),
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
        print(f'POST RESPONSE: {response.text}')
        
        if response.status_code != 200:
            return jsonify({
                "success": False,
                "message": f"Failed to compute synonyms: {response.reason}",
                "details": response.text
            }), response.status_code
        
        # Parse the response
        try:
            result = response.json()
            
            # Format the response based on the API format
            # The API returns {"original_query":"...", "processed_query":"...", "statusAPI":"OK"}
            if "processed_query" in result:
                return jsonify({
                    "processed_query": result.get("processed_query", request_data["query"])
                })
            else:
                # Fallback for the old format if it exists
                return jsonify({
                    "synonyms": result.get("expanded_text", request_data["query"])
                })
                
        except Exception as e:
            print(f"Error parsing response: {str(e)}")
            # If parsing fails, return a simplified response
            return jsonify({
                "processed_query": request_data["query"]
            })
        
    except Exception as e:
        print(f"Error computing synonyms: {str(e)}")
        return jsonify({
            "success": False,
            "message": "Error computing synonyms",
            "error": str(e)
        }), 500

@application.route("/api/compute-metadata", methods=["POST"])
@auth_required(groups=["admin"])
def api_compute_metadata():
    """API endpoint to trigger metadata computation for all documents"""
    # Get the access token from auth instance
    access_token = get_token_from_cookie('cognito_access_token')
    id_token = get_token_from_cookie('cognito_id_token')
    
    # Get the request data (may contain optional metadata_uuid)
    request_data = request.get_json() or {}
    
    # Using the rust_compute_metadata API endpoint
    API_END_POINT_SERVICE = 'rust_compute_metadata'
    
    # Construct API endpoint
    if application.config['API_STAGE']:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
    else:
        api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
    
    print(f"Calling API endpoint: {api_endpoint}")
    
    # Prepare headers for the request
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {id_token}",
        "x-api-key": application.config.get('API_KEY', '')
    }
    
    try:
        # Make the request to the API
        response = requests.post(
            api_endpoint,
            data=json.dumps(request_data),
            headers=headers
        )
        
        # Print the results to the console for debugging
        print(f'POST STATUS: {response.status_code} REASON: {response.reason}')
        print(f'POST RESPONSE: {response.text}')
        
        if response.status_code != 200:
            return jsonify({
                "success": False,
                "message": f"Failed to compute metadata: {response.reason}",
                "details": response.text
            }), response.status_code
        
        try:
            # Try to parse the response as JSON
            result = response.json()
            result["success"] = True
            return jsonify(result)
        except ValueError:
            # If not JSON, return a success response with the text
            return jsonify({
                "success": True,
                "message": "Metadata computation initiated",
                "response": response.text
            })
        
    except Exception as e:
        print(f"Error computing metadata: {str(e)}")
        return jsonify({
            "success": False,
            "message": "Error computing metadata",
            "error": str(e)
        }), 500

@application.route("/api/s3-upload-url", methods=["POST"])
@auth_required()
def s3_upload_url():
    """API endpoint to call the rust_s3_upload_url service to generate a presigned URL for file uploading"""
    try:
        # Get the access token from auth instance
        access_token = get_token_from_cookie('cognito_access_token')
        id_token = get_token_from_cookie('cognito_id_token')
        
        request_data = request.get_json()
        
        if not request_data or "file_name" not in request_data or "content_type" not in request_data:
            print("Error: File name and content type are required")
            return jsonify({"status": "error", "message": "File name and content type are required"}), 400
            
        # Get the original file name
        original_file_name = request_data["file_name"]
        
        # Remove blanks/spaces and URL encode the file name
        cleaned_file_name = original_file_name.replace(" ", "")
        url_encoded_file_name = urllib.parse.quote(cleaned_file_name)
        
        print(f"Original file name: {original_file_name}")
        print(f"Cleaned file name (no spaces): {cleaned_file_name}")
        print(f"URL encoded file name: {url_encoded_file_name}")
        
        content_type = request_data["content_type"]
        # Default to 1 hour (3600 seconds) if no expiration provided
        expiration = request_data.get("expiration", 3600)
        
        # Construct the request payload with the URL encoded file name
        payload = {
            "file_name": url_encoded_file_name,
            "content_type": content_type,
            "expiration": expiration
        }
        
        # Using the rust_s3_upload_url API endpoint
        API_END_POINT_SERVICE = 'rust_s3_upload_url'
        
        # Construct API endpoint - check if API_STAGE is empty
        if application.config['API_STAGE']:
            api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
        else:
            api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
        
        # Prepare the headers for the request
        headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {id_token}",
            "x-api-key": application.config.get('API_KEY', '')
        }
        
        print(f"Calling API endpoint: {api_endpoint}")
        print(f"Requesting presigned URL for file upload: {url_encoded_file_name}")
        
        # Make the request
        response = requests.post(api_endpoint, data=json.dumps(payload), headers=headers, timeout=10)
        
        # Log the response status
        print(f"S3 Upload URL API call status: {response.status_code}")
        
        if response.status_code == 200:
            try:
                result = response.json()
                print("S3 UPLOAD URL RESPONSE:")
                print(f"Status: {result.get('status')}")
                print(f"File name: {result.get('file_name')}")
                print(f"Bucket: {result.get('bucket')}")
                print(f"Key: {result.get('key')}")
                print(f"Expiration: {result.get('expiration')} seconds")
                print(f"Presigned URL: {result.get('presigned_url')}")
                
                # Check if we got a valid presigned URL
                if not result.get('presigned_url') or len(result.get('presigned_url', '')) < 10:
                    print("WARNING: Received invalid or empty presigned URL")
                    return jsonify({"status": "error", "message": "Invalid presigned URL received from backend service"}), 500
                
                response_data = {
                    "status": result.get("status", "success"), 
                    "presigned_url": result.get("presigned_url", ""),
                    "original_file_name": original_file_name,
                    "cleaned_file_name": cleaned_file_name,
                    "encoded_file_name": url_encoded_file_name,
                    "file_name": result.get("file_name", ""),
                    "bucket": result.get("bucket", ""),
                    "key": result.get("key", ""),
                    "expiration": result.get("expiration", 3600)
                }
                
                print("Sending response to client:", json.dumps(response_data))
                return jsonify(response_data), 200
                
            except Exception as json_error:
                print(f"Error parsing JSON response: {str(json_error)}")
                return jsonify({"status": "error", "message": "Invalid response format"}), 500
        else:
            print(f"Error response: {response.text}")
            return jsonify({"status": "error", "message": f"Service error: {response.status_code}"}), response.status_code
            
    except Exception as e:
        print(f"Error in S3 upload URL endpoint: {str(e)}")
        return jsonify({"status": "error", "message": "Server error"}), 500

@application.route("/api/document-presigned-url", methods=["POST"])
@auth_required()
def document_presigned_url():
    """API endpoint to call the rust_document_presigned_url service to generate a presigned URL for document viewing"""
    try:
        # Get the access token from auth instance
        access_token = get_token_from_cookie('cognito_access_token')
        id_token = get_token_from_cookie('cognito_id_token')
        
        request_data = request.get_json()
        
        if not request_data or "document_uuid" not in request_data:
            print("Error: Document UUID is required")
            return jsonify({"status": "error", "message": "Document UUID is required"}), 400
            
        document_uuid = request_data["document_uuid"]
        # Default to 1 hour (3600 seconds) if no expiration provided
        expiration = request_data.get("expiration", 3600)
        
        # Construct the request payload
        payload = {
            "document_uuid": document_uuid,
            "expiration": expiration
        }
        
        # Using the rust_document_presigned_url API endpoint
        API_END_POINT_SERVICE = 'rust_document_presigned_url'
        
        # Construct API endpoint - check if API_STAGE is empty
        if application.config['API_STAGE']:
            api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{application.config['API_STAGE']}/{API_END_POINT_SERVICE}"
        else:
            api_endpoint = f"{application.config['API_ENDPOINT_ROOT']}/{API_END_POINT_SERVICE}"
        
        # Prepare the headers for the request
        headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {id_token}",
            "x-api-key": application.config.get('API_KEY', '')
        }
        
        
        print(f"Calling API endpoint: {api_endpoint}")
        print(f"Requesting presigned URL for document: {document_uuid}")
        
        # Make the request
        response = requests.post(api_endpoint, data=json.dumps(payload), headers=headers, timeout=10)
        
        # Log the response status
        print(f"Presigned URL API call status: {response.status_code}")
        
        if response.status_code == 200:
            try:
                result = response.json()
                # Simplified: Just log the presigned URL to console
                print("PRESIGNED URL RESPONSE:")
                print(f"Status: {result.get('status')}")
                print(f"Document name: {result.get('document_name')}")
                print(f"Expiration: {result.get('expiration')} seconds")
                print(f"Presigned URL: {result.get('presigned_url')}")
                
                # Return the full response to the client, ensuring the presigned_url is properly included
                return jsonify({
                    "status": result.get("status", "success"), 
                    "presigned_url": result.get("presigned_url", ""),
                    "document_name": result.get("document_name", ""),
                    "expiration": result.get("expiration", 3600)
                }), 200
                
            except Exception as json_error:
                print(f"Error parsing JSON response: {str(json_error)}")
                return jsonify({"status": "error", "message": "Invalid response format"}), 500
        else:
            print(f"Error response: {response.text}")
            return jsonify({"status": "error", "message": f"Service error: {response.status_code}"}), response.status_code
            
    except Exception as e:
        print(f"Error in document presigned URL endpoint: {str(e)}")
        print("Returning server error response")
        return jsonify({"status": "error", "message": "Server error"}), 500

if __name__ == "__main__":
    if application.config["LOCAL_DEBUG"]:
        application.run(host="0.0.0.0", port=application.config["LOCAL_DEBUG_PORT"], debug=True)
    else:
        application.run()