use super::*;
use crate::bidding::Rules;
use crate::bidding::context::{Context, DecisionProfile};
use crate::bidding::trie::Classifier;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Hand, Strain};

/// The highest-logit call a sub-builder makes for a hand in a context
pub(super) fn best(rules: &Rules, auction: &[Call], hand: &str) -> Call {
    best_on(rules, auction, hand, DecisionProfile::default())
}

/// The highest-logit call under an explicit classify-time profile.
pub(super) fn best_on(
    rules: &Rules,
    auction: &[Call],
    hand: &str,
    profile: DecisionProfile,
) -> Call {
    let hand: Hand = hand.parse().expect("valid test hand");
    let context = Context::new(RelativeVulnerability::NONE, auction).with_profile(profile);
    let logits = rules.classify(hand, &context);
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty")
}

/// The ported constructive packages hold the row invariants (alerts;
/// totality is exact-node-exempt).  Gates are ignored by the probe, so the
/// default-off NMF package is checked too.
#[test]
fn row_package_invariants() {
    crate::bidding::rows::assert_package_invariants(
        &Agreements::default(),
        &[
            openings::package(),
            weak_twos::package(),
            responses::package(),
            responses::choice_of_games_continuations(),
            responses::minor_keycard_continuations(),
            xyz::package(),
            nmf::package(),
            notrump::base(),
            notrump::cue(),
            notrump::minor_slam(),
            notrump::crawling(),
            notrump::invitational_majors(),
            notrump::heart_transfer_rebids(),
            notrump::spade_transfer_rebids(),
            notrump::heart_transfer_slam_try(),
            notrump::spade_transfer_slam_try(),
            notrump::spade_transfer_game_force(),
            notrump::heart_transfer_game_force(),
            notrump::sixcard_invite(),
            notrump::both_majors_relay(),
            notrump::five_card_max(),
            notrump::puppet(),
            notrump::european_three_club(),
            notrump::both_majors_three_diamond(),
            notrump::notrump_splinter(),
            notrump::texas_transfers(),
            notrump::texas_drive(),
            notrump::diamond_transfer(),
            notrump::european_two_notrump(),
            notrump::two_spade_two_way(),
            notrump::european_two_spade(),
            notrump::two_notrump_structure(),
            notrump::two_notrump_rebids(),
            rebids::forcing_notrump_continuations(),
            rebids::invitational_minor_continuations(),
            rebids::major_jump_rebid_continuations(),
            rebids::forcing_nt_two_suiter_continuations(),
            rebids::meckstroth_two_notrump_continuations(),
            rebids::one_heart_one_spade_rebid(),
            rebids::major_rebid_tail_continuations(),
            rebids::fourth_suit_forcing_continuations(),
            rebids::remaining_rebid_bases(),
            game_force::base(),
            game_force::opener_third_continuations(),
            game_force::second_suit_agreement_continuations(),
            game_force::backstops(),
            raises::jacoby_continuations(),
            raises::major_game_try_continuations(),
            raises::limit_raise_acceptance_continuations(),
            strong_two::package(),
            strong_two::minor_keycard_continuations(),
        ],
    );
}

/// The shipped-path twin of
/// [`each_compact_axis_moves_its_slots_and_only_live_ones_move_the_net`][crate::bidding::features]:
/// [`american`] must build its [`ConventionCard`][crate::bidding::features::ConventionCard]
/// from the *live* knobs, so a knob that moves a compact slot moves the default
/// floor's inputs.
///
/// That test shells [`ConfiguredFloorV5`][crate::bidding::neural_floor::ConfiguredFloorV5]
/// directly, to isolate the net from the book; this one deliberately does not,
/// so it is the only cover for `american()`'s own capture expression and
/// `common::with_floor_v5`.
///
/// Asserts on the logit vector, never on the chosen call — `relocating`
/// (compact slot 1) decides roughly one auction in seven hundred, so a
/// call-level assertion would be asserting noise.  What this catches is someone
/// hoisting the capture into a `LazyLock`: the only symptom would be every
/// future axis A/B measuring zero.
#[test]
fn the_default_floor_reads_the_live_agreements() {
    use crate::bidding::Bidder as _;
    use crate::bidding::instinct::RkcbVariant;

    // Not a forced auction, and no authored node — the net answers.
    let auction = [
        call(1, Strain::Hearts),
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ];
    let hand: Hand = "92.K53.AQJ42.962".parse().expect("valid test hand");
    let logits = |stance: &crate::bidding::Stance| {
        stance
            .classify(hand, RelativeVulnerability::NONE, &auction)
            .expect("the floor always answers")
    };

    let plain_agreements = crate::bidding::agreements::Agreements::default();
    let plain = logits(&american(&plain_agreements).against());
    let mut relocated_agreements = plain_agreements;
    relocated_agreements.decision.reading.rkcb_variant = RkcbVariant::Kickback;
    let relocated = logits(&american(&relocated_agreements).against());

    assert_ne!(
        plain.0.into_iter().collect::<Vec<_>>(),
        relocated.0.into_iter().collect::<Vec<_>>(),
        "the default floor must read the agreements the live knobs describe"
    );
}
