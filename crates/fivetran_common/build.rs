use std::{
    hash::Hasher,
    io::Result,
    path::{
        Path,
        PathBuf,
    },
};

use fxhash::FxHasher32;

// Make sure to select a rev off the `production` branch of the sdk
// https://github.com/fivetran/fivetran_sdk/tree/production
const REV: &str = "466a61bddfc0e541bfec3cb0cc6a3cf3704d64be";

const FILES: &[&str] = &[
    "common.proto",
    "connector_sdk.proto",
    "destination_sdk.proto",
];

// File hash to protect against accidental changes to the vendored files:
// Update this when updating `REV` above.
const FILE_HASH: u64 = 1411440539;

cfg_if::cfg_if! {
    if #[cfg(target_os = "macos")] {
        const PROTOC_BINARY_NAME: &str = "protoc-macos-universal";
    } else if #[cfg(all(target_os = "linux", target_arch = "aarch64"))] {
        const PROTOC_BINARY_NAME: &str = "protoc-linux-aarch64";
    } else if #[cfg(all(target_os = "linux", target_arch = "x86_64"))] {
        const PROTOC_BINARY_NAME: &str = "protoc-linux-x86_64";
    } else {
        panic!("no protoc binary available for this architecture");
    }
}

fn set_protoc_path() {
    let root = Path::new("../pb_build/protoc");
    if root.exists() {
        let include_path = std::fs::canonicalize(root.join("include"))
            .expect("Failed to canonicalize protoc include path");
        std::env::set_var("PROTOC_INCLUDE", include_path);
        let binary_path = std::fs::canonicalize(root.join(PROTOC_BINARY_NAME))
            .expect("Failed to canonicalize protoc path");
        std::env::set_var("PROTOC", binary_path);
    }
}

fn main() -> Result<()> {
    set_protoc_path();
    let protos_dir = PathBuf::from(format!("./fivetran_sdk/{REV}"));

    let mut proto_files = Vec::new();
    let mut hasher = FxHasher32::default();
    for file in FILES {
        let path = protos_dir.join(file);
        let contents = std::fs::read(&path)?;
        hasher.write(&contents);
        proto_files.push(path);
    }
    let hash = hasher.finish();
    if hash != FILE_HASH {
        panic!("Files have hash {hash}, expected {FILE_HASH}");
    }
    tonic_build::configure()
        .btree_map(["."])
        .compile_protos(&proto_files, &[protos_dir])?;

    Ok(())
}
