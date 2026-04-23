use super::*;
use dds_bridge::Level;

fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid {
        level: Level::new(level),
        strain,
    })
}

#[test]
fn call_roundtrip() -> Result<(), ParseCallError> {
    for call in [
        Call::Pass,
        Call::Double,
        Call::Redouble,
        bid(1, Strain::Spades),
        bid(3, Strain::Notrump),
        bid(7, Strain::Clubs),
    ] {
        assert_eq!(call.to_string().parse::<Call>()?, call);
    }
    Ok(())
}

#[test]
fn call_parses_aliases_case_insensitive() -> Result<(), ParseCallError> {
    assert_eq!("p".parse::<Call>()?, Call::Pass);
    assert_eq!("PASS".parse::<Call>()?, Call::Pass);
    assert_eq!("pass".parse::<Call>()?, Call::Pass);
    assert_eq!("x".parse::<Call>()?, Call::Double);
    assert_eq!("dbl".parse::<Call>()?, Call::Double);
    assert_eq!("DOUBLE".parse::<Call>()?, Call::Double);
    assert_eq!("xx".parse::<Call>()?, Call::Redouble);
    assert_eq!("RDBL".parse::<Call>()?, Call::Redouble);
    assert_eq!("redouble".parse::<Call>()?, Call::Redouble);
    Ok(())
}

#[test]
fn call_rejects_garbage() {
    for s in ["", "Q", "8C", "1Z", "pas", "xxx"] {
        assert!(s.parse::<Call>().is_err(), "should reject: {s:?}");
    }
}

#[test]
fn relative_vulnerability_roundtrip() -> Result<(), ParseRelativeVulnerabilityError> {
    for v in [
        RelativeVulnerability::NONE,
        RelativeVulnerability::WE,
        RelativeVulnerability::THEY,
        RelativeVulnerability::ALL,
    ] {
        assert_eq!(v.to_string().parse::<RelativeVulnerability>()?, v);
    }
    Ok(())
}

#[test]
fn relative_vulnerability_parses_case_insensitive_and_aliases()
-> Result<(), ParseRelativeVulnerabilityError> {
    assert_eq!(
        "NONE".parse::<RelativeVulnerability>()?,
        RelativeVulnerability::NONE,
    );
    assert_eq!(
        "We".parse::<RelativeVulnerability>()?,
        RelativeVulnerability::WE,
    );
    assert_eq!(
        "all".parse::<RelativeVulnerability>()?,
        RelativeVulnerability::ALL,
    );
    assert!("ns".parse::<RelativeVulnerability>().is_err());
    Ok(())
}

#[test]
fn auction_roundtrip() -> anyhow::Result<()> {
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
    let s = auction.to_string();
    assert_eq!(s, "P 1♠ 2♥ X P P P");
    assert_eq!(s.parse::<Auction>()?, auction);
    Ok(())
}

#[test]
fn empty_auction_roundtrip() -> Result<(), ParseAuctionError> {
    let auction = Auction::new();
    assert_eq!(auction.to_string(), "");
    assert_eq!("".parse::<Auction>()?, auction);
    assert_eq!("   \t ".parse::<Auction>()?, auction);
    Ok(())
}

#[test]
fn auction_rejects_illegal_sequence() {
    // 2♠ after 3♥ is insufficient
    let err = "3♥ 2♠".parse::<Auction>().unwrap_err();
    assert!(matches!(err, ParseAuctionError::Illegal(_)));
}

#[test]
fn auction_rejects_bad_token() {
    let err = "P 1♠ Q".parse::<Auction>().unwrap_err();
    assert!(matches!(err, ParseAuctionError::Call(_)));
}
