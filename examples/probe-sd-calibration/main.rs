//! Calibrate the three play brackets per contract level against Pavlicek.
//!
//! Plain DD, sd-lead ([`single_dummy_lead_tricks`]), and the sd-declarer
//! playout ([`single_dummy_playout`]) model progressively more of real play.
//! Pavlicek's actual-vs-DD study (rpbridge.net/8j45.htm) gives the target
//! shape: at 1NT the table *beats* DD by ≈+7pp of make-rate (the blind lead),
//! the gap tapering to zero as the level rises; at slam level the table makes
//! *fewer* contracts than DD promises (declarer misguesses).  This probe bids
//! random boards out in self-play, prices every reached contract under all
//! three brackets, and reports make-rate and mean declarer tricks per level —
//! the sd-lead column should reproduce the fading lead gap, and the
//! sd-declarer column the growing misguess haircut.
//!
//! Slam-level rows are rare in self-play, so `--min-level-count` keeps dealing
//! until every level from 1 to `--target-level` has enough contracts (bounded
//! by `--max-batches`).  The playout is sequential per board; expect minutes,
//! not seconds.
//!
//! ```text
//! cargo run --release --example probe-sd-calibration -- --count 20000
//! ```

use clap::Parser;
use contract_bridge::auction::Auction;
use contract_bridge::{AbsoluteVulnerability, Contract, FullDeal, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::Family;
use pons::bidding::context::relative;
use pons::scoring::final_contract;
use pons::single_dummy::{LeadQuestion, single_dummy_leads, single_dummy_playout};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_out, seat_to_act, seeded_deals};

#[derive(Parser)]
#[command(about = "Per-level make-rates under plain DD, sd-lead, and sd-declarer")]
struct Args {
    /// Boards to bid per batch (self-play, dealer rotates)
    #[arg(short, long, default_value_t = 20_000)]
    count: usize,
    /// Deal seed base (board i seeded base+i); fresh per experiment
    #[arg(long, default_value_t = 20_260_716)]
    seed: u64,
    /// Vulnerability the boards are bid and scored at
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,
    /// Worlds per blind lead and per declarer decision
    #[arg(long, default_value_t = 16)]
    sd_worlds: usize,
    /// Cap of playouts per contract level (the DD and sd-lead columns still
    /// see every contract; only the expensive playout is subsampled)
    #[arg(long, default_value_t = 500)]
    per_level: usize,
    /// Keep dealing batches until every level up to --target-level has this
    /// many playouts (0 = a single batch)
    #[arg(long, default_value_t = 200)]
    min_level_count: usize,
    /// Highest level the top-up loop chases (7 = grands; they are so rare
    /// that chasing them can exhaust --max-batches)
    #[arg(long, default_value_t = 6)]
    target_level: u8,
    /// Upper bound on dealt batches while topping up rare levels
    #[arg(long, default_value_t = 50)]
    max_batches: usize,
}

/// One reached contract: its level, whether the bid was made under each
/// bracket, and each bracket's declarer tricks.
struct Row {
    level: u8,
    need: u8,
    dd: u8,
    sd_lead: u8,
    sd_line: Option<u8>,
}

fn main() {
    let args = Args::parse();
    let stance = american().against(Family::NATURAL);
    let mut rows: Vec<Row> = Vec::new();
    let mut playouts_at = [0usize; 8];

    for batch in 0..args.max_batches.max(1) {
        // Deal and bid this batch (bidding parallelizes; the solver never
        // leaves the main thread).
        let deals = seeded_deals(args.seed + (batch * args.count) as u64, args.count);
        let boards: Vec<(Seat, FullDeal, Auction)> = deals
            .into_par_iter()
            .enumerate()
            .map(|(i, deal)| {
                let dealer = Seat::ALL[i % 4];
                let auction = bid_out(&stance, &stance, true, dealer, args.vulnerability, &deal);
                (dealer, deal, auction)
            })
            .collect();

        // Plain DD for every reached contract, batched in one fan-out.
        let reached: Vec<(Seat, FullDeal, Auction, Contract, Seat)> = boards
            .into_iter()
            .filter_map(|(dealer, deal, auction)| {
                let (contract, declarer) = final_contract(&auction, dealer)?;
                Some((dealer, deal, auction, contract, declarer))
            })
            .collect();
        let solve: Vec<FullDeal> = reached.iter().map(|&(_, deal, ..)| deal).collect();
        let tables = Solver::lock().solve_deals(&solve, NonEmptyStrainFlags::ALL);

        // All blind leads in one pooled solve (straggler-bound otherwise),
        // then the expensive playouts only on the per-level subsample.
        let mut rng = StdRng::seed_from_u64(args.seed ^ 0x5dca_11b8 ^ batch as u64);
        let view = |auction: &Auction, dealer: Seat, seat: Seat| {
            let cut = (auction.len().saturating_sub(3)..=auction.len())
                .find(|&len| seat_to_act(dealer, len) == seat)
                .expect("one of four consecutive lengths reaches every seat");
            stance.infer(relative(args.vulnerability, seat), &auction[..cut])
        };
        let questions: Vec<LeadQuestion> = reached
            .iter()
            .map(
                |&(dealer, deal, ref auction, contract, declarer)| LeadQuestion {
                    deal,
                    strain: contract.bid.strain,
                    declarer,
                    inferences: view(auction, dealer, declarer.lho()),
                },
            )
            .collect();
        let mut leads = Vec::with_capacity(questions.len());
        for chunk in questions.chunks(4096) {
            leads.extend(single_dummy_leads(chunk, &mut rng, args.sd_worlds));
        }

        for (((dealer, deal, auction, contract, declarer), table), (lead, lead_tricks)) in
            reached.into_iter().zip(tables).zip(leads)
        {
            let level = contract.bid.level.get();
            let sd_line = (playouts_at[usize::from(level)] < args.per_level).then(|| {
                playouts_at[usize::from(level)] += 1;
                u8::from(single_dummy_playout(
                    &deal,
                    contract.bid.strain,
                    declarer,
                    lead,
                    &view(&auction, dealer, declarer),
                    &mut rng,
                    args.sd_worlds,
                ))
            });
            rows.push(Row {
                level,
                need: 6 + level,
                dd: u8::from(table[contract.bid.strain].get(declarer)),
                sd_lead: u8::from(lead_tricks),
                sd_line,
            });
        }

        let filled = (1..=args.target_level)
            .all(|level| playouts_at[usize::from(level)] >= args.min_level_count);
        if filled || args.min_level_count == 0 {
            break;
        }
        eprintln!(
            "batch {batch}: playouts per level {:?}, topping up…",
            &playouts_at[1..=usize::from(args.target_level)]
        );
    }

    println!(
        "=== sd calibration: {} contracts, vul {}, {} worlds, seed {} ===",
        rows.len(),
        args.vulnerability,
        args.sd_worlds,
        args.seed,
    );
    println!(
        "{:>5} {:>6} | {:>8} {:>8} {:>7} | {:>6} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "level",
        "n",
        "DD mk%",
        "lead mk%",
        "Δlead",
        "n line",
        "DD mk%*",
        "lead%*",
        "line mk%",
        "Δguess*",
        "Δtable*",
    );
    println!(
        "(Δlead = blind lead − DD, all contracts.  Starred columns are the playout subsample: \
         Δguess = playout − sd-lead, the pure misguess haircut; Δtable = playout − DD, the \
         full table-proxy shift.)"
    );
    for level in 1..=7u8 {
        let at: Vec<&Row> = rows.iter().filter(|row| row.level == level).collect();
        if at.is_empty() {
            continue;
        }
        #[allow(clippy::cast_precision_loss)]
        let pct = |made: usize, of: usize| 100.0 * made as f64 / of.max(1) as f64;
        let made = |set: &[&Row], tricks: fn(&Row) -> u8| {
            pct(
                set.iter().filter(|row| tricks(row) >= row.need).count(),
                set.len(),
            )
        };
        let lined: Vec<&Row> = at
            .iter()
            .filter(|row| row.sd_line.is_some())
            .copied()
            .collect();
        let dd_all = made(&at, |row| row.dd);
        let lead_all = made(&at, |row| row.sd_lead);
        let dd_sub = made(&lined, |row| row.dd);
        let lead_sub = made(&lined, |row| row.sd_lead);
        let line_sub = made(&lined, |row| row.sd_line.expect("filtered Some"));
        println!(
            "{level:>5} {:>6} | {:>7.1}% {:>7.1}% {:>+6.1}pp | {:>6} {:>7.1}% {:>7.1}% {:>7.1}% {:>+6.1}pp {:>+6.1}pp",
            at.len(),
            dd_all,
            lead_all,
            lead_all - dd_all,
            lined.len(),
            dd_sub,
            lead_sub,
            line_sub,
            line_sub - lead_sub,
            line_sub - dd_sub,
        );
    }
}
