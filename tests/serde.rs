//! JSON round-trip tests for the `serde` feature.

#![cfg(feature = "serde")]

use dds_bridge::{Bid, Strain};
use pons::bidding::{Auction, Call, IllegalCall, RelativeVulnerability};
use pons::deck::Deck;
use pons::stats::{Accumulator, HistogramRow, HistogramTable, Statistics};

fn roundtrip<T>(value: &T) -> Result<(), serde_json::Error>
where
    T: serde::Serialize + serde::de::DeserializeOwned + core::fmt::Debug + PartialEq,
{
    let json = serde_json::to_string(value)?;
    let parsed: T = serde_json::from_str(&json)?;
    assert_eq!(&parsed, value, "round-trip mismatch for {json}");
    Ok(())
}

const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

#[test]
fn call_json_is_string() -> Result<(), serde_json::Error> {
    assert_eq!(serde_json::to_string(&Call::Pass)?, "\"P\"");
    assert_eq!(serde_json::to_string(&Call::Double)?, "\"X\"");
    assert_eq!(serde_json::to_string(&Call::Redouble)?, "\"XX\"");
    assert_eq!(serde_json::to_string(&bid(3, Strain::Notrump))?, "\"3NT\"",);
    for call in [
        Call::Pass,
        Call::Double,
        Call::Redouble,
        bid(1, Strain::Spades),
        bid(7, Strain::Clubs),
    ] {
        roundtrip(&call)?;
    }
    Ok(())
}

#[test]
fn auction_json_is_string() -> anyhow::Result<()> {
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
        auction.try_push(call)?;
    }
    assert_eq!(serde_json::to_string(&auction)?, "\"P 1♠ 2♥ X P P P\"");
    roundtrip(&auction)?;
    roundtrip(&Auction::new())?;
    Ok(())
}

#[test]
fn relative_vulnerability_json_roundtrip() -> Result<(), serde_json::Error> {
    for v in [
        RelativeVulnerability::NONE,
        RelativeVulnerability::WE,
        RelativeVulnerability::THEY,
        RelativeVulnerability::ALL,
    ] {
        roundtrip(&v)?;
    }
    Ok(())
}

#[test]
fn deck_json_is_string() -> anyhow::Result<()> {
    roundtrip(&Deck::ALL)?;
    roundtrip(&Deck::EMPTY)?;
    let deck: Deck = "AKQJ.T98.765.432".parse()?;
    assert_eq!(serde_json::to_string(&deck)?, "\"AKQJ.T98.765.432\"");
    roundtrip(&deck)?;
    Ok(())
}

#[test]
fn statistics_and_accumulator_roundtrip() -> Result<(), serde_json::Error> {
    roundtrip(&Statistics::new(1.5, 0.75))?;
    let mut acc = Accumulator::new();
    for x in [1.0, 2.0, 3.0, 4.0] {
        acc.push(x);
    }
    roundtrip(&acc)?;
    Ok(())
}

#[test]
fn histogram_roundtrip() -> Result<(), serde_json::Error> {
    roundtrip(&HistogramRow::new())?;
    roundtrip(&HistogramTable::new())?;
    Ok(())
}

#[test]
fn illegal_call_roundtrip() -> Result<(), serde_json::Error> {
    roundtrip(&IllegalCall::AfterFinalPass)?;
    Ok(())
}
