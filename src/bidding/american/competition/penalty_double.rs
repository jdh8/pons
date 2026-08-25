//! Responder's `X` and `Pass` over their overcall, and opener's answer
//!
//! [`DoubleStyle`] says what responder's double of their overcall means —
//! `Optional` (the shipped default), `Penalty`, or `Takeout` — and drives
//! opener's leave-in (`agreements.competition.penalty_double_leave_in`) or
//! cooperation.  The penalty pass (`agreements.competition.penalty_pass`) and
//! the trap pass (`agreements.competition.trap_pass`) are the two ways
//! responder passes for value.
use super::*;

/// The meaning of responder's double of the overcall in `1NT (overcall) X`.
///
/// All variants are *authored* in the book (a finite logit), so the instinct
/// floor's own takeout double — whose `hcp(12..)` threshold is too strong here —
/// is shadowed and we control the strength. Opener's continuation is authored to
/// match the style: penalty → `opener_leaves_in_penalty_double` sits; optional →
/// `opener_cooperates_optional` stands on a fit and runs with a doubleton.
/// Gated behind `agreements.competition.double_style`; [`DoubleStyle::Optional`] (2-3/8+) is the
/// default.
///
/// A/B verdict (`ab-lebensohl`, NS vs EW with both pairs Transfer, 200k,
/// ~1500 divergent), once **both** the doubler's partner *and* the takeout
/// baseline are handled fairly: **Optional > Penalty > Takeout**. Optional beats
/// penalty by **+1.59** and takeout by **+2.14 IMPs/divergent**; penalty beats
/// takeout by **+0.51**. The earlier penalty-vs-takeout disagreement (plain DD
/// favored takeout, perfect-defense favored penalty) was an **artifact of opener
/// pulling responder's penalty double** — once opener sits, both measures favor
/// penalty over takeout; once opener also *cooperates* with a 2-3-card optional
/// double (stand on a fit, run with a doubleton) optional wins outright. The
/// ranking is robust to the responder's-double reading. `Takeout`/`Penalty` stay
/// selectable for A/B.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DoubleStyle {
    /// Classic takeout, `len(over, ..=3) & hcp(8..)` (former default; best plain-DD
    /// double only while penalty doubles were pulled — see [`DoubleStyle`]).
    Takeout,
    /// Penalty — length and values in their suit, `len(over, 4..) & hcp(9..)`;
    /// opener sits (see `agreements.competition.penalty_double_leave_in`).
    Penalty,
    /// Penalty at a lower floor: `len(over, 4..) & hcp(7..)`
    PenaltyLight,
    /// Default: cooperative / optional takeout, never short: `len(over, 2..=3) &
    /// hcp(8..)`; opener stands on a fit and runs with a doubleton (see
    /// `opener_cooperates_optional`).
    #[default]
    Optional,
}

/// Opener's reply to responder's **penalty** double of their overcall of our 1NT
/// (`1NT (2X) X -`): always sit and defend, since responder promised length and
/// values in their suit
///
/// A 3NT escape (opener-max with their suit stopped) was A/B'd a clear *loss* vs
/// always sitting (+0.328 vs +0.507 IMPs/divergent on `ab-lebensohl`): defending the
/// doubled overcall beats a fragile notrump game, especially when opener also holds
/// length in their suit — so opener never pulls.
///
/// The book dual of the penalty latch's leave-in: without an authored node here the
/// floor reads `… X -` as a takeout advance and *pulls* the penalty double (opener
/// is usually short in their suit, so its own length-gated leave-in never fires).
pub(super) fn opener_leaves_in_penalty_double() -> Rules {
    Rules::new().rule(Call::Pass, 150, hcp(0..))
}

/// Opener's reply to responder's **optional** (cooperative) double of their `over`
/// overcall of our 1NT (`1NT (2X) X -`): responder showed only 2-3 cards in
/// their suit, so opener *decides* — stand (defend) with a three-card-plus fit, but
/// **run with a doubleton** to a real five-card suit, escaping a thin defense
///
/// The floor would stand only with four-plus behind their suit and pull everything
/// else, so it runs the three-card fits opener should defend — the optional dual of
/// the penalty-double leak.  Without a five-card suit a short opener has nowhere to
/// run, so it sits (the catch-all `Pass`).
pub(super) fn opener_cooperates_optional(over: Suit) -> Rules {
    // Stand by default: a fit defends, and a short hand with no suit has no better.
    let mut rules = Rules::new().rule(Call::Pass, 150, hcp(0..));
    // Run with a doubleton-or-less to a real five-card suit (cheapest legal level).
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit == over {
            continue;
        }
        let strain = Strain::from(suit);
        for level in 2..=3 {
            rules = rules.rule(
                Bid::new(level, strain),
                160,
                min_level_is(level, strain) & len(over, ..=2) & len(suit, 5..),
            );
        }
    }
    rules
}

/// Author responder's double of their `over` overcall per the active
/// [`DoubleStyle`] (or the `agreements.competition.double_override` spec). Shadows the instinct
/// floor's takeout double so the threshold is the one chosen here.
pub(super) fn responder_double(rules: Rules, over: Suit, agreements: &Agreements) -> Rules {
    if let Some((lo, hi, floor)) = agreements.competition.double_override {
        // The sweep spans the penalty/takeout continuum: a minimum of two in
        // their suit is what separates the shipped `Optional` double (2..=3)
        // from `Takeout` (..=3, which admits shortness), so it is also where
        // the PDI trigger tag starts.
        return rules
            .rule(Call::Double, 155, len(over, lo..=hi) & hcp(floor..))
            .penalty_if(lo >= 2);
    }
    // The `len` ranges have distinct types, so author inside each arm.
    match agreements.competition.double_style {
        DoubleStyle::Takeout => rules.rule(Call::Double, 155, len(over, ..=3) & hcp(8..)),
        DoubleStyle::Penalty => rules
            .rule(Call::Double, 155, len(over, 4..) & hcp(9..))
            .penalty(),
        DoubleStyle::PenaltyLight => rules
            .rule(Call::Double, 155, len(over, 4..) & hcp(7..))
            .penalty(),
        DoubleStyle::Optional => rules
            .rule(Call::Double, 155, len(over, 2..=3) & hcp(8..))
            .penalty(),
    }
}

#[cfg(test)]
mod tests;
