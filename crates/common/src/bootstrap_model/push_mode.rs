/// Whether a push is a dry run or a normal live push.
///
/// Dry-run pushes commit schema and index changes so that `wait_for_schema`
/// can validate existing data, but `finish_push` rolls them back instead of
/// activating them. Normal pushes activate the changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushMode {
    Normal,
    DryRun,
}

impl PushMode {
    pub fn is_dry_run(self) -> bool {
        self == PushMode::DryRun
    }
}
