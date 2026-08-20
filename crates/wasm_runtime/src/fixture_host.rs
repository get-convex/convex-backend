//! The host the fixtures and benchmarks run against.
//!
//! Fixtures exist to exercise and measure the runtime itself rather than the
//! database, so their syscalls are answered out of a `BTreeMap` here instead of
//! a transaction. That makes this a [`ConvexSyscallHost`] like any other: the
//! fixture harness and the UDF path run the same [`crate::udf::UdfHostState`]
//! over the same wire format, and differ only in who answers.

use std::collections::BTreeMap;

use serde_json::{
    json,
    Value,
};

use crate::udf::{
    ConvexSyscallHost,
    PendingSyscall,
    SyscallOutcome,
};

/// Pinned so a fixture that stamps a record is byte-comparable between runs.
const FIXED_NOW_MS: u64 = 1_700_000_000_000;

#[derive(Default)]
pub struct FixtureHost {
    records: BTreeMap<String, Value>,
    logs: Vec<String>,
    next_uuid: u64,
    syscall_counts: BTreeMap<String, usize>,
}

impl FixtureHost {
    pub fn new() -> Self {
        Self {
            next_uuid: 1,
            ..Default::default()
        }
    }

    pub fn seed_record(&mut self, key: &str, value: Value) {
        self.records.insert(key.to_owned(), value);
    }

    pub fn take_logs(&mut self) -> Vec<String> {
        std::mem::take(&mut self.logs)
    }

    /// How many times `name` has been dispatched. Lets a test pin down how many
    /// host round trips a handler actually costs.
    pub fn syscall_count(&self, name: &str) -> usize {
        self.syscall_counts.get(name).copied().unwrap_or(0)
    }

    fn dispatch(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
        match name {
            "db/get" => {
                let key = string_arg(args, 0)?;
                Ok(self.records.get(key).cloned().unwrap_or(Value::Null))
            },
            "db/set" => {
                let key = string_arg(args, 0)?.to_owned();
                let value = args
                    .get(1)
                    .cloned()
                    .ok_or_else(|| "db/set needs a value".to_owned())?;
                self.records.insert(key, value);
                Ok(Value::Null)
            },
            "db/delete" => {
                let key = string_arg(args, 0)?;
                Ok(Value::Bool(self.records.remove(key).is_some()))
            },
            "time/now" => Ok(json!(FIXED_NOW_MS)),
            "crypto/randomUuid" => {
                let uuid = format!("00000000-0000-4000-8000-{:012}", self.next_uuid);
                self.next_uuid += 1;
                Ok(Value::String(uuid))
            },
            _ => Err(format!("unknown syscall {name}")),
        }
    }
}

fn string_arg(args: &[Value], index: usize) -> Result<&str, String> {
    args.get(index)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("argument {index} should be a string"))
}

impl ConvexSyscallHost for FixtureHost {
    fn syscall(&mut self, name: &str, args_json: &str) -> anyhow::Result<SyscallOutcome> {
        *self.syscall_counts.entry(name.to_owned()).or_default() += 1;

        let args: Vec<Value> = match serde_json::from_str(args_json) {
            Ok(args) => args,
            Err(error) => {
                return Ok(Err(format!(
                    "syscall arguments should be an array: {error}"
                )))
            },
        };

        Ok(self.dispatch(name, &args).map(|value| value.to_string()))
    }

    /// Fixtures park on nothing: their `db` calls are sync syscalls behind a
    /// resolved promise, so the driver never has a batch to run. A pending op
    /// here means a fixture reached for a host capability that no longer
    /// exists.
    async fn async_syscalls(
        &mut self,
        pending: &[PendingSyscall],
    ) -> anyhow::Result<Vec<(i32, SyscallOutcome)>> {
        Ok(pending
            .iter()
            .map(|call| {
                (
                    call.op_id,
                    Err(format!(
                        "the fixture host has no async syscall {}",
                        call.name
                    )),
                )
            })
            .collect())
    }

    fn trace(&mut self, _level: &str, messages: Vec<String>) -> anyhow::Result<()> {
        self.logs.push(messages.join(" "));
        Ok(())
    }
}
