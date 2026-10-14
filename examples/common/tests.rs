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
