use dds_bridge::contract::Call;
use dds_bridge::deal::Hand;
use pons::bidding::*;

#[test]
fn test_pass_everything() {
    fn just_pass() -> Array<Logit> {
        let mut table = Array::default();
        table[Call::Pass] = Logit(0.0);
        table
    }

    let mut trie = Trie::new();
    trie.insert(&[Call::Pass], |_| just_pass());

    let f = trie.get(&[Call::Pass]).expect("I just inserted this!");
    assert_eq!(f(Hand::default()), just_pass());
}
