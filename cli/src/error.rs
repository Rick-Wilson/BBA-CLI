//! Error types for BBA-CLI

use thiserror::Error;

/// Main error type for BBA-CLI operations
#[derive(Error, Debug)]
pub enum BbaError {
    /// Failed to create EPBot engine instance
    #[error("Failed to create EPBot engine: {0}")]
    EngineCreationFailed(String),

    /// EPBot engine returned an error
    #[error("EPBot error (code {0}): {1}")]
    EngineError(i32, String),

    /// Invalid PBN file format
    #[error("Invalid PBN format: {0}")]
    InvalidPbn(String),

    /// Invalid convention file
    #[error("Invalid convention file: {0}")]
    InvalidConventionFile(String),

    /// File I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// String conversion error (null bytes in strings)
    #[error("String conversion error: {0}")]
    StringError(#[from] std::ffi::NulError),

    /// Invalid file path (non-UTF8 or other issues)
    #[error("Invalid file path")]
    InvalidPath,

    /// Auction generation exceeded maximum length
    #[error("Auction too long (exceeded {0} bids)")]
    AuctionTooLong(usize),

    /// Auction is already complete
    #[error("Auction is complete")]
    AuctionComplete,

    /// Buffer too small for output
    #[error("Buffer too small")]
    BufferTooSmall,

    /// FFI error - wrapper DLL not found or failed to load
    #[error("FFI error: {0}")]
    FfiError(String),
}

/// Result type alias for BBA operations
pub type BbaResult<T> = Result<T, BbaError>;
