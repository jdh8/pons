use super::*;
use contract_bridge::Level;
use contract_bridge::auction::RelativeVulnerability;

const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid {
        level: Level::new(level),
        strain,
    })
}

fn ctx(auction: &[Call]) -> Context<'_> {
    Context::new(RelativeVulnerability::NONE, auction)
}

#[test]
fn derive_reads_doubles_by_book() {
    // A direct double of their 1♥ opening, in the defensive book: takeout.
    let auction = [bid(1, Strain::Hearts)];
    let c = ctx(&auction);
    assert_eq!(derive("defensive", Call::Double, &c).0, vec!["T/O"]);

    // We opened, they overcalled: responder's double is negative.
    let contested = [bid(1, Strain::Spades), bid(2, Strain::Clubs)];
    let c = ctx(&contested);
    assert_eq!(derive("competitive", Call::Double, &c).0, vec!["NEG"]);
}

#[test]
fn derives_american_game_force() {
    // 1♥ -, then 2♣ is a game-forcing two-over-one.
    let auction = [bid(1, Strain::Hearts), Call::Pass];
    let c = ctx(&auction);
    assert_eq!(
        derive("constructive", bid(2, Strain::Clubs), &c).0,
        vec!["FG", "NAT"]
    );
}
