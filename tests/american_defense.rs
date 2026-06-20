//! Integration tests for two-suited overcalls, their advances, and responsive doubles
//! in the 2/1 defensive book

use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain};
use pons::american;
use pons::bidding::american::set_responsive_overcall;
use pons::bidding::array::Logits;
use pons::bidding::{Family, Stance, System};

const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// The 2/1 pair bound against natural opponents
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

// --- Michaels cue-bid -------------------------------------------------------

/// (1♦) with 5-5 majors → Michaels 2♦
#[test]
fn test_michaels_over_minor() {
    let system = stance();
    // 11 HCP, five spades and five hearts over their 1♦
    assert_eq!(
        best_call(&system, &[call(1, Strain::Diamonds)], "KQJ54.AJ965.2.92"),
        call(2, Strain::Diamonds),
    );
}

// --- Unusual 2NT ------------------------------------------------------------

/// (1♠) with 5-5 minors → Unusual 2NT
#[test]
fn test_unusual_2nt_over_spades() {
    let system = stance();
    // 11 HCP, five diamonds and five clubs over their 1♠
    assert_eq!(
        best_call(&system, &[call(1, Strain::Spades)], "2.95.KQJ54.AJ965"),
        call(2, Strain::Notrump),
    );
}

// --- Michaels over major ----------------------------------------------------

/// (1♥) with spades + clubs → Michaels 2♥
#[test]
fn test_michaels_over_heart() {
    let system = stance();
    // 11 HCP, five spades and five clubs over their 1♥
    assert_eq!(
        best_call(&system, &[call(1, Strain::Hearts)], "KQJ54.2.95.AJ965"),
        call(2, Strain::Hearts),
    );
}

// --- Responsive double ------------------------------------------------------

/// (1♥) – X – (2♥) with 4-4 minors → responsive double
#[test]
fn test_responsive_double() {
    let system = stance();
    // 11 HCP, four-four in clubs and diamonds; partner made takeout double
    assert_eq!(
        best_call(
            &system,
            &[
                call(1, Strain::Hearts),
                Call::Double,
                call(2, Strain::Hearts),
            ],
            "KQ5.32.K964.QJ92"
        ),
        Call::Double,
    );
}

/// (1♥) – 2♣ – (2♥) with 4-4 in the unbid suits → overcall responsive double,
/// but only when the opt-in toggle is on (off by default → floored, not a double)
#[test]
fn test_responsive_overcall_double_toggle() {
    // 10 HCP, four spades and four diamonds (the two suits unbid by opener and the
    // 2♣ overcaller); partner overcalled 2♣, they raised to 2♥.
    let auction = [
        call(1, Strain::Hearts),
        call(2, Strain::Clubs),
        call(2, Strain::Hearts),
    ];
    let hand = "KQ54.32.KQ54.932";

    set_responsive_overcall(true);
    assert_eq!(best_call(&stance(), &auction, hand), Call::Double);

    set_responsive_overcall(false);
    assert_ne!(best_call(&stance(), &auction, hand), Call::Double);
}

// --- Unusual 2NT advance ----------------------------------------------------

/// (1♠) – 2NT – (P) with diamonds longer than clubs → 3♦
#[test]
fn test_unusual_nt_advance_longer_diamond() {
    let system = stance();
    // 7 HCP, three diamonds and two clubs — prefer the longer suit
    assert_eq!(
        best_call(
            &system,
            &[
                call(1, Strain::Spades),
                call(2, Strain::Notrump),
                Call::Pass,
            ],
            "Q432.Q765.K54.92"
        ),
        call(3, Strain::Diamonds),
    );
}

// --- Regression: single five-card suit still overcalls naturally ------------

/// (1♣) with only one five-card suit → 1♠, not a two-suited bid
#[test]
fn test_regression_single_suit_overcall() {
    let system = stance();
    // 9 HCP, five spades only — should still overcall 1♠, not Michaels
    assert_eq!(
        best_call(&system, &[call(1, Strain::Clubs)], "AQJ32.853.Q42.92"),
        call(1, Strain::Spades),
    );
}

// --- Defense to a weak two --------------------------------------------------

/// (2♠) with opening values and short spades → takeout double
#[test]
fn test_weak_two_takeout_double() {
    let system = stance();
    // 14 HCP, a spade doubleton — the workhorse takeout double
    assert_eq!(
        best_call(&system, &[call(2, Strain::Spades)], "32.AKJ5.KJ54.Q92"),
        Call::Double,
    );
}

/// (2♠) with 15–18 balanced and a stopper → natural 2NT overcall
#[test]
fn test_weak_two_notrump_overcall() {
    let system = stance();
    // 17 HCP, 3=3=4=3 with a spade stopper
    assert_eq!(
        best_call(&system, &[call(2, Strain::Spades)], "KQ5.AQ5.KJ54.Q92"),
        call(2, Strain::Notrump),
    );
}

/// (2♠) with a five-card minor and modest values → natural 3♣ overcall
#[test]
fn test_weak_two_suit_overcall() {
    let system = stance();
    // 11 HCP, five clubs — overcall at the cheapest level, a rung up
    assert_eq!(
        best_call(&system, &[call(2, Strain::Spades)], "432.32.K54.AKJ54"),
        call(3, Strain::Clubs),
    );
}

// --- Advancing a takeout double of a weak two -------------------------------

/// (2♠) – X – (P) with a weak spade stack → pass for penalty
#[test]
fn test_advance_double_penalty_pass() {
    let system = stance();
    // 6 HCP, KQJ9x of spades sitting over the weak two and nothing else — convert
    // for penalty. The default Transfer Lebensohl keeps this weak penalty pass;
    // only stopper-plus-game-values hands push on to 3NT (see below).
    assert_eq!(
        best_call(
            &system,
            &[call(2, Strain::Spades), Call::Double, Call::Pass],
            "KQJ95.J32.432.32"
        ),
        Call::Pass,
    );
}

/// (2♠) – X – (P) with a stopper and game values → 3NT
#[test]
fn test_advance_double_three_notrump() {
    let system = stance();
    // 14 HCP, balanced with a spade stopper and no four-card major
    assert_eq!(
        best_call(
            &system,
            &[call(2, Strain::Spades), Call::Double, Call::Pass],
            "A32.K32.KQ54.Q92"
        ),
        call(3, Strain::Notrump),
    );
}

/// (2♠) – X – (P) with four hearts and game values → 3♠ cue (Stayman)
#[test]
fn test_advance_double_major_cue() {
    let system = stance();
    // 14 HCP, four hearts opposite the takeout double — the default Transfer
    // Lebensohl bids the 3♠ cue (Stayman) to find the heart fit, rather than
    // jumping blind to 4♥.
    assert_eq!(
        best_call(
            &system,
            &[call(2, Strain::Spades), Call::Double, Call::Pass],
            "A32.KQ54.K32.Q92"
        ),
        call(3, Strain::Spades),
    );
}

/// Recognition (default, no `set_delayed_cue`): a partner who plays the delayed
/// cue — (2♠)–X–(P)–2NT–(P)–3♣–(P)–3♠ = Stayman with a spade stopper — is answered
/// even though the bot never bids it itself. With four hearts the answerer shows
/// the fit (4♥, since 3♥ is below the 3♠ cue).
#[test]
fn test_recognize_delayed_cue_major_fit() {
    let system = stance();
    let auction = [
        call(2, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
    ];
    // Doubler with four hearts opposite the delayed cue → 4♥ (the fit).
    assert_eq!(
        best_call(&system, &auction, "32.KQ54.AK32.Q92"),
        call(4, Strain::Hearts),
    );
    // No four-card major → 3NT (partner promised the spade stopper).
    assert_eq!(
        best_call(&system, &auction, "K2.KJ2.AQ32.KJ32"),
        call(3, Strain::Notrump),
    );
}

/// The same advance machinery answers over a one-level opening: (1♦) – X – (P)
/// with a weak five-card major → cheapest-level natural advance 1♠
#[test]
fn test_advance_double_over_one_bid() {
    let system = stance();
    // 6 HCP, five spades — pick the major at the one level
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Diamonds), Call::Double, Call::Pass],
            "KQJ54.432.32.432"
        ),
        call(1, Strain::Spades),
    );
}
