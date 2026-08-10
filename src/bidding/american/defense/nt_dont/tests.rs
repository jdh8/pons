use super::super::tests::{best_call_with, call};
use crate::bidding::agreements::Agreements;
use crate::bidding::american::NotrumpDefense;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

/// Best call with direct-seat DONT forced on.
fn direct_dont(auction: &[Call], hand: &str) -> (Call, bool) {
    let mut agreements = Agreements::default();
    agreements.decision.reading.notrump_defense = NotrumpDefense::DirectDont;
    best_call_with(&agreements, auction, hand)
}

#[test]
fn direct_dont_replaces_the_penalty_double() {
    // Direct seat over (1NT) with DONT on: the conventional structure, not the
    // natural penalty-X + overcalls.
    let over_1nt = [call(1, Strain::Notrump)];

    // Clubs + a higher major (5♣-4♠) → 2♣  (♣+♦ would be 2NT, not authored here).
    let (c, floored) = direct_dont(&over_1nt, "KJ32.32.4.AQ876");
    assert_eq!(c, call(2, Strain::Clubs));
    assert!(!floored, "DONT 2♣ must come from the book node");

    // Diamonds + a major (5♦-4♥) → 2♦.
    let (c, _) = direct_dont(&over_1nt, "32.KJ32.AQ876.4");
    assert_eq!(c, call(2, Strain::Diamonds));

    // Both majors (5♠-4♥) → 2♥.
    let (c, _) = direct_dont(&over_1nt, "AJ932.K842.32.32");
    assert_eq!(c, call(2, Strain::Hearts));

    // A spade one-suiter bids the natural 2♠ directly (not the X relay).
    let (c, _) = direct_dont(&over_1nt, "AKJ87.432.32.432");
    assert_eq!(c, call(2, Strain::Spades));

    // A non-spade (heart) one-suiter → X, the one-suiter relay double.
    let (c, _) = direct_dont(&over_1nt, "432.AKJ87.32.432");
    assert_eq!(c, Call::Double);

    // 15+ balanced has no DONT bid → Pass; the penalty double is gone.
    let (c, _) = direct_dont(&over_1nt, "AKQ2.KQ2.KJ2.432");
    assert_eq!(c, Call::Pass);
}

#[test]
fn direct_dont_one_suiter_double_relays_then_names() {
    // `(1NT) X -`: with DONT on the direct-seat X is a one-suiter, so the advancer
    // relays 2♣ (a book node now keyed at the direct seat, not floored)...
    let nt = call(1, Strain::Notrump);
    let p = Call::Pass;
    let mut agreements = Agreements::default();
    agreements.decision.reading.notrump_defense = NotrumpDefense::DirectDont;
    let (relay, floored) = best_call_with(&agreements, &[nt, Call::Double, p], "Q32.Q32.Q432.432");
    // ...and the doubler with a long heart suit names it.
    let after_relay = [nt, Call::Double, p, call(2, Strain::Clubs), p];
    let (name, _) = best_call_with(&agreements, &after_relay, "432.AKJ87.32.432");
    // And if they redouble the one-suiter X, the advancer still relays 2♣ —
    // never sits in 1NTxx.
    let (escape, esc_floored) = best_call_with(
        &agreements,
        &[nt, Call::Double, Call::Redouble],
        "Q32.Q32.Q432.432",
    );
    // And if they double our artificial 2♣ relay, the doubler still names the
    // real suit (2♥ here) rather than sitting in the 2♣x misfit.
    let relay_doubled = [
        nt,
        Call::Double,
        Call::Redouble,
        call(2, Strain::Clubs),
        Call::Double,
    ];
    let (named, nd_floored) = best_call_with(&agreements, &relay_doubled, "432.AKJ87.32.432");
    assert_eq!(relay, call(2, Strain::Clubs));
    assert!(!floored, "the direct-seat relay must come from the book");
    assert_eq!(name, call(2, Strain::Hearts));
    assert_eq!(escape, call(2, Strain::Clubs), "must escape 1NTxx, not sit");
    assert!(!esc_floored, "the redouble escape must come from the book");
    assert_eq!(
        named,
        call(2, Strain::Hearts),
        "must escape 2♣x to the real suit"
    );
    assert!(
        !nd_floored,
        "the doubled-relay escape must come from the book"
    );
}
