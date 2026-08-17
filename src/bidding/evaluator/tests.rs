use super::*;
use crate::bidding::Context;
use crate::bidding::features::FEATURES_VERSION_EVAL;
use contract_bridge::auction::RelativeVulnerability;

fn hand(s: &str) -> Hand {
    s.parse().expect("valid test hand")
}

/// The hand-rolled forward pass must reproduce the trainer's candle
/// outputs on the exported fixture — the legacy envelope-union-off weights.
#[test]
fn matches_candle_fixture() {
    check_fixture(
        include_str!("../weights/evaluator_v2.fixture.json"),
        u64::from(FEATURES_VERSION_EVAL),
        IN,
        |x| forward_on(false, x),
    );
}

/// The knob-on twin (the shipped default) against its own fixture.
#[test]
fn union_reading_matches_candle_fixture() {
    check_fixture(
        include_str!("../weights/evaluator_v2_dnf.fixture.json"),
        u64::from(FEATURES_VERSION_EVAL),
        IN,
        |x| forward_on(true, x),
    );
}

/// The v3 calls-tail artifact against its own fixture — served directly,
/// no knobs, since the weight set is explicit.
#[test]
fn v3_matches_candle_fixture() {
    check_fixture(
        include_str!("../weights/evaluator_v3_dnf.fixture.json"),
        3,
        IN_V3,
        |x| forward_with::<IN_V3>(&WEIGHTS_V3_UNION_READING, x),
    );
}

/// The v4 shape-reading artifact against its own fixture.
#[test]
fn v4_matches_candle_fixture() {
    check_fixture(
        include_str!("../weights/evaluator_v4_dnf.fixture.json"),
        u64::from(crate::bidding::features::FEATURES_VERSION_EVAL_V4),
        IN_V4,
        |x| forward_with::<IN_V4>(&WEIGHTS_V4_UNION_READING, x),
    );
}

/// The v4 knob's contract: off it changes nothing, on it supersedes v3 and
/// still lands in the plausible band.  Restores the crate defaults.
#[test]
fn eval_shape_knob_contract() {
    use contract_bridge::{Bid, Level};
    let auction = [
        Call::Bid(Bid {
            level: Level::new(1),
            strain: Strain::Spades,
        }),
        Call::Pass,
    ];
    let ctx = Context::new(RelativeVulnerability::NONE, &auction);
    let inf = Inferences::read(&ctx);
    let h = hand("AQ32.K53.QJ4.A92");

    let mut profile = DecisionProfile::default();
    profile.reading.envelope_union = true;
    profile.eval_auction = true;
    profile.eval_shape = false;
    let v3 = trick_estimates_with_auction_on(&profile, h, &inf, &auction);
    assert!(
        !DecisionProfile::default().eval_shape,
        "the v4 knob ships off"
    );

    profile.eval_shape = true;
    let v4 = trick_estimates_with_auction_on(&profile, h, &inf, &auction);
    profile.eval_shape = false;
    assert_eq!(
        trick_estimates_with_auction_on(&profile, h, &inf, &auction),
        v3,
        "knob off must be byte-identical"
    );

    assert_ne!(v4, v3, "the v4 artifact should not shadow v3 exactly");
    for strain in Strain::ASC {
        for who in [
            Relative::Me,
            Relative::Lho,
            Relative::Partner,
            Relative::Rho,
        ] {
            let g = v4.get(strain, who);
            assert!(
                (0.0..=13.0).contains(&g.mean) && g.sd > 0.0 && g.sd < 13.0,
                "{strain:?} {who:?}: {g:?}"
            );
        }
    }
}

fn check_fixture(fixture: &str, version: u64, in_dim: usize, fwd: impl Fn(&[f32]) -> [f32; OUT]) {
    let fx: serde_json::Value = serde_json::from_str(fixture).unwrap();

    // The blob's own guard is a byte count, and a byte count cannot tell a
    // 54-wide v2 artifact from any other blob of the same size — so pin the
    // layout tag too, or a stale fixture would sail through on width alone.
    assert_eq!(
        fx["feature_version"].as_u64(),
        Some(version),
        "fixture layout tag disagrees with the crate's"
    );

    let rows = fx["features"].as_array().unwrap();
    let golds = fx["outputs"].as_array().unwrap();
    assert!(!rows.is_empty(), "fixture has no rows");

    let to_vec = |v: &serde_json::Value| -> Vec<f32> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap() as f32)
            .collect()
    };

    let mut max_abs = 0f32;
    for (frow, grow) in rows.iter().zip(golds) {
        let x = to_vec(frow);
        let gold = to_vec(grow);
        assert_eq!(x.len(), in_dim);
        assert_eq!(gold.len(), OUT);
        for (pred, g) in fwd(&x).iter().zip(&gold) {
            max_abs = max_abs.max((pred - g).abs());
        }
    }
    assert!(max_abs < 1.0e-3, "max abs diff {max_abs} exceeds tolerance");
}

/// Knob off, `trick_estimates_with_auction` is exactly `trick_estimates`
/// — the byte-identity half of the knob contract.  Knob on (the shipped
/// default) in the envelope-union regime, the v3 artifact serves: same plausibility
/// bounds, and the auction tail visibly moves the estimate.
#[test]
fn with_auction_knob_contract() {
    use contract_bridge::{Bid, Level};
    let auction = [
        Call::Bid(Bid {
            level: Level::new(1),
            strain: Strain::Spades,
        }),
        Call::Pass,
    ];
    let ctx = Context::new(RelativeVulnerability::NONE, &auction);
    let inf = Inferences::read(&ctx);
    let h = hand("AQ32.K53.QJ4.A92");

    let mut profile = DecisionProfile::default();
    profile.reading.envelope_union = true;
    profile.eval_auction = false;
    let v2 = trick_estimates(h, &inf);
    assert_eq!(
        trick_estimates_with_auction_on(&profile, h, &inf, &auction),
        v2
    );

    profile.eval_auction = true;
    let v3 = trick_estimates_with_auction_on(&profile, h, &inf, &auction);

    assert_ne!(v3, v2, "v3 artifact should not shadow v2 exactly");
    for strain in Strain::ASC {
        for who in [
            Relative::Me,
            Relative::Lho,
            Relative::Partner,
            Relative::Rho,
        ] {
            let g = v3.get(strain, who);
            assert!((-1.0..=14.0).contains(&g.mean), "{strain:?} {who:?} {g:?}");
            assert!((0.1..=5.0).contains(&g.sd), "{strain:?} {who:?} {g:?}");
        }
    }
}

#[test]
fn estimates_are_positive_and_plausible() {
    let ctx = Context::new(RelativeVulnerability::NONE, &[]);
    let e = trick_estimates(hand("AKQ32.K532.QJ4.9"), &Inferences::read(&ctx));
    for strain in Strain::ASC {
        for who in [
            Relative::Me,
            Relative::Lho,
            Relative::Partner,
            Relative::Rho,
        ] {
            let g = e.get(strain, who);
            assert!(g.sd > 0.0, "{strain:?} {who:?} non-positive sd: {g:?}");
            assert!(
                (-1.0..=14.0).contains(&g.mean),
                "{strain:?} {who:?} mean off-scale: {g:?}"
            );
            // Nobody is ever sure to within a tenth of a trick, and nobody
            // is ever clueless to within half a deal.
            assert!(
                (0.1..=5.0).contains(&g.sd),
                "{strain:?} {who:?} implausible sd: {g:?}"
            );
        }
    }
}

/// A strong hand should not read the same as a bust one — the net must be
/// looking at the hand block, not just the (identical) ranges.
#[test]
fn strength_moves_the_estimate() {
    let ctx = Context::new(RelativeVulnerability::NONE, &[]);
    let inf = Inferences::read(&ctx);
    let strong = trick_estimates(hand("AKQJ.AKQ.AKQ.AKQ"), &inf);
    let weak = trick_estimates(hand("8432.7532.652.32"), &inf);
    let notrump = |e: &TrickEstimates| e.get(Strain::Notrump, Relative::Me).mean;
    assert!(
        notrump(&strong) > notrump(&weak) + 3.0,
        "strong {} vs weak {}",
        notrump(&strong),
        notrump(&weak)
    );
}

/// Φ against textbook values, and the CDF's contract at the mean.
#[test]
fn normal_cdf_is_accurate() {
    for (z, want) in [
        (-3.0, 0.001_350),
        (-1.96, 0.025_000),
        (-1.0, 0.158_655),
        (0.0, 0.500_000),
        (1.0, 0.841_345),
        (1.96, 0.975_000),
        (3.0, 0.998_650),
    ] {
        let got = standard_normal_cdf(z);
        assert!((got - want).abs() < 1e-5, "Φ({z}) = {got}, want {want}");
    }
}

#[test]
fn p_at_least_reads_off_the_gaussian() {
    let g = Gaussian {
        mean: 10.0,
        sd: 1.5,
    };
    // Exactly at the half-trick correction: μ − 0.5 is a third of a σ below
    // the mean, so making ten is a shade better than even money.
    assert!((g.p_at_least(10) - 0.630_559).abs() < 1e-5);
    assert!((g.cdf(10.0) - 0.5).abs() < 1e-6);
    // Monotone, and saturating in both directions.
    assert!(g.p_at_least(7) > g.p_at_least(10));
    assert!(g.p_at_least(10) > g.p_at_least(13));
    assert!(g.p_at_least(0) > 0.999);
}
