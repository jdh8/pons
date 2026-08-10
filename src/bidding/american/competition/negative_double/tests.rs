use super::super::tests::{best_call_with, call};
use crate::bidding::agreements::Agreements;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

#[test]
fn modern_negative_double_is_exactly_four_over_one_heart() {
    let mut arm = Agreements::default();
    arm.competition.negative_double_shape = super::negative_double::NegativeDoubleShape::Modern;
    // `1♦ (1♥)`: five spades bid the free 1♠; exactly four double.
    let auction = [call(1, Strain::Diamonds), call(1, Strain::Hearts)];
    let (free, floored) = best_call_with(&arm, &auction, "AQ542.95.964.Q32");
    assert_eq!(free, call(1, Strain::Spades), "five spades bid the suit");
    assert!(!floored, "an authored node, not the floor");
    let (neg, _) = best_call_with(&arm, &auction, "AQ54.95.9642.Q32");
    assert_eq!(neg, Call::Double, "exactly four doubles");
}

#[test]
fn cachalot_rotates_the_one_level() {
    let mut arm = Agreements::default();
    arm.competition.negative_double_shape = super::negative_double::NegativeDoubleShape::Cachalot;
    let auction = [call(1, Strain::Clubs), call(1, Strain::Diamonds)];
    // X = 4+ hearts; 1♥ = 4+ spades; 1♠ = the residual takeout hand.
    let (x, floored) = best_call_with(&arm, &auction, "K52.QJ54.964.Q32");
    assert_eq!(x, Call::Double, "X shows the adjacent major");
    assert!(!floored, "an authored node, not the floor");
    let (transfer, _) = best_call_with(&arm, &auction, "QJ54.K52.964.Q32");
    assert_eq!(transfer, call(1, Strain::Hearts), "1♥ shows spades");
    let (takeout, _) = best_call_with(&arm, &auction, "K52.Q54.964.QJ32");
    assert_eq!(takeout, call(1, Strain::Spades), "1♠ is the takeout hand");
    // Opener completes the heart transfer with exactly three, raises four.
    let complete = [
        call(1, Strain::Clubs),
        call(1, Strain::Diamonds),
        Call::Double,
        Call::Pass,
    ];
    let (three, _) = best_call_with(&arm, &complete, "AQ54.K52.96.QJ32");
    assert_eq!(three, call(1, Strain::Hearts), "exactly three completes");
    let (four, _) = best_call_with(&arm, &complete, "AQ5.K542.96.QJ32");
    assert_eq!(four, call(2, Strain::Hearts), "four raises");
}

#[test]
fn cachalot_probe_spades3() {
    use contract_bridge::Strain::*;
    let mut arm = Agreements::default();
    arm.competition.negative_double_shape = super::negative_double::NegativeDoubleShape::Cachalot;
    let h = "A2.KJ54.KQ543.A2"; // opener-ish, 4 spades
    let cases = vec![
        (
            "1♦ (1♥) X (1♠)",
            vec![
                call(1, Diamonds),
                call(1, Hearts),
                Call::Double,
                call(1, Spades),
            ],
        ),
        (
            "1♦ (1♥) X (1NT)",
            vec![
                call(1, Diamonds),
                call(1, Hearts),
                Call::Double,
                call(1, Notrump),
            ],
        ),
        (
            "1♦ (1♥) X (2♣)",
            vec![
                call(1, Diamonds),
                call(1, Hearts),
                Call::Double,
                call(2, Clubs),
            ],
        ),
        (
            "1♣ (1♥) X (2♣)",
            vec![
                call(1, Clubs),
                call(1, Hearts),
                Call::Double,
                call(2, Clubs),
            ],
        ),
        (
            "natural 1♦ (1♥) 1♠ (2♣)",
            vec![
                call(1, Diamonds),
                call(1, Hearts),
                call(1, Spades),
                call(2, Clubs),
            ],
        ),
        (
            "1♣ (1♦) X (2♣) [hearts family]",
            vec![
                call(1, Clubs),
                call(1, Diamonds),
                Call::Double,
                call(2, Clubs),
            ],
        ),
        (
            "natural 1♣ (1♦) 1♥ (2♣)",
            vec![
                call(1, Clubs),
                call(1, Diamonds),
                call(1, Hearts),
                call(2, Clubs),
            ],
        ),
    ];
    for (t, a) in cases {
        eprintln!("{t:28}: {:?}", best_call_with(&arm, &a, h));
    }
}

#[test]
fn cachalot_probe_spades2() {
    let mut arm = Agreements::default();
    arm.competition.negative_double_shape = super::negative_double::NegativeDoubleShape::Cachalot;
    // spades-family PASS-OUT: does the authored completion even fire over (1♥)?
    let passout = [
        call(1, Strain::Diamonds),
        call(1, Strain::Hearts),
        Call::Double,
        Call::Pass,
    ];
    eprintln!(
        "1♦ (1♥) X -; opener 4♠: {:?}",
        best_call_with(&arm, &passout, "AQ54.K2.KQ543.A2")
    );
    eprintln!(
        "1♦ (1♥) X -; opener 3♠: {:?}",
        best_call_with(&arm, &passout, "AQ5.K42.KQ543.A2")
    );
    // and the (1D) hearts pass-out for contrast:
    let ph = [
        call(1, Strain::Clubs),
        call(1, Strain::Diamonds),
        Call::Double,
        Call::Pass,
    ];
    eprintln!(
        "1♣ (1♦) X -; opener 4♥: {:?}",
        best_call_with(&arm, &ph, "A2.KQ54.A3.KJ654")
    );
}

#[test]
fn cachalot_probe_spades() {
    let mut arm = Agreements::default();
    arm.competition.negative_double_shape = super::negative_double::NegativeDoubleShape::Cachalot;
    // Is `1♦ (1♥) X` even the spade transfer? And does reveal fire?
    let respond = [call(1, Strain::Diamonds), call(1, Strain::Hearts)];
    eprintln!(
        "responder 4=spades: {:?}",
        best_call_with(&arm, &respond, "KJ54.952.A64.Q32")
    );
    for (a, tag) in [
        (
            vec![
                call(1, Strain::Diamonds),
                call(1, Strain::Hearts),
                Call::Double,
                call(2, Strain::Clubs),
            ],
            "X 2C",
        ),
        (
            vec![
                call(1, Strain::Diamonds),
                call(1, Strain::Hearts),
                call(1, Strain::Spades),
                call(2, Strain::Clubs),
            ],
            "1S 2C",
        ),
        (
            vec![
                call(1, Strain::Clubs),
                call(1, Strain::Hearts),
                Call::Double,
                call(2, Strain::Clubs),
            ],
            "1C:X 2C",
        ),
    ] {
        eprintln!("{tag}: {:?}", best_call_with(&arm, &a, "A2.KJ54.KQ543.A2"));
    }
}

#[test]
fn cachalot_x_contested_answer_raises_the_shown_major() {
    // Under competition the pass-out completion doesn't fire; opener's
    // authored contested answer raises the major the X showed at the level
    // the intervention forces — a fit the floor would otherwise leave for a
    // bare double. Hearts over (1♦), spades over (1♥).
    let mut arm = Agreements::default();
    arm.competition.negative_double_shape = super::negative_double::NegativeDoubleShape::Cachalot;
    // `1♣ (1♦) X (2♦)` (X = 4+♥): opener with four hearts jumps to 3♥.
    let x_hearts = [
        call(1, Strain::Clubs),
        call(1, Strain::Diamonds),
        Call::Double,
        call(2, Strain::Diamonds),
    ];
    let (raise, _) = best_call_with(&arm, &x_hearts, "A2.KQ42.A3.KJ654");
    assert_eq!(raise, call(3, Strain::Hearts), "four-card support jumps");
    // Three-card support makes the simple raise to 2♥ (2♥ is above 2♦).
    let (simple, _) = best_call_with(&arm, &x_hearts, "A32.KQ4.A63.KJ54");
    assert_eq!(simple, call(2, Strain::Hearts), "three-card simple raise");
    // No support ⇒ opener passes to defend, not a phantom bid.
    let (defend, _) = best_call_with(&arm, &x_hearts, "AQ32.J.KQ632.A54");
    assert_eq!(defend, Call::Pass, "no fit defends");

    // `1♦ (1♥) X (2♣)` (X = 4+♠): over (1♥) the X shows spades — opener raises.
    let x_spades = [
        call(1, Strain::Diamonds),
        call(1, Strain::Hearts),
        Call::Double,
        call(2, Strain::Clubs),
    ];
    let (raise, _) = best_call_with(&arm, &x_spades, "KQ54.A2.KJ543.A2");
    assert_eq!(raise, call(3, Strain::Spades), "over (1♥) four spades jump");
}

#[test]
fn sputnik_negative_double_is_the_residual() {
    let mut arm = Agreements::default();
    arm.competition.negative_double_shape = super::negative_double::NegativeDoubleShape::Sputnik;
    // `1♣ (1♦)`: a 4-card major is bid naturally at the 1-level...
    let auction = [call(1, Strain::Clubs), call(1, Strain::Diamonds)];
    // 7 HCP: a flat 6-count reads 5 on the rule-of-N+8 scale and passes.
    let (spades, floored) = best_call_with(&arm, &auction, "KJ54.952.964.QJ2");
    assert_eq!(spades, call(1, Strain::Spades), "four spades bid the suit");
    assert!(!floored, "an authored node, not the floor");
    // ...while X denies a biddable major — the residual, ≤3 in each.
    let (neg, _) = best_call_with(&arm, &auction, "K52.Q54.J964.Q32");
    assert_eq!(
        neg,
        Call::Double,
        "≤3 in both majors is the residual double"
    );
}

#[test]
fn cachalot_natural_free_bids_get_the_forcing_answers() {
    let mut arm = Agreements::default();
    arm.competition.negative_double_shape = super::negative_double::NegativeDoubleShape::Cachalot;
    // A natural 2-level free bid reaches Section 4d's forcing answers:
    // opener raises partner's freely bid diamonds with three.
    let answer = [
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    let (raise, floored) = best_call_with(&arm, &answer, "A5.K52.Q64.KJ632");
    assert_eq!(raise, call(3, Strain::Diamonds), "the free bid is raised");
    assert!(!floored, "an authored node, not the floor");
    // The rotated 1-level call stays with its Section-9 completion —
    // 1♥ over (1♦) shows spades; exactly three completes 1♠.
    let complete = [
        call(1, Strain::Clubs),
        call(1, Strain::Diamonds),
        call(1, Strain::Hearts),
        Call::Pass,
    ];
    let (three, _) = best_call_with(&arm, &complete, "AQ5.K52.964.QJ32");
    assert_eq!(
        three,
        call(1, Strain::Spades),
        "the rotation completes, not answer_free_bid"
    );
}

#[test]
fn sputnik_free_major_raise_needs_four() {
    let mut arm = Agreements::default();
    arm.competition.negative_double_shape = super::negative_double::NegativeDoubleShape::Sputnik;
    // Sputnik's natural 1-level major promises only four, so opener's
    // two-level raise demands four trumps — three would be a Moysian.
    let answer = [
        call(1, Strain::Clubs),
        call(1, Strain::Diamonds),
        call(1, Strain::Hearts),
        Call::Pass,
    ];
    let (three, floored) = best_call_with(&arm, &answer, "K52.Q52.K94.AJ63");
    assert_eq!(
        three,
        call(1, Strain::Notrump),
        "three trumps bid 1NT, not the Moysian raise"
    );
    assert!(!floored, "an authored node, not the floor");
    let (four, _) = best_call_with(&arm, &answer, "K5.Q542.K94.AJ63");
    assert_eq!(four, call(2, Strain::Hearts), "four trumps raise");
}

#[test]
fn negative_double_then_suit_is_game_forcing() {
    let mut arm = Agreements::default();
    arm.competition.free_bid_style = super::free_bids::FreeBidStyle::Negative;
    // The doubler clarifies with the concealed long suit — forcing to
    // game — and opener answers it with the forcing-answer table.
    let rebid = [
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
    ];
    let (fg, floored) = best_call_with(&arm, &rebid, "52.A4.AKJ642.Q53");
    assert_eq!(fg, call(2, Strain::Diamonds), "X then the suit is FG");
    assert!(!floored, "an authored node, not the floor");
    let answer = [
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    let (raise, raise_floored) = best_call_with(&arm, &answer, "A5.K52.Q64.KJ632");
    assert_eq!(
        raise,
        call(3, Strain::Diamonds),
        "opener answers the FG suit"
    );
    assert!(!raise_floored, "an authored node, not the floor");
}
