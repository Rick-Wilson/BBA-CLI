//! BBA-CLI Library
//!
//! This library provides the core functionality for the Bridge Bidding Analyzer CLI.
//! It interfaces with the EPBot .NET DLL through a C# wrapper to generate
//! bridge auctions for deals in PBN files.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
//! │   Rust CLI      │────▶│  C# Wrapper      │────▶│  EPBot64.dll    │
//! │   (bba-cli)     │JSON │ (epbot-wrapper)  │ CLR │  (.NET 4.8)     │
//! └─────────────────┘     └──────────────────┘     └─────────────────┘
//!         │
//!         ▼
//! ┌─────────────────┐
//! │  dealer-core    │  (Bridge types: Card, Hand, Deal, Position)
//! │  dealer-pbn     │  (PBN parsing/formatting)
//! └─────────────────┘
//! ```
//!
//! # Modules
//!
//! - [`epbot`] - Wrapper around the C# EPBot bridge
//! - [`batch`] - Batch processing for PBN files
//! - [`error`] - Error types

pub mod batch;
pub mod epbot;
pub mod error;

pub use batch::process_pbn_file;
pub use epbot::{DealResult, DealSpec, EPBot, Side, Vulnerability};
pub use error::{BbaError, BbaResult};
