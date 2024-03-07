#[cfg(any(test, feature = "testing"))]
mod test_pause {
    use std::collections::BTreeMap;

    use futures::{
        channel::mpsc,
        SinkExt,
        StreamExt,
    };

    #[derive(Default)]
    pub struct PauseClient {
        channels: BTreeMap<&'static str, mpsc::Receiver<()>>,
    }

    impl PauseClient {
        /// Create a new, disconnected `PauseClient`. To actually set up
        /// breakpoints, use `PauseController`'s constructor.
        pub fn new() -> Self {
            Self {
                channels: BTreeMap::new(),
            }
        }

        /// Wait for the named breakpoint, blocking until the controller
        /// `unpause`s it.
        pub async fn wait(&mut self, label: &'static str) {
            let rendezvous = match self.channels.get_mut(&label) {
                Some(r) => r,
                None => {
                    tracing::debug!("Waiting on unregistered label: {label:?}");
                    return;
                },
            };
            tracing::info!("Waiting on {label}");
            // Start waiting on the channel to signal to the controller that we're paused.
            if rendezvous.next().await.is_none() {
                tracing::info!("Rendezvous disconnected for {label:?}, continuing...");
                self.channels.remove(&label);
                return;
            }
            tracing::info!("PauseController successfully paused {label}");
            // Wait for the controller to give us another value.
            if rendezvous.next().await.is_none() {
                self.channels.remove(&label);
                tracing::info!("Rendezvous disconnected after pause for {label:?}, continuing...");
            }
            tracing::info!("PauseController successfully unpaused {label}");
        }

        pub fn close(&mut self, label: &'static str) {
            if let Some(mut rendezvous) = self.channels.remove(&label) {
                rendezvous.close();
            }
        }
    }

    pub struct PauseController {
        channels: BTreeMap<&'static str, mpsc::Sender<()>>,
    }

    pub struct PauseGuard<'a> {
        controller: &'a mut PauseController,
        label: &'static str,
        unpaused: bool,
    }

    impl<'a> PauseGuard<'a> {
        /// Allow the tested code to resume.
        pub fn unpause(&mut self) {
            if self.unpaused {
                return;
            }
            self.unpaused = true;
            let rendezvous = match self.controller.channels.get_mut(&self.label) {
                Some(r) => r,
                None => {
                    tracing::info!("Tried to unpause waiter who's gone away: {:?}", self.label);
                    self.controller.channels.remove(&self.label);
                    return;
                },
            };
            if let Err(e) = rendezvous.try_send(()) {
                tracing::info!("Failed to unpause waiter: {e:?}");
                self.controller.channels.remove(&self.label);
            }
        }
    }

    impl<'a> Drop for PauseGuard<'a> {
        fn drop(&mut self) {
            if !self.unpaused {
                tracing::info!("Unpausing waiter for {:?} on unclean drop", self.label);
                self.unpause();
            }
        }
    }

    /// Create a `PauseController` with a list of named breakpoints in a test,
    /// and then install the returned `PauseClient` in your tested code.
    impl PauseController {
        pub fn new(labels: impl IntoIterator<Item = &'static str>) -> (Self, PauseClient) {
            let mut controller = Self {
                channels: BTreeMap::new(),
            };
            let mut client = PauseClient {
                channels: BTreeMap::new(),
            };
            for label in labels {
                // Use a "rendezvous" channel of zero capacity to hand off control between the
                // controller and tested code. For example, the controller will block on sending
                // to the channel until the tested code is ready to receive the
                // breakpoint. Then, the controller will regain execution until
                // it hands it back to the test by unpausing it.
                let (tx, rx) = mpsc::channel(0);
                controller.channels.insert(label, tx);
                client.channels.insert(label, rx);
            }
            (controller, client)
        }

        /// Wait for the tested code to hit the named breakpoint, returning a
        /// `PauseGuard` if it's blocked. If the tested code has exited
        /// or manually closed the breakpoint, return `None`.
        pub async fn wait_for_blocked(&mut self, label: &'static str) -> Option<PauseGuard<'_>> {
            let rendezvous = match self.channels.get_mut(&label) {
                Some(r) => r,
                None => {
                    tracing::info!("Waiting on unregistered label: {label:?}");
                    return None;
                },
            };
            if rendezvous.send(()).await.is_err() {
                tracing::info!("Waiter closed for {label:?}");
                self.channels.remove(&label);
                return None;
            }
            Some(PauseGuard {
                controller: self,
                label,
                unpaused: false,
            })
        }
    }
}
#[cfg(any(test, feature = "testing"))]
pub use self::test_pause::{
    PauseClient,
    PauseController,
};

#[cfg(not(any(test, feature = "testing")))]
mod prod_pause {
    #[derive(Default)]
    pub struct PauseClient;

    impl PauseClient {
        pub fn new() -> Self {
            Self
        }

        pub async fn wait(&mut self, _label: &'static str) {}

        pub fn close(&mut self, _label: &'static str) {}
    }
}
#[cfg(not(any(test, feature = "testing")))]
pub use self::prod_pause::PauseClient;
