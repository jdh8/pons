//! A 2/1 game-forcing bidding system
//!
//! [`two_over_one()`][crate::bidding::two_over_one::two_over_one] assembles a
//! [`Pair`] for the Two-over-One Game Forcing system, the modern North
//! American standard: five-card majors, a strong 15–17 notrump, the strong
//! artificial 2♣, and — the defining feature — a new suit at the two level in
//! response to a one-of-a-major opening is **game forcing**.
//!
//! The system is authored entirely from the constraint vocabulary
//! ([`constraint`][crate::bidding::constraint]), the [`Rules`] classifier, and
//! the role-aware books — the strictly uncontested core in a [`Constructive`]
//! book, [`competition()`][crate::bidding::two_over_one::competition] over our
//! openings in a [`Competitive`][super::Competitive] book, and our actions
//! over their openings in a [`Defensive`][super::Defensive] book; nothing here
//! is system infrastructure.
//!
//! # Conventions
//!
//! - **Openings**: 15–17 1NT, 20–21 2NT, strong artificial 2♣ (22+),
//!   five-card majors (light in 3rd/4th seat), better minor, weak twos,
//!   three-level preempts.
//! - **Responses**: 2/1 game forces with full continuations to game and the
//!   slam-try level, forcing 1NT (with the three-card limit raise rebid),
//!   Jacoby 2NT with shortness/second-suit rebids, splinters, inverted
//!   minors, weak jump shifts.
//! - **The 2♣ structure**: 2♦ waiting, 2♥ double negative, natural positives;
//!   notrump rebids carry the 2NT machinery ("system on").
//! - **Notrump structures**: Stayman and Jacoby transfers at the two and
//!   three levels, quantitative 4NT at every notrump strength.
//! - **Weak twos**: Ogust 2NT, RONF raises, forcing new suits.
//! - **Slam**: RKCB 1430 with the 5NT king ask
//!   ([`slam`]) below every major-suit trump agreement.
//! - **Competition**: cue-bid (limit-plus) raises, preemptive jump raises,
//!   negative doubles, system-on over their double, support
//!   doubles/redoubles.
//! - **Defense**: overcalls, takeout doubles, 1NT overcall, Michaels and the
//!   unusual 2NT with advances, responsive doubles, defense to 1NT.
//!
//! Deeper competitive sequences (lebensohl, reopening actions) and minor-suit
//! keycard are left for later passes — see the crate changelog.
//!
//! # Forcing by omission
//!
//! There is no "forcing" flag.  A bid is forcing when the *next* node for our
//! side carries no [`Pass`][Call::Pass] rule, so passing scores
//! [`f32::NEG_INFINITY`].  Responders keep a pass below their action threshold;
//! opener-rebid nodes after a response omit it entirely.
//!
//! # Weights
//!
//! Within one decision node the highest-weighted *satisfied* call wins (a
//! satisfied crisp constraint contributes `0`, so the logit is its weight).
//! Constraints are kept disjoint where practical; where calls can both apply,
//! the weights order them so the more descriptive bid wins.

use super::fallback::{Fallback, Guard};
use super::trie::Classifier;
use super::{Constructive, Family, Pair, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain};
use std::sync::Arc;

mod competition;
mod defense;
mod game_force;
mod notrump;
mod openings;
mod raises;
mod rebids;
mod responses;
mod slam;
mod strong_two;
mod weak_twos;

pub use competition::competition;
pub use defense::defense_to_suit;
pub use notrump::notrump_responses;
pub use openings::openings;
pub use responses::{major_responses, minor_responses};

/// A bid as a [`Call`], for trie keys
const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

// ---------------------------------------------------------------------------
// Seat-fan helpers
// ---------------------------------------------------------------------------

/// Insert one classifier at `suffix` under every leading-pass prefix
///
/// For each `n` in `0..=max_passes` the classifier is keyed at `[P; n] ++
/// suffix`, sharing one [`Arc`] across all of them (pointer-cheap, see
/// [`insert_arc`][super::Trie::insert_arc]).  This authors a table once and
/// makes it answer in every seat that could have reached it.
fn insert_all_seats(
    book: &mut Trie,
    suffix: &[Call],
    max_passes: usize,
    rules: impl Classifier + 'static,
) {
    let shared: Arc<dyn Classifier> = Arc::new(rules);
    for n in 0..=max_passes {
        let key: Vec<Call> = core::iter::repeat_n(Call::Pass, n)
            .chain(suffix.iter().copied())
            .collect();
        book.insert_arc(&key, Arc::clone(&shared));
    }
}

/// Interleave one opposing pass after each of our calls
///
/// The constructive book keys the *raw table auction*, so an undisturbed
/// sequence of our calls `[1♥, 1♠]` lives at `[1♥, P, 1♠, P]` (plus leading
/// passes for the opener's seat).  This is the one place that spells out the
/// interleaving; author keys through it, never by hand.
fn uncontested(our_calls: &[Call]) -> Vec<Call> {
    our_calls
        .iter()
        .flat_map(|&call| [call, Call::Pass])
        .collect()
}

/// Insert a continuation table after our undisturbed `our_calls`, every seat
///
/// Keys at `uncontested(our_calls)` under every leading-pass prefix
/// (`0..=3`), so the table answers regardless of which seat opened.  An empty
/// `our_calls` registers an opening table.
fn insert_uncontested(book: &mut Trie, our_calls: &[Call], rules: impl Classifier + 'static) {
    insert_all_seats(book, &uncontested(our_calls), 3, rules);
}

/// Attach a guarded fallback at `suffix` under every leading-pass prefix
fn fallback_all_seats(
    book: &mut Trie,
    suffix: &[Call],
    max_passes: usize,
    guard: Arc<dyn Guard>,
    fallback: Fallback,
) {
    for n in 0..=max_passes {
        let key: Vec<Call> = core::iter::repeat_n(Call::Pass, n)
            .chain(suffix.iter().copied())
            .collect();
        book.fallback_arc_at(&key, Arc::clone(&guard), fallback.clone());
    }
}

// ---------------------------------------------------------------------------
// Assembly
// ---------------------------------------------------------------------------

/// Build the basic 2/1 game-forcing system as one side's [`Pair`]
///
/// Bind it against the opponents' [`Family`] for a playable system, and seat
/// two pairs with [`Table::of_pairs`][super::Table::of_pairs] for a full
/// table.
///
/// ```
/// use pons::two_over_one;
/// use pons::bidding::{Family, System};
/// use contract_bridge::auction::{Call, RelativeVulnerability};
/// use contract_bridge::{Bid, Strain};
///
/// let stance = two_over_one().against(Family::NATURAL);
/// let hand = "AQ32.K53.QJ4.A92".parse().unwrap(); // 16 HCP, balanced
/// let logits = stance
///     .classify(hand, RelativeVulnerability::NONE, &[])
///     .expect("an opening decision");
/// let best = (&logits.0)
///     .into_iter()
///     .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
///     .map(|(call, _)| call)
///     .unwrap();
/// assert_eq!(best, Call::Bid(Bid::new(1, Strain::Notrump)));
/// ```
#[must_use]
pub fn two_over_one() -> Pair {
    let mut c = Constructive::new();

    openings::register(&mut c);
    responses::register(&mut c);
    notrump::register(&mut c);
    rebids::register(&mut c);
    game_force::register(&mut c);
    raises::register(&mut c);
    strong_two::register(&mut c);
    weak_twos::register(&mut c);

    Pair::new(
        Family::NATURAL,
        c,
        competition::competition(),
        defense::defensive(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::Rules;
    use crate::bidding::context::Context;
    use contract_bridge::auction::RelativeVulnerability;
    use contract_bridge::{Hand, Suit};

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
    fn notrump_responses_transfer_and_stayman() {
        let r = notrump_responses();
        let a = [call(1, Strain::Notrump), Call::Pass];
        assert_eq!(best(&r, &a, "KJ542.Q32.K43.92"), call(2, Strain::Hearts));
        assert_eq!(best(&r, &a, "KJ54.Q32.K43.Q92"), call(2, Strain::Clubs));
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
}
