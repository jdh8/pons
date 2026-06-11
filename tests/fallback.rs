use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Level, Strain};
use pons::bidding::array::Logits;
use pons::bidding::fallback::{Always, Fallback, FirstIs, OvercallAtMost, ReplaceNext, rewriter};
use pons::bidding::trie::{Provenance, REBASE_LIMIT, classifier};
use pons::bidding::{Context, System, Trie};

const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid {
        level: Level::new(level),
        strain,
    })
}

const fn marker_logits(value: f32) -> Logits {
    let mut logits = Logits::new();
    *logits.0.get_mut(Call::Pass) = value;
    logits
}

fn marker(value: f32) -> Fallback {
    Fallback::classify(classifier(move |_, _| marker_logits(value)))
}

fn resolve_marker(trie: &Trie, auction: &[Call]) -> Option<(f32, Provenance)> {
    let context = Context::new(RelativeVulnerability::NONE, auction);
    let (classifier, provenance) = trie.resolve(&context, auction)?;
    let logits = classifier.classify(Hand::default(), &context);
    Some((*logits.0.get(Call::Pass), provenance))
}

#[test]
fn test_exact_beats_fallback() {
    let mut trie = Trie::new();
    let auction = [bid(1, Strain::Clubs)];
    trie.insert(&auction, classifier(|_, _| marker_logits(1.0)));
    trie.fallback_at(&[], Always, marker(9.0));

    let (value, provenance) = resolve_marker(&trie, &auction).expect("expected exact match");
    assert_eq!(value, 1.0);
    assert_eq!(
        provenance,
        Provenance {
            depth: 1,
            fallback: None,
            rebases: 0,
        },
    );
}

#[test]
fn test_deeper_fallback_beats_shallower() {
    let mut trie = Trie::new();
    let one_h = bid(1, Strain::Hearts);
    trie.fallback_at(&[], Always, marker(1.0));
    trie.fallback_at(&[one_h], Always, marker(2.0));

    let query = [one_h, bid(1, Strain::Spades)];
    let (value, provenance) = resolve_marker(&trie, &query).expect("expected fallback");
    assert_eq!(value, 2.0);
    assert_eq!(
        provenance,
        Provenance {
            depth: 1,
            fallback: Some(0),
            rebases: 0,
        },
    );

    // A query diverging at the root only reaches the root fallback.
    let (value, _) = resolve_marker(&trie, &[bid(2, Strain::Clubs)]).expect("expected fallback");
    assert_eq!(value, 1.0);
}

#[test]
fn test_declaration_order_within_node() {
    let mut trie = Trie::new();
    let one_h = bid(1, Strain::Hearts);
    trie.fallback_at(
        &[one_h],
        OvercallAtMost(Bid::new(2, Strain::Spades)),
        marker(1.0),
    );
    trie.fallback_at(&[one_h], Always, marker(2.0));

    // Both guards admit a 1♠ overcall: the first declared entry wins.
    let (value, provenance) =
        resolve_marker(&trie, &[one_h, bid(1, Strain::Spades)]).expect("expected fallback");
    assert_eq!(value, 1.0);
    assert_eq!(provenance.fallback, Some(0));

    // Only the catch-all admits a 3♣ overcall.
    let (value, provenance) =
        resolve_marker(&trie, &[one_h, bid(3, Strain::Clubs)]).expect("expected fallback");
    assert_eq!(value, 2.0);
    assert_eq!(provenance.fallback, Some(1));
}

#[test]
fn test_rebase_system_on_over_double() {
    let mut trie = Trie::new();
    let one_nt = bid(1, Strain::Notrump);
    let two_h = bid(2, Strain::Hearts);

    // The undisturbed book: responses to 1NT after a pass.
    trie.insert(
        &[one_nt, Call::Pass, two_h],
        classifier(|_, _| marker_logits(1.0)),
    );
    // System on over their double: treat the double as a pass.
    trie.fallback_at(
        &[one_nt],
        FirstIs(Call::Double),
        Fallback::rebase(ReplaceNext(Call::Pass)),
    );

    // [1NT, X, 2♥] resolves through [1NT, P, 2♥].
    let (value, provenance) =
        resolve_marker(&trie, &[one_nt, Call::Double, two_h]).expect("expected rebase");
    assert_eq!(value, 1.0);
    assert_eq!(
        provenance,
        Provenance {
            depth: 3,
            fallback: None,
            rebases: 1,
        },
    );

    // A continuation the rewritten book does not cover still fails.
    assert!(resolve_marker(&trie, &[one_nt, Call::Double, bid(2, Strain::Spades)]).is_none());
}

#[test]
fn test_rebase_cycle_hits_limit() {
    let mut trie = Trie::new();
    // A rewrite that never makes progress.
    trie.fallback_at(
        &[],
        Always,
        Fallback::rebase(rewriter(|auction: &[Call], _| Some(auction.to_vec()))),
    );

    assert!(resolve_marker(&trie, &[bid(1, Strain::Clubs)]).is_none());
}

#[test]
fn test_rebase_limit_is_reported() {
    let mut trie = Trie::new();
    let one_c = bid(1, Strain::Clubs);
    // Each resolution of [1♣] rebases once onto [P], which the book covers.
    trie.insert(&[Call::Pass], classifier(|_, _| marker_logits(1.0)));
    trie.fallback_at(&[], Always, Fallback::rebase(ReplaceNext(Call::Pass)));

    let (_, provenance) = resolve_marker(&trie, &[one_c]).expect("expected rebase");
    assert_eq!(provenance.rebases, 1);
    assert!(provenance.rebases <= REBASE_LIMIT);
}

#[test]
fn test_root_default_always_answers() {
    let mut trie = Trie::new();
    trie.fallback_at(&[], Always, marker(7.0));

    for auction in [
        &[][..],
        &[bid(1, Strain::Clubs)][..],
        &[bid(3, Strain::Notrump), Call::Double, Call::Redouble][..],
    ] {
        let (value, _) = resolve_marker(&trie, auction).expect("root default answers");
        assert_eq!(value, 7.0);
    }
}

#[test]
fn test_system_classify_uses_fallbacks() {
    let mut trie = Trie::new();
    trie.fallback_at(&[], Always, marker(3.0));

    let logits = trie.classify(
        Hand::default(),
        RelativeVulnerability::NONE,
        &[bid(2, Strain::Diamonds)],
    );
    assert_eq!(logits, Some(marker_logits(3.0)));
}
