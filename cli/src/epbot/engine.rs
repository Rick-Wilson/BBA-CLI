//! EPBot engine wrapper
//!
//! Calls the C# epbot-wrapper.exe via stdin/stdout JSON communication.

use crate::error::{BbaError, BbaResult};
use dealer_core::Position;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Request to the C# wrapper for batch processing
#[derive(Serialize)]
struct BatchRequest {
    ns_conventions: Option<String>,
    ew_conventions: Option<String>,
    deals: Vec<DealInput>,
}

/// Single deal input
#[derive(Serialize)]
struct DealInput {
    pbn: String,
    dealer: String,
    vulnerability: String,
}

/// Response from the C# wrapper
#[derive(Deserialize)]
struct BatchResponse {
    results: Vec<DealResult>,
}

/// Result for a single deal
#[derive(Deserialize, Debug, Clone)]
pub struct DealResult {
    pub deal: Option<String>,
    pub auction: Option<Vec<String>>,
    pub success: bool,
    pub error: Option<String>,
}

/// EPBot engine that communicates with the C# wrapper
pub struct EPBot {
    wrapper_path: String,
    ns_conventions: Option<String>,
    ew_conventions: Option<String>,
}

impl EPBot {
    /// Create a new EPBot instance
    ///
    /// # Arguments
    ///
    /// * `wrapper_path` - Path to epbot-wrapper.exe
    pub fn new(wrapper_path: &str) -> BbaResult<Self> {
        // Verify wrapper exists
        if !Path::new(wrapper_path).exists() {
            return Err(BbaError::FfiError(format!(
                "EPBot wrapper not found at: {}",
                wrapper_path
            )));
        }

        Ok(Self {
            wrapper_path: wrapper_path.to_string(),
            ns_conventions: None,
            ew_conventions: None,
        })
    }

    /// Load conventions for a partnership
    pub fn load_conventions(&mut self, path: &Path, side: Side) -> BbaResult<()> {
        let path_str = path.to_str().ok_or(BbaError::InvalidPath)?.to_string();

        match side {
            Side::NorthSouth => self.ns_conventions = Some(path_str),
            Side::EastWest => self.ew_conventions = Some(path_str),
        }

        Ok(())
    }

    /// Generate auctions for multiple deals in a single call
    ///
    /// This is more efficient than calling generate_auction for each deal
    /// because it only spawns the C# wrapper once.
    pub fn generate_auctions(&self, deals: Vec<DealSpec>) -> BbaResult<Vec<DealResult>> {
        let request = BatchRequest {
            ns_conventions: self.ns_conventions.clone(),
            ew_conventions: self.ew_conventions.clone(),
            deals: deals
                .into_iter()
                .map(|d| DealInput {
                    pbn: d.pbn,
                    dealer: position_to_string(d.dealer),
                    vulnerability: d.vulnerability.to_pbn().to_string(),
                })
                .collect(),
        };

        let json_input = serde_json::to_string(&request)
            .map_err(|e| BbaError::InvalidPbn(format!("JSON serialization error: {}", e)))?;

        // Spawn the wrapper process
        let mut child = Command::new(&self.wrapper_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| BbaError::FfiError(format!("Failed to spawn wrapper: {}", e)))?;

        // Write JSON to stdin
        {
            let stdin = child
                .stdin
                .as_mut()
                .ok_or(BbaError::FfiError("Failed to open stdin".to_string()))?;
            stdin
                .write_all(json_input.as_bytes())
                .map_err(|e| BbaError::FfiError(format!("Failed to write to stdin: {}", e)))?;
        }

        // Wait for output
        let output = child
            .wait_with_output()
            .map_err(|e| BbaError::FfiError(format!("Failed to read output: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BbaError::FfiError(format!(
                "Wrapper exited with error: {}",
                stderr
            )));
        }

        // Parse JSON response
        let stdout = String::from_utf8_lossy(&output.stdout);
        let response: BatchResponse = serde_json::from_str(&stdout).map_err(|e| {
            BbaError::FfiError(format!(
                "Failed to parse response: {} - Output: {}",
                e, stdout
            ))
        })?;

        Ok(response.results)
    }

    /// Generate auction for a single deal
    pub fn generate_auction(&self, deal: DealSpec) -> BbaResult<Vec<String>> {
        let results = self.generate_auctions(vec![deal])?;

        let result = results
            .into_iter()
            .next()
            .ok_or(BbaError::FfiError("No result returned".to_string()))?;

        if result.success {
            result
                .auction
                .ok_or(BbaError::FfiError("No auction in result".to_string()))
        } else {
            Err(BbaError::EngineError(
                -1,
                result.error.unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }
}

/// Specification for a deal to process
#[derive(Debug, Clone)]
pub struct DealSpec {
    pub pbn: String,
    pub dealer: Position,
    pub vulnerability: Vulnerability,
}

/// Vulnerability settings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vulnerability {
    None,
    NorthSouth,
    EastWest,
    Both,
}

impl Vulnerability {
    /// Parse vulnerability from PBN string
    pub fn from_pbn(s: &str) -> Option<Self> {
        match s.trim().to_uppercase().as_str() {
            "NONE" | "-" | "" => Some(Vulnerability::None),
            "NS" | "N-S" => Some(Vulnerability::NorthSouth),
            "EW" | "E-W" => Some(Vulnerability::EastWest),
            "BOTH" | "ALL" => Some(Vulnerability::Both),
            _ => None,
        }
    }

    /// Convert to PBN string
    pub fn to_pbn(self) -> &'static str {
        match self {
            Vulnerability::None => "None",
            Vulnerability::NorthSouth => "NS",
            Vulnerability::EastWest => "EW",
            Vulnerability::Both => "Both",
        }
    }
}

/// Partnership side for convention configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    NorthSouth,
    EastWest,
}

fn position_to_string(pos: Position) -> String {
    match pos {
        Position::North => "N",
        Position::East => "E",
        Position::South => "S",
        Position::West => "W",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vulnerability_parsing() {
        assert_eq!(Vulnerability::from_pbn("None"), Some(Vulnerability::None));
        assert_eq!(Vulnerability::from_pbn("-"), Some(Vulnerability::None));
        assert_eq!(
            Vulnerability::from_pbn("NS"),
            Some(Vulnerability::NorthSouth)
        );
        assert_eq!(Vulnerability::from_pbn("EW"), Some(Vulnerability::EastWest));
        assert_eq!(Vulnerability::from_pbn("Both"), Some(Vulnerability::Both));
        assert_eq!(Vulnerability::from_pbn("All"), Some(Vulnerability::Both));
        assert_eq!(Vulnerability::from_pbn("invalid"), None);
    }

    #[test]
    fn test_vulnerability_to_pbn() {
        assert_eq!(Vulnerability::None.to_pbn(), "None");
        assert_eq!(Vulnerability::NorthSouth.to_pbn(), "NS");
        assert_eq!(Vulnerability::EastWest.to_pbn(), "EW");
        assert_eq!(Vulnerability::Both.to_pbn(), "Both");
    }
}
