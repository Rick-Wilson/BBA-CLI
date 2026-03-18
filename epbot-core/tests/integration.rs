//! Integration tests for epbot-core.
//! Run with: DYLD_LIBRARY_PATH=../epbot-libs/macos/arm64 cargo test

use epbot_core::*;

#[test]
fn test_version() {
    let v = version().expect("Failed to get version");
    assert!(v > 0, "Version should be positive, got {}", v);
    println!("EPBot version: {}", v);
}

#[test]
fn test_copyright() {
    let c = copyright().expect("Failed to get copyright");
    assert!(!c.is_empty(), "Copyright should not be empty");
    println!("Copyright: {}", c);
}

#[test]
fn test_generate_auction_no_conventions() {
    // A simple deal — generate auction with default conventions
    let pbn = "N:A653.Q97.K64.954 KQ4.AT8432.A72.A JT987.65.QT85.K3 2.KJ.J93.QJT8762";
    let result = generate_auction(pbn, 0, 0, Scoring::Matchpoints, None, None);

    println!("Success: {}", result.success);
    if let Some(ref err) = result.error {
        println!("Error: {}", err);
    }
    for bid in &result.bids {
        let meaning = bid
            .meaning
            .as_deref()
            .map(|m| format!(" ({})", m))
            .unwrap_or_default();
        let alert = if bid.is_alert { " !" } else { "" };
        println!(
            "  {} bids {}{}{} [code={}]",
            ["N", "E", "S", "W"][bid.position as usize],
            bid.bid,
            alert,
            meaning,
            bid.code
        );
    }

    assert!(result.success, "Auction should succeed");
    assert!(!result.bids.is_empty(), "Should have at least one bid");

    // Auction should end (last 3 or 4 bids are Pass)
    let last_bids: Vec<&str> = result.bids.iter().rev().take(3).map(|b| b.bid.as_str()).collect();
    let all_pass = last_bids.iter().all(|b| *b == "Pass");
    assert!(all_pass, "Auction should end with passes: {:?}", last_bids);
}

#[test]
fn test_generate_auction_with_conventions() {
    let pbn = "N:A653.Q97.K64.954 KQ4.AT8432.A72.A JT987.65.QT85.K3 2.KJ.J93.QJT8762";
    let conv_content = std::fs::read_to_string("../conventions/21GF.bbsa")
        .expect("Failed to read convention file");
    let card = ConventionCard::from_content(&conv_content);

    let result = generate_auction(
        pbn,
        0, // N deals
        0, // None vul
        Scoring::Matchpoints,
        Some(&card),
        Some(&card),
    );

    println!("Auction with 21GF conventions:");
    for bid in &result.bids {
        let meaning = bid.meaning.as_deref().unwrap_or("");
        println!(
            "  {} bids {} {}",
            ["N", "E", "S", "W"][bid.position as usize],
            bid.bid,
            meaning,
        );
    }

    assert!(result.success, "Auction with conventions should succeed: {:?}", result.error);
}
