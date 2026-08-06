use super::*;
use crate::bidding::american::{set_notrump_minors, set_nt_splinter, set_xyz};

/// The checked-in cards are current
///
/// This is the gate the whole module exists for: ship a convention, forget
/// the card, and this goes red.  `cards/*.bbsa` are snapshots for humans and
/// for `--disclose FILE`; the generator is the source of truth.  Bless with
/// `cargo run --example bba-card -- --system american >cards/American.bbsa`
/// (and `--system dutch >cards/Dutch.bbsa`).
#[test]
fn the_checked_in_cards_match_the_generator() {
    assert_eq!(
        american_card().to_string(),
        include_str!("../../../cards/American.bbsa"),
        "cards/American.bbsa is stale — re-bless it (see this test's doc)",
    );
    assert_eq!(
        dutch_card().to_string(),
        include_str!("../../../cards/Dutch.bbsa"),
        "cards/Dutch.bbsa is stale — re-bless it (see this test's doc)",
    );
}

/// The card never claims a relocation the floor cannot make
///
/// `Kickback 1430` rides `relocating_now()`, which is `set_rkcb_variant` AND
/// `set_floor_rkcb`.  Before 2026-08-03 it read the variant alone, so
/// turning the floor's keycard machinery off while a relocation was selected
/// published a convention we then never bid — an undisclosed-system fault
/// before it is a measurement one, and it invalidates a kickback-vs-BBA
/// anchor.  This row is now the **only** channel by which the knob reaches
/// the floor — the v3 twin selection that read the same predicate is gone —
/// so the pin matters more, not less.
#[test]
fn the_card_discloses_kickback_only_when_the_floor_can_ask() {
    use crate::bidding::instinct::{RkcbVariant, set_floor_rkcb, set_rkcb_variant};

    let row = |name: &str| american_card().to_string().lines().any(|l| l == name);
    let claims_kickback = || row("Kickback 1430 = 1");
    assert!(row("Kickback 1430 = 0"), "the shipped default is plain 4NT");

    set_rkcb_variant(RkcbVariant::Kickback);
    assert!(claims_kickback(), "a live relocation must be disclosed");

    set_floor_rkcb(false);
    assert!(
        !claims_kickback(),
        "with the floor's keycard ask off there is nothing to relocate, \
         so the card must not claim Kickback"
    );

    set_floor_rkcb(true);
    set_rkcb_variant(RkcbVariant::Plain);
}

#[test]
fn schema_has_no_duplicate_rows() {
    let mut names: Vec<_> = SCHEMA.to_vec();
    names.sort_unstable();
    let count = names.len();
    names.dedup();
    assert_eq!(names.len(), count, "duplicate row name in the .bbsa schema");
}

/// A `PONS_SCHEMA` name EPBot recognises would *stick*, silently flipping a
/// real BBA convention instead of occupying a spare slot.  `SCHEMA` is the
/// transcription of EPBot's list, so disjointness from it is the check
/// available here (no EPBot in `cargo test`).
#[test]
fn pons_rows_do_not_shadow_the_schema() {
    for name in PONS_SCHEMA {
        assert!(
            !SCHEMA.contains(name),
            "`{name}` is a real EPBot row — a card that sets it would change BBA's bidding"
        );
    }
    let mut names = PONS_SCHEMA.to_vec();
    names.sort_unstable();
    let count = names.len();
    names.dedup();
    assert_eq!(names.len(), count, "duplicate row name in `PONS_SCHEMA`");
}

#[test]
fn the_rendered_card_has_the_cards_full_length() {
    // Header + named rows + filler + `Opponent type`, as BBA writes it.  The
    // pons-only rows spend filler slots, so the total is unmoved.
    let text = american_card().to_string();
    assert_eq!(text.lines().count(), 1 + SCHEMA.len() + NOT_DEFINED + 1);
    assert_eq!(
        text.lines()
            .filter(|line| *line == "Not defined = 0")
            .count(),
        NOT_DEFINED - PONS_SCHEMA.len()
    );
    assert_eq!(text.lines().next(), Some("System type = 0"));
    assert_eq!(text.lines().next_back(), Some("Opponent type = 0"));
}

#[test]
fn a_knob_moves_its_row() {
    set_nt_splinter(false);
    assert_eq!(american_card().row("1N-3M splinter"), Some(0));
    set_nt_splinter(true);
    assert_eq!(american_card().row("1N-3M splinter"), Some(1));

    set_xyz(false);
    assert_eq!(american_card().row("Two Way New Minor Forcing"), Some(0));
    set_xyz(true);
    assert_eq!(american_card().row("Two Way New Minor Forcing"), Some(1));

    // The minor scheme is a radio group: exactly one of Puppet `3♣` and the
    // European `3♣`-diamond transfer is ever live.
    set_notrump_minors(EUROPEAN);
    let card = american_card();
    assert_eq!(card.row("1N-3C Puppet Stayman"), Some(0));
    assert_eq!(card.row("1N-3C transfer to diamonds"), Some(1));
    assert_eq!(card.row("1N-2N transfer to diamonds"), Some(0));
    set_notrump_minors(crate::bidding::american::PUPPET);
}

#[test]
fn dutch_differs_from_american_in_the_diamond_opening() {
    let (american, dutch) = (american_card(), dutch_card());
    assert_eq!(dutch.system, 2, "Dutch declares the WJ base");
    let moved: Vec<_> = SCHEMA
        .iter()
        .filter(|name| american.row(name) != dutch.row(name))
        .collect();
    assert_eq!(moved, [&"1D opening with 5 cards"]);
}

#[test]
#[should_panic(expected = "not a row of the .bbsa schema")]
fn setting_an_unknown_row_panics() {
    american_card().set("Ghestem Cuebid", 1);
}
