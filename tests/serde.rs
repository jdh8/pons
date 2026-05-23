//! JSON round-trip tests for the `serde` feature.

#![cfg(feature = "serde")]

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
