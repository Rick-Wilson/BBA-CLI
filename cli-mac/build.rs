//! Build script for bba-cli-mac
//!
//! Tells Cargo where to find EPBotWrapper.dylib for linking.
//! Creates a symlink with the `lib` prefix that the macOS linker expects.

fn main() {
    // Check for the dylib in several locations, in priority order:
    // 1. EPBOT_DYLIB_DIR environment variable
    // 2. ../dist/bba-cli-mac/ (distribution directory)
    // 3. ../../bba-mac-private/epbot-aot/bin/Release/net10.0/osx-arm64/publish/ (dev build)

    let search_dirs = [
        std::env::var("EPBOT_DYLIB_DIR").ok(),
        Some("../dist/bba-cli-mac".to_string()),
        Some("../../bba-mac-private/epbot-aot/bin/Release/net10.0/osx-arm64/publish".to_string()),
    ];

    let mut found = false;
    for dir in search_dirs.iter().flatten() {
        let path = std::path::Path::new(dir);
        let dylib = path.join("EPBotWrapper.dylib");
        if dylib.exists() {
            let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

            // macOS linker expects lib prefix: libEPBotWrapper.dylib
            // Create symlink if needed
            let lib_symlink = abs.join("libEPBotWrapper.dylib");
            if !lib_symlink.exists() {
                let _ = std::os::unix::fs::symlink(
                    abs.join("EPBotWrapper.dylib"),
                    &lib_symlink,
                );
            }

            println!("cargo:rustc-link-search=native={}", abs.display());
            eprintln!("Found EPBotWrapper.dylib in: {}", abs.display());
            found = true;
            break;
        }
    }

    if !found {
        eprintln!("WARNING: EPBotWrapper.dylib not found in any search path");
        eprintln!("  Set EPBOT_DYLIB_DIR or build the dylib first:");
        eprintln!("  cd ../../bba-mac-private && ./build.sh");
    }

    // Link against the dylib
    println!("cargo:rustc-link-lib=dylib=EPBotWrapper");

    // Re-run if the dylib changes
    println!("cargo:rerun-if-env-changed=EPBOT_DYLIB_DIR");
    println!("cargo:rerun-if-changed=../dist/bba-cli-mac/EPBotWrapper.dylib");
}
