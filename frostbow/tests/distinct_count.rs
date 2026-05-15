//! End-to-end test for distinct-count metadata population.
//!
//! Pre-stage the source CSV (one-time):
//!   aws s3 cp s3://embucket-testdata/tpch/lineitem.csv /tmp/lineitem.csv
//!
//! Run:
//!   cargo test -p frostbow --test distinct_count -- --nocapture
//!
//! Or override the source path:
//!   DISTINCT_COUNT_TEST_CSV=/path/to/lineitem.csv cargo test ...

use std::{collections::HashMap, path::Path, sync::Arc};

use datafusion::{
    common::tree_node::{TransformedResult, TreeNode},
    execution::{context::SessionContext, SessionStateBuilder},
};
use datafusion_iceberg::{
    catalog::catalog::IcebergCatalog,
    planner::{iceberg_transform, IcebergQueryPlanner},
};
use iceberg_file_catalog::FileCatalog;
use iceberg_rust::{
    catalog::{identifier::Identifier, tabular::Tabular, Catalog},
    object_store::ObjectStoreBuilder,
};
use object_store::local::LocalFileSystem;
use tempfile::TempDir;

const CSV_PATH_ENV: &str = "DISTINCT_COUNT_TEST_CSV";
const DEFAULT_CSV_PATH: &str = "/tmp/lineitem.csv";
const TARGET_COLUMNS: &[&str] = &["l_orderkey", "l_partkey", "l_suppkey"];

const COLS_DDL: &str = "L_ORDERKEY BIGINT NOT NULL, L_PARTKEY BIGINT NOT NULL, \
    L_SUPPKEY BIGINT NOT NULL, L_LINENUMBER INT NOT NULL, L_QUANTITY DOUBLE NOT NULL, \
    L_EXTENDED_PRICE DOUBLE NOT NULL, L_DISCOUNT DOUBLE NOT NULL, L_TAX DOUBLE NOT NULL, \
    L_RETURNFLAG CHAR NOT NULL, L_LINESTATUS CHAR NOT NULL, L_SHIPDATE DATE NOT NULL, \
    L_COMMITDATE DATE NOT NULL, L_RECEIPTDATE DATE NOT NULL, L_SHIPINSTRUCT VARCHAR NOT NULL, \
    L_SHIPMODE VARCHAR NOT NULL, L_COMMENT VARCHAR NOT NULL";

async fn run_sql(ctx: &SessionContext, sql: &str) {
    let plan = ctx.state().create_logical_plan(sql).await.unwrap();
    let plan = plan.transform(iceberg_transform).data().unwrap();
    ctx.execute_logical_plan(plan)
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn distinct_counts_populated_on_insert() {
    let csv_path = std::env::var(CSV_PATH_ENV).unwrap_or_else(|_| DEFAULT_CSV_PATH.to_string());
    if !Path::new(&csv_path).exists() {
        eprintln!(
            "SKIP: CSV not found at {csv_path}. Set {CSV_PATH_ENV} or run: \
             aws s3 cp s3://embucket-testdata/tpch/lineitem.csv {DEFAULT_CSV_PATH}"
        );
        return;
    }

    let warehouse = TempDir::new().unwrap();
    let warehouse_url = format!("file://{}", warehouse.path().display());
    let object_store = ObjectStoreBuilder::Filesystem(Arc::new(LocalFileSystem::new()));

    let iceberg_catalog: Arc<dyn Catalog> = Arc::new(
        FileCatalog::new(&warehouse_url, object_store)
            .await
            .expect("create FileCatalog"),
    );
    let df_catalog = Arc::new(
        IcebergCatalog::new(iceberg_catalog.clone(), None)
            .await
            .expect("wrap as DataFusion CatalogProvider"),
    );

    let state = SessionStateBuilder::new()
        .with_default_features()
        .with_query_planner(Arc::new(IcebergQueryPlanner::new()))
        .build();
    let ctx = SessionContext::new_with_state(state);
    ctx.register_catalog("warehouse", df_catalog);

    run_sql(&ctx, "CREATE SCHEMA warehouse.tpch;").await;

    run_sql(
        &ctx,
        &format!(
            "CREATE EXTERNAL TABLE lineitem ({COLS_DDL}) STORED AS CSV \
             LOCATION '{csv_path}' OPTIONS ('has_header' 'false');"
        ),
    )
    .await;

    run_sql(
        &ctx,
        &format!(
            "CREATE EXTERNAL TABLE warehouse.tpch.lineitem ({COLS_DDL}) \
             STORED AS ICEBERG LOCATION '{warehouse_url}/tpch/lineitem' \
             PARTITIONED BY (\"month(L_SHIPDATE)\") \
             OPTIONS ('write.metadata.metrics.distinct-counts.enabled' 'true');"
        ),
    )
    .await;

    run_sql(
        &ctx,
        "INSERT INTO warehouse.tpch.lineitem SELECT * FROM lineitem;",
    )
    .await;

    let identifier = Identifier::new(&["tpch".to_string()], "lineitem");
    let tabular = iceberg_catalog
        .load_tabular(&identifier)
        .await
        .expect("load warehouse.tpch.lineitem from iceberg catalog");
    let table = match tabular {
        Tabular::Table(t) => t,
        other => panic!("expected Tabular::Table, got {other:?}"),
    };

    let schema = table.current_schema(None).expect("current schema");
    let id_by_name: HashMap<String, i32> = schema
        .iter()
        .map(|f| (f.name.to_lowercase(), f.id))
        .collect();
    let target_ids: Vec<i32> = TARGET_COLUMNS
        .iter()
        .map(|name| {
            *id_by_name
                .get(*name)
                .unwrap_or_else(|| panic!("column {name} not found in schema"))
        })
        .collect();

    let manifests = table.manifests(None, None).await.expect("read manifests");
    assert!(!manifests.is_empty(), "no manifests written");

    let entries: Vec<_> = table
        .datafiles(&manifests, None, (None, None))
        .await
        .expect("read data files")
        .collect::<Result<Vec<_>, _>>()
        .expect("datafiles iteration");
    assert!(!entries.is_empty(), "no data files in manifests");

    let mut saw_positive = false;
    for (_path, entry) in entries {
        let df = entry.data_file();
        let dc_map = df
            .distinct_counts()
            .as_ref()
            .expect("distinct_counts missing on DataFile");
        for (col_id, col_name) in target_ids.iter().zip(TARGET_COLUMNS.iter()) {
            let count = dc_map.get(col_id).unwrap_or_else(|| {
                panic!("distinct_counts missing column {col_name} (id {col_id})")
            });
            println!("col={col_name} id={col_id} distinct_count={count}");
            assert!(
                *count > 0,
                "distinct_count <= 0 for column {col_name} (id {col_id})"
            );
            saw_positive = true;
        }
    }
    assert!(
        saw_positive,
        "no positive distinct_counts seen across data files"
    );
}
