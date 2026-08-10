use super::*;
use crate::bidding::constraint::{and, described, hcp, len, or, points, suit_hcp};
use contract_bridge::Suit;
use rand::SeedableRng;
use rand::rngs::StdRng;

fn hand(text: &str) -> Hand {
    text.parse().expect("valid test hand")
}

/// Sampling at scale finds counterexamples, so 4000 hands pins any
/// suit-length or HCP-bound disagreement with overwhelming probability.
const N: usize = 4000;

fn rng() -> StdRng {
    StdRng::seed_from_u64(0xC0FFEE)
}

#[test]
fn accepts_matches_crisp_eval() {
    let ctx = empty_context();
    assert!(accepts(&hcp(15..=17), hand("AKQ2.K53.QJ4.T92"), &ctx));
    assert!(!accepts(&hcp(18..), hand("AKQ2.K53.QJ4.T92"), &ctx));
}

#[test]
fn identical_constraints_agree() {
    let ctx = empty_context();
    let reference = points(12..=21) & len(Suit::Hearts, 5..);
    let candidate = points(12..=21) & len(Suit::Hearts, 5..);
    let report = compare(
        predicate(&reference, &ctx),
        predicate(&candidate, &ctx),
        &mut rng(),
        N,
    );

    assert_eq!(report.tested, N);
    assert!(report.agrees(), "a faithful recompile must not disagree");
    assert_eq!(report.agreed, N);
    // The clause is reachable, so the oracle is not vacuously accepting none.
    assert!(report.reference_accepts > 0, "5+ hearts openers do occur");
}

#[test]
fn off_by_one_suit_length_is_caught() {
    // The doc's canonical break: "5+ ♥" mis-compiled as four-or-more.
    let ctx = empty_context();
    let reference = len(Suit::Hearts, 5..);
    let candidate = len(Suit::Hearts, 4..);
    let report = compare(
        predicate(&reference, &ctx),
        predicate(&candidate, &ctx),
        &mut rng(),
        N,
    );

    assert!(!report.agrees(), "4+ vs 5+ hearts must disagree");
    // Every witness is a four-card heart holding: accepted by 4+, not 5+.
    for &witness in &report.disagreements {
        assert_eq!(witness[Suit::Hearts].len(), 4, "{witness}");
    }
    // The looser candidate accepts strictly more hands.
    assert!(report.candidate_accepts > report.reference_accepts);
}

#[test]
fn off_by_one_strength_is_caught() {
    let ctx = empty_context();
    let report = compare(
        predicate(&hcp(15..=17), &ctx),
        predicate(&hcp(15..=18), &ctx),
        &mut rng(),
        N,
    );
    assert!(!report.agrees(), "15–17 vs 15–18 HCP must disagree");
    // The looser upper bound accepts the extra 18-HCP hands and no fewer.
    assert!(report.candidate_accepts > report.reference_accepts);
}

#[test]
fn wrong_combinator_is_caught() {
    let ctx = empty_context();
    let reference = hcp(15..=17) & len(Suit::Spades, 5..);
    let candidate = hcp(15..=17) | len(Suit::Spades, 5..);
    let report = compare(
        predicate(&reference, &ctx),
        predicate(&candidate, &ctx),
        &mut rng(),
        N,
    );
    assert!(!report.agrees(), "AND vs OR must disagree");
}

#[test]
fn broken_described_closure_is_caught() {
    // The escape-hatch body the M4.1 round-trip cannot see: intent is
    // "♦ at least as long as ♣" (≥); the candidate implements strict >.
    let ctx = empty_context();
    let reference = described("prefers diamonds", |hand: Hand, _: &Context<'_>| {
        hand[Suit::Diamonds].len() >= hand[Suit::Clubs].len()
    });
    let candidate = described("prefers diamonds", |hand: Hand, _: &Context<'_>| {
        hand[Suit::Diamonds].len() > hand[Suit::Clubs].len()
    });
    // Both round-trip identically (same label) — only behavior tells them apart.
    assert_eq!(reference.describe(), candidate.describe());

    let report = compare(
        predicate(&reference, &ctx),
        predicate(&candidate, &ctx),
        &mut rng(),
        N,
    );
    assert!(!report.agrees(), "≥ vs > on equal lengths must disagree");
    // Witnesses are exactly the equal-length hands the strict form drops.
    for &witness in &report.disagreements {
        assert_eq!(
            witness[Suit::Diamonds].len(),
            witness[Suit::Clubs].len(),
            "{witness}"
        );
    }
}

#[test]
fn check_examples_flags_the_mislabeled_hand() {
    let ctx = empty_context();
    let strong_notrump = hcp(15..=17);
    let examples = [
        (hand("AKQ2.K53.QJ4.T92"), true),  // 15 HCP — accepted, label agrees
        (hand("AKQJ.AKQ.QJ4.T92"), true),  // 20 HCP — label wrong: rejected
        (hand("98432.K53.QJ4.92"), false), // 6 HCP — rejected, label agrees
    ];
    let failures = check_examples(&strong_notrump, &ctx, &examples);
    assert_eq!(failures.len(), 1, "exactly the 20-HCP mislabel fails");
    assert_eq!(failures[0], hand("AKQJ.AKQ.QJ4.T92"));
}

#[test]
fn determinism_same_seed_same_report() {
    let ctx = empty_context();
    let a = compare(
        predicate(&hcp(15..=17), &ctx),
        predicate(&hcp(15..=18), &ctx),
        &mut rng(),
        N,
    );
    let b = compare(
        predicate(&hcp(15..=17), &ctx),
        predicate(&hcp(15..=18), &ctx),
        &mut rng(),
        N,
    );
    assert_eq!(a.tested, b.tested);
    assert_eq!(a.agreed, b.agreed);
    assert_eq!(a.disagreements, b.disagreements);
}

/// The projection soundness invariant: every hand a constraint accepts must
/// fall within the forward `Envelope` `project` reports.  A
/// violation is a witness hand inside `eval` but outside `project` — exactly
/// the bug that would let the forward reader under-constrain a player and
/// raise a phantom suit.  Spans primitives, conjunction, the disjoint-suit
/// disjunctions of Landy/Multi, a negative-inference shape, and the opaque
/// escape hatch (which must stay sound by projecting no info).
#[test]
fn projection_contains_every_accepted_hand() {
    use crate::bidding::constraint::Constraint;

    let ctx = empty_context();
    let battery: [Box<dyn Constraint>; 14] = [
        Box::new(len(Suit::Hearts, 5..)),
        Box::new(points(8..=16)),
        Box::new(hcp(15..=17)),
        Box::new(len(Suit::Hearts, 5..) & points(8..)),
        Box::new(
            (len(Suit::Hearts, 5..) & len(Suit::Spades, 4..))
                | (len(Suit::Hearts, 4..) & len(Suit::Spades, 5..)),
        ),
        Box::new(len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..)),
        Box::new(len(Suit::Spades, ..4) & points(8..)),
        Box::new(described("opaque", |_: Hand, _: &Context<'_>| true)),
        // The `and`/`or` suit-set combinators (M6.2d): `and` floors every named
        // suit (tight), `or` unions the arms (loose — must stay sound).
        Box::new(and([Suit::Hearts, Suit::Spades], 4..)),
        Box::new(and([Suit::Hearts, Suit::Spades], 4..) & or([Suit::Hearts, Suit::Spades], 5..)),
        Box::new(or([Suit::Hearts, Suit::Spades], 6..) & and([Suit::Clubs, Suit::Diamonds], ..=4)),
        // The per-suit HCP axis: alone, `&`-composed with its length (the
        // stopper-quality idiom), and `|`-composed across suits.
        Box::new(suit_hcp(Suit::Clubs, 5..)),
        Box::new(len(Suit::Clubs, 5..) & suit_hcp(Suit::Clubs, 5..)),
        Box::new(suit_hcp(Suit::Hearts, 4..) | suit_hcp(Suit::Spades, 4..)),
    ];

    let mut rng = rng();
    for constraint in &battery {
        let envelope = constraint.project(&ctx);
        for hand in random_hands(&mut rng).take(N) {
            if constraint.eval(hand, &ctx) > f32::NEG_INFINITY {
                assert!(
                    envelope.contains(hand),
                    "projection unsound: {hand} accepted but outside {envelope:?}"
                );
                // The lenient `contains` never reads the gauge axes; the
                // strict sibling holds the ceilings to account too.
                assert!(
                    envelope.boxes().iter().any(|b| b.accepts(hand)),
                    "projection unsound strictly: {hand} accepted but no box \
                     of {envelope:?} accepts it"
                );
            }
        }
    }
}

/// M6.2b equivalence anchor: the generic `authored_reading` projection pass
/// reproduces the hand-written declarative `*_reading` decoders, signature
/// suit ranges and points, straight off the rule.
///
/// The readers re-derive a convention's meaning by hand off the auction shape;
/// the projection pass reads it off the authored rule's own `len`/`points`
/// constraint, the single source of truth.  Three declarative anchors:
/// `transfer_major_reading` (the cleanest, uncontested), `leaping_michaels`,
/// and `landy` core — each on a *prefixed* context via `Stance`, the trie
/// access M6.2c will wire into the keyless sampler/features paths for real.
/// Opaque (`described()`) conventions project no info and need M6.2d, so they
/// are out of this harness.
#[test]
fn projection_reproduces_the_declarative_readers() {
    use crate::american;
    use crate::bidding::agreements::Agreements;
    use crate::bidding::inference::{Inferences, Range, Relative, authored_reading};
    use contract_bridge::auction::{Call, RelativeVulnerability};
    use contract_bridge::{Bid, Level, Strain};

    let bid = |level, strain| {
        Call::Bid(Bid {
            level: Level::new(level),
            strain,
        })
    };
    let full = Range::new(0, 37);

    // Project and read on the same prefixed context; assert the projection pass
    // pins the reader's exact ranges on the convention's signature seat.
    let agree = |agreements: &Agreements,
                 auction: &[Call],
                 who: Relative,
                 suits: &[(Suit, Range)],
                 points: Range| {
        let stance = american(agreements).against();
        let ctx = stance.prefixed_context(RelativeVulnerability::NONE, auction);
        let reader = *Inferences::read(&ctx).get(who);
        let projected = *authored_reading(&ctx).get(who);
        for &(suit, want) in suits {
            assert_eq!(
                reader.length(suit),
                want,
                "reader oracle drifted on {suit:?}"
            );
            assert_eq!(
                projected.length(suit),
                want,
                "projection diverged from reader on {suit:?}"
            );
        }
        assert_eq!(
            reader.strength.points, points,
            "reader points oracle drifted"
        );
        assert_eq!(
            projected.strength.points, points,
            "projection points diverged"
        );
    };

    // Jacoby transfer to hearts (on by default): `1NT - 2♦ - 2♥ -`, the
    // responder is Me at length 6; the 2♦ rule is `len(♥,5..) & …`.
    agree(
        &Agreements::default(),
        &[
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
        ],
        Relative::Me,
        &[(Suit::Hearts, Range::new(5, 13))],
        full,
    );

    // Leaping Michaels: (2♥) 4♣ - = clubs + the other major (spades), 14+;
    // partner at length 3.  `len(♣,5..) & len(♠,5..) & points(14..)`.
    let mut leaping = Agreements::default();
    leaping.defense.leaping_michaels_enabled = true;
    agree(
        &leaping,
        &[bid(2, Strain::Hearts), bid(4, Strain::Clubs), Call::Pass],
        Relative::Partner,
        &[
            (Suit::Clubs, Range::new(5, 13)),
            (Suit::Spades, Range::new(5, 13)),
        ],
        Range::new(14, 37),
    );

    // Landy: (1NT) 2♣ - = both majors, at least 4-4, 8+; partner at length 3.
    // `((len(♥,5..)&len(♠,4..)) | (len(♥,4..)&len(♠,5..))) & points(8..)`.
    let mut landy = Agreements::default();
    landy.decision.reading.landy = true;
    landy.decision.reading.convention_points = (8, 15);
    landy.defense.leaping_michaels_enabled = false;
    agree(
        &landy,
        &[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass],
        Relative::Partner,
        &[
            (Suit::Hearts, Range::new(4, 13)),
            (Suit::Spades, Range::new(4, 13)),
        ],
        Range::new(8, 37),
    );
}
