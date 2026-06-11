//! Role-aware partnership books
//!
//! A partnership writes its notes from its own side of the table.  The natural
//! split is by *who makes the first non-pass call*:
//!
//! - a [`Constructive`] book covers the auctions where **we** open — our
//!   openings (in every seat, keyed by their leading passes) and the
//!   continuations, including our competitive bidding when they interfere (as
//!   guarded [`Fallback`][crate::bidding::fallback::Fallback]s);
//! - a [`Defensive`] book covers the auctions where **they** open — our
//!   overcalls, takeout doubles, and defense to their conventional openings.
//!
//! Both wrap the low-level [`Trie`] engine and add nothing to authoring: they
//! deref to it, so [`insert`][Trie::insert], [`fallback_at`][Trie::fallback_at],
//! and friends are available directly.  What the newtype adds is a *gated*
//! [`System`] implementation that answers only for its role; a [`Partnership`]
//! pairs the two into one side's complete system.
//!
//! # Standard pass only
//!
//! These types assume a **standard pass**: a leading [`Pass`][Call::Pass] is
//! neutral and the opener is whoever makes the first non-pass call.  Forcing or
//! strong-pass systems, where the opening pass itself carries meaning, are out
//! of scope — author them as a bare [`Trie`] table model (which keys on the
//! literal auction with no pass semantics) until a dedicated type exists.

use super::System;
use super::array::Logits;
use super::context::Context;
use super::trie::Trie;
use contract_bridge::Hand;
use contract_bridge::auction::{Call, RelativeVulnerability};
use core::ops::{Deref, DerefMut};

/// Whether the opponents made the first non-pass call
///
/// `Versus` dispatches a partnership by parity, so within a partnership's
/// `classify` the side to act owns the indices with `auction.len()` parity.
/// The opener owns the indices with `opening_index` parity; the opponents
/// opened iff those parities differ.  With no opening yet (all passes) the side
/// to act may still open, which is constructive, so this is `false`.
fn they_opened(auction: &[Call]) -> bool {
    match auction.iter().position(|&call| call != Call::Pass) {
        Some(opening) => opening % 2 != auction.len() % 2,
        None => false,
    }
}

/// Resolve `auction` against `trie` exactly like the bare table model
fn resolve(
    trie: &Trie,
    hand: Hand,
    vul: RelativeVulnerability,
    auction: &[Call],
) -> Option<Logits> {
    let context = Context::new(vul, auction).with_prefixes(trie.common_prefixes(auction));
    let (classifier, _) = trie.resolve(&context, auction)?;
    Some(classifier.classify(hand, &context))
}

/// Our book for the auctions where **we** open
///
/// Keyed by the raw table auction, so seats are explicit leading passes: the
/// opening lives at `[]`, `[P]`, `[P, P]`, `[P, P, P]` for 1st through 4th seat,
/// and continuations hang off the matching prefix.  As a [`System`] it answers
/// only when our side opened (or is about to); see the [module docs][self].
#[derive(Clone, Debug, Default)]
pub struct Constructive(pub Trie);

impl Constructive {
    /// Construct an empty constructive book
    #[must_use]
    pub const fn new() -> Self {
        Self(Trie::new())
    }
}

impl Deref for Constructive {
    type Target = Trie;

    fn deref(&self) -> &Trie {
        &self.0
    }
}

impl DerefMut for Constructive {
    fn deref_mut(&mut self) -> &mut Trie {
        &mut self.0
    }
}

impl System for Constructive {
    fn classify(&self, hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> Option<Logits> {
        (!they_opened(auction))
            .then(|| resolve(&self.0, hand, vul, auction))
            .flatten()
    }
}

/// Our book for the auctions where **they** open
///
/// Keyed by the raw table auction starting from their opening: `[1♠]` is our
/// overcall decision over their 1♠, and continuations hang off it.  As a
/// [`System`] it answers only when the opponents opened; see the
/// [module docs][self].
#[derive(Clone, Debug, Default)]
pub struct Defensive(pub Trie);

impl Defensive {
    /// Construct an empty defensive book
    #[must_use]
    pub const fn new() -> Self {
        Self(Trie::new())
    }
}

impl Deref for Defensive {
    type Target = Trie;

    fn deref(&self) -> &Trie {
        &self.0
    }
}

impl DerefMut for Defensive {
    fn deref_mut(&mut self) -> &mut Trie {
        &mut self.0
    }
}

impl System for Defensive {
    fn classify(&self, hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> Option<Logits> {
        they_opened(auction)
            .then(|| resolve(&self.0, hand, vul, auction))
            .flatten()
    }
}

/// One side's complete system: its constructive and defensive books
///
/// As a [`System`] it routes each query to the book for the auction — the
/// [`Constructive`] book when our side opened (or is about to), the
/// [`Defensive`] book when the opponents did.  Compose two partnerships into a
/// table with [`System::vs`].
#[derive(Clone, Debug, Default)]
pub struct Partnership {
    /// The book for the auctions where we open
    pub constructive: Constructive,
    /// The book for the auctions where they open
    pub defensive: Defensive,
}

impl Partnership {
    /// Pair a constructive and a defensive book into one side's system
    #[must_use]
    pub const fn new(constructive: Constructive, defensive: Defensive) -> Self {
        Self {
            constructive,
            defensive,
        }
    }
}

impl System for Partnership {
    fn classify(&self, hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> Option<Logits> {
        if they_opened(auction) {
            self.defensive.classify(hand, vul, auction)
        } else {
            self.constructive.classify(hand, vul, auction)
        }
    }
}
