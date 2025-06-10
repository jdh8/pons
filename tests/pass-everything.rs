use dds_bridge::contract::Call;
use dds_bridge::deal::Hand;
use pons::bidding::*;

#[test]
fn test_pass_everything() {
    let mut trie = Trie::new();
    trie.insert(&[Call::Pass], Filter::new(|_| Frequency(u8::MAX)));

    let filter = trie.get(&[Call::Pass]).expect("I just inserted this!");
    assert_eq!(filter(Hand::default()), Frequency(u8::MAX));
}
