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
