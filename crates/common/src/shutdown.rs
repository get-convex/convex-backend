use std::sync::Arc;

use crate::errors::report_error_sync;

// Used by the database to signal it has encountered a fatal error.
#[derive(Clone)]
pub struct ShutdownSignal {
    shutdown_tx: Option<async_broadcast::Sender<ShutdownMessage>>,
    instance_name: String,
    generation_id: u64,
}

#[derive(Clone, Debug)]
pub struct ShutdownMessage {
    pub error: Arc<anyhow::Error>,
    pub instance_name: String,
    pub generation_id: u64,
}

impl ShutdownSignal {
    pub fn new(
        shutdown_tx: async_broadcast::Sender<ShutdownMessage>,
        instance_name: String,
        generation_id: u64,
    ) -> Self {
        Self {
            shutdown_tx: Some(shutdown_tx),
            instance_name,
            generation_id,
        }
    }

    pub fn signal(&self, mut fatal_error: anyhow::Error) {
        report_error_sync(&mut fatal_error);
        if let Some(ref shutdown_tx) = self.shutdown_tx {
            _ = shutdown_tx.try_broadcast(ShutdownMessage {
                error: Arc::new(fatal_error),
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
        let (sender, _receiver) = async_broadcast::broadcast(1);
        Self {
            shutdown_tx: Some(sender),
            instance_name: "".to_owned(),
            generation_id: 0,
        }
    }
}
