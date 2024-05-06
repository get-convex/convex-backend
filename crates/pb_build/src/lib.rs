use std::{
    ffi::OsStr,
    fs,
    io::Result,
    path::{
        Path,
        PathBuf,
    },
};

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
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("protoc");
    let include_path = std::fs::canonicalize(root.join("include"))
        .expect("Failed to canonicalize protoc include path");
    std::env::set_var("PROTOC_INCLUDE", include_path);
    let binary_path = std::fs::canonicalize(root.join(PROTOC_BINARY_NAME))
        .expect("Failed to canonicalize protoc path");
    std::env::set_var("PROTOC", binary_path);
}

pub fn pb_build() -> Result<()> {
    set_protoc_path();
    println!("cargo:rerun-if-changed=protos");
    let mut paths = vec![];
    let mut packages = vec![];
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    for dent in std::fs::read_dir("protos")? {
        let dent = dent?;
        let path = dent.path();
        if path.extension() == Some(OsStr::new("proto")) {
            let package_name = path.file_stem().unwrap().to_str().unwrap().to_owned();
            packages.push(package_name);
            paths.push(format!(
                "protos/{}",
                path.file_name().unwrap().to_str().unwrap()
            ));
        }
    }
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("descriptors.bin"))
        .compile(&paths, &["protos/"])?;

    // We sort the package names just so we're generating the lib.rs
    // deterministically to avoid NOOP commits.
    packages.sort();

    let mut mods = vec![];
    for m in std::fs::read_dir("src")? {
        let m = m?;
        let path = m.path();
        if path.extension() == Some(OsStr::new("rs")) && path.file_stem() != Some(OsStr::new("lib"))
        {
            let mod_name = path.file_stem().unwrap().to_str().unwrap().to_owned();
            mods.push(mod_name);
        }
    }
    mods.sort();

    // Now let's build the lib.rs file.
    let mut lib_file_contents = String::new();
    lib_file_contents.push_str("// @generated - do not modify. Modify build.rs instead.\n");
    lib_file_contents.push_str("#![allow(clippy::match_single_binding)]\n");
    for m in mods {
        lib_file_contents.push_str(&format!("pub mod {m};\n"));
    }
    for package_name in packages {
        lib_file_contents.push_str(&format!(
            "pub mod {package_name} {{\n    include!(concat!(env!(\"OUT_DIR\"), \
             \"/{package_name}.rs\"));\n}}\n",
        ));
    }

    lib_file_contents.push_str(&format!(
        "\npub const FILE_DESCRIPTOR_BYTES: &[u8] =\n    \
         include_bytes!(concat!(env!(\"OUT_DIR\"), \"/descriptors.bin\"));\n"
    ));

    let out_file = Path::new("src/lib.rs");
    if fs::read_to_string(out_file)? != lib_file_contents {
        fs::write(out_file, lib_file_contents)?;
    }

    Ok(())
}
