# PostgreSQL Docker Setup

This directory contains the necessary files to set up a PostgreSQL database using Docker.

## Getting Started

To get the PostgreSQL database up and running, follow these steps:

1. Build the image:

1.1. DOCKER
```sh
docker build -t harvest-postgres .
```

1.2. PODMAN
```sh
    podman build -t harvest-postgres .
```

2. Run the Docker container:

2.1 DOCKER
```sh
docker run --name harvest-postgres-container -p 5432:5432 -d harvest-postgres
```

2.2 PODMAN
```sh
podman run --name harvest-postgres-container -p 5432:5432 -d harvest-postgres
```



## Credentials

- **Username:** harvestpostgres
- **Password:** Lapin21!!!
- **Database:** harvestdb

The database will be created automatically when the container is started.

## Connecting to the Database

Use DBEAVER or any other database management tool to connect to the database. The connection details are as bellows.