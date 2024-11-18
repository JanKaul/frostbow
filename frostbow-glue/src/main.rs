use std::{process::ExitCode, sync::Arc};

use aws_config::BehaviorVersion;
use clap::Parser;
use datafusion::{
    catalog_common::MemoryCatalogProviderList,
    execution::{context::SessionContext, SessionStateBuilder},
};
use datafusion_cli::{
    exec,
    print_format::PrintFormat,
    print_options::{MaxRows, PrintOptions},
};
use datafusion_iceberg::{
    catalog::catalog::IcebergCatalog, error::Error, planner::IcebergQueryPlanner,
};
use frostbow::{Args, IcebergContext};
use iceberg_glue_catalog::GlueCatalog;
use iceberg_rust::catalog::bucket::ObjectStoreBuilder;
use object_store::{aws::AmazonS3Builder, local::LocalFileSystem};

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(e) = main_inner().await {
        println!("Error: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

async fn main_inner() -> Result<(), Error> {
    let args = Args::parse();

    let bucket = args.bucket;
    let command = args.command;

    let object_store: ObjectStoreBuilder = match &bucket {
        Some(bucket) => {
            if bucket.starts_with("s3://") {
                let builder = AmazonS3Builder::from_env().with_bucket_name(bucket);

                ObjectStoreBuilder::S3(builder)
            } else {
                ObjectStoreBuilder::Filesystem(Arc::new(LocalFileSystem::new()))
            }
        }
        _ => ObjectStoreBuilder::memory(),
    };

    let config = aws_config::load_defaults(BehaviorVersion::v2024_03_28()).await;

    let iceberg_catalog = Arc::new(
        GlueCatalog::new(&config, "glue", object_store)
            .map_err(iceberg_rust::error::Error::from)?,
    );

    let catalog = Arc::new(IcebergCatalog::new(iceberg_catalog.clone(), None).await?);

    let iceberg_catalog_list = Arc::new(MemoryCatalogProviderList::new());

    iceberg_catalog_list
        .catalogs
        .insert("glue".to_owned(), catalog);

    let state = SessionStateBuilder::new()
        .with_default_features()
        .with_catalog_list(iceberg_catalog_list)
        .with_query_planner(Arc::new(IcebergQueryPlanner {}))
        .build();

    let mut print_options = PrintOptions {
        format: PrintFormat::Automatic,
        quiet: true,
        maxrows: MaxRows::Unlimited,
        color: true,
    };

    let ctx = SessionContext::new_with_state(state);

    // ctx.register_udf(ScalarUDF::from(RefreshMaterializedView::new(
    //     iceberg_catalog_list,
    // )));

    let mut ctx = IcebergContext(ctx);

    if command.is_empty() {
        exec::exec_from_repl(&mut ctx, &mut print_options)
            .await
            .unwrap();
    } else {
        exec::exec_from_commands(&mut ctx, command, &mut print_options)
            .await
            .unwrap()
    }

    Ok(())
}
