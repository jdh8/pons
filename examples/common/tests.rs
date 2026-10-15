use super::*;

#[test]
fn neighbouring_bases_share_no_boards() {
    // The retired `base + i` key slid: base + 3 replayed base's board i + 3.
    let legacy = |base: u64| {
        (0..16)
            .map(|i| full_deal(&mut StdRng::seed_from_u64(base + i)))
            .collect::<Vec<_>>()
    };
    assert_eq!(legacy(1003)[..13], legacy(1000)[3..]);

    let reference = seeded_deals(1000, 64);
    for k in 1..8 {
        let shifted = seeded_deals(1000 + k, 64);
        assert!(shifted.iter().all(|deal| !reference.contains(deal)));
    }
}

#[test]
fn a_base_replays_its_boards_in_order() {
    assert_eq!(seeded_deals(42, 8), seeded_deals(42, 8));
    assert_eq!(seeded_deals(42, 5)[..], seeded_deals(42, 8)[..5]);
}

#[test]
fn a_straddling_ci_is_non_inferior_only_inside_the_margin() {
    // K = 8 vs K = 4 (floor-sweep), plain DD both vul: lower bound −0.0001
    assert_eq!(verdict(0.0044, 0.0045, 1.0), "non-inferior");
    // ... and NV: lower bound −0.0031 ≤ −δ, so the data have not ruled out a loss
    assert_eq!(verdict(0.0008, 0.0039, 1.0), "inconclusive");
    assert_eq!(verdict(0.0050, 0.0040, 1.0), "win");
    assert_eq!(verdict(-0.0010, 0.0005, 1.0), "loss");
}

#[test]
fn an_enriched_run_scales_the_margin_by_density() {
    // 1 board in 10 accepted: δ per measured board is 0.02
    assert_eq!(verdict(0.0, 0.0150, 0.1), "non-inferior");
    assert_eq!(verdict(0.0, 0.0250, 0.1), "inconclusive");
}

#[test]
fn a_ci_without_width_proves_nothing() {
    // Under two boards there is no variance: the CI is unbounded, not zero-width
    assert_eq!(mean_with_ci(&[]), (0.0, f64::INFINITY));
    assert_eq!(mean_with_ci(&[3]), (3.0, f64::INFINITY));
    assert_eq!(mean_with_ci_f64(&[0.5]), (0.5, f64::INFINITY));
    let (mean, ci) = mean_with_ci(&[0]);
    assert_eq!(verdict(mean, ci, 1.0), "inconclusive");
    assert_eq!(verdict(f64::NAN, 0.0, 1.0), "inconclusive");
    assert_eq!(verdict(0.0, f64::NAN, 1.0), "inconclusive");
}

#[test]
fn density_must_be_a_share_of_random_deals() {
    assert_eq!(parse_density("1"), Ok(1.0));
    assert_eq!(parse_density("0.004"), Ok(0.004));
    for bad in ["0", "-0.1", "1.5", "NaN", "inf", "x"] {
        assert!(parse_density(bad).is_err(), "{bad}");
    }
}

#[test]
#[should_panic(expected = "not a share of random deals")]
fn a_zero_density_cannot_reach_a_verdict() {
    // δ / 0 = ∞ would label any straddling CI non-inferior
    let _ = verdict(0.0, 1.0, 0.0);
}

#[test]
fn a_shared_reference_cancels_in_points_but_not_in_imps() {
    // #8: S_ref = 0, S_on = 420, S_off = 400
    assert_eq!(direct_imps(420, 400), 1);
    assert_eq!(reference_gain_imps(420, 0, 400, 0), 0);
}

#[test]
fn the_two_metrics_agree_when_the_baseline_is_the_reference() {
    // R = S_off: the gain is IMP(S_on − S_off) − IMP(0), the direct A/B
    assert_eq!(
        direct_imps(420, 400),
        reference_gain_imps(420, 400, 400, 400)
    );
    assert_eq!(
        direct_imps(-100, 620),
        reference_gain_imps(-100, 620, 620, 620)
    );
    // An unchanged board is 0 on both
    assert_eq!(direct_imps(400, 400), reference_gain_imps(400, 0, 400, 0));
}

#[test]
fn a_reference_gain_reprices_every_contract_on_one_table() {
    use ddss::TrickCountRow;
    // N/S take 10 tricks in spades, 9 at notrump, 7 elsewhere
    let mut table = TrickCountTable([TrickCountRow::new(7, 6, 7, 6); 5]);
    table.0[Strain::Spades as usize] = TrickCountRow::new(10, 3, 10, 3);
    table.0[Strain::Notrump as usize] = TrickCountRow::new(9, 4, 9, 4);
    let reached = |level, strain| {
        Some((
            Contract::new(level, strain, contract_bridge::Penalty::Undoubled),
            Seat::North,
        ))
    };
    let four_spades = reached(4, Strain::Spades);
    let three_notrump = reached(3, Strain::Notrump);
    let vul = AbsoluteVulnerability::NONE;

    // One board: on reaches 4♠ (420), off 3NT (400), the reference passes out
    let direct = score_solved(
        &[(four_spades, three_notrump)],
        vec![0],
        vec![table],
        vul,
        ns_score_contract,
    );
    assert_eq!((direct.total_points, direct.total_imps), (20, 1));
    let gain = score_reference_solved(
        &[((four_spades, None), (three_notrump, None))],
        vec![0],
        vec![table],
        vul,
        ns_score_contract,
    );
    assert_eq!((gain.total_points, gain.total_imps), (20, 0));

    // Control: the reference is off's own 3NT, and the metrics agree
    let control = score_reference_solved(
        &[((four_spades, three_notrump), (three_notrump, three_notrump))],
        vec![0],
        vec![table],
        vul,
        ns_score_contract,
    );
    assert_eq!((control.total_points, control.total_imps), (20, 1));
}

#[test]
fn a_moved_reference_alone_diverges_a_board() {
    let one_club = Some((
        Contract::new(1, Strain::Clubs, contract_bridge::Penalty::Undoubled),
        Seat::East,
    ));
    let still: (ArmVsReference, ArmVsReference) = ((None, None), (None, None));
    let moved = ((None, None), (None, one_club));
    assert_eq!(divergent_boards(&[still, moved]), [1]);
}
