use super::*;
use contract_bridge::Strain;
use contract_bridge::auction::RelativeVulnerability;

const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid {
        level: contract_bridge::Level::new(level),
        strain,
    })
}

fn empty_context() -> Context<'static> {
    Context::new(RelativeVulnerability::NONE, &[])
}

#[test]
fn test_first_is() {
    let guard = FirstIs(Call::Double);
    let context = empty_context();
    assert!(guard.admits(&context, &[Call::Double]));
    assert!(guard.admits(&context, &[Call::Double, Call::Pass]));
    assert!(!guard.admits(&context, &[Call::Pass, Call::Double]));
    assert!(!guard.admits(&context, &[]));
}

#[test]
fn test_overcall_at_most() {
    let guard = OvercallAtMost(Bid::new(2, Strain::Spades));
    let context = empty_context();
    assert!(guard.admits(&context, &[bid(1, Strain::Spades)]));
    assert!(guard.admits(&context, &[bid(2, Strain::Spades)]));
    assert!(!guard.admits(&context, &[bid(2, Strain::Notrump)]));
    assert!(!guard.admits(&context, &[Call::Double]));
    assert!(!guard.admits(&context, &[bid(1, Strain::Spades), Call::Pass]));
}

#[test]
fn test_suffix_is() {
    let guard = SuffixIs(vec![bid(2, Strain::Hearts), Call::Double, Call::Pass]);
    let context = empty_context();
    assert!(guard.admits(
        &context,
        &[bid(2, Strain::Hearts), Call::Double, Call::Pass]
    ));
    assert!(!guard.admits(&context, &[bid(2, Strain::Hearts), Call::Double]));
    assert!(!guard.admits(&context, &[]));
    assert!(SuffixIs(vec![]).admits(&context, &[]));
    assert_eq!(guard.describe().expect("self-describing"), "2♥ X -");
}

#[test]
fn test_described_wrappers() {
    let guard = described_guard("(their overcall) cue -", Always);
    let context = empty_context();
    assert!(guard.admits(&context, &[Call::Double]));
    assert_eq!(guard.describe().as_deref(), Some("(their overcall) cue -"));
    assert_eq!(guard.plan(), GuardPlan::Always);

    let rewrite = described_rewrite("systems on", ReplaceNext(Call::Pass));
    assert_eq!(rewrite.describe().as_deref(), Some("systems on"));
    assert_eq!(rewrite.plan(), RewritePlan::ReplaceNext(Call::Pass));
    assert_eq!(
        rewrite.rewrite(&[bid(1, Strain::Notrump), Call::Double], 1),
        Some(vec![bid(1, Strain::Notrump), Call::Pass]),
    );
}

#[test]
fn test_replace_next() {
    let rewrite = ReplaceNext(Call::Pass);
    let auction = [bid(1, Strain::Notrump), Call::Double, Call::Pass];

    assert_eq!(
        rewrite.rewrite(&auction, 1),
        Some(vec![bid(1, Strain::Notrump), Call::Pass, Call::Pass]),
    );
    assert_eq!(rewrite.rewrite(&auction, 3), None);
}

#[test]
fn test_builtin_plans() {
    let suffix = vec![bid(2, Strain::Hearts), Call::Double];

    assert_eq!(Always.plan(), GuardPlan::Always);
    assert_eq!(Undisturbed.plan(), GuardPlan::Undisturbed);
    assert_eq!(
        FirstIs(Call::Double).plan(),
        GuardPlan::FirstIs(Call::Double),
    );
    assert_eq!(
        OvercallAtMost(Bid::new(2, Strain::Spades)).plan(),
        GuardPlan::OvercallAtMost(Bid::new(2, Strain::Spades)),
    );
    assert_eq!(SuffixIs(suffix.clone()).plan(), GuardPlan::SuffixIs(suffix),);
    assert_eq!(
        ReplaceNext(Call::Pass).plan(),
        RewritePlan::ReplaceNext(Call::Pass),
    );
}

#[test]
fn test_closure_plans_are_opaque() {
    let closure_guard = guard(|_: &Context<'_>, suffix: &[Call]| suffix.is_empty());
    let closure_rewrite = rewriter(|auction: &[Call], _: usize| Some(auction.to_vec()));

    assert_eq!(closure_guard.plan(), GuardPlan::Opaque);
    assert_eq!(closure_rewrite.plan(), RewritePlan::Opaque);

    let context = empty_context();
    assert!(closure_guard.admits(&context, &[]));
    assert_eq!(
        closure_rewrite.rewrite(&[Call::Pass], 0),
        Some(vec![Call::Pass]),
    );
}
