// NOTE: there is deliberately no test here that drives EPBot.  Its NativeAOT
// runtime segfaults when called from a `cargo test` thread — the
// pre-existing 7-symbol `classify` path does too, which is why this module
// has only ever tested pure parsing.  The ABI self-check that would live
// here runs on the main thread instead, as
// `cargo run --example probe-bba-bilans -- --self-check`.

/// The vendored BEN card parses to BEN's declared system: 2/1 (id 0) with
/// its known toggle tweaks vs stock BBA 2/1 (see docs/ben-gap-campaign.md).
#[test]
fn ben_card_parses() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/vendor/ben/BEN-21GF.bbsa");
    let card = super::load_bbsa(path).expect("vendored card parses");
    assert_eq!(card.system, super::SYSTEM_2_OVER_1);
    assert_eq!(card.toggles.len(), 257);
    let get = |name: &str| {
        card.toggles
            .iter()
            .find(|(n, _)| n.to_str() == Ok(name))
            .unwrap_or_else(|| panic!("card has `{name}`"))
            .1
    };
    // All 10 toggle lines that differ from stock BBA-21GF.bbsa.
    assert_eq!(get("Blackwood 1430"), 1);
    assert_eq!(get("Blackwood 0314"), 0);
    assert_eq!(get("Leaping Michaels"), 1);
    assert_eq!(get("New Minor Forcing"), 0);
    assert_eq!(get("Two Way New Minor Forcing"), 1);
    assert_eq!(get("Strength Lawrence structure"), 1);
    assert_eq!(get("Shape Bergen structure"), 0);
    assert_eq!(get("1N-3M splinter"), 1);
    assert_eq!(get("Gerber only for NT openings"), 1);
    assert_eq!(get("Extended Stayman"), 0);
}
