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
//! value wants a contested harness (e.g. `bba-match`).  Divergent boards are
//! solved once and scored with **both** brackets — plain DD and perfect defense.
//!
//! ```text
//! cargo run --release --example ab-alert-reading -- --count 5000 --seed "$SEED_BASE"
//! ```

use clap::Parser;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::Family;
use pons::bidding::set_alert_reading;
use pons::scoring::final_contract;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_uncontested, report_brackets, seeded_deals};

/// Alert-reading A/B: blind-to-strong-artificials floor vs reads-the-alert floor
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
                set_alert_reading(on);
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
        "=== Alert-reading A/B: {} boards, vulnerability {}, seed {} ===",
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
