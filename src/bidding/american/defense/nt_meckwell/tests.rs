use super::super::tests::{best_call, call};
use crate::bidding::american::{NotrumpDefense, set_notrump_defense};
use contract_bridge::Strain;
use contract_bridge::auction::Call;

/// Best call with Meckwell forced on, restored after so it never leaks to a
/// sibling test on this thread.
fn meckwell(auction: &[Call], hand: &str) -> (Call, bool) {
    let prev = super::nt_defense::notrump_defense();
    set_notrump_defense(NotrumpDefense::Meckwell);
    let result = best_call(auction, hand);
    set_notrump_defense(prev);
    result
}

#[test]
fn meckwell_overcalls_replace_the_penalty_double() {
    let over_1nt = [call(1, Strain::Notrump)];

    // A single 6+ minor (long clubs, short elsewhere) → the two-way X, from the book.
    let (c, floored) = meckwell(&over_1nt, "32.32.432.AKQ876");
    assert_eq!(c, Call::Double);
    assert!(!floored, "Meckwell X must come from the book node");

    // Both majors (5-4) → the two-way X too (default four-four accepts it).
    let (c, _) = meckwell(&over_1nt, "AJ32.KQ876.32.32");
    assert_eq!(c, Call::Double);

    // Clubs + a major (5♣-4♠) → 2♣.
    let (c, floored) = meckwell(&over_1nt, "KJ32.32.4.AQ876");
    assert_eq!(c, call(2, Strain::Clubs));
    assert!(!floored, "Meckwell 2♣ must come from the book node");

    // Diamonds + a major (5♦-4♥) → 2♦.
    let (c, _) = meckwell(&over_1nt, "32.KJ32.AQ876.4");
    assert_eq!(c, call(2, Strain::Diamonds));

    // A natural single-suited 6-card heart hand → 2♥ (not the both-majors X).
    let (c, floored) = meckwell(&over_1nt, "32.AKJ876.432.32");
    assert_eq!(c, call(2, Strain::Hearts));
    assert!(!floored, "natural 2♥ must come from the book node");

    // A natural single-suited spade hand → 2♠.
    let (c, _) = meckwell(&over_1nt, "AKJ876.32.432.32");
    assert_eq!(c, call(2, Strain::Spades));

    // Both minors (5-5) → 2NT (the Unusual overlay, on by default).
    let (c, _) = meckwell(&over_1nt, "3.3.AJ876.KQ876");
    assert_eq!(c, call(2, Strain::Notrump));

    // 15+ balanced has no Meckwell bid → Pass; the penalty double is gone.
    let (c, _) = meckwell(&over_1nt, "AKQ2.KQ2.KJ2.432");
    assert_eq!(c, Call::Pass);
}

#[test]
fn meckwell_two_way_double_relays_then_names() {
    let nt = call(1, Strain::Notrump);
    let p = Call::Pass;
    let c2 = call(2, Strain::Clubs);
    let prev = super::nt_defense::notrump_defense();
    set_notrump_defense(NotrumpDefense::Meckwell);

    // `(1NT) X -`: advancer relays 2♣ (pass-or-correct), from the book.
    let (relay, relay_floored) = best_call(&[nt, Call::Double, p], "Q32.Q32.Q432.432");
    // `(1NT) X - 2♣ -`: a diamond one-suiter doubler names 2♦ (real diamonds).
    let (diamonds, _) = best_call(&[nt, Call::Double, p, c2, p], "32.32.AKQ876.432");
    // …a both-majors doubler bids 2♥ (4+ hearts here ⇒ both majors).
    let (majors, majors_floored) = best_call(&[nt, Call::Double, p, c2, p], "AJ32.KQ87.32.32");
    // …a club one-suiter doubler passes (plays 2♣).
    let (clubs, _) = best_call(&[nt, Call::Double, p, c2, p], "32.32.432.AKQ876");
    // `(1NT) X (XX)`: their redouble — the advancer still relays 2♣, never sits 1NTxx.
    let (escape, esc_floored) = best_call(&[nt, Call::Double, Call::Redouble], "Q32.Q32.Q432.432");
    // `(1NT) X - 2♣ (X)`: they double our relay — the diamond doubler still names 2♦,
    // never sits in the doubled 2♣x misfit.
    let (named, nd_floored) =
        best_call(&[nt, Call::Double, p, c2, Call::Double], "32.32.AKQ876.432");
    set_notrump_defense(prev);

    assert_eq!(relay, c2, "advancer relays 2♣ over the two-way X");
    assert!(!relay_floored, "the relay must come from the book");
    assert_eq!(
        diamonds,
        call(2, Strain::Diamonds),
        "diamond one-suiter names 2♦"
    );
    assert_eq!(majors, call(2, Strain::Hearts), "both majors shown as 2♥");
    assert!(
        !majors_floored,
        "the both-majors show must come from the book"
    );
    assert_eq!(clubs, Call::Pass, "club one-suiter passes to play 2♣");
    assert_eq!(escape, c2, "must escape 1NTxx with the relay, not sit");
    assert!(!esc_floored, "the redouble escape must come from the book");
    assert_eq!(
        named,
        call(2, Strain::Diamonds),
        "must escape 2♣x to real diamonds"
    );
    assert!(
        !nd_floored,
        "the doubled-relay escape must come from the book"
    );
}
