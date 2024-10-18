use std::{process::ExitCode, sync::Arc};

use clap::Parser;
use datafusion::{
    execution::{
        config::SessionConfig,
        context::{SessionContext, SessionState},
        runtime_env::RuntimeEnv,
    },
    logical_expr::ScalarUDF,
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
use frostbow::IcebergContext;
use iceberg_rust::catalog::bucket::{Bucket, ObjectStoreBuilder};
use object_store::{aws::AmazonS3Builder, memory::InMemory};

#[cfg(feature = "rest")]
use iceberg_rest_catalog::{apis::configuration::Configuration, catalog::RestCatalogList};
#[cfg(feature = "sql")]
use iceberg_sql_catalog::SqlCatalogList;

#[cfg(all(feature = "rest", feature = "sql"))]
compile_error!("feature \"rest\" and feature \"sql\" cannot be enabled at the same time");

#[derive(Debug, Parser)]
#[clap(version, about)]
struct Args {
    #[clap(short = 'u', long = "catalog-url")]
    catalog_url: Option<String>,
    #[clap(short = 'b', long)]
    bucket: Option<String>,
    #[clap(short = 'U', long)]
    username: Option<String>,
    #[clap(short = 'W', long)]
    password: Option<String>,
}

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
        "Environment variable".to_string(),
        "ICEBERG_CATALOG_URL".to_string(),
    ))?;

    let bucket = args.bucket;

    let username = args.username;
    let password = args.password;

    let object_store = match &bucket {
        Some(bucket) => {
            let builder = AmazonS3Builder::from_env().with_bucket_name(bucket);

            ObjectStoreBuilder::S3(builder)
        }
        _ => ObjectStoreBuilder::Memory(Arc::new(InMemory::new())),
    };

    #[cfg(feature = "rest")]
    let iceberg_catalog_list = {
        let configuration = Configuration {
            base_path: catalog_url,
            user_agent: None,
            client: reqwest_middleware::ClientBuilder::new(reqwest::Client::new()).build(),
            basic_auth: username.map(|username| (username, password)),
            oauth_access_token: None,
            bearer_access_token: None,
            api_key: None,
        };

        Arc::new(RestCatalogList::new(configuration, object_store))
    };

    #[cfg(feature = "sql")]
    let iceberg_catalog_list = {
        Arc::new(
            SqlCatalogList::new(
                &catalog_url,
                match &bucket {
                    Some(bucket) => object_store.build(Bucket::S3(&bucket))?,
                    None => object_store.build(Bucket::Local)?,
                },
            )
            .await
            .unwrap(),
        )
    };

    let catalog_list = Arc::new(IcebergCatalogList::new(iceberg_catalog_list.clone()).await?);

    let session_config = SessionConfig::from_env()?
        .with_create_default_catalog_and_schema(true)
        .with_information_schema(true);

    let runtime_env = Arc::new(RuntimeEnv::default());

    let state = SessionState::new_with_config_rt_and_catalog_list(
        session_config,
        runtime_env,
        catalog_list,
    )
    .with_query_planner(Arc::new(IcebergQueryPlanner {}));

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

    exec::exec_from_repl(&mut ctx, &mut print_options)
        .await
        .unwrap();

    Ok(())
}
