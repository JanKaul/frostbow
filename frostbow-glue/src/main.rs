use std::{process::ExitCode, sync::Arc};

use aws_config::BehaviorVersion;
use clap::Parser;
use datafusion::{
    catalog::MemoryCatalogProviderList,
    execution::{context::SessionContext, SessionStateBuilder},
    prelude::SessionConfig,
};
use datafusion_cli::{
    exec,
    print_format::PrintFormat,
    print_options::{MaxRows, PrintOptions},
};
use datafusion_iceberg::{
    catalog::catalog::IcebergCatalog, error::Error, planner::IcebergQueryPlanner,
};
use frostbow::{get_storage, Args, IcebergContext};
use iceberg_glue_catalog::GlueCatalog;

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

    let storage = args.storage;
    let command = args.command;
    let files = args.file;

    let object_store = get_storage(storage.as_deref()).await?;

    let config = aws_config::load_defaults(BehaviorVersion::v2025_01_17()).await;

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
        .with_config(SessionConfig::default().with_information_schema(true))
        .with_catalog_list(iceberg_catalog_list)
        .with_query_planner(Arc::new(IcebergQueryPlanner::new()))
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

    let ctx = IcebergContext(ctx);

    if !command.is_empty() {
        exec::exec_from_commands(&ctx, command, &print_options)
            .await
            .unwrap()
    } else if !files.is_empty() {
        exec::exec_from_files(&ctx, files, &print_options)
            .await
            .unwrap();
    } else {
        exec::exec_from_repl(&ctx, &mut print_options)
            .await
            .unwrap();
    }

    Ok(())
}
