//! Deterministic safety shell over the distilled neural floor — AI-bidder M1.3.
//!
//! [`neural::classify_bba_v4`][crate::bidding::neural::classify_bba_v4] is a
//! bare MLP: it emits a finite logit for every one of the 38 calls, with no
//! built-in respect for the laws or for the floor's non-negotiable
//! forced-situation rails.
//! [`ConfiguredFloorBba`][crate::bidding::neural_floor::ConfiguredFloorBba]
//! wraps it so it is safe to
//! attach as the floor, exactly where [`instinct()`][super::instinct::instinct]
//! attaches (see [`american`][super::american::american]).
//!
//! The shell has two paths:
//!
//! - **Forced** — when `instinct::forced` reports an
//!   *auction-determined* forced situation (partner's live takeout double, a
//!   prior call committing us to game, partner's just-made transfer over our
//!   strong notrump, or a live keycard conversation — partner's 4NT, their
//!   1430 answer to ours, or their placement over our answer), it returns the
//!   deterministic [`instinct()`] answer verbatim.  The net is never trusted
//!   on the rails; delegating reproduces the already-tested behavior exactly.
//!   The keycard rail exists because a convention in motion is not judgement:
//!   left to itself the net passed out asks, played 1430 answers, and
//!   redoubled doubled answers (the reading-drift A/B's worst boards).
//! - **Judgement** — otherwise it returns the net's logits, legality-masked: any
//!   call the laws forbid is set to `-∞`, while `Pass` (always legal) stays
//!   finite so a distribution always exists.  This is the vast middle the net is
//!   here to learn.
//!
//! Hand-conditioned game forces (a strong-notrump responder who *holds* game
//! values) are deliberately left to the net — that is judgement, measured in
//! aggregate by the A/B examples, not guarded here.
//!
//! Both paths are book-independent, so this shell is also what
//! [`american_floor`][super::american::american_floor] stands on with no
//! authored book at all.

use super::Rules;
use super::array::Logits;
use super::context::Context;
use super::features::{CompactConfig, Config};
use super::instinct::forced;
use super::trie::Classifier;
use super::{features, neural};
use contract_bridge::Hand;
use contract_bridge::auction::{Auction, Call};
use std::sync::Arc;

/// The **configured** BBA-distilled floor — the card-input (v4) floor
///
/// The shipped default until 2026-08-08, when [`ConfiguredFloorV5`] won its
/// gate A/B.  It stays reachable through
/// [`american_with_config`][super::american::american_with_config] and
/// [`dutch`][crate::dutch()], whose own gate has not run.
///
/// A [`Classifier`] drop-in for [`instinct()`][super::instinct::instinct]: the
/// learned net in the judgement middle, the deterministic rails preserved by
/// delegation.  Distilled from the vendored **EPBot 2/1** oracle
/// ([`neural::classify_bba_v4`]) — a stronger learned prior than the
/// deterministic ladder it floors (EPBot clears our instinct floor by ~1.9
/// IMPs/board).  See the [module docs][self].
///
/// It is fed [`features_v4`][super::features::features_v4], whose last 280
/// floats are **both partnerships' convention cards**.  So the regime is an
/// *input* rather than a choice of artifact, and an A/B arm differs by a card
/// row instead of by a separately trained net — the confound
/// `docs/ai-bidder/configured-net.md` exists to remove.  It replaced the v3
/// twin scheme on gate 1's verdict: +0.1933/+0.2469 plain DD and
/// +0.5256/+0.5358 PD at 2,000,000 fresh boards per cell.
///
/// The [`Config`] is captured **once, when the floor is built**, from whatever
/// the knobs said at that moment (see
/// [`american`][super::american::american]).  A stance is
/// built per A/B arm and a card is an agreement, not a per-call decision, so
/// this is the right granularity — and it keeps the per-decision path from
/// reading ambient state that could silently change what a feature vector
/// means.
///
/// The deterministic ladder the forced path delegates to is captured the same
/// way, as a constructor argument rather than a process-wide `LazyLock`:
/// [`instinct()`][super::instinct::instinct] reads build-time knobs too
/// (`relocating_now()` picks the kickback keycard ladder over the plain one), so
/// a process that builds two differently-knobbed pairs must not share one ladder
/// frozen at whichever came first.
/// `common::with_floor` builds it once per pair and gives the same `Arc` to the
/// constructive book.
#[derive(Clone, Debug)]
pub struct ConfiguredFloorBba(Config, Arc<Rules>);

impl ConfiguredFloorBba {
    /// Attach the floor to one configuration cell — what each side is declared
    /// to play — over one deterministic ladder for the forced rails
    #[must_use]
    pub const fn new(config: Config, ladder: Arc<Rules>) -> Self {
        Self(config, ladder)
    }
}

impl Classifier for ConfiguredFloorBba {
    fn classify(&self, hand: Hand, context: &Context<'_>) -> Logits {
        if forced(context) {
            // Rails: trust the deterministic floor, never the net.
            return self.1.classify(hand, context);
        }
        // The context arrives from the trie without a config — only this floor
        // knows the cell — so attach ours for the extractor to read.  The clone
        // copies scalars and borrows; the auction and prefixes are not copied.
        let configured = context.clone().with_config(&self.0);
        let mut logits = neural::classify_bba_v4(&features::features_v4(hand, &configured));
        mask_illegal(&mut logits, context.auction());
        logits
    }
}

/// The **compact-config** BBA-distilled floor — the v5 candidate
///
/// Exactly [`ConfiguredFloorBba`] but for the regime input: a
/// [`CompactConfig`] (both sides' [`ConventionCard`][features::ConventionCard], 28
/// slots each) instead of a 280-float card pair, feeding
/// [`neural::classify_bba_v5`] through
/// [`features_v5`][features::features_v5].  Same rails, same mask, same
/// build-time capture granularity.
///
/// **The shipped floor** since its 2026-08-08 gate A/B (+0.0353/+0.0262 plain
/// DD per board at none/both vul, PD wash): it floors
/// [`american`][super::american::american],
/// [`american_with_card`][super::american::american_with_card] and
/// the [`american_floor`][super::american::american_floor] ablation.  Unlike
/// v4's card blocks, every slot it reads is an axis a corpus varied, so the
/// 13 trained ones per side move the logits and the rest are folded to zero —
/// `each_compact_axis_moves_its_slots_and_only_live_ones_move_the_net` pins
/// both halves.
#[derive(Clone, Debug)]
pub struct ConfiguredFloorV5(CompactConfig, Arc<Rules>);

impl ConfiguredFloorV5 {
    /// Attach the floor to one configuration cell over one deterministic
    /// ladder for the forced rails
    #[must_use]
    pub const fn new(compact: CompactConfig, ladder: Arc<Rules>) -> Self {
        Self(compact, ladder)
    }
}

impl Classifier for ConfiguredFloorV5 {
    fn classify(&self, hand: Hand, context: &Context<'_>) -> Logits {
        if forced(context) {
            // Rails: trust the deterministic floor, never the net.
            return self.1.classify(hand, context);
        }
        let configured = context.clone().with_compact(&self.0);
        let mut logits = neural::classify_bba_v5(&features::features_v5(hand, &configured));
        mask_illegal(&mut logits, context.auction());
        logits
    }
}

/// Set every call the laws forbid to `-∞`, leaving the rest as the net set them
///
/// Reuses [`Auction::can_push`] — the very predicate the driver filters with —
/// so the mask can never drift from the laws.  `Pass` is always legal, so it
/// stays finite and a distribution always exists (invariant §0.2).
pub(crate) fn mask_illegal(logits: &mut Logits, auction: &[Call]) {
    let mut played = Auction::new();
    // The slice is a real prior auction, so every call in it is legal.
    played
        .try_extend(auction.iter().copied())
        .expect("a prior table auction is legal");
    for (call, slot) in logits.iter_mut() {
        if played.can_push(call).is_err() {
            *slot = f32::NEG_INFINITY;
        }
    }
}

#[cfg(test)]
mod tests;
