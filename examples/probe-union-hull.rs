//! Does recomputing `players` from the union reach the bidder? (`union_hull`)
//!
//! Throwaway reach probe for `docs/pdi.md` follow-on 1.  `Inferences::players`
//! is a redundant cache of `unions[i].hull()`, except where a **post-walk**
//! union collapses against the finished walk — the pre-walk fold cannot see
//! that.  `union_hull` closes the gap; this counts how often it changes what a
//! seat is read as, before any A/B pays for the answer.
//!
//! Replay only: every prefix of both tables' auctions, read twice (knob off and
//! on), the four `get()` hulls compared.  No bidding, no solver.
//!
//! ```sh
//! cargo run --release --features serde --example probe-union-hull -- DUMP_DIR
//! ```

use clap::Parser;
use contract_bridge::Seat;
use contract_bridge::auction::{Auction, Call, display_calls};
use pons::american;
use pons::bidding::Relative;
use pons::bidding::agreements::Agreements;
use pons::bidding::context::relative;
use rayon::prelude::*;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{load_dump, next_call, seat_to_act, vs_bba_agreements};

#[derive(Parser)]
struct Args {
    /// A `bba-gen` dump file or shard directory
    dump: String,

    /// Sample auctions to print
    #[arg(long, default_value_t = 10)]
    show: usize,

    /// Arm `pdi_latch` on **both** sides, so the delta is `union_hull` alone under
    /// a system that authors a post-walk union of its own (docs/pdi.md follow-on 1)
    #[arg(long, default_value_t = false)]
    pdi_latch: bool,

    /// Compare `get()` against `announced()` **inside the OFF arm** instead of
    /// across the two arms: what the book gates read versus what the nets read
    #[arg(long, default_value_t = false)]
    announced_gap: bool,
}

#[derive(Default, Clone)]
struct Tally {
    decisions: u64,
    drifted: u64,
    /// Drifted decisions where the **call** the bidder would make flips
    flipped: u64,
    /// A drifted seat the knob *widened* — must stay 0 (the fold is a narrowing)
    unsound: u64,
    /// Drifted decisions by the seat whose hull moved, relative to the actor
    by_seat: [u64; 4],
    /// Which axis moved: lengths, points, hcp, support_points, suit_hcp
    by_axis: [u64; 5],
    /// Drifted decisions where the seat that moved is the one **to act**
    samples: Vec<String>,
}

impl Tally {
    fn merge(&mut self, other: Self, show: usize) {
        self.decisions += other.decisions;
        self.drifted += other.drifted;
        self.flipped += other.flipped;
        self.unsound += other.unsound;
        for i in 0..4 {
            self.by_seat[i] += other.by_seat[i];
        }
        for i in 0..5 {
            self.by_axis[i] += other.by_axis[i];
        }
        for sample in other.samples {
            if self.samples.len() < show {
                self.samples.push(sample);
            }
        }
    }
}

fn main() {
    let args = Args::parse();
    let dump = load_dump(&args.dump);
    let vul = dump.vulnerability;
    let base = || {
        let mut a = vs_bba_agreements(Agreements::default());
        a.decision.reading.pdi_latch = args.pdi_latch;
        a
    };
    let off = american(&base()).bind();
    let on = american(&{
        let mut a = base();
        a.decision.reading.union_hull = true;
        a
    })
    .bind();

    let (tally, boards) = dump
        .boards
        .par_iter()
        .map(|board| {
            let mut tally = Tally::default();
            let mut any = false;
            for (table, auction) in [("A", &board.table_a), ("B", &board.table_b)] {
                let calls: &[Call] = auction;
                for index in 0..calls.len() {
                    let seat = seat_to_act(board.dealer, index);
                    // Table A seats our pair N/S, table B seats it E/W.
                    let ours = matches!(
                        (table, seat),
                        ("A", Seat::North | Seat::South) | ("B", Seat::East | Seat::West)
                    );
                    if !ours {
                        continue;
                    }
                    tally.decisions += 1;
                    let who = relative(vul, seat);
                    let a = off.infer(who, &calls[..index]);
                    let b = on.infer(who, &calls[..index]);
                    let b = if args.announced_gap { &a } else { &b };
                    let moved: Vec<Relative> = [
                        Relative::Me,
                        Relative::Partner,
                        Relative::Lho,
                        Relative::Rho,
                    ]
                    .into_iter()
                    .filter(|&r| {
                        if args.announced_gap {
                            a.get(r) != a.announced(r)
                        } else {
                            a.get(r) != b.get(r)
                        }
                    })
                    .collect();
                    if moved.is_empty() {
                        continue;
                    }
                    tally.drifted += 1;
                    any = true;
                    for r in &moved {
                        tally.by_seat[*r as usize] += 1;
                        // On must be a subset of off: every box of the union is
                        // inside the walk's hull, and so is the contradiction
                        // fallback.  A widening here refutes the whole idea.
                        let y = if args.announced_gap {
                            a.announced(*r)
                        } else {
                            b.get(*r)
                        };
                        if y.intersect(a.get(*r)) != *y {
                            tally.unsound += 1;
                        }
                        let x = a.get(*r);
                        for (slot, moved) in [
                            x.lengths != y.lengths,
                            x.strength.points != y.strength.points,
                            x.strength.hcp != y.strength.hcp,
                            x.strength.support_points != y.strength.support_points,
                            x.strength.suit_hcp != y.strength.suit_hcp,
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            if moved {
                                tally.by_axis[slot] += 1;
                            }
                        }
                    }
                    // Would the bidder actually call differently?  Same hand,
                    // same prefix, the two pinned partnerships.
                    let mut prefix = Auction::new();
                    for &call in &calls[..index] {
                        prefix.push(call);
                    }
                    let hand = board.deal[seat];
                    let flip = !args.announced_gap
                        && next_call(&off, hand, board.dealer, vul, &prefix)
                            != next_call(&on, hand, board.dealer, vul, &prefix);
                    if flip {
                        tally.flipped += 1;
                    }
                    if flip && tally.samples.len() < args.show {
                        tally.samples.push(format!(
                            "  {table} {:<40} {moved:?} {} -> {}",
                            display_calls(&calls[..index]).to_string(),
                            next_call(&off, hand, board.dealer, vul, &prefix),
                            next_call(&on, hand, board.dealer, vul, &prefix),
                        ));
                    }
                }
            }
            (tally, u64::from(any))
        })
        .reduce(
            || (Tally::default(), 0),
            |mut acc, item| {
                acc.0.merge(item.0, args.show);
                acc.1 += item.1;
                acc
            },
        );

    let pct = |n: u64, d: u64| {
        if d == 0 {
            0.0
        } else {
            100.0 * n as f64 / d as f64
        }
    };
    println!("boards            : {}", dump.boards.len());
    println!("our decisions     : {}", tally.decisions);
    println!(
        "hull drift        : {} decisions ({:.4}%), on {boards} boards ({:.4}%)",
        tally.drifted,
        pct(tally.drifted, tally.decisions),
        pct(boards, dump.boards.len() as u64),
    );
    println!(
        "call flips        : {} decisions ({:.4}%)",
        tally.flipped,
        pct(tally.flipped, tally.decisions),
    );
    println!("unsound widenings : {}", tally.unsound);
    for (i, name) in ["me", "partner", "lho", "rho"].iter().enumerate() {
        println!("  seat {name:<8}: {}", tally.by_seat[i]);
    }
    for (i, name) in ["lengths", "points", "hcp", "support_pts", "suit_hcp"]
        .iter()
        .enumerate()
    {
        println!("  axis {name:<12}: {}", tally.by_axis[i]);
    }
    if !tally.samples.is_empty() {
        println!("samples:");
        for sample in &tally.samples {
            println!("{sample}");
        }
    }
}
