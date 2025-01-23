#[cfg(any(test, feature = "testing"))]
mod test_pause {
    use std::{
        collections::BTreeMap,
        mem,
        sync::Arc,
    };

    use parking_lot::Mutex;
    use tokio::sync::oneshot;

    use super::Fault;
    use crate::sync::{
        rendezvous,
        RendezvousReceiver,
        RendezvousSender,
    };

    #[derive(Default, Clone)]
    pub struct PauseClient {
        channels: Arc<Mutex<BTreeMap<&'static str, RendezvousReceiver<oneshot::Receiver<Fault>>>>>,
    }

    impl PauseClient {
        /// Create a new, disconnected `PauseClient`. To actually set up
        /// breakpoints, use `PauseController`'s constructor.
        pub fn new() -> Self {
            Self {
                channels: Arc::new(Mutex::new(BTreeMap::new())),
            }
        }

        /// Wait for the named breakpoint, blocking until the controller
        /// `unpause`s it.
        pub async fn wait(&self, label: &'static str) -> Fault {
            let mut rendezvous = match self.channels.lock().remove(&label) {
                Some(r) => r,
                None => {
                    tracing::debug!("Waiting on unregistered label: {label:?}");
                    return Fault::Noop;
                },
            };
            tracing::info!("PauseClient waiting on {label}");
            // Start waiting on the channel to signal to the controller that we're paused.
            let Some(rx) = rendezvous.recv().await else {
                tracing::info!("Rendezvous disconnected for {label:?}, continuing...");
                return Fault::Noop;
            };
            tracing::info!("PauseClient successfully paused {label}");
            // Wait for the controller to give us another value.
            let fault = rx.await.unwrap_or_else(|_| {
                tracing::info!("Rendezvous disconnected after pause for {label:?}, continuing...");
                Fault::Noop
            });
            tracing::info!("PauseClient successfully unpaused {label}");
            fault
        }
    }

    pub struct PauseController {
        client: PauseClient,
    }

    #[must_use]
    pub struct HoldGuard {
        label: &'static str,
        sender: RendezvousSender<oneshot::Receiver<Fault>>,
    }

    impl HoldGuard {
        /// Wait for the tested code to hit the named breakpoint, returning a
        /// `PauseGuard` if it's blocked. If the tested code has exited
        /// or manually closed the breakpoint, return `None`.
        pub async fn wait_for_blocked(mut self) -> Option<PauseGuard> {
            tracing::info!("PauseController waiting for {}", self.label);
            let (tx, rx) = oneshot::channel();
            if self.sender.send(rx).await.is_err() {
                tracing::info!("Waiter closed for {}", self.label);
                return None;
            }
            tracing::info!("PauseController paused {}", self.label);
            Some(PauseGuard {
                sender: Some(tx),
                label: self.label,
                fault: Fault::Noop,
            })
        }
    }

    pub struct PauseGuard {
        sender: Option<oneshot::Sender<Fault>>,
        label: &'static str,
        fault: Fault,
    }

    impl PauseGuard {
        pub fn inject_error(&mut self, error: anyhow::Error) {
            self.fault = Fault::Error(error);
        }

        /// Allow the tested code to resume.
        pub fn unpause(mut self) {
            tracing::info!("PauseController unpausing {}", self.label);
            let fault = mem::take(&mut self.fault);
            if let Some(sender) = self.sender.take() {
                if sender.send(fault).is_err() {
                    tracing::info!("Failed to unpause waiter");
                }
            }
        }
    }

    impl Drop for PauseGuard {
        fn drop(&mut self) {
            if let Some(sender) = self.sender.take() {
                tracing::info!("Unpausing waiter for {:?} on unclean drop", self.label);
                if sender.send(mem::take(&mut self.fault)).is_err() {
                    tracing::info!("Failed to unpause waiter");
                }
            }
        }
    }

    /// Create a `PauseController` with a list of named breakpoints in a test,
    /// and then install the returned `PauseClient` in your tested code.
    impl PauseController {
        pub fn new() -> (Self, PauseClient) {
            let client = PauseClient {
                channels: Default::default(),
            };
            let controller = Self {
                client: client.clone(),
            };
            (controller, client)
        }

        pub fn hold(&self, label: &'static str) -> HoldGuard {
            let (tx, rx) = rendezvous();
            if self.client.channels.lock().insert(label, rx).is_some() {
                panic!("Already holding {label}");
            }
            HoldGuard { label, sender: tx }
        }
    }
}
#[cfg(any(test, feature = "testing"))]
pub use self::test_pause::{
    HoldGuard,
    PauseClient,
    PauseController,
    PauseGuard,
};

#[derive(Default)]
pub enum Fault {
    #[default]
    Noop,
    Error(anyhow::Error),
}

#[cfg(not(any(test, feature = "testing")))]
mod prod_pause {
    use super::Fault;

    #[derive(Default, Clone)]
    pub struct PauseClient;

    impl PauseClient {
        pub fn new() -> Self {
            Self
        }

        pub async fn wait(&self, _label: &'static str) -> Fault {
            Fault::Noop
        }

        pub fn close(&self, _label: &'static str) {}
    }
}
#[cfg(not(any(test, feature = "testing")))]
pub use self::prod_pause::PauseClient;
