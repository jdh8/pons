use super::*;

fn self_play_fixture(floor: OurFloor) -> Arm {
    let partnership = floor
        .with(&pons::bidding::agreements::Agreements::default())
        .unwrap();
    let vul = AbsoluteVulnerability::NONE;
    let boards = common::seeded_deals(0xD3C0_5EED, 8)
        .into_iter()
        .enumerate()
        .map(|(index, deal)| {
            let dealer = Seat::ALL[index % Seat::ALL.len()];
            common::Board {
                table_a: common::bid_out(&partnership, &partnership, true, dealer, vul, &deal),
                table_b: common::bid_out(&partnership, &partnership, false, dealer, vul, &deal),
                deal,
                dealer,
            }
        })
        .collect::<Vec<_>>();
    arm_from_dump(Dump {
        our_label: format!("fixture {floor:?}"),
        their_label: "fixture reference".into(),
        vulnerability: vul,
        seed: Some(0xD3C0_5EED),
        gen_args: Vec::new(),
        declared_european_minors: None,
        boards,
    })
}

fn sample_row() -> Row {
    Row {
        arm: 0,
        board: 0,
        swing_plain: -3,
        swing_pd: -4,
        points: -100,
        div_index: 1,
        phase: Phase::Defensive,
        bucket: "Defensive / book / round-1".into(),
        prov: "book".into(),
        rule: "test rule".into(),
        family: "round-1",
        direction: "overbid",
        our_call: "1♠".into(),
        their_call: "P".into(),
        hand: "fixture hand".into(),
    }
}

#[test]
fn floor_cli_defaults_to_instinct_and_accepts_american() {
    let default = Args::try_parse_from(["bba-decompose", "fixture.json"]).unwrap();
    assert_eq!(default.our_floor, OurFloor::AmericanInstinct);
    let american =
        Args::try_parse_from(["bba-decompose", "fixture.json", "--our-floor", "american"]).unwrap();
    assert_eq!(american.our_floor, OurFloor::American);
}

#[test]
fn european_minors_derives_from_ben_label_and_accepts_false() {
    let default = Args::try_parse_from(["bba-decompose", "fixture.json"]).unwrap();
    assert_eq!(default.european_minors, None);
    let off = Args::try_parse_from([
        "bba-decompose",
        "fixture.json",
        "--european-minors",
        "false",
    ])
    .unwrap();
    assert_eq!(off.european_minors, Some(false));
}

#[test]
fn both_floor_selections_replay_their_small_fixture_exactly() {
    for floor in [OurFloor::American, OurFloor::AmericanInstinct] {
        let arm = self_play_fixture(floor);
        let (checked, mismatched) = replay_verify(
            &floor
                .with(&pons::bidding::agreements::Agreements::default())
                .unwrap(),
            &arm,
        );
        assert!(checked > 0);
        assert_eq!(mismatched, 0, "{floor:?}");
    }
}

#[test]
fn report_comparison_uses_the_dump_reference_label() {
    let rendered = call_comparison(&sample_row(), "BEN v0.8.8.4 21GF/F");
    assert_eq!(rendered, "1♠ ours vs P BEN v0.8.8.4 21GF/F");
    assert!(!rendered.contains("BBA"));
}

#[test]
fn json_emits_generic_call_and_compatibility_alias() {
    let arm = arm_from_dump(Dump {
        our_label: "ours".into(),
        their_label: "BEN".into(),
        vulnerability: AbsoluteVulnerability::NONE,
        seed: Some(7),
        gen_args: Vec::new(),
        declared_european_minors: None,
        boards: Vec::new(),
    });
    let row = sample_row();
    let json = row_json(
        &row,
        &Arm {
            origin: vec![(Some(7), 11)],
            ..arm
        },
    );
    assert_eq!(json["their_call"], "P");
    assert_eq!(json["bba_call"], json["their_call"]);
}

/// The census trigger mirrors `instinct::their_live_game_bid`: their last
/// action is an undoubled bid at the four-level or higher
#[test]
fn live_game_bid_is_their_undoubled_four_plus() {
    let bid = |level: u8, strain: Strain| Call::Bid(Bid::new(level, strain));
    // `1♠ (4♥)` to responder: RHO just bid it.
    assert_eq!(
        live_game_bid(&[bid(1, Strain::Spades), bid(4, Strain::Hearts)]),
        Some(Bid::new(4, Strain::Hearts))
    );
    // `(4♥) - -` to fourth seat: LHO bid it, two passes since, our pass ends it.
    assert!(live_game_bid(&[bid(4, Strain::Hearts), Call::Pass, Call::Pass]).is_some());
    // Doubled, or ours, or below game: no trigger.
    assert!(
        live_game_bid(&[bid(1, Strain::Spades), bid(4, Strain::Hearts), Call::Double]).is_none()
    );
    assert!(live_game_bid(&[bid(4, Strain::Hearts), Call::Pass]).is_none());
    assert!(live_game_bid(&[bid(1, Strain::Spades), bid(3, Strain::Hearts)]).is_none());
}
