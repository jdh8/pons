use super::super::tests::{best_call_vul, call};
use contract_bridge::Strain;
use contract_bridge::auction::{Call, RelativeVulnerability};

#[test]
fn vulnerable_weak_two_overcall_demands_more() {
    // The shipped `set_weak_two_overcall_discipline` branch is invisible to
    // every other test in this file, because they all classify at
    // `RelativeVulnerability::NONE` — where the rule reduces to the flat
    // band it replaced.  This is the only check that reaches it.
    //
    // An 11-count with five diamonds: over their (2♠) that is a *three*
    // level overcall, which vulnerable wants 15 for, so it passes.
    let minimum = "J32.J32.KQT43.KJ";
    let over_2s = [call(2, Strain::Spades)];

    assert_eq!(
        best_call_vul(&over_2s, minimum, RelativeVulnerability::NONE),
        call(3, Strain::Diamonds),
        "non-vulnerable keeps the flat 10-16 band, so an 11-count overcalls"
    );
    assert_eq!(
        best_call_vul(&over_2s, minimum, RelativeVulnerability::WE),
        Call::Pass,
        "vulnerable at the three level needs 15, so the same hand passes"
    );

    // Their vulnerability is not ours: the discipline keys on `vulnerable()`
    // alone, which is what the `-v ns` cell of the A/B established.
    assert_eq!(
        best_call_vul(&over_2s, minimum, RelativeVulnerability::THEY),
        call(3, Strain::Diamonds),
        "only OUR vulnerability tightens the band"
    );
}
