use super::*;
use clap::error::ErrorKind;

#[test]
fn floor_selection_defaults_to_american_and_accepts_both_systems() {
    assert_eq!(
        Args::try_parse_from(["ben-gen"]).unwrap().our_floor,
        "american"
    );
    for floor in ["american", "dutch"] {
        let args = Args::try_parse_from(["ben-gen", "--our-floor", floor]).unwrap();
        assert_eq!(args.our_floor, floor);
    }
    let error = Args::try_parse_from(["ben-gen", "--our-floor", "american-instinct"])
        .err()
        .expect("only the two shipped systems are supported");
    assert_eq!(error.kind(), ErrorKind::InvalidValue);
}

#[test]
fn explicit_floor_selection_conflicts_with_modes_that_do_not_seat_pons() {
    for mode in [&["--calibrate-epbot"][..], &["--self-play", "corpus.jsonl"]] {
        let args = Args::try_parse_from(core::iter::once("ben-gen").chain(mode.iter().copied()))
            .expect("an implicit default must preserve the existing modes");
        assert_eq!(args.our_floor, "american");
        for floor in ["american", "dutch"] {
            let error = Args::try_parse_from(
                ["ben-gen", "--our-floor", floor]
                    .into_iter()
                    .chain(mode.iter().copied()),
            )
            .err()
            .expect("an explicit unused floor must be rejected");
            assert_eq!(error.kind(), ErrorKind::ArgumentConflict);
        }
    }
}
