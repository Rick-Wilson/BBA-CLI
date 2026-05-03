//! Duplicate-bridge contract scoring.
//!
//! Computes the raw score awarded to the declaring side for a contract +
//! number-of-tricks-taken under standard ACBL/WBF duplicate scoring rules.
//! The same number is used for both Matchpoint and IMP scoring on a per-board
//! basis — those scoring modes differ only in cross-board aggregation, not
//! in this single-board figure.
//!
//! The return value is signed: positive if the contract made (declaring side
//! wins), negative if the contract went down (declaring side loses).

/// A contract strain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strain {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
    NoTrump,
}

impl Strain {
    /// Parse the suit/strain character from a contract string (e.g., '3' from "3NT", 'H' from "4H").
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'C' | 'c' => Some(Strain::Clubs),
            'D' | 'd' => Some(Strain::Diamonds),
            'H' | 'h' => Some(Strain::Hearts),
            'S' | 's' => Some(Strain::Spades),
            'N' | 'n' => Some(Strain::NoTrump),
            _ => None,
        }
    }

    fn is_minor(self) -> bool {
        matches!(self, Strain::Clubs | Strain::Diamonds)
    }
}

/// Doubling state of a contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Doubled {
    Undoubled,
    Doubled,
    Redoubled,
}

/// Whether the declaring side is vulnerable.
///
/// Convert from board vulnerability + declarer side: declaring side is
/// vulnerable iff the deal vulnerability covers their pair.
pub fn declarer_vulnerable(vul: u8, declarer: u8) -> bool {
    // Vulnerability codes match `bba_hash`: 0=None, 1=EW, 2=NS, 3=Both.
    // Declarer codes: 0=N, 1=E, 2=S, 3=W. NS pair = {0, 2}.
    let declarer_is_ns = declarer == 0 || declarer == 2;
    match vul {
        0 => false,
        1 => !declarer_is_ns,
        2 => declarer_is_ns,
        3 => true,
        _ => false,
    }
}

/// Compute the score awarded to the declaring side for a contract result.
///
/// - `level`: 1..=7
/// - `strain`: trump suit or NT
/// - `doubled`: undoubled / doubled / redoubled
/// - `tricks`: number of tricks taken by the declaring side (0..=13)
/// - `vulnerable`: whether the declaring side is vulnerable
pub fn score(
    level: u8,
    strain: Strain,
    doubled: Doubled,
    tricks: u8,
    vulnerable: bool,
) -> i32 {
    debug_assert!((1..=7).contains(&level));
    debug_assert!(tricks <= 13);

    let needed = 6 + level as i32;
    let made = tricks as i32 >= needed;

    if !made {
        return -undertrick_penalty(needed - tricks as i32, doubled, vulnerable);
    }

    let trick_pts = contract_trick_points(level, strain, doubled);
    let overtrick_pts = overtrick_points(tricks as i32 - needed, strain, doubled, vulnerable);
    let game_or_part = game_or_part_bonus(trick_pts, vulnerable);
    let slam = slam_bonus(level, vulnerable);
    let insult = match doubled {
        Doubled::Undoubled => 0,
        Doubled::Doubled => 50,
        Doubled::Redoubled => 100,
    };

    trick_pts + overtrick_pts + game_or_part + slam + insult
}

fn contract_trick_points(level: u8, strain: Strain, doubled: Doubled) -> i32 {
    let per_trick: i32 = if strain.is_minor() { 20 } else { 30 };
    let mut pts = per_trick * level as i32;
    if matches!(strain, Strain::NoTrump) {
        // NT: extra 10 for the first trick (40 total for first, 30 each after).
        pts += 10;
    }
    pts * doubling_multiplier(doubled)
}

fn overtrick_points(overtricks: i32, strain: Strain, doubled: Doubled, vul: bool) -> i32 {
    if overtricks <= 0 {
        return 0;
    }
    match doubled {
        Doubled::Undoubled => {
            let per = if strain.is_minor() { 20 } else { 30 };
            per * overtricks
        }
        Doubled::Doubled => {
            let per = if vul { 200 } else { 100 };
            per * overtricks
        }
        Doubled::Redoubled => {
            let per = if vul { 400 } else { 200 };
            per * overtricks
        }
    }
}

fn game_or_part_bonus(contract_pts: i32, vul: bool) -> i32 {
    if contract_pts >= 100 {
        if vul { 500 } else { 300 }
    } else {
        50
    }
}

fn slam_bonus(level: u8, vul: bool) -> i32 {
    match level {
        6 => if vul { 750 } else { 500 },
        7 => if vul { 1500 } else { 1000 },
        _ => 0,
    }
}

fn doubling_multiplier(doubled: Doubled) -> i32 {
    match doubled {
        Doubled::Undoubled => 1,
        Doubled::Doubled => 2,
        Doubled::Redoubled => 4,
    }
}

/// Penalty for going down, given undertricks, doubling, and vulnerability.
fn undertrick_penalty(undertricks: i32, doubled: Doubled, vul: bool) -> i32 {
    debug_assert!(undertricks > 0);
    match (doubled, vul) {
        (Doubled::Undoubled, false) => 50 * undertricks,
        (Doubled::Undoubled, true) => 100 * undertricks,
        // Doubled NV: 1st = 100, 2nd-3rd = 200 each, 4th+ = 300 each.
        (Doubled::Doubled, false) => doubled_undertrick(undertricks, false),
        // Doubled V: 1st = 200, 2nd+ = 300 each.
        (Doubled::Doubled, true) => doubled_undertrick(undertricks, true),
        (Doubled::Redoubled, vul) => doubled_undertrick(undertricks, vul) * 2,
    }
}

fn doubled_undertrick(undertricks: i32, vul: bool) -> i32 {
    let mut total = 0;
    for n in 1..=undertricks {
        total += if vul {
            if n == 1 { 200 } else { 300 }
        } else if n == 1 {
            100
        } else if n <= 3 {
            200
        } else {
            300
        };
    }
    total
}

/// Compute the score from the NS pair's perspective.
///
/// Returns positive when NS gain points (NS contract makes, or EW contract
/// goes down) and negative otherwise. This is the value emitted in the
/// `[Score "NS ±N"]` PBN tag.
pub fn score_for_ns(
    level: u8,
    strain: Strain,
    doubled: Doubled,
    declarer: u8,
    vul_code: u8,
    tricks: u8,
) -> i32 {
    let declarer_is_ns = declarer == 0 || declarer == 2;
    let vulnerable = declarer_vulnerable(vul_code, declarer);
    let raw = score(level, strain, doubled, tricks, vulnerable);
    if declarer_is_ns { raw } else { -raw }
}

/// Parse a contract string like "3NT", "4S", "5HX", "7DXX", "Pass" into its
/// component parts. Returns `None` for "Pass" or malformed input.
pub fn parse_contract(s: &str) -> Option<(u8, Strain, Doubled)> {
    let s = s.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("pass") {
        return None;
    }
    let mut chars = s.chars();
    let level_c = chars.next()?;
    let level = level_c.to_digit(10)? as u8;
    if !(1..=7).contains(&level) {
        return None;
    }

    // Strain may be one or two chars ("NT" or single suit letter, plus possible doubled suffix).
    // Easiest: pull the next chars until we hit X.
    let rest: String = chars.collect();
    let rest_upper = rest.to_uppercase();
    let (strain_str, doubled_str): (&str, &str) = if rest_upper.starts_with("NT") {
        ("NT", &rest_upper[2..])
    } else if rest_upper.starts_with('N') {
        ("NT", &rest_upper[1..])
    } else if !rest_upper.is_empty() {
        (&rest_upper[..1], &rest_upper[1..])
    } else {
        return None;
    };

    let strain = Strain::from_char(strain_str.chars().next()?)?;
    let doubled = match doubled_str {
        "" => Doubled::Undoubled,
        "X" => Doubled::Doubled,
        "XX" => Doubled::Redoubled,
        _ => return None,
    };

    Some((level, strain, doubled))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_contract_examples() {
        assert_eq!(parse_contract("3NT"), Some((3, Strain::NoTrump, Doubled::Undoubled)));
        assert_eq!(parse_contract("3N"),  Some((3, Strain::NoTrump, Doubled::Undoubled)));
        assert_eq!(parse_contract("4S"),  Some((4, Strain::Spades, Doubled::Undoubled)));
        assert_eq!(parse_contract("5HX"), Some((5, Strain::Hearts, Doubled::Doubled)));
        assert_eq!(parse_contract("7DXX"), Some((7, Strain::Diamonds, Doubled::Redoubled)));
        assert_eq!(parse_contract("Pass"), None);
        assert_eq!(parse_contract(""), None);
    }

    // Standard duplicate-bridge scoring values; checked against multiple
    // reference tables (e.g. ACBL).
    #[test]
    fn nv_partial_made_exact() {
        // 2C making 8: 40 trick + 50 part-game = 90.
        assert_eq!(score(2, Strain::Clubs, Doubled::Undoubled, 8, false), 90);
    }

    #[test]
    fn nv_3nt_making_exact() {
        // 3NT making 9: 100 trick + 300 game = 400.
        assert_eq!(score(3, Strain::NoTrump, Doubled::Undoubled, 9, false), 400);
    }

    #[test]
    fn vul_4s_making_eleven() {
        // 4S V making 11: 120 trick + 500 game + 30 overtrick = 650.
        assert_eq!(score(4, Strain::Spades, Doubled::Undoubled, 11, true), 650);
    }

    #[test]
    fn nv_4s_making_ten() {
        // 4S NV making 10: 120 + 300 = 420.
        assert_eq!(score(4, Strain::Spades, Doubled::Undoubled, 10, false), 420);
    }

    #[test]
    fn nv_4h_making_ten_v8643() {
        // From v8643 PBN, Board 2: Contract 6H by N, Vul=NS, Result=12 → Score NS 1430.
        // Declarer N → NS is declaring side, NS vul → declarer V.
        // 6H V making 12: 180 trick + 500 game + 750 slam = 1430. ✓
        assert_eq!(score(6, Strain::Hearts, Doubled::Undoubled, 12, true), 1430);
    }

    #[test]
    fn small_slam_nv_made_exact() {
        // 6S NV making 12: 180 + 300 + 500 = 980.
        assert_eq!(score(6, Strain::Spades, Doubled::Undoubled, 12, false), 980);
    }

    #[test]
    fn grand_slam_vul_made() {
        // 7NT V making 13: 220 + 500 + 1500 = 2220.
        assert_eq!(score(7, Strain::NoTrump, Doubled::Undoubled, 13, true), 2220);
    }

    #[test]
    fn nv_4s_down_one() {
        // 4S NV down 1: -50.
        assert_eq!(score(4, Strain::Spades, Doubled::Undoubled, 9, false), -50);
    }

    #[test]
    fn vul_4s_down_two() {
        // 4S V down 2: -200.
        assert_eq!(score(4, Strain::Spades, Doubled::Undoubled, 8, true), -200);
    }

    #[test]
    fn doubled_4s_made_v() {
        // 4SX V making 10: (120*2) + 500 game + 50 insult = 790.
        assert_eq!(score(4, Strain::Spades, Doubled::Doubled, 10, true), 790);
    }

    #[test]
    fn doubled_overtricks_v() {
        // 4SX V making 11: 240 trick + 500 game + 200 overtrick + 50 insult = 990.
        assert_eq!(score(4, Strain::Spades, Doubled::Doubled, 11, true), 990);
    }

    #[test]
    fn doubled_down_three_nv() {
        // Doubled NV down 3: 100 + 200 + 200 = 500.
        assert_eq!(score(4, Strain::Spades, Doubled::Doubled, 7, false), -500);
    }

    #[test]
    fn doubled_down_four_v() {
        // Doubled V down 4: 200 + 300 + 300 + 300 = 1100.
        assert_eq!(score(4, Strain::Spades, Doubled::Doubled, 6, true), -1100);
    }

    #[test]
    fn redoubled_made_v() {
        // 4SXX V making 10: 240*2 = 480 trick + 500 game + 100 insult = 1080.
        assert_eq!(score(4, Strain::Spades, Doubled::Redoubled, 10, true), 1080);
    }

    #[test]
    fn ns_perspective_signs() {
        // 4S making 10 by S (NS declaring), vul=NS → declarer V → +620 NS.
        assert_eq!(
            score_for_ns(4, Strain::Spades, Doubled::Undoubled, /*decl=*/ 2, /*vul=*/ 2, 10),
            620
        );
        // 4S making 10 by E (EW declaring), vul=NS → declarer NV → 420 EW → -420 NS.
        assert_eq!(
            score_for_ns(4, Strain::Spades, Doubled::Undoubled, /*decl=*/ 1, /*vul=*/ 2, 10),
            -420
        );
        // 4S making 10 by E, vul=Both → declarer V → 620 EW → -620 NS.
        assert_eq!(
            score_for_ns(4, Strain::Spades, Doubled::Undoubled, /*decl=*/ 1, /*vul=*/ 3, 10),
            -620
        );
    }

    #[test]
    fn declarer_vulnerable_table() {
        // (vul, declarer) → vulnerable?
        assert!(!declarer_vulnerable(0, 0)); // None, N
        assert!(declarer_vulnerable(2, 0));  // NS, N
        assert!(!declarer_vulnerable(2, 1)); // NS, E
        assert!(declarer_vulnerable(1, 1));  // EW, E
        assert!(declarer_vulnerable(3, 3));  // Both, W
    }
}
