//! Native EPBot engine — calls EPBotWrapper.dylib directly via FFI
//!
//! Provides the same DealSpec/DealResult interface as the Windows subprocess wrapper,
//! enabling drop-in compatibility with the batch processor.

#![allow(dead_code)]

use crate::ffi;
use anyhow::{bail, Context, Result};
use dealer_core::Position;
use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use std::os::raw::c_void;
use std::path::Path;

/// Error codes from EPBotWrapper
const OK: i32 = 0;
const ERR_AUCTION_COMPLETE: i32 = -9;

/// Result for a single deal (matches Windows wrapper JSON format)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DealResult {
    pub deal: Option<String>,
    pub auction: Option<Vec<String>>,
    pub success: bool,
    pub error: Option<String>,
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
    /// Parse from PBN string
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

    /// Convert to FFI integer (0=None, 1=NS, 2=EW, 3=Both)
    fn to_ffi(self) -> i32 {
        match self {
            Vulnerability::None => 0,
            Vulnerability::NorthSouth => 1,
            Vulnerability::EastWest => 2,
            Vulnerability::Both => 3,
        }
    }
}

/// Partnership side for convention configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    NorthSouth,
    EastWest,
}

impl Side {
    fn to_ffi(self) -> i32 {
        match self {
            Side::NorthSouth => 0,
            Side::EastWest => 1,
        }
    }
}

/// Native EPBot engine using FFI to EPBotWrapper.dylib
pub struct EPBot {
    ns_conventions: Option<String>,
    ew_conventions: Option<String>,
}

impl EPBot {
    /// Create a new EPBot engine instance.
    /// The dylib must be loadable (in DYLD_LIBRARY_PATH or rpath).
    pub fn new() -> Result<Self> {
        // Verify the dylib is loadable by creating and immediately destroying an instance
        let inst = unsafe { ffi::epbot_create() };
        if inst.is_null() {
            bail!("Failed to create EPBot instance — EPBotWrapper.dylib may not be loadable");
        }
        unsafe { ffi::epbot_destroy(inst) };

        Ok(Self {
            ns_conventions: None,
            ew_conventions: None,
        })
    }

    /// Load conventions for a partnership
    pub fn load_conventions(&mut self, path: &Path, side: Side) -> Result<()> {
        let path_str = path
            .to_str()
            .context("Convention file path is not valid UTF-8")?
            .to_string();

        if !path.exists() {
            bail!("Convention file not found: {}", path_str);
        }

        match side {
            Side::NorthSouth => self.ns_conventions = Some(path_str),
            Side::EastWest => self.ew_conventions = Some(path_str),
        }

        Ok(())
    }

    /// Get EPBot version string
    pub fn version(&self) -> Option<String> {
        unsafe {
            let ptr = ffi::epbot_get_version();
            if ptr.is_null() {
                return None;
            }
            CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_string())
        }
    }

    /// Generate auctions for multiple deals
    pub fn generate_auctions(&self, deals: Vec<DealSpec>) -> Result<Vec<DealResult>> {
        let mut results = Vec::with_capacity(deals.len());

        for deal in &deals {
            let result = self.process_one_deal(deal);
            results.push(result);
        }

        Ok(results)
    }

    /// Generate auction for a single deal
    pub fn generate_auction(&self, deal: DealSpec) -> Result<Vec<String>> {
        let result = self.process_one_deal(&deal);
        if result.success {
            result
                .auction
                .ok_or_else(|| anyhow::anyhow!("No auction in result"))
        } else {
            bail!(
                "EPBot error: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            )
        }
    }

    fn process_one_deal(&self, deal: &DealSpec) -> DealResult {
        let inst = unsafe { ffi::epbot_create() };
        if inst.is_null() {
            return DealResult {
                deal: Some(deal.pbn.clone()),
                auction: None,
                success: false,
                error: Some("Failed to create EPBot instance".to_string()),
            };
        }

        let result = self.run_deal(inst, deal);

        unsafe { ffi::epbot_destroy(inst) };

        result
    }

    fn run_deal(&self, inst: *mut c_void, deal: &DealSpec) -> DealResult {
        // Set deal
        let pbn_c = match CString::new(deal.pbn.as_str()) {
            Ok(c) => c,
            Err(e) => {
                return DealResult {
                    deal: Some(deal.pbn.clone()),
                    auction: None,
                    success: false,
                    error: Some(format!("Invalid PBN string: {}", e)),
                }
            }
        };

        let rc = unsafe { ffi::epbot_set_deal(inst, pbn_c.as_ptr()) };
        if rc != OK {
            return error_result(&deal.pbn, inst, "set_deal", rc);
        }

        // Set dealer
        let dealer_int = match deal.dealer {
            Position::North => 0,
            Position::East => 1,
            Position::South => 2,
            Position::West => 3,
        };
        let rc = unsafe { ffi::epbot_set_dealer(inst, dealer_int) };
        if rc != OK {
            return error_result(&deal.pbn, inst, "set_dealer", rc);
        }

        // Set vulnerability
        let rc = unsafe { ffi::epbot_set_vulnerability(inst, deal.vulnerability.to_ffi()) };
        if rc != OK {
            return error_result(&deal.pbn, inst, "set_vulnerability", rc);
        }

        // Load conventions
        if let Some(ref path) = self.ns_conventions {
            if let Ok(path_c) = CString::new(path.as_str()) {
                let rc = unsafe { ffi::epbot_load_conventions(inst, path_c.as_ptr(), 0) };
                if rc != OK {
                    return error_result(&deal.pbn, inst, "load_conventions(NS)", rc);
                }
            }
        }
        if let Some(ref path) = self.ew_conventions {
            if let Ok(path_c) = CString::new(path.as_str()) {
                let rc = unsafe { ffi::epbot_load_conventions(inst, path_c.as_ptr(), 1) };
                if rc != OK {
                    return error_result(&deal.pbn, inst, "load_conventions(EW)", rc);
                }
            }
        }

        // Run auction
        let mut bids = Vec::new();
        let mut bid_buf = [0i8; 32];

        for _ in 0..100 {
            let rc = unsafe {
                ffi::epbot_get_next_bid(inst, bid_buf.as_mut_ptr(), bid_buf.len() as i32)
            };

            if rc == ERR_AUCTION_COMPLETE {
                break;
            }
            if rc != OK {
                return error_result(&deal.pbn, inst, "get_next_bid", rc);
            }

            let bid_str = unsafe {
                CStr::from_ptr(bid_buf.as_ptr())
                    .to_str()
                    .unwrap_or("?")
                    .to_string()
            };
            bids.push(bid_str);

            // Check if auction is complete
            let mut is_done: u8 = 0;
            let rc = unsafe { ffi::epbot_is_auction_complete(inst, &mut is_done) };
            if rc != OK {
                return error_result(&deal.pbn, inst, "is_auction_complete", rc);
            }
            if is_done != 0 {
                break;
            }
        }

        DealResult {
            deal: Some(deal.pbn.clone()),
            auction: Some(bids),
            success: true,
            error: None,
        }
    }
}

fn error_result(pbn: &str, _inst: *mut c_void, op: &str, code: i32) -> DealResult {
    let err_msg = unsafe {
        let ptr = ffi::epbot_get_last_error();
        if ptr.is_null() {
            format!("{} failed with code {}", op, code)
        } else {
            let msg = CStr::from_ptr(ptr)
                .to_str()
                .unwrap_or("?")
                .to_string();
            format!("{} failed (code {}): {}", op, code, msg)
        }
    };

    DealResult {
        deal: Some(pbn.to_string()),
        auction: None,
        success: false,
        error: Some(err_msg),
    }
}

fn position_to_string(pos: Position) -> &'static str {
    match pos {
        Position::North => "N",
        Position::East => "E",
        Position::South => "S",
        Position::West => "W",
    }
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
    }

    #[test]
    fn test_vulnerability_ffi() {
        assert_eq!(Vulnerability::None.to_ffi(), 0);
        assert_eq!(Vulnerability::NorthSouth.to_ffi(), 1);
        assert_eq!(Vulnerability::EastWest.to_ffi(), 2);
        assert_eq!(Vulnerability::Both.to_ffi(), 3);
    }
}
