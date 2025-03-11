pub mod split_rw_lock;
pub mod spsc;
pub mod state_channel;

use std::sync::Arc;

pub use tokio::sync::{
    broadcast,
    watch,
    Mutex,
    MutexGuard,
    Notify,
};
use tokio::sync::{
    mpsc,
    Semaphore,
};

pub struct RendezvousSender<T> {
    semaphore: Arc<Semaphore>,
    channel: mpsc::Sender<T>,
}

impl<T> RendezvousSender<T> {
    pub async fn send(&mut self, value: T) -> anyhow::Result<()> {
        // Wait for the receiver to be ready.
        let permit = self.semaphore.acquire().await?;

        // Forget the permit once we acquire it -- if the future is canceled
        // past this point, we want to wait for another spot in the semaphore.
        permit.forget();

        self.channel
            .send(value)
            .await
            .map_err(|_| anyhow::anyhow!("Failed to send value"))
    }
}

pub struct RendezvousReceiver<T> {
    semaphore: Arc<Semaphore>,
    channel: mpsc::Receiver<T>,
}

impl<T> RendezvousReceiver<T> {
    pub async fn recv(&mut self) -> Option<T> {
        self.semaphore.add_permits(1);
        self.channel.recv().await
    }

    pub fn close(mut self) {
        self.semaphore.close();
        self.channel.close();
    }
}

// Simulate a zero-capacity SPSC channel, where the sender blocks until the
// receiver is blocked on receiving from the channel.
pub fn rendezvous<T>() -> (RendezvousSender<T>, RendezvousReceiver<T>) {
    // NB: tokio::mpsc doesn't support zero-capacity channels, so simulate it
    // with a semaphore and a channel.
    let semaphore = Arc::new(Semaphore::new(0));
    let (tx, rx) = mpsc::channel(1);
    (
        RendezvousSender {
            semaphore: semaphore.clone(),
            channel: tx,
        },
        RendezvousReceiver {
            semaphore,
            channel: rx,
        },
    )
}
