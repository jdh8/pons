use super::super::tests::{best_call, call};
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain};

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
        &super::overcall::defense_to_suit(Bid::new(1, Strain::Hearts)),
        &hcp(18..),
    );
    // Legacy gauge: points(17..).
    super::overcall::set_strong_double_hcp(None);
    let legacy = super::overcall::defense_to_suit(Bid::new(1, Strain::Clubs));
    super::overcall::set_strong_double_hcp(Some(18));
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
    let baseline = super::overcall::defense_to_suit(their_opening);
    super::overcall::set_overcall_four_card(false);
    let explicit_off = super::overcall::defense_to_suit(their_opening);
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
    super::overcall::set_overcall_four_card(true);
    let on = super::overcall::defense_to_suit(their_opening).classify(hand, &context);
    super::overcall::set_overcall_four_card(false);
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

    super::overcall::set_two_level_minor_overcall_tight(true);
    let (tight_call, _) = best_call(&over_1s, minimum);
    // A 17-count is not silenced by the knob — it competes (a takeout X first).
    let (strong_call, _) = best_call(&over_1s, "A2.K2.Q432.AKJ87"); // 17 HCP
    super::overcall::set_two_level_minor_overcall_tight(false);
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

    super::overcall::set_strong_double_hcp(None);
    let (legacy_call, _) = best_call(&over_1h, shaped);
    super::overcall::set_strong_double_hcp(Some(18));
    assert_eq!(
        legacy_call,
        Call::Double,
        "the points partition (off arm): 18 points reads as the strong tier"
    );
    set_point_scale(PointScale::PointCount);
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

    super::overcall::set_nt_overcall_no_major(true);
    let (gated_call, _) = best_call(&over_1d, five_heart);
    let (flat_call, _) = best_call(&over_1d, "A32.KQ4.KQ4.J432"); // 15 HCP 4333, no 5M
    super::overcall::set_nt_overcall_no_major(false);
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
