use super::super::tests::best;
use super::defense_to_suit;
use crate::bidding::american::{NotrumpDefense, american};
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain};

pub(super) const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// The ported row package holds the row invariants (alerts; totality is
/// exact-node-exempt).
#[test]
fn row_package_invariants() {
    crate::bidding::rows::assert_package_invariants(
        &crate::bidding::agreements::Agreements::current(),
        &[
            super::weak_two_defense_package(),
            super::suit_defense_package(),
            super::notrump_defense_package(),
            super::landy_advance_package(),
            super::both_majors_double_package(),
            super::their_stayman_defense_package(),
            super::their_transfer_defense_package(),
            super::their_minor_transfer_defense_package(),
            super::their_diamond_transfer_defense_package(),
            super::unusual_notrump_advance_package(),
            super::direct_dont_advance_package(),
            super::meckwell_advance_package(),
            super::advance_double_package(),
            super::rich_advance_double_package(),
            super::responsive_double_package(),
            super::responsive_overcall_package(),
            super::weak_two_notrump_advance_package(),
            super::leaping_michaels_package(),
            super::woolsey_package(),
            super::advance_of_double_package(),
            super::gladiator_package(),
            super::gladiator_sohl_package(),
        ],
    );
}

/// `american()`'s best call for a hand in an auction, and whether the instinct
/// floor (not a book node) produced it
pub(super) fn best_call(auction: &[Call], hand: &str) -> (Call, bool) {
    let hand: Hand = hand.parse().expect("valid test hand");
    let (logits, prov) = american()
        .against()
        .classify_with_provenance(hand, RelativeVulnerability::NONE, auction)
        .expect("a legal auction classifies");
    let best = (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty");
    (best, prov.depth == 0 && prov.fallback.is_some())
}

/// [`best_call`] at a chosen vulnerability, for the rules that read one.
pub(super) fn best_call_vul(auction: &[Call], hand: &str, vul: RelativeVulnerability) -> Call {
    let hand: Hand = hand.parse().expect("valid test hand");
    let (logits, _) = american()
        .against()
        .classify_with_provenance(hand, vul, auction)
        .expect("a legal auction classifies");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty")
}

/// Per-call exclusivity: in every named 1NT-defense config the `(1NT)` node
/// authors at most one rule per call.  This is the invariant the alert-tag gate
/// must preserve — two rules on one call would let a hand fire the wrong
/// convention and a reading mis-decode it (e.g. a natural overcall leaking onto
/// a slot an artificial alert owns).  Pass is always authored (these configs all
/// own the auction).
#[test]
fn defense_to_notrump_authors_one_rule_per_call() {
    fn reset() {
        super::nt_defense::set_notrump_defense(NotrumpDefense::Natural);
        super::nt_landy::set_landy(None);
        super::michaels::set_unusual_notrump_defense(Some((8, 13)));
    }

    let configs: [(&str, fn()); 7] = [
        ("natural+unusual2nt", || {}),
        ("natural+landy", || {
            super::nt_landy::set_landy(Some((8, 15)))
        }),
        ("woolsey", || {
            super::nt_defense::set_notrump_defense(NotrumpDefense::Woolsey)
        }),
        ("dont", || {
            super::nt_defense::set_notrump_defense(NotrumpDefense::DirectDont)
        }),
        ("meckwell", || {
            super::nt_defense::set_notrump_defense(NotrumpDefense::Meckwell)
        }),
        ("direct-landy-x", || {
            super::nt_landy::set_direct_landy_double(Some(false))
        }),
        ("always-pass", || {
            super::nt_defense::set_notrump_defense(NotrumpDefense::AlwaysPass)
        }),
    ];

    for (label, setup) in configs {
        reset();
        setup();
        let calls: Vec<Call> = super::nt_defense::defense_to_notrump()
            .rules()
            .iter()
            .map(|r| r.call())
            .collect();
        reset();
        assert!(
            calls.contains(&Call::Pass),
            "{label}: the owning Pass is missing",
        );
        for i in 0..calls.len() {
            for j in (i + 1)..calls.len() {
                assert!(
                    calls[i] != calls[j],
                    "{label}: call {:?} authored by two rules at the [1NT] node",
                    calls[i],
                );
            }
        }
    }
}

/// D1b: the 5-box `semi_balanced` union accepts exactly the shapes the
/// legacy `balanced() | described("5422/6322/7222", …)` composite did,
/// exhaustively over the 560-shape length lattice.
#[test]
fn semi_balanced_boxes_match_closure() {
    use super::michaels::semi_balanced;
    use crate::bidding::constraint::{Constraint as _, for_each_shape};
    use crate::bidding::context::Context;

    let ctx = Context::new(RelativeVulnerability::NONE, &[]);
    let gate = semi_balanced();
    for_each_shape(|lengths, hand| {
        let mut sorted = lengths;
        sorted.sort_unstable();
        let reference = matches!(
            sorted,
            // balanced …
            [3, 3, 3, 4] | [2, 3, 4, 4] | [2, 3, 3, 5]
            // … or the replaced closure's patterns
            | [2, 2, 4, 5] | [2, 2, 3, 6] | [2, 2, 2, 7]
        );
        assert_eq!(
            gate.eval(hand, &ctx).is_finite(),
            reference,
            "semi_balanced disagrees at {lengths:?}",
        );
    });
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
