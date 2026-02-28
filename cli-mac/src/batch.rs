//! Batch processor for PBN files
//!
//! Reads PBN files, generates auctions for each deal using the native EPBot engine,
//! and writes results to an output file with rich PBN formatting matching BBA.exe output.

use crate::engine::{DealResult, DealSpec, EPBot, Side, Vulnerability};
use anyhow::{Context, Result};
use dealer_core::Position;
use dealer_pbn::{format_deal_tag, parse_deal_tag, PbnDeal};
use log::{debug, error, info, warn};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

/// Statistics from batch processing
#[derive(Debug, Default)]
pub struct ProcessingStats {
    pub deals_processed: usize,
    pub auctions_generated: usize,
    pub errors: usize,
}

/// Configuration for PBN output formatting
pub struct OutputConfig {
    pub event: String,
    pub ns_system_name: String,
    pub ew_system_name: String,
    pub ns_conventions_path: String,
    pub ew_conventions_path: String,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            event: String::new(),
            ns_system_name: "2/1GF - 2/1 Game Force".to_string(),
            ew_system_name: "2/1GF - 2/1 Game Force".to_string(),
            ns_conventions_path: String::new(),
            ew_conventions_path: String::new(),
        }
    }
}

/// A parsed PBN game with its tags
#[derive(Debug, Clone)]
struct PbnGame {
    tags: Vec<(String, String)>,
    deal: Option<PbnDeal>,
    other_lines: Vec<String>,
}

impl PbnGame {
    fn new() -> Self {
        Self {
            tags: Vec::new(),
            deal: None,
            other_lines: Vec::new(),
        }
    }

    fn get_tag(&self, name: &str) -> Option<&str> {
        self.tags
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    fn dealer(&self) -> Option<Position> {
        self.get_tag("Dealer").and_then(|s| match s.trim() {
            "N" => Some(Position::North),
            "E" => Some(Position::East),
            "S" => Some(Position::South),
            "W" => Some(Position::West),
            _ => None,
        })
    }

    fn vulnerability(&self) -> Option<Vulnerability> {
        self.get_tag("Vulnerable")
            .and_then(|s| Vulnerability::from_pbn(s))
    }
}

/// Process a PBN file, generating auctions for each deal
pub fn process_pbn_file(
    input_path: &Path,
    output_path: &Path,
    ns_conventions: &Path,
    ew_conventions: &Path,
    _threads: usize,
    dry_run: bool,
    config: &OutputConfig,
) -> Result<ProcessingStats> {
    let mut stats = ProcessingStats::default();

    // Read and parse the input PBN file
    info!("Reading PBN file: {:?}", input_path);
    let games = read_pbn_file(input_path)?;
    info!("Found {} games in input file", games.len());

    // Create native EPBot engine
    let mut engine = EPBot::new().context("Failed to create EPBot engine")?;

    if let Some(ver) = engine.version() {
        info!("EPBot version: {}", ver);
    }

    // Load conventions
    engine
        .load_conventions(ns_conventions, Side::NorthSouth)
        .context("Failed to load NS conventions")?;
    engine
        .load_conventions(ew_conventions, Side::EastWest)
        .context("Failed to load EW conventions")?;

    // Collect all deals
    let mut deal_specs: Vec<DealSpec> = Vec::new();
    let mut game_indices: Vec<usize> = Vec::new();

    for (idx, game) in games.iter().enumerate() {
        if let Some(deal) = &game.deal {
            let dealer = game.dealer().unwrap_or(Position::North);
            let vulnerability = game.vulnerability().unwrap_or(Vulnerability::None);

            let deal_str = format_deal_tag(&deal.deal, deal.first_seat);
            let deal_content = deal_str
                .strip_prefix("[Deal \"")
                .and_then(|s| s.strip_suffix("\"]"))
                .unwrap_or(&deal_str)
                .to_string();

            deal_specs.push(DealSpec {
                pbn: deal_content,
                dealer,
                vulnerability,
            });
            game_indices.push(idx);
        }
    }

    stats.deals_processed = deal_specs.len();
    info!("Processing {} deals...", deal_specs.len());

    // Generate all auctions
    let results = engine
        .generate_auctions(deal_specs)
        .context("Failed to generate auctions")?;

    // Write rich PBN output
    if !dry_run {
        info!("Writing output to {:?}", output_path);
        write_rich_pbn(output_path, &games, &results, &game_indices, config, &mut stats)?;
    } else {
        // Still count stats
        for result in &results {
            if result.success && result.auction.is_some() {
                stats.auctions_generated += 1;
            } else if !result.success {
                stats.errors += 1;
            }
        }
    }

    Ok(stats)
}

/// Write the rich PBN output file matching BBA.exe format
fn write_rich_pbn(
    path: &Path,
    games: &[PbnGame],
    results: &[DealResult],
    game_indices: &[usize],
    config: &OutputConfig,
    stats: &mut ProcessingStats,
) -> Result<()> {
    let file = File::create(path).context("Failed to create output PBN file")?;
    let mut writer = BufWriter::new(file);

    let today = chrono_date();

    // Write PBN header
    writeln!(writer, "% PBN 2.1")?;
    writeln!(writer, "% Generated by bba-cli-mac")?;
    if !config.ns_conventions_path.is_empty() {
        writeln!(writer, "% CC1 - {}", config.ns_conventions_path)?;
    }
    if !config.ew_conventions_path.is_empty() {
        writeln!(writer, "% CC2 - {}", config.ew_conventions_path)?;
    }

    // Build a map from game index -> result index
    let mut result_map: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for (result_idx, &game_idx) in game_indices.iter().enumerate() {
        result_map.insert(game_idx, result_idx);
    }

    let mut first_game = true;
    for (game_idx, game) in games.iter().enumerate() {
        if !first_game {
            writeln!(writer)?;
        }
        first_game = false;

        // Get the result for this game (if any)
        let result = result_map.get(&game_idx).map(|&ri| &results[ri]);
        let has_auction = result.map_or(false, |r| r.success && r.auction.is_some());

        if has_auction {
            stats.auctions_generated += 1;
        }
        if result.map_or(false, |r| !r.success) {
            stats.errors += 1;
            if let Some(r) = result {
                error!(
                    "Game {}: {}",
                    game_idx + 1,
                    r.error.as_deref().unwrap_or("Unknown error")
                );
            }
        }

        let dealer = game.dealer().unwrap_or(Position::North);
        let vulnerability = game.vulnerability().unwrap_or(Vulnerability::None);
        let board_num = game
            .get_tag("Board")
            .unwrap_or(&(game_idx + 1).to_string())
            .to_string();
        let deal_value = game.get_tag("Deal").unwrap_or("").to_string();

        // Write tags in BBA.exe order
        writeln!(writer, "[Event \"{}\"]", config.event)?;
        writeln!(writer, "[Site \"\"]")?;
        writeln!(writer, "[Date \"{}\"]", today)?;
        writeln!(writer, "[Board \"{}\"]", board_num)?;
        writeln!(writer, "[North \"EPBot\"]")?;
        writeln!(writer, "[East \"EPBot\"]")?;
        writeln!(writer, "[South \"EPBot\"]")?;
        writeln!(writer, "[West \"EPBot\"]")?;
        writeln!(
            writer,
            "[Dealer \"{}\"]",
            position_char(dealer)
        )?;
        writeln!(writer, "[Vulnerable \"{}\"]", vulnerability.to_pbn())?;
        writeln!(writer, "[Deal \"{}\"]", deal_value)?;

        // Write Shape/HCP/Losers from parsed deal
        if let Some(deal) = &game.deal {
            write_hand_analysis(&mut writer, &deal.deal)?;
        }

        // Derive contract/declarer from auction
        let bids = result.and_then(|r| r.auction.as_ref());
        let meanings = result.and_then(|r| r.bid_meanings.as_ref());

        if has_auction {
            if let Some(bids) = bids {
                let (contract, declarer) = derive_contract_declarer(bids, dealer);
                writeln!(writer, "[Declarer \"{}\"]", declarer)?;
                writeln!(writer, "[Contract \"{}\"]", contract)?;
            }
        }

        // Write auction with annotations
        if has_auction {
            if let Some(bids) = bids {
                writeln!(
                    writer,
                    "[Auction \"{}\"]",
                    position_char(dealer)
                )?;
                write_annotated_auction(&mut writer, bids, meanings)?;
            }
        }

        // BidSystem tags
        writeln!(
            writer,
            "[BidSystemEW \"{}\"]",
            config.ew_system_name
        )?;
        writeln!(
            writer,
            "[BidSystemNS \"{}\"]",
            config.ns_system_name
        )?;

        debug!("Game {}: written", game_idx + 1);
    }

    writer.flush()?;
    Ok(())
}

/// Write {Shape}, {HCP}, {Losers} comments from the parsed deal
fn write_hand_analysis(writer: &mut impl Write, deal: &dealer_core::Deal) -> Result<()> {
    let positions = [Position::North, Position::East, Position::South, Position::West];

    // Shape: suit lengths as digits (SHDC) for each hand in N E S W order
    let shapes: Vec<String> = positions
        .iter()
        .map(|&pos| {
            let lengths = deal.hand(pos).suit_lengths(); // [S, H, D, C]
            format!("{}{}{}{}", lengths[0], lengths[1], lengths[2], lengths[3])
        })
        .collect();
    writeln!(writer, "{{Shape {} {} {} {}}}", shapes[0], shapes[1], shapes[2], shapes[3])?;

    // HCP
    let hcps: Vec<u8> = positions.iter().map(|&pos| deal.hand(pos).hcp()).collect();
    writeln!(writer, "{{HCP {} {} {} {}}}", hcps[0], hcps[1], hcps[2], hcps[3])?;

    // Losers
    let losers: Vec<u8> = positions.iter().map(|&pos| deal.hand(pos).losers()).collect();
    writeln!(
        writer,
        "{{Losers {} {} {} {}}}",
        losers[0], losers[1], losers[2], losers[3]
    )?;

    Ok(())
}

/// Derive contract and declarer from the auction
fn derive_contract_declarer(bids: &[String], dealer: Position) -> (String, String) {
    // Find the last non-Pass, non-X, non-XX bid
    let mut last_contract_bid = None;
    let mut last_contract_idx = 0;
    let mut doubled = false;
    let mut redoubled = false;

    for (i, bid) in bids.iter().enumerate() {
        match bid.as_str() {
            "Pass" | "P" => {}
            "X" => {
                doubled = true;
                redoubled = false;
            }
            "XX" => {
                redoubled = true;
                doubled = false;
            }
            _ => {
                last_contract_bid = Some(bid.as_str());
                last_contract_idx = i;
                doubled = false;
                redoubled = false;
            }
        }
    }

    let contract_bid = match last_contract_bid {
        Some(bid) => bid,
        None => return ("Pass".to_string(), "?".to_string()),
    };

    // Contract string with doubles
    let contract = if redoubled {
        format!("{}XX", contract_bid)
    } else if doubled {
        format!("{}X", contract_bid)
    } else {
        contract_bid.to_string()
    };

    // Determine declarer: the first player on the declaring side who bid the contract strain
    let positions = [Position::North, Position::East, Position::South, Position::West];
    let declaring_pos = positions[(dealer as usize + last_contract_idx) % 4];
    let declaring_side_is_ns = matches!(declaring_pos, Position::North | Position::South);

    // Extract strain from contract bid (everything after the level digit)
    let strain = &contract_bid[1..];

    // Find the first player on the declaring side who bid this strain
    let mut declarer = declaring_pos;
    for (i, bid) in bids.iter().enumerate() {
        let bidder = positions[(dealer as usize + i) % 4];
        let bidder_is_ns = matches!(bidder, Position::North | Position::South);
        if bidder_is_ns != declaring_side_is_ns {
            continue;
        }
        if bid.len() > 1 && &bid[1..] == strain {
            declarer = bidder;
            break;
        }
    }

    (contract, position_char(declarer).to_string())
}

/// Write auction with column alignment and =N= annotations
fn write_annotated_auction(
    writer: &mut impl Write,
    bids: &[String],
    meanings: Option<&Vec<String>>,
) -> Result<()> {
    // Collect note annotations: (note_number, meaning_text)
    let mut notes: Vec<(usize, String)> = Vec::new();

    // Build formatted bid entries with annotations
    let mut entries: Vec<String> = Vec::new();
    for (i, bid) in bids.iter().enumerate() {
        let meaning = meanings
            .and_then(|m| m.get(i))
            .map(|s| s.as_str())
            .unwrap_or("");

        if !meaning.is_empty() {
            let note_num = notes.len() + 1;
            notes.push((note_num, meaning.to_string()));
            entries.push(format!("{} ={}=", bid, note_num));
        } else {
            entries.push(bid.clone());
        }
    }

    // Write auction lines, 4 entries per line, column-aligned
    // Each column is 6 chars wide; annotations that overflow get +4 padding
    for chunk in entries.chunks(4) {
        let mut line = String::new();
        for (j, entry) in chunk.iter().enumerate() {
            let is_last = j == chunk.len() - 1;
            if is_last {
                line.push_str(entry);
            } else {
                let width = if entry.len() <= 4 {
                    6 // Standard column width
                } else {
                    entry.len() + 4 // Wider for annotated bids
                };
                line.push_str(&format!("{:<width$}", entry, width = width));
            }
        }
        writeln!(writer, "{}", line.trim_end())?;
    }

    // Write Note tags
    for (num, meaning) in &notes {
        writeln!(writer, "[Note \"{}:{}\"]", num, meaning)?;
    }

    Ok(())
}

/// Read and parse a PBN file into games
fn read_pbn_file(path: &Path) -> Result<Vec<PbnGame>> {
    let file = File::open(path).context("Failed to open PBN file")?;
    let reader = BufReader::new(file);

    let mut games = Vec::new();
    let mut current_game: Option<PbnGame> = None;

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if let Some(game) = current_game.take() {
                if !game.tags.is_empty() {
                    games.push(game);
                }
            }
            continue;
        }

        if trimmed.starts_with('%') || trimmed.starts_with(';') {
            // Skip header/comment lines from input — we write our own
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if current_game.is_none() {
                current_game = Some(PbnGame::new());
            }

            if let Some((name, value)) = parse_tag_pair(trimmed) {
                if let Some(ref mut game) = current_game {
                    game.tags.push((name.clone(), value.clone()));

                    if name.eq_ignore_ascii_case("Deal") {
                        let deal_tag = format!("[Deal \"{}\"]", value);
                        match parse_deal_tag(&deal_tag) {
                            Ok(deal) => game.deal = Some(deal),
                            Err(e) => {
                                warn!("Failed to parse Deal tag: {}", e);
                            }
                        }
                    }
                }
            }
        } else {
            if let Some(ref mut game) = current_game {
                game.other_lines.push(line.clone());
            }
        }
    }

    if let Some(game) = current_game {
        if !game.tags.is_empty() {
            games.push(game);
        }
    }

    Ok(games)
}

/// Parse a tag pair: [Name "Value"]
fn parse_tag_pair(line: &str) -> Option<(String, String)> {
    let inner = line.trim_start_matches('[').trim_end_matches(']');
    let mut parts = inner.splitn(2, ' ');
    let name = parts.next()?.to_string();
    let value = parts.next()?.trim_matches('"').to_string();
    Some((name, value))
}

/// Get today's date in PBN format (YYYY.MM.DD)
fn chrono_date() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple date calculation without chrono dependency
    let days = now / 86400;
    let mut year = 1970i64;
    let mut remaining_days = days as i64;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for &dim in &days_in_months {
        if remaining_days < dim {
            break;
        }
        remaining_days -= dim;
        month += 1;
    }
    let day = remaining_days + 1;

    format!("{:04}.{:02}.{:02}", year, month, day)
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn position_char(pos: Position) -> &'static str {
    match pos {
        Position::North => "N",
        Position::East => "E",
        Position::South => "S",
        Position::West => "W",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tag_pair() {
        let (name, value) = parse_tag_pair("[Deal \"N:AKQ...\"]").unwrap();
        assert_eq!(name, "Deal");
        assert_eq!(value, "N:AKQ...");

        let (name, value) = parse_tag_pair("[Dealer \"N\"]").unwrap();
        assert_eq!(name, "Dealer");
        assert_eq!(value, "N");
    }

    #[test]
    fn test_derive_contract_declarer() {
        // Simple: 1N by South (dealer), Stayman, 3N by South
        let bids = vec![
            "1N".to_string(),
            "Pass".to_string(),
            "2C".to_string(),
            "Pass".to_string(),
            "2S".to_string(),
            "Pass".to_string(),
            "3N".to_string(),
            "Pass".to_string(),
            "Pass".to_string(),
            "Pass".to_string(),
        ];
        let (contract, declarer) = derive_contract_declarer(&bids, Position::South);
        assert_eq!(contract, "3N");
        // South opened 1N, North bid 3N — first to bid N strain on NS side is South (1N)
        assert_eq!(declarer, "S");

        // All pass
        let bids = vec![
            "Pass".to_string(),
            "Pass".to_string(),
            "Pass".to_string(),
            "Pass".to_string(),
        ];
        let (contract, _) = derive_contract_declarer(&bids, Position::North);
        assert_eq!(contract, "Pass");

        // Doubled contract
        let bids = vec![
            "1H".to_string(),
            "X".to_string(),
            "Pass".to_string(),
            "Pass".to_string(),
            "Pass".to_string(),
        ];
        let (contract, declarer) = derive_contract_declarer(&bids, Position::North);
        assert_eq!(contract, "1HX");
        assert_eq!(declarer, "N");
    }

    #[test]
    fn test_chrono_date() {
        let date = chrono_date();
        // Should be in YYYY.MM.DD format
        assert_eq!(date.len(), 10);
        assert_eq!(&date[4..5], ".");
        assert_eq!(&date[7..8], ".");
    }
}
