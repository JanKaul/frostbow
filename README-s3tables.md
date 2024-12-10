# S3Tables catalog

## Catalog URL

```bash
frostbow -u arn:aws:s3tables:us-east-1:123456789:bucket/my-bucket-prefix-
```

For the S3Tables catalog you have to specify a table bucket prefix. All table buckets with the given prefix can be used as a SQL catalog by using the part after the prefix as the name. If you have the table bucket `s3://my-org-staging` you can use the SQL catalog `staging` by providing the table bucket prefix `s3://my-org-`. You can then reference a table in the `staging` catalog with `staging.sales.orders`.

## Credentials

Frostbow uses the aws-sdk to determine your credentials. Please read the official [AWS documentation](https://docs.aws.amazon.com/cli/latest/userguide/cli-chap-authentication.html) on how to obtain your credentials.

## Location

The S3Tables catalog automatically determines the `LOCATION` parameter for the table creation. However, you still have to add an empty string in the SQL command, like so:

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