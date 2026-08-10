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
    assert_eq!(direct.rho().strength.points.min, 15);

    // A passed hand doubling: dealer passes, RHO opens 1NT, two passes, then the
    // dealer (now a passed hand) doubles — both majors, not a 15+ penalty double.
    let passed = inferences(&[
        Call::Pass,
        bid(1, Strain::Notrump),
        Call::Pass,
        Call::Pass,
        Call::Double,
    ]);
    assert!(passed.rho().strength.points.min < 15);
}

/// The latch's subsequent penalty double reads as four-plus in the doubled
/// suit, so partner reads it as penalty (and leaves it in) instead of takeout.
#[test]
fn reads_latched_penalty_double_of_the_runout() {
    use crate::bidding::instinct::set_penalty_latch;
    // `(1NT) X (2♦) X -`: our penalty X, their runout, partner's penalty double.
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
                prop_assert!(shown.strength.points.contains(point_count(deal[other])));
            }
        }
    });
}

/// A richer auction whose constraints land on more than one player.
#[test]
fn respects_a_developed_auction() {
    let actor = Seat::North;
    // After `1♥ (1♠)`, RHO's overcall shows 5+ spades and 8+.  Inferences
    // reads partner's opening and RHO's overcall; we sample around them.
    let auction = [bid(1, Strain::Hearts), bid(1, Strain::Spades)];
    let inf = inferences(&auction);
    let mut rng = StdRng::seed_from_u64(7);
    let layouts = sample_layouts(short_heart_actor(), actor, &inf, &mut rng, 16);

    assert!(!layouts.is_empty(), "the auction is feasible");
    for deal in &layouts {
        let partner = deal[actor.partner()];
        assert!(partner[Suit::Hearts].len() >= 5);
        assert!(inf.partner().strength.points.contains(point_count(partner)));
        let rho = deal[actor.rho()];
        assert!(rho[Suit::Spades].len() >= 5);
        assert!(inf.rho().strength.points.contains(point_count(rho)));
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
    let policy = crate::american(&crate::bidding::agreements::Agreements::current()).against();
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
    let mut agreements = crate::bidding::agreements::Agreements::current();
    agreements.decision.reading.pass = true;
    agreements.decision.reading.table_alerts = true;
    let stance = crate::american(&agreements).against();
    let inf =
        Inferences::read(&stance.prefixed_context(RelativeVulnerability::NONE, &[Call::Pass]));

    assert_eq!(
        inf.rho().strength.points.max,
        11,
        "a no-open pass caps at 11"
    );

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
    let policy = crate::american(&crate::bidding::agreements::Agreements::current()).against();
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
        let mut agreements = crate::bidding::agreements::Agreements::current();
        agreements.game_force.game_backstop = on;
        crate::american(&agreements).against()
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
}
