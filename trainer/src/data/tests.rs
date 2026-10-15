use super::*;

/// A v3+ prefix is all the board scan reads: hand 10 | context 36.
const WIDTH: usize = 46;

/// One decision row.  `hand` stands for the acting hand's summary block;
/// `bid` is the contract-to-beat flag and `seat` is `auction.len() % 4`.
fn row(hand: f32, bid: bool, seat: usize) -> Vec<f32> {
    let mut row = vec![0.0; WIDTH];
    row[HAND.start] = hand;
    row[BID_PRESENT] = f32::from(bid);
    row[SEAT_DEALER + seat] = 1.0;
    row
}

/// One table of `n` decisions: the dealer opens (holding `dealer`), then
/// everyone acts over a bid.  Non-dealer hands are distinct from every dealer.
fn table(dealer: f32, n: usize) -> Vec<Vec<f32>> {
    (0..n)
        .map(|i| match i {
            0 => row(dealer, false, 0),
            _ => row(-(i as f32), true, i % 4),
        })
        .collect()
}

fn dataset(version: u32, rows: &[Vec<f32>]) -> Dataset {
    let n = rows.len();
    let meta: Meta = serde_json::from_value(serde_json::json!({
        "feature_version": version,
        "features_len": WIDTH,
        "softmax_len": SOFTMAX_LEN,
        "row_len": WIDTH + SOFTMAX_LEN,
        "seed": 0,
        "rows": n,
        "contested_rows": 0,
    }))
    .unwrap();
    Dataset {
        features: rows.concat(),
        targets: vec![0.0; n * SOFTMAX_LEN],
        dd: Vec::new(),
        tags: vec![0; n],
        seq: Vec::new(),
        seq_row: 0,
        rows: n,
        features_len: WIDTH,
        dd_len: 0,
        meta,
    }
}

fn split(d: Dataset, val_frac: f64, row_split: bool) -> usize {
    mix(&["d".to_string()], vec![d], val_frac, 4, row_split)
        .unwrap()
        .1
}

/// Ten boards of seven rows; a 15% tail would start at row 59, mid board 8.
fn plain() -> Vec<Vec<f32>> {
    (0..10).flat_map(|b| table(b as f32 + 1.0, 7)).collect()
}

#[test]
fn the_tail_starts_at_the_next_board() {
    assert_eq!(split(dataset(6, &plain()), 0.15, false), 63);
}

#[test]
fn row_split_keeps_the_historical_cut() {
    assert_eq!(split(dataset(6, &plain()), 0.15, true), 59);
}

#[test]
fn a_nominal_cut_on_a_board_start_stays_put() {
    // 70 rows, 10% → nominal 63, which is board 9's first row.
    assert_eq!(split(dataset(8, &plain()), 0.10, false), 63);
}

#[test]
fn a_replayed_board_is_not_split_between_its_tables() {
    // Ten boards, two tables of three rows each, the dealer's hand shared.
    let rows: Vec<_> = (0..10)
        .flat_map(|b| [table(b as f32 + 1.0, 3), table(b as f32 + 1.0, 3)].concat())
        .collect();
    // 25% → nominal 45: board 7's second table.  Board 8 starts at 48.
    assert_eq!(split(dataset(6, &rows), 0.25, false), 48);
}

#[test]
fn a_board_without_an_opening_row_is_skipped_not_split() {
    let mut rows = plain();
    // Board 9 (rows 63..70): the dealer's decision was a forced pass, so the
    // table starts at seat 1 with no bid, and no row proves the board start.
    rows[63] = row(-100.0, false, 1);
    let d = dataset(6, &rows);
    assert_eq!(d.board_start_from(59), None);
    // Falls back to the nominal cut (with a warning) rather than guessing.
    assert_eq!(split(d, 0.15, false), 59);
}

#[test]
fn layouts_before_v3_fall_back_to_the_nominal_row() {
    assert_eq!(dataset(2, &plain()).board_start_from(1), None);
    assert_eq!(split(dataset(2, &plain()), 0.15, false), 59);
}

#[test]
fn each_dump_in_a_mixture_is_aligned_on_its_own() {
    // Dump b: twenty boards of five rows; 15% → nominal 85, board start 85.
    let b: Vec<_> = (0..20).flat_map(|i| table(i as f32 + 50.0, 5)).collect();
    let (ds, ntrain) = mix(
        &["a".to_string(), "b".to_string()],
        vec![dataset(6, &plain()), dataset(6, &b)],
        0.15,
        4,
        false,
    )
    .unwrap();
    assert_eq!(ntrain, 63 + 85);
    // The held-out region is a's tail (board 9) then b's (board 17 on).
    let opening = |r: usize| ds.features[r * WIDTH];
    assert_eq!(opening(ntrain), 10.0);
    assert_eq!(opening(ntrain + 7), 67.0);
}
