//! Opener's rebid over a single-suited 5-card-major game invite after 1NT.
//! (AI-bidder; see `docs/ai-bidder/`.)
//!
//! Opposite a 15-17 balanced 1NT, responder with **exactly five** cards in a major
//! (and ≤3 in the other — a single-suiter, no second four-card suit) and
//! *invitational* values (≈8 HCP) transfers (`2♦/2♥`), opener completes (`2♥/2♠`),
//! then responder bids `2NT` to invite (for spades this is `1NT–2♥–2♠–2NT`; for
//! hearts the `2NT` step is taken by the 5♥4♠ invite, so the single-suiter relays
//! through `2♠`).  Opener — holding *two or three* trumps (the fit is unknown) and
//! a minimum or a maximum — must place the contract:
//!
//! - **min + 2 trumps** → rest in `2NT` (no fit, minimum)
//! - **min + 3 trumps** → `3M`   (the 5-3 partscore)
//! - **max + 2 trumps** → `3NT`  (no fit, game)
//! - **max + 3 trumps** → `4M`   (the 5-3 game) — *unless* a flat 4-3-3-3 plays
//!   better in `3NT`?  That is the open question this screen settles.
//!
//! Experiment **O** is the deliverable: holding the invite population, partition by
//! opener strength (min 15-16 / max 17), trump support (2 / 3+), and flat-4333, and
//! print the IMP delta of the decisive contract pair so the rule reads straight off.
//! Experiment **R** sanity-checks the responder invite band (weak `2M` / invite /
//! blast-game) by HCP — confirming exactly-8 is the invite point, matching the
//! heart side.
//!
//! ```text
//! cargo run --release --example probe-fivecard-invite-eval -- 200000 0
//! ```
//! Args (positional, optional): `count` (default 200000), `seed` (default 0).  The
//! accept decision bites near the 15-17 boundary, so it wants many deals — run via
//! `scripts/idle-run.sh`.
//!
//! ponytail: declarer = best of N/S per contract (right-siding is DD-blind — we
//! score the *level/strain*, 4M vs 3NT, not who declares).  Invite band fixed to 8
//! for experiment O; experiment R prints the bucketed scores so the cut can be
//! re-read without a rebuild.

use contract_bridge::eval::{self};
use contract_bridge::{
    AbsoluteVulnerability, Bid, Contract, FullDeal, Hand, Penalty, Seat, Strain, Suit,
};
use ddss::{NonEmptyStrainFlags, Solver, TrickCountTable};
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

/// Flat 4-3-3-3: balanced with no doubleton (every suit ≥ 3).
fn is_flat_4333(h: Hand) -> bool {
    is_balanced(h) && Suit::ASC.iter().all(|&s| h[s].len() >= 3)
}

/// The anchor major: a major with *exactly* five cards and ≤3 in the other major.
/// `None` for 5-5, 5-4, six-baggers, or no five-card major.
fn anchor(h: Hand) -> Option<Suit> {
    let (hh, ss) = (h[Suit::Hearts].len(), h[Suit::Spades].len());
    if hh == 5 && ss <= 3 {
        Some(Suit::Hearts)
    } else if ss == 5 && hh <= 3 {
        Some(Suit::Spades)
    } else {
        None
    }
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

/// Mean ± CI of `imps(a − b)` over the deals in `idx`, with `pick(d, k)` selecting
/// score column k (0=2M,1=2NT,2=3M,3=3NT,4=4M) for deal d.
fn delta(idx: &[usize], pick: &dyn Fn(usize, usize) -> i64, a: usize, b: usize) -> (f64, f64) {
    let col: Vec<i64> = idx.iter().map(|&d| imps(pick(d, a) - pick(d, b))).collect();
    mean_ci(&col)
}

fn main() {
    let mut argv = std::env::args().skip(1);
    let count: usize = argv.next().and_then(|s| s.parse().ok()).unwrap_or(200_000);
    let seed: u64 = argv.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let attempt_cap = count.saturating_mul(4000).max(50_000_000);

    // Opener (North) 15-17 balanced; responder (South) an exactly-5 single-suited
    // major anchor in a 6..=11 band straddling the weak/invite and invite/game cuts.
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
        if !(6..=11).contains(&raw_hcp(resp)) {
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
    let mut resp_hcp: Vec<u8> = Vec::with_capacity(n);
    let mut op_hcp: Vec<u8> = Vec::with_capacity(n);
    let mut op_support: Vec<usize> = Vec::with_capacity(n);
    let mut op_flat: Vec<bool> = Vec::with_capacity(n);
    // scores[deal][vul] = [2M, 2NT, 3M, 3NT, 4M]
    let mut scores: Vec<[[i64; 5]; 2]> = Vec::with_capacity(n);
    for i in 0..n {
        let (opener, table, m) = (deals[i][Seat::North], &tables[i], majors[i]);
        let ms = Strain::from(m);
        let nt = Strain::Notrump;
        let mut per_vul = [[0i64; 5]; 2];
        for (vi, &(_, vul)) in vuls.iter().enumerate() {
            per_vul[vi] = [
                score1(2, ms, table, vul),
                score1(2, nt, table, vul),
                score1(3, ms, table, vul),
                score1(3, nt, table, vul),
                score1(4, ms, table, vul),
            ];
        }
        scores.push(per_vul);
        resp_hcp.push(raw_hcp(deals[i][Seat::South]));
        op_hcp.push(raw_hcp(opener));
        op_support.push(opener[m].len());
        op_flat.push(is_flat_4333(opener));
    }

    // --- O: opener's rebid table.  Invite population = responder HCP 8 (the live
    // exactly-8 band).  Partition by strength / support / shape and print the IMP
    // delta of the decisive contract pair so the accept rule reads straight off.
    let inv: Vec<usize> = (0..n).filter(|&d| resp_hcp[d] == 8).collect();
    println!(
        "\n=== O. OPENER rebid over the single-suited invite ({} boards, responder HCP 8) ===",
        inv.len()
    );
    println!("    (cols: 0=2M 1=2NT 2=3M 3=3NT 4=4M; delta = imps(first − second) ± 95% CI)");
    for (vi, (vname, _)) in vuls.iter().enumerate() {
        let pick = |d: usize, k: usize| scores[d][vi][k];
        println!("  vul {vname}:");

        let max3_flat: Vec<usize> = inv
            .iter()
            .copied()
            .filter(|&d| op_hcp[d] == 17 && op_support[d] >= 3 && op_flat[d])
            .collect();
        let max3_shapely: Vec<usize> = inv
            .iter()
            .copied()
            .filter(|&d| op_hcp[d] == 17 && op_support[d] >= 3 && !op_flat[d])
            .collect();
        let max2: Vec<usize> = inv
            .iter()
            .copied()
            .filter(|&d| op_hcp[d] == 17 && op_support[d] == 2)
            .collect();
        let min3: Vec<usize> = inv
            .iter()
            .copied()
            .filter(|&d| op_hcp[d] <= 16 && op_support[d] >= 3)
            .collect();
        let min2: Vec<usize> = inv
            .iter()
            .copied()
            .filter(|&d| op_hcp[d] <= 16 && op_support[d] == 2)
            .collect();

        let row = |label: &str, idx: &[usize], a: usize, b: usize, want: &str| {
            let (m, c) = delta(idx, &pick, a, b);
            let sign = if m - c > 0.0 {
                "first"
            } else if m + c < 0.0 {
                "second"
            } else {
                "tie"
            };
            println!(
                "    {label:<28} n={:>6}  {m:>+7.3} ± {c:<6.3}  → {sign:<6} ({want})",
                idx.len()
            );
        };

        // The (4333) question: does max+fit prefer 4M (first) or 3NT (second)?
        row("max,3+ fit  4M vs 3NT", &max3_shapely, 4, 3, "shapely");
        row("max,3+ flat 4M vs 3NT", &max3_flat, 4, 3, "flat-4333");
        // The confident cells.
        row("max,2trump  3NT vs 4M", &max2, 3, 4, "want 3NT");
        row("min,3+ fit  3M vs 2NT", &min3, 2, 1, "want 3M");
        row("min,2trump  2NT vs 3M", &min2, 1, 2, "want 2NT");
        row("min,2trump  2NT vs 2M", &min2, 1, 0, "rest level");
    }

    // --- R: responder band.  Per HCP bucket, mean best-of-N/S score (vul none) of
    // weak (transfer-then-pass = 2M) / invite (opener oracle over {2NT,3M,3NT,4M}) /
    // blast (responder forces, best of {3NT,4M}).  Confirms the 8 invite point.
    println!("\n=== R. RESPONDER strategy by HCP (vul none, mean NS score; opener oracle) ===");
    println!(
        "  {:>4}  {:>6}  {:>8}  {:>8}  {:>8}   best",
        "hcp", "n", "weak2M", "invite", "blast"
    );
    for h in 6..=11u8 {
        let idx: Vec<usize> = (0..n).filter(|&d| resp_hcp[d] == h).collect();
        if idx.is_empty() {
            continue;
        }
        let mean = |f: &dyn Fn(usize) -> i64| -> f64 {
            idx.iter().map(|&d| f(d)).sum::<i64>() as f64 / idx.len() as f64
        };
        let weak = mean(&|d| scores[d][0][0]);
        let invite = mean(&|d| *scores[d][0][1..=4].iter().max().expect("4 contracts"));
        let blast = mean(&|d| scores[d][0][3].max(scores[d][0][4]));
        let best = if weak >= invite && weak >= blast {
            "weak"
        } else if blast >= invite {
            "blast"
        } else {
            "invite"
        };
        println!(
            "  {h:>4}  {:>6}  {weak:>8.1}  {invite:>8.1}  {blast:>8.1}   {best}",
            idx.len()
        );
    }

    println!(
        "\n(O: delta = imps(first − second); → first/second/tie by 95% CI.  The two `max,3+`\n rows answer the flat-4333 carve: if `max,3+ flat` says `second`, opener bids 3NT on a\n flat 4-3-3-3 maximum even with three-card support.\n R: weak=2M, invite=oracle over {{2NT,3M,3NT,4M}}, blast=best game; the invite-wins band\n confirms the exactly-8 invite point.)"
    );
}
