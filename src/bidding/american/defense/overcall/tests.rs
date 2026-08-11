use super::super::tests::{best_call, best_call_with, call};
use crate::bidding::agreements::Agreements;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain};

#[test]
fn direct_overcall_candidate_defaults() {
    let defense = Agreements::default().defense;
    assert!(!defense.nt_overcall_prefer_one_level_major);
    assert!(!defense.nt_overcall_without_stopper);
    assert!(defense.direct_minor_weak_jump_overcall);
    assert!(!defense.two_level_overcall_quality);
}

#[test]
fn suit_defense_rows_bind_every_opening_and_pass_fan() {
    use crate::bidding::rows::compile_into;
    use crate::bidding::trie::Trie;
    use contract_bridge::Suit;

    let agreements = Agreements::default();
    let mut book = Trie::new();
    compile_into(
        &mut book,
        &agreements,
        &[super::overcall::suit_defense_package()],
    );

    for passes in 0..=3 {
        for suit in Suit::ASC {
            let strain = Strain::from(suit);
            let opening = Bid::new(1, strain);
            let direct: Vec<Call> = core::iter::repeat_n(Call::Pass, passes)
                .chain([Call::Bid(opening)])
                .collect();
            let rules = book
                .get(&direct)
                .expect("direct defense row exists")
                .as_rules()
                .expect("direct row is authored Rules");
            assert!(
                rules
                    .rules()
                    .iter()
                    .any(|rule| rule.call() == Call::Bid(Bid::new(2, strain))),
                "{passes} leading pass(es), (1{suit}) bound the Michaels cue",
            );

            for continuation in [
                [Call::Bid(Bid::new(2, strain)), Call::Pass],
                [Call::Bid(Bid::new(2, Strain::Notrump)), Call::Pass],
            ] {
                let mut advance = direct.clone();
                advance.extend(continuation);
                assert!(
                    book.get(&advance).is_some(),
                    "{passes} leading pass(es), (1{suit}) advance row exists",
                );
            }
        }
    }
}

/// The direct-seat pass gate is the strong tier's complement, authored so
/// the pass reading can project it — byte-identical to the old `hcp(0..)`
/// catch-all: below the tier's floor the gate scores the same, above it
/// the shape-free tier is finite and outscores a weight-0 pass, so pass
/// never won.  Certify both halves on dealt hands, on both tier gauges:
/// the table never rejects a hand wholesale (no floor fallthrough the old
/// catch-all prevented), and no hand above the tier's floor has Pass as
/// its best call.
#[test]
fn direct_pass_gate_is_the_strong_tiers_complement() {
    use crate::bidding::constraint::{hcp, points};
    use crate::bidding::context::Context;
    use crate::bidding::rules::Rules;
    use crate::bidding::trie::Classifier;
    use contract_bridge::Seat;
    use contract_bridge::deck::full_deal;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn certify(rules: &Rules, tier: &dyn crate::bidding::constraint::Constraint) {
        let context = Context::new(RelativeVulnerability::NONE, &[]);
        let mut rng = StdRng::seed_from_u64(1784259000);
        let mut strong = 0;
        for _ in 0..512 {
            let deal = full_deal(&mut rng);
            for hand in [Seat::North, Seat::East, Seat::South, Seat::West].map(|s| deal[s]) {
                let logits = rules.classify(hand, &context);
                let (best, &top) = (&logits.0)
                    .into_iter()
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
                    .expect("array is never empty");
                assert!(top.is_finite(), "the table tiles: no floor fallthrough");
                if tier.eval(hand, &context).is_finite() {
                    strong += 1;
                    assert_ne!(best, Call::Pass, "strong hands double first regardless");
                }
            }
        }
        assert!(strong > 50, "the strong tier fired on a real sample");
    }

    // Shipped gauge: hcp(18..).
    let agreements = Agreements::default();
    certify(
        &super::overcall::defense_to_suit(Bid::new(1, Strain::Hearts), &agreements),
        &hcp(18..),
    );
    // Legacy gauge: points(17..).
    let mut legacy_gauge = Agreements::default();
    legacy_gauge.defense.strong_double_hcp = None;
    let legacy = super::overcall::defense_to_suit(Bid::new(1, Strain::Clubs), &legacy_gauge);
    certify(&legacy, &points(17..));
}

#[test]
fn four_card_overcall_is_opt_in() {
    use crate::bidding::context::Context;
    use crate::bidding::trie::Classifier;
    use contract_bridge::Seat;
    use contract_bridge::deck::full_deal;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    let their_opening = Bid::new(1, Strain::Clubs);
    let auction = [Call::Bid(their_opening)];
    let context = Context::new(RelativeVulnerability::NONE, &auction);
    let agreements = Agreements::default();
    let baseline = super::overcall::defense_to_suit(their_opening, &agreements);
    let mut off_arm = agreements;
    off_arm.defense.overcall_four_card = false;
    let explicit_off = super::overcall::defense_to_suit(their_opening, &off_arm);
    let mut rng = StdRng::seed_from_u64(0x4CA4_D0C1);
    for _ in 0..64 {
        let deal = full_deal(&mut rng);
        for hand in Seat::ALL.map(|seat| deal[seat]) {
            assert_eq!(
                baseline.classify(hand, &context).0,
                explicit_off.classify(hand, &context).0
            );
        }
    }

    let hand: Hand = "AQJ9.K42.763.542".parse().expect("valid test hand");
    let one_s = call(1, Strain::Spades);
    let off = explicit_off.classify(hand, &context);
    let best = |logits: &crate::bidding::Array<f32>| {
        logits
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("array is never empty")
    };
    assert_ne!(best(&off.0), one_s);
    let mut on_arm = agreements;
    on_arm.defense.overcall_four_card = true;
    let on = super::overcall::defense_to_suit(their_opening, &on_arm).classify(hand, &context);
    assert_eq!(best(&on.0), one_s);
}

#[test]
fn two_level_minor_overcall_tight_gates_the_minimum() {
    // Over their (1♠): a single-suited 5-card club minimum overcalls 2♣ by
    // default; the tight knob makes it too weak, so it passes (partner
    // reopens) — the book's finite Pass catch-all must shadow the floor's
    // own overcall instinct, or the suppression is a no-op.
    let over_1s = [call(1, Strain::Spades)];
    let minimum = "J2.K2.Q432.AQ876"; // 12 HCP, 5 clubs, no takeout/two-suiter shape
    let (default_call, floored) = best_call(&over_1s, minimum);
    assert_eq!(default_call, call(2, Strain::Clubs), "default overcalls 2♣");
    assert!(!floored, "the 2♣ overcall is a book node");

    let mut tight = Agreements::default();
    tight.defense.two_level_minor_overcall_tight = true;
    let (tight_call, _) = best_call_with(&tight, &over_1s, minimum);
    // A 17-count is not silenced by the knob — it competes (a takeout X first).
    let (strong_call, _) = best_call_with(&tight, &over_1s, "A2.K2.Q432.AKJ87"); // 17 HCP
    assert_eq!(
        tight_call,
        Call::Pass,
        "tight strands the minimum into Pass"
    );
    assert_ne!(
        strong_call,
        Call::Pass,
        "a 17-count still competes, not silenced"
    );
}

#[test]
fn direct_weak_jump_overcall_is_disjoint_and_reads_exactly() {
    use crate::bidding::Relative;
    use contract_bridge::Suit;

    let over_1d = [call(1, Strain::Diamonds)];
    let weak_six = "T9.KQJ875.4.A987"; // 10 HCP, exactly six hearts
    let five = "T9.KQJ87.43.A987"; // same values, only five hearts
    let strong_six = "T9.AKQJ87.4.A982"; // 14 HCP

    let mut off_agreements = Agreements::default();
    off_agreements.defense.direct_weak_jump_overcall = false;
    let (off, off_floored) = best_call_with(&off_agreements, &over_1d, weak_six);
    assert_eq!(off, call(1, Strain::Hearts));
    assert!(!off_floored, "the baseline simple overcall is authored");

    let on = Agreements::default();
    assert!(
        on.defense.direct_weak_jump_overcall,
        "the measured treatment ships by default"
    );
    let jump_rules = super::overcall::defense_to_suit(Bid::new(1, Strain::Diamonds), &on);
    let jump_rule = jump_rules
        .rules()
        .iter()
        .find(|rule| rule.call() == call(2, Strain::Hearts))
        .expect("the candidate authors the 2♥ jump");
    assert_eq!(
        jump_rule.alert(),
        Some(super::overcall::WEAK_JUMP_OVERCALL),
        "the weak range must be disclosed to the inference reader"
    );
    let (jump, jump_floored) = best_call_with(&on, &over_1d, weak_six);
    assert_eq!(jump, call(2, Strain::Hearts));
    assert!(!jump_floored, "the candidate jump is an authored book call");
    assert_eq!(
        best_call_with(&on, &over_1d, five).0,
        call(1, Strain::Hearts),
        "five-card hands retain the simple overcall"
    );
    assert_eq!(
        best_call_with(&on, &over_1d, strong_six).0,
        call(1, Strain::Hearts),
        "12+ HCP six-card hands retain the simple overcall"
    );

    // At the advancer's turn the authored projection is the disclosure: the
    // jump promises exactly six hearts, 8+ points, and at most 11 HCP.  The
    // call is natural, but its range-specific disclosure tag is what carries
    // both ceilings; no artificial continuation is needed, and the existing
    // preempt-advance floor consumes this reading.
    let read = crate::bidding::american::american(&on).bind().infer(
        RelativeVulnerability::NONE,
        &[
            call(1, Strain::Diamonds),
            call(2, Strain::Hearts),
            Call::Pass,
        ],
    );
    let overcaller = read.announced(Relative::Partner);
    assert_eq!(overcaller.length(Suit::Hearts).min, 6);
    assert_eq!(overcaller.length(Suit::Hearts).max, 6);
    assert!(overcaller.strength.points.min >= 8);
    assert!(overcaller.strength.hcp.max <= 11);

    let (advance, advance_floored) = best_call_with(
        &on,
        &[
            call(1, Strain::Diamonds),
            call(2, Strain::Hearts),
            Call::Pass,
        ],
        "A42.T94.AKJ2.Q63",
    );
    assert_eq!(
        advance,
        call(3, Strain::Hearts),
        "game values and three-card support advance the weak jump naturally"
    );
    assert!(
        advance_floored,
        "a natural preempt deliberately advances through the general floor"
    );
}

#[test]
fn direct_minor_weak_jump_is_exactly_one_club_two_diamonds() {
    use crate::bidding::Relative;
    use contract_bridge::Suit;

    let over_1c = [call(1, Strain::Clubs)];
    let weak_six = "T9.84.KQJ875.A97"; // 10 HCP, exactly six diamonds
    let too_weak_six = "T9.84.JT9875.Q97"; // below the 8-point floor
    let five = "T9.842.KQJ87.A97";
    let strong_six = "T9.Q4.AQJ987.K87"; // exactly 12 HCP
    let seven = "T9.8.KQJ8754.A97";

    let mut off = Agreements::default();
    off.defense.direct_minor_weak_jump_overcall = false;
    assert_eq!(
        best_call_with(&off, &over_1c, weak_six).0,
        call(1, Strain::Diamonds)
    );

    let mut on = off;
    on.defense.direct_minor_weak_jump_overcall = true;
    let jump_rules = super::overcall::defense_to_suit(Bid::new(1, Strain::Clubs), &on);
    let jump_rule = jump_rules
        .rules()
        .iter()
        .find(|rule| rule.call() == call(2, Strain::Diamonds))
        .expect("the candidate authors the 2♦ jump");
    assert_eq!(jump_rule.alert(), Some(super::overcall::WEAK_JUMP_OVERCALL),);
    assert_eq!(
        best_call_with(&on, &over_1c, weak_six).0,
        call(2, Strain::Diamonds)
    );
    assert_eq!(
        best_call_with(&on, &over_1c, five).0,
        call(1, Strain::Diamonds)
    );
    assert_ne!(
        best_call_with(&on, &over_1c, too_weak_six).0,
        call(2, Strain::Diamonds),
    );
    assert_eq!(
        best_call_with(&on, &over_1c, strong_six).0,
        call(1, Strain::Diamonds),
    );
    assert_ne!(
        best_call_with(&on, &over_1c, seven).0,
        call(2, Strain::Diamonds),
        "seven diamonds stay outside the exact-six weak jump",
    );

    for opening in Suit::ASC {
        let rules = super::overcall::defense_to_suit(Bid::new(1, opening.into()), &on);
        for rule in rules
            .rules()
            .iter()
            .filter(|rule| rule.alert() == Some(super::overcall::WEAK_JUMP_OVERCALL))
        {
            let Call::Bid(bid) = rule.call() else {
                panic!("a weak jump alert belongs on a bid")
            };
            assert_eq!(bid.level.get(), 2, "O3 adds no three-level weak jump");
            assert!(
                bid.strain != Strain::Diamonds || opening == Suit::Clubs,
                "the weak 2D alert exists only over 1C",
            );
        }
    }

    let read = crate::bidding::american::american(&on).bind().infer(
        RelativeVulnerability::NONE,
        &[
            call(1, Strain::Clubs),
            call(2, Strain::Diamonds),
            Call::Pass,
        ],
    );
    let overcaller = read.announced(Relative::Partner);
    assert_eq!(overcaller.length(Suit::Diamonds).min, 6);
    assert_eq!(overcaller.length(Suit::Diamonds).max, 6);
    assert!(overcaller.strength.points.min >= 8);
    assert!(overcaller.strength.hcp.max <= 11);

    let (advance, floored) = best_call_with(
        &on,
        &[
            call(1, Strain::Clubs),
            call(2, Strain::Diamonds),
            Call::Pass,
        ],
        "A42.T94.AKJ2.Q63",
    );
    assert_eq!(
        advance,
        call(2, Strain::Notrump),
        "the general floor chooses the natural preempt continuation",
    );
    assert!(floored, "the preempt continuation belongs to the floor");
}

#[test]
fn strong_double_hcp_repartitions_overcall_vs_double() {
    use crate::bidding::constraint::PointScale;
    // Calibrated to the rule-of-N+8 opt-out — the scale these example
    // hands' points assume (the 6-3-3-1 reads 18, not the point-count 17).
    let mut agreements = Agreements::default();
    agreements.decision.reading.point_scale = PointScale::RuleOfNFloored;
    // Over their (1♥): a shaped 17-HCP six-carder reads 18 points, which
    // overflows the shipped overcall band top (17) into the strong-tier
    // double — the point-count remnant's X↔bid seam (the forensic dump's
    // worst boards double first and lose to the natural 1♠).  The HCP
    // partition keeps it overcalling; a flat 19 still doubles first on
    // either setting ("too strong to overcall" = high cards, not shape).
    let over_1h = [call(1, Strain::Hearts)];
    let shaped = "AKQT42.A76.Q75.Q"; // 17 HCP, 18 points
    let flat = "AQ2.K42.KQ2.AJ32"; // 19 HCP, 3=3=3=4
    let (default_call, default_floored) = best_call_with(&agreements, &over_1h, shaped);
    assert_eq!(
        default_call,
        call(1, Strain::Spades),
        "default (HCP partition): the 17-HCP shaped hand overcalls"
    );
    assert!(!default_floored, "the overcall is a book node");
    let (strong_call, _) = best_call_with(&agreements, &over_1h, flat);
    assert_eq!(strong_call, Call::Double, "19 HCP still doubles first");

    let mut legacy_gauge = agreements;
    legacy_gauge.defense.strong_double_hcp = None;
    let (legacy_call, _) = best_call_with(&legacy_gauge, &over_1h, shaped);
    assert_eq!(
        legacy_call,
        Call::Double,
        "the points partition (off arm): 18 points reads as the strong tier"
    );
}

#[test]
fn nt_overcall_major_preference_respects_the_available_level() {
    let over_1d = [call(1, Strain::Diamonds)];
    let five_heart = "32.KQJ82.KQ4.A32"; // 15 HCP, 5332, 5 hearts, ♦ stopper
    let (default_call, _) = best_call(&over_1d, five_heart);
    assert_eq!(
        default_call,
        call(1, Strain::Notrump),
        "default buries the major in 1NT"
    );

    let mut no_major = Agreements::default();
    no_major.defense.nt_overcall_no_major = true;
    let (gated_call, _) = best_call_with(&no_major, &over_1d, five_heart);
    let mut cheap_major = Agreements::default();
    cheap_major.defense.nt_overcall_prefer_one_level_major = true;
    let (cheap_call, _) = best_call_with(&cheap_major, &over_1d, five_heart);
    // 15 HCP 4333, no 5M
    let (flat_call, _) = best_call_with(&no_major, &over_1d, "A32.KQ4.KQ4.J432");
    assert_eq!(
        gated_call,
        call(1, Strain::Hearts),
        "5-card major overcalls the suit"
    );
    assert_eq!(cheap_call, call(1, Strain::Hearts));
    assert_eq!(
        flat_call,
        call(1, Strain::Notrump),
        "no 5M still overcalls 1NT"
    );
    assert_eq!(
        best_call_with(&no_major, &over_1d, "AQ3.KJ4.KJ4.A432").0,
        call(1, Strain::Notrump),
        "the authored 18-HCP endpoint still overcalls 1NT",
    );
    assert_eq!(
        best_call_with(&no_major, &[call(1, Strain::Spades)], "AQJ82.K32.KQ4.32").0,
        call(1, Strain::Notrump),
        "strict permits five cards in opener's major",
    );

    // Hearts are unbid over (1♠), but only available at the two level.  The
    // cheap-major arm keeps 1NT; strict still shows hearts and wins if both are
    // accidentally selected.
    let over_1s = [call(1, Strain::Spades)];
    let five_hearts_over_spades = "K32.AQJ82.KQ4.32";
    assert_eq!(
        best_call_with(&cheap_major, &over_1s, five_hearts_over_spades).0,
        call(1, Strain::Notrump),
    );
    assert_eq!(
        best_call_with(&no_major, &over_1s, five_hearts_over_spades).0,
        call(2, Strain::Hearts),
    );
    let mut both = cheap_major;
    both.defense.nt_overcall_no_major = true;
    assert_eq!(
        best_call_with(&both, &over_1s, five_hearts_over_spades).0,
        call(2, Strain::Hearts),
    );
}

#[test]
fn nt_overcall_without_stopper_is_an_independent_balanced_arm() {
    let over_1s = [call(1, Strain::Spades)];
    let no_stopper = "T32.AQJ2.KQ4.K83"; // 15 HCP, 4333, no spade stopper
    let off = Agreements::default();
    assert_ne!(
        best_call_with(&off, &over_1s, no_stopper).0,
        call(1, Strain::Notrump),
    );

    let mut on = off;
    on.defense.nt_overcall_without_stopper = true;
    assert_eq!(
        best_call_with(&on, &over_1s, no_stopper).0,
        call(1, Strain::Notrump),
    );
}

#[test]
fn nt_overcall_opt_ins_keep_their_bidders_admitted() {
    use crate::bidding::Relative;

    let direct = [
        call(1, Strain::Spades),
        call(1, Strain::Notrump),
        Call::Pass,
    ];
    let systems_on = [
        call(1, Strain::Spades),
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Hearts),
        Call::Pass,
    ];
    let gladiator = [
        call(1, Strain::Spades),
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
    ];

    type Configure = fn(&mut Agreements);
    let cases: [(&str, Configure); 4] = [
        ("AQ3.KJ4.KJ4.A432", |_| {}), // authored 18-HCP endpoint
        ("AQJ82.K32.KQ4.32", |agreements| {
            agreements.defense.nt_overcall_no_major = true;
        }),
        ("K32.AQJ82.KQ4.32", |agreements| {
            agreements.defense.nt_overcall_prefer_one_level_major = true;
        }),
        ("T32.AQJ2.KQ4.K83", |agreements| {
            agreements.defense.nt_overcall_without_stopper = true;
        }),
    ];

    for (hand, configure) in cases {
        let hand: Hand = hand.parse().expect("valid test hand");
        let mut agreements = Agreements::default();
        configure(&mut agreements);

        agreements.decision.reading.nt_overcall_systems_on = false;
        let partnership = crate::bidding::american::american(&agreements).bind();
        assert!(
            partnership
                .infer(RelativeVulnerability::NONE, &direct)
                .admits(Relative::Partner, hand),
            "direct reading excludes {hand}",
        );

        agreements.decision.reading.nt_overcall_systems_on = true;
        let partnership = crate::bidding::american::american(&agreements).bind();
        assert!(
            partnership
                .infer(RelativeVulnerability::NONE, &systems_on)
                .admits(Relative::Partner, hand),
            "systems-on reading excludes {hand}",
        );

        agreements.decision.reading.nt_overcall_gladiator = true;
        let partnership = crate::bidding::american::american(&agreements).bind();
        assert!(
            partnership
                .infer(RelativeVulnerability::NONE, &gladiator)
                .admits(Relative::Partner, hand),
            "Gladiator reading excludes {hand}",
        );
    }
}

/// `DefenseKnobs::suppress_flat_4333_takeout` routes a weak flat 4-3-3-3 to Pass: a
/// 13-HCP 4-3-3-3 doubles their `1♦` by default, but stops doubling once the
/// knob is on (no ruffing value in a flat hand).
#[test]
fn suppress_flat_4333_takeout_routes_to_pass() {
    // 4♠-3♥-3♦-3♣, 13 HCP (AK + Q + Q + Q), short in their diamonds.
    let over_1d = [call(1, Strain::Diamonds)];
    let hand = "AKxx.Qxx.Qxx.Qxx";

    let mut off_arm = Agreements::default();
    off_arm.defense.suppress_flat_4333_takeout = false;
    let (off, _) = best_call_with(&off_arm, &over_1d, hand);
    assert_eq!(off, Call::Double, "flat 4333 doubles by default (knob off)");

    let mut on_arm = Agreements::default();
    on_arm.defense.suppress_flat_4333_takeout = true;
    let (on, _) = best_call_with(&on_arm, &over_1d, hand);
    assert_ne!(
        on,
        Call::Double,
        "knob on suppresses the takeout double on a weak flat 4333",
    );
}

/// `DefenseKnobs::suppress_4432_vs_major` routes a weak 4-4-3-2 to Pass **when the
/// opponents opened a major** (the worst 4-4-3-2 slice; a minimum double is
/// outgunned once they own a fit).  Over their `1♥` a 12-HCP 4♠-2♥-3♦-4♣
/// doubles by default; the knob routes it to Pass.  The vs-minor knob leaves
/// it alone (opener is a major).
#[test]
fn suppress_4432_vs_major_routes_to_pass() {
    // 4♠-2♥-3♦-4♣, 12 HCP (KQ + AK), short in their hearts.
    let over_1h = [call(1, Strain::Hearts)];
    let hand = "KQxx.xx.xxx.AKxx";

    let (off, _) = best_call(&over_1h, hand);
    assert_eq!(off, Call::Double, "4432 vs a major doubles by default");

    let mut minor_arm = Agreements::default();
    minor_arm.defense.suppress_4432_vs_minor = true;
    let (minor, _) = best_call_with(&minor_arm, &over_1h, hand);
    assert_eq!(
        minor,
        Call::Double,
        "the vs-minor knob leaves a major opening"
    );

    let mut major_arm = Agreements::default();
    major_arm.defense.suppress_4432_vs_major = true;
    let (on, _) = best_call_with(&major_arm, &over_1h, hand);
    assert_ne!(on, Call::Double, "vs-major knob suppresses the 4432 double");
}

/// `DefenseKnobs::suppress_4432_vs_minor` routes a weak 4-4-3-2 to Pass **when the
/// opponents opened a minor**, and the vs-major knob leaves it alone.
#[test]
fn suppress_4432_vs_minor_routes_to_pass() {
    // 4♠-4♥-3♦-2♣, 12 HCP (AK + KQ), short in their clubs (minor opening).
    let over_1c = [call(1, Strain::Clubs)];
    let hand = "AKxx.KQxx.xxx.xx";

    let (off, _) = best_call(&over_1c, hand);
    assert_eq!(off, Call::Double, "4432 vs a minor doubles by default");

    let mut major_arm = Agreements::default();
    major_arm.defense.suppress_4432_vs_major = true;
    let (major, _) = best_call_with(&major_arm, &over_1c, hand);
    assert_eq!(
        major,
        Call::Double,
        "the vs-major knob leaves a minor opening"
    );

    let mut minor_arm = Agreements::default();
    minor_arm.defense.suppress_4432_vs_minor = true;
    let (on, _) = best_call_with(&minor_arm, &over_1c, hand);
    assert_ne!(on, Call::Double, "vs-minor knob suppresses the 4432 double");
}

/// `DefenseKnobs::suppress_5332_takeout` (shipped default-on) routes a weak 5-3-3-2 off
/// the takeout double to its natural overcall — a 5-3-3-2 has no 4-card major
/// so the double cannot find a fit.  A 13-HCP 5♣ hand doubles their `1♦` with
/// the knob off; the default makes it bid the five-card suit instead.
#[test]
fn suppress_5332_takeout_bids_the_suit() {
    // 5♣-3♠-3♥-2♦, 13 HCP, short in their diamonds.
    let over_1d = [call(1, Strain::Diamonds)];
    let hand = "AQx.KJx.xx.QT9xx";

    let mut off_arm = Agreements::default();
    off_arm.defense.suppress_5332_takeout = false;
    let (off, _) = best_call_with(&off_arm, &over_1d, hand);
    assert_eq!(off, Call::Double, "5332 doubles with the knob off");

    let mut on_arm = Agreements::default();
    on_arm.defense.suppress_5332_takeout = true;
    let (on, _) = best_call_with(&on_arm, &over_1d, hand);
    assert_ne!(
        on,
        Call::Double,
        "the default suppresses the 5332 takeout double"
    );
}

/// `DefenseKnobs::suppress_5card_major_takeout` routes a hand with an unbid five-card
/// major off the takeout double to its natural overcall.  Over a weak two the
/// 12+ shapely double outguns the two-level major overcall; the knob prefers
/// the overcall — show the major rather than double into partner's short suit.
#[test]
fn suppress_5card_major_takeout_overcalls() {
    // 5♠-3♥-2♦-3♣, 15 HCP, short in their diamonds — over a weak 2♦.
    let over_2d = [call(2, Strain::Diamonds)];
    let hand = "AKQ62.J94.32.KJ6";

    let mut off_arm = Agreements::default();
    off_arm.defense.suppress_5card_major_takeout = false;
    let (off, _) = best_call_with(&off_arm, &over_2d, hand);
    assert_eq!(off, Call::Double, "5-card major doubles with the knob off");

    let mut on_arm = Agreements::default();
    on_arm.defense.suppress_5card_major_takeout = true;
    let (on, _) = best_call_with(&on_arm, &over_2d, hand);
    assert_eq!(
        on,
        call(2, Strain::Spades),
        "the knob overcalls the five-card major instead of doubling"
    );
}
