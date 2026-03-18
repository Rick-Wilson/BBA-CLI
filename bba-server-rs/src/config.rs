/// Server configuration loaded from environment variables.
#[derive(Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub api_key: String,
    pub admin_key: String,
    pub default_ns_card: String,
    pub default_ew_card: String,
    pub github_raw_base_url: String,
    pub log_path: String,
    pub max_concurrency: usize,
    pub admin_users: Vec<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(5000),
            api_key: std::env::var("API_KEY").unwrap_or_default(),
            admin_key: std::env::var("ADMIN_KEY").unwrap_or_default(),
            default_ns_card: std::env::var("DEFAULT_NS_CARD")
                .unwrap_or_else(|_| "21GF-DEFAULT".into()),
            default_ew_card: std::env::var("DEFAULT_EW_CARD")
                .unwrap_or_else(|_| "21GF-GIB".into()),
            github_raw_base_url: std::env::var("GITHUB_RAW_BASE_URL").unwrap_or_else(|_| {
                "https://raw.githubusercontent.com/ADavidBailey/Practice-Bidding-Scenarios/main"
                    .into()
            }),
            log_path: std::env::var("LOG_PATH").unwrap_or_else(|_| "logs".into()),
            max_concurrency: std::env::var("MAX_CONCURRENCY")
                .ok()
                .and_then(|c| c.parse().ok())
                .unwrap_or(4),
            admin_users: std::env::var("ADMIN_USERS")
                .unwrap_or_else(|_| {
                    "Valerie_Perez,Travis_Scott,Tom_Martinez,Carol_Jordan,Joe_Evans,Rebecca_Coleman,Timothy_Carter".into()
                })
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        }
    }
}
