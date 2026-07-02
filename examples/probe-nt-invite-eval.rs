//! Which hand evaluator best decides the 1NT-response invite/force boundary, and
//! does the answer differ by responder shape? (AI-bidder; see `docs/ai-bidder/`.)
//!
//! The book invites at exactly 8 HCP and forces game at 9+, using **raw HCP**
//! and a **uniform** cut for every responder.  Fifths was tested at the lumped
//! 3NT-force rule and lost — but that aggregate could mask a shape-dependent
//! effect.  This screen isolates the decision and asks, per shape class, whether
//! any evaluator out-ranks HCP at the boundary.
//!
//! Two responder shape classes (a balanced hand with no four-card major always
//! has a three-card major, so "2♠" and "3♣" are not distinct shapes — they are
//! the *invite* and *force* sides of the same hand):
//!
//! - **Stayman** — a four-card major (`2♣`).  Fit (opener 4+ in that major) ⇒
//!   game `4M` / invite `3M`; misfit ⇒ `3NT` / `2NT`.
//! - **No-4-major** — balanced, no four-card major (the `2♠`-invite vs `3♣`-Puppet
//!   boundary).  The *invite* is the `2♠` notrump size-ask (no fit search:
//!   accept `3NT`, decline `2NT`); the *force* is `3♣` Puppet, which *does* find a
//!   5-3 major fit (`4M`) — else `3NT`.  A flat 4-3-3-3 never fit-searches.
//!
//! Method: deal responder (South) in each class with HCP in the boundary band
//! 7..=10 opposite a 15-17 balanced opener (North); double-dummy solve once.  For
//! each deal score the three actions — pass = `1NT`, force = game, invite =
//! opener-accepts-with-17 ? game/3NT : partscore.  The control rule is HCP
//! (≤7 pass, 8 invite, 9+ force).  Each candidate evaluator is **rank-calibrated**
//! to the control's exact action frequencies, so the IMP delta vs control is a
//! pure *ranking-quality* test (same selectivity, not more aggression).
//!
//! ```text
//! cargo run --release --example probe-nt-invite-eval -- 30000 0
//! ```
//! Args (positional, optional): `count` per class (default 30000), `seed`
//! (default 0).  Heavy — run via `scripts/idle-run.sh`.
//!
//! ponytail: declarer = best of N/S per contract (matches probe-nt-range-split);
//! opener acceptance held at the book's 17+ rule; minor-suit games omitted.

use contract_bridge::eval::{self, HandEvaluator};
use contract_bridge::{
    AbsoluteVulnerability, Bid, Contract, FullDeal, Hand, Penalty, Rank, Seat, Strain, Suit,
};
use ddss::{NonEmptyStrainFlags, Solver, TrickCountTable};
use pons::bidding::constraint::point_count;
use pons::scoring::{imps, ns_score_contract};
use rand::SeedableRng;
use rand::rngs::StdRng;

// --- evaluators (responder hand → scalar) -----------------------------------

fn raw_hcp(hand: Hand) -> u8 {
    Suit::ASC.iter().map(|&s| eval::hcp::<u8>(hand[s])).sum()
}

/// Controls: ace = 2, king = 1.
fn controls(hand: Hand) -> u8 {
    Suit::ASC
        .into_iter()
        .map(|s| 2 * u8::from(hand[s].contains(Rank::A)) + u8::from(hand[s].contains(Rank::K)))
        .sum()
}

fn ev_hcp(h: Hand) -> f64 {
    f64::from(raw_hcp(h))
}
fn ev_points(h: Hand) -> f64 {
    f64::from(point_count(h))
}
fn ev_fifths(h: Hand) -> f64 {
    eval::FIFTHS.eval(h)
}
fn ev_bumrap(h: Hand) -> f64 {
    eval::BUMRAP.eval(h)
}
fn ev_cccc(h: Hand) -> f64 {
    eval::cccc(h)
}
fn ev_controls(h: Hand) -> f64 {
    f64::from(controls(h))
}

/// A named hand evaluator: `(label, fn)`.
type Eval = (&'static str, fn(Hand) -> f64);

/// HCP first — its calibrated arm must score ~0 vs the control (the sanity check).
const EVALS: &[Eval] = &[
    ("HCP", ev_hcp),
    ("points", ev_points),
    ("fifths", ev_fifths),
    ("bumrap", ev_bumrap),
    ("cccc", ev_cccc),
    ("controls", ev_controls),
];

// --- shape predicates -------------------------------------------------------

/// Balanced: 4-3-3-3, 4-4-3-2, or 5-3-3-2 (no void/singleton, ≤1 doubleton, ≤5 long).
fn is_balanced(h: Hand) -> bool {
    let lens = Suit::ASC.map(|s| h[s].len());
    lens.iter().all(|&l| l >= 2)
        && lens.iter().filter(|&&l| l == 2).count() <= 1
        && lens.iter().all(|&l| l <= 5)
}

/// Flat 4-3-3-3 — the only shape with every suit 3+ (sum 13 forces 4-3-3-3).
fn is_flat4333(h: Hand) -> bool {
    Suit::ASC.into_iter().all(|s| h[s].len() >= 3)
}

/// Stayman shape: a four-card major (never on a flat 4-3-3-3 — it plays notrump),
/// no five-card major (those transfer).
fn is_stayman(h: Hand) -> bool {
    let (hh, ss) = (h[Suit::Hearts].len(), h[Suit::Spades].len());
    !is_flat4333(h) && (hh == 4 || ss == 4) && hh < 5 && ss < 5
}

/// No-4-major shape: balanced, and either no four-card major or a flat 4-3-3-3
/// (the 2♠ size-ask / 3♣ Puppet hand).  Disjoint from [`is_stayman`].
fn is_no4major(h: Hand) -> bool {
    is_balanced(h) && ((h[Suit::Hearts].len() < 4 && h[Suit::Spades].len() < 4) || is_flat4333(h))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Stayman,
    No4Major,
}

/// The four-card major opener completes a 4-4 Stayman fit in (higher first).
fn stayman_fit(resp: Hand, opener: Hand) -> Option<Suit> {
    [Suit::Spades, Suit::Hearts]
        .into_iter()
        .find(|&m| resp[m].len() == 4 && opener[m].len() >= 4)
}

/// The three-card major opener completes a 5-3 Puppet fit in; never on a 4-3-3-3.
fn puppet_fit(resp: Hand, opener: Hand) -> Option<Suit> {
    if is_flat4333(resp) {
        return None;
    }
    [Suit::Spades, Suit::Hearts]
        .into_iter()
        .find(|&m| resp[m].len() == 3 && opener[m].len() >= 5)
}

/// `(pass, force, invite-accepted, invite-declined)` contracts for this deal.
fn contracts(class: Class, resp: Hand, opener: Hand) -> [(u8, Strain); 4] {
    let nt = Strain::Notrump;
    match class {
        Class::Stayman => match stayman_fit(resp, opener) {
            Some(m) => {
                let ms = Strain::from(m);
                [(1, nt), (4, ms), (4, ms), (3, ms)]
            }
            None => [(1, nt), (3, nt), (3, nt), (2, nt)],
        },
        Class::No4Major => {
            // Force = Puppet (finds the 5-3 fit); invite = 2♠ notrump size-ask.
            let force = puppet_fit(resp, opener).map_or((3, nt), |m| (4, Strain::from(m)));
            [(1, nt), force, (3, nt), (2, nt)]
        }
    }
}

// --- scoring ----------------------------------------------------------------

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

/// HCP control action: 0 = pass (≤7), 1 = invite (8), 2 = force (9+).
fn control_action(hcp: u8) -> usize {
    match hcp {
        0..=7 => 0,
        8 => 1,
        _ => 2,
    }
}

/// Rank-calibrate an evaluator to the control's action frequencies: the bottom
/// `n_pass` hands → pass, the next `n_inv` → invite, the rest → force.  Same
/// selectivity as the control, so any IMP delta is a ranking difference.
fn calibrate(vals: &[f64], n_pass: usize, n_inv: usize) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..vals.len()).collect();
    idx.sort_by(|&a, &b| {
        vals[a]
            .partial_cmp(&vals[b])
            .expect("evaluators are never NaN")
    });
    let mut action = vec![2usize; vals.len()];
    for (rank, &i) in idx.iter().enumerate() {
        action[i] = if rank < n_pass {
            0
        } else if rank < n_pass + n_inv {
            1
        } else {
            2
        };
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
    // Load-bearing shape-predicate self-checks (disjointness + the 4333 rule).
    assert_eq!(control_action(7), 0);
    assert_eq!(control_action(8), 1);
    assert_eq!(control_action(9), 2);

    let mut argv = std::env::args().skip(1);
    let count: usize = argv.next().and_then(|s| s.parse().ok()).unwrap_or(30_000);
    let seed: u64 = argv.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let attempt_cap = count.saturating_mul(4000).max(50_000_000);

    // Deal both classes in one sweep: opener 15-17 balanced, responder in band.
    let mut deals: Vec<FullDeal> = Vec::new();
    let mut classes: Vec<Class> = Vec::new();
    let (mut n_stay, mut n_no4) = (0usize, 0usize);
    let mut rng = StdRng::seed_from_u64(seed);
    let mut attempts = 0usize;
    while (n_stay < count || n_no4 < count) && attempts < attempt_cap {
        attempts += 1;
        let deal = contract_bridge::deck::full_deal(&mut rng);
        let opener = deal[Seat::North];
        if !is_balanced(opener) || !(15..=17).contains(&raw_hcp(opener)) {
            continue;
        }
        let resp = deal[Seat::South];
        if !(7..=10).contains(&raw_hcp(resp)) {
            continue;
        }
        debug_assert!(!(is_stayman(resp) && is_no4major(resp)), "classes overlap");
        let class = if is_stayman(resp) && n_stay < count {
            n_stay += 1;
            Class::Stayman
        } else if is_no4major(resp) && n_no4 < count {
            n_no4 += 1;
            Class::No4Major
        } else {
            continue;
        };
        deals.push(deal);
        classes.push(class);
    }

    eprintln!(
        "dealt: Stayman {n_stay}, No-4-major {n_no4} ({attempts} attempts); solving {} boards…",
        deals.len()
    );
    let tables = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);

    let vuls = [
        ("none", AbsoluteVulnerability::NONE),
        ("both", AbsoluteVulnerability::ALL),
    ];
    let class_list = [
        ("Stayman (2♣)", Class::Stayman),
        ("No-4-major (2♠/3♣)", Class::No4Major),
    ];

    for (cname, class) in class_list {
        // Gather this class's deals, responder evaluators, and per-vul action values.
        let mut resp_evals: Vec<[f64; 6]> = Vec::new();
        let mut hcps: Vec<u8> = Vec::new();
        // values[deal][vul][action]
        let mut values: Vec<[[i64; 3]; 2]> = Vec::new();
        for (i, &c) in classes.iter().enumerate() {
            if c != class {
                continue;
            }
            let (resp, opener, table) = (deals[i][Seat::South], deals[i][Seat::North], &tables[i]);
            let cs = contracts(class, resp, opener);
            let accept = raw_hcp(opener) >= 17;
            let mut per_vul = [[0i64; 3]; 2];
            for (vi, &(_, vul)) in vuls.iter().enumerate() {
                let v = |(l, s): (u8, Strain)| score1(l, s, table, vul);
                let invite = if accept { v(cs[2]) } else { v(cs[3]) };
                per_vul[vi] = [v(cs[0]), invite, v(cs[1])]; // pass, invite, force
            }
            values.push(per_vul);
            hcps.push(raw_hcp(resp));
            resp_evals.push(std::array::from_fn(|e| EVALS[e].1(resp)));
        }

        let n = values.len();
        let control: Vec<usize> = hcps.iter().map(|&h| control_action(h)).collect();
        let n_pass = control.iter().filter(|&&a| a == 0).count();
        let n_inv = control.iter().filter(|&&a| a == 1).count();
        let n_force = n - n_pass - n_inv;

        println!(
            "\n=== {cname}: {n} deals  (control: pass {n_pass} / invite {n_inv} / force {n_force}) ==="
        );
        for (vi, (vname, _)) in vuls.iter().enumerate() {
            println!("  vul {vname}:");
            println!(
                "    {:<9} {:>9}  {:>14}  {:>8}",
                "evaluator", "IMP/bd", "95% CI", "diverg"
            );
            for (e, &(ename, _)) in EVALS.iter().enumerate() {
                let col: Vec<f64> = resp_evals.iter().map(|r| r[e]).collect();
                let action = calibrate(&col, n_pass, n_inv);
                let mut board_imps = vec![0i64; n];
                let mut diverg = 0usize;
                for d in 0..n {
                    if action[d] != control[d] {
                        diverg += 1;
                        let v = &values[d][vi];
                        board_imps[d] = imps(v[action[d]] - v[control[d]]);
                    }
                }
                let (mean, ci) = mean_ci(&board_imps);
                let flag = if (mean - ci > 0.0 || mean + ci < 0.0) && ename != "HCP" {
                    " *"
                } else {
                    ""
                };
                println!("    {ename:<9} {mean:>+9.4}  ±{ci:>12.4}  {diverg:>8}{flag}");
            }
        }
    }
    println!("\n(* = 95% CI excludes 0; HCP row is the rank-calibration noise floor, expect ~0)");
}
