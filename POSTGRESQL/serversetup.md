# Install Psql on Ubuntu

More information: https://www.digitalocean.com/community/tutorials/how-to-install-and-use-postgresql-on-ubuntu-20-04-fr


## INSTALLATION OF POSTGRES

```bash
sudo apt update
sudo apt dist-upgrade
sudo apt install postgresql postgresql-contrib
```

## TEST CONNECTION FOR POSTGRES (LOCAL SERVER)

```bash
sudo -i -u postgres
psql
```

## CREATIOND OF A NEW ROLE

```bash
sudo -i -u postgres
#create the role (inside postgres)
createuser --interactive
# for exemple vassilyrole
```

#### CREATE DATABASE

```bash
sudo -i -u postgres
#create the DB
createdb testdb
```

### CREATE A USER POSTGRES
```bash
sudo -i -u postgres
psql
```

```sql
CREATE ROLE vassilyrole2 WITH LOGIN PASSWORD 'lapin21';
ALTER ROLE vassilyrole2 SUPERUSER
```


### GRANT THE NEW USER WITH THE ROLE
```sql
GRANT **role_name** TO **user_name**;
```


## MODIF the authorization file
```bash
sudo apt-get install
sudo apt-get update
sudo apt-get dist-upgrade
sudo apt-get install postgresql
cd /etc/postgresql/16/main/
sudo nano pg_hba.conf 
```

# ADD this line at the end of the file:
```bash
hostssl    all             all             0.0.0.0/0            scram-sha-256
```
# RESTART POSTGRESQL
```bash
sudo systemctl restart postgresql
```

# add all addresses
```bash
listen_addresses = '*'          # what IP address(es) to listen on;
```
# RELOAD POSTGRESQL
```bash
sudo pg_ctlcluster 16 main reload
```


## Install pgvector
```bash
sudo apt install postgresql-server-dev-16
```

## INSTALLATION DE L"EXTENION
```bash
sudo su -c psql - postgres
```

```sql
create user bfoucque login;
CREATE DATABASE harvestdb3 owner bfoucque;
COMMENT ON DATABASE harvestdb3 is 'harvest POC';
\l
\password bfoucque
CREATE EXTENSION IF NOT EXISTS vector;
\c harvestdb3
CREATE EXTENSION IF NOT EXISTS vector;
\c - bfoucque
```


Let’s go through each step:

### 1. `create user bfoucque login;`
- **Explanation:**  
  Creates a new PostgreSQL user (role) named `bfoucque` that is allowed to log in to the database.

---

### 2. `CREATE DATABASE harvestdb3 owner bfoucque;`
- **Explanation:**  
  Creates a new database called `harvestdb3` with the user `bfoucque` as its owner.

---

### 3. `COMMENT ON DATABASE harvestdb3 is 'harvest POC';`
- **Explanation:**  
  Adds a textual comment/description to the database `harvestdb3`.  
  Here, `'harvest POC'` means this is a "proof-of-concept" database for the Harvest project.

---

### 4. `\l`
- **Explanation:**  
  A `psql` meta-command that lists all databases in the PostgreSQL instance.

---

### 5. `\password bfoucque`
- **Explanation:**  
  Prompts you to enter (and set) a password for the user `bfoucque`.  
  This step is used to secure the new user's account.

---

### 6. `CREATE EXTENSION IF NOT EXISTS vector;`
- **Explanation:**  
  Installs the `vector` extension in the current database (by default, `postgres` if you haven't changed it yet).  
  The [vector extension](https://github.com/pgvector/pgvector) is often used for similarity search (for AI, embeddings, etc.).

---

### 7. `\c harvestdb3`
- **Explanation:**  
  Another `psql` meta-command. Changes the current connection to use the `harvestdb3` database.

---

### 8. `CREATE EXTENSION IF NOT EXISTS vector;`
- **Explanation:**  
  Installs the `vector` extension in the `harvestdb3` database.  
  (Extensions must be installed separately in each database where you want to use them.)

---

### 9. `\c - bfoucque`
- **Explanation:**  
  Connects to the current database as user `bfoucque`.  
  The hyphen `-` maintains the current database (here, `harvestdb3`), and switches to the specified user (`bfoucque`).

---

## **Summary**

This script:
1. Creates a user and a database owned by that user.
2. Sets a comment on the database.
3. Sets a password for the user.
4. Installs the `vector` extension in both the default and target database.
5. Switches connections appropriately (including to the new user).

This is typical for setting up a new project or proof-of-concept PostgreSQL database that needs vector search capabilities.