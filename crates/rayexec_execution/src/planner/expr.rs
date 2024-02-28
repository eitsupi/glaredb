use rayexec_error::{RayexecError, Result};
use rayexec_parser::ast;

use crate::{
    expr::{scalar::ScalarValue, Expression},
    types::batch::DataBatchSchema,
};

use super::{
    operator::LogicalExpression,
    plan::PlanContext,
    scope::{ColumnRef, Scope, TableReference},
};

/// An expanded select expression.
// TODO: Expand wildcard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpandedSelectExpr {
    /// A typical expression. Can be a reference to a column, or a more complex
    /// expression.
    Expr {
        /// The original expression.
        expr: ast::Expr,
        /// Either an alias provided by the user or a name we generate for the
        /// expression. If this references a column, then the name will just
        /// match that column.
        name: String,
    },
    /// An index of a column in the current scope. This is needed for wildcards
    /// since they're expanded to match some number of columns in the current
    /// scope.
    Column {
        /// Index of the column the current scope.
        idx: usize,
        /// Name of the column.
        name: String,
    },
}

impl ExpandedSelectExpr {
    pub fn column_name(&self) -> &str {
        match self {
            ExpandedSelectExpr::Expr { name, .. } => name,
            Self::Column { name, .. } => name,
        }
    }
}

/// Context for planning expressions.
#[derive(Debug, Clone)]
pub struct ExpressionContext<'a> {
    /// Plan context containing this expression.
    pub plan_context: &'a PlanContext<'a>,
    /// Scope for this expression.
    pub scope: &'a Scope,
    /// Schema of input that this expression will be executed on.
    pub input: &'a DataBatchSchema,
}

impl<'a> ExpressionContext<'a> {
    pub fn new(
        plan_context: &'a PlanContext,
        scope: &'a Scope,
        input: &'a DataBatchSchema,
    ) -> Self {
        ExpressionContext {
            plan_context,
            scope,
            input,
        }
    }

    pub fn expand_select_expr(&self, expr: ast::SelectExpr) -> Result<Vec<ExpandedSelectExpr>> {
        Ok(match expr {
            ast::SelectExpr::Expr(expr) => vec![ExpandedSelectExpr::Expr {
                expr,
                name: "?column?".to_string(),
            }],
            ast::SelectExpr::AliasedExpr(expr, alias) => vec![ExpandedSelectExpr::Expr {
                expr,
                name: alias.value,
            }],
            ast::SelectExpr::Wildcard(wildcard) => {
                // TODO: Exclude, replace
                // TODO: Need to omit "hidden" columns that may have been added to the scope.
                self.scope
                    .items
                    .iter()
                    .enumerate()
                    .map(|(idx, col)| ExpandedSelectExpr::Column {
                        idx,
                        name: col.column.clone(),
                    })
                    .collect()
            }
            ast::SelectExpr::QualifiedWildcard(reference, wildcard) => {
                // TODO: Exclude, replace
                // TODO: Need to omit "hidden" columns that may have been added to the scope.
                self.scope
                    .items
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, col)| match &col.alias {
                        // TODO: I got lazy. Need to check the entire reference.
                        Some(alias) if alias.table == reference.base().unwrap().value => {
                            Some(ExpandedSelectExpr::Column {
                                idx,
                                name: col.column.clone(),
                            })
                        }
                        _ => None,
                    })
                    .collect()
            }
        })
    }

    pub fn plan_expression(&self, expr: ast::Expr) -> Result<LogicalExpression> {
        match expr {
            ast::Expr::Ident(ident) => self.plan_ident(ident),
            ast::Expr::CompoundIdent(idents) => self.plan_idents(idents),
            ast::Expr::Literal(literal) => self.plan_literal(literal),
            ast::Expr::BinaryExpr { left, op, right } => Ok(LogicalExpression::Binary {
                op: op.try_into()?,
                left: Box::new(self.plan_expression(*left)?),
                right: Box::new(self.plan_expression(*right)?),
            }),
            _ => unimplemented!(),
        }
    }

    /// Plan a sql literal
    fn plan_literal(&self, literal: ast::Literal) -> Result<LogicalExpression> {
        Ok(match literal {
            ast::Literal::Number(n) => {
                if let Ok(n) = n.parse::<i64>() {
                    LogicalExpression::Literal(ScalarValue::Int64(n))
                } else if let Ok(n) = n.parse::<u64>() {
                    LogicalExpression::Literal(ScalarValue::UInt64(n))
                } else if let Ok(n) = n.parse::<f64>() {
                    LogicalExpression::Literal(ScalarValue::Float64(n))
                } else {
                    return Err(RayexecError::new(format!(
                        "Unable to parse {n} as a number"
                    )));
                }
            }
            ast::Literal::Boolean(b) => LogicalExpression::Literal(ScalarValue::Boolean(b)),
            ast::Literal::Null => LogicalExpression::Literal(ScalarValue::Null),
            ast::Literal::SingleQuotedString(s) => {
                LogicalExpression::Literal(ScalarValue::Utf8(s.to_string()))
            }
            other => {
                return Err(RayexecError::new(format!(
                    "Unusupported SQL literal: {other:?}"
                )))
            }
        })
    }

    /// Plan a single identifier.
    ///
    /// Assumed to be a column name either in the current scope or one of the
    /// outer scopes.
    fn plan_ident(&self, ident: ast::Ident) -> Result<LogicalExpression> {
        match self
            .scope
            .resolve_column(&self.plan_context.outer_scopes, None, &ident.value)?
        {
            Some(col) => Ok(LogicalExpression::ColumnRef(col)),
            None => Err(RayexecError::new(format!(
                "Missing column for reference: {}",
                &ident.value
            ))),
        }
    }

    /// Plan a compound identifier.
    ///
    /// Assumed to be a reference to a column either in the current scope or one
    /// of the outer scopes.
    fn plan_idents(&self, mut idents: Vec<ast::Ident>) -> Result<LogicalExpression> {
        fn format_err(table_ref: &TableReference, col: &str) -> String {
            format!("Missing column for reference: {table_ref}.{col}")
        }

        match idents.len() {
            0 => Err(RayexecError::new("Empty identifier")),
            1 => {
                // Single column.
                let ident = idents.pop().unwrap();
                self.plan_ident(ident)
            }
            2 | 3 | 4 => {
                // Qualified column.
                // 2 => 'table.column'
                // 3 => 'schema.table.column'
                // 4 => 'database.schema.table.column'
                // TODO: Struct fields.
                let col = idents.pop().unwrap();
                let table_ref = TableReference {
                    table: idents.pop().map(|ident| ident.value).unwrap(), // Must exist
                    schema: idents.pop().map(|ident| ident.value),         // May exist
                    database: idents.pop().map(|ident| ident.value),       // May exist
                };
                match self.scope.resolve_column(
                    &self.plan_context.outer_scopes,
                    Some(&table_ref),
                    &col.value,
                )? {
                    Some(col) => Ok(LogicalExpression::ColumnRef(col)),
                    None => Err(RayexecError::new(format_err(&table_ref, &col.value))), // Struct fields here.
                }
            }
            _ => Err(RayexecError::new(format!(
                "Too many identifier parts in {}",
                ast::ObjectReference(idents),
            ))), // TODO: Struct fields.
        }
    }
}
