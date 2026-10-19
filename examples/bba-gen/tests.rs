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

/// `--declare-their-european-minors` parses, defaults off, and leaves the
/// armed knobs alone: it is a harness flag on the reading channel, not a
/// system knob, so `default_args_arm_the_shipped_system` holds with it on.
#[test]
fn the_european_minors_flag_is_a_reading_flag_only() {
    let bare = Args::parse_from(["bba-gen"]);
    assert!(!bare.declare_their_european_minors);
    let on = Args::parse_from(["bba-gen", "--declare-their-european-minors"]);
    assert!(on.declare_their_european_minors);
    assert_eq!(arm_knobs(&on).unwrap(), arm_knobs(&bare).unwrap());
}

/// With the declaration our seat reads their `(1NT) - (2NT)` as the European
/// invitation, not our Puppet-scheme diamond transfer
#[test]
fn declared_european_minors_read_their_2nt_as_an_invitation() {
    use contract_bridge::auction::{Call, RelativeVulnerability};
    use contract_bridge::{Bid, Strain, Suit};

    let agreements = arm_knobs(&Args::parse_from(["bba-gen"])).unwrap();
    let auction = [
        Call::Bid(Bid::new(1, Strain::Notrump)),
        Call::Pass,
        Call::Bid(Bid::new(2, Strain::Notrump)),
    ];
    let ours = super::seat_floor("american", &agreements).unwrap();
    let symmetric = ours.infer(RelativeVulnerability::NONE, &auction);
    let declared = super::european_minors_opponents(ours, &agreements)
        .infer(RelativeVulnerability::NONE, &auction);
    assert!(
        symmetric.rho().length(Suit::Diamonds).min >= 5,
        "undeclared, their 2NT reads as our diamond transfer: {:?}",
        symmetric.rho()
    );
    assert!(
        declared.rho().length(Suit::Diamonds).min < 5,
        "declared, their 2NT shows no diamond suit: {:?}",
        declared.rho()
    );
}
