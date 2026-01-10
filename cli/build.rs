//! Build script for bba-cli
//!
//! This script tells Cargo where to find the EPBotWrapper DLL for linking.

fn main() {
    // Tell Cargo where to find the wrapper DLL
    // During development, it's in the wrapper build directory
    // For release, it should be alongside the executable

    // Check for local build first
    let wrapper_dir = std::path::Path::new("../wrapper/build/bin");
    if wrapper_dir.exists() {
        println!(
            "cargo:rustc-link-search=native={}",
            wrapper_dir.display()
        );
    }

    // Also check Release subdirectory (Visual Studio default)
    let wrapper_dir_release = std::path::Path::new("../wrapper/build/Release");
    if wrapper_dir_release.exists() {
        println!(
            "cargo:rustc-link-search=native={}",
            wrapper_dir_release.display()
        );
    }

    // Link against the wrapper DLL
    println!("cargo:rustc-link-lib=dylib=EPBotWrapper");

    // Re-run if the header changes
    println!("cargo:rerun-if-changed=../wrapper/include/epbot_ffi.h");

    // Re-run if the wrapper DLL changes
    println!("cargo:rerun-if-changed=../wrapper/build/bin/EPBotWrapper.dll");
    println!("cargo:rerun-if-changed=../wrapper/build/Release/EPBotWrapper.dll");
}
