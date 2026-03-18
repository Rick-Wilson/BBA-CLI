use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use std::time::Instant;
use crate::models::*;
use crate::AppState;
use crate::services::ip_anonymizer;
use epbot_core::{ConventionCard, Scoring};

/// Extract client IP from headers (Cloudflare → X-Forwarded-For → connection).
fn get_client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("CF-Connecting-IP")
        .or_else(|| headers.get("X-Forwarded-For"))
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
}

fn get_client_version(headers: &HeaderMap) -> String {
    headers
        .get("X-Client-Version")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string()
}

/// POST /api/auction/generate
pub async fn generate_auction(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AuctionRequest>,
) -> Json<AuctionResponse> {
    let start = Instant::now();
    let raw_ip = get_client_ip(&headers);
    let anon_ip = ip_anonymizer::anonymize(raw_ip.as_deref());
    let client_version = get_client_version(&headers);

    // Determine convention cards
    let conventions = if let Some(ref conv) = request.conventions {
        conv.clone()
    } else if let Some(ref scenario) = request.scenario {
        let (ns, ew) = state
            .convention_service
            .get_conventions_for_scenario(scenario)
            .await;
        ConventionCards { ns, ew }
    } else {
        ConventionCards::default()
    };

    // Fetch convention card content from GitHub
    let ns_content = state
        .convention_service
        .get_bbsa_content(&conventions.ns)
        .await;
    let ew_content = state
        .convention_service
        .get_bbsa_content(&conventions.ew)
        .await;

    let response = match (ns_content, ew_content) {
        (Ok(ns_text), Ok(ew_text)) => {
            let ns_card = ConventionCard::from_content(&ns_text);
            let ew_card = ConventionCard::from_content(&ew_text);

            // Parse dealer
            let dealer = parse_dealer(&request.deal.dealer);
            let vul = parse_vulnerability(&request.deal.vulnerability);
            let scoring = if request.deal.scoring.eq_ignore_ascii_case("IMP") {
                Scoring::Imps
            } else {
                Scoring::Matchpoints
            };

            // Acquire semaphore permit for concurrency limiting
            let _permit = state.semaphore.acquire().await.unwrap();

            // Clone values needed after the spawn_blocking move
            let pbn = request.deal.pbn.clone();
            let deal_dealer = request.deal.dealer.clone();
            let deal_vul = request.deal.vulnerability.clone();
            let deal_scoring = request.deal.scoring.clone();
            let deal_pbn = request.deal.pbn.clone();
            let scenario_clone = request.scenario.clone();

            let result = tokio::task::spawn_blocking(move || {
                epbot_core::generate_auction(
                    &pbn,
                    dealer,
                    vul,
                    scoring,
                    Some(&ns_card),
                    Some(&ew_card),
                )
            })
            .await
            .unwrap();

            if result.success {
                let auction: Vec<String> = result.bids.iter().map(|b| b.bid.clone()).collect();
                let encoded = encode_bbo_format(&auction);
                let meanings: Vec<BidMeaning> = result
                    .bids
                    .iter()
                    .enumerate()
                    .map(|(i, b)| BidMeaning {
                        position: i,
                        bid: b.bid.clone(),
                        meaning: b.meaning.clone(),
                        is_alert: b.is_alert,
                    })
                    .collect();

                let alerts_str = format_alerts(&meanings);
                let duration = start.elapsed().as_millis() as u64;

                state.audit_log.log_request(
                    &anon_ip,
                    &client_version,
                    duration,
                    state.epbot_version,
                    &deal_dealer,
                    &deal_vul,
                    &deal_scoring,
                    &conventions.ns,
                    &conventions.ew,
                    scenario_clone.as_deref().unwrap_or(""),
                    &deal_pbn,
                    true,
                    &encoded,
                    &alerts_str,
                    "",
                );

                AuctionResponse {
                    success: true,
                    auction: Some(auction),
                    auction_encoded: Some(encoded),
                    conventions_used: Some(conventions),
                    meanings: Some(meanings),
                    error: None,
                }
            } else {
                let err = result.error.unwrap_or_else(|| "Unknown error".into());
                let duration = start.elapsed().as_millis() as u64;

                state.audit_log.log_request(
                    &anon_ip,
                    &client_version,
                    duration,
                    state.epbot_version,
                    &deal_dealer,
                    &deal_vul,
                    &deal_scoring,
                    &conventions.ns,
                    &conventions.ew,
                    scenario_clone.as_deref().unwrap_or(""),
                    &deal_pbn,
                    false,
                    "",
                    "",
                    &err,
                );

                AuctionResponse {
                    success: false,
                    auction: None,
                    auction_encoded: None,
                    conventions_used: Some(conventions),
                    meanings: None,
                    error: Some(err),
                }
            }
        }
        (Err(e), _) | (_, Err(e)) => AuctionResponse {
            success: false,
            auction: None,
            auction_encoded: None,
            conventions_used: Some(conventions),
            meanings: None,
            error: Some(e),
        },
    };

    Json(response)
}

/// POST /api/scenario/select
pub async fn select_scenario(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ScenarioSelectRequest>,
) -> Json<serde_json::Value> {
    let raw_ip = get_client_ip(&headers);
    let anon_ip = ip_anonymizer::anonymize(raw_ip.as_deref());
    let client_version = get_client_version(&headers);

    state.audit_log.log_scenario_selection(
        &anon_ip,
        &client_version,
        request.scenario.as_deref().unwrap_or(""),
    );

    Json(serde_json::json!({ "success": true }))
}

/// GET /api/scenarios
pub async fn list_scenarios(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    match state.convention_service.get_scenario_list().await {
        Ok(scenarios) => Json(serde_json::json!({ "scenarios": scenarios })),
        Err(e) => Json(serde_json::json!({ "scenarios": [], "error": e })),
    }
}

fn parse_dealer(dealer: &str) -> i32 {
    match dealer.to_uppercase().as_str() {
        "N" | "NORTH" => 0,
        "E" | "EAST" => 1,
        "S" | "SOUTH" => 2,
        "W" | "WEST" => 3,
        _ => 0,
    }
}

fn parse_vulnerability(vul: &str) -> i32 {
    match vul.to_uppercase().replace('-', "").as_str() {
        "NONE" | "" => 0,
        "EW" | "EASTWEST" => 1,
        "NS" | "NORTHSOUTH" => 2,
        "BOTH" | "ALL" => 3,
        _ => 0,
    }
}

fn encode_bbo_format(bids: &[String]) -> String {
    bids.iter()
        .map(|bid| match bid.as_str() {
            "Pass" => "--".to_string(),
            "X" => "Db".to_string(),
            "XX" => "Rd".to_string(),
            b if b.ends_with("NT") => format!("{}N", &b[..1]),
            b => format!("{:<2}", b),
        })
        .collect()
}

fn format_alerts(meanings: &[BidMeaning]) -> String {
    meanings
        .iter()
        .filter(|m| m.is_alert && m.meaning.is_some())
        .map(|m| format!("{}={}", m.bid, m.meaning.as_deref().unwrap_or("")))
        .collect::<Vec<_>>()
        .join("; ")
}
