use super::*;

/// The first line of `sol100000.txt` (same fixture as the `gib` tests).
const LINE: &str =
    "T5.K4.652.A98542 K6.QJT976.QT7.Q6 432.A.AKJ93.JT73 AQJ987.8532.84.K:65658888888843433232";

fn fixture() -> (FullDeal, TrickCountTable) {
    gib::parse_line(LINE).expect("valid GIB line")
}

#[test]
fn row_round_trips() {
    let (deal, table) = fixture();
    let row = encode_row(&deal, &table);
    assert_eq!(decode_row(&row), Some((deal, table)));
}

#[test]
fn rejects_corrupt_rows() {
    let (deal, table) = fixture();
    let clean = encode_row(&deal, &table);

    // A rank bit outside 2..=14 is not a card.
    let mut row = clean;
    row[0] |= 1;
    assert_eq!(decode_row(&row), None);

    // North duplicated into East is no longer a partition.
    let mut row = clean;
    let north: [u8; 8] = clean[..8].try_into().unwrap();
    row[8..16].copy_from_slice(&north);
    assert_eq!(decode_row(&row), None);

    // Trick nibbles above 13.
    let mut row = clean;
    row[24] = 0xFF;
    assert_eq!(decode_row(&row), None);
}

#[test]
fn counts_whole_rows() {
    let head = MAGIC.len() as u64;
    // Shorter than the magic, or a bare header: no rows, no underflow.
    assert_eq!(rows_in(0), 0);
    assert_eq!(rows_in(head - 1), 0);
    assert_eq!(rows_in(head), 0);
    // Exact multiples, and every ragged tail rounding down to them.
    for rows in 1..4u64 {
        let clean = head + rows * ROW_LEN as u64;
        assert_eq!(rows_in(clean), rows);
        assert_eq!(rows_in(clean - 1), rows - 1);
        assert_eq!(rows_in(clean + ROW_LEN as u64 - 1), rows);
    }
}

#[test]
fn slices_by_row() {
    let (deal, table) = fixture();
    let mut bin = MAGIC.to_vec();
    for _ in 0..3 {
        bin.extend_from_slice(&encode_row(&deal, &table));
    }
    let path = std::env::temp_dir().join("pons-pdd-slice-test.pdd");
    std::fs::write(&path, &bin).unwrap();

    assert_eq!(load_slice(&path, 0, 3).unwrap().len(), 3);
    assert_eq!(load_slice(&path, 1, 2).unwrap().len(), 2);
    // A slice past the tail returns the rows that exist.
    assert_eq!(load_slice(&path, 2, 5).unwrap(), [(deal, table)]);
    assert_eq!(load_slice(&path, 3, 5).unwrap(), []);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn sniffs_both_formats() {
    let (deal, table) = fixture();
    let mut bin = MAGIC.to_vec();
    bin.extend_from_slice(&encode_row(&deal, &table));
    assert_eq!(from_bytes(&bin).unwrap(), [(deal, table)]);
    assert_eq!(
        from_bytes(format!("{LINE}\n").as_bytes()).unwrap(),
        [(deal, table)]
    );
    assert!(from_bytes(&bin[..bin.len() - 1]).is_err());
}
