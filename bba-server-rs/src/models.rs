use serde::{Deserialize, Serialize};

/// Request to generate an auction for a deal.
#[derive(Deserialize)]
pub struct AuctionRequest {
    pub deal: DealInfo,
    pub scenario: Option<String>,
    pub conventions: Option<ConventionCards>,
}

/// Deal information in PBN format.
#[derive(Deserialize)]
pub struct DealInfo {
    pub pbn: String,
    pub dealer: String,
    pub vulnerability: String,
    #[serde(default = "default_scoring")]
    pub scoring: String,
}

fn default_scoring() -> String {
    "MP".to_string()
}

/// Convention card specifications.
#[derive(Deserialize, Serialize, Clone)]
pub struct ConventionCards {
    #[serde(default = "default_ns_card")]
    pub ns: String,
    #[serde(default = "default_ew_card")]
    pub ew: String,
}

fn default_ns_card() -> String {
    "21GF-DEFAULT".to_string()
}
fn default_ew_card() -> String {
    "21GF-GIB".to_string()
}

impl Default for ConventionCards {
    fn default() -> Self {
        Self {
            ns: default_ns_card(),
            ew: default_ew_card(),
        }
    }
}

/// Response from auction generation.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuctionResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auction: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auction_encoded: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conventions_used: Option<ConventionCards>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meanings: Option<Vec<BidMeaning>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Meaning of a bid from EPBot.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BidMeaning {
    pub position: usize,
    pub bid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meaning: Option<String>,
    pub is_alert: bool,
}

/// Request to record a scenario selection.
#[derive(Deserialize)]
pub struct ScenarioSelectRequest {
    pub scenario: Option<String>,
}
