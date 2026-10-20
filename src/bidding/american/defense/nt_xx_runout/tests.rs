use super::super::tests::{best_call_with, call};
use super::{XxMajorFirst, advancer_runout, xx_dont_runout_package};
use crate::bidding::Relative;
use crate::bidding::agreements::Agreements;
use crate::bidding::american::{NotrumpDefense, american};
use crate::bidding::constraint::{Constraint as _, hcp};
use crate::bidding::context::Context;
use crate::bidding::trie::Classifier as _;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Hand, Seat, Strain, Suit};
use rand::SeedableRng as _;

const P: Call = Call::Pass;
const X: Call = Call::Double;
const XX: Call = Call::Redouble;
const NT: Call = call(1, Strain::Notrump);
const C2: Call = call(2, Strain::Clubs);
const D2: Call = call(2, Strain::Diamonds);
const H2: Call = call(2, Strain::Hearts);
const S2: Call = call(2, Strain::Spades);

/// `(1NT) X (XX)` — the advancer to act
const RUNOUT: [Call; 3] = [NT, X, XX];
/// `(1NT) X (XX) - -` — the doubler to act over the forcing pass
const RELAY: [Call; 5] = [NT, X, XX, P, P];

fn on() -> Agreements {
    let mut agreements = Agreements::default();
    agreements.defense.xx_dont_runout = true;
    agreements
}

/// The call the knob-on book makes, asserting it came from the book
#[track_caller]
fn book(auction: &[Call], hand: &str) -> Call {
    let (call, floored) = best_call_with(&on(), auction, hand);
    assert!(!floored, "{hand} at {auction:?} must come from the book");
    call
}

/// [`book`] under a major-first mode
#[track_caller]
fn book_with(mode: XxMajorFirst, auction: &[Call], hand: &str) -> Call {
    let (call, floored) = best_call_with(&major_first(mode), auction, hand);
    assert!(
        !floored,
        "{mode:?}: {hand} at {auction:?} must come from the book"
    );
    call
}

fn with(prefix: &[Call], tail: &[Call]) -> Vec<Call> {
    prefix.iter().chain(tail).copied().collect()
}

/// The knob on under a major-first mode
fn major_first(mode: XxMajorFirst) -> Agreements {
    let mut agreements = on();
    agreements.defense.xx_dont_major_first = mode;
    agreements
}

const MODES: [XxMajorFirst; 3] = [
    XxMajorFirst::Off,
    XxMajorFirst::Longer,
    XxMajorFirst::AtLeastAsLong,
];

#[test]
fn row_invariants_hold_with_the_knob_on() {
    for mode in MODES {
        crate::bidding::rows::assert_package_invariants(
            &major_first(mode),
            &[xx_dont_runout_package()],
        );
    }
}

#[test]
fn knob_defaults_off_and_the_nodes_floor() {
    assert!(!Agreements::default().defense.xx_dont_runout);
    assert_eq!(
        Agreements::default().defense.xx_dont_major_first,
        XxMajorFirst::Off
    );
    for (auction, hand) in [
        (&RUNOUT[..], "J432.432.432.432"),
        (&RELAY[..], "AKQ2.KQ5.AJ4.932"),
    ] {
        let (_, floored) = best_call_with(&Agreements::default(), auction, hand);
        assert!(floored, "knob off: {auction:?} belongs to the floor");
        let (_, floored) = best_call_with(&on(), auction, hand);
        assert!(!floored, "knob on: {auction:?} is a book node");
    }
}

#[test]
fn only_the_natural_direct_seat_is_keyed() {
    let hand = "J432.432.432.432";
    // Balancing seat: `(1NT) - - X (XX)` — the advancer may hold values there.
    let (_, floored) = best_call_with(&on(), &[NT, P, P, X, XX], hand);
    assert!(floored, "the balancing seat is not this package's");
    // A passed doubler's `X` is not the 15+ penalty double.
    let (_, floored) = best_call_with(&on(), &[P, P, P, NT, X, XX], hand);
    assert!(floored, "a passed doubler is not this package's");
    // Behind the other systems the `X` means something else: the knob is inert.
    for system in [NotrumpDefense::DirectDont, NotrumpDefense::Meckwell] {
        let mut armed = on();
        armed.decision.reading.notrump_defense = system;
        let mut bare = Agreements::default();
        bare.decision.reading.notrump_defense = system;
        for hand in [hand, "Q432.J432.432.32", "432.J5432.43.432"] {
            assert_eq!(
                best_call_with(&armed, &RUNOUT, hand),
                best_call_with(&bare, &RUNOUT, hand),
                "{system:?}: the knob must not touch {hand}",
            );
        }
    }
}

#[test]
fn advancer_picks_the_cheapest_two_suiter() {
    for (hand, expected, why) in [
        ("Q432.J432.432.32", H2, "4=4=3=2: both majors"),
        (
            "J432.2.Q432.5432",
            C2,
            "4=1=4=4: clubs + a higher suit, cheapest",
        ),
        ("J432.432.Q432.32", D2, "4=3=4=2: diamonds + a major"),
        ("32.432.Q432.J432", C2, "2=3=4=4: both minors bid 2♣"),
        ("2.J432.Q432.5432", C2, "the three-suiter bids 2♣"),
        ("J5432.432.432.2", S2, "five spades alone: 2♠"),
        ("J5432.J432.432.2", H2, "5♠-4♥: both majors"),
        ("432.J5432.43.432", P, "a five-heart one-suiter passes"),
        ("43.43.J5432.Q432", C2, "5♦-4♣ is a club two-suiter"),
        ("432.43.432.J5432", P, "a five-club one-suiter passes"),
        (
            "J432.432.432.432",
            P,
            "4♠ 4-3-3-3 passes (2♠ promises five)",
        ),
        ("432.432.432.J432", P, "4♣ 4-3-3-3 passes"),
    ] {
        assert_eq!(book(&RUNOUT, hand), expected, "{hand}: {why}");
    }
}

/// Every hand the arithmetic allows has exactly one call: the table is a
/// partition, so each call's reading is its exact shape.
#[test]
fn advancer_table_partitions_every_reachable_hand() {
    for mode in MODES {
        assert_partition(mode);
    }
}

fn assert_partition(mode: XxMajorFirst) {
    let rules = advancer_runout(mode);
    let context = Context::new(RelativeVulnerability::NONE, &RUNOUT);
    let weak = hcp(..=super::ADVANCER_HCP_MAX);
    let mut rng = rand::rngs::StdRng::seed_from_u64(20261020);
    let mut seen = 0;
    while seen < 4000 {
        let deal = contract_bridge::deck::full_deal(&mut rng);
        for hand in [Seat::North, Seat::East, Seat::South, Seat::West].map(|seat| deal[seat]) {
            if !weak.eval(hand, &context).is_finite() {
                continue;
            }
            seen += 1;
            let logits = rules.classify(hand, &context);
            let live = (&logits.0)
                .into_iter()
                .filter(|(_, logit)| logit.is_finite())
                .count();
            assert_eq!(
                live, 1,
                "{mode:?}: {hand} has {live} live calls at (1NT) X (XX)"
            );
        }
    }
}

#[test]
fn artificial_calls_are_alerted_and_the_natural_ones_are_not() {
    for mode in MODES {
        let rules = advancer_runout(mode);
        for rule in rules.rules() {
            let alerted = rule.alert().is_some();
            let natural = matches!(rule.call(), Call::Pass) || rule.call() == S2;
            assert_eq!(
                alerted,
                !natural,
                "{}: alert {:?}",
                rule.call(),
                rule.alert()
            );
        }
    }
}

/// The forcing pass, end to end: a four-spade 4-3-3-3 passes, the doubler
/// relays whatever it holds, the advancer names spades, and the doubler sits.
#[test]
fn flat_four_three_three_three_runs_through_the_relay() {
    let flat = "J432.432.432.432";
    assert_eq!(book(&RUNOUT, flat), P);
    // The doubler never passes the forcing pass — not even with a five-card suit.
    for doubler in ["AKQ2.KQ5.AJ4.932", "AKJ32.KQ5.A4.932", "K2.AQ5.KQ4.AJ932"] {
        assert_eq!(book(&RELAY, doubler), C2, "{doubler} relays");
    }
    let named = with(&RELAY, &[C2, P]);
    assert_eq!(book(&named, flat), S2, "the 4333 names its four-card suit");
    assert_eq!(
        book(&named, "432.J5432.43.432"),
        H2,
        "a one-suiter names it"
    );
    assert_eq!(book(&named, "432.43.432.J5432"), P, "clubs pass the relay");
    // Their double of the relay changes nothing.
    let doubled = with(&RELAY, &[C2, X]);
    assert_eq!(book(&doubled, "432.J5432.43.432"), H2);
    assert_eq!(book(&doubled, "432.43.432.J5432"), P);
    // The doubler sits once the suit is named, passed or doubled.
    let doubler = "AKQ2.KQ5.AJ4.932";
    assert_eq!(book(&with(&named, &[S2, P]), doubler), P);
    assert_eq!(book(&with(&named, &[S2, X]), doubler), P);
    // …and so does the advancer after their double comes back round.
    assert_eq!(book(&with(&named, &[S2, X, P, P]), flat), P);
}

#[test]
fn doubler_answers_the_two_suiters_pass_or_correct() {
    // 2♣: pass with three clubs, else relay 2♦.
    let clubs = with(&RUNOUT, &[C2, P]);
    assert_eq!(book(&clubs, "AQ3.KQ5.KJ43.932"), P);
    assert_eq!(book(&clubs, "AQ32.KQ5.KJ43.92"), D2);
    // …and the advancer names the higher suit (or passes with diamonds).
    let relay = with(&clubs, &[D2, P]);
    assert_eq!(book(&relay, "32.Q432.32.J5432"), H2);
    assert_eq!(book(&relay, "32.432.Q432.J432"), P);
    // 2♦: pass with three diamonds, else 2♥ pass-or-correct.
    let diamonds = with(&RUNOUT, &[D2, P]);
    assert_eq!(book(&diamonds, "AQ3.KQ5.K43.Q932"), P);
    assert_eq!(book(&diamonds, "AQ32.KQ5.K4.Q932"), H2);
    assert_eq!(book(&with(&diamonds, &[H2, P]), "J432.32.Q5432.32"), S2);
    // 2♥: pass with hearts at least as long, correct to longer spades.
    let majors = with(&RUNOUT, &[H2, P]);
    assert_eq!(book(&majors, "AQ3.KQ5.K432.Q32"), P);
    assert_eq!(book(&majors, "AQ32.KQ5.K43.Q32"), S2);
    // A natural 2♠ is sat.
    assert_eq!(book(&with(&RUNOUT, &[S2, P]), "AQ3.KQ5.K432.Q32"), P);
}

/// Their double of a two-suiter: the doubler passes with three, else the SOS
/// redouble; the advancer then bids the other suit and both sit after that.
#[test]
fn sos_redouble_rescues_a_doubled_two_suiter() {
    let doubled = with(&RUNOUT, &[C2, X]);
    assert_eq!(book(&doubled, "AQ3.KQ5.KJ43.932"), P, "three clubs sit");
    assert_eq!(book(&doubled, "AQ32.KQ5.KJ43.92"), XX, "two clubs: SOS");
    let sos = with(&doubled, &[XX, P]);
    assert_eq!(book(&sos, "32.Q432.32.J5432"), H2, "♣+♥ bids hearts");
    assert_eq!(book(&sos, "2.Q432.J432.5432"), D2, "three-suiter: cheapest");
    let rescued = with(&sos, &[H2, X]);
    assert_eq!(book(&rescued, "AQ32.KQ5.KJ43.92"), P, "the doubler sits");
    // 2♦ doubled: same SOS, then the major.
    let doubled = with(&RUNOUT, &[D2, X]);
    assert_eq!(book(&doubled, "AQ32.KQ5.K4.Q932"), XX);
    assert_eq!(book(&with(&doubled, &[XX, P]), "J432.32.Q5432.32"), S2);
    // 2♥ doubled: the other suit is known, so pass or 2♠ as undoubled.
    let doubled = with(&RUNOUT, &[H2, X]);
    assert_eq!(book(&doubled, "AQ32.KQ5.K43.Q32"), S2);
    assert_eq!(book(&doubled, "AQ3.KQ5.K432.Q32"), P);
}

/// The forcing pass reads as what it shows: a weak one-suiter in ♣/♦/♥ or a
/// 4-3-3-3, never a two-suiter or five spades.
#[test]
fn forcing_pass_reads_one_suiter_or_flat_and_weak() {
    let read = american(&on())
        .bind()
        .infer(RelativeVulnerability::NONE, &RELAY);
    let parse = |hand: &str| -> Hand { hand.parse().expect("valid test hand") };
    for hand in ["432.J5432.43.432", "432.43.432.J5432", "J432.432.432.432"] {
        assert!(
            read.admits(Relative::Partner, parse(hand)),
            "the pass admits {hand}"
        );
    }
    for hand in ["Q432.J432.432.32", "J5432.432.432.2", "32.432.Q432.J432"] {
        assert!(
            !read.admits(Relative::Partner, parse(hand)),
            "the pass excludes {hand}"
        );
    }
    let advancer = read.announced(Relative::Partner);
    assert!(advancer.length(Suit::Spades).max <= 4, "no five spades");
    assert!(
        advancer.strength.hcp.max <= super::ADVANCER_HCP_MAX,
        "weak: {:?}",
        advancer.strength.hcp
    );

    // Knob off, the same pass is the floor's and reads nothing of the kind.
    let off = american(&Agreements::default())
        .bind()
        .infer(RelativeVulnerability::NONE, &RELAY);
    assert!(off.admits(Relative::Partner, parse("Q432.J432.432.32")));
}

#[test]
fn artificial_calls_read_their_real_shape() {
    let read = |auction: &[Call]| {
        *american(&on())
            .bind()
            .infer(RelativeVulnerability::NONE, auction)
            .announced(Relative::Partner)
    };
    // The two-suiters.
    let two_clubs = read(&with(&RUNOUT, &[C2, P]));
    assert!(two_clubs.length(Suit::Clubs).min >= 4);
    let two_hearts = read(&with(&RUNOUT, &[H2, P]));
    assert!(two_hearts.length(Suit::Hearts).min >= 4);
    assert!(two_hearts.length(Suit::Spades).min >= 4);
    // The doubler's relay promises no clubs.
    let relay = read(&with(&RELAY, &[C2, P]));
    assert!(
        relay.length(Suit::Clubs).min <= 2,
        "{:?}",
        relay.length(Suit::Clubs)
    );
    // The SOS redouble denies three of the doubled suit.
    let sos = read(&with(&RUNOUT, &[C2, X, XX, P]));
    assert!(
        sos.length(Suit::Clubs).max <= 2,
        "{:?}",
        sos.length(Suit::Clubs)
    );
}

/// A five-plus major with a shorter four-plus minor runs to the major: spades
/// bid `2♠`, hearts pass and name hearts over the relay.  A 5-5 joins them only
/// under `AtLeastAsLong`; a longer minor and both majors never move.
#[test]
fn major_first_runs_a_longer_major_to_the_major() {
    let off = |hand| best_call_with(&on(), &RUNOUT, hand).0;
    let longer = |hand| book_with(XxMajorFirst::Longer, &RUNOUT, hand);
    let five_five = |hand| book_with(XxMajorFirst::AtLeastAsLong, &RUNOUT, hand);
    for (hand, built, expected, tied, why) in [
        ("J5432.432.2.Q432", C2, S2, S2, "5♠-4♣"),
        ("J5432.432.Q432.2", D2, S2, S2, "5♠-4♦"),
        ("J65432.32.Q432.2", D2, S2, S2, "6♠-4♦"),
        ("32.J5432.Q432.32", D2, P, P, "5♥-4♦"),
        ("32.J5432.43.Q432", C2, P, P, "5♥-4♣"),
        ("2.J5432.Q432.432", D2, P, P, "5♥-4♦-3♣"),
        ("J5432.2.Q5432.32", D2, D2, S2, "5♠-5♦"),
        ("2.J5432.32.Q5432", C2, C2, P, "5♥-5♣"),
        ("43.J432.Q5432.32", D2, D2, D2, "4♥-5♦: the minor is longer"),
        ("J5432.J432.32.32", H2, H2, H2, "5♠-4♥: both majors"),
        ("J5432.2.Q432.J32", D2, S2, S2, "5♠-4♦-3♣"),
    ] {
        assert_eq!(off(hand), built, "Off: {hand} ({why})");
        assert_eq!(longer(hand), expected, "Longer: {hand} ({why})");
        assert_eq!(five_five(hand), tied, "AtLeastAsLong: {hand} ({why})");
    }
    // Over the relay a heart-first hand names hearts, even with four-plus
    // diamonds, and the 4-3-3-3s still name their suit.
    for mode in [XxMajorFirst::Longer, XxMajorFirst::AtLeastAsLong] {
        let named = with(&RELAY, &[C2, P]);
        for (hand, expected) in [
            ("2.J5432.Q432.432", H2),
            ("32.J5432.43.Q432", H2),
            ("432.432.Q432.432", D2),
            ("432.43.J5432.432", D2),
            ("432.43.432.J5432", P),
            ("J432.432.432.432", S2),
        ] {
            assert_eq!(book_with(mode, &named, hand), expected, "{mode:?}: {hand}");
        }
        if mode == XxMajorFirst::AtLeastAsLong {
            assert_eq!(book_with(mode, &named, "2.J5432.32.Q5432"), H2, "5♥-5♣");
        }
    }
}

/// The moved hands move in the readings too: `2♠` admits a 5♠-4m and the
/// forcing pass a 5♥-4m.  The minor two-suiters still admit both: their
/// exclusion fold (`!major_first`) overruns the projection's box budget and is
/// dropped, which costs precision, never soundness.
#[test]
fn major_first_readings_follow_the_hands() {
    let parse = |hand: &str| -> Hand { hand.parse().expect("valid test hand") };
    let admits = |auction: &[Call], hand: &str| {
        american(&major_first(XxMajorFirst::Longer))
            .bind()
            .infer(RelativeVulnerability::NONE, auction)
            .admits(Relative::Partner, parse(hand))
    };
    let spades = with(&RUNOUT, &[S2, P]);
    let clubs = with(&RUNOUT, &[C2, P]);
    assert!(admits(&spades, "J5432.432.2.Q432"));
    assert!(admits(&RELAY, "32.J5432.Q432.32"));
    assert!(admits(&clubs, "J5432.2.32.Q5432"), "5-5 stays a two-suiter");
    assert!(
        !admits(&spades, "J5432.2.32.Q5432"),
        "Longer: 2♠ holds no 5-5"
    );
    assert!(
        !admits(&RELAY, "J5432.432.2.Q432"),
        "the pass never holds five spades"
    );
}

/// The package is ours alone (`common::mirror_agreements`): when *we* open
/// `1NT` and redouble their penalty `X`, their runouts keep the house reading,
/// so their pass is not read as forcing nor their `2♣` as a relay.
#[test]
fn their_runouts_over_our_redouble_keep_the_house_reading() {
    let read = |agreements: &Agreements, auction: &[Call]| {
        american(agreements)
            .bind()
            .infer(RelativeVulnerability::NONE, auction)
    };
    for auction in [
        &[NT, X, XX, P][..],
        &[NT, X, XX, H2],
        &[NT, X, XX, P, P, C2],
    ] {
        assert_eq!(
            read(&on(), auction),
            read(&Agreements::default(), auction),
            "{auction:?}"
        );
    }
}
