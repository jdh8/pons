//! Integration tests for the authored Puppet Stayman (1NT–3♣) and the
//! minor-suit transfers (1NT–2NT diamonds, 1NT–2♠ clubs/invite): the 2♣-vs-3♣
//! carve, the 5-3 major hunt, the 3♦-deny Smolen 4-4 hunt, the diamond
//! pass-or-correct, and the two-way 2♠ (max/min reply, signoff, game splinter).

use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain};
use pons::american;
use pons::bidding::array::Logits;
use pons::bidding::{Family, Stance, System};

const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

fn stance() -> Stance {
    american().against(Family::NATURAL)
}

/// The single highest-logit call the system assigns the hand for the auction
fn best_call(system: &impl System, auction: &[Call], hand: &str) -> Call {
    let hand: Hand = hand.parse().expect("valid test hand");
    let logits: Logits = system
        .classify(hand, RelativeVulnerability::NONE, auction)
        .expect("system covers this auction");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty")
}

const P: Call = Call::Pass;

/// `1NT P` plus the given tail of our-side calls (RHO passes interleaved)
fn after_1nt(tail: &[Call]) -> Vec<Call> {
    let mut auction = vec![call(1, Strain::Notrump), P];
    for &c in tail {
        auction.push(c);
        auction.push(P);
    }
    auction
}

// --- Choosing 2♣ vs 3♣ ------------------------------------------------------

#[test]
fn flat_four_three_three_three_game_force_bids_3nt() {
    let system = stance();
    // Flat 4-3-3-3 (four spades), 11 HCP: no Puppet, no Stayman — a flat hand has
    // no ruffing value, so it plays 3NT rather than hunt a major fit.
    assert_eq!(
        best_call(&system, &after_1nt(&[]), "KJ54.Q32.K43.Q92"),
        call(3, Strain::Notrump),
    );
}

#[test]
fn balanced_three_card_major_game_force_still_puppets() {
    let system = stance();
    // 4♠-3♥-4♦-2♣ (a club doubleton), 11 HCP: a *non-flat* balanced game force
    // still Puppets (3♣, outranking Stayman) — only the flat 4-3-3-3 is diverted
    // to 3NT.
    assert_eq!(
        best_call(&system, &after_1nt(&[]), "KJ54.Q32.K432.Q9"),
        call(3, Strain::Clubs),
    );
}

#[test]
fn four_four_game_force_uses_stayman() {
    let system = stance();
    // 4-4 in the majors: plain Stayman (no three-card major to Puppet with).
    assert_eq!(
        best_call(&system, &after_1nt(&[]), "KJ54.KQ32.43.Q92"),
        call(2, Strain::Clubs),
    );
}

#[test]
fn flat_four_three_three_three_eight_passes() {
    let system = stance();
    // A flat 4-3-3-3, bare 8: it neither Staymans (no ruff — plays 3NT, not the 4-4
    // fit) nor invites.  The flat shape is its high cards and nothing more, so it
    // plays a level too high opposite a 15-17; a double-dummy probe scores passing
    // over the 2♠ size ask at +0.64 IMPs/board (`examples/probe-uninvite-4333`).
    assert_eq!(
        best_call(&system, &after_1nt(&[]), "KJ54.Q32.J43.J92"),
        Call::Pass,
    );
}

#[test]
fn flat_minor_four_three_three_three_game_force_bids_3nt() {
    let system = stance();
    // 3-3 majors, four clubs, balanced 11: a flat 4-3-3-3 — no Puppet to hunt a
    // five-card major, it just plays 3NT.
    assert_eq!(
        best_call(&system, &after_1nt(&[]), "K32.Q43.KJ4.Q932"),
        call(3, Strain::Notrump),
    );
}

// --- Puppet: opener's answer and the 5-3 fit --------------------------------

#[test]
fn opener_shows_a_five_card_major_over_puppet() {
    let system = stance();
    let auction = after_1nt(&[call(3, Strain::Clubs)]);
    // 17 HCP balanced with five hearts: 3♥.
    assert_eq!(
        best_call(&system, &auction, "Kx.AQJ32.Kxx.Axx"),
        call(3, Strain::Hearts),
    );
}

#[test]
fn responder_raises_the_five_three_fit_to_game() {
    let system = stance();
    let auction = after_1nt(&[call(3, Strain::Clubs), call(3, Strain::Hearts)]);
    // Three-card heart support opposite opener's five: an eight-card fit, 4♥.
    assert_eq!(
        best_call(&system, &auction, "Kxx.Qxx.KJxx.Qx"),
        call(4, Strain::Hearts),
    );
}

// --- Puppet: opener denies (3♦) and the Smolen 4-4 hunt ---------------------

#[test]
fn opener_denies_a_five_card_major_with_three_diamonds() {
    let system = stance();
    let auction = after_1nt(&[call(3, Strain::Clubs)]);
    // 3-3-4-3, no five-card major: deny with the artificial 3♦.
    assert_eq!(
        best_call(&system, &auction, "KQx.Kxx.KQxx.Axx"),
        call(3, Strain::Diamonds),
    );
}

#[test]
fn responder_bids_the_short_major_to_find_a_four_four() {
    let system = stance();
    let auction = after_1nt(&[call(3, Strain::Clubs), call(3, Strain::Diamonds)]);
    // 4♠-3♥: bid the shorter major (3♥) to show the four spades, Smolen-style.
    assert_eq!(
        best_call(&system, &auction, "KJxx.Qxx.Kxx.Qxx"),
        call(3, Strain::Hearts),
    );
}

#[test]
fn opener_completes_the_four_four_spade_fit() {
    let system = stance();
    // …3♣ 3♦ 3♥ named four spades; opener with four raises to 4♠.
    let auction = after_1nt(&[
        call(3, Strain::Clubs),
        call(3, Strain::Diamonds),
        call(3, Strain::Hearts),
    ]);
    assert_eq!(
        best_call(&system, &auction, "AQxx.Kxx.KQx.Axx"),
        call(4, Strain::Spades),
    );
}

#[test]
fn responder_signs_off_in_3nt_without_a_four_card_major() {
    let system = stance();
    let auction = after_1nt(&[call(3, Strain::Clubs), call(3, Strain::Diamonds)]);
    // 3-3 majors, no four-card major: nothing to find, settle in 3NT.
    assert_eq!(
        best_call(&system, &auction, "Kxx.Qxx.KJx.Qxxx"),
        call(3, Strain::Notrump),
    );
}

// --- Diamond transfer (1NT–2NT) ---------------------------------------------

#[test]
fn opener_completes_the_diamond_transfer_with_a_fit() {
    let system = stance();
    let auction = after_1nt(&[call(2, Strain::Notrump)]);
    // Three diamonds: complete to 3♦ (an assured eight-card fit).
    assert_eq!(
        best_call(&system, &auction, "Axx.Kxx.Qxx.AKxx"),
        call(3, Strain::Diamonds),
    );
}

#[test]
fn opener_pass_or_corrects_the_diamond_transfer_when_short() {
    let system = stance();
    let auction = after_1nt(&[call(2, Strain::Notrump)]);
    // Doubleton diamond: bid 3♣ instead, pass-or-correct.
    assert_eq!(
        best_call(&system, &auction, "AKxx.KQxx.xx.Axx"),
        call(3, Strain::Clubs),
    );
}

#[test]
fn weak_diamond_transfer_passes_the_partscore() {
    let system = stance();
    let auction = after_1nt(&[call(2, Strain::Notrump), call(3, Strain::Diamonds)]);
    // Six diamonds, sub-game values: pass the 3♦ partscore.
    assert_eq!(best_call(&system, &auction, "xx.xx.KJxxxx.xxx"), P);
}

// --- Two-way 2♠ (clubs or balanced invite) ----------------------------------

#[test]
fn opener_shows_strength_over_two_spades() {
    let system = stance();
    let auction = after_1nt(&[call(2, Strain::Spades)]);
    // Maximum (17): 3♣.
    assert_eq!(
        best_call(&system, &auction, "AQx.KJx.KQx.Axxx"),
        call(3, Strain::Clubs),
    );
    // Minimum (15): 2NT.
    assert_eq!(
        best_call(&system, &auction, "KQx.KJx.Qxx.Axxx"),
        call(2, Strain::Notrump),
    );
}

#[test]
fn weak_clubs_signs_off_over_either_reply() {
    let system = stance();
    // Over the minimum 2NT: correct to 3♣.
    let after_min = after_1nt(&[call(2, Strain::Spades), call(2, Strain::Notrump)]);
    assert_eq!(
        best_call(&system, &after_min, "xx.xx.xxx.KQxxxx"),
        call(3, Strain::Clubs),
    );
    // Over the maximum 3♣: pass.
    let after_max = after_1nt(&[call(2, Strain::Spades), call(3, Strain::Clubs)]);
    assert_eq!(best_call(&system, &after_max, "xx.xx.xxx.KQxxxx"), P);
}

#[test]
fn balanced_invite_plays_2nt_opposite_a_minimum() {
    let system = stance();
    let auction = after_1nt(&[call(2, Strain::Spades), call(2, Strain::Notrump)]);
    // Balanced 8, no four-card major: opener is minimum, settle in 2NT.
    assert_eq!(best_call(&system, &auction, "Kxx.Qxx.Qxx.Jxxx"), P);
}

#[test]
fn game_going_clubs_splinter_for_the_better_game() {
    let system = stance();
    let auction = after_1nt(&[call(2, Strain::Spades), call(3, Strain::Clubs)]);
    // Six clubs, game values, a singleton spade: splinter 3♠ so opener picks
    // between 3NT and 5♣.
    assert_eq!(
        best_call(&system, &auction, "x.Kxx.Kxx.AQxxxx"),
        call(3, Strain::Spades),
    );
}

// --- Smolen reachability: a game-forcing 5-4 keeps off the transfer ---------

#[test]
fn game_forcing_five_four_takes_stayman_not_a_transfer() {
    let system = stance();
    // 5♠-4♥, 14 HCP: kept off the spade transfer so it can Smolen via 2♣.
    assert_eq!(
        best_call(&system, &after_1nt(&[]), "AQJxx.KJxx.Kx.xx"),
        call(2, Strain::Clubs),
    );
}

#[test]
fn five_four_smolens_over_the_stayman_denial() {
    let system = stance();
    // 1NT 2♣ 2♦ (no major): jump 3♥ to show five spades and four hearts.
    let auction = after_1nt(&[call(2, Strain::Clubs), call(2, Strain::Diamonds)]);
    assert_eq!(
        best_call(&system, &auction, "AQJxx.KJxx.Kx.xx"),
        call(3, Strain::Hearts),
    );
}
