# Web_Service_RAG

This repository contains a web service that utilizes Retrieval-Augmented Generation (RAG) to provide enhanced responses by integrating external knowledge sources.

The specific goal is to create a web service that leverages RAG techniques to improve the quality and relevance of code documentation and other text-based outputs.

## Features

- **Retrieval-Augmented Generation**: Combines traditional language generation with retrieval of external knowledge to improve response accuracy and relevance.
- **Knowledge Integration**: Integrates information from various sources, allowing for more informed and context-aware responses.
- **API Access**: Provides a API for easy integration with other applications and services.

- **Web Service**: Used AWS API Gateway, RDS and Lambda for easy access and scalability. Deployed with Elastic Beanstalk.

## Getting Started

To get started with the Web_Service_RAG, follow these steps:

1. Clone the repository:

   ```bash
   git clone https://github.com/yourusername/Web_Service_RAG.git
   ```

2. Install the required dependencies:

   ```bash
   cd Web_Service_RAG
   pip install -r requirements.txt
   ```

3. Start the web service:

   ```bash
   uvicorn main:app --reload
   ```

4. Access the API documentation at `http://localhost:8000/docs`.
