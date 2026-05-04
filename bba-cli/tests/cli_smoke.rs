//! Smoke test: run `bba-cli --single-dummy` against the curated fixture deals
//! and compare to `tests/fixtures/expected/deals-with-sd.pbn`.
//!
//! This catches regressions in any of the recently-added single-dummy plumbing:
//! BBA hash encoding, score derivation, SD trick lookup, or PBN writer changes.
//! It also exercises the cross-platform dynamic-loader path setup.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture_path(rel: &str) -> PathBuf {
    let mut p = manifest_dir();
    p.pop(); // bba-cli -> repo root
    p.push("tests/fixtures");
    p.push(rel);
    p
}

fn epbot_libs_dir() -> PathBuf {
    let mut p = manifest_dir();
    p.pop(); // repo root
    let triple = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "macos/arm64"
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "linux/x64"
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "linux/arm64"
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "windows/x64"
    } else {
        "unsupported"
    };
    p.push("epbot-libs");
    p.push(triple);
    p
}

/// Drop content that's deterministic on the algorithm but not on the run:
/// today's date, and the absolute paths to the convention files.
fn normalize(s: &str) -> String {
    s.lines()
        .filter(|l| {
            !l.starts_with("[Date ")
                && !l.starts_with("% CC1 ")
                && !l.starts_with("% CC2 ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Spawn bba-cli on `input` with single-dummy enabled and compare its
/// normalized output to `golden`. Panics on divergence with a diff path.
fn run_and_compare(label: &str, input: PathBuf, golden: PathBuf, ns_card: PathBuf, ew_card: PathBuf) {
    let bin = env!("CARGO_BIN_EXE_bba-cli");
    let tmp = std::env::temp_dir().join(format!("bba-cli-smoke-{label}.pbn"));
    let _ = fs::remove_file(&tmp);

    // Cross-platform dynamic-loader env. Windows finds the dll via PATH.
    let lib_var = if cfg!(target_os = "macos") {
        "DYLD_LIBRARY_PATH"
    } else if cfg!(target_os = "linux") {
        "LD_LIBRARY_PATH"
    } else {
        "PATH"
    };

    let status = Command::new(bin)
        .env(lib_var, epbot_libs_dir())
        .args([
            "--input", input.to_str().unwrap(),
            "--output", tmp.to_str().unwrap(),
            "--ns-conventions", ns_card.to_str().unwrap(),
            "--ew-conventions", ew_card.to_str().unwrap(),
            "--single-dummy",
        ])
        .status()
        .expect("failed to spawn bba-cli");
    assert!(status.success(), "bba-cli ({label}) exited with {status}");

    let actual = fs::read_to_string(&tmp).expect("read produced PBN");
    let expected = fs::read_to_string(&golden).expect("read golden PBN");

    let actual_n = normalize(&actual);
    let expected_n = normalize(&expected);

    if actual_n != expected_n {
        let diff_path = std::env::temp_dir().join(format!("bba-cli-smoke-{label}-diff.txt"));
        let _ = fs::write(
            &diff_path,
            format!("--- expected ---\n{expected_n}\n\n--- actual ---\n{actual_n}\n"),
        );
        panic!(
            "{label}: fixture output diverged from golden. See {} for full content.",
            diff_path.display()
        );
    }
}

/// Fast smoke test: 8 curated deals across all dealer/vul combos. Catches
/// any structural regression (hash, score, SD, tag emission). Runs every
/// `cargo test`.
#[test]
fn fixture_deals_with_single_dummy_match_golden() {
    run_and_compare(
        "fast",
        fixture_path("deals.pbn"),
        fixture_path("expected/deals-with-sd.pbn"),
        fixture_path("21GF-DEFAULT.bbsa"),
        fixture_path("21GF-DEFAULT.bbsa"),
    );
}

/// Slow regression test: 500-board PBN. Together with `slow_1N`, covers
/// 1000 deals. Catches subtle bidder drift (e.g., EPBot version bumps) and
/// memory issues that 8 deals can't surface. Excluded from default test
/// runs because it adds ~3s; opt in with `cargo test -- --ignored`.
#[test]
#[ignore = "slow: 500-board fixture, run with --ignored"]
fn slow_fourth_suit_forcing() {
    run_and_compare(
        "slow-fsf",
        fixture_path("slow/Fourth_Suit_Forcing.pbn"),
        fixture_path("expected/slow/Fourth_Suit_Forcing.pbn"),
        fixture_path("21GF-DEFAULT.bbsa"),
        fixture_path("21GF-DEFAULT.bbsa"),
    );
}

/// Slow regression test: 500-board 1NT-opening PBN. Different dealer (S)
/// and different bidding shape than `slow_fourth_suit_forcing`.
#[test]
#[ignore = "slow: 500-board fixture, run with --ignored"]
fn slow_one_no_trump() {
    run_and_compare(
        "slow-1n",
        fixture_path("slow/1N.pbn"),
        fixture_path("expected/slow/1N.pbn"),
        fixture_path("21GF-DEFAULT.bbsa"),
        fixture_path("21GF-DEFAULT.bbsa"),
    );
}
