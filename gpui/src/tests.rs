use super::*;

/// The structs and the bidding box read the engine's JSON by guesswork about
/// its spelling; a board bid to the end proves every guess at once.
#[test]
fn a_practice_board_round_trips_through_the_client_types() {
    let mut table = WebTable::new("7");
    let mut snap: Snapshot =
        serde_json::from_str(&table.deal_practice("S", "N", "none", 0)).expect("snapshot");
    assert_eq!(snap.mode, "practice");

    // Every legal call is a button the bidding box draws
    let grid: Vec<String> = (1..=7)
        .flat_map(|level| STRAINS.map(|strain| format!("{level}{strain}")))
        .chain(["P", "X", "XX"].map(String::from))
        .collect();
    assert!(snap.your_turn && !snap.legal.is_empty());
    assert!(snap.legal.iter().all(|call| grid.contains(call)));
    assert!(snap.legal.contains(&"7NT".to_string()));

    let hint: Vec<Hint> = serde_json::from_str(&table.hint()).expect("hint");
    assert_eq!(hint.len(), 5);

    while !snap.ended {
        snap = serde_json::from_str(&table.bid("P")).expect("snapshot");
    }
    assert!(!snap.feedback.is_empty());

    let dd: Dd = serde_json::from_str(&table.dd_table()).expect("dd table");
    assert_eq!(dd.rows.len(), 5);
    assert!(dd.rows.iter().all(|row| row.tricks.len() == 4));

    // `null` on a pass-out; otherwise the parallel batch counts every sample
    if let Ok(oracle) = serde_json::from_str::<Oracle>(&table.oracle(ORACLE_CHUNK)) {
        assert_eq!(oracle.n, ORACLE_CHUNK);
    }
}

#[test]
fn the_book_and_the_settings_registry_parse() {
    let nodes: Vec<Node> = serde_json::from_str(&pons_web::book("ns")).expect("book");
    assert!(!nodes.is_empty());

    let settings: Vec<Setting> =
        serde_json::from_str(&pons_web::describe_options()).expect("settings");
    assert!(settings.iter().any(|setting| setting.kind == "toggle"));
    assert!(
        settings
            .iter()
            .filter(|setting| setting.kind == "choice")
            .all(|setting| !setting.variants.is_empty())
    );
}

#[test]
fn the_filter_types_suits_as_letters() {
    assert_eq!(typeable("1NT - 2♣"), "1nt - 2c");
}

#[test]
fn a_registry_key_reads_as_a_label() {
    assert_eq!(humanize("their_2d_multi"), "Their 2♦ multi");
    assert_eq!(humanize("rkcb_over_1nt"), "RKCB over 1NT");
}
