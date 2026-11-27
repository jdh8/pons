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

/// The European-minors declaration is on by default and a reading flag only:
/// opting out leaves the armed knobs alone, so
/// `default_args_arm_the_shipped_system` holds either way.  It stands down
/// where there is no EPBot opponent to describe.
#[test]
fn the_european_minors_declaration_is_a_default_reading_flag() {
    let bare = Args::parse_from(["bba-gen"]);
    assert!(super::declares_european_minors(&bare));
    let off = Args::parse_from(["bba-gen", "--no-declare-their-european-minors"]);
    assert!(!super::declares_european_minors(&off));
    assert_eq!(arm_knobs(&off).unwrap(), arm_knobs(&bare).unwrap());
    for pons_seat in [
        &["bba-gen", "--their-floor", "american"][..],
        &["bba-gen", "--isolate-opening", "pons"],
        &["bba-gen", "--ns-blind-opponent-reading"],
    ] {
        let args = Args::parse_from(pons_seat);
        assert!(!super::declares_european_minors(&args), "{pons_seat:?}");
    }
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

/// The reading of `auction` under the declared EPBot book of the arm that
/// `argv` builds, every seat (an arm knob can move any of them)
fn declared_reading(argv: &[&str], auction: &[contract_bridge::auction::Call]) -> String {
    use contract_bridge::auction::RelativeVulnerability;
    let agreements = arm_knobs(&Args::parse_from(argv)).unwrap();
    let ours = super::seat_floor("american", &agreements).unwrap();
    let reading = super::european_minors_opponents(ours, &agreements)
        .infer(RelativeVulnerability::NONE, auction);
    format!("{reading:?}")
}

/// An arm knob must not re-read EPBot's calls: `auction` reads the same
/// with and without `flags`
fn assert_declared_ignores(flags: &[&str], auction: &[contract_bridge::auction::Call]) {
    let armed: Vec<&str> = std::iter::once("bba-gen")
        .chain(flags.iter().copied())
        .collect();
    assert_eq!(
        declared_reading(&["bba-gen"], auction),
        declared_reading(&armed, auction)
    );
}

#[test]
fn declared_opponents_ignore_our_two_over_one_suit_order() {
    use contract_bridge::auction::Call;
    use contract_bridge::{Bid, Strain};
    let auction = [
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Pass,
        Call::Bid(Bid::new(2, Strain::Hearts)),
    ];
    assert_declared_ignores(&["--ns-two-over-one-hearts-first"], &auction);
}

#[test]
fn declared_opponents_ignore_our_single_raise_gap() {
    use contract_bridge::auction::Call;
    use contract_bridge::{Bid, Strain};
    // Not their `2♥`: the knob moves the reading of their 2/1 (a gate board).
    let auction = [
        Call::Bid(Bid::new(1, Strain::Hearts)),
        Call::Pass,
        Call::Bid(Bid::new(2, Strain::Clubs)),
    ];
    assert_declared_ignores(&["--ns-single-raise-gap"], &auction);
}

#[test]
fn declared_opponents_ignore_our_strong_jump_shift() {
    use contract_bridge::auction::Call;
    use contract_bridge::{Bid, Strain};
    let auction = [
        Call::Bid(Bid::new(1, Strain::Hearts)),
        Call::Pass,
        Call::Bid(Bid::new(2, Strain::Spades)),
    ];
    assert_declared_ignores(&["--ns-heart-spade-strong-jump-shift", "15"], &auction);
}
