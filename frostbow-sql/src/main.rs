use std::{process::ExitCode, sync::Arc};

use clap::Parser;
use datafusion::{
    execution::{context::SessionContext, SessionStateBuilder},
    logical_expr::ScalarUDF,
    prelude::SessionConfig,
};
use datafusion_cli::{
    exec,
    print_format::PrintFormat,
    print_options::{MaxRows, PrintOptions},
};
use datafusion_iceberg::{
    catalog::catalog_list::IcebergCatalogList,
    error::Error,
    planner::{IcebergQueryPlanner, RefreshMaterializedView},
};
use frostbow::{get_storage, Args, IcebergContext};

use iceberg_sql_catalog::SqlCatalogList;

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

    let catalog_url = args.catalog_url.ok_or(Error::NotFound(
        "Argument".to_string(),
        "ICEBERG_CATALOG_URL".to_string(),
    ))?;

    let storage = args.storage;
    let command = args.command;

    let object_store = get_storage(storage.as_deref())?;

    let iceberg_catalog_list = {
        Arc::new(
            SqlCatalogList::new(&catalog_url, object_store)
                .await
                .unwrap(),
        )
    };

    let catalog_list = Arc::new(IcebergCatalogList::new(iceberg_catalog_list.clone()).await?);

    let state = SessionStateBuilder::new()
        .with_default_features()
        .with_config(SessionConfig::default().with_information_schema(true))
        .with_catalog_list(catalog_list)
        .with_query_planner(Arc::new(IcebergQueryPlanner {}))
        .build();

    let mut print_options = PrintOptions {
        format: PrintFormat::Automatic,
        quiet: true,
        maxrows: MaxRows::Unlimited,
        color: true,
    };

    let ctx = SessionContext::new_with_state(state);

    ctx.register_udf(ScalarUDF::from(RefreshMaterializedView::new(
        iceberg_catalog_list,
    )));

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
