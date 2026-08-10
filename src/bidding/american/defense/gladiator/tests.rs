use super::super::tests::{best_call_with, call};
use crate::bidding::agreements::Agreements;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

fn best_call(auction: &[Call], hand: &str) -> (Call, bool) {
    let mut agreements = Agreements::current();
    agreements.decision.reading.nt_overcall_gladiator = true;
    best_call_with(&agreements, auction, hand)
}

fn systems_on_best_call(auction: &[Call], hand: &str) -> (Call, bool) {
    let mut agreements = Agreements::current();
    agreements.decision.reading.nt_overcall_gladiator = false;
    agreements.decision.reading.nt_overcall_systems_on = true;
    best_call_with(&agreements, auction, hand)
}

#[test]
fn gladiator_club_three_way() {
    // Clubs split three ways by strength: a weak 6+♣ hand transfers via 2NT
    // (overcaller completes 3♣); an invitational 6+♣ hand goes 2♣→2♦→3♣; a
    // game-forcing club hand bids 3♣ directly.  Locks the user's structure.
    let s = || call(1, Strain::Spades);
    let nt = || call(1, Strain::Notrump);
    let p = Call::Pass;
    let c = |n| call(n, Strain::Clubs);
    let d2 = call(2, Strain::Diamonds);
    let (weak, _) = best_call(&[s(), nt(), p], "43.72.852.KJ9876"); // weak 6♣
    let (complete, _) = best_call(
        &[s(), nt(), p, call(2, Strain::Notrump), p],
        "AQ4.KQ4.AK92.65", // overcaller completes the transfer
    );
    let (gf, _) = best_call(&[s(), nt(), p], "A3.K2.42.AKQ9876"); // GF clubs
    let (relay, _) = best_call(&[s(), nt(), p], "43.72.K5.KQ9876"); // INV 6♣, 8 HCP
    let (pull, _) = best_call(&[s(), nt(), p, c(2), p, d2, p], "43.72.K5.KQ9876");
    assert_eq!(weak, call(2, Strain::Notrump), "weak 6♣ transfers via 2NT");
    assert_eq!(complete, c(3), "overcaller completes the club transfer");
    assert_eq!(gf, c(3), "game-forcing clubs bid 3♣ directly");
    assert_eq!(relay, c(2), "invitational 6♣ starts with the 2♣ relay");
    assert_eq!(pull, c(3), "invitational 6♣ pulls to 3♣ over the forced 2♦");
}

#[test]
fn nt_overcall_systems_on_grafts_the_1nt_structure() {
    // Over their (1♦), our 1NT overcall runs systems on: the advancer plays
    // the opening-1NT responses verbatim.  Game-going 4-4 majors bid 2♣
    // Stayman (not a cue of their suit); a five-card spade suit transfers
    // (2♥ → spades), preserving right-siding — the whole point; the
    // overcaller answers Stayman with 4 hearts (2♥) from the grafted table.
    let d = || call(1, Strain::Diamonds);
    let nt = || call(1, Strain::Notrump);
    let (stayman, floored) = systems_on_best_call(
        &[d(), nt(), Call::Pass],
        "A432.KQ84.32.QJ4", // 12 HCP, 4-4 majors
    );
    let (transfer, _) = systems_on_best_call(
        &[d(), nt(), Call::Pass],
        "KQ432.K84.32.QJ4", // 10 HCP, 5 spades — Jacoby transfer, not Stayman
    );
    let (answer, _) = systems_on_best_call(
        &[d(), nt(), Call::Pass, call(2, Strain::Clubs), Call::Pass],
        "Q3.KJ84.AQ54.KQ2", // 17 HCP, 4 hearts, ♦ stopper
    );
    assert_eq!(stayman, call(2, Strain::Clubs), "advancer bids 2♣ Stayman");
    assert!(
        !floored,
        "the grafted Stayman is a book node, not the floor"
    );
    assert_eq!(
        transfer,
        call(2, Strain::Hearts),
        "a five-card spade suit transfers"
    );
    assert_eq!(answer, call(2, Strain::Hearts), "overcaller shows 4 hearts");
}

#[test]
fn gladiator_replaces_the_major_graft() {
    // Over their (1♠), our 1NT overcall runs Gladiator (not systems-on): a
    // hand with exactly 4 hearts + invitational values cues 2♠ (Stayman for
    // the one unbid major); a weak hand takes the 2♣ relay; the overcaller
    // jumps to 4♥ over the cue with a maximum heart fit.
    let s = || call(1, Strain::Spades);
    let nt = || call(1, Strain::Notrump);
    let (cue, floored) = best_call(
        &[s(), nt(), Call::Pass],
        "K84.KQ84.QJ32.42", // 11 HCP, exactly 4 hearts
    );
    let (relay, _) = best_call(
        &[s(), nt(), Call::Pass],
        "432.J8.QJ543.J32", // 5 HCP, weak with 5♦ — the escape relay
    );
    let (flat, _) = best_call(
        &[s(), nt(), Call::Pass],
        "432.J84.J543.J32", // 3 HCP flat, no escape suit — passes 1NT
    );
    let (answer, _) = best_call(
        &[s(), nt(), Call::Pass, call(2, Strain::Spades), Call::Pass],
        "AQ.KQ84.AQ54.J32", // 18 HCP, 4 hearts, ♠ stopper — max fit
    );
    assert_eq!(
        cue,
        call(2, Strain::Spades),
        "advancer cues 2♠ = Stayman for hearts"
    );
    assert!(!floored, "the Gladiator cue is a book node, not the floor");
    assert_eq!(
        relay,
        call(2, Strain::Clubs),
        "a weak hand with a 5-card escape suit bids the 2♣ relay"
    );
    assert_eq!(
        flat,
        Call::Pass,
        "a flat weak hand passes 1NT, not the relay"
    );
    assert_eq!(
        answer,
        call(4, Strain::Hearts),
        "overcaller jumps to 4♥ with a maximum fit"
    );
}

#[test]
fn gladiator_over_2c_steals_the_relay_with_a_double() {
    // (1♠) 1NT (2♣): systems on, but it is Gladiator.  2♣ steals no room, so
    // the now-unbiddable relay reappears as X; every other advance keeps its
    // meaning, and the overcaller answers the stolen relay with the forced 2♦.
    let s = || call(1, Strain::Spades);
    let nt = || call(1, Strain::Notrump);
    let c2 = call(2, Strain::Clubs);
    let p = Call::Pass;
    // The weak 5♦ relay hand now doubles (the stolen relay).
    let (relay_x, _) = best_call(&[s(), nt(), c2], "432.J8.QJ543.J32");
    // Overcaller answers the stolen relay with the forced 2♦, as over 2♣.
    let (forced, _) = best_call(&[s(), nt(), c2, Call::Double, p], "AQ4.KQ4.AK92.65");
    // A cue-Stayman hand keeps cueing 2♠ (2♣ stole only the relay).
    let (cue, cue_floored) = best_call(&[s(), nt(), c2], "K84.KQ84.QJ32.42");
    assert_eq!(
        relay_x,
        Call::Double,
        "the stolen relay is shown with a Double"
    );
    assert_eq!(
        forced,
        call(2, Strain::Diamonds),
        "overcaller answers the stolen relay with the forced 2♦"
    );
    assert_eq!(
        cue,
        call(2, Strain::Spades),
        "the cue-Stayman survives systems-on"
    );
    assert!(
        !cue_floored,
        "the systems-on cue is a book node, not the floor"
    );
}

#[test]
fn gladiator_over_two_level_runs_transfer_lebensohl() {
    // Once RHO takes the two level there is no room for the relay tree, so
    // advancer plays the partnership's Transfer Lebensohl — book nodes, not
    // the floor.  Over (2♦) the 3♣-Stayman leg fires instead.
    let nt = || call(1, Strain::Notrump);
    let p = Call::Pass;
    // (1♠) 1NT (2♥): a weak long-diamond hand relays 2NT (→ 3♣ → correct);
    // the overcaller completes with the forced 3♣.
    let s = call(1, Strain::Spades);
    let h2 = call(2, Strain::Hearts);
    let (relay, relay_floored) = best_call(&[s, nt(), h2], "J2.43.KQ9876.32");
    let (complete, _) = best_call(
        &[s, nt(), h2, call(2, Strain::Notrump), p],
        "AQ4.A4.A32.KQ932",
    );
    // (1♥) 1NT (2♦): a 4-4-majors game-force takes the (2♦) 3♣-Stayman leg.
    let h = call(1, Strain::Hearts);
    let d2 = call(2, Strain::Diamonds);
    let (stayman, stayman_floored) = best_call(&[h, nt(), d2], "AQ32.KJ32.A2.432");
    assert_eq!(relay, call(2, Strain::Notrump), "weak long suit relays 2NT");
    assert!(
        !relay_floored,
        "the Lebensohl relay is a book node, not the floor"
    );
    assert_eq!(complete, call(3, Strain::Clubs), "overcaller completes 3♣");
    assert_eq!(
        stayman,
        call(3, Strain::Clubs),
        "the (2♦) leg bids 3♣-Stayman"
    );
    assert!(
        !stayman_floored,
        "the 3♣-Stayman is a book node, not the floor"
    );
}

#[test]
fn gladiator_continuations_reach_game() {
    // The completed book must drive game-forcing advances to game rather than
    // dying in the floor's partscore.  After `(1♠) 1NT - 2♠ - 3♥`, a
    // game-forcing advancer raises to 4♥.  And a game-forcing natural 3♥
    // (5+ hearts) is raised to 4♥ by the overcaller's heart fit.
    let s = || call(1, Strain::Spades);
    let nt = || call(1, Strain::Notrump);
    let h = |n| call(n, Strain::Hearts);
    let (place, _) = best_call(
        &[
            s(),
            nt(),
            Call::Pass,
            call(2, Strain::Spades),
            Call::Pass,
            h(3),
            Call::Pass,
        ],
        "K84.KQ84.KJ32.42", // 12 HCP, 4 hearts, GF — over a min fit, bid game
    );
    let (raise, floored) = best_call(
        &[s(), nt(), Call::Pass, h(3), Call::Pass],
        "AQ2.KQ8.AQ54.K93", // 18 HCP, 3 hearts — raise the GF 3♥ to game
    );
    assert_eq!(place, h(4), "GF advancer raises the min fit to 4♥");
    assert!(
        !floored,
        "the overcaller's raise is a book node, not the floor"
    );
    assert_eq!(raise, h(4), "overcaller raises the game-forcing 3♥ to 4♥");
}

#[test]
fn gladiator_delayed_cue_finds_the_five_three_fit() {
    // A (1♠) 1NT overcall may hold a balanced 5-card heart suit.  An advancer
    // with exactly 3 hearts, INV+, and a doubleton (NOT flat 4333, so it has
    // ruffing value) routes 2♣ relay → forced 2♦ → 2♠ (delayed cue) to check
    // the 5-3 fit the direct cue (promising 4) would miss; the overcaller with
    // 5 hearts and a maximum jumps to 4♥.
    let s = || call(1, Strain::Spades);
    let nt = || call(1, Strain::Notrump);
    let c = || call(2, Strain::Clubs);
    let d = || call(2, Strain::Diamonds);
    let (cue, floored) = best_call(
        &[s(), nt(), Call::Pass, c(), Call::Pass, d(), Call::Pass],
        "84.KJ8.KQ32.QJ32", // 12 HCP, exactly 3 hearts, doubleton ♠ — not 4333
    );
    let (answer, _) = best_call(
        &[
            s(),
            nt(),
            Call::Pass,
            c(),
            Call::Pass,
            d(),
            Call::Pass,
            call(2, Strain::Spades),
            Call::Pass,
        ],
        "AQ2.KQ842.AK5.32", // 18 HCP, 5 hearts, ♠ stopper — max fit
    );
    assert_eq!(
        cue,
        call(2, Strain::Spades),
        "exactly-3-heart non-flat advancer delayed-cues 2♠"
    );
    assert!(!floored, "the delayed cue is a book node, not the floor");
    assert_eq!(
        answer,
        call(4, Strain::Hearts),
        "overcaller with 5 hearts + a maximum jumps to 4♥"
    );
}

#[test]
fn gladiator_cues_barred_with_flat_4333() {
    // The 4333 curse: a flat (4333) has no ruffing value, so neither cue is
    // made — it invites/plays notrump instead of chasing a major fit.
    let s = || call(1, Strain::Spades);
    let nt = || call(1, Strain::Notrump);
    // Direct cue barred: flat 4333 with exactly 4 hearts, GF → 3NT, not 2♠.
    let (direct, _) = best_call(
        &[s(), nt(), Call::Pass],
        "K84.KQ84.K84.Q84", // 13 HCP, 3-4-3-3 flat, 4 hearts
    );
    // Delayed cue barred: flat 4333 with exactly 3 hearts, INV → 2NT relay-invite.
    let (delayed, _) = best_call(
        &[
            s(),
            nt(),
            Call::Pass,
            call(2, Strain::Clubs),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
        ],
        "J843.KJ8.Q84.Q84", // 9 HCP, 4-3-3-3 flat, 3 hearts
    );
    assert_eq!(
        direct,
        call(3, Strain::Notrump),
        "flat 4333 with 4 hearts bids 3NT, not the direct cue"
    );
    assert_eq!(
        delayed,
        call(2, Strain::Notrump),
        "flat 4333 with 3 hearts invites 2NT, not the delayed cue"
    );
}
