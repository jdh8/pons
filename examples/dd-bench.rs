//! Double-dummy throughput benchmark: full tables and trick-one lead solves
//!
//! Sizes the solver at the process's `DDSS_THREADS`: random deals, full
//! tables through `solve_deals` and trick-one `Legal` lead boards through
//! `solve_boards`, with wall time per chunk on stderr and the totals on
//! stdout.  One process cannot use a large box (DDS packs 200 boards per
//! call), so run several and sum; the measured layouts and rules are in
//! `docs/shared-machine-data-gen.md` § Sizing the solver.

use clap::Parser;
use contract_bridge::{Seat, Strain};
use ddss::{Board, CurrentTrick, NonEmptyStrainFlags, Objective, Target};
use std::time::Instant;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::seeded_deals;

#[derive(Parser)]
struct Args {
    /// Full tables to solve (all five strains, four declarers)
    #[arg(long, default_value_t = 4000)]
    tables: usize,
    /// Trick-one `Legal` lead boards to solve (notrump, West on lead)
    #[arg(long, default_value_t = 4000)]
    boards: usize,
    /// Deals per `solve_deals` / `solve_boards` call
    #[arg(long, default_value_t = 4096)]
    batch: usize,
    #[arg(long, default_value_t = 20261019)]
    seed: u64,
}

fn main() {
    let args = Args::parse();
    let solver = pons::dd::solver();
    let info = ddss::system_info();
    println!("DDSS_THREADS={:?} {info}", pons::dd::threads());
    let n = args.tables.max(args.boards);
    let deals = seeded_deals(args.seed, n);

    if args.tables > 0 {
        let deals = &deals[..args.tables];
        let start = Instant::now();
        let mut checksum = 0u64;
        for chunk in deals.chunks(args.batch) {
            let t = Instant::now();
            let tables = solver.solve_deals(chunk, NonEmptyStrainFlags::ALL);
            for table in &tables {
                checksum += u64::from(u8::from(table[Strain::Notrump].get(Seat::North)));
            }
            eprintln!(
                "  tables chunk {} in {:.2}s = {:.3} ms/table",
                chunk.len(),
                t.elapsed().as_secs_f64(),
                1e3 * t.elapsed().as_secs_f64() / chunk.len() as f64
            );
        }
        let s = start.elapsed().as_secs_f64();
        println!(
            "tables: {} in {:.2}s = {:.3} ms/table wall, {:.1} tables/s (checksum {checksum})",
            deals.len(),
            s,
            1e3 * s / deals.len() as f64,
            deals.len() as f64 / s
        );
    }

    if args.boards > 0 {
        let objectives: Vec<_> = deals[..args.boards]
            .iter()
            .map(|&deal| Objective {
                board: Board::try_new(deal.into(), CurrentTrick::new(Strain::Notrump, Seat::West))
                    .expect("fresh deal"),
                target: Target::Legal,
            })
            .collect();
        let start = Instant::now();
        let mut checksum = 0usize;
        for chunk in objectives.chunks(args.batch) {
            let t = Instant::now();
            let found = solver.solve_boards(chunk);
            checksum += found.iter().map(|f| f.plays.len()).sum::<usize>();
            eprintln!(
                "  boards chunk {} in {:.2}s = {:.3} ms/board",
                chunk.len(),
                t.elapsed().as_secs_f64(),
                1e3 * t.elapsed().as_secs_f64() / chunk.len() as f64
            );
        }
        let s = start.elapsed().as_secs_f64();
        println!(
            "boards: {} in {:.2}s = {:.3} ms/board wall, {:.1} boards/s (checksum {checksum})",
            objectives.len(),
            s,
            1e3 * s / objectives.len() as f64,
            objectives.len() as f64 / s
        );
    }
}
