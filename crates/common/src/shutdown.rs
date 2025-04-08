use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::mpsc;

use crate::errors::report_error_sync;

// Used by the database to signal it has encountered a fatal error.
#[derive(Clone)]
pub struct ShutdownSignal {
    shutdown_tx: Option<Arc<Mutex<Option<mpsc::UnboundedSender<ShutdownMessage>>>>>,
    instance_name: String,
    generation_id: u64,
}

#[derive(Debug)]
pub struct ShutdownMessage {
    pub error: anyhow::Error,
    pub instance_name: String,
    pub generation_id: u64,
}

impl ShutdownSignal {
    pub fn new(
        shutdown_tx: mpsc::UnboundedSender<ShutdownMessage>,
        instance_name: String,
        generation_id: u64,
    ) -> Self {
        Self {
            shutdown_tx: Some(Arc::new(Mutex::new(Some(shutdown_tx)))),
            instance_name,
            generation_id,
        }
    }

    pub fn signal(&self, mut fatal_error: anyhow::Error) {
        report_error_sync(&mut fatal_error);
        if let Some(ref shutdown_tx_mutex) = self.shutdown_tx {
            let Some(shutdown_tx) = shutdown_tx_mutex.lock().take() else {
                // A shutdown message has already been sent for this instance. Do nothing.
                return;
            };
            _ = shutdown_tx.send(ShutdownMessage {
                error: fatal_error,
                instance_name: self.instance_name.clone(),
                generation_id: self.generation_id,
            });
        } else {
            // We don't anyone to shutdown signal configured. Just panic.
            panic!("Shutting down due to fatal error: {}", fatal_error);
        }
    }

    // Creates a new ShutdownSignal that panics when signaled.
    pub fn panic() -> Self {
        Self {
            shutdown_tx: None,
            instance_name: "".to_owned(),
            generation_id: 0,
        }
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn no_op() -> Self {
        Self {
            shutdown_tx: Some(Arc::new(Mutex::new(None))),
            instance_name: "".to_owned(),
            generation_id: 0,
        }
    }
}
