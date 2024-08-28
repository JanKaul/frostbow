use datafusion::logical_expr::{CreateExternalTable, UserDefinedLogicalNode};
use std::{fmt::Debug, hash::Hash, sync::Arc};

#[derive(Clone)]
pub struct CreateIcebergTable(CreateExternalTable);

impl Debug for CreateIcebergTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = &self.0.name;
        let constraints = &self.0.constraints;
        write!(f, "CreateIcebergTable: {name:?}{constraints}")
    }
}

impl UserDefinedLogicalNode for CreateIcebergTable {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "CreateIcebergTable"
    }

    fn inputs(&self) -> Vec<&datafusion::logical_expr::LogicalPlan> {
        vec![]
    }

    fn schema(&self) -> &datafusion::common::DFSchemaRef {
        &self.0.schema
    }

    fn expressions(&self) -> Vec<datafusion::prelude::Expr> {
        vec![]
    }

    fn fmt_for_explain(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let name = &self.0.name;
        let constraints = &self.0.constraints;
        write!(f, "CreateIcebergTable: {name:?}{constraints}")
    }

    fn with_exprs_and_inputs(
        &self,
        _exprs: Vec<datafusion::prelude::Expr>,
        _inputs: Vec<datafusion::logical_expr::LogicalPlan>,
    ) -> datafusion::error::Result<std::sync::Arc<dyn UserDefinedLogicalNode>> {
        Ok(Arc::new(self.clone()))
    }

    fn dyn_hash(&self, mut state: &mut dyn std::hash::Hasher) {
        self.0.hash(&mut state)
    }

    fn dyn_eq(&self, other: &dyn UserDefinedLogicalNode) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<CreateIcebergTable>() {
            self.0.eq(&other.0)
        } else {
            false
        }
    }
}
