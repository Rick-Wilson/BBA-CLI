//! Batch processor for PBN files
//!
//! Handles reading PBN files, generating auctions for each deal,
//! and writing the results to an output file.

use crate::epbot::{DealSpec, EPBot, Side, Vulnerability};
use crate::error::{BbaError, BbaResult};
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

/// A parsed PBN game with its tags
#[derive(Debug, Clone)]
struct PbnGame {
    /// All tag lines in order (including [Deal], [Dealer], [Vulnerable], etc.)
    tags: Vec<(String, String)>,
    /// Parsed deal (if Deal tag was present and valid)
    deal: Option<PbnDeal>,
    /// Comments and other non-tag lines
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

    /// Get a tag value by name
    fn get_tag(&self, name: &str) -> Option<&str> {
        self.tags
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// Set or update a tag value
    fn set_tag(&mut self, name: &str, value: &str) {
        if let Some(pos) = self
            .tags
            .iter()
            .position(|(n, _)| n.eq_ignore_ascii_case(name))
        {
            self.tags[pos].1 = value.to_string();
        } else {
            self.tags.push((name.to_string(), value.to_string()));
        }
    }

    /// Get dealer position from Dealer tag
    fn dealer(&self) -> Option<Position> {
        self.get_tag("Dealer").and_then(|s| match s.trim() {
            "N" => Some(Position::North),
            "E" => Some(Position::East),
            "S" => Some(Position::South),
            "W" => Some(Position::West),
            _ => None,
        })
    }

    /// Get vulnerability from Vulnerable tag
    fn vulnerability(&self) -> Option<Vulnerability> {
        self.get_tag("Vulnerable")
            .and_then(|s| Vulnerability::from_pbn(s))
    }
}

/// Process a PBN file, generating auctions for each deal
///
/// # Arguments
///
/// * `input_path` - Path to input PBN file
/// * `output_path` - Path to output PBN file
/// * `ns_conventions` - Path to NS convention file (.bbsa)
/// * `ew_conventions` - Path to EW convention file (.bbsa)
/// * `wrapper_path` - Path to epbot-wrapper.exe
/// * `threads` - Number of worker threads (currently unused, reserved for future)
/// * `dry_run` - If true, don't write output file
///
/// # Returns
///
/// Processing statistics including number of deals processed and errors.
pub fn process_pbn_file(
    input_path: &Path,
    output_path: &Path,
    ns_conventions: &Path,
    ew_conventions: &Path,
    wrapper_path: &Path,
    _threads: usize,
    dry_run: bool,
) -> BbaResult<ProcessingStats> {
    let mut stats = ProcessingStats::default();

    // Read and parse the input PBN file
    info!("Reading PBN file: {:?}", input_path);
    let games = read_pbn_file(input_path)?;
    info!("Found {} games in input file", games.len());

    // Create EPBot engine instance
    let wrapper_str = wrapper_path
        .to_str()
        .ok_or(BbaError::InvalidPath)?;
    let mut engine = EPBot::new(wrapper_str)?;

    // Load conventions
    engine.load_conventions(ns_conventions, Side::NorthSouth)?;
    engine.load_conventions(ew_conventions, Side::EastWest)?;

    // Collect all deals to process in batch
    let mut deal_specs: Vec<DealSpec> = Vec::new();
    let mut game_indices: Vec<usize> = Vec::new(); // Track which games have deals

    for (idx, game) in games.iter().enumerate() {
        if let Some(deal) = &game.deal {
            let dealer = game.dealer().unwrap_or(Position::North);
            let vulnerability = game.vulnerability().unwrap_or(Vulnerability::None);

            // Get PBN deal content
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
    info!("Processing {} deals in batch...", deal_specs.len());

    // Generate all auctions in one call to the wrapper
    let results = engine.generate_auctions(deal_specs)?;

    // Apply results back to games
    let mut processed_games = games;

    for (result_idx, result) in results.iter().enumerate() {
        let game_idx = game_indices[result_idx];
        let game = &mut processed_games[game_idx];

        if result.success {
            if let Some(ref bids) = result.auction {
                stats.auctions_generated += 1;

                let dealer = game.dealer().unwrap_or(Position::North);
                let auction_str = format_auction(bids, dealer);

                // Add Auction tag
                let dealer_char = match dealer {
                    Position::North => "N",
                    Position::East => "E",
                    Position::South => "S",
                    Position::West => "W",
                };
                game.set_tag("Auction", dealer_char);
                game.other_lines.push(auction_str);

                debug!("Game {}: Generated auction with {} bids", game_idx + 1, bids.len());
            }
        } else {
            stats.errors += 1;
            error!(
                "Game {}: {}",
                game_idx + 1,
                result.error.as_deref().unwrap_or("Unknown error")
            );
        }
    }

    // Write output file
    if !dry_run {
        info!("Writing output to {:?}", output_path);
        write_pbn_file(output_path, &processed_games)?;
    }

    Ok(stats)
}

/// Read and parse a PBN file into games
fn read_pbn_file(path: &Path) -> BbaResult<Vec<PbnGame>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut games = Vec::new();
    let mut current_game: Option<PbnGame> = None;

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        // Empty line may separate games
        if trimmed.is_empty() {
            if let Some(game) = current_game.take() {
                if !game.tags.is_empty() {
                    games.push(game);
                }
            }
            continue;
        }

        // Escape lines (start with %)
        if trimmed.starts_with('%') {
            if let Some(ref mut game) = current_game {
                game.other_lines.push(line.clone());
            }
            continue;
        }

        // Comments (start with ;)
        if trimmed.starts_with(';') {
            if let Some(ref mut game) = current_game {
                game.other_lines.push(line.clone());
            }
            continue;
        }

        // Tag pairs: [Name "Value"]
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if current_game.is_none() {
                current_game = Some(PbnGame::new());
            }

            if let Some((name, value)) = parse_tag_pair(trimmed) {
                if let Some(ref mut game) = current_game {
                    game.tags.push((name.clone(), value.clone()));

                    // If this is a Deal tag, also parse it
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
            // Other lines (auction data, etc.)
            if let Some(ref mut game) = current_game {
                game.other_lines.push(line.clone());
            }
        }
    }

    // Don't forget the last game
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

/// Format an auction as PBN auction content
fn format_auction(bids: &[String], _dealer: Position) -> String {
    // PBN auction format: bids listed with proper column alignment
    // Each line has up to 4 bids (one for each player)

    let mut result = String::new();
    let mut line_bids = Vec::new();

    for bid in bids {
        line_bids.push(bid.as_str());

        // After 4 bids, start a new line
        if line_bids.len() == 4 {
            result.push_str(&line_bids.join(" "));
            result.push('\n');
            line_bids.clear();
        }
    }

    // Handle remaining bids
    if !line_bids.is_empty() {
        result.push_str(&line_bids.join(" "));
        result.push('\n');
    }

    result
}

/// Write processed games to a PBN file
fn write_pbn_file(path: &Path, games: &[PbnGame]) -> BbaResult<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    for (idx, game) in games.iter().enumerate() {
        // Blank line between games (except before first)
        if idx > 0 {
            writeln!(writer)?;
        }

        // Write tags
        for (name, value) in &game.tags {
            writeln!(writer, "[{} \"{}\"]", name, value)?;
        }

        // Write other lines (auction content, comments, etc.)
        for line in &game.other_lines {
            writeln!(writer, "{}", line)?;
        }
    }

    writer.flush()?;
    Ok(())
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
    fn test_format_auction() {
        let bids = vec![
            "1H".to_string(),
            "Pass".to_string(),
            "2H".to_string(),
            "Pass".to_string(),
            "Pass".to_string(),
            "Pass".to_string(),
        ];
        let formatted = format_auction(&bids, Position::North);
        assert!(formatted.contains("1H Pass 2H Pass"));
        assert!(formatted.contains("Pass Pass"));
    }
}
