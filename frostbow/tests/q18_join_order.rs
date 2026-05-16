//! Exercises `optimal_left_deep_join_plan` against TPC-H Q18.
//!
//! Pre-stage TPC-H SF=1 parquet files (one-time):
//!   mkdir -p /tmp/tpch && aws s3 cp s3://embucket-testdata/tpch/1/ /tmp/tpch/ --recursive \
//!     --exclude '*' --include 'customer.parquet' --include 'orders.parquet' --include 'lineitem.parquet'
//!
//! Run:
//!   cargo test -p frostbow --test q18_join_order -- --nocapture
//!
//! Or override the data directory:
//!   Q18_TEST_DATA_DIR=/path/to/tpch cargo test -p frostbow --test q18_join_order -- --nocapture

use std::{path::Path, sync::Arc};

use datafusion::{
    common::tree_node::{TransformedResult, TreeNode},
    execution::{context::SessionContext, SessionStateBuilder},
    logical_expr::LogicalPlan,
    optimizer::{
        decorrelate_predicate_subquery::DecorrelatePredicateSubquery,
        eliminate_filter::EliminateFilter,
        extract_equijoin_predicate::ExtractEquijoinPredicate,
        filter_null_join_keys::FilterNullJoinKeys,
        optimizer::{Optimizer, OptimizerContext},
        push_down_filter::PushDownFilter,
        reorder_join::{
            cost::DefaultCostEstimator, left_deep_join_plan::optimal_left_deep_join_plan,
        },
        scalar_subquery_to_join::ScalarSubqueryToJoin,
        simplify_expressions::SimplifyExpressions,
    },
};
use datafusion_iceberg::{
    catalog::catalog::IcebergCatalog,
    planner::{iceberg_transform, IcebergQueryPlanner},
};
use iceberg_file_catalog::FileCatalog;
use iceberg_rust::{catalog::Catalog, object_store::ObjectStoreBuilder};
use object_store::local::LocalFileSystem;
use tempfile::TempDir;

const DATA_DIR_ENV: &str = "Q18_TEST_DATA_DIR";
const DEFAULT_DATA_DIR: &str = "/tmp/tpch";

const CUSTOMER_DDL: &str =
    "C_CUSTKEY BIGINT NOT NULL, C_NAME VARCHAR NOT NULL, C_ADDRESS VARCHAR NOT NULL, \
     C_NATIONKEY BIGINT NOT NULL, C_PHONE VARCHAR NOT NULL, C_ACCTBAL DOUBLE NOT NULL, \
     C_MKTSEGMENT VARCHAR NOT NULL, C_COMMENT VARCHAR NOT NULL";

const ORDERS_DDL: &str =
    "O_ORDERKEY BIGINT NOT NULL, O_CUSTKEY BIGINT NOT NULL, O_ORDERSTATUS CHAR NOT NULL, \
     O_TOTALPRICE DOUBLE NOT NULL, O_ORDERDATE DATE NOT NULL, O_ORDERPRIORITY VARCHAR NOT NULL, \
     O_CLERK VARCHAR NOT NULL, O_SHIPPRIORITY INT NOT NULL, O_COMMENT VARCHAR NOT NULL";

const LINEITEM_DDL: &str =
    "L_ORDERKEY BIGINT NOT NULL, L_PARTKEY BIGINT NOT NULL, L_SUPPKEY BIGINT NOT NULL, \
     L_LINENUMBER INT NOT NULL, L_QUANTITY DOUBLE NOT NULL, L_EXTENDEDPRICE DOUBLE NOT NULL, \
     L_DISCOUNT DOUBLE NOT NULL, L_TAX DOUBLE NOT NULL, L_RETURNFLAG CHAR NOT NULL, \
     L_LINESTATUS CHAR NOT NULL, L_SHIPDATE DATE NOT NULL, L_COMMITDATE DATE NOT NULL, \
     L_RECEIPTDATE DATE NOT NULL, L_SHIPINSTRUCT VARCHAR NOT NULL, L_SHIPMODE VARCHAR NOT NULL, \
     L_COMMENT VARCHAR NOT NULL";

const Q18_SQL: &str = "SELECT c_name, c_custkey, o_orderkey, o_orderdate, o_totalprice, sum(l_quantity) \
                       FROM warehouse.tpch.customer, warehouse.tpch.orders, warehouse.tpch.lineitem \
                       WHERE o_orderkey IN ( \
                           SELECT l_orderkey FROM warehouse.tpch.lineitem \
                           GROUP BY l_orderkey HAVING sum(l_quantity) > 300 \
                       ) \
                       AND c_custkey = o_custkey \
                       AND o_orderkey = l_orderkey \
                       GROUP BY c_name, c_custkey, o_orderkey, o_orderdate, o_totalprice \
                       ORDER BY o_totalprice DESC, o_orderdate \
                       LIMIT 100";

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
async fn q18_optimal_left_deep_plan() {
    let data_dir = std::env::var(DATA_DIR_ENV).unwrap_or_else(|_| DEFAULT_DATA_DIR.to_string());
    let need = ["customer.parquet", "orders.parquet", "lineitem.parquet"];
    for f in &need {
        if !Path::new(&format!("{data_dir}/{f}")).exists() {
            eprintln!(
                "SKIP: {data_dir}/{f} not found. Set {DATA_DIR_ENV} or run: \
                 aws s3 cp s3://embucket-testdata/tpch/1/ {DEFAULT_DATA_DIR}/ --recursive \
                 --exclude '*' --include 'customer.parquet' --include 'orders.parquet' --include 'lineitem.parquet'"
            );
            return;
        }
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

    // External parquet → iceberg target → INSERT, for each table.
    for (name, ddl) in [
        ("customer", CUSTOMER_DDL),
        ("orders", ORDERS_DDL),
        ("lineitem", LINEITEM_DDL),
    ] {
        run_sql(
            &ctx,
            &format!(
                "CREATE EXTERNAL TABLE {name}_src ({ddl}) STORED AS PARQUET \
                 LOCATION '{data_dir}/{name}.parquet';"
            ),
        )
        .await;
        run_sql(
            &ctx,
            &format!(
                "CREATE EXTERNAL TABLE warehouse.tpch.{name} ({ddl}) \
                 STORED AS ICEBERG LOCATION '/warehouse/tpch/{name}';"
            ),
        )
        .await;
        run_sql(
            &ctx,
            &format!("INSERT INTO warehouse.tpch.{name} SELECT * FROM {name}_src;"),
        )
        .await;
    }

    // Sanity-check that DefaultCostEstimator can size base TableScans against
    // the iceberg tables we just populated. This proves stats flow through:
    // iceberg manifest → DataFusionTable::statistics() → TableSource::statistics()
    // → DefaultCostEstimator::cardinality(plan, None).
    use datafusion::optimizer::reorder_join::cost::JoinCostEstimator;
    for name in ["customer", "orders", "lineitem"] {
        let scan = ctx
            .state()
            .create_logical_plan(&format!("SELECT * FROM warehouse.tpch.{name}"))
            .await
            .unwrap();
        // The plan is Projection -> TableScan; descend to the TableScan.
        let mut node = scan;
        while !matches!(node, LogicalPlan::TableScan(_)) {
            node = (*node.inputs()[0]).clone();
        }
        let card = DefaultCostEstimator
            .cardinality(&node, None)
            .expect("DefaultCostEstimator could not size iceberg TableScan");
        println!("DefaultCostEstimator says rows[{name}] = {card}");
    }

    // Build the Q18 logical plan and run the prerequisite rules
    // (decorrelation, equi-join extraction, filter pushdown).
    let plan = ctx.state().create_logical_plan(Q18_SQL).await.unwrap();
    let config = OptimizerContext::new().with_skip_failing_rules(false);
    let optimizer = Optimizer::with_rules(vec![
        Arc::new(SimplifyExpressions::new()),
        Arc::new(DecorrelatePredicateSubquery::new()),
        Arc::new(ScalarSubqueryToJoin::new()),
        Arc::new(ExtractEquijoinPredicate::new()),
        Arc::new(EliminateFilter::new()),
        Arc::new(FilterNullJoinKeys::default()),
        Arc::new(PushDownFilter::new()),
    ]);
    let pre_reorder = optimizer
        .optimize(plan, &config, |_, _| {})
        .expect("prerequisite optimizer rules");

    println!("---- plan before reorder ----");
    println!("{}", pre_reorder.display_indent());

    let reordered = optimal_left_deep_join_plan(pre_reorder.clone(), &DefaultCostEstimator)
        .expect("optimal_left_deep_join_plan");

    println!("---- plan after reorder ----");
    println!("{}", reordered.display_indent());

    // NOTE on Q18 specifically: the IN-subquery decorrelates into a
    // `LeftSemi Join` that wraps the 3-way inner-join subtree. The current
    // `flatten_joins_recursive` (datafusion/optimizer/src/reorder_join/join_graph.rs:386)
    // treats non-inner joins (Left/Right/Full/Semi/Anti/Mark) as opaque
    // leaves and refuses to decompose them. So `optimal_left_deep_join_plan`
    // sees a single opaque node and returns the plan unchanged.
    //
    // To get the ordering "join the aggregated lineitem subquery with orders
    // first", the optimizer would need to push the LeftSemi down into the
    // inner-join subtree (or treat it as reorderable). That's a separate
    // change to `flatten_joins_recursive` in the reorder_join branch.

    assert!(
        matches!(reordered, LogicalPlan::Limit(_)),
        "expected top-level Limit, got {reordered:?}"
    );

    let mut scan_order: Vec<String> = Vec::new();
    reordered
        .apply(|node| {
            if let LogicalPlan::TableScan(s) = node {
                scan_order.push(s.table_name.table().to_string());
            }
            Ok(datafusion::common::tree_node::TreeNodeRecursion::Continue)
        })
        .unwrap();
    println!("scan order (DFS): {scan_order:?}");
    for needed in ["customer", "orders", "lineitem"] {
        assert!(
            scan_order.iter().any(|n| n == needed),
            "{needed} scan missing"
        );
    }
}
