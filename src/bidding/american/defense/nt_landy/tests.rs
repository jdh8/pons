use super::super::tests::{best_call_with, call};
use crate::bidding::agreements::Agreements;
use crate::bidding::american::NotrumpDefense;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

/// Coupling: a Landy range feeds the one shared two-suiter band, so Landy's and
/// Woolsey's identical both-majors `2♣` can never carry divergent strengths.
#[test]
fn landy_range_and_woolsey_use_the_shared_band() {
    let mut agreements = Agreements::default();
    agreements.decision.reading.landy_range = Some((9, 16));
    agreements.decision.reading.woolsey_points = (9, 16);
    assert_eq!(
        agreements.decision.reading.landy_range,
        Some(agreements.decision.reading.woolsey_points),
    );
}

#[test]
fn direct_landy_double_shows_both_majors_and_runs_clean() {
    let nt = call(1, Strain::Notrump);
    let p = Call::Pass;
    let x = Call::Double;
    let xx = Call::Redouble;
    let d2 = call(2, Strain::Diamonds);
    // Direct Landy is two fields: select the system and store its 4-4 policy.
    let mut arm = Agreements::default();
    arm.decision.reading.notrump_defense = NotrumpDefense::DirectLandy;
    arm.defense.direct_landy_four_four = false; // 5-4
    arm.defense.direct_landy_double_floor = 8; // low floor so these 10-14 hands fire the X

    // Both majors 5-4 → X (the both-majors takeout double), from the book.
    let (dbl, floored) = best_call_with(&arm, &[nt], "AJ32.KQ876.32.32");
    // 15+ balanced has no penalty double now → Pass.
    let (pass, _) = best_call_with(&arm, &[nt], "AKQ2.KQ2.KJ2.432");
    // Advancer, equal majors and weak → 2♦ relay ("pick a major").
    let (relay, relay_floored) = best_call_with(&arm, &[nt, x, p], "Q32.Q43.J432.432");
    // They double the artificial relay → doubler still names the longer major
    // (5-4 hearts → 2♥), never sits in the short-diamond 2♦x misfit.
    let (named, named_floored) = best_call_with(&arm, &[nt, x, p, d2, x], "AJ32.KQ876.32.32");
    // They redouble our X.  Clean runout: equal majors / no suit → Pass = ask back
    // (the doubler will name its major), never the phantom 2♦ relay.
    let (ask, ask_floored) = best_call_with(&arm, &[nt, x, xx], "Q32.Q43.J432.432");
    // …and a long-club, short-major advancer escapes to its own 2♣ (to play) —
    // the club rung the two-level 2♣ over the redoubled 1NT gives us.
    let (clubs, _) = best_call_with(&arm, &[nt, x, xx], "32.43.432.AKQ876");
    // After the ask, the doubler names its five-card major.
    let (named_xx, named_xx_floored) = best_call_with(&arm, &[nt, x, xx, p, p], "AJ32.KQ876.32.32");
    // After we name our major (via the undoubled relay) and they double it, SIT —
    // play 2♥x (our 5-4+ fit), never run to 3♦.  `(1NT) X - 2♦ (X) 2♥ (X) - -`.
    let sit_auction = [nt, x, p, d2, x, call(2, Strain::Hearts), x, p, p];
    let (settle, settle_floored) = best_call_with(&arm, &sit_auction, "AJ32.KQ876.32.32");

    assert_eq!(ask, Call::Pass, "equal majors over XX → Pass = ask back");
    assert!(!ask_floored, "the ask-Pass must come from the book");
    assert_eq!(
        clubs,
        call(2, Strain::Clubs),
        "long clubs over XX → 2♣ to play"
    );
    assert_eq!(
        named_xx,
        call(2, Strain::Hearts),
        "doubler names its major after the ask"
    );
    assert!(!named_xx_floored, "the named major must come from the book");
    assert_eq!(
        settle,
        Call::Pass,
        "must sit in our doubled major, not run to 3♦"
    );
    assert!(!settle_floored, "the settle-Pass must come from the book");
    assert_eq!(dbl, Call::Double);
    assert!(!floored, "the both-majors X must come from the book node");
    assert_eq!(pass, Call::Pass, "no penalty double when it is replaced");
    assert_eq!(relay, d2, "weak equal majors relays 2♦");
    assert!(!relay_floored, "the relay must come from the book");
    assert_eq!(
        named,
        call(2, Strain::Hearts),
        "must pull from the doubled 2♦ relay"
    );
    assert!(
        !named_floored,
        "the doubled-relay escape must come from the book"
    );
}

#[test]
fn direct_landy_penalty_pass_defends_1ntx() {
    let nt = call(1, Strain::Notrump);
    let p = Call::Pass;
    let x = Call::Double;
    let mut without_pass = Agreements::default();
    without_pass.decision.reading.notrump_defense = NotrumpDefense::DirectLandy;
    without_pass.defense.direct_landy_four_four = false; // 5-4
    without_pass.defense.direct_landy_double_floor = 8; // floor 8 → penalty needs 22-8 = 14+
    let mut with_pass = without_pass;
    with_pass.defense.direct_landy_penalty_pass = true;

    // No major fit (2-2) + defensive values: with the knob OFF the advancer is
    // forced to bid (no Pass rule); with it ON it passes to defend 1NTx.
    let defensive = "AQ.KQ.QJ876.K432"; // 14 HCP, 2♠-2♥
    let (forced, _) = best_call_with(&without_pass, &[nt, x, p], defensive);
    let (penalty, pen_floored) = best_call_with(&with_pass, &[nt, x, p], defensive);
    // A hand WITH a major fit still bids even with the knob on (not a penalty pass).
    let (with_fit, _) = best_call_with(&with_pass, &[nt, x, p], "QJ32.K.QJ876.K43"); // 4 spades

    assert_ne!(forced, Call::Pass, "knob off: advancer is forced to bid");
    assert_eq!(
        penalty,
        Call::Pass,
        "knob on, no fit + values → pass for penalty"
    );
    assert!(!pen_floored, "the penalty pass must come from the book");
    assert_ne!(
        with_fit,
        Call::Pass,
        "a major fit still bids, never penalty-passes"
    );
}
