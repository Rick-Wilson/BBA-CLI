//! BBA-style 28-character hexadecimal board fingerprint.
//!
//! Reproduces the deal-encoding algorithm Edward Piwowar publishes on the BBA
//! site (`Sample code - coding and decoding BBA hand number`, in VB.NET). The
//! result is a deterministic 28-hex-char string that round-trips: encode →
//! decode recovers the four hands plus the dealer/vulnerability tuple.
//!
//! Layout (28 hex chars):
//! - char 0: `board_extension` nibble (0..F). BBA stores its session counter
//!   here; for our use we derive it from the PBN board number so consecutive
//!   boards within a file get distinct hashes.
//! - char 1: `dealer * 4 + vulnerability` (each in 0..3).
//! - chars 2..28: 13 ranks (A,K,Q,J,T,9..2), 2 hex chars per rank. Each byte
//!   packs which player holds that rank in each suit (bits 0-1=clubs,
//!   2-3=diamonds, 4-5=hearts, 6-7=spades, with players 0=N,1=E,2=S,3=W),
//!   XOR'd with `BOARD_NUMBER[dealer][vul]` (1..16).
//!
//! Vulnerability codes match BBA's wire format (which differs from PBN string
//! conventions): 0=None, 1=EW, 2=NS, 3=Both. Dealer codes: 0=N, 1=E, 2=S, 3=W.

const RANKS: &[u8; 13] = b"AKQJT98765432";

/// Standard duplicate-bridge board number for each (dealer, vulnerability)
/// pair, as used by BBA. Indexed `BOARD_NUMBER[dealer][vul]` returning 1..16.
/// This is the "encryption byte" XOR'd into every rank byte.
const BOARD_NUMBER: [[u8; 4]; 4] = [
    // vul order: None=0, EW=1, NS=2, Both=3
    [1, 9, 5, 13],  // dealer = N
    [14, 6, 2, 10], // dealer = E
    [11, 3, 15, 7], // dealer = S
    [8, 16, 12, 4], // dealer = W
];

/// Encoded view of one hand: 4 suit strings using rank chars from `RANKS`,
/// in suit order C, D, H, S.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct HandSuits {
    pub clubs: String,
    pub diamonds: String,
    pub hearts: String,
    pub spades: String,
}

impl HandSuits {
    fn suit(&self, suit: usize) -> &str {
        match suit {
            0 => &self.clubs,
            1 => &self.diamonds,
            2 => &self.hearts,
            3 => &self.spades,
            _ => "",
        }
    }

    fn suit_mut(&mut self, suit: usize) -> &mut String {
        match suit {
            0 => &mut self.clubs,
            1 => &mut self.diamonds,
            2 => &mut self.hearts,
            3 => &mut self.spades,
            _ => unreachable!(),
        }
    }

    fn contains(&self, suit: usize, rank: u8) -> bool {
        self.suit(suit).as_bytes().contains(&rank)
    }
}

/// Encoded BBA hash plus the inputs needed to reconstruct it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedHash {
    pub board_extension: u8,
    pub dealer: u8,
    pub vulnerability: u8,
    pub hands: [HandSuits; 4],
}

/// Encode a BBA-style 28-hex board hash.
///
/// `hands[0..4]` are indexed N=0, E=1, S=2, W=3. Suits within each hand are
/// in C, D, H, S order, using the rank characters from `RANKS`.
pub fn encode(
    hands: &[HandSuits; 4],
    dealer: u8,
    vulnerability: u8,
    board_extension: u8,
) -> String {
    debug_assert!(dealer < 4 && vulnerability < 4 && board_extension < 16);

    let encryption_byte = BOARD_NUMBER[dealer as usize][vulnerability as usize];
    let mut out = String::with_capacity(28);

    out.push(hex_nibble(board_extension));
    out.push(hex_nibble(dealer * 4 + vulnerability));

    for &rank in RANKS {
        let mut byte: u8 = 0;
        for suit in 0..4 {
            for player in 0..4 {
                if hands[player].contains(suit, rank) {
                    byte += (player as u8) * (1 << (2 * suit));
                }
            }
        }
        byte ^= encryption_byte;
        out.push(hex_nibble(byte >> 4));
        out.push(hex_nibble(byte & 0xF));
    }

    out
}

/// Decode a 28-hex BBA board hash. Returns `None` if the input is malformed.
///
/// Provided primarily for round-trip testing of the encoder.
pub fn decode(hash: &str) -> Option<DecodedHash> {
    if hash.len() != 28 {
        return None;
    }
    let bytes = hash.as_bytes();

    let board_extension = parse_nibble(bytes[0])?;
    let header_byte = parse_nibble(bytes[1])?;
    let dealer = header_byte / 4;
    let vulnerability = header_byte % 4;

    let encryption_byte = BOARD_NUMBER[dealer as usize][vulnerability as usize];

    let mut hands: [HandSuits; 4] = Default::default();

    for (rank_idx, &rank) in RANKS.iter().enumerate() {
        let off = 2 + 2 * rank_idx;
        let hi = parse_nibble(bytes[off])?;
        let lo = parse_nibble(bytes[off + 1])?;
        let mut packed = (hi << 4) | lo;
        packed ^= encryption_byte;

        for suit in 0..4 {
            let player = ((packed >> (2 * suit)) & 0b11) as usize;
            hands[player].suit_mut(suit).push(rank as char);
        }
    }

    Some(DecodedHash {
        board_extension,
        dealer,
        vulnerability,
        hands,
    })
}

#[inline]
fn hex_nibble(n: u8) -> char {
    let n = n & 0xF;
    if n < 10 {
        (b'0' + n) as char
    } else {
        (b'A' + n - 10) as char
    }
}

#[inline]
fn parse_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'F' => Some(b - b'A' + 10),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

/// Convenience: compute board_extension from a PBN `[Board "N"]` value, per
/// our chosen convention (see module docs).
pub fn board_extension_for(board_number: u32) -> u8 {
    (((board_number.saturating_sub(1)) / 16) % 16) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Edward's published example deal (from his VB sample), deal=234.
    /// `(deal-1) mod 16 = 9`, so `dealers[9]=E`, `vulnerability[9]=Both`.
    /// `board_extension = (deal-1)/16 mod 16 = 14 = 0xE`.
    /// `BOARD_NUMBER[E][Both] = 10`, so encryption byte = 10.
    fn edward_example() -> [HandSuits; 4] {
        [
            HandSuits { clubs: "987".into(),  diamonds: "KQJT7".into(), hearts: "A74".into(),  spades: "A6".into() },
            HandSuits { clubs: "T652".into(), diamonds: "3".into(),     hearts: "QJ82".into(), spades: "J732".into() },
            HandSuits { clubs: "J3".into(),   diamonds: "A9842".into(), hearts: "K3".into(),   spades: "T984".into() },
            HandSuits { clubs: "AKQ4".into(), diamonds: "65".into(),    hearts: "T965".into(), spades: "KQ5".into() },
        ]
    }

    #[test]
    fn edward_example_round_trips() {
        // Edward's sample: deal=234 → board_extension=14, dealer=E(1), vul=Both(3).
        // Header = Hex(14) + Hex(1*4+3) = "E" + "7" = "E7".
        let hands = edward_example();
        let hash = encode(&hands, /*dealer=*/ 1, /*vul=*/ 3, /*board_ext=*/ 14);
        assert_eq!(hash.len(), 28);
        assert_eq!(&hash[..2], "E7", "header should be E (board_ext) + 7 (dealer=1*4+vul=3)");

        let decoded = decode(&hash).expect("decode");
        assert_eq!(decoded.board_extension, 14);
        assert_eq!(decoded.dealer, 1);
        assert_eq!(decoded.vulnerability, 3);
        for player in 0..4 {
            for suit in 0..4 {
                let original: String = {
                    let mut chars: Vec<char> = hands[player].suit(suit).chars().collect();
                    chars.sort_by_key(|&c| RANKS.iter().position(|&r| r as char == c).unwrap_or(99));
                    chars.into_iter().collect()
                };
                let decoded_sorted: String = {
                    let mut chars: Vec<char> = decoded.hands[player].suit(suit).chars().collect();
                    chars.sort_by_key(|&c| RANKS.iter().position(|&r| r as char == c).unwrap_or(99));
                    chars.into_iter().collect()
                };
                assert_eq!(original, decoded_sorted,
                    "round-trip mismatch at player={} suit={}", player, suit);
            }
        }
    }

    #[test]
    fn matches_v8643_board1_known_hash() {
        // Board 1 from BBA.exe v8643's Fourth_Suit_Forcing.pbn (committed at
        // commit 615761034 in Practice-Bidding-Scenarios). This is a known
        // good (deal, hash) pair from a real BBA output, so a successful
        // match here proves the algorithm exactly reproduces BBA's output.
        let hands: [HandSuits; 4] = [
            HandSuits { clubs: "J".into(),     diamonds: "AQT832".into(), hearts: "A54".into(),  spades: "A53".into() },
            HandSuits { clubs: "K43".into(),   diamonds: "K654".into(),   hearts: "986".into(),  spades: "QT9".into() },
            HandSuits { clubs: "Q872".into(),  diamonds: "J7".into(),     hearts: "KQT".into(),  spades: "KJ74".into() },
            HandSuits { clubs: "AT965".into(), diamonds: "9".into(),      hearts: "J732".into(), spades: "862".into() },
        ];
        // Dealer=S(2), Vul=None(0), board_extension=0 (Board 1 → (1-1)/16 = 0).
        let hash = encode(&hands, /*dealer=*/ 2, /*vul=*/ 0, /*board_ext=*/ 0);
        assert_eq!(hash, "0808AE69B36854D9B1DC0C8E3AF9",
            "must reproduce BBA.exe v8643 hash for Fourth_Suit_Forcing Board 1");
    }

    #[test]
    fn board_extension_helper() {
        assert_eq!(board_extension_for(1), 0);
        assert_eq!(board_extension_for(16), 0);
        assert_eq!(board_extension_for(17), 1);
        assert_eq!(board_extension_for(33), 2);
        assert_eq!(board_extension_for(257), 0); // wraps mod 16
    }

    #[test]
    fn empty_hands_decode_safely() {
        // Bogus input length should return None.
        assert!(decode("0").is_none());
        assert!(decode(&"X".repeat(28)).is_none());
    }
}
