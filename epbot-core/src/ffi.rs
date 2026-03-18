//! Raw FFI bindings to Edward Piwowar's native EPBot library.
//!
//! These match the C API exported by the NativeAOT-compiled EPBot library
//! (libEPBot.dylib / libEPBot.so / EPBot.dll), which wraps the EPBotFFI.cs
//! entry points.
//!
//! Memory convention:
//! - String return values are written to caller-provided buffers (ptr + size).
//! - Array return values are written to caller-provided buffers with a count out-param.
//! - Instance handles are opaque pointers (IntPtr to a managed GCHandle).
//! - Return value: 0 = success, negative = error code.

#![allow(dead_code)]

use std::os::raw::{c_char, c_int, c_uchar, c_void};

/// Success
pub const OK: c_int = 0;
/// Null instance handle passed
pub const ERR_NULL_HANDLE: c_int = -1;
/// Exception thrown inside EPBot
pub const ERR_EXCEPTION: c_int = -2;
/// Caller-provided buffer too small
pub const ERR_BUFFER_TOO_SMALL: c_int = -3;

// Platform-specific library name for the linker.
// On macOS: libEPBot.dylib, on Linux: libEPBot.so, on Windows: EPBot.dll
#[cfg_attr(target_os = "macos", link(name = "EPBot"))]
#[cfg_attr(target_os = "linux", link(name = "EPBot"))]
#[cfg_attr(target_os = "windows", link(name = "EPBot"))]
extern "C" {
    // ========================================================================
    // Instance lifecycle
    // ========================================================================

    /// Create a new EPBot instance. Returns opaque handle, or null on failure.
    pub fn epbot_create() -> *mut c_void;

    /// Destroy an EPBot instance and free its resources.
    pub fn epbot_destroy(instance: *mut c_void);

    /// Get the last error message. Returns pointer to UTF-8 string, or null.
    pub fn epbot_get_last_error() -> *const c_char;

    // ========================================================================
    // Core bidding
    // ========================================================================

    /// Initialize a hand for a player.
    /// `longer_ptr`: newline-separated suit strings (C.D.H.S order, 4 suits).
    pub fn epbot_new_hand(
        instance: *mut c_void,
        player_position: c_int,
        longer_ptr: *const c_char,
        dealer: c_int,
        vulnerability: c_int,
        repeating: c_int,
        b_playing: c_int,
    ) -> c_int;

    /// Get the next bid for this player. Returns bid code.
    pub fn epbot_get_bid(instance: *mut c_void) -> c_int;

    /// Broadcast a bid to this player instance.
    /// `spare`: position of bidder, `new_value`: bid code, `str_alert_ptr`: alert string.
    pub fn epbot_set_bid(
        instance: *mut c_void,
        spare: c_int,
        new_value: c_int,
        str_alert_ptr: *const c_char,
    ) -> c_int;

    /// Set the array of bids (newline-separated string).
    pub fn epbot_set_arr_bids(instance: *mut c_void, bids_ptr: *const c_char) -> c_int;

    /// Interpret a bid code (updates internal state).
    pub fn epbot_interpret_bid(instance: *mut c_void, bid_code: c_int) -> c_int;

    /// Ask for bid (alternative to get_bid). Returns bid code.
    pub fn epbot_ask(instance: *mut c_void) -> c_int;

    // ========================================================================
    // Conventions
    // ========================================================================

    /// Get a convention value. Returns 1 (true) or 0 (false).
    pub fn epbot_get_conventions(
        instance: *mut c_void,
        site: c_int,
        convention_ptr: *const c_char,
    ) -> c_int;

    /// Set a convention value.
    pub fn epbot_set_conventions(
        instance: *mut c_void,
        site: c_int,
        convention_ptr: *const c_char,
        value: c_int,
    ) -> c_int;

    /// Get the system type for a side.
    pub fn epbot_get_system_type(instance: *mut c_void, system_number: c_int) -> c_int;

    /// Set the system type for a side.
    pub fn epbot_set_system_type(
        instance: *mut c_void,
        system_number: c_int,
        value: c_int,
    ) -> c_int;

    /// Get the opponent type for a side.
    pub fn epbot_get_opponent_type(instance: *mut c_void, system_number: c_int) -> c_int;

    /// Set the opponent type for a side.
    pub fn epbot_set_opponent_type(
        instance: *mut c_void,
        system_number: c_int,
        value: c_int,
    ) -> c_int;

    /// Get the index of a convention by name.
    pub fn epbot_convention_index(instance: *mut c_void, name_ptr: *const c_char) -> c_int;

    /// Get convention name by index (into buffer).
    pub fn epbot_convention_name(
        instance: *mut c_void,
        index: c_int,
        buffer: *mut c_char,
        buffer_size: c_int,
    ) -> c_int;

    /// Get convention display name by index (into buffer).
    pub fn epbot_get_convention_name(
        instance: *mut c_void,
        index: c_int,
        buffer: *mut c_char,
        buffer_size: c_int,
    ) -> c_int;

    /// Get selected conventions (newline-separated into buffer).
    pub fn epbot_selected_conventions(
        instance: *mut c_void,
        buffer: *mut c_char,
        buffer_size: c_int,
        count_out: *mut c_int,
    ) -> c_int;

    /// Get system name by number (into buffer).
    pub fn epbot_system_name(
        instance: *mut c_void,
        system_number: c_int,
        buffer: *mut c_char,
        buffer_size: c_int,
    ) -> c_int;

    // ========================================================================
    // Scoring & settings
    // ========================================================================

    /// Get scoring mode (0 = MP, 1 = IMP).
    pub fn epbot_get_scoring(instance: *mut c_void) -> c_int;

    /// Set scoring mode.
    pub fn epbot_set_scoring(instance: *mut c_void, value: c_int) -> c_int;

    /// Get playing skills level.
    pub fn epbot_get_playing_skills(instance: *mut c_void) -> c_int;

    /// Set playing skills level.
    pub fn epbot_set_playing_skills(instance: *mut c_void, value: c_int) -> c_int;

    /// Get defensive skills level.
    pub fn epbot_get_defensive_skills(instance: *mut c_void) -> c_int;

    /// Set defensive skills level.
    pub fn epbot_set_defensive_skills(instance: *mut c_void, value: c_int) -> c_int;

    /// Get licence value.
    pub fn epbot_get_licence(instance: *mut c_void) -> c_int;

    /// Set licence value.
    pub fn epbot_set_licence(instance: *mut c_void, value: c_int) -> c_int;

    /// Get bcalconsole path (into buffer).
    pub fn epbot_get_bcalconsole_path(
        instance: *mut c_void,
        buffer: *mut c_char,
        buffer_size: c_int,
    ) -> c_int;

    /// Set bcalconsole path.
    pub fn epbot_set_bcalconsole_path(instance: *mut c_void, path_ptr: *const c_char) -> c_int;

    // ========================================================================
    // State queries
    // ========================================================================

    /// Get the player's position.
    pub fn epbot_get_position(instance: *mut c_void) -> c_int;

    /// Get the dealer position.
    pub fn epbot_get_dealer(instance: *mut c_void) -> c_int;

    /// Get the vulnerability setting.
    pub fn epbot_get_vulnerability(instance: *mut c_void) -> c_int;

    /// Get EPBot version number.
    pub fn epbot_version(instance: *mut c_void) -> c_int;

    /// Get copyright string (into buffer).
    pub fn epbot_copyright(
        instance: *mut c_void,
        buffer: *mut c_char,
        buffer_size: c_int,
    ) -> c_int;

    /// Get last EPBot-internal error (into buffer).
    pub fn epbot_get_last_epbot_error(
        instance: *mut c_void,
        buffer: *mut c_char,
        buffer_size: c_int,
    ) -> c_int;

    /// Get the bidding as a string (into buffer).
    pub fn epbot_get_str_bidding(
        instance: *mut c_void,
        buffer: *mut c_char,
        buffer_size: c_int,
    ) -> c_int;

    // ========================================================================
    // Analysis
    // ========================================================================

    /// Get probable level for a strain.
    pub fn epbot_get_probable_level(instance: *mut c_void, strain: c_int) -> c_int;

    /// Get probable levels array (into buffer).
    pub fn epbot_get_probable_levels(
        instance: *mut c_void,
        buffer: *mut c_int,
        buffer_size: c_int,
        count_out: *mut c_int,
    ) -> c_int;

    /// Get SD tricks analysis.
    pub fn epbot_get_sd_tricks(
        instance: *mut c_void,
        partner_longer_ptr: *const c_char,
        tricks_buffer: *mut c_int,
        tricks_buffer_size: c_int,
        tricks_count_out: *mut c_int,
        pct_buffer: *mut c_int,
        pct_buffer_size: c_int,
        pct_count_out: *mut c_int,
    ) -> c_int;

    // ========================================================================
    // Info / meaning (bid interpretation data)
    // ========================================================================

    /// Get bid meaning string for position k (into buffer).
    pub fn epbot_get_info_meaning(
        instance: *mut c_void,
        k: c_int,
        buffer: *mut c_char,
        buffer_size: c_int,
    ) -> c_int;

    /// Set bid meaning string for position k.
    pub fn epbot_set_info_meaning(
        instance: *mut c_void,
        k: c_int,
        value_ptr: *const c_char,
    ) -> c_int;

    /// Get extended bid meaning for position (into buffer).
    pub fn epbot_get_info_meaning_extended(
        instance: *mut c_void,
        position: c_int,
        buffer: *mut c_char,
        buffer_size: c_int,
    ) -> c_int;

    /// Set extended bid meaning for position.
    pub fn epbot_set_info_meaning_extended(
        instance: *mut c_void,
        position: c_int,
        value_ptr: *const c_char,
    ) -> c_int;

    /// Get info feature array for position (into buffer).
    pub fn epbot_get_info_feature(
        instance: *mut c_void,
        position: c_int,
        buffer: *mut c_int,
        buffer_size: c_int,
        count_out: *mut c_int,
    ) -> c_int;

    /// Set info feature array for position.
    pub fn epbot_set_info_feature(
        instance: *mut c_void,
        position: c_int,
        data_ptr: *const c_int,
        count: c_int,
    ) -> c_int;

    /// Get info min length array for position (into buffer).
    pub fn epbot_get_info_min_length(
        instance: *mut c_void,
        position: c_int,
        buffer: *mut c_int,
        buffer_size: c_int,
        count_out: *mut c_int,
    ) -> c_int;

    /// Set info min length array for position.
    pub fn epbot_set_info_min_length(
        instance: *mut c_void,
        position: c_int,
        data_ptr: *const c_int,
        count: c_int,
    ) -> c_int;

    /// Get info max length array for position (into buffer).
    pub fn epbot_get_info_max_length(
        instance: *mut c_void,
        position: c_int,
        buffer: *mut c_int,
        buffer_size: c_int,
        count_out: *mut c_int,
    ) -> c_int;

    /// Set info max length array for position.
    pub fn epbot_set_info_max_length(
        instance: *mut c_void,
        position: c_int,
        data_ptr: *const c_int,
        count: c_int,
    ) -> c_int;

    /// Get info probable length array for position (into buffer).
    pub fn epbot_get_info_probable_length(
        instance: *mut c_void,
        position: c_int,
        buffer: *mut c_int,
        buffer_size: c_int,
        count_out: *mut c_int,
    ) -> c_int;

    /// Set info probable length array for position.
    pub fn epbot_set_info_probable_length(
        instance: *mut c_void,
        position: c_int,
        data_ptr: *const c_int,
        count: c_int,
    ) -> c_int;

    /// Get info honors array for position (into buffer).
    pub fn epbot_get_info_honors(
        instance: *mut c_void,
        position: c_int,
        buffer: *mut c_int,
        buffer_size: c_int,
        count_out: *mut c_int,
    ) -> c_int;

    /// Set info honors array for position.
    pub fn epbot_set_info_honors(
        instance: *mut c_void,
        position: c_int,
        data_ptr: *const c_int,
        count: c_int,
    ) -> c_int;

    /// Get info suit power array for position (into buffer).
    pub fn epbot_get_info_suit_power(
        instance: *mut c_void,
        position: c_int,
        buffer: *mut c_int,
        buffer_size: c_int,
        count_out: *mut c_int,
    ) -> c_int;

    /// Set info suit power array for position.
    pub fn epbot_set_info_suit_power(
        instance: *mut c_void,
        position: c_int,
        data_ptr: *const c_int,
        count: c_int,
    ) -> c_int;

    /// Get info strength array for position (into buffer).
    pub fn epbot_get_info_strength(
        instance: *mut c_void,
        position: c_int,
        buffer: *mut c_int,
        buffer_size: c_int,
        count_out: *mut c_int,
    ) -> c_int;

    /// Set info strength array for position.
    pub fn epbot_set_info_strength(
        instance: *mut c_void,
        position: c_int,
        data_ptr: *const c_int,
        count: c_int,
    ) -> c_int;

    /// Get info stoppers array for position (into buffer).
    pub fn epbot_get_info_stoppers(
        instance: *mut c_void,
        position: c_int,
        buffer: *mut c_int,
        buffer_size: c_int,
        count_out: *mut c_int,
    ) -> c_int;

    /// Set info stoppers array for position.
    pub fn epbot_set_info_stoppers(
        instance: *mut c_void,
        position: c_int,
        data_ptr: *const c_int,
        count: c_int,
    ) -> c_int;

    /// Get info alerting for position k. Returns 1 (alert) or 0 (no alert).
    pub fn epbot_get_info_alerting(instance: *mut c_void, k: c_int) -> c_int;

    /// Set info alerting for position k.
    pub fn epbot_set_info_alerting(instance: *mut c_void, k: c_int, value: c_int) -> c_int;

    /// Get used conventions for item. Returns convention value.
    pub fn epbot_get_used_conventions(instance: *mut c_void, item: c_int) -> c_int;

    /// Set used conventions for item.
    pub fn epbot_set_used_conventions(instance: *mut c_void, item: c_int, value: c_int) -> c_int;

    // ========================================================================
    // Card play
    // ========================================================================

    /// Get opening lead (into buffer).
    pub fn epbot_get_lead(
        instance: *mut c_void,
        force_lead: c_int,
        buffer: *mut c_char,
        buffer_size: c_int,
    ) -> c_int;

    /// Set opening lead.
    pub fn epbot_set_lead(instance: *mut c_void, card_ptr: *const c_char) -> c_int;

    /// Set dummy information.
    pub fn epbot_set_dummy(
        instance: *mut c_void,
        dummy: c_int,
        cards_ptr: *const c_char,
        all_data: c_int,
        without_final_length_ptr: *mut c_uchar,
    ) -> c_int;

    /// Get cards string (into buffer).
    pub fn epbot_get_cards(
        instance: *mut c_void,
        buffer: *mut c_char,
        buffer_size: c_int,
    ) -> c_int;

    /// Get hand for player position (newline-separated suits into buffer).
    pub fn epbot_get_hand(
        instance: *mut c_void,
        player_position: c_int,
        buffer: *mut c_char,
        buffer_size: c_int,
        count_out: *mut c_int,
    ) -> c_int;

    /// Get arranged suits (newline-separated into buffer).
    pub fn epbot_get_arr_suits(
        instance: *mut c_void,
        current_longers: c_int,
        buffer: *mut c_char,
        buffer_size: c_int,
        count_out: *mut c_int,
    ) -> c_int;
}
