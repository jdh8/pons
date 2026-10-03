use super::super::tests::{best_call, best_call_with, call};
use crate::bidding::agreements::Agreements;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

#[test]
fn jordan_truscott_over_their_double() {
    let mut arm = Agreements::default();
    arm.competition.jordan_truscott = true;
    let auction = [call(1, Strain::Spades), Call::Double];
    // Jordan 2NT: 4 trumps, limit+.
    let (jordan, floored) = best_call_with(&arm, &auction, "Q542.A5.K964.Q32");
    assert_eq!(jordan, call(2, Strain::Notrump), "Jordan/Truscott");
    assert!(!floored, "an authored node, not the floor");
    // Value redouble: 10+ without the fit.
    let (xx, _) = best_call_with(&arm, &auction, "K2.A54.K964.Q532");
    assert_eq!(xx, Call::Redouble, "the value redouble");
    // The jump raise flips preemptive.
    let (preempt, _) = best_call_with(&arm, &auction, "Q542.9.96432.Q32");
    assert_eq!(preempt, call(3, Strain::Spades), "preemptive jump raise");
    // A weak 2-level new suit is non-forcing — opener passes a minimum.
    let weak = [
        call(1, Strain::Spades),
        Call::Double,
        call(2, Strain::Clubs),
        Call::Pass,
    ];
    let (pass, weak_floored) = best_call_with(&arm, &weak, "AQ542.K54.96.432");
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
    let (accept, _) = best_call_with(&arm, &answer, "AKQ54.K54.96.A32");
    assert_eq!(accept, call(4, Strain::Spades), "a maximum accepts");
    let (decline, _) = best_call_with(&arm, &answer, "AQ542.954.96.A32");
    assert_eq!(decline, call(3, Strain::Spades), "a minimum declines");
}

#[test]
fn redouble_answer_shadows_the_rebase_blast() {
    // `1♠ (X) XX -`: opener's rebid.  The systems-on rebase strips the
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

    let mut off = Agreements::default();
    off.competition.redouble_answer = false;
    let (off_call, _) = best_call_with(&off, &auction, opener);
    assert_ne!(
        off_call,
        Call::Pass,
        "the off arm: the rebase + floor bids on blindly"
    );
}

/// The weak new suit is not a 2/1: over opener's raise responder bids game on
/// a maximum and passes a minimum, instead of replaying the uncontested
/// game-forcing tree through the systems-on rebase.
#[test]
fn weak_new_suit_rebid_is_not_game_forcing() {
    let arm = Agreements::default();
    let raised = |opening, suit| {
        [
            call(1, opening),
            Call::Double,
            call(2, suit),
            Call::Pass,
            call(3, suit),
            Call::Pass,
        ]
    };
    let spades_clubs = raised(Strain::Spades, Strain::Clubs);
    let (minimum, floored) = best_call_with(&arm, &spades_clubs, "J9.A62.JT4.97543");
    assert_eq!(minimum, Call::Pass);
    assert!(!floored, "an authored node, not the rebase");
    let (maximum, _) = best_call_with(&arm, &spades_clubs, "J9.Q62.KT4.AQ543");
    assert_eq!(maximum, call(3, Strain::Notrump));
    let spades_hearts = raised(Strain::Spades, Strain::Hearts);
    let (game, _) = best_call_with(&arm, &spades_hearts, "J9.AJ962.K43.973");
    assert_eq!(game, call(4, Strain::Hearts));
}

/// Opener's strong hands over the weak new suit bid on instead of passing it
/// out, and responder answers the invitations naturally — not as a 2/1.
#[test]
fn weak_new_suit_extras_bid_strong_hands() {
    let arm = Agreements::default();
    let spades_clubs = [
        call(1, Strain::Spades),
        Call::Double,
        call(2, Strain::Clubs),
        Call::Pass,
    ];
    let cases = [
        ("AKQJ64.AQ8.Q2.6", call(4, Strain::Spades)), // 18 HCP, six spades
        ("AKJ764.KQ8.K3.6", call(3, Strain::Spades)), // 16 HCP, six spades
        ("AKJ76.KQ8.Q2.K62", call(3, Strain::Clubs)), // 18 HCP, three clubs
        ("AKJ76.KQ8.Q63.K6", call(2, Strain::Notrump)), // 18 HCP, 5=3=3=2
        ("KQ764.Q85.K32.J6", Call::Pass),             // minimum
    ];
    for (hand, expected) in cases {
        let (got, floored) = best_call_with(&arm, &spades_clubs, hand);
        assert_eq!(got, expected, "{hand}");
        assert!(!floored, "{hand}: authored");
    }
    let mut off = Agreements::default();
    off.competition.weak_new_suit_extras = false;
    let (off, _) = best_call_with(&off, &spades_clubs, "AKJ764.KQ8.K3.6");
    assert_eq!(off, Call::Pass, "off: the catch-all passes");

    let mut invited = spades_clubs.to_vec();
    invited.extend([call(3, Strain::Spades), Call::Pass]);
    let (accept, floored) = best_call_with(&arm, &invited, "J9.A62.JT4.Q8543");
    assert_eq!(accept, call(4, Strain::Spades));
    assert!(!floored);
    let (decline, _) = best_call_with(&arm, &invited, "98.862.JT4.KQ543");
    assert_eq!(decline, Call::Pass);

    let mut notrump = spades_clubs.to_vec();
    notrump.extend([call(2, Strain::Notrump), Call::Pass]);
    let (game, _) = best_call_with(&arm, &notrump, "9.A62.QT4.Q85432");
    assert_eq!(game, call(3, Strain::Notrump));
    let (sign_off, _) = best_call_with(&arm, &notrump, "9.862.JT4.KQ5432");
    assert_eq!(sign_off, Call::Pass);
}

/// The weak new suit's length is a knob: at 6 (the default) a five-card suit
/// no longer bids `2y` and falls through to pass (the natural `1NT` denies
/// a five-card suit since `doubled_notrump_max_length` 4).
#[test]
fn weak_new_suit_length_gates_the_five_card_suit() {
    let auction = [call(1, Strain::Spades), Call::Double];
    let six = Agreements::default();
    let mut five = Agreements::default();
    five.competition.weak_new_suit_length = 5;
    let cases = [
        // (hand, five-card arm, default)
        ("92.KJ853.Q43.J82", call(2, Strain::Hearts), Call::Pass),
        ("9.KJ853.T43.J872", call(2, Strain::Hearts), Call::Pass),
        (
            "92.KJ8532.Q4.J82",
            call(2, Strain::Hearts),
            call(2, Strain::Hearts),
        ),
    ];
    for (hand, five_call, six_call) in cases {
        let (got, _) = best_call_with(&five, &auction, hand);
        assert_eq!(got, five_call, "{hand} at 5");
        let (got, floored) = best_call_with(&six, &auction, hand);
        assert_eq!(got, six_call, "{hand} at 6");
        assert!(!floored, "{hand}: authored");
    }
}

/// `1M (X) 1NT -` is not the forcing notrump: with the knob on (the default), a balanced
/// minimum passes and every other hand keeps the rebase's natural rebid.
#[test]
fn doubled_notrump_pass_drops_the_forcing_rebid() {
    let auction = [
        call(1, Strain::Spades),
        Call::Double,
        call(1, Strain::Notrump),
        Call::Pass,
    ];
    let mut arm = Agreements::default();
    arm.competition.doubled_notrump_pass = true;
    let mut off = Agreements::default();
    off.competition.doubled_notrump_pass = false;
    let (got, _) = best_call_with(&off, &auction, "AQ542.K54.Q96.32");
    assert_ne!(got, Call::Pass, "off: the rebase's forcing-notrump rebid");
    let (got, floored) = best_call_with(&arm, &auction, "AQ542.K54.Q96.32");
    assert_eq!(got, Call::Pass, "a balanced minimum passes");
    assert!(!floored, "authored");
    let (got, _) = best_call_with(&arm, &auction, "AQ5432.K54.Q9.32");
    assert_eq!(got, call(2, Strain::Spades), "a six-card suit rebids it");
    let (got, _) = best_call_with(&arm, &auction, "AK542.K54.AQ6.K2");
    assert_eq!(got, call(2, Strain::Notrump), "18-19 still invites");
}

/// The natural `1NT`'s longest suit is a knob: at 4 (the default) the
/// five-card suit below opener's passes, a balanced hand still bids `1NT`.
#[test]
fn doubled_notrump_max_length_passes_the_five_card_suit() {
    let auction = [call(1, Strain::Spades), Call::Double];
    let arm = Agreements::default();
    let mut any = Agreements::default();
    any.competition.doubled_notrump_max_length = 13;
    let (got, _) = best_call_with(&any, &auction, "92.KJ853.Q43.J82");
    assert_eq!(got, call(1, Strain::Notrump), "13: 1NT on five hearts");
    let (got, floored) = best_call_with(&arm, &auction, "92.KJ853.Q43.J82");
    assert_eq!(got, Call::Pass, "capped at 4: pass");
    assert!(!floored, "authored");
    let (got, _) = best_call_with(&arm, &auction, "92.KJ85.Q43.J872");
    assert_eq!(
        got,
        call(1, Strain::Notrump),
        "four-card suits still bid 1NT"
    );
}
