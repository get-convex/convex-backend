use futures::{
    Stream,
    TryStreamExt,
};

/// Use this instead of `try_chunks` to pass through errors directly.
/// TryStreamExt::try_chunks wraps errors which can lose stacktrace and context.
pub trait TryChunksExt<T, E> {
    fn try_chunks2(self, cap: usize) -> impl Stream<Item = Result<Vec<T>, E>>;
}

impl<T, E, S: Stream<Item = Result<T, E>>> TryChunksExt<T, E> for S {
    fn try_chunks2(self, cap: usize) -> impl Stream<Item = Result<Vec<T>, E>> {
        #[allow(clippy::disallowed_methods)]
        TryStreamExt::try_chunks(self, cap).map_err(|e| e.1)
    }
}
