use super::super::tests::{best_call_with, call};
use crate::bidding::agreements::Agreements;
use crate::bidding::american::{
    NotrumpDefense, set_notrump_defense, set_woolsey_double_floor, set_woolsey_points,
};
use contract_bridge::Strain;
use contract_bridge::auction::Call;

/// Best call with Woolsey forced on (default ranges) and the conflicting
/// overlay off, independent of any other test on this thread.  Resets the
/// system cell afterward so it cannot leak into a non-Woolsey test.
fn woolsey(auction: &[Call], hand: &str) -> (Call, bool) {
    set_woolsey_points(9, 19);
    set_woolsey_double_floor(11);
    set_notrump_defense(NotrumpDefense::Woolsey);
    // The additive both-minors `2NT` is off for this arm so it cannot outrank
    // a Woolsey call.
    let mut arm = Agreements::current();
    arm.defense.unusual_notrump_range = None;
    let result = best_call_with(&arm, auction, hand);
    set_notrump_defense(NotrumpDefense::Natural);
    result
}

#[test]
fn woolsey_direct_seat_routes_every_shape() {
    let over_1nt = [call(1, Strain::Notrump)];
    // 2♦ Multi: a single 6-card heart suit (other major short).
    let (multi, floored) = woolsey(&over_1nt, "32.KQJ987.A32.32");
    assert_eq!(multi, call(2, Strain::Diamonds));
    assert!(
        !floored,
        "the Woolsey overcall must come from the book node"
    );
    // 2♣ both majors: 5-4.
    assert_eq!(
        woolsey(&over_1nt, "AJ987.KQ32.32.32").0,
        call(2, Strain::Clubs)
    );
    // 2♥ Muiderberg: exactly 5 hearts + a 4-card minor, short spades.
    assert_eq!(
        woolsey(&over_1nt, "32.AQJ98.K987.2").0,
        call(2, Strain::Hearts)
    );
    // X: a 4-card major + a longer (5-card) minor, 11+.
    assert_eq!(woolsey(&over_1nt, "AKQ8.32.KJ987.32").0, Call::Double);
}

#[test]
fn woolsey_has_no_penalty_double() {
    let over_1nt = [call(1, Strain::Notrump)];
    // A flat 22-count has no Woolsey bid — it passes, exactly as in BBA's read
    // (there is no penalty double in this structure).
    let (strong, floored) = woolsey(&over_1nt, "AQ32.KQ3.KQ3.AQ2");
    assert_eq!(strong, Call::Pass);
    assert!(!floored, "the settling Pass must come from the book node");
    // A bare 5332 with a five-card major (no 4-card minor) also passes.
    assert_eq!(woolsey(&over_1nt, "AKJ32.K32.Q32.32").0, Call::Pass);
}

#[test]
fn woolsey_multi_advance_pass_or_corrects() {
    // `(1NT) 2♦ -` — a weak advancer bids the 2♥ pass-or-correct.
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, floored) = woolsey(&auction, "32.K32.J32.J5432");
    assert_eq!(c, call(2, Strain::Hearts));
    assert!(!floored, "the Multi advance must come from the book node");
}

#[test]
fn woolsey_x_advance_never_sits_for_penalty() {
    // `(1NT) X -` — the X is takeout, so a weak no-major advancer relays 2♣
    // (names the doubler's minor), never passing to defend a phantom 1NTx.
    let auction = [call(1, Strain::Notrump), Call::Double, Call::Pass];
    let (relay, floored) = woolsey(&auction, "432.432.432.5432");
    assert_eq!(relay, call(2, Strain::Clubs));
    assert!(!floored, "the X advance must come from the book node");
    // With a good 5-card major of its own, the advancer bids it to play.
    assert_eq!(
        woolsey(&auction, "KQ982.32.432.432").0,
        call(2, Strain::Spades)
    );
}

#[test]
fn woolsey_muiderberg_advance_raises_and_asks() {
    // `(1NT) 2♥ -` — a known 5-card heart suit.  With support + game values the
    // advancer raises to 4♥; with no fit it asks the minor via 2NT (a book node).
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        Call::Pass,
    ];
    let (raise, floored) = woolsey(&auction, "32.K54.AK32.AQ32");
    assert_eq!(raise, call(4, Strain::Hearts));
    assert!(
        !floored,
        "the Muiderberg advance must come from the book node"
    );
    // No heart fit (singleton), invitational+ → 2NT minor-ask, never a floored guess.
    assert_eq!(
        woolsey(&auction, "KQJ2.2.K432.Q432").0,
        call(2, Strain::Notrump)
    );
}

#[test]
fn woolsey_muiderberg_doubled_escapes_a_misfit() {
    // `(1NT) 2♥ (X)` — a weak hand short in hearts escapes the doubled misfit via
    // the 2NT minor-ask rather than sitting in a doubled 5-1 fit.
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        Call::Double,
    ];
    let (escape, floored) = woolsey(&auction, "Q432.2.J432.J432");
    assert_eq!(escape, call(2, Strain::Notrump));
    assert!(!floored, "the doubled escape must come from the book node");
    // With a genuine fit it sits for 2♥x (a known 8-card trump fit).
    assert_eq!(woolsey(&auction, "Q43.K52.J432.432").0, Call::Pass);
}

#[test]
fn woolsey_muiderberg_2nt_names_the_minor() {
    // `(1NT) 2♥ - 2NT -` — the overcaller answers the minor-ask: 3♦ with
    // diamonds, 3♣ with clubs (it always holds a 4+ minor).
    let asked = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    assert_eq!(
        woolsey(&asked, "2.AKJ32.Q432.32").0,
        call(3, Strain::Diamonds)
    );
    assert_eq!(woolsey(&asked, "2.AKJ32.32.Q432").0, call(3, Strain::Clubs));
}
