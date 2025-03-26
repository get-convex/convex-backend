use std::sync::Arc;

use parking_lot::{
    RwLock,
    RwLockReadGuard,
    RwLockWriteGuard,
};

/// Create a "split" lock with a reader half and a writer half. The reader
/// implements `Clone`, but the writer is unique.
pub fn new_split_rw_lock<T>(value: T) -> (Reader<T>, Writer<T>) {
    let inner = Arc::new(RwLock::new(value));
    (
        Reader {
            inner: inner.clone(),
        },
        Writer { inner },
    )
}

pub struct Reader<T> {
    pub(crate) inner: Arc<RwLock<T>>,
}

impl<T> Clone for Reader<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Reader<T> {
    pub fn lock(&self) -> RwLockReadGuard<T> {
        self.inner.read()
    }
}

pub struct Writer<T> {
    pub(crate) inner: Arc<RwLock<T>>,
}

impl<T> Writer<T> {
    pub fn reader(&self) -> Reader<T> {
        Reader {
            inner: self.inner.clone(),
        }
    }

    pub fn write(&mut self) -> RwLockWriteGuard<T> {
        self.inner.write()
    }

    pub fn read(&self) -> RwLockReadGuard<T> {
        self.inner.read()
    }
}
