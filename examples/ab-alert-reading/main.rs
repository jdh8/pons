//! Alert-reading A/B: floor blind to strength-showing artificials vs reading them.
//!
//! `project_authored` decided "artificial" purely structurally — a call floors a
//! suit it does not name (Jacoby 2♦ → 5+♥).  That misses the *strength*-showing
//! artificials that floor no foreign suit: the strong 2♣ opening (22+, no shape),
//! its 2♦ waiting / 2♥ double negative, and Puppet 3♣.  Those were read as a
//! natural suit, so partner (and the keyless floor behind it) thought opener held
//! clubs.  Marking every artificial call with an [`Alert`] and reading the alert
//! (`set_alert_reading`) lets the floor suppress the phantom-suit read and project
//! the convention instead.  Both arms run the same 2/1 system; the only difference
//! is the toggle.
//!
//! Read at *runtime* inside `Inferences::read`, so the two arms cannot be
//! interleaved: bid every board with the flag off (baseline), then on (fix), and
//! compare per board.  Opponents are silenced — this is the *constructive* value
//! (our own artificials read correctly by partner); the contested defense-switch
//! value wants a contested harness (e.g. `bba-match`).
//!
//! ```text
//! cargo run --release --example ab-alert-reading -- --count 5000
//! ```

use clap::Parser;
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::Family;
use pons::bidding::set_alert_reading;
use pons::scoring::{final_contract, imps, ns_score_contract};
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::bid_uncontested;

/// Alert-reading A/B: blind-to-strong-artificials floor vs reads-the-alert floor
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "5000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let mut rng = rand::rng();
    let sys = american().against(Family::NATURAL);

    let boards: Vec<(Seat, FullDeal)> = (0..args.count)
        .map(|i| (Seat::ALL[i % 4], full_deal(&mut rng)))
        .collect();

    // The flag is thread-local, so each par_iter worker sets it for its own
    // thread. The two passes stay sequential so the flag is stable within each.
    let vul = args.vulnerability;
    let contracts = |on: bool| {
        boards
            .par_iter()
            .map(|(dealer, deal)| {
                set_alert_reading(on);
                final_contract(&bid_uncontested(&sys, *dealer, vul, deal), *dealer)
            })
            .collect::<Vec<_>>()
    };
    let baseline = contracts(false);
    let fixed = contracts(true);

    // Only boards whose arms diverge can swing; solve those once.
    let divergent: Vec<usize> = (0..args.count)
        .filter(|&i| baseline[i] != fixed[i])
        .collect();
    let solve_deals: Vec<FullDeal> = divergent.iter().map(|&i| boards[i].1).collect();
    let tables = Solver::lock().solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

    let mut points = 0i64;
    let mut total_imps = 0i64;
    for (&i, table) in divergent.iter().zip(tables.iter()) {
        let base = ns_score_contract(baseline[i], table, args.vulnerability);
        let fix = ns_score_contract(fixed[i], table, args.vulnerability);
        points += fix - base;
        total_imps += imps(fix - base);
    }

    println!(
        "=== Alert-reading A/B: {} boards, vulnerability {} ===",
        args.count, args.vulnerability,
    );
    println!("(opponents silenced — constructive value only)");
    println!(
        "Divergent boards: {} of {} ({:.1}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );
    println!(
        "Reads the alert: {points:+} points, {total_imps:+} IMPs ({:+.3} IMPs/board, {:+.3} IMPs/divergent)",
        total_imps as f64 / args.count.max(1) as f64,
        total_imps as f64 / divergent.len().max(1) as f64,
    );
}
