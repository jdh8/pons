//! sd-lead paired delta of two aligned `bba-gen` dumps — the middle bracket.
//!
//! [`ab-dump-diff`](../ab-dump-diff/main.rs) scores each arm's reached contract
//! with double-dummy play (plain) or a perfect-defense doubler (pd).  This
//! third scorer prices the one information seam DD gets most wrong: the opening
//! lead.  The leader (declarer's LHO) chooses a card *single-dummy* — best over
//! `sd_worlds` layouts consistent with the auction as their own book reads it —
//! and play thereafter is double-dummy on the actual deal (`single_dummy_leads`;
//! Pavlicek: a blind lead pays declarer ≈+0.3 tricks at the 1NT level).  Truth
//! sits between plain DD (under-punishes overbids) and pd (over-punishes); the
//! blind lead is the realistic middle, so a plain-win / pd-loss "doubling
//! artifact" that is *also* sd-positive is real value pd merely over-punishes.
//!
//! Compared at **auction** granularity, not contract: the same contract reached
//! through a different auction can draw a different blind lead, so an sd swing
//! exists wherever the two `table_a` auctions differ.
//!
//! ```text
//! cargo run --release --features serde --example ab-dump-sd -- on.json off.json
//! ```

use clap::Parser;
use contract_bridge::auction::Auction;
use contract_bridge::{AbsoluteVulnerability, Contract, Seat};
use pons::american;
use pons::bidding::american::{
    NegativeDoubleShape, set_free_bids, set_negative_double_shape, set_rule_of_20,
};
use pons::bidding::context::relative;
use pons::bidding::{Family, Inferences, Stance};
use pons::scoring::{final_contract, imps};
use pons::single_dummy::{LeadQuestion, single_dummy_leads};
use rand::SeedableRng;
use rand::rngs::StdRng;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Dump, mean_with_ci, seat_to_act};

#[derive(Parser)]
struct Args {
    /// Dump bid with the feature ON (its `table_a` auction is scored)
    on: String,
    /// Dump bid with the feature OFF, same seed/deals (the baseline)
    off: String,
    /// Re-price at this vulnerability instead of the dump's
    #[arg(short, long)]
    vulnerability: Option<AbsoluteVulnerability>,
    /// Worlds sampled per blind lead (the validated GTO setting is 16)
    #[arg(long, default_value_t = 16)]
    sd_worlds: usize,
    /// Seed for the world-sampling RNG (report it to reproduce a run)
    #[arg(long, default_value_t = 20_240_607)]
    sd_seed: u64,
    /// Read the ON arm's auctions with the Rule-of-20 opener disclosure (its
    /// light 10-11 openings widen what an opening bid shows to the leader)
    #[arg(long, default_value_t = false)]
    on_rule_of_20: bool,
    /// Read the ON arm's auctions with responder's free bids authored
    /// (`set_free_bids`; opener's answers ride along)
    #[arg(long, default_value_t = false)]
    on_ns_free_bids: bool,
    /// Read the ON arm's auctions under this negative-double school:
    /// modern (shipped default) | both-majors | cachalot | sputnik
    /// (`set_negative_double_shape`; all but both-majors imply the free bids)
    #[arg(long, default_value = "modern")]
    on_ns_negative_double_shape: String,
    /// Show this many of the biggest swings (each way)
    #[arg(long, default_value_t = 8)]
    show: usize,
}

/// Signed-for-NS score of a contract given declarer's (single-dummy) tricks.
/// Copied from `ab-nt-defense-matrix` (the campaign's `ns_score_tricks`
/// promotion to `src/scoring.rs` is still a TODO).
fn ns_score_tricks(
    contract: Contract,
    declarer: Seat,
    tricks: u8,
    vul: AbsoluteVulnerability,
) -> i64 {
    let declarer_vul = vul.contains(match declarer {
        Seat::North | Seat::South => AbsoluteVulnerability::NS,
        Seat::East | Seat::West => AbsoluteVulnerability::EW,
    });
    let score = i64::from(contract.score(tricks, declarer_vul));
    match declarer {
        Seat::North | Seat::South => score,
        Seat::East | Seat::West => -score,
    }
}

/// The (contract, declarer, leader-view inferences) of one auction, read
/// through `stance`; `None` for a pass-out (sd score 0).  Mirrors
/// `ab-nt-defense-matrix::lead_inputs` with a single stance.
fn lead_inputs(
    auction: &Auction,
    stance: &Stance,
    dealer: Seat,
    vul: AbsoluteVulnerability,
) -> Option<(Contract, Seat, Inferences)> {
    let (contract, declarer) = final_contract(auction, dealer)?;
    let leader = declarer.lho();
    // Align the read prefix so the leader is the player on lead: the last
    // non-pass call sits within the final four calls, so exactly one of these
    // prefix lengths keeps every non-pass call and puts the leader to act.
    let cut = (auction.len().saturating_sub(3)..=auction.len())
        .find(|&len| seat_to_act(dealer, len) == leader)
        .expect("one of four consecutive lengths reaches every seat");
    Some((
        contract,
        declarer,
        stance.infer(relative(vul, leader), &auction[..cut]),
    ))
}

fn read_dump(path: &str) -> Dump {
    serde_json::from_reader(std::io::BufReader::new(
        std::fs::File::open(path).unwrap_or_else(|e| panic!("open dump {path}: {e}")),
    ))
    .unwrap_or_else(|e| panic!("parse dump {path}: {e}"))
}

fn main() {
    let args = Args::parse();
    let on = read_dump(&args.on);
    let off = read_dump(&args.off);
    assert_eq!(on.boards.len(), off.boards.len(), "dumps must be aligned");
    let vul = args.vulnerability.unwrap_or(on.vulnerability);
    let n = on.boards.len();

    // Leader-view stances (knobs are read at book-construction time).  The OFF
    // arm always reads with the default book; the ON arm discloses whatever
    // knobs its auctions were bid with.
    let shape = |name: &str| match name {
        "both-majors" => NegativeDoubleShape::BothMajors,
        "modern" => NegativeDoubleShape::Modern,
        "cachalot" => NegativeDoubleShape::Cachalot,
        "sputnik" => NegativeDoubleShape::Sputnik,
        other => panic!("unknown negative-double shape {other}"),
    };
    set_rule_of_20(args.on_rule_of_20);
    set_free_bids(args.on_ns_free_bids);
    set_negative_double_shape(shape(&args.on_ns_negative_double_shape));
    let stance_on = american().against(Family::NATURAL);
    set_rule_of_20(false);
    set_free_bids(false);
    set_negative_double_shape(NegativeDoubleShape::Modern);
    let stance_off = american().against(Family::NATURAL);

    // Build every blind-lead question on the boards whose auctions differ; a
    // pass-out contributes score 0 (its arm is simply omitted here).
    let mut pending: Vec<(usize, bool, Contract, Seat)> = Vec::new();
    let mut questions: Vec<LeadQuestion> = Vec::new();
    let mut fired = 0usize;
    for (i, (a, b)) in on.boards.iter().zip(&off.boards).enumerate() {
        assert_eq!(a.deal, b.deal, "dumps not seed-aligned");
        if a.table_a == b.table_a {
            continue; // identical auction ⇒ identical blind lead ⇒ swing 0
        }
        fired += 1;
        for (arm_on, board, stance) in [(true, a, &stance_on), (false, b, &stance_off)] {
            if let Some((contract, declarer, inferences)) =
                lead_inputs(&board.table_a, stance, board.dealer, vul)
            {
                pending.push((i, arm_on, contract, declarer));
                questions.push(LeadQuestion {
                    deal: board.deal,
                    strain: contract.bid.strain,
                    declarer,
                    inferences,
                });
            }
        }
    }

    let mut rng = StdRng::seed_from_u64(args.sd_seed);
    let mut on_score = vec![0i64; n];
    let mut off_score = vec![0i64; n];
    const CHUNK: usize = 4096;
    for (asked, chunk) in pending.chunks(CHUNK).zip(questions.chunks(CHUNK)) {
        let answers = single_dummy_leads(chunk, &mut rng, args.sd_worlds);
        for (&(i, arm_on, contract, declarer), &(_, tricks)) in asked.iter().zip(&answers) {
            let score = ns_score_tricks(contract, declarer, u8::from(tricks), vul);
            if arm_on {
                on_score[i] = score;
            } else {
                off_score[i] = score;
            }
        }
    }

    let board_imps: Vec<i64> = (0..n).map(|i| imps(on_score[i] - off_score[i])).collect();
    let (mean, ci) = mean_with_ci(&board_imps);
    let total: i64 = board_imps.iter().sum();
    #[allow(clippy::cast_precision_loss)]
    {
        println!(
            "sd-lead ON {} vs OFF {} ({n} boards, vul {vul}, {} worlds): {fired} fired ({:.2}%)",
            on.our_label,
            off.our_label,
            args.sd_worlds,
            100.0 * fired as f64 / n.max(1) as f64,
        );
        println!(
            "Delta (run − sit): {total:+} IMPs, {mean:+.4} IMPs/board [95% CI ±{ci:.4}], {:+.3} IMPs/fired",
            total as f64 / fired.max(1) as f64,
        );
    }

    let mut swings: Vec<(usize, i64)> = board_imps
        .iter()
        .enumerate()
        .filter(|&(_, &imp)| imp != 0)
        .map(|(i, &imp)| (i, imp))
        .collect();
    swings.sort_by_key(|&(_, imp)| imp);
    let show = args.show.min(swings.len());
    if show > 0 {
        println!("--- {show} worst (for the feature) ---");
        for &(i, imp) in swings.iter().take(show) {
            println!(
                "[{imp:+} IMP] {}\n  on:  {}\n  off: {}",
                on.boards[i].deal, on.boards[i].table_a, off.boards[i].table_a
            );
        }
    }
}
