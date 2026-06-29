//! What are the best terms for opener to accept a 6-card-major game invite?
//! (AI-bidder; see `docs/ai-bidder/`.)
//!
//! After `1NT` opposite a 15-17 balanced opener, responder with a 6+ major and
//! ≤4 in the other major and *invitational* values (≈7-8 HCP, one-two short of
//! the book's 9-HCP Texas-to-game floor) takes the **2-level Jacoby transfer**
//! (`2♦/2♥`), opener completes (`2♥/2♠`), then responder jumps to `3M` — a
//! natural game invite in the now-established 6-2+ major fit.  Opener then
//! passes `3M` (partscore) or bids `4M` (game).  (Texas, the `4♣/4♦` route, is
//! the neighbouring 9+ HCP game blast — not this invite.)
//!
//! This screen answers two questions on double-dummy IMPs:
//!
//! - **R (responder cut):** at each responder-HCP bucket, which of pass-`2M` /
//!   invite / blast-`4M` scores best?  Sets the weak/invite and invite/game cuts.
//! - **O (opener accept rule):** holding the invite population fixed, sweep a
//!   threshold on each opener evaluator (accept iff `eval ≥ T`, else pass `3M`)
//!   and report the IMP delta vs always-declining.  The oracle (accept iff
//!   `4M > 3M`) is the ceiling.  Best `(evaluator, T)` = the accept rule to author.
//!
//! Evaluators (opener guaranteed 2 trumps, responder shows 6):
//! - `HCP`        — raw HCP.
//! - `points`     — `point_count` (HCP + shape), no trump term.
//! - `hcp+trump`  — raw HCP + 1 per trump beyond 2 (the 3rd-trump buff).
//! - `fit_value`  — `point_count` + 1 per trump beyond 2 (the full proposal — a
//!   16 with a 3rd trump rates as a 17 without).
//!
//! ```text
//! cargo run --release --example probe-jacoby-invite-eval -- 200000 0
//! ```
//! Args (positional, optional): `count` (default 200000), `seed` (default 0).
//! The accept decision bites only near the 15-17 boundary, so it wants many
//! deals — run via `scripts/idle-run.sh`.
//!
//! ponytail: declarer = best of N/S per contract (right-siding is DD-blind — we
//! score the *level*, 3M vs 4M, not who declares).  Invite band fixed to 7..=8
//! HCP for experiment O; experiment R prints the bucketed scores so the cut can
//! be re-read without a rebuild.

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

/// The anchor major: a 6+ major with ≤4 in the other major.  `None` otherwise.
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

const EVAL_NAMES: [&str; 4] = ["HCP", "points", "hcp+trump", "fit_value"];

/// The four evaluator values for `hand` with `assured` trumps known in `major`.
fn evals(hand: Hand, major: Suit, assured: usize) -> [f64; 4] {
    let hcp = f64::from(raw_hcp(hand));
    let pts = f64::from(point_count(hand));
    let excess = hand[major].len().saturating_sub(assured) as f64;
    [hcp, pts, hcp + excess, pts + excess]
}

/// Responder-side evaluators for the *floor* (blast-4M vs pass-2M) decision: raw
/// HCP, distribution `point_count`, a fit-adjusted `points + (trump len − 5)`
/// (responder shows 6, so every card past a 5-bagger is a length trick), and
/// Kaplan–Rubens CCCC (honor placement + shape, tuned for suit play).  The two
/// `+len` columns add the same trump-length term so the gauge (`point_count` vs
/// CCCC) is the only difference.
const RESP_EVAL_NAMES: [&str; 4] = ["HCP", "pts+len", "CCCC", "CCCC+len"];

fn resp_evals(hand: Hand, major: Suit) -> [f64; 4] {
    let hcp = f64::from(raw_hcp(hand));
    let pts = f64::from(point_count(hand));
    let cccc = eval::cccc(hand);
    let excess = hand[major].len().saturating_sub(5) as f64;
    [hcp, pts + excess, cccc, cccc + excess]
}

/// Rank-calibrate to a fixed count: the top `n_hi` values → action 1, rest 0.
/// Equal-action sets across evaluators make the IMP delta a pure ranking test.
fn calibrate(vals: &[f64], n_hi: usize) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..vals.len()).collect();
    idx.sort_by(|&a, &b| vals[a].partial_cmp(&vals[b]).expect("never NaN"));
    let mut action = vec![0usize; vals.len()];
    let cut = vals.len().saturating_sub(n_hi);
    for (rank, &i) in idx.iter().enumerate() {
        if rank >= cut {
            action[i] = 1;
        }
    }
    action
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

    // Opener (North) 15-17 balanced; responder (South) a 6-card-major anchor in a
    // 5..=10 band straddling the weak/invite and invite/game (Texas-9) cuts.
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
        if !(5..=10).contains(&raw_hcp(resp)) {
            continue;
        }
        let Some(m) = anchor(resp) else { continue };
        deals.push(deal);
        majors.push(m);
    }

    eprintln!(
        "dealt {} invite boards ({attempts} attempts); solving…",
        deals.len()
    );
    let tables = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);

    let vuls = [
        ("none", AbsoluteVulnerability::NONE),
        ("both", AbsoluteVulnerability::ALL),
    ];

    let n = deals.len();
    let mut opener_evals: Vec<[f64; 4]> = Vec::with_capacity(n);
    let mut resp_hcp: Vec<u8> = Vec::with_capacity(n);
    // scores[deal][vul] = [2M, 3M, 4M]
    let mut scores: Vec<[[i64; 3]; 2]> = Vec::with_capacity(n);
    for i in 0..n {
        let (opener, table, m) = (deals[i][Seat::North], &tables[i], majors[i]);
        let ms = Strain::from(m);
        let mut per_vul = [[0i64; 3]; 2];
        for (vi, &(_, vul)) in vuls.iter().enumerate() {
            per_vul[vi] = [
                score1(2, ms, table, vul),
                score1(3, ms, table, vul),
                score1(4, ms, table, vul),
            ];
        }
        scores.push(per_vul);
        resp_hcp.push(raw_hcp(deals[i][Seat::South]));
        opener_evals.push(evals(opener, m, 2)); // opener guaranteed 2 trumps
    }

    // --- R: responder cut.  Per HCP bucket, mean best-of-N/S score (vul none) of
    // pass-2M / invite (opener accepts per oracle: 4M iff 4M>3M, else 3M) / blast-4M.
    println!(
        "\n=== R. RESPONDER strategy by HCP (vul none, mean NS score; opener plays the oracle) ==="
    );
    println!(
        "  {:>4}  {:>6}  {:>8}  {:>8}  {:>8}   best",
        "hcp", "n", "pass2M", "invite", "blast4M"
    );
    for h in 5..=10u8 {
        let idx: Vec<usize> = (0..n).filter(|&d| resp_hcp[d] == h).collect();
        if idx.is_empty() {
            continue;
        }
        let mean = |f: &dyn Fn(usize) -> i64| -> f64 {
            idx.iter().map(|&d| f(d)).sum::<i64>() as f64 / idx.len() as f64
        };
        let pass = mean(&|d| scores[d][0][0]);
        let invite = mean(&|d| {
            let s = &scores[d][0];
            if s[2] > s[1] { s[2] } else { s[1] }
        });
        let blast = mean(&|d| scores[d][0][2]);
        let best = if pass >= invite && pass >= blast {
            "pass"
        } else if blast >= invite {
            "blast"
        } else {
            "invite"
        };
        println!(
            "  {h:>4}  {:>6}  {pass:>8.1}  {invite:>8.1}  {blast:>8.1}   {best}",
            idx.len()
        );
    }

    // --- R2: IMP-optimal game floor.  Per HCP bucket, mean imps of blast-4M and
    // play-3M vs the pass-2M baseline (real, no oracle).  Where 4M goes positive is
    // the floor to which the to-play game should extend.
    println!("\n=== R2. game-vs-partscore IMP by HCP (vs pass-2M baseline) ===");
    for (vi, (vname, _)) in vuls.iter().enumerate() {
        println!("  vul {vname}:");
        println!(
            "    {:>4}  {:>6}  {:>10}  {:>10}",
            "hcp", "n", "4M-2M", "3M-2M"
        );
        for h in 5..=10u8 {
            let idx: Vec<usize> = (0..n).filter(|&d| resp_hcp[d] == h).collect();
            if idx.is_empty() {
                continue;
            }
            let col4: Vec<i64> = idx
                .iter()
                .map(|&d| imps(scores[d][vi][2] - scores[d][vi][0]))
                .collect();
            let col3: Vec<i64> = idx
                .iter()
                .map(|&d| imps(scores[d][vi][1] - scores[d][vi][0]))
                .collect();
            let (m4, c4) = mean_ci(&col4);
            let (m3, c3) = mean_ci(&col3);
            println!(
                "    {h:>4}  {:>6}  {m4:>+7.3}±{c4:.2}  {m3:>+7.3}±{c3:.2}",
                idx.len()
            );
        }
    }

    // --- O: opener accept rule.  Invite population = responder HCP 7..=8.  For each
    // evaluator sweep T; accept iff eval≥T → 4M else 3M.  IMP/bd vs always-decline.
    let invite_pop: Vec<usize> = (0..n).filter(|&d| (7..=8).contains(&resp_hcp[d])).collect();
    println!(
        "\n=== O. OPENER accept-vs-decline ({} invite boards, responder HCP 7-8) ===",
        invite_pop.len()
    );
    for (vi, (vname, _)) in vuls.iter().enumerate() {
        // Oracle ceiling: accept iff 4M>3M.
        let oracle: Vec<i64> = invite_pop
            .iter()
            .map(|&d| {
                let s = &scores[d][vi];
                if s[2] > s[1] { imps(s[2] - s[1]) } else { 0 }
            })
            .collect();
        let (om, oc) = mean_ci(&oracle);
        println!("  vul {vname}:  oracle ceiling {om:+.4} ± {oc:.4}");
        println!(
            "    {:<10} {:>7}  {:>+9}  {:>14}  {:>8}",
            "evaluator", "best T", "IMP/bd", "95% CI", "accept%"
        );
        for (e, ename) in EVAL_NAMES.iter().enumerate() {
            // Sweep T over the observed value range (0.5 steps) and keep the peak.
            let vals: Vec<f64> = invite_pop.iter().map(|&d| opener_evals[d][e]).collect();
            let lo = vals.iter().cloned().fold(f64::INFINITY, f64::min);
            let hi = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let mut best = (f64::NEG_INFINITY, 0.0f64, 0.0f64, 0.0f64); // (mean, T, ci, acc%)
            let mut t = lo;
            while t <= hi + 0.001 {
                let imps_vec: Vec<i64> = invite_pop
                    .iter()
                    .map(|&d| {
                        let s = &scores[d][vi];
                        if opener_evals[d][e] >= t {
                            imps(s[2] - s[1]) // accept → 4M vs the 3M baseline
                        } else {
                            0
                        }
                    })
                    .collect();
                let acc = invite_pop
                    .iter()
                    .filter(|&&d| opener_evals[d][e] >= t)
                    .count() as f64
                    / invite_pop.len() as f64;
                let (m, c) = mean_ci(&imps_vec);
                if m > best.0 {
                    best = (m, t, c, acc * 100.0);
                }
                t += 0.5;
            }
            println!(
                "    {ename:<10} {:>7.1}  {:>+9.4}  ±{:>12.4}  {:>7.1}",
                best.1, best.0, best.2, best.3
            );
        }
    }

    // --- F: floor evaluator.  Does point_count / fit_value / CCCC separate the
    // blast-4M-vs-pass-2M (10-trick) line better than raw HCP?  Rank-calibrate each
    // to the shipped HCP rule's blast-rate (HCP≥7) over the whole 5-10 band, then
    // measure the IMP delta vs that HCP control on the boards where they diverge.
    // The HCP row is the rank-calibration noise floor (≈0); a positive, CI-excl-0
    // row for another evaluator means it picks the game hands better than raw HCP.
    let resp_e: Vec<[f64; 4]> = (0..n)
        .map(|d| resp_evals(deals[d][Seat::South], majors[d]))
        .collect();
    let ctrl: Vec<usize> = resp_hcp.iter().map(|&h| usize::from(h >= 7)).collect();
    let n_blast = ctrl.iter().filter(|&&a| a == 1).count();
    println!(
        "\n=== F. FLOOR evaluator: blast-4M vs pass-2M (control = HCP≥7, {} blast / {} pass) ===",
        n_blast,
        n - n_blast
    );
    for (vi, (vname, _)) in vuls.iter().enumerate() {
        println!("  vul {vname}:");
        println!(
            "    {:<10} {:>9}  {:>14}  {:>8}",
            "evaluator", "IMP/bd", "95% CI", "diverg"
        );
        for (e, ename) in RESP_EVAL_NAMES.iter().enumerate() {
            let col: Vec<f64> = (0..n).map(|d| resp_e[d][e]).collect();
            let action = calibrate(&col, n_blast);
            let mut board_imps = vec![0i64; n];
            let mut diverg = 0usize;
            for d in 0..n {
                if action[d] != ctrl[d] {
                    diverg += 1;
                    let s = &scores[d][vi];
                    let pick = |a: usize| if a == 1 { s[2] } else { s[0] }; // 1→4M, 0→2M
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

    // --- G: floor THRESHOLD sweep — the gate to author.  For HCP, fit_value and
    // CCCC, sweep T; blast iff eval≥T → 4M else 2M; report the IMP-optimal T, its
    // mean IMP/bd over the whole band (vs all-pass-2M), and the blast-rate there.
    // The peak fit_value/CCCC IMP over the peak HCP IMP is the realisable gain.
    println!("\n=== G. floor THRESHOLD sweep (blast iff eval≥T → 4M, else 2M; vs all-2M) ===");
    for (vi, (vname, _)) in vuls.iter().enumerate() {
        println!("  vul {vname}:");
        println!(
            "    {:<10} {:>7}  {:>9}  {:>14}  {:>8}",
            "evaluator", "best T", "IMP/bd", "95% CI", "blast%"
        );
        for &e in &[0usize, 2, 3] {
            let vals: Vec<f64> = (0..n).map(|d| resp_e[d][e]).collect();
            let lo = vals.iter().cloned().fold(f64::INFINITY, f64::min);
            let hi = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let mut best = (f64::NEG_INFINITY, 0.0f64, 0.0f64, 0.0f64); // mean,T,ci,blast%
            let mut t = lo;
            while t <= hi + 0.001 {
                let col: Vec<i64> = (0..n)
                    .map(|d| {
                        if resp_e[d][e] >= t {
                            imps(scores[d][vi][2] - scores[d][vi][0]) // blast → 4M vs 2M
                        } else {
                            0
                        }
                    })
                    .collect();
                let blast = vals.iter().filter(|&&v| v >= t).count() as f64 / n as f64;
                let (m, c) = mean_ci(&col);
                if m > best.0 {
                    best = (m, t, c, blast * 100.0);
                }
                t += 0.5;
            }
            println!(
                "    {:<10} {:>7.1}  {:>+9.4}  ±{:>12.4}  {:>7.1}",
                RESP_EVAL_NAMES[e], best.1, best.0, best.2, best.3
            );
        }
    }

    // --- I: INVITE head-to-head.  The decisive test the others skip — does a
    // *realistic* invite (responder bids 3M; opener accepts iff its
    // `point_count + trumps-beyond-2` ≥ T*, else rests in 3M) beat the BINARY
    // system's best (pass-2M or blast-4M) over a tight band?  Baseline = pass-2M
    // (= 0).  blast = imps(4M − 2M).  invite = imps((accept?4M:3M) − 2M), T swept
    // for the peak.  The invite pays the real 3M-down-1 tax (its rest is 3M, the
    // binary's is 2M) yet routes the level decision to opener's known fit.  A band
    // where invite > max(0, blast) with CI clearance is a home for 3M.
    println!(
        "\n=== I. INVITE head-to-head (baseline pass-2M=0; opener accepts iff point_count+trump ≥ T*) ==="
    );
    let bands: [(&str, &dyn Fn(u8) -> bool); 8] = [
        ("h=5", &|h| h == 5),
        ("h=6", &|h| h == 6),
        ("h=7", &|h| h == 7),
        ("h=8", &|h| h == 8),
        ("h=9", &|h| h == 9),
        ("h=10", &|h| h == 10),
        ("6-7", &|h| (6..=7).contains(&h)),
        ("7-8", &|h| (7..=8).contains(&h)),
    ];
    for (vi, (vname, _)) in vuls.iter().enumerate() {
        println!("  vul {vname}:");
        println!(
            "    {:<6} {:>6}  {:>16}  {:>26}  {:>6}",
            "band", "n", "blast4M (IMP±CI)", "invite@T* (IMP±CI, T, acc%)", "winner"
        );
        for (bname, pred) in &bands {
            let idx: Vec<usize> = (0..n).filter(|&d| pred(resp_hcp[d])).collect();
            if idx.is_empty() {
                continue;
            }
            let s = |d: usize| &scores[d][vi];
            let blast_col: Vec<i64> = idx.iter().map(|&d| imps(s(d)[2] - s(d)[0])).collect();
            let (bm, bc) = mean_ci(&blast_col);
            // Sweep opener's fit_value (point_count + trumps beyond 2) for the peak.
            let ovals: Vec<f64> = idx.iter().map(|&d| opener_evals[d][3]).collect();
            let lo = ovals.iter().cloned().fold(f64::INFINITY, f64::min);
            let hi = ovals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let mut best = (f64::NEG_INFINITY, 0.0f64, 0.0f64, 0.0f64); // mean,T,ci,acc%
            let mut t = lo;
            while t <= hi + 0.001 {
                let col: Vec<i64> = idx
                    .iter()
                    .map(|&d| {
                        let contract = if opener_evals[d][3] >= t {
                            s(d)[2]
                        } else {
                            s(d)[1]
                        };
                        imps(contract - s(d)[0])
                    })
                    .collect();
                let acc = idx.iter().filter(|&&d| opener_evals[d][3] >= t).count() as f64
                    / idx.len() as f64;
                let (m, c) = mean_ci(&col);
                if m > best.0 {
                    best = (m, t, c, acc * 100.0);
                }
                t += 0.5;
            }
            let winner = if best.0 >= bm.max(0.0) {
                "invite"
            } else if bm >= 0.0 {
                "blast"
            } else {
                "pass"
            };
            println!(
                "    {bname:<6} {:>6}  {bm:>+8.3}±{bc:<7.3}  {:>+8.3}±{:<6.3} T={:<4.1} {:>5.1}  {winner}",
                idx.len(),
                best.0,
                best.2,
                best.1,
                best.3
            );
        }
    }

    println!(
        "\n(O: IMP/bd is accept-rule vs always-pass-3M; oracle is the per-board ceiling.\n F: IMP/bd vs the HCP-calibrated control; * = 95% CI excludes 0, HCP row ≈ noise floor.\n G: best-threshold floor per evaluator; peak fit_value/CCCC over peak HCP = realisable gain.\n I: realistic invite vs binary pass-2M(=0)/blast-4M; invite-wins band with CI clearance = a home for 3M.)"
    );
}
