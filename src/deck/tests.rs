use super::*;
use dds_bridge::hand::ParseHandError;

fn roundtrip(deck: Deck) -> Result<(), ParseHandError> {
    assert_eq!(deck.to_string().parse::<Deck>()?, deck);
    Ok(())
}

#[test]
fn full_and_empty_roundtrip() -> Result<(), ParseHandError> {
    roundtrip(Deck::ALL)?;
    roundtrip(Deck::EMPTY)?;
    Ok(())
}

#[test]
fn partial_deck_roundtrip() -> anyhow::Result<()> {
    let mut deck = Deck::EMPTY;
    for s in ["♠A", "♥K", "♦Q", "♣J", "♠2"] {
        deck.insert(s.parse()?);
    }
    roundtrip(deck)?;
    Ok(())
}

#[test]
fn parses_from_hand_notation() -> Result<(), ParseHandError> {
    let deck: Deck = "AKQJ.T98.765.432".parse()?;
    assert_eq!(deck.len(), 13);
    assert_eq!(deck.to_string(), "AKQJ.T98.765.432");
    Ok(())
}
