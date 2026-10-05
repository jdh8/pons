//! System-independent build helpers shared across bidding systems
//!
//! Call/suit helpers, guarded seat fan-out, and floor-attachment wiring that
//! have nothing to do with any one system.
//! [`american`][super::american] — and any future system — imports these from
//! here rather than carrying them itself.

use super::agreements::{Agreements, OpeningKnobs, RebidKnobs, TheirDisclosures};
use super::fallback::{Always, Fallback, Guard};
use super::features::{CompactConfig, Config};
use super::instinct::instinct;
use super::neural_floor::{ConfiguredFloorBba, ConfiguredFloorV6};
use super::{Rules, System, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};
use std::sync::Arc;

/// A bid as a [`Call`], for trie keys
pub(in crate::bidding) const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// The other major
pub(in crate::bidding) const fn other_major(major: Suit) -> Suit {
    match major {
        Suit::Hearts => Suit::Spades,
        _ => Suit::Hearts,
    }
}

/// The other minor
pub(in crate::bidding) const fn other_minor(minor: Suit) -> Suit {
    match minor {
        Suit::Clubs => Suit::Diamonds,
        _ => Suit::Clubs,
    }
}

/// Attach a guarded fallback at `suffix` under every leading-pass prefix
// ponytail: `guard`/`fallback` stay by-value — callers pass a freshly built
// `Arc::new(ConcreteGuard)`, which unsize-coerces to `Arc<dyn Guard>` only on
// the move; a `&Arc<dyn Guard>` param would force a `let` binding at all ~20
// call sites for no real gain.
#[allow(clippy::needless_pass_by_value)]
pub(in crate::bidding) fn fallback_all_seats(
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
// Floor attachment
// ---------------------------------------------------------------------------

/// Attach one ladder and one contested floor to a system's three books
///
/// A root `Always` fallback on both contested books, shared through the
/// `Fallback`'s `Arc`.  Resolution reaches the root last, so the floor never
/// overrides an authored rule — it only catches the auctions that fall past all
/// of them.
///
/// Uncontested auctions never reach the contested floor, so an off-book
/// constructive sequence would pass out below a cold game (e.g. `1♦ - 1♥ - 1NT`
/// passed out on a balanced 16 opposite the 12–14 rebid).  The constructive
/// book therefore gets `ladder` — the natural milestone bidder reaches game or
/// slam on those sequences.
///
/// `ladder` is a parameter rather than a fresh [`instinct()`] because the
/// contested floor needs the *same* one: [`ConfiguredFloorBba`] delegates forced
/// situations to it, and [`instinct()`] itself reads the pinned profile at
/// build time (its RKCB fields pick the kickback ladder over the plain one).
/// Two independent builds could disagree; one `Arc` cannot.
fn with_floors(mut system: System, ladder: &Arc<Rules>, contested: Fallback) -> System {
    system
        .competitive
        .fallback_at(&[], Always, contested.clone());
    system.defensive.fallback_at(&[], Always, contested);
    system
        .constructive
        .fallback_at(&[], Always, Fallback::Classify(ladder.clone()));
    system
}

/// Attach the card-input (v4) BBA-distilled floor to a system's contested books
///
/// This remains the explicit card-input v4 entry point behind
/// [`american_with_config`][super::american::american_with_config].
pub(in crate::bidding) fn with_floor(
    system: System,
    config: Config,
    agreements: &Agreements,
) -> System {
    let ladder = Arc::new(instinct(agreements));
    let contested = Fallback::classify(ConfiguredFloorBba::new(config, Arc::clone(&ladder)));
    with_floors(system, &ladder, contested)
}

/// Attach the shipped compact-config v6 floor and honest evaluator twin.
pub(in crate::bidding) fn with_floor_v6(
    system: System,
    compact: CompactConfig,
    agreements: &Agreements,
) -> System {
    let ladder = Arc::new(instinct(agreements));
    let contested = Fallback::classify(ConfiguredFloorV6::new(compact, Arc::clone(&ladder)));
    with_floors(system, &ladder, contested)
}

/// Attach the v8 floor (v6 plus the artificial block).
pub(in crate::bidding) fn with_floor_v8(
    system: System,
    compact: CompactConfig,
    agreements: &Agreements,
) -> System {
    let ladder = Arc::new(instinct(agreements));
    let contested = Fallback::classify(ConfiguredFloorV6::new_v8(compact, Arc::clone(&ladder)));
    with_floors(system, &ladder, contested)
}

/// Attach a logit mean over run-time v6 blobs (the floor sweep's candidate).
pub(in crate::bidding) fn with_floor_mean(
    system: System,
    compact: CompactConfig,
    agreements: &Agreements,
    blobs: Arc<[Vec<f32>]>,
) -> System {
    let ladder = Arc::new(instinct(agreements));
    let contested = Fallback::classify(ConfiguredFloorV6::new_mean(
        compact,
        Arc::clone(&ladder),
        blobs,
    ));
    with_floors(system, &ladder, contested)
}

/// Attach the v6 twin retrained on BBA's disclosed readings.
pub(in crate::bidding) fn with_floor_v6_their(
    system: System,
    compact: CompactConfig,
    agreements: &Agreements,
) -> System {
    let ladder = Arc::new(instinct(agreements));
    let contested = Fallback::classify(ConfiguredFloorV6::new_their(compact, Arc::clone(&ladder)));
    with_floors(system, &ladder, contested)
}

/// Attach the deterministic instinct floor to a system's contested books
///
/// The fully-disclosable reference wiring: one ladder on all three books.
pub(in crate::bidding) fn with_instinct_floor(system: System, agreements: &Agreements) -> System {
    let ladder = Arc::new(instinct(agreements));
    let contested = Fallback::Classify(ladder.clone());
    with_floors(system, &ladder, contested)
}

/// The agreements a [mirror book][crate::bidding::book::System::opponents] is
/// built from — ours with the opponents' disclosures cleared and our opt-in
/// Watermelon overlays reset and the `2♦`-waiting structure (with its looser
/// positives and the tree below them) pinned off
///
/// [`None`] when nothing is declared, no overlay is on and those three are off,
/// in which case the mirror would be a second copy of the same books and is
/// not built at all.
pub(in crate::bidding) fn mirror_agreements(agreements: &Agreements) -> Option<Agreements> {
    let mut mirror = *agreements;
    mirror.decision.their = TheirDisclosures::default();
    // Our opt-in Watermelon overlays are ours alone: an undeclared opponent
    // plays the house structure, so their natural `1♣ - 1M - 2♦`, `2♦` and
    // `1♣ - 1♦` must not decode as Odwrotka, the Multi, or the wide-1♣ relay.
    let (opening, rebid) = (OpeningKnobs::default(), RebidKnobs::default());
    mirror.opening.five_five_four_two = opening.five_five_four_two;
    mirror.opening.wide_one_club = opening.wide_one_club;
    mirror.opening.multi_two_diamonds = opening.multi_two_diamonds;
    mirror.rebid.odwrotka = rebid.odwrotka;
    // Likewise our light 11-count majors: their `1M` keeps the sound reading.
    mirror.opening.eleven_count_majors = opening.eleven_count_majors;
    mirror.opening.ten_count_majors = opening.ten_count_majors;
    // Our `2♦`-waiting structure over 2♣ is pinned *off* in the mirror: an
    // undeclared opponent's `2♣ - 2♦` decodes as the double-negative
    // structure, so the knob moves only boards we open.  Measured
    // (`scripts/ab-strong-two-waiting.sh`): decoding their 2♣ as waiting too
    // costs ~0.002 IMPs/board under the v6 floor, though BBA does wait with
    // 2♦.  Since the knob ships on, this builds a mirror on every default
    // build; deleting the line is the reversal.
    mirror.decision.strong_two_waiting = false;
    // Likewise the tree after a natural positive and the looser positive
    // itself: their `2♣ - 2NT - 3♠` keeps the floor's reading and their
    // `2♣ - 2♠` the two-honor one, so the knobs move only boards we open.
    mirror.rebid.strong_two_positive = false;
    mirror.rebid.strong_two_grand = false;
    mirror.rebid.strong_two_positive_notrump = false;
    mirror.response.strong_two_loose_positive = false;
    // Likewise the slam try through a game try: their `1M - 2M - 3x` stays a
    // 16–18 try and their `4NT` any 22+ hand, so the knob moves only boards
    // we open.
    mirror.response.major_raise_slam_try = false;
    // Likewise opener's rebid over the preemptive minor raise: their
    // `1m - 3m - 3NT` and pass keep the floor's reading.
    mirror.response.preemptive_minor_raise_pass = None;
    // Likewise the 2/1 suit order: their `1♠ - 2m` keeps the weight ladder's
    // reading.
    mirror.response.two_over_one_hearts_first = false;
    // Likewise the minor before spades over `1♥`: their `1♥ - 1♠` and
    // `1♥ - 2m - R - 2♠` keep the floor's reading.
    mirror.response.two_over_one_minor_before_spades = false;
    // Likewise responder's six-card invite after the forcing `1NT`: their
    // `1M - 1NT - 2x - 3y` keeps the floor's reading.
    mirror.rebid.forcing_notrump_suit_invite = false;
    // Likewise opener's answer to the Modern double: their `1m (1M) X -`
    // answers keep the floor's reading.
    mirror.competition.modern_double_answer = false;
    mirror.competition.modern_double_both_majors = false;
    mirror.competition.weak_new_suit_rebid = false;
    mirror.competition.weak_new_suit_extras = false;
    mirror.competition.weak_new_suit_length = 5;
    mirror.competition.doubled_notrump_pass = false;
    mirror.competition.doubled_notrump_max_length = 13;
    (mirror != *agreements).then_some(mirror)
}
