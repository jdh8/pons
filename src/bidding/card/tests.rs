use super::*;
use crate::bidding::agreements::Agreements;

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
        american_card(&crate::bidding::agreements::Agreements::current()).to_string(),
        include_str!("../../../cards/American.bbsa"),
        "cards/American.bbsa is stale — re-bless it (see this test's doc)",
    );
    assert_eq!(
        dutch_card(&crate::bidding::agreements::Agreements::current()).to_string(),
        include_str!("../../../cards/Dutch.bbsa"),
        "cards/Dutch.bbsa is stale — re-bless it (see this test's doc)",
    );
}

/// The card never claims a relocation the floor cannot make
///
/// `Kickback 1430` rides the combination of [`rkcb_variant`] and
/// [`floor_rkcb`][field@crate::bidding::inference::ReadingProfile::floor_rkcb].
/// Before 2026-08-03 it read the variant alone, so
/// turning the floor's keycard machinery off while a relocation was selected
/// published a convention we then never bid — an undisclosed-system fault
/// before it is a measurement one, and it invalidates a kickback-vs-BBA
/// anchor.  This row is now the **only** channel by which the knob reaches
/// the floor — the v3 twin selection that read the same predicate is gone —
/// so the pin matters more, not less.
#[test]
fn the_card_discloses_kickback_only_when_the_floor_can_ask() {
    use crate::bidding::instinct::RkcbVariant;

    let row = |agreements: &Agreements, name: &str| {
        american_card(agreements)
            .to_string()
            .lines()
            .any(|l| l == name)
    };
    let plain = Agreements::default();
    assert!(
        row(&plain, "Kickback 1430 = 0"),
        "the shipped default is plain 4NT"
    );

    let mut kickback = plain;
    kickback.decision.reading.rkcb_variant = RkcbVariant::Kickback;
    assert!(
        row(&kickback, "Kickback 1430 = 1"),
        "a live relocation must be disclosed"
    );

    kickback.decision.reading.floor_rkcb = false;
    assert!(
        !row(&kickback, "Kickback 1430 = 1"),
        "with the floor's keycard ask off there is nothing to relocate, \
         so the card must not claim Kickback"
    );
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
    let text = american_card(&crate::bidding::agreements::Agreements::current()).to_string();
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
    let mut agreements = Agreements::default();
    agreements.decision.reading.nt_splinter = false;
    assert_eq!(american_card(&agreements).row("1N-3M splinter"), Some(0));
    agreements.decision.reading.nt_splinter = true;
    assert_eq!(american_card(&agreements).row("1N-3M splinter"), Some(1));

    agreements.decision.reading.xyz = false;
    assert_eq!(
        american_card(&agreements).row("Two Way New Minor Forcing"),
        Some(0)
    );
    agreements.decision.reading.xyz = true;
    assert_eq!(
        american_card(&agreements).row("Two Way New Minor Forcing"),
        Some(1)
    );

    // The minor scheme is a radio group: exactly one of Puppet `3♣` and the
    // European `3♣`-diamond transfer is ever live.
    agreements.decision.reading.notrump_minors = EUROPEAN;
    let card = american_card(&agreements);
    assert_eq!(card.row("1N-3C Puppet Stayman"), Some(0));
    assert_eq!(card.row("1N-3C transfer to diamonds"), Some(1));
    assert_eq!(card.row("1N-2N transfer to diamonds"), Some(0));

    // The off-shape treatment admits *any* 5422 plus 4441/5431 with a singleton
    // honour, so it owns two shape rows of its own — the shape ladder alone
    // never reaches 4441.
    let onshape = Agreements::current();
    let mut offshape = onshape;
    offshape.opening.one_notrump_offshape = true;
    assert_eq!(
        american_card(&onshape).row("1NT opening shape 4441"),
        Some(0)
    );
    assert_eq!(
        american_card(&offshape).row("1NT opening shape 4441"),
        Some(1)
    );

    // 5422 is already on at the shipped `Wide6322`, so the off-shape half of
    // that row is only visible from `Balanced`.
    let mut balanced = onshape;
    balanced.opening.notrump_shape = NotrumpShape::Balanced;
    let mut balanced_offshape = balanced;
    balanced_offshape.opening.one_notrump_offshape = true;
    assert_eq!(
        american_card(&balanced).row("1NT opening shape 5422"),
        Some(0)
    );
    assert_eq!(
        american_card(&balanced_offshape).row("1NT opening shape 5422"),
        Some(1)
    );
}

#[test]
fn dutch_differs_from_american_in_the_diamond_opening() {
    let (american, dutch) = (
        american_card(&crate::bidding::agreements::Agreements::current()),
        dutch_card(&crate::bidding::agreements::Agreements::current()),
    );
    assert_eq!(dutch.system, 2, "Dutch declares the WJ base");
    let moved: Vec<_> = SCHEMA
        .iter()
        .filter(|name| american.row(name) != dutch.row(name))
        .collect();
    assert_eq!(moved, [&"1D opening with 5 cards"]);
}

/// [`foreign_card`] reproduces the schema half and zeroes the pons-only half
///
/// Fed our own values it must rebuild our own card everywhere EPBot has a row,
/// and differ *only* on [`PONS_SCHEMA`] — the two rows no foreign engine holds.
/// That pins both halves of the contract: same names in the same order (a drift
/// would shift every later feature of `features_v4`), and no pons row smuggled
/// into a description of somebody else's system.
#[test]
fn a_foreign_card_mirrors_the_schema_and_zeroes_the_pons_rows() {
    let ours = american_card(&crate::bidding::agreements::Agreements::current());
    let mirrored = foreign_card(ours.system, |name| {
        ours.row(name)
            .expect("`read` is called with schema names only")
    });

    assert_eq!(mirrored.rows.len(), ours.rows.len());
    for name in SCHEMA {
        assert_eq!(mirrored.row(name), ours.row(name), "schema row `{name}`");
    }
    for name in PONS_SCHEMA {
        assert_eq!(mirrored.row(name), Some(0), "pons-only row `{name}`");
        assert_eq!(ours.row(name), Some(1), "the fixture needs a nonzero row");
    }
}

#[test]
#[should_panic(expected = "not a row of the .bbsa schema")]
fn setting_an_unknown_row_panics() {
    american_card(&crate::bidding::agreements::Agreements::current()).set("Ghestem Cuebid", 1);
}
