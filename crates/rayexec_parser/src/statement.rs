use crate::ast::{Expr, ObjectReference};

#[derive(Debug, Clone)]
pub enum Statement<'a> {
    Query {},

    /// CREATE SCHEMA ...
    CreateSchema {
        reference: ObjectReference<'a>,
        if_not_exists: bool,
    },
    /// SET <variable> TO <value>
    SetVariable {
        reference: ObjectReference<'a>,
        value: Expr<'a>,
    },
}
