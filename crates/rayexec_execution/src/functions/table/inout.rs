use std::fmt::Debug;
use std::task::Context;

use dyn_clone::DynClone;
use rayexec_error::Result;

use crate::arrays::batch::Batch;
use crate::execution::operators::{PollFinalize, PollPush};

pub trait TableInOutFunction: Debug + Sync + Send + DynClone {
    fn create_states(
        &self,
        num_partitions: usize,
    ) -> Result<Vec<Box<dyn TableInOutPartitionState>>>;
}

#[derive(Debug)]
pub enum InOutPollPull {
    Batch { batch: Batch, row_nums: Vec<usize> },
    Pending,
    Exhausted,
}

pub trait TableInOutPartitionState: Debug + Sync + Send {
    fn poll_push(&mut self, cx: &mut Context, inputs: Batch) -> Result<PollPush>;
    fn poll_finalize_push(&mut self, cx: &mut Context) -> Result<PollFinalize>;
    fn poll_pull(&mut self, cx: &mut Context) -> Result<InOutPollPull>;
}

impl Clone for Box<dyn TableInOutFunction> {
    fn clone(&self) -> Self {
        dyn_clone::clone_box(&**self)
    }
}
