use async_trait::async_trait;
use clap::Parser;
use std::sync::Arc;

use datafusion::{
    common::tree_node::{TransformedResult, TreeNode},
    dataframe::DataFrame,
    error::DataFusionError,
    execution::{
        context::{SessionContext, SessionState},
        TaskContext,
    },
    logical_expr::LogicalPlan,
};
use datafusion_cli::{
    cli_context::CliSessionContext,
    object_storage::{AwsOptions, GcpOptions},
};
use datafusion_iceberg::planner::iceberg_transform;
use object_store::ObjectStore;

#[derive(Debug, Parser)]
#[clap(version, about)]
pub struct Args {
    #[clap(short = 'u', long = "catalog-url")]
    pub catalog_url: Option<String>,
    #[clap(short = 'b', long)]
    pub bucket: Option<String>,
    #[clap(short = 'U', long)]
    pub username: Option<String>,
    #[clap(short = 'W', long)]
    pub password: Option<String>,
}

pub struct IcebergContext(pub SessionContext);

#[async_trait]
impl CliSessionContext for IcebergContext {
    fn task_ctx(&self) -> Arc<TaskContext> {
        self.0.task_ctx()
    }

    fn session_state(&self) -> SessionState {
        self.0.state()
    }

    fn register_object_store(
        &self,
        url: &url::Url,
        object_store: Arc<dyn ObjectStore>,
    ) -> Option<Arc<dyn ObjectStore + 'static>> {
        self.0.register_object_store(url, object_store)
    }

    fn register_table_options_extension_from_scheme(&self, scheme: &str) {
        match scheme {
            // For Amazon S3 or Alibaba Cloud OSS
            "s3" | "oss" | "cos" => {
                // Register AWS specific table options in the session context:
                self.0
                    .register_table_options_extension(AwsOptions::default())
            }
            // For Google Cloud Storage
            "gs" | "gcs" => {
                // Register GCP specific table options in the session context:
                self.0
                    .register_table_options_extension(GcpOptions::default())
            }
            // For unsupported schemes, do nothing:
            _ => {}
        }
    }

    async fn execute_logical_plan(&self, plan: LogicalPlan) -> Result<DataFrame, DataFusionError> {
        let plan = plan.transform(iceberg_transform).data()?;
        self.0.execute_logical_plan(plan).await
    }
}
