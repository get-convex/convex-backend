use std::sync::Arc;

// Used by the database to signal it has encountered a fatal error.
#[derive(Clone)]
pub struct ShutdownSignal {
    shutdown_tx: Option<async_broadcast::Sender<ShutdownMessage>>,
    instance_name: String,
}

#[derive(Clone, Debug)]
pub struct ShutdownMessage {
    pub error: Arc<anyhow::Error>,
    pub instance_name: String,
}

impl ShutdownSignal {
    pub fn new(
        shutdown_tx: async_broadcast::Sender<ShutdownMessage>,
        instance_name: String,
    ) -> Self {
        Self {
            shutdown_tx: Some(shutdown_tx),
            instance_name,
        }
    }

    pub fn signal(&self, fatal_error: anyhow::Error) {
        if let Some(ref shutdown_tx) = self.shutdown_tx {
            _ = shutdown_tx.try_broadcast(ShutdownMessage {
                error: Arc::new(fatal_error),
                instance_name: self.instance_name.clone(),
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
        }
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn no_op() -> Self {
        let (sender, _receiver) = async_broadcast::broadcast(1);
        Self {
            shutdown_tx: Some(sender),
            instance_name: "".to_owned(),
        }
    }
}
