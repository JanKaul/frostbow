# Copy csv data

In this example we will read data from a CSV file and insert it into an Iceberg table using the S3Tables catalog.
Start by setting up Frostbow. Please refer to the [Installation guide](../Installation.md) or have a look at [How to setup Frostbow on EC2](ec2.md).

As a first step create a Table bucket in the AWS console with the name `my-prefix-warehouse`. Make sure you or the EC2 instance you're running on has the required permissions to access the new table bucket. You can find more information [here](https://docs.aws.amazon.com/AmazonS3/latest/userguide/s3-tables-setting-up.html).

You can start the Frostbow CLI with:

```bash
frostbow -u arn:aws:s3tables:us-east-1:123456789:bucket/my-prefix-
```

In the next step we will create an external table for a csv file that is stored in S3. Creating external tables enables to reference the CSV files by SQL identifiers. Additionally we can specify the schema that is to be expected in the csv file.

```sql
CREATE EXTERNAL TABLE lineitem ( 
    L_ORDERKEY BIGINT NOT NULL, 
    L_PARTKEY BIGINT NOT NULL, 
    L_SUPPKEY BIGINT NOT NULL, 
    L_LINENUMBER INT NOT NULL, 
    L_QUANTITY DOUBLE NOT NULL, 
    L_EXTENDED_PRICE DOUBLE NOT NULL, 
    L_DISCOUNT DOUBLE NOT NULL, 
    L_TAX DOUBLE NOT NULL, 
    L_RETURNFLAG CHAR NOT NULL, 
    L_LINESTATUS CHAR NOT NULL, 
    L_SHIPDATE DATE NOT NULL, 
    L_COMMITDATE DATE NOT NULL, 
    L_RECEIPTDATE DATE NOT NULL, 
    L_SHIPINSTRUCT VARCHAR NOT NULL, 
    L_SHIPMODE VARCHAR NOT NULL, 
    L_COMMENT VARCHAR NOT NULL ) STORED AS CSV LOCATION 's3://iceberg-tpch-csv/lineitem.csv' OPTIONS ('has_header' 'false');
```

Next, we need a schema for the S3Tables we want to create. The following command will create the `tpch` schema. Execute this only if you haven't created the schema before.

```sql
CREATE SCHEMA warehouse.tpch;
```

The next command will create the corresponding S3Tables table we want to insert the csv data into. We will partition the table by `shipdate` to speed up queries.

```sql
CREATE TABLE warehouse.tpch.lineitem ( 
    L_ORDERKEY BIGINT NOT NULL, 
    L_PARTKEY BIGINT NOT NULL, 
    L_SUPPKEY BIGINT NOT NULL, 
    L_LINENUMBER INT NOT NULL, 
    L_QUANTITY DOUBLE NOT NULL, 
    L_EXTENDED_PRICE DOUBLE NOT NULL, 
    L_DISCOUNT DOUBLE NOT NULL, 
    L_TAX DOUBLE NOT NULL, 
    L_RETURNFLAG CHAR NOT NULL, 
    L_LINESTATUS CHAR NOT NULL, 
    L_SHIPDATE DATE NOT NULL, 
    L_COMMITDATE DATE NOT NULL, 
    L_RECEIPTDATE DATE NOT NULL, 
    L_SHIPINSTRUCT VARCHAR NOT NULL, 
    L_SHIPMODE VARCHAR NOT NULL, 
    L_COMMENT VARCHAR NOT NULL ) STORED AS ICEBERG LOCATION '' PARTITIONED BY ( "month(L_SHIPDATE)" );
```

Finally we can insert the data from the csv file into the newly created Iceberg table. Notice that we can reference the csv file as `lineitem`.

```sql
INSERT INTO warehouse.tpch.lineitem select * from lineitem;
```
