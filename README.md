# Frostbow

A small wrapper around the Datafusion query engine to use Datafusion with Apache Iceberg

## Commands

### Create table

Create an Iceberg Table in object storage:

```sql
CREATE EXTERNAL TABLE iceberg.public.customers (
      id BIGINT NOT NULL,
      first_name TEXT,
      last_name TEXT,
      email TEXT,
      segment TEXT,
)
STORED AS ICEBERG
LOCATION 's3://iceberg/customers';
```

### Insert

Insert data into an ieberg table

```sql

INSERT INTO iceberg.public.customers (id, first_name, last_name, email, segment) VALUES 
        (1, 'John', 'Doe', 'john.doe@medicare.com', 'Healthcare'),
        (2, 'Jane', 'Smith', 'jane.smith@bankofamerica.com', 'Finance'),
        (3, 'Michael', 'Johnson', 'michael.johnson@microsoft.com', 'Technology'),
        (4, 'Emily', 'Williams', 'emily.williams@toyota.com', 'Manufacturing'),
        (5, 'David', 'Brown', 'david.brown@hospitals.org', 'Healthcare'),
        (6, 'Sarah', 'Davis', 'sarah.davis@goldmansachs.com', 'Finance'),
        (7, 'William', 'Miller', 'william.miller@amazon.com', 'Technology'),
        (8, 'Olivia', 'Wilson', 'olivia.wilson@ford.com', 'Manufacturing'),
        (9, 'James', 'Anderson', 'james.anderson@pharmaceuticals.com', 'Healthcare'),
        (10, 'Ava', 'Thomas', 'ava.thomas@americanexpress.com', 'Finance'),
        (11, 'Robert', 'Jackson', 'robert.jackson@ibm.com', 'Technology'),
        (12, 'Isabella', 'White', 'isabella.white@siemens.com', 'Technology'),
        (13, 'Richard', 'Harris', 'richard.harris@healthinsurance.com', 'Healthcare'),
        (14, 'Sophia', 'Martin', 'sophia.martin@jpmorgan.com', 'Finance'),
        (15, 'Charles', 'Thompson', 'charles.thompson@oracle.com', 'Technology'),
        (16, 'Mia', 'Garcia', 'mia.garcia@ge.com', 'Manufacturing'),
        (17, 'Thomas', 'Martinez', 'thomas.martinez@hospitals.org', 'Healthcare'),
        (18, 'Charlotte', 'Robinson', 'charlotte.robinson@google.com', 'Technology'),
        (19, 'Ronald', 'Clark', 'ronald.clark@microsoft.com', 'Technology'),
        (20, 'Abigail', 'Rodriguez', 'abigail.rodriguez@johnsonandjohnson.com', 'Healthcare');

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
