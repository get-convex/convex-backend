//! Test helpers for types defined in this crate
#[cfg(test)]
mod schema;
mod test_id_generator;
mod test_persistence;

use std::fmt::Display;

pub use cmd_util::env::config_test as init_test_logging;
use proptest::{
    arbitrary::{
        any,
        any_with,
        Arbitrary,
    },
    strategy::{
        Strategy,
        ValueTree,
    },
    test_runner::{
        Config,
        TestRunner,
    },
};
pub use sync_types::testing::assert_roundtrips;
pub use test_id_generator::TestIdGenerator;
pub use test_persistence::TestPersistence;

pub mod persistence_test_suite;

pub fn generate<T: Arbitrary>() -> T {
    let mut runner = TestRunner::new(Config::default());
    let tree = any::<T>()
        .new_tree(&mut runner)
        .expect("Failed to create value tree");
    tree.current()
}

pub fn generate_with<T: Arbitrary>(args: T::Parameters) -> T {
    let mut runner = TestRunner::new(Config::default());
    let tree = any_with::<T>(args)
        .new_tree(&mut runner)
        .expect("Failed to create value tree");
    tree.current()
}

pub fn assert_contains(error: &impl Display, expected: &str) {
    assert!(
        format!("{}", error).contains(expected),
        "\nExpected: {expected}\nActual: {error}"
    );
}
