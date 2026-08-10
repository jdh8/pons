use super::super::tests::{best_call, best_call_with, call};
use crate::bidding::agreements::Agreements;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

#[test]
fn two_suiter_hcp_floor_bars_garbage_michaels() {
    use crate::bidding::constraint::PointScale;
    // Calibrated to the rule-of-N+8 opt-out — the scale these example
    // hands' points assume (the 6-6 freak reads 9, not the point-count 7).
    let mut agreements = Agreements::current();
    agreements.decision.reading.point_scale = PointScale::RuleOfNFloored;
    // Over their (1♥): a 5-HCP 6-6 freak reads 9 points and cues Michaels
    // at weight 2.0 straight into a penalty double (−17..−21 IMPs a board
    // in the remnant dump).  The documented gate was always "8+ HCP"; the
    // floor makes it real, and the hand overcalls its spades instead.  A
    // sound 11-count 5-5 still cues.
    let over_1h = [call(1, Strain::Hearts)];
    let garbage = "KJ9532.5..JT7632"; // 5 HCP, 9 points
    let sound = "KQ953.5.2.AQ632"; // 11 HCP, 5-5
    let (default_call, _) = best_call_with(&agreements, &over_1h, garbage);
    assert_eq!(
        default_call,
        call(1, Strain::Spades),
        "default: the floor bars the cue; the freak overcalls"
    );
    let (sound_call, _) = best_call_with(&agreements, &over_1h, sound);
    assert_eq!(
        sound_call,
        call(2, Strain::Hearts),
        "a sound 5-5 still cues Michaels"
    );

    let mut no_floor = agreements;
    no_floor.defense.two_suiter_hcp_floor = None;
    let (legacy_call, _) = best_call_with(&no_floor, &over_1h, garbage);
    assert_eq!(
        legacy_call,
        call(2, Strain::Hearts),
        "the bare points gate (off arm): 9 points cue Michaels"
    );
}

#[test]
fn doubled_unusual_2nt_runs_never_sits() {
    // Their 1NT, our both-minors 2NT (on by default), their penalty X — the
    // advancer must run to the longer minor, never sit in the doomed 2NT-X.
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Notrump),
        Call::Double,
    ];
    // Clubs longer → 3♣ (a book node, not a floored pass).
    let (c, floored) = best_call(&auction, "432.32.QJ8.T9876");
    assert_eq!(c, call(3, Strain::Clubs));
    assert!(!floored, "the runout must come from the book");
    // Diamonds longer → 3♦.
    let (d, _) = best_call(&auction, "432.32.QJ876.T98");
    assert_eq!(d, call(3, Strain::Diamonds));
}
