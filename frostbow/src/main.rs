use std::{process::ExitCode, str::FromStr, sync::Arc};

use aws_config::BehaviorVersion;
use aws_credential_types::provider::ProvideCredentials;
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
use iceberg_rest_catalog::{
    apis::configuration::{AWSv4Key, ConfigurationBuilder},
    catalog::RestNoPrefixCatalogList,
};
use iceberg_rust::{catalog::CatalogList, error::Error as IcebergError};

#[cfg(feature = "rest")]
use iceberg_rest_catalog::catalog::RestCatalogList;
use iceberg_s3tables_catalog::S3TablesCatalogList;
use secrecy::SecretString;

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

    let mut catalog_url = args
        .catalog_url
        .ok_or(IcebergError::NotFound("ICEBERG_CATALOG_URL".to_string()))?;

    let storage = args.storage.or(Some("s3".to_owned()));
    let command = args.command;
    let files = args.file;

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
        } else if catalog_url.starts_with("https://glue") {
            let config = aws_config::load_defaults(BehaviorVersion::v2024_03_28()).await;

            if catalog_url == "https://glue" {
                catalog_url.push_str(&format!(
                    ".{}.amazonaws.com/iceberg",
                    &config
                        .region()
                        .ok_or(IcebergError::InvalidFormat("Region missing.".to_owned()))?
                        .to_string(),
                ));
            }

            let credentials = config
                .credentials_provider()
                .ok_or(IcebergError::NotFound("Region".to_owned()))?
                .provide_credentials()
                .await
                .unwrap();

            let aws_key = AWSv4Key {
                access_key: credentials.access_key_id().to_owned(),
                secret_key: SecretString::from_str(credentials.secret_access_key()).unwrap(),
                region: config
                    .region()
                    .ok_or(IcebergError::NotFound("Region".to_owned()))?
                    .to_string(),
                service: "glue".to_owned(),
            };

            let configuration = ConfigurationBuilder::default()
                .base_path(catalog_url.clone())
                .aws_v4_key(aws_key)
                .build()
                .unwrap();

            Arc::new(RestNoPrefixCatalogList::new(
                "iceberg",
                configuration,
                object_store,
            )) as Arc<dyn CatalogList>
        } else {
            let configuration = ConfigurationBuilder::default()
                .base_path(catalog_url.clone())
                .build()
                .unwrap();

            Arc::new(RestCatalogList::new(configuration, object_store))
        }
    };

    let catalog_list = Arc::new(IcebergCatalogList::new(iceberg_catalog_list.clone()).await?);

    let state = SessionStateBuilder::new()
        .with_default_features()
        .with_config(SessionConfig::default().with_information_schema(true))
        .with_catalog_list(catalog_list)
        .with_query_planner(Arc::new(IcebergQueryPlanner::new()))
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

    if !command.is_empty() {
        exec::exec_from_commands(&mut ctx, command, &mut print_options)
            .await
            .unwrap()
    } else if !files.is_empty() {
        exec::exec_from_files(&mut ctx, files, &mut print_options)
            .await
            .unwrap();
    } else {
        exec::exec_from_repl(&mut ctx, &mut print_options)
            .await
            .unwrap();
    }

    Ok(())
}
