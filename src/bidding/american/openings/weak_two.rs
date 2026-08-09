//! Weak-two opening strength and suit-length agreements

use crate::bidding::Rules;
use crate::bidding::agreements::OpeningKnobs;
use crate::bidding::constraint::{cccc, hcp, len, nltc, nth_seat, points};
use contract_bridge::{Bid, Strain, Suit};
use std::cell::Cell;

thread_local! {
    /// The weak-two opening's strength band, gauged in raw HCP when `Some`.
    /// Default `None`: byte-identical `points(5..=10)`.  See [`set_weak_two_hcp`].
    static WEAK_TWO_HCP: Cell<Option<(u8, u8)>> = const { Cell::new(None) };
}

thread_local! {
    /// The weak-two opening's honor-location evaluator gauge when `Some`.
    /// Default `None`: byte-identical.  Wins over [`WEAK_TWO_HCP`] if both are
    /// armed.  See [`set_weak_two_eval`].
    static WEAK_TWO_EVAL: Cell<Option<WeakTwoEval>> = const { Cell::new(None) };
}

thread_local! {
    /// Whether weak twos admit five-card suits and a wider strength band.
    /// Default `false` (byte-identical). See [`set_weak_two_wild`].
    static WEAK_TWO_WILD: Cell<bool> = const { Cell::new(false) };
}

/// Gauge the weak-two openings in raw HCP over `lo..=hi` instead of the default
/// rule-of-N+8 `points(5..=10)` (opt-in; the default is byte-identical).
///
/// The opening is *fit-unknown*, so a preempt's length is already pinned by the
/// six-card requirement and gauging its *strength* in shape-crediting `points`
/// double-counts that length: a six-card suit reads `+max(0, L2−8)`, i.e. +0 on
/// 6-2-2-3 up to +2 on 6-4-2-1, so no single `points` shift restores a clean
/// cutoff — the shapely hands slip in one-to-two HCP light while the top edge
/// blurs.  Raw HCP is the disciplined, disclosable gauge: partner can trust the
/// count for games, sacrifices, and leads.
///
/// Only the fit-unknown *opening* moves.  The Ogust min/max answers stay on
/// `points`, deliberately: responder's 2NT promises support, so those are
/// *fit-known* and re-credit shape (the split mirrors the 2/1 gate's
/// hcp/support-points fit-split).
///
/// **Rejected default-on** (opt-in only): fix-vs-shipped `hcp(5..=10)` measured a
/// wash on the honest sd-lead scorer (−0.0045 NV / −0.0018 vul, CIs span 0) — a
/// weak two is a preempt, and the plain-DD "remnant" the point-count campaign
/// priced on this family is the obstruction/disclosure wall, not a fixable gauge
/// (the marginal weak twos over-disclose to the opponents' blind leads).  A
/// major-only carve measured strictly worse (sd-vul −0.0113).  Retained as a
/// single-dummy re-measure candidate (docs/point-count-threshold-campaign.md).
pub fn set_weak_two_hcp(band: Option<(u8, u8)>) {
    WEAK_TWO_HCP.with(|cell| cell.set(band));
}

/// The configured raw-HCP weak-two band, or `None` when off
pub(super) fn weak_two_hcp() -> Option<(u8, u8)> {
    WEAK_TWO_HCP.with(Cell::get)
}

/// An honor-location evaluator gauge for the weak-two openings
/// ([`set_weak_two_eval`])
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

/// Gauge the weak-two openings with an honor-location evaluator
/// ([`WeakTwoEval`]) instead of the default rule-of-N+8 `points(5..=10)`
/// (opt-in; the default `None` is byte-identical; wins over
/// [`set_weak_two_hcp`] if both are armed).
///
/// The raw-HCP re-gauge ([`set_weak_two_hcp`]) was rejected on the sd-lead
/// scorer: the marginal weak twos it admits over-disclose to the opponents'
/// blind leads.  CCCC and NLTC test the follow-up hypothesis that *where the
/// honors sit* — concentrated in the six-card suit versus scattered through
/// the short suits — separates the weak twos worth their disclosure from the
/// rest.  Like the HCP knob, only the fit-unknown opening moves; the Ogust
/// min/max answers stay on fit-known `points`.
pub fn set_weak_two_eval(gauge: Option<WeakTwoEval>) {
    WEAK_TWO_EVAL.with(|cell| cell.set(gauge));
}

/// The configured honour-location weak-two gauge, or `None` when off
pub(super) fn weak_two_eval() -> Option<WeakTwoEval> {
    WEAK_TWO_EVAL.with(Cell::get)
}

/// Admit five-card suits and `points(3..=12)` to weak-two openings (opt-in;
/// the default `false` is byte-identical).
pub fn set_weak_two_wild(on: bool) {
    WEAK_TWO_WILD.with(|cell| cell.set(on));
}

pub(super) fn weak_two_wild() -> bool {
    WEAK_TWO_WILD.with(Cell::get)
}

pub(super) fn with_weak_twos(rules: Rules, knobs: &OpeningKnobs) -> Rules {
    let mut rules = rules;
    // Weak twos (six-card suit, not in fourth seat).  Strength gauged by an
    // honor-location evaluator when `set_weak_two_eval` is armed, in raw HCP
    // when `set_weak_two_hcp` is armed (the Root-A preempt-discipline fix —
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
