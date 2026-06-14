//! Round-trip verification for the `Constraint`-DSL authoring compiler (M4.1)
//!
//! The authoring compiler ([`docs/ai-bidder/dsl-spec.md`]) turns an English
//! gloss into a `Constraint`.  Its correctness criterion is a string compare:
//! a compilation of gloss `G` is right when `compiled.describe().to_string() ==
//! G`, because the canonical English *is* `describe()`'s output (milestone
//! M4.0).  This black-box test — it uses only the public
//! [`pons::bidding::constraint`] API, exactly as the compiler's consumer would —
//! pins that round-trip in three parts:
//!
//! 1. [`vocabulary_glosses`] — one assertion per primitive, verifying every
//!    entry of the spec's vocabulary table (§3) against `describe()`.  This is
//!    the guard against `describe()` drift silently invalidating the spec.
//! 2. [`combinator_rendering`] — the `&`/`|`/`!` rendering rules of the grammar
//!    (§2): comma-list flattening, `"or"`, `"not (…)"`, double-negation
//!    cancelling, and the parenthesization of a nested `Any`/`All`.
//! 3. [`held_out_rules`] — the **M4.1 measure**.  Real 2/1 book rules *not* used
//!    as gold examples in the spec, compiled from their gloss alone (no peeking
//!    at the original source) by following `dsl-spec.md`.  Every one reproduces
//!    its gloss exactly.
//!
//! What this does *not* test: the body of a `described("label", closure)` escape
//! hatch.  `describe()` renders only the label, so the closure is a placeholder
//! here; its accept/reject behavior is milestone M4.2's job (a verifier over
//! random hands).  The held-out set is therefore biased toward
//! primitive-expressible rules, with a couple of `described` cases to exercise
//! escape-hatch *recognition* (the compiler must spot the non-primitive meaning
//! and reproduce its label verbatim).

use contract_bridge::{Hand, Strain, Suit};
use pons::bidding::Context;
use pons::bidding::constraint::{
    Constraint, balanced, cccc_at_least, described, fifths, hcp, len, min_level_is, nltc_at_most,
    nth_seat, partner_shown_len, partner_shown_points, partner_suit_is, passed_hand, points,
    short_in_their_suits, stopper_in, stopper_in_their_suits, support, they_bid, they_vulnerable,
    top_honors, undisturbed, vulnerable,
};

/// Render a constraint to its canonical English — the compiler's target.
fn gloss(constraint: impl Constraint) -> String {
    constraint.describe().to_string()
}

/// A placeholder body for a `described` escape hatch.  The round-trip checks the
/// *label* only; M4.2 verifies behavior, so any closure renders identically.
fn stub(_: Hand, _: &Context<'_>) -> bool {
    true
}

/// §3 of the spec: every primitive renders to the documented gloss.
#[test]
fn vocabulary_glosses() {
    // Strength
    assert_eq!(gloss(hcp(15..=17)), "15–17 HCP");
    assert_eq!(gloss(points(12..=21)), "12–21 points");
    assert_eq!(gloss(fifths(15.0..18.0)), "15.0–18.0 fifths");
    assert_eq!(gloss(nltc_at_most(7.0)), "NLTC ≤ 7");
    assert_eq!(gloss(cccc_at_least(14.9)), "CCCC ≥ 14.9");

    // Shape
    assert_eq!(gloss(len(Suit::Spades, 5..)), "5+ ♠");
    assert_eq!(gloss(balanced()), "balanced");

    // Suit quality
    assert_eq!(
        gloss(top_honors(Suit::Clubs, 2..)),
        "2+ of the top honors in ♣"
    );
    assert_eq!(gloss(stopper_in(Suit::Hearts)), "stopper in ♥");
    assert_eq!(gloss(stopper_in_their_suits()), "stopper in their suit(s)");

    // Partnership
    assert_eq!(gloss(support(3..)), "3+ card support for partner");
    assert_eq!(
        gloss(partner_suit_is(Suit::Hearts)),
        "partner's last suit is ♥"
    );
    assert_eq!(
        gloss(partner_shown_len(Suit::Diamonds, 3..)),
        "3+ ♦ shown by partner",
    );
    assert_eq!(
        gloss(partner_shown_points(12..)),
        "12+ points shown by partner"
    );

    // Auction state
    assert_eq!(gloss(they_bid(Strain::Spades)), "opponents bid ♠");
    assert_eq!(
        gloss(short_in_their_suits()),
        "at most three cards in each of their suits",
    );
    assert_eq!(
        gloss(min_level_is(2, Strain::Diamonds)),
        "2♦ is the cheapest bid"
    );
    assert_eq!(gloss(passed_hand()), "a passed hand");
    assert_eq!(gloss(undisturbed()), "the opponents have passed throughout");
    assert_eq!(gloss(nth_seat(3)), "opening in seat 3");

    // Vulnerability
    assert_eq!(gloss(vulnerable()), "vulnerable");
    assert_eq!(gloss(they_vulnerable()), "opponents vulnerable");

    // Escape hatch: renders its label verbatim.
    assert_eq!(
        gloss(described("prefers diamonds", stub)),
        "prefers diamonds"
    );
}

/// §4 range conventions: distinct Rust spellings normalize to one gloss.
#[test]
fn range_rendering() {
    assert_eq!(gloss(hcp(16..)), "16+ HCP");
    assert_eq!(gloss(len(Suit::Hearts, 6..=6)), "exactly 6 ♥");
    assert_eq!(gloss(len(Suit::Hearts, 6..7)), "exactly 6 ♥");
    assert_eq!(gloss(len(Suit::Spades, ..5)), "≤4 ♠");
    assert_eq!(gloss(points(..=11)), "≤11 points");
    // `..=11` and `..12` are interchangeable spellings of the same gloss.
    assert_eq!(gloss(points(..12)), gloss(points(..=11)));
    assert_eq!(gloss(hcp(0..)), "0+ HCP");
    assert_eq!(gloss(fifths(22.0..)), "22.0+ fifths");
}

/// §2 grammar: how the `&`/`|`/`!` tree renders.
#[test]
fn combinator_rendering() {
    // `&` → comma list with "and " before the last; flattens when chained.
    assert_eq!(gloss(hcp(15..=17) & balanced()), "15–17 HCP, and balanced");
    assert_eq!(
        gloss(points(12..=21) & len(Suit::Spades, 5..) & balanced()),
        "12–21 points, 5+ ♠, and balanced",
    );
    // `|` → "or ".
    assert_eq!(
        gloss(len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..)),
        "5+ ♣, or 5+ ♦",
    );
    // `!` → "not (…)"; double negation cancels.
    assert_eq!(gloss(!hcp(16..)), "not (16+ HCP)");
    assert_eq!(gloss(!!balanced()), "balanced");
    // A nested `Any` inside an `All` is parenthesized.
    assert_eq!(
        gloss(points(9..=11) & (nth_seat(3) | nth_seat(4))),
        "9–11 points, and (opening in seat 3, or opening in seat 4)",
    );
}

/// The M4.1 measure: held-out real 2/1 rules compiled from gloss alone, each
/// reproducing its gloss exactly.  Disjoint from the spec's gold examples.
#[test]
fn held_out_rules() {
    let cases: Vec<(String, &str)> = vec![
        // 1♥ opening
        (
            gloss(points(12..=21) & len(Suit::Hearts, 5..)),
            "12–21 points, and 5+ ♥",
        ),
        // strong 2NT opening
        (
            gloss(fifths(20.0..22.0) & balanced()),
            "20.0–22.0 fifths, and balanced",
        ),
        // weak 2♠
        (
            gloss(len(Suit::Spades, 6..=6) & points(5..=10) & !nth_seat(4)),
            "exactly 6 ♠, 5–10 points, and not (opening in seat 4)",
        ),
        // 2NT-invite over a strong 2NT: 11–12 balanced-ish, no major
        (
            gloss(hcp(11..=12) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5)),
            "11–12 HCP, ≤4 ♥, and ≤4 ♠",
        ),
        // simple raise of partner's suit
        (gloss(support(3..)), "3+ card support for partner"),
        // forcing new suit over a weak two
        (
            gloss(len(Suit::Diamonds, 5..) & top_honors(Suit::Diamonds, 2..) & points(14..)),
            "5+ ♦, 2+ of the top honors in ♦, and 14+ points",
        ),
        // 3NT with values and their suit stopped
        (
            gloss(hcp(13..) & stopper_in_their_suits()),
            "13+ HCP, and stopper in their suit(s)",
        ),
        // strong notrumpish action with a stopper in a specific suit
        (
            gloss(stopper_in(Suit::Hearts) & hcp(15..)),
            "stopper in ♥, and 15+ HCP",
        ),
        // Ogust: minimum weak-two with a poor suit
        (
            gloss(points(5..=7) & !top_honors(Suit::Clubs, 2..)),
            "5–7 points, and not (2+ of the top honors in ♣)",
        ),
        // raising partner's specific (second) suit
        (
            gloss(partner_suit_is(Suit::Spades) & len(Suit::Spades, 2..)),
            "partner's last suit is ♠, and 2+ ♠",
        ),
        // RKCB 5♠ answer: 2 keycards with the trump queen (two escape hatches)
        (
            gloss(described("exactly 2 keycards", stub) & described("holds the ♠ queen", stub)),
            "exactly 2 keycards, and holds the ♠ queen",
        ),
        // RKCB 5♣ answer: 1 or 4 keycards (disjunction of escape hatches)
        (
            gloss(described("exactly 1 keycards", stub) | described("exactly 4 keycards", stub)),
            "exactly 1 keycards, or exactly 4 keycards",
        ),
    ];

    let mut reproduced = 0;
    for (compiled, expected) in &cases {
        assert_eq!(compiled, expected, "held-out rule failed to round-trip");
        reproduced += 1;
    }
    assert_eq!(reproduced, cases.len());
    assert_eq!(
        cases.len(),
        12,
        "held-out set size (reported in dsl-spec.md)"
    );
}
