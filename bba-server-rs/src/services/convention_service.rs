use regex::Regex;
use tracing::{info, warn};

/// Fetches convention cards (.bbsa) and scenario files (.pbs) from GitHub.
pub struct ConventionService {
    client: reqwest::Client,
    github_raw_base_url: String,
    default_ns_card: String,
    default_ew_card: String,
}

impl ConventionService {
    pub fn new(github_raw_base_url: &str, default_ns_card: &str, default_ew_card: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            github_raw_base_url: github_raw_base_url.to_string(),
            default_ns_card: default_ns_card.to_string(),
            default_ew_card: default_ew_card.to_string(),
        }
    }

    pub fn default_ns(&self) -> &str {
        &self.default_ns_card
    }

    pub fn default_ew(&self) -> &str {
        &self.default_ew_card
    }

    /// Fetch a .bbsa convention card from GitHub and return its content.
    pub async fn get_bbsa_content(&self, card_name: &str) -> Result<String, String> {
        let url = format!("{}/bbsa/{}.bbsa", self.github_raw_base_url, card_name);
        info!("Fetching convention card from GitHub: {}", url);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch convention card: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!(
                "Convention card not found on GitHub: {} (HTTP {})",
                card_name,
                resp.status()
            ));
        }

        resp.text()
            .await
            .map_err(|e| format!("Failed to read convention card: {}", e))
    }

    /// Get conventions for a scenario by fetching its .pbs file from GitHub.
    pub async fn get_conventions_for_scenario(
        &self,
        scenario: &str,
    ) -> (String, String) {
        match self.get_convention_cards_from_pbs(scenario).await {
            (Some(ns), Some(ew)) => (ns, ew),
            (Some(ns), None) => (ns, self.default_ew_card.clone()),
            (None, Some(ew)) => (self.default_ns_card.clone(), ew),
            (None, None) => (self.default_ns_card.clone(), self.default_ew_card.clone()),
        }
    }

    async fn get_convention_cards_from_pbs(&self, scenario: &str) -> (Option<String>, Option<String>) {
        let url = format!("{}/pbs-release/{}.pbs", self.github_raw_base_url, scenario);

        let resp = match self.client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                warn!("Error fetching PBS file for {}: {}", scenario, e);
                return (None, None);
            }
        };

        if !resp.status().is_success() {
            warn!("PBS file not found for {} (HTTP {})", scenario, resp.status());
            return (None, None);
        }

        let content = match resp.text().await {
            Ok(c) => c,
            Err(_) => return (None, None),
        };

        // Try new format: convention-card-ns and convention-card-ew
        let ns_re = Regex::new(r"(?mi)^convention-card-ns:\s*(.+)$").unwrap();
        let ew_re = Regex::new(r"(?mi)^convention-card-ew:\s*(.+)$").unwrap();

        let ns = ns_re.captures(&content).map(|c| c[1].trim().to_string());
        let ew = ew_re.captures(&content).map(|c| c[1].trim().to_string());

        if ns.is_some() || ew.is_some() {
            info!(
                "Found convention cards NS='{}', EW='{}' for scenario '{}'",
                ns.as_deref().unwrap_or("(default)"),
                ew.as_deref().unwrap_or("(default)"),
                scenario
            );
            return (ns, ew);
        }

        // Fall back to old format: convention-card (NS only)
        let old_re = Regex::new(r"(?mi)^#?\s*convention-card:\s*(.+)$").unwrap();
        if let Some(caps) = old_re.captures(&content) {
            let value = caps[1].trim().to_string();
            if !value.is_empty() {
                info!(
                    "Found convention card '{}' (old format) for scenario '{}'",
                    value, scenario
                );
                return (Some(value), None);
            }
        }

        info!("No convention card specified for scenario '{}', using defaults", scenario);
        (None, None)
    }

    /// Get the list of available scenarios from GitHub.
    pub async fn get_scenario_list(&self) -> Result<Vec<String>, String> {
        let url =
            "https://api.github.com/repos/ADavidBailey/Practice-Bidding-Scenarios/contents/pbs-release";

        let resp = self
            .client
            .get(url)
            .header("User-Agent", "BBA-Server")
            .send()
            .await
            .map_err(|e| format!("Failed to fetch scenario list: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("GitHub API returned {}", resp.status()));
        }

        let items: Vec<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse GitHub response: {}", e))?;

        let mut scenarios: Vec<String> = items
            .iter()
            .filter_map(|item| {
                let name = item["name"].as_str()?;
                if name.ends_with(".pbs") {
                    Some(name.trim_end_matches(".pbs").to_string())
                } else {
                    None
                }
            })
            .collect();

        scenarios.sort();
        Ok(scenarios)
    }
}
