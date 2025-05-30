use pons::bidding::*;

#[test]
fn test_pass_everything() {
    let mut trie = Trie::new();
    trie.insert(&[], Strategy::new(|_| Call::Pass));

    let strategy = trie.get(&[]).expect("I just inserted this!");
    assert_eq!(strategy(Hand::default()), Call::Pass);
}
