//! Bidding-only wall time of one floor, both sides seated with it (no DD)
//!
//! The floor sweep's KR3 gate (docs/archive/floor-sweep.md, Phase 1): time
//! `american-file` at `K = 1` vs the candidate `K` on the same seeded deals.
//! Single-threaded, so the numbers are per-core latency, not throughput.
//!
//! ```text
//! PONS_FLOOR_WEIGHTS=a.f32,b.f32 cargo run --release --features serde \
//!   --example floor-timing -- --floor american-file --count 20000
//! ```

use clap::Parser;
use contract_bridge::{AbsoluteVulnerability, Seat};
use pons::bidding::agreements::Agreements;
use std::time::Instant;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_out, seat_floor, seeded_deals, vs_bba_agreements};

#[derive(Parser)]
struct Args {
    /// Floor name, as `bba-gen --our-floor` takes it
    #[arg(long, default_value = "american")]
    floor: String,
    #[arg(long, default_value_t = 20_000)]
    count: usize,
    #[arg(long, default_value_t = 1)]
    seed: u64,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let partnership = seat_floor(&args.floor, &vs_bba_agreements(Agreements::default()))?;
    let deals = seeded_deals(args.seed, args.count);
    let start = Instant::now();
    let mut calls = 0;
    for (i, deal) in deals.iter().enumerate() {
        let dealer = Seat::ALL[i % 4];
        use AbsoluteVulnerability as V;
        let vul = [V::NONE, V::NS, V::EW, V::ALL][i / 4 % 4];
        calls += bid_out(&partnership, &partnership, true, dealer, vul, deal).len();
    }
    let elapsed = start.elapsed();
    println!(
        "{}: {} boards, {calls} calls, {:.3} s, {:.2} µs/board, {:.3} µs/call",
        args.floor,
        args.count,
        elapsed.as_secs_f64(),
        elapsed.as_secs_f64() * 1e6 / args.count as f64,
        elapsed.as_secs_f64() * 1e6 / calls as f64,
    );
    Ok(())
}
