/// Per the Supreme Naming Committee, our codebase uses American spelling for
/// the word "canceled", not the infinitely seductive Australian spelling
/// "cancelled". Tokio, unfortunately, chose poorly, and this trait covers up
/// their error.
pub trait IsCanceled {
    fn is_canceled(&self) -> bool;
}

impl IsCanceled for tokio::task::JoinError {
    fn is_canceled(&self) -> bool {
        // Avert your eyes!
        self.is_cancelled()
    }
}
