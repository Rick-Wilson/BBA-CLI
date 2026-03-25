use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing::warn;

const AUCTION_CSV_HEADER: &str = "Timestamp,RequestIP,ClientVersion,Extension,Browser,OS,DurationMs,Version,EPBotVersion,Dealer,Vulnerability,Scoring,NSConvention,EWConvention,Scenario,PBN,Success,Auction,Alerts,Error";
const SCENARIO_CSV_HEADER: &str = "Timestamp,RequestIP,ClientVersion,Extension,Browser,OS,Version,Scenario";

pub struct AuditLogService {
    log_directory: PathBuf,
    version: String,
    lock: Mutex<()>,
}

impl AuditLogService {
    pub fn new(log_directory: &str, version: &str) -> Self {
        let path = PathBuf::from(log_directory);
        let _ = fs::create_dir_all(&path);

        let svc = Self {
            log_directory: path,
            version: version.to_string(),
            lock: Mutex::new(()),
        };
        svc.cleanup_old_logs(30);
        svc
    }

    #[allow(clippy::too_many_arguments)]
    pub fn log_request(
        &self,
        request_ip: &str,
        client_version: &str,
        extension: &str,
        browser: &str,
        os: &str,
        duration_ms: u64,
        epbot_version: i32,
        dealer: &str,
        vulnerability: &str,
        scoring: &str,
        ns_convention: &str,
        ew_convention: &str,
        scenario: &str,
        pbn: &str,
        success: bool,
        auction: &str,
        alerts: &str,
        error: &str,
    ) {
        let now = chrono::Local::now();
        let log_file = self
            .log_directory
            .join(format!("audit-auction-{}.csv", now.format("%Y-%m")));
        let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();

        let row = format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            escape_csv(&timestamp),
            escape_csv(request_ip),
            escape_csv(client_version),
            escape_csv(extension),
            escape_csv(browser),
            escape_csv(os),
            duration_ms,
            escape_csv(&self.version),
            epbot_version,
            escape_csv(dealer),
            escape_csv(vulnerability),
            escape_csv(scoring),
            escape_csv(ns_convention),
            escape_csv(ew_convention),
            escape_csv(scenario),
            escape_csv(pbn),
            if success { "true" } else { "false" },
            escape_csv(auction),
            escape_csv(alerts),
            escape_csv(error),
        );

        self.append_to_csv(&log_file, AUCTION_CSV_HEADER, &row);
    }

    pub fn log_scenario_selection(
        &self,
        request_ip: &str,
        client_version: &str,
        extension: &str,
        browser: &str,
        os: &str,
        scenario: &str,
    ) {
        let now = chrono::Local::now();
        let log_file = self
            .log_directory
            .join(format!("audit-scenario-{}.csv", now.format("%Y-%m")));
        let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();

        let row = format!(
            "{},{},{},{},{},{},{},{}",
            escape_csv(&timestamp),
            escape_csv(request_ip),
            escape_csv(client_version),
            escape_csv(extension),
            escape_csv(browser),
            escape_csv(os),
            escape_csv(&self.version),
            escape_csv(scenario),
        );

        self.append_to_csv(&log_file, SCENARIO_CSV_HEADER, &row);
    }

    /// Get list of log files with metadata.
    pub fn get_log_files(&self) -> Vec<serde_json::Value> {
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.log_directory) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let file_type = if name.ends_with(".csv") {
                        "csv"
                    } else if name.ends_with(".log") {
                        "log"
                    } else {
                        "other"
                    };
                    files.push(serde_json::json!({
                        "name": name,
                        "size": meta.len(),
                        "lastModified": meta.modified().ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_millis()),
                        "type": file_type,
                    }));
                }
            }
        }
        files.sort_by(|a, b| {
            b["name"].as_str().cmp(&a["name"].as_str())
        });
        files
    }

    /// Read a log file and parse CSV data if applicable.
    pub fn get_log_data(&self, filename: &str) -> Option<serde_json::Value> {
        // Sanitize filename
        if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
            return None;
        }

        let path = self.log_directory.join(filename);
        let content = fs::read_to_string(&path).ok()?;

        if filename.ends_with(".csv") {
            let rows = parse_csv(&content);
            Some(serde_json::json!({
                "filename": filename,
                "rowCount": rows.len(),
                "data": rows,
            }))
        } else {
            Some(serde_json::json!({
                "filename": filename,
                "content": content,
            }))
        }
    }

    fn append_to_csv(&self, path: &Path, header: &str, row: &str) {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let write_header = !path.exists();

        match fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            Ok(mut file) => {
                if write_header {
                    let _ = writeln!(file, "{}", header);
                }
                let _ = writeln!(file, "{}", row);
            }
            Err(e) => warn!("Failed to write audit log: {}", e),
        }
    }

    fn cleanup_old_logs(&self, retention_days: i64) {
        let cutoff = chrono::Local::now() - chrono::Duration::days(retention_days);
        if let Ok(entries) = fs::read_dir(&self.log_directory) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        let modified_time: chrono::DateTime<chrono::Local> = modified.into();
                        if modified_time < cutoff {
                            let _ = fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
    }
}

fn escape_csv(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn parse_csv(content: &str) -> Vec<serde_json::Map<String, serde_json::Value>> {
    let mut lines = content.lines();
    let headers: Vec<&str> = match lines.next() {
        Some(h) => h.split(',').collect(),
        None => return Vec::new(),
    };

    lines
        .filter(|l| !l.is_empty())
        .map(|line| {
            let values = split_csv_line(line);
            let mut map = serde_json::Map::new();
            for (i, header) in headers.iter().enumerate() {
                let val = values.get(i).map(|s| s.as_str()).unwrap_or("");
                map.insert(header.to_string(), serde_json::Value::String(val.to_string()));
            }
            map
        })
        .collect()
}

fn split_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(ch);
            }
        } else if ch == '"' {
            in_quotes = true;
        } else if ch == ',' {
            fields.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }
    fields.push(current);
    fields
}
