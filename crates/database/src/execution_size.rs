use crate::{
    TransactionReadSize,
    TransactionWriteSize,
};

/// Metrics related to a function execution.
pub struct FunctionExecutionSize {
    pub num_intervals: usize,
    pub read_size: TransactionReadSize,
    pub write_size: TransactionWriteSize,
    pub scheduled_size: ScheduledFunctionsSize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScheduledFunctionsSize {
    /// Number of scheduled functions
    pub num_writes: usize,
    /// Sum of scheduled argument sizes
    pub size: usize,
    /// Max of scheduled argument sizes
    pub max_args_size: usize,
}
