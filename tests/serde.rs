//! JSON round-trip tests for the `serde` feature.

#![cfg(feature = "serde")]

use contract_bridge::auction::RelativeVulnerability;
use pons::bidding::{Context, Envelope, EnvelopeUnion, Inferences};
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

#[test]
fn envelope_union_and_inferences_use_canonical_keys() -> Result<(), serde_json::Error> {
    let one = EnvelopeUnion::from(Envelope::unknown());
    roundtrip(&one)?;
    assert_eq!(
        serde_json::to_vec(&one)?,
        serde_json::to_vec(&vec![Envelope::unknown()])?,
        "inline singleton keeps the old newtype-sequence wire shape"
    );

    let mut second = Envelope::unknown();
    second.lengths[0].min = 5;
    let many = one.union(EnvelopeUnion::from(second));
    roundtrip(&many)?;
    assert_eq!(
        serde_json::to_vec(&many)?,
        serde_json::to_vec(&vec![Envelope::unknown(), second])?,
        "heap-backed unions keep the old sequence wire shape"
    );

    let empty: EnvelopeUnion = serde_json::from_str("[]")?;
    assert!(
        empty.boxes().is_empty(),
        "serde acceptance must not tighten"
    );

    let context = Context::new(RelativeVulnerability::NONE, &[]);
    let inferences = Inferences::read(&context);
    let value = serde_json::to_value(&inferences)?;
    let object = value
        .as_object()
        .expect("Inferences serializes as an object");
    assert!(object.contains_key("unions"));
    assert!(object.contains_key("announced_unions"));
    assert!(!object.contains_key("dnf"));
    assert!(!object.contains_key("announced"));

    let parsed: Inferences = serde_json::from_value(value.clone())?;
    assert_eq!(serde_json::to_value(parsed)?, value);

    let mut legacy_unions = value.clone();
    let legacy_object = legacy_unions
        .as_object_mut()
        .expect("Inferences serializes as an object");
    let unions = legacy_object
        .remove("unions")
        .expect("canonical unions field");
    legacy_object.insert("dnf".to_owned(), unions);
    assert!(serde_json::from_value::<Inferences>(legacy_unions).is_err());

    let mut legacy_announced = value;
    let legacy_object = legacy_announced
        .as_object_mut()
        .expect("Inferences serializes as an object");
    let announced_unions = legacy_object
        .remove("announced_unions")
        .expect("canonical announced-unions field");
    legacy_object.insert("announced".to_owned(), announced_unions);
    assert!(serde_json::from_value::<Inferences>(legacy_announced).is_err());
    Ok(())
}
