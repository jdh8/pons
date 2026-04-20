//! JSON round-trip tests for the `serde` feature.

#![cfg(feature = "serde")]

use dds_bridge::{Bid, Strain};
use pons::bidding::{Auction, Call, IllegalCall, RelativeVulnerability};
use pons::deck::Deck;
use pons::stats::{Accumulator, HistogramRow, HistogramTable, Statistics};

fn roundtrip<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + core::fmt::Debug + PartialEq,
{
    let json = serde_json::to_string(value).unwrap();
    let parsed: T = serde_json::from_str(&json).unwrap();
    assert_eq!(&parsed, value, "round-trip mismatch for {json}");
}

const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

#[test]
fn call_json_is_string() {
    assert_eq!(serde_json::to_string(&Call::Pass).unwrap(), "\"P\"");
    assert_eq!(serde_json::to_string(&Call::Double).unwrap(), "\"X\"");
    assert_eq!(serde_json::to_string(&Call::Redouble).unwrap(), "\"XX\"");
    assert_eq!(
        serde_json::to_string(&bid(3, Strain::Notrump)).unwrap(),
        "\"3NT\"",
    );
    for call in [
        Call::Pass,
        Call::Double,
        Call::Redouble,
        bid(1, Strain::Spades),
        bid(7, Strain::Clubs),
    ] {
        roundtrip(&call);
    }
}

#[test]
fn auction_json_is_string() {
    let mut auction = Auction::new();
    for call in [
        Call::Pass,
        bid(1, Strain::Spades),
        bid(2, Strain::Hearts),
        Call::Double,
        Call::Pass,
        Call::Pass,
        Call::Pass,
    ] {
        auction.try_push(call).unwrap();
    }
    assert_eq!(
        serde_json::to_string(&auction).unwrap(),
        "\"P 1♠ 2♥ X P P P\"",
    );
    roundtrip(&auction);
    roundtrip(&Auction::new());
}

#[test]
fn relative_vulnerability_json_roundtrip() {
    for v in [
        RelativeVulnerability::NONE,
        RelativeVulnerability::WE,
        RelativeVulnerability::THEY,
        RelativeVulnerability::ALL,
    ] {
        roundtrip(&v);
    }
}

#[test]
fn deck_json_is_string() {
    roundtrip(&Deck::ALL);
    roundtrip(&Deck::EMPTY);
    let deck: Deck = "AKQJ.T98.765.432".parse().unwrap();
    assert_eq!(
        serde_json::to_string(&deck).unwrap(),
        "\"AKQJ.T98.765.432\"",
    );
    roundtrip(&deck);
}

#[test]
fn statistics_and_accumulator_roundtrip() {
    roundtrip(&Statistics::new(1.5, 0.75));
    let mut acc = Accumulator::new();
    for x in [1.0, 2.0, 3.0, 4.0] {
        acc.push(x);
    }
    roundtrip(&acc);
}

#[test]
fn histogram_roundtrip() {
    roundtrip(&HistogramRow::new());
    roundtrip(&HistogramTable::new());
}

#[test]
fn illegal_call_roundtrip() {
    roundtrip(&IllegalCall::AfterFinalPass);
}
