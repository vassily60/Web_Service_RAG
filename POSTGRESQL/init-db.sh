#!/bin/bash
set -e

# Create a new database named harvestdb3 and enable the vector extension
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    CREATE DATABASE harvestdb3;
    \c harvestdb3;
    CREATE EXTENSION IF NOT EXISTS vector;
EOSQL
