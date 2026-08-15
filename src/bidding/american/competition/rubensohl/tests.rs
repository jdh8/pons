use super::super::tests::{bid_transfer, bid_transfer_dbl, call};
use contract_bridge::Strain;
use contract_bridge::auction::Call;

#[test]
fn transfer_smolen_three_clubs_is_stayman() {
    // 1NT (2♦): a 4-4 majors game-force bids 3♣ Stayman (a book node).
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid_transfer(&auction, "AQ32.KJ32.A2.432");
    assert_eq!(c, call(3, Strain::Clubs));
    assert!(!floored, "Stayman must come from the book");
}

#[test]
fn transfer_smolen_opener_answers_stayman() {
    // 1NT (2♦) 3♣: opener shows a 4-card major (3♥ here).
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    let (c, floored) = bid_transfer(&auction, "K2.AQ54.A32.Q432");
    assert_eq!(c, call(3, Strain::Hearts));
    assert!(!floored, "the Stayman answer must come from the book");
}

#[test]
fn transfer_smolen_three_diamonds_is_the_heart_transfer() {
    // The reshuffle: 1NT (2♦) 3♦ shows hearts (the freed cue slot), a book node.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid_transfer(&auction, "K3.KQ976.A32.432");
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the heart transfer must come from the book");

    // Opener auto-drives the INV+ transfer to game with a fit.
    let opener = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, _) = bid_transfer(&opener, "AQ5.A432.KQ4.J32");
    assert_eq!(c, call(4, Strain::Hearts));
}

#[test]
fn transfer_smolen_routes_five_four_to_stayman_not_a_transfer() {
    // A 5♠4♥ game-force must bid 3♣ Stayman (1.85), not the 3♥ spade transfer
    // (1.8) — else Smolen could never show the 5-4.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, _) = bid_transfer(&auction, "AKJ54.Q432.K2.32");
    assert_eq!(c, call(3, Strain::Clubs));
}

#[test]
fn transfer_smolen_jumps_smolen_after_the_denial() {
    // `1NT (2♦) 3♣ - 3♦ -` (no major): responder bids Smolen 3♥ to show 5 spades.
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(3, Strain::Clubs),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, floored) = bid_transfer(&auction, "AKJ54.Q432.K2.32");
    assert_eq!(c, call(3, Strain::Hearts));
    assert!(!floored, "Smolen must come from the book");

    // Opener completes in the five-card spade game.
    let mut full = auction.to_vec();
    full.push(call(3, Strain::Hearts));
    full.push(Call::Pass);
    let (c, _) = bid_transfer(&full, "Q32.A65.AQ43.K32");
    assert_eq!(c, call(4, Strain::Spades));
}

#[test]
fn transfer_smolen_leaping_michaels_both_majors() {
    // 1NT (2♦) 4♦ = both majors 5-5, game-forcing.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid_transfer(&auction, "KQ954.AJ876.2.32");
    assert_eq!(c, call(4, Strain::Diamonds));
    assert!(!floored, "Leaping Michaels must come from the book");

    // Opener bids game in the better major (4♠ on three-card support).
    let opener = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(4, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, _) = bid_transfer(&opener, "A32.K43.AQ32.Q42");
    assert_eq!(c, call(4, Strain::Spades));
}

#[test]
fn transfer_smolen_keeps_cohen_over_a_major_overcall() {
    // Over (2♥), Transfer is plain Cohen: 5 spades transfers through
    // hearts — 3♦ shows spades (the Smolen reshuffle is (2♦)-only).
    let auction = [call(1, Strain::Notrump), call(2, Strain::Hearts)];
    let (c, floored) = bid_transfer(&auction, "AKQ65.43.K32.J32");
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the Cohen transfer must come from the book");
}

#[test]
fn transfer_lebensohl_shows_spades_through_their_hearts() {
    // 1NT (2♥); responder, 5 spades and game values, transfers *through*
    // hearts: 3♦ shows spades (not diamonds), a book node.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Hearts)];
    let (c, floored) = bid_transfer(&auction, "AKQ65.43.K32.J32");
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the transfer must come from the book");
}

#[test]
fn transfer_lebensohl_opener_bids_game_not_a_partscore() {
    // After 1NT (2♥) 3♦ (transfer to spades), opener with a fit must bid
    // the spade *game*, never a 3♠ partscore (the Rubensohl-v1 failure).
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, _) = bid_transfer(&auction, "AK5.KQ52.A43.432");
    assert_eq!(c, call(4, Strain::Spades));
}

#[test]
fn transfer_lebensohl_cue_is_stayman() {
    // 1NT (2♥) 3♥ is the cue = Stayman; opener answers a 4-card major.
    // (Over (2♦) the cue slot is freed for the Smolen 3♣-Stayman instead.)
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let (c, floored) = bid_transfer(&auction, "AQ32.K43.A32.K32");
    assert_eq!(c, call(3, Strain::Spades));
    assert!(!floored, "the Stayman answer must come from the book");
}

#[test]
fn transfer_lebensohl_keeps_the_penalty_double() {
    // Length and values in their suit, no game bid of our own: with the
    // `Penalty` style on, double from the book — Rubensohl v1 lost this by
    // shadowing the floor. (The default is now `Optional` (2-3 cards), which
    // would route this 4-card-diamond hand elsewhere; see
    // [`takeout_authored_double`].)
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid_transfer_dbl(
        super::penalty_double::DoubleStyle::Penalty,
        &auction,
        "K2.K43.J932.Q432",
    );
    assert_eq!(c, Call::Double);
    assert!(!floored, "the penalty double must come from the book");
}

#[test]
fn transfer_lebensohl_weak_bids_natural_two_level() {
    // Weak 5-card heart hand still bids natural 2♥ (transfers are INV+).
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid_transfer(&auction, "K2.QJ976.432.432");
    assert_eq!(c, call(2, Strain::Hearts));
    assert!(!floored, "the natural 2-level bid must come from the book");
}

#[test]
fn transfer_lebensohl_top_step_is_a_clubs_transfer() {
    // The top step (no suit above to transfer into) is a forced game-force
    // transfer to clubs: 6+♣, game values, no stopper in their suit. The same
    // 10-HCP hand bids it over every overcall — 3♠ over (2♦)/(2♥), 3♥ over
    // (2♠) — a book node, never the natural floor. Tested under `Penalty`: the
    // default `Takeout` (≤3 in their suit, 1.55) outranks the clubs transfer
    // (1.45) and would hijack this short-suit hand into a takeout double — a
    // known weight interaction; the structural node is checked here in
    // isolation.
    let hand = "32.543.32.AKQJ86";
    for (over, top) in [
        (Strain::Diamonds, Strain::Spades),
        (Strain::Hearts, Strain::Spades),
        (Strain::Spades, Strain::Hearts),
    ] {
        let auction = [call(1, Strain::Notrump), call(2, over)];
        let (c, floored) =
            bid_transfer_dbl(super::penalty_double::DoubleStyle::Penalty, &auction, hand);
        assert_eq!(c, call(3, top), "top step → clubs over (2{over:?})");
        assert!(!floored, "the clubs transfer must come from the book");
    }
}

#[test]
fn transfer_lebensohl_traps_a_too_good_stopper() {
    // Over 1NT (2♥) with game values, a *too-good* heart stopper (♥AQ86, 6
    // HCP in their suit) traps: pass and wait for opener's reopening takeout
    // double, then convert. A merely *adequate* stopper (♥A964, 4 HCP) is a
    // source of tricks and still declares 3NT. (Trap pass on by default.)
    // The trap is a takeout-style mechanism — under the default Penalty style
    // this 4-card-heart hand doubles for penalty directly — so it is pinned to
    // Takeout here; the 3NT line (1.7) outranks any double, so it is style-free.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Hearts)];
    let (trap, _) = bid_transfer_dbl(
        super::penalty_double::DoubleStyle::Takeout,
        &auction,
        "K32.AQ86.KJ5.J32",
    );
    assert_eq!(
        trap,
        Call::Pass,
        "a too-good stopper (6 HCP in hearts) traps"
    );
    // Also pinned to Takeout: under Penalty default this 4-card-heart hand
    // prefers the penalty double (1.55) to the relay's direct 3NT (1.5) — four
    // trumps behind declarer beat one fragile stopper, which is sound.
    let (bid, _) = bid_transfer_dbl(
        super::penalty_double::DoubleStyle::Takeout,
        &auction,
        "K32.A964.KJ5.Q32",
    );
    assert_eq!(
        bid,
        call(3, Strain::Notrump),
        "an adequate stopper (4 HCP in hearts) still bids 3NT"
    );
}

#[test]
fn transfer_lebensohl_top_step_opener_completes_at_game() {
    // After 1NT (2♥) 3♠ (transfer to clubs, forced GF): opener bids 3NT with
    // a heart stopper, else raises to 5♣ — 3♣ is unplayable, so it reaches game.
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        call(3, Strain::Spades),
        Call::Pass,
    ];
    let (c, floored) = bid_transfer(&auction, "A432.KQ5.A32.432");
    assert_eq!(c, call(3, Strain::Notrump), "stopper → 3NT");
    assert!(!floored, "the completion must come from the book");

    let (c, _) = bid_transfer(&auction, "A432.543.AKQ.432");
    assert_eq!(c, call(5, Strain::Clubs), "no stopper → 5♣");
}

// ---- N4: their (2♦) as a Multi (`their.two_diamonds_multi`) ----

fn multi_arm() -> crate::bidding::agreements::Agreements {
    let mut arm = crate::bidding::agreements::Agreements::default();
    arm.decision.their.two_diamonds_multi = true;
    arm
}

fn multi_stopper_arm(mode: super::MultiStopperAsk) -> crate::bidding::agreements::Agreements {
    let mut arm = multi_arm();
    arm.competition.multi_stopper_ask = mode;
    arm
}

fn multi_corrected() -> Vec<Call> {
    vec![
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        Call::Double,
        call(2, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Spades),
    ]
}

fn multi_corrected_over_double() -> Vec<Call> {
    vec![
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        Call::Double,
        call(2, Strain::Hearts),
        Call::Double,
        call(2, Strain::Spades),
    ]
}

fn one_nt_two_diamonds() -> [Call; 2] {
    [call(1, Strain::Notrump), call(2, Strain::Diamonds)]
}

/// The values double carries no diamond claim: three small with 8+ doubles
/// under the Multi, and a diamond suit is not the reason.
#[test]
fn multi_double_is_values_not_diamonds() {
    use super::super::tests::best_call_with;
    let auction = one_nt_two_diamonds();
    // ♦432, 9 HCP, no major to show: the values double.
    let (c, floored) = best_call_with(&multi_arm(), &auction, "432.K32.KQ2.J432");
    assert_eq!(
        c,
        Call::Double,
        "9 with nothing better doubles under the Multi"
    );
    assert!(!floored, "the values double must be a book node");
    // 7 HCP flat: BBA's band (5–17) — still the double (v6).
    let (c, _) = best_call_with(&multi_arm(), &auction, "Q32.K32.432.Q432");
    assert_eq!(c, Call::Double, "7 flat doubles: BBA's values band");
    // 5 HCP flat: nothing to say.
    let (c, _) = best_call_with(&multi_arm(), &auction, "Q32.J32.432.Q432");
    assert_eq!(c, Call::Pass);
    // Four diamonds to the AKJ and 8 HCP: still the values double — no
    // diamond gate either way.
    let (c, _) = best_call_with(&multi_arm(), &auction, "432.432.AKJ2.432");
    assert_eq!(
        c,
        Call::Double,
        "diamond length neither enables nor blocks the double"
    );
}

/// The constructive leg is untouched by the disclosure: Stayman, the Jacoby
/// transfers and Leaping Michaels bid the same calls on the same hands.
#[test]
fn multi_keeps_the_constructive_leg() {
    use super::super::tests::best_call_with;
    let auction = one_nt_two_diamonds();
    for (hand, expected) in [
        ("AQ32.KJ32.A2.432", call(3, Strain::Clubs)), // 4-4 majors GF: Stayman
        ("K3.KQ976.A32.432", call(3, Strain::Diamonds)), // 5 hearts INV+: transfer
        ("KQ976.K3.A32.432", call(3, Strain::Hearts)), // 5 spades INV+: transfer
        ("AKJ54.KQ432.2.32", call(4, Strain::Diamonds)), // 5-5 majors: Leaping Michaels
    ] {
        let (c, floored) = best_call_with(&multi_arm(), &auction, hand);
        assert_eq!(c, expected, "{hand}");
        assert!(!floored, "{hand} must come from the book");
    }
}

/// Direct 3NT under the Multi wants both majors stopped (v4, back from the
/// v2/v3 blind blast that perfect defense priced at −3.7/−4.3 a board); a game
/// hand with a major open doubles and places once they name the suit.
#[test]
fn multi_three_notrump_needs_both_majors_stopped() {
    use super::super::tests::best_call_with;
    let auction = one_nt_two_diamonds();
    let (c, floored) = best_call_with(&multi_arm(), &auction, "K32.Q32.J432.AQ3");
    assert_eq!(c, call(3, Strain::Notrump), "both majors stopped: blast");
    assert!(!floored);
    let (c, _) = best_call_with(&multi_arm(), &auction, "432.Q32.KJ43.AQ3");
    assert_eq!(c, Call::Double, "spades open: double and place later");
    // …and once they name hearts, the heart stopper is enough for 3NT.
    let hearts_confirmed = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        Call::Double,
        call(2, Strain::Hearts),
        Call::Pass,
        Call::Pass,
    ];
    let (c, floored) = best_call_with(&multi_arm(), &hearts_confirmed, "432.Q32.KJ43.AQ3");
    assert_eq!(c, call(3, Strain::Notrump), "hearts named and stopped: 3NT");
    assert!(!floored);
    // Spades named instead: no stopper, no fourth trump — sell out.
    let spades_confirmed = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        Call::Double,
        call(2, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Spades),
    ];
    let (c, floored) = best_call_with(&multi_arm(), &spades_confirmed, "432.Q32.KJ43.AQ3");
    assert_eq!(
        c,
        Call::Pass,
        "spades named and open, three spades: nothing to say (BBA's blind 3NT was PD-refused in v6)"
    );
    assert!(!floored);
}

/// Opener passes the relay's weak sign-off — the seat the first A/B left to
/// the floor, which raised 3♦ to 3NT.
#[test]
fn multi_opener_passes_the_relay_signoff() {
    use super::super::tests::best_call_with;
    let signed_off = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(2, Strain::Notrump),
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, floored) = best_call_with(&multi_arm(), &signed_off, "AKQ5.KQ2.A2.J432");
    assert_eq!(c, Call::Pass, "a maximum still passes the weak 3♦");
    assert!(!floored, "the pass must be a book node");
}

/// `2NT` then `3♦` is natural under the Multi — the rung the natural leg
/// cannot have, since there their `2♦` *is* diamonds.
#[test]
fn multi_relay_then_three_diamonds_is_natural() {
    use super::super::tests::best_call_with;
    let weak_diamonds = "32.432.KQJ765.32";
    let auction = one_nt_two_diamonds();
    let (c, floored) = best_call_with(&multi_arm(), &auction, weak_diamonds);
    assert_eq!(
        c,
        call(2, Strain::Notrump),
        "a weak six-card diamond hand relays"
    );
    assert!(!floored);
    let relayed = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(2, Strain::Notrump),
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    let (c, floored) = best_call_with(&multi_arm(), &relayed, weak_diamonds);
    assert_eq!(c, call(3, Strain::Diamonds), "…and signs off in diamonds");
    assert!(!floored, "the 3♦ sign-off must be a book node");
    // Natural leg: no relay for a diamond hand at all (their suit).
    let (c, _) = bid_transfer(&auction, weak_diamonds);
    assert_ne!(
        c,
        call(2, Strain::Notrump),
        "the natural leg never relays with diamonds"
    );
}

/// Opener sits over their pass of the values double, and over the advancer's
/// pass-or-correct major doubles with four trumps, else waits.
#[test]
fn multi_opener_sits_then_doubles_with_four_trumps() {
    use super::super::tests::best_call_with;
    let sat = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        Call::Double,
        Call::Pass,
    ];
    // v6: BBA's answer with the 3♦ cue replaced by a pass — a four-card
    // major is shown (hearts first), nothing else pulls the double.
    let (c, floored) = best_call_with(&multi_arm(), &sat, "AQ32.AQ32.K3.Q32");
    assert_eq!(c, call(2, Strain::Hearts), "opener shows a four-card major");
    assert!(!floored, "the answer must be a book node");
    let (c, floored) = best_call_with(&multi_arm(), &sat, "AQ3.KQ3.K432.Q32");
    assert_eq!(c, Call::Pass, "no four-card major: sit (BBA cues 3♦)");
    assert!(!floored, "the sit must be a book node");

    let advanced = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        Call::Double,
        call(2, Strain::Hearts),
    ];
    let (c, floored) = best_call_with(&multi_arm(), &advanced, "AQ3.KJ32.K32.Q32");
    assert_eq!(c, Call::Double, "four hearts double the pass-or-correct 2♥");
    assert!(!floored);
    let (c, floored) = best_call_with(&multi_arm(), &advanced, "AQ32.K32.K32.Q32");
    assert_eq!(c, Call::Pass, "three hearts wait for them to resolve");
    assert!(!floored, "the wait is a book node too");
}

/// The values double and opener's penalty double must *read* — an unalerted
/// double is not decoded at all, and the whole point of both is what they
/// tell partner and the floor.
#[test]
fn multi_doubles_read_as_values_and_as_trumps() {
    use crate::bidding::Relative;
    use contract_bridge::Suit;
    use contract_bridge::auction::RelativeVulnerability;
    let system = crate::bidding::american::american(&multi_arm()).bind();

    // `1NT (2♦) X -` from opener's seat: partner showed 8+ and no suit.
    let read = system.infer(
        RelativeVulnerability::NONE,
        &[
            call(1, Strain::Notrump),
            call(2, Strain::Diamonds),
            Call::Double,
            Call::Pass,
        ],
    );
    let partner = read.announced(Relative::Partner);
    assert!(
        partner.strength.hcp.min >= 6,
        "the values double reads as 6+ (BBA's band)"
    );
    assert_eq!(
        partner.length(Suit::Diamonds).min,
        0,
        "…and claims no diamonds"
    );

    // `1NT (2♦) X (2♥) X -` from responder's seat: opener showed four hearts.
    let read = system.infer(
        RelativeVulnerability::NONE,
        &[
            call(1, Strain::Notrump),
            call(2, Strain::Diamonds),
            Call::Double,
            call(2, Strain::Hearts),
            Call::Double,
            Call::Pass,
        ],
    );
    assert!(
        read.announced(Relative::Partner).length(Suit::Hearts).min >= 4,
        "opener's double reads as the four hearts it promised"
    );
}

/// Their double of the forced `3♣` (or of the `2NT` relay) must not strand the
/// sign-off: the first A/B's worst board was responder passing `3♣x` with five
/// diamonds because the doubled suffix was unauthored.
#[test]
fn multi_relay_survives_their_double() {
    use super::super::tests::best_call_with;
    let weak_diamonds = "32.432.KQJ765.32";
    let doubled_completion = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(2, Strain::Notrump),
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Double,
    ];
    let (c, floored) = best_call_with(&multi_arm(), &doubled_completion, weak_diamonds);
    assert_eq!(
        c,
        call(3, Strain::Diamonds),
        "corrects to diamonds over their X of 3♣"
    );
    assert!(!floored, "the doubled sign-off must be a book node");
    let doubled_relay = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(2, Strain::Notrump),
        Call::Double,
    ];
    let (c, floored) = best_call_with(&multi_arm(), &doubled_relay, "AQ5.A432.KQ4.J32");
    assert_eq!(
        c,
        call(3, Strain::Clubs),
        "opener completes over their X of the relay"
    );
    assert!(!floored);
}

/// v3: the double family's continuations — responder's second call after their
/// pass-or-correct resolves, and both sits — are book nodes, because the floor
/// pulled the penalty doubles it could not read (v1/v2 worst boards).
#[test]
fn multi_double_family_continuations_are_book_nodes() {
    use super::super::tests::best_call_with;
    let nt = call(1, Strain::Notrump);
    let d = call(2, Strain::Diamonds);
    let h = call(2, Strain::Hearts);
    let s = call(2, Strain::Spades);
    // Opener sat, they passed 2♥ (hearts confirmed) — v7, BBA's table less
    // the PD-refused rungs: four spades short in hearts is the takeout
    // double; five weak spades bid 2♠; game values with a heart stopper 3NT;
    // everything else (the 8-9 hand, the stopperless 10-count) sells out.
    let hearts_confirmed = [nt, d, Call::Double, h, Call::Pass, Call::Pass];
    let (c, floored) = best_call_with(&multi_arm(), &hearts_confirmed, "K32.QJ32.Q32.432");
    assert_eq!(
        c,
        Call::Pass,
        "8 HCP: sell out (BBA's 2NT invite was PD-refused twice)"
    );
    assert!(!floored);
    let (c, floored) = best_call_with(&multi_arm(), &hearts_confirmed, "KQ32.2.K432.Q432");
    assert_eq!(
        c,
        Call::Double,
        "four spades, one heart: takeout showing spades"
    );
    assert!(!floored);
    let (c, floored) = best_call_with(&multi_arm(), &hearts_confirmed, "KQ32.432.K432.Q4");
    assert_eq!(
        c,
        Call::Pass,
        "four spades with heart length, 9: no try (PD-refused), sell out"
    );
    assert!(!floored);
    let (c, floored) = best_call_with(&multi_arm(), &hearts_confirmed, "KJ432.32.Q432.32");
    assert_eq!(c, call(2, Strain::Spades), "five weak spades: 2♠ to play");
    assert!(!floored);
    let (c, floored) = best_call_with(&multi_arm(), &hearts_confirmed, "K32.Q32.J432.432");
    assert_eq!(c, Call::Pass, "6 HCP flat: sell out");
    assert!(!floored, "the sell-out is a book node too");
    // Opener sat, they corrected to 2♠: four spades and 7+ double (penalty).
    let corrected = [nt, d, Call::Double, h, Call::Pass, s];
    let (c, floored) = best_call_with(&multi_arm(), &corrected, "QJ32.32.K432.K32");
    assert_eq!(c, Call::Double, "four spades over the correction");
    assert!(!floored);
    // Opener doubled 2♥: responder sits.
    let opener_doubled = [nt, d, Call::Double, h, Call::Double, Call::Pass];
    let (c, floored) = best_call_with(&multi_arm(), &opener_doubled, "K32.32.KQ432.432");
    assert_eq!(c, Call::Pass, "responder sits for opener's penalty double");
    assert!(!floored);
    // Responder's takeout double of the confirmed 2♥: opener sits with four
    // hearts, bids the 4-4 spade fit, else a four-card minor, else 2NT.
    let responder_doubled = [
        nt,
        d,
        Call::Double,
        h,
        Call::Pass,
        Call::Pass,
        Call::Double,
        Call::Pass,
    ];
    for (hand, expected, why) in [
        ("AQ3.KJ32.K32.Q32", Call::Pass, "four hearts: sit"),
        ("AKQ32.2.AJ32.K32", call(2, Strain::Spades), "the spade fit"),
        (
            "AQ3.K32.KJ32.Q32",
            call(3, Strain::Diamonds),
            "a four-card minor",
        ),
        (
            "AQ3.K32.KJ3.Q432",
            call(3, Strain::Clubs),
            "clubs when only clubs are four",
        ),
    ] {
        let (c, floored) = best_call_with(&multi_arm(), &responder_doubled, hand);
        assert_eq!(c, expected, "{why}");
        assert!(!floored, "{why}: book node");
    }
    // Opener sits for the penalty double after they ran to spades.
    let ran_doubled = [
        nt,
        d,
        Call::Double,
        h,
        Call::Pass,
        s,
        Call::Double,
        Call::Pass,
    ];
    let (c, floored) = best_call_with(&multi_arm(), &ran_doubled, "AKQ32.2.AJ32.K32");
    assert_eq!(c, Call::Pass, "opener sits for the penalty double of 2♠");
    assert!(!floored);
    // The quantitative 4NT is answered from the book.
    let quant = [
        nt,
        d,
        Call::Double,
        h,
        Call::Pass,
        Call::Pass,
        call(4, Strain::Notrump),
        Call::Pass,
    ];
    let (c, floored) = best_call_with(&multi_arm(), &quant, "AQ5.A32.KQ4.KJ32");
    assert_eq!(c, call(6, Strain::Notrump), "17 opposite 16+: slam");
    assert!(!floored);
    let (c, floored) = best_call_with(&multi_arm(), &quant, "AQ5.A32.KQ4.J432");
    assert_eq!(c, Call::Pass, "15 declines");
    assert!(!floored);
}

/// v3: after the weak relay sign-off, both of us pass whatever they do — their
/// double, their bid over it, their balance.
#[test]
fn multi_relay_signoff_is_fenced_against_their_competition() {
    use super::super::tests::best_call_with;
    let prefix = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(2, Strain::Notrump),
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
        call(3, Strain::Diamonds),
    ];
    let mut over_it = prefix.to_vec();
    over_it.push(call(3, Strain::Spades));
    let (c, floored) = best_call_with(&multi_arm(), &over_it, "AKQ5.KQ2.A2.J432");
    assert_eq!(c, Call::Pass, "opener passes their bid over the sign-off");
    assert!(!floored);
    let mut balanced = prefix.to_vec();
    balanced.extend([Call::Pass, Call::Pass, call(3, Strain::Spades)]);
    let (c, floored) = best_call_with(&multi_arm(), &balanced, "32.432.KQJ765.32");
    assert_eq!(c, Call::Pass, "responder passes their balance");
    assert!(!floored);
}

#[test]
fn multi_stopper_ask_is_confined_to_the_two_corrected_paths() {
    use super::super::tests::best_call_with;
    let ask_hand = "32.Q32.KJ43.A432"; // 10 points, no spade stopper
    for mode in [
        super::MultiStopperAsk::FitSearch,
        super::MultiStopperAsk::OpenerPlaces,
    ] {
        let arm = multi_stopper_arm(mode);
        for auction in [multi_corrected(), multi_corrected_over_double()] {
            let (c, floored) = best_call_with(&arm, &auction, ask_hand);
            assert_eq!(c, call(3, Strain::Spades), "{mode:?} in {auction:?}");
            assert!(!floored, "the artificial ask must be book-owned");
        }

        let non_ran = [
            call(1, Strain::Notrump),
            call(2, Strain::Diamonds),
            Call::Double,
            call(2, Strain::Hearts),
            Call::Pass,
            Call::Pass,
        ];
        let (c, _) = best_call_with(&arm, &non_ran, "32.432.AKJ4.QJ43");
        assert_ne!(c, call(3, Strain::Spades), "the non-ran path has no ask");
    }

    let (c, _) = best_call_with(
        &multi_stopper_arm(super::MultiStopperAsk::Off),
        &multi_corrected(),
        ask_hand,
    );
    assert_ne!(c, call(3, Strain::Spades), "mode Off preserves N4 v7");
}

#[test]
fn multi_stopper_ask_rejects_wrong_strength_stopper_and_four_spades() {
    use super::super::tests::best_call_with;
    let arm = multi_stopper_arm(super::MultiStopperAsk::FitSearch);
    let auction = multi_corrected();
    for (hand, expected, why) in [
        ("32.Q32.J543.K432", Call::Pass, "below 10 points"),
        ("32.AQ2.KJ43.A432", Call::Pass, "above 12 points"),
        (
            "Q32.432.KJ43.A432",
            call(3, Strain::Notrump),
            "a spade stopper",
        ),
        (
            "9876.Q2.AJ43.KQ3",
            Call::Double,
            "four spades keep the penalty double",
        ),
    ] {
        let (c, floored) = best_call_with(&arm, &auction, hand);
        assert_eq!(c, expected, "{why}");
        assert!(!floored, "{why}: the continuation must be book-owned");
    }
}

#[test]
fn multi_stopper_answers_and_places_directly() {
    use super::super::tests::best_call_with;
    let mut asked = multi_corrected();
    asked.extend([call(3, Strain::Spades), Call::Pass]);

    for mode in [
        super::MultiStopperAsk::FitSearch,
        super::MultiStopperAsk::OpenerPlaces,
    ] {
        let arm = multi_stopper_arm(mode);
        let (c, floored) = best_call_with(&arm, &asked, "AQ2.K32.KJ43.Q32");
        assert_eq!(c, call(3, Strain::Notrump), "a stopper answers 3NT");
        assert!(!floored);
    }

    let fit = multi_stopper_arm(super::MultiStopperAsk::FitSearch);
    let place = multi_stopper_arm(super::MultiStopperAsk::OpenerPlaces);
    for (hand, searched, placed) in [
        (
            "32.AQ2.KQ2.AKJ43",
            call(4, Strain::Clubs),
            call(5, Strain::Clubs),
        ),
        (
            "32.AQ2.AKJ43.KQ2",
            call(4, Strain::Diamonds),
            call(5, Strain::Diamonds),
        ),
        (
            "32.AKJ43.AQ2.KQ2",
            call(4, Strain::Hearts),
            call(4, Strain::Hearts),
        ),
        // No four-card side suit: the same deterministic longest-suit
        // partition supplies the finite fallback (equal threes prefer hearts).
        (
            "T987.AQ2.AK2.KQ2",
            call(4, Strain::Hearts),
            call(4, Strain::Hearts),
        ),
    ] {
        let (c, floored) = best_call_with(&fit, &asked, hand);
        assert_eq!(c, searched, "FitSearch: {hand}");
        assert!(!floored);
        let (c, floored) = best_call_with(&place, &asked, hand);
        assert_eq!(c, placed, "OpenerPlaces: {hand}");
        assert!(!floored);
    }
}

#[test]
fn multi_fit_search_finishes_and_fences_every_game() {
    use super::super::tests::best_call_with;
    let arm = multi_stopper_arm(super::MultiStopperAsk::FitSearch);
    let ask = multi_corrected();

    let mut after_clubs = ask.clone();
    after_clubs.extend([
        call(3, Strain::Spades),
        Call::Pass,
        call(4, Strain::Clubs),
        Call::Pass,
    ]);
    for (hand, expected) in [
        ("32.Q32.KJ43.A432", call(5, Strain::Clubs)),
        ("32.432.AKJ43.Q32", call(4, Strain::Diamonds)),
        ("32.AQJ43.KJ3.432", call(4, Strain::Hearts)),
    ] {
        let (c, floored) = best_call_with(&arm, &after_clubs, hand);
        assert_eq!(c, expected, "{hand}");
        assert!(!floored);
    }

    let mut unfinished = after_clubs;
    unfinished.extend([call(4, Strain::Diamonds), Call::Pass]);
    let (c, floored) = best_call_with(&arm, &unfinished, "2.AQ2.KJ43.AQJ43");
    assert_eq!(c, call(5, Strain::Diamonds), "diamond support places 5♦");
    assert!(!floored);
    let (c, floored) = best_call_with(&arm, &unfinished, "32.AQ2.KJ2.AQJ43");
    assert_eq!(
        c,
        call(5, Strain::Clubs),
        "without support opener returns to 5♣"
    );
    assert!(!floored);

    let mut signed = ask;
    signed.extend([
        call(3, Strain::Spades),
        Call::Pass,
        call(4, Strain::Clubs),
        Call::Pass,
        call(5, Strain::Clubs),
        Call::Double,
    ]);
    let (c, floored) = best_call_with(&arm, &signed, "32.AQ2.KJ43.AQ32");
    assert_eq!(c, Call::Pass, "a doubled game is terminal");
    assert!(!floored, "the terminal pass must be book-owned");
}

#[test]
fn multi_stopper_ask_double_rebases_the_whole_search() {
    use super::super::tests::best_call_with;
    let arm = multi_stopper_arm(super::MultiStopperAsk::FitSearch);
    let mut doubled = multi_corrected();
    doubled.extend([call(3, Strain::Spades), Call::Double]);
    let (c, floored) = best_call_with(&arm, &doubled, "32.AQ2.KQ2.AKJ43");
    assert_eq!(c, call(4, Strain::Clubs));
    assert!(!floored);

    doubled.extend([call(4, Strain::Clubs), Call::Pass]);
    let (c, floored) = best_call_with(&arm, &doubled, "32.Q32.KJ43.A432");
    assert_eq!(c, call(5, Strain::Clubs));
    assert!(!floored, "the continuation must also ride the rebase");
}

#[test]
fn multi_stopper_four_spade_raise_uses_the_forcing_pass() {
    use super::super::tests::best_call_with;
    let arm = multi_stopper_arm(super::MultiStopperAsk::OpenerPlaces);
    let mut raised = multi_corrected();
    raised.extend([call(3, Strain::Spades), call(4, Strain::Spades)]);

    for hand in ["AQ2.K32.KJ43.Q32", "T987.AQ2.AK2.KQ2"] {
        let (c, floored) = best_call_with(&arm, &raised, hand);
        assert_eq!(c, Call::Double, "stopper or four spades doubles: {hand}");
        assert!(!floored);
    }
    let (c, floored) = best_call_with(&arm, &raised, "32.AQ2.KQ2.AKJ43");
    assert_eq!(c, Call::Pass, "without either, Pass is forcing");
    assert!(!floored);

    let mut after_pass = raised.clone();
    after_pass.extend([Call::Pass, Call::Pass]);
    let (c, floored) = best_call_with(&arm, &after_pass, "32.Q32.KJ43.A432");
    assert_eq!(c, call(5, Strain::Diamonds), "the longest side suit");
    assert!(!floored);

    let mut after_double = raised;
    after_double.extend([Call::Double, Call::Pass]);
    let (c, floored) = best_call_with(&arm, &after_double, "32.Q32.KJ43.A432");
    assert_eq!(c, Call::Pass, "partner passes the penalty double");
    assert!(!floored);

    after_pass.extend([call(5, Strain::Diamonds), Call::Double]);
    let (c, floored) = best_call_with(&arm, &after_pass, "32.AQ2.KQ2.AKJ43");
    assert_eq!(c, Call::Pass, "the doubled five-level signoff is fenced");
    assert!(!floored);
}
