use super::*;
use clap::error::ErrorKind;

#[test]
fn chunks_preserve_the_seeded_deals_and_original_dealer_rotation() {
    let seed = 1784237746;
    let mut rng = StdRng::seed_from_u64(seed);
    let original: Vec<_> = (0..13)
        .map(|index| (full_deal(&mut rng), Seat::ALL[index % 4]))
        .collect();
    let chunks: Vec<_> = [(0, 3), (3, 5), (8, 5)]
        .into_iter()
        .flat_map(|(start, count)| match_deals(seed).skip(start).take(count))
        .collect();
    assert_eq!(chunks, original);
    assert_eq!(match_deals(seed).take(13).collect::<Vec<_>>(), original);
}

#[test]
fn a_match_offset_requires_a_seed_and_rejects_self_play() {
    assert_eq!(Args::try_parse_from(["ben-gen"]).unwrap().start_board, 0);
    assert!(Args::try_parse_from(["ben-gen", "--start-board", "3"]).is_err());
    let args = ["ben-gen", "--start-board", "3", "--seed", "42"];
    assert_eq!(Args::try_parse_from(args).unwrap().start_board, 3);
    assert!(Args::try_parse_from(args.into_iter().chain(["--self-play", "out.jsonl"])).is_err());
}

#[test]
fn floor_selection_defaults_to_american() {
    assert_eq!(
        Args::try_parse_from(["ben-gen"]).unwrap().our_floor,
        "american"
    );
    let args = Args::try_parse_from(["ben-gen", "--our-floor", "american"]).unwrap();
    assert_eq!(args.our_floor, "american");
    let error = Args::try_parse_from(["ben-gen", "--our-floor", "american-instinct"])
        .err()
        .expect("only the shipped system is supported");
    assert_eq!(error.kind(), ErrorKind::InvalidValue);
}

#[test]
fn explicit_floor_selection_conflicts_with_modes_that_do_not_seat_pons() {
    for mode in [&["--calibrate-epbot"][..], &["--self-play", "corpus.jsonl"]] {
        let args = Args::try_parse_from(core::iter::once("ben-gen").chain(mode.iter().copied()))
            .expect("an implicit default must preserve the existing modes");
        assert_eq!(args.our_floor, "american");
        let error = Args::try_parse_from(
            ["ben-gen", "--our-floor", "american"]
                .into_iter()
                .chain(mode.iter().copied()),
        )
        .err()
        .expect("an explicit unused floor must be rejected");
        assert_eq!(error.kind(), ErrorKind::ArgumentConflict);
    }
}

/// `ben-gen-parallel.sh --verify` and ab-lib's fingerprint call these forms.
#[test]
fn shard_check_and_provenance_parse() {
    let args = Args::try_parse_from([
        "ben-gen",
        "--verify",
        "d/shard-3.json",
        "--count",
        "800",
        "--seed",
        "45",
    ])
    .unwrap();
    assert_eq!(args.verify.as_deref(), Some("d/shard-3.json"));
    assert_eq!((args.count, args.seed), (800, Some(45)));
    assert!(
        Args::try_parse_from(["ben-gen", "--provenance"])
            .unwrap()
            .provenance
    );
}
