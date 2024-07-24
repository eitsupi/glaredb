use std::{
    collections::VecDeque,
    fmt::{self, Debug},
    sync::Arc,
};

use crate::{metadata::Metadata, reader::AsyncBatchReader};
use futures::future::BoxFuture;
use rayexec_bullet::{batch::Batch, field::Schema};
use rayexec_error::Result;
use rayexec_execution::{
    database::table::{DataTable, DataTableScan},
    runtime::ExecutionRuntime,
};
use rayexec_io::{
    location::{AccessConfig, FileLocation},
    FileSource,
};

/// Data table implementation which parallelizes on row groups. During scanning,
/// each returned scan object is responsible for distinct row groups to read.
#[derive(Debug)]
pub struct RowGroupPartitionedDataTable {
    pub metadata: Arc<Metadata>,
    pub schema: Schema,
    pub location: FileLocation,
    pub conf: AccessConfig,
    pub runtime: Arc<dyn ExecutionRuntime>,
}

impl DataTable for RowGroupPartitionedDataTable {
    fn scan(&self, num_partitions: usize) -> Result<Vec<Box<dyn DataTableScan>>> {
        let file_provider = self.runtime.file_provider();

        let mut partitioned_row_groups = vec![VecDeque::new(); num_partitions];

        // Split row groups into individual partitions.
        for row_group in 0..self.metadata.parquet_metadata.row_groups().len() {
            let partition = row_group % num_partitions;
            partitioned_row_groups[partition].push_back(row_group);
        }

        let readers = partitioned_row_groups
            .into_iter()
            .map(|row_groups| {
                let reader = file_provider.file_source(self.location.clone(), &self.conf)?;
                const BATCH_SIZE: usize = 2048; // TODO
                AsyncBatchReader::try_new(
                    reader,
                    row_groups,
                    self.metadata.clone(),
                    &self.schema,
                    BATCH_SIZE,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        let scans: Vec<Box<dyn DataTableScan>> = readers
            .into_iter()
            .map(|reader| Box::new(RowGroupsScan { reader }) as _)
            .collect();

        Ok(scans)
    }
}

struct RowGroupsScan {
    reader: AsyncBatchReader<Box<dyn FileSource>>,
}

impl DataTableScan for RowGroupsScan {
    fn pull(&mut self) -> BoxFuture<'_, Result<Option<Batch>>> {
        Box::pin(async { self.reader.read_next().await })
    }
}

impl fmt::Debug for RowGroupsScan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RowGroupsScan").finish_non_exhaustive()
    }
}