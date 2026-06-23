//! GIB hand-record format: `<West-first PBN>:<20 hex DD digits>`.
//!
//! Each line of a GIB deal database (e.g. `../ddss-sys/vendor/hands/sol100000.txt`)
//! is exactly 88 ASCII chars: a 67-char West-first PBN deal, a `:`, then the
//! 20-hex-digit double-dummy tail. The tail encoding (strain order `NT,S,H,D,C`,
//! declarers `E,N,W,S`, E/W stored as `13 − tricks`) lives in
//! [`ddss::TrickCountTable::gib`]/[`from_gib`](ddss::TrickCountTable::from_gib);
//! this module is the line-level glue plus the per-row training label.
//!
//! Reading these files costs no double-dummy solving — the expensive part is
//! cached in the tail — so a teacher dump or evaluator calibration over the 100K
//! database is essentially free I/O.

use contract_bridge::{FullDeal, Seat, Strain};
use ddss::TrickCountTable;

/// Strains in the order the relativized DD label lists them (the GIB tail order).
const STRAINS: [Strain; 5] = [
    Strain::Notrump,
    Strain::Spades,
    Strain::Hearts,
    Strain::Diamonds,
    Strain::Clubs,
];

/// Parse one GIB line into its deal and double-dummy table.
///
/// Returns `None` for any line that is not exactly 88 chars or whose PBN prefix
/// does not parse — so callers can `lines().filter_map(parse_line)` over a file
/// with a trailing newline or stray blank lines.
#[must_use]
pub fn parse_line(line: &str) -> Option<(FullDeal, TrickCountTable)> {
    if line.len() != 88 {
        return None;
    }
    let deal = format!("W:{}", &line[..67]).parse().ok()?;
    let table = TrickCountTable::from_gib(&line.as_bytes()[68..]);
    Some((deal, table))
}

/// Format a deal and its DD table as one GIB line (88 chars, no newline).
///
/// The inverse of [`parse_line`]: `deal.display(Seat::West)` yields the
/// `"W:<67-char body>"` PBN, whose `"W:"` tag we strip before appending the
/// 20-hex tail.
#[must_use]
pub fn format_line(deal: &FullDeal, table: &TrickCountTable) -> String {
    let pbn = deal.display(Seat::West).to_string();
    let body = &pbn[2..]; // drop the "W:" dealer tag
    format!("{body}:{:X}", table.gib())
}

/// The per-row training label: the full 20-cell DD table re-oriented to `seat`.
///
/// Per strain (`NT,S,H,D,C`) the four makeable-trick counts `[me, lho, partner,
/// rho]` ÷ 13. Re-orienting to the acting seat lines the label up with the
/// hand's own-perspective feature vector; all four seats are still present, so
/// no information is dropped.
#[must_use]
pub fn relativized_tricks(table: &TrickCountTable, seat: Seat) -> Vec<f32> {
    let mut out = Vec::with_capacity(STRAINS.len() * 4);
    for &strain in &STRAINS {
        let row = table[strain];
        for s in [seat, seat.lho(), seat.partner(), seat.rho()] {
            out.push(f32::from(row.get(s).get()) / 13.0);
        }
    }
    out
}

#[cfg(test)]
mod tests {
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
}
