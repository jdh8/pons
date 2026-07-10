//! Localize *where* Cachalot loses to Modern — decompose the paired delta by
//! the auction feature, not one number over the blend.
//!
//! Cachalot's ≈−0.0073 IMPs/bd vs Modern (memory `project_school-tournament-
//! responses`) is a net of two opposed mechanisms: a support-double-equivalent
//! it gets uncontested (opener's transfer completion shows exactly 3), against
//! spade concealment + wrap economics on the contested boards. A single number
//! hides which dominates. This pairs the Cachalot (`on`) and Modern (`off`)
//! `table_a` (our-pair-NS) contracts across every shard, and for each divergent
//! board reports the plain-DD + perfect-defense delta split four ways:
//!
//!   1. the rotated call responder made at the divergence (the shape),
//!   2. whether the opponents competed *after* the rotation (the wrap),
//!   3. our final strain (spade concealment shows here),
//!   4. who declares our final contract, and whether it went doubled.
//!
//! Each axis partitions the divergent set, so its buckets' IMPs/bd sum to the
//! total — reading a bucket's sign says which population carries the loss.
//!
//! ```text
//! cargo run --release --features serde --example ab-dump-cachalot -- \
//!     ab-results/school-negx/school-negx-cachalot-both \
//!     ab-results/school-negx/school-negx-modern-both
//! ```

use clap::Parser;
use contract_bridge::auction::Call;
use contract_bridge::{AbsoluteVulnerability, Penalty, Seat, Strain};
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Board, Dump, Reached, mean_with_ci, score_boards, seat_to_act};

#[derive(Parser)]
struct Args {
    /// Directory of Cachalot-arm shard-*.json (the ON school)
    on_dir: String,
    /// Directory of Modern-arm shard-*.json, same seeds/deals (the baseline)
    off_dir: String,
    /// Re-price at this vulnerability instead of the dump's
    #[arg(short, long)]
    vulnerability: Option<AbsoluteVulnerability>,
}

/// The rotated call responder made where the two schools first parted.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// `X` — the artificial 4-card-major shower.
    Double,
    /// `1♥` over their `(1♦)` — the transfer that *conceals* spades.
    SpadeTransfer,
    /// `1♠` — the residual takeout, no biddable major.
    Takeout,
    /// A 2-level free bid, or a divergence deeper than responder's first call.
    Other,
}

/// One divergent board's coordinates on the four axes.
struct Split {
    shape: Shape,
    /// The opponents bid again after responder's rotation (the wrap).
    wrapped: bool,
    /// Our final contract's strain, `None` on a pass-out.
    strain: Option<Strain>,
    /// Our pair declares the final contract (right-side / play value).
    we_declare: bool,
    /// The final contract went (re)doubled — the wrap's doubled-minus tell.
    doubled: bool,
}

const fn is_ns(seat: Seat) -> bool {
    matches!(seat, Seat::North | Seat::South)
}

/// A per-bucket predicate over a divergent board's split coordinates.
type Pred<'a> = &'a dyn Fn(&Split) -> bool;

fn load_dir(dir: &str) -> (AbsoluteVulnerability, Vec<Board>) {
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .expect("read arm dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.starts_with("shard-") && s.ends_with(".json"))
        })
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no shard-*.json in {dir}");
    let mut vul = None;
    let mut boards = Vec::new();
    for path in paths {
        let dump: Dump = serde_json::from_reader(std::io::BufReader::new(
            std::fs::File::open(&path).expect("open shard"),
        ))
        .expect("parse shard");
        vul = Some(dump.vulnerability);
        boards.extend(dump.boards);
    }
    (vul.expect("at least one shard"), boards)
}

/// Classify a divergent board; `None` when the two `table_a` auctions match
/// (that board can never swing, so it is not in the fired set anyway).
fn classify(on: &Board, off: &Board) -> Option<Split> {
    let a = &on.table_a;
    let b = &off.table_a;
    let i = (0..a.len().min(b.len())).find(|&i| a[i] != b[i])?;

    let shape = match a[i] {
        Call::Double => Shape::Double,
        // 1♥ at a Cachalot divergence is only ever the spade transfer over (1♦):
        // Cachalot rotates only over minor openings, and a matching 1♥ would not
        // be a divergence — so ON == 1♥ here means responder holds spades.
        Call::Bid(bid) if bid.level.get() == 1 && bid.strain == Strain::Hearts => {
            Shape::SpadeTransfer
        }
        Call::Bid(bid) if bid.level.get() == 1 && bid.strain == Strain::Spades => Shape::Takeout,
        _ => Shape::Other,
    };

    // Wrapped: any opponent (EW) makes a non-pass call after the rotation.
    let wrapped =
        (i + 1..a.len()).any(|j| !is_ns(seat_to_act(on.dealer, j)) && !matches!(a[j], Call::Pass));

    let reached = final_contract(&on.table_a, on.dealer);
    let (strain, we_declare, doubled) = match reached {
        Some((contract, seat)) => (
            Some(contract.bid.strain),
            is_ns(seat),
            !matches!(contract.penalty, Penalty::Undoubled),
        ),
        None => (None, false, false),
    };

    Some(Split {
        shape,
        wrapped,
        strain,
        we_declare,
        doubled,
    })
}

fn main() {
    let args = Args::parse();
    let (on_vul, on) = load_dir(&args.on_dir);
    let (_, off) = load_dir(&args.off_dir);
    assert_eq!(
        on.len(),
        off.len(),
        "arms must be aligned (same board count)"
    );
    let vul = args.vulnerability.unwrap_or(on_vul);

    let mut deals = Vec::with_capacity(on.len());
    let contracts: Vec<(Reached, Reached)> = on
        .iter()
        .zip(&off)
        .map(|(a, b)| {
            assert_eq!(a.deal, b.deal, "arms not seed-aligned");
            deals.push(a.deal);
            (
                final_contract(&a.table_a, a.dealer),
                final_contract(&b.table_a, b.dealer),
            )
        })
        .collect();

    let split: Vec<Option<Split>> = on.iter().zip(&off).map(|(a, b)| classify(a, b)).collect();

    // One plain solve of the divergent boards; re-price the same tables under PD.
    let scored = score_boards(&contracts, &deals, vul, ns_score_contract);
    let plain = scored.board_imps.clone();
    let mut pd = vec![0i64; contracts.len()];
    let mut divergent = vec![false; contracts.len()];
    for (k, &idx) in scored.divergent.iter().enumerate() {
        divergent[idx] = true;
        let table = &scored.tables[k];
        let (con, coff) = contracts[idx];
        pd[idx] = imps(ns_score_pd(con, table, vul) - ns_score_pd(coff, table, vul));
    }

    println!(
        "== Cachalot vs Modern: {} boards, vul {vul} — {} contract-divergent ({:.2}%) ==",
        on.len(),
        scored.divergent.len(),
        100.0 * scored.divergent.len() as f64 / on.len().max(1) as f64,
    );

    let report_axis = |title: &str, imps: &[i64], rows: &[(&str, Pred)]| {
        println!(
            "\n  {title:<22} {:>6}  {:>10}  {:>12}  {:>11}",
            "fired", "IMPs/bd", "±95% CI", "IMPs/fired"
        );
        for (label, pred) in rows {
            let masked: Vec<i64> = imps
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    if split[i].as_ref().is_some_and(pred) {
                        v
                    } else {
                        0
                    }
                })
                .collect();
            let fired = (0..imps.len())
                .filter(|&i| divergent[i] && split[i].as_ref().is_some_and(pred))
                .count();
            let total: i64 = masked.iter().sum();
            let (mean, ci) = mean_with_ci(&masked);
            let per_fired = if fired == 0 {
                0.0
            } else {
                total as f64 / fired as f64
            };
            println!(
                "  {label:<22} {fired:>6}  {mean:>+10.4}  {:>12}  {per_fired:>+11.3}",
                format!("±{ci:.4}")
            );
        }
    };

    let is_minor = |s: &Strain| matches!(s, Strain::Clubs | Strain::Diamonds);
    for (bracket, imps) in [("PLAIN DD", &plain), ("PERFECT DEFENSE", &pd)] {
        println!("\n--- {bracket} ---");
        report_axis(
            "shape (resp. call)",
            imps,
            &[
                ("X (major-shower)", &|s| s.shape == Shape::Double),
                ("1H = spade transfer", &|s| s.shape == Shape::SpadeTransfer),
                ("1S residual takeout", &|s| s.shape == Shape::Takeout),
                ("2-lvl / deeper", &|s| s.shape == Shape::Other),
            ],
        );
        report_axis(
            "wrap (opps competed)",
            imps,
            &[
                ("clear (passed out)", &|s| !s.wrapped),
                ("wrapped (opps bid)", &|s| s.wrapped),
            ],
        );
        report_axis(
            "our final strain",
            imps,
            &[
                ("spades", &|s| s.strain == Some(Strain::Spades)),
                ("hearts", &|s| s.strain == Some(Strain::Hearts)),
                ("notrump", &|s| s.strain == Some(Strain::Notrump)),
                ("a minor", &|s| s.strain.as_ref().is_some_and(is_minor)),
                ("passed out", &|s| s.strain.is_none()),
            ],
        );
        report_axis(
            "declarer / penalty",
            imps,
            &[
                ("we declare", &|s| s.we_declare && !s.doubled),
                ("we decl., doubled", &|s| s.we_declare && s.doubled),
                ("they declare", &|s| {
                    !s.we_declare && s.strain.is_some() && !s.doubled
                }),
                ("they decl., doubled", &|s| {
                    !s.we_declare && s.strain.is_some() && s.doubled
                }),
            ],
        );
        report_axis(
            "X-double cross-tab",
            imps,
            &[
                ("X · clear (authored)", &|s| {
                    s.shape == Shape::Double && !s.wrapped
                }),
                ("X · wrapped (floored)", &|s| {
                    s.shape == Shape::Double && s.wrapped
                }),
                ("X · we play spades", &|s| {
                    s.shape == Shape::Double && s.strain == Some(Strain::Spades)
                }),
                ("X · we play hearts", &|s| {
                    s.shape == Shape::Double && s.strain == Some(Strain::Hearts)
                }),
            ],
        );
        report_axis("all fired", imps, &[("all divergent", &|_| true)]);
    }
}
