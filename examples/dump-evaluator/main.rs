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
//! stock (`/nfs2/jdh8/pons/*.pdd`, ~94M solved deals); the only work is bidding.
//!
//! ```text
//! cargo run --release --example dump-evaluator -- \
//!     --deals /nfs2/jdh8/pons/22.pdd --count 100000 --seed $(date +%s)
//! ```
//!
//! Output is a flat little-endian `f32` file of `features_len + 20` floats per
//! row, a JSON sidecar pinning the layout, and a sibling `.tags` byte per row
//! (bit 0 = contested phase, bit 1 = system index). The loop is **deal-major**
//! so a contiguous validation split stays deal-disjoint — the ~10 rows a board
//! contributes all share one DD label, and a shuffled split would leak it.

use clap::Parser;
use contract_bridge::auction::{Auction, Call, RelativeVulnerability};
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Rank, Seat, Strain, Suit};
use ddss::TrickCountTable;
use pons::bidding::context::{Context, relative};
use pons::bidding::features::{
    FEATURES_LEN_EVAL, FEATURES_LEN_EVAL_POINTS, FEATURES_LEN_EVAL_SHAPE, FEATURES_LEN_EVAL_V3,
    FEATURES_LEN_EVAL_V4, FEATURES_VERSION_EVAL, FEATURES_VERSION_EVAL_V4, LEN_HAND_EVAL,
    LEN_HAND_V3, features_eval, features_eval_points, features_eval_shape, features_eval_v3,
    features_eval_v4, features_v3,
};
use pons::bidding::tags::derive;
use pons::bidding::{Inferences, Phase, Stance, System};
use pons::{american, dutch, gib};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use rayon::prelude::*;
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

/// The structural tag vocabulary of [`derive`], in first-appearance order in
/// `src/bidding/tags.rs` — the multi-hot axis of the `--auction` block.
const TAGS: [&str; 21] = [
    "NF", "RDBL", "T/O", "NEG", "PEN", "NAT", "BAL", "ART", "STR", "PRE", "WK", "STAY", "TRF", "F",
    "QUANT", "CUE", "SUPP", "L/S", "SPL", "WJS", "FG",
];

/// Width of one call's `--auction` slot: the 7-value bid encoding
/// (`[present, level/7, strain one-hot ×5]`, the `push_bid_encoding` layout),
/// 3 call-kind bits `[is_pass, is_double, is_redouble]` (all-zero = no such
/// call), the [`TAGS`] multi-hot, an alerted bit, and an 8-bucket FNV-1a
/// one-hot of the alert name.
const LEN_CALL_SLOT: usize = 7 + 3 + TAGS.len() + 1 + 8;

/// Number of most-recent calls the `--auction` block encodes, most recent
/// first.
const AUCTION_CALLS: usize = 4;

/// Width of the whole `--auction` block (`--encoding bits` only): the
/// auction+alert ablation's extra columns, appended after the 79-float
/// superset.
const AUCTION_LEN: usize = AUCTION_CALLS * LEN_CALL_SLOT;

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
    /// `features_eval_v3` verbatim (94 floats): the serving extractor for the
    /// calls-only v3 evaluator, so the corpus and the crate agree by
    /// construction
    Eval3,
    /// `features_eval_shape` verbatim (199 floats): the shape-reading research
    /// superset — the `eval3` vector with each hidden seat's shape distribution
    /// spliced in beside its hull endpoints, so one corpus trains both the
    /// control arm and every distributional arm
    Shape,
    /// `features_eval_v4` verbatim (97 floats): the serving extractor for the
    /// shape-reading evaluator, so the corpus and the crate agree by
    /// construction
    Eval4,
    /// `features_eval_points` verbatim (136 floats): the strength-reading
    /// research superset — the `eval4` vector with each hidden seat's raw-HCP
    /// endpoints and strength distribution spliced in beside its `points`
    /// endpoints, so one corpus trains the v4 control arm and every strength arm
    Points,
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
    /// Row encoding: `summary` (features_eval's 54 floats), `onehot` (52 card
    /// bits — the texture ablation), `bits` (the 79-float research superset),
    /// `eval3` (features_eval_v3's 94 floats — the calls-only serving
    /// extractor, verbatim), `shape` (features_eval_shape's 289 floats — the
    /// shape-reading superset: endpoints *and* shape distribution per seat),
    /// `eval4` (features_eval_v4's 97 floats — the shape-reading serving
    /// extractor, verbatim), or `points` (features_eval_points' 136 floats —
    /// the strength-reading superset: `eval4` plus raw-HCP endpoints and the
    /// strength distribution per seat)
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
    /// Append the auction+alert block: the 4 most-recent calls, each as bid
    /// encoding + call-kind bits + structural tag multi-hot + the winning
    /// rule's alert (bit + hashed name). Requires `--encoding bits`; mutually
    /// exclusive with the oracle columns — the trainer's `ben-auction` arm
    /// reads these.
    #[arg(long, conflicts_with_all = ["oracle", "oracle_all"])]
    auction: bool,
    /// Output path stem; writes `<out>.f32`, `<out>.json`, `<out>.tags`
    #[arg(long, default_value = "target/evaluator-data")]
    out: String,
    /// Bid and read with `set_dnf_reading(true)` — the F2b corpus. Both the
    /// auctions and the range features come from the knob-on regime, matching
    /// what a knob-on bidder serves the evaluator.
    #[arg(long)]
    dnf: bool,
    /// Fold both box closures (`set_sum_closure` + `set_upgrade_closure`) into
    /// the hulls the features see — the canonicalized-reading corpus. The
    /// closures are flipped on only around `infer`/`encode`, so the *auctions*
    /// are still bid knob-off and stay byte-identical to a dump without this
    /// flag: same rows, same targets, only the hull columns tighten. Requires
    /// `--dnf` (the closures fold inside `Dnf::tidy`, which is a no-op
    /// knob-off).
    #[arg(long, requires = "dnf")]
    closed_hulls: bool,
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
    // Main thread's copy; the rayon workers get theirs via `broadcast` below.
    pons::bidding::set_dnf_reading(args.dnf);
    let encoding = match args.encoding.as_str() {
        "summary" => Encoding::Summary,
        "onehot" => Encoding::Onehot,
        "bits" => Encoding::Bits,
        "eval3" => Encoding::Eval3,
        "shape" => Encoding::Shape,
        "eval4" => Encoding::Eval4,
        "points" => Encoding::Points,
        other => anyhow::bail!(
            "--encoding must be summary|onehot|bits|eval3|shape|eval4|points, got {other:?}"
        ),
    };
    let base_len = match encoding {
        Encoding::Summary => FEATURES_LEN_EVAL,
        Encoding::Onehot => LEN_HAND_ONEHOT + LEN_RANGES,
        Encoding::Bits => LEN_HAND_BITS + LEN_RANGES / 2 * 3,
        Encoding::Eval3 => FEATURES_LEN_EVAL_V3,
        Encoding::Shape => FEATURES_LEN_EVAL_SHAPE,
        Encoding::Eval4 => FEATURES_LEN_EVAL_V4,
        Encoding::Points => FEATURES_LEN_EVAL_POINTS,
    };
    anyhow::ensure!(
        !(args.oracle || args.oracle_all) || matches!(encoding, Encoding::Bits),
        "--oracle/--oracle-all only extend the `bits` superset the trainer arms mask over"
    );
    anyhow::ensure!(
        !args.auction || matches!(encoding, Encoding::Bits),
        "--auction only extends the `bits` superset the trainer arms mask over"
    );
    let features_len = base_len
        + if args.oracle_all {
            ORACLE_ALL_LEN
        } else if args.oracle {
            ORACLE_LEN
        } else if args.auction {
            AUCTION_LEN
        } else {
            0
        };
    let row_len = features_len + DD_LEN;

    let systems: Vec<(&str, Stance)> = args
        .systems
        .split(',')
        .map(|name| match name.trim() {
            "american" => Ok(("american", american().against())),
            "dutch" => Ok(("dutch", dutch().against())),
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

    let f32_path = format!("{}.f32", args.out);
    let mut writer = BufWriter::new(std::fs::File::create(&f32_path)?);
    let mut tags = BufWriter::new(std::fs::File::create(format!("{}.tags", args.out))?);

    // The reading knob is a thread-local: every rayon worker needs the same
    // setting main got at the top, and `broadcast` forces the pool up so no
    // worker is born later with the bare default.
    rayon::broadcast(|_| pons::bidding::set_dnf_reading(args.dnf));

    let (mut rows, mut contested, mut forced_pass) = (0u64, 0u64, 0u64);

    // Deal-major: every row a board contributes stays contiguous, so the
    // trainer's contiguous validation tail is deal-disjoint.  Parallel over
    // deals — the DD tables are pre-solved, so per-deal work is pure bidding.
    // (dealer, vul) derive from a per-deal RNG keyed on (seed, index), making
    // the output independent of thread schedule; the stream differs from the
    // old sequential dumper at the same seed (sidecar records the scheme).
    // ponytail: chunked collect bounds memory at ~CHUNK deals of rows; a
    // bounded-channel pipeline would overlap write with bidding if the writer
    // ever becomes the bottleneck.
    const CHUNK: usize = 4096;
    for (chunk_index, chunk) in deals.chunks(CHUNK).enumerate() {
        let buffers: Vec<(Vec<u8>, Vec<u8>, u64)> = chunk
            .par_iter()
            .enumerate()
            .map(|(in_chunk, (deal, table))| {
                let index = (chunk_index * CHUNK + in_chunk) as u64;
                let mut rng =
                    StdRng::seed_from_u64(args.seed ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15));
                let dealer = rng.random_range(0..4usize);
                let vul = VULS[rng.random_range(0..4usize)];

                let mut row = vec![0f32; row_len];
                let mut bytes = Vec::new();
                let mut tag_bytes = Vec::new();
                let mut forced = 0u64;
                for (sys_idx, (_, stance)) in systems.iter().enumerate() {
                    let mut auction = Auction::new();
                    // Parallel to the auction under `--auction`: the alert of
                    // each call's winning rule, `None` for forced or rule-less
                    // calls.
                    let mut alerts: Vec<Option<&'static str>> = Vec::new();
                    while !auction.has_ended() {
                        let seat = Seat::ALL[(dealer + auction.len()) % 4];
                        let hand = deal[seat];
                        let rel = relative(vul, seat);

                        let Some(mut logits) = stance.classify(hand, rel, &auction) else {
                            forced += 1;
                            if args.auction {
                                alerts.push(None);
                            }
                            auction.push(Call::Pass);
                            continue;
                        };
                        for (call, slot) in logits.iter_mut() {
                            if auction.can_push(call).is_err() {
                                *slot = f32::NEG_INFINITY;
                            }
                        }

                        // The trie-prefixed reading, so conventional calls
                        // decode off their authoring rules rather than as
                        // natural suits. Under `--closed-hulls` the closures
                        // are on for exactly this read: `classify` above ran
                        // knob-off, so the auctions never move.
                        if args.closed_hulls {
                            pons::bidding::set_sum_closure(true);
                            pons::bidding::set_upgrade_closure(true);
                        }
                        let inferences = stance.infer(rel, &auction);
                        encode(&mut row[..base_len], hand, &inferences, &auction, encoding);
                        if args.closed_hulls {
                            pons::bidding::set_sum_closure(false);
                            pons::bidding::set_upgrade_closure(false);
                        }
                        if args.oracle_all {
                            write_oracle_all(&mut row[base_len..features_len], deal, seat);
                        } else if args.oracle {
                            write_oracle(&mut row[base_len..features_len], deal[seat.partner()]);
                        } else if args.auction {
                            write_auction_block(
                                &mut row[base_len..features_len],
                                &auction,
                                &alerts,
                            );
                        }
                        row[features_len..].copy_from_slice(&gib::relativized_tricks(table, seat));
                        for value in &row {
                            bytes.extend_from_slice(&value.to_le_bytes());
                        }

                        let contested_row = Phase::of(&auction) != Phase::Constructive;
                        tag_bytes.push(u8::from(contested_row) | (sys_idx as u8) << 1);

                        let call = argmax_legal(&logits);
                        if args.auction {
                            // The winning rule's alert, read off the same
                            // routing that chose the call; `None` when a floor
                            // (not rule-backed) or an unalerted (natural) rule
                            // won.
                            alerts.push(
                                stance
                                    .explain_call(hand, rel, &auction, call)
                                    .and_then(|(_, rule)| rule)
                                    .and_then(|rule| rule.alert),
                            );
                        }
                        auction.push(call);
                    }
                }
                (bytes, tag_bytes, forced)
            })
            .collect();

        for (bytes, tag_bytes, forced) in buffers {
            rows += (tag_bytes.len()) as u64;
            contested += tag_bytes.iter().map(|&t| u64::from(t & 1)).sum::<u64>();
            forced_pass += forced;
            writer.write_all(&bytes)?;
            tags.write_all(&tag_bytes)?;
        }
    }
    writer.flush()?;
    tags.flush()?;

    let metadata = serde_json::json!({
        // The extractor layout version: `eval3` rows are `features_eval_v3`.
        "feature_version": match encoding {
            // `eval3` rows are `features_eval_v3`; `shape` splices its superset
            // around the same v3 blocks.
            Encoding::Eval3 | Encoding::Shape => 3,
            // `points` splices its superset around the same v4 blocks.
            Encoding::Eval4 | Encoding::Points => FEATURES_VERSION_EVAL_V4,
            _ => FEATURES_VERSION_EVAL,
        },
        "features_len": features_len,
        "dd_len": DD_LEN,
        "row_len": row_len,
        "row_bytes": row_len * 4,
        "dtype": "f32-le",
        "encoding": args.encoding,
        "oracle": args.oracle,
        "oracle_all": args.oracle_all,
        "auction": args.auction,
        "closed_hulls": args.closed_hulls,
        "auction_layout": format!(
            "{AUCTION_CALLS} most-recent calls (most recent first) × {LEN_CALL_SLOT} = \
             [7 bid: present, level/7, strain one-hot CDHSN][3: pass, X, XX]\
             [{} tag multi-hot][1 alerted][8 fnv1a(alert) % 8 one-hot]",
            TAGS.len()
        ),
        "shape_layout": "encoding=shape: [24 hand][3 seats × (10 hull endpoints + 65 shape)]\
                         [4 calls × 10]; shape = [4 E len][4 sd len][56 P(len=k)][1 pinned].  \
                         encoding=eval4: [24 hand][3 seats × (2 points + 9 shape)][4 calls × 10]; \
                         shape = [4 E len][4 sd len][1 pinned].  \
                         encoding=points: [24 hand][3 seats × (10 hull endpoints + 9 shape + \
                         2 hcp endpoints + 3 strength)][4 calls × 10]; \
                         strength = [E hcp][sd hcp][1 pinned]",
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
        "deal_rng": "per-deal StdRng(seed ^ index·0x9E3779B97F4A7C15); \
                     not byte-compatible with pre-parallel sequential dumps",
        "dnf": args.dnf,
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
fn encode(
    out: &mut [f32],
    hand: Hand,
    inferences: &Inferences,
    calls: &[Call],
    encoding: Encoding,
) {
    match encoding {
        // The serving extractors verbatim — corpus/serving parity by
        // construction, nothing to reassemble.
        Encoding::Eval3 => {
            out.copy_from_slice(&features_eval_v3(hand, inferences, calls));
            return;
        }
        Encoding::Shape => {
            out.copy_from_slice(&features_eval_shape(hand, inferences, calls));
            return;
        }
        Encoding::Eval4 => {
            out.copy_from_slice(&features_eval_v4(hand, inferences, calls));
            return;
        }
        Encoding::Points => {
            out.copy_from_slice(&features_eval_points(hand, inferences, calls));
            return;
        }
        _ => {}
    }
    let feats = features_eval(hand, inferences);
    let (hand_block, ranges) = feats.split_at(LEN_HAND_EVAL);
    let cut = match encoding {
        Encoding::Eval3 | Encoding::Shape | Encoding::Eval4 | Encoding::Points => {
            unreachable!("returned above")
        }
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
                .as_chunks_mut::<LEN_SUIT_BITS>()
                .0
                .iter_mut()
                .zip(v3[..LEN_HAND_V3].as_chunks::<2>().0)
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
        for (triple, pair) in tail
            .as_chunks_mut::<3>()
            .0
            .iter_mut()
            .zip(ranges.as_chunks::<2>().0)
        {
            *triple = [pair[0], pair[1], pair[1] - pair[0]];
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
    for (pair, h) in controls
        .as_chunks_mut::<2>()
        .0
        .iter_mut()
        .zip(holdings.clone())
    {
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

/// Write the [`AUCTION_LEN`] `--auction` columns: one [`LEN_CALL_SLOT`] slot
/// per most-recent call, most recent first, zero-filled slots beyond the
/// auction's start.  `alerts` is the alert history parallel to `auction`.
///
/// The 7-value bid encoding clones the layout of the crate-private
/// `push_bid_encoding` (`src/bidding/features.rs`): `[present, level/7,
/// strain one-hot ×5]` in [`Strain::ASC`] order.  Tags come from the same
/// structural [`derive`] the corpus exporter uses: the [`Context`] of the
/// auction prefix *before* the call (whose side to act is the seat that made
/// it), under the book [`Phase::of`] names for that prefix.
fn write_auction_block(out: &mut [f32], auction: &[Call], alerts: &[Option<&'static str>]) {
    out.fill(0.0);
    let slots = out.as_chunks_mut::<LEN_CALL_SLOT>().0.iter_mut();
    for (slot, j) in slots.zip((0..auction.len()).rev()) {
        let call = auction[j];
        if let Call::Bid(bid) = call {
            slot[0] = 1.0;
            slot[1] = f32::from(bid.level.get()) / 7.0;
            for (flag, strain) in slot[2..7].iter_mut().zip(Strain::ASC) {
                *flag = f32::from(bid.strain == strain);
            }
        }
        slot[7] = f32::from(call == Call::Pass);
        slot[8] = f32::from(call == Call::Double);
        slot[9] = f32::from(call == Call::Redouble);

        let prefix = &auction[..j];
        let book = match Phase::of(prefix) {
            Phase::Constructive => "constructive",
            Phase::Competitive => "competitive",
            Phase::Defensive => "defensive",
        };
        let ctx = Context::new(RelativeVulnerability::NONE, prefix);
        for tag in derive(book, call, &ctx).0 {
            let index = TAGS
                .iter()
                .position(|&t| t == tag)
                .expect("derive's tags stay within the TAGS vocabulary");
            slot[10 + index] = 1.0;
        }

        if let Some(name) = alerts.get(j).copied().flatten() {
            slot[10 + TAGS.len()] = 1.0;
            slot[11 + TAGS.len() + (fnv1a(name) % 8) as usize] = 1.0;
        }
    }
}

/// Standard 64-bit FNV-1a over the string's bytes.
fn fnv1a(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3)
    })
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
        let stance = american().against();
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
        encode(&mut row, hand, &inferences, &[], Encoding::Bits);

        let (hand_block, triples) = row.split_at(LEN_HAND_BITS);
        for block in hand_block[..4 * LEN_SUIT_BITS]
            .as_chunks::<LEN_SUIT_BITS>()
            .0
        {
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
            .as_chunks::<3>()
            .0
            .iter()
            .zip(feats[LEN_HAND_EVAL..].as_chunks::<2>().0)
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
        encode(&mut summary, hand, &inferences, &[], Encoding::Summary);
        let mut bits = vec![0f32; LEN_HAND_BITS + LEN_RANGES / 2 * 3];
        encode(&mut bits, hand, &inferences, &[], Encoding::Bits);

        let live = ben_live_columns();
        assert_eq!(live.len(), 54, "the ben arm's documented live width");
        assert_eq!(live.len(), summary.len());
        for (i, (&col, &want)) in live.iter().zip(&summary).enumerate() {
            assert_eq!(bits[col], want, "summary[{i}] should be bits[{col}]");
        }
    }

    /// The `--auction` block, pinned on `1NT–P–2♣` (our Stayman): the
    /// most-recent slot carries the 2♣ bid encoding, the `STAY` tag bit, the
    /// alerted bit, and exactly one hash bucket; the Pass slot is `is_pass`
    /// with no bid present; slots past the auction's start stay all-zero.
    #[test]
    fn auction_block_encodes_stayman() {
        let auction: Vec<Call> = ["1N", "P", "2C"]
            .iter()
            .map(|c| c.parse().expect("valid test call"))
            .collect();
        // Invitational with both four-card majors — a live Stayman hand, so
        // the alerted 2♣ rule gives it a finite logit and wins attribution.
        let hand: Hand = "AQ32.KJ54.876.54".parse().expect("valid test hand");
        let stance = american().against();
        let vul = relative(AbsoluteVulnerability::NONE, Seat::South);
        let alert = stance
            .explain_call(hand, vul, &auction[..2], auction[2])
            .and_then(|(_, rule)| rule)
            .and_then(|rule| rule.alert);
        assert_eq!(alert, Some("stayman"), "the Stayman rule's alert");

        let alerts = vec![None, None, alert];
        let mut block = [0f32; AUCTION_LEN];
        write_auction_block(&mut block, &auction, &alerts);

        // Slot 0, most recent = 2♣: present, level 2, clubs first in ASC.
        let (recent, rest) = block.split_at(LEN_CALL_SLOT);
        assert_eq!(recent[..7], [1.0, 2.0 / 7.0, 1.0, 0.0, 0.0, 0.0, 0.0]);
        assert_eq!(recent[7..10], [0.0; 3]);
        let stay = TAGS.iter().position(|&t| t == "STAY").expect("STAY tag");
        let tags: Vec<usize> = (0..TAGS.len()).filter(|&i| recent[10 + i] != 0.0).collect();
        assert_eq!(tags, [stay], "exactly the STAY tag bit");
        assert_eq!(recent[10 + TAGS.len()], 1.0, "alerted bit");
        let buckets = &recent[11 + TAGS.len()..];
        assert_eq!(buckets.len(), 8);
        assert!(buckets.iter().all(|&b| b == 0.0 || b == 1.0));
        assert_eq!(buckets.iter().sum::<f32>(), 1.0, "exactly one hash bucket");

        // Slot 1 = the Pass: no bid present, only the is_pass call-kind bit.
        let pass = &rest[..LEN_CALL_SLOT];
        assert_eq!(pass[..8], [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
        assert_eq!(pass[8..10], [0.0; 2]);

        // Slot 3 is beyond the 3-call auction: all-zero, unlike a real Pass.
        assert!(block[3 * LEN_CALL_SLOT..].iter().all(|&x| x == 0.0));
    }
}
