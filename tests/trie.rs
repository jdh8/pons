use dds_bridge::{Bid, Hand, Level, Strain};
use pons::bidding::array::Logits;
use pons::bidding::trie::Forest;
use pons::bidding::{Call, RelativeVulnerability, System, Trie};

const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid {
        level: Level::new(level),
        strain,
    })
}

fn just_pass() -> Logits {
    let mut logits = Logits::new();
    *logits.0.get_mut(Call::Pass) = 0.0;
    logits
}

fn marker_logits(value: f32) -> Logits {
    let mut logits = Logits::new();
    *logits.0.get_mut(Call::Pass) = value;
    logits
}

fn classify_at(trie: &Trie, auction: &[Call]) -> Logits {
    let f = trie
        .get(auction)
        .expect("classifier missing at exact auction");
    f.classify(
        Hand::default(),
        RelativeVulnerability::NONE,
        trie.common_prefixes(auction),
    )
}

#[test]
fn test_trie_new_is_empty() {
    let trie = Trie::new();
    assert!(trie.is_prefix(&[]));
    assert!(trie.get(&[]).is_none());
    assert!(trie.iter().next().is_none());
    assert!(trie.longest_prefix(&[]).is_none());
}

#[test]
fn test_trie_default_constructs() {
    let trie = Trie::default();
    assert!(trie.iter().next().is_none());
}

#[test]
fn test_trie_insert_and_get_round_trip() {
    let mut trie = Trie::new();
    let auction = [bid(1, Strain::Clubs)];
    trie.insert(&auction, |_, _| marker_logits(1.0));

    assert_eq!(classify_at(&trie, &auction), marker_logits(1.0));
}

#[test]
fn test_trie_insert_returns_previous_when_overwriting() {
    let mut trie = Trie::new();
    let auction = [bid(1, Strain::Clubs)];
    assert!(trie.insert(&auction, |_, _| marker_logits(1.0)).is_none());
    assert!(trie.insert(&auction, |_, _| marker_logits(2.0)).is_some());
    assert_eq!(classify_at(&trie, &auction), marker_logits(2.0));
}

#[test]
fn test_trie_is_prefix() {
    let mut trie = Trie::new();
    let auction = [bid(1, Strain::Clubs), Call::Pass, bid(1, Strain::Hearts)];
    trie.insert(&auction, |_, _| just_pass());

    assert!(trie.is_prefix(&[]));
    assert!(trie.is_prefix(&auction[..1]));
    assert!(trie.is_prefix(&auction[..2]));
    assert!(trie.is_prefix(&auction));

    assert!(!trie.is_prefix(&[bid(1, Strain::Spades)]));
    assert!(!trie.is_prefix(&[bid(1, Strain::Clubs), bid(2, Strain::Clubs)]));
}

#[test]
fn test_trie_longest_prefix_picks_deepest_match() {
    let mut trie = Trie::new();
    let one_c = bid(1, Strain::Clubs);
    let one_h = bid(1, Strain::Hearts);
    let one_d = bid(1, Strain::Diamonds);

    trie.insert(&[], |_, _| marker_logits(0.0));
    trie.insert(&[one_c], |_, _| marker_logits(1.0));
    trie.insert(&[one_c, Call::Pass, one_h], |_, _| marker_logits(2.0));

    // Diverges after [1C, P]: deepest match is [1C].
    let query = [one_c, Call::Pass, one_d];
    let (prefix, _) = trie.longest_prefix(&query).expect("expected a match");
    assert_eq!(prefix, &[one_c]);

    // Exact match returns the deepest classifier.
    let exact = [one_c, Call::Pass, one_h];
    let (prefix, _) = trie.longest_prefix(&exact).expect("expected a match");
    assert_eq!(prefix, &exact[..]);

    // Empty query falls back to the root classifier.
    let (prefix, _) = trie.longest_prefix(&[]).expect("expected root match");
    assert!(prefix.is_empty());
}

#[test]
fn test_trie_longest_prefix_no_match_returns_none() {
    let trie = Trie::new();
    assert!(trie.longest_prefix(&[bid(1, Strain::Clubs)]).is_none());
}

#[test]
fn test_trie_longest_prefix_skips_empty_intermediate() {
    // Root has no classifier; only [1C, P, 1H] does. Querying that exact
    // auction must still return the deepest match.
    let mut trie = Trie::new();
    let one_c = bid(1, Strain::Clubs);
    let one_h = bid(1, Strain::Hearts);
    let auction = [one_c, Call::Pass, one_h];
    trie.insert(&auction, |_, _| just_pass());

    let (prefix, _) = trie
        .longest_prefix(&auction)
        .expect("expected deepest match");
    assert_eq!(prefix, &auction[..]);

    // A sibling query yields no match because no ancestor has a classifier.
    assert!(trie.longest_prefix(&[bid(1, Strain::Spades)]).is_none());
}

#[test]
fn test_trie_suffixes_enumerate_with_correct_paths() {
    let mut trie = Trie::new();
    let one_c = bid(1, Strain::Clubs);
    let one_h = bid(1, Strain::Hearts);

    trie.insert(&[one_c], |_, _| marker_logits(1.0));
    trie.insert(&[one_c, Call::Pass, one_h], |_, _| marker_logits(2.0));
    trie.insert(&[bid(1, Strain::Spades)], |_, _| marker_logits(3.0));

    let suffixes: Vec<Box<[Call]>> = trie.suffixes(&[one_c]).map(|(s, _)| s).collect();
    assert_eq!(suffixes.len(), 2);
    assert!(suffixes.iter().any(|s| s.is_empty()));
    assert!(suffixes.iter().any(|s| **s == [Call::Pass, one_h]));
}

#[test]
fn test_trie_suffixes_empty_when_prefix_absent() {
    let trie = Trie::new();
    assert_eq!(trie.suffixes(&[bid(1, Strain::Clubs)]).count(), 0);
}

#[test]
fn test_trie_suffixes_isolated_to_subtree() {
    let mut trie = Trie::new();
    trie.insert(&[bid(1, Strain::Clubs)], |_, _| just_pass());
    trie.insert(&[bid(1, Strain::Hearts)], |_, _| just_pass());

    assert_eq!(trie.suffixes(&[bid(1, Strain::Clubs)]).count(), 1);
}

#[test]
fn test_trie_suffixes_is_fused() {
    let trie = Trie::new();
    let mut iter = trie.suffixes(&[]);
    assert!(iter.next().is_none());
    assert!(iter.next().is_none());
    assert!(iter.next().is_none());
}

#[test]
fn test_trie_iter_visits_every_classifier() {
    let mut trie = Trie::new();
    trie.insert(&[bid(1, Strain::Clubs)], |_, _| just_pass());
    trie.insert(&[bid(1, Strain::Hearts)], |_, _| just_pass());
    trie.insert(&[bid(1, Strain::Hearts), Call::Pass], |_, _| just_pass());

    assert_eq!(trie.iter().count(), 3);
}

#[test]
fn test_trie_common_prefixes_returns_ancestors_with_classifiers() {
    let mut trie = Trie::new();
    let one_c = bid(1, Strain::Clubs);
    let one_h = bid(1, Strain::Hearts);

    trie.insert(&[], |_, _| marker_logits(0.0));
    trie.insert(&[one_c], |_, _| marker_logits(1.0));
    trie.insert(&[one_c, Call::Pass, one_h], |_, _| marker_logits(2.0));

    let query = [one_c, Call::Pass, one_h];
    let prefixes: Vec<Vec<Call>> = trie
        .common_prefixes(&query)
        .map(|(p, _)| p.to_vec())
        .collect();

    assert_eq!(prefixes.len(), 3);
    assert_eq!(prefixes[0], Vec::<Call>::new());
    assert_eq!(prefixes[1], vec![one_c]);
    assert_eq!(prefixes[2], vec![one_c, Call::Pass, one_h]);
}

#[test]
fn test_trie_common_prefixes_skips_uncovered_intermediate() {
    let mut trie = Trie::new();
    let one_c = bid(1, Strain::Clubs);
    let one_h = bid(1, Strain::Hearts);
    // Only the deepest node has a classifier; intermediates do not.
    trie.insert(&[one_c, Call::Pass, one_h], |_, _| just_pass());

    let query = [one_c, Call::Pass, one_h];
    let prefixes: Vec<Vec<Call>> = trie
        .common_prefixes(&query)
        .map(|(p, _)| p.to_vec())
        .collect();
    assert_eq!(prefixes, vec![vec![one_c, Call::Pass, one_h]]);
}

#[test]
fn test_trie_common_prefixes_empty_when_diverges() {
    let mut trie = Trie::new();
    trie.insert(&[bid(1, Strain::Clubs)], |_, _| just_pass());

    let query = [bid(1, Strain::Spades)];
    assert_eq!(trie.common_prefixes(&query).count(), 0);
}

#[test]
fn test_forest_indexes_each_vulnerability() {
    let forest = Forest::new();
    let _: &Trie = &forest[RelativeVulnerability::NONE];
    let _: &Trie = &forest[RelativeVulnerability::WE];
    let _: &Trie = &forest[RelativeVulnerability::THEY];
    let _: &Trie = &forest[RelativeVulnerability::ALL];
}

#[test]
fn test_forest_default_constructs() {
    let _: Forest = Forest::default();
}

#[test]
fn test_forest_from_fn_called_once_per_vulnerability() {
    use std::cell::RefCell;
    let calls = RefCell::new(Vec::<RelativeVulnerability>::new());
    let _forest = Forest::from_fn(|vul| {
        calls.borrow_mut().push(vul);
        Trie::new()
    });
    let calls = calls.into_inner();
    assert_eq!(calls.len(), 4);
    assert!(calls.contains(&RelativeVulnerability::NONE));
    assert!(calls.contains(&RelativeVulnerability::WE));
    assert!(calls.contains(&RelativeVulnerability::THEY));
    assert!(calls.contains(&RelativeVulnerability::ALL));
}

#[test]
fn test_forest_index_mut_modifies_targeted_trie() {
    let mut forest = Forest::new();
    forest[RelativeVulnerability::WE].insert(&[], |_, _| just_pass());

    assert!(forest[RelativeVulnerability::WE].get(&[]).is_some());
    assert!(forest[RelativeVulnerability::NONE].get(&[]).is_none());
    assert!(forest[RelativeVulnerability::THEY].get(&[]).is_none());
    assert!(forest[RelativeVulnerability::ALL].get(&[]).is_none());
}

#[test]
fn test_pass_everything_classifies_root() {
    let mut trie = Trie::new();
    trie.insert(&[], |_, _| just_pass());

    let result = trie.classify(Hand::default(), RelativeVulnerability::NONE, &[]);
    assert_eq!(result, Some(just_pass()));
}

#[test]
fn test_system_trie_returns_none_for_unknown_auction() {
    let trie = Trie::new();
    let result = trie.classify(Hand::default(), RelativeVulnerability::NONE, &[]);
    assert!(result.is_none());
}

#[test]
fn test_system_forest_dispatches_by_vulnerability() {
    let mut forest = Forest::new();
    forest[RelativeVulnerability::NONE].insert(&[], |_, _| marker_logits(1.0));
    forest[RelativeVulnerability::ALL].insert(&[], |_, _| marker_logits(2.0));

    assert_eq!(
        forest.classify(Hand::default(), RelativeVulnerability::NONE, &[]),
        Some(marker_logits(1.0))
    );
    assert_eq!(
        forest.classify(Hand::default(), RelativeVulnerability::ALL, &[]),
        Some(marker_logits(2.0))
    );
    assert!(
        forest
            .classify(Hand::default(), RelativeVulnerability::WE, &[])
            .is_none()
    );
}
