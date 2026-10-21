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

/// A bare command line plays the crate-default agreements, so omitting every
/// `--ns-*` flag stays byte-identical to the shipped system.
#[test]
fn no_flags_play_the_crate_default_agreements() {
    let agreements = ns_agreements(&Args::try_parse_from(["ben-gen"]).unwrap());
    assert!(agreements == pons::bidding::agreements::Agreements::default());
}

#[test]
fn the_notrump_defense_flags_each_move_their_knob() {
    use pons::bidding::american::NotrumpDefense;
    let armed = |flags: &[&str]| {
        ns_agreements(
            &Args::try_parse_from(core::iter::once("ben-gen").chain(flags.iter().copied()))
                .unwrap(),
        )
    };
    let default = armed(&[]);
    assert_eq!(
        default.decision.reading.notrump_defense,
        NotrumpDefense::Natural
    );
    assert!(default.defense.woolsey_muiderberg_sit && default.defense.woolsey_tails);

    let woolsey = armed(&["--ns-notrump-defense", "woolsey"]);
    assert_eq!(
        woolsey.decision.reading.notrump_defense,
        NotrumpDefense::Woolsey
    );
    assert!(woolsey.defense.woolsey_muiderberg_sit && woolsey.defense.woolsey_tails);

    // DONT and Meckwell field bba-gen's card: the 8–14 unusual `2NT`, and
    // DONT's one-suiter minimum from its flag.
    let dont = armed(&[
        "--ns-notrump-defense",
        "direct-dont",
        "--ns-dont-one-suiter-min",
        "6",
    ]);
    assert_eq!(dont.defense.unusual_notrump_range, Some((8, 14)));
    assert_eq!(dont.defense.direct_dont_one_suiter_min, 6);
    let meckwell = armed(&["--ns-notrump-defense", "meckwell"]);
    assert_eq!(meckwell.defense.unusual_notrump_range, Some((8, 14)));
    assert_eq!(woolsey.defense.unusual_notrump_range, Some((8, 13)));

    let no_sit = armed(&["--no-ns-woolsey-muiderberg-sit"]);
    assert!(!no_sit.defense.woolsey_muiderberg_sit && no_sit.defense.woolsey_tails);
    let no_tails = armed(&["--no-ns-woolsey-tails"]);
    assert!(no_tails.defense.woolsey_muiderberg_sit && !no_tails.defense.woolsey_tails);
}

#[test]
fn the_xx_major_first_flag_moves_its_knob() {
    use pons::bidding::american::XxMajorFirst;
    let parsed = |flags: &[&str]| {
        ns_agreements(
            &Args::try_parse_from(core::iter::once("ben-gen").chain(flags.iter().copied()))
                .unwrap(),
        )
        .defense
        .xx_dont_major_first
    };
    assert_eq!(parsed(&[]), XxMajorFirst::Longer);
    assert_eq!(
        parsed(&["--ns-xx-dont-major-first", "off"]),
        XxMajorFirst::Off
    );
    assert_eq!(
        parsed(&["--ns-xx-dont-major-first", "at-least-as-long"]),
        XxMajorFirst::AtLeastAsLong
    );
}

#[test]
fn defense_isolation_rejects_offsets_and_self_play() {
    assert!(Args::try_parse_from(["ben-gen", "--isolate-defense"]).is_ok());
    assert!(Args::try_parse_from(["ben-gen", "--isolate-defense", "--filter-1nt"]).is_ok());
    let offset = [
        "ben-gen",
        "--isolate-defense",
        "--seed",
        "4",
        "--start-board",
        "3",
    ];
    assert!(Args::try_parse_from(offset).is_err());
    assert!(
        Args::try_parse_from(["ben-gen", "--isolate-defense", "--self-play", "c.jsonl"]).is_err()
    );
}

/// Passes throughout: the defender that never competes.
struct Passer;

impl Bidder for Passer {
    fn classify(&self, _: Hand, _: RelativeVulnerability, _: &[Call]) -> Option<Logits> {
        Some(one_hot(Call::Pass))
    }
}

/// Opens 1NT on 15–17 HCP in any seat while the auction is all passes, else
/// passes — a stand-in for BEN that makes the isolation predicate checkable.
struct NotrumpOpener;

fn hand_hcp(hand: Hand) -> u32 {
    [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
        .into_iter()
        .map(|suit| hcp::<u32>(hand[suit]))
        .sum()
}

impl Bidder for NotrumpOpener {
    fn classify(&self, hand: Hand, _: RelativeVulnerability, auction: &[Call]) -> Option<Logits> {
        let opens =
            auction.iter().all(|&call| call == Call::Pass) && (15..=17).contains(&hand_hcp(hand));
        Some(one_hot(if opens {
            Call::Bid(Bid {
                level: Level::new(1),
                strain: Strain::Notrump,
            })
        } else {
            Call::Pass
        }))
    }
}

fn numbered(seed: u64) -> impl Iterator<Item = (FullDeal, Seat, usize)> {
    match_deals(seed)
        .zip(1..)
        .map(|((deal, dealer), scanned)| (deal, dealer, scanned))
}

/// Without isolation the match is the first `count` deals at their own
/// dealers, table B with us at E/W — what `ben-gen` always bid.
#[test]
fn the_plain_match_bids_every_dealt_board() {
    let seed = 1784237746;
    let vul = AbsoluteVulnerability::NONE;
    let (boards, scanned) = bid_match(numbered(seed), 9, false, vul, &Passer, &NotrumpOpener);
    assert_eq!(scanned, 9);
    let expected: Vec<_> = match_deals(seed).take(9).collect();
    assert_eq!(boards.len(), 9);
    for (board, (deal, dealer)) in boards.iter().zip(expected) {
        assert_eq!((board.deal, board.dealer), (deal, dealer));
        assert_eq!(
            board.table_b,
            bid_out(&Passer, &NotrumpOpener, false, dealer, vul, &deal)
        );
    }
}

/// `--isolate-defense` keeps exactly the boards where E/W open 1NT against our
/// silent N/S, rotates the dealer per kept board, and bids table B all-BEN.
#[test]
fn defense_isolation_keeps_their_1nt_openings_only() {
    let seed = 1784237746;
    let vul = AbsoluteVulnerability::NONE;
    let (boards, scanned) = bid_match(numbered(seed), 12, true, vul, &Passer, &NotrumpOpener);
    // With our side passing throughout, a board is kept iff E or W holds 15–17.
    let mut expected = Vec::new();
    let mut dealt = 0;
    for (deal, _) in match_deals(seed) {
        if expected.len() == 12 {
            break;
        }
        dealt += 1;
        if [Seat::East, Seat::West]
            .into_iter()
            .any(|seat| (15..=17).contains(&hand_hcp(deal[seat])))
        {
            expected.push(deal);
        }
    }
    assert_eq!(scanned, dealt);
    assert_eq!(boards.len(), 12);
    for (index, (board, deal)) in boards.iter().zip(expected).enumerate() {
        assert_eq!(board.deal, deal);
        assert_eq!(board.dealer, Seat::ALL[index % 4]);
        assert!(matches!(
            opening_1nt(&board.table_a, board.dealer),
            Some((_, false))
        ));
        assert_eq!(
            board.table_b,
            bid_out(
                &NotrumpOpener,
                &NotrumpOpener,
                true,
                board.dealer,
                vul,
                &deal
            )
        );
    }
}
