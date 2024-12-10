# Frostbow

Frostbow is an [Apache Datafusion](https://github.com/apache/datafusion) distribution with support for Apache Iceberg. It is a fast, in-process, analytical query engine tailored to work with Iceberg. It supports reading and writing Iceberg tables and comes with all the capabilities of Datafusion.

## Usage

Start the Frostbow cli with a S3Tables catalog:

```bash
frostbow -u arn:aws:s3tables:us-east-1:123456789:bucket/my-bucket-prefix-
```

## Parameters

| Parameter | Description |
|-----------|-------------|
| `-u`  | URL of the catalog. If it starts with "arn:.." the S3Tables catalog is used, if it starts with "s3://..." the filesystem catalog is used. Please refer to the catalog documentation for more details. |
| `-s`  | Storage backend. Can be either `s3` or `gcs`. Defaults to 's3'. |

## Catalogs

Frostbow comes bundled with support for the S3Tables and Filesystem catalogs. Please read the Documentation for further information.

- [S3Tables](README-s3tables.md)
- [Filesystem](README-file.md)

If you want to use Frostbow with the SQL or Glue catalog, please visit the corresponding crates:

- [SQL](frostbow-sql/README.md)
- [Glue](frostbow-glue/README.md)

## Installation

Please refer to the [Installation guide](Installation.md).

## Commands

The following are SQL commands that are specific to Frostbow, for general Datafusion usage please refer to the [Datafusion SQL Reference](https://datafusion.apache.org/user-guide/sql/index.html).

### Create table

Create an Iceberg Table in object storage:

```sql
CREATE TABLE iceberg.public.orders (
      id BIGINT NOT NULL,
      order_date DATE NOT NULL,
      customer_id INTEGER NOT NULL,
      total_price DOUBLE NOT NULL
)
STORED AS ICEBERG
LOCATION ''
PARTITIONED BY ( "month(order_date)" );
```

For the S3Tables and File catalog, the location is set by the catalog. However, you still need to pass an empty string as an argument. 
For the other catalogs, the location has to be set correctly.

### Insert

Insert data into an iceberg table

```sql

INSERT INTO iceberg.public.orders (id, order_date, customer_id, total_price) VALUES 
        (1, '2022-01-01', 1, 100.00),
        (2, '2022-01-02', 2, 200.00),
        (3, '2022-01-03', 3, 50.00),
        (4, '2022-01-04', 1, 150.00),
        (5, '2022-02-05', 4, 75.00),
        (6, '2022-02-06', 2, 250.00),
        (7, '2022-02-07', 5, 30.00),
        (8, '2022-02-08', 3, 120.00),
        (9, '2022-02-09', 1, 180.00),
        (10, '2022-03-10', 6, 60.00);
```

### Create schema

Create a schema in the iceberg catalog:

```sql
CREATE SCHEMA iceberg.public;
```