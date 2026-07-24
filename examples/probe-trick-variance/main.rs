//! probe-trick-variance — does hand SHAPE widen the double-dummy trick range?
//!
//! Reads the pre-solved `.pdd` bank (no solver) and buckets N-S par tricks by
//! the shape of one hand at MATCHED HCP. Tests jdh8's claim that unbalanced
//! hands produce a wider *range* of tricks than flat hands — the empirical half
//! of "bid game with shape" (convex scoring rewards the fatter making-tail).
//!
//!   cargo run --release --example probe-trick-variance -- \
//!       --deals /nfs2/jdh8/pons/24.pdd --skip 5000000 --count 4000000
use clap::Parser;
use contract_bridge::eval::hcp as holding_hcp;
use contract_bridge::{Seat, Strain, Suit};
use pons::bidding::constraint::{point_count, upgrade};

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

fn hcp_of(hand: contract_bridge::Hand) -> u8 {
    Suit::ASC.iter().map(|&s| holding_hcp::<u8>(hand[s])).sum()
}

/// 4333 / 4432 / 5332: no void/singleton and at most one doubleton.
fn is_balanced(hand: contract_bridge::Hand) -> bool {
    let mut l = Suit::ASC.map(|s| hand[s].len());
    l.sort_unstable();
    l[0] >= 2 && l[1] >= 3
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
    Ok(())
}
