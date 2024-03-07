use std::{
    backtrace::Backtrace,
    sync::Arc,
};

#[derive(Clone, Debug, derive_more::Display)]
pub struct StackTrace(Arc<Backtrace>);

impl PartialEq for StackTrace {
    fn eq(&self, _other: &Self) -> bool {
        // Ignore stack_traces for testing equality
        true
    }
}
impl Eq for StackTrace {}

impl StackTrace {
    pub fn new() -> Self {
        StackTrace(Arc::new(Backtrace::capture()))
    }
}
