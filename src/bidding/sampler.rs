//! Constrained layout sampling — the inverse of [`Inferences`]
//!
//! [`Inferences`] reads an auction *forward* into per-player shown ranges (suit
//! lengths and points).  This module runs that backward: given the player to
//! act, their actual hand, and those ranges, it deals the *other three* hands at
//! random so each falls within everything the calls have shown.
//!
//! Because every range starts at [`Envelope::unknown`] and only ever narrows
//! soundly (`Range::intersect`'s soundness-over-tightness), a hand that truly
//! made these calls always lands inside its range.  The sampled layouts are
//! therefore a sound population of "full deals this auction could have come
//! from" — the substrate a double-dummy search scores each candidate call over
//! (AI-bidder M2.1, the prerequisite for M2.2's call-EV evaluator).
//!
//! # Method
//!
//! Rejection sampling on top of [`fill_deals`][contract_bridge::deck::fill_deals]:
//! the actor's known thirteen cards are pinned into a partial deal, so every
//! draw deals only the other thirty-nine; a draw is kept iff LHO, partner, and
//! RHO each land within their shown ranges, and discarded otherwise.  This is
//! correct by construction — an accepted layout satisfies every range by the
//! acceptance test itself — and reuses the battle-tested dealer rather than
//! reinventing constrained shuffling.
//!
//! The shown ranges are deliberately loose, so acceptance is workable; a tight
//! or jointly-infeasible auction is bounded by an attempt cap (see
//! [`sample_layouts`]) and may return fewer layouts than requested rather than
//! loop forever.  A smarter importance sampler can replace the rejection loop
//! later if EV throughput demands it; the signature would not change.

use super::System;
use super::inference::{Inferences, Relative, relative_of};
use contract_bridge::auction::{Auction, Call, RelativeVulnerability};
use contract_bridge::deck::fill_deals;
use contract_bridge::{Builder, Card, FullDeal, Hand, Seat};
use rand::Rng;
use rand::seq::SliceRandom;

/// Random deals tried per requested layout before giving up
///
/// Rejection sampling needs roughly `1 / acceptance` draws per kept layout, so
/// the total budget is `n * MAX_ATTEMPTS_PER_LAYOUT`.  The cap exists only to
/// terminate auctions whose ranges no hand can satisfy.
///
/// It was 256, which the ranges *do* approach: `probe-replay-yield` measured
/// 59–93 % fills on ordinary auctions (`(1NT) X`, `1H (2C)`, a 2/1 sequence),
/// each exhausting the whole budget — the shortfall was the cap, not
/// infeasibility.  A short fill is the expensive failure: `ev_all` then averages
/// over fewer, edge-biased worlds.  A draw is ~0.24 µs (same probe), so even a
/// fully-spent 128-layout budget costs ~125 ms against the double-dummy solve
/// each kept layout pays — look harder rather than loosen the envelope
/// ([`Inferences`] soundness).
const MAX_ATTEMPTS_PER_LAYOUT: usize = 4096;

/// Random splits tried per requested defender world, before topping up hard-only
///
/// The mid-play sampler ([`sample_defender_remnants`]) keeps the old, tighter
/// budget: it runs at every declarer turn of every single-dummy playout, and a
/// starved draw there degrades gracefully (the hard masks still hold) rather
/// than shortening the population.
const DEFENDER_ATTEMPTS_PER_WORLD: usize = 256;

/// Total random deals the *replay* sampler will draw for one request — a generous
/// ceiling (~10-20 s, in tempo for a human bid), since a deal is a ~0.3 µs shuffle
/// and the accept test a few classifies, both far below the double-dummy solve
/// each *accepted* layout then pays.  Look as hard as it takes rather than fall
/// back to the unfaithful ranges.
///
/// This is only a backstop: [`REPLAY_DRY_LIMIT`] governs termination in practice,
/// so a feasible auction stops when it fills and an infeasible one bails early.
const REPLAY_DRAW_CAP: usize = 50_000_000;

/// Consecutive rejected draws after which the replay sampler gives up on the
/// current request — the auction is *feasibility*-limited, not budget-limited
/// (e.g. a penalty double needs the doubler to hold 15+, impossible when the
/// actor is strong), so more draws will not help and the caller tops up with the
/// ranges.  Resets on every accept, so it never cuts short an auction yielding
/// more than roughly `1 / REPLAY_DRY_LIMIT`.
const REPLAY_DRY_LIMIT: usize = 1 << 20;

/// How far below its best legal call the policy may rank a player's actual call
/// and still accept the hand, in nats (the replay sampler's relaxation knob).
///
/// Strict argmax (`0.0`) over-tightens — every committal call becomes an
/// independent hurdle and the rejection loop starves.  This margin accepts
/// near-ties, the population the loose range readers approximated.  Tuned for
/// sampler yield; see the plan.
const MARGIN: f32 = 3.0;

/// Deal up to `n` full layouts consistent with what an auction has shown
///
/// `hand` is the actor's own thirteen cards and `seat` their absolute seat;
/// both are held fixed while the other three hands are dealt at random so that
/// LHO, partner, and RHO each fall within their [`Inferences`] ranges (which are
/// relative to `seat`, the side to act).  `rng` is the caller's — the model
/// never samples, so the learned floor stays deterministic (invariant §0.5).
///
/// Returns at most `n` layouts.  Fewer (possibly none) means the attempt budget
/// of `n * 4096` draws ran out first, which happens only when the shown ranges
/// are tight or jointly infeasible given `hand`; a caller should read a short
/// result as a weak or absent signal, not an error.
#[must_use]
pub fn sample_layouts(
    hand: Hand,
    seat: Seat,
    inferences: &Inferences,
    rng: &mut impl Rng,
    n: usize,
) -> Vec<FullDeal> {
    let budget = n.saturating_mul(MAX_ATTEMPTS_PER_LAYOUT);
    sample_with(hand, seat, rng, n, budget, usize::MAX, |deal| {
        within_ranges(deal, seat, inferences)
    })
}

/// Deal up to `n` layouts, accepting each by *replaying the rule* on top of the
/// [`Inferences`] ranges (gated by
/// [`set_rule_accept`][super::inference::set_rule_accept]).
///
/// A hand is kept iff it (a) falls within `inferences` — the old range reading,
/// which covers every call — *and* (b) at every **authored** node a non-actor
/// player bid ([`System::authored_at`]), `policy` re-run on the candidate ranks
/// the made call within a margin of its best legal call.  Replay only tightens
/// where a rule answers; a bid the keyless floor handled (a competitive
/// raise/rebid with no authored node) is left to the range reading alone.  `vul`
/// is relative to `seat` (the actor): partner shares it, the opponents see it
/// side-swapped.
///
/// Short-result semantics match [`sample_layouts`], but with a far larger draw
/// budget: replay is tight, and looking harder is cheap next to the double-dummy
/// solve each accepted layout pays.
#[must_use]
// Each argument is a distinct fact of the decision, as in [`ev_all`].
#[allow(clippy::too_many_arguments)]
pub fn sample_layouts_replay(
    hand: Hand,
    seat: Seat,
    policy: &dyn System,
    vul: RelativeVulnerability,
    auction: &[Call],
    inferences: &Inferences,
    rng: &mut impl Rng,
    n: usize,
) -> Vec<FullDeal> {
    sample_with(
        hand,
        seat,
        rng,
        n,
        REPLAY_DRAW_CAP,
        REPLAY_DRY_LIMIT,
        |deal| {
            within_ranges(deal, seat, inferences) && rules_accept(deal, seat, policy, vul, auction)
        },
    )
}

/// Rejection-sample up to `n` layouts whose other three hands pass `accept`,
/// drawing at most `budget` random deals and giving up early after `dry_limit`
/// consecutive rejects (pass `usize::MAX` to disable the early-out).
///
/// The actor's thirteen cards are pinned, so each draw deals only the other
/// thirty-nine.
// ponytail: the `build_partial` expect cannot fire — one hand placed in an
// otherwise empty builder is always a valid partial deal.
fn sample_with(
    hand: Hand,
    seat: Seat,
    rng: &mut impl Rng,
    n: usize,
    budget: usize,
    dry_limit: usize,
    accept: impl Fn(&FullDeal) -> bool,
) -> Vec<FullDeal> {
    let mut out = Vec::with_capacity(n);
    if n == 0 {
        return out;
    }

    let mut builder = Builder::new();
    builder[seat] = hand;
    let partial = builder
        .build_partial()
        .expect("one thirteen-card hand is a valid partial deal");

    let mut dry = 0usize;
    for deal in fill_deals(rng, partial).take(budget) {
        if accept(&deal) {
            out.push(deal);
            if out.len() == n {
                break;
            }
            dry = 0;
        } else {
            dry += 1;
            if dry >= dry_limit {
                break;
            }
        }
    }
    out
}

/// Whether LHO, partner, and RHO in `deal` each fall within their shown ranges
///
/// The actor's own hand was pinned, so it is consistent by construction and is
/// not re-checked.
fn within_ranges(deal: &FullDeal, seat: Seat, inferences: &Inferences) -> bool {
    [
        (seat.lho(), Relative::Lho),
        (seat.partner(), Relative::Partner),
        (seat.rho(), Relative::Rho),
    ]
    .into_iter()
    .all(|(other, who)| inferences.admits(who, deal[other]))
}

/// Whether LHO, partner, and RHO in `deal` could each have made their actual
/// calls under `policy` (the rule-replay accept test; see
/// [`sample_layouts_replay`]).
fn rules_accept(
    deal: &FullDeal,
    seat: Seat,
    policy: &dyn System,
    vul: RelativeVulnerability,
    auction: &[Call],
) -> bool {
    let len = auction.len();
    let theirs = swap_sides(vul);
    [
        (seat.lho(), Relative::Lho, theirs),
        (seat.partner(), Relative::Partner, vul),
        (seat.rho(), Relative::Rho, theirs),
    ]
    .into_iter()
    .all(|(other, who, pvul)| {
        let hand = deal[other];
        // This player's own call indices, deepest first — the tightest node
        // rejects fastest, short-circuiting the rest.
        (0..len)
            .rev()
            .filter(|&i| relative_of(len, i) == who)
            .all(|i| made_plausibly(hand, policy, pvul, &auction[..i], auction[i]))
    })
}

/// Whether `policy`, classifying `hand` at `prefix`, ranks the `made` call
/// within [`MARGIN`] of its best legal call.  A call no rule authors (nothing
/// to replay) abstains so the range reader handles it, and an off-book node
/// has no opinion; both accept.  A **pass** replays like any call — the
/// negative inference the interval ranges cannot express: a candidate whose
/// best alternative beats the pass is rejected (hard where the pass gate is
/// `-∞`, e.g. a 12-count at the opening node; soft within [`MARGIN`] of
/// weight-close alternatives such as a preempt).  A candidate the node
/// rejects wholesale accepts (`-∞ ≥ -∞ − MARGIN`) — the floor-pass worlds
/// stay in.
fn made_plausibly(
    hand: Hand,
    policy: &dyn System,
    vul: RelativeVulnerability,
    prefix: &[Call],
    made: Call,
) -> bool {
    if !policy.authored_at(vul, prefix) {
        return true;
    }
    let Some(logits) = policy.classify(hand, vul, prefix) else {
        return true;
    };
    // Best over *legal* calls only — a fallback book may offer a call now illegal
    // at this node, which must not inflate the argmax the made call is judged
    // against (the made call is legal by construction).
    let mut played = Auction::new();
    played
        .try_extend(prefix.iter().copied())
        .expect("a prior table auction is legal");
    let best = logits
        .0
        .iter()
        .filter(|(call, _)| played.can_push(*call).is_ok())
        .fold(f32::NEG_INFINITY, |best, (_, &logit)| best.max(logit));
    *logits.0.get(made) >= best - MARGIN
}

/// Deal exactly `n` mid-play defender worlds consistent with declarer's view
///
/// The single-dummy playout ([`single_dummy_playout`][crate::single_dummy_playout])
/// asks, at each of declarer's turns: how might the cards declarer *cannot*
/// see — `pool`, both defenders' unplayed cards — lie?  Each world splits
/// `pool` between the two defenders at their remaining hand sizes (derived
/// from `lho_played`/`rho_played`: a defender's remnant is thirteen minus
/// what they have played), subject to two layers of constraint:
///
/// - **Hard** — `lho_may`/`rho_may`, the cards each defender can still hold
///   (a defender who showed out of a suit holds none of it; declarer saw
///   that).  Satisfied *constructively*: cards only one defender may hold are
///   forced there and the rest split at random, so every world respects the
///   masks — a violating world would be an impossible layout (and a revoke
///   waiting to happen at the solver's door).
/// - **Soft** — the shown ranges of `inferences`, read from **declarer's**
///   perspective and applied to each defender's reconstructed *original*
///   hand (remnant ∪ played).  Rejection-sampled; when the reading is too
///   tight to fill within the attempt budget, the remainder is topped up
///   hard-only (a weak signal, not an error — the playout must price its
///   candidates over *some* population, and the lead scorer tops up the same
///   way), so this function always returns exactly `n` worlds.
///
/// # Panics
///
/// Panics if the inputs do not reconstruct two thirteen-card defenders
/// (`pool.len() + lho_played.len() + rho_played.len() == 26` with `pool`
/// disjoint from both), or if the masks make the true layout impossible
/// (a pool card neither defender may hold, or more forced cards than a
/// remnant has room for) — the caller tracks the play, so either is a
/// bookkeeping bug.
#[must_use]
// Each argument is a distinct fact of the position, as in `sample_layouts_replay`.
#[allow(clippy::too_many_arguments)]
pub fn sample_defender_remnants(
    pool: Hand,
    lho_played: Hand,
    rho_played: Hand,
    lho_may: Hand,
    rho_may: Hand,
    inferences: &Inferences,
    rng: &mut impl Rng,
    n: usize,
) -> Vec<(Hand, Hand)> {
    assert!(
        pool & (lho_played | rho_played) == Hand::EMPTY
            && pool.len() + lho_played.len() + rho_played.len() == 26,
        "pool and played cards must reconstruct two thirteen-card defenders"
    );
    // Cards only one defender may hold are forced; the rest split at random.
    let lho_forced = pool - rho_may;
    let rho_forced = pool - lho_may;
    let lho_len = 13 - lho_played.len();
    assert!(
        lho_forced & rho_forced == Hand::EMPTY
            && lho_forced.len() <= lho_len
            && rho_forced.len() <= pool.len() - lho_len,
        "hard masks must admit the true layout"
    );
    let mut free: Vec<Card> = (pool - lho_forced - rho_forced).into_iter().collect();
    let lho_free = lho_len - lho_forced.len();
    let mut split = move |rng: &mut _| {
        let (drawn, _) = free.partial_shuffle(rng, lho_free);
        let lho = lho_forced | drawn.iter().copied().collect();
        (lho, pool - lho)
    };

    let mut out = Vec::with_capacity(n);
    for _ in 0..n.saturating_mul(DEFENDER_ATTEMPTS_PER_WORLD) {
        if out.len() == n {
            break;
        }
        let (lho, rho) = split(rng);
        if inferences.admits(Relative::Lho, lho | lho_played)
            && inferences.admits(Relative::Rho, rho | rho_played)
        {
            out.push((lho, rho));
        }
    }
    // Tight or jointly-infeasible reading: top up hard-only.
    while out.len() < n {
        out.push(split(rng));
    }
    out
}

/// Vulnerability seen from the opposing side: swap the WE and THEY bits.
fn swap_sides(vul: RelativeVulnerability) -> RelativeVulnerability {
    let mut out = RelativeVulnerability::NONE;
    out.set(
        RelativeVulnerability::WE,
        vul.contains(RelativeVulnerability::THEY),
    );
    out.set(
        RelativeVulnerability::THEY,
        vul.contains(RelativeVulnerability::WE),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::constraint::point_count;
    use crate::bidding::context::Context;
    use contract_bridge::auction::{Call, RelativeVulnerability};
    use contract_bridge::deck::full_deal;
    use contract_bridge::{Bid, Level, Strain, Suit};
    use proptest::prelude::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    const fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid {
            level: Level::new(level),
            strain,
        })
    }

    /// Inferences relative to the side to act, read from an auction
    fn inferences(auction: &[Call]) -> Inferences {
        Inferences::read(&Context::new(RelativeVulnerability::NONE, auction))
    }

    /// The natural penalty double of their 1NT shows 15+, and a passed doubler's
    /// double (both majors) is left unnarrowed — the floor must read the two apart.
    #[test]
    fn reads_natural_penalty_double_of_their_notrump() {
        // (1NT) X by an unpassed seat — RHO of the side to act (the 1NT responder).
        let direct = inferences(&[bid(1, Strain::Notrump), Call::Double]);
        assert_eq!(direct.rho().points.min, 15);

        // A passed hand doubling: dealer passes, RHO opens 1NT, two passes, then the
        // dealer (now a passed hand) doubles — both majors, not a 15+ penalty double.
        let passed = inferences(&[
            Call::Pass,
            bid(1, Strain::Notrump),
            Call::Pass,
            Call::Pass,
            Call::Double,
        ]);
        assert!(passed.rho().points.min < 15);
    }

    /// The latch's subsequent penalty double reads as four-plus in the doubled
    /// suit, so partner reads it as penalty (and leaves it in) instead of takeout.
    #[test]
    fn reads_latched_penalty_double_of_the_runout() {
        use crate::bidding::instinct::set_penalty_latch;
        // (1NT) X (2♦) X (P): our penalty X, their runout, partner's penalty double.
        let auction = [
            bid(1, Strain::Notrump),
            Call::Double,
            bid(2, Strain::Diamonds),
            Call::Double,
            Call::Pass,
        ];
        // Off: the later double reads as nothing — no length shown.
        set_penalty_latch(false);
        assert_eq!(inferences(&auction).partner().length(Suit::Diamonds).min, 0);
        // On (the default): the latch's double promises four-plus diamonds (the stack).
        set_penalty_latch(true);
        assert_eq!(inferences(&auction).partner().length(Suit::Diamonds).min, 4);
    }

    /// A fixed hand short in hearts, so an RHO who must hold 5+ hearts is easy
    /// to satisfy and the sampler reaches its requested count quickly.
    fn short_heart_actor() -> Hand {
        "AKQ32.32.AKQ2.32".parse().expect("valid test hand")
    }

    /// Soundness: every sampled layout keeps the actor's hand fixed and places
    /// the other three within their shown ranges.  Holds vacuously when the
    /// draw is infeasible, so it is robust to a hand that crowds out a range.
    #[test]
    fn sampled_layouts_respect_ranges() {
        let actor = Seat::North;
        // RHO opened 1H (5+ hearts, 12-21); LHO and partner are unconstrained.
        let inf = inferences(&[bid(1, Strain::Hearts)]);

        proptest!(|(seed in any::<u64>())| {
            let mut rng = StdRng::seed_from_u64(seed);
            let hand = full_deal(&mut rng)[actor];
            for deal in sample_layouts(hand, actor, &inf, &mut rng, 4) {
                prop_assert_eq!(deal[actor], hand);
                for (other, shown) in [
                    (actor.lho(), inf.lho()),
                    (actor.partner(), inf.partner()),
                    (actor.rho(), inf.rho()),
                ] {
                    for suit in Suit::ASC {
                        #[allow(clippy::cast_possible_truncation)]
                        let length = deal[other][suit].len() as u8;
                        prop_assert!(shown.length(suit).contains(length));
                    }
                    prop_assert!(shown.points.contains(point_count(deal[other])));
                }
            }
        });
    }

    /// A richer auction whose constraints land on more than one player.
    #[test]
    fn respects_a_developed_auction() {
        let actor = Seat::North;
        // Partner opened 1H, then RHO overcalled 1S (5+ spades, 8+).  Inferences
        // reads partner's opening and RHO's overcall; we sample around them.
        let auction = [bid(1, Strain::Hearts), bid(1, Strain::Spades)];
        let inf = inferences(&auction);
        let mut rng = StdRng::seed_from_u64(7);
        let layouts = sample_layouts(short_heart_actor(), actor, &inf, &mut rng, 16);

        assert!(!layouts.is_empty(), "the auction is feasible");
        for deal in &layouts {
            let partner = deal[actor.partner()];
            assert!(partner[Suit::Hearts].len() >= 5);
            assert!(inf.partner().points.contains(point_count(partner)));
            let rho = deal[actor.rho()];
            assert!(rho[Suit::Spades].len() >= 5);
            assert!(inf.rho().points.contains(point_count(rho)));
        }
    }

    /// The opener's shown shape and strength are honored on every layout.
    #[test]
    fn opener_constraint_is_enforced() {
        let actor = Seat::North;
        let inf = inferences(&[bid(1, Strain::Hearts)]);
        let mut rng = StdRng::seed_from_u64(1);
        let layouts = sample_layouts(short_heart_actor(), actor, &inf, &mut rng, 32);

        assert_eq!(layouts.len(), 32, "a 1H opening is easy to satisfy");
        for deal in &layouts {
            let opener = deal[actor.rho()];
            assert!(opener[Suit::Hearts].len() >= 5);
            assert!((10..=21).contains(&point_count(opener)));
        }
    }

    /// Coverage: the dealt population is not degenerate — both a constrained and
    /// an unconstrained seat take a spread of shapes across samples.
    #[test]
    fn coverage_is_not_degenerate() {
        let actor = Seat::North;
        let inf = inferences(&[bid(1, Strain::Hearts)]);
        let mut rng = StdRng::seed_from_u64(99);
        let layouts = sample_layouts(short_heart_actor(), actor, &inf, &mut rng, 40);

        // RHO's heart length (constrained to 5+) still varies; LHO is free.
        let rho_hearts: std::collections::HashSet<usize> = layouts
            .iter()
            .map(|deal| deal[actor.rho()][Suit::Hearts].len())
            .collect();
        let lho_spades: std::collections::HashSet<usize> = layouts
            .iter()
            .map(|deal| deal[actor.lho()][Suit::Spades].len())
            .collect();
        assert!(rho_hearts.len() >= 2, "constrained seat should still vary");
        assert!(lho_spades.len() >= 3, "free seat should vary widely");
    }

    /// An infeasible auction terminates within the budget and returns nothing,
    /// rather than looping forever.
    #[test]
    fn infeasible_auction_returns_empty() {
        let actor = Seat::North;
        // RHO opened 1H, demanding 5+ hearts, but the actor holds nine of them,
        // leaving only four in the deck — no layout can satisfy the opening.
        let inf = inferences(&[bid(1, Strain::Hearts)]);
        let hoard: Hand = "32.AKQJT9876.2.2".parse().expect("valid test hand");
        assert_eq!(hoard[Suit::Hearts].len(), 9);
        let mut rng = StdRng::seed_from_u64(5);

        let layouts = sample_layouts(hoard, actor, &inf, &mut rng, 5);
        assert!(layouts.is_empty());
    }

    /// Requesting zero layouts samples nothing.
    #[test]
    fn zero_request_is_empty() {
        let actor = Seat::North;
        let inf = inferences(&[bid(1, Strain::Hearts)]);
        let mut rng = StdRng::seed_from_u64(0);
        assert!(sample_layouts(short_heart_actor(), actor, &inf, &mut rng, 0).is_empty());
    }

    /// Rule-replay acceptance reproduces each bidder's shape from the policy,
    /// frozen at its node and surviving intervention: partner opened 1♥ (5+
    /// hearts) and RHO overcalled 2♣ (5+ clubs), so every accepted layout honors
    /// both — read by the rule, not a hand-written range.
    #[test]
    fn replay_honors_both_sides_under_competition() {
        let policy = crate::american().against(crate::bidding::Family::NATURAL);
        let actor = Seat::North;
        // len 2, North to act: index 0 is partner's 1♥, index 1 is RHO's 2♣.
        let auction = [bid(1, Strain::Hearts), bid(2, Strain::Clubs)];
        let inf = inferences(&auction);
        let mut rng = StdRng::seed_from_u64(3);
        let layouts = sample_layouts_replay(
            short_heart_actor(),
            actor,
            &policy,
            RelativeVulnerability::NONE,
            &auction,
            &inf,
            &mut rng,
            16,
        );

        assert!(!layouts.is_empty(), "the auction is feasible");
        for deal in &layouts {
            assert!(
                deal[actor.partner()][Suit::Hearts].len() >= 5,
                "partner's 1H opening promises 5+ hearts"
            );
            assert!(
                deal[actor.rho()][Suit::Clubs].len() >= 5,
                "RHO's 2C overcall promises 5+ clubs"
            );
        }
    }

    /// The pass reading flows into sampling with no sampler change: a booked
    /// read of an all-pass auction caps the passed seat, and the range gate
    /// already enforces the cap on every sampled layout.
    #[test]
    fn reads_a_passed_seat_as_bounded() {
        use crate::bidding::constraint::point_count;
        crate::bidding::set_pass_reading(true);
        crate::bidding::set_table_alert_reading(true);
        let stance = crate::american().against(crate::bidding::Family::NATURAL);
        let inf =
            Inferences::read(&stance.prefixed_context(RelativeVulnerability::NONE, &[Call::Pass]));

        assert_eq!(inf.rho().points.max, 11, "a no-open pass caps at 11");

        let actor = Seat::North;
        let mut rng = StdRng::seed_from_u64(7);
        let hand = full_deal(&mut rng)[actor];
        let layouts = sample_layouts(hand, actor, &inf, &mut rng, 8);
        assert!(!layouts.is_empty(), "a passed RHO is easy to deal");
        for deal in &layouts {
            assert!(point_count(deal[actor.rho()]) <= 11);
        }
    }

    /// A pass at an authored node replays like any call: a candidate the
    /// opening table would have opened cannot stand in for a dealer who
    /// passed (hard rejection — the pass gate is `-∞` on a 13-count).  A
    /// preempt-worthy hand within [`MARGIN`] of the pass would survive: the
    /// soft margin, tuned by A/B, not here.
    #[test]
    fn replay_rejects_implausible_passers() {
        let policy = crate::american().against(crate::bidding::Family::NATURAL);
        let opener: Hand = "AKQ2.K53.QJ4.T92".parse().expect("valid test hand");
        assert!(!made_plausibly(
            opener,
            &policy,
            RelativeVulnerability::NONE,
            &[],
            Call::Pass
        ));
        let quiet: Hand = "A2.K53.J9642.T92".parse().expect("valid test hand");
        assert!(made_plausibly(
            quiet,
            &policy,
            RelativeVulnerability::NONE,
            &[],
            Call::Pass
        ));
    }

    /// The game backstop is a *partial* table — it names only 4♥/4♠/3NT, so
    /// every other call sits at `-∞` while its unconditional 3NT keeps the
    /// node's best finite.  The gate then rejects partner's 3♣ for **every**
    /// hand: the 0% replay fill `probe-replay-yield` reports on this auction.
    /// Dropping the node lands resolution on the keyless floor, where
    /// [`System::authored_at`] is false and the gate abstains.
    #[test]
    fn game_backstop_rejects_every_hand_until_deleted() {
        let prefix = [
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
        ];
        let made = bid(3, Strain::Clubs);
        let vul = RelativeVulnerability::NONE;
        let mut rng = StdRng::seed_from_u64(11);
        let hands: Vec<Hand> = (0..16).map(|_| full_deal(&mut rng)[Seat::South]).collect();
        let policy = |on| {
            crate::bidding::american::set_game_backstop(on);
            crate::american().against(crate::bidding::Family::NATURAL)
        };

        let with = policy(true);
        assert!(
            hands
                .iter()
                .all(|&hand| !made_plausibly(hand, &with, vul, &prefix, made)),
            "the partial backstop rejects 3♣ out of hand"
        );

        let without = policy(false);
        assert!(
            hands
                .iter()
                .all(|&hand| made_plausibly(hand, &without, vul, &prefix, made)),
            "with no node the floor answers and the gate abstains"
        );
        crate::bidding::american::set_game_backstop(false); // restore the default
    }
}
