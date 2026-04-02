#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

/// Bidding in contract bridge
///
/// This module re-exports data structures from [`dds_bridge`] for
/// convenience.
pub mod bidding;

/// Hand evaluation
pub mod eval;

/// One-variable statistics
pub mod stats;

/// Advanced solvers not covered by [`dds_bridge::solver`]
pub mod solver;
