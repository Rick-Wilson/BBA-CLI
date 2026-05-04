// Adds an rpath load command pointing at the executable's directory so
// libEPBot.{dylib,so} is auto-discovered when dropped next to the binary.
// See bba-cli/build.rs for the rationale.

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
