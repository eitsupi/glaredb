use num_traits::Float;
use rayexec_bullet::array::{Array, ArrayData};
use rayexec_bullet::datatype::DataType;
use rayexec_bullet::executor::builder::{ArrayBuilder, PrimitiveBuffer};
use rayexec_bullet::executor::physical_type::PhysicalStorage;
use rayexec_bullet::executor::scalar::UnaryExecutor;
use rayexec_bullet::storage::PrimitiveStorage;
use rayexec_error::Result;

use super::{UnaryInputNumericOperation, UnaryInputNumericScalar};

pub type Acos = UnaryInputNumericScalar<AcosOp>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcosOp;

impl UnaryInputNumericOperation for AcosOp {
    const NAME: &'static str = "acos";
    const DESCRIPTION: &'static str = "Compute the arccosine of value";

    fn execute_float<'a, S>(input: &'a Array, ret: DataType) -> Result<Array>
    where
        S: PhysicalStorage<'a>,
        S::Type: Float + Default,
        ArrayData: From<PrimitiveStorage<S::Type>>,
    {
        let builder = ArrayBuilder {
            datatype: ret,
            buffer: PrimitiveBuffer::with_len(input.logical_len()),
        };
        UnaryExecutor::execute::<S, _, _>(input, builder, |v, buf| buf.put(&v.acos()))
    }
}
