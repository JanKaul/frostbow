use std::{process::ExitCode, sync::Arc};

use aws_config::BehaviorVersion;
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
use iceberg_file_catalog::FileCatalogList;
use iceberg_rust::{catalog::CatalogList, error::Error as IcebergError};

#[cfg(feature = "rest")]
use iceberg_rest_catalog::{apis::configuration::Configuration, catalog::RestCatalogList};
use iceberg_s3tables_catalog::S3TablesCatalogList;

#[cfg(not(feature = "rest"))]
compile_error!("feature \"rest\" must be enabled for cli");

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

    let catalog_url = args
        .catalog_url
        .ok_or(IcebergError::NotFound("ICEBERG_CATALOG_URL".to_string()))?;

    let storage = args.storage;
    let command = args.command;

    let object_store = get_storage(storage.as_deref()).await?;

    #[cfg(feature = "rest")]
    let iceberg_catalog_list = {
        if catalog_url.starts_with("s3://") {
            Arc::new(
                FileCatalogList::new(&catalog_url, object_store)
                    .await
                    .map_err(iceberg_rust::error::Error::from)?,
            ) as Arc<dyn CatalogList>
        } else if catalog_url.starts_with("arn:") {
            let config = aws_config::load_defaults(BehaviorVersion::v2024_03_28()).await;

            Arc::new(S3TablesCatalogList::new(
                &config,
                &catalog_url,
                object_store,
            )) as Arc<dyn CatalogList>
        } else {
            let configuration = Configuration {
                base_path: catalog_url,
                user_agent: None,
                client: reqwest_middleware::ClientBuilder::new(reqwest::Client::new()).build(),
                basic_auth: None,
                oauth_access_token: None,
                bearer_access_token: None,
                api_key: None,
            };

            Arc::new(RestCatalogList::new(configuration, object_store))
        }
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
