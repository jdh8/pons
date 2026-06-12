//! The table driver: seat rotation, vulnerability conversion, legality
//! filtering, and bidding out a deal

use contract_bridge::auction::{Auction, Call, RelativeVulnerability};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Bid, Hand, Level, Seat, Strain};
use pons::bidding::array::Logits;
use pons::bidding::trie::classifier;
use pons::bidding::{Competitive, Constructive, Defensive, Family, Pair, System, Table};

const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid {
        level: Level::new(level),
        strain,
    })
}

const ONE_CLUB: Call = bid(1, Strain::Clubs);
const ONE_SPADE: Call = bid(1, Strain::Spades);
const TWO_CLUBS: Call = bid(2, Strain::Clubs);

fn single(call: Call, logit: f32) -> Logits {
    let mut logits = Logits::new();
    *logits.0.get_mut(call) = logit;
    logits
}

/// Hand-blind system bidding from a fixed preference list
struct Prefers(&'static [(Call, f32)]);

impl System for Prefers {
    fn classify(&self, _: Hand, _: RelativeVulnerability, _: &[Call]) -> Option<Logits> {
        let mut logits = Logits::new();
        for &(call, logit) in self.0 {
            *logits.0.get_mut(call) = logit;
        }
        Some(logits)
    }
}

/// System with no answer at all
struct Silent;

impl System for Silent {
    fn classify(&self, _: Hand, _: RelativeVulnerability, _: &[Call]) -> Option<Logits> {
        None
    }
}

/// System echoing the vulnerability it receives
struct VulProbe;

impl System for VulProbe {
    fn classify(&self, _: Hand, vul: RelativeVulnerability, _: &[Call]) -> Option<Logits> {
        Some(single(Call::Pass, f32::from(vul.bits())))
    }
}

#[test]
fn test_seat_rotation() {
    let table = Table::new(Silent, Silent, Seat::South, AbsoluteVulnerability::NONE);

    assert_eq!(table.seat_to_act(0), Seat::South);
    assert_eq!(table.seat_to_act(1), Seat::West);
    assert_eq!(table.seat_to_act(2), Seat::North);
    assert_eq!(table.seat_to_act(3), Seat::East);
    assert_eq!(table.seat_to_act(4), Seat::South);

    for dealer in Seat::ALL {
        let table = Table::new(Silent, Silent, dealer, AbsoluteVulnerability::NONE);
        assert_eq!(table.seat_to_act(0), dealer);
    }
}

#[test]
fn test_vulnerability_is_seat_relative() {
    let table = Table::new(VulProbe, VulProbe, Seat::North, AbsoluteVulnerability::NS);
    let passes = [Call::Pass; 3];

    // With North/South vulnerable, they see "we" and East/West see "they".
    let expected = [
        RelativeVulnerability::WE,   // North
        RelativeVulnerability::THEY, // East
        RelativeVulnerability::WE,   // South
        RelativeVulnerability::THEY, // West
    ];

    for (len, &vul) in expected.iter().enumerate() {
        let logits = table
            .classify(Hand::default(), &passes[..len])
            .expect("probe always answers");
        let marker = *logits.0.get(Call::Pass);
        assert!((marker - f32::from(vul.bits())).abs() <= f32::EPSILON);
    }
}

#[test]
fn test_next_call_picks_highest_legal() {
    const EAGER: &[(Call, f32)] = &[(ONE_CLUB, 2.0), (Call::Double, 1.0), (TWO_CLUBS, 0.5)];
    let eager = Prefers(EAGER);
    let table = Table::new(&eager, &eager, Seat::North, AbsoluteVulnerability::NONE);

    // On an empty auction the highest-logit call is legal.
    assert_eq!(table.next_call(Hand::default(), &Auction::new()), ONE_CLUB);

    // Over their 1♠ the 1♣ is insufficient; the double is the best legal call.
    let mut auction = Auction::new();
    auction.push(ONE_SPADE);
    assert_eq!(table.next_call(Hand::default(), &auction), Call::Double);
}

#[test]
fn test_next_call_defaults_to_pass() {
    // A double of nothing is inadmissible, and no other call is wanted.
    const DOUBLER: &[(Call, f32)] = &[(Call::Double, 1.0)];
    let table = Table::new(
        Prefers(DOUBLER),
        Silent,
        Seat::North,
        AbsoluteVulnerability::NONE,
    );
    assert_eq!(
        table.next_call(Hand::default(), &Auction::new()),
        Call::Pass
    );

    // An uncovered auction also resolves to a pass.
    let table = Table::new(Silent, Silent, Seat::North, AbsoluteVulnerability::NONE);
    assert_eq!(
        table.next_call(Hand::default(), &Auction::new()),
        Call::Pass
    );
}

#[test]
fn test_bid_out_all_pass_board() {
    let table = Table::new(Silent, Silent, Seat::West, AbsoluteVulnerability::NONE);
    let deal = full_deal(&mut rand::rng());
    assert_eq!(&table.bid_out(&deal)[..], &[Call::Pass; 4]);
}

#[test]
fn test_bid_out_terminates_after_a_contract() {
    const OPENER: &[(Call, f32)] = &[(ONE_CLUB, 1.0)];
    let opener = Prefers(OPENER);
    let table = Table::new(&opener, Silent, Seat::North, AbsoluteVulnerability::NONE);
    let deal = full_deal(&mut rand::rng());

    // North opens 1♣; for everyone after, 1♣ is illegal or unwanted.
    assert_eq!(
        &table.bid_out(&deal)[..],
        &[ONE_CLUB, Call::Pass, Call::Pass, Call::Pass]
    );
}

#[test]
fn test_of_pairs_binds_and_plays() {
    let mut constructive = Constructive::new();
    constructive.insert(&[], classifier(|_, _| single(ONE_CLUB, 1.0)));

    let ns = Pair::new(
        Family::NATURAL,
        constructive,
        Competitive::new(),
        Defensive::new(),
    );
    let ew = Pair::default();

    let table = Table::of_pairs(&ns, &ew, Seat::North, AbsoluteVulnerability::NONE);
    let deal = full_deal(&mut rand::rng());

    assert_eq!(
        &table.bid_out(&deal)[..],
        &[ONE_CLUB, Call::Pass, Call::Pass, Call::Pass]
    );
}
