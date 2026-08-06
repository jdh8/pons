use crate::bidding::american::{
    LebensohlStyle, NotrumpDefense, american, set_advance_sohl_style, set_direct_landy_double,
    set_leaping_michaels, set_notrump_defense, set_unusual_notrump_defense,
    set_woolsey_double_floor, set_woolsey_points,
};
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain};

const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// The ported row package holds the row invariants (alerts; totality is
/// exact-node-exempt).
#[test]
fn row_package_invariants() {
    crate::bidding::rows::assert_package_invariants(&[
        super::weak_two_defense_package(),
        super::suit_defense_package(),
        super::notrump_defense_package(),
        super::landy_advance_package(),
        super::both_majors_double_package(),
        super::their_stayman_defense_package(),
        super::their_transfer_defense_package(),
        super::their_minor_transfer_defense_package(),
        super::their_diamond_transfer_defense_package(),
        super::unusual_notrump_advance_package(),
        super::direct_dont_advance_package(),
        super::meckwell_advance_package(),
        super::advance_double_package(),
        super::rich_advance_double_package(),
        super::responsive_double_package(),
        super::responsive_overcall_package(),
        super::weak_two_notrump_advance_package(),
        super::leaping_michaels_package(),
        super::woolsey_package(),
        super::advance_of_double_package(),
        super::gladiator_package(),
        super::gladiator_sohl_package(),
    ]);
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
    certify(
        &super::defense_to_suit(Bid::new(1, Strain::Hearts)),
        &hcp(18..),
    );
    // Legacy gauge: points(17..).
    super::set_strong_double_hcp(None);
    let legacy = super::defense_to_suit(Bid::new(1, Strain::Clubs));
    super::set_strong_double_hcp(Some(18));
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
    let baseline = super::defense_to_suit(their_opening);
    super::set_overcall_four_card(false);
    let explicit_off = super::defense_to_suit(their_opening);
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
    super::set_overcall_four_card(true);
    let on = super::defense_to_suit(their_opening).classify(hand, &context);
    super::set_overcall_four_card(false);
    assert_eq!(best(&on.0), one_s);
}

/// `american()`'s best call for a hand in an auction, and whether the instinct
/// floor (not a book node) produced it
fn best_call(auction: &[Call], hand: &str) -> (Call, bool) {
    let hand: Hand = hand.parse().expect("valid test hand");
    let (logits, prov) = american()
        .against()
        .classify_with_provenance(hand, RelativeVulnerability::NONE, auction)
        .expect("a legal auction classifies");
    let best = (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty");
    (best, prov.depth == 0 && prov.fallback.is_some())
}

/// [`best_call`] at a chosen vulnerability, for the rules that read one.
fn best_call_vul(auction: &[Call], hand: &str, vul: RelativeVulnerability) -> Call {
    let hand: Hand = hand.parse().expect("valid test hand");
    let (logits, _) = american()
        .against()
        .classify_with_provenance(hand, vul, auction)
        .expect("a legal auction classifies");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty")
}

#[test]
fn vulnerable_weak_two_overcall_demands_more() {
    // The shipped `set_weak_two_overcall_discipline` branch is invisible to
    // every other test in this file, because they all classify at
    // `RelativeVulnerability::NONE` — where the rule reduces to the flat
    // band it replaced.  This is the only check that reaches it.
    //
    // An 11-count with five diamonds: over their (2♠) that is a *three*
    // level overcall, which vulnerable wants 15 for, so it passes.
    let minimum = "J32.J32.KQT43.KJ";
    let over_2s = [call(2, Strain::Spades)];

    assert_eq!(
        best_call_vul(&over_2s, minimum, RelativeVulnerability::NONE),
        call(3, Strain::Diamonds),
        "non-vulnerable keeps the flat 10-16 band, so an 11-count overcalls"
    );
    assert_eq!(
        best_call_vul(&over_2s, minimum, RelativeVulnerability::WE),
        Call::Pass,
        "vulnerable at the three level needs 15, so the same hand passes"
    );

    // Their vulnerability is not ours: the discipline keys on `vulnerable()`
    // alone, which is what the `-v ns` cell of the A/B established.
    assert_eq!(
        best_call_vul(&over_2s, minimum, RelativeVulnerability::THEY),
        call(3, Strain::Diamonds),
        "only OUR vulnerability tightens the band"
    );
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

    super::set_two_level_minor_overcall_tight(true);
    let (tight_call, _) = best_call(&over_1s, minimum);
    // A 17-count is not silenced by the knob — it competes (a takeout X first).
    let (strong_call, _) = best_call(&over_1s, "A2.K2.Q432.AKJ87"); // 17 HCP
    super::set_two_level_minor_overcall_tight(false);
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
fn strong_double_hcp_repartitions_overcall_vs_double() {
    use crate::bidding::constraint::{PointScale, set_point_scale};
    // Calibrated to the rule-of-N+8 opt-out — the scale these example
    // hands' points assume (the 6-3-3-1 reads 18, not the point-count 17).
    set_point_scale(PointScale::RuleOfNFloored);
    // Over their (1♥): a shaped 17-HCP six-carder reads 18 points, which
    // overflows the shipped overcall band top (17) into the strong-tier
    // double — the point-count remnant's X↔bid seam (the forensic dump's
    // worst boards double first and lose to the natural 1♠).  The HCP
    // partition keeps it overcalling; a flat 19 still doubles first on
    // either setting ("too strong to overcall" = high cards, not shape).
    let over_1h = [call(1, Strain::Hearts)];
    let shaped = "AKQT42.A76.Q75.Q"; // 17 HCP, 18 points
    let flat = "AQ2.K42.KQ2.AJ32"; // 19 HCP, 3=3=3=4
    let (default_call, default_floored) = best_call(&over_1h, shaped);
    assert_eq!(
        default_call,
        call(1, Strain::Spades),
        "default (HCP partition): the 17-HCP shaped hand overcalls"
    );
    assert!(!default_floored, "the overcall is a book node");
    let (strong_call, _) = best_call(&over_1h, flat);
    assert_eq!(strong_call, Call::Double, "19 HCP still doubles first");

    super::set_strong_double_hcp(None);
    let (legacy_call, _) = best_call(&over_1h, shaped);
    super::set_strong_double_hcp(Some(18));
    assert_eq!(
        legacy_call,
        Call::Double,
        "the points partition (off arm): 18 points reads as the strong tier"
    );
    set_point_scale(PointScale::PointCount);
}

#[test]
fn two_suiter_hcp_floor_bars_garbage_michaels() {
    use crate::bidding::constraint::{PointScale, set_point_scale};
    // Calibrated to the rule-of-N+8 opt-out — the scale these example
    // hands' points assume (the 6-6 freak reads 9, not the point-count 7).
    set_point_scale(PointScale::RuleOfNFloored);
    // Over their (1♥): a 5-HCP 6-6 freak reads 9 points and cues Michaels
    // at weight 2.0 straight into a penalty double (−17..−21 IMPs a board
    // in the remnant dump).  The documented gate was always "8+ HCP"; the
    // floor makes it real, and the hand overcalls its spades instead.  A
    // sound 11-count 5-5 still cues.
    let over_1h = [call(1, Strain::Hearts)];
    let garbage = "KJ9532.5..JT7632"; // 5 HCP, 9 points
    let sound = "KQ953.5.2.AQ632"; // 11 HCP, 5-5
    let (default_call, _) = best_call(&over_1h, garbage);
    assert_eq!(
        default_call,
        call(1, Strain::Spades),
        "default: the floor bars the cue; the freak overcalls"
    );
    let (sound_call, _) = best_call(&over_1h, sound);
    assert_eq!(
        sound_call,
        call(2, Strain::Hearts),
        "a sound 5-5 still cues Michaels"
    );

    super::set_two_suiter_hcp_floor(None);
    let (legacy_call, _) = best_call(&over_1h, garbage);
    super::set_two_suiter_hcp_floor(Some(8));
    assert_eq!(
        legacy_call,
        call(2, Strain::Hearts),
        "the bare points gate (off arm): 9 points cue Michaels"
    );
    set_point_scale(PointScale::PointCount);
}

#[test]
fn longest_first_advance_bids_the_longer_suit() {
    // (1♣)–X–(P): 5 diamonds + 4 spades, a 7-HCP minimum.  This pins the
    // *flat* book (`--no-ns-rich-advance`): it scores both 4+ suits alike, so
    // the argmax bids the higher-ranking major 1♠; with the knob on, the
    // weight climbs with length and the longer diamonds win the advance.
    let over_1c = [call(1, Strain::Clubs), Call::Double, Call::Pass];
    let hand = "KJ32.32.K8765.32"; // 4 spades, 5 diamonds
    super::set_rich_advance_double(false);
    super::set_longest_first_advance(false);
    let (flat, _) = best_call(&over_1c, hand);
    assert_eq!(
        flat,
        call(1, Strain::Spades),
        "flat advance bids the higher-ranking 4-card major",
    );

    super::set_longest_first_advance(true);
    let (longest, floored) = best_call(&over_1c, hand);
    // Equal-length ties still break to the higher-ranking suit: 4-4 majors → 1♠
    // (the flat book has no invitational jump; the rich book would jump to 2M).
    let (tie, _) = best_call(&over_1c, "KJ32.KQ32.543.32");
    super::set_longest_first_advance(true); // restore defaults
    super::set_rich_advance_double(true);

    assert_eq!(
        longest,
        call(1, Strain::Diamonds),
        "longest-first bids the 5-card diamonds over the 4-card spades",
    );
    assert!(
        !floored,
        "the natural advance is a book node, not the floor"
    );
    assert_eq!(
        tie,
        call(1, Strain::Spades),
        "4-4 majors advance the higher-ranking spades",
    );
}

#[test]
fn longest_first_advance_governs_the_rich_book() {
    // The rich advance's *negative* suit picks obey the same discipline: the
    // weak natural suit and the forced-when-broke suit both go longest-first.
    let over_1c = [call(1, Strain::Clubs), Call::Double, Call::Pass];
    let over_1h = [call(1, Strain::Hearts), Call::Double, Call::Pass];
    super::set_rich_advance_double(true);
    super::set_longest_first_advance(false);

    // Weak two-suiter (7 HCP, 4 spades + 5 diamonds): flat rich advances the
    // higher-ranking 1♠, longest-first advances the longer 1♦.
    let two_suiter = "KJ32.32.K8765.32";
    let (flat_weak, _) = best_call(&over_1c, two_suiter);
    super::set_longest_first_advance(true);
    let (longest_weak, _) = best_call(&over_1c, two_suiter);
    super::set_longest_first_advance(false);

    // Forced bust (0 HCP, no 4-card suit outside their hearts): flat rich is
    // forced by the argmax into the higher-level 2♦; longest-first prefers the
    // higher-ranking — and cheaper — 1♠ among the equal 3-card suits.
    let bust = "432.5432.432.432";
    let (flat_forced, _) = best_call(&over_1h, bust);
    super::set_longest_first_advance(true);
    let (longest_forced, floored) = best_call(&over_1h, bust);

    super::set_longest_first_advance(true); // restore defaults
    super::set_rich_advance_double(true);

    assert_eq!(
        flat_weak,
        call(1, Strain::Spades),
        "flat rich weak → higher spades"
    );
    assert_eq!(
        longest_weak,
        call(1, Strain::Diamonds),
        "rich weak → longer diamonds"
    );
    assert_eq!(
        flat_forced,
        call(2, Strain::Diamonds),
        "flat rich forced → higher index"
    );
    assert_eq!(
        longest_forced,
        call(1, Strain::Spades),
        "rich forced → higher spades"
    );
    assert!(
        !floored,
        "the forced advance is a rich book node, not the floor"
    );
}

/// The forced rung's priority is the **cheapest bid**, not the highest
/// rank: with no 4-card suit outside theirs, the advance keeps the auction
/// as low as possible ([`cheapest_forced`]).
#[test]
fn forced_three_card_advance_bids_the_cheapest() {
    let over_1s = [call(1, Strain::Spades), Call::Double, Call::Pass];
    let over_1c = [call(1, Strain::Clubs), Call::Double, Call::Pass];
    super::set_rich_advance_double(true);
    super::set_longest_first_advance(true);

    // Broke with four small of their spades and 3-3-3 outside: no sit (no
    // top honors), no 4-card rung — the forced rung bids 2♣, not 2♥.
    let (forced, floored) = best_call(&over_1s, "5432.432.432.432");
    assert_eq!(forced, call(2, Strain::Clubs), "forced → cheapest 2♣");
    assert!(!floored, "the forced advance is a book node, not the floor");

    // Over (1♣) every advance sits at the one level, so cheapest means
    // lowest-ranking: 1♦, not 1♠.
    let (forced, _) = best_call(&over_1c, "432.432.432.5432");
    assert_eq!(forced, call(1, Strain::Diamonds), "forced → cheapest 1♦");
}

/// The weak sit yields to a 4-card unbid major under
/// [`set_advance_pass_yield_major`]: below the cue band the trump stack
/// bids the ladder; a 10+ hand or a majorless one sits as before.
#[test]
fn advance_pass_yields_to_a_major_only_when_weak() {
    let over_1c = [call(1, Strain::Clubs), Call::Double, Call::Pass];
    super::set_rich_advance_double(true);
    super::set_longest_first_advance(true);

    // 4 HCP, five clubs, four spades: sits by default...
    let stack = "KJ32.32.32.87654";
    let (sit, _) = best_call(&over_1c, stack);
    assert_eq!(sit, Call::Pass, "default: the weak stack sits");

    super::set_advance_pass_yield_major(true);
    // ...but yields to the spade major under the knob.
    let (yielded, _) = best_call(&over_1c, stack);
    assert_eq!(yielded, call(1, Strain::Spades), "yield: bid the major");

    // A cue-band sit (10 HCP) stands...
    let (strong, _) = best_call(&over_1c, "QJ32.Q32.2.KQ654");
    assert_eq!(strong, Call::Pass, "strong sit stands");

    // ...and so does a weak sit with no 4-card major.
    let (majorless, _) = best_call(&over_1c, "32.432.32.KJ8765");
    assert_eq!(majorless, Call::Pass, "majorless sit stands");

    // The flat book folds the same yield.
    super::set_rich_advance_double(false);
    let (flat, _) = best_call(&over_1c, "J432.32.32.KQ654");
    assert_eq!(flat, call(1, Strain::Spades), "flat book yields too");

    super::set_rich_advance_double(true);
    super::set_advance_pass_yield_major(false);
}

/// The 4-card sit's quality gate under [`set_advance_sit_hcp_gate`]:
/// `Some(5)` admits exactly AJxx (KJTx still advances), `Some(6)` drops
/// bare KQxx but keeps KQJx, and `None` keeps the shipped honor gate.
#[test]
fn advance_sit_hcp_gate_reshapes_the_4card_sit() {
    let over_1c = [call(1, Strain::Clubs), Call::Double, Call::Pass];
    super::set_rich_advance_double(true);

    // AJxx: 5 suit HCP but one top honor — forced 1♦ by default...
    let ajxx = "432.432.432.AJ32";
    let (default, _) = best_call(&over_1c, ajxx);
    assert_eq!(default, call(1, Strain::Diamonds), "default: AJxx advances");

    // ...sits under the 5+ floor...
    super::set_advance_sit_hcp_gate(Some(5));
    let (sits, _) = best_call(&over_1c, ajxx);
    assert_eq!(sits, Call::Pass, "5+ floor: AJxx sits");

    // ...while KJTx (4) still advances.
    let (kjtx, _) = best_call(&over_1c, "432.432.432.KJT2");
    assert_eq!(kjtx, call(1, Strain::Diamonds), "5+ floor: KJTx advances");

    // The 6+ floor drops bare KQxx (5) but keeps KQJx (6).
    super::set_advance_sit_hcp_gate(Some(6));
    let (kqxx, _) = best_call(&over_1c, "432.432.432.KQ32");
    assert_eq!(
        kqxx,
        call(1, Strain::Diamonds),
        "6+ floor: bare KQxx advances"
    );
    let (kqjx, _) = best_call(&over_1c, "432.432.432.KQJ2");
    assert_eq!(kqjx, Call::Pass, "6+ floor: KQJx sits");

    // Back to the honor gate: bare KQxx sits as shipped.
    super::set_advance_sit_hcp_gate(None);
    let (shipped, _) = best_call(&over_1c, "432.432.432.KQ32");
    assert_eq!(shipped, Call::Pass, "honor gate: KQxx sits");
}

/// The [`longest_unbid`] condition is an exact box union: `eval` and
/// knob-on box membership agree, opener's suit never competes, and an
/// equal-length tie goes to the higher rank — the reading the retired
/// weight ladder could never project.
#[test]
fn longest_unbid_reads_the_relative_length() {
    use super::Context;
    use crate::bidding::constraint::Constraint as _;
    use crate::bidding::inference::set_envelope_union_reading;
    use contract_bridge::Suit;

    set_envelope_union_reading(true);
    let context = Context::new(RelativeVulnerability::NONE, &[]);
    // The ♦ instance over their (1♥): rivals ♠ (higher rank, must stay
    // strictly shorter) and ♣ (lower rank, may equal).
    let diamonds = super::longest_unbid(Suit::Diamonds, Suit::Hearts);
    let boxes = diamonds.project(&context);

    // Holdings are spades.hearts.diamonds.clubs.
    for (hand, held, why) in [
        ("432.32.K8765.432", true, "5♦ over 3♠/3♣ is the longest"),
        ("432.32.K876.5432", true, "the ♦=♣ tie goes to the higher ♦"),
        (
            "5432.32.K876.432",
            false,
            "the ♠=♦ tie goes to the higher ♠",
        ),
        ("65432.32.K876.32", false, "5♠ out-lengths 4♦"),
        ("32.65432.K876.32", true, "opener's longer ♥ never competes"),
    ] {
        let hand: Hand = hand.parse().unwrap();
        assert_eq!(boxes.contains(hand), held, "boxes: {why}");
        assert_eq!(
            diamonds.eval(hand, &context).is_finite(),
            held,
            "eval: {why}"
        );
    }
}

#[test]
fn gladiator_club_three_way() {
    // Clubs split three ways by strength: a weak 6+♣ hand transfers via 2NT
    // (overcaller completes 3♣); an invitational 6+♣ hand goes 2♣→2♦→3♣; a
    // game-forcing club hand bids 3♣ directly.  Locks the user's structure.
    super::set_nt_overcall_gladiator(true);
    let s = || call(1, Strain::Spades);
    let nt = || call(1, Strain::Notrump);
    let p = Call::Pass;
    let c = |n| call(n, Strain::Clubs);
    let d2 = call(2, Strain::Diamonds);
    let (weak, _) = best_call(&[s(), nt(), p], "43.72.852.KJ9876"); // weak 6♣
    let (complete, _) = best_call(
        &[s(), nt(), p, call(2, Strain::Notrump), p],
        "AQ4.KQ4.AK92.65", // overcaller completes the transfer
    );
    let (gf, _) = best_call(&[s(), nt(), p], "A3.K2.42.AKQ9876"); // GF clubs
    let (relay, _) = best_call(&[s(), nt(), p], "43.72.K5.KQ9876"); // INV 6♣, 8 HCP
    let (pull, _) = best_call(&[s(), nt(), p, c(2), p, d2, p], "43.72.K5.KQ9876");
    super::set_nt_overcall_gladiator(false);
    assert_eq!(weak, call(2, Strain::Notrump), "weak 6♣ transfers via 2NT");
    assert_eq!(complete, c(3), "overcaller completes the club transfer");
    assert_eq!(gf, c(3), "game-forcing clubs bid 3♣ directly");
    assert_eq!(relay, c(2), "invitational 6♣ starts with the 2♣ relay");
    assert_eq!(pull, c(3), "invitational 6♣ pulls to 3♣ over the forced 2♦");
}

#[test]
fn nt_overcall_no_major_routes_five_card_major_to_the_suit() {
    // Over their (1♦): a 15-18 balanced hand with a five-card major overcalls
    // 1NT by default (burying the suit); the knob bars that so it overcalls
    // the major naturally, letting partner find the fit.
    let over_1d = [call(1, Strain::Diamonds)];
    let five_heart = "32.KQJ82.KQ4.A32"; // 15 HCP, 5332, 5 hearts, ♦ stopper
    let (default_call, _) = best_call(&over_1d, five_heart);
    assert_eq!(
        default_call,
        call(1, Strain::Notrump),
        "default buries the major in 1NT"
    );

    super::set_nt_overcall_no_major(true);
    let (gated_call, _) = best_call(&over_1d, five_heart);
    let (flat_call, _) = best_call(&over_1d, "A32.KQ4.KQ4.J432"); // 15 HCP 4333, no 5M
    super::set_nt_overcall_no_major(false);
    assert_eq!(
        gated_call,
        call(1, Strain::Hearts),
        "5-card major overcalls the suit"
    );
    assert_eq!(
        flat_call,
        call(1, Strain::Notrump),
        "no 5M still overcalls 1NT"
    );
}

#[test]
fn nt_overcall_systems_on_grafts_the_1nt_structure() {
    // Over their (1♦), our 1NT overcall runs systems on: the advancer plays
    // the opening-1NT responses verbatim.  Game-going 4-4 majors bid 2♣
    // Stayman (not a cue of their suit); a five-card spade suit transfers
    // (2♥ → spades), preserving right-siding — the whole point; the
    // overcaller answers Stayman with 4 hearts (2♥) from the grafted table.
    super::set_nt_overcall_systems_on(true);
    let d = || call(1, Strain::Diamonds);
    let nt = || call(1, Strain::Notrump);
    let (stayman, floored) = best_call(
        &[d(), nt(), Call::Pass],
        "A432.KQ84.32.QJ4", // 12 HCP, 4-4 majors
    );
    let (transfer, _) = best_call(
        &[d(), nt(), Call::Pass],
        "KQ432.K84.32.QJ4", // 10 HCP, 5 spades — Jacoby transfer, not Stayman
    );
    let (answer, _) = best_call(
        &[d(), nt(), Call::Pass, call(2, Strain::Clubs), Call::Pass],
        "Q3.KJ84.AQ54.KQ2", // 17 HCP, 4 hearts, ♦ stopper
    );
    super::set_nt_overcall_systems_on(false);
    assert_eq!(stayman, call(2, Strain::Clubs), "advancer bids 2♣ Stayman");
    assert!(
        !floored,
        "the grafted Stayman is a book node, not the floor"
    );
    assert_eq!(
        transfer,
        call(2, Strain::Hearts),
        "a five-card spade suit transfers"
    );
    assert_eq!(answer, call(2, Strain::Hearts), "overcaller shows 4 hearts");
}

#[test]
fn gladiator_replaces_the_major_graft() {
    // Over their (1♠), our 1NT overcall runs Gladiator (not systems-on): a
    // hand with exactly 4 hearts + invitational values cues 2♠ (Stayman for
    // the one unbid major); a weak hand takes the 2♣ relay; the overcaller
    // jumps to 4♥ over the cue with a maximum heart fit.
    super::set_nt_overcall_gladiator(true);
    let s = || call(1, Strain::Spades);
    let nt = || call(1, Strain::Notrump);
    let (cue, floored) = best_call(
        &[s(), nt(), Call::Pass],
        "K84.KQ84.QJ32.42", // 11 HCP, exactly 4 hearts
    );
    let (relay, _) = best_call(
        &[s(), nt(), Call::Pass],
        "432.J8.QJ543.J32", // 5 HCP, weak with 5♦ — the escape relay
    );
    let (flat, _) = best_call(
        &[s(), nt(), Call::Pass],
        "432.J84.J543.J32", // 3 HCP flat, no escape suit — passes 1NT
    );
    let (answer, _) = best_call(
        &[s(), nt(), Call::Pass, call(2, Strain::Spades), Call::Pass],
        "AQ.KQ84.AQ54.J32", // 18 HCP, 4 hearts, ♠ stopper — max fit
    );
    super::set_nt_overcall_gladiator(false);
    assert_eq!(
        cue,
        call(2, Strain::Spades),
        "advancer cues 2♠ = Stayman for hearts"
    );
    assert!(!floored, "the Gladiator cue is a book node, not the floor");
    assert_eq!(
        relay,
        call(2, Strain::Clubs),
        "a weak hand with a 5-card escape suit bids the 2♣ relay"
    );
    assert_eq!(
        flat,
        Call::Pass,
        "a flat weak hand passes 1NT, not the relay"
    );
    assert_eq!(
        answer,
        call(4, Strain::Hearts),
        "overcaller jumps to 4♥ with a maximum fit"
    );
}

#[test]
fn gladiator_over_2c_steals_the_relay_with_a_double() {
    // (1♠) 1NT (2♣): systems on, but it is Gladiator.  2♣ steals no room, so
    // the now-unbiddable relay reappears as X; every other advance keeps its
    // meaning, and the overcaller answers the stolen relay with the forced 2♦.
    super::set_nt_overcall_gladiator(true);
    let s = || call(1, Strain::Spades);
    let nt = || call(1, Strain::Notrump);
    let c2 = call(2, Strain::Clubs);
    let p = Call::Pass;
    // The weak 5♦ relay hand now doubles (the stolen relay).
    let (relay_x, _) = best_call(&[s(), nt(), c2], "432.J8.QJ543.J32");
    // Overcaller answers the stolen relay with the forced 2♦, as over 2♣.
    let (forced, _) = best_call(&[s(), nt(), c2, Call::Double, p], "AQ4.KQ4.AK92.65");
    // A cue-Stayman hand keeps cueing 2♠ (2♣ stole only the relay).
    let (cue, cue_floored) = best_call(&[s(), nt(), c2], "K84.KQ84.QJ32.42");
    super::set_nt_overcall_gladiator(false);
    assert_eq!(
        relay_x,
        Call::Double,
        "the stolen relay is shown with a Double"
    );
    assert_eq!(
        forced,
        call(2, Strain::Diamonds),
        "overcaller answers the stolen relay with the forced 2♦"
    );
    assert_eq!(
        cue,
        call(2, Strain::Spades),
        "the cue-Stayman survives systems-on"
    );
    assert!(
        !cue_floored,
        "the systems-on cue is a book node, not the floor"
    );
}

#[test]
fn advance_game_threshold_tracks_the_notrump_band() {
    // There is no invitational tier because eight opposite 16 is game
    // values — so the threshold has to *move* when the band's floor does,
    // or a widened band drives advancer to game on 23 total points.  Same
    // hand, same auction, one point of band: 8 HCP and five diamonds.
    let auction = [
        call(2, Strain::Hearts),
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    let hand = "K84.732.KQT43.42";

    super::set_weak_two_notrump_advances(true);
    let (at_16, _) = best_call(&auction, hand);
    super::set_weak_two_notrump_points(15, 17);
    let (at_15, _) = best_call(&auction, hand);
    super::set_weak_two_notrump_points(16, 17);
    super::set_weak_two_notrump_advances(false);

    assert_eq!(
        at_16,
        call(3, Strain::Diamonds),
        "opposite 16-17, eight is game values: force with the five-card suit"
    );
    assert_eq!(
        at_15,
        call(3, Strain::Clubs),
        "opposite 15-17 the same eight is not, so it relays for a partscore"
    );
}

#[test]
fn weak_two_notrump_advances_route_each_hand_class() {
    // Over their (2♥) our 2NT is 16–17 with a stopper, so eight opposite is
    // game values and there is no invitational tier: 3♣ or game.
    super::set_weak_two_notrump_advances(true);
    let auction = [
        call(2, Strain::Hearts),
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    let (stayman, stayman_floored) = best_call(&auction, "KQ85.732.K843.42");
    let (natural, _) = best_call(&auction, "A84.732.KQT43.42");
    let (relay, relay_floored) = best_call(&auction, "843.732.QT843.42");
    super::set_weak_two_notrump_advances(false);

    assert_eq!(
        stayman,
        call(3, Strain::Hearts),
        "exactly 4 spades with game values cues for Stayman"
    );
    assert!(!stayman_floored, "the cue is a book node, not the floor");
    assert_eq!(
        natural,
        call(3, Strain::Diamonds),
        "5 diamonds with game values bids them naturally"
    );
    assert_eq!(
        relay,
        call(3, Strain::Clubs),
        "a weak 5-card diamond hand relays instead of passing 2NT"
    );
    assert!(!relay_floored, "the relay is a book node, not the floor");
}

#[test]
fn weak_two_notrump_relay_lands_in_diamonds() {
    // 3♣ is forced to 3♦, which advancer passes — or cues with six-plus
    // diamonds to say 4♦ is safe.  Both halves must come off the book.
    super::set_weak_two_notrump_advances(true);
    let opening = || call(2, Strain::Hearts);
    let nt = || call(2, Strain::Notrump);
    let c = || call(3, Strain::Clubs);
    let d = || call(3, Strain::Diamonds);
    let (forced, forced_floored) = best_call(
        &[opening(), nt(), Call::Pass, c(), Call::Pass],
        "AQ2.KQ8.A54.K932", // the 16–17 overcall itself: reply is blind
    );
    let (sign_off, _) = best_call(
        &[
            opening(),
            nt(),
            Call::Pass,
            c(),
            Call::Pass,
            d(),
            Call::Pass,
        ],
        "843.732.QT843.42", // exactly five — play 3♦
    );
    let (cue, cue_floored) = best_call(
        &[
            opening(),
            nt(),
            Call::Pass,
            c(),
            Call::Pass,
            d(),
            Call::Pass,
        ],
        "84.732.QT8432.42", // six — 4♦ is safe
    );
    super::set_weak_two_notrump_advances(false);

    assert_eq!(forced, d(), "the relay reply is a forced 3♦");
    assert!(!forced_floored, "the forced reply is a book node");
    assert_eq!(sign_off, Call::Pass, "five diamonds plays 3♦");
    assert_eq!(
        cue,
        call(3, Strain::Hearts),
        "six diamonds cues to show 4♦ is safe"
    );
    assert!(!cue_floored, "the delayed cue is a book node");
}

#[test]
fn weak_two_notrump_relay_reads_as_diamonds_not_clubs() {
    // The phantom-suit guard.  `3♣` shows *diamonds*; if the natural walk
    // floors clubs off it, the floor raises a suit advancer does not hold
    // the moment the opponents come back in.  The alert is what suppresses
    // the walk, so this test is what proves the alert is wired.
    use crate::bidding::Relative;
    use contract_bridge::Suit;

    super::set_weak_two_notrump_advances(true);
    let read = american().against().infer(
        RelativeVulnerability::NONE,
        &[
            call(2, Strain::Hearts),
            call(2, Strain::Notrump),
            Call::Pass,
            call(3, Strain::Clubs),
        ],
    );
    let shown = read.announced(Relative::Rho);
    super::set_weak_two_notrump_advances(false);

    assert!(
        shown.length(Suit::Diamonds).min >= 5,
        "the relay reads as five-plus diamonds, got {:?}",
        shown.length(Suit::Diamonds)
    );
    assert_eq!(
        shown.length(Suit::Clubs).min,
        0,
        "the relay must promise no clubs at all"
    );
}

#[test]
fn gladiator_over_two_level_runs_transfer_lebensohl() {
    // Once RHO takes the two level there is no room for the relay tree, so
    // advancer plays the partnership's Transfer Lebensohl — book nodes, not
    // the floor.  Over (2♦) the 3♣-Stayman leg fires instead.
    super::set_nt_overcall_gladiator(true);
    let nt = || call(1, Strain::Notrump);
    let p = Call::Pass;
    // (1♠) 1NT (2♥): a weak long-diamond hand relays 2NT (→ 3♣ → correct);
    // the overcaller completes with the forced 3♣.
    let s = call(1, Strain::Spades);
    let h2 = call(2, Strain::Hearts);
    let (relay, relay_floored) = best_call(&[s, nt(), h2], "J2.43.KQ9876.32");
    let (complete, _) = best_call(
        &[s, nt(), h2, call(2, Strain::Notrump), p],
        "AQ4.A4.A32.KQ932",
    );
    // (1♥) 1NT (2♦): a 4-4-majors game-force takes the (2♦) 3♣-Stayman leg.
    let h = call(1, Strain::Hearts);
    let d2 = call(2, Strain::Diamonds);
    let (stayman, stayman_floored) = best_call(&[h, nt(), d2], "AQ32.KJ32.A2.432");
    super::set_nt_overcall_gladiator(false);
    assert_eq!(relay, call(2, Strain::Notrump), "weak long suit relays 2NT");
    assert!(
        !relay_floored,
        "the Lebensohl relay is a book node, not the floor"
    );
    assert_eq!(complete, call(3, Strain::Clubs), "overcaller completes 3♣");
    assert_eq!(
        stayman,
        call(3, Strain::Clubs),
        "the (2♦) leg bids 3♣-Stayman"
    );
    assert!(
        !stayman_floored,
        "the 3♣-Stayman is a book node, not the floor"
    );
}

#[test]
fn gladiator_continuations_reach_game() {
    // The completed book must drive game-forcing advances to game rather than
    // dying in the floor's partscore.  (1♠) 1NT, cue 2♠, min-fit answer 3♥:
    // a game-forcing advancer raises to 4♥.  And a game-forcing natural 3♥
    // (5+ hearts) is raised to 4♥ by the overcaller's heart fit.
    super::set_nt_overcall_gladiator(true);
    let s = || call(1, Strain::Spades);
    let nt = || call(1, Strain::Notrump);
    let h = |n| call(n, Strain::Hearts);
    let (place, _) = best_call(
        &[
            s(),
            nt(),
            Call::Pass,
            call(2, Strain::Spades),
            Call::Pass,
            h(3),
            Call::Pass,
        ],
        "K84.KQ84.KJ32.42", // 12 HCP, 4 hearts, GF — over a min fit, bid game
    );
    let (raise, floored) = best_call(
        &[s(), nt(), Call::Pass, h(3), Call::Pass],
        "AQ2.KQ8.AQ54.K93", // 18 HCP, 3 hearts — raise the GF 3♥ to game
    );
    super::set_nt_overcall_gladiator(false);
    assert_eq!(place, h(4), "GF advancer raises the min fit to 4♥");
    assert!(
        !floored,
        "the overcaller's raise is a book node, not the floor"
    );
    assert_eq!(raise, h(4), "overcaller raises the game-forcing 3♥ to 4♥");
}

#[test]
fn gladiator_delayed_cue_finds_the_five_three_fit() {
    // A (1♠) 1NT overcall may hold a balanced 5-card heart suit.  An advancer
    // with exactly 3 hearts, INV+, and a doubleton (NOT flat 4333, so it has
    // ruffing value) routes 2♣ relay → forced 2♦ → 2♠ (delayed cue) to check
    // the 5-3 fit the direct cue (promising 4) would miss; the overcaller with
    // 5 hearts and a maximum jumps to 4♥.
    super::set_nt_overcall_gladiator(true);
    let s = || call(1, Strain::Spades);
    let nt = || call(1, Strain::Notrump);
    let c = || call(2, Strain::Clubs);
    let d = || call(2, Strain::Diamonds);
    let (cue, floored) = best_call(
        &[s(), nt(), Call::Pass, c(), Call::Pass, d(), Call::Pass],
        "84.KJ8.KQ32.QJ32", // 12 HCP, exactly 3 hearts, doubleton ♠ — not 4333
    );
    let (answer, _) = best_call(
        &[
            s(),
            nt(),
            Call::Pass,
            c(),
            Call::Pass,
            d(),
            Call::Pass,
            call(2, Strain::Spades),
            Call::Pass,
        ],
        "AQ2.KQ842.AK5.32", // 18 HCP, 5 hearts, ♠ stopper — max fit
    );
    super::set_nt_overcall_gladiator(false);
    assert_eq!(
        cue,
        call(2, Strain::Spades),
        "exactly-3-heart non-flat advancer delayed-cues 2♠"
    );
    assert!(!floored, "the delayed cue is a book node, not the floor");
    assert_eq!(
        answer,
        call(4, Strain::Hearts),
        "overcaller with 5 hearts + a maximum jumps to 4♥"
    );
}

#[test]
fn gladiator_cues_barred_with_flat_4333() {
    // The 4333 curse: a flat (4333) has no ruffing value, so neither cue is
    // made — it invites/plays notrump instead of chasing a major fit.
    super::set_nt_overcall_gladiator(true);
    let s = || call(1, Strain::Spades);
    let nt = || call(1, Strain::Notrump);
    // Direct cue barred: flat 4333 with exactly 4 hearts, GF → 3NT, not 2♠.
    let (direct, _) = best_call(
        &[s(), nt(), Call::Pass],
        "K84.KQ84.K84.Q84", // 13 HCP, 3-4-3-3 flat, 4 hearts
    );
    // Delayed cue barred: flat 4333 with exactly 3 hearts, INV → 2NT relay-invite.
    let (delayed, _) = best_call(
        &[
            s(),
            nt(),
            Call::Pass,
            call(2, Strain::Clubs),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
        ],
        "J843.KJ8.Q84.Q84", // 9 HCP, 4-3-3-3 flat, 3 hearts
    );
    super::set_nt_overcall_gladiator(false);
    assert_eq!(
        direct,
        call(3, Strain::Notrump),
        "flat 4333 with 4 hearts bids 3NT, not the direct cue"
    );
    assert_eq!(
        delayed,
        call(2, Strain::Notrump),
        "flat 4333 with 3 hearts invites 2NT, not the delayed cue"
    );
}

/// Coupling: a Landy range feeds the one shared two-suiter band, so Landy's and
/// Woolsey's identical both-majors `2♣` can never carry divergent strengths.
#[test]
fn landy_range_feeds_the_shared_woolsey_band() {
    super::set_landy(Some((9, 16)));
    assert_eq!(
        super::woolsey_points(),
        (9, 16),
        "a Landy range sets the shared band"
    );
    // Turning Landy off must not clobber an explicit Woolsey band.
    set_woolsey_points(7, 18);
    super::set_landy(None);
    assert_eq!(
        super::woolsey_points(),
        (7, 18),
        "set_landy(None) leaves the band alone"
    );
    // Restore the default for any sibling test sharing this thread.
    set_woolsey_points(8, 19);
}

/// Per-call exclusivity: in every named 1NT-defense config the `[1NT]` node
/// authors at most one rule per call.  This is the invariant the alert-tag gate
/// must preserve — two rules on one call would let a hand fire the wrong
/// convention and a reading mis-decode it (e.g. a natural overcall leaking onto
/// a slot an artificial alert owns).  Pass is always authored (these configs all
/// own the auction).
#[test]
fn defense_to_notrump_authors_one_rule_per_call() {
    fn reset() {
        super::set_notrump_defense(NotrumpDefense::Natural);
        super::set_landy(None);
        super::set_unusual_notrump_defense(Some((8, 13)));
    }

    let configs: [(&str, fn()); 7] = [
        ("natural+unusual2nt", || {}),
        ("natural+landy", || super::set_landy(Some((8, 15)))),
        ("woolsey", || {
            super::set_notrump_defense(NotrumpDefense::Woolsey)
        }),
        ("dont", || {
            super::set_notrump_defense(NotrumpDefense::DirectDont)
        }),
        ("meckwell", || {
            super::set_notrump_defense(NotrumpDefense::Meckwell)
        }),
        ("direct-landy-x", || {
            super::set_direct_landy_double(Some(false))
        }),
        ("always-pass", || {
            super::set_notrump_defense(NotrumpDefense::AlwaysPass)
        }),
    ];

    for (label, setup) in configs {
        reset();
        setup();
        let calls: Vec<Call> = super::defense_to_notrump()
            .rules()
            .iter()
            .map(|r| r.call())
            .collect();
        reset();
        assert!(
            calls.contains(&Call::Pass),
            "{label}: the owning Pass is missing",
        );
        for i in 0..calls.len() {
            for j in (i + 1)..calls.len() {
                assert!(
                    calls[i] != calls[j],
                    "{label}: call {:?} authored by two rules at the [1NT] node",
                    calls[i],
                );
            }
        }
    }
}

/// Best call with the advance-of-double sohl forced to `style` (independent of
/// any other test on this thread having changed it)
fn advance(style: LebensohlStyle, auction: &[Call], hand: &str) -> (Call, bool) {
    set_advance_sohl_style(style);
    best_call(auction, hand)
}

/// `(2♦)–X–(P)` — partner doubled their weak two, advancer to act
fn over_2d() -> [Call; 3] {
    [call(2, Strain::Diamonds), Call::Double, Call::Pass]
}

#[test]
fn off_keeps_the_flat_advance_no_relay() {
    // Default Off: a weak six-club hand bids the natural 3♣ (advance_double),
    // not the 2NT relay — the toggle gates the new structure.
    let (c, _) = advance(LebensohlStyle::Off, &over_2d(), "32.43.32.KQ9876");
    assert_eq!(c, call(3, Strain::Clubs));
}

#[test]
fn plain_weak_long_suit_relays_then_completes() {
    // Plain: weak hand (6 HCP), six clubs → 2NT relay; doubler forced to 3♣.
    let (c, floored) = advance(LebensohlStyle::Plain, &over_2d(), "J2.43.32.KQ9876");
    assert_eq!(c, call(2, Strain::Notrump));
    assert!(!floored, "the relay must come from the book");

    let relayed = [
        call(2, Strain::Diamonds),
        Call::Double,
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    let (completion, _) = advance(LebensohlStyle::Plain, &relayed, "AKJ2.KQ52.4.A532");
    assert_eq!(completion, call(3, Strain::Clubs));
}

#[test]
fn plain_forcing_three_level_is_a_book_node() {
    // Plain: five spades and game values → forcing 3♠ (a jump over 2♦),
    // never a weak partscore.
    let (c, floored) = advance(LebensohlStyle::Plain, &over_2d(), "KQT95.A43.32.J32");
    assert_eq!(c, call(3, Strain::Spades));
    assert!(!floored, "the forcing 3-level bid must come from the book");
}

#[test]
fn transfer_shows_spades_through_their_hearts() {
    // Transfer: over (2♥), five spades and game values transfer *through*
    // hearts — 3♦ shows spades (not diamonds), a book node.
    let over_2h = [call(2, Strain::Hearts), Call::Double, Call::Pass];
    let (c, floored) = advance(LebensohlStyle::Transfer, &over_2h, "AKQ65.43.K32.J32");
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the transfer must come from the book");
}

#[test]
fn transfer_doubler_bids_game_not_partscore() {
    // After (2♥)–X–(P)–3♦ (transfer to spades), the doubler with a fit bids
    // the spade *game*, never a 3♠ partscore.
    let auction = [
        call(2, Strain::Hearts),
        Call::Double,
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, _) = advance(LebensohlStyle::Transfer, &auction, "AK52.4.A432.K432");
    assert_eq!(c, call(4, Strain::Spades));
}

#[test]
fn transfer_cue_is_stayman() {
    // (2♥)–X–(P)–3♥ is the cue = Stayman; the doubler shows a 4-card major.
    // (Over (2♦) the cue slot is freed for the Smolen 3♣-Stayman instead.)
    let auction = [
        call(2, Strain::Hearts),
        Call::Double,
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let (c, floored) = advance(LebensohlStyle::Transfer, &auction, "AQ32.K32.4.KJ432");
    assert_eq!(c, call(3, Strain::Spades));
    assert!(!floored, "the Stayman answer must come from the book");
}

#[test]
fn penalty_pass_sits_for_the_double() {
    // A trump stack in their suit (five spades over 2♠) has no constructive
    // call — the book's terminal Pass leaves the takeout double in for
    // penalty, exactly as the flat ladder would.
    let over_2s = [call(2, Strain::Spades), Call::Double, Call::Pass];
    let (c, floored) = advance(LebensohlStyle::Plain, &over_2s, "KQJ95.J32.432.32");
    assert_eq!(c, Call::Pass);
    assert!(!floored, "the sign-off Pass must come from the book node");
}

#[test]
fn transfer_over_2d_is_three_club_stayman() {
    // (2♦)–X–(P): Transfer's (2♦)-only Smolen leg bids 3♣-Stayman for a 4-4
    // majors GF advancer, a book node (over (2♥)/(2♠) it is plain Cohen, whose
    // 3♣ is not Stayman).
    let (c, floored) = advance(LebensohlStyle::Transfer, &over_2d(), "AQ32.KJ32.A2.432");
    assert_eq!(c, call(3, Strain::Clubs));
    assert!(!floored, "the Stayman bid must come from the book");
}

#[test]
fn always_pass_defense_passes_over_1nt() {
    // The always-pass baseline: a 15-count balanced hand that would normally make a
    // penalty double passes instead, and the Pass is a book node (not the floor)
    // so it shadows whatever the floor would have done over their 1NT.
    let over_1nt = [call(1, Strain::Notrump)];
    set_notrump_defense(NotrumpDefense::AlwaysPass);
    let (c, floored) = best_call(&over_1nt, "AQ32.KQ3.K32.Q32");
    set_notrump_defense(NotrumpDefense::Natural);
    assert_eq!(c, Call::Pass);
    assert!(!floored, "the always-pass must come from the book node");
}

/// `set_suppress_flat_4333_takeout` routes a weak flat 4-3-3-3 to Pass: a
/// 13-HCP 4-3-3-3 doubles their `1♦` by default, but stops doubling once the
/// opt-in knob is on (no ruffing value in a flat hand).  Reset the knob so it
/// cannot leak into a sibling test on this thread.
#[test]
fn suppress_flat_4333_takeout_routes_to_pass() {
    // 4♠-3♥-3♦-3♣, 13 HCP (AK + Q + Q + Q), short in their diamonds.
    let over_1d = [call(1, Strain::Diamonds)];
    let hand = "AKxx.Qxx.Qxx.Qxx";

    crate::bidding::constraint::set_suppress_flat_4333_takeout(false);
    let (off, _) = best_call(&over_1d, hand);
    assert_eq!(off, Call::Double, "flat 4333 doubles by default (knob off)");

    crate::bidding::constraint::set_suppress_flat_4333_takeout(true);
    let (on, _) = best_call(&over_1d, hand);
    crate::bidding::constraint::set_suppress_flat_4333_takeout(false);
    assert_ne!(
        on,
        Call::Double,
        "knob on suppresses the takeout double on a weak flat 4333",
    );
}

/// `set_suppress_4432_vs_major` routes a weak 4-4-3-2 to Pass **when the
/// opponents opened a major** (the worst 4-4-3-2 slice; a minimum double is
/// outgunned once they own a fit).  Over their `1♥` a 12-HCP 4♠-2♥-3♦-4♣
/// doubles by default; the knob routes it to Pass.  The vs-minor knob leaves
/// it alone (opener is a major).  Reset so it cannot leak into a sibling.
#[test]
fn suppress_4432_vs_major_routes_to_pass() {
    // 4♠-2♥-3♦-4♣, 12 HCP (KQ + AK), short in their hearts.
    let over_1h = [call(1, Strain::Hearts)];
    let hand = "KQxx.xx.xxx.AKxx";

    let (off, _) = best_call(&over_1h, hand);
    assert_eq!(off, Call::Double, "4432 vs a major doubles by default");

    crate::bidding::constraint::set_suppress_4432_vs_minor(true);
    let (minor, _) = best_call(&over_1h, hand);
    crate::bidding::constraint::set_suppress_4432_vs_minor(false);
    assert_eq!(
        minor,
        Call::Double,
        "the vs-minor knob leaves a major opening"
    );

    crate::bidding::constraint::set_suppress_4432_vs_major(true);
    let (on, _) = best_call(&over_1h, hand);
    crate::bidding::constraint::set_suppress_4432_vs_major(false);
    assert_ne!(on, Call::Double, "vs-major knob suppresses the 4432 double");
}

/// `set_suppress_4432_vs_minor` routes a weak 4-4-3-2 to Pass **when the
/// opponents opened a minor**, and the vs-major knob leaves it alone.
#[test]
fn suppress_4432_vs_minor_routes_to_pass() {
    // 4♠-4♥-3♦-2♣, 12 HCP (AK + KQ), short in their clubs (minor opening).
    let over_1c = [call(1, Strain::Clubs)];
    let hand = "AKxx.KQxx.xxx.xx";

    let (off, _) = best_call(&over_1c, hand);
    assert_eq!(off, Call::Double, "4432 vs a minor doubles by default");

    crate::bidding::constraint::set_suppress_4432_vs_major(true);
    let (major, _) = best_call(&over_1c, hand);
    crate::bidding::constraint::set_suppress_4432_vs_major(false);
    assert_eq!(
        major,
        Call::Double,
        "the vs-major knob leaves a minor opening"
    );

    crate::bidding::constraint::set_suppress_4432_vs_minor(true);
    let (on, _) = best_call(&over_1c, hand);
    crate::bidding::constraint::set_suppress_4432_vs_minor(false);
    assert_ne!(on, Call::Double, "vs-minor knob suppresses the 4432 double");
}

/// `set_suppress_5332_takeout` (shipped default-on) routes a weak 5-3-3-2 off
/// the takeout double to its natural overcall — a 5-3-3-2 has no 4-card major
/// so the double cannot find a fit.  A 13-HCP 5♣ hand doubles their `1♦` with
/// the knob off; the default makes it bid the five-card suit instead.
#[test]
fn suppress_5332_takeout_bids_the_suit() {
    // 5♣-3♠-3♥-2♦, 13 HCP, short in their diamonds.
    let over_1d = [call(1, Strain::Diamonds)];
    let hand = "AQx.KJx.xx.QT9xx";

    crate::bidding::constraint::set_suppress_5332_takeout(false);
    let (off, _) = best_call(&over_1d, hand);
    assert_eq!(off, Call::Double, "5332 doubles with the knob off");

    crate::bidding::constraint::set_suppress_5332_takeout(true);
    let (on, _) = best_call(&over_1d, hand);
    crate::bidding::constraint::set_suppress_5332_takeout(false);
    assert_ne!(
        on,
        Call::Double,
        "the default suppresses the 5332 takeout double"
    );
}

/// `set_suppress_5card_major_takeout` routes a hand with an unbid five-card
/// major off the takeout double to its natural overcall.  Over a weak two the
/// 12+ shapely double outguns the two-level major overcall; the knob prefers
/// the overcall — show the major rather than double into partner's short suit.
#[test]
fn suppress_5card_major_takeout_overcalls() {
    // 5♠-3♥-2♦-3♣, 15 HCP, short in their diamonds — over a weak 2♦.
    let over_2d = [call(2, Strain::Diamonds)];
    let hand = "AKQ62.J94.32.KJ6";

    crate::bidding::constraint::set_suppress_5card_major_takeout(false);
    let (off, _) = best_call(&over_2d, hand);
    assert_eq!(off, Call::Double, "5-card major doubles with the knob off");

    crate::bidding::constraint::set_suppress_5card_major_takeout(true);
    let (on, _) = best_call(&over_2d, hand);
    crate::bidding::constraint::set_suppress_5card_major_takeout(false);
    assert_eq!(
        on,
        call(2, Strain::Spades),
        "the knob overcalls the five-card major instead of doubling"
    );
}

/// The rich advance gives the advancer a cue (invite+) and a forced 3-card
/// response when broke — both absent from the flat floor.
#[test]
fn rich_advance_double_cues_and_forces() {
    // (1♥) X (P) ? — advancer to act.
    let auction = [call(1, Strain::Hearts), Call::Double, Call::Pass];

    // 15 HCP, 4-3-3-3 with 4 spades but no heart stopper: game-forcing with
    // no limited natural home (3NT wants a stopper, 4♠ wants five) — cue.
    let force = "AQxx.xxx.AQx.Kxx";
    // 0 HCP, 3-4-3-3: no 4-card suit outside hearts, no trump stack — must
    // still bid (a takeout double cannot be passed for want of a call).
    let broke = "xxx.xxxx.xxx.xxx";

    super::set_rich_advance_double(true);
    let (cued, _) = best_call(&auction, force);
    let (forced, _) = best_call(&auction, broke);
    super::set_rich_advance_double(false);
    let (flat_force, _) = best_call(&auction, force);
    super::set_rich_advance_double(true); // restore default

    assert_eq!(
        cued,
        call(2, Strain::Hearts),
        "a game force with no limited natural home cues opener's suit"
    );
    assert!(
        matches!(forced, Call::Bid(_)),
        "a broke advancer bids rather than passing partner's takeout (got {forced:?})"
    );
    assert_ne!(
        flat_force,
        call(2, Strain::Hearts),
        "the flat floor has no cue-bid advance"
    );
}

/// The cue is invitational-or-better; the advancer clarifies over the
/// doubler's minimum answer — a game force reaches game, an invite stops.
#[test]
fn advance_cue_rebid_forces_or_invites() {
    // (1♥) X (P) 2♥cue (P) 2♠min (P) ? — advancer to clarify, with a known
    // spade fit from partner's minimum cheap-major answer.
    let auction = [
        call(1, Strain::Hearts),
        Call::Double,
        Call::Pass,
        call(2, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ];
    // 15 HCP with 3 spades: game force — raise partner's spades to game.
    let force = "Axx.xxx.AKQx.Qxx";
    // 10 HCP with 3 spades: mere invite — partner showed a minimum, so stop.
    let invite = "Axx.xxx.AJxx.xxx";

    super::set_rich_advance_double(true);
    let (driven, _) = best_call(&auction, force);
    let (rested, _) = best_call(&auction, invite);
    super::set_rich_advance_double(true); // restore default

    assert_eq!(
        driven,
        call(4, Strain::Spades),
        "a game-forcing advancer raises the cue answer to game"
    );
    assert_eq!(
        rested,
        Call::Pass,
        "an invitational advancer passes the doubler's minimum"
    );
}

/// Without a Rubens transfer, `4M` is two-way: a shapely *weak* hand blasts
/// game just like a minimum game force (11–15 points), so a long-major
/// preempt is not stranded below a makeable game (the advance-double-v5
/// regression: pure MIN-FG `hcp(12..=15)` missed it).
#[test]
fn rich_advance_weak_shapely_blasts_game() {
    // (1♠) X (P) ? — advancer with a weak two-suiter, six hearts.
    let auction = [call(1, Strain::Spades), Call::Double, Call::Pass];
    // 8 HCP, 6-4 in hearts and clubs: too weak to invite (3♥ wants 10+), but
    // opposite a takeout double the shapely hand belongs in 4♥.
    let weak = "T3.AT9753.5.KQT7";

    super::set_rich_advance_double(true);
    let (blast, _) = best_call(&auction, weak);
    super::set_rich_advance_double(true); // restore default

    assert_eq!(
        blast,
        call(4, Strain::Hearts),
        "a shapely weak long-major hand blasts the two-way 4M, not a quiet 2-level"
    );
}

/// The invitational minor jump (`set_advance_minor_jump`): a three-level
/// minor jump shows a 5+ one-suiter (10–12) denying a 4-card unbid major.
/// With a 4-card major the advancer cues to find the fit; with a stopper it
/// prefers notrump (the jump ranks below the notrump ladder).
#[test]
fn advance_minor_jump_shows_invitational_one_suiter() {
    // (1♥) X (P) ? — advancer to act; the unbid major is spades.
    let auction = [call(1, Strain::Hearts), Call::Double, Call::Pass];
    super::set_rich_advance_double(true);
    super::set_advance_minor_jump(true);

    // 11 HCP, 5 diamonds, 3 spades (no 4-card major), no heart stopper: an
    // invitational one-suiter → jump 3♦.
    let one_suiter = "xxx.xx.AQJxx.KJx";
    let (jump, _) = best_call(&auction, one_suiter);
    // 11 HCP, 4 spades + 5 diamonds, no heart stopper: a 4-card unbid major →
    // cue 2♥ to find the fit, not the minor jump.
    let with_major = "KQxx.xx.AQxxx.xx";
    let (cued, _) = best_call(&auction, with_major);
    // 11 HCP, 5 diamonds, heart stopper, balanced: a minor needs eleven
    // tricks, so it prefers 2NT — the jump ranks below the notrump ladder.
    let stopped = "Qxx.KJx.AJxxx.xx";
    let (notrump, _) = best_call(&auction, stopped);

    super::set_advance_minor_jump(false);
    // Knob off: the same one-suiter has no minor jump, so it cues instead of
    // jumping.
    let (off, _) = best_call(&auction, one_suiter);
    super::set_advance_minor_jump(true); // restore default-on
    super::set_rich_advance_double(true); // restore rich default

    assert_eq!(
        jump,
        call(3, Strain::Diamonds),
        "invitational 5+ minor, no major → 3♦ jump"
    );
    assert_eq!(
        cued,
        call(2, Strain::Hearts),
        "a 4-card unbid major cues, not the minor jump"
    );
    assert_eq!(
        notrump,
        call(2, Strain::Notrump),
        "a stopper prefers notrump over the minor jump"
    );
    assert_ne!(off, call(3, Strain::Diamonds), "knob off → no minor jump");
}

/// The doubler's accept/decline of the minor jump, and the advancer's
/// placement over the doubler's forcing new suit: the limited invite lets the
/// doubler pass (unlike the forcing cue), accept a game-forcing 5+ suit, or
/// bid `3NT` to play — then the advancer raises the shown suit with support.
#[test]
fn doubler_accepts_or_declines_the_minor_jump() {
    // (1♠) X (P) 3♦ (P) ? — doubler acts over the invitational 3♦ jump; the
    // unbid major is hearts.
    let jump = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    super::set_rich_advance_double(true);
    super::set_advance_minor_jump(true);

    // Maximum with a 5-card heart suit: accept by showing it, game-forcing.
    let max_major = "x.AKQxx.Kxx.AQxx"; // 18 HCP, 5♥, no spade stopper
    let (accept, _) = best_call(&jump, max_major);
    // Balanced maximum with a spade stopper, no 5-card suit: 3NT to play.
    let max_flat = "KQx.AJx.Qxx.KQxx"; // 17 HCP, spade stopper
    let (notrump, _) = best_call(&jump, max_flat);
    // Minimum takeout double: decline the limited invitation.
    let minimum = "Kxxx.Qxx.xx.Kxxx"; // ~8 HCP, minimum
    let (decline, _) = best_call(&jump, minimum);

    // Advancer places game over the doubler's forcing 3♥: raise with support.
    let after_major = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let with_fit = "xx.Kxx.AQJxx.xxx"; // 10 HCP, 5♦, 3♥ support (a valid 3♦ jump)
    let (raise, _) = best_call(&after_major, with_fit);

    super::set_advance_minor_jump(true); // restore default-on
    super::set_rich_advance_double(true); // restore rich default

    assert_eq!(
        accept,
        call(3, Strain::Hearts),
        "maximum shows a new 5+ suit, game-forcing"
    );
    assert_eq!(
        notrump,
        call(3, Strain::Notrump),
        "balanced maximum accepts 3NT to play"
    );
    assert_eq!(decline, Call::Pass, "minimum declines the limited invite");
    assert_eq!(
        raise,
        call(4, Strain::Hearts),
        "advancer raises the doubler's shown major to game"
    );
}

/// The doubler's stopper-ask cue over the minor jump: with game values but no
/// stopper (and no biddable side suit) the doubler cues their suit; the
/// advancer bids `3NT` with a stopper (right-sided) or signs off in the minor
/// game.
#[test]
fn doubler_stopper_ask_over_the_minor_jump() {
    // (1♠) X (P) 3♣ (P) ? — doubler acts over the invitational 3♣ jump.
    let jump = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    super::set_rich_advance_double(true);
    super::set_advance_minor_jump(true);

    // 18 HCP, no spade stopper, no 5-card side suit: cue 3♠ to ask.
    let ask = "xxx.AKx.AQx.KQxx";
    let (cue, _) = best_call(&jump, ask);

    // (1♠) X (P) 3♣ (P) 3♠ (P) ? — advancer answers the stopper-ask.
    let after_ask = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
    ];
    // A spade stopper: right-side the notrump game.
    let stopped = "Kx.xxx.xxx.AQJxx";
    let (notrump, _) = best_call(&after_ask, stopped);
    // No stopper anywhere: play the minor game.
    let no_stop = "xx.xxx.xxx.AKJxx";
    let (minor, _) = best_call(&after_ask, no_stop);

    super::set_advance_minor_jump(true); // restore default-on
    super::set_rich_advance_double(true); // restore rich default

    assert_eq!(
        cue,
        call(3, Strain::Spades),
        "game values without a stopper cue the stopper-ask"
    );
    assert_eq!(
        notrump,
        call(3, Strain::Notrump),
        "the advancer right-sides 3NT with a stopper"
    );
    assert_eq!(
        minor,
        call(5, Strain::Clubs),
        "without a stopper the advancer signs off in the minor game"
    );
}

/// The doubler answers the advancer's invitational `2NT` naturally — declines
/// with a minimum, accepts to play with a balanced maximum, or shows a 5-card
/// major game-forcing — instead of the floor passing a game.
#[test]
fn doubler_accepts_or_declines_the_2nt_invite() {
    // (1♠) X (P) 2NT (P) ? — doubler acts over the invitational 2NT; the
    // unbid major is hearts.
    let invite = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    super::set_rich_advance_double(true);
    super::set_advance_2nt_continuation(true);

    // Maximum with a 5-card heart suit: accept by showing it, game-forcing.
    let max_major = "x.AKQxx.Kxx.AQxx"; // 18 HCP, 5♥
    let (accept, _) = best_call(&invite, max_major);
    // Balanced maximum, no 5-card major: 3NT to play.
    let max_flat = "KQx.AJx.Qxx.KQxx"; // 17 HCP, balanced
    let (notrump, _) = best_call(&invite, max_flat);
    // Minimum takeout double: decline the limited invite, pass 2NT.
    let minimum = "KQxx.Qxx.xx.KQxx"; // 12 HCP, minimum
    let (decline, _) = best_call(&invite, minimum);

    // Advancer places game over the doubler's forcing 3♥: raise with support.
    let after_major = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let with_fit = "Axx.Kxx.Qxx.QJxx"; // 12 HCP, 3♥ support, spade stopper
    let (raise, _) = best_call(&after_major, with_fit);

    super::set_advance_2nt_continuation(true); // restore default
    super::set_rich_advance_double(true); // restore rich default

    assert_eq!(
        accept,
        call(3, Strain::Hearts),
        "maximum shows a 5-card major, game-forcing"
    );
    assert_eq!(
        notrump,
        call(3, Strain::Notrump),
        "balanced maximum accepts 3NT to play"
    );
    assert_eq!(decline, Call::Pass, "minimum declines the limited invite");
    assert_eq!(
        raise,
        call(4, Strain::Hearts),
        "advancer raises the doubler's shown major to game"
    );
}

/// The Rubens layer: a 5+ unbid major transfers via the rank below it, and
/// the doubler completes by declaring that major — over every opening where
/// the transfer is a genuine jump-cue (`1♣`/`1♦`/`1♥`).
#[test]
fn rubens_transfer_completes_into_the_major() {
    super::set_rich_advance_double(true);
    super::set_advance_rubens(true);

    // Advancer with 5 spades, 10 HCP: transfer via 3♥ (the rank below spades)
    // over each opening; the doubler completes to spades and declares.
    let advancer = "KQJ42.xx.KJx.xxx"; // 5 spades, 10 HCP
    for open in [Strain::Clubs, Strain::Diamonds, Strain::Hearts] {
        let start = [call(1, open), Call::Double, Call::Pass];
        let (xfer, _) = best_call(&start, advancer);
        assert_eq!(
            xfer,
            call(3, Strain::Hearts),
            "5-spade INV+ transfers via 3♥ over (1{open:?})"
        );
        let after = [
            call(1, open),
            Call::Double,
            Call::Pass,
            call(3, Strain::Hearts),
            Call::Pass,
        ];
        let (complete, floored) = best_call(&after, "AKx.xxx.Axxx.xxx");
        assert_eq!(
            complete,
            call(3, Strain::Spades),
            "doubler completes the transfer into spades over (1{open:?})"
        );
        assert!(
            !floored,
            "the completion must come from the book, not the floor"
        );
    }

    super::set_advance_rubens(false);
    super::set_rich_advance_double(true); // restore default
}

/// Best call with Woolsey forced on (default ranges) and the conflicting
/// overlays reset, independent of any other test on this thread.  Resets the
/// toggle afterward so it cannot leak into a non-Woolsey test.
fn woolsey(auction: &[Call], hand: &str) -> (Call, bool) {
    set_notrump_defense(NotrumpDefense::Natural);
    set_unusual_notrump_defense(None);
    set_woolsey_points(9, 19);
    set_woolsey_double_floor(11);
    set_notrump_defense(NotrumpDefense::Woolsey);
    let result = best_call(auction, hand);
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
    // [1NT, 2♦, P] — a weak advancer bids the 2♥ pass-or-correct.
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
    // [1NT, X, P] — the X is takeout, so a weak no-major advancer relays 2♣
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
    // [1NT, 2♥, P] — a known 5-card heart suit.  With support + game values the
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
    // [1NT, 2♥, X] — a weak hand short in hearts escapes the doubled misfit via
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
    // [1NT, 2♥, P, 2NT, P] — the overcaller answers the minor-ask: 3♦ with
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

#[test]
fn transfer_over_2h_is_plain_cohen() {
    // Over (2♥) Transfer is plain Cohen: a 5-spade GF transfers *through*
    // hearts — 3♦ shows spades, a book node (the diamond Smolen leg only
    // fires over (2♦)).
    let over_2h = [call(2, Strain::Hearts), Call::Double, Call::Pass];
    let (c, floored) = advance(LebensohlStyle::Transfer, &over_2h, "AKQ65.43.K32.J32");
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the transfer must come from the book");
}

/// Best call with Leaping Michaels forced to `on` (and the sohl toggles reset,
/// independent of any other test on this thread)
fn leaping(on: bool, auction: &[Call], hand: &str) -> (Call, bool) {
    set_advance_sohl_style(LebensohlStyle::Off);
    set_leaping_michaels(on);
    best_call(auction, hand)
}

#[test]
fn leaping_michaels_minor_plus_other_major_over_a_major() {
    // Over (2♥): 5-5 clubs+spades, game values → 4♣; 5-5 diamonds+spades → 4♦.
    let over_2h = [call(2, Strain::Hearts)];
    let (c, floored) = leaping(true, &over_2h, "AKQ65.4.32.KQJ76");
    assert_eq!(c, call(4, Strain::Clubs));
    assert!(!floored, "Leaping Michaels must come from the book node");

    let (d, _) = leaping(true, &over_2h, "AKQ65.4.KQJ76.32");
    assert_eq!(d, call(4, Strain::Diamonds));
}

#[test]
fn leaping_michaels_cue_shows_both_majors_over_2d() {
    // Over (2♦): 5-5 in the majors → 4♦ (the cue), both majors.
    let over_2d = [call(2, Strain::Diamonds)];
    let (c, floored) = leaping(true, &over_2d, "AKQ65.KQJ76.4.32");
    assert_eq!(c, call(4, Strain::Diamonds));
    assert!(!floored, "Leaping Michaels must come from the book node");
}

#[test]
fn leaping_michaels_advancer_picks_the_major_game() {
    // (2♥)–4♣–(P): partner shows clubs + spades. With spade support the
    // advancer bids the 4♠ game; with none, the 5♣ minor game (never pass 4♣).
    let auction = [call(2, Strain::Hearts), call(4, Strain::Clubs), Call::Pass];
    let (fit, floored) = leaping(true, &auction, "KQ7.32.J865.A432");
    assert_eq!(fit, call(4, Strain::Spades));
    assert!(!floored, "the advance must come from the book node");

    // A doubleton (7-card fit) still takes the 4♠ game — it scores well and
    // needs only ten tricks.
    let (thin, _) = leaping(true, &auction, "K7.QJ32.8654.A32");
    assert_eq!(thin, call(4, Strain::Spades));

    // A genuine major misfit (≤1) retreats to the 5♣ game, not a passed 4♣.
    let (no_fit, _) = leaping(true, &auction, "2.QJ32.J8654.KQ4");
    assert_eq!(no_fit, call(5, Strain::Clubs));
}

#[test]
fn leaping_michaels_advancer_picks_longer_major_over_2d_cue() {
    // (2♦)–4♦–(P): the cue shows both majors; advancer picks the longer.
    let auction = [
        call(2, Strain::Diamonds),
        call(4, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, floored) = leaping(true, &auction, "AQ32.K8.654.9432");
    assert_eq!(c, call(4, Strain::Spades));
    assert!(!floored, "the advance must come from the book node");
}

#[test]
fn leaping_michaels_2d_4c_pass_or_correct() {
    // (2♦)–4♣–(P): clubs + an unknown major → 4♥ pass-or-correct, then the
    // overcaller with spades corrects to 4♠.
    let advance = [
        call(2, Strain::Diamonds),
        call(4, Strain::Clubs),
        Call::Pass,
    ];
    let (relay, _) = leaping(true, &advance, "K32.A87.9654.J32");
    assert_eq!(relay, call(4, Strain::Hearts));

    let rebid = [
        call(2, Strain::Diamonds),
        call(4, Strain::Clubs),
        Call::Pass,
        call(4, Strain::Hearts),
        Call::Pass,
    ];
    let (correct, _) = leaping(true, &rebid, "AKQ65.4.32.KQJ76");
    assert_eq!(correct, call(4, Strain::Spades));
}

#[test]
fn leaping_michaels_silent_when_disabled() {
    // Turned off: the same club-spade two-suiter never jumps to 4♣ (the
    // escape hatch back to the pre-Leaping-Michaels weak-two defense).
    let over_2h = [call(2, Strain::Hearts)];
    let (c, _) = leaping(false, &over_2h, "AKQ65.4.32.KQJ76");
    assert_ne!(c, call(4, Strain::Clubs));
}

/// Best call with direct-seat DONT forced on, restored after so it never leaks
/// into a sibling test on this thread.
fn direct_dont(auction: &[Call], hand: &str) -> (Call, bool) {
    let prev = super::notrump_defense();
    set_notrump_defense(NotrumpDefense::DirectDont);
    let result = best_call(auction, hand);
    set_notrump_defense(prev);
    result
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
    // [1NT,X,P]: with DONT on the direct-seat X is a one-suiter, so the advancer
    // relays 2♣ (a book node now keyed at the direct seat, not floored)...
    let nt = call(1, Strain::Notrump);
    let p = Call::Pass;
    let prev = super::notrump_defense();
    set_notrump_defense(NotrumpDefense::DirectDont);
    let (relay, floored) = best_call(&[nt, Call::Double, p], "Q32.Q32.Q432.432");
    // ...and the doubler with a long heart suit names it.
    let after_relay = [nt, Call::Double, p, call(2, Strain::Clubs), p];
    let (name, _) = best_call(&after_relay, "432.AKJ87.32.432");
    // And if they redouble the one-suiter X, the advancer still relays 2♣ —
    // never sits in 1NTxx.
    let (escape, esc_floored) = best_call(&[nt, Call::Double, Call::Redouble], "Q32.Q32.Q432.432");
    // And if they double our artificial 2♣ relay, the doubler still names the
    // real suit (2♥ here) rather than sitting in the 2♣x misfit.
    let relay_doubled = [
        nt,
        Call::Double,
        Call::Redouble,
        call(2, Strain::Clubs),
        Call::Double,
    ];
    let (named, nd_floored) = best_call(&relay_doubled, "432.AKJ87.32.432");
    set_notrump_defense(prev);
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

/// Best call with Meckwell forced on, restored after so it never leaks to a
/// sibling test on this thread.
fn meckwell(auction: &[Call], hand: &str) -> (Call, bool) {
    let prev = super::notrump_defense();
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
    let prev = super::notrump_defense();
    set_notrump_defense(NotrumpDefense::Meckwell);

    // [1NT,X,P]: advancer relays 2♣ (pass-or-correct), from the book.
    let (relay, relay_floored) = best_call(&[nt, Call::Double, p], "Q32.Q32.Q432.432");
    // [1NT,X,P,2♣,P]: a diamond one-suiter doubler names 2♦ (real diamonds).
    let (diamonds, _) = best_call(&[nt, Call::Double, p, c2, p], "32.32.AKQ876.432");
    // …a both-majors doubler bids 2♥ (4+ hearts here ⇒ both majors).
    let (majors, majors_floored) = best_call(&[nt, Call::Double, p, c2, p], "AJ32.KQ87.32.32");
    // …a club one-suiter doubler passes (plays 2♣).
    let (clubs, _) = best_call(&[nt, Call::Double, p, c2, p], "32.32.432.AKQ876");
    // [1NT,X,XX]: their redouble — the advancer still relays 2♣, never sits 1NTxx.
    let (escape, esc_floored) = best_call(&[nt, Call::Double, Call::Redouble], "Q32.Q32.Q432.432");
    // [1NT,X,P,2♣,X]: they double our relay — the diamond doubler still names 2♦,
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

#[test]
fn direct_landy_double_shows_both_majors_and_runs_clean() {
    let nt = call(1, Strain::Notrump);
    let p = Call::Pass;
    let x = Call::Double;
    let xx = Call::Redouble;
    let d2 = call(2, Strain::Diamonds);
    let prev = super::direct_landy_double();
    let prev_floor = super::direct_landy_double_floor();
    set_direct_landy_double(Some(false)); // 5-4
    super::set_direct_landy_double_floor(8); // low floor so these 10-14 hands fire the X

    // Both majors 5-4 → X (the both-majors takeout double), from the book.
    let (dbl, floored) = best_call(&[nt], "AJ32.KQ876.32.32");
    // 15+ balanced has no penalty double now → Pass.
    let (pass, _) = best_call(&[nt], "AKQ2.KQ2.KJ2.432");
    // Advancer, equal majors and weak → 2♦ relay ("pick a major").
    let (relay, relay_floored) = best_call(&[nt, x, p], "Q32.Q43.J432.432");
    // They double the artificial relay → doubler still names the longer major
    // (5-4 hearts → 2♥), never sits in the short-diamond 2♦x misfit.
    let (named, named_floored) = best_call(&[nt, x, p, d2, x], "AJ32.KQ876.32.32");
    // They redouble our X.  Clean runout: equal majors / no suit → Pass = ask back
    // (the doubler will name its major), never the phantom 2♦ relay.
    let (ask, ask_floored) = best_call(&[nt, x, xx], "Q32.Q43.J432.432");
    // …and a long-club, short-major advancer escapes to its own 2♣ (to play) —
    // the club rung the two-level 2♣ over the redoubled 1NT gives us.
    let (clubs, _) = best_call(&[nt, x, xx], "32.43.432.AKQ876");
    // After the ask, the doubler names its five-card major.
    let (named_xx, named_xx_floored) = best_call(&[nt, x, xx, p, p], "AJ32.KQ876.32.32");
    // After we name our major (via the undoubled relay) and they double it, SIT —
    // play 2♥x (our 5-4+ fit), never run to 3♦.  `[1NT,X,P,2♦,X,2♥,X,P,P]`.
    let sit_auction = [nt, x, p, d2, x, call(2, Strain::Hearts), x, p, p];
    let (settle, settle_floored) = best_call(&sit_auction, "AJ32.KQ876.32.32");

    set_direct_landy_double(prev);
    super::set_direct_landy_double_floor(prev_floor);
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
    let prev = super::direct_landy_double();
    let prev_pen = super::direct_landy_penalty_pass();
    let prev_floor = super::direct_landy_double_floor();
    set_direct_landy_double(Some(false)); // 5-4
    super::set_direct_landy_double_floor(8); // floor 8 → penalty needs 22-8 = 14+

    // No major fit (2-2) + defensive values: with the knob OFF the advancer is
    // forced to bid (no Pass rule); with it ON it passes to defend 1NTx.
    let defensive = "AQ.KQ.QJ876.K432"; // 14 HCP, 2♠-2♥
    super::set_direct_landy_penalty_pass(false);
    let (forced, _) = best_call(&[nt, x, p], defensive);
    super::set_direct_landy_penalty_pass(true);
    let (penalty, pen_floored) = best_call(&[nt, x, p], defensive);
    // A hand WITH a major fit still bids even with the knob on (not a penalty pass).
    let (with_fit, _) = best_call(&[nt, x, p], "QJ32.K.QJ876.K43"); // 4 spades

    set_direct_landy_double(prev);
    super::set_direct_landy_penalty_pass(prev_pen);
    super::set_direct_landy_double_floor(prev_floor);
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

#[test]
fn doubled_unusual_2nt_runs_never_sits() {
    // Their 1NT, our both-minors 2NT (on by default), their penalty X — the
    // advancer must run to the longer minor, never sit in the doomed 2NT-X.
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Notrump),
        Call::Double,
    ];
    // Clubs longer → 3♣ (a book node, not a floored pass).
    let (c, floored) = best_call(&auction, "432.32.QJ8.T9876");
    assert_eq!(c, call(3, Strain::Clubs));
    assert!(!floored, "the runout must come from the book");
    // Diamonds longer → 3♦.
    let (d, _) = best_call(&auction, "432.32.QJ876.T98");
    assert_eq!(d, call(3, Strain::Diamonds));
}

/// D1b: the 5-box `semi_balanced` union accepts exactly the shapes the
/// legacy `balanced() | described("5422/6322/7222", …)` composite did,
/// exhaustively over the 560-shape length lattice.
#[test]
fn semi_balanced_boxes_match_closure() {
    use super::semi_balanced;
    use crate::bidding::constraint::{Constraint as _, for_each_shape};
    use crate::bidding::context::Context;

    let ctx = Context::new(RelativeVulnerability::NONE, &[]);
    let gate = semi_balanced();
    for_each_shape(|lengths, hand| {
        let mut sorted = lengths;
        sorted.sort_unstable();
        let reference = matches!(
            sorted,
            // balanced …
            [3, 3, 3, 4] | [2, 3, 4, 4] | [2, 3, 3, 5]
            // … or the replaced closure's patterns
            | [2, 2, 4, 5] | [2, 2, 3, 6] | [2, 2, 2, 7]
        );
        assert_eq!(
            gate.eval(hand, &ctx).is_finite(),
            reference,
            "semi_balanced disagrees at {lengths:?}",
        );
    });
}
