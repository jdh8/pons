//! Deterministic safety shell over the distilled neural floor — AI-bidder M1.3.
//!
//! [`neural::classify_bba`][crate::bidding::neural::classify_bba] is a bare MLP:
//! it emits a finite logit for every one of the 38 calls, with no built-in
//! respect for the laws or for the floor's non-negotiable forced-situation
//! rails.  [`NeuralFloorBba`][crate::bidding::neural_floor::NeuralFloorBba] wraps it so it is safe to attach as the floor,
//! exactly where [`instinct()`][super::instinct::instinct] attaches (see
//! [`american`][super::american::american]).
//!
//! The shell has two paths:
//!
//! - **Forced** — when `instinct::forced` reports an
//!   *auction-determined* forced situation (partner's live takeout double, a
//!   prior call committing us to game, or partner's just-made transfer over our
//!   strong notrump), it returns the deterministic [`instinct()`] answer
//!   verbatim.  The net is never trusted on the rails; delegating reproduces the
//!   already-tested behavior exactly.
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
use super::instinct::{forced, instinct};
use super::trie::Classifier;
use super::{features, neural};
use contract_bridge::Hand;
use contract_bridge::auction::{Auction, Call};
use std::sync::LazyLock;

/// The deterministic ladder, built once; the forced path reuses it per board.
static LADDER: LazyLock<Rules> = LazyLock::new(instinct);

/// The BBA-distilled disclosable floor, made safe to attach under the book
///
/// A [`Classifier`] drop-in for [`instinct()`][super::instinct::instinct]: the
/// learned net in the judgement middle, the deterministic rails preserved by
/// delegation.  It is fed the *disclosable-only*
/// [`features_v3`][super::features::features_v3] (no card-specific values) and
/// distilled from the vendored **EPBot 2/1** oracle
/// ([`neural::classify_bba`]) — a stronger learned prior than the deterministic
/// ladder it floors (EPBot clears our instinct floor by ~1.9 IMPs/board).  See
/// the [module docs][self].
#[derive(Clone, Copy, Debug, Default)]
pub struct NeuralFloorBba;

impl Classifier for NeuralFloorBba {
    fn classify(&self, hand: Hand, context: &Context<'_>) -> Logits {
        if forced(context) {
            // Rails: trust the deterministic floor, never the net.
            return LADDER.classify(hand, context);
        }
        let mut logits = neural::classify_bba(&features::features_v3(hand, context));
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
mod tests {
    use super::*;
    use contract_bridge::auction::RelativeVulnerability;
    use contract_bridge::{Bid, Strain};

    const fn call(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid::new(level, strain))
    }

    /// The shelled net's logits for a hand in an auction
    fn shelled(auction: &[Call], hand: &str) -> Logits {
        let hand: Hand = hand.parse().expect("valid test hand");
        let context = Context::new(RelativeVulnerability::NONE, auction);
        NeuralFloorBba.classify(hand, &context)
    }

    /// The shelled net's highest-logit call
    fn best(auction: &[Call], hand: &str) -> Call {
        let logits = shelled(auction, hand);
        (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("array is never empty")
    }

    // The five §0.4 safety properties, enforced by the shell against the learned
    // net.  The four forced rails delegate to `instinct()`, so they reproduce
    // its tested calls exactly; the legality rail exercises the net + mask.

    #[test]
    fn advancing_a_double_delegates_to_instinct() {
        // Partner doubled their 3♣ for takeout; the shell delegates to instinct,
        // reproducing its calls — advance a bust with an outside suit, defend with
        // four cards behind their suit (the settle floor, default on).
        let auction = [call(3, Strain::Clubs), Call::Double, Call::Pass];
        assert_eq!(best(&auction, "96432.J85.9742.2"), call(3, Strain::Spades));
        assert_eq!(best(&auction, "964.J85.974.9632"), Call::Pass);
    }

    #[test]
    fn forced_to_game_never_passes_below_game() {
        // 2♣ (strong) – 2♦ (game-forcing) – 2NT: the auction forces game, so the
        // shell delegates to instinct and never passes below game.
        let auction = [
            call(2, Strain::Clubs),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "QJ52.K43.T62.J32"), call(3, Strain::Notrump));
        assert_eq!(best(&auction, "3.QJ9854.K32.J32"), call(4, Strain::Hearts));
    }

    #[test]
    fn completes_partners_transfer_over_notrump() {
        // We opened 1NT and partner transferred 2♦ (hearts): the shell delegates
        // to instinct and completes with 2♥.
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "AQ32.KJ5.KQ4.Q92"), call(2, Strain::Hearts));
    }

    #[test]
    fn forced_game_steps_aside_when_penalizing() {
        // 2♣ – 2♦ – 2NT, then they sacrifice in 3♦ and partner doubles for
        // penalty.  The auction still forces game, so the shell delegates to
        // instinct, which shows the six-card suit rather than a stopperless 3NT.
        let auction = [
            call(2, Strain::Clubs),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
            call(2, Strain::Notrump),
            call(3, Strain::Diamonds),
            Call::Double,
            Call::Pass,
        ];
        let chosen = best(&auction, "K3.KQ4.65.QJ8765");
        assert_eq!(chosen, call(4, Strain::Clubs));
        assert_ne!(chosen, call(3, Strain::Notrump));
    }

    #[test]
    fn doubles_only_their_live_bids() {
        // Not a forced auction → the net + legality mask.  The call to beat is
        // our own 2♠ (partner raised our overcall); the net emits a finite Double
        // logit, but doubling our own side is illegal, so the mask zeroes it —
        // while Pass stays finite so a distribution always exists.
        let auction = [
            call(1, Strain::Hearts),
            call(1, Strain::Spades),
            Call::Pass,
            call(2, Strain::Spades),
            Call::Pass,
        ];
        let logits = shelled(&auction, "92.K53.AQJ42.962");
        assert_eq!(*logits.0.get(Call::Double), f32::NEG_INFINITY);
        assert!(logits.0.get(Call::Pass).is_finite());
    }
}
