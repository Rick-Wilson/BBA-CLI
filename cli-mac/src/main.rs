//! BBA-CLI-Mac: Bridge Bidding Analyzer CLI for macOS
//!
//! Uses the native EPBotWrapper.dylib (NativeAOT-compiled from decompiled EPBot source)
//! instead of the Windows subprocess wrapper.

use anyhow::{Context, Result};
use clap::Parser;
use log::{debug, error, info};
use std::path::PathBuf;

mod batch;
mod engine;
mod ffi;

use batch::process_pbn_file;

/// Bridge Bidding Analyzer CLI (macOS native)
///
/// Generates bridge auctions for deals in PBN files using the native EPBot engine.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Args {
    /// Input PBN file containing deals to analyze
    #[arg(short, long, value_name = "FILE")]
    input: PathBuf,

    /// Output PBN file for results with generated auctions
    #[arg(short, long, value_name = "FILE")]
    output: PathBuf,

    /// Convention file (.bbsa) for North-South partnership
    #[arg(long = "ns-conventions", value_name = "FILE")]
    ns_conventions: PathBuf,

    /// Convention file (.bbsa) for East-West partnership
    #[arg(long = "ew-conventions", value_name = "FILE")]
    ew_conventions: PathBuf,

    /// Enable verbose logging (use -vv for debug output)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Number of worker threads for parallel processing (future feature)
    #[arg(short = 'j', long, default_value_t = 1, value_name = "N")]
    threads: usize,

    /// Dry run - parse input but don't write output
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = match args.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level))
        .format_timestamp_millis()
        .init();

    info!("BBA-CLI-Mac v{} (native EPBot engine)", env!("CARGO_PKG_VERSION"));
    debug!("Input: {:?}", args.input);
    debug!("Output: {:?}", args.output);
    debug!("NS Conventions: {:?}", args.ns_conventions);
    debug!("EW Conventions: {:?}", args.ew_conventions);

    // Validate input files
    if !args.input.exists() {
        error!("Input file not found: {:?}", args.input);
        anyhow::bail!("Input file not found: {:?}", args.input);
    }

    if !args.ns_conventions.exists() {
        error!("NS conventions file not found: {:?}", args.ns_conventions);
        anyhow::bail!("NS conventions file not found: {:?}", args.ns_conventions);
    }

    if !args.ew_conventions.exists() {
        error!("EW conventions file not found: {:?}", args.ew_conventions);
        anyhow::bail!("EW conventions file not found: {:?}", args.ew_conventions);
    }

    // Process
    info!("Processing {:?}...", args.input);

    let stats = process_pbn_file(
        &args.input,
        &args.output,
        &args.ns_conventions,
        &args.ew_conventions,
        args.threads,
        args.dry_run,
    )
    .context("Failed to process PBN file")?;

    info!(
        "Processed {} deals, generated {} auctions",
        stats.deals_processed, stats.auctions_generated
    );

    if stats.errors > 0 {
        error!("{} deals had errors", stats.errors);
    }

    if args.dry_run {
        info!("Dry run complete - no output written");
    } else {
        info!("Output written to {:?}", args.output);
    }

    Ok(())
}
