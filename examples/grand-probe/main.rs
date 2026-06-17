//! Grand-slam probe — why the M3.1 search target bid 7NT so often.
//!
//! Diagnostic-only sibling of [`search-dump`](../search-dump/main.rs).  It
//! replays the same self-play with the live double-dummy search bidder
//! ([`american_search`][pons::american_search]) and, at every *off-book*
//! node where the search's arg-max is **7NT**, characterizes that decision.
//!
//! It was written to test the hypothesis that the 7NT flood was *double-dummy
//! slam optimism* (the literature's warning that DD over-values grands).  It
//! **falsified** that: the 7NT make-rate at these nodes is ~0% — they are not
//! cold grands but **phantom sacrifices** in runaway competitive auctions, where
//! the rollout under-doubles so a failing save prices too cheaply.  The fix is
//! perfect-defense doubling in the EV scorer
//! ([`scoring::ns_score_doubling_failures`][pons::scoring::ns_score_doubling_failures]),
//! now the default; this probe is kept as the regression check that the grand
//! flood stays gone.
//!
//! Per node it reports: the **7NT DD make-rate**, the fixed-contract points, and
//! a **points-vs-IMP** recompute of 7NT-vs-6NT (the raw-point objective the EV
//! averages vs the [`imps`] the harness grades on).
//!
//! ```text
//! cargo run --release --features search --example grand-probe -- --seed 1 --hits 12
//! cargo run --release --features search --example grand-probe -- --census --boards 100
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Bid, Contract, Hand, Level, Penalty, Seat, Strain};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::bidding::Family;
use pons::bidding::array::Logits;
use pons::bidding::context::{Context, relative};
use pons::bidding::ev::ev_all;
use pons::bidding::inference::Inferences;
use pons::bidding::sampler::sample_layouts;
use pons::bidding::search_floor::SearchFloor;
use pons::scoring::{imps, ns_score};
use pons::{american_neural, american_search_with};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

#[derive(Parser)]
#[command(about = "Characterize the search's 7NT decisions: DD make-rate and points-vs-IMP flip")]
struct Args {
    /// Max boards to scan before giving up (stops early once `--hits` reached)
    #[arg(long, default_value_t = 200)]
    boards: usize,
    /// RNG seed (1 reproduces the M3.1 dataset's board stream)
    #[arg(long, default_value_t = 1)]
    seed: u64,
    /// How many 7NT decisions to probe, then stop
    #[arg(long, default_value_t = 12)]
    hits: usize,
    /// Navigation search depth (kept cheaper than the dataset's 128 — we only
    /// need to *reach* the same kind of confident-7NT node, not reproduce it
    /// bit-for-bit; the probe below uses its own high-resolution sample)
    #[arg(long, default_value_t = 64)]
    nav_layouts: usize,
    /// Navigation shortlist width
    #[arg(long, default_value_t = 6)]
    nav_shortlist: usize,
    /// Layouts sampled and solved per probe (the make-rate / IMP resolution)
    #[arg(long, default_value_t = 256)]
    probe_n: usize,
    /// Census mode: bid out `--boards` boards and just tally the advancing-call
    /// level histogram (no per-hit DD probing).  Fast check that slam+ / 7NT
    /// advancing calls stay rare under perfect-defense doubling.
    #[arg(long)]
    census: bool,
}

const VULS: [AbsoluteVulnerability; 4] = [
    AbsoluteVulnerability::NONE,
    AbsoluteVulnerability::NS,
    AbsoluteVulnerability::EW,
    AbsoluteVulnerability::ALL,
];

/// `7NT` and `6NT` as fixed contracts, declared by the actor.
fn nt(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid {
        level: Level::new(level),
        strain,
    })
}

/// One probed 7NT decision's numbers.
struct Probe {
    make_rate: f64,
    mean_tricks: f64,
    pts7: f64,
    pts6: f64,
    imp_7v6: f64,
    ev7_cont: f32,
    ev_pass_cont: f32,
}

fn main() {
    let args = Args::parse();
    let search = american_search_with(SearchFloor {
        layouts: args.nav_layouts,
        shortlist: args.nav_shortlist,
        temperature: 100.0,
    })
    .against(Family::NATURAL);
    let policy = american_neural().against(Family::NATURAL);
    let mut rng = StdRng::seed_from_u64(args.seed);
    let seven_nt = nt(7, Strain::Notrump);

    let mut probes: Vec<Probe> = Vec::new();
    let mut node_id = 0u64;
    // Census tallies: advancing-call level histogram over off-book decisions.
    let mut offbook_decisions = 0u64;
    let mut level_hist = [0u64; 8]; // index 0 = Pass/Dbl/Rdbl, 1..=7 = bid level
    let mut seven_nt_count = 0u64;

    'boards: for board in 0..args.boards {
        let deal = full_deal(&mut rng);
        let dealer = rng.random_range(0..4usize);
        let vul = VULS[rng.random_range(0..4usize)];

        let mut auction = Auction::new();
        while !auction.has_ended() {
            let seat = Seat::ALL[(dealer + auction.len()) % 4];
            let hand = deal[seat];
            let rel = relative(vul, seat);

            let Some((logits, provenance)) = search.classify_with_provenance(hand, rel, &auction)
            else {
                auction.push(Call::Pass);
                continue;
            };
            let off_book = provenance.depth == 0 && provenance.fallback.is_some();
            let logits = masked(logits, &auction);
            let chosen = argmax_legal(&logits);

            if off_book {
                offbook_decisions += 1;
                level_hist[bid_level(chosen)] += 1;
                if chosen == seven_nt {
                    seven_nt_count += 1;
                }
            }

            if args.census {
                auction.push(chosen);
                continue;
            }

            if off_book && chosen == seven_nt {
                node_id += 1;
                if let Some(probe) = probe_node(
                    hand,
                    seat,
                    vul,
                    rel,
                    &auction,
                    &policy,
                    args.probe_n,
                    node_id,
                ) {
                    let peak = logits
                        .softmax()
                        .map_or(0.0, |s| s.into_values().fold(0.0_f32, f32::max));
                    print_hit(
                        probes.len() + 1,
                        board,
                        vul,
                        dealer,
                        seat,
                        &auction,
                        hand,
                        peak,
                        &probe,
                    );
                    probes.push(probe);
                    if probes.len() >= args.hits {
                        break 'boards;
                    }
                }
            }
            auction.push(chosen);
        }
    }

    print_census(offbook_decisions, &level_hist, seven_nt_count);
    if !args.census {
        print_summary(&probes);
    }
}

/// Histogram bucket of an advancing call: `0` for pass/double/redouble, else the
/// bid level `1..=7`.
fn bid_level(call: Call) -> usize {
    match call {
        Call::Bid(bid) => usize::from(bid.level.get()),
        _ => 0,
    }
}

/// Print the advancing-call level histogram over off-book decisions.
fn print_census(offbook: u64, hist: &[u64; 8], seven_nt: u64) {
    let pct = |n: u64| {
        if offbook == 0 {
            0.0
        } else {
            100.0 * n as f64 / offbook as f64
        }
    };
    println!("\n=== off-book advancing-call census ({offbook} decisions) ===");
    println!("  pass/dbl/rdbl: {:>6} ({:4.1}%)", hist[0], pct(hist[0]));
    for (level, &count) in hist.iter().enumerate().take(8).skip(1) {
        println!("  level {level}:      {count:>6} ({:4.1}%)", pct(count));
    }
    let slam_plus = hist[6] + hist[7];
    println!(
        "  --> level>=6 (slam+): {} ({:.1}%)   7NT: {} ({:.1}%)",
        slam_plus,
        pct(slam_plus),
        seven_nt,
        pct(seven_nt)
    );
}

/// Solve auction-consistent layouts and score the 7NT decision two ways.
#[allow(clippy::too_many_arguments)]
fn probe_node(
    hand: Hand,
    seat: Seat,
    vul: AbsoluteVulnerability,
    rel: contract_bridge::auction::RelativeVulnerability,
    auction: &Auction,
    policy: &pons::bidding::Stance,
    n: usize,
    node_id: u64,
) -> Option<Probe> {
    let context = Context::new(rel, auction);
    let inferences = Inferences::read(&context);
    let mut rng = StdRng::seed_from_u64(0x6_7a5 ^ node_id);
    let deals = sample_layouts(hand, seat, &inferences, &mut rng, n);
    if deals.is_empty() {
        return None;
    }
    let tables = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);

    // Actor-side sign: ns_score is +ve for NS, so flip for an EW actor.
    let sign: i64 = if matches!(seat, Seat::North | Seat::South) {
        1
    } else {
        -1
    };
    let c7 = Some((
        Contract {
            bid: Bid {
                level: Level::new(7),
                strain: Strain::Notrump,
            },
            penalty: Penalty::Undoubled,
        },
        seat,
    ));
    let c6 = Some((
        Contract {
            bid: Bid {
                level: Level::new(6),
                strain: Strain::Notrump,
            },
            penalty: Penalty::Undoubled,
        },
        seat,
    ));

    let (mut makes, mut tricks_sum, mut p7, mut p6, mut imp_sum) = (0u64, 0u64, 0i64, 0i64, 0i64);
    for table in &tables {
        let tricks = u8::from(table[Strain::Notrump].get(seat));
        if tricks >= 13 {
            makes += 1;
        }
        tricks_sum += u64::from(tricks);
        let s7 = sign * ns_score(c7, table, vul);
        let s6 = sign * ns_score(c6, table, vul);
        p7 += s7;
        p6 += s6;
        imp_sum += imps(s7 - s6); // per-layout IMPs of bidding 7NT over 6NT
    }
    let len = deals.len() as f64;

    // The search's own decision signal: continuation-aware points EV of bidding
    // 7NT vs passing (neural self-play finishes both auctions).  ev_all signs to
    // the actor already.
    let mut rng2 = StdRng::seed_from_u64(0x5eed ^ node_id);
    let evs = ev_all(
        hand,
        seat,
        vul,
        &context,
        &[nt(7, Strain::Notrump), Call::Pass],
        policy,
        &mut rng2,
        n,
    );

    Some(Probe {
        make_rate: makes as f64 / len,
        mean_tricks: tricks_sum as f64 / len,
        pts7: p7 as f64 / len,
        pts6: p6 as f64 / len,
        imp_7v6: imp_sum as f64 / len,
        ev7_cont: evs[0],
        ev_pass_cont: evs[1],
    })
}

#[allow(clippy::too_many_arguments)]
fn print_hit(
    k: usize,
    board: usize,
    vul: AbsoluteVulnerability,
    dealer: usize,
    seat: Seat,
    auction: &Auction,
    hand: Hand,
    peak: f32,
    p: &Probe,
) {
    let calls: Vec<String> = auction.iter().map(|c| format!("{c}")).collect();
    println!(
        "\n#{k}  board={board}  vul={vul:?}  dealer={:?}  actor={seat:?}  peak={peak:.2}",
        Seat::ALL[dealer]
    );
    println!("   auction: {}", calls.join(" "));
    println!("   hand:    {hand}");
    println!(
        "   7NT DD make-rate: {:.0}%   (mean NT tricks {:.1})",
        100.0 * p.make_rate,
        p.mean_tricks
    );
    println!(
        "   fixed-contract points (actor side):  7NT {:+.0}   6NT {:+.0}   → 7NT−6NT = {:+.0} pts",
        p.pts7,
        p.pts6,
        p.pts7 - p.pts6
    );
    let verdict = if p.imp_7v6 <= 0.0 {
        "IMPs prefer the SMALL SLAM"
    } else {
        "IMPs still bid the grand"
    };
    println!(
        "   IMP(7NT vs 6NT), per-layout avg:     {:+.2} IMPs   ← {verdict}",
        p.imp_7v6
    );
    println!(
        "   search's own points-EV:  bid 7NT {:+.0}   pass→neural {:+.0}   (margin {:+.0} pts)",
        p.ev7_cont,
        p.ev_pass_cont,
        p.ev7_cont - p.ev_pass_cont
    );
}

fn print_summary(probes: &[Probe]) {
    let n = probes.len().max(1) as f64;
    let mean = |f: &dyn Fn(&Probe) -> f64| probes.iter().map(f).sum::<f64>() / n;
    let imp_negative = probes.iter().filter(|p| p.imp_7v6 <= 0.0).count();
    println!(
        "\n========== SUMMARY over {} probed 7NT nodes ==========",
        probes.len()
    );
    println!(
        "  mean 7NT DD make-rate:        {:.0}%",
        100.0 * mean(&|p| p.make_rate)
    );
    println!(
        "  mean NT tricks:               {:.1}",
        mean(&|p| p.mean_tricks)
    );
    println!(
        "  mean points(7NT − 6NT):       {:+.0}",
        mean(&|p| p.pts7 - p.pts6)
    );
    println!(
        "  mean IMP(7NT vs 6NT):         {:+.2}",
        mean(&|p| p.imp_7v6)
    );
    println!(
        "  nodes where IMPs prefer 6NT:  {}/{}  (the objective-mismatch share)",
        imp_negative,
        probes.len()
    );
}

/// Mask illegal calls to `-∞`, leaving a distribution over the legal calls.
fn masked(mut logits: Logits, auction: &Auction) -> Logits {
    for (call, slot) in logits.iter_mut() {
        if auction.can_push(call).is_err() {
            *slot = f32::NEG_INFINITY;
        }
    }
    logits
}

/// The highest-logit legal call, defaulting to a pass.
fn argmax_legal(logits: &Logits) -> Call {
    logits
        .iter()
        .filter(|(_, l)| l.is_finite())
        .max_by(|a, b| a.1.partial_cmp(b.1).expect("logits are never NaN"))
        .map_or(Call::Pass, |(call, _)| call)
}
