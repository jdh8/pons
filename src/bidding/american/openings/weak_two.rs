//! Weak-two opening strength and suit-length agreements

use crate::bidding::Rules;
use crate::bidding::agreements::OpeningKnobs;
use crate::bidding::constraint::{cccc, hcp, len, nltc, nth_seat, points};
use contract_bridge::{Bid, Strain, Suit};

/// An honor-location evaluator gauge for the weak-two openings
/// ([`OpeningKnobs::weak_two_eval`])
///
/// Both evaluators reward honors sitting *in the long suit* — the weak twos
/// whose offense is real and whose disclosure to the opponents' blind leads
/// costs least (the sd-lead bias that rejected the raw-HCP re-gauge).  The
/// `*Band` forms replace the strength leg outright (evaluator-as-gauge); the
/// `CcccFloor`/`NltcCeil` forms AND onto the shipped `points(5..=10)` band
/// (evaluator-as-discipline — a strict subset, so the opening's `points
/// 5..=10` inference reading stays exactly sound).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WeakTwoEval {
    /// Kaplan–Rubens CCCC in `lo..hi` replaces `points(5..=10)`.
    CcccBand(f64, f64),
    /// `points(5..=10)` plus a CCCC floor pruning scattered-honor hands.
    CcccFloor(f64),
    /// NLTC in `lo..=hi` replaces `points(5..=10)`; *fewer* losers = stronger,
    /// so `lo` is the strong edge and `hi` the junk edge.
    NltcBand(f64, f64),
    /// `points(5..=10)` plus an NLTC ceiling: at most this many losers.
    NltcCeil(f64),
}

pub(super) fn with_weak_twos(rules: Rules, knobs: &OpeningKnobs) -> Rules {
    let mut rules = rules;
    // Weak twos (six-card suit, not in fourth seat).  Strength gauged by an
    // honor-location evaluator when `weak_two_eval` is armed, in raw HCP
    // when `weak_two_hcp` is armed (the Root-A preempt-discipline fix —
    // sound bridge, but it measured a wash on the honest sd-lead scorer, so it
    // stays opt-in), else the default rule-of-N+8 `points(5..=10)`.
    let weak_two_eval = knobs.weak_two_eval;
    let weak_two_band = knobs.weak_two_hcp;
    for suit in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let bid = Bid::new(2, Strain::from(suit));
        let six = move || len(suit, 6..=6);
        if knobs.weak_two_wild {
            rules = rules.rule(bid, 100, len(suit, 5..=6) & points(3..=12) & !nth_seat(4));
            continue;
        }
        rules = match (weak_two_eval, weak_two_band) {
            (Some(WeakTwoEval::CcccBand(lo, hi)), _) => {
                rules.rule(bid, 100, six() & cccc(lo..hi) & !nth_seat(4))
            }
            (Some(WeakTwoEval::CcccFloor(x)), _) => {
                rules.rule(bid, 100, six() & points(5..=10) & cccc(x..) & !nth_seat(4))
            }
            (Some(WeakTwoEval::NltcBand(lo, hi)), _) => {
                rules.rule(bid, 100, six() & nltc(lo..=hi) & !nth_seat(4))
            }
            (Some(WeakTwoEval::NltcCeil(x)), _) => {
                rules.rule(bid, 100, six() & points(5..=10) & nltc(..=x) & !nth_seat(4))
            }
            (None, Some((lo, hi))) => rules.rule(bid, 100, six() & hcp(lo..=hi) & !nth_seat(4)),
            (None, None) => rules.rule(bid, 100, six() & points(5..=10) & !nth_seat(4)),
        };
    }
    rules
}
