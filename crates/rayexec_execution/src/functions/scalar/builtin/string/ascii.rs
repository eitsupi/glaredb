use rayexec_bullet::array::Array;
use rayexec_bullet::datatype::{DataType, DataTypeId};
use rayexec_bullet::executor::builder::{ArrayBuilder, PrimitiveBuffer};
use rayexec_bullet::executor::physical_type::PhysicalUtf8;
use rayexec_bullet::executor::scalar::UnaryExecutor;
use rayexec_error::Result;

use crate::expr::Expression;
use crate::functions::scalar::{PlannedScalarFunction, ScalarFunction, ScalarFunctionImpl};
use crate::functions::{invalid_input_types_error, plan_check_num_args, FunctionInfo, Signature};
use crate::logical::binder::table_list::TableList;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ascii;

impl FunctionInfo for Ascii {
    fn name(&self) -> &'static str {
        "ascii"
    }

    fn signatures(&self) -> &[Signature] {
        &[Signature {
            input: &[DataTypeId::Utf8],
            variadic: None,
            return_type: DataTypeId::Int32,
        }]
    }
}

impl ScalarFunction for Ascii {
    fn plan(
        &self,
        table_list: &TableList,
        inputs: Vec<Expression>,
    ) -> Result<PlannedScalarFunction> {
        plan_check_num_args(self, &inputs, 1)?;

        match inputs[0].datatype(table_list)? {
            DataType::Utf8 => Ok(PlannedScalarFunction {
                function: Box::new(*self),
                return_type: DataType::Int32,
                inputs,
                function_impl: Box::new(AsciiImpl),
            }),
            a => Err(invalid_input_types_error(self, &[a])),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsciiImpl;

impl ScalarFunctionImpl for AsciiImpl {
    fn execute(&self, inputs: &[&Array]) -> Result<Array> {
        let input = inputs[0];
        let builder = ArrayBuilder {
            datatype: DataType::Int32,
            buffer: PrimitiveBuffer::with_len(inputs[0].logical_len()),
        };

        UnaryExecutor::execute::<PhysicalUtf8, _, _>(input, builder, |v, buf| {
            let v = v.chars().next().map(|c| c as i32).unwrap_or(0);
            buf.put(&v)
        })
    }
}