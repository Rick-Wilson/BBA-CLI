//! Raw FFI bindings to EPBotWrapper.dylib
//!
//! These match the C API exported by the NativeAOT-compiled EPBot library.

#![allow(dead_code)]

use std::os::raw::{c_char, c_int, c_void};

#[link(name = "EPBotWrapper")]
extern "C" {
    // Instance management
    pub fn epbot_create() -> *mut c_void;
    pub fn epbot_destroy(instance: *mut c_void);
    pub fn epbot_get_last_error() -> *const c_char;
    pub fn epbot_get_version() -> *const c_char;

    // Deal setup
    pub fn epbot_set_deal(instance: *mut c_void, deal_pbn: *const c_char) -> c_int;
    pub fn epbot_set_dealer(instance: *mut c_void, dealer: c_int) -> c_int;
    pub fn epbot_set_vulnerability(instance: *mut c_void, vul: c_int) -> c_int;

    // Bidding
    pub fn epbot_get_next_bid(
        instance: *mut c_void,
        buffer: *mut c_char,
        buffer_size: c_int,
    ) -> c_int;
    pub fn epbot_set_bid(
        instance: *mut c_void,
        position: c_int,
        bid: *const c_char,
    ) -> c_int;
    pub fn epbot_get_bid(
        instance: *mut c_void,
        bid_index: c_int,
        buffer: *mut c_char,
        buffer_size: c_int,
    ) -> c_int;
    pub fn epbot_get_bid_count(instance: *mut c_void, count: *mut c_int) -> c_int;
    pub fn epbot_clear_auction(instance: *mut c_void) -> c_int;
    pub fn epbot_is_auction_complete(instance: *mut c_void, is_complete: *mut u8) -> c_int;

    // Convention configuration
    pub fn epbot_load_conventions(
        instance: *mut c_void,
        file_path: *const c_char,
        side: c_int,
    ) -> c_int;
    pub fn epbot_set_convention(
        instance: *mut c_void,
        key: *const c_char,
        value: c_int,
        side: c_int,
    ) -> c_int;
}
