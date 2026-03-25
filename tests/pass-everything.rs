use dds_bridge::contract::Call;
use dds_bridge::deal::Hand;
use pons::bidding::*;

const JUST_PASS: array::Logits = {
    let mut table = array::Logits::new();
    *table.0.get_mut(Call::Pass) = 0.0;
    table
};

#[test]
fn test_pass_everything() {
    let mut trie = Trie::new();
    trie.insert(&[Call::Pass], |_| JUST_PASS);

    let f = trie.get(&[Call::Pass]).expect("I just inserted this!");
    assert_eq!(f(Hand::default()), JUST_PASS);
}
