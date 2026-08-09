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
//! - **endpoints** — the 30 hidden-seat values `push_inference` emits, the
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
//! Since the shape-reading campaign the probe reports a **third** group: the
//! per-seat shape distribution [`features_eval_shape`] adds beside the
//! endpoints.  It is the encoding proposed to replace them, and the claim it
//! must survive is exactly this one — an information-free closure moves the
//! endpoints by multiple σ and must move the shape columns by nothing.  Read
//! the two groups against each other; the σ column shows each column really
//! does vary across the corpus, so "never moved" is inertness, not deadness.
//!
//! ```sh
//! cargo run --release --example probe-closure-features -- -c 2000
//! ```

use clap::Parser;
use contract_bridge::{AbsoluteVulnerability, Seat};
use pons::american;
use pons::bidding::context::relative;
use pons::bidding::features::{LEN_HAND_EVAL, LEN_INFERENCE, LEN_SEAT_SHAPE, features_eval_shape};
use pons::bidding::{
    Relative, sample_layouts, set_gauge_membership, set_pass_exclusion_reading, set_sum_closure,
    set_upgrade_closure,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_out, seat_to_act, seeded_deals};

/// The three hidden-seat blocks of [`features_eval_shape`]: they follow the
/// own-hand block and precede the call tail.
const OFFSET: usize = LEN_HAND_EVAL;
const END: usize = OFFSET + 3 * LEN_SEAT_SHAPE;

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

    /// Probe the pass-exclusion reading (`set_pass_exclusion_reading`)
    /// instead — not a closure: it narrows the described hand set, so
    /// membership rejections are the earnable picture, C1's (0 rejections)
    /// the unearnable one
    #[arg(long, conflicts_with = "upgrade")]
    pass_exclusion: bool,

    /// Give the strength gauges membership teeth in *both* arms
    /// ([`set_gauge_membership`]).  C2's membership effect should vanish
    /// under it: C2 bounds `points` by `hcp + upgrade_ceiling(lengths)`, and
    /// a hand inside the box has `upgrade <= upgrade_ceiling`, so any hand
    /// the closure rejects was already rejected by the direct `hcp` test.
    #[arg(long)]
    gauge: bool,
}

/// The 10 endpoint column kinds one seat contributes, for the per-column report
const ENDPOINTS: [&str; LEN_INFERENCE] = [
    "♣ min", "♣ max", "♦ min", "♦ max", "♥ min", "♥ max", "♠ min", "♠ max", "pts min", "pts max",
];

/// Suit glyphs in `Suit::ASC` order — the order every feature block uses
const SUITS: [&str; 4] = ["♣", "♦", "♥", "♠"];

/// Labels for the [`LEN_SEAT_SHAPE`] columns one seat contributes: the
/// endpoints, then the shape distribution in `push_shape_dist`'s emission
/// order — gauss summary, per-suit length marginal, log-mass last.
fn columns() -> Vec<String> {
    let per_suit = |prefix: &str| SUITS.map(|s| format!("{s} {prefix}"));
    let mut out: Vec<String> = ENDPOINTS.iter().map(|s| (*s).to_owned()).collect();
    out.extend(per_suit("E"));
    out.extend(per_suit("sd"));
    for suit in SUITS {
        for len in 0..14 {
            out.push(format!("{suit} ={len}"));
        }
    }
    out.push("mass".to_owned());
    assert_eq!(out.len(), LEN_SEAT_SHAPE);
    out
}

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
    let (knob, name): (fn(bool), &str) = if args.pass_exclusion {
        (set_pass_exclusion_reading, "pass-exclusion")
    } else if args.upgrade {
        (set_upgrade_closure, "C2 upgrade")
    } else {
        (set_sum_closure, "C1 sum")
    };
    // The knob is captured into a stance at build, so the probe needs one book
    // per setting: `stances[0]` knob-off (it also does the bidding), `[1]` on.
    set_gauge_membership(args.gauge);
    knob(false);
    let off = american().against();
    knob(true);
    let stances = [off, american().against()];
    knob(false);
    let stance = &stances[0];

    // Per column kind: every |Δ| that was nonzero, plus every value seen
    // knob-off (for the corpus σ that puts the movement on the net's scale).
    let mut moved: Vec<Vec<f64>> = vec![Vec::new(); LEN_SEAT_SHAPE];
    let mut seen: Vec<Vec<f64>> = vec![Vec::new(); LEN_SEAT_SHAPE];
    let (mut nodes, mut nodes_moved, mut shape_moved) = (0_u64, 0_u64, 0_u64);
    let (mut sampled, mut drawn, mut rejected) = (0_u64, 0_u64, 0_u64);
    let mut witness: Option<String> = None;
    let mut shape_witness: Option<String> = None;

    for (board, deal) in seeded_deals(base, args.count).into_iter().enumerate() {
        let dealer = Seat::ALL[board % 4];
        // Bid under the baseline in both arms: the corpus of readings is fixed,
        // so this isolates the encoding perturbation from any bidding change.
        let auction = bid_out(stance, stance, true, dealer, vul, &deal);

        for cut in 1..=auction.len() {
            let seat = seat_to_act(dealer, cut);
            let prefix = &auction[..cut];
            let rel = relative(vul, seat);

            let read = |on: bool| stances[usize::from(on)].infer(rel, prefix);
            let (off, on) = (read(false), read(true));
            let (a, b) = (
                features_eval_shape(deal[seat], &off, prefix),
                features_eval_shape(deal[seat], &on, prefix),
            );

            nodes += 1;
            let (mut any, mut any_shape) = (false, false);
            for (i, (&x, &y)) in a[OFFSET..END].iter().zip(&b[OFFSET..END]).enumerate() {
                let col = i % LEN_SEAT_SHAPE;
                seen[col].push(f64::from(x));
                let delta = f64::from(y - x).abs();
                if delta > 0.0 {
                    moved[col].push(delta);
                    if col < LEN_INFERENCE {
                        any = true;
                    } else {
                        any_shape = true;
                        // A shape column can only move if the *set of shapes*
                        // the union admits moved — i.e. the closure changed the
                        // reading, not just its bounding box.  That should not
                        // happen for a membership-inert closure, so name the
                        // first one rather than average it away.
                        if shape_witness.is_none() {
                            let who = [Relative::Lho, Relative::Partner, Relative::Rho]
                                [i / LEN_SEAT_SHAPE];
                            shape_witness = Some(format!(
                                "board {board} cut {cut}, {who:?}, column {col} moved {delta:.6}\
                                 \n  loose boxes  {:?}\n  closed boxes {:?}",
                                off.announced_union(who).boxes(),
                                on.announced_union(who).boxes(),
                            ));
                        }
                    }
                }
            }
            shape_moved += u64::from(any_shape);
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

    let pct = |x: u64| 100.0 * x as f64 / nodes as f64;
    println!(
        "{name}: {} boards, {nodes} nodes, seed {base}\n\
         endpoints moved at {nodes_moved} nodes ({:.2}%)\n\
         shape distribution moved at {shape_moved} nodes ({:.2}%)\n",
        args.count,
        pct(nodes_moved),
        pct(shape_moved),
    );

    let labels = columns();
    for (title, range) in [
        (
            "endpoint columns (feature units; 13ths for lengths, 37ths for points)",
            0..LEN_INFERENCE,
        ),
        (
            "shape-distribution columns — an information-free closure must not move these",
            LEN_INFERENCE..LEN_SEAT_SHAPE,
        ),
    ] {
        println!("── {title} ──");
        for col in range {
            let n = seen[col].len() as f64;
            let mean = seen[col].iter().sum::<f64>() / n;
            let sigma = (seen[col].iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n).sqrt();
            Spread(std::mem::take(&mut moved[col])).report(&labels[col], sigma);
        }
        println!();
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
    if let Some(w) = shape_witness {
        println!("\nfirst shape-column witness (the closure moved the admitted shape set):\n{w}");
    }
}
