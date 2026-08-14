use crate::bidding::agreements::Agreements;
use crate::bidding::american::american;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain};

pub(super) const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// The ported row packages hold the compile-time invariants: guarded
/// tables total (the 7NT rule — a guarded table cannot fall through to
/// the floor), and artificial rows alerted (the row extension of
/// `artificial_calls_are_alerted`).
#[test]
fn row_package_invariants() {
    crate::bidding::rows::assert_package_invariants(
        &crate::bidding::agreements::Agreements::default(),
        &[
            super::direct_seat_package(),
            super::splinter_doubled_package(),
            super::support_double_package(),
            super::transfer_free_bid_package(),
            super::answer_negative_double_package(),
            super::cue_raise_answer_package(),
            super::cue_minor_raise_answer_package(),
            super::free_bid_answer_package(),
            super::high_overcall_package(),
            super::weak_two_competition_package(),
            super::strong_two_competition_package(),
            super::jordan_truscott_package(),
            super::uvu_over_majors_package(),
            super::cachalot_package(),
            super::sputnik_residual_answer_package(),
            super::uvu_package(),
            super::lebensohl_package(),
            super::competition_over_stayman_package(),
            super::competition_over_transfer_package(),
            super::competition_over_minor_transfer_package(),
            super::competition_over_diamond_transfer_package(),
        ],
    );
}

/// The Landy counter's three arms are all opt-in, so the default-agreements
/// run above never walks a single one of their rows.  Probe each arm on its
/// own — this is the only place the counter's totality and alert invariants are
/// checked at all.
#[test]
fn landy_counter_package_invariants() {
    for (cues, transfer) in [(false, false), (true, false), (false, true)] {
        let mut arm = Agreements::default();
        arm.their.two_clubs_landy = true;
        arm.competition.defense_2c_landy_cues = cues;
        arm.competition.defense_2c_landy_transfer = transfer;
        crate::bidding::rows::assert_package_invariants(&arm, &[super::lebensohl_package()]);
    }
}

/// `american()`'s best call for a hand in an auction, and whether the instinct
/// floor (not a book node) produced it
pub(super) fn best_call(auction: &[Call], hand: &str) -> (Call, bool) {
    best_call_with(
        &crate::bidding::agreements::Agreements::default(),
        auction,
        hand,
    )
}

/// [`best_call`], but under an explicit set of agreements
pub(super) fn best_call_with(
    agreements: &crate::bidding::agreements::Agreements,
    auction: &[Call],
    hand: &str,
) -> (Call, bool) {
    let hand: Hand = hand.parse().expect("valid test hand");
    let (logits, prov) = american(agreements)
        .bind()
        .classify_with_provenance(hand, RelativeVulnerability::NONE, auction)
        .expect("a legal auction classifies");
    let best = (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty");
    (best, prov.depth == 0 && prov.fallback.is_some())
}

/// As [`best_call`], with plain Lebensohl pinned on
pub(super) fn bid(auction: &[Call], hand: &str) -> (Call, bool) {
    let mut arm = Agreements::default();
    arm.competition.lebensohl_style = super::lebensohl::LebensohlStyle::Plain;
    best_call_with(&arm, auction, hand)
}

/// As [`best_call`], with Transfer Lebensohl pinned on
pub(super) fn bid_transfer(auction: &[Call], hand: &str) -> (Call, bool) {
    let mut arm = Agreements::default();
    arm.competition.lebensohl_style = super::lebensohl::LebensohlStyle::Transfer;
    best_call_with(&arm, auction, hand)
}

/// As [`best_call`], with the opponents' `2♣` declared as Landy (so the
/// counter-defense engages)
pub(super) fn bid_landy(auction: &[Call], hand: &str) -> (Call, bool) {
    let mut arm = Agreements::default();
    arm.their.two_clubs_landy = true;
    best_call_with(&arm, auction, hand)
}

/// As [`bid_landy`], with the N1b GF-minor-cue overlay on
pub(super) fn bid_landy_cues(auction: &[Call], hand: &str) -> (Call, bool) {
    let mut arm = Agreements::default();
    arm.their.two_clubs_landy = true;
    arm.competition.defense_2c_landy_cues = true;
    best_call_with(&arm, auction, hand)
}

/// As [`bid_landy`], with the N1c re-rung minors on (which imply the cues)
pub(super) fn bid_landy_transfer(auction: &[Call], hand: &str) -> (Call, bool) {
    let mut arm = Agreements::default();
    arm.their.two_clubs_landy = true;
    arm.competition.defense_2c_landy_transfer = true;
    best_call_with(&arm, auction, hand)
}

/// As [`best_call`], with the Unusual-vs-Unusual `(2NT)` structure pinned on
/// at the default A/B floors
pub(super) fn bid_uvu(auction: &[Call], hand: &str) -> (Call, bool) {
    let mut arm = Agreements::default();
    arm.competition.uvu = true;
    arm.competition.uvu_x_floor = 9;
    arm.competition.uvu_cue_floor = 8;
    best_call_with(&arm, auction, hand)
}

/// As [`best_call`], with our Jacoby-transfer competition + jump super-accept
/// enabled (both opt-in/default-off after the DD-negative A/B)
pub(super) fn bid_xfer(auction: &[Call], hand: &str) -> (Call, bool) {
    let mut arm = Agreements::default();
    arm.competition.competition_over_transfer = true;
    arm.notrump.transfer_super_accept = true;
    best_call_with(&arm, auction, hand)
}

/// As [`best_call`], with our 2♠ minor-transfer competition (Side A) pinned on
/// (it is also the default, but pin it so the arm is explicit)
pub(super) fn bid_minor(auction: &[Call], hand: &str) -> (Call, bool) {
    let mut arm = Agreements::default();
    arm.competition.competition_over_minor_transfer = true;
    best_call_with(&arm, auction, hand)
}

/// As [`best_call`], with our 2NT diamond-transfer competition (Side A) pinned
/// on (it is also the default, but pin it so the arm is explicit)
pub(super) fn bid_diamond(auction: &[Call], hand: &str) -> (Call, bool) {
    let mut arm = Agreements::default();
    arm.competition.competition_over_diamond_transfer = true;
    best_call_with(&arm, auction, hand)
}

/// As [`bid_transfer`], with the given double meaning pinned on
pub(super) fn bid_transfer_dbl(
    style: super::penalty_double::DoubleStyle,
    auction: &[Call],
    hand: &str,
) -> (Call, bool) {
    let mut arm = Agreements::default();
    arm.competition.lebensohl_style = super::lebensohl::LebensohlStyle::Transfer;
    arm.competition.double_style = style;
    best_call_with(&arm, auction, hand)
}

/// Renderability invariant: every guarded fallback in the competitive book
/// describes itself — the guard names its condition and a rebase names its
/// rewrite — so `render-book` and the web book show the whole book.  A new
/// bare `guard(closure)` fails here; wrap it in `described_guard`.
#[test]
fn competitive_fallbacks_are_renderable() {
    use crate::bidding::fallback::Fallback;

    let book = super::competition(&crate::bidding::agreements::Agreements::default());
    let all = book.0.fallbacks();
    assert!(
        all.len() > 30,
        "the competitive book has {} guarded entries — the walk is broken",
        all.len()
    );

    for (auction, guard, fallback) in &all {
        let key = contract_bridge::auction::display_calls(auction).to_string();
        assert!(
            guard.describe().is_some(),
            "unlabeled guard at [{key}] — wrap it in described_guard"
        );
        if let Fallback::Rebase(rewrite) = fallback {
            assert!(
                rewrite.describe().is_some(),
                "opaque rebase at [{key}] — wrap it in described_rewrite"
            );
        }
    }

    // Two concrete probes.  The `1♠` systems-on rebase renders with its
    // first-call label (the direct-seat table itself became exact
    // per-overcall nodes and no longer lives in the fallback layer)…
    let (_, guard, fallback) = all
        .iter()
        .find(|(auction, ..)| auction.as_ref() == [Call::Bid(Bid::new(1, Strain::Spades))])
        .expect("a guarded entry at [1♠]");
    assert_eq!(guard.describe().as_deref(), Some("X …"));
    assert!(
        matches!(fallback, Fallback::Rebase(_)),
        "the systems-on entry is a rebase"
    );
    // …and the exact `1♠ (2♣)` node carries the negative double.
    let auction = [
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Bid(Bid::new(2, Strain::Clubs)),
    ];
    let rules = book
        .0
        .get(&auction)
        .expect("an exact node per overcall")
        .as_rules()
        .expect("an authored Rules table");
    assert!(
        rules.rules().iter().any(|rule| rule.call() == Call::Double),
        "the negative double renders"
    );
}

/// Build one package into a fresh trie under the given agreements.
fn compiled_package(agreements: &Agreements, package: super::Package) -> crate::bidding::Trie {
    let mut book = crate::bidding::Trie::new();
    super::compile_into(&mut book, agreements, &[package]);
    book
}

/// Assert two wirings of one package resolve and classify identically
/// over a superset of probe auctions: equal `Option<Logits>` per hand
/// catches both over- and under-expansion (an auction only one wiring
/// answers shows up as `Some` vs `None`).
fn assert_wirings_match(
    agreements: &Agreements,
    legacy: super::Package,
    current: super::Package,
    auctions: &[Vec<Call>],
    label: &str,
) {
    use crate::bidding::context::Context;

    let hands: Vec<Hand> = [
        "QJ9862.43.752.83",
        "83.QJ9862.752.43",
        "KQ72.QJ84.652.83",
        "K53.Q42.J932.T87",
        "K5.Q4.J9532.KT87",
        "KJ8.QT7.AJ94.986",
        "AKQ2.KQ5.AQJ4.92",
        "AQ2.K53.QJ42.T92",
        "2.98653.QJ742.92",
        "AQJ83.K4.KT7.J93",
    ]
    .iter()
    .map(|hand| hand.parse().expect("valid probe hand"))
    .collect();

    let old_book = compiled_package(agreements, legacy);
    let new_book = compiled_package(agreements, current);
    for auction in auctions {
        let context = Context::new(RelativeVulnerability::NONE, auction);
        for &hand in &hands {
            // Massless normalizes to unanswered: a guarded table that
            // rejects a hand is re-found on the fall-through pass (stuck
            // massless), while an exact node rejects-to-floor — the
            // documented exact-node semantic, and in the full book the
            // floor then answers where the guard wedged.  Mass-bearing
            // answers must match exactly.
            let classify = |book: &crate::bidding::Trie| {
                book.classify_floored(hand, &context, auction)
                    .map(|(logits, _)| logits)
                    .filter(super::super::super::array::Logits::has_mass)
            };
            assert_eq!(
                classify(&old_book),
                classify(&new_book),
                "{label}: {} with {hand}",
                contract_bridge::auction::display_calls(auction),
            );
        }
    }
}

/// Only legal auctions probe the wirings: a guard never checks legality
/// (it answers `2♣ (1♣)` if asked), but play can never ask.
fn ascending(auction: &[Call]) -> bool {
    let bids: Vec<Bid> = auction
        .iter()
        .filter_map(|call| match call {
            Call::Bid(bid) => Some(*bid),
            _ => None,
        })
        .collect();
    bids.windows(2).all(|pair| pair[0] < pair[1])
}

/// Every bid, for superset probing.
fn all_bids() -> Vec<Bid> {
    (1..=7u8)
        .flat_map(|level| {
            [
                Strain::Clubs,
                Strain::Diamonds,
                Strain::Hearts,
                Strain::Spades,
                Strain::Notrump,
            ]
            .into_iter()
            .map(move |strain| Bid::new(level, strain))
        })
        .collect()
}

/// Every converted package resolves and classifies exactly as its retired
/// guarded wiring, over a superset of the guard's auction space.
#[test]
fn converted_packages_match_legacy() {
    use super::free_bids::FreeBidStyle;
    use super::negative_double::NegativeDoubleShape;

    let shipped = Agreements::default();

    // Section 4: opener answers the negative double of a 2-level minor.
    let mut auctions = Vec::new();
    for major in [Strain::Hearts, Strain::Spades] {
        for bid in all_bids() {
            auctions.push(vec![
                call(1, major),
                Call::Bid(bid),
                Call::Double,
                Call::Pass,
            ]);
        }
    }
    auctions.retain(|auction| ascending(auction));
    assert_wirings_match(
        &shipped,
        super::negative_double::answer_negative_double_package_legacy(),
        super::answer_negative_double_package(),
        &auctions,
        "answer-negative-double",
    );

    // Section 10: their jump / 3-level overcalls, and the double behind.
    let mut auctions = Vec::new();
    for opening in [
        Strain::Clubs,
        Strain::Diamonds,
        Strain::Hearts,
        Strain::Spades,
    ] {
        for bid in all_bids() {
            auctions.push(vec![call(1, opening), Call::Bid(bid)]);
            auctions.push(vec![
                call(1, opening),
                Call::Bid(bid),
                Call::Double,
                Call::Pass,
            ]);
        }
    }
    auctions.retain(|auction| ascending(auction));
    assert_wirings_match(
        &shipped,
        super::high_overcall::high_overcall_package_legacy(),
        super::high_overcall_package(),
        &auctions,
        "high-overcall",
    );

    // Section 8: the contested strong 2♣, both seats.
    let mut auctions = Vec::new();
    for bid in all_bids() {
        auctions.push(vec![call(2, Strain::Clubs), Call::Bid(bid)]);
        auctions.push(vec![
            call(2, Strain::Clubs),
            Call::Bid(bid),
            Call::Pass,
            Call::Pass,
        ]);
    }
    auctions.retain(|auction| ascending(auction));
    assert_wirings_match(
        &shipped,
        super::our_preempts::strong_two_competition_package_legacy(),
        super::strong_two_competition_package(),
        &auctions,
        "strong-two-competition",
    );

    // Section 4d/4d′: opener answers the free bid, across the style knobs
    // that reshape the free-bid grammar.
    let mut auctions = Vec::new();
    for opening in [
        Strain::Clubs,
        Strain::Diamonds,
        Strain::Hearts,
        Strain::Spades,
    ] {
        for ovc in all_bids() {
            for free in all_bids() {
                if free.level.get() > 2 {
                    continue;
                }
                auctions.push(vec![
                    call(1, opening),
                    Call::Bid(ovc),
                    Call::Bid(free),
                    Call::Pass,
                ]);
            }
        }
    }
    auctions.retain(|auction| ascending(auction));
    for shape in [
        NegativeDoubleShape::BothMajors,
        NegativeDoubleShape::Modern,
        NegativeDoubleShape::Cachalot,
        NegativeDoubleShape::Sputnik,
    ] {
        for style in [
            FreeBidStyle::Forcing,
            FreeBidStyle::Negative,
            FreeBidStyle::Transfer,
        ] {
            let mut arm = shipped;
            arm.competition.negative_double_shape = shape;
            arm.competition.free_bid_style = style;
            assert_wirings_match(
                &arm,
                super::free_bids::free_bid_answer_package_legacy(),
                super::free_bid_answer_package(),
                &auctions,
                &format!("free-bid-answer ({shape:?}, {style:?})"),
            );
        }
    }

    // Section 4b/4c: opener answers the cue-raise, majors and minors.  One
    // auction space serves both — each package's own ceiling decides which
    // columns it claims.
    let mut auctions = Vec::new();
    for opening in [
        Strain::Clubs,
        Strain::Diamonds,
        Strain::Hearts,
        Strain::Spades,
    ] {
        for ovc in all_bids() {
            for cue in all_bids() {
                auctions.push(vec![
                    call(1, opening),
                    Call::Bid(ovc),
                    Call::Bid(cue),
                    Call::Pass,
                ]);
            }
        }
    }
    auctions.retain(|auction| ascending(auction));
    assert_wirings_match(
        &shipped,
        super::cue_raise::cue_raise_answer_package_legacy(),
        super::cue_raise_answer_package(),
        &auctions,
        "cue-raise-answer",
    );
    assert_wirings_match(
        &shipped,
        super::cue_raise::cue_minor_raise_answer_package_legacy(),
        super::cue_minor_raise_answer_package(),
        &auctions,
        "cue-minor-raise-answer",
    );

    // Section 9: the Cachalot contested X — every intervention, plus the
    // pass-out the completions shadow.
    let mut auctions = Vec::new();
    for (opening, overcall) in [
        (Strain::Clubs, Strain::Diamonds),
        (Strain::Clubs, Strain::Hearts),
        (Strain::Diamonds, Strain::Hearts),
    ] {
        for intervention in all_bids()
            .into_iter()
            .map(Call::Bid)
            .chain([Call::Pass, Call::Redouble])
        {
            auctions.push(vec![
                call(1, opening),
                call(1, overcall),
                Call::Double,
                intervention,
            ]);
        }
    }
    auctions.retain(|auction| ascending(auction));
    let mut cachalot = shipped;
    cachalot.competition.negative_double_shape = NegativeDoubleShape::Cachalot;
    assert_wirings_match(
        &cachalot,
        super::negative_double::cachalot_package_legacy(),
        super::cachalot_package(),
        &auctions,
        "cachalot-answer",
    );
}
