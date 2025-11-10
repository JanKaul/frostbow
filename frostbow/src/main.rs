use std::{process::ExitCode, str::FromStr, sync::Arc};

use aws_config::BehaviorVersion;
use aws_credential_types::provider::ProvideCredentials;
use clap::Parser;
use datafusion::{
    execution::{
        context::SessionContext, memory_pool::GreedyMemoryPool, runtime_env::RuntimeEnvBuilder,
        SessionStateBuilder,
    },
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
use frostbow::{get_storage, Args, IcebergContext, BYTES_IN_GIBIBYTE};
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
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(not(feature = "rest"))]
compile_error!("feature \"rest\" must be enabled for cli");

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "frostbow=info".into()),
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

    let mut catalog_url = args
        .catalog_url
        .ok_or(IcebergError::NotFound("ICEBERG_CATALOG_URL".to_string()))?;

    let storage = args.storage.or(Some("s3".to_owned()));
    let command = args.command;
    let files = args.file;

    tracing::info!("Initializing storage with provider: {:?}", storage);
    let object_store = get_storage(storage.as_deref()).await?;

    #[cfg(feature = "rest")]
    let iceberg_catalog_list = {
        if catalog_url.starts_with("s3://") {
            tracing::info!("Using file catalog with URL: {}", catalog_url);
            Arc::new(
                FileCatalogList::new(&catalog_url, object_store)
                    .await
                    .map_err(iceberg_rust::error::Error::from)?,
            ) as Arc<dyn CatalogList>
        } else if catalog_url.starts_with("arn:") {
            tracing::info!("Using S3 tables catalog with ARN: {}", catalog_url);
            let config = aws_config::load_defaults(BehaviorVersion::v2025_08_07()).await;

            Arc::new(S3TablesCatalogList::new(
                &config,
                &catalog_url,
                object_store,
            )) as Arc<dyn CatalogList>
        } else if catalog_url.starts_with("https://glue") {
            tracing::info!("Using Glue catalog with URL: {}", catalog_url);
            let config = aws_config::load_defaults(BehaviorVersion::v2025_08_07()).await;

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
                session_token: credentials
                    .session_token()
                    .map(SecretString::from_str)
                    .transpose()
                    .unwrap(),
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
                Some(object_store),
            )) as Arc<dyn CatalogList>
        } else if catalog_url.starts_with("https://s3tables") {
            tracing::info!("Using S3 tables REST catalog with URL: {}", catalog_url);
            let config = aws_config::load_defaults(BehaviorVersion::v2025_08_07()).await;

            if catalog_url == "https://s3tables" {
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
                session_token: credentials
                    .session_token()
                    .map(SecretString::from_str)
                    .transpose()
                    .unwrap(),
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

            Arc::new(RestCatalogList::new(configuration, Some(object_store)))
                as Arc<dyn CatalogList>
        } else {
            tracing::info!("Using REST catalog with URL: {}", catalog_url);
            let configuration = ConfigurationBuilder::default()
                .base_path(catalog_url.clone())
                .build()
                .unwrap();

            Arc::new(RestCatalogList::new(configuration, Some(object_store)))
        }
    };

    let catalog_list = Arc::new(IcebergCatalogList::new(iceberg_catalog_list.clone()).await?);

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
        .with_catalog_list(catalog_list)
        .with_query_planner(Arc::new(IcebergQueryPlanner::new()))
        .build();

    let mut print_options = PrintOptions {
        format: PrintFormat::Automatic,
        quiet: true,
        maxrows: MaxRows::Limited(10000),
        color: true,
    };

    let ctx = SessionContext::new_with_state(state);

    ctx.register_udf(ScalarUDF::from(RefreshMaterializedView::new(
        iceberg_catalog_list,
    )));

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
