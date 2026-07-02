//! Does a fit-adjusted evaluator beat raw HCP at the South-African-Texas
//! game-vs-slam-try boundary?  (AI-bidder; see `docs/ai-bidder/`.)
//!
//! After `1NT` opposite a 15-17 balanced opener, responder with a 6+ major and
//! ≤4 in the other major chooses between two routes (`notrump.rs`):
//!
//! - **game** — `4♣/4♦` Texas, opener completes to `4M` (to-play, book 9-14 HCP).
//! - **slam-try** — direct `4♥/4♠` (book 15-18 HCP).  Opener passes a minimum
//!   (15-16 → `4M`) or launches RKCB with a maximum (17 → small slam, modelled as
//!   `6M`).
//!
//! Unlike the no-fit balanced invite boundary (`probe-nt-invite-eval`, where raw
//! HCP won because game-or-not is a "raw-25 question" opposite a flat opener),
//! this node has an **assured 6-2+ trump fit**, so distributional points and
//! trump length should matter.  This screen rank-calibrates each evaluator to the
//! HCP control's exact slam-try frequency and measures the IMP delta — a pure
//! ranking-quality test.  Evaluators:
//!
//! - `HCP`        — raw HCP (control; calibrated arm must score ~0).
//! - `points`     — `point_count` (HCP + shape upgrade), no trump term.
//! - `hcp+trump`  — raw HCP + 1 per trump beyond 6 (pure excess-trump buff).
//! - `fit_value`  — `point_count` + 1 per trump beyond 6 (the full proposal; the
//!   Texas mirror of `notrump.rs::fit_value`, which counts from 4 for the 4-4
//!   Stayman fit — here responder shows 6, opener ≥2).
//!
//! ```text
//! cargo run --release --example probe-texas-slam-eval -- 200000 0
//! ```
//! Args (positional, optional): `count` (default 200000), `seed` (default 0).
//! The decision only bites on max (17) openers, so it needs many deals — run via
//! `scripts/idle-run.sh`.
//!
//! ponytail: declarer = best of N/S per contract; opener acceptance held at the
//! book's 17 = RKCB rule; RKCB modelled as always reaching `6M` on a max (the
//! same contract model for every evaluator, so the *delta* is fair — absolute
//! IMPs are optimistic on slam frequency).

use contract_bridge::eval::{self};
use contract_bridge::{
    AbsoluteVulnerability, Bid, Contract, FullDeal, Hand, Penalty, Seat, Strain, Suit,
};
use ddss::{NonEmptyStrainFlags, Solver, TrickCountTable};
use pons::bidding::constraint::point_count;
use pons::scoring::{imps, ns_score_contract};
use rand::SeedableRng;
use rand::rngs::StdRng;

fn raw_hcp(hand: Hand) -> u8 {
    Suit::ASC.iter().map(|&s| eval::hcp::<u8>(hand[s])).sum()
}

/// Balanced: no void/singleton, ≤1 doubleton, ≤5 long.
fn is_balanced(h: Hand) -> bool {
    let lens = Suit::ASC.map(|s| h[s].len());
    lens.iter().all(|&l| l >= 2)
        && lens.iter().filter(|&&l| l == 2).count() <= 1
        && lens.iter().all(|&l| l <= 5)
}

/// The Texas anchor major: a 6+ major with ≤4 in the other major (the book's
/// `len(other, ..5)` guard makes this unique).  `None` if no such hand.
fn anchor(h: Hand) -> Option<Suit> {
    let (hh, ss) = (h[Suit::Hearts].len(), h[Suit::Spades].len());
    if hh >= 6 && ss <= 4 {
        Some(Suit::Hearts)
    } else if ss >= 6 && hh <= 4 {
        Some(Suit::Spades)
    } else {
        None
    }
}

// --- evaluators -------------------------------------------------------------

const EVAL_NAMES: [&str; 4] = ["HCP", "points", "hcp+trump", "fit_value"];

/// The four evaluator values for `hand` whose `assured` trumps in `major` are
/// already known (responder 6, opener 2): raw HCP, adjusted points, and each
/// buffed by one point per trump beyond `assured`.
fn evals(hand: Hand, major: Suit, assured: usize) -> [f64; 4] {
    let hcp = f64::from(raw_hcp(hand));
    let pts = f64::from(point_count(hand));
    let excess = hand[major].len().saturating_sub(assured) as f64;
    [hcp, pts, hcp + excess, pts + excess]
}

/// Best-of-N/S double-dummy NS score of one contract (undoubled).
fn score1(level: u8, strain: Strain, table: &TrickCountTable, vul: AbsoluteVulnerability) -> i64 {
    let c = Contract {
        bid: Bid::new(level, strain),
        penalty: Penalty::Undoubled,
    };
    [Seat::North, Seat::South]
        .into_iter()
        .map(|d| ns_score_contract(Some((c, d)), table, vul))
        .max()
        .expect("two declarers")
}

/// Rank-calibrate to the control: the top `n_try` hands → slam-try (1), rest game (0).
fn calibrate(vals: &[f64], n_try: usize) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..vals.len()).collect();
    idx.sort_by(|&a, &b| vals[a].partial_cmp(&vals[b]).expect("never NaN"));
    let mut action = vec![0usize; vals.len()];
    let cut = vals.len().saturating_sub(n_try);
    for (rank, &i) in idx.iter().enumerate() {
        if rank >= cut {
            action[i] = 1;
        }
    }
    action
}

fn mean_ci(values: &[i64]) -> (f64, f64) {
    let n = values.len();
    if n < 2 {
        return (0.0, 0.0);
    }
    let mean = values.iter().sum::<i64>() as f64 / n as f64;
    let var = values
        .iter()
        .map(|&v| {
            let d = v as f64 - mean;
            d * d
        })
        .sum::<f64>()
        / (n - 1) as f64;
    (mean, 1.96 * (var / n as f64).sqrt())
}

fn main() {
    let mut argv = std::env::args().skip(1);
    let count: usize = argv.next().and_then(|s| s.parse().ok()).unwrap_or(200_000);
    let seed: u64 = argv.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let attempt_cap = count.saturating_mul(4000).max(50_000_000);

    // Deal: opener (North) 15-17 balanced; responder (South) a Texas shape in the
    // 11..=18 boundary band (straddles the book's 14/15 game/slam-try cut).
    let mut deals: Vec<FullDeal> = Vec::new();
    let mut majors: Vec<Suit> = Vec::new();
    let mut rng = StdRng::seed_from_u64(seed);
    let mut attempts = 0usize;
    while deals.len() < count && attempts < attempt_cap {
        attempts += 1;
        let deal = contract_bridge::deck::full_deal(&mut rng);
        let opener = deal[Seat::North];
        if !is_balanced(opener) || !(15..=17).contains(&raw_hcp(opener)) {
            continue;
        }
        let resp = deal[Seat::South];
        if !(11..=18).contains(&raw_hcp(resp)) {
            continue;
        }
        let Some(m) = anchor(resp) else { continue };
        deals.push(deal);
        majors.push(m);
    }

    eprintln!(
        "dealt {} Texas boards ({attempts} attempts); solving…",
        deals.len()
    );
    let tables = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);

    let vuls = [
        ("none", AbsoluteVulnerability::NONE),
        ("both", AbsoluteVulnerability::ALL),
    ];

    // Per-deal: both sides' evaluator values, both HCPs, and the 4M/6M NS scores
    // per vul (action-free — each analysis maps actions to these two contracts).
    let n = deals.len();
    let mut resp_evals: Vec<[f64; 4]> = Vec::with_capacity(n);
    let mut opener_evals: Vec<[f64; 4]> = Vec::with_capacity(n);
    let mut resp_hcp: Vec<u8> = Vec::with_capacity(n);
    let mut opener_hcp: Vec<u8> = Vec::with_capacity(n);
    // scores[deal][vul] = [4M, 6M]
    let mut scores: Vec<[[i64; 2]; 2]> = Vec::with_capacity(n);
    for i in 0..n {
        let (resp, opener, table, m) = (
            deals[i][Seat::South],
            deals[i][Seat::North],
            &tables[i],
            majors[i],
        );
        let ms = Strain::from(m);
        let mut per_vul = [[0i64; 2]; 2];
        for (vi, &(_, vul)) in vuls.iter().enumerate() {
            per_vul[vi] = [score1(4, ms, table, vul), score1(6, ms, table, vul)];
        }
        scores.push(per_vul);
        resp_hcp.push(raw_hcp(resp));
        opener_hcp.push(raw_hcp(opener));
        resp_evals.push(evals(resp, m, 6)); // responder shows 6
        opener_evals.push(evals(opener, m, 2)); // opener guaranteed 2
    }

    // Report one calibrated comparison: for each evaluator, calibrate to the
    // control's accept-rate over `idx`, then the IMP delta of (accept→6M / reject
    // →4M) vs the control on divergent boards.  `control[d]==1` means the high
    // action (slam-try / accept).
    let report =
        |title: &str, idx: &[usize], evals: &[[f64; 4]], control: &dyn Fn(usize) -> usize| {
            let m = idx.len();
            let ctrl: Vec<usize> = idx.iter().map(|&d| control(d)).collect();
            let n_hi = ctrl.iter().filter(|&&a| a == 1).count();
            println!(
                "\n=== {title}: {m} deals  (high {n_hi} / low {}) ===",
                m - n_hi
            );
            for (vi, (vname, _)) in vuls.iter().enumerate() {
                println!("  vul {vname}:");
                println!(
                    "    {:<10} {:>9}  {:>14}  {:>8}",
                    "evaluator", "IMP/bd", "95% CI", "diverg"
                );
                for (e, ename) in EVAL_NAMES.iter().enumerate() {
                    let col: Vec<f64> = idx.iter().map(|&d| evals[d][e]).collect();
                    let action = calibrate(&col, n_hi);
                    let mut board_imps = vec![0i64; m];
                    let mut diverg = 0usize;
                    for (k, &d) in idx.iter().enumerate() {
                        if action[k] != ctrl[k] {
                            diverg += 1;
                            let s = &scores[d][vi]; // [4M, 6M] = [low, high]
                            board_imps[k] = imps(s[action[k]] - s[ctrl[k]]);
                        }
                    }
                    let (mean, ci) = mean_ci(&board_imps);
                    let flag = if (mean - ci > 0.0 || mean + ci < 0.0) && *ename != "HCP" {
                        " *"
                    } else {
                        ""
                    };
                    println!("    {ename:<10} {mean:>+9.4}  ±{ci:>12.4}  {diverg:>8}{flag}");
                }
            }
        };

    // A — RESPONDER: game (4M) vs slam-try, opener accept held at the book hcp≥17.
    // Slam-try reaches 6M only opposite a max opener; else it collapses to 4M, so
    // a board's "high" action maps to 6M when opener is max, 4M otherwise.  We
    // encode that by routing min-opener slam-tries to the 4M column via a per-
    // board contract pick inside a dedicated reporter.
    {
        let all: Vec<usize> = (0..n).collect();
        let n_hi = resp_hcp.iter().filter(|&&h| h >= 15).count();
        let n_max = opener_hcp.iter().filter(|&&h| h >= 17).count();
        println!(
            "\n=== A. RESPONDER game-vs-slam-try: {n} deals  (control slam-try {n_hi} / game {}; max openers {n_max}) ===",
            n - n_hi
        );
        for (vi, (vname, _)) in vuls.iter().enumerate() {
            println!("  vul {vname}:");
            println!(
                "    {:<10} {:>9}  {:>14}  {:>8}",
                "evaluator", "IMP/bd", "95% CI", "diverg"
            );
            for (e, ename) in EVAL_NAMES.iter().enumerate() {
                let col: Vec<f64> = all.iter().map(|&d| resp_evals[d][e]).collect();
                let action = calibrate(&col, n_hi);
                let ctrl: Vec<usize> = resp_hcp.iter().map(|&h| usize::from(h >= 15)).collect();
                let mut board_imps = vec![0i64; n];
                let mut diverg = 0usize;
                for d in 0..n {
                    if action[d] != ctrl[d] {
                        diverg += 1;
                        let s = &scores[d][vi];
                        let max = opener_hcp[d] >= 17;
                        // slam-try contract = 6M opposite a max, else 4M (= game).
                        let pick = |a: usize| if a == 1 && max { s[1] } else { s[0] };
                        board_imps[d] = imps(pick(action[d]) - pick(ctrl[d]));
                    }
                }
                let (mean, ci) = mean_ci(&board_imps);
                let flag = if (mean - ci > 0.0 || mean + ci < 0.0) && *ename != "HCP" {
                    " *"
                } else {
                    ""
                };
                println!("    {ename:<10} {mean:>+9.4}  ±{ci:>12.4}  {diverg:>8}{flag}");
            }
        }
    }

    // B — OPENER: given a book slam-try maker (responder HCP 15-18), accept (6M)
    // vs pass (4M).  Control accepts on raw hcp≥17; candidates add the fit term.
    let try_makers: Vec<usize> = (0..n)
        .filter(|&d| (15..=18).contains(&resp_hcp[d]))
        .collect();
    report(
        "B. OPENER accept-vs-pass (responder slam-try makers)",
        &try_makers,
        &opener_evals,
        &|d| usize::from(opener_hcp[d] >= 17),
    );

    println!("\n(* = 95% CI excludes 0; HCP row is the rank-calibration noise floor, expect ~0)");
}
