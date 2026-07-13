//! 1NT invite-acceptance A/B: blind opener vs an opener that reads the raise.
//!
//! `american()` had no opener rebid after `1NT–2NT`, so opener passed the
//! invitation even with a maximum — `Inferences::read` was silent on a notrump
//! raise of our *own* 1NT opening (its NT-raise reading was gated to one-of-a-suit
//! openings).  Teaching the inference that `1NT–2NT` shows ≈8–9 (and `1NT–3NT` 10+)
//! lets the keyless floor see responder's strength and accept game opposite a
//! maximum — no hand-authored acceptance node.  Both arms run the same 2/1 system;
//! the only difference is the [`set_nt_invite_inference`] toggle.
//!
//! Unlike the Meckstroth toggle (read at book-construction time), this one is read
//! at *runtime* inside `Inferences::read`, so the two arms cannot be interleaved:
//! we bid every board with the flag off (baseline), then again with it on (fix),
//! and compare per board.  Opponents are silenced — this is the constructive value.
//! Divergent boards are solved once and scored with **both** brackets — plain DD
//! and perfect defense — per the measurement playbook.
//!
//! ```text
//! cargo run --release --example ab-nt-invite -- --count 5000 --seed "$SEED_BASE"
//! ```

use clap::Parser;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::Family;
use pons::bidding::set_nt_invite_inference;
use pons::scoring::final_contract;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_uncontested, report_brackets, seeded_deals};

/// 1NT invite-acceptance A/B: blind opener vs reads-the-raise opener
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "5000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Base seed — fresh per experiment (`SEED_BASE=$(date +%s)`), shared
    /// across arms/vuls; random when omitted
    #[arg(short, long)]
    seed: Option<u64>,
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let base = args.seed.unwrap_or_else(rand::random);
    let vul = args.vulnerability;
    let sys = american().against(Family::NATURAL);

    // Deals are seeded per board (base + index) so every arm/vul of the
    // experiment replays the identical stream.
    let deals = seeded_deals(base, args.count);

    // The flag is thread-local, so each par_iter worker sets it for its own
    // thread. The two passes stay sequential so the flag is stable within each.
    let bid_pass = |on: bool| {
        deals
            .par_iter()
            .enumerate()
            .map(|(i, deal)| {
                let dealer = Seat::ALL[i % 4];
                set_nt_invite_inference(on);
                final_contract(&bid_uncontested(&sys, dealer, vul, deal), dealer)
            })
            .collect::<Vec<_>>()
    };
    let baseline = bid_pass(false);
    let fixed = bid_pass(true);
    // [off = baseline (flag off), on = fixed (flag on)].
    let contracts: Vec<[_; 2]> = (0..args.count).map(|i| [baseline[i], fixed[i]]).collect();

    // Only boards whose arms diverge can swing; solve those once and score both
    // brackets (plain DD + perfect defense) from the same tables.
    let divergent: Vec<usize> = (0..args.count)
        .filter(|&i| contracts[i][0] != contracts[i][1])
        .collect();
    let solve_deals: Vec<FullDeal> = divergent.iter().map(|&i| deals[i]).collect();
    let tables = Solver::lock().solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

    println!(
        "=== 1NT invite-acceptance A/B: {} boards, vulnerability {}, seed {} ===",
        args.count, vul, base,
    );
    println!("(opponents silenced — constructive value only)");
    println!(
        "Divergent boards: {} of {} ({:.2}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );

    report_brackets(args.count, &divergent, &tables, &contracts, vul);
}
