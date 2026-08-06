use super::*;
use crate::bidding::Rules;
use crate::bidding::context::Context;
use crate::bidding::trie::Classifier;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain, Suit};

/// The highest-logit call a sub-builder makes for a hand in a context
fn best(rules: &Rules, auction: &[Call], hand: &str) -> Call {
    let hand: Hand = hand.parse().expect("valid test hand");
    let context = Context::new(RelativeVulnerability::NONE, auction);
    let logits = rules.classify(hand, &context);
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty")
}

#[test]
fn openings_pick_the_descriptive_bid() {
    let o = openings();
    // 16 balanced -> 1NT; 22 -> 2♣; five hearts -> 1♥; six spades, weak -> 2♠.
    assert_eq!(best(&o, &[], "AQ32.K53.QJ4.A92"), call(1, Strain::Notrump));
    assert_eq!(best(&o, &[], "AKQ2.AKJ.KQ4.932"), call(2, Strain::Clubs));
    assert_eq!(best(&o, &[], "A2.KQJ53.Q42.J92"), call(1, Strain::Hearts));
    assert_eq!(best(&o, &[], "KQJ732.53.842.92"), call(2, Strain::Spades));
}

#[test]
fn openings_suppress_weak_twos_in_fourth_seat() {
    // The same six-spade 6-count opens 2♠ in first seat but passes in fourth.
    let o = openings();
    assert_eq!(best(&o, &[], "KQJ732.53.842.92"), call(2, Strain::Spades));
    assert_eq!(best(&o, &[Call::Pass; 3], "KQJ732.53.842.92"), Call::Pass,);
}

#[test]
fn major_responses_run_the_2_over_1_ladder() {
    let r = major_responses(Suit::Hearts);
    let a = [call(1, Strain::Hearts), Call::Pass];
    assert_eq!(best(&r, &a, "K2.KQ54.A964.Q92"), call(2, Strain::Notrump));
    assert_eq!(best(&r, &a, "Q32.J53.A964.Q92"), call(2, Strain::Hearts));
    assert_eq!(best(&r, &a, "A2.K3.Q543.KJ85"), call(2, Strain::Clubs));
}

#[test]
fn choice_of_games_three_notrump() {
    let a = [call(1, Strain::Hearts), Call::Pass];
    let on = major_responses(Suit::Hearts);
    set_major_choice_of_games(false);
    let off = major_responses(Suit::Hearts);
    set_major_choice_of_games(true);

    // Flat (4333) with four trumps, 13 HCP: 3NT outranks Jacoby 2NT.
    assert_eq!(best(&on, &a, "K32.KQ54.A96.J92"), call(3, Strain::Notrump));
    assert_eq!(best(&off, &a, "K32.KQ54.A96.J92"), call(2, Strain::Notrump));
    // Flat (4333) with three trumps, 12 HCP: 3NT; off it is a forcing 1NT.
    assert_eq!(best(&on, &a, "K32.K54.A964.Q92"), call(3, Strain::Notrump));
    assert_eq!(best(&off, &a, "K32.K54.A964.Q92"), call(1, Strain::Notrump));
    // 4=3=3=3 over 1♥ keeps bidding 1♠ — the spade exclusion is load-bearing.
    assert_eq!(best(&on, &a, "KQ32.K54.A96.Q92"), call(1, Strain::Spades));
    assert_eq!(best(&off, &a, "KQ32.K54.A96.Q92"), call(1, Strain::Spades));
}

#[test]
fn two_over_one_fit_leg_and_gates() {
    use crate::bidding::constraint::{PointScale, set_point_scale};
    // Calibrated to the rule-of-N+8 opt-out — the scale the Points13 arm's
    // example hand assumes (the 6-4 reads 13, not the point-count 12).
    set_point_scale(PointScale::RuleOfNFloored);
    let a = [call(1, Strain::Hearts), Call::Pass];
    // Arms are relative to the legacy gate; the shipped default is
    // fit + Points13 (the `fit` arm below; restored at the end).
    set_two_over_one_fit(false);
    set_two_over_one_gate(TwoOverOneGate::Points13);
    let baseline = major_responses(Suit::Hearts);
    set_two_over_one_fit(true);
    let fit = major_responses(Suit::Hearts);
    set_two_over_one_fit(false);
    set_two_over_one_gate(TwoOverOneGate::Hcp13);
    let hcp13 = major_responses(Suit::Hearts);
    set_two_over_one_gate(TwoOverOneGate::Hcp12);
    let hcp12 = major_responses(Suit::Hearts);
    set_two_over_one_fit(true);
    set_two_over_one_gate(TwoOverOneGate::Points13);

    // Fit leg: exactly three trumps, 11 HCP + spade singleton reads 13
    // support points — a 2/1 preparing the heart raise; off, a 1NT.
    assert_eq!(best(&fit, &a, "7.K54.A964.KJ932"), call(2, Strain::Clubs));
    assert_eq!(
        best(&baseline, &a, "7.K54.A964.KJ932"),
        call(1, Strain::Notrump)
    );
    // Hcp13 demotes a shaped 12 (6-4 reads 13 points) back to 1NT.
    assert_eq!(
        best(&baseline, &a, "32.Q4.AKJ964.Q93"),
        call(2, Strain::Diamonds)
    );
    assert_eq!(
        best(&hcp13, &a, "32.Q4.AKJ964.Q93"),
        call(1, Strain::Notrump)
    );
    // Hcp12 admits a no-fit flat 12 the shipped gate leaves in 1NT.
    assert_eq!(
        best(&baseline, &a, "K32.54.A964.KQ92"),
        call(1, Strain::Notrump)
    );
    assert_eq!(best(&hcp12, &a, "K32.54.A964.KQ92"), call(2, Strain::Clubs));
    set_point_scale(PointScale::PointCount);
}

#[test]
fn two_over_one_natural_lengths_and_light_major() {
    let a = [call(1, Strain::Spades), Call::Pass];
    // The major discount subtracts from an `Hcp*` floor (`hcp_floor -
    // discount`); the shipped `Points13` gate hardcodes `points(13..)` and
    // ignores it, so pin the raw-HCP gate this knob was designed against.
    set_two_over_one_gate(TwoOverOneGate::Hcp13);
    set_two_over_one_natural_lengths(true);
    let nat = major_responses(Suit::Spades);
    set_two_over_one_major_discount(true);
    let nat_light = major_responses(Suit::Spades);
    set_two_over_one_natural_lengths(false);
    set_two_over_one_major_discount(false);
    let baseline = major_responses(Suit::Spades);

    // 1♠-2♣ is the catch-all and may be three: a 2=4=4=3 game force bids
    // the cheaper club (weight 1.1) once three qualifies; on the uniform
    // 4+ floor it must show its four-card diamond instead.
    assert_eq!(
        best(&baseline, &a, "AK.KJ54.KJ54.432"),
        call(2, Strain::Diamonds)
    );
    assert_eq!(best(&nat, &a, "AK.KJ54.KJ54.432"), call(2, Strain::Clubs));

    // 1♠-2♥ promises five, and the discount lets a 12-HCP five-carder with
    // no spade fit force game; without it the no-fit floor is 13 and the
    // hand makes a forcing 1NT.
    assert_eq!(best(&nat, &a, "Q2.KQJ54.K32.J43"), call(1, Strain::Notrump));
    assert_eq!(
        best(&nat_light, &a, "Q2.KQJ54.K32.J43"),
        call(2, Strain::Hearts)
    );
    set_two_over_one_gate(TwoOverOneGate::default());
}

#[test]
fn notrump_responses_transfer_and_stayman() {
    let r = notrump_responses();
    let a = [call(1, Strain::Notrump), Call::Pass];
    assert_eq!(best(&r, &a, "KJ542.Q32.K43.92"), call(2, Strain::Hearts));
    // Four-four in the majors takes Stayman; a 4-3 hand would Puppet (3♣).
    assert_eq!(best(&r, &a, "KJ54.KQ32.43.Q92"), call(2, Strain::Clubs));
}

/// The ported constructive packages hold the row invariants (alerts;
/// totality is exact-node-exempt).  Gates are ignored by the probe, so the
/// default-off NMF package is checked too.
#[test]
fn row_package_invariants() {
    crate::bidding::rows::assert_package_invariants(&[
        openings::package(),
        weak_twos::package(),
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
        strong_two::package(),
        strong_two::minor_keycard_continuations(),
    ]);
}

#[test]
fn defense_doubles_with_strength() {
    let r = defense_to_suit(Bid::new(1, Strain::Diamonds));
    let a = [call(1, Strain::Diamonds)];
    // 18 HCP with length in their suit still doubles (planning to bid again).
    assert_eq!(best(&r, &a, "A.Q6.KJ852.AKJ42"), Call::Double);
    // A light five-card major overcalls.
    assert_eq!(best(&r, &a, "AQJ32.853.42.K92"), call(1, Strain::Spades));
}

/// The shipped-path twin of
/// [`the_configured_floor_reads_its_card`][crate::bidding::neural_floor]:
/// [`american`] must build its card from the *live* knobs, so a knob that
/// moves a card row moves the default floor's inputs.
///
/// Asserts on the logit vector, never on the chosen call — the
/// `Kickback 1430` row decides roughly one auction in seven hundred, so a
/// call-level assertion would be asserting noise.  What this catches is
/// someone hoisting the card into a `LazyLock`: the only symptom would be
/// every future card-row A/B measuring zero.
#[test]
fn the_default_floor_reads_the_live_card() {
    use crate::bidding::System as _;
    use crate::bidding::instinct::{RkcbVariant, set_rkcb_variant};

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

    let plain = logits(&american().against());
    set_rkcb_variant(RkcbVariant::Kickback);
    let relocated = logits(&american().against());
    set_rkcb_variant(RkcbVariant::Plain);

    assert_ne!(
        plain.0.into_iter().collect::<Vec<_>>(),
        relocated.0.into_iter().collect::<Vec<_>>(),
        "the default floor must read the card the live knobs describe"
    );
}
