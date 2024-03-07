use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    sync::Arc,
};

use common::runtime::UnixTimestamp;
use parking_lot::Mutex;

use crate::environment::{
    action::task::{
        TaskId,
        TaskRequest,
        TaskRequestEnum,
    },
    AsyncOpRequest,
};

/// Enforce ordering on sleep resolution.
/// The setTimeout spec has the requirement that if you do
/// `setTimeout(f, 100); setTimeout(g, 100);` then `f` will execute before `g`.
/// Proof of enforcement:
/// Already ordered: task request and response channels, rt.monotonic_now(),
/// and v8 microtask queue.
/// We add an ordering constraint on FuturesUnordered by calling
/// `push_running_task` before starting the future, waiting while
/// `sleep_is_blocked`, and calling `pop_running_task` after the future is done.
#[derive(Clone, Default)]
pub struct TaskOrder {
    inner: Arc<Mutex<TaskOrderInner>>,
}

#[derive(Default)]
struct TaskOrderInner {
    tasks: BTreeMap<TaskId, UnixTimestamp>,
    running_sleeps: BTreeSet<UnixTimestamp>,
}

impl TaskOrder {
    pub fn push_running_task(&self, task_request: &TaskRequest) {
        let mut inner = self.inner.lock();
        if let TaskRequestEnum::AsyncOp(AsyncOpRequest::Sleep { until, .. }) = &task_request.variant
        {
            inner.tasks.insert(task_request.task_id, *until);
            inner.running_sleeps.insert(*until);
        }
    }

    // A sleep is blocked until it has the minimum `until` time of all currently
    // running sleeps.
    pub fn sleep_is_blocked(&self, until: &UnixTimestamp) -> bool {
        if let Some(min_until) = self.inner.lock().running_sleeps.first() {
            min_until < until
        } else {
            false
        }
    }

    pub fn pop_running_task(&self, task_id: TaskId) {
        let mut inner = self.inner.lock();
        if let Some(until) = inner.tasks.remove(&task_id) {
            inner.running_sleeps.remove(&until);
        }
    }

    /// Count tasks that are actively running.
    /// This count determines if we have hit the parallelism limit.
    /// This does not include sleeps because they do not consume resources to
    /// run in parallel. See CX-4968.
    pub fn active_task_count(&self) -> usize {
        let inner = self.inner.lock();
        inner.tasks.len() - inner.running_sleeps.len()
    }
}
