//! BEN-projection Phase-4 recon: how much of the slam slice can a projected
//! `keycards` axis actually *reach*?
//!
//! Phase 3 proved the ceiling: partner's true keycards cut the slam-slice
//! evaluator MAE 2.66 → 1.41. But a projection recovers keycards only where the
//! auction has *shown* them — after an RKCB response — and yields ⊤ (no info)
//! everywhere else. Realizable gain ≈ reach-fraction × ceiling, so the go/no-go
//! for building the axis is: of the evaluator rows whose DD truth is a slam,
//! what fraction sit at a decision point where partner has already answered
//! RKCB?
//!
//! Walks the *same* auctions [`dump-evaluator`] does — full self-play over
//! pre-solved deals, a row at every decision, a slam cell per DD target ≥ 12
//! tricks — and latches, per seat, whether that seat has revealed keycards:
//!
//! - **book** — the response resolved to an authored `keycards(...)` rule
//!   (`explain_call` description says "keycards"). This is exactly what Phase-4
//!   step 2's projecting `Constraint` will recover; the *realizable* reach.
//! - **struct** — the call is `5♣/♦/♥/♠` and partner's previous call was `4NT`,
//!   regardless of who served it. A superset that also counts floor-served RKCB,
//!   which the step-2 projection does *not* capture; the floor-inclusive ceiling.
//!
//! ```sh
//! cargo run --release --example probe-keycard-reach -- \
//!     --deals /nfs2/jdh8/22.pdd --count 200000 --seed 1
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Bid, FullDeal, Seat, Strain};
use ddss::TrickCountTable;
use pons::bidding::context::relative;
use pons::bidding::{Family, Stance, System};
use pons::{american, dutch, gib};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use rayon::prelude::*;

/// A slam is 12+ of 13 tricks; compare on the `/13`-normalised label with a
/// mid-gap threshold so f32 rounding at the 11↔12 boundary can't flip a cell.
const SLAM_TRICKS: f32 = 11.5 / 13.0;

/// The four absolute vulnerabilities, sampled uniformly per board (matches
/// `dump-evaluator`'s stream shape, though not its exact per-deal assignment —
/// reach is an aggregate over random dealer/vul, so per-deal seeding is fine).
const VULS: [AbsoluteVulnerability; 4] = [
    AbsoluteVulnerability::NONE,
    AbsoluteVulnerability::NS,
    AbsoluteVulnerability::EW,
    AbsoluteVulnerability::ALL,
];

// Accumulator column indices. Rows/slam-cells, each split none/book/struct;
// "ours" restricts slam cells to our-side declarers (me, partner).
const N: usize = 12;
const ROWS: usize = 0;
const ROWS_BOOK: usize = 1;
const ROWS_STRUCT: usize = 2;
const SLAM: usize = 3;
const SLAM_BOOK: usize = 4;
const SLAM_STRUCT: usize = 5;
const OURS: usize = 6;
const OURS_BOOK: usize = 7;
const OURS_STRUCT: usize = 8;
const AUC: usize = 9;
const AUC_BOOK: usize = 10;
const AUC_STRUCT: usize = 11;

#[derive(Parser)]
#[command(about = "Phase-4 recon: keycard-projection reach over the slam slice")]
struct Args {
    /// Pre-solved deal database: binary `.pdd`
    #[arg(long)]
    deals: String,
    /// Skip this many deals before reading
    #[arg(long, default_value_t = 0)]
    skip: u64,
    /// Number of deals to bid out
    #[arg(long, default_value_t = 200_000)]
    count: usize,
    /// Seed for the per-deal dealer/vulnerability stream
    #[arg(long, default_value_t = 0)]
    seed: u64,
}

/// The highest-logit finite (hence legal, after masking) call, defaulting to a
/// pass so the auction always terminates. Verbatim from `dump-evaluator`.
fn argmax_legal(logits: &pons::bidding::array::Logits) -> Call {
    logits
        .iter()
        .filter(|(_, l)| l.is_finite())
        .max_by(|a, b| a.1.partial_cmp(b.1).expect("logits are never NaN"))
        .map_or(Call::Pass, |(call, _)| call)
}

/// Bid one auction under `stance` and fold its rows into `acc`. `rkcb` is
/// prebuilt so the hot loop only compares `Call`s: `[0..4]` are the four
/// `5♣/♦/♥/♠` responses, `[4]` is the `4NT` ask.
fn walk(
    acc: &mut [u64; N],
    stance: &Stance,
    dealer: usize,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
    table: &TrickCountTable,
    rkcb: &[Call; 5],
) {
    let mut auction = Auction::new();
    // Per-seat latch: has this seat revealed its keycards to partner?
    let mut revealed_book = [false; 4];
    let mut revealed_struct = [false; 4];

    while !auction.has_ended() {
        let seat = Seat::ALL[(dealer + auction.len()) % 4];
        let hand = deal[seat];
        let rel = relative(vul, seat);

        let Some(mut logits) = stance.classify(hand, rel, &auction) else {
            // Forced pass: dump-evaluator emits no row here, so neither do we.
            auction.push(Call::Pass);
            continue;
        };
        for (call, slot) in logits.iter_mut() {
            if auction.can_push(call).is_err() {
                *slot = f32::NEG_INFINITY;
            }
        }

        // The row: reach is whether *partner* has revealed keycards by now.
        let partner = seat.partner() as usize;
        let (rb, rs) = (revealed_book[partner], revealed_struct[partner]);
        acc[ROWS] += 1;
        acc[ROWS_BOOK] += u64::from(rb);
        acc[ROWS_STRUCT] += u64::from(rs);
        for (idx, &value) in gib::relativized_tricks(table, seat).iter().enumerate() {
            if value >= SLAM_TRICKS {
                acc[SLAM] += 1;
                acc[SLAM_BOOK] += u64::from(rb);
                acc[SLAM_STRUCT] += u64::from(rs);
                // Declarer is the target's low 2 bits: 0 = me, 2 = partner.
                if idx % 4 == 0 || idx % 4 == 2 {
                    acc[OURS] += 1;
                    acc[OURS_BOOK] += u64::from(rb);
                    acc[OURS_STRUCT] += u64::from(rs);
                }
            }
        }

        let call = argmax_legal(&logits);
        // A structural RKCB response: `5♣/♦/♥/♠` when partner's previous call
        // (always 2 back — partners sit 2 seats apart) was `4NT`.
        if rkcb[..4].contains(&call) && auction.iter().rev().nth(1) == Some(&rkcb[4]) {
            revealed_struct[seat as usize] = true;
            // Book-confirmed iff an authored `keycards(...)` rule served it.
            if stance
                .explain_call(hand, rel, &auction, call)
                .and_then(|(_, r)| r)
                .is_some_and(|r| r.description.contains("keycards"))
            {
                revealed_book[seat as usize] = true;
            }
        }
        auction.push(call);
    }
    acc[AUC] += 1;
    acc[AUC_BOOK] += u64::from(revealed_book.iter().any(|&b| b));
    acc[AUC_STRUCT] += u64::from(revealed_struct.iter().any(|&b| b));
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let deals = pons::pdd::load_slice(&args.deals, args.skip, args.count)?;
    eprintln!("keycard-reach: {} deals × 2 systems", deals.len());

    let systems = [
        american().against(Family::NATURAL),
        dutch().against(Family::NATURAL),
    ];
    let rkcb = [
        Call::Bid(Bid::new(5, Strain::Clubs)),
        Call::Bid(Bid::new(5, Strain::Diamonds)),
        Call::Bid(Bid::new(5, Strain::Hearts)),
        Call::Bid(Bid::new(5, Strain::Spades)),
        Call::Bid(Bid::new(4, Strain::Notrump)),
    ];

    let totals = deals
        .par_iter()
        .enumerate()
        .map(|(index, (deal, table))| {
            // Per-deal seeding: order-independent, so rayon is free.
            let mut rng = StdRng::seed_from_u64(args.seed.wrapping_add(index as u64));
            let dealer = rng.random_range(0..4usize);
            let vul = VULS[rng.random_range(0..4usize)];
            let mut acc = [0u64; N];
            for stance in &systems {
                walk(&mut acc, stance, dealer, vul, deal, table, &rkcb);
            }
            acc
        })
        .reduce(
            || [0u64; N],
            |mut a, b| {
                for (x, y) in a.iter_mut().zip(b) {
                    *x += y;
                }
                a
            },
        );

    #[allow(clippy::cast_precision_loss)]
    let pct = |n: usize, d: usize| {
        if d == 0 {
            0.0
        } else {
            100.0 * n as f64 / d as f64
        }
    };
    let (rows, slam, ours, auc) = (
        totals[ROWS] as usize,
        totals[SLAM] as usize,
        totals[OURS] as usize,
        totals[AUC] as usize,
    );
    println!(
        "=== keycard reach: {} deals, seed {} ===",
        deals.len(),
        args.seed
    );
    println!("auctions {auc}   rows {rows}   slam-cells {slam}   our-side slam-cells {ours}\n");
    println!("                          book-RKCB      struct-RKCB (floor-incl)");
    println!(
        "auctions w/ reveal   {:9} {:6.3}%   {:9} {:6.3}%",
        totals[AUC_BOOK],
        pct(totals[AUC_BOOK] as usize, auc),
        totals[AUC_STRUCT],
        pct(totals[AUC_STRUCT] as usize, auc),
    );
    println!(
        "rows readable        {:9} {:6.3}%   {:9} {:6.3}%",
        totals[ROWS_BOOK],
        pct(totals[ROWS_BOOK] as usize, rows),
        totals[ROWS_STRUCT],
        pct(totals[ROWS_STRUCT] as usize, rows),
    );
    println!(
        "SLAM cells readable  {:9} {:6.3}%   {:9} {:6.3}%",
        totals[SLAM_BOOK],
        pct(totals[SLAM_BOOK] as usize, slam),
        totals[SLAM_STRUCT],
        pct(totals[SLAM_STRUCT] as usize, slam),
    );
    println!(
        "  our-side only      {:9} {:6.3}%   {:9} {:6.3}%",
        totals[OURS_BOOK],
        pct(totals[OURS_BOOK] as usize, ours),
        totals[OURS_STRUCT],
        pct(totals[OURS_STRUCT] as usize, ours),
    );
    println!(
        "\nrealizable slam-slice gain (book reach × 1.257 ceiling) ≈ {:.3} tricks",
        pct(totals[SLAM_BOOK] as usize, slam) / 100.0 * 1.257,
    );
    Ok(())
}
