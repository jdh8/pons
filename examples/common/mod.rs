//! Shared helpers for the A/B / measurement harnesses (`ab-*`, some `probe-*`).
//!
//! Pulled in verbatim with
//! `#[path = "../common/mod.rs"] #[allow(dead_code)] mod common;` — a sibling
//! directory holding only `mod.rs` (no `main.rs`) is invisible to Cargo's example
//! auto-discovery, so this never compiles as a standalone example. Each harness
//! uses only the subset it needs, hence the `#[allow(dead_code)]` on the `mod`.

use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::eval::hcp as holding_hcp;
use contract_bridge::{AbsoluteVulnerability, Contract, FullDeal, Hand, Seat, Suit};
use ddss::{NonEmptyStrainFlags, Solver, TrickCountTable};
use pons::bidding::context::relative;
use pons::bidding::{Stance, System};
use pons::scoring::{imps, ns_score_contract, ns_score_pd};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// Total HCP of a hand
pub fn hand_hcp(hand: Hand) -> u8 {
    Suit::ASC.iter().map(|&s| holding_hcp::<u8>(hand[s])).sum()
}

/// The seat acting after `len` calls from `dealer`
pub const fn seat_to_act(dealer: Seat, len: usize) -> Seat {
    Seat::ALL[(dealer as usize + len) % 4]
}

/// `count` deals, board `i` seeded `base + i`, so every arm of an experiment
/// replays the identical stream (the seed-hygiene invariant the A/B scripts
/// rely on — one `SEED_BASE` shared across arms).  Bidding is pure and
/// parallelizes downstream; only the deal generation is centralized here.
pub fn seeded_deals(base: u64, count: usize) -> Vec<FullDeal> {
    (0..count)
        .map(|i| full_deal(&mut StdRng::seed_from_u64(base.wrapping_add(i as u64))))
        .collect()
}

/// The highest-logit *legal* call, defaulting to a pass
pub fn next_call(
    stance: &Stance,
    hand: Hand,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    auction: &Auction,
) -> Call {
    let seat = seat_to_act(dealer, auction.len());
    let Some(logits) = stance.classify(hand, relative(vul, seat), auction) else {
        return Call::Pass;
    };
    let mut scored: Vec<(Call, f32)> = logits
        .iter()
        .map(|(call, &logit)| (call, logit))
        .filter(|&(_, logit)| logit.is_finite())
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("logits are never NaN"));
    scored
        .into_iter()
        .map(|(call, _)| call)
        .find(|&call| auction.can_push(call).is_ok())
        .unwrap_or(Call::Pass)
}

/// Bid one deal with the convention pair on the side picked by `conv_is_ns`
pub fn bid_out(
    conv: &Stance,
    baseline: &Stance,
    conv_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let stance = if seat_is_ns == conv_is_ns {
            conv
        } else {
            baseline
        };
        auction.push(next_call(stance, deal[seat], dealer, vul, &auction));
    }
    auction
}

/// Bid one deal with the opponents (East/West) forced to pass throughout
pub fn bid_uncontested(
    stance: &Stance,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let call = if matches!(seat, Seat::East | Seat::West) {
            Call::Pass
        } else {
            next_call(stance, deal[seat], dealer, vul, &auction)
        };
        auction.push(call);
    }
    auction
}

/// One board: the deal, the dealer, and both tables' auctions.  The interchange
/// unit between `bba-gen` (writes it) and `bba-score` (reads it).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Board {
    pub deal: FullDeal,
    pub dealer: Seat,
    /// Our pair North/South
    pub table_a: Auction,
    /// Our pair East/West
    pub table_b: Auction,
}

/// A serialized match: the bidder labels, the vulnerability the boards were bid
/// at, and every board.  `bba-gen` writes it; `bba-score` reads and scores it.
/// The serde derive is feature-gated so non-`serde` harnesses still compile this
/// shared module (the struct is then just dead code).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Dump {
    pub our_label: String,
    pub their_label: String,
    pub vulnerability: AbsoluteVulnerability,
    /// The deal seed this shard was generated from, so the exact board stream
    /// is reproducible forever.  `None` in dumps written before the anchor
    /// campaign (serde-defaulted for backwards compatibility).
    #[cfg_attr(feature = "serde", serde(default))]
    pub seed: Option<u64>,
    /// The generating command line (`argv[1..]`), so a scorer can rebuild the
    /// exact book configuration instead of guessing knob state.  Empty in
    /// older dumps.
    #[cfg_attr(feature = "serde", serde(default))]
    pub gen_args: Vec<String>,
    pub boards: Vec<Board>,
}

/// Load a dump from `path`.  A plain file is read directly; a **directory**
/// globs its `shard-*.json` (sorted, so paired arms concatenate in the same
/// order) and folds every shard's boards into one dump.  Handing a harness the
/// whole board set lets it solve the entire divergent set in a single DDS
/// fan-out, instead of launching one solver process per shard — each of which
/// spins a full-core DDS pool and oversubscribes the box.
#[cfg(feature = "serde")]
pub fn load_dump(path: &str) -> Dump {
    fn read(path: &std::path::Path) -> Dump {
        serde_json::from_reader(std::io::BufReader::new(
            std::fs::File::open(path)
                .unwrap_or_else(|e| panic!("open dump {}: {e}", path.display())),
        ))
        .unwrap_or_else(|e| panic!("parse dump {}: {e}", path.display()))
    }
    let root = std::path::Path::new(path);
    if !root.is_dir() {
        return read(root);
    }
    let mut shards: Vec<std::path::PathBuf> = std::fs::read_dir(root)
        .unwrap_or_else(|e| panic!("read dir {path}: {e}"))
        .map(|entry| entry.expect("dir entry").path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("shard-") && n.ends_with(".json"))
        })
        .collect();
    assert!(!shards.is_empty(), "no shard-*.json in {path}");
    shards.sort();
    let mut dump = read(&shards[0]);
    for shard in &shards[1..] {
        dump.boards.extend(read(shard).boards);
    }
    dump
}

/// Sample mean and the half-width of its 95% confidence interval
///
/// The mean is the headline IMPs/board; the half-width is `1.96 · SE` from the
/// per-board sample standard deviation, so a CI that excludes 0 is a result
/// distinguishable from noise.
#[allow(clippy::cast_precision_loss)]
pub fn mean_with_ci(values: &[i64]) -> (f64, f64) {
    let n = values.len();
    if n < 2 {
        return (0.0, 0.0);
    }
    let mean = values.iter().sum::<i64>() as f64 / n as f64;
    let variance = values
        .iter()
        .map(|&v| {
            let d = v as f64 - mean;
            d * d
        })
        .sum::<f64>()
        / (n - 1) as f64;
    (mean, 1.96 * (variance / n as f64).sqrt())
}

/// The outcome of scoring a board set against itself.
pub struct Scored {
    /// Indices (into the input) of boards whose two tables reached different
    /// contracts — the only ones that can swing.
    pub divergent: Vec<usize>,
    /// Per-board IMP swing, 0 for non-divergent boards (for the mean and its CI).
    pub board_imps: Vec<i64>,
    /// Per divergent board: `(index, point swing, IMP swing)`, for the dump.
    pub swings: Vec<(usize, i64, i64)>,
    pub total_points: i64,
    pub total_imps: i64,
    /// DD trick tables of the divergent boards, parallel to `divergent` — kept so
    /// callers can run further per-board analysis (e.g. a counterfactual line).
    pub tables: Vec<TrickCountTable>,
}

/// A reached contract and its declarer, or `None` for a pass-out — what
/// `final_contract` yields and what a `scorer` prices.
pub type Reached = Option<(Contract, Seat)>;

/// Solve the divergent boards double dummy and score each table's contract with
/// `scorer`, crediting the swing to the pair sitting NS at table A / EW at table
/// B.  This is the shared core of every A/B / BBA duplicate harness; only
/// `scorer` varies — `ns_score_contract` for plain DD, a perfect-defense closure
/// for PD.  `deals[i]` must be the deal of `contracts[i]`.
pub fn score_boards(
    contracts: &[(Reached, Reached)],
    deals: &[FullDeal],
    vul: AbsoluteVulnerability,
    scorer: impl Fn(Reached, &TrickCountTable, AbsoluteVulnerability) -> i64,
) -> Scored {
    let divergent: Vec<usize> = (0..contracts.len())
        .filter(|&index| contracts[index].0 != contracts[index].1)
        .collect();
    let solve: Vec<FullDeal> = divergent.iter().map(|&index| deals[index]).collect();
    let tables = Solver::lock().solve_deals(&solve, NonEmptyStrainFlags::ALL);

    let mut total_points = 0i64;
    let mut board_imps = vec![0i64; contracts.len()];
    let mut swings: Vec<(usize, i64, i64)> = Vec::with_capacity(divergent.len());
    for (&index, table) in divergent.iter().zip(tables.iter()) {
        let (contract_a, contract_b) = contracts[index];
        let swing = scorer(contract_a, table, vul) - scorer(contract_b, table, vul);
        total_points += swing;
        board_imps[index] = imps(swing);
        swings.push((index, swing, imps(swing)));
    }
    let total_imps = board_imps.iter().sum();
    Scored {
        divergent,
        board_imps,
        swings,
        total_points,
        total_imps,
        tables,
    }
}

/// Print the measurement playbook's dual bracket for a divergent-only solved
/// A/B: the swing scored **both** ways — plain DD (`ns_score_contract`, the
/// contract's actual penalty) and perfect defense (`ns_score_pd`, a failing
/// contract priced as doubled).  `contracts[i]` is `[off, on]`, and `tables`
/// are the solved divergent boards, parallel to `divergent`.  Each line reports
/// total IMPs, IMPs/board (over all `count`) and IMPs/divergent, with 95% CIs.
///
/// Unlike [`score_boards`], this scores twice from one solve — the harness
/// solves the divergent set once and reads both brackets off the same tables.
pub fn report_brackets(
    count: usize,
    divergent: &[usize],
    tables: &[TrickCountTable],
    contracts: &[[Option<(Contract, Seat)>; 2]],
    vul: AbsoluteVulnerability,
) {
    for (label, scorer) in [
        ("plain DD", ns_score_contract as fn(_, _, _) -> i64),
        ("perfect defense", ns_score_pd),
    ] {
        let mut per_board = vec![0i64; count];
        for (&i, table) in divergent.iter().zip(tables.iter()) {
            let off = scorer(contracts[i][0], table, vul);
            let on = scorer(contracts[i][1], table, vul);
            per_board[i] = imps(on - off);
        }
        let fired: Vec<i64> = divergent.iter().map(|&i| per_board[i]).collect();
        let (per_board_mean, per_board_ci) = mean_with_ci(&per_board);
        let (fired_mean, fired_ci) = mean_with_ci(&fired);
        println!(
            "{label:>15}: {:+} IMPs — {per_board_mean:+.4} ± {per_board_ci:.4} IMPs/board, \
             {fired_mean:+.3} ± {fired_ci:.3} IMPs/divergent",
            fired.iter().sum::<i64>(),
        );
    }
}
