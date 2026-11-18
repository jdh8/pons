//! Where does the default system still answer with calls it cannot make?
//!
//! Rows drop illegal rules from exact tables at build time
//! (`Rules::legal_after`), but a guarded table or a rebase answers auctions
//! other than its key, so the filter cannot see them.  This bids seeded
//! self-play and classifies every decision into
//!
//! - `forced`: the answer has mass, none of it on a legal call, so the driver
//!   passes for want of one (the `1♠ - 2♥ - 3♥ -` mechanism);
//! - `empty`: the answer has no mass at all (a rebase's rejection reads as
//!   Pass);
//! - `top`: the best call is illegal but a legal one stands behind it.
//!
//! ```sh
//! cargo run --release --example probe-illegal-mass -- -c 400000
//! ```

use clap::Parser;
use contract_bridge::auction::Auction;
use contract_bridge::{AbsoluteVulnerability, Seat};
use pons::american_default;
use rayon::prelude::*;
use std::collections::HashMap;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{next_call, seat_to_act, seeded_deals};
use pons::bidding::context::relative;

#[derive(Parser)]
struct Args {
    /// Deals to bid
    #[arg(short, long, default_value = "400000")]
    count: usize,

    /// Deal seed base
    #[arg(long, default_value = "1")]
    seed: u64,

    /// Auctions to list per kind
    #[arg(long, default_value = "40")]
    top: usize,
}

/// (kind, exact node?, rebases, auction) → (decisions, one hand)
type Tally = HashMap<(&'static str, bool, usize, String), (usize, String)>;

fn main() {
    let args = Args::parse();
    let partnership = american_default().bind();
    let tally: Tally = seeded_deals(args.seed, args.count)
        .into_par_iter()
        .enumerate()
        .fold(Tally::new, |mut tally, (index, deal)| {
            let dealer = Seat::ALL[index % 4];
            let vul = [
                AbsoluteVulnerability::NONE,
                AbsoluteVulnerability::NS,
                AbsoluteVulnerability::EW,
                AbsoluteVulnerability::ALL,
            ][(index / 4) % 4];
            let mut auction = Auction::new();
            while !auction.has_ended() {
                let seat = seat_to_act(dealer, auction.len());
                let hand = deal[seat];
                if let Some((logits, provenance)) =
                    partnership.classify_with_provenance(hand, relative(vul, seat), &auction)
                {
                    let kind = common::illegal_mass_kind(&logits, &auction);
                    if let Some(kind) = kind {
                        let calls: Vec<String> = auction.iter().map(|c| format!("{c}")).collect();
                        let key = (
                            kind,
                            provenance.fallback.is_none(),
                            provenance.rebases,
                            calls.join(" "),
                        );
                        let entry = tally.entry(key).or_insert_with(|| (0, format!("{hand}")));
                        entry.0 += 1;
                    }
                }
                auction.push(next_call(&partnership, hand, dealer, vul, &auction));
            }
            tally
        })
        .reduce(Tally::new, |mut a, b| {
            for (key, (n, hand)) in b {
                a.entry(key).or_insert((0, hand)).0 += n;
            }
            a
        });

    for kind in ["forced", "empty", "top"] {
        let mut rows: Vec<_> = tally.iter().filter(|(key, _)| key.0 == kind).collect();
        // Ties broken on the auction so the listing is stable across runs.
        rows.sort_by(|a, b| b.1.0.cmp(&a.1.0).then_with(|| a.0.3.cmp(&b.0.3)));
        let total: usize = rows.iter().map(|(_, (n, _))| n).sum();
        println!(
            "## {kind}: {total} decisions at {} auctions ({} deals)",
            rows.len(),
            args.count
        );
        for ((_, exact, rebases, auction), (n, hand)) in rows.into_iter().take(args.top) {
            let node = if *exact { "exact" } else { "fallback" };
            println!("{n:6}  {node:8} rebases={rebases}  {auction}  | {hand}");
        }
    }
}
