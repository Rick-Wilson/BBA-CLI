//! BBA-CLI: Bridge Bidding Analyzer Command Line Interface
//!
//! Processes PBN files using the EPBot bidding engine to generate auctions.

use anyhow::{Context, Result};
use clap::Parser;
use log::{debug, error, info};
use std::path::PathBuf;

mod batch;
mod epbot;
mod error;

use batch::{process_pbn_file_with_config, OutputConfig};

/// Bridge Bidding Analyzer CLI
///
/// Generates bridge auctions for deals in PBN files using the EPBot engine.
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

    /// Path to epbot-wrapper.exe (default: look in same directory as bba.exe)
    #[arg(long = "wrapper", value_name = "FILE")]
    wrapper_path: Option<PathBuf>,

    /// Event name for PBN output
    #[arg(long, default_value = "")]
    event: String,

    /// Convention system name for N-S (for BidSystemNS tag)
    #[arg(long = "ns-system-name", default_value = "2/1GF - 2/1 Game Force")]
    ns_system_name: String,

    /// Convention system name for E-W (for BidSystemEW tag)
    #[arg(long = "ew-system-name", default_value = "2/1GF - 2/1 Game Force")]
    ew_system_name: String,

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

    // Initialize logging based on verbosity level
    let log_level = match args.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level))
        .format_timestamp_millis()
        .init();

    info!("BBA-CLI v{}", env!("CARGO_PKG_VERSION"));
    debug!("Input: {:?}", args.input);
    debug!("Output: {:?}", args.output);
    debug!("NS Conventions: {:?}", args.ns_conventions);
    debug!("EW Conventions: {:?}", args.ew_conventions);
    debug!("Threads: {}", args.threads);

    // Validate input files exist
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

    // Find wrapper executable
    let wrapper_path = match args.wrapper_path {
        Some(p) => p,
        None => {
            // Look for epbot-wrapper.exe in the same directory as bba.exe
            let exe_dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."));
            exe_dir.join("epbot-wrapper.exe")
        }
    };

    if !wrapper_path.exists() {
        error!("EPBot wrapper not found: {:?}", wrapper_path);
        anyhow::bail!(
            "EPBot wrapper not found: {:?}. Use --wrapper to specify the path.",
            wrapper_path
        );
    }

    debug!("Wrapper: {:?}", wrapper_path);

    // Build output config
    let config = OutputConfig {
        event: args.event,
        ns_system_name: args.ns_system_name,
        ew_system_name: args.ew_system_name,
        ns_conventions_path: args.ns_conventions.display().to_string(),
        ew_conventions_path: args.ew_conventions.display().to_string(),
    };

    // Process the PBN file
    info!("Processing {:?}...", args.input);

    let stats = process_pbn_file_with_config(
        &args.input,
        &args.output,
        &args.ns_conventions,
        &args.ew_conventions,
        &wrapper_path,
        args.threads,
        args.dry_run,
        &config,
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
