//! Rust package for contract bridge
//!
//! This package provides tools for analyzing and simulating hands in the card
//! game contract bridge.  It is named after [an anatomical part of the
//! brainstem][pons] and also "bridge" in Latin.
//!
//! [pons]: https://en.wikipedia.org/wiki/Pons
#![warn(missing_docs)]

/// Bidding in contract bridge
///
/// This module re-exports data structures from [`dds_bridge`] for
/// convenience.
pub mod bidding;

/// Hand evaluation
pub mod eval;
