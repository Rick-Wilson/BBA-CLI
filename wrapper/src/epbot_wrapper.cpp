/**
 * EPBot C++/CLI Wrapper
 *
 * This file implements the C API defined in epbot_ffi.h by bridging to the
 * EPBot .NET assembly using C++/CLI managed/unmanaged interop.
 *
 * Compile with: /clr /EHa
 * Reference: EPBot64.dll (or EPBot86.dll for 32-bit)
 */

#include "../include/epbot_ffi.h"

#include <string>
#include <fstream>
#include <sstream>
#include <unordered_map>
#include <vcclr.h>

// Reference the EPBot .NET assembly
#using <EPBot64.dll>

using namespace System;
using namespace System::Runtime::InteropServices;

// Thread-local storage for last error message
thread_local std::string g_last_error;

// Internal wrapper structure holding managed EPBot instance
struct EPBotHandle {
    gcroot<EPBot64::EPBot^> instance;
    std::vector<std::string> auction;  // Recorded bids
    int current_position;              // Current bidder (0=dealer, increments)
    EPBotDealer dealer;
    bool auction_started;
};

/* ============================================================
 * Helper Functions
 * ============================================================ */

// Convert System::String to std::string
static std::string ManagedToStd(String^ managed) {
    if (managed == nullptr) return "";
    IntPtr ptr = Marshal::StringToHGlobalAnsi(managed);
    std::string result(static_cast<const char*>(ptr.ToPointer()));
    Marshal::FreeHGlobal(ptr);
    return result;
}

// Convert std::string/const char* to System::String
static String^ StdToManaged(const char* str) {
    if (str == nullptr) return nullptr;
    return gcnew String(str);
}

// Set error message
static void SetError(const char* msg) {
    g_last_error = msg ? msg : "Unknown error";
}

static void SetError(String^ msg) {
    g_last_error = ManagedToStd(msg);
}

// Parse a .bbsa convention file
static std::unordered_map<std::string, int> ParseBbsaFile(const char* path) {
    std::unordered_map<std::string, int> conventions;
    std::ifstream file(path);

    if (!file.is_open()) {
        return conventions;
    }

    std::string line;
    while (std::getline(file, line)) {
        // Skip empty lines and comments
        if (line.empty() || line[0] == '#' || line[0] == ';') {
            continue;
        }

        // Remove carriage return if present (Windows line endings)
        if (!line.empty() && line.back() == '\r') {
            line.pop_back();
        }

        // Parse key = value
        auto eq_pos = line.find('=');
        if (eq_pos != std::string::npos) {
            std::string key = line.substr(0, eq_pos);
            std::string value_str = line.substr(eq_pos + 1);

            // Trim whitespace from key
            size_t start = key.find_first_not_of(" \t");
            size_t end = key.find_last_not_of(" \t");
            if (start != std::string::npos && end != std::string::npos) {
                key = key.substr(start, end - start + 1);
            }

            // Trim whitespace from value
            start = value_str.find_first_not_of(" \t");
            end = value_str.find_last_not_of(" \t\r\n");
            if (start != std::string::npos && end != std::string::npos) {
                value_str = value_str.substr(start, end - start + 1);
            }

            try {
                int value = std::stoi(value_str);
                conventions[key] = value;
            } catch (...) {
                // Skip invalid values
            }
        }
    }

    return conventions;
}

// Get position index (0-3) for current bidder
static int GetCurrentBidderPosition(EPBotHandle* handle) {
    // Dealer is position 0 in the auction sequence
    // Current position cycles: dealer -> next -> partner of dealer -> partner of next
    return (static_cast<int>(handle->dealer) + handle->current_position) % 4;
}

// Check if auction is complete (3 consecutive passes after at least one bid)
static bool IsAuctionComplete(const std::vector<std::string>& auction) {
    if (auction.size() < 4) return false;

    // Need at least one non-pass bid
    bool has_bid = false;
    for (const auto& bid : auction) {
        if (bid != "Pass" && bid != "P") {
            has_bid = true;
            break;
        }
    }
    if (!has_bid) {
        // All passes - auction complete if 4 passes
        return auction.size() >= 4;
    }

    // Check for 3 consecutive passes at the end
    if (auction.size() >= 3) {
        const auto& b1 = auction[auction.size() - 1];
        const auto& b2 = auction[auction.size() - 2];
        const auto& b3 = auction[auction.size() - 3];
        if ((b1 == "Pass" || b1 == "P") &&
            (b2 == "Pass" || b2 == "P") &&
            (b3 == "Pass" || b3 == "P")) {
            return true;
        }
    }

    return false;
}

/* ============================================================
 * Instance Management
 * ============================================================ */

EPBOT_API EPBotInstance epbot_create(void) {
    try {
        EPBotHandle* handle = new EPBotHandle();
        handle->instance = gcnew EPBot64::EPBot();
        handle->current_position = 0;
        handle->dealer = DEALER_NORTH;
        handle->auction_started = false;
        return handle;
    } catch (Exception^ ex) {
        SetError(ex->Message);
        return nullptr;
    } catch (...) {
        SetError("Failed to create EPBot instance");
        return nullptr;
    }
}

EPBOT_API void epbot_destroy(EPBotInstance instance) {
    if (instance != nullptr) {
        EPBotHandle* handle = static_cast<EPBotHandle*>(instance);
        delete handle;
    }
}

EPBOT_API const char* epbot_get_last_error(void) {
    return g_last_error.c_str();
}

EPBOT_API const char* epbot_get_version(void) {
    // Version from DLL metadata (8736)
    static std::string version = "8736";
    return version.c_str();
}

/* ============================================================
 * Hand Setup
 * ============================================================ */

EPBOT_API EPBotError epbot_set_deal(EPBotInstance instance, const char* deal_pbn) {
    if (instance == nullptr) {
        SetError("Null instance handle");
        return EPBOT_ERR_NULL_HANDLE;
    }
    if (deal_pbn == nullptr) {
        SetError("Null deal string");
        return EPBOT_ERR_INVALID_HAND;
    }

    try {
        EPBotHandle* handle = static_cast<EPBotHandle*>(instance);

        // EPBot expects the deal in a specific format
        // We need to parse the PBN deal and set each hand
        // PBN format: "N:S.H.D.C S.H.D.C S.H.D.C S.H.D.C"
        // where hands are listed clockwise from the specified seat

        String^ pbn = StdToManaged(deal_pbn);

        // The EPBot might have a HAND property or similar
        // Based on DLL analysis, try set_HAND
        handle->instance->HAND = pbn;

        // Clear any existing auction when setting a new deal
        handle->auction.clear();
        handle->current_position = 0;
        handle->auction_started = false;

        return EPBOT_OK;
    } catch (Exception^ ex) {
        SetError(ex->Message);
        return EPBOT_ERR_INVALID_HAND;
    }
}

EPBOT_API EPBotError epbot_set_dealer(EPBotInstance instance, EPBotDealer dealer) {
    if (instance == nullptr) {
        SetError("Null instance handle");
        return EPBOT_ERR_NULL_HANDLE;
    }
    if (dealer < DEALER_NORTH || dealer > DEALER_WEST) {
        SetError("Invalid dealer position");
        return EPBOT_ERR_INVALID_DEALER;
    }

    try {
        EPBotHandle* handle = static_cast<EPBotHandle*>(instance);
        handle->dealer = dealer;

        // Set dealer in EPBot
        // Based on DLL analysis, EPBot likely has a Dealer property
        handle->instance->Dealer = static_cast<int>(dealer);

        return EPBOT_OK;
    } catch (Exception^ ex) {
        SetError(ex->Message);
        return EPBOT_ERR_INVALID_DEALER;
    }
}

EPBOT_API EPBotError epbot_set_vulnerability(EPBotInstance instance, EPBotVulnerability vul) {
    if (instance == nullptr) {
        SetError("Null instance handle");
        return EPBOT_ERR_NULL_HANDLE;
    }
    if (vul < VUL_NONE || vul > VUL_BOTH) {
        SetError("Invalid vulnerability");
        return EPBOT_ERR_INVALID_VULNERABILITY;
    }

    try {
        EPBotHandle* handle = static_cast<EPBotHandle*>(instance);

        // Set vulnerability in EPBot
        handle->instance->Vulnerability = static_cast<int>(vul);

        return EPBOT_OK;
    } catch (Exception^ ex) {
        SetError(ex->Message);
        return EPBOT_ERR_INVALID_VULNERABILITY;
    }
}

/* ============================================================
 * Bidding
 * ============================================================ */

EPBOT_API EPBotError epbot_get_next_bid(EPBotInstance instance, char* buffer, int32_t buffer_size) {
    if (instance == nullptr) {
        SetError("Null instance handle");
        return EPBOT_ERR_NULL_HANDLE;
    }
    if (buffer == nullptr || buffer_size < 1) {
        SetError("Invalid buffer");
        return EPBOT_ERR_OUT_OF_MEMORY;
    }

    try {
        EPBotHandle* handle = static_cast<EPBotHandle*>(instance);

        // Check if auction is already complete
        if (IsAuctionComplete(handle->auction)) {
            SetError("Auction is complete");
            return EPBOT_ERR_AUCTION_COMPLETE;
        }

        // Set the current position for EPBot
        int bidder_position = GetCurrentBidderPosition(handle);
        handle->instance->Position = bidder_position;

        // Get bid recommendation from EPBot
        // The DLL analysis showed a 'bid' property - this likely gets the recommended bid
        String^ bid_managed = handle->instance->bid;
        std::string bid = ManagedToStd(bid_managed);

        // Normalize bid format
        if (bid.empty()) {
            bid = "Pass";
        }

        // Record the bid
        handle->auction.push_back(bid);
        handle->current_position++;
        handle->auction_started = true;

        // Copy to output buffer
        if (static_cast<int>(bid.length()) >= buffer_size) {
            SetError("Buffer too small");
            return EPBOT_ERR_OUT_OF_MEMORY;
        }

        strncpy(buffer, bid.c_str(), buffer_size - 1);
        buffer[buffer_size - 1] = '\0';

        return EPBOT_OK;
    } catch (Exception^ ex) {
        SetError(ex->Message);
        return EPBOT_ERR_BIDDING_FAILED;
    }
}

EPBOT_API EPBotError epbot_set_bid(EPBotInstance instance, int32_t bid_index, const char* bid) {
    if (instance == nullptr) {
        SetError("Null instance handle");
        return EPBOT_ERR_NULL_HANDLE;
    }
    if (bid == nullptr) {
        SetError("Null bid string");
        return EPBOT_ERR_BIDDING_FAILED;
    }

    try {
        EPBotHandle* handle = static_cast<EPBotHandle*>(instance);

        // Extend auction vector if needed
        while (static_cast<int>(handle->auction.size()) <= bid_index) {
            handle->auction.push_back("");
        }

        handle->auction[bid_index] = bid;

        // Update EPBot's internal state
        // The DLL has set_bid method
        String^ bid_managed = StdToManaged(bid);
        handle->instance->bid = bid_managed;

        return EPBOT_OK;
    } catch (Exception^ ex) {
        SetError(ex->Message);
        return EPBOT_ERR_BIDDING_FAILED;
    }
}

EPBOT_API EPBotError epbot_get_bid(EPBotInstance instance, int32_t bid_index,
                                    char* buffer, int32_t buffer_size) {
    if (instance == nullptr) {
        SetError("Null instance handle");
        return EPBOT_ERR_NULL_HANDLE;
    }
    if (buffer == nullptr || buffer_size < 1) {
        SetError("Invalid buffer");
        return EPBOT_ERR_OUT_OF_MEMORY;
    }

    try {
        EPBotHandle* handle = static_cast<EPBotHandle*>(instance);

        if (bid_index < 0 || bid_index >= static_cast<int>(handle->auction.size())) {
            SetError("Bid index out of range");
            return EPBOT_ERR_BIDDING_FAILED;
        }

        const std::string& bid = handle->auction[bid_index];

        if (static_cast<int>(bid.length()) >= buffer_size) {
            SetError("Buffer too small");
            return EPBOT_ERR_OUT_OF_MEMORY;
        }

        strncpy(buffer, bid.c_str(), buffer_size - 1);
        buffer[buffer_size - 1] = '\0';

        return EPBOT_OK;
    } catch (Exception^ ex) {
        SetError(ex->Message);
        return EPBOT_ERR_BIDDING_FAILED;
    }
}

EPBOT_API EPBotError epbot_get_bid_count(EPBotInstance instance, int32_t* count) {
    if (instance == nullptr) {
        SetError("Null instance handle");
        return EPBOT_ERR_NULL_HANDLE;
    }
    if (count == nullptr) {
        SetError("Null count pointer");
        return EPBOT_ERR_BIDDING_FAILED;
    }

    EPBotHandle* handle = static_cast<EPBotHandle*>(instance);
    *count = static_cast<int32_t>(handle->auction.size());

    return EPBOT_OK;
}

EPBOT_API EPBotError epbot_clear_auction(EPBotInstance instance) {
    if (instance == nullptr) {
        SetError("Null instance handle");
        return EPBOT_ERR_NULL_HANDLE;
    }

    try {
        EPBotHandle* handle = static_cast<EPBotHandle*>(instance);
        handle->auction.clear();
        handle->current_position = 0;
        handle->auction_started = false;

        return EPBOT_OK;
    } catch (Exception^ ex) {
        SetError(ex->Message);
        return EPBOT_ERR_CLR_EXCEPTION;
    }
}

EPBOT_API EPBotError epbot_is_auction_complete(EPBotInstance instance, bool* is_complete) {
    if (instance == nullptr) {
        SetError("Null instance handle");
        return EPBOT_ERR_NULL_HANDLE;
    }
    if (is_complete == nullptr) {
        SetError("Null output pointer");
        return EPBOT_ERR_BIDDING_FAILED;
    }

    EPBotHandle* handle = static_cast<EPBotHandle*>(instance);
    *is_complete = IsAuctionComplete(handle->auction);

    return EPBOT_OK;
}

/* ============================================================
 * Convention Configuration
 * ============================================================ */

EPBOT_API EPBotError epbot_load_conventions(EPBotInstance instance,
                                             const char* file_path,
                                             EPBotSide side) {
    if (instance == nullptr) {
        SetError("Null instance handle");
        return EPBOT_ERR_NULL_HANDLE;
    }
    if (file_path == nullptr) {
        SetError("Null file path");
        return EPBOT_ERR_INVALID_CONVENTION_FILE;
    }

    try {
        EPBotHandle* handle = static_cast<EPBotHandle*>(instance);

        // Parse the .bbsa file
        auto conventions = ParseBbsaFile(file_path);

        if (conventions.empty()) {
            SetError("Failed to parse convention file or file is empty");
            return EPBOT_ERR_INVALID_CONVENTION_FILE;
        }

        // Apply conventions to EPBot
        // The DLL has many setter methods for conventions
        // We need to map the .bbsa keys to EPBot properties

        // Common convention mappings (based on DLL string analysis)
        for (const auto& [key, value] : conventions) {
            try {
                // EPBot has properties like Bergen, Stayman, etc.
                // We use reflection-like approach or direct property setting
                epbot_set_convention(instance, key.c_str(), value, side);
            } catch (...) {
                // Skip unknown conventions
            }
        }

        return EPBOT_OK;
    } catch (Exception^ ex) {
        SetError(ex->Message);
        return EPBOT_ERR_INVALID_CONVENTION_FILE;
    }
}

EPBOT_API EPBotError epbot_set_convention(EPBotInstance instance,
                                           const char* key,
                                           int32_t value,
                                           EPBotSide side) {
    if (instance == nullptr) {
        SetError("Null instance handle");
        return EPBOT_ERR_NULL_HANDLE;
    }
    if (key == nullptr) {
        SetError("Null convention key");
        return EPBOT_ERR_CLR_EXCEPTION;
    }

    try {
        EPBotHandle* handle = static_cast<EPBotHandle*>(instance);
        EPBot64::EPBot^ bot = handle->instance;

        std::string keyStr(key);

        // Map common convention names to EPBot properties
        // This is a subset - the full DLL has 150+ convention setters

        // Note: The actual property names depend on the EPBot .NET API
        // These are educated guesses based on DLL string analysis
        // May need adjustment after testing with actual DLL

        if (keyStr == "Bergen" || keyStr == "Bergen raises") {
            bot->Bergen = value;
        }
        else if (keyStr == "Stayman") {
            bot->Stayman = value;
        }
        else if (keyStr == "Blackwood 0314") {
            bot->Blackwood_0314 = value;
        }
        else if (keyStr == "Blackwood 1430") {
            bot->Blackwood_1430 = value;
        }
        else if (keyStr == "Jacoby 2NT") {
            bot->Jacoby_2NT = value;
        }
        else if (keyStr == "Cappelletti") {
            bot->Cappelletti = value;
        }
        else if (keyStr == "Drury") {
            bot->Drury = value;
        }
        else if (keyStr == "Lebensohl") {
            bot->Lebensohl = value;
        }
        else if (keyStr == "Michaels Cuebid") {
            bot->Michaels_Cuebid = value;
        }
        else if (keyStr == "Splinter") {
            bot->Splinter = value;
        }
        else if (keyStr == "Texas Transfer") {
            bot->Texas_Transfer = value;
        }
        else if (keyStr == "Unusual 2NT") {
            bot->Unusual_2NT = value;
        }
        // Add more convention mappings as needed

        // For unknown conventions, we could use reflection
        // but that's more complex in C++/CLI

        return EPBOT_OK;
    } catch (Exception^ ex) {
        SetError(ex->Message);
        return EPBOT_ERR_CLR_EXCEPTION;
    }
}
