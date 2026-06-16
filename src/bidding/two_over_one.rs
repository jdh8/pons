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
//!   (`slam`) below every major-suit trump agreement.
//! - **Competition**: cue-bid (limit-plus) raises, preemptive jump raises,
//!   negative doubles, system-on over their double, support
//!   doubles/redoubles.
//! - **Defense**: overcalls, takeout doubles, 1NT overcall, Michaels and the
//!   unusual 2NT with advances, advancing partner's takeout double, responsive
//!   doubles, defense to 1NT, and defense to weak twos (takeout double, natural
//!   2NT and suit overcalls).
//! - **Instinct floor**: both contested books carry the
//!   [`instinct`][crate::bidding::instinct()] ladder as a root fallback, so
//!   every contested auction gets a sane natural answer — in particular,
//!   partner's takeout double is never passed without a trump stack.
//!
//! Deeper competitive sequences (lebensohl, reopening actions) and minor-suit
//! keycard are left for later authored passes — until then the instinct floor
//! answers those auctions; see the crate changelog.
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

use super::fallback::{Always, Fallback, Guard};
use super::instinct::instinct;
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
pub use defense::{advance_double, defense_to_suit, defense_to_weak_two};
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
    with_instinct_floor(bare_two_over_one())
}

/// The 2/1 pair with the distilled **neural** floor (AI-bidder M1.3)
///
/// Exactly [`two_over_one`] but for the floor: the deterministic
/// [`instinct`][crate::bidding::instinct()] ladder is replaced by the
/// [`NeuralFloor`][crate::bidding::neural_floor::NeuralFloor] safety shell — the learned
/// net in the judgement middle, the forced rails preserved by delegation.  An
/// added option, never a replacement; [`two_over_one`] stays the baseline.  Bind
/// it against the opponents' [`Family`] with [`Pair::against`] and seat it the
/// same way.  Gated behind the `neural-floor` feature.
#[cfg(feature = "neural-floor")]
#[must_use]
pub fn two_over_one_neural() -> Pair {
    with_floor(bare_two_over_one(), super::neural_floor::NeuralFloor)
}

/// The 2/1 pair with the **tag-augmented** distilled neural floor (AI-bidder M5.1)
///
/// Exactly [`two_over_one_neural`] but for the floor's feature extractor: the net
/// also sees the WBF tags of the recent calls
/// ([`features_v2`][crate::bidding::features::features_v2]), wrapped in the same
/// [`NeuralFloorV2`][crate::bidding::neural_floor::NeuralFloorV2] safety shell —
/// the learned net in the judgement middle, the forced rails preserved by
/// delegation.  An added option, never a replacement: [`two_over_one`] stays the
/// baseline and [`two_over_one_neural`] the v1 learned floor.  Bind it against the
/// opponents' [`Family`] with [`Pair::against`] and seat it the same way.  Gated
/// behind the `neural-floor` feature.
#[cfg(feature = "neural-floor")]
#[must_use]
pub fn two_over_one_neural_v2() -> Pair {
    with_floor(bare_two_over_one(), super::neural_floor::NeuralFloorV2)
}

/// The 2/1 pair with the **search-target** distilled neural floor (AI-bidder M3.2)
///
/// Exactly [`two_over_one_neural`] in shape — v1 features, the
/// [`NeuralFloorSearch`][crate::bidding::neural_floor::NeuralFloorSearch] safety
/// shell with the same forced-rail delegation — but the net is distilled from the
/// **live-search teacher** (M2.3's EV-grounded targets, dumped at M3.1), *not* from
/// the deterministic [`two_over_one`] and *not* the live search bidder
/// [`two_over_one_search`] itself.  The fast net that learned the search's
/// judgement.  An added option, never a replacement: [`two_over_one`] stays the
/// baseline and [`two_over_one_neural`] the teacher-distilled floor.  Bind it
/// against the opponents' [`Family`] with [`Pair::against`] and seat it the same
/// way.  Gated behind the `neural-floor` feature.
#[cfg(feature = "neural-floor")]
#[must_use]
pub fn two_over_one_neural_search() -> Pair {
    with_floor(bare_two_over_one(), super::neural_floor::NeuralFloorSearch)
}

/// The 2/1 pair with the gated live-**search** floor (AI-bidder M2.3)
///
/// Exactly [`two_over_one`] but for the floor: the deterministic
/// [`instinct`][crate::bidding::instinct()] ladder is replaced by the
/// [`SearchFloor`][crate::bidding::search_floor::SearchFloor] safety shell, which
/// at each non-forced decision shortlists the distilled net's top calls and
/// scores them by cardplay EV before bidding the best — the policy *thinks*
/// before it bids.  Strong but slow; an added gated option, never a replacement.
/// Bind it against the opponents' [`Family`] with [`Pair::against`] and seat it
/// the same way.  Gated behind the `search` feature (which implies
/// `neural-floor`).
#[cfg(feature = "search")]
#[must_use]
pub fn two_over_one_search() -> Pair {
    two_over_one_search_with(super::search_floor::SearchFloor::default())
}

/// The 2/1 pair with a caller-tuned live-search floor (AI-bidder M3.1)
///
/// Like [`two_over_one_search`] but with explicit
/// [`SearchFloor`][crate::bidding::search_floor::SearchFloor] knobs, so
/// data-generation and tuning runs can trade strength for speed (smaller
/// `layouts`/`shortlist`) without re-wiring the floor.  `two_over_one_search()`
/// is exactly `two_over_one_search_with(SearchFloor::default())`.  Gated behind
/// the `search` feature.
#[cfg(feature = "search")]
#[must_use]
pub fn two_over_one_search_with(floor: super::search_floor::SearchFloor) -> Pair {
    with_floor(bare_two_over_one(), floor)
}

/// Build a 2/1 pair whose **constructive** book falls back to an arbitrary
/// classifier — the knob the standard constructors deny
///
/// [`two_over_one`] and the neural variants hard-wire the deterministic
/// [`instinct`][crate::bidding::instinct()] ladder onto the constructive book —
/// the learned floors own only the contested books.  This
/// builder lifts that wiring so a learned floor (the live
/// [`SearchFloor`][crate::bidding::search_floor::SearchFloor] or the distilled
/// [`NeuralFloorSearch`][crate::bidding::neural_floor::NeuralFloorSearch]) can be
/// *measured* on uncontested constructive auctions against the instinct baseline
/// — the `constructive-abc` A/B/C example.  The contested books are left bare:
/// that harness silences the opponents, so they never resolve.  Gated behind the
/// `neural-floor` feature.
#[cfg(feature = "neural-floor")]
#[must_use]
pub fn two_over_one_constructive_floor<C: Classifier + 'static>(floor: C) -> Pair {
    let mut pair = bare_two_over_one();
    pair.constructive
        .fallback_at(&[], Always, Fallback::classify(floor));
    pair
}

/// Attach any classifier as the floor on a pair's contested books
///
/// A root `Always` fallback on both contested books, shared through the
/// `Fallback`'s `Arc`.  Resolution reaches the root last, so the floor never
/// overrides an authored rule — it only catches the auctions that fall past all
/// of them.  Generic over the floor so [`two_over_one`] (the deterministic
/// [`instinct`][crate::bidding::instinct()]) and
/// [`two_over_one_neural`] (the distilled net) share one wiring.
fn with_floor<C: Classifier + 'static>(mut pair: Pair, floor: C) -> Pair {
    let floor = Fallback::classify(floor);
    pair.competitive.fallback_at(&[], Always, floor.clone());
    pair.defensive.fallback_at(&[], Always, floor);
    // Uncontested auctions never reach the contested floor, so an off-book
    // constructive sequence would pass out below a cold game (e.g. `1♦–1♥–1NT`
    // passed out on a balanced 16 opposite the 12–14 rebid).  Floor the
    // constructive book with the deterministic instinct ladder — the learned
    // floors are trained on contested auctions only, so the natural milestone
    // bidder is the right answer here — and those sequences reach game or slam.
    pair.constructive
        .fallback_at(&[], Always, Fallback::classify(instinct()));
    pair
}

/// Attach the deterministic instinct floor to a pair's contested books
fn with_instinct_floor(pair: Pair) -> Pair {
    with_floor(pair, instinct())
}

/// Build the 2/1 pair *without* the instinct floor: the bare authored books
///
/// This is the ablation handle for measuring the floor.  A driver seating
/// this pair passes whenever the books run out — the pre-floor behavior,
/// including passing partner's takeout double on a worthless hand.
/// [`two_over_one()`] is exactly this pair with
/// [`instinct`][crate::bidding::instinct()] attached to both contested books;
/// see the `instinct-floor` example for an A/B match between the two.
#[must_use]
pub fn bare_two_over_one() -> Pair {
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
