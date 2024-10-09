pub mod split_rw_lock;
pub mod state_channel;

use futures::future;
use tokio::sync::oneshot;
pub use tokio::sync::{
    broadcast,
    mpsc,
    watch,
    Mutex,
    MutexGuard,
    Notify,
};

/// Wait until a sender's corresponding receiver has been closed.
pub async fn oneshot_receiver_closed<T>(sender: &mut oneshot::Sender<T>) {
    future::poll_fn(|cx| sender.poll_closed(cx)).await
}
