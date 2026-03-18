//! Build script for epbot-core.
//!
//! Locates the native EPBot library for the current platform/arch.
//! Library search order:
//! 1. EPBOT_LIB_DIR environment variable
//! 2. ../epbot-libs/{os}/{arch}/ relative to this crate

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    // Map target to library subdirectory
    let (os_dir, arch_dir) = match (target_os.as_str(), target_arch.as_str()) {
        ("macos", "aarch64") => ("macos", "arm64"),
        ("macos", "x86_64") => ("macos", "x64"),
        ("linux", "aarch64") => ("linux", "arm64"),
        ("linux", "x86_64") => ("linux", "x64"),
        ("windows", "aarch64") => ("windows", "arm64"),
        ("windows", "x86_64") => ("windows", "x64"),
        _ => {
            eprintln!(
                "WARNING: Unsupported platform {}-{} for EPBot native library",
                target_os, target_arch
            );
            return;
        }
    };

    // Search for the library
    let search_dirs = [
        std::env::var("EPBOT_LIB_DIR").ok(),
        Some(format!("../epbot-libs/{}/{}", os_dir, arch_dir)),
    ];

    let lib_name = match target_os.as_str() {
        "macos" => "libEPBot.dylib",
        "linux" => "libEPBot.so",
        "windows" => "EPBot.dll",
        _ => "libEPBot.so",
    };

    let mut found = false;
    for dir in search_dirs.iter().flatten() {
        let path = std::path::Path::new(dir);
        let lib_path = path.join(lib_name);
        if lib_path.exists() {
            let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
            println!("cargo:rustc-link-search=native={}", abs.display());
            eprintln!("Found {} in: {}", lib_name, abs.display());
            found = true;
            break;
        }
    }

    if !found {
        eprintln!(
            "WARNING: {} not found. Set EPBOT_LIB_DIR or ensure epbot-libs/{}/{} exists.",
            lib_name, os_dir, arch_dir
        );
    }

    // Link the library
    println!("cargo:rustc-link-lib=dylib=EPBot");

    // Re-run if library or env changes
    println!("cargo:rerun-if-env-changed=EPBOT_LIB_DIR");
    println!(
        "cargo:rerun-if-changed=../epbot-libs/{}/{}/{}",
        os_dir, arch_dir, lib_name
    );
}
