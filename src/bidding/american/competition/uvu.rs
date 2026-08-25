//! Unusual vs Unusual — over their `2NT` overcall of our `1NT`
//!
//! Their `2NT` shows both minors and steals the whole two level, so responder's
//! answers are re-keyed: the cue, the splinter, and the natural bids each get
//! their own floor (`agreements.competition.uvu_cue_floor`, `agreements.competition.uvu_x_floor`,
//! `agreements.competition.uvu_natural_floor`).  Gated by `agreements.competition.uvu`.

use super::rubensohl::{lm_2d_both_majors_advance, stayman_2d_answer, stayman_2d_fit_rebid};
use super::*;

/// Responder's first call over `1NT (2NT)` — their 2NT shows both minors
///
/// `X` is penalty ("I can beat ≥1 of their suits", `hcp(x_floor..)`); the cues
/// find a major fit — `3♣` = INV+ Stayman (a 4-card major) or 5+♠, `3♦` = INV+
/// 5+♥. `4♣`/`4♦` are FG+ 5-5-majors splinters into the short minor (every 5-5
/// hand is short in exactly one minor, so they cover all of them — 5-5 never goes
/// through Stayman). `3♥`/`3♠` are weak natural sign-offs (a clean 6-card major);
/// `3NT` is to play. The `agreements.competition.uvu_x_floor` /
/// `agreements.competition.uvu_cue_floor` knobs sweep the strength ranges.
/// Pass is the finite catch-all.
pub(super) fn uvu_responder(agreements: &Agreements) -> Rules {
    let x_floor = agreements.competition.uvu_x_floor;
    let cue_floor = agreements.competition.uvu_cue_floor;
    let weak = cue_floor.saturating_sub(1); // points cap for the weak naturals
    let both_majors_55 = len(Suit::Spades, 5..) & len(Suit::Hearts, 5..);

    let mut rules = Rules::new();

    // FG+ 5-5-majors splinters: shortness in the named minor (always available —
    // a 5-5 hand holds ≤1 in exactly one minor).
    rules = rules
        .rule(
            Bid::new(4, Strain::Clubs),
            200,
            both_majors_55.clone() & len(Suit::Clubs, ..=1) & points(10..),
        )
        .alert(UVU_SPLINTER)
        .rule(
            Bid::new(4, Strain::Diamonds),
            200,
            both_majors_55 & len(Suit::Diamonds, ..=1) & points(10..),
        )
        .alert(UVU_SPLINTER);

    // 3♣ = INV+ Stayman (a 4-card major) or 5+♠ (not 5-5); 3♦ = INV+ 5+♥ (≤3♠, so
    // 5♥4♠ prefers 3♣ to hunt the spade fit, and 5-5 is excluded).
    rules = rules
        .rule(
            Bid::new(3, Strain::Clubs),
            185,
            ((len(Suit::Spades, 5..) & len(Suit::Hearts, ..=4))
                | len(Suit::Spades, 4..=4)
                | (len(Suit::Hearts, 4..=4) & len(Suit::Spades, ..=3)))
                & points(cue_floor..),
        )
        .alert(UVU_CUE)
        .rule(
            Bid::new(3, Strain::Diamonds),
            180,
            len(Suit::Hearts, 5..) & len(Suit::Spades, ..=3) & points(cue_floor..),
        )
        .alert(UVU_CUE);

    // 3NT to play: game values, both minors stopped, no major to pursue.
    rules = rules.rule(
        Bid::new(3, Strain::Notrump),
        150,
        points(10..) & stopper_in(Suit::Clubs) & stopper_in(Suit::Diamonds),
    );

    // Penalty X: values *and* a minor we can punish — the (either-or) suit
    // penalty, "I can beat their clubs OR their diamonds". They are 5-5, so a
    // punishing holding is a trick sitting over a 5-card suit: 4+ length, or 4+
    // HCP (AJ/KJ/KQ/AQ) of honors. (Length alone is rare when they hold the
    // minors — honors carry it.) The A/B sweep knob; the suit-specific chase of
    // their actual runout is the encircling follow-up.
    rules = rules
        .rule(
            Call::Double,
            140,
            hcp(x_floor..)
                & (len(Suit::Clubs, 4..)
                    | suit_hcp(Suit::Clubs, 4..)
                    | len(Suit::Diamonds, 4..)
                    | suit_hcp(Suit::Diamonds, 4..)),
        )
        .penalty();

    // Weak natural sign-offs: a long major below INV values, to play. The length
    // floor (default 6) drops to 5 to let a five-card major escape when defending
    // the both-minors overcall looks bad (the A/B sweep knob).
    let nat = usize::from(agreements.competition.uvu_natural_floor);
    rules = rules
        .rule(
            Bid::new(3, Strain::Hearts),
            130,
            len(Suit::Hearts, nat..) & points(..=weak),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            130,
            len(Suit::Spades, nat..) & points(..=weak),
        );

    rules.rule(Call::Pass, 0, hcp(0..))
}

/// Responder's symmetric Smolen after `1NT (2NT) 3♣ - 3♦ -` — opener
/// denied a 4-card major, so show the 5-card major right-sided into opener
///
/// `3♥` = 5+♠, `3♠` = 5+♥; neither promises the *other* major (opener's `3♦`
/// already killed any 4-4 fit, leaving only the 5-3 hunt). `3NT` is the
/// no-five-card-major catch-all (the plain 4-4 Stayman hand).
pub(super) fn uvu_smolen() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 150, len(Suit::Spades, 5..))
        .alert(SMOLEN)
        .rule(Bid::new(3, Strain::Spades), 150, len(Suit::Hearts, 5..))
        .alert(SMOLEN)
        .rule(Bid::new(3, Strain::Notrump), 50, hcp(0..))
}

/// Responder's rebid after `1NT (2NT) 3♣ - 3♥ -` (opener showed 4 hearts)
///
/// Raise to `4♥` with a fit; with 5+♠ and no heart fit show the spades (`3♠`,
/// opener places); else `3NT` (the finite catch-all).
pub(super) fn uvu_rebid_over_3h() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Hearts), 150, len(Suit::Hearts, 4..))
        .rule(Bid::new(3, Strain::Spades), 140, len(Suit::Spades, 5..))
        .rule(Bid::new(3, Strain::Notrump), 50, hcp(0..))
}

/// Section 5d as a row package: Unusual vs Unusual over a both-minors `(2NT)`
/// overcall of our `1NT` (`agreements.competition.uvu`)
///
/// Responder's `X` is penalty; `3♣`/`3♦` are INV+ cues (Stayman/5+♠, 5+♥);
/// `4♣`/`4♦` are FG+ 5-5-majors splinters; the `3♣`→`3♦` denial runs symmetric
/// Smolen.  Opener's `3♣` answer, the Smolen completions, the splinter advance,
/// and the `3♠` rebid are all shared with the `(2♦)` Transfer machinery.
pub(super) fn uvu_package() -> Package {
    Package {
        name: "uvu-over-1nt",
        gate: |agreements| agreements.competition.uvu,
        entries: |agreements| {
            // Responder's first action: the uncovered suffix is exactly their 2NT.
            let mut entries = rows_of(Pattern::after("P* 1NT", "(2NT)"), uvu_responder(agreements));
            for (suffix, rules) in [
                // 3♣ Stayman/5+♠: opener answers, then symmetric Smolen / fit rebids.
                ("(2NT) 3♣ -", stayman_2d_answer()),
                ("(2NT) 3♣ - 3♦ -", uvu_smolen()),
                (
                    "(2NT) 3♣ - 3♦ - 3♥ -",
                    smolen_completion(Suit::Spades, agreements),
                ),
                (
                    "(2NT) 3♣ - 3♦ - 3♠ -",
                    smolen_completion(Suit::Hearts, agreements),
                ),
                ("(2NT) 3♣ - 3♥ -", uvu_rebid_over_3h()),
                ("(2NT) 3♣ - 3♠ -", stayman_2d_fit_rebid(Suit::Spades)),
                // 3♦ = 5+♥: opener raises with a fit, else 3NT.
                ("(2NT) 3♦ -", smolen_completion(Suit::Hearts, agreements)),
                // 4♣/4♦ = FG+ 5-5-majors splinters: opener bids the better major game.
                ("(2NT) 4♣ -", lm_2d_both_majors_advance()),
                ("(2NT) 4♦ -", lm_2d_both_majors_advance()),
            ] {
                entries.extend(rows_of(Pattern::after("P* 1NT", suffix), rules));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
