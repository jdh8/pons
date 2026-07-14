//! Compact binary DD database format (`.pdd`).
//!
//! A `.pdd` file is the 8-byte [`MAGIC`](crate::pdd::MAGIC) followed by fixed
//! 34-byte rows, one per deal — 2.6× smaller than GIB text (34 vs 89 bytes)
//! and decoded with `from_le_bytes` instead of PBN parsing. Row layout,
//! little-endian throughout:
//!
//! - 3 × u64: North, East, South as [`Hand::to_bits`](contract_bridge::Hand::to_bits)
//!   (suits ♣,♦,♥,♠ low to high, rank bits 2..=14 per 16-bit holding). West is
//!   the complement.
//! - 5 × u16: trick rows in [`Strain`](contract_bridge::Strain) discriminant
//!   order (♣,♦,♥,♠,NT); each packs seat nibbles N,E,S,W at bits `4 × seat`,
//!   raw trick counts 0..=13 (no GIB `13 − tricks` folding).
//!
//! Decoding validates for free:
//! [`Hand::from_bits`](contract_bridge::Hand::from_bits) rejects stray rank
//! bits, [`Builder::build_full`](contract_bridge::Builder::build_full) rejects
//! any non-partition, and [`TrickCountRow::try_new`](ddss::TrickCountRow::try_new)
//! rejects nibbles above 13.

use crate::gib;
use contract_bridge::{Builder, FullDeal, Hand, Seat};
use ddss::{TrickCountRow, TrickCountTable};
use std::io;
use std::path::Path;

/// File magic; the trailing digits version the format.
pub const MAGIC: [u8; 8] = *b"ponsDD01";

/// Bytes per row: three hand words plus five trick-row words.
pub const ROW_LEN: usize = 34;

/// The three stored seats, in row order; West is reconstructed on decode.
const STORED_SEATS: [Seat; 3] = [Seat::North, Seat::East, Seat::South];

/// Seats in nibble order within a trick-row word (nibble `i` = seat `i`).
const SEATS: [Seat; 4] = [Seat::North, Seat::East, Seat::South, Seat::West];

/// Encode one deal and its DD table as a fixed-width row.
#[must_use]
pub fn encode_row(deal: &FullDeal, table: &TrickCountTable) -> [u8; ROW_LEN] {
    let mut row = [0; ROW_LEN];
    for (chunk, seat) in row.chunks_exact_mut(8).zip(STORED_SEATS) {
        chunk.copy_from_slice(&deal[seat].to_bits().to_le_bytes());
    }
    for (chunk, tricks) in row[24..].chunks_exact_mut(2).zip(table.0) {
        let bits = SEATS.into_iter().enumerate().fold(0u16, |acc, (i, seat)| {
            acc | u16::from(tricks.get(seat).get()) << (4 * i)
        });
        chunk.copy_from_slice(&bits.to_le_bytes());
    }
    row
}

/// Decode one row into its deal and double-dummy table.
///
/// Returns `None` if the hand words are not a partition of the deck or a
/// trick nibble exceeds 13 — the inverse of [`encode_row`], mirroring
/// [`gib::parse_line`].
#[must_use]
pub fn decode_row(row: &[u8; ROW_LEN]) -> Option<(FullDeal, TrickCountTable)> {
    let word = |i: usize| u64::from_le_bytes(row[8 * i..8 * i + 8].try_into().unwrap());
    let (n, e, s) = (word(0), word(1), word(2));
    let deal = Builder::new()
        .north(Hand::from_bits(n)?)
        .east(Hand::from_bits(e)?)
        .south(Hand::from_bits(s)?)
        // Each word is a subset of the deck, so the XOR is too; build_full
        // rejects overlaps between the stored hands (West then exceeds 13).
        .west(Hand::from_bits_retain(Hand::ALL.to_bits() ^ (n | e | s)))
        .build_full()
        .ok()?;
    let mut table = TrickCountTable([TrickCountRow::new(0, 0, 0, 0); 5]);
    for (chunk, tricks) in row[24..].chunks_exact(2).zip(&mut table.0) {
        let bits = u16::from_le_bytes(chunk.try_into().unwrap());
        let nib = |i: u16| (bits >> (4 * i) & 15) as u8;
        *tricks = TrickCountRow::try_new(nib(0), nib(1), nib(2), nib(3)).ok()?;
    }
    Some((deal, table))
}

/// Decode a whole DD database, sniffing the format.
///
/// Bytes starting with [`MAGIC`] are `.pdd` rows — a truncated tail or an
/// invalid row is an [`io::ErrorKind::InvalidData`] error. Anything else is
/// treated as GIB text, permissively skipping unparsable lines like every
/// existing consumer.
pub fn from_bytes(bytes: &[u8]) -> io::Result<Vec<(FullDeal, TrickCountTable)>> {
    let invalid = |what| io::Error::new(io::ErrorKind::InvalidData, what);
    let Some(rows) = bytes.strip_prefix(&MAGIC) else {
        let text = str::from_utf8(bytes).map_err(|_| invalid("neither .pdd nor GIB text"))?;
        return Ok(text.lines().filter_map(gib::parse_line).collect());
    };
    if !rows.len().is_multiple_of(ROW_LEN) {
        return Err(invalid("truncated .pdd file"));
    }
    rows.chunks_exact(ROW_LEN)
        .map(|row| decode_row(row.try_into().unwrap()).ok_or_else(|| invalid("corrupt .pdd row")))
        .collect()
}

/// Read a DD database file in either format ([`from_bytes`] on its contents).
pub fn load(path: impl AsRef<Path>) -> io::Result<Vec<(FullDeal, TrickCountTable)>> {
    from_bytes(&std::fs::read(path)?)
}

#[cfg(test)]
mod tests {
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
}
