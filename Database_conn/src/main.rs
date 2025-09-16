use tokio_postgres::Client;
use openssl::ssl::{SslConnector, SslMethod};
use postgres_native_tls::MakeTlsConnector;
use native_tls::TlsConnector;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use clap::Parser;
use anyhow::{Result, Context};
use std::time::Instant;
use chrono::Local;
use dotenv::dotenv;

// Document struct to map query results
#[derive(Debug, Serialize, Deserialize)]
struct DocumentChunkCount {
    document_uuid: String,
    lenght_chunk: i64,
}

// Schema object structures for holding database metadata
#[derive(Debug)]
struct Column {
    name: String,
    data_type: String,
    is_nullable: bool,
    default_value: Option<String>,
}

#[derive(Debug)]
struct Table {
    schema: String,
    name: String,
    columns: Vec<Column>,
    primary_key: Option<Vec<String>>,
}

#[derive(Debug)]
struct Index {
    schema: String,
    table: String,
    name: String,
    definition: String,
}

#[derive(Debug)]
struct Sequence {
    schema: String,
    name: String,
    definition: String,
}

#[derive(Debug)]
struct ForeignKey {
    constraint_name: String,
    schema: String,
    table_name: String,
    column_names: Vec<String>,
    foreign_schema: String,
    foreign_table_name: String,
    foreign_column_names: Vec<String>,
    update_rule: String,
    delete_rule: String,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the .env file containing database configuration
    #[arg(long, default_value = ".env")]
    env_file: PathBuf,

    /// PostgreSQL host
    #[arg(long)]
    db_host: Option<String>,

    /// PostgreSQL database name
    #[arg(long)]
    db_name: Option<String>,

    /// PostgreSQL user
    #[arg(long)]
    db_user: Option<String>,

    /// PostgreSQL password
    #[arg(long)]
    db_password: Option<String>,

    /// PostgreSQL port
    #[arg(long)]
    db_port: Option<String>,

    /// Output SQL file path
    #[arg(long, default_value = "schema_export.sql")]
    output_file: PathBuf,

    /// Schemas to export (comma-separated)
    #[arg(long, default_value = "public")]
    schemas: String,
    
    /// Run the original document chunk query
    #[arg(long)]
    run_chunk_query: bool,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    
    // Load environment variables from the specified .env file
    if let Some(env_path) = args.env_file.to_str() {
        if std::path::Path::new(env_path).exists() {
            println!("Loading environment from file: {}", env_path);
            dotenv::from_path(&args.env_file).context("Failed to load .env file")?;
        } else {
            println!("Warning: .env file not found at {}. Using environment variables instead.", env_path);
        }
    }
    
    // Set up the TLS connector
    SslConnector::builder(SslMethod::tls()).unwrap();
    
    // PostgreSQL connection parameters
    let db_server = args.db_host.or_else(|| env::var("DB_HOST").ok())
        .context("DB_HOST environment variable is not set")?;
    let database = args.db_name.or_else(|| env::var("DB_NAME").ok())
        .context("DB_NAME environment variable is not set")?;
    let db_username = args.db_user.or_else(|| env::var("DB_USER").ok())
        .context("DB_USER environment variable is not set")?;
    let db_password = args.db_password.or_else(|| env::var("DB_PASSWORD").ok())
        .context("DB_PASSWORD environment variable is not set")?;
    let db_port = args.db_port.or_else(|| env::var("DB_PORT").ok())
        .context("DB_PORT environment variable is not set")?;


    let tls_connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true) // Disable certificate validation
        .build()
        .context("Failed to build TLS connector")?;
    let tls = MakeTlsConnector::new(tls_connector);

    println!("Connecting to PostgreSQL server at {}:{} as user {}...", db_server, db_port, db_username);
    
    // Connect to the database
    let (client, connection) = tokio_postgres::connect(
        &format!("host={} port={} user={} password={} dbname={}", 
        db_server, db_port, db_username, db_password, database),
        tls
    ).await.context("Failed to connect to database")?;
    
    // Spawn a new task to manage the connection
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    println!("Successfully connected to database '{}'", database);

    if args.run_chunk_query {
        // Execute the original document chunk query
        let start_time = Instant::now();
        let query = "select dc.document_uuid as document_uuid, sum(dc.chunck_lenght) as lenght_chunk from document_library.document_chunks dc group by dc.document_uuid;";
        let rows = client.query(query, &[]).await?;
        
        // Parse rows into DocumentChunkCount structs
        let mut documents: Vec<DocumentChunkCount> = Vec::new();
        for row in rows {
            let document = DocumentChunkCount {
                document_uuid: row.get("document_uuid"),
                lenght_chunk: row.get("lenght_chunk"),
            };
            println!("Document UUID: {}, Length Chunk: {}", document.document_uuid, document.lenght_chunk);
            documents.push(document);
        }
        
        // You can now work with the strongly-typed documents collection
        println!("Query executed in {:?}", start_time.elapsed());
        println!("Total documents: {}", documents.len());
    } else {
        // Export database schema to SQL file
        let schemas: Vec<&str> = args.schemas.split(',').collect();
        println!("Exporting schemas: {:?} to '{}'", schemas, args.output_file.display());
        
        export_schema_to_sql(&client, &args.output_file, &schemas).await
            .context("Failed to export schema")?;
    }

    Ok(())
}

/// Export database schema to a SQL file
async fn export_schema_to_sql(
    client: &Client,
    output_path: &PathBuf,
    schemas: &[&str],
) -> Result<(), anyhow::Error> {
    let start_time = Instant::now();
    println!("Starting schema export...");
    
    // Create or open the output file
    let mut file = std::fs::File::create(output_path)
        .context("Failed to create output file")?;
    
    // Write SQL file header
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    writeln!(file, "-- PostgreSQL Schema Export")?;
    writeln!(file, "-- Generated on: {}", timestamp)?;
    writeln!(file, "-- Schemas: {:?}\n", schemas)?;
    
    // Start transaction
    writeln!(file, "BEGIN;\n")?;

    // Export schemas
    for schema in schemas {
        writeln!(file, "-- Create schema if it doesn't exist")?;
        // Quote the schema name to handle reserved keywords
        writeln!(file, "CREATE SCHEMA IF NOT EXISTS \"{}\";\n", schema)?;
    }
    println!("avant get tables");
    // Get all tables in the requested schemas
    let tables = get_tables(client, schemas).await?;
    println!("Found {} tables in the specified schemas", tables.len());
    
    // Get all sequences
    let sequences = get_sequences(client, schemas).await?;
    println!("Found {} sequences in the specified schemas", sequences.len());
    
    // Get all custom indexes
    let indexes = get_indexes(client, schemas).await?;
    println!("Found {} indexes in the specified schemas", indexes.len());
    
    // Get all foreign keys
    let foreign_keys = get_foreign_keys(client, schemas).await?;
    println!("Found {} foreign key constraints", foreign_keys.len());
    
    // Export sequences first
    if !sequences.is_empty() {
        writeln!(file, "-- Sequences")?;
        for sequence in &sequences {
            writeln!(file, "{}", sequence.definition)?;
        }
        writeln!(file)?;
    }
    
    // Export table structures without foreign keys
    writeln!(file, "-- Tables")?;
    for table in &tables {
        write_table_definition(&mut file, table)?;
    }
    
    // Export indexes
    if !indexes.is_empty() {
        writeln!(file, "-- Indexes")?;
        for index in &indexes {
            writeln!(file, "{}", index.definition)?;
        }
        writeln!(file)?;
    }
    
    // Export foreign key constraints last
    if !foreign_keys.is_empty() {
        writeln!(file, "-- Foreign Key Constraints")?;
        for fk in &foreign_keys {
            write_foreign_key_definition(&mut file, fk)?;
        }
        writeln!(file)?;
    }
    
    // Commit transaction
    writeln!(file, "COMMIT;")?;
    
    println!("Schema export completed in {:?}", start_time.elapsed());
    println!("Schema exported to '{}'", output_path.display());
    
    Ok(())
}

/// Get all tables and their columns from the specified schemas
async fn get_tables(client: &Client, schemas: &[&str]) -> Result<Vec<Table>, anyhow::Error> {
    let schema_list = schemas
        .iter()
        .map(|s| format!("'{}'", s))
        .collect::<Vec<String>>()
        .join(",");
    println!("in get table");
    // Query to get table information
    let table_query = format!(
        "SELECT 
            n.nspname as schema_name,
            c.relname as table_name
         FROM 
            pg_catalog.pg_class c
            LEFT JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
         WHERE 
            c.relkind = 'r'
            AND n.nspname IN ({})
         ORDER BY 
            schema_name, table_name",
        schema_list
    );
    println!("in get table2");
    let tables_rows = client.query(&table_query, &[]).await?;
    println!("in get table3");
    let mut tables = Vec::new();
    
    // For each table, get its columns and primary key
    for table_row in tables_rows {
        println!("in get table4");
        println!("{:?}", table_row);  // Use debug format specifier for Row
        let schema: String = table_row.get("schema_name");
        println!("in get table4a");
        let name: String = table_row.get("table_name");
        println!("in get table4b");
        // Get columns for this table
        let columns = get_table_columns(client, &schema, &name).await?;
        println!("in get table5");
        // Get primary key for this table
        let primary_key = get_primary_key(client, &schema, &name).await?;
        println!("in get table6");
        tables.push(Table {
            schema,
            name,
            columns,
            primary_key,
        });
    }
    
    Ok(tables)
}

/// Get columns for a specific table
async fn get_table_columns(client: &Client, schema: &str, table: &str) -> Result<Vec<Column>, anyhow::Error> {
    // This improved query gets the column info including domain-based and array types
    println!("schema:{:}",schema);
    println!("schema:{:}",table);
    let column_query = "
        SELECT 
            c.column_name,
            CASE 
                WHEN t.typtype = 'd' THEN 
                    COALESCE(pg_catalog.format_type(t.typbasetype, NULL), 'user_defined_type')
                WHEN t.typelem <> 0 AND t.typlen = -1 THEN 
                    pg_catalog.format_type(t.typelem, NULL) || '[]'
                WHEN c.udt_name = 'USER-DEFINED' THEN
                    c.data_type || '_' || c.udt_schema || '_' || c.udt_name
                ELSE 
                    pg_catalog.format_type(t.oid, c.character_maximum_length)
            END as data_type,
            c.is_nullable,
            c.column_default
        FROM 
            information_schema.columns c
            JOIN pg_catalog.pg_namespace n ON n.nspname = c.table_schema
            JOIN pg_catalog.pg_class cls ON cls.relname = c.table_name AND cls.relnamespace = n.oid
            JOIN pg_catalog.pg_attribute a ON a.attrelid = cls.oid AND a.attname = c.column_name
            JOIN pg_catalog.pg_type t ON t.oid = a.atttypid
        WHERE 
            c.table_schema = $1::text 
            AND c.table_name = $2::text
        ORDER BY 
            c.ordinal_position";
    println!("avant query");
    let rows = client.query(column_query, &[&schema, &table]).await?;
    println!("apres query");
    let mut columns = Vec::new();

    for row in rows {
        columns.push(Column {
            name: row.get("column_name"),
            data_type: row.get("data_type"),
            is_nullable: row.get::<_, String>("is_nullable") == "YES",
            default_value: row.get("column_default"),
        });
    }
    
    Ok(columns)
}

/// Get primary key columns for a table
async fn get_primary_key(client: &Client, schema: &str, table: &str) -> Result<Option<Vec<String>>, anyhow::Error> {
    let pk_query = "
        SELECT 
            a.attname as column_name
        FROM
            pg_index i
            JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
            JOIN pg_class c ON c.oid = i.indrelid
            JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE
            i.indisprimary
            AND n.nspname = $1::text
            AND c.relname = $2::text
        ORDER BY
            a.attnum";
    
    let rows = client.query(pk_query, &[&schema, &table]).await?;
    
    if rows.is_empty() {
        return Ok(None);
    }
    
    let mut pk_columns = Vec::new();
    for row in rows {
        pk_columns.push(row.get("column_name"));
    }
    
    Ok(Some(pk_columns))
}

/// Get all sequences in specified schemas
async fn get_sequences(client: &Client, schemas: &[&str]) -> Result<Vec<Sequence>, anyhow::Error> {
    let schema_list = schemas
        .iter()
        .map(|s| format!("'{}'", s))
        .collect::<Vec<String>>()
        .join(",");
    
    let sequence_query = format!(
        "SELECT 
            sequence_schema,
            sequence_name,
            'CREATE SEQUENCE IF NOT EXISTS ' || quote_ident(sequence_schema) || '.' || 
            quote_ident(sequence_name) || ' INCREMENT BY ' || 
            increment || ' MINVALUE ' || minimum_value || 
            ' MAXVALUE ' || maximum_value || 
            ' START WITH ' || start_value || ';' as definition
         FROM 
            information_schema.sequences
         WHERE 
            sequence_schema IN ({})
         ORDER BY 
            sequence_schema, sequence_name",
        schema_list
    );
    
    let rows = client.query(&sequence_query, &[]).await?;
    
    let mut sequences = Vec::new();
    for row in rows {
        sequences.push(Sequence {
            schema: row.get("sequence_schema"),
            name: row.get("sequence_name"),
            definition: row.get("definition"),
        });
    }
    
    Ok(sequences)
}

/// Get all custom indexes in specified schemas
async fn get_indexes(client: &Client, schemas: &[&str]) -> Result<Vec<Index>, anyhow::Error> {
    let schema_list = schemas
        .iter()
        .map(|s| format!("'{}'", s))
        .collect::<Vec<String>>()
        .join(",");
    
    let index_query = format!(
        "SELECT 
            n.nspname as schema_name,
            t.relname as table_name,
            i.relname as index_name,
            pg_get_indexdef(i.oid) as definition
         FROM 
            pg_index x
            JOIN pg_class i ON i.oid = x.indexrelid
            JOIN pg_class t ON t.oid = x.indrelid
            JOIN pg_namespace n ON n.oid = t.relnamespace
         WHERE 
            n.nspname IN ({})
            AND NOT x.indisprimary  -- Skip primary keys as we handle them separately
         ORDER BY 
            schema_name, table_name, index_name",
        schema_list
    );
    
    let rows = client.query(&index_query, &[]).await?;
    
    let mut indexes = Vec::new();
    for row in rows {
        indexes.push(Index {
            schema: row.get("schema_name"),
            table: row.get("table_name"),
            name: row.get("index_name"),
            definition: row.get("definition"),
        });
    }
    
    Ok(indexes)
}

/// Get all foreign key constraints in specified schemas
async fn get_foreign_keys(client: &Client, schemas: &[&str]) -> Result<Vec<ForeignKey>, anyhow::Error> {
    let schema_list = schemas
        .iter()
        .map(|s| format!("'{}'", s))
        .collect::<Vec<String>>()
        .join(",");
    
    let fk_query = format!(
        "SELECT
            con.conname as constraint_name,
            ns.nspname as schema_name,
            cl.relname as table_name,
            ARRAY(
                SELECT attname FROM pg_attribute 
                WHERE attrelid = con.conrelid AND ARRAY[attnum] <@ con.conkey
            ) as column_names,
            nf.nspname as foreign_schema,
            clf.relname as foreign_table_name,
            ARRAY(
                SELECT attname FROM pg_attribute 
                WHERE attrelid = con.confrelid AND ARRAY[attnum] <@ con.confkey
            ) as foreign_column_names,
            CASE con.confupdtype
                WHEN 'a' THEN 'NO ACTION'
                WHEN 'r' THEN 'RESTRICT'
                WHEN 'c' THEN 'CASCADE'
                WHEN 'n' THEN 'SET NULL'
                WHEN 'd' THEN 'SET DEFAULT'
                ELSE NULL
            END as update_rule,
            CASE con.confdeltype
                WHEN 'a' THEN 'NO ACTION'
                WHEN 'r' THEN 'RESTRICT'
                WHEN 'c' THEN 'CASCADE'
                WHEN 'n' THEN 'SET NULL'
                WHEN 'd' THEN 'SET DEFAULT'
                ELSE NULL
            END as delete_rule
        FROM
            pg_constraint con
            JOIN pg_class cl ON con.conrelid = cl.oid
            JOIN pg_namespace ns ON cl.relnamespace = ns.oid
            JOIN pg_class clf ON con.confrelid = clf.oid
            JOIN pg_namespace nf ON clf.relnamespace = nf.oid
        WHERE
            con.contype = 'f'
            AND ns.nspname IN ({})
        ORDER BY
            schema_name, table_name, constraint_name",
        schema_list
    );
    
    let rows = client.query(&fk_query, &[]).await?;
    
    let mut foreign_keys = Vec::new();
    for row in rows {
        foreign_keys.push(ForeignKey {
            constraint_name: row.get("constraint_name"),
            schema: row.get("schema_name"),
            table_name: row.get("table_name"),
            column_names: row.get("column_names"),
            foreign_schema: row.get("foreign_schema"),
            foreign_table_name: row.get("foreign_table_name"),
            foreign_column_names: row.get("foreign_column_names"),
            update_rule: row.get("update_rule"),
            delete_rule: row.get("delete_rule"),
        });
    }
    
    Ok(foreign_keys)
}

/// Write table definition to the output SQL file
fn write_table_definition(file: &mut File, table: &Table) -> Result<(), anyhow::Error> {
    writeln!(file, "-- Table: {}.{}", table.schema, table.name)?;
    writeln!(file, "CREATE TABLE IF NOT EXISTS {}.{} (", table.schema, table.name)?;
    
    // Write column definitions
    for (i, column) in table.columns.iter().enumerate() {
        let nullable = if column.is_nullable { "NULL" } else { "NOT NULL" };
        let default = match &column.default_value {
            Some(default_val) => format!(" DEFAULT {}", default_val),
            None => String::new(),
        };
        
        let comma = if i < table.columns.len() - 1 || table.primary_key.is_some() { "," } else { "" };
        writeln!(file, "    {} {} {}{}{}", column.name, column.data_type, nullable, default, comma)?;
    }
    
    // Write primary key constraint if it exists
    if let Some(pk_columns) = &table.primary_key {
        let pk_columns_str = pk_columns.join(", ");
        writeln!(file, "    PRIMARY KEY ({})", pk_columns_str)?;
    }
    
    writeln!(file, ");")?;
    writeln!(file)?;
    
    Ok(())
}

/// Write foreign key constraint definition to the output SQL file
fn write_foreign_key_definition(file: &mut File, fk: &ForeignKey) -> Result<(), anyhow::Error> {
    writeln!(
        file,
        "ALTER TABLE {}.{} ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {}.{} ({}) ON UPDATE {} ON DELETE {};",
        fk.schema,
        fk.table_name,
        fk.constraint_name,
        fk.column_names.join(", "),
        fk.foreign_schema,
        fk.foreign_table_name,
        fk.foreign_column_names.join(", "),
        fk.update_rule,
        fk.delete_rule
    )?;
    
    Ok(())
}