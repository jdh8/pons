use super::*;
use crate::bidding::Rules;
use crate::bidding::constraint::{hcp, partner_shown_points};
use crate::bidding::fallback::{Always, FirstIs, ReplaceNext};
use contract_bridge::auction::RelativeVulnerability;
use contract_bridge::{Bid, Strain};

/// A deliberately partial book node — it only passes weak hands — must not
/// shadow the floor: a strong hand it rejects (all-`-∞` logits) falls
/// through to the total floor rather than leaving the driver with no call.
/// This is the 7NT degenerate-result regression.
#[test]
fn partial_node_falls_through_to_the_floor() {
    let auction = [Call::Bid(Bid::new(1, Strain::Clubs))];
    let weak_only = Rules::new().rule(Call::Pass, 0, hcp(..6) & partner_shown_points(0..));
    // A total floor: `hcp(0..)` accepts every hand, so Pass is always finite.
    let floor = Rules::new().rule(Call::Pass, 0, hcp(0..) & partner_shown_points(0..));

    let mut trie = Trie::new();
    trie.insert(&auction, weak_only);
    trie.fallback_at(&[], Always, Fallback::classify(floor));

    let strong: Hand = "AKQ2.KQ5.AQJ4.92".parse().expect("valid test hand");
    let uncached = Context::new(RelativeVulnerability::NONE, &auction);

    // The exact node alone rejects this 21-count: all-`-∞`, no mass.
    let (exact, _) = trie.resolve(&uncached, &auction).expect("exact node");
    assert!(!exact.classify(strong, &uncached).has_mass());

    // `classify_floored` falls through to the total floor instead. Both
    // ladders consult the full-auction reading, but the decision scope
    // initializes it only once across exact rejection and fallback.
    let context = Context::new(RelativeVulnerability::NONE, &auction).with_decision_cache(strong);
    let (logits, provenance) = trie
        .classify_floored(strong, &context, &auction)
        .expect("the floor answers");
    assert!(logits.has_mass(), "the floor gives the hand a finite call");
    assert_eq!(provenance.depth, 0, "the answer came from the root floor");
    assert!(
        provenance.fallback.is_some(),
        "via a fallback, not the book"
    );
    assert_eq!(context.decision_cache_init_counts(), Some((1, 0, 0)));
}

/// A node that *does* cover the hand keeps its own answer — fall-through
/// triggers only on a no-mass result, never overriding a live book rule.
#[test]
fn exact_node_with_mass_is_not_floored() {
    let auction = [Call::Bid(Bid::new(1, Strain::Clubs))];
    let opener = Rules::new().rule(Call::Pass, 0, hcp(0..));
    let floor = Rules::new().rule(Call::Pass, -500, hcp(0..));

    let mut trie = Trie::new();
    trie.insert(&auction, opener);
    trie.fallback_at(&[], Always, Fallback::classify(floor));

    let hand: Hand = "AKQ2.KQ5.AQJ4.92".parse().expect("valid test hand");
    let context = Context::new(RelativeVulnerability::NONE, &auction);
    let (_, provenance) = trie
        .classify_floored(hand, &context, &auction)
        .expect("the node answers");
    assert_eq!(
        provenance.fallback, None,
        "the exact node wins, not the floor"
    );
}

#[test]
fn rebase_guard_and_rewritten_classifier_share_the_decision_cache() {
    let one_nt = Call::Bid(Bid::new(1, Strain::Notrump));
    let two_hearts = Call::Bid(Bid::new(2, Strain::Hearts));
    let rewritten = [one_nt, Call::Pass, two_hearts];
    let auction = [one_nt, Call::Double, two_hearts];
    let rules = Rules::new().rule(Call::Pass, 0, hcp(0..) & partner_shown_points(0..));

    let mut trie = Trie::new();
    trie.insert(&rewritten, rules);
    trie.fallback_at(
        &[one_nt],
        |context: &Context<'_>, suffix: &[Call]| {
            let _ = context.inferences();
            FirstIs(Call::Double).admits(context, suffix)
        },
        Fallback::rebase(ReplaceNext(Call::Pass)),
    );

    let hand: Hand = "AKQ2.KQ5.AQJ4.92".parse().expect("valid test hand");
    let context = Context::new(RelativeVulnerability::NONE, &auction).with_decision_cache(hand);
    let (_, provenance) = trie
        .classify_floored(hand, &context, &auction)
        .expect("the rewritten node answers");
    assert_eq!(provenance.rebases, 1);
    assert_eq!(context.decision_cache_init_counts(), Some((1, 0, 0)));
}

/// [`Trie::fallbacks`] yields every entry, in declaration order within a
/// node, and visits the Pass child last — a seat-fanned entry (one `Arc`
/// shared under leading-pass prefixes) surfaces its pass-less key first,
/// so the renderers' first-seen dedup keeps the canonical heading.
#[test]
fn fallbacks_enumerate_pass_less_key_first() {
    use crate::bidding::fallback::{Guard, SuffixIs};
    use std::sync::Arc;

    let opening = [Call::Bid(Bid::new(1, Strain::Spades))];
    let seat_two: Vec<Call> = core::iter::once(Call::Pass)
        .chain(opening.iter().copied())
        .collect();

    let rules = || Fallback::classify(Rules::new().rule(Call::Pass, 0, hcp(0..)));
    let shared: Arc<dyn Guard> = Arc::new(SuffixIs(vec![Call::Double]));

    let mut trie = Trie::new();
    // Declaration order within the `1♠` node: first the double guard,
    // then an Always entry.
    trie.fallback_arc_at(&opening, Arc::clone(&shared), rules());
    trie.fallback_at(&opening, Always, rules());
    // The seat-fanned variant of the first entry, under a leading pass.
    trie.fallback_arc_at(&seat_two, Arc::clone(&shared), rules());

    let all = trie.fallbacks();
    let keys: Vec<&[Call]> = all.iter().map(|(key, ..)| &**key).collect();
    assert_eq!(
        keys,
        [&opening[..], &opening[..], &seat_two[..]],
        "pass-less key first, declaration order within the node"
    );
    assert_eq!(
        all[0].1.describe().as_deref(),
        Some("X"),
        "the guard rides along"
    );
}

/// The best call by logit — the argmax the driver takes
fn best_call(logits: &crate::bidding::array::Logits) -> Call {
    logits
        .iter()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(call, _)| call)
        .expect("logits are non-empty")
}

/// A total floor covering every hand, ranking 3NT over 2NT over the pass
fn ranked_floor() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(0..))
        .rule(Bid::new(2, Strain::Notrump), 50, hcp(0..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// A tombstone at a floor-owned node masks the floor's own choice: the vetoed
/// call reads exactly `-∞` and the runner-up wins, while the floor still
/// answers — a veto is a call-level prohibition, not a node.
#[test]
fn tombstone_masks_the_floors_top_call() {
    let auction = [Call::Bid(Bid::new(1, Strain::Clubs))];
    let three_nt = Call::Bid(Bid::new(3, Strain::Notrump));
    let hand: Hand = "AKQ2.KQ5.AQJ4.92".parse().expect("valid test hand");

    let mut control = Trie::new();
    control.fallback_at(&[], Always, Fallback::classify(ranked_floor()));
    let context = Context::new(RelativeVulnerability::NONE, &auction);
    let (logits, _) = control
        .classify_floored(hand, &context, &auction)
        .expect("the floor answers");
    assert_eq!(best_call(&logits), three_nt, "control: the floor bids 3NT");

    let mut vetoed = Trie::new();
    vetoed.fallback_at(&[], Always, Fallback::classify(ranked_floor()));
    vetoed.tombstone(&auction, three_nt);
    let (logits, provenance) = vetoed
        .classify_floored(hand, &context, &auction)
        .expect("the floor still answers");
    assert_eq!(
        logits.0[three_nt],
        f32::NEG_INFINITY,
        "the vetoed call is masked, not merely demoted"
    );
    assert_eq!(
        best_call(&logits),
        Call::Bid(Bid::new(2, Strain::Notrump)),
        "the runner-up wins"
    );
    assert_eq!(
        provenance.depth, 0,
        "the floor still answers — a veto does not author a node"
    );
    assert!(
        !vetoed.vetoes(&[], three_nt),
        "vetoes key at the exact node, never the subtree"
    );
}

/// A node with a finite catch-all keeps shadowing the floor, and a tombstone
/// on a call it never authors is inert — the registration assert makes vetoed
/// and authored disjoint, so the masked slot was already `-∞`.
#[test]
fn tombstone_is_inert_beside_a_shadowing_table() {
    let auction = [Call::Bid(Bid::new(1, Strain::Clubs))];
    let table = || {
        Rules::new()
            .rule(Bid::new(2, Strain::Clubs), 100, hcp(6..))
            .rule(Call::Pass, 0, hcp(0..))
    };
    let hand: Hand = "AKQ2.KQ5.AQJ4.92".parse().expect("valid test hand");
    let context = Context::new(RelativeVulnerability::NONE, &auction);

    let mut control = Trie::new();
    control.insert(&auction, table());
    control.fallback_at(&[], Always, Fallback::classify(ranked_floor()));
    let (before, _) = control
        .classify_floored(hand, &context, &auction)
        .expect("the book answers");

    let mut vetoed = Trie::new();
    vetoed.insert(&auction, table());
    vetoed.fallback_at(&[], Always, Fallback::classify(ranked_floor()));
    vetoed.tombstone(&auction, Call::Redouble);
    let (after, _) = vetoed
        .classify_floored(hand, &context, &auction)
        .expect("the book still answers");

    assert_eq!(before, after, "every logit is byte-identical");
}

/// A veto-only node carries no classifier, so the two predicates disagree —
/// which is exactly how the fourth state is told from the third.
#[test]
fn a_veto_only_node_is_tombstoned_but_not_authored() {
    use crate::bidding::Bidder;

    let auction = [Call::Bid(Bid::new(1, Strain::Clubs))];
    let vul = RelativeVulnerability::NONE;
    let mut trie = Trie::new();
    trie.fallback_at(&[], Always, Fallback::classify(ranked_floor()));
    trie.tombstone(&auction, Call::Redouble);

    assert!(
        !trie.authored_at(vul, &auction),
        "a veto authors nothing — the floor still owns the node"
    );
    assert!(trie.tombstoned_at(vul, &auction, Call::Redouble));
    assert!(
        !trie.tombstoned_at(vul, &auction, Call::Double),
        "vetoes are per call"
    );
    assert!(
        !trie.tombstoned_at(vul, &[], Call::Redouble),
        "and per node"
    );
}

/// Merging fragments unions their vetoes: no fragment can resurrect a call
/// another one forbade.
#[test]
fn merge_unions_the_veto_masks() {
    let auction = [Call::Bid(Bid::new(1, Strain::Clubs))];
    let mut left = Trie::new();
    left.tombstone(&auction, Call::Redouble);
    let mut right = Trie::new();
    right.tombstone(&auction, Call::Double);

    assert!(left.merge(right).is_empty(), "no classifier collides");
    assert!(left.vetoes(&auction, Call::Redouble));
    assert!(left.vetoes(&auction, Call::Double));
}

#[test]
#[should_panic(expected = "the pass can never be tombstoned")]
fn tombstoning_the_pass_panics() {
    Trie::new().tombstone(&[Call::Bid(Bid::new(1, Strain::Clubs))], Call::Pass);
}

#[test]
#[should_panic(expected = "a call cannot be both agreed and vetoed")]
fn tombstoning_an_authored_call_panics() {
    let auction = [Call::Bid(Bid::new(1, Strain::Clubs))];
    let mut trie = Trie::new();
    trie.insert(&auction, Rules::new().rule(Call::Redouble, 100, hcp(0..)));
    trie.tombstone(&auction, Call::Redouble);
}
