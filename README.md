# Frostbow

Frostbow is a [Apache Datafusion](https://github.com/apache/datafusion) distribution with support for Apache Iceberg.

## Usage

Start the Frostbow cli with a s3tables catalog:

```bash
frostbow -s s3 -u arn:aws:s3tables:us-east-1:123456789:bucket/my-bucket-prefix
```

Pass object-store credentials:
```bash
AWS_ACCESS_KEY_ID=... AWS_SECRET_ACCESS_KEY=... frostbow -s s3 -u arn:aws:s3tables:us-east-1:123456789:bucket/my-bucket-prefix
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

## Commands

For general Datafusion usage please refer to the [Datafusion SQL Reference](https://datafusion.apache.org/user-guide/sql/index.html).

### Create table

Create an Iceberg Table in object storage:

```sql
CREATE EXTERNAL TABLE iceberg.public.orders (
      id BIGINT NOT NULL,
      order_date DATE NOT NULL,
      customer_id INTEGER NOT NULL,
      total_price DOUBLE NOT NULL
)
STORED AS ICEBERG
PARTITIONED BY ( "month(order_date)" )
LOCATION '';
```

### Insert

Insert data into an ieberg table

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