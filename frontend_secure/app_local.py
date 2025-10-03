from os import environ, path, urandom
from dotenv import load_dotenv
from flask import Flask, jsonify, redirect, render_template, request, session, url_for
import json
import platform
import datetime

# Load variables from .env.local
basedir = path.abspath(path.dirname(__file__))
load_dotenv(path.join(basedir, ".env.local"))


class Config:
    """Set Flask configuration vars for local development."""
    # General Config
    SECRET_KEY = environ.get("SECRET_KEY", urandom(32))
    FLASK_application = "TEST_application"
    FLASK_ENV = "TESTING"
    
    # Local Debug Configuration
    LOCAL_DEBUG = True
    LOCAL_DEBUG_PORT = 5001

    # Mock data for demonstration
    SUPERSET_URL = "http://superset.palo-it.hk:8080/"


application = Flask(__name__)
application.config.from_object(Config)

# Mock user session data
def mock_user_session():
    session['claims'] = {
        'sub': 'mock-user-id',
        'email': 'mock@example.com',
        'name': 'Mock User'
    }
    session['user_info'] = {
        'username': 'mockuser',
        'groups': ['admin']  # Give admin access to see all features
    }

@application.route("/")
def home():
    return render_template("home.html")

@application.route("/login")
def login():
    # Bypass login and redirect directly to welcome page
    mock_user_session()
    return redirect(url_for("welcomepage"))

@application.route("/welcome")
def welcomepage():
    mock_user_session()
    return render_template("welcome.html")

@application.route("/search", methods=["GET", "POST"])
def search():
    mock_user_session()
    if request.method == "POST" and request.is_json:
        # Mock search response
        return jsonify({
            "llm_response": "This is a mock search response",
            "chunks": [
                {
                    "content": "This is a sample search result chunk 1",
                    "metadata": {"source": "Sample Document 1", "page": 1}
                },
                {
                    "content": "This is a sample search result chunk 2",
                    "metadata": {"source": "Sample Document 2", "page": 1}
                }
            ]
        })
    return render_template("search.html")

@application.route("/admin-documents")
def admin_documents():
    mock_user_session()
    return render_template("admin_documents.html", superset_url=application.config["SUPERSET_URL"])

@application.route("/settings")
def settings():
    mock_user_session()
    return render_template("settings.html")

@application.route("/metadata")
def metadata():
    mock_user_session()
    return render_template("metadata.html")

@application.route("/recurrent-query")
def recurrent_query():
    mock_user_session()
    return render_template("recurrent_query.html")

@application.route("/api/recurrent-query", methods=["GET"])
def get_recurrent_queries():
    # Mock recurrent queries data
    mock_queries = [
        {
            "recurrent_query_uuid": "123",
            "recurrent_query_name": "Sample Query 1",
            "query_type": "daily",
            "query_content": "What are the latest updates?",
            "created_at": "2025-10-01"
        },
        {
            "recurrent_query_uuid": "456",
            "recurrent_query_name": "Sample Query 2",
            "query_type": "weekly",
            "query_content": "Show me the weekly summary",
            "created_at": "2025-10-02"
        }
    ]
    return jsonify(mock_queries)

@application.route("/hello-world-debug")
def hello_world_debug():
    """Debug endpoint to display basic environment and configuration info"""
    mock_user_session()
    
    # Collect basic environment info
    env_info = {
        "python_version": platform.python_version(),
        "platform": platform.platform(),
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

@application.route("/logout")
def logout():
    return redirect(url_for("home"))

if __name__ == "__main__":
    application.run(
        debug=application.config["LOCAL_DEBUG"],
        port=application.config["LOCAL_DEBUG_PORT"]
    )