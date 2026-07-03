//! Minor-opening continuation A/B: baseline vs any combination of the
//! longer-major response discipline (`--longer-major`), the up-the-line
//! completion (`--up-the-line`), and the XYZ two-way checkback (`--xyz`).
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
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::Family;
use pons::bidding::american::{set_longer_major_response, set_up_the_line, set_xyz};
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_uncontested, mean_with_ci};

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
}

/// Set all three knobs at once
fn set_knobs(longer_major: bool, up_the_line: bool, xyz: bool) {
    set_longer_major_response(longer_major);
    set_up_the_line(up_the_line);
    set_xyz(xyz);
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    assert!(
        args.longer_major || args.up_the_line || args.xyz,
        "select at least one treatment: --longer-major / --up-the-line / --xyz",
    );
    let base = args.seed.unwrap_or_else(rand::random);
    let vul = args.vulnerability;

    // arm 0 = baseline (all off), arm 1 = the selected treatment set.  The
    // knobs are read at book-construction time, so each arm bakes its own
    // books; the longer-major knob is *also* read at classify time by the
    // M6.4 control-bid classifier, so the bidding loop re-sets it per arm
    // inside the worker (thread-locals are per rayon thread).
    set_knobs(false, false, false);
    let baseline = american().against(Family::NATURAL);
    set_knobs(args.longer_major, args.up_the_line, args.xyz);
    let treatment = american().against(Family::NATURAL);
    set_knobs(false, false, false);
    let stances = [baseline, treatment];

    // Deals are seeded per board (base + index) so any arm of the experiment
    // replays the identical deal set; bidding is pure and parallelizes, the
    // DD solver stays on the main thread below.
    let deals: Vec<FullDeal> = (0..args.count)
        .map(|i| {
            let mut rng = StdRng::seed_from_u64(base.wrapping_add(i as u64));
            full_deal(&mut rng)
        })
        .collect();
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
    ]
    .iter()
    .filter(|(_, on)| *on)
    .map(|&(name, _)| name)
    .collect();
    println!(
        "=== minor-continuations A/B: {} boards, vulnerability {}, seed {}, treatment [{}] ===",
        args.count,
        vul,
        base,
        treatments.join(" "),
    );
    println!("(opponents silenced — constructive value only)");
    println!(
        "Divergent boards: {} of {} ({:.2}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );

    for (label, scorer) in [
        ("plain DD", ns_score_contract as fn(_, _, _) -> i64),
        ("perfect defense", ns_score_pd),
    ] {
        let mut per_board = vec![0i64; args.count];
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
