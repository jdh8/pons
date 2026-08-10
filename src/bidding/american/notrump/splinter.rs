//! The `1NT` splinter — `3♥`/`3♠` showing shortness with a minor
//!
//! Responder jumps to the *short* major, agreeing clubs or diamonds with slam
//! interest. Gated by
//! [`nt_splinter`][field@crate::bidding::inference::ReadingProfile::nt_splinter],
//! with its own HCP floor
//! ([`NotrumpKnobs::nt_splinter_floor`][crate::bidding::agreements::NotrumpKnobs::nt_splinter_floor]).

use super::transfer_gf::splinter_short;
use super::*;

/// Responder's `3♥`/`3♠` splinter over our 1NT — shortness in the *bid* major
///
/// The Bridge World Standard / Polish Club treatment of the only two slots our
/// response ladder leaves empty.  BWS 2017 IV.F(e) words it "three of a major =
/// at most one card in the suit bid", and the BWS 2001 poll gave "both minors
/// with shortness in the bid suit" the expert plurality (37% panel, 29% readers,
/// against 21/25 for natural-and-forcing).
///
/// The shape is pinned rather than merely floored, because every neighbouring
/// slot already owns its half of the residue:
///
/// | axis | value | why not elsewhere |
/// | --- | --- | --- |
/// | bid major | void or low singleton | a stiff A/K un-wastes opener's honors |
/// | other major | 2–3 | four is Stayman's, five transfers |
/// | diamonds | exactly 4 | `♦5+` is the `2NT` transfer |
/// | clubs | 5–6 | `♣7+` keeps the `2♠` transfer |
///
/// so `♣ = 9 − (both majors)` closes the shape at `3-1-4-5`, `1-2-4-6` and
/// `0-3-4-6` (and mirrors).  The `♣6` rows *do* qualify for the `2♠` club
/// transfer, and the rule outranks it deliberately: after `2♠` responder can
/// never show the four diamonds, which is the whole 6-4 slam lane.
///
/// This is a **splinter, not a fragment** — responder names the short major, not
/// the three-card one.  Fragments draw a lead-directing double less often, but
/// the double hurts far more when it comes (the lead runs through responder's
/// real three cards with the doubler's length sitting behind them), whereas an
/// LDX of a splinter directs into a void or singleton and warns us for free.
/// Double dummy is blind to both effects, so the choice is settled on theory.
///
/// BBA/EPBot's `1N-3M splinter` toggle is a *different* convention from the same
/// family — the GIB form, "singleton or void in the suit bid, at least 4 cards
/// in the other 3 suits, no 5-card major", which pins the other major at exactly
/// four and is therefore the hand Stayman already finds.  See
/// `docs/ai-bidder/bba-1nt-splinter.md` for the measured read of it.
///
/// On by default since 2026-07-28, its A/B (`examples/ab-nt-splinter`) having
/// won in all four cells; see
/// [`nt_splinter`][field@crate::bidding::inference::ReadingProfile::nt_splinter] for
/// the measured numbers.
pub(super) fn nt_splinter_rules(agreements: &Agreements) -> Rules {
    if !agreements.decision.reading.nt_splinter {
        return Rules::new();
    }
    let floor = agreements.notrump.nt_splinter_floor;
    let mut rules = Rules::new();
    for (short, other) in [(Suit::Hearts, Suit::Spades), (Suit::Spades, Suit::Hearts)] {
        rules = rules
            .rule(
                Bid::new(3, Strain::from(short)),
                // Above the `2♠`/`2NT` minor transfers (130) so the `♣6` rows
                // route here.  Nothing else can match: Stayman (150) wants a
                // four-card major, Puppet `3♣` (160) wants `balanced()`, the
                // both-majors `3♦` (210) wants 5-5 majors, and the Jacoby
                // transfers (200) want a five-card major.
                170,
                splinter_short(short)
                    & len(other, 2..=3)
                    & len(Suit::Diamonds, 4..=4)
                    & len(Suit::Clubs, 5..=6)
                    & hcp(floor..),
            )
            .alert(SPLINTER);
    }
    rules
}

/// Opener's answer to the `1NT - 3M` splinter — place the game
///
/// Authored because the floor **measured inert here**: given the whole pinned
/// shape it bid `3NT` on every hand, including a total-wastage `♥KQJ` and five
/// spades opposite a known three-card holding.  A 600 000-board self-play A/B
/// found 217 firings and 9 divergent boards (−4 IMPs plain, −22 PD) — responder
/// described perfectly and opener ignored every word of it.  Deleting the node
/// hands the position back to the floor, which is exactly where it does nothing.
///
/// The judgement the convention exists for is **3NT versus 5m**, and the pivot
/// is a stopper in the short major.  Responder holds 0–1 there and opener 2–4,
/// so the opponents own eight to eleven cards in it: without a guard they cash
/// out before `3NT` runs nine tricks, while the minor game has a known 8–9 card
/// fit, 24+ combined points and a ruffing value.  With a guard, `3NT` is nine
/// tricks against eleven and wins the level.
///
/// Opener places the *game* rather than probing at the four level, because the
/// probe has no safe landing: `4♣`/`4♦` is below game in a game-forcing auction,
/// so a floor-generated pass would strand the partnership in a partscore.  The
/// 4-4 major fit is deliberately not hunted — responder holds 2–3 in the other
/// major and opener at most five, so the best case is a 5-3 that `3NT` or the
/// minor game usually beats, and hunting it costs the `3♥`/`3♠` symmetry (over
/// `3♠` there is no room below `3NT` at all).  Slam stays with the floor, which
/// reads the shape and keeps its keycard machinery over the minor game.
fn nt_splinter_answer(short: Suit) -> Rules {
    Rules::new()
        // A guard in the short major: `3NT` is the nine-trick game.
        .rule(Bid::new(3, Strain::Notrump), 100, stopper_in(short))
        // No guard.  Prefer the longer fit: responder's 5-6 clubs opposite three
        // is a nine-card trump suit, and only when opener is short in clubs does
        // the 4-4 diamond fit win.
        .rule(
            Bid::new(5, Strain::Clubs),
            120,
            !stopper_in(short) & len(Suit::Clubs, 3..),
        )
        .rule(
            Bid::new(5, Strain::Diamonds),
            120,
            !stopper_in(short) & len(Suit::Clubs, ..3) & len(Suit::Diamonds, 4..),
        )
        // Finite catch-all: no guard, no fit (a 4-4 major hand with a doubleton
        // club and three diamonds) — take the nine-trick game anyway.
        .rule(Bid::new(3, Strain::Notrump), 50, hcp(0..))
}

/// Opener's game placement after responder's opt-in 1NT - 3M splinter
pub(crate) fn notrump_splinter() -> Package {
    Package {
        name: "nt-splinter",
        gate: |agreements| agreements.decision.reading.nt_splinter,
        entries: |_| {
            expand(
                "P* 1NT - 3M -",
                |_| true,
                |b| nt_splinter_answer(b.suit('M')),
            )
        },
    }
}
