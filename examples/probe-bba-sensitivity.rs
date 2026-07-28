//! Which rows of a `.bbsa` card actually change BBA's bidding when disclosed?
//!
//! `cards/American.bbsa` has 258 rows, of which 123 are `Not defined` and one is
//! the `Opponent type` meta row.  Auditing all of them against `american()` costs
//! the same whether a row is load-bearing or decorative, so partition first:
//! flip each row on the **opponents' seats only** (the disclosure channel wired
//! by [`BbaOracle::with_opponents`]) and count how many real decisions move.
//!
//! The positions come from an existing anchor shard dump, so the population is
//! genuine pons-vs-BBA auctions rather than invented ones.  Only positions where
//! a *BBA* seat is to act are replayed — those are the decisions disclosure can
//! reach.  Each row is tested at **both** values, because a row whose card value
//! already matches EPBot's default cannot move a call today yet becomes live the
//! moment the audit changes it.
//!
//! The replay doubles as a self-check: with nothing disclosed, EPBot must
//! reproduce the call the dump recorded.  A low agreement rate means the replay
//! is misaligned and every count below is noise, so it is reported first.
//!
//! ```text
//! cargo run --release --features serde --example probe-bba-sensitivity -- \
//!     ab-results/anchor/2026-07-26-eb02d9d/none --boards 200 \
//!     --out docs/ai-bidder/bba-disclosure-sweep.md
//! ```
//!
//! A count alone never says what a row *means*.  `--explain <ROW>` prints the
//! auctions it moves instead of tallying them, which is how the
//! `1NT opening natural` / `NT style` pair was identified as the Stayman and
//! transfer switch (docs/ai-bidder/bba-card-audit.md):
//!
//! ```text
//! cargo run --release --features serde --example probe-bba-sensitivity -- \
//!     ab-results/anchor/2026-07-26-eb02d9d/none --explain "1NT opening natural"
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Hand, Seat};
use std::ffi::c_int;
use std::fmt::Write as _;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::oracle::{BbaOracle, ConventionCard, DEFAULT_LIB, load_bbsa, next_call};
use common::{load_dump, seat_to_act};

/// Partition a convention card into rows that move BBA's calls and rows that do not
#[derive(Parser)]
struct Args {
    /// Shard directories (or single dump files) to draw positions from
    #[arg(required = true)]
    dumps: Vec<String>,

    /// The card whose rows are swept
    #[arg(long, default_value = "cards/American.bbsa")]
    card: String,

    /// Boards to take from each dump; every BBA decision in both tables is used
    #[arg(long, default_value_t = 200)]
    boards: usize,

    /// Write the partition table here as Markdown (stdout gets the summary)
    #[arg(long)]
    out: Option<String>,

    /// EPBot system for BBA's own seats
    #[arg(long, default_value_t = 0)]
    system: c_int,

    /// Instead of sweeping, print the auctions this row moves — the sweep counts
    /// decisions but never says *which*, which is what names a row's semantics.
    #[arg(long, value_name = "ROW")]
    explain: Option<String>,
}

/// One decision BBA made in a recorded auction
struct Position {
    hand: Hand,
    seat: Seat,
    vul: AbsoluteVulnerability,
    auction: Auction,
    /// What the dump recorded BBA bidding here — the replay self-check
    recorded: Call,
}

/// Every BBA decision in one dump's first `boards` boards
///
/// `table_a` seats our pair North/South and `table_b` East/West (see
/// [`common::Board`]), so BBA holds the other pair in each.
fn positions(path: &str, boards: usize) -> Vec<Position> {
    let dump = load_dump(path);
    let vul = dump.vulnerability;
    let mut out = Vec::new();
    for board in dump.boards.iter().take(boards) {
        for (auction, ours_is_ns) in [(&board.table_a, true), (&board.table_b, false)] {
            let mut prefix = Auction::new();
            for &recorded in auction.iter() {
                let seat = seat_to_act(board.dealer, prefix.len());
                let seat_is_ns = matches!(seat, Seat::North | Seat::South);
                if seat_is_ns != ours_is_ns {
                    out.push(Position {
                        hand: board.deal[seat],
                        seat,
                        vul,
                        auction: prefix.clone(),
                        recorded,
                    });
                }
                prefix.push(recorded);
            }
        }
    }
    out
}

/// BBA's call at every position under the current disclosure
fn calls(bba: &BbaOracle, positions: &[Position]) -> Vec<Call> {
    positions
        .iter()
        .map(|p| next_call(bba, p.hand, p.seat, p.vul, &p.auction))
        .collect()
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let lib = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
    let card = load_bbsa(&args.card)?;

    let positions: Vec<Position> = args
        .dumps
        .iter()
        .flat_map(|path| positions(path, args.boards))
        .collect();
    anyhow::ensure!(!positions.is_empty(), "no BBA decisions in the given dumps");
    println!(
        "{} BBA decisions from {} dump(s), sweeping {} rows of {}",
        positions.len(),
        args.dumps.len(),
        card.toggles.len(),
        args.card,
    );

    // Nothing disclosed — must reproduce the dump, or the replay is misaligned.
    let mut bba = BbaOracle::load(&lib, args.system, Vec::new())?;
    let baseline = calls(&bba, &positions);
    let agreed = baseline
        .iter()
        .zip(&positions)
        .filter(|(call, p)| **call == p.recorded)
        .count();
    #[allow(clippy::cast_precision_loss)]
    let agreement = 100.0 * agreed as f64 / positions.len() as f64;
    println!(
        "replay self-check: {agreed}/{} recorded calls reproduced ({agreement:.1}%)",
        positions.len()
    );
    anyhow::ensure!(
        agreement > 99.0,
        "replay reproduces only {agreement:.1}% of recorded calls — the sweep would be noise"
    );

    // `--explain`: name one row's semantics by showing its work.  Same flip as
    // the sweep, but printing the auction instead of counting it.
    if let Some(wanted) = &args.explain {
        let (name, card_value) = card
            .toggles
            .iter()
            .find(|(name, _)| name.to_string_lossy() == *wanted)
            .ok_or_else(|| anyhow::anyhow!("no row named {wanted:?} in {}", args.card))?;
        println!("\n`{wanted}` (card says {card_value}) — auctions it moves:");
        for value in [0, 1] {
            bba = bba.with_opponents(Some(ConventionCard {
                system: card.system,
                toggles: vec![(name.clone(), value)],
            }));
            for ((now, was), p) in calls(&bba, &positions)
                .iter()
                .zip(&baseline)
                .zip(&positions)
            {
                if now != was {
                    let prefix = p
                        .auction
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" ");
                    println!(
                        "  ={value}  {:?} holding {}\n         after [{prefix}]  {was} -> {now}",
                        p.seat, p.hand,
                    );
                }
            }
        }
        return Ok(());
    }

    // Each row at both values, isolated: `set_system` reloads the defaults on
    // their seats first, so only this row differs from the baseline.
    let mut rows: Vec<(String, c_int, usize)> = Vec::with_capacity(card.toggles.len());
    for (name, card_value) in &card.toggles {
        let mut moved = 0;
        for value in [0, 1] {
            bba = bba.with_opponents(Some(ConventionCard {
                system: card.system,
                toggles: vec![(name.clone(), value)],
            }));
            moved += calls(&bba, &positions)
                .iter()
                .zip(&baseline)
                .filter(|(now, was)| now != was)
                .count();
        }
        rows.push((name.to_string_lossy().into_owned(), *card_value, moved));
    }

    let live = rows.iter().filter(|(_, _, moved)| *moved > 0).count();
    println!(
        "partition: {live} live rows, {} cosmetic",
        rows.len() - live
    );

    let mut report = String::new();
    writeln!(report, "# Disclosure sensitivity of `{}`\n", args.card)?;
    writeln!(
        report,
        "{} BBA decisions replayed from `{}`; each row flipped to 0 and to 1 on \
         the seats pons occupies, counting decisions whose call moved. Replay \
         self-check: {agreement:.1}% of recorded calls reproduced with nothing \
         disclosed.\n",
        positions.len(),
        args.dumps.join("`, `"),
    )?;
    for (heading, keep) in [("Live", true), ("Cosmetic", false)] {
        let group: Vec<_> = rows
            .iter()
            .filter(|(_, _, moved)| (*moved > 0) == keep)
            .collect();
        writeln!(report, "## {heading} ({} rows)\n", group.len())?;
        writeln!(report, "| row | card value | decisions moved |")?;
        writeln!(report, "| --- | --- | --- |")?;
        let mut group = group;
        group.sort_by_key(|(_, _, moved)| std::cmp::Reverse(*moved));
        for (name, value, moved) in group {
            writeln!(report, "| {name} | {value} | {moved} |")?;
        }
        report.push('\n');
    }
    match &args.out {
        Some(path) => {
            std::fs::write(path, &report)?;
            println!("wrote {path}");
        }
        None => print!("{report}"),
    }
    Ok(())
}
