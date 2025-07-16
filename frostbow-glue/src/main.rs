use std::{process::ExitCode, sync::Arc};

use aws_config::BehaviorVersion;
use clap::Parser;
use datafusion::{
    catalog::MemoryCatalogProviderList,
    execution::{
        context::SessionContext, memory_pool::GreedyMemoryPool, runtime_env::RuntimeEnvBuilder,
        SessionStateBuilder,
    },
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
use frostbow::{get_storage, Args, IcebergContext, BYTES_IN_GIBIBYTE};
use iceberg_glue_catalog::GlueCatalog;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "frostbow_glue=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    if let Err(e) = main_inner().await {
        tracing::error!("Error: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

async fn main_inner() -> Result<(), Error> {
    let args = Args::parse();

    let storage = args.storage;
    let command = args.command;
    let files = args.file;

    tracing::info!("Initializing storage with provider: {:?}", storage);
    let object_store = get_storage(storage.as_deref()).await?;

    tracing::info!("Loading AWS configuration");
    let config = aws_config::load_defaults(BehaviorVersion::v2025_01_17()).await;

    tracing::info!("Initializing Glue catalog");
    let iceberg_catalog = Arc::new(
        GlueCatalog::new(&config, "glue", object_store)
            .map_err(iceberg_rust::error::Error::from)?,
    );

    let catalog = Arc::new(IcebergCatalog::new(iceberg_catalog.clone(), None).await?);

    let iceberg_catalog_list = Arc::new(MemoryCatalogProviderList::new());

    iceberg_catalog_list
        .catalogs
        .insert("glue".to_owned(), catalog);

    let runtime_env_builder = RuntimeEnvBuilder::new();
    let runtime_env_builder = if let Some(limit) = args.memory {
        runtime_env_builder
            .with_memory_pool(Arc::new(GreedyMemoryPool::new(limit * BYTES_IN_GIBIBYTE)))
    } else {
        runtime_env_builder
    };
    let runtime_env = Arc::new(runtime_env_builder.build()?);

    tracing::info!("Initializing DataFusion session");
    let state = SessionStateBuilder::new()
        .with_default_features()
        .with_config(SessionConfig::default().with_information_schema(true))
        .with_runtime_env(runtime_env)
        .with_catalog_list(iceberg_catalog_list)
        .with_query_planner(Arc::new(IcebergQueryPlanner::new()))
        .build();

    let mut print_options = PrintOptions {
        format: PrintFormat::Automatic,
        quiet: true,
        maxrows: MaxRows::Limited(10000),
        color: true,
    };

    let ctx = SessionContext::new_with_state(state);

    // ctx.register_udf(ScalarUDF::from(RefreshMaterializedView::new(
    //     iceberg_catalog_list,
    // )));

    let ctx = IcebergContext(ctx);

    if !command.is_empty() {
        tracing::info!("Executing command: {:?}", command);
        exec::exec_from_commands(&ctx, command, &print_options)
            .await
            .unwrap()
    } else if !files.is_empty() {
        tracing::info!("Executing files: {:?}", files);
        exec::exec_from_files(&ctx, files, &print_options)
            .await
            .unwrap();
    } else {
        tracing::info!("Starting REPL");
        exec::exec_from_repl(&ctx, &mut print_options)
            .await
            .unwrap();
    }

    Ok(())
}
