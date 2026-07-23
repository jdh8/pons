//! Behavioral verification for the authoring compiler (AI-bidder M4.2)
//!
//! The authoring compiler (`docs/ai-bidder/dsl-spec.md`) turns an English
//! gloss into a [`Constraint`][crate::bidding::constraint::Constraint].  Milestone M4.1's round-trip check is a *string*
//! compare — `compiled.describe().to_string() == gloss` — which proves the
//! compiled tree *renders* to the intended meaning.  That check is blind in
//! exactly two places the compiler can still go wrong:
//!
//! 1. The body of a [`described`][crate::bidding::constraint::described()] escape hatch.
//!    `describe()` renders only the *label*, so a closure for "prefers diamonds"
//!    or "exactly 2 keycards" could accept the wrong hands and round-trip anyway.
//! 2. Porting from looser human notes (M4.3), where "matches the original rule"
//!    is a question about *which hands are accepted*, not about a string.
//!
//! This module closes both with a **behavioral** check: sample random hands,
//! compare a candidate's accept/reject set against an intent oracle (the original
//! rule when porting, or hand-labeled examples), and surface counterexamples.
//! The model proposes; this deterministic Rust check disposes — an LLM
//! mis-compilation becomes a failing test, not a silent bidding bug.
//!
//! # What "accepts" means
//!
//! A crisp [`Constraint`][crate::bidding::constraint::Constraint] contributes `0.0` when satisfied and
//! [`f32::NEG_INFINITY`] when violated; the trait forbids `+∞`, so **finite ⇔
//! satisfied**.  [`accepts`] is therefore `eval(hand, ctx) > f32::NEG_INFINITY`,
//! the very test `classify` and
//! [`explain`][super::Rules::explain] use to admit a call.  All current
//! primitives are crisp; a fuzzy evaluator would need a threshold instead, which
//! this first cut does not model.
//!
//! # Scope and honest limits
//!
//! - **Fixed context.** Comparison is over the *hand* space at a caller-supplied
//!   [`Context`] (an empty one by default).  The dominant intent disagreements —
//!   shape, strength, and every [`described`][crate::bidding::constraint::described()] hand predicate — are context-free,
//!   as is the canonical soundness case ("5+ ♥" must not accept four-card
//!   holdings).  Varying the context across legal auctions is future work.
//! - **Sampling, not proof.** A disagreement confined to a single rare holding
//!   can be missed by a finite sample, so callers pick `n` large (the tests and
//!   example use several thousand) — enough that any off-by-one bound or wrong
//!   comparator surfaces with overwhelming probability.  Agreement here is strong
//!   evidence, not a proof of equivalence.

use super::constraint::Constraint;
use super::context::Context;
use contract_bridge::auction::RelativeVulnerability;
use contract_bridge::deck::full_deal;
use contract_bridge::{Hand, Seat};
use rand::Rng;

/// The most counterexample hands a [`Report`] retains
///
/// A disagreement is a bug to fix, not a statistic to total precisely; a handful
/// of witnesses is enough to diagnose it, and the bound keeps the report cheap to
/// build and print.
const MAX_COUNTEREXAMPLES: usize = 16;

/// Whether a constraint accepts a hand in a context
///
/// The crisp-accept convention of the module: a finite logit means satisfied.
#[must_use]
pub fn accepts(constraint: &impl Constraint, hand: Hand, context: &Context<'_>) -> bool {
    constraint.eval(hand, context) > f32::NEG_INFINITY
}

/// View a constraint as a fixed-context hand predicate
///
/// The common adapter for [`compare`]: borrows the constraint and a context and
/// returns `|hand| accepts(constraint, hand, context)`.  A book [`Rule`] is used
/// directly instead — `|hand| rule.eval(hand, ctx).is_finite()` — since its
/// constraint is private but its [`eval`][super::rules::Rule::eval] is not.
///
/// [`Rule`]: super::rules::Rule
pub fn predicate<'a>(
    constraint: &'a impl Constraint,
    context: &'a Context<'a>,
) -> impl Fn(Hand) -> bool + 'a {
    move |hand| accepts(constraint, hand, context)
}

/// Outcome of a behavioral comparison over sampled hands
///
/// `reference` is the intent oracle (the original rule, or hand labels);
/// `candidate` is the compiler's output.  The accept counts are a cheap sanity
/// signal on their own: a candidate that accepts *nothing* (`candidate_accepts ==
/// 0`) is a common mis-compile visible before reading any counterexample.
#[derive(Clone, Debug)]
pub struct Report {
    /// How many hands were drawn and evaluated
    pub tested: usize,
    /// How many hands the two predicates agreed on
    pub agreed: usize,
    /// How many hands the reference accepted (its accept rate over `tested`)
    pub reference_accepts: usize,
    /// How many hands the candidate accepted
    pub candidate_accepts: usize,
    /// A bounded sample of hands where the two disagreed (the witnesses)
    pub disagreements: Vec<Hand>,
}

impl Report {
    /// Whether the candidate matched the reference on every sampled hand
    #[must_use]
    pub fn agrees(&self) -> bool {
        self.disagreements.is_empty()
    }
}

/// An iterator of uniform random hands drawn from full deals
///
/// Each `full_deal` is a uniform shuffle, so its four hands are four uniform
/// 13-card hands (a sound population though not mutually independent — they
/// partition one deck).  Taking all four amortizes the shuffle across four
/// samples, which matters when `compare` draws several thousand.
fn random_hands(rng: &mut impl Rng) -> impl Iterator<Item = Hand> + '_ {
    core::iter::repeat_with(move || full_deal(rng))
        .flat_map(|deal| Seat::ALL.map(|seat| deal[seat]))
}

/// Sample `n` random hands and report where two predicates disagree
///
/// `reference` is intent, `candidate` is the compiler's output (see [`Report`]).
/// For a constraint, wrap it with [`predicate`]; for a book rule, pass
/// `|hand| rule.eval(hand, ctx).is_finite()`.  Up to `MAX_COUNTEREXAMPLES`
/// disagreeing hands are retained as witnesses; the counts always cover all `n`.
pub fn compare(
    reference: impl Fn(Hand) -> bool,
    candidate: impl Fn(Hand) -> bool,
    rng: &mut impl Rng,
    n: usize,
) -> Report {
    let mut report = Report {
        tested: 0,
        agreed: 0,
        reference_accepts: 0,
        candidate_accepts: 0,
        disagreements: Vec::new(),
    };

    for hand in random_hands(rng).take(n) {
        let want = reference(hand);
        let got = candidate(hand);
        report.tested += 1;
        report.reference_accepts += usize::from(want);
        report.candidate_accepts += usize::from(got);
        if want == got {
            report.agreed += 1;
        } else if report.disagreements.len() < MAX_COUNTEREXAMPLES {
            report.disagreements.push(hand);
        }
    }
    report
}

/// Check a constraint against hand-labeled intent, returning the failing hands
///
/// Each example is a hand and whether intent says the constraint should accept
/// it; a returned hand is one the constraint classified against its label.  This
/// is the oracle for meanings with no natural reference constraint — a handful of
/// textbook hands the author *knows* the right verdict for.
#[must_use]
pub fn check_examples(
    constraint: &impl Constraint,
    context: &Context<'_>,
    examples: &[(Hand, bool)],
) -> Vec<Hand> {
    examples
        .iter()
        .filter(|&&(hand, want)| accepts(constraint, hand, context) != want)
        .map(|&(hand, _)| hand)
        .collect()
}

/// An empty auction context — the default ground for a context-free comparison
///
/// Most constraints the compiler authors (shape, strength, every [`described`]
/// hand predicate) ignore the auction, so an empty, non-vulnerable context is the
/// natural place to verify them.
///
/// [`described`]: super::constraint::described
#[must_use]
pub fn empty_context() -> Context<'static> {
    Context::new(RelativeVulnerability::NONE, &[])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::constraint::{and, described, hcp, len, or, points};
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
        let battery: [Box<dyn Constraint>; 11] = [
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
            Box::new(
                and([Suit::Hearts, Suit::Spades], 4..) & or([Suit::Hearts, Suit::Spades], 5..),
            ),
            Box::new(
                or([Suit::Hearts, Suit::Spades], 6..) & and([Suit::Clubs, Suit::Diamonds], ..=4),
            ),
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
        use crate::bidding::Family;
        use crate::bidding::american::{set_landy, set_leaping_michaels};
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
        let agree = |auction: &[Call], who: Relative, suits: &[(Suit, Range)], points: Range| {
            let stance = american().against(Family::NATURAL);
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

        // Jacoby transfer to hearts (on by default): [1NT, P, 2♦, P, 2♥, P], the
        // responder is Me at length 6; the 2♦ rule is `len(♥,5..) & …`.
        agree(
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

        // Leaping Michaels: (2♥)–4♣–(P) = clubs + the other major (spades), 14+;
        // partner at length 3.  `len(♣,5..) & len(♠,5..) & points(14..)`.
        set_leaping_michaels(true);
        agree(
            &[bid(2, Strain::Hearts), bid(4, Strain::Clubs), Call::Pass],
            Relative::Partner,
            &[
                (Suit::Clubs, Range::new(5, 13)),
                (Suit::Spades, Range::new(5, 13)),
            ],
            Range::new(14, 37),
        );
        set_leaping_michaels(false);

        // Landy: (1NT)–2♣–(P) = both majors, at least 4-4, 8+; partner at length 3.
        // `((len(♥,5..)&len(♠,4..)) | (len(♥,4..)&len(♠,5..))) & points(8..)`.
        set_landy(Some((8, 15)));
        agree(
            &[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass],
            Relative::Partner,
            &[
                (Suit::Hearts, Range::new(4, 13)),
                (Suit::Spades, Range::new(4, 13)),
            ],
            Range::new(8, 37),
        );
        set_landy(None);
    }
}
