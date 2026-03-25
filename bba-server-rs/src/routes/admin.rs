use axum::extract::connect_info::ConnectInfo;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Json, Redirect};
use serde::Deserialize;
use std::net::SocketAddr;

use crate::services::ip_anonymizer;
use crate::AppState;

#[derive(Deserialize)]
pub struct AdminQuery {
    pub key: Option<String>,
}

fn get_admin_context(
    headers: &HeaderMap,
    query: &AdminQuery,
    conn: &SocketAddr,
) -> (Option<String>, String, Option<String>) {
    let raw_ip = headers
        .get("CF-Connecting-IP")
        .or_else(|| headers.get("X-Forwarded-For"))
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .or_else(|| Some(conn.ip().to_string()));

    let anon_ip = ip_anonymizer::anonymize(raw_ip.as_deref());
    let key = query.key.clone();
    (raw_ip, anon_ip, key)
}

fn is_allowed(state: &AppState, raw_ip: &Option<String>, anon_ip: &str, key: &Option<String>) -> bool {
    // Always allow localhost
    if let Some(ref ip) = raw_ip {
        if ip == "127.0.0.1" || ip == "::1" {
            return true;
        }
    }

    // Check admin key
    if let Some(ref k) = key {
        if !state.config.admin_key.is_empty() && k == &state.config.admin_key {
            return true;
        }
    }

    // Check IP whitelist
    state.config.admin_users.contains(&anon_ip.to_string())
}

/// GET /admin/whoami
pub async fn whoami(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminQuery>,
    ConnectInfo(conn): ConnectInfo<SocketAddr>,
) -> Json<serde_json::Value> {
    let (raw_ip, anon_ip, key) = get_admin_context(&headers, &query, &conn);
    let allowed = is_allowed(&state, &raw_ip, &anon_ip, &key);

    Json(serde_json::json!({
        "rawIP": raw_ip,
        "anonymizedIP": anon_ip,
        "isAllowed": allowed,
        "adminUsers": state.config.admin_users,
    }))
}

/// GET /admin
pub async fn admin_root(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminQuery>,
    ConnectInfo(conn): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let (raw_ip, anon_ip, key) = get_admin_context(&headers, &query, &conn);
    if !is_allowed(&state, &raw_ip, &anon_ip, &key) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let redirect = match &key {
        Some(k) if !k.is_empty() => format!("/admin/dashboard?key={}", k),
        _ => "/admin/dashboard".to_string(),
    };
    Redirect::to(&redirect).into_response()
}

/// GET /admin/dashboard
pub async fn dashboard(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminQuery>,
    ConnectInfo(conn): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let (raw_ip, anon_ip, key) = get_admin_context(&headers, &query, &conn);
    if !is_allowed(&state, &raw_ip, &anon_ip, &key) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    // Try disk first (enables live editing without rebuild), fall back to embedded
    let html = std::fs::read_to_string("wwwroot/dashboard.html")
        .unwrap_or_else(|_| include_str!("../../wwwroot/dashboard.html").to_string());
    Html(html).into_response()
}

/// GET /admin/api/logs
pub async fn list_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminQuery>,
    ConnectInfo(conn): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let (raw_ip, anon_ip, key) = get_admin_context(&headers, &query, &conn);
    if !is_allowed(&state, &raw_ip, &anon_ip, &key) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    Json(state.audit_log.get_log_files()).into_response()
}

/// GET /admin/api/logs/:filename
pub async fn get_log(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminQuery>,
    ConnectInfo(conn): ConnectInfo<SocketAddr>,
    Path(filename): Path<String>,
) -> impl IntoResponse {
    let (raw_ip, anon_ip, key) = get_admin_context(&headers, &query, &conn);
    if !is_allowed(&state, &raw_ip, &anon_ip, &key) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    match state.audit_log.get_log_data(&filename) {
        Some(data) => Json(data).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// GET /admin/api/stats — compute stats from auction audit log
pub async fn stats(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminQuery>,
    ConnectInfo(conn): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let (raw_ip, anon_ip, key) = get_admin_context(&headers, &query, &conn);
    if !is_allowed(&state, &raw_ip, &anon_ip, &key) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    // Find the current month's auction log
    let now = chrono::Local::now();
    let filename = format!("audit-auction-{}.csv", now.format("%Y-%m"));
    let data = state.audit_log.get_log_data(&filename);

    let stats = compute_auction_stats(data);
    Json(stats).into_response()
}

/// GET /admin/api/scenario-stats
pub async fn scenario_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminQuery>,
    ConnectInfo(conn): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let (raw_ip, anon_ip, key) = get_admin_context(&headers, &query, &conn);
    if !is_allowed(&state, &raw_ip, &anon_ip, &key) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let now = chrono::Local::now();
    let filename = format!("audit-scenario-{}.csv", now.format("%Y-%m"));
    let data = state.audit_log.get_log_data(&filename);

    let stats = compute_scenario_stats(data);
    Json(stats).into_response()
}

fn compute_auction_stats(data: Option<serde_json::Value>) -> serde_json::Value {
    let rows = match data {
        Some(ref d) => d["data"].as_array(),
        None => None,
    };

    let rows = match rows {
        Some(r) => r,
        None => {
            return serde_json::json!({
                "totalRequests": 0,
                "successfulRequests": 0,
                "failedRequests": 0,
                "averageDurationMs": 0,
                "maxDurationMs": 0,
                "requestsByDay": {},
                "requestsByScenario": {},
                "requestsByUser": {},
                "recentErrors": [],
            });
        }
    };

    let total = rows.len();
    let mut successful = 0;
    let mut durations: Vec<u64> = Vec::new();
    let mut by_day = serde_json::Map::new();
    let mut by_scenario = serde_json::Map::new();
    let mut by_user = serde_json::Map::new();
    let mut errors = Vec::new();

    for row in rows {
        if row["Success"].as_str() == Some("true") {
            successful += 1;
        } else {
            let ts = row["Timestamp"].as_str().unwrap_or("");
            let scenario = row["Scenario"].as_str().unwrap_or("");
            let error = row["Error"].as_str().unwrap_or("");
            errors.push(serde_json::json!({
                "timestamp": ts,
                "scenario": scenario,
                "error": error,
            }));
        }

        if let Some(d) = row["DurationMs"].as_str().and_then(|s| s.parse::<u64>().ok()) {
            durations.push(d);
        }

        if let Some(ts) = row["Timestamp"].as_str() {
            let day = ts.split(' ').next().unwrap_or(ts);
            *by_day
                .entry(day.to_string())
                .or_insert(serde_json::Value::Number(0.into())) = {
                let v = by_day
                    .get(day)
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                serde_json::Value::Number((v + 1).into())
            };
        }

        if let Some(scenario) = row["Scenario"].as_str() {
            if !scenario.is_empty() {
                let count = by_scenario
                    .get(scenario)
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                by_scenario.insert(
                    scenario.to_string(),
                    serde_json::Value::Number((count + 1).into()),
                );
            }
        }

        if let Some(user) = row["RequestIP"].as_str() {
            let count = by_user
                .get(user)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            by_user.insert(
                user.to_string(),
                serde_json::Value::Number((count + 1).into()),
            );
        }
    }

    let avg_duration = if durations.is_empty() {
        0
    } else {
        durations.iter().sum::<u64>() / durations.len() as u64
    };
    let max_duration = durations.iter().max().copied().unwrap_or(0);

    // Keep only last 10 errors
    let recent_errors: Vec<_> = errors.into_iter().rev().take(10).collect();

    serde_json::json!({
        "totalRequests": total,
        "successfulRequests": successful,
        "failedRequests": total - successful,
        "averageDurationMs": avg_duration,
        "maxDurationMs": max_duration,
        "requestsByDay": by_day,
        "requestsByScenario": by_scenario,
        "requestsByUser": by_user,
        "recentErrors": recent_errors,
    })
}

fn compute_scenario_stats(data: Option<serde_json::Value>) -> serde_json::Value {
    let rows = match data {
        Some(ref d) => d["data"].as_array(),
        None => None,
    };

    let rows = match rows {
        Some(r) => r,
        None => {
            return serde_json::json!({
                "totalSelections": 0,
                "selectionsByScenario": {},
                "selectionsByUser": {},
                "selectionsByDay": {},
            });
        }
    };

    let mut by_scenario = serde_json::Map::new();
    let mut by_user = serde_json::Map::new();
    let mut by_day = serde_json::Map::new();

    for row in rows {
        if let Some(scenario) = row["Scenario"].as_str() {
            if !scenario.is_empty() {
                let count = by_scenario
                    .get(scenario)
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                by_scenario.insert(
                    scenario.to_string(),
                    serde_json::Value::Number((count + 1).into()),
                );
            }
        }

        if let Some(user) = row["RequestIP"].as_str() {
            let count = by_user
                .get(user)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            by_user.insert(
                user.to_string(),
                serde_json::Value::Number((count + 1).into()),
            );
        }

        if let Some(ts) = row["Timestamp"].as_str() {
            let day = ts.split(' ').next().unwrap_or(ts);
            let count = by_day
                .get(day)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            by_day.insert(
                day.to_string(),
                serde_json::Value::Number((count + 1).into()),
            );
        }
    }

    serde_json::json!({
        "totalSelections": rows.len(),
        "selectionsByScenario": by_scenario,
        "selectionsByUser": by_user,
        "selectionsByDay": by_day,
    })
}
