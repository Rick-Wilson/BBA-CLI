/**
 * EPBot FFI Header
 *
 * C API for interfacing with the EPBot .NET DLL from Rust.
 * This header defines the interface between the C++/CLI wrapper and Rust FFI.
 */

#ifndef EPBOT_FFI_H
#define EPBOT_FFI_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

#ifdef EPBOT_WRAPPER_EXPORTS
#define EPBOT_API __declspec(dllexport)
#else
#define EPBOT_API __declspec(dllimport)
#endif

/* Opaque handle to EPBot instance */
typedef struct EPBotHandle* EPBotInstance;

/* Error codes */
typedef enum EPBotError {
    EPBOT_OK = 0,
    EPBOT_ERR_NULL_HANDLE = -1,
    EPBOT_ERR_INVALID_HAND = -2,
    EPBOT_ERR_INVALID_DEALER = -3,
    EPBOT_ERR_INVALID_VULNERABILITY = -4,
    EPBOT_ERR_INVALID_CONVENTION_FILE = -5,
    EPBOT_ERR_BIDDING_FAILED = -6,
    EPBOT_ERR_CLR_EXCEPTION = -7,
    EPBOT_ERR_OUT_OF_MEMORY = -8,
    EPBOT_ERR_AUCTION_COMPLETE = -9,
} EPBotError;

/* Dealer positions (matches PBN standard) */
typedef enum EPBotDealer {
    DEALER_NORTH = 0,
    DEALER_EAST = 1,
    DEALER_SOUTH = 2,
    DEALER_WEST = 3,
} EPBotDealer;

/* Vulnerability settings */
typedef enum EPBotVulnerability {
    VUL_NONE = 0,
    VUL_NS = 1,
    VUL_EW = 2,
    VUL_BOTH = 3,
} EPBotVulnerability;

/* Partnership side for conventions */
typedef enum EPBotSide {
    SIDE_NS = 0,
    SIDE_EW = 1,
} EPBotSide;

/* ============================================================
 * Instance Management
 * ============================================================ */

/**
 * Create a new EPBot instance.
 * @return Handle to the instance, or NULL on failure.
 */
EPBOT_API EPBotInstance epbot_create(void);

/**
 * Destroy an EPBot instance and free resources.
 * @param instance Handle to destroy (safe to call with NULL).
 */
EPBOT_API void epbot_destroy(EPBotInstance instance);

/**
 * Get the last error message (thread-local).
 * @return Pointer to null-terminated string, valid until next API call.
 */
EPBOT_API const char* epbot_get_last_error(void);

/**
 * Get EPBot version string.
 * @return Version string (e.g., "8736").
 */
EPBOT_API const char* epbot_get_version(void);

/* ============================================================
 * Hand Setup
 * ============================================================ */

/**
 * Set the deal for analysis using PBN Deal format.
 * @param instance EPBot instance handle.
 * @param deal_pbn PBN Deal string, e.g., "N:AKQ2.K32.A54.K32 J653.A73.985.J97 ..."
 *                 Format: "FirstSeat:Hand0 Hand1 Hand2 Hand3" (clockwise from FirstSeat)
 * @return EPBOT_OK on success.
 */
EPBOT_API EPBotError epbot_set_deal(EPBotInstance instance, const char* deal_pbn);

/**
 * Set the dealer position.
 * @param instance EPBot instance handle.
 * @param dealer Dealer position (N=0, E=1, S=2, W=3).
 * @return EPBOT_OK on success.
 */
EPBOT_API EPBotError epbot_set_dealer(EPBotInstance instance, EPBotDealer dealer);

/**
 * Set vulnerability.
 * @param instance EPBot instance handle.
 * @param vul Vulnerability setting.
 * @return EPBOT_OK on success.
 */
EPBOT_API EPBotError epbot_set_vulnerability(EPBotInstance instance, EPBotVulnerability vul);

/* ============================================================
 * Bidding
 * ============================================================ */

/**
 * Get the next bid recommendation for the current position.
 * The auction position advances automatically based on bids made.
 * @param instance EPBot instance handle.
 * @param buffer Output buffer for the bid string (e.g., "1H", "Pass", "X", "XX", "2NT").
 * @param buffer_size Size of the output buffer (recommend at least 8 bytes).
 * @return EPBOT_OK on success, EPBOT_ERR_AUCTION_COMPLETE if auction is finished.
 */
EPBOT_API EPBotError epbot_get_next_bid(EPBotInstance instance, char* buffer, int32_t buffer_size);

/**
 * Record a bid in the auction at the specified index.
 * @param instance EPBot instance handle.
 * @param bid_index 0-based index (0 = dealer's first bid).
 * @param bid Bid string (e.g., "1H", "Pass", "X", "XX").
 * @return EPBOT_OK on success.
 */
EPBOT_API EPBotError epbot_set_bid(EPBotInstance instance, int32_t bid_index, const char* bid);

/**
 * Get a bid from the auction at the specified index.
 * @param instance EPBot instance handle.
 * @param bid_index 0-based index of the bid to retrieve.
 * @param buffer Output buffer for the bid string.
 * @param buffer_size Size of output buffer.
 * @return EPBOT_OK on success.
 */
EPBOT_API EPBotError epbot_get_bid(EPBotInstance instance, int32_t bid_index,
                                    char* buffer, int32_t buffer_size);

/**
 * Get the number of bids in the current auction.
 * @param instance EPBot instance handle.
 * @param count Output for bid count.
 * @return EPBOT_OK on success.
 */
EPBOT_API EPBotError epbot_get_bid_count(EPBotInstance instance, int32_t* count);

/**
 * Clear the current auction (reset bids but keep hand/conventions).
 * @param instance EPBot instance handle.
 * @return EPBOT_OK on success.
 */
EPBOT_API EPBotError epbot_clear_auction(EPBotInstance instance);

/**
 * Check if the auction is complete (ended with 3 passes after a bid).
 * @param instance EPBot instance handle.
 * @param is_complete Output: true if auction is complete.
 * @return EPBOT_OK on success.
 */
EPBOT_API EPBotError epbot_is_auction_complete(EPBotInstance instance, bool* is_complete);

/* ============================================================
 * Convention Configuration
 * ============================================================ */

/**
 * Load conventions from a .bbsa file.
 * @param instance EPBot instance handle.
 * @param file_path Path to the .bbsa convention file.
 * @param side Which partnership to configure (SIDE_NS or SIDE_EW).
 * @return EPBOT_OK on success, EPBOT_ERR_INVALID_CONVENTION_FILE on failure.
 */
EPBOT_API EPBotError epbot_load_conventions(EPBotInstance instance,
                                             const char* file_path,
                                             EPBotSide side);

/**
 * Set a single convention parameter.
 * @param instance EPBot instance handle.
 * @param key Convention parameter name (e.g., "Stayman", "Bergen").
 * @param value Integer value for the parameter (typically 0 or 1).
 * @param side Which partnership to configure.
 * @return EPBOT_OK on success.
 */
EPBOT_API EPBotError epbot_set_convention(EPBotInstance instance,
                                           const char* key,
                                           int32_t value,
                                           EPBotSide side);

#ifdef __cplusplus
}
#endif

#endif /* EPBOT_FFI_H */
