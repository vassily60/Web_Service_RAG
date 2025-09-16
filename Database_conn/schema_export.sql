-- PostgreSQL Schema Export
-- Generated on: 2025-07-25 15:58:04
-- Schemas: ["document_library"]

BEGIN;

-- Create schema if it doesn't exist
CREATE SCHEMA IF NOT EXISTS "document_library";

-- Tables
-- Table: document_library.document_chunks
CREATE TABLE IF NOT EXISTS document_library.document_chunks (
    document_chunk_uuid text NOT NULL,
    document_uuid text NULL,
    chunck_lenght integer NULL,
    chunck_overlap integer NULL,
    chunck_hash text NULL,
    embebed_text text NULL,
    creation_date timestamp with time zone NULL,
    created_by text NULL,
    updated_date timestamp with time zone NULL,
    updated_by text NULL,
    comments text NULL,
    PRIMARY KEY (document_chunk_uuid)
);

-- Table: document_library.document_embeding_mistral
CREATE TABLE IF NOT EXISTS document_library.document_embeding_mistral (
    document_embeding_uuid text NOT NULL,
    document_chunk_uuid text NULL,
    embeder_type text NULL,
    embedding_token integer NULL,
    embedding_time double precision NULL,
    embedding vector NULL,
    creation_date timestamp with time zone NULL,
    created_by text NULL,
    updated_date timestamp with time zone NULL,
    updated_by text NULL,
    comments text NULL,
    PRIMARY KEY (document_embeding_uuid)
);

-- Table: document_library.document_embeding_mistral_generic
CREATE TABLE IF NOT EXISTS document_library.document_embeding_mistral_generic (
    document_embeding_uuid text NOT NULL,
    document_chunk_uuid text NULL,
    embeder_type text NULL,
    embedding_token integer NULL,
    embedding_time double precision NULL,
    embedding vector NULL,
    creation_date timestamp with time zone NULL,
    created_by text NULL,
    updated_date timestamp with time zone NULL,
    updated_by text NULL,
    comments text NULL,
    PRIMARY KEY (document_embeding_uuid)
);

-- Table: document_library.document_embeding_openai
CREATE TABLE IF NOT EXISTS document_library.document_embeding_openai (
    document_embeding_uuid text NOT NULL,
    document_chunk_uuid text NULL,
    embeder_type text NULL,
    embedding_token integer NULL,
    embedding_time double precision NULL,
    embedding vector NULL,
    creation_date timestamp with time zone NULL,
    created_by text NULL,
    updated_date timestamp with time zone NULL,
    updated_by text NULL,
    comments text NULL,
    PRIMARY KEY (document_embeding_uuid)
);

-- Table: document_library.document_metadatas
CREATE TABLE IF NOT EXISTS document_library.document_metadatas (
    document_metadata_uuid text NOT NULL,
    query_ledger_uuid text NULL,
    document_uuid text NULL,
    metadata_uuid text NULL,
    metadata_value_float double precision NULL,
    metadata_value_string text NULL,
    metadata_value_int integer NULL,
    metadata_value_date date NULL,
    metadata_value_boolean boolean NULL,
    creation_date timestamp with time zone NULL,
    created_by text NULL,
    updated_date timestamp with time zone NULL,
    updated_by text NULL,
    comments text NULL,
    PRIMARY KEY (document_metadata_uuid)
);

-- Table: document_library.document_security_groups
CREATE TABLE IF NOT EXISTS document_library.document_security_groups (
    document_security_group_uuid text NOT NULL,
    security_group_uuid text NULL,
    document_uuid text NULL,
    creation_date timestamp with time zone NULL,
    created_by text NULL,
    updated_date timestamp with time zone NULL,
    updated_by text NULL,
    comments text NULL,
    PRIMARY KEY (document_security_group_uuid)
);

-- Table: document_library.documents
CREATE TABLE IF NOT EXISTS document_library.documents (
    document_uuid text NOT NULL,
    document_name text NULL,
    document_location text NULL,
    document_hash text NULL,
    document_type text NULL,
    document_lenght integer NULL,
    document_size double precision NULL,
    document_status text NULL,
    chunk_time double precision NULL,
    creation_date timestamp with time zone NULL,
    created_by text NULL,
    updated_date timestamp with time zone NULL,
    updated_by text NULL,
    comments text NULL,
    tags text[] NULL,
    PRIMARY KEY (document_uuid)
);

-- Table: document_library.metadatas
CREATE TABLE IF NOT EXISTS document_library.metadatas (
    metadata_uuid text NOT NULL,
    metadata_name text NULL,
    metadata_description text NULL,
    metadata_type text NULL,
    creation_date timestamp with time zone NULL,
    created_by text NULL,
    updated_date timestamp with time zone NULL,
    updated_by text NULL,
    comments text NULL,
    PRIMARY KEY (metadata_uuid)
);

-- Table: document_library.query_answer_chunks
CREATE TABLE IF NOT EXISTS document_library.query_answer_chunks (
    query_answer_chunk_uuid text NOT NULL,
    query_answer_uuid text NULL,
    query_ledger_uuid text NULL,
    chunk_uuid text NULL,
    creation_date timestamp with time zone NULL,
    created_by text NULL,
    updated_date timestamp with time zone NULL,
    updated_by text NULL,
    comments text NULL,
    PRIMARY KEY (query_answer_chunk_uuid)
);

-- Table: document_library.query_answer_documents
CREATE TABLE IF NOT EXISTS document_library.query_answer_documents (
    query_answer_document_uuid text NOT NULL,
    query_answer_uuid text NULL,
    document_uuid text NULL,
    creation_date timestamp with time zone NULL,
    created_by text NULL,
    updated_date timestamp with time zone NULL,
    updated_by text NULL,
    comments text NULL,
    PRIMARY KEY (query_answer_document_uuid)
);

-- Table: document_library.query_ledgers
CREATE TABLE IF NOT EXISTS document_library.query_ledgers (
    query_ledger_uuid text NOT NULL,
    query_type text NULL,
    query_content text NULL,
    user_uuid text NULL,
    query_tags text NULL,
    query_start_document_date date NULL,
    query_end_document_date date NULL,
    query_answer text NULL,
    creation_date timestamp with time zone NULL,
    created_by text NULL,
    updated_date timestamp with time zone NULL,
    updated_by text NULL,
    comments text NULL,
    PRIMARY KEY (query_ledger_uuid)
);

-- Table: document_library.query_ledgers_extended
CREATE TABLE IF NOT EXISTS document_library.query_ledgers_extended (
    query_ledger_uuid text NOT NULL,
    query_type text NULL,
    query_content text NULL,
    metadata_uuid text NULL,
    user_uuid text NULL,
    query_tags text NULL,
    query_start_document_date date NULL,
    query_end_document_date date NULL,
    query_answer text NULL,
    creation_date timestamp with time zone NULL,
    created_by text NULL,
    updated_date timestamp with time zone NULL,
    updated_by text NULL,
    comments text NULL,
    PRIMARY KEY (query_ledger_uuid)
);

-- Table: document_library.query_metadatas
CREATE TABLE IF NOT EXISTS document_library.query_metadatas (
    query_metadata_uuid text NOT NULL,
    query_ledger_uuid text NULL,
    metadata_uuid text NULL,
    creation_date timestamp with time zone NULL,
    created_by text NULL,
    updated_date timestamp with time zone NULL,
    updated_by text NULL,
    comments text NULL,
    PRIMARY KEY (query_metadata_uuid)
);

-- Table: document_library.recurrent_queries
CREATE TABLE IF NOT EXISTS document_library.recurrent_queries (
    recurrent_query_uuid text NOT NULL,
    query_type text NULL,
    query_content text NULL,
    user_uuid text NULL,
    query_tags text NULL,
    query_start_document_date date NULL,
    query_end_document_date date NULL,
    creation_date timestamp with time zone NULL,
    created_by text NULL,
    updated_date timestamp with time zone NULL,
    updated_by text NULL,
    comments text NULL,
    PRIMARY KEY (recurrent_query_uuid)
);

-- Table: document_library.recurrent_queries_extended
CREATE TABLE IF NOT EXISTS document_library.recurrent_queries_extended (
    recurrent_query_uuid text NOT NULL,
    recurrent_query_name text NULL,
    query_type text NULL,
    query_content text NULL,
    user_uuid text NULL,
    query_tags text NULL,
    query_start_document_date date NULL,
    query_end_document_date date NULL,
    creation_date timestamp with time zone NULL,
    created_by text NULL,
    updated_date timestamp with time zone NULL,
    updated_by text NULL,
    comments text NULL,
    PRIMARY KEY (recurrent_query_uuid)
);

-- Table: document_library.recurrent_query_metadatas
CREATE TABLE IF NOT EXISTS document_library.recurrent_query_metadatas (
    recurrent_query_metadata_uuid text NOT NULL,
    recurrent_query_uuid text NULL,
    metadata_uuid text NULL,
    creation_date timestamp with time zone NULL,
    created_by text NULL,
    updated_date timestamp with time zone NULL,
    updated_by text NULL,
    comments text NULL,
    PRIMARY KEY (recurrent_query_metadata_uuid)
);

-- Table: document_library.security_groups
CREATE TABLE IF NOT EXISTS document_library.security_groups (
    security_group_uuid text NOT NULL,
    security_group_name text NULL,
    security_group_description text NULL,
    security_group_type text NULL,
    creation_date timestamp with time zone NULL,
    created_by text NULL,
    updated_date timestamp with time zone NULL,
    updated_by text NULL,
    comments text NULL,
    PRIMARY KEY (security_group_uuid)
);

-- Table: document_library.synonyms
CREATE TABLE IF NOT EXISTS document_library.synonyms (
    synonym_uuid text NOT NULL,
    synonym_name text NULL,
    synonym_value text NULL,
    creation_date timestamp with time zone NULL,
    created_by text NULL,
    updated_date timestamp with time zone NULL,
    updated_by text NULL,
    comments text NULL,
    PRIMARY KEY (synonym_uuid)
);

-- Table: document_library.user_security_groups
CREATE TABLE IF NOT EXISTS document_library.user_security_groups (
    user_security_group_uuid text NOT NULL,
    security_group_uuid text NULL,
    user_uuid text NULL,
    creation_date timestamp with time zone NULL,
    created_by text NULL,
    updated_date timestamp with time zone NULL,
    updated_by text NULL,
    comments text NULL,
    PRIMARY KEY (user_security_group_uuid)
);

-- Table: document_library.users
CREATE TABLE IF NOT EXISTS document_library.users (
    user_uuid text NOT NULL,
    user_name text NULL,
    user_email text NULL,
    department text NULL,
    sso_unique_id text NULL,
    creation_date timestamp with time zone NULL,
    created_by text NULL,
    updated_date timestamp with time zone NULL,
    updated_by text NULL,
    comments text NULL,
    PRIMARY KEY (user_uuid)
);

-- Indexes
CREATE INDEX idx_documents_name ON document_library.documents USING btree (document_name)

-- Foreign Key Constraints
ALTER TABLE document_library.document_metadatas ADD CONSTRAINT document_metadatas_document_uuid_fkey FOREIGN KEY (document_uuid) REFERENCES document_library.documents (document_uuid) ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE document_library.document_metadatas ADD CONSTRAINT document_metadatas_metadata_uuid_fkey FOREIGN KEY (metadata_uuid) REFERENCES document_library.metadatas (metadata_uuid) ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE document_library.document_security_groups ADD CONSTRAINT document_security_groups_document_uuid_fkey FOREIGN KEY (document_uuid) REFERENCES document_library.documents (document_uuid) ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE document_library.document_security_groups ADD CONSTRAINT document_security_groups_security_group_uuid_fkey FOREIGN KEY (security_group_uuid) REFERENCES document_library.security_groups (security_group_uuid) ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE document_library.query_answer_documents ADD CONSTRAINT query_answer_documents_document_uuid_fkey FOREIGN KEY (document_uuid) REFERENCES document_library.documents (document_uuid) ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE document_library.query_ledgers ADD CONSTRAINT query_ledgers_user_uuid_fkey FOREIGN KEY (user_uuid) REFERENCES document_library.users (user_uuid) ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE document_library.query_ledgers_extended ADD CONSTRAINT query_ledgers_extended_user_uuid_fkey FOREIGN KEY (user_uuid) REFERENCES document_library.users (user_uuid) ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE document_library.query_metadatas ADD CONSTRAINT query_metadatas_metadata_uuid_fkey FOREIGN KEY (metadata_uuid) REFERENCES document_library.metadatas (metadata_uuid) ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE document_library.query_metadatas ADD CONSTRAINT query_metadatas_query_ledger_uuid_fkey FOREIGN KEY (query_ledger_uuid) REFERENCES document_library.query_ledgers (query_ledger_uuid) ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE document_library.recurrent_queries ADD CONSTRAINT recurrent_queries_user_uuid_fkey FOREIGN KEY (user_uuid) REFERENCES document_library.users (user_uuid) ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE document_library.recurrent_query_metadatas ADD CONSTRAINT recurrent_query_metadatas_metadata_uuid_fkey FOREIGN KEY (metadata_uuid) REFERENCES document_library.metadatas (metadata_uuid) ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE document_library.recurrent_query_metadatas ADD CONSTRAINT recurrent_query_metadatas_recurrent_query_uuid_fkey FOREIGN KEY (recurrent_query_uuid) REFERENCES document_library.recurrent_queries (recurrent_query_uuid) ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE document_library.user_security_groups ADD CONSTRAINT user_security_groups_security_group_uuid_fkey FOREIGN KEY (security_group_uuid) REFERENCES document_library.security_groups (security_group_uuid) ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE document_library.user_security_groups ADD CONSTRAINT user_security_groups_user_uuid_fkey FOREIGN KEY (user_uuid) REFERENCES document_library.users (user_uuid) ON UPDATE NO ACTION ON DELETE NO ACTION;

COMMIT;
