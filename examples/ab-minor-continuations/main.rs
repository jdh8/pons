//! Minor-opening continuation A/B: baseline vs any combination of the
//! longer-major response discipline (`--longer-major`), the up-the-line
//! completion (`--up-the-line`), and the XYZ two-way checkback (`--xyz`).
//! `--nmf` swaps New Minor Forcing onto the four `1m-1M-1NT` slots and is
//! measured *against an XYZ baseline* (both arms run XYZ, so the divergence
//! isolates the swap); `--dump-worst N` prints the N worst plain-DD divergent
//! boards for the treatment (the iron rule's divergent-board trace).
//!
//! Both arms run the same 2/1 system over the same deals; the only difference
//! is the selected `set_*` knobs.  Opponents are silenced (East/West always
//! pass), so this measures the *constructive* value of the treatments.  Each
//! board is bid twice, once per arm; boards whose arms reach different
//! contracts are solved double dummy once and scored with **both** scorers —
//! plain DD (`ns_score_contract`) and perfect defense (`ns_score_pd`) — per
//! the measurement playbook's bracket.
//!
//! Seed hygiene: pass `--seed "$SEED_BASE"` (fresh per experiment, shared by
//! every arm and vulnerability of that experiment).
//!
//! ```text
//! cargo run --release --example ab-minor-continuations -- \
//!     --count 200000 --vulnerability none --seed "$SEED_BASE" --xyz
//! ```

use clap::Parser;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::Family;
use pons::bidding::american::{
    set_longer_major_response, set_new_minor_forcing, set_up_the_line, set_xyz,
};
use pons::scoring::{final_contract, imps, ns_score_contract};
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_uncontested, report_brackets, seeded_deals};

/// Minor-opening continuation A/B: baseline vs the selected treatments
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "20000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Base seed — fresh per experiment (`SEED_BASE=$(date +%s)`), shared
    /// across the experiment's arms; random when omitted
    #[arg(short, long)]
    seed: Option<u64>,

    /// Treatment: the longer-major response discipline
    /// (`set_longer_major_response`)
    #[arg(long, default_value_t = false)]
    longer_major: bool,

    /// Treatment: the up-the-line completion (`set_up_the_line`)
    #[arg(long, default_value_t = false)]
    up_the_line: bool,

    /// Treatment: XYZ (`set_xyz`)
    #[arg(long, default_value_t = false)]
    xyz: bool,

    /// Treatment: New Minor Forcing in place of XYZ on the four `1m-1M-1NT`
    /// slots (`set_new_minor_forcing`).  Measured *against XYZ*, not the floor:
    /// both arms run XYZ, the treatment arm swaps NMF onto those four slots, so
    /// the divergent boards isolate the swap.
    #[arg(long, default_value_t = false)]
    nmf: bool,

    /// Forensics: after scoring, print the N worst plain-DD boards for the
    /// treatment — the deal, both arms' auctions, and both contracts — to trace
    /// where a measured loss comes from (the iron rule's divergent-board trace).
    #[arg(long, default_value_t = 0)]
    dump_worst: usize,
}

/// Set all four knobs at once
fn set_knobs(longer_major: bool, up_the_line: bool, xyz: bool, nmf: bool) {
    set_longer_major_response(longer_major);
    set_up_the_line(up_the_line);
    set_xyz(xyz);
    set_new_minor_forcing(nmf);
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    assert!(
        args.longer_major || args.up_the_line || args.xyz || args.nmf,
        "select at least one treatment: --longer-major / --up-the-line / --xyz / --nmf",
    );
    let base = args.seed.unwrap_or_else(rand::random);
    let vul = args.vulnerability;

    // arm 0 = baseline, arm 1 = the selected treatment set.  The knobs are read
    // at book-construction time, so each arm bakes its own books; the
    // longer-major knob is *also* read at classify time by the M6.4 control-bid
    // classifier, so the bidding loop re-sets it per arm inside the worker
    // (thread-locals are per rayon thread).  NMF is measured against an XYZ
    // baseline (it *replaces* XYZ on four slots), so `--nmf` turns XYZ on in
    // both arms; every other treatment is measured against the bare floor.
    let baseline_xyz = args.nmf;
    set_knobs(false, false, baseline_xyz, false);
    let baseline = american().against(Family::NATURAL);
    set_knobs(
        args.longer_major,
        args.up_the_line,
        args.xyz || args.nmf,
        args.nmf,
    );
    let treatment = american().against(Family::NATURAL);
    set_knobs(false, false, false, false);
    let stances = [baseline, treatment];

    // Deals are seeded per board (base + index) so any arm of the experiment
    // replays the identical deal set; bidding is pure and parallelizes, the
    // DD solver stays on the main thread below.
    let deals = seeded_deals(base, args.count);
    let contracts: Vec<[_; 2]> = deals
        .par_iter()
        .enumerate()
        .map(|(index, deal)| {
            let dealer = Seat::ALL[index % 4];
            std::array::from_fn(|arm| {
                set_longer_major_response(arm == 1 && args.longer_major);
                let auction = bid_uncontested(&stances[arm], dealer, vul, deal);
                set_longer_major_response(false);
                final_contract(&auction, dealer)
            })
        })
        .collect();

    // Only boards whose arms diverge can swing; solve those once and score
    // both brackets from the same solved tables.
    let divergent: Vec<usize> = (0..args.count)
        .filter(|&i| contracts[i][0] != contracts[i][1])
        .collect();
    let solve_deals: Vec<FullDeal> = divergent.iter().map(|&i| deals[i]).collect();
    let tables = Solver::lock().solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

    let treatments: Vec<&str> = [
        ("longer-major", args.longer_major),
        ("up-the-line", args.up_the_line),
        ("xyz", args.xyz),
        ("nmf", args.nmf),
    ]
    .iter()
    .filter(|(_, on)| *on)
    .map(|&(name, _)| name)
    .collect();
    println!(
        "=== minor-continuations A/B: {} boards, vulnerability {}, seed {}, baseline [{}], treatment [{}] ===",
        args.count,
        vul,
        base,
        if baseline_xyz { "xyz" } else { "floor" },
        treatments.join(" "),
    );
    println!("(opponents silenced — constructive value only)");
    println!(
        "Divergent boards: {} of {} ({:.2}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );

    report_brackets(args.count, &divergent, &tables, &contracts, vul);

    if args.dump_worst > 0 {
        let mut scored: Vec<(usize, i64)> = divergent
            .iter()
            .zip(tables.iter())
            .map(|(&i, table)| {
                let off = ns_score_contract(contracts[i][0], table, vul);
                let on = ns_score_contract(contracts[i][1], table, vul);
                (i, imps(on - off))
            })
            .collect();
        scored.sort_by_key(|&(_, swing)| swing); // ascending: treatment's worst first
        println!(
            "\n--- {} worst plain-DD boards for the treatment ---",
            args.dump_worst
        );
        for &(i, swing) in scored.iter().take(args.dump_worst) {
            let dealer = Seat::ALL[i % 4];
            let baseline_auction = bid_uncontested(&stances[0], dealer, vul, &deals[i]);
            let treatment_auction = bid_uncontested(&stances[1], dealer, vul, &deals[i]);
            println!("board {i}  swing {swing:+} IMPs (treatment − baseline)  dealer {dealer:?}",);
            println!(
                "  N {}   S {}",
                deals[i][Seat::North],
                deals[i][Seat::South]
            );
            println!("  baseline: {baseline_auction}  → {:?}", contracts[i][0]);
            println!("  treatment:{treatment_auction}  → {:?}", contracts[i][1]);
        }
    }
}
