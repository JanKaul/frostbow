# Frostbow

A small wrapper around the Datafusion query engine to use Datafusion with Apache Iceberg

## Usage

Start the Frostbow cli with a sql catalog:

```bash
frostbow-sql -u postgres://user:password@localhost:5432 -b s3://my-bucket
```

Pass object-store credentials:
```bash
AWS_ACCESS_KEY_ID=... AWS_SECRET_ACCESS_KEY=... frostbow-sql -u postgres://user:password@localhost:5432 -b s3://my-bucket
```

## Commands

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
LOCATION 's3://iceberg/orders';
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

### Create Materialized View

Create an Iceberg Materialized View

```sql

CREATE MATERIALIZED VIEW iceberg.public.monthly_sales_by_segment AS 
    select 
        sum(o.total_price), 
        date_trunc('month', o.order_date::timestamp)::date as month,
        c.segment
    from 
        iceberg.public.orders as o
    join
        iceberg.public.customers as c
    on
        o.customer_id = c.id
    group by 
        month,
        c.segment;

```

### Refresh Materialzied View

Refresh the Materialized View given a specific identifier

```sql
select refresh_materialized_view('iceberg.public.monthly_sales_by_segment');
```
