//! Build script for bba-cli-mac
//!
//! Supports two linking modes:
//! - Dynamic (default): links EPBotWrapper.dylib at runtime
//! - Static (EPBOT_LINK_STATIC=1): links EPBotWrapper.a into the binary
//!
//! For dynamic linking, creates a symlink with the `lib` prefix that the macOS linker expects.

fn main() {
    let link_static = std::env::var("EPBOT_LINK_STATIC").unwrap_or_default() == "1";

    if link_static {
        link_static_lib();
    } else {
        link_dynamic_lib();
    }

    println!("cargo:rerun-if-env-changed=EPBOT_LINK_STATIC");
    println!("cargo:rerun-if-env-changed=EPBOT_DYLIB_DIR");
    println!("cargo:rerun-if-env-changed=EPBOT_STATIC_LIB");
    println!("cargo:rerun-if-changed=../dist/bba-cli-mac/EPBotWrapper.dylib");
}

fn link_static_lib() {
    // Search for EPBotWrapper.a
    let search_dirs = [
        std::env::var("EPBOT_STATIC_LIB").ok(),
        Some("../../bba-mac-private/epbot-aot/bin/static-publish".to_string()),
    ];

    let mut found = false;
    for dir in search_dirs.iter().flatten() {
        let path = std::path::Path::new(dir);
        let lib = path.join("EPBotWrapper.a");
        if lib.exists() {
            let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

            // Create lib-prefixed symlink for the linker
            let lib_symlink = abs.join("libEPBotWrapper.a");
            if !lib_symlink.exists() {
                let _ = std::os::unix::fs::symlink(abs.join("EPBotWrapper.a"), &lib_symlink);
            }

            println!("cargo:rustc-link-search=native={}", abs.display());
            eprintln!("Static linking EPBotWrapper.a from: {}", abs.display());
            found = true;
            break;
        }
    }

    if !found {
        panic!("EPBotWrapper.a not found. Build with: dotnet publish epbot-aot/ -r osx-arm64 -c Release -p:NativeLib=Static -o epbot-aot/bin/static-publish");
    }

    // Link the static library
    println!("cargo:rustc-link-lib=static=EPBotWrapper");

    // NativeAOT runtime support libraries
    let nativeaot_dir = format!(
        "{}/.nuget/packages/microsoft.netcore.app.runtime.nativeaot.osx-arm64/10.0.3/runtimes/osx-arm64/native",
        std::env::var("HOME").unwrap()
    );
    let nativeaot_path = std::path::Path::new(&nativeaot_dir);
    if !nativeaot_path.exists() {
        panic!("NativeAOT runtime libs not found at: {nativeaot_dir}");
    }
    println!("cargo:rustc-link-search=native={nativeaot_dir}");

    // Link the bootstrapper object file
    println!("cargo:rustc-link-arg={nativeaot_dir}/libbootstrapperdll.o");

    // NativeAOT runtime static libraries
    println!("cargo:rustc-link-lib=static=Runtime.WorkstationGC");
    println!("cargo:rustc-link-lib=static=eventpipe-disabled");
    println!("cargo:rustc-link-lib=static=standalonegc-disabled");
    println!("cargo:rustc-link-lib=static=stdc++compat");
    println!("cargo:rustc-link-lib=static=aotminipal");
    // Force-load .NET native support libraries to override dynamic references
    println!("cargo:rustc-link-arg=-force_load");
    println!("cargo:rustc-link-arg={nativeaot_dir}/libSystem.Native.a");
    println!("cargo:rustc-link-arg=-force_load");
    println!("cargo:rustc-link-arg={nativeaot_dir}/libSystem.Globalization.Native.a");
    println!("cargo:rustc-link-arg=-force_load");
    println!("cargo:rustc-link-arg={nativeaot_dir}/libSystem.Security.Cryptography.Native.Apple.a");

    // macOS system frameworks and libraries
    println!("cargo:rustc-link-lib=framework=CoreFoundation");
    println!("cargo:rustc-link-lib=framework=CryptoKit");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=Network");
    println!("cargo:rustc-link-lib=framework=Security");
    println!("cargo:rustc-link-lib=framework=GSS");
    println!("cargo:rustc-link-lib=dylib=objc");
    println!("cargo:rustc-link-lib=dylib=icucore");
    println!("cargo:rustc-link-lib=dylib=swiftCore");
    println!("cargo:rustc-link-lib=dylib=swiftFoundation");
    println!("cargo:rustc-link-lib=dylib=z");
    println!("cargo:rustc-link-lib=dylib=c++");
}

fn link_dynamic_lib() {
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

    println!("cargo:rustc-link-lib=dylib=EPBotWrapper");
}
