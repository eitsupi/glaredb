use crate::{
    explain::explainable::{ExplainConfig, ExplainEntry, Explainable},
    expr::Expression,
};

use super::{
    binder::bind_context::{MaterializationRef, TableRef},
    operator::{LogicalNode, Node},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalMaterializationScan {
    /// Reference to the materialization in the bind context.
    pub mat: MaterializationRef,
    /// Table references of the output of the materialization.
    ///
    /// These should match the references that are stored on the materialization
    /// in the bind context. They are duplicated here for convenience.
    pub table_refs: Vec<TableRef>,
}

impl Explainable for LogicalMaterializationScan {
    fn explain_entry(&self, conf: ExplainConfig) -> ExplainEntry {
        let mut ent =
            ExplainEntry::new("MaterializationScan").with_value("materialization_ref", self.mat);
        if conf.verbose {
            ent = ent.with_values("table_refs", &self.table_refs)
        }
        ent
    }
}

impl LogicalNode for Node<LogicalMaterializationScan> {
    fn get_output_table_refs(&self) -> Vec<TableRef> {
        self.node.table_refs.clone()
    }
}

/// An alternative materialized scan operator that projects out of the
/// materialization and removes duplicates.
///
/// This should only be found in the child of a magic join.
///
/// These are created during subquery decorrelation such that the branch in the
/// plan representing work for a subquery is working with deduplicated inputs.
///
/// Essentially this encodes a disctint, project, and materialized scan into a
/// single operator, e.g.:
/// ```text
/// DISTINCT column1, column2
///   PROJECT <expr> as column1, <expr> as column2
///      MATERIALIZED_SCAN ...
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalMagicMaterializationScan {
    /// Reference to the materialization in the bind context.
    pub mat: MaterializationRef,
    /// Projections out of the materialization scan that will have duplicates
    /// removed.
    pub projections: Vec<Expression>,
    /// The table ref for this scan.
    ///
    /// This operator exposes a new reference since all parent operators must
    /// reference the deduplicated projections, and nothing inside the
    /// materialization.
    pub table_ref: TableRef,
}

impl Explainable for LogicalMagicMaterializationScan {
    fn explain_entry(&self, conf: ExplainConfig) -> ExplainEntry {
        let mut ent = ExplainEntry::new("MagicMaterializationScan")
            .with_value("materialization_ref", self.mat)
            .with_values("projections", &self.projections);
        if conf.verbose {
            ent = ent.with_value("table_ref", self.table_ref)
        }
        ent
    }
}

impl LogicalNode for Node<LogicalMagicMaterializationScan> {
    fn get_output_table_refs(&self) -> Vec<TableRef> {
        vec![self.node.table_ref]
    }
}
