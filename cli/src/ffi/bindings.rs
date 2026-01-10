//! Raw FFI bindings to the EPBot C wrapper
//!
//! These bindings match the C API defined in epbot_ffi.h

use std::os::raw::{c_char, c_int};

/// Opaque handle to EPBot instance
#[repr(C)]
pub struct EPBotHandle {
    _private: [u8; 0],
}

/// Pointer to EPBot instance (opaque handle)
pub type EPBotInstance = *mut EPBotHandle;

/// Error codes returned by EPBot wrapper functions
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EPBotError {
    Ok = 0,
    ErrNullHandle = -1,
    ErrInvalidHand = -2,
    ErrInvalidDealer = -3,
    ErrInvalidVulnerability = -4,
    ErrInvalidConventionFile = -5,
    ErrBiddingFailed = -6,
    ErrClrException = -7,
    ErrOutOfMemory = -8,
    ErrAuctionComplete = -9,
}

impl EPBotError {
    /// Check if this error code represents success
    pub fn is_ok(self) -> bool {
        self == EPBotError::Ok
    }

    /// Convert to a human-readable description
    pub fn description(self) -> &'static str {
        match self {
            EPBotError::Ok => "Success",
            EPBotError::ErrNullHandle => "Null instance handle",
            EPBotError::ErrInvalidHand => "Invalid hand/deal format",
            EPBotError::ErrInvalidDealer => "Invalid dealer position",
            EPBotError::ErrInvalidVulnerability => "Invalid vulnerability",
            EPBotError::ErrInvalidConventionFile => "Invalid convention file",
            EPBotError::ErrBiddingFailed => "Bidding operation failed",
            EPBotError::ErrClrException => "CLR/.NET exception",
            EPBotError::ErrOutOfMemory => "Buffer too small or out of memory",
            EPBotError::ErrAuctionComplete => "Auction is complete",
        }
    }
}

/// Dealer positions matching PBN standard
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EPBotDealer {
    North = 0,
    East = 1,
    South = 2,
    West = 3,
}

/// Vulnerability settings
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EPBotVulnerability {
    None = 0,
    NS = 1,
    EW = 2,
    Both = 3,
}

/// Partnership side for convention configuration
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EPBotSide {
    NS = 0,
    EW = 1,
}

// External functions from EPBotWrapper.dll
#[link(name = "EPBotWrapper")]
extern "C" {
    // Instance management
    pub fn epbot_create() -> EPBotInstance;
    pub fn epbot_destroy(instance: EPBotInstance);
    pub fn epbot_get_last_error() -> *const c_char;
    pub fn epbot_get_version() -> *const c_char;

    // Hand setup
    pub fn epbot_set_deal(instance: EPBotInstance, deal_pbn: *const c_char) -> EPBotError;
    pub fn epbot_set_dealer(instance: EPBotInstance, dealer: EPBotDealer) -> EPBotError;
    pub fn epbot_set_vulnerability(
        instance: EPBotInstance,
        vul: EPBotVulnerability,
    ) -> EPBotError;

    // Bidding
    pub fn epbot_get_next_bid(
        instance: EPBotInstance,
        buffer: *mut c_char,
        buffer_size: c_int,
    ) -> EPBotError;
    pub fn epbot_set_bid(
        instance: EPBotInstance,
        bid_index: c_int,
        bid: *const c_char,
    ) -> EPBotError;
    pub fn epbot_get_bid(
        instance: EPBotInstance,
        bid_index: c_int,
        buffer: *mut c_char,
        buffer_size: c_int,
    ) -> EPBotError;
    pub fn epbot_get_bid_count(instance: EPBotInstance, count: *mut c_int) -> EPBotError;
    pub fn epbot_clear_auction(instance: EPBotInstance) -> EPBotError;
    pub fn epbot_is_auction_complete(instance: EPBotInstance, is_complete: *mut bool)
        -> EPBotError;

    // Convention configuration
    pub fn epbot_load_conventions(
        instance: EPBotInstance,
        file_path: *const c_char,
        side: EPBotSide,
    ) -> EPBotError;
    pub fn epbot_set_convention(
        instance: EPBotInstance,
        key: *const c_char,
        value: c_int,
        side: EPBotSide,
    ) -> EPBotError;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_descriptions() {
        assert_eq!(EPBotError::Ok.description(), "Success");
        assert!(EPBotError::Ok.is_ok());
        assert!(!EPBotError::ErrNullHandle.is_ok());
    }

    #[test]
    fn test_enum_sizes() {
        // Ensure enums are the expected size for FFI
        assert_eq!(std::mem::size_of::<EPBotError>(), std::mem::size_of::<c_int>());
        assert_eq!(std::mem::size_of::<EPBotDealer>(), std::mem::size_of::<c_int>());
        assert_eq!(
            std::mem::size_of::<EPBotVulnerability>(),
            std::mem::size_of::<c_int>()
        );
        assert_eq!(std::mem::size_of::<EPBotSide>(), std::mem::size_of::<c_int>());
    }
}
