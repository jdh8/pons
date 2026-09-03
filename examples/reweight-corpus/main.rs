//! Outcome-reweight a teacher corpus: advantage-weighted behaviour cloning.
//!
//! The distiller's loss is soft-target cross-entropy against the teacher's call
//! distribution and nothing else — `--dd-weight 0` in every shipped net, so the
//! auxiliary value head contributes no gradient.  The net is therefore trained
//! to *imitate BBA*, never to *score well*.  This pass emits a `.w` sidecar,
//! one `f32` per row, that multiplies each row's cross-entropy by
//! `exp(β · A)`: advantage-weighted regression, where `exp(β · A)` is the
//! closed-form solution of "improve the policy but stay near the teacher, in
//! KL", so β is a trust radius rather than a learning rate.  Weights are
//! normalised to mean 1, and **β = 0 emits all-ones** — the built-in control,
//! which must train byte-identically to no sidecar at all.
//!
//! **The teacher target is one-hot.**  Measured across every shard type: 100%
//! of rows carry exactly one call at probability 1.0.  BBA returns a single
//! chosen call, so the corpus's "teacher_softmax" is a hard label and per-*call*
//! reweighting is impossible — rescaling one support point renormalises straight
//! back to 1.0.  The advantage therefore acts per *row*.
//!
//! ## A is a Monte-Carlo return, not a per-call score
//!
//! The first version of this tool priced each call *as if it ended the
//! auction*, against the opponents' par.  That is what M5.2 run 2 trained on,
//! and it measured a loss with a mechanism this module now exists to avoid:
//!
//! - 63% of priced calls are 1- or 2-level bids, i.e. mid-auction.  A `1♥`
//!   opening scored as a *1♥ contract* against a par of 620 looks terrible, so
//!   low bids were systematically downweighted and high ones upweighted.  Mean
//!   weight by level ran 1.60 / 1.51 / 1.57 / 1.94 / 1.83 / 2.78 / 3.09 for
//!   levels 1-7.
//! - `Pass`/`X`/`XX` returned "no advantage" and kept weight 1 (0.897 after
//!   normalisation), so *every* bid outweighed *every* pass.
//!
//! Both biases point the same way — bid, and bid high — and the A/B measured
//! exactly that: 5-level-plus contracts rose from +0.895pp over control to
//! +1.302pp, doubled contracts from +0.713pp to +0.935pp.
//!
//! The fix needs no new data, because the corpus stores rows **in auction
//! order**: the `.seq` step counter runs 0, 1, … and resets, so a `0` starts a
//! new auction, and every row of one auction shares one deal (verified: 300 of
//! 300 sampled auctions have DD tables that agree after de-rotating by seat).
//! So the whole call sequence is recoverable from the one-hot targets, and with
//! it the auction's *outcome*.  A is then
//!
//! ```text
//! A = imps(score(final contract, DD tricks, perfect-defense doubling) − par)
//! ```
//!
//! credited to every decision in the auction and signed for the deciding side.
//! That fixes both biases at once: a call is priced by where the auction landed
//! rather than by its own level, and `Pass` is priced like anything else — a
//! passout on a 620 par is now a large negative, which the old version could
//! not see at all.
//!
//! Par is the reference because it is *per-deal*: subtracting it removes the
//! deal's intrinsic value and leaves only the bidding error.  A global baseline
//! would upweight every auction on a 30-HCP deal and re-teach "hands like this
//! are good", which is the disease being cured.  A is two-sided — an auction
//! that beats par because the opponents misbid earns a positive A — so this is
//! a genuine advantage, not a one-sided regret.
//!
//! ## What this deliberately does not model
//!
//! - **Uniform credit.**  Every decision in an auction gets the same A, so a
//!   good call inside a bad auction is punished with it.  That is plain
//!   Monte-Carlo credit assignment; it averages out across rows sharing a
//!   feature vector, which is why β must stay small.
//! - **The DD table is hindsight.**  A call that is right on the auction's
//!   information but unlucky on this deal is penalised.  Not a per-row verdict.
//! - **`--contested-only` (the default) leaves constructive rows at weight 1**,
//!   including the constructive rows *inside* a contested auction — the tag is
//!   per decision, not per auction (31,124 of 31,250 sampled auctions are
//!   mixed).  That confines the reweight to the floor's own domain and leaves
//!   the constructive book pinned to BBA, at the cost of a mild version of the
//!   asymmetry described above.
use std::fs;
use std::io::{BufWriter, Write as _};
use std::path::PathBuf;

use clap::Parser;
use contract_bridge::auction::AbsoluteVulnerability;
use contract_bridge::seat::Seat;
use contract_bridge::{Bid, Contract, Level, Penalty, Strain};
use ddss::{TrickCountRow, TrickCountTable, Vulnerability, calculate_par};
use pons::scoring::imps;
use pons::stats::{HistogramTable, average_ns_par};

/// Softmax width: `Pass`, `X`, `XX`, then 7 levels x 5 strains.
const SOFTMAX_LEN: usize = 38;
/// Double-dummy label width: 5 strains x 4 seats, relative to the actor.
const DD_LEN: usize = 20;
/// v6 feature width.
const FEATURES_LEN: usize = 176;
/// Bytes of one `.seq` row; byte 0 is the step counter.
const SEQ_ROW_BYTES: usize = 1121;
/// Offset of the 2-value vulnerability block *in the v6 layout*.
///
/// NOT `features::OFFSET_VUL`, which is the v3 offset (86): v6 widened the
/// inference block from 40 to 72, so the block that `push_context` writes last
/// lands 32 floats further along.  Asserted against the sidecar below.
const OFFSET_VUL_V6: usize = 118;
/// Seats in `[me, lho, partner, rho]` order, with "me" mapped to North.
///
/// The corpus never stores an absolute seat, so an auction is scored in the
/// frame of its own first actor.  Mapping that actor to North also makes North
/// the dealer, which is what it is: the first actor *is* the dealer.
const SEATS: [Seat; 4] = [Seat::North, Seat::East, Seat::South, Seat::West];

/// Agreement floor for the par cross-check below.
///
/// `average_ns_par` under-reports on ~0.6% of deals by its own construction, so
/// this only has to separate "the known artifact" from "the frame is wrong",
/// and a wrong frame disagrees essentially everywhere.
const AGREE_FLOOR: f64 = 0.98;

/// The DD label lists strains in GIB tail order `[NT, S, H, D, C]`, the exact
/// reverse of `Strain::ASC` that the softmax is indexed by.
const fn dd_strain(asc: usize) -> usize {
    4 - asc
}

#[derive(Parser)]
#[command(about = "Outcome-reweight teacher corpora by double-dummy advantage over par")]
struct Args {
    /// Corpus stems to reweight (each needs `.f32`, `.json`, `.tags`, `.seq`).
    #[arg(long = "data", required = true)]
    data: Vec<PathBuf>,
    /// Directory to write the `.w` sidecars into (one per stem).
    #[arg(long)]
    out: PathBuf,
    /// Trust radius. 0 reproduces the input bit-for-bit.
    #[arg(long, default_value_t = 0.0)]
    beta: f64,
    /// Double a failing contract only when it is down this many or more.
    ///
    /// `1` is textbook perfect defense and matches both the A/B's arbiter
    /// (`ns_score_pd`) and the par reference, so it is the default.  Higher
    /// values forgive shallow sacrifices in the *outcome* term only; par is
    /// always computed under perfect defense.
    #[arg(long, default_value_t = 1)]
    double_from: u8,
    /// Weight constructive rows too (default: contested rows only, weight 1).
    #[arg(long)]
    all_phases: bool,
    /// Clamp weights into `[1/c, c]` so one freak board cannot dominate a batch.
    #[arg(long, default_value_t = 5.0)]
    clamp: f64,
}

/// The final contract of an auction and the *position* of its declarer in the
/// auction's own call list.
///
/// Positions stay relative because the corpus has no absolute seats; seats
/// alternate, so "same side" is "same parity of call index".  `None` is a
/// passed-out auction — a real outcome worth 0, not a missing one.
fn settle(calls: &[usize]) -> Option<(Contract, usize)> {
    let last = calls.iter().rposition(|&k| k >= 3)?;
    let asc = (calls[last] - 3) % 5;
    #[allow(clippy::cast_possible_truncation)]
    let level = ((calls[last] - 3) / 5 + 1) as u8;
    let penalty = calls[last + 1..]
        .iter()
        .fold(Penalty::Undoubled, |p, &c| match c {
            1 => Penalty::Doubled,
            2 => Penalty::Redoubled,
            _ => p,
        });
    let declarer = (last % 2..=last)
        .step_by(2)
        .find(|&i| calls[i] >= 3 && (calls[i] - 3) % 5 == asc)?;
    Some((
        Contract {
            bid: Bid {
                level: Level::new(level),
                strain: Strain::ASC[asc],
            },
            penalty,
        },
        declarer,
    ))
}

/// The deal's double-dummy table, in the frame of the auction's first actor.
fn table_of(dd: &[f32]) -> TrickCountTable {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let t = |i: usize| ((f64::from(dd[i]) * 13.0).round() as i64).clamp(0, 13) as u8;
    TrickCountTable(std::array::from_fn(|asc| {
        let d = dd_strain(asc) * 4;
        // [me, lho, partner, rho] == [N, E, S, W] once "me" is North.
        TrickCountRow::new(t(d), t(d + 1), t(d + 2), t(d + 3))
    }))
}

/// IMP advantage of an auction's outcome over par, signed for the side that
/// dealt (the first row's actor).
fn auction_advantage(
    dd: &[f32],
    we_vul: bool,
    they_vul: bool,
    calls: &[usize],
    double_from: u8,
) -> i64 {
    let table = table_of(dd);
    let mut vul = Vulnerability::empty();
    vul.set(Vulnerability::NS, we_vul);
    vul.set(Vulnerability::EW, they_vul);
    let par = i64::from(calculate_par(table, vul, Seat::North).score);

    let ours = settle(calls).map_or(0, |(contract, at)| {
        let we_declare = at % 2 == 0;
        let tricks = table[contract.bid.strain].get(SEATS[at % 4]).get();
        let needed = contract.bid.level.get() + 6;
        let contract = Contract {
            penalty: match contract.penalty {
                Penalty::Undoubled if tricks < needed && needed - tricks >= double_from => {
                    Penalty::Doubled
                }
                kept => kept,
            },
            ..contract
        };
        let score = i64::from(contract.score(tricks, if we_declare { we_vul } else { they_vul }));
        if we_declare { score } else { -score }
    });
    imps(ours - par)
}

/// Cross-check `ddss::calculate_par` against the in-crate `average_ns_par`.
///
/// The two are independent implementations of the same quantity, so agreeing
/// on a sample pins down both the NS sign convention and `table_of`'s seat
/// mapping — the two places where a silent frame error would turn every weight
/// into noise while leaving the run looking perfectly healthy.  A frame error
/// disagrees on nearly every deal, which is what [`AGREE_FLOOR`] tests for.
///
/// They are *not* expected to agree everywhere.  `ddss` is authoritative here;
/// `average_ns_par` runs a sequential per-seat improvement loop, so one seat's
/// greedy bid can permanently block its own partnership's better lower
/// contract — measured at ~0.6% of deals, always under-reporting.  Worked
/// example: NS make 3NT by South for 600, but North improves to 4♣ first and
/// the loop settles at 130, because 3NT is below 4♣ and can no longer be bid.
fn check_par(table: TrickCountTable, we_vul: bool, they_vul: bool) -> Option<(i64, i64)> {
    let mut vul = Vulnerability::empty();
    vul.set(Vulnerability::NS, we_vul);
    vul.set(Vulnerability::EW, they_vul);
    let mut abs = AbsoluteVulnerability::empty();
    abs.set(AbsoluteVulnerability::NS, we_vul);
    abs.set(AbsoluteVulnerability::EW, they_vul);

    let native = i64::from(calculate_par(table, vul, Seat::North).score);
    let hist: HistogramTable = std::iter::once(table).collect();
    #[allow(clippy::cast_possible_truncation)]
    let ours = average_ns_par(hist, abs, Seat::North)?.score.round() as i64;
    Some((native, ours))
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    fs::create_dir_all(&args.out)?;
    let mut checked = 0u64;
    let mut agreed = 0u64;

    for stem in &args.data {
        let name = stem
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("{} has no file name", stem.display()))?;
        let meta: serde_json::Value =
            serde_json::from_slice(&fs::read(stem.with_extension("json"))?)?;
        let row_len = meta["row_len"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("{}.json has no row_len", stem.display()))?
            as usize;
        anyhow::ensure!(
            meta["features_len"].as_u64() == Some(FEATURES_LEN as u64),
            "{}: this pass reads the v6 layout only",
            stem.display()
        );

        let bytes = fs::read(stem.with_extension("f32"))?;
        let floats: Vec<f32> = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .copied()
            .map(f32::from_le_bytes)
            .collect();
        let rows = floats.len() / row_len;
        let tags = fs::read(stem.with_extension("tags"))?;
        anyhow::ensure!(tags.len() == rows, "{}: tag/row mismatch", stem.display());
        let seq = fs::read(stem.with_extension("seq"))?;
        anyhow::ensure!(
            seq.len() == rows * SEQ_ROW_BYTES,
            "{}: seq/row mismatch",
            stem.display()
        );
        let step = |r: usize| seq[r * SEQ_ROW_BYTES];
        anyhow::ensure!(
            rows > 0 && step(0) == 0,
            "{}: does not start on an auction boundary",
            stem.display()
        );

        let row = |r: usize| &floats[r * row_len..(r + 1) * row_len];
        let chosen = |r: usize| {
            let soft = &row(r)[FEATURES_LEN..FEATURES_LEN + SOFTMAX_LEN];
            soft.iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map_or(0, |(i, _)| i)
        };

        // Auctions are the maximal runs between `.seq` step-counter resets.
        // The counter saturates at MAX_STEPS_V7 for long auctions, so only its
        // zeroes are load-bearing; a row's own index within the run gives its
        // seat offset.
        let starts: Vec<usize> = (0..rows).filter(|&r| step(r) == 0).collect();
        let mut weights = vec![1.0f32; rows];
        let mut priced = 0usize;
        let mut worst = 0i64;

        for (i, &start) in starts.iter().enumerate() {
            let end = starts.get(i + 1).copied().unwrap_or(rows);
            let first = row(start);
            let we_vul = first[OFFSET_VUL_V6] > 0.5;
            let they_vul = first[OFFSET_VUL_V6 + 1] > 0.5;
            let dd = &first[FEATURES_LEN + SOFTMAX_LEN..][..DD_LEN];
            let calls: Vec<usize> = (start..end).map(chosen).collect();

            if checked < 5000 {
                checked += 1;
                if let Some((native, ours)) = check_par(table_of(dd), we_vul, they_vul)
                    && native == ours
                {
                    agreed += 1;
                }
            }

            let advantage = auction_advantage(dd, we_vul, they_vul, &calls, args.double_from);
            worst = worst.min(advantage);
            for r in start..end {
                if !args.all_phases && tags[r] != 1 {
                    continue;
                }
                // Seats alternate, so an odd offset from the first actor is an
                // opponent and takes the advantage with the sign flipped.
                let signed = if (r - start) % 2 == 0 {
                    advantage
                } else {
                    -advantage
                };
                #[allow(clippy::cast_possible_truncation)]
                let w = (args.beta * signed as f64)
                    .exp()
                    .clamp(1.0 / args.clamp, args.clamp) as f32;
                weights[r] = w;
                priced += 1;
            }
        }

        let mean = f64::from(weights.iter().sum::<f32>()) / rows as f64;
        #[allow(clippy::cast_possible_truncation)]
        for w in &mut weights {
            *w = (f64::from(*w) / mean) as f32;
        }
        let (lo, hi) = weights
            .iter()
            .fold((f32::MAX, 0f32), |(l, h), &w| (l.min(w), h.max(w)));

        let path = args.out.join(name).with_extension("w");
        let mut out = BufWriter::new(fs::File::create(&path)?);
        for w in &weights {
            out.write_all(&w.to_le_bytes())?;
        }
        out.flush()?;
        println!(
            "{}: {rows} rows in {} auctions, {priced} priced, weight range [{lo:.3}, {hi:.3}], worst auction {worst} IMPs",
            name.to_string_lossy(),
            starts.len(),
        );
    }

    #[allow(clippy::cast_precision_loss)]
    let rate = agreed as f64 / checked.max(1) as f64;
    anyhow::ensure!(
        checked > 0 && rate >= AGREE_FLOOR,
        "par cross-check failed: ddss and average_ns_par agree on only {agreed} of {checked} \
         auctions ({:.1}%) — below the {:.0}% floor, so this is a frame or sign error, not the \
         known ~0.6% blocking artifact",
        100.0 * rate,
        100.0 * AGREE_FLOOR,
    );
    println!(
        "par cross-check: ddss == average_ns_par on {agreed}/{checked} sampled auctions ({:.1}%)",
        100.0 * rate
    );
    Ok(())
}
