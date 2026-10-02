//! Seam-gated rollout lookahead — AI-bidder M8, the search re-derived.
//!
//! The M2.3 `SearchFloor` (net proposes, double dummy disposes) was deleted
//! with the M1–M3 line.  This is its successor, shaped by the one measurement
//! that passed since: `examples/probe-seam-lookahead`
//! (`docs/exact-posterior.md` §5, Phase 3b).  At a **gated** auction — a seam
//! the caller names — [`Lookahead`][crate::bidding::lookahead::Lookahead] reconsiders the book's call:
//!
//! 1. **Candidates** are the answering table's own rungs
//!    ([`Partnership::rungs`][crate::bidding::Partnership::rungs]), so whatever it picks is a call partner reads.
//! 2. **Worlds** come from the replay sampler
//!    ([`sample_layouts_replay`][crate::bidding::sampler::sample_layouts_replay]) — deals on which our own bidder reproduces
//!    the auction.  Worlds drawn from the readings alone failed the same gate
//!    twice; a short draw keeps the book's call.
//! 3. **Pricing** is one double-dummy solve per world, shared by every
//!    candidate; each is bid out by the bare [`Partnership`][crate::bidding::Partnership] at all four seats
//!    (self-play, and no nested lookahead) and priced as an IMP swing against
//!    the book's call ([`swings`][crate::bidding::ev::swings]) under plain DD and perfect defense.
//! 4. **The rule** is the standing both-scorer one: deviate iff one candidate
//!    has the best mean swing on both scorers and clears
//!    [`margin`][crate::bidding::lookahead::Lookahead::margin] on both.
//!
//! Everywhere else it is the wrapped partnership, call for call.
//!
//! # Determinism
//!
//! [`Bidder::classify`][crate::bidding::Bidder::classify] must be pure, yet the rollout samples.  The RNG is
//! seeded from the decision itself (hand, vulnerability, auction), so the same
//! decision always draws the same worlds.  The actor is pinned to
//! [`Seat::North`][contract_bridge::Seat::North]: every quantity is relative to the actor, so the absolute
//! seat is free.
//!
//! # Threads
//!
//! A gated [`classify`][crate::bidding::Bidder::classify] takes the process-wide ddss
//! [`Solver`][ddss::Solver] lock, which fans out over the core pool itself.  Drive a
//! `Lookahead` from the main thread, never from a rayon worker.

use super::array::Logits;
use super::book::Partnership;
use super::ev::{dealer_of, swings};
use super::sampler::sample_layouts_replay;
use super::table::select_legal_call;
use super::{Bidder, context};
use contract_bridge::auction::{Auction, Call, RelativeVulnerability};
use contract_bridge::{AbsoluteVulnerability, Hand, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::hash::{Hash, Hasher};

/// A [`Partnership`] that looks ahead at the auctions `gate` admits
///
/// See the [module docs][self].  The defaults — 64 worlds, a 0.25 IMP margin —
/// are the registered ones of Phase 3b.
#[derive(Clone, Debug)]
pub struct Lookahead<G> {
    partnership: Partnership,
    gate: G,
    /// Worlds sampled and solved per gated decision
    pub layouts: usize,
    /// IMPs the chosen candidate's mean swing must clear on both scorers
    pub margin: f64,
}

impl<G: Fn(&[Call]) -> bool> Lookahead<G> {
    /// Wrap `partnership`, looking ahead wherever `gate` admits the auction
    #[must_use]
    pub const fn new(partnership: Partnership, gate: G) -> Self {
        Self {
            partnership,
            gate,
            layouts: 64,
            margin: 0.25,
        }
    }

    /// The rung to play instead of the book's `own` call, if one clears the rule
    ///
    /// [`None`] keeps the book: the table offers no other legal rung, the
    /// replay draw came up short, or no candidate cleared the margin on both
    /// scorers.  Not gated — [`classify`][Bidder::classify] applies the gate.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // averaging a handful of IMP swings
    pub fn pick(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
        own: Call,
    ) -> Option<Call> {
        let mut prefix = Auction::new();
        prefix.try_extend(auction.iter().copied()).ok()?;
        let mut candidates = vec![own];
        candidates.extend(
            self.partnership
                .rungs(hand, vul, auction)
                .into_iter()
                .filter(|&call| call != own && prefix.can_push(call).is_ok()),
        );
        if candidates.len() < 2 {
            return None;
        }

        let seat = Seat::North;
        let mut rng = StdRng::seed_from_u64(decision_seed(hand, vul, auction));
        let worlds = sample_layouts_replay(
            hand,
            seat,
            &self.partnership,
            vul,
            auction,
            &self.partnership.infer(vul, auction),
            &mut rng,
            self.layouts,
        );
        if worlds.len() < self.layouts {
            return None;
        }
        let tables = Solver::lock(None).solve_deals(&worlds, NonEmptyStrainFlags::ALL);
        let swings = swings(
            &candidates,
            auction,
            dealer_of(seat, auction.len()),
            seat,
            &worlds,
            &tables,
            &self.partnership,
            &self.partnership,
            absolute(vul),
        );

        let mean = |candidate: usize, scorer: usize| {
            swings[candidate].iter().map(|s| s[scorer]).sum::<i64>() as f64 / worlds.len() as f64
        };
        let best = |scorer: usize| {
            (1..candidates.len()).fold(0, |best, c| {
                if mean(c, scorer) > mean(best, scorer) {
                    c
                } else {
                    best
                }
            })
        };
        let chosen = best(0);
        (chosen != 0
            && chosen == best(1)
            && mean(chosen, 0) > self.margin
            && mean(chosen, 1) > self.margin)
            .then(|| candidates[chosen])
    }
}

impl<G: Fn(&[Call]) -> bool> Bidder for Lookahead<G> {
    fn classify(&self, hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> Option<Logits> {
        let mut logits = self.partnership.classify(hand, vul, auction)?;
        if (self.gate)(auction) {
            let mut prefix = Auction::new();
            prefix.try_extend(auction.iter().copied()).ok()?;
            let own = select_legal_call(Some(logits), &prefix);
            if let Some(call) = self.pick(hand, vul, auction, own) {
                // Seat the pick above the book's whole ladder.
                let top = logits.values().copied().fold(0.0, f32::max);
                *logits.get_mut(call) = top + 1.0;
            }
        }
        Some(logits)
    }

    fn authored_at(&self, vul: RelativeVulnerability, auction: &[Call]) -> bool {
        self.partnership.authored_at(vul, auction)
    }

    fn tombstoned_at(&self, vul: RelativeVulnerability, auction: &[Call], call: Call) -> bool {
        self.partnership.tombstoned_at(vul, auction, call)
    }
}

/// The absolute vulnerability matching a relative one read at North's seat —
/// the inverse of [`context::relative`]`(_, North)`
fn absolute(vul: RelativeVulnerability) -> AbsoluteVulnerability {
    let mut out = AbsoluteVulnerability::NONE;
    out.set(
        AbsoluteVulnerability::NS,
        vul.contains(RelativeVulnerability::WE),
    );
    out.set(
        AbsoluteVulnerability::EW,
        vul.contains(RelativeVulnerability::THEY),
    );
    debug_assert_eq!(context::relative(out, Seat::North), vul);
    out
}

/// FNV-1a as a [`Hasher`]: unlike `DefaultHasher`, the same on every toolchain
struct Fnv(u64);

impl Hasher for Fnv {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}

/// A deterministic RNG seed from the decision: its hand, vulnerability, auction
fn decision_seed(hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> u64 {
    let mut hasher = Fnv(0xcbf2_9ce4_8422_2325);
    (hand, vul.bits(), auction).hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests;
