#[macro_export]
macro_rules! try_anyhow {
    ($block:block) => {
        try bikeshed ::anyhow::Result<_> $block
    };
}
