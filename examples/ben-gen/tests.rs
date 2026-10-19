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

/// `--filter-1nt` deals `bba-gen --filter-1nt`'s accepted set: keep a deal
/// when the shared scan accepts it, rotate the dealer per kept deal.
#[test]
fn the_1nt_filter_deals_bba_gens_accepted_set() {
    let seed = 1784237746;
    for wide in [false, true] {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut expected = Vec::new();
        let mut scanned = 0;
        while expected.len() < 12 {
            let deal = full_deal(&mut rng);
            scanned += 1;
            if Filter::OneNt.accepts(&deal, wide) {
                expected.push((deal, Seat::ALL[expected.len() % 4], scanned));
            }
        }
        assert_eq!(
            filtered_deals(seed, wide).take(12).collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn the_1nt_filter_rejects_offsets_and_self_play() {
    assert!(Args::try_parse_from(["ben-gen", "--filter-1nt", "--wide-1nt"]).is_ok());
    assert!(Args::try_parse_from(["ben-gen", "--wide-1nt"]).is_err());
    let offset = [
        "ben-gen",
        "--filter-1nt",
        "--seed",
        "4",
        "--start-board",
        "3",
    ];
    assert!(Args::try_parse_from(offset).is_err());
    assert!(Args::try_parse_from(["ben-gen", "--filter-1nt", "--self-play", "c.jsonl"]).is_err());
}

#[test]
fn the_overcall_floor_flag_is_optional() {
    assert_eq!(
        Args::try_parse_from(["ben-gen"])
            .unwrap()
            .ns_nt_overcall_hcp_floor,
        None
    );
    let args = Args::try_parse_from(["ben-gen", "--ns-nt-overcall-hcp-floor", "8"]).unwrap();
    assert_eq!(args.ns_nt_overcall_hcp_floor, Some(8));
}
