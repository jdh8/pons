use dds_bridge::hand::ParseHandError;
use dds_bridge::{Bid, Hand, Level, Strain};
use pons::bidding::{Auction, Call, ParseAuctionError, ParseCallError};
use pons::deck::Deck;
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

fn strain() -> impl Strategy<Value = Strain> {
    prop_oneof![
        Just(Strain::Clubs),
        Just(Strain::Diamonds),
        Just(Strain::Hearts),
        Just(Strain::Spades),
        Just(Strain::Notrump),
    ]
}

fn bid() -> impl Strategy<Value = Bid> {
    (1u8..=7, strain()).prop_map(|(level, strain)| Bid {
        level: Level::new(level),
        strain,
    })
}

fn call() -> impl Strategy<Value = Call> {
    prop_oneof![
        3 => bid().prop_map(Call::Bid),
        3 => Just(Call::Pass),
        1 => Just(Call::Double),
        1 => Just(Call::Redouble),
    ]
}

fn auction() -> impl Strategy<Value = Auction> {
    prop::collection::vec(call(), 0..48).prop_map(|calls| {
        let mut a = Auction::new();
        for c in calls {
            if a.has_ended() {
                break;
            }
            let _ = a.try_push(c);
        }
        a
    })
}

fn deck() -> impl Strategy<Value = Deck> {
    any::<u64>().prop_map(|bits| Deck::from(Hand::from_bits_truncate(bits)))
}

proptest! {
    #[test]
    fn call_display_parse_roundtrip(c in call()) {
        let printed = c.to_string();
        let parsed: Call = printed.parse().map_err(|e: ParseCallError| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(parsed, c);
    }

    #[test]
    fn auction_display_parse_roundtrip(a in auction()) {
        let printed = a.to_string();
        let parsed: Auction = printed.parse().map_err(|e: ParseAuctionError| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(parsed, a);
    }

    #[test]
    fn deck_display_parse_roundtrip(d in deck()) {
        let printed = d.to_string();
        let parsed: Deck = printed.parse().map_err(|e: ParseHandError| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(parsed, d);
    }
}
