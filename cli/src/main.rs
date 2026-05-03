//! BBA-CLI: Bridge Bidding Analyzer Command Line Interface
//!
//! Generates bridge auctions for deals in PBN files using the native EPBot engine.
//! Cross-platform: macOS, Linux, and Windows.

use anyhow::{Context, Result};
use clap::Parser;
use log::{debug, error, info};
use std::path::PathBuf;

mod batch;

use batch::{process_pbn_file, OutputConfig};

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

    /// Dry run - parse input but don't write output
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Force the first N bids of every auction (whitespace-separated, e.g.
    /// "1C Pass 1H Pass"). Each token must be Pass, X, XX, or {1-7}{C|D|H|S|NT}.
    /// EPBot resumes normal bidding after the prefix. Useful for "what if it had
    /// gone X" practice and for A/B testing alongside bba-server.
    #[arg(long = "auction-prefix", value_name = "BIDS")]
    auction_prefix: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let log_level = match args.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level))
        .format_timestamp_millis()
        .init();

    // Show version info
    match epbot_core::version() {
        Ok(v) => info!("BBA-CLI v{} (EPBot {})", env!("CARGO_PKG_VERSION"), v),
        Err(_) => info!("BBA-CLI v{}", env!("CARGO_PKG_VERSION")),
    }

    debug!("Input: {:?}", args.input);
    debug!("Output: {:?}", args.output);
    debug!("NS Conventions: {:?}", args.ns_conventions);
    debug!("EW Conventions: {:?}", args.ew_conventions);

    // Validate input files
    if !args.input.exists() {
        anyhow::bail!("Input file not found: {:?}", args.input);
    }
    if !args.ns_conventions.exists() {
        anyhow::bail!("NS conventions file not found: {:?}", args.ns_conventions);
    }
    if !args.ew_conventions.exists() {
        anyhow::bail!("EW conventions file not found: {:?}", args.ew_conventions);
    }

    let auction_prefix: Option<Vec<String>> = args
        .auction_prefix
        .as_deref()
        .map(|s| s.split_whitespace().map(|t| t.to_string()).collect());

    if let Some(ref bids) = auction_prefix {
        info!("Auction prefix: {} bid(s) — {}", bids.len(), bids.join(" "));
    }

    let config = OutputConfig {
        event: args.event,
        ns_system_name: args.ns_system_name,
        ew_system_name: args.ew_system_name,
        ns_conventions_path: args.ns_conventions.display().to_string(),
        ew_conventions_path: args.ew_conventions.display().to_string(),
    };

    info!("Processing {:?}...", args.input);

    let stats = process_pbn_file(
        &args.input,
        &args.output,
        &args.ns_conventions,
        &args.ew_conventions,
        args.dry_run,
        &config,
        auction_prefix.as_deref(),
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
