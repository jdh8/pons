//! probe-trick-variance — does hand SHAPE widen the double-dummy trick range?
//!
//! Reads the pre-solved `.pdd` bank (no solver) and buckets N-S par tricks by
//! the shape of one hand at MATCHED HCP. Tests jdh8's claim that unbalanced
//! hands produce a wider *range* of tricks than flat hands — the empirical half
//! of "bid game with shape" (convex scoring rewards the fatter making-tail).
//!
//! **Cut C** is the pre-registered kill gate for the additive `(μ, ln σ)` hand
//! valuation (`docs/binky-points.md`): at *matched* partnership HCP, does the
//! conditional σ of N-S notrump tricks move enough between shape classes to
//! change a game decision? A game gate fires on `P(T ≥ 9) ≥ break-even`, and
//! `dP/dσ ≈ 0.121/σ` at the vulnerable break-even (37.5%), `0.045/σ` at the
//! non-vulnerable one (45.5%) — σ has *zero* effect exactly at the boundary and
//! bites only through that asymmetry. Against this repo's standing 8-point
//! NV-vs-vul yardstick, the σ column needs a conditional spread of **≳ 0.3
//! tricks** to be worth publishing. Cut C measures that spread directly, with
//! no model fitted.
//!
//!   cargo run --release --example probe-trick-variance -- \
//!       --deals /nfs2/jdh8/pons/24.pdd --skip 5000000 --count 4000000
use clap::Parser;
use contract_bridge::eval::hcp as holding_hcp;
use contract_bridge::{Hand, Seat, Strain, Suit};
use pons::bidding::constraint::{point_count, upgrade};
use pons::bidding::evaluator::Gaussian;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "/nfs2/jdh8/pons/24.pdd")]
    deals: String,
    #[arg(long, default_value_t = 5_000_000)]
    skip: u64,
    #[arg(long, default_value_t = 4_000_000)]
    count: usize,
}

#[derive(Default)]
struct Moment {
    n: u64,
    sum: f64,
    sumsq: f64,
}

impl Moment {
    fn push(&mut self, x: u8) {
        let x = f64::from(x);
        self.n += 1;
        self.sum += x;
        self.sumsq += x * x;
    }
    fn mean(&self) -> f64 {
        self.sum / self.n as f64
    }
    fn sd(&self) -> f64 {
        let m = self.mean();
        (self.sumsq / self.n as f64 - m * m).max(0.0).sqrt()
    }
}

#[derive(Default)]
struct Acc {
    par: Moment, // best-strain (par) N-S tricks
    nt: Moment,  // fixed-strain: N-S notrump tricks
    ge10: u64,   // a suit game makes on tricks (par >= 10)
    ge11: u64,
    le8: u64,
    nt_ge9: u64, // 3NT makes
}

impl Acc {
    fn push(&mut self, par: u8, nt: u8) {
        self.par.push(par);
        self.nt.push(nt);
        self.ge10 += u64::from(par >= 10);
        self.ge11 += u64::from(par >= 11);
        self.le8 += u64::from(par <= 8);
        self.nt_ge9 += u64::from(nt >= 9);
    }
    fn report(&self, label: &str) {
        if self.par.n == 0 {
            println!("{label:<28} n=0");
            return;
        }
        let n = self.par.n as f64;
        println!(
            "{label:<28} n={:>7}  par mean={:.3} SD={:.3}  |  NT mean={:.3} SD={:.3}  |  \
             P(par>=10)={:>5.1}% P(par>=11)={:>5.1}% P(par<=8)={:>5.1}% P(3NT)={:>5.1}%",
            self.par.n,
            self.par.mean(),
            self.par.sd(),
            self.nt.mean(),
            self.nt.sd(),
            100.0 * self.ge10 as f64 / n,
            100.0 * self.ge11 as f64 / n,
            100.0 * self.le8 as f64 / n,
            100.0 * self.nt_ge9 as f64 / n,
        );
    }
}

fn hcp_of(hand: Hand) -> u8 {
    Suit::ASC.iter().map(|&s| holding_hcp::<u8>(hand[s])).sum()
}

/// 4333 / 4432 / 5332: no void/singleton and at most one doubleton.
fn is_balanced(hand: Hand) -> bool {
    let mut l = Suit::ASC.map(|s| hand[s].len());
    l.sort_unstable();
    l[0] >= 2 && l[1] >= 3
}

/// The partnership's longest combined suit — Cut C's shape axis, chosen because
/// it is the notrump variance driver: a nine-card suit either runs or it does
/// not, and which one it is depends on cards N-S cannot see.
fn longest_fit(north: Hand, south: Hand) -> usize {
    Suit::ASC
        .iter()
        .map(|&s| north[s].len() + south[s].len())
        .max()
        .unwrap_or(0)
}

/// An opponent with nothing to bid: real auctions disclose exactly the E-W
/// cards whose placement drives the spread being priced, so the honest
/// replication of Cut C is the slice where both opponents would pass throughout.
fn quiet(hand: Hand) -> bool {
    hcp_of(hand) < 12 && Suit::ASC.iter().all(|&s| hand[s].len() <= 5)
}

/// Lowest partnership HCP that Cut C bands, then three per band.
const BAND_LO: u8 = 18;
/// Bands 18-20, 21-23, 24-26, 27-29 — where a 3NT decision is actually live.
const BANDS: usize = 4;
/// Longest combined suit: ≤7, 8, 9, ≥10.
const CLASSES: usize = 4;
/// Deals a cell needs before its SD enters the spread; below this the SD's own
/// standard error (≈ σ/√2n) is the size of the effect being measured.
const MIN_CELL: u64 = 10_000;

/// Partnership HCP × shape, the grid Cut C reads its verdict off.
#[derive(Default)]
struct Grid([[Acc; CLASSES]; BANDS]);

impl Grid {
    fn push(&mut self, pair_hcp: u8, fit: usize, par: u8, nt: u8) {
        let Some(offset) = pair_hcp.checked_sub(BAND_LO) else {
            return;
        };
        let band = usize::from(offset) / 3;
        if band < BANDS {
            self.0[band][fit.saturating_sub(7).min(CLASSES - 1)].push(par, nt);
        }
    }

    fn report(&self, title: &str) {
        println!("\n== Cut C: {title} ==");
        for (b, row) in self.0.iter().enumerate() {
            let lo = usize::from(BAND_LO) + 3 * b;
            println!("  partnership {lo}-{} HCP", lo + 2);
            let mut spreads = Vec::new();
            for (c, acc) in row.iter().enumerate() {
                acc.report(["    fit <=7", "    fit 8", "    fit 9", "    fit >=10"][c]);
                if acc.nt.n >= MIN_CELL {
                    spreads.push(acc.nt.sd());
                }
            }
            let (Some(lo_sd), Some(hi_sd)) = (
                spreads.iter().copied().reduce(f64::min),
                spreads.iter().copied().reduce(f64::max),
            ) else {
                continue;
            };
            let (p_lo, p_hi) = p_3nt_at_matched_mean(lo_sd, hi_sd);
            println!(
                "    -> sigma {lo_sd:.3}..{hi_sd:.3}  spread={:.3} tricks  |  \
                 P(3NT) at matched mu: {:.1}% -> {:.1}%  swing={:+.1}pp  [{}]",
                hi_sd - lo_sd,
                100.0 * p_lo,
                100.0 * p_hi,
                100.0 * (p_hi - p_lo),
                if hi_sd - lo_sd >= 0.3 { "PASS" } else { "fail" },
            );
        }
    }
}

/// What the σ column is worth, in the only currency that matters: two
/// partnerships of *equal* μ whose spreads differ land this far apart in
/// `P(3NT)`. Parks μ at the vulnerable game break-even for the middle spread,
/// then re-reads the probability at each end — σ has no effect at all when μ
/// sits exactly on the boundary, so the asymmetry is the whole mechanism.
fn p_3nt_at_matched_mean(sd_lo: f64, sd_hi: f64) -> (f64, f64) {
    /// Φ⁻¹(0.375) — the vulnerable game break-even, in units of σ below 8.5.
    const Z_VUL: f64 = -0.318_639;

    let mean = 8.5 + Z_VUL * 0.5 * (sd_lo + sd_hi);
    let p = |sd: f64| {
        f64::from(
            Gaussian {
                mean: mean as f32,
                sd: sd as f32,
            }
            .p_at_least(9),
        )
    };
    (p(sd_lo), p(sd_hi))
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let boards = pons::pdd::load_slice(&args.deals, args.skip, args.count)?;

    // Cut A — hand = 12 HCP exactly, partner 12-14: isolate SHAPE at equal strength.
    let mut a_bal = Acc::default();
    let mut a_unb = Acc::default();
    // Cut B — the points13 bet: flat 13 (Hcp13 forces) vs unbalanced 11-12 upgraded to >=13.
    let mut b_base = Acc::default();
    let mut b_cand = Acc::default();
    // Cut C — the (mu, sigma) kill gate: conditional SD at MATCHED partnership HCP.
    let mut c_all = Grid::default();
    let mut c_quiet = Grid::default();
    // sanity: is the bank uniform-random? (expect mean HCP ~10, ~47% balanced)
    let (mut all_n, mut all_hcp, mut all_bal) = (0u64, 0u64, 0u64);

    for (deal, table) in &boards {
        let ns = |strain: Strain| {
            table[strain]
                .get(Seat::North)
                .get()
                .max(table[strain].get(Seat::South).get())
        };
        let par = Strain::ASC.iter().map(|&s| ns(s)).max().unwrap();
        let nt = ns(Strain::Notrump);

        let hand = deal[Seat::South];
        let partner = deal[Seat::North];
        let h = hcp_of(hand);
        let bal = is_balanced(hand);

        all_n += 1;
        all_hcp += u64::from(h);
        all_bal += u64::from(bal);

        // Cut C sees every deal — it conditions on partnership strength, not on
        // one hand being opener-ish, so it must run before Cut A/B's filter.
        let pair_hcp = h + hcp_of(partner);
        let fit = longest_fit(partner, hand);
        c_all.push(pair_hcp, fit, par, nt);
        if quiet(deal[Seat::East]) && quiet(deal[Seat::West]) {
            c_quiet.push(pair_hcp, fit, par, nt);
        }

        if !(12..=14).contains(&hcp_of(partner)) {
            continue; // opener-ish partner
        }
        if h == 12 {
            if bal {
                a_bal.push(par, nt);
            } else {
                a_unb.push(par, nt);
            }
        }
        if bal && h == 13 {
            b_base.push(par, nt);
        }
        if !bal && (11..=12).contains(&h) && upgrade(hand) >= 1 && point_count(hand) >= 13 {
            b_cand.push(par, nt);
        }
    }

    println!(
        "bank={} slice {}..{}  boards={}",
        args.deals,
        args.skip,
        args.skip + boards.len() as u64,
        boards.len()
    );
    println!(
        "sanity (all South hands): n={all_n} mean_hcp={:.2} balanced={:.1}%",
        all_hcp as f64 / all_n as f64,
        100.0 * all_bal as f64 / all_n as f64
    );
    println!("\n== Cut A: hand = 12 HCP, partner 12-14 HCP -- isolate SHAPE at equal strength ==");
    a_bal.report("  balanced 12");
    a_unb.report("  unbalanced 12");
    println!("\n== Cut B: the points13 bet (partner 12-14 HCP) ==");
    b_base.report("  flat 13 (Hcp13 forces)");
    b_cand.report("  unbal 11-12 -> pts>=13");

    println!(
        "\n\nCut C -- the (mu, sigma) kill gate. Conditional SD of N-S notrump tricks at\n\
         MATCHED partnership HCP. The sigma column of an additive hand valuation is worth\n\
         publishing only if shape moves that SD by >= 0.3 tricks; below ~0.2 it is decorative."
    );
    c_all.report("all deals");
    c_quiet.report("both opponents quiet (<12 HCP, no 6-card suit) -- the honest replication");
    Ok(())
}
