use crate::{
    TransactionReadSize,
    TransactionWriteSize,
};

/// Metrics related to a function execution.
pub struct FunctionExecutionSize {
    pub num_intervals: usize,
    pub read_size: TransactionReadSize,
    pub write_size: TransactionWriteSize,
    pub scheduled_size: TransactionWriteSize,
}
