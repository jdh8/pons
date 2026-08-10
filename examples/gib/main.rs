//! GIB hand-record tool: **read**, **generate**, **verify**, and **convert**
//! DD deal files.
//!
//! Two formats, chosen by extension: `.pdd` is the compact binary format
//! ([`pons::pdd`]); anything else is GIB text, one
//! `<West-first PBN>:<20 hex DD digits>` line per deal ([`pons::gib`]).
//! Readers sniff the magic, so every subcommand accepts either. Double-dummy
//! solving is the expensive step; the file caches it, so a database produced
//! once is reused for free. With this tool every machine can independently
//! produce a shard — `generate` is deterministic in its `--seed`, so shards
//! from distinct seeds just concatenate into a bigger database
//! (`cat shard-*.txt > all.txt`, or `convert shard-* --out all.pdd`), no
//! online coordination needed.
//!
//! Determinism also makes a shard *resumable*: the deals on disk are an exact
//! prefix of the seed's stream, so `--append` replays the RNG past them — free,
//! since only solving is expensive — and continues where a killed run stopped.
//!
//! ```text
//! gib generate --count 100000 --seed 1 --out shard-1.pdd
//! gib generate --append --count 100000 --seed 1 --out shard-1.pdd  # now 200k
//! gib verify shard-1.pdd        # re-solve and confirm the cached tables
//! gib read shard-1.pdd | head   # human-readable deal + DD grid
//! gib convert shard-1.pdd --out shard-1.txt   # binary <-> text
//! ```

use clap::{Parser, Subcommand};
use contract_bridge::deck::full_deal;
use contract_bridge::{FullDeal, Seat, Strain};
use core::num::NonZero;
use ddss::{NonEmptyStrainFlags, Solver, TrickCountTable};
use pons::{gib, pdd};
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
        /// Grow an existing `--out` shard instead of overwriting it: replay
        /// `--seed` past the deals already stored and append `--count` more.
        /// Only valid on a file this seed produced — the deals on disk must be
        /// a prefix of its stream.
        #[arg(long)]
        append: bool,
        /// Cap the DDS thread pool (default: one worker per hardware core).
        /// Under a background QoS class, cap at the efficiency-core count so
        /// the workers stop oversubscribing the cores the OS grants.
        #[arg(long)]
        threads: Option<NonZero<usize>>,
    },
    /// Re-solve every deal and check the stored DD table matches.
    Verify { file: String },
    /// Rewrite deal files into the format named by the output extension.
    Convert {
        /// Input files in either format (concatenated in order)
        inputs: Vec<String>,
        /// Output file: `.pdd` -> binary, anything else -> GIB text
        #[arg(long)]
        out: String,
    },
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
        Cmd::Generate {
            count,
            seed,
            out,
            append,
            threads,
        } => generate(count, seed, out.as_deref(), append, threads),
        Cmd::Verify { file } => verify(&file),
        Cmd::Convert { inputs, out } => convert(&inputs, &out),
    }
}

/// Whether an output path names the binary format.
fn is_pdd(path: &str) -> bool {
    path.ends_with(".pdd")
}

fn read(file: &str) -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut w = BufWriter::new(stdout.lock());
    for (i, (deal, table)) in pdd::load(file)?.iter().enumerate() {
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

/// Write one deal in the format `binary` names.
fn write_deal(
    w: &mut impl Write,
    binary: bool,
    deal: &FullDeal,
    table: &TrickCountTable,
) -> std::io::Result<()> {
    if binary {
        w.write_all(&pdd::encode_row(deal, table))
    } else {
        writeln!(w, "{}", gib::format_line(deal, table))
    }
}

/// Bytes per GIB text record: the 88-char line plus its newline.
const TEXT_LEN: u64 = 89;

/// Prepare `path` to be appended to, returning the deals it already holds.
///
/// `None` means there is nothing to resume — no file, or one too short to even
/// carry the header — so the caller should create it from scratch. A killed
/// generator leaves a partial record (the output buffer is not a record
/// multiple), which every reader rejects as truncated; that ragged tail is
/// trimmed here, discarding fewer bytes than one record and never a whole deal.
fn resume(path: &str, binary: bool) -> std::io::Result<Option<u64>> {
    let Ok(len) = std::fs::metadata(path).map(|meta| meta.len()) else {
        return Ok(None);
    };
    let head = if binary { pdd::MAGIC.len() as u64 } else { 0 };
    if len < head {
        return Ok(None);
    }
    // The scavenger adopts shards by glob, so refuse to append this seed's
    // stream to bytes we did not write: a bad guess corrupts silently.
    if binary {
        let mut magic = [0; pdd::MAGIC.len()];
        std::io::Read::read_exact(&mut std::fs::File::open(path)?, &mut magic)?;
        if magic != pdd::MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("{path} is not a .pdd file"),
            ));
        }
    }
    let (done, record) = if binary {
        (pdd::rows_in(len), pdd::ROW_LEN as u64)
    } else {
        (len / TEXT_LEN, TEXT_LEN)
    };
    let clean = head + done * record;
    if len != clean {
        eprintln!("gib generate: trimming {} ragged byte(s)", len - clean);
        std::fs::OpenOptions::new()
            .write(true)
            .open(path)?
            .set_len(clean)?;
    }
    Ok(Some(done))
}

fn generate(
    count: usize,
    seed: u64,
    out: Option<&str>,
    append: bool,
    threads: Option<NonZero<usize>>,
) -> std::io::Result<()> {
    // Apply the pool cap up front and report the effective size — the
    // observable that E-core confinement is verified against.  The chunk
    // loop re-locks with the same value, which ddss treats as a free re-lock.
    drop(Solver::lock(threads));
    eprintln!(
        "gib generate: {} DDS threads",
        ddss::system_info().num_threads()
    );
    let mut rng = StdRng::seed_from_u64(seed);
    let binary = out.is_some_and(is_pdd);
    // A shard is an exact prefix of its seed's stream, so resuming is just
    // replaying the RNG past what is stored — nanoseconds a deal, against
    // milliseconds to solve one, so the skipped deals are effectively free.
    let done = match out.filter(|_| append) {
        Some(path) => resume(path, binary)?,
        None => None,
    };
    if let Some(done) = done {
        eprintln!("gib generate: appending after {done} deals");
        for _ in 0..done {
            let _ = full_deal(&mut rng);
        }
    }
    let mut w: BufWriter<Box<dyn Write>> = BufWriter::new(match out {
        Some(path) if done.is_some() => {
            Box::new(std::fs::OpenOptions::new().append(true).open(path)?)
        }
        Some(path) => Box::new(std::fs::File::create(path)?),
        None => Box::new(std::io::stdout()),
    });
    if binary && done.is_none() {
        w.write_all(&pdd::MAGIC)?;
    }
    // Solve in chunks so memory stays flat and output streams for huge files.
    const CHUNK: usize = 4096;
    let mut done = 0;
    // ponytail: serial by design. `Solver::lock` takes the process-global DDS
    // lock and already fans each 4096-deal chunk across the whole pool, so rayon
    // out here would oversubscribe the box, not speed it up (CLAUDE.md:102);
    // dealing a chunk is microseconds against milliseconds a board to solve.
    while done < count {
        let n = CHUNK.min(count - done);
        let deals: Vec<FullDeal> = (0..n).map(|_| full_deal(&mut rng)).collect();
        let tables = Solver::lock(threads).solve_deals(&deals, NonEmptyStrainFlags::ALL);
        for (deal, table) in deals.iter().zip(&tables) {
            write_deal(&mut w, binary, deal, table)?;
        }
        done += n;
    }
    w.flush()?;
    eprintln!("gib generate: wrote {count} deals (seed {seed})");
    Ok(())
}

fn verify(file: &str) -> std::io::Result<()> {
    let parsed = pdd::load(file)?;
    let deals: Vec<FullDeal> = parsed.iter().map(|&(deal, _)| deal).collect();
    let solved = Solver::lock(None).solve_deals(&deals, NonEmptyStrainFlags::ALL);

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

fn convert(inputs: &[String], out: &str) -> std::io::Result<()> {
    let binary = is_pdd(out);
    let mut w = BufWriter::new(std::fs::File::create(out)?);
    if binary {
        w.write_all(&pdd::MAGIC)?;
    }
    let mut total = 0usize;
    for input in inputs {
        for (deal, table) in pdd::load(input)? {
            write_deal(&mut w, binary, &deal, &table)?;
            total += 1;
        }
    }
    w.flush()?;
    eprintln!("gib convert: wrote {total} deals to {out}");
    Ok(())
}
