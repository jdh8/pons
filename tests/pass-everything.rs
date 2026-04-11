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
    trie.insert(&[], |_, _, _| JUST_PASS);

    assert_eq!(
        trie.classify(Hand::default(), RelativeVulnerability::NONE, Auction::new()),
        Some(JUST_PASS)
    );
}
