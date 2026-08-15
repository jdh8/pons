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
        "8+ with nothing better doubles under the Multi"
    );
    assert!(!floored, "the values double must be a book node");
    // 7 HCP flat: nothing to say.
    let (c, _) = best_call_with(&multi_arm(), &auction, "Q32.K32.432.Q432");
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
    assert_eq!(c, Call::Pass, "spades named and open: nothing to say");
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
    // A maximum with both majors: the floor would pull to game; the book waits.
    let (c, floored) = best_call_with(&multi_arm(), &sat, "AQ32.AQ32.K3.Q32");
    assert_eq!(c, Call::Pass, "opener sits for the values double");
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
        partner.strength.hcp.min >= 8,
        "the values double reads as 8+"
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
    // Opener sat, they passed 2♥ (hearts confirmed): four hearts double, three pass.
    let hearts_confirmed = [nt, d, Call::Double, h, Call::Pass, Call::Pass];
    let (c, floored) = best_call_with(&multi_arm(), &hearts_confirmed, "K32.QJ32.K32.432");
    assert_eq!(c, Call::Double, "four hearts: penalty");
    assert!(!floored);
    let (c, floored) = best_call_with(&multi_arm(), &hearts_confirmed, "K32.Q32.K432.432");
    assert_eq!(
        c,
        Call::Pass,
        "three hearts, 8 HCP: sell out (v5's 2NT invite was refuted on PD)"
    );
    assert!(!floored, "the sell-out is a book node too");
    // Opener sat, they corrected to 2♠: four spades double.
    let corrected = [nt, d, Call::Double, h, Call::Pass, s];
    let (c, floored) = best_call_with(&multi_arm(), &corrected, "QJ32.32.K432.K32");
    assert_eq!(c, Call::Double, "four spades over the correction");
    assert!(!floored);
    // Opener doubled 2♥: responder sits.
    let opener_doubled = [nt, d, Call::Double, h, Call::Double, Call::Pass];
    let (c, floored) = best_call_with(&multi_arm(), &opener_doubled, "K32.32.KQ432.432");
    assert_eq!(c, Call::Pass, "responder sits for opener's penalty double");
    assert!(!floored);
    // Responder doubled the confirmed 2♥: opener sits, even with a fit elsewhere.
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
    let (c, floored) = best_call_with(&multi_arm(), &responder_doubled, "AKQ32.2.AJ32.K32");
    assert_eq!(c, Call::Pass, "opener sits for responder's penalty double");
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
