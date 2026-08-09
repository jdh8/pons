use super::*;
use crate::bidding::inference::set_envelope_union_reading;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Strain};

/// 15 HCP, 4333 — spades.hearts.diamonds.clubs
const BALANCED_15: &str = "AKQ2.K53.QJ4.T92";

fn hand(s: &str) -> Hand {
    s.parse().expect("valid test hand")
}

fn empty_context() -> Context<'static> {
    Context::new(RelativeVulnerability::NONE, &[])
}

fn assert_pass(logit: f32) {
    assert!(logit.is_finite() && logit.abs() <= f32::EPSILON);
}

fn assert_reject(logit: f32) {
    assert!(logit.is_infinite() && logit.is_sign_negative());
}

/// [`Constraint::announce`] defaults to [`Constraint::project`], and the
/// combinators forward it — so the two folds are the same fold everywhere
/// until [`announced`] deliberately splits them.  This is what makes the
/// agreement overlay byte-identical to the projection overlay across the
/// whole book, and the pilot's A/B a clean one-site experiment.
#[test]
fn announce_defaults_to_project() {
    let context = empty_context();
    set_envelope_union_reading(false);

    // A leaf, and both combinators over leaves.
    let same = |c: &dyn Constraint| {
        assert_eq!(c.project(&context).hull(), c.announce(&context).hull());
    };
    same(&points(12..));
    same(&(points(12..) & len(Suit::Spades, 5..)));
    same(&(points(20..) | len(Suit::Hearts, 6..)));

    // `announced` splits them: evaluation and projection stay on the
    // judgment (an opaque `pred`, so ⊤), the agreement carries the box.
    let split = announced(pred(|_, _| true), points(11..));
    assert_eq!(
        split.project(&context).hull().strength.points,
        Range::FULL_POINTS,
        "the judgment's projection is what the sampler still sees"
    );
    assert_eq!(
        split.announce(&context).hull().strength.points.min,
        11,
        "the agreement is what the table is told"
    );
    assert_pass(split.eval(hand(BALANCED_15), &context));
    set_envelope_union_reading(true);
}

#[test]
fn project_band_carries_ceilings() {
    let context = empty_context();
    set_envelope_union_reading(false);
    // `points` gauges the shared scalar: both bounds exact.  (`.hull()`
    // collapses the single-box C1 `EnvelopeUnion` to its `Envelope`.)
    assert_eq!(
        points(..12).project_band(&context).hull().strength.points,
        Range::new(0, 11)
    );
    // An HCP ceiling owes the scale its maximum upgrade (point-count
    // default: 2); the floor matches `project`.
    assert_eq!(
        hcp(..6).project_band(&context).hull().strength.points,
        Range::new(0, 7)
    );
    // `project` itself stays floor-only — the alert path is untouched.
    assert_eq!(
        hcp(..6).project(&context).hull().strength.points,
        Range::FULL_POINTS
    );
    // Composition is tight per arm: the 1NT pass gate (`notrump.rs`) — an
    // off-major weak arm unioned with the flat-eight arm — caps points at
    // 10 (the flat-eight arm's 8 HCP + the point-count max upgrade 2) and
    // both majors at five.
    let gate = (hcp(..8) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5))
        | (hcp(8..=8)
            & balanced()
            & len(Suit::Clubs, 3..)
            & len(Suit::Diamonds, 3..)
            & len(Suit::Hearts, 3..)
            & len(Suit::Spades, 3..));
    let band = gate.project_band(&context).hull();
    assert_eq!(band.strength.points, Range::new(0, 10));
    assert_eq!(band.length(Suit::Hearts).max, 5);
    assert_eq!(band.length(Suit::Spades).max, 5);
    // A trivial catch-all claims nothing — the trap-pass safeguard.
    assert_eq!(hcp(0..).project_band(&context).hull(), Envelope::unknown());
    set_envelope_union_reading(true);
}

#[test]
fn test_hcp_and_balanced() {
    let context = empty_context();
    assert_pass(hcp(15..=17).eval(hand(BALANCED_15), &context));
    assert_reject(hcp(16..).eval(hand(BALANCED_15), &context));
    assert_pass(balanced().eval(hand(BALANCED_15), &context));
    assert_reject(balanced().eval(hand("AKQJ2.K543.QJ4.2"), &context));
}

#[test]
fn strength_dial_role_detection() {
    let one_club = Call::Bid(Bid::new(1, Strain::Clubs));
    let one_spade = Call::Bid(Bid::new(1, Strain::Spades));
    let role =
        |auction: &[Call]| dial_shift(2, &Context::new(RelativeVulnerability::NONE, auction));

    assert_eq!(role(&[]), DialShift::Add(2)); // opener
    assert_eq!(
        role(&[one_club, one_spade, Call::Pass, Call::Pass, Call::Double,]),
        DialShift::Add(2)
    ); // overcaller
    assert_eq!(role(&[one_club, Call::Pass]), DialShift::Subtract(2)); // responder
    assert_eq!(
        role(&[one_club, one_spade, Call::Pass]),
        DialShift::Subtract(2)
    ); // advancer
    assert_eq!(role(&[one_club, Call::Pass, Call::Pass]), DialShift::Add(2)); // balancer
}

#[test]
fn strength_dial_zero_is_identical_in_both_roles() {
    let one_club = Call::Bid(Bid::new(1, Strain::Clubs));
    let responder_auction = [one_club, Call::Pass];
    let opener = empty_context();
    let responder = Context::new(RelativeVulnerability::NONE, &responder_auction);
    let test_hand = hand(BALANCED_15);

    let baseline_hcp = hcp(15..=17);
    let baseline_points = points(15..=17);
    let baseline_support = support_points(Suit::Spades, 15..=17);
    set_strength_dial(0);
    let zero_hcp = hcp(15..=17);
    let zero_points = points(15..=17);
    let zero_support = support_points(Suit::Spades, 15..=17);

    for context in [&opener, &responder] {
        assert_eq!(
            baseline_hcp.eval(test_hand, context),
            zero_hcp.eval(test_hand, context)
        );
        assert_eq!(
            baseline_points.eval(test_hand, context),
            zero_points.eval(test_hand, context)
        );
        assert_eq!(
            baseline_support.eval(test_hand, context),
            zero_support.eval(test_hand, context)
        );
    }
}

#[test]
fn strength_dial_two_moves_points_antisymmetrically() {
    let test_hand = hand("KQ765.A8765.32.2"); // 11 points
    let one_club = Call::Bid(Bid::new(1, Strain::Clubs));
    let responder_auction = [one_club, Call::Pass];
    let opener = empty_context();
    let responder = Context::new(RelativeVulnerability::NONE, &responder_auction);

    let gate = points(13..);

    // The dial is read when the gate is *evaluated*, not when it is built, so
    // it stays armed across both classifications.  An 11-count opens as though
    // it were 13 and responds as though it were 9.
    set_strength_dial(2);
    let opener_verdict = gate.eval(test_hand, &opener);
    let responder_verdict = gate.eval(test_hand, &responder);
    set_strength_dial(0);

    assert_pass(opener_verdict);
    assert_reject(responder_verdict);

    // And with the dial back at rest the same gate rejects in both roles —
    // proof the constraint carries no baked-in dial of its own.
    assert_reject(gate.eval(test_hand, &opener));
    assert_reject(gate.eval(test_hand, &responder));
}

/// A stance pins the dial it was built under, so the deviation panel's deviant
/// seat keeps its calibration after the harness resets the thread
/// (`examples/common/mod.rs`'s `deviant_floor` arms, builds, and resets).
#[test]
fn strength_dial_survives_on_a_pinned_stance() {
    use crate::bidding::System;
    use crate::bidding::american::american_book;

    let hand = hand("KQ765.A8765.32.2"); // 11 points

    set_strength_dial(2);
    let deviant = american_book(&crate::bidding::agreements::Agreements::current()).against();
    set_strength_dial(0);
    let plain = american_book(&crate::bidding::agreements::Agreements::current()).against();

    // An 11-count opens 1♥ only on the dialled stance; the plain one passes.
    let opened = |stance: &crate::bidding::book::Stance| {
        stance
            .classify(hand, RelativeVulnerability::NONE, &[])
            .map(|logits| {
                (&logits.0)
                    .into_iter()
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("comparable logits"))
                    .map(|(call, _)| call)
                    .expect("a decision")
            })
    };
    assert_ne!(
        opened(&deviant),
        opened(&plain),
        "the dial the deviant stance pinned must outlive the thread's reset"
    );
}

#[test]
fn test_wasted() {
    let working = ["", "2", "32", "A2", "K2", "KT", "AKQ", "QJ2", "J32"];
    let wasted_holdings = [
        "A", "K", "Q", "J", "Q2", "J2", "AK", "AQ", "AJ", "KQ", "KJ", "QJ",
    ];

    for text in working {
        let holding: Holding = text.parse().expect(text);
        assert!(!wasted(holding), "{text} should not be wasted");
    }
    for text in wasted_holdings {
        let holding: Holding = text.parse().expect(text);
        assert!(wasted(holding), "{text} should be wasted");
    }
}

#[test]
fn test_upgrade() {
    // Balanced hands never upgrade, clean doubleton or not.
    assert_eq!(upgrade(hand(BALANCED_15)), 0);
    assert_eq!(upgrade(hand("AQJ32.K53.QJ4.92")), 0);

    // Unbalanced, clean singleton: +1.
    assert_eq!(upgrade(hand("KQ765.A876.532.2")), 1);

    // Two-suiter with 10+ cards in the two longest suits: +2.
    assert_eq!(upgrade(hand("KQ765.A8765.32.2")), 2);
    assert_eq!(upgrade(hand("KQ8765.A876.32.2")), 2);

    // Ax and Kx are working short holdings, not wasted.
    assert_eq!(upgrade(hand("KQ765.87654.A2.2")), 2);

    // Wasted short honors cost one point each; the upgrade floors at zero.
    assert_eq!(upgrade(hand("KQ765.A876.532.K")), 0); // +1 base, −1 stiff K
    assert_eq!(upgrade(hand("KQ765.A8765.Q2.2")), 1); // +2 base, −1 Qx
    assert_eq!(upgrade(hand("KQ765.87654.AK.2")), 1); // +2 base, −1 AK tight
}

/// `points(12..)` *is* the Rule of 20 — the identity the dedicated
/// `rule_of_20()` constraint was deleted in favour of
#[test]
fn points_twelve_is_the_rule_of_20() {
    // The N+8 = Rule-of-20 identity is now the opt-out, not the default:
    // pin the rule-of-N+8 scale these example hands' points assume.
    set_point_scale(PointScale::RuleOfNFloored);
    let context = empty_context();
    let opens = |text: &str| points(12..).eval(hand(text), &context);
    // Raw HCP + the two longest suits, the classic Rule-of-20 kernel.
    let rule_of_20 = |text: &str| {
        let hand = hand(text);
        raw_hcp(hand) + longest_two_suits(hand) >= 20
    };

    // 11 HCP, 5-4: 11 + 9 = 20.  The wasted J9 that voids the *legacy*
    // upgrade is irrelevant to the rule-of-N+8 scale, which is pure shape.
    // 11 HCP, 6-6: 11 + 12 = 23.  10 HCP, 6-4: 10 + 10 = 20.  A 7-count 7-6
    // clears too (7 + 13) — the opening rule's own HCP floor, not the point
    // count, is what keeps such freaks out.
    for text in [
        "AK986.J9.QJT6.64",
        ".KQ7542.A.Q96542",
        "KJ9876.5.KQJ4.32",
        "A765432.K76543..",
    ] {
        assert!(rule_of_20(text));
        assert_pass(opens(text));
    }
    // 11 HCP, 5-3-3-2: 11 + 8 = 19 — a pass under both readings.
    assert!(!rule_of_20("KQ876.K32.Q32.J2"));
    assert_reject(opens("KQ876.K32.Q32.J2"));

    // The identity holds wherever the two longest suits reach 8 cards, i.e.
    // every shape but flat 4-3-3-3, where the floored scale reads raw HCP:
    // a flat 12-count is `points(12..)` but not Rule-of-20 (12 + 7 = 19).
    assert!(!rule_of_20("KJ32.K32.K32.Q32"));
    assert_pass(opens("KJ32.K32.K32.Q32"));
    // A flat 11-count is short of both.
    assert!(!rule_of_20("KQ32.K32.Q32.J32"));
    assert_reject(opens("KQ32.K32.Q32.J32"));
    set_point_scale(PointScale::PointCount);
}

#[test]
fn test_unbid_support() {
    // RHO opened 1♥; the unbid suits are ♣ ♦ ♠.
    let auction = [Call::Bid(Bid::new(1, Strain::Hearts))];
    let context = Context::new(RelativeVulnerability::NONE, &auction);

    // 4-1-4-4 short in their suit: 3+ in every unbid suit → passes both gates.
    let shapely = hand("AQ82.5.KJ64.Q975");
    assert_pass(unbid_support(0).eval(shapely, &context));
    assert_pass(unbid_support(1).eval(shapely, &context));

    // 5-3-3-2 with the doubleton in an unbid suit (♠): exactly one unbid suit
    // short → lenient admits, strict rejects.
    let semi = hand("Q2.A54.K54.KJ876");
    assert_reject(unbid_support(0).eval(semi, &context));
    assert_pass(unbid_support(1).eval(semi, &context));

    // 2-3-2-6 one-suiter (6 clubs), short in two unbid suits (♦ ♠): both gates
    // reject — this hand belongs in the 17+ any-shape double tier.
    let one_suiter = hand("K2.A54.Q2.KJ8763");
    assert_reject(unbid_support(0).eval(one_suiter, &context));
    assert_reject(unbid_support(1).eval(one_suiter, &context));
}

#[test]
fn test_points_and_fifths() {
    let context = empty_context();

    // This test exercises the shipped raw-HCP+upgrade (PointCount) default
    // scale; the rule-of-N+8 arms live in `test_point_scale`, and the
    // fit-known candidate rides on `support_points` (see `test_support_points`).

    // 9 HCP, clean 5-5: 9 + upgrade 2 = 11 points (the clean two-suiter
    // agrees with rule-of-N+8's 9 + 10 − 8 here).
    let two_suiter = hand("KQ765.A8765.32.2");
    assert_pass(points(11..=11).eval(two_suiter, &context));
    assert_reject(points(..=10).eval(two_suiter, &context));

    // The floor blocks the flat 4-3-3-3 downgrade: raw HCP, not HCP − 1.
    assert_pass(points(15..=15).eval(hand(BALANCED_15), &context));

    // BALANCED_15 is 15 HCP but only 14.6 Fifths: its queens and jacks
    // are worth less toward 3NT.  The banded value averages Fifths with
    // the honor companion (≈14.55 BUM-RAP, 14.8 HCP — same verdict either
    // way), so it still drops out of a 15-17 notrump but stays inside a
    // 12-14 one.  Fifths is default-off now (raw HCP beat it in the A6
    // audit), so this test enables the gauge it exercises.
    set_fuzzy_fifths(true);
    assert_reject(fifths(15.0..18.0).eval(hand(BALANCED_15), &context));
    assert_pass(fifths(12.0..15.0).eval(hand(BALANCED_15), &context));
    set_fuzzy_fifths(false); // restore the shipped default

    // CCCC of this 4333 is 14.90 (oracle-verified in contract-bridge).
    assert_pass(cccc_at_least(14.9).eval(hand("AQ32.K53.QJ4.A92"), &context));
    assert_reject(cccc_at_least(15.0).eval(hand("AQ32.K53.QJ4.A92"), &context));
    assert_pass(cccc(14.0..15.0).eval(hand("AQ32.K53.QJ4.A92"), &context));
    assert_reject(cccc(..14.9).eval(hand("AQ32.K53.QJ4.A92"), &context));

    // Honor location: same 6 HCP, but KQJ concentrated in the 6-card suit
    // versus banished to short suits.  CCCC pays for the concentration;
    // NLTC discounts honors that don't guard length (the doubleton KQ's
    // queen saves no loser — only 3+ card suits check the queen slot).
    let concentrated = hand("KQJ862.943.75.82");
    let scattered = hand("986432.94.KQ.J82");
    assert!(eval::cccc(concentrated) > eval::cccc(scattered));
    assert!(eval::NLTC.eval(concentrated) < eval::NLTC.eval(scattered));
    // NLTC of the concentrated hand: ♠1.5 + ♥3 + ♦2.5 + ♣2.5 = 9.5 losers.
    assert_pass(nltc(..=9.5).eval(concentrated, &context));
    assert_reject(nltc(..9.5).eval(concentrated, &context));
    assert_pass(nltc(9.0..=10.0).eval(concentrated, &context));
}

#[test]
fn test_support_points() {
    let context = empty_context();

    // 9 HCP, clean 5-5-2-1.  The candidate scale counts hcp_plus (useful
    // shortness: +1 doubleton, +2 singleton) plus the long-suit term:
    // 9 + 1 + 2 + 1 = 13, above the legacy raw-HCP-plus-upgrade of 11.
    let two_suiter = hand("KQ765.A8765.32.2");

    // Off (the A/B baseline arm): byte-identical to the global `point_count`
    // that `points` gauges — a gate swapped `points`→`support_points` doesn't
    // move.  (Rule of N+8 and the legacy upgrade agree on this clean 5-5.)
    set_support_points(false);
    assert_eq!(support_point_count(two_suiter), point_count(two_suiter));
    assert_eq!(support_point_count(two_suiter), 11);
    assert_pass(support_points(Suit::Spades, 11..=11).eval(two_suiter, &context));

    // On (the shipped default): the hotter hcp_plus scale, strictly above
    // legacy for a shaped hand (the singleton and doubleton now add).
    set_support_points(true);
    assert_eq!(support_point_count(two_suiter), 13);
    assert!(support_point_count(two_suiter) > point_count(two_suiter));
    assert_pass(support_points(Suit::Spades, 13..=13).eval(two_suiter, &context));
    assert_reject(support_points(Suit::Spades, ..=12).eval(two_suiter, &context));

    // Flat hands carry no useful shortness, so the support scale sticks to
    // raw HCP — and the floored rule-of-N+8 default agrees on a 4-3-3-3.
    let flat = hand("AQ32.K53.QJ4.A92"); // 16 HCP, 4-3-3-3
    assert_eq!(support_point_count(flat), 16);
    assert_eq!(point_count(flat), 16);
    // Left on — the shipped default — for the rest of the suite.
}

#[test]
fn test_suit_support_points() {
    let context = empty_context();
    // 9 HCP, clean 5-5-2-1: suit-blind SP = 13 (9 + 1 + 2 + 1 double-fit).
    let two_suiter = hand("KQ765.A8765.32.2");

    // The ≥3-trump identity: a 3+ card holding earns no shortness value on
    // either scale, so SP_in = SP exactly — every fit-known gate (all
    // conjoin 3+ support) reads the familiar suit-blind count.
    assert_eq!(support_point_count_in(two_suiter, Suit::Spades), 13);
    assert_eq!(support_point_count_in(two_suiter, Suit::Hearts), 13);
    // Short-trump corners, where the scales diverge: the suit-blind scale
    // counted ruffing shortness *in trump* (doubleton +1, singleton +2);
    // suit-indexed, trumps are trumps, not ruffs.
    assert_eq!(support_point_count_in(two_suiter, Suit::Diamonds), 12); // = 13 − 1
    assert_eq!(support_point_count_in(two_suiter, Suit::Clubs), 11); // = 13 − 2

    // Flat 16-count: no shortness anywhere, so both scales are raw HCP.
    let flat = hand("AQ32.K53.QJ4.A92");
    assert_eq!(support_point_count_in(flat, Suit::Spades), 16);
    assert_eq!(
        support_point_count_in(flat, Suit::Spades),
        support_point_count(flat)
    );

    // The gate tests SP_in for its authored trump against the band.
    assert_pass(support_points(Suit::Spades, 13..=13).eval(two_suiter, &context));
    assert_pass(support_points(Suit::Spades, 13..).eval(two_suiter, &context));
    assert_reject(support_points(Suit::Spades, ..=12).eval(two_suiter, &context));
    assert_reject(support_points(Suit::Spades, 14..).eval(two_suiter, &context));
    // The corner bites through the DSL: a stiff-club "trump" reads 11.
    assert_reject(support_points(Suit::Clubs, 13..).eval(two_suiter, &context));
}

#[test]
fn test_fifths_companion() {
    let context = empty_context();
    // Quack-heavy 18-count: 18.2 Fifths, 18 HCP, 16.5 BUM-RAP.  The
    // Fifths/HCP average (18.1) tops a 15-17 notrump, but the lighter
    // Fifths/BUM-RAP average (17.35) keeps it inside — the two gauges
    // straddle the band edge.
    let quacky = hand("AQ4.QJT.QJT.KQJT");

    // The companion only matters inside the Fifths gauge, which is
    // default-off now (raw HCP beat it in the A6 audit) — enable it here.
    set_fuzzy_fifths(true);
    set_fifths_companion(FifthsCompanion::Hcp);
    assert_reject(fifths(15.0..18.0).eval(quacky, &context));

    set_fifths_companion(FifthsCompanion::Bumrap);
    assert_pass(fifths(15.0..18.0).eval(quacky, &context));
    set_fuzzy_fifths(false); // restore the shipped default
}

#[test]
fn test_fuzzy_strength_toggle() {
    let context = empty_context();
    let two_suiter = hand("KQ765.A8765.32.2");

    // These toggles swing `points` between raw HCP and the legacy
    // raw-HCP-plus-upgrade scale (both historical arms now).
    set_point_scale(PointScale::Hcp);
    set_fuzzy_fifths(false);
    // Raw HCP: 9 points, and fifths degrades to raw HCP too.
    assert_pass(points(9..=9).eval(two_suiter, &context));
    assert_pass(fifths(15.0..18.0).eval(hand(BALANCED_15), &context));
    assert_reject(fifths(15.5..18.0).eval(hand(BALANCED_15), &context));

    // The legacy upgrade arm agrees with rule-of-N+8 on this clean 5-5.
    set_point_scale(PointScale::PointCount);
    assert_pass(points(11..=11).eval(two_suiter, &context));

    // Restore the shipped default for the rest of the suite.
    set_point_scale(PointScale::PointCount);
}

#[test]
fn test_point_scale() {
    let context = empty_context();
    let two_suiter = hand("KQ765.A8765.32.2"); // 9 HCP, 5-5-2-1
    let flat = hand("AQ32.K53.QJ4.A92"); // 16 HCP, 4-3-3-3

    // Rule of N+8: raw HCP + two longest suit lengths − 8, so a
    // `points(12..)` gate is exactly the Rule of 20.
    set_point_scale(PointScale::RuleOfN);
    // Clean 5-5 agrees with the legacy upgrade: 9 + 10 − 8 = 9 + 2.
    assert_eq!(point_count(two_suiter), 11);
    assert_pass(points(11..=11).eval(two_suiter, &context));
    // Flat 4-3-3-3 reads one under its HCP: 16 + 7 − 8.
    assert_eq!(point_count(flat), 15);
    assert_reject(points(16..).eval(flat, &context));
    // A wasted stiff K voids the legacy upgrade but its shape still
    // counts here: 12 + 9 − 8 = 13 vs legacy 12.
    let wasted = hand("KQ765.A876.532.K");
    assert_eq!(point_count(wasted), 13);
    assert_eq!(point_count(wasted), raw_hcp(wasted) + 1);

    // Blocking the downgrade: flat 4-3-3-3 reads its raw HCP, every
    // other shape agrees with plain rule-of-N+8.
    set_point_scale(PointScale::RuleOfNFloored);
    assert_eq!(point_count(flat), 16);
    assert_eq!(point_count(two_suiter), 11);

    set_point_scale(PointScale::Hcp);
    assert_eq!(point_count(two_suiter), 9);

    // The deposed legacy scale stays reachable as the opt-out.
    set_point_scale(PointScale::PointCount);
    assert_eq!(point_count(two_suiter), 11);

    // Restore the shipped default for the rest of the suite.
    set_point_scale(PointScale::PointCount);
}

#[test]
fn test_combinators() {
    let context = empty_context();
    let strong_notrump = hcp(15..=17) & balanced();
    assert_pass(strong_notrump.eval(hand(BALANCED_15), &context));

    let either = hcp(16..) | len(Suit::Spades, 4..);
    assert_pass(either.eval(hand(BALANCED_15), &context));

    let neither = hcp(16..) | len(Suit::Spades, 5..);
    assert_reject(neither.eval(hand(BALANCED_15), &context));

    assert_reject((!balanced()).eval(hand(BALANCED_15), &context));
    assert_pass((!hcp(16..)).eval(hand(BALANCED_15), &context));
}

#[test]
fn test_support_and_stoppers() {
    // Partner overcalled 1♥ over their 1♦ opening.
    let auction = [
        Call::Bid(Bid::new(1, Strain::Diamonds)),
        Call::Bid(Bid::new(1, Strain::Hearts)),
        Call::Pass,
    ];
    let context = Context::new(RelativeVulnerability::NONE, &auction);

    assert_pass(support(3..).eval(hand(BALANCED_15), &context));
    assert_reject(support(4..).eval(hand(BALANCED_15), &context));

    // QJ4 of diamonds stops their suit; T92 of clubs would not, but
    // clubs is not their suit.
    assert_pass(stopper_in_their_suits().eval(hand(BALANCED_15), &context));
    assert_reject(stopper_in_their_suits().eval(hand("AKQ2.K53.T92.QJ4"), &context));
}

#[test]
fn test_partner_shown_len_and_points() {
    // Partner opened 1♦ (3+ diamonds, 10+ HCP — `points(12..)` is the Rule
    // of 20), RHO passed; we act.
    let auction = [Call::Bid(Bid::new(1, Strain::Diamonds)), Call::Pass];
    let context = Context::new(RelativeVulnerability::NONE, &auction);

    assert_pass(partner_shown_len(Suit::Diamonds, 3..).eval(hand(BALANCED_15), &context));
    assert_reject(partner_shown_len(Suit::Diamonds, 4..).eval(hand(BALANCED_15), &context));
    assert_pass(partner_shown_points(10..).eval(hand(BALANCED_15), &context));
    assert_reject(partner_shown_points(11..).eval(hand(BALANCED_15), &context));

    // Nothing shown in an unbid suit: the minimum is zero.
    assert_reject(partner_shown_len(Suit::Spades, 1..).eval(hand(BALANCED_15), &context));
}

#[test]
fn test_support_without_partner_suit() {
    let context = empty_context();
    assert_reject(support(0..).eval(hand(BALANCED_15), &context));
}

#[test]
fn test_top_honors_and_stopper_in() {
    let context = empty_context();
    // AKQ2 of spades has all three top honors; T92 of clubs has none.
    assert_pass(top_honors(Suit::Spades, 3..).eval(hand(BALANCED_15), &context));
    assert_pass(top_honors(Suit::Hearts, 1..=1).eval(hand(BALANCED_15), &context));
    assert_reject(top_honors(Suit::Clubs, 1..).eval(hand(BALANCED_15), &context));

    // K53 of hearts stops the suit; T92 of clubs does not.
    assert_pass(stopper_in(Suit::Hearts).eval(hand(BALANCED_15), &context));
    assert_reject(stopper_in(Suit::Clubs).eval(hand(BALANCED_15), &context));
}

#[test]
fn test_partner_suit_and_min_level() {
    // Partner overcalled 1♥ over their 1♦ opening.
    let auction = [
        Call::Bid(Bid::new(1, Strain::Diamonds)),
        Call::Bid(Bid::new(1, Strain::Hearts)),
        Call::Pass,
    ];
    let context = Context::new(RelativeVulnerability::NONE, &auction);

    assert_pass(partner_suit_is(Suit::Hearts).eval(hand(BALANCED_15), &context));
    assert_reject(partner_suit_is(Suit::Spades).eval(hand(BALANCED_15), &context));

    assert_pass(min_level_is(1, Strain::Spades).eval(hand(BALANCED_15), &context));
    assert_pass(min_level_is(2, Strain::Diamonds).eval(hand(BALANCED_15), &context));
    assert_reject(min_level_is(2, Strain::Spades).eval(hand(BALANCED_15), &context));
}

#[test]
fn test_vulnerability_and_seats() {
    let auction = [Call::Pass];
    let context = Context::new(RelativeVulnerability::WE, &auction);

    assert_pass(vulnerable().eval(hand(BALANCED_15), &context));
    assert_reject(they_vulnerable().eval(hand(BALANCED_15), &context));
    assert_pass(nth_seat(2).eval(hand(BALANCED_15), &context));
    assert_reject(nth_seat(1).eval(hand(BALANCED_15), &context));
}

/// Render a constraint to its prose, the inverse of evaluation.
fn prose(constraint: &impl Constraint) -> String {
    constraint.describe().to_string()
}

#[test]
fn test_describe_ranges() {
    // Closed, open-ended, capped, and exact integer bands.
    assert_eq!(prose(&hcp(15..=17)), "15–17 HCP");
    assert_eq!(prose(&hcp(16..)), "16+ HCP");
    assert_eq!(prose(&hcp(..11)), "≤10 HCP"); // half-open → inclusive
    assert_eq!(prose(&points(12..=21)), "12–21 points");
    assert_eq!(prose(&len(Suit::Spades, 5..)), "5+ ♠");
    assert_eq!(prose(&len(Suit::Hearts, 6..=6)), "exactly 6 ♥");
    assert_eq!(prose(&support(3..)), "3+ card support for partner");
    assert_eq!(
        prose(&top_honors(Suit::Spades, 2..)),
        "2+ of the top honors in ♠"
    );
    assert_eq!(
        prose(&partner_shown_len(Suit::Diamonds, 3..)),
        "3+ ♦ shown by partner",
    );
    assert_eq!(
        prose(&partner_shown_points(12..)),
        "12+ points shown by partner"
    );
    // Fifths print as a literal float band, never nudged to "≤17.999".
    assert_eq!(prose(&fifths(15.0..18.0)), "15.0–18.0 fifths");
    assert_eq!(prose(&fifths(20.0..22.0)), "20.0–22.0 fifths");
}

#[test]
fn test_describe_atoms() {
    assert_eq!(prose(&balanced()), "balanced");
    assert_eq!(prose(&cccc_at_least(14.9)), "14.9+ CCCC");
    assert_eq!(prose(&cccc(9.0..13.0)), "9.0–13.0 CCCC");
    assert_eq!(prose(&nltc(..=8.5)), "≤8.5 NLTC");
    assert_eq!(prose(&stopper_in(Suit::Hearts)), "stopper in ♥");
    assert_eq!(prose(&stopper_in_their_suits()), "stopper in their suit(s)");
    assert_eq!(
        prose(&short_in_their_suits()),
        "at most three cards in each of their suits",
    );
    assert_eq!(prose(&they_bid(Strain::Spades)), "opponents bid ♠");
    assert_eq!(prose(&they_bid(Strain::Notrump)), "opponents bid NT");
    assert_eq!(
        prose(&partner_suit_is(Suit::Hearts)),
        "partner's last suit is ♥"
    );
    assert_eq!(
        prose(&min_level_is(2, Strain::Diamonds)),
        "2♦ is the cheapest bid"
    );
    assert_eq!(prose(&passed_hand()), "a passed hand");
    assert_eq!(
        prose(&undisturbed()),
        "the opponents have passed throughout"
    );
    assert_eq!(prose(&vulnerable()), "vulnerable");
    assert_eq!(prose(&they_vulnerable()), "opponents vulnerable");
    assert_eq!(prose(&nth_seat(3)), "opening in seat 3");
}

#[test]
fn test_describe_composition() {
    // `&` flattens into one comma list with a trailing "and".
    assert_eq!(
        prose(&(points(12..=21) & len(Suit::Spades, 5..))),
        "12–21 points, and 5+ ♠",
    );
    assert_eq!(
        prose(&(points(12..=21) & len(Suit::Spades, 5..) & balanced())),
        "12–21 points, 5+ ♠, and balanced",
    );
    // `|` flattens with a trailing "or"; `!` wraps in "not (…)".
    assert_eq!(
        prose(&(len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..))),
        "5+ ♣, or 5+ ♦",
    );
    assert_eq!(prose(&!hcp(16..)), "not (16+ HCP)");
    // Double negation cancels.
    assert_eq!(prose(&!!balanced()), "balanced");
    // A nested group is parenthesized so a mixed tree stays unambiguous.
    assert_eq!(
        prose(&(points(9..=11) & len(Suit::Spades, 5..) & (nth_seat(3) | nth_seat(4)))),
        "9–11 points, 5+ ♠, and (opening in seat 3, or opening in seat 4)",
    );
}

#[test]
fn test_describe_opaque_and_labeled() {
    // A bare predicate carries no recoverable meaning.
    assert_eq!(pred(|_, _| true).describe(), Description::Opaque);
    assert_eq!(prose(&pred(|_, _| true)), "(opaque condition)");
    // Opacity surfaces as one element, not a whole-conjunction collapse.
    assert_eq!(
        prose(&(hcp(15..) & pred(|_, _| true))),
        "15+ HCP, and (opaque condition)",
    );
    // The labeled escape hatch describes to its label and still evaluates.
    let prefers_diamonds = described("prefers diamonds", |hand: Hand, _: &Context<'_>| {
        hand[Suit::Diamonds].len() >= hand[Suit::Clubs].len()
    });
    assert_eq!(prose(&prefers_diamonds), "prefers diamonds");
    assert_pass(prefers_diamonds.eval(hand(BALANCED_15), &empty_context()));
}

/// D1: complements read as unions of halves knob-on, hull to the legacy
/// single-envelope readings knob-off.
#[test]
fn complement_union_boxes() {
    let ctx = empty_context();
    set_envelope_union_reading(false);

    // Knob-off: a two-sided band's complement hulls to ⊤ (the legacy
    // reading), and De Morgan stays ⊤.
    let band = (!hcp(15..=17)).project(&ctx);
    assert_eq!(band.hull().strength.hcp, Range::FULL_POINTS);
    let demorgan = (!(len(Suit::Spades, 4..) & hcp(13..))).project(&ctx);
    assert_eq!(demorgan.hull(), Envelope::unknown());

    set_envelope_union_reading(true);

    // Two-sided band → two outer halves on the raw-HCP gauge.
    let band = (!hcp(15..=17)).project(&ctx);
    let halves: Vec<Range> = band.boxes().iter().map(|b| b.strength.hcp).collect();
    assert_eq!(halves, [Range::new(0, 14), Range::new(18, 37)]);

    // De Morgan on `&`: `!(4+ ♠ & 13+ HCP)` = `≤3 ♠ | ≤12 HCP`.
    let demorgan = (!(len(Suit::Spades, 4..) & hcp(13..))).project(&ctx);
    let boxes = demorgan.boxes();
    assert_eq!(boxes.len(), 2);
    assert_eq!(boxes[0].length(Suit::Spades), Range::new(0, 3));
    assert_eq!(boxes[1].strength.hcp, Range::new(0, 12));

    // De Morgan on `|`: `!(5+ ♠ | 5+ ♥)` = both majors ≤4, one box.
    let neither = (!(len(Suit::Spades, 5..) | len(Suit::Hearts, 5..))).project(&ctx);
    let hull = neither.hull();
    assert_eq!(hull.length(Suit::Spades), Range::new(0, 4));
    assert_eq!(hull.length(Suit::Hearts), Range::new(0, 4));

    // Double negation reads the inner band again.
    let double = (!!hcp(15..=17)).project(&ctx);
    assert_eq!(double.hull().strength.hcp, Range::new(15, 17));

    set_envelope_union_reading(true);
}

/// D1b: `balanced` projects the exact 5-box union knob-on and hulls back
/// to the historical readings knob-off (⊤ forward, 2..=5 band).
#[test]
fn balanced_projection_boxes() {
    let ctx = empty_context();
    set_envelope_union_reading(false);
    assert_eq!(balanced().project(&ctx).hull(), Envelope::unknown());
    assert_eq!(
        balanced().project_band(&ctx).hull().lengths,
        [Range::new(2, 5); 4],
    );

    set_envelope_union_reading(true);
    let union = balanced().project(&ctx);
    assert_eq!(union.boxes().len(), 5);
    // Exhaustive over the length lattice: the union admits exactly the
    // balanced shapes.
    for_each_shape(|lengths, hand| {
        assert_eq!(
            union.boxes().iter().any(|envelope| envelope.admits(hand)),
            is_balanced(hand),
            "balanced boxes disagree at {lengths:?}",
        );
    });
    set_envelope_union_reading(true);
}

/// G: the comparative staircases evaluate exactly as the `described`
/// closures they replace, over the whole length lattice.
#[test]
fn comparative_staircases_match_closures() {
    let ctx = empty_context();
    let longer = longer_suit(Suit::Spades, Suit::Hearts);
    let at_least = at_least_as_long(Suit::Hearts, Suit::Spades);
    let equal = equal_length("equal majors", Suit::Hearts, Suit::Spades);
    assert_eq!(prose(&longer), "♠ longer than ♥");
    assert_eq!(prose(&at_least), "♥ at least as long as ♠");
    for_each_shape(|lengths, hand| {
        let [_, _, hearts, spades] = lengths;
        assert_eq!(
            longer.eval(hand, &ctx).is_finite(),
            spades > hearts,
            "longer_suit at {lengths:?}",
        );
        assert_eq!(
            at_least.eval(hand, &ctx).is_finite(),
            hearts >= spades,
            "at_least_as_long at {lengths:?}",
        );
        assert_eq!(
            equal.eval(hand, &ctx).is_finite(),
            hearts == spades,
            "equal_length at {lengths:?}",
        );
    });
}

/// G: `!balanced` reads the exact 20-box unbalanced union knob-on and
/// stays ⊤ knob-off.
#[test]
fn unbalanced_complement_boxes() {
    let ctx = empty_context();
    set_envelope_union_reading(false);
    assert_eq!((!balanced()).project(&ctx).hull(), Envelope::unknown());

    set_envelope_union_reading(true);
    let union = (!balanced()).project(&ctx);
    assert_eq!(union.boxes().len(), 20);
    for_each_shape(|lengths, hand| {
        assert_eq!(
            union.boxes().iter().any(|envelope| envelope.admits(hand)),
            !is_balanced(hand),
            "unbalanced boxes disagree at {lengths:?}",
        );
    });
    set_envelope_union_reading(true);
}

/// G: `top_honors` floors its suit length and raw HCP knob-on; a
/// `points | hcp` disjunction keeps its implied HCP floor through the
/// containment dedup (the strong-2♣ swallow).
#[test]
fn honor_and_points_gauge_boxes() {
    let ctx = empty_context();
    set_envelope_union_reading(false);
    assert_eq!(
        top_honors(Suit::Spades, 2..).project(&ctx).hull(),
        Envelope::unknown(),
    );

    set_envelope_union_reading(true);
    let honors = top_honors(Suit::Spades, 2..).project(&ctx);
    let envelope = honors.boxes()[0];
    assert_eq!(envelope.length(Suit::Spades), Range::new(2, 13));
    assert_eq!(envelope.strength.hcp, Range::new(5, 37));
    // The same cheapest-honors floor, on the suit's own axis.
    assert_eq!(
        envelope.strength.suit_hcp[Suit::Spades as usize],
        Range::new(5, 10),
    );

    // The points box carries its implied HCP floor (down by the scale's
    // maximum upgrade — 5 on the default rule-of-N+8-floored scale), so
    // after tidy (correctly) swallows the tighter `hcp` arm into the
    // wider `points` arm, the HCP knowledge survives.
    let strong_two = (points(22..) | hcp(22..)).project_band(&ctx);
    assert!(
        strong_two
            .boxes()
            .iter()
            .any(|b| b.strength.hcp != Range::FULL_POINTS),
        "22+ points | 22+ HCP lost its HCP floor: {strong_two:?}",
    );
    set_envelope_union_reading(true);
}

/// The [`SuitHcp`] projection folds are exact on the `suit_hcp` axis —
/// including the forward ceiling, the one forward projection that keeps
/// one (own-scale, no upgrade slack exists to make it unsound).
#[test]
fn suit_hcp_folds_are_exact() {
    let ctx = empty_context();
    let slot = |union: &EnvelopeUnion, suit: Suit| {
        union
            .boxes()
            .iter()
            .map(|b| b.strength.suit_hcp[suit as usize])
            .collect::<Vec<_>>()
    };

    set_envelope_union_reading(true);
    let band = suit_hcp(Suit::Spades, 5..=7).project_band(&ctx);
    assert_eq!(slot(&band, Suit::Spades), [Range::new(5, 7)]);
    // Forward = band: the ceiling survives the alert path too.
    let forward = suit_hcp(Suit::Hearts, ..5).project(&ctx);
    assert_eq!(slot(&forward, Suit::Hearts), [Range::new(0, 4)]);
    // A bounded band complements to its two outer halves knob-on…
    let complement = (!suit_hcp(Suit::Spades, 5..=7)).project(&ctx);
    assert_eq!(
        slot(&complement, Suit::Spades),
        [Range::new(0, 4), Range::new(8, 10)],
    );

    // …and knob-off the halves hull back to the full axis.
    set_envelope_union_reading(false);
    let hulled = (!suit_hcp(Suit::Spades, 5..=7)).project(&ctx);
    assert_eq!(hulled.hull(), Envelope::unknown());
    set_envelope_union_reading(true);
}

/// The length ↔ suit-HCP cap tables, exhaustively pinned against every
/// one of the 2¹³ holdings.
///
/// These tables are the *documented contract* for future gauge-membership
/// and stopper work — deliberately **not** live `canonicalize` couplings:
/// each candidate coupling either writes an old axis (a shipped-reading
/// change) or manufactures the containment that lets `EnvelopeUnion::tidy`'s
/// correct dedup swallow the arm carrying the suit knowledge.
#[test]
fn suit_hcp_cap_tables() {
    let mut max_hcp_of_len = [0u8; 14];
    let mut min_len_of_hcp = [u8::MAX; 11];
    let mut min_hcp_of_honors = [u8::MAX; 4];
    let mut min_len_of_honors = [u8::MAX; 4];
    for bits in 0..1u16 << 13 {
        // Rank bit positions run 2..=14.
        let holding = Holding::from_bits_truncate(bits << 2);
        let hcp = eval::hcp::<u8>(holding);
        let len = holding.len();
        let honors = [Rank::A, Rank::K, Rank::Q]
            .into_iter()
            .filter(|&rank| holding.contains(rank))
            .count();
        max_hcp_of_len[len] = max_hcp_of_len[len].max(hcp);
        // SAFETY: a holding has at most 13 cards.
        #[allow(clippy::cast_possible_truncation)]
        let len = len as u8;
        min_len_of_hcp[hcp as usize] = min_len_of_hcp[hcp as usize].min(len);
        min_hcp_of_honors[honors] = min_hcp_of_honors[honors].min(hcp);
        min_len_of_honors[honors] = min_len_of_honors[honors].min(len);
    }
    // A one-card suit holds at most an ace, two AK, three AKQ, four+ AKQJ.
    assert_eq!(
        max_hcp_of_len,
        [0, 4, 7, 9, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10]
    );
    // 1–4 HCP fit in one card, 5–7 need two (AK), 8–9 three (AKQ), 10 four.
    assert_eq!(min_len_of_hcp, [0, 1, 1, 1, 1, 2, 2, 2, 3, 3, 4]);
    // `TopHonors::project`'s floors: cheapest n of {A, K, Q} — Q, QK, QKA —
    // and n top honors are n cards.
    assert_eq!(min_hcp_of_honors, [0, 2, 5, 9]);
    assert_eq!(min_len_of_honors, [0, 1, 2, 3]);
}

#[test]
fn dependency_hooks_are_conservative_and_projection_folds_are_independent() {
    let opaque = pred(|_, _| true);
    assert_eq!(opaque.dependencies(), ConstraintDependencies::ALL);
    assert_eq!(
        opaque.projection_dependencies(),
        ProjectionDependencies::all(ConstraintDependencies::NONE)
    );

    let length = len(Suit::Spades, 5..);
    assert_eq!(length.dependencies(), ConstraintDependencies::HAND);
    assert_eq!(
        length
            .projection_dependencies()
            .get(ProjectionKind::Forward),
        ConstraintDependencies::NONE
    );
    assert_eq!(
        length
            .projection_dependencies()
            .get(ProjectionKind::Complement),
        ConstraintDependencies::PROFILE
    );

    let contextual = support(3..);
    assert!(
        contextual
            .projection_dependencies()
            .get(ProjectionKind::Forward)
            .intersects(ConstraintDependencies::CONTEXT)
    );

    let split = announced(pred(|_, _| true), len(Suit::Hearts, 5..));
    assert_eq!(
        split.projection_dependencies().get(ProjectionKind::Forward),
        ConstraintDependencies::NONE
    );
    assert_eq!(
        split
            .projection_dependencies()
            .get(ProjectionKind::Announcement),
        ConstraintDependencies::NONE
    );
}

#[test]
fn combinator_eval_remains_eager_and_left_to_right() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let context = empty_context();
    let test_hand = hand(BALANCED_15);

    let sequence = Arc::new(AtomicUsize::new(0));
    let left_sequence = Arc::clone(&sequence);
    let right_sequence = Arc::clone(&sequence);
    let conjunction = Cons(move |_: Hand, _: &Context<'_>| {
        assert_eq!(left_sequence.fetch_add(1, Ordering::SeqCst), 0);
        f32::NEG_INFINITY
    }) & Cons(move |_: Hand, _: &Context<'_>| {
        assert_eq!(right_sequence.fetch_add(1, Ordering::SeqCst), 1);
        0.0
    });
    assert_reject(conjunction.eval(test_hand, &context));
    assert_eq!(sequence.load(Ordering::SeqCst), 2);

    let sequence = Arc::new(AtomicUsize::new(0));
    let left_sequence = Arc::clone(&sequence);
    let right_sequence = Arc::clone(&sequence);
    let disjunction = Cons(move |_: Hand, _: &Context<'_>| {
        assert_eq!(left_sequence.fetch_add(1, Ordering::SeqCst), 0);
        0.0
    }) | Cons(move |_: Hand, _: &Context<'_>| {
        assert_eq!(right_sequence.fetch_add(1, Ordering::SeqCst), 1);
        f32::NEG_INFINITY
    });
    assert_pass(disjunction.eval(test_hand, &context));
    assert_eq!(sequence.load(Ordering::SeqCst), 2);
}
