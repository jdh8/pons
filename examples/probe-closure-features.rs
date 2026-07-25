//! Does a hull-tightening closure move the net's *inputs* without moving the
//! *distribution* those inputs claim to describe?
//!
//! C1 (`set_sum_closure`) is membership-inert: every real hand satisfies
//! `Σ len = 13`, so narrowing a box by it cannot change `Envelope::admits`.
//! The set of hands consistent with a reading is therefore identical in both
//! arms — every conditional moment is *exactly* unchanged — while
//! `features::push_inference` feeds the evaluator net raw `{min, max}`
//! endpoints of the hull, which does move.  That gap is the encoding
//! indictment: the feature map is not invariant to information-preserving
//! re-representations of the same claim.
//!
//! This probe measures both sides on one fixed corpus of auctions (bid out
//! under the baseline, so the arms cannot diverge in *which* readings they
//! see):
//!
//! - **endpoints** — the 30 hidden-seat values of [`features_eval`], the
//!   evaluator's actual input, diffed knob-off vs knob-on and reported in raw
//!   units and in units of each column's corpus σ (the scale the first layer
//!   sees);
//! - **membership** — [`sample_layouts`] drawn under each arm and cross-tested
//!   against the *other* arm's reading.  Inertness says both directions accept
//!   everything; anything rejected is a real change to the set of hands the
//!   reading describes, and hence to every moment over it.
//!
//! A large endpoint movement beside zero rejections confirms the perturbation
//! is information-free, and no training run is needed to say so.  C1 measures
//! exactly that.  C2 does not — it bounds `points`, which membership tests,
//! from `hcp`, which it does not, so it moves the sampler too.
//!
//! ```sh
//! cargo run --release --example probe-closure-features -- -c 2000
//! ```

use clap::Parser;
use contract_bridge::{AbsoluteVulnerability, Seat};
use pons::american;
use pons::bidding::context::relative;
use pons::bidding::features::LEN_INFERENCE;
use pons::bidding::{
    FEATURES_LEN_EVAL, Family, Relative, features_eval, sample_layouts, set_sum_closure,
    set_upgrade_closure,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_out, seat_to_act, seeded_deals};

/// Where the three hidden-seat range blocks start in [`features_eval`]
const OFFSET: usize = FEATURES_LEN_EVAL - 3 * LEN_INFERENCE;

#[derive(Parser)]
struct Args {
    /// Deals to bid
    #[arg(short, long, default_value = "2000")]
    count: usize,

    /// Seed base; random when omitted
    #[arg(short, long)]
    seed: Option<u64>,

    /// Layouts to sample per node for the moment side (0 disables it)
    #[arg(long, default_value = "64")]
    samples: usize,

    /// Probe C2 (`set_upgrade_closure`) instead of C1
    #[arg(long)]
    upgrade: bool,
}

/// The 10 column kinds one seat contributes, for the per-column report
const COLUMNS: [&str; LEN_INFERENCE] = [
    "♣ min", "♣ max", "♦ min", "♦ max", "♥ min", "♥ max", "♠ min", "♠ max", "pts min", "pts max",
];

/// Running mean / p50 / p90 / max over a stream of magnitudes
struct Spread(Vec<f64>);

impl Spread {
    fn report(&mut self, label: &str, scale: f64) {
        if self.0.is_empty() {
            println!("{label:>10}  (never moved)");
            return;
        }
        self.0.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
        let n = self.0.len();
        let mean = self.0.iter().sum::<f64>() / n as f64;
        let q = |p: f64| self.0[((n as f64 - 1.0) * p) as usize];
        println!(
            "{label:>10}  moved {n:>8}  mean {mean:.4}  p50 {:.4}  p90 {:.4}  max {:.4}   \
             (σ {scale:.4} → p90 {:.2}σ)",
            q(0.5),
            q(0.9),
            q(1.0),
            q(0.9) / scale.max(f64::EPSILON),
        );
    }
}

fn main() {
    let args = Args::parse();
    let base = args.seed.unwrap_or_else(rand::random);
    let vul = AbsoluteVulnerability::NONE;
    let stance = american().against(Family::NATURAL);
    let knob: fn(bool) = if args.upgrade {
        set_upgrade_closure
    } else {
        set_sum_closure
    };
    let name = if args.upgrade { "C2 upgrade" } else { "C1 sum" };

    // Per column kind: every |Δ| that was nonzero, plus every value seen
    // knob-off (for the corpus σ that puts the movement on the net's scale).
    let mut moved: Vec<Vec<f64>> = vec![Vec::new(); LEN_INFERENCE];
    let mut seen: Vec<Vec<f64>> = vec![Vec::new(); LEN_INFERENCE];
    let (mut nodes, mut nodes_moved) = (0_u64, 0_u64);
    let (mut sampled, mut drawn, mut rejected) = (0_u64, 0_u64, 0_u64);
    let mut witness: Option<String> = None;

    for (board, deal) in seeded_deals(base, args.count).into_iter().enumerate() {
        let dealer = Seat::ALL[board % 4];
        // Bid under the baseline in both arms: the corpus of readings is fixed,
        // so this isolates the encoding perturbation from any bidding change.
        knob(false);
        let auction = bid_out(&stance, &stance, true, dealer, vul, &deal);

        for cut in 1..=auction.len() {
            let seat = seat_to_act(dealer, cut);
            let prefix = &auction[..cut];
            let rel = relative(vul, seat);

            let read = |on: bool| {
                knob(on);
                stance.infer(rel, prefix)
            };
            let (off, on) = (read(false), read(true));
            let (a, b) = (
                features_eval(deal[seat], &off),
                features_eval(deal[seat], &on),
            );

            nodes += 1;
            let mut any = false;
            for (i, (&x, &y)) in a[OFFSET..].iter().zip(&b[OFFSET..]).enumerate() {
                let col = i % LEN_INFERENCE;
                seen[col].push(f64::from(x));
                let delta = f64::from(y - x).abs();
                if delta > 0.0 {
                    moved[col].push(delta);
                    any = true;
                }
            }
            if !any {
                continue;
            }
            nodes_moved += 1;

            // Moment side, only where the endpoints moved.  Sample under each
            // arm and cross-test every layout against the *other* arm's
            // reading: membership-inertness says both directions accept
            // everything, so any rejection is a real change to the set of
            // hands the reading describes — the thing the moments average over.
            if args.samples > 0 {
                let draw = |inf: &_| {
                    let mut rng = StdRng::seed_from_u64(base ^ (nodes << 8));
                    sample_layouts(deal[seat], seat, inf, &mut rng, args.samples)
                };
                let (la, lb) = (draw(&off), draw(&on));
                sampled += 1;
                drawn += (la.len() + lb.len()) as u64;

                let hidden = [
                    (seat.lho(), Relative::Lho),
                    (seat.partner(), Relative::Partner),
                    (seat.rho(), Relative::Rho),
                ];
                for (layouts, judge, tag) in [(&la, &on, "narrowed"), (&lb, &off, "loose")] {
                    for layout in layouts {
                        for (abs, who) in hidden {
                            if judge.admits(who, layout[abs]) {
                                continue;
                            }
                            rejected += 1;
                            if witness.is_none() {
                                witness = Some(format!(
                                    "board {board} cut {cut}: a layout drawn under the \
                                     other arm is rejected by the {tag} reading\n  \
                                     {who:?} holds {:?}\n  loose hull {:?}\n  closed hull {:?}",
                                    layout[abs],
                                    off.get(who),
                                    on.get(who),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    knob(false);
    let pct = |x: u64| 100.0 * x as f64 / nodes as f64;
    println!(
        "{name}: {} boards, {nodes} nodes, seed {base}\n\
         endpoints moved at {nodes_moved} nodes ({:.2}%)\n",
        args.count,
        pct(nodes_moved),
    );

    println!("── endpoint columns (feature units; 13ths for lengths, 37ths for points) ──");
    for (col, label) in COLUMNS.iter().enumerate() {
        let n = seen[col].len() as f64;
        let mean = seen[col].iter().sum::<f64>() / n;
        let sigma = (seen[col].iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n).sqrt();
        Spread(std::mem::take(&mut moved[col])).report(label, sigma);
    }

    if args.samples > 0 {
        println!(
            "\n── membership (sample_layouts, {} draws per arm, cross-tested) ──\n\
             {sampled} moved nodes sampled, {drawn} layouts drawn, \
             {rejected} rejected by the other arm",
            args.samples,
        );
        if let Some(w) = witness {
            println!("\nfirst witness:\n{w}");
        }
    }
}
