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
            super::nt_high_overcall_package(),
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

/// The `(3♣)` transfer variant replaces rows the default walk above does see,
/// so its arm needs its own totality / alert / weight-tie check.
#[test]
fn nt_high_overcall_transfer_package_invariants() {
    let mut arm = Agreements::default();
    arm.competition.nt_high_overcall_responses = true;
    arm.competition.nt_3c_transfers = true;
    crate::bidding::rows::assert_package_invariants(&arm, &[super::nt_high_overcall_package()]);
}

/// The leave-in arm adds a `Pass` row the default walk never sees, so it needs
/// its own totality / alert / weight-tie check — without this, a second `Pass`
/// row at the same weight (the shape a "cells" extension of the gate wants)
/// would tie undetected.
#[test]
fn nt_high_overcall_leave_in_package_invariants() {
    let mut arm = Agreements::default();
    arm.competition.nt_high_overcall_responses = true;
    arm.competition.nt_high_overcall_x_leave_in = true;
    crate::bidding::rows::assert_package_invariants(&arm, &[super::nt_high_overcall_package()]);
    arm.competition.nt_high_overcall_x_leave_in_three = true;
    crate::bidding::rows::assert_package_invariants(&arm, &[super::nt_high_overcall_package()]);
}

/// The Landy counter's arms are all opt-in, so the default-agreements run
/// above never walks a single one of their rows.  Probe each arm on its own —
/// this is the only place the counter's totality and alert invariants are
/// checked at all.  The tuples are (cues, transfer, cue_floor, fit_answers,
/// competition, low_minors, hcp_rungs): the three original arms, each N1d/e/f
/// refinement alone, the stacked A/B arms d → d+e → d+e+f, and N1h / N1i each
/// alone and stacked.
#[test]
fn landy_counter_package_invariants() {
    for (cues, transfer, cue_floor, fit_answers, competition, low_minors, hcp_rungs) in [
        (false, false, false, false, false, false),
        (true, false, false, false, false, false),
        (false, true, false, false, false, false),
        (false, false, true, false, false, false),
        (false, false, false, true, false, false),
        (false, false, false, false, true, false),
        (false, false, true, true, false, false),
        (false, false, true, true, true, false),
        // The shipped default since 2026-08-14: the whole stack.
        (false, true, true, true, true, false),
        // N1h alone, and the A/B's ON arm (the stack plus N1h).
        (false, false, false, false, false, true),
        (false, true, true, true, true, true),
    ]
    .into_iter()
    .map(|(c, t, cf, fa, comp, low)| (c, t, cf, fa, comp, low, false))
    // N1i alone, and stacked — the `hcp` regrading of the same rungs.
    .chain([
        (false, false, false, false, false, false, true),
        (false, true, true, true, true, false, true),
    ]) {
        let mut arm = Agreements::default();
        arm.decision.their.two_clubs_landy = true;
        arm.competition.defense_2c_landy_cues = cues;
        arm.competition.defense_2c_landy_transfer = transfer;
        arm.competition.defense_2c_landy_cue_floor = cue_floor;
        arm.competition.defense_2c_landy_fit_answers = fit_answers;
        arm.competition.defense_2c_landy_competition = competition;
        arm.competition.defense_2c_landy_low_minors = low_minors;
        arm.competition.defense_2c_landy_hcp_rungs = hcp_rungs;
        // The stack arms under test, not the N1j ladder that now defaults on.
        arm.competition.defense_2c_landy_bba = false;
        crate::bidding::rows::assert_package_invariants(&arm, &[super::lebensohl_package()]);
    }
    // N1j: the BBA ladder alone, and with its weak-2♦ cap arm.  The stack
    // knobs are inert under it, so two arms cover the whole surface — crossed
    // with §N1l's doubler rebid ladder, which adds four nodes plus three
    // answers under each and is the only default-off knob in this lane.
    for cap in [false, true] {
        for doubler_rebids in [false, true] {
            let mut arm = Agreements::default();
            arm.decision.their.two_clubs_landy = true;
            arm.competition.defense_2c_landy_bba = true;
            arm.competition.defense_2c_landy_weak_2d_cap = cap;
            arm.competition.landy_doubler_rebids = doubler_rebids;
            crate::bidding::rows::assert_package_invariants(&arm, &[super::lebensohl_package()]);
        }
    }
}

/// The Multi stopper ask is default-off, so probe both opt-in packages
/// explicitly.  This checks every guarded continuation is total and every
/// artificial row (the `3♠` ask) carries its alert.
#[test]
fn multi_stopper_package_invariants() {
    for mode in [
        super::MultiStopperAsk::FitSearch,
        super::MultiStopperAsk::OpenerPlaces,
    ] {
        let mut arm = Agreements::default();
        arm.decision.their.two_diamonds_multi = true;
        arm.competition.multi_stopper_ask = mode;
        crate::bidding::rows::assert_package_invariants(&arm, &[super::lebensohl_package()]);
    }
}

/// The Kokish–Kraft counter is default-off *and* gated on the `(2♦)`
/// disclosure, so no default sweep ever builds its rows.  This is the arm that
/// does: every guarded continuation total, every artificial row alerted, no
/// same-call weight ties.  Its composition matrix rides along — the variant
/// shares seats with `multi_weak_escape` and `multi_balance`, and a table that
/// is total on one arm can be holed on another.
#[test]
fn kokish_kraft_package_invariants() {
    for (weak, balance) in [
        (Some(6), false),
        (None, false),
        (Some(6), true),
        (None, true),
    ] {
        // The `4m` slam try adds rows at four seats and a whole RKCB ladder, so
        // it sweeps as its own axis: `None` is the shipped table, and both A/B
        // floors must leave every guarded continuation total.
        for slam in [None, Some(13), Some(15)] {
            // And the doubler's natural other major, which adds two answer
            // tables on the `2♠` leg and one on each `3♥` leg — crossed with
            // the P/X split, which arms a *third* leg, re-weights the rung to
            // 148 and swaps the delayed-`2NT` answer.  The two knobs overlap
            // on two legs and must still emit one rung apiece, which is
            // exactly the same-call weight tie this sweep exists to catch.
            for (doubler_major, px_split) in
                [(false, false), (true, false), (false, true), (true, true)]
            {
                let mut arm = Agreements::default();
                arm.decision.their.two_diamonds_multi = true;
                arm.competition.multi_kokish_kraft = true;
                arm.competition.multi_weak_escape = weak;
                arm.competition.multi_balance = balance;
                arm.competition.multi_minor_slam_try = slam;
                arm.competition.multi_doubler_major = doubler_major;
                arm.competition.multi_px_split = px_split;
                crate::bidding::rows::assert_package_invariants(
                    &arm,
                    &[super::lebensohl_package()],
                );
            }
        }
    }
}

/// Kokish–Kraft is the PDI negative control: its penalty/takeout split is
/// **trie geometry** — the repeated double and the delayed double sit at
/// different nodes — so a book node shadows the floor at every seat the
/// generalized latch could otherwise reach.  Arming `pdi_latch` must move
/// nothing here.  `docs/pdi.md`.
#[test]
fn kokish_kraft_unchanged_under_pdi() {
    let mut off = crate::bidding::agreements::Agreements::default();
    off.decision.their.two_diamonds_multi = true;
    off.competition.multi_kokish_kraft = true;
    let mut on = off;
    on.decision.reading.pdi_latch = true;

    let nt = Call::Bid(Bid::new(1, Strain::Notrump));
    let auctions = [
        // `1NT (2♦) X (2♥)` — back to opener over their pass-or-correct major.
        vec![
            nt,
            Call::Bid(Bid::new(2, Strain::Diamonds)),
            Call::Double,
            Call::Bid(Bid::new(2, Strain::Hearts)),
        ],
        // `1NT (2♦) X (2♥) - (2♠)` — they correct, opener acts again.
        vec![
            nt,
            Call::Bid(Bid::new(2, Strain::Diamonds)),
            Call::Double,
            Call::Bid(Bid::new(2, Strain::Hearts)),
            Call::Pass,
            Call::Bid(Bid::new(2, Strain::Spades)),
        ],
        // `1NT (2♦) X (2♥) X -` — back to the values doubler over partner's
        // penalty double of their major.
        vec![
            nt,
            Call::Bid(Bid::new(2, Strain::Diamonds)),
            Call::Double,
            Call::Bid(Bid::new(2, Strain::Hearts)),
            Call::Double,
            Call::Pass,
        ],
    ];
    for auction in &auctions {
        for hand in [
            "AQ74.KQT.A82.K63",
            "AQ7.KQT9.A82.K63",
            "AQJT.65.AK82.Q63",
            "AK.QJT9.KQ82.A63",
        ] {
            assert_eq!(
                best_call_with(&on, auction, hand),
                best_call_with(&off, auction, hand),
                "pdi_latch moved {hand} at {auction:?}"
            );
        }
    }
}

/// An opponent's double and conversion cannot latch PDI for our side.
#[test]
fn pdi_does_not_move_an_untriggered_side() {
    let auction = vec![
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Bid(Bid::new(1, Strain::Notrump)),
        Call::Bid(Bid::new(2, Strain::Spades)),
        Call::Double,
        Call::Pass,
        Call::Pass,
    ];
    let off = Agreements::default();
    let mut on = off;
    on.decision.reading.pdi_latch = true;

    let system = american(&on).bind();
    assert!(
        !system
            .infer(RelativeVulnerability::NONE, &auction)
            .pdi_latched()
    );
    assert_eq!(
        best_call_with(&on, &auction, "K93.KJT32.AT.KQ4"),
        best_call_with(&off, &auction, "K93.KJT32.AT.KQ4")
    );
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
    // `max_by` keeps the *last* maximum; production keeps the *first strict*
    // one (`select_with_legal_state`), so a cross-call weight tie resolves the
    // opposite way.  `reduce` with a strict `>` matches production.
    let best = (&logits.0)
        .into_iter()
        .reduce(|best, next| if next.1 > best.1 { next } else { best })
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

/// The declared-Landy agreements with the measured arms' historical knob
/// state pinned explicitly — the 2026-08-14 default flip turned the whole
/// N1d/e/f stack on, so a helper that wants the *base* counter (or a single
/// overlay) must say so rather than ride the defaults.
fn landy_arm(cues: bool, transfer: bool) -> Agreements {
    let mut arm = Agreements::default();
    arm.decision.their.two_clubs_landy = true;
    arm.competition.defense_2c_landy_cues = cues;
    arm.competition.defense_2c_landy_transfer = transfer;
    arm.competition.defense_2c_landy_cue_floor = false;
    arm.competition.defense_2c_landy_fit_answers = false;
    arm.competition.defense_2c_landy_competition = false;
    // The 2026-08-15 flip made the N1j ladder the default; these helpers
    // pin the historical stack arms.
    arm.competition.defense_2c_landy_bba = false;
    arm
}

/// As [`best_call`], with the opponents' `2♣` declared as Landy and the
/// **base** counter pinned (the pre-stack N1 arm)
pub(super) fn bid_landy(auction: &[Call], hand: &str) -> (Call, bool) {
    best_call_with(&landy_arm(false, false), auction, hand)
}

/// As [`bid_landy`], with the N1b GF-minor-cue overlay on (alone)
pub(super) fn bid_landy_cues(auction: &[Call], hand: &str) -> (Call, bool) {
    best_call_with(&landy_arm(true, false), auction, hand)
}

/// As [`bid_landy`], with the N1c re-rung minors on (which imply the cues),
/// N1d/e/f pinned off — the arm the N1c A/B measured
pub(super) fn bid_landy_transfer(auction: &[Call], hand: &str) -> (Call, bool) {
    best_call_with(&landy_arm(false, true), auction, hand)
}

/// As [`bid_landy_transfer`], with any of the N1d/N1e/N1f refinements on —
/// the cue floor, the doubleton-notrump answers, the interfered tails.  Each
/// implies N1c on its own.
pub(super) fn bid_landy_n1(
    cue_floor: bool,
    fit_answers: bool,
    competition: bool,
    auction: &[Call],
    hand: &str,
) -> (Call, bool) {
    let mut arm = Agreements::default();
    arm.decision.their.two_clubs_landy = true;
    arm.competition.defense_2c_landy_cue_floor = cue_floor;
    arm.competition.defense_2c_landy_fit_answers = fit_answers;
    arm.competition.defense_2c_landy_competition = competition;
    arm.competition.defense_2c_landy_bba = false;
    best_call_with(&arm, auction, hand)
}

/// As [`bid_landy`], with the N1j BBA ladder on (optionally with the weak-2♦
/// `hcp(..=6)` cap).  The N1b–N1i stack knobs are inert under it.
pub(super) fn bid_landy_bba(cap: bool, auction: &[Call], hand: &str) -> (Call, bool) {
    let mut arm = Agreements::default();
    arm.decision.their.two_clubs_landy = true;
    arm.competition.defense_2c_landy_bba = true;
    arm.competition.defense_2c_landy_weak_2d_cap = cap;
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
