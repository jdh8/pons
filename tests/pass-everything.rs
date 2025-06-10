use pons::bidding::*;
use std::sync::Arc;

#[test]
fn test_pass_everything() {
    let mut trie = Trie::new();
    trie.insert(&[Call::Pass], Arc::new(|_| Fitness));

    let filter = trie.get(&[Call::Pass]).expect("I just inserted this!");
    assert_eq!(filter(Hand::default()), Fitness);
}
