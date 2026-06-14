//! Behavioral constraint verifier (AI-bidder M4.2)
//!
//! The runnable face of [`pons::bidding::verify`] and the template the Polish Club
//! port (M4.3) will drive.  The authoring compiler turns an English gloss into a
//! candidate `Constraint`; M4.1's round-trip proves it *renders* back to the
//! gloss, but a string compare cannot see whether it *accepts the right hands*.
//! This example shows the behavioral check that can:
//!
//! 1. **Porting against a real rule.**  It pulls the 1♠ opening straight from the
//!    2/1 books, treats its accept set as intent, and compares two recompiles of
//!    its gloss — a faithful one (agrees) and a deliberately-broken one (caught,
//!    with counterexample hands).
//! 2. **The escape-hatch blind spot.**  Two `described("prefers diamonds", …)`
//!    closures that render to the *same* gloss — so the round-trip cannot tell
//!    them apart — are separated here by behavior alone.  This is the core reason
//!    M4.2 exists: it checks the closure body the label hides.
//!
//! Run with `cargo run --example verify-constraint`.

use contract_bridge::auction::Call;
use contract_bridge::{Hand, Suit};
use pons::bidding::Context;
use pons::bidding::constraint::{Constraint, described, len, points};
use pons::bidding::rules::Rule;
use pons::bidding::trie::Trie;
use pons::bidding::two_over_one::bare_two_over_one;
use pons::bidding::verify::{Report, compare, empty_context, predicate};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// Hands drawn per comparison — enough to surface any one-suit or HCP-bound break.
const N: usize = 8000;

fn main() {
    let mut rng = StdRng::seed_from_u64(0x_005E_A1ED);
    let ctx = empty_context();

    porting_against_a_real_rule(&ctx, &mut rng);
    the_escape_hatch_blind_spot(&ctx, &mut rng);
}

/// Demonstration 1: recompile a real book rule's gloss, faithfully and broken.
fn porting_against_a_real_rule(ctx: &Context<'_>, rng: &mut StdRng) {
    println!("── porting against a real book rule ─────────────────────────────");

    // The 1♠ opening, read straight from the constructive book.
    let rule = opening_rule("12–21 points, and 5+ ♠").expect("the 1♠ opener exists");
    println!(
        "reference rule:  {}  ⇒  \"{}\"",
        rule.call(),
        rule.describe()
    );

    // The oracle: a real rule accepts a hand iff its logit is finite.
    let reference = |hand: Hand| rule.eval(hand, ctx).is_finite();

    // A faithful recompile of the gloss per the DSL spec.
    let faithful = points(12..=21) & len(Suit::Spades, 5..);
    report_line(
        "faithful  points(12..=21) & len(♠, 5..)",
        &compare(reference, predicate(&faithful, ctx), rng, N),
    );

    // A deliberate break: the suit length dropped from 5+ to 4+.
    let broken = points(12..=21) & len(Suit::Spades, 4..);
    report_line(
        "broken    points(12..=21) & len(♠, 4..)",
        &compare(reference, predicate(&broken, ctx), rng, N),
    );
    println!();
}

/// Demonstration 2: two closures with one gloss, told apart by behavior.
fn the_escape_hatch_blind_spot(ctx: &Context<'_>, rng: &mut StdRng) {
    println!("── the described() escape-hatch blind spot ──────────────────────");

    // Intent: "♦ at least as long as ♣" (≥) — the books' actual 1♦ predicate.
    let reference = described("prefers diamonds", |hand: Hand, _: &Context<'_>| {
        hand[Suit::Diamonds].len() >= hand[Suit::Clubs].len()
    });
    // A plausible mis-compile: strict > drops the equal-length hands.
    let broken = described("prefers diamonds", |hand: Hand, _: &Context<'_>| {
        hand[Suit::Diamonds].len() > hand[Suit::Clubs].len()
    });

    println!(
        "both render to:  \"{}\"  — the round-trip (M4.1) sees no difference",
        reference.describe()
    );
    report_line(
        "broken    > instead of ≥",
        &compare(predicate(&reference, ctx), predicate(&broken, ctx), rng, N),
    );
    println!();
}

/// Find a rule in the constructive opening node by its rendered gloss.
fn opening_rule(gloss: &str) -> Option<Rule> {
    let pair = bare_two_over_one();
    let opening = find_node(&pair.constructive.0, &[])?;
    opening
        .rules()
        .iter()
        .find(|rule| rule.describe().to_string() == gloss)
        .cloned()
}

/// The authored rules classifying a given auction prefix, if any.
fn find_node<'a>(trie: &'a Trie, auction: &[Call]) -> Option<&'a pons::bidding::Rules> {
    trie.iter()
        .find(|(prefix, _)| prefix.as_ref() == auction)
        .and_then(|(_, classifier)| classifier.as_rules())
}

/// Print one comparison: a verdict, the accept rates, and a few witnesses.
fn report_line(label: &str, report: &Report) {
    if report.agrees() {
        println!(
            "  {label:<40}  {} hands, 0 disagreements ✓  (accepts {}/{})",
            report.tested, report.candidate_accepts, report.tested,
        );
        return;
    }

    let disagreements = report.tested - report.agreed;
    println!(
        "  {label:<40}  {disagreements} disagreements ✗  \
         (reference {}, candidate {} of {})",
        report.reference_accepts, report.candidate_accepts, report.tested,
    );
    for hand in report.disagreements.iter().take(3) {
        println!("      e.g. {hand}");
    }
}
