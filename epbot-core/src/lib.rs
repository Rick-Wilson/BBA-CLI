//! EPBot Core — shared bridge bidding engine orchestration.
//!
//! Wraps Edward Piwowar's native EPBot library (C FFI) into a high-level
//! Rust API for generating bridge auctions. Used by both the CLI and web server.

pub mod ffi;

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use thiserror::Error;

// Re-export FFI constants
pub use ffi::{ERR_BUFFER_TOO_SMALL, ERR_EXCEPTION, ERR_NULL_HANDLE, OK};

/// Errors from the EPBot engine.
#[derive(Error, Debug)]
pub enum EPBotError {
    #[error("Failed to create EPBot instance")]
    CreateFailed,
    #[error("EPBot FFI error (code {code}): {message}")]
    FfiError { code: i32, message: String },
    #[error("Invalid PBN deal: {0}")]
    InvalidDeal(String),
    #[error("Convention loading error: {0}")]
    ConventionError(String),
}

/// A single bid in an auction with optional meaning.
#[derive(Debug, Clone)]
pub struct BidInfo {
    /// The bid string (e.g., "1NT", "Pass", "X")
    pub bid: String,
    /// The bid code (0=Pass, 1=X, 2=XX, 5-39=contract bids)
    pub code: i32,
    /// Position of the bidder (0=N, 1=E, 2=S, 3=W)
    pub position: i32,
    /// Short bid meaning from partner's perspective (if alertable)
    pub meaning: Option<String>,
    /// Longer/detailed bid meaning from partner's perspective (if alertable)
    pub meaning_extended: Option<String>,
    /// Whether this bid should be alerted
    pub is_alert: bool,
}

/// Result of generating an auction for a deal.
#[derive(Debug, Clone)]
pub struct AuctionResult {
    pub bids: Vec<BidInfo>,
    pub success: bool,
    pub error: Option<String>,
}

/// Scoring mode for the auction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scoring {
    Matchpoints = 0,
    Imps = 1,
}

impl Default for Scoring {
    fn default() -> Self {
        Scoring::Matchpoints
    }
}

/// Parsed convention card content (lines from a .bbsa file).
#[derive(Debug, Clone)]
pub struct ConventionCard {
    pub lines: Vec<String>,
}

impl ConventionCard {
    /// Parse a .bbsa file from its content string.
    pub fn from_content(content: &str) -> Self {
        Self {
            lines: content.lines().map(|l| l.to_string()).collect(),
        }
    }

    /// Parse a .bbsa file from a line array.
    pub fn from_lines(lines: Vec<String>) -> Self {
        Self { lines }
    }

    /// Load conventions into an EPBot instance for the given side (0=NS, 1=EW).
    /// Mirrors the C# LoadConventions logic from EPBotService.cs.
    fn apply_to(&self, instance: *mut c_void, side: i32) -> Result<(), EPBotError> {
        for line in &self.lines {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
                continue;
            }

            let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
            if parts.len() != 2 {
                continue;
            }

            let key = parts[0].trim();
            let value_str = parts[1].trim();

            // Try parsing as integer
            if let Ok(int_value) = value_str.parse::<i32>() {
                if key.eq_ignore_ascii_case("System type") {
                    let rc = unsafe { ffi::epbot_set_system_type(instance, side, int_value) };
                    if rc < 0 && rc != ffi::ERR_EXCEPTION {
                        log::warn!("set_system_type({}, {}) returned {}", side, int_value, rc);
                    }
                } else if key.eq_ignore_ascii_case("Opponent type") {
                    let rc = unsafe { ffi::epbot_set_opponent_type(instance, side, int_value) };
                    if rc < 0 && rc != ffi::ERR_EXCEPTION {
                        log::warn!("set_opponent_type({}, {}) returned {}", side, int_value, rc);
                    }
                } else if let Ok(key_c) = CString::new(key) {
                    let bool_val = if int_value == 1 { 1 } else { 0 };
                    unsafe {
                        ffi::epbot_set_conventions(instance, side, key_c.as_ptr(), bool_val);
                    }
                }
            } else if value_str.eq_ignore_ascii_case("true") {
                if let Ok(key_c) = CString::new(key) {
                    unsafe {
                        ffi::epbot_set_conventions(instance, side, key_c.as_ptr(), 1);
                    }
                }
            } else if value_str.eq_ignore_ascii_case("false") {
                if let Ok(key_c) = CString::new(key) {
                    unsafe {
                        ffi::epbot_set_conventions(instance, side, key_c.as_ptr(), 0);
                    }
                }
            }
        }
        Ok(())
    }
}

/// Parse a PBN deal string into per-player hands in EPBot's C.D.H.S order.
///
/// Input format: "N:AKQ.JT9.876.543 ... ... ..."
/// PBN suit order is S.H.D.C, but EPBot expects C.D.H.S.
///
/// Returns (first_seat, hands) where hands[position] = "clubs\ndiamonds\nhearts\nspades"
fn parse_pbn_deal(pbn: &str) -> Result<(i32, [String; 4]), EPBotError> {
    let colon_pos = pbn
        .find(':')
        .ok_or_else(|| EPBotError::InvalidDeal("Missing colon in PBN deal".into()))?;

    if colon_pos == 0 {
        return Err(EPBotError::InvalidDeal("Missing seat before colon".into()));
    }

    let first_seat_char = &pbn[colon_pos - 1..colon_pos];
    let first_seat = match first_seat_char.to_uppercase().as_str() {
        "N" => 0,
        "E" => 1,
        "S" => 2,
        "W" => 3,
        _ => return Err(EPBotError::InvalidDeal(format!("Invalid seat: {}", first_seat_char))),
    };

    let hands_str = &pbn[colon_pos + 1..];
    let hand_parts: Vec<&str> = hands_str.split_whitespace().collect();

    if hand_parts.len() != 4 {
        return Err(EPBotError::InvalidDeal(format!(
            "Expected 4 hands, got {}",
            hand_parts.len()
        )));
    }

    let mut hands: [String; 4] = Default::default();

    for (i, hand_str) in hand_parts.iter().enumerate() {
        let pos = (first_seat + i as i32) % 4;
        let suits: Vec<&str> = hand_str.split('.').collect();

        if suits.len() != 4 {
            return Err(EPBotError::InvalidDeal(format!(
                "Expected 4 suits in hand {}, got {}",
                i,
                suits.len()
            )));
        }

        // PBN is S.H.D.C, EPBot wants C.D.H.S (reversed)
        hands[pos as usize] = format!("{}\n{}\n{}\n{}", suits[3], suits[2], suits[1], suits[0]);
    }

    Ok((first_seat, hands))
}

/// Decode an EPBot bid code to a human-readable string.
pub fn decode_bid(code: i32) -> String {
    match code {
        0 => "Pass".to_string(),
        1 => "X".to_string(),
        2 => "XX".to_string(),
        c if (5..=39).contains(&c) => {
            let adjusted = c - 5;
            let level = adjusted / 5 + 1;
            let suit = adjusted % 5;
            let suit_str = match suit {
                0 => "C",
                1 => "D",
                2 => "H",
                3 => "S",
                4 => "NT",
                _ => "?",
            };
            format!("{}{}", level, suit_str)
        }
        _ => format!("?{}", code),
    }
}

/// Encode a bid string to an EPBot bid code, returning an error on invalid input.
/// Pass=0, X=1, XX=2, 1C=5, 1D=6, ..., 7NT=39 (also tolerates "1N" as "1NT").
pub fn try_encode_bid(bid: &str) -> Result<i32, String> {
    match bid {
        "Pass" | "P" | "pass" => Ok(0),
        "X" => Ok(1),
        "XX" => Ok(2),
        _ => {
            let bytes = bid.as_bytes();
            if bytes.len() >= 2 && bytes[0].is_ascii_digit() {
                let level = (bytes[0] - b'0') as i32;
                let suit = match &bid[1..] {
                    "C" => 0,
                    "D" => 1,
                    "H" => 2,
                    "S" => 3,
                    "N" | "NT" => 4,
                    other => return Err(format!("unrecognised strain '{}'", other)),
                };
                if (1..=7).contains(&level) {
                    Ok(5 + (level - 1) * 5 + suit)
                } else {
                    Err(format!("invalid bid level: {}", level))
                }
            } else {
                Err(format!("unrecognised bid '{}'", bid))
            }
        }
    }
}

/// Encode a bid string to an EPBot bid code (silently returns 0 on invalid input).
/// Prefer `try_encode_bid` when invalid input should be reported.
pub fn encode_bid(bid: &str) -> i32 {
    try_encode_bid(bid).unwrap_or(0)
}

/// Get the EPBot library version number.
pub fn version() -> Result<i32, EPBotError> {
    let inst = unsafe { ffi::epbot_create() };
    if inst.is_null() {
        return Err(EPBotError::CreateFailed);
    }
    let v = unsafe { ffi::epbot_version(inst) };
    unsafe { ffi::epbot_destroy(inst) };
    Ok(v)
}

/// Get the EPBot copyright string.
pub fn copyright() -> Result<String, EPBotError> {
    let inst = unsafe { ffi::epbot_create() };
    if inst.is_null() {
        return Err(EPBotError::CreateFailed);
    }
    let mut buf = [0 as c_char; 512];
    let rc = unsafe { ffi::epbot_copyright(inst, buf.as_mut_ptr(), buf.len() as i32) };
    unsafe { ffi::epbot_destroy(inst) };

    if rc != ffi::OK {
        return Err(EPBotError::FfiError {
            code: rc,
            message: "copyright() failed".into(),
        });
    }

    let s = unsafe { CStr::from_ptr(buf.as_ptr()) }
        .to_str()
        .unwrap_or("?")
        .to_string();
    Ok(s)
}

/// Get the last FFI error message.
fn get_last_error() -> String {
    unsafe {
        let ptr = ffi::epbot_get_last_error();
        if ptr.is_null() {
            "Unknown error".to_string()
        } else {
            CStr::from_ptr(ptr)
                .to_str()
                .unwrap_or("?")
                .to_string()
        }
    }
}

/// Generate a complete auction for a deal.
///
/// This is the main entry point. It:
/// 1. Parses the PBN deal string
/// 2. Creates 4 EPBot instances (one per player)
/// 3. Loads conventions into each instance
/// 4. Runs the bidding loop until the auction completes
/// 5. Collects bid meanings from partner perspectives
///
/// # Arguments
/// * `pbn` - PBN deal string, e.g. "N:AKQ.JT9.876.543 ..."
/// * `dealer` - Dealer position (0=N, 1=E, 2=S, 3=W)
/// * `vulnerability` - 0=None, 1=EW, 2=NS, 3=Both (EPBot convention from EPBotService.cs)
/// * `scoring` - Scoring mode
/// * `ns_card` - Convention card for NS (or None for defaults)
/// * `ew_card` - Convention card for EW (or None for defaults)
pub fn generate_auction(
    pbn: &str,
    dealer: i32,
    vulnerability: i32,
    scoring: Scoring,
    ns_card: Option<&ConventionCard>,
    ew_card: Option<&ConventionCard>,
) -> AuctionResult {
    generate_auction_with_prefix(pbn, dealer, vulnerability, scoring, ns_card, ew_card, None)
}

/// Generate an auction, optionally forcing the first N bids from `auction_prefix`.
///
/// When `auction_prefix` is provided, the bidding loop uses those bids for the
/// first N positions instead of calling `epbot_get_bid`. `epbot_set_bid` is still
/// called on every player so EPBot's internal state stays consistent. After the
/// prefix the bots resume normal bidding from the new state.
pub fn generate_auction_with_prefix(
    pbn: &str,
    dealer: i32,
    vulnerability: i32,
    scoring: Scoring,
    ns_card: Option<&ConventionCard>,
    ew_card: Option<&ConventionCard>,
    auction_prefix: Option<&[String]>,
) -> AuctionResult {
    match generate_auction_inner(pbn, dealer, vulnerability, scoring, ns_card, ew_card, auction_prefix) {
        Ok(bids) => AuctionResult {
            bids,
            success: true,
            error: None,
        },
        Err(e) => AuctionResult {
            bids: Vec::new(),
            success: false,
            error: Some(e.to_string()),
        },
    }
}

fn generate_auction_inner(
    pbn: &str,
    dealer: i32,
    vulnerability: i32,
    scoring: Scoring,
    ns_card: Option<&ConventionCard>,
    ew_card: Option<&ConventionCard>,
    auction_prefix: Option<&[String]>,
) -> Result<Vec<BidInfo>, EPBotError> {
    let (_first_seat, hands) = parse_pbn_deal(pbn)?;

    // Create 4 EPBot instances — one per player
    let mut players: [*mut c_void; 4] = [std::ptr::null_mut(); 4];
    let empty_alert = CString::new("").unwrap();

    for i in 0..4 {
        players[i] = unsafe { ffi::epbot_create() };
        if players[i].is_null() {
            // Clean up already-created instances
            for j in 0..i {
                unsafe { ffi::epbot_destroy(players[j]) };
            }
            return Err(EPBotError::CreateFailed);
        }
    }

    // Use a closure-like pattern to ensure cleanup on any error
    let result = run_auction(&players, &hands, dealer, vulnerability, scoring, ns_card, ew_card, &empty_alert, auction_prefix);

    // Always destroy all instances
    for p in &players {
        if !p.is_null() {
            unsafe { ffi::epbot_destroy(*p) };
        }
    }

    result
}

fn run_auction(
    players: &[*mut c_void; 4],
    hands: &[String; 4],
    dealer: i32,
    vulnerability: i32,
    scoring: Scoring,
    ns_card: Option<&ConventionCard>,
    ew_card: Option<&ConventionCard>,
    empty_alert: &CString,
    auction_prefix: Option<&[String]>,
) -> Result<Vec<BidInfo>, EPBotError> {
    // Initialize each player
    for i in 0..4 {
        let hand_c = CString::new(hands[i].as_str()).map_err(|e| {
            EPBotError::InvalidDeal(format!("Invalid hand string for position {}: {}", i, e))
        })?;

        let rc = unsafe {
            ffi::epbot_new_hand(
                players[i],
                i as i32,
                hand_c.as_ptr(),
                dealer,
                vulnerability,
                0, // not repeating
                0, // not playing
            )
        };

        if rc < 0 {
            return Err(EPBotError::FfiError {
                code: rc,
                message: format!("new_hand failed for position {}: {}", i, get_last_error()),
            });
        }

        // Set scoring
        unsafe { ffi::epbot_set_scoring(players[i], scoring as i32) };

        // Load conventions
        if let Some(card) = ns_card {
            card.apply_to(players[i], 0)?;
        }
        if let Some(card) = ew_card {
            card.apply_to(players[i], 1)?;
        }
    }

    // Run the auction
    let mut bids = Vec::new();
    let mut current_pos = dealer;
    let mut pass_count = 0;
    let mut has_bid = false;
    let prefix_len = auction_prefix.map(|p| p.len()).unwrap_or(0);

    for round in 0..100 {
        // Get bid: from forced prefix if we're still in it, otherwise from EPBot.
        let (bid_code, bid_str) = if round < prefix_len {
            let forced = &auction_prefix.unwrap()[round];
            let code = try_encode_bid(forced).map_err(|e| EPBotError::FfiError {
                code: 0,
                message: format!("Invalid auctionPrefix at index {}: {}", round, e),
            })?;
            (code, decode_bid(code))
        } else {
            let code = unsafe { ffi::epbot_get_bid(players[current_pos as usize]) };
            if code < 0 {
                return Err(EPBotError::FfiError {
                    code,
                    message: format!("get_bid failed for position {}: {}", current_pos, get_last_error()),
                });
            }
            (code, decode_bid(code))
        };

        // Broadcast bid to all players
        for j in 0..4 {
            let rc = unsafe {
                ffi::epbot_set_bid(
                    players[j],
                    current_pos,
                    bid_code,
                    empty_alert.as_ptr(),
                )
            };
            if rc < 0 {
                log::warn!(
                    "set_bid({}, {}, {}) failed with code {}",
                    j,
                    current_pos,
                    bid_code,
                    rc
                );
            }
        }

        // Get bid meaning from partner's perspective
        let partner_pos = (current_pos + 2) % 4;
        let mut meaning = None;
        let mut meaning_extended = None;
        let mut is_alert = false;

        let alert_rc = unsafe { ffi::epbot_get_info_alerting(players[partner_pos as usize], current_pos) };
        if alert_rc == 1 {
            is_alert = true;
            let mut buf = [0 as c_char; 1024];
            let meaning_rc = unsafe {
                ffi::epbot_get_info_meaning(
                    players[partner_pos as usize],
                    current_pos,
                    buf.as_mut_ptr(),
                    buf.len() as i32,
                )
            };
            if meaning_rc == ffi::OK {
                let s = unsafe { CStr::from_ptr(buf.as_ptr()) }
                    .to_str()
                    .unwrap_or("")
                    .to_string();
                if !s.is_empty() {
                    meaning = Some(s);
                }
            }

            // Fetch the extended meaning independently — a failure here must
            // not drop the short meaning we already have.
            let mut ext_buf = [0 as c_char; 4096];
            let ext_rc = unsafe {
                ffi::epbot_get_info_meaning_extended(
                    players[partner_pos as usize],
                    current_pos,
                    ext_buf.as_mut_ptr(),
                    ext_buf.len() as i32,
                )
            };
            if ext_rc == ffi::OK {
                let s = unsafe { CStr::from_ptr(ext_buf.as_ptr()) }
                    .to_str()
                    .unwrap_or("")
                    .to_string();
                if !s.is_empty() {
                    meaning_extended = Some(s);
                }
            }
        }

        bids.push(BidInfo {
            bid: bid_str.clone(),
            code: bid_code,
            position: current_pos,
            meaning,
            meaning_extended,
            is_alert,
        });

        // Track passes for auction termination
        if bid_str == "Pass" {
            pass_count += 1;
        } else {
            pass_count = 0;
            has_bid = true;
        }

        // Auction ends: 3 passes after a bid, or 4 initial passes
        if (has_bid && pass_count >= 3) || (!has_bid && pass_count >= 4) {
            break;
        }

        current_pos = (current_pos + 1) % 4;
    }

    Ok(bids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_bid() {
        assert_eq!(decode_bid(0), "Pass");
        assert_eq!(decode_bid(1), "X");
        assert_eq!(decode_bid(2), "XX");
        assert_eq!(decode_bid(5), "1C");
        assert_eq!(decode_bid(9), "1NT");
        assert_eq!(decode_bid(10), "2C");
        assert_eq!(decode_bid(39), "7NT");
    }

    #[test]
    fn test_encode_bid() {
        assert_eq!(encode_bid("Pass"), 0);
        assert_eq!(encode_bid("X"), 1);
        assert_eq!(encode_bid("XX"), 2);
        assert_eq!(encode_bid("1C"), 5);
        assert_eq!(encode_bid("1NT"), 9);
        assert_eq!(encode_bid("7NT"), 39);
    }

    #[test]
    fn test_parse_pbn_deal() {
        let pbn = "N:AKQ.JT9.876.543 JT9.876.543.AKQ 876.543.AKQ.JT9 543.AKQ.JT9.876";
        let (first_seat, hands) = parse_pbn_deal(pbn).unwrap();
        assert_eq!(first_seat, 0); // North

        // North hand: PBN is S.H.D.C = AKQ.JT9.876.543
        // EPBot wants C.D.H.S = 543.876.JT9.AKQ
        assert_eq!(hands[0], "543\n876\nJT9\nAKQ");
    }

    #[test]
    fn test_parse_pbn_deal_south_first() {
        let pbn = "S:AKQ.JT9.876.543 JT9.876.543.AKQ 876.543.AKQ.JT9 543.AKQ.JT9.876";
        let (first_seat, hands) = parse_pbn_deal(pbn).unwrap();
        assert_eq!(first_seat, 2); // South

        // First hand in string is South's
        assert_eq!(hands[2], "543\n876\nJT9\nAKQ");
    }

    #[test]
    fn test_convention_card_parse() {
        let content = "# Comment\nSystem type = 5\nOpponent type = 0\nSMOLEN = 1\n; another comment\nGarbage Stayman = true\nUnused = false\n";
        let card = ConventionCard::from_content(content);
        assert_eq!(card.lines.len(), 7);
    }
}
