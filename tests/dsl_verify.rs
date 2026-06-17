//! Behavioral verification for the authoring compiler (AI-bidder M4.2)
//!
//! The companion to [`dsl_roundtrip.rs`].  Where the round-trip check proves a
//! compiled `Constraint` *renders* to the intended gloss (a string compare), this
//! proves it *accepts the intended hands* (a behavioral compare over a random
//! sample).  The two are complementary: the round-trip is blind to the body of a
//! [`described`] escape hatch and to whether a primitive's bounds match looser
//! human intent, and those are exactly what this catches.
//!
//! The **M4.2 measure** is [`broken_constraints_are_caught`] and
//! [`broken_described_closure_is_caught`]: a battery of deliberately-mis-compiled
//! constraints, each of which the verifier flags with counterexamples, while the
//! faithful recompiles in [`faithful_recompile_agrees`] pass clean.  This is a
//! black-box test — it uses only the public [`pons::bidding::verify`] API, exactly
//! as the compiler's consumer will.
//!
//! [`described`]: pons::bidding::constraint::described

use contract_bridge::{Hand, Suit};
use pons::bidding::Context;
use pons::bidding::constraint::{Constraint, balanced, described, hcp, len, points};
use pons::bidding::verify::{check_examples, compare, empty_context, predicate};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// Enough hands that any single suit-length or HCP-bound disagreement surfaces
/// with overwhelming probability (a four-card suit alone is ~35% of hands).
const N: usize = 8000;

fn rng() -> StdRng {
    StdRng::seed_from_u64(0x_004D_3420)
}

fn hand(text: &str) -> Hand {
    text.parse().expect("valid test hand")
}

/// Compare two constraints at the empty context over `N` sampled hands.
fn agree(reference: impl Constraint, candidate: impl Constraint) -> bool {
    let ctx = empty_context();
    compare(
        predicate(&reference, &ctx),
        predicate(&candidate, &ctx),
        &mut rng(),
        N,
    )
    .agrees()
}

/// A faithful recompile of a real rule's gloss accepts exactly its hands.
#[test]
fn faithful_recompile_agrees() {
    // 1♠ opening: "12–21 points, and 5+ ♠".
    assert!(agree(
        points(12..=21) & len(Suit::Spades, 5..),
        points(12..=21) & len(Suit::Spades, 5..),
    ));
    // strong 1NT: "15–17 HCP, and balanced".
    assert!(agree(hcp(15..=17) & balanced(), hcp(15..=17) & balanced()));
    // weak 2♥: "exactly 6 ♥, 5–10 points" — alternate but equal range spellings.
    assert!(agree(
        len(Suit::Hearts, 6..=6) & points(5..=10),
        len(Suit::Hearts, 6..7) & points(5..=10),
    ));
}

/// The M4.2 measure: each deliberately-broken constraint is flagged.
#[test]
fn broken_constraints_are_caught() {
    // The doc's canonical break: "5+ ♥" mis-compiled as four-or-more.
    assert!(!agree(len(Suit::Hearts, 5..), len(Suit::Hearts, 4..)));
    // Off-by-one strength band.
    assert!(!agree(hcp(15..=17), hcp(15..=18)));
    // Wrong combinator: conjunction read as disjunction.
    assert!(!agree(
        hcp(15..=17) & len(Suit::Spades, 5..),
        hcp(15..=17) | len(Suit::Spades, 5..),
    ));
    // A dropped clause: the candidate forgot the suit-length requirement.
    assert!(!agree(
        points(12..=21) & len(Suit::Spades, 5..),
        points(12..=21),
    ));
    // A spurious extra clause: the candidate over-constrains.
    assert!(!agree(
        points(12..=21),
        points(12..=21) & len(Suit::Spades, 5..),
    ));
}

/// Counterexamples pin *why* a break disagrees — the canonical case in detail.
#[test]
fn counterexamples_explain_the_break() {
    let ctx = empty_context();
    let report = compare(
        predicate(&len(Suit::Hearts, 5..), &ctx),
        predicate(&len(Suit::Hearts, 4..), &ctx),
        &mut rng(),
        N,
    );
    assert!(!report.agrees());
    assert!(!report.disagreements.is_empty());
    // Every witness holds exactly four hearts: the looser candidate's extra hands.
    for &witness in &report.disagreements {
        assert_eq!(witness[Suit::Hearts].len(), 4, "{witness}");
    }
    // The looser bound accepts strictly more hands than the reference.
    assert!(report.candidate_accepts > report.reference_accepts);
}

/// The escape-hatch body the round-trip cannot see: same label, different logic.
#[test]
fn broken_described_closure_is_caught() {
    // Intent: "♦ at least as long as ♣" (≥).  Candidate implements strict >.
    let reference = described("prefers diamonds", |hand: Hand, _: &Context<'_>| {
        hand[Suit::Diamonds].len() >= hand[Suit::Clubs].len()
    });
    let candidate = described("prefers diamonds", |hand: Hand, _: &Context<'_>| {
        hand[Suit::Diamonds].len() > hand[Suit::Clubs].len()
    });
    // Identical glosses: the M4.1 round-trip cannot tell them apart.
    assert_eq!(reference.describe(), candidate.describe());
    // Behavior can.
    assert!(!agree(reference, candidate));
}

/// `check_examples` is the oracle for meanings with no reference constraint:
/// a handful of textbook hands the author knows the verdict for.
#[test]
fn check_examples_flags_mislabels() {
    let ctx = empty_context();
    let strong_notrump = hcp(15..=17);
    let examples = [
        (hand("AKQ2.K53.QJ4.T92"), true),  // 15 HCP — correctly in range
        (hand("AKQJ.AKQ.QJ4.T92"), true),  // 20 HCP — label is wrong: out of range
        (hand("98432.K53.QJ4.92"), false), // 6 HCP — correctly out of range
    ];
    let failures = check_examples(&strong_notrump, &ctx, &examples);
    assert_eq!(failures, vec![hand("AKQJ.AKQ.QJ4.T92")]);
}
