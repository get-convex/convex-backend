/// This file includes the generated table of all of the system JS
/// files, which we mount under "_system/" in the module namespace.
mod system_udf_js_data {
    include!(concat!(env!("OUT_DIR"), "/system_udf_js_data.rs"));
}
pub use self::system_udf_js_data::FILES as SYSTEM_UDF_FILES;
#[allow(dead_code)]
static UNUSED: [u8; 32] = self::system_udf_js_data::FILES_SHA256;

/// This file includes the generated table of all of the system JS
/// files, which we mount under "_system/" in the module namespace.
mod node_executor_js_data {
    include!(concat!(env!("OUT_DIR"), "/node_executor_js_data.rs"));
}
pub use self::node_executor_js_data::{
    FILES as NODE_EXECUTOR_FILES,
    FILES_SHA256 as NODE_EXECUTOR_SHA256,
};

#[cfg(any(test, feature = "testing"))]
pub const UDF_TEST_BUNDLE_PATH: &str = concat!(env!("OUT_DIR"), "/udf_test_bundle.json");
