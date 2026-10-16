use super::{Args, arm_knobs};
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
