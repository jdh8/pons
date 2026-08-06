use super::*;

/// The first line of `sol100000.txt`.
const LINE: &str =
    "T5.K4.652.A98542 K6.QJT976.QT7.Q6 432.A.AKJ93.JT73 AQJ987.8532.84.K:65658888888843433232";

#[test]
fn line_round_trips() {
    assert_eq!(LINE.len(), 88);
    let (deal, table) = parse_line(LINE).expect("valid GIB line");
    assert_eq!(format_line(&deal, &table), LINE);
}

#[test]
fn rejects_malformed() {
    assert!(parse_line("").is_none());
    assert!(parse_line(&LINE[..87]).is_none());
}

#[test]
fn label_is_seat_relative() {
    let (_, table) = parse_line(LINE).expect("valid GIB line");
    let label = relativized_tricks(&table, Seat::North);
    assert_eq!(label.len(), 20);
    // Strain index 1 = Spades → offset 4; slot 0 = "me" (North), 8 tricks.
    assert!((label[4] - 8.0 / 13.0).abs() < 1e-6);
    // Slot 1 = "lho" (East) spades; E/W stored as 13−tricks, decoded to 5.
    assert!((label[5] - 5.0 / 13.0).abs() < 1e-6);
}
