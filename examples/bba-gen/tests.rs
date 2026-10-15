use super::{Args, Dump, arm_knobs, verify_shard, write_atomically};
use clap::Parser;
use pons::bidding::agreements::Agreements;

/// Default CLI arms the shipped system **plus the vs-BBA disclosure
/// corrections** — `bba-decompose`'s replay contract.
///
/// The decompose replays a dump through the same
/// [`vs_bba_agreements`][super::common::vs_bba_agreements]-corrected
/// defaults and demands 100% bit-reproduction, so every board it
/// attributes to a bucket assumes this equality. When it broke
/// (2026-08-10 anchor, 69 and 66 mismatched calls of ~2.1M) nothing
/// failed loudly: the replay-verified fraction just slipped below 100%
/// in a report line.  The only sanctioned divergence from
/// `Agreements::default()` is the disclosure channel (`their`), derived
/// from the opponent this harness hardwires — keep the shared transform
/// and `their_2c_landy`'s no-declaration arm in lockstep.
#[test]
fn default_args_arm_the_shipped_system() {
    let armed = arm_knobs(&Args::parse_from(["bba-gen"])).unwrap();
    let shipped = super::common::vs_bba_agreements(Agreements::default());
    if armed != shipped {
        let (a, s) = (format!("{armed:#?}"), format!("{shipped:#?}"));
        let diff: Vec<_> = a
            .lines()
            .zip(s.lines())
            .filter(|(x, y)| x != y)
            .map(|(x, y)| format!("armed{x}\n  shipped{y}"))
            .collect();
        panic!(
            "--ns-* defaults drifted from the shipped system:\n{}",
            diff.join("\n")
        );
    }
}

/// A shard written by `--output` round-trips through `--verify`, which
/// rejects a wrong board count, a wrong seed and a truncated file.
#[test]
fn written_shard_verifies_and_truncation_does_not() {
    let dir = std::env::temp_dir().join(format!("bba-gen-verify-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("shard-0.json");
    let path = path.to_str().unwrap();
    let dump = Dump {
        our_label: "us".into(),
        their_label: "them".into(),
        vulnerability: contract_bridge::AbsoluteVulnerability::NONE,
        seed: Some(7),
        gen_args: Vec::new(),
        boards: Vec::new(),
    };
    write_atomically(path, &dump).unwrap();
    // The temp file was renamed away, not left beside the shard.
    let names: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, ["shard-0.json"]);

    verify_shard(path, 0, Some(7)).unwrap();
    verify_shard(path, 0, None).unwrap();
    assert!(verify_shard(path, 1, Some(7)).is_err(), "board count");
    assert!(verify_shard(path, 0, Some(8)).is_err(), "seed");

    let bytes = std::fs::read(path).unwrap();
    std::fs::write(path, &bytes[..bytes.len() - 1]).unwrap();
    assert!(verify_shard(path, 0, Some(7)).is_err(), "truncated");
    std::fs::remove_dir_all(&dir).unwrap();
}
