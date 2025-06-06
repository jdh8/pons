use pons::bidding::*;

#[test]
fn test_pass_everything() {
    let mut trie = Trie::new();
    trie.insert(&[], |_, _, _| Call::Pass);

    let strategy = trie.get(&[]).expect("I just inserted this!");
    assert_eq!(
        strategy(Hand::default(), &[], Vulnerability::empty()),
        Call::Pass
    );
}
