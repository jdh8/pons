//! Lazy composition of bidding systems
//!
//! These combinators wrap arbitrary [`System`]s — a bound [`Stance`], a
//! learned model, anything implementing the trait — so they stay lazy
//! wrapper structs rather than materialized tries.  Structural fusion of
//! books belongs to `merge` on [`Trie`] instead.
//!
//! Neither combinator touches vulnerability: per the [`System`] convention
//! it is relative to the side to act, so it passes through unchanged.
//!
//! [`Stance`]: super::book::Stance
//! [`Trie`]: super::Trie

use super::System;
use super::array::Logits;
use contract_bridge::Hand;
use contract_bridge::auction::{Call, RelativeVulnerability};

/// A table where two partnership systems oppose each other
///
/// Constructed by [`System::vs`]: `a.vs(b)` is the table where the
/// partnership playing `a` is the dealer's side.  Sides alternate strictly
/// by call index, so dispatch is by parity alone: the dealer's side acts at
/// even auction lengths, the other side at odd ones.
///
/// For a full board, pick the seating by dealer — `ns.vs(ew)` when
/// North/South deal, `ew.vs(ns)` otherwise.  The opposing slot is also
/// where an *opponent model* goes: an engine knows its own system exactly
/// and models the opponents approximately.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Versus<A, B> {
    dealer_side: A,
    other: B,
}

impl<A, B> Versus<A, B> {
    /// Compose a table from the dealer's side and the other side
    pub const fn new(dealer_side: A, other: B) -> Self {
        Self { dealer_side, other }
    }
}

impl<A: System, B: System> System for Versus<A, B> {
    fn classify(&self, hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> Option<Logits> {
        if auction.len().is_multiple_of(2) {
            self.dealer_side.classify(hand, vul, auction)
        } else {
            self.other.classify(hand, vul, auction)
        }
    }
}

/// A layered system falling through to a second one
///
/// Constructed by [`System::or_else`]: `a.or_else(b)` answers from `a`,
/// falling through to `b` when `a` returns [`None`] or logits without any
/// probability mass (the book covers the auction, but no call fits the
/// hand).  Typical layering: an authored book over a learned model or a
/// generic sane bidder.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OrElse<A, B> {
    first: A,
    second: B,
}

impl<A, B> OrElse<A, B> {
    /// Compose a layered system from the primary and the fallback
    pub const fn new(first: A, second: B) -> Self {
        Self { first, second }
    }
}

impl<A: System, B: System> System for OrElse<A, B> {
    fn classify(&self, hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> Option<Logits> {
        self.first
            .classify(hand, vul, auction)
            .filter(Logits::has_mass)
            .or_else(|| self.second.classify(hand, vul, auction))
    }
}
