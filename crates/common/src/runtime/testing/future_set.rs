use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    mem,
    pin::Pin,
    sync::Arc,
    task::{
        Context,
        Wake,
        Waker,
    },
};

use futures::{
    Future,
    FutureExt,
};
use parking_lot::Mutex;

struct FutureSetWaker {
    id: usize,
    wake_set: Arc<Mutex<BTreeSet<usize>>>,
    parent: Waker,
}

impl Wake for FutureSetWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref()
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_set.lock().insert(self.id);
        self.parent.wake_by_ref();
    }
}

pub struct FutureSet {
    next_id: usize,
    futures: BTreeMap<usize, Pin<Box<dyn Future<Output = ()> + Send + 'static>>>,
    wake_set: Arc<Mutex<BTreeSet<usize>>>,

    waiting_on_insertion: Option<Waker>,
}

impl FutureSet {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            futures: BTreeMap::new(),
            wake_set: Arc::new(Mutex::new(BTreeSet::new())),
            waiting_on_insertion: None,
        }
    }

    pub fn insert(&mut self, f: impl Future<Output = ()> + Send + 'static) {
        if let Some(waker) = self.waiting_on_insertion.take() {
            waker.wake();
        }
        let id = self.next_id;
        self.next_id += 1;
        assert!(self.futures.insert(id, f.boxed()).is_none());
        self.wake_set.lock().insert(id);
    }

    pub fn poll_ready(&mut self, cx: &mut Context) {
        self.waiting_on_insertion = Some(cx.waker().clone());

        // Execute a fixed set of polls for fairness: If we continuously drained
        // the waker set, adversarial future implementations could continuously
        // add new readiness notifications, causing us to loop forever.
        let ready = {
            let mut wake_set = self.wake_set.lock();
            mem::take(&mut *wake_set)
        };
        for id in ready {
            let future = match self.futures.get_mut(&id) {
                Some(f) => f,
                // Spurious wakeup for a completed future, skip it.
                None => continue,
            };
            let fsw = FutureSetWaker {
                id,
                wake_set: self.wake_set.clone(),
                parent: cx.waker().clone(),
            };
            let waker = Waker::from(Arc::new(fsw));
            let mut cx = Context::from_waker(&waker);
            if future.poll_unpin(&mut cx).is_ready() {
                self.futures.remove(&id);
            }
        }
        // Schedule ourselves for a wakeup if there are ready tasks. This can happen
        // if polling one future above woke up another future we manage.
        let is_empty = {
            let wake_set = self.wake_set.lock();
            wake_set.is_empty()
        };
        // NB: `wake_by_ref` could potentially acquire the lock, so be sure to
        // have dropped it first.
        if !is_empty {
            cx.waker().wake_by_ref();
        }
    }
}
