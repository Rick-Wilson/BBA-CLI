//! Build script for epbot-core.
//!
//! Locates the native EPBot library for the current platform/arch.
//! On Windows, generates an import library (.lib) from the DLL.
//!
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

    let mut found_dir = None;
    for dir in search_dirs.iter().flatten() {
        let path = std::path::Path::new(dir);
        let lib_path = path.join(lib_name);
        if lib_path.exists() {
            let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
            eprintln!("Found {} in: {}", lib_name, abs.display());
            found_dir = Some(abs);
            break;
        }
    }

    if let Some(ref dir) = found_dir {
        if target_os == "windows" {
            // On Windows, the MSVC linker needs an import library (.lib).
            // Generate one from a .def file listing the exports.
            let out_dir = std::env::var("OUT_DIR").unwrap();
            let def_path = std::path::Path::new(&out_dir).join("EPBot.def");
            let lib_path = std::path::Path::new(&out_dir).join("EPBot.lib");

            // Write a .def file with all EPBot exports
            std::fs::write(&def_path, generate_def_file()).unwrap();

            // Use dlltool (from MinGW, available on GH Actions) or lib.exe
            let machine = if target_arch == "aarch64" {
                "arm64"
            } else {
                "x64"
            };

            // Try lib.exe (MSVC) first
            let status = std::process::Command::new("lib")
                .arg(format!("/DEF:{}", def_path.display()))
                .arg(format!("/OUT:{}", lib_path.display()))
                .arg(format!("/MACHINE:{}", machine))
                .arg("/NOLOGO")
                .status();

            match status {
                Ok(s) if s.success() => {
                    eprintln!("Generated EPBot.lib using lib.exe");
                }
                _ => {
                    // Fall back to dlltool
                    let dlltool_machine = if target_arch == "aarch64" {
                        "arm64"
                    } else {
                        "i386:x86-64"
                    };
                    let status = std::process::Command::new("dlltool")
                        .arg("-d")
                        .arg(&def_path)
                        .arg("-l")
                        .arg(&lib_path)
                        .arg("-m")
                        .arg(dlltool_machine)
                        .status();

                    match status {
                        Ok(s) if s.success() => {
                            eprintln!("Generated EPBot.lib using dlltool");
                        }
                        _ => {
                            eprintln!("WARNING: Could not generate EPBot.lib - link may fail");
                        }
                    }
                }
            }

            println!("cargo:rustc-link-search=native={}", out_dir);
        } else {
            println!("cargo:rustc-link-search=native={}", dir.display());
        }
    } else {
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

/// Generate a Windows .def file listing all EPBot exports.
fn generate_def_file() -> String {
    let exports = [
        "epbot_create", "epbot_destroy", "epbot_get_last_error",
        "epbot_new_hand", "epbot_get_bid", "epbot_set_bid",
        "epbot_set_arr_bids", "epbot_interpret_bid", "epbot_ask",
        "epbot_get_conventions", "epbot_set_conventions",
        "epbot_get_system_type", "epbot_set_system_type",
        "epbot_get_opponent_type", "epbot_set_opponent_type",
        "epbot_convention_index", "epbot_convention_name",
        "epbot_get_convention_name", "epbot_selected_conventions",
        "epbot_system_name",
        "epbot_get_scoring", "epbot_set_scoring",
        "epbot_get_playing_skills", "epbot_set_playing_skills",
        "epbot_get_defensive_skills", "epbot_set_defensive_skills",
        "epbot_get_licence", "epbot_set_licence",
        "epbot_get_bcalconsole_path", "epbot_set_bcalconsole_path",
        "epbot_get_position", "epbot_get_dealer", "epbot_get_vulnerability",
        "epbot_version", "epbot_copyright",
        "epbot_get_last_epbot_error", "epbot_get_str_bidding",
        "epbot_get_probable_level", "epbot_get_probable_levels",
        "epbot_get_sd_tricks",
        "epbot_get_info_meaning", "epbot_set_info_meaning",
        "epbot_get_info_meaning_extended", "epbot_set_info_meaning_extended",
        "epbot_get_info_feature", "epbot_set_info_feature",
        "epbot_get_info_min_length", "epbot_set_info_min_length",
        "epbot_get_info_max_length", "epbot_set_info_max_length",
        "epbot_get_info_probable_length", "epbot_set_info_probable_length",
        "epbot_get_info_honors", "epbot_set_info_honors",
        "epbot_get_info_suit_power", "epbot_set_info_suit_power",
        "epbot_get_info_strength", "epbot_set_info_strength",
        "epbot_get_info_stoppers", "epbot_set_info_stoppers",
        "epbot_get_info_alerting", "epbot_set_info_alerting",
        "epbot_get_used_conventions", "epbot_set_used_conventions",
        "epbot_get_lead", "epbot_set_lead",
        "epbot_set_dummy", "epbot_get_cards",
        "epbot_get_hand", "epbot_get_arr_suits",
    ];

    let mut def = String::from("LIBRARY EPBot\nEXPORTS\n");
    for export in &exports {
        def.push_str(&format!("    {}\n", export));
    }
    def
}
