use super::*;

fn auction(calls: &str) -> Auction {
    calls.parse().expect("legal auction")
}

#[test]
fn shared_penalty_honors_a_redouble_both_arms_saw() {
    let sit = auction("1NT X XX P P P");
    let run = auction("1NT X XX P P 2C P P P");
    assert_eq!(shared_penalty(&sit, &run), Penalty::Redoubled);
    // The runout is a new contract: nothing shared stands on it
    assert_eq!(shared_penalty(&run, &sit), Penalty::Undoubled);
}

#[test]
fn shared_penalty_ignores_doubles_after_the_split() {
    let doubled = auction("1NT P P X P P P");
    let passed = auction("1NT P P P");
    assert_eq!(shared_penalty(&doubled, &passed), Penalty::Undoubled);
    assert_eq!(shared_penalty(&passed, &doubled), Penalty::Undoubled);
}

#[test]
fn shared_penalty_of_identical_auctions_is_the_table_penalty() {
    let a = auction("1NT X P P P");
    assert_eq!(shared_penalty(&a, &a), Penalty::Doubled);
}
