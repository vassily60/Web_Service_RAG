# PostgreSQL Schema Export Tool

This tool connects to a PostgreSQL database and exports the database schema (tables, constraints, indexes, and relationships) to a SQL file.

## Features

- **Database Connection**: Connects to PostgreSQL using connection parameters from command-line arguments or environment variables
- **Schema Export**: Exports schema structure from specified schemas (default: `public`)
- **Comprehensive Export**: The tool exports:
  - Table definitions with columns, data types, and constraints
  - Primary key definitions
  - Foreign key relationships
  - Custom indexes
  - Sequences

## Usage

```bash
# Using environment variables (from .env file)
# First, create a .env file with your database connection parameters:
# DB_HOST=localhost
# DB_NAME=mydatabase
# DB_USER=postgres
# DB_PASSWORD=password
# DB_PORT=5432

# Export the public schema
cargo run

# Use a specific .env file
cargo run -- --env-file path/to/.env

# Export specific schemas
cargo run -- --schemas "document_library,public"

# Override environment variables with command line arguments
cargo run -- --db-host "localhost" --db-port "5432" --db-name "mydatabase" --db-user "postgres" --db-password "password"

# Specify output file
cargo run -- --output-file "my_schema.sql"

# Run the document chunk query instead of exporting schema
cargo run -- --run-chunk-query
```

## Output

The tool generates a SQL file at the location specified by `--output-file` (default: `schema_export.sql`). This SQL file contains:

1. A transaction block (`BEGIN;` and `COMMIT;`)
2. `CREATE SCHEMA` statements for each schema
3. `CREATE TABLE` statements for all tables with their columns and primary keys
4. Custom indexes
5. `ALTER TABLE` statements to add foreign key constraints after all tables are created

## Command Line Arguments

- `--env-file`: Path to .env file with database connection parameters (default: ".env")
- `--db-host`: PostgreSQL host (overrides environment variable)
- `--db-name`: PostgreSQL database name (overrides environment variable)
- `--db-user`: PostgreSQL user (overrides environment variable)
- `--db-password`: PostgreSQL password (overrides environment variable)
- `--db-port`: PostgreSQL port (overrides environment variable)
- `--output-file`: Output SQL file path (default: "schema_export.sql")
- `--schemas`: Schemas to export, comma-separated (default: "public")
- `--run-chunk-query`: Run the document chunk query instead of exporting schema

## Environment Variables

The following environment variables must be set in your .env file or system environment:

- `DB_HOST`: PostgreSQL host
- `DB_NAME`: PostgreSQL database name
- `DB_USER`: PostgreSQL user
- `DB_PASSWORD`: PostgreSQL password
- `DB_PORT`: PostgreSQL port

## Special Data Type Handling

The tool correctly handles PostgreSQL-specific data types:

- Array types (like `text[]`)
- Vector types for AI embeddings
- Time zone aware timestamps
- Double precision numbers

## Troubleshooting

- **Connection errors**: Check your PostgreSQL connection parameters
- **Permission issues**: Ensure the user has permission to access the schemas and tables
- **Output file issues**: Verify that the output directory is writable
- **SSL/TLS errors**: The tool accepts invalid certificates by default for ease of use, but this can be modified for production use
