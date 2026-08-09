use super::super::call;
use super::*;
use crate::bidding::Table;
use crate::bidding::american::american;
use contract_bridge::auction::Auction;
use contract_bridge::{AbsoluteVulnerability, Hand, Seat};

/// Responder's call over `open` -, through the **production** tie-break
///
/// `Table::next_call` sorts descending and takes the first legal call, so
/// equal-weight rules resolve to the *cheapest* bid — the opposite of the
/// `max_by` argmax the module tests elsewhere use.  The 2♥/2♠ collision is
/// only visible on this path.
fn responds(open: Call, hand: &str) -> Call {
    let stance = american(&crate::bidding::agreements::Agreements::current()).against();
    let table = Table::new(
        stance.clone(),
        stance,
        Seat::North,
        AbsoluteVulnerability::NONE,
    );
    let hand: Hand = hand.parse().expect("valid test hand");
    let mut auction = Auction::new();
    auction.push(open);
    auction.push(Call::Pass);
    table.next_call(hand, &auction)
}

/// Board [40] of the kickback phase-5 divergence audit: 6-5 in the majors
/// opposite a weak 2♦ shows **spades**, the longer and higher suit.  The
/// shipped node tied both majors at 1.5 and bid 2♥.
#[test]
fn five_five_majors_respond_in_the_longer_major() {
    assert_eq!(
        responds(call(2, Strain::Diamonds), "AKT862.AKJ92.4.3"),
        call(2, Strain::Spades)
    );
    // Exactly 5-5: the tie goes to the higher rank.
    assert_eq!(
        responds(call(2, Strain::Diamonds), "AKT86.AKJ92.4.32"),
        call(2, Strain::Spades)
    );
    // Longer hearts win over shorter spades.
    assert_eq!(
        responds(call(2, Strain::Diamonds), "AKT86.AKJ952.4.3"),
        call(2, Strain::Hearts)
    );
}

/// Ablating [`set_weak_two_longest_first`] restores the argmax race that
/// board [40] of the kickback audit was bid under — the probe's `tie` arm.
#[test]
fn longest_first_ablation_restores_the_cheaper_major() {
    set_weak_two_longest_first(false);
    let off = responds(call(2, Strain::Diamonds), "AKT862.AKJ92.4.3");
    set_weak_two_longest_first(true);
    assert_eq!(off, call(2, Strain::Hearts));
}

/// The tie-break is doctrine, not a diamond-specific patch: it holds over
/// 2♥ and 2♠ too, where the longer suit may be the *dearer* bid.
#[test]
fn longest_first_holds_over_the_major_weak_twos() {
    // Over 2♠, six hearts beat five clubs even though 3♣ is cheaper.
    assert_eq!(
        responds(call(2, Strain::Spades), "3.AKJ942.4.AKQ82"),
        call(3, Strain::Hearts)
    );
    // Six clubs beat five hearts.
    assert_eq!(
        responds(call(2, Strain::Spades), "3.AKJ92.4.AKQ842"),
        call(3, Strain::Clubs)
    );
}

/// [`set_weak_two_major_priority`] lifts a good five-card major above the
/// Ogust ask over 2♦ — and only there.  Knob-off restores Ogust priority.
#[test]
fn major_priority_outranks_ogust_over_two_diamonds() {
    // 16 HCP, five good spades, two diamonds: the major by default.
    let hand = "AKQ82.KQ2.43.J65";
    assert_eq!(
        responds(call(2, Strain::Diamonds), hand),
        call(2, Strain::Spades)
    );
    // Over 2♠ the ask keeps priority: a new suit there costs the 3 level.
    assert_eq!(
        responds(call(2, Strain::Spades), "43.AKQ82.KQ2.J65"),
        call(2, Strain::Notrump)
    );

    set_weak_two_major_priority(false);
    let off = responds(call(2, Strain::Diamonds), hand);
    set_weak_two_major_priority(true);
    assert_eq!(off, call(2, Strain::Notrump));
}
