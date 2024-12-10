# Filesystem catalog

## Catalog URL

```bash
frostbow -u s3://warehouse
```

For the Filesystem catalog you have to specify a bucket. The File catalog then translates SQL identifiers into file paths (object prefixes). If you use the table identifier `staging.sales.orders` the table location will be translated to `s3://warehouse/staging/sales/orders`.

## Credentials

Frostbow uses the aws-sdk to determine your credentials. Please read the official [AWS documentation](https://docs.aws.amazon.com/cli/latest/userguide/cli-chap-authentication.html) on how to obtain your credentials.

## Location

The Filesystem catalog automatically determines the `LOCATION` parameter for the table creation. However, you still have to add an empty string in the SQL command, like so:

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