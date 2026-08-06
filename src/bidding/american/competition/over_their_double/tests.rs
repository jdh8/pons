use super::super::tests::{best_call, call};
use contract_bridge::Strain;
use contract_bridge::auction::Call;

#[test]
fn jordan_truscott_over_their_double() {
    super::over_their_double::set_jordan_truscott(true);
    let auction = [call(1, Strain::Spades), Call::Double];
    // Jordan 2NT: 4 trumps, limit+.
    let (jordan, floored) = best_call(&auction, "Q542.A5.K964.Q32");
    assert_eq!(jordan, call(2, Strain::Notrump), "Jordan/Truscott");
    assert!(!floored, "an authored node, not the floor");
    // Value redouble: 10+ without the fit.
    let (xx, _) = best_call(&auction, "K2.A54.K964.Q532");
    assert_eq!(xx, Call::Redouble, "the value redouble");
    // The jump raise flips preemptive.
    let (preempt, _) = best_call(&auction, "Q542.9.96432.Q32");
    assert_eq!(preempt, call(3, Strain::Spades), "preemptive jump raise");
    // A weak 2-level new suit is non-forcing — opener passes a minimum.
    let weak = [
        call(1, Strain::Spades),
        Call::Double,
        call(2, Strain::Clubs),
        Call::Pass,
    ];
    let (pass, weak_floored) = best_call(&weak, "AQ542.K54.96.432");
    assert_eq!(pass, Call::Pass, "the weak new suit is dropped");
    assert!(!weak_floored, "an authored node, not the floor");
    // Opener answers Jordan with the cue-raise ladder (not Jacoby 2NT,
    // which the systems-on rebase would have reached).
    let answer = [
        call(1, Strain::Spades),
        Call::Double,
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    let (accept, _) = best_call(&answer, "AKQ54.K54.96.A32");
    assert_eq!(accept, call(4, Strain::Spades), "a maximum accepts");
    let (decline, _) = best_call(&answer, "AQ542.954.96.A32");
    assert_eq!(decline, call(3, Strain::Spades), "a minimum declines");
    super::over_their_double::set_jordan_truscott(true);
}

#[test]
fn redouble_answer_shadows_the_rebase_blast() {
    // [1♠ (X) XX (P)]: opener's rebid.  The systems-on rebase strips the
    // double and the redouble, so opener replays uncontested with
    // responder's shown 10+ unseen, and the floor re-prices this shaped
    // minimum (12 HCP, 15 points) as game-going — the remnant report's
    // worst per-board family (−16..−17 IMPs/board vulnerable).  The
    // authored answer passes — even with a long suit (one-of-a-suit
    // redoubled makes with overtricks; a 2M escape rung measured
    // −11 IMPs/fired and was deleted) — and shadows the floor.
    let auction = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Redouble,
        Call::Pass,
    ];
    let opener = "KQ652..AKT764.85"; // 12 HCP 5=0=6=2, opened 1♠
    let (default_call, default_floored) = best_call(&auction, opener);
    assert_eq!(default_call, Call::Pass, "the authored answer passes");
    assert!(!default_floored, "the node shadows the floor");
    let (long, long_floored) = best_call(&auction, "KQJT65.2.KJ85.T4"); // 10 HCP, 6 spades
    assert_eq!(
        long,
        Call::Pass,
        "a long-suit minimum sits for the redoubled make"
    );
    assert!(!long_floored, "the sit is authored too");

    super::over_their_double::set_redouble_answer(false);
    let (off_call, _) = best_call(&auction, opener);
    super::over_their_double::set_redouble_answer(true);
    assert_ne!(
        off_call,
        Call::Pass,
        "the off arm: the rebase + floor bids on blindly"
    );
}
