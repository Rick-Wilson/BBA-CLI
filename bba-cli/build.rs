// Adds an rpath load command pointing at the executable's directory so
// libEPBot.dylib (macOS) / libEPBot.so (Linux) is auto-discovered when
// dropped next to the binary. This makes the released artifact self-
// contained — no DYLD_LIBRARY_PATH / LD_LIBRARY_PATH dance for users.
//
// Windows uses the executable's directory in its DLL search order
// automatically, so no rpath equivalent is needed there.

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    match target_os.as_str() {
        "macos" => {
            println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");
        }
        "linux" => {
            println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
        }
        _ => {}
    }
}
