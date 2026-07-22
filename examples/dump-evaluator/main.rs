//! Trick-evaluator corpus (bilans session C)
//!
//! Bids out pre-solved deals with our own books and records, at every decision
//! point, a training row of `(features, dd_tricks)` for the **trick evaluator**
//! — the net that answers "given my cards and range envelopes on the three
//! hidden hands, how many double-dummy tricks does each declarer take in each
//! strain?".
//!
//! - **features** — [`features_eval`][pons::bidding::features::features_eval]:
//!   54 floats of own-hand honour decomposition plus the LHO/partner/RHO range
//!   blocks read by [`Stance::infer`]. No auction, no seat, no vulnerability:
//!   the auction enters only through the ranges, which is what makes the
//!   evaluator bidding-system agnostic. `--encoding onehot` swaps the 24-float
//!   hand block for 52 card bits (the texture ablation), same walk;
//!   `--encoding bits` emits the 79-float research superset (honour bits, a
//!   spot count, and a width beside every range pair) for a featurization
//!   sweep. All three walk the same auctions and differ only in this row.
//! - **dd_tricks** — the deal's cached double-dummy table re-oriented to the
//!   acting seat ([`gib::relativized_tricks`]): 20 targets, strain-major in GIB
//!   order (NT,♠,♥,♦,♣) × declarer `[me, lho, partner, rho]`. This is ground
//!   truth on the actual deal, not a teacher's opinion, so distillation bias
//!   cannot enter.
//!
//! **No solver and no EPBot run here.** The labels are already in the `.pdd`
//! stock (`/nfs2/jdh8/*.pdd`, ~94M solved deals); the only work is bidding.
//!
//! ```text
//! cargo run --release --example dump-evaluator -- \
//!     --deals /nfs2/jdh8/22.pdd --count 100000 --seed $(date +%s)
//! ```
//!
//! Output is a flat little-endian `f32` file of `features_len + 20` floats per
//! row, a JSON sidecar pinning the layout, and a sibling `.tags` byte per row
//! (bit 0 = contested phase, bit 1 = system index). The loop is **deal-major**
//! so a contiguous validation split stays deal-disjoint — the ~10 rows a board
//! contributes all share one DD label, and a shuffled split would leak it.

use clap::Parser;
use contract_bridge::auction::{Auction, Call, RelativeVulnerability};
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Rank, Seat, Suit};
use ddss::TrickCountTable;
use pons::bidding::context::{Context, relative};
use pons::bidding::features::{
    FEATURES_LEN_EVAL, FEATURES_VERSION_EVAL, LEN_HAND_EVAL, LEN_HAND_V3, features_eval,
    features_v3,
};
use pons::bidding::{Family, Inferences, Phase, Stance, System};
use pons::{american, dutch, gib};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use std::io::{BufWriter, Write};

/// Width of the double-dummy label: 5 strains × 4 declarers.
const DD_LEN: usize = 20;

/// Width of the `--encoding onehot` hand block: 4 suits × 13 ranks.
const LEN_HAND_ONEHOT: usize = 52;

/// Width of one suit's `--encoding bits` block: `len/13`, `#spots/8`,
/// `suit_hcp/10`, then one bit each for A, K, Q, J, T.
const LEN_SUIT_BITS: usize = 8;

/// The five honours `--encoding bits` flags per suit.  Everything else in a
/// suit is a spot card — ranks 2..9, hence the divisor 8 rather than 13.
const HONOURS: [Rank; 5] = [Rank::A, Rank::K, Rank::Q, Rank::J, Rank::T];

/// Width of the `--encoding bits` hand block: 4 suits × [`LEN_SUIT_BITS`] plus
/// the two globals (`hcp/40`, `upgrade/2`) taken verbatim from `features_v3`.
const LEN_HAND_BITS: usize = 4 * LEN_SUIT_BITS + 2;

/// Width of `features_eval`'s range tail: 3 hidden seats × 10, i.e. 15
/// `(min, max)` pairs.  `--encoding bits` widens each into a
/// `(min, max, max − min)` triple, so its tail is 45 instead.
const LEN_RANGES: usize = FEATURES_LEN_EVAL - LEN_HAND_EVAL;

/// Phase-3 honour oracle (`--oracle`, `bits` only): partner's *true* per-strain
/// keycards (aces + trump-K, `/5`) for the four suit strains, then the four
/// trump-Q bits.  A truth column that upper-bounds any projected `keycards`
/// axis — if even this washes on the slam slice, the axis is dead.
const ORACLE_LEN: usize = 8;

/// Hidden-seat axis survey (`--oracle-all`): the keycard oracle verbatim, then
/// per-axis truth blocks for all three hidden seats in `features_eval` order
/// [LHO, partner, RHO], suits in `Suit::ASC` within a seat.  Axis-major so
/// every trainer arm's mask is one contiguous range:
///
/// - **Q**uality, 12 = 3×4: per-suit `suit_hcp/10`
/// - **S**hortness, 12 = 3×4: per-suit `len ≤ 1` bit
/// - **C**ontrols, 24 = 3×8: per-suit ace bit then king bit
/// - **St**opper, 12 = 3×4: per-suit A/Kx/Qxx/Jxxx bit
///
/// Per-suit truth, never "the shown suit" — collapsing onto a shown or agreed
/// suit would manufacture the fit-indicator product the projection design
/// forbids, and the 20 outputs are already strain-indexed.
const ORACLE_ALL_LEN: usize = ORACLE_LEN + 3 * (4 + 4 + 8 + 4);

/// Own-hand encoding selected by `--encoding`
#[derive(Clone, Copy)]
enum Encoding {
    /// `features_eval`'s 24-float honour block, verbatim
    Summary,
    /// 52 card bits in place of the hand block — the texture ablation
    Onehot,
    /// The 79-float research superset: per-suit honour bits and spot count,
    /// plus a width beside every range pair
    Bits,
}

#[derive(Parser)]
#[command(about = "Dump (features, dd_tricks) rows for the trick evaluator")]
struct Args {
    /// Pre-solved deal database: binary `.pdd` (sliceable) or GIB text
    #[arg(long)]
    deals: String,
    /// Skip this many deals before reading (shards a multi-gigabyte database)
    #[arg(long, default_value_t = 0)]
    skip: u64,
    /// Number of deals to bid out
    #[arg(long, default_value_t = 100_000)]
    count: usize,
    /// RNG seed for the dealer/vulnerability stream
    #[arg(long, default_value_t = 0)]
    seed: u64,
    /// Comma-separated books to bid each deal with. Pooling systems widens the
    /// range-shape coverage; the physics being learned is the same for all.
    #[arg(long, default_value = "american,dutch")]
    systems: String,
    /// Own-hand encoding: `summary` (24 honour floats), `onehot` (52 card
    /// bits — the texture ablation), or `bits` (the 79-float research superset)
    #[arg(long, default_value = "summary")]
    encoding: String,
    /// Append the Phase-3 honour oracle: 8 columns of partner's *true*
    /// per-strain keycards + trump-Q. Requires `--encoding bits`; the trainer's
    /// `ben-oracle`/`baseline-drop-both-oracle` arms read them, every other arm
    /// masks them off.
    #[arg(long)]
    oracle: bool,
    /// Append the full hidden-seat axis-survey oracle: the 8 keycard columns
    /// plus quality/shortness/controls/stopper truth for all three hidden
    /// seats (68 columns total). Requires `--encoding bits`; supersets
    /// `--oracle`. One corpus serves every survey arm — the trainer masks.
    #[arg(long)]
    oracle_all: bool,
    /// Output path stem; writes `<out>.f32`, `<out>.json`, `<out>.tags`
    #[arg(long, default_value = "target/evaluator-data")]
    out: String,
}

/// The four absolute vulnerabilities, sampled uniformly per board.
const VULS: [AbsoluteVulnerability; 4] = [
    AbsoluteVulnerability::NONE,
    AbsoluteVulnerability::NS,
    AbsoluteVulnerability::EW,
    AbsoluteVulnerability::ALL,
];

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let encoding = match args.encoding.as_str() {
        "summary" => Encoding::Summary,
        "onehot" => Encoding::Onehot,
        "bits" => Encoding::Bits,
        other => anyhow::bail!("--encoding must be summary|onehot|bits, got {other:?}"),
    };
    let base_len = match encoding {
        Encoding::Summary => FEATURES_LEN_EVAL,
        Encoding::Onehot => LEN_HAND_ONEHOT + LEN_RANGES,
        Encoding::Bits => LEN_HAND_BITS + LEN_RANGES / 2 * 3,
    };
    anyhow::ensure!(
        !(args.oracle || args.oracle_all) || matches!(encoding, Encoding::Bits),
        "--oracle/--oracle-all only extend the `bits` superset the trainer arms mask over"
    );
    let features_len = base_len
        + if args.oracle_all {
            ORACLE_ALL_LEN
        } else if args.oracle {
            ORACLE_LEN
        } else {
            0
        };
    let row_len = features_len + DD_LEN;

    let systems: Vec<(&str, Stance)> = args
        .systems
        .split(',')
        .map(|name| match name.trim() {
            "american" => Ok(("american", american().against(Family::NATURAL))),
            "dutch" => Ok(("dutch", dutch().against(Family::NATURAL))),
            other => anyhow::bail!("--systems entries must be american|dutch, got {other:?}"),
        })
        .collect::<anyhow::Result<_>>()?;
    anyhow::ensure!(systems.len() <= 2, "the tag byte holds two system slots");

    let deals = load_deals(&args.deals, args.skip, args.count)?;
    eprintln!(
        "evaluator-dump: {} deals × {} systems, {features_len} features + {DD_LEN} labels",
        deals.len(),
        systems.len()
    );

    let mut rng = StdRng::seed_from_u64(args.seed);
    let f32_path = format!("{}.f32", args.out);
    let mut writer = BufWriter::new(std::fs::File::create(&f32_path)?);
    let mut tags = BufWriter::new(std::fs::File::create(format!("{}.tags", args.out))?);

    let (mut rows, mut contested, mut forced_pass) = (0u64, 0u64, 0u64);
    let mut row = vec![0f32; row_len];

    // Deal-major: every row a board contributes stays contiguous, so the
    // trainer's contiguous validation tail is deal-disjoint.
    for (deal, table) in &deals {
        let dealer = rng.random_range(0..4usize);
        let vul = VULS[rng.random_range(0..4usize)];
        for (sys_idx, (_, stance)) in systems.iter().enumerate() {
            let mut auction = Auction::new();
            while !auction.has_ended() {
                let seat = Seat::ALL[(dealer + auction.len()) % 4];
                let hand = deal[seat];
                let rel = relative(vul, seat);

                let Some(mut logits) = stance.classify(hand, rel, &auction) else {
                    forced_pass += 1;
                    auction.push(Call::Pass);
                    continue;
                };
                for (call, slot) in logits.iter_mut() {
                    if auction.can_push(call).is_err() {
                        *slot = f32::NEG_INFINITY;
                    }
                }

                // The trie-prefixed reading, so conventional calls decode off
                // their authoring rules rather than as natural suits.
                let inferences = stance.infer(rel, &auction);
                encode(&mut row[..base_len], hand, &inferences, encoding);
                if args.oracle_all {
                    write_oracle_all(&mut row[base_len..features_len], deal, seat);
                } else if args.oracle {
                    write_oracle(&mut row[base_len..features_len], deal[seat.partner()]);
                }
                row[features_len..].copy_from_slice(&gib::relativized_tricks(table, seat));
                for value in &row {
                    writer.write_all(&value.to_le_bytes())?;
                }

                let contested_row = Phase::of(&auction) != Phase::Constructive;
                tags.write_all(&[u8::from(contested_row) | (sys_idx as u8) << 1])?;
                rows += 1;
                contested += u64::from(contested_row);

                auction.push(argmax_legal(&logits));
            }
        }
    }
    writer.flush()?;
    tags.flush()?;

    let metadata = serde_json::json!({
        "feature_version": FEATURES_VERSION_EVAL,
        "features_len": features_len,
        "dd_len": DD_LEN,
        "row_len": row_len,
        "row_bytes": row_len * 4,
        "dtype": "f32-le",
        "encoding": args.encoding,
        "oracle": args.oracle,
        "oracle_all": args.oracle_all,
        "layout": format!("row = [{features_len} features][{DD_LEN} dd_tricks]"),
        "label_order": "strain-major NT,S,H,D,C × declarer [me,lho,partner,rho], tricks/13",
        "tags": "sibling .tags: one u8 per row, bit 0 = contested phase, bit 1 = system index",
        "systems": systems.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
        "deals": args.deals,
        "skip": args.skip,
        "count": args.count,
        "boards": deals.len(),
        "git_sha": git_sha(),
        "seed": args.seed,
        "rows": rows,
        "contested_rows": contested,
        "forced_pass_decisions": forced_pass,
    });
    std::fs::write(format!("{}.json", args.out), format!("{metadata:#}\n"))?;

    eprintln!(
        "evaluator-dump: {rows} rows → {f32_path} ({:.1} MB), {:.0}% contested, \
         {forced_pass} forced passes.",
        (rows as usize * row_len * 4) as f64 / 1e6,
        if rows == 0 {
            0.0
        } else {
            100.0 * contested as f64 / rows as f64
        },
    );
    Ok(())
}

/// Write one feature row: the hand block (summary, 52 card bits, or the `bits`
/// honour/spot decomposition) followed by the three hidden seats' range blocks,
/// which `features_eval` already lays out.
fn encode(out: &mut [f32], hand: Hand, inferences: &Inferences, encoding: Encoding) {
    let feats = features_eval(hand, inferences);
    let (hand_block, ranges) = feats.split_at(LEN_HAND_EVAL);
    let cut = match encoding {
        Encoding::Summary => {
            out[..LEN_HAND_EVAL].copy_from_slice(hand_block);
            LEN_HAND_EVAL
        }
        Encoding::Onehot => {
            for (slot, (suit, rank)) in out.iter_mut().zip(
                Suit::ASC
                    .into_iter()
                    .flat_map(|s| (2..=14).map(move |r| (s, r))),
            ) {
                *slot = f32::from(hand[suit].contains(Rank::new(rank)));
            }
            LEN_HAND_ONEHOT
        }
        Encoding::Bits => {
            // `len`, `suit_hcp`, and the two globals come from `features_v3`:
            // `features_eval`'s block is the honour decomposition and no longer
            // carries them, and `upgrade` is not public API to recompute.  An
            // empty `Context` is correct — the v3 hand block reads the hand
            // alone and never the auction.
            let v3 = features_v3(hand, &Context::new(RelativeVulnerability::NONE, &[]));
            // `v3[..LEN_HAND_V3]` is 4 `(len, suit_hcp)` pairs then the 2
            // globals; zipping `Suit::ASC` stops before the globals.
            for ((block, pair), suit) in out
                .chunks_exact_mut(LEN_SUIT_BITS)
                .zip(v3[..LEN_HAND_V3].chunks_exact(2))
                .zip(Suit::ASC)
            {
                let holding = hand[suit];
                // A suit holds any *subset* of the honours, so count what is
                // actually there; the rest of its length is spot cards.
                let held = HONOURS.map(|rank| holding.contains(rank));
                let spots = holding.len() - held.iter().filter(|&&h| h).count();
                block[0] = pair[0];
                block[1] = spots as f32 / 8.0;
                block[2] = pair[1];
                block[3..].copy_from_slice(&held.map(f32::from));
            }
            out[4 * LEN_SUIT_BITS..LEN_HAND_BITS].copy_from_slice(&v3[2 * 4..LEN_HAND_V3]);
            LEN_HAND_BITS
        }
    };
    let tail = &mut out[cut..];
    if matches!(encoding, Encoding::Bits) {
        // Widen every `(min, max)` into `(min, max, max − min)`.  The width
        // inherits its pair's normalisation, so nothing extra to divide by.
        for (triple, pair) in tail.chunks_exact_mut(3).zip(ranges.chunks_exact(2)) {
            triple.copy_from_slice(&[pair[0], pair[1], pair[1] - pair[0]]);
        }
    } else {
        tail.copy_from_slice(ranges);
    }
}

/// Write the [`ORACLE_LEN`] honour-oracle columns for `partner`'s actual hand:
/// four per-strain keycard counts (aces + trump-K, `/5`) in `Suit::ASC` order,
/// then the four trump-Q bits. All keycards regardless of strain, plus the
/// trump king — the RKCB census, which is the whole reach of the axis it bounds.
fn write_oracle(out: &mut [f32], partner: Hand) {
    let aces = Suit::ASC
        .into_iter()
        .filter(|&s| partner[s].contains(Rank::A))
        .count();
    let (keycards, queens) = out.split_at_mut(4);
    for (slot, suit) in keycards.iter_mut().zip(Suit::ASC) {
        *slot = (aces + usize::from(partner[suit].contains(Rank::K))) as f32 / 5.0;
    }
    for (slot, suit) in queens.iter_mut().zip(Suit::ASC) {
        *slot = f32::from(partner[suit].contains(Rank::Q));
    }
}

/// Write the [`ORACLE_ALL_LEN`] axis-survey columns: the keycard oracle for
/// partner verbatim, then quality, shortness, controls, and stopper truth for
/// the three hidden seats.  Layout documented at [`ORACLE_ALL_LEN`].
fn write_oracle_all(out: &mut [f32], deal: &FullDeal, seat: Seat) {
    write_oracle(&mut out[..ORACLE_LEN], deal[seat.partner()]);
    let hidden = [seat.lho(), seat.partner(), seat.rho()].map(|s| deal[s]);
    let holdings = hidden.iter().flat_map(|hand| Suit::ASC.map(|s| hand[s]));

    let (quality, rest) = out[ORACLE_LEN..].split_at_mut(12);
    let (shortness, rest) = rest.split_at_mut(12);
    let (controls, stoppers) = rest.split_at_mut(24);
    for (slot, h) in quality.iter_mut().zip(holdings.clone()) {
        let hcp = 4 * u8::from(h.contains(Rank::A))
            + 3 * u8::from(h.contains(Rank::K))
            + 2 * u8::from(h.contains(Rank::Q))
            + u8::from(h.contains(Rank::J));
        *slot = f32::from(hcp) / 10.0;
    }
    for (slot, h) in shortness.iter_mut().zip(holdings.clone()) {
        *slot = f32::from(h.len() <= 1);
    }
    for (pair, h) in controls.chunks_exact_mut(2).zip(holdings.clone()) {
        pair[0] = f32::from(h.contains(Rank::A));
        pair[1] = f32::from(h.contains(Rank::K));
    }
    for (slot, h) in stoppers.iter_mut().zip(holdings) {
        // The crisp textbook stopper — A, Kx, Qxx, or Jxxx — restated because
        // the crate's `has_stopper` is not public API.
        *slot = f32::from(
            h.contains(Rank::A)
                || (h.contains(Rank::K) && h.len() >= 2)
                || (h.contains(Rank::Q) && h.len() >= 3)
                || (h.contains(Rank::J) && h.len() >= 4),
        );
    }
}

/// The highest-logit finite (hence legal, after masking) call, defaulting to a
/// pass so the auction always terminates.
fn argmax_legal(logits: &pons::bidding::array::Logits) -> Call {
    logits
        .iter()
        .filter(|(_, l)| l.is_finite())
        .max_by(|a, b| a.1.partial_cmp(b.1).expect("logits are never NaN"))
        .map_or(Call::Pass, |(call, _)| call)
}

/// Load a slice of a pre-solved database: seek-based for binary `.pdd`, else
/// read the GIB text whole (it has no fixed row width to seek by) and slice.
fn load_deals(
    path: &str,
    skip: u64,
    count: usize,
) -> std::io::Result<Vec<(FullDeal, TrickCountTable)>> {
    pons::pdd::load_slice(path, skip, count).or_else(|_| {
        Ok(pons::pdd::load(path)?
            .into_iter()
            .skip(skip as usize)
            .take(count)
            .collect())
    })
}

/// Best-effort current commit, for the metadata sidecar; `"unknown"` on failure.
fn git_sha() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map_or_else(|| "unknown".to_string(), |s| s.trim().to_string())
}

/// The `--encoding bits` row is self-describing: 79 floats, each suit's
/// length is exactly its spots plus the honours it flags, each suit's HCP
/// is what those honour bits imply, and every range triple carries the
/// width of the pair it widened.  And `--encoding summary` is exactly the
/// subset of it the trainer's `--arm ben` keeps.
#[cfg(test)]
mod tests {
    use super::*;

    /// Ben-arm column offsets within a suit's [`LEN_SUIT_BITS`] block: `#spots`
    /// and the five honour flags, i.e. everything but `len` and `suit_hcp`.
    const BEN_SUIT: [usize; 6] = [1, 3, 4, 5, 6, 7];

    /// Ben-arm offsets within a `(min, max, width)` range triple: the width the
    /// `bits` encoding added is dropped again.
    const BEN_TRIPLE: [usize; 2] = [0, 1];

    /// The shared fixture: a void, an honourless suit, and two mixed holdings,
    /// so `#spots` is exercised as "length minus honours held" and not as a
    /// constant, under an auction that actually shows something.
    fn fixture() -> (Hand, Inferences) {
        let hand: Hand = "AT2.KQ98.J76543.".parse().expect("valid test hand");
        let auction: Vec<Call> = ["1S", "P", "2H"]
            .iter()
            .map(|c| c.parse().expect("valid test call"))
            .collect();
        let stance = american().against(Family::NATURAL);
        let vul = relative(AbsoluteVulnerability::NONE, Seat::North);
        (hand, stance.infer(vul, &auction))
    }

    /// The `bits` columns the trainer's `--arm ben` leaves live, re-derived from
    /// its offset table rather than transcribed as a 54-element literal.
    fn ben_live_columns() -> Vec<usize> {
        let mut cols = Vec::new();
        for suit in 0..4 {
            cols.extend(BEN_SUIT.map(|o| suit * LEN_SUIT_BITS + o));
        }
        // Columns 32 (`hcp/40`) and 33 (`upgrade/2`) are the globals the arm
        // drops, so the range triples follow immediately.
        for triple in 0..LEN_RANGES / 2 {
            cols.extend(BEN_TRIPLE.map(|o| LEN_HAND_BITS + 3 * triple + o));
        }
        cols
    }

    #[test]
    fn bits_row_is_self_consistent() {
        let (hand, inferences) = fixture();

        let mut row = vec![0f32; LEN_HAND_BITS + LEN_RANGES / 2 * 3];
        assert_eq!(row.len(), 79);
        encode(&mut row, hand, &inferences, Encoding::Bits);

        let (hand_block, triples) = row.split_at(LEN_HAND_BITS);
        for block in hand_block[..4 * LEN_SUIT_BITS].chunks_exact(LEN_SUIT_BITS) {
            let (spots, honours) = (block[1] * 8.0, &block[3..]);
            // Span identity: len = #spots + A + K + Q + J + T.  Both sides
            // divide by 13 rather than multiplying out, so the compare is exact.
            assert_eq!(block[0], (spots + honours.iter().sum::<f32>()) / 13.0);
            // Suit HCP is exactly what the honour bits say: 4A + 3K + 2Q + J.
            let hcp = 4.0 * honours[0] + 3.0 * honours[1] + 2.0 * honours[2] + honours[3];
            assert_eq!(block[2], hcp / 10.0);
        }

        // Every range pair survives verbatim and gains its width beside it.
        let feats = features_eval(hand, &inferences);
        assert_eq!(triples.len(), 45);
        for (triple, pair) in triples
            .chunks_exact(3)
            .zip(feats[LEN_HAND_EVAL..].chunks_exact(2))
        {
            assert_eq!(triple[..2], *pair);
            assert_eq!(triple[2], triple[1] - triple[0]);
        }
    }

    /// `features_eval` is now exactly the `ben` arm of the `bits` superset: the
    /// same 24 honour columns and the same 30 range bounds, in the same order.
    /// Nothing else checks that coupling, and it is silent when it breaks — the
    /// trainer would fit a net on one column order while the crate serves it
    /// another, permuted, with no width mismatch to trip over.  So gather the
    /// `bits` row at the arm's live columns and demand the `summary` row back.
    ///
    /// Exact float equality is right here: both sides are copies of the very
    /// same computed floats, not two roundings of one quantity.
    /// The oracle counts every ace as a keycard, adds the trump king per strain,
    /// and flags the trump queen — in `Suit::ASC` (♣♦♥♠) order, keycards then
    /// queens. `AKQJ.AK2.Q32.432` has two aces (♠♥), so ♥/♠ read 3 keycards
    /// (own trump king) and ♣/♦ read 2, with the queen only under ♦ and ♠.
    #[test]
    fn oracle_counts_partner_keycards_and_trump_queen() {
        let partner: Hand = "AKQJ.AK2.Q32.432".parse().expect("valid test hand");
        let mut out = [0f32; ORACLE_LEN];
        write_oracle(&mut out, partner);
        assert_eq!(out, [0.4, 0.4, 0.6, 0.6, 0.0, 1.0, 0.0, 1.0]);
    }

    /// The survey oracle, pinned cell by cell: keycard head verbatim (same
    /// fixture hand as [`oracle_counts_partner_keycards_and_trump_queen`],
    /// placed at South's partner), then each axis block for the hidden seats
    /// [LHO, partner, RHO] = [West, North, East] in ♣♦♥♠ order.
    #[test]
    fn oracle_all_layout_is_axis_major() {
        let deal: FullDeal = "W:T9876.QJT9.J54.A AKQJ.AK2.Q32.432 5432.87.AKT9.KQJ \
                              .6543.876.T98765"
            .parse()
            .expect("valid test deal");
        let mut out = [0f32; ORACLE_ALL_LEN];
        write_oracle_all(&mut out, &deal, Seat::South);

        // Keycard head: partner (North) holds AKQJ.AK2.Q32.432.
        assert_eq!(out[..8], [0.4, 0.4, 0.6, 0.6, 0.0, 1.0, 0.0, 1.0]);
        // Quality: per-suit HCP / 10.
        let quality = [
            [0.4, 0.1, 0.3, 0.0], // West: ♣A, ♦J54, ♥QJT9, ♠T9876
            [0.0, 0.2, 0.7, 1.0], // North: ♣432, ♦Q32, ♥AK2, ♠AKQJ
            [0.6, 0.7, 0.0, 0.0], // East: ♣KQJ, ♦AKT9, ♥87, ♠5432
        ];
        assert_eq!(out[8..20], quality.concat()[..]);
        // Shortness: only West's singleton ♣A qualifies.
        let mut shortness = [0.0; 12];
        shortness[0] = 1.0;
        assert_eq!(out[20..32], shortness);
        // Controls: (ace, king) per suit.
        let controls = [
            [1., 0., 0., 0., 0., 0., 0., 0.], // West: ♣A only
            [0., 0., 0., 0., 1., 1., 1., 1.], // North: ♥AK, ♠AKQJ
            [0., 1., 1., 1., 0., 0., 0., 0.], // East: ♣KQJ, ♦AKT9
        ];
        assert_eq!(out[32..56], controls.concat()[..]);
        // Stoppers: A / Kx / Qxx / Jxxx — West's ♦J54 is a J with only three.
        let stoppers = [
            [1.0, 0.0, 1.0, 0.0], // West: ♣A, ♥QJT9
            [0.0, 1.0, 1.0, 1.0], // North: ♦Qxx, ♥AK2, ♠AKQJ
            [1.0, 1.0, 0.0, 0.0], // East: ♣KQJ, ♦AKT9
        ];
        assert_eq!(out[56..68], stoppers.concat()[..]);
    }

    #[test]
    fn summary_is_the_ben_gather_of_bits() {
        let (hand, inferences) = fixture();

        let mut summary = vec![0f32; FEATURES_LEN_EVAL];
        encode(&mut summary, hand, &inferences, Encoding::Summary);
        let mut bits = vec![0f32; LEN_HAND_BITS + LEN_RANGES / 2 * 3];
        encode(&mut bits, hand, &inferences, Encoding::Bits);

        let live = ben_live_columns();
        assert_eq!(live.len(), 54, "the ben arm's documented live width");
        assert_eq!(live.len(), summary.len());
        for (i, (&col, &want)) in live.iter().zip(&summary).enumerate() {
            assert_eq!(bits[col], want, "summary[{i}] should be bits[{col}]");
        }
    }
}
