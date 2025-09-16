## Copy database structure to SQL file

Using the existing file database_conn/src/main.rs, you can connect to a Postgresql database.

Now from this connection i want you to export the schema of the database to a SQL file. 

The modified file will connects to a PostgreSQL database and exports the schema (tables, constraints, and relationships) to a SQL file.

Features
Connects to PostgreSQL using connection parameters from .env file using clap. 
Dumps table schemas from specified schemas
Generates CREATE TABLE statements with columns, data types, constraints
Preserves primary key definitions
Includes custom indexes
Exports sequences
Handles foreign key relationships correctly by adding them after all tables are created
Properly formats SQL output for readability


Output
The tool will generate a SQL file at the location specified in OUTPUT_SQL_FILE. This SQL file will:

Start a transaction
Create schemas if they don't exist
Create tables with their columns and primary keys
Add foreign key constraints after all tables are created
Commit the transaction
Troubleshooting
If you get connection errors, double-check your PostgreSQL connection parameters in .env
Make sure the specified schemas exist in your database
Ensure the user has permission to access the schemas and tables
Check that the output file path is writable by the current user