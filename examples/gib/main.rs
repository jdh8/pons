//! GIB hand-record tool: **read**, **generate**, and **verify** GIB DD files.
//!
//! A GIB file is one `<West-first PBN>:<20 hex DD digits>` line per deal (see
//! [`pons::gib`]). Double-dummy solving is the expensive step; the tail caches
//! it, so a database produced once is reused for free. With this tool every
//! machine can independently produce a shard — `generate` is deterministic in
//! its `--seed`, so shards from distinct seeds just concatenate into a bigger
//! database (`cat shard-*.txt > all.txt`), no online coordination needed.
//!
//! ```text
//! gib generate --count 100000 --seed 1 --out shard-1.txt
//! gib verify shard-1.txt        # re-solve and confirm the cached tails
//! gib read shard-1.txt | head   # human-readable deal + DD grid
//! ```

use clap::{Parser, Subcommand};
use contract_bridge::deck::full_deal;
use contract_bridge::{FullDeal, Seat, Strain};
use ddss::{NonEmptyStrainFlags, Solver, TrickCountTable};
use pons::gib;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::io::{BufWriter, Write};

#[derive(Parser)]
#[command(about = "Read, generate, and verify GIB double-dummy deal files")]
struct Args {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Pretty-print every deal and its DD table.
    Read { file: String },
    /// Deal random boards, solve them, and write GIB lines.
    Generate {
        /// Number of deals to produce
        #[arg(long, default_value_t = 1000)]
        count: usize,
        /// RNG seed (distinct seeds give disjoint, concatenable shards)
        #[arg(long, default_value_t = 0)]
        seed: u64,
        /// Output file (default: stdout)
        #[arg(long)]
        out: Option<String>,
    },
    /// Re-solve every deal and check the stored DD tail matches.
    Verify { file: String },
}

/// Strains in GIB tail order, with display labels for `read`.
const STRAINS: [(&str, Strain); 5] = [
    ("NT", Strain::Notrump),
    ("S", Strain::Spades),
    ("H", Strain::Hearts),
    ("D", Strain::Diamonds),
    ("C", Strain::Clubs),
];

fn main() -> std::io::Result<()> {
    match Args::parse().cmd {
        Cmd::Read { file } => read(&file),
        Cmd::Generate { count, seed, out } => generate(count, seed, out.as_deref()),
        Cmd::Verify { file } => verify(&file),
    }
}

fn read(file: &str) -> std::io::Result<()> {
    let text = std::fs::read_to_string(file)?;
    let stdout = std::io::stdout();
    let mut w = BufWriter::new(stdout.lock());
    for (i, line) in text.lines().enumerate() {
        let Some((deal, table)) = gib::parse_line(line) else {
            continue;
        };
        writeln!(w, "# {}: {}", i + 1, deal.display(Seat::West))?;
        writeln!(w, "        N   E   S   W")?;
        for (label, strain) in STRAINS {
            let row = table[strain];
            writeln!(
                w,
                "  {label:>4} {:>3} {:>3} {:>3} {:>3}",
                row.get(Seat::North).get(),
                row.get(Seat::East).get(),
                row.get(Seat::South).get(),
                row.get(Seat::West).get(),
            )?;
        }
    }
    w.flush()
}

fn generate(count: usize, seed: u64, out: Option<&str>) -> std::io::Result<()> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut w: BufWriter<Box<dyn Write>> = BufWriter::new(match out {
        Some(path) => Box::new(std::fs::File::create(path)?),
        None => Box::new(std::io::stdout()),
    });
    // Solve in chunks so memory stays flat and output streams for huge files.
    const CHUNK: usize = 4096;
    let mut done = 0;
    while done < count {
        let n = CHUNK.min(count - done);
        let deals: Vec<FullDeal> = (0..n).map(|_| full_deal(&mut rng)).collect();
        let tables = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);
        for (deal, table) in deals.iter().zip(&tables) {
            writeln!(w, "{}", gib::format_line(deal, table))?;
        }
        done += n;
    }
    w.flush()?;
    eprintln!("gib generate: wrote {count} deals (seed {seed})");
    Ok(())
}

fn verify(file: &str) -> std::io::Result<()> {
    let text = std::fs::read_to_string(file)?;
    let parsed: Vec<(FullDeal, TrickCountTable)> =
        text.lines().filter_map(gib::parse_line).collect();
    let deals: Vec<FullDeal> = parsed.iter().map(|&(deal, _)| deal).collect();
    let solved = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);

    let mut mismatches = 0usize;
    for (i, ((_, stored), fresh)) in parsed.iter().zip(&solved).enumerate() {
        if stored != fresh {
            mismatches += 1;
            if mismatches <= 10 {
                eprintln!(
                    "line {}: stored {:X} != solved {:X}",
                    i + 1,
                    stored.gib(),
                    fresh.gib(),
                );
            }
        }
    }
    println!(
        "gib verify: {} deals, {mismatches} mismatch(es)",
        parsed.len()
    );
    if mismatches > 0 {
        std::process::exit(1);
    }
    Ok(())
}
