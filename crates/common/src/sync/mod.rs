pub mod split_rw_lock;
pub mod state_channel;

// It's safe to use these `tokio` sync primitives in our runtime-generic code
// since they don't internally depend on the `tokio` runtime. Feel free to add
// more if you need them, but generally prefer using `futures`-based primitives
// if sufficient.
pub use tokio::sync::{
    broadcast,
    // This channel is useful over `futures::channel::mpsc` since it doesn't require `&mut self` on
    // `try_send`. The `futures` implementation conforms to their `Sink` trait which unnecessarily
    // requires mutability.
    mpsc,
    watch,
    Mutex,
    MutexGuard,
    Notify,
};
