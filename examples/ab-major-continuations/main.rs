//! Major-opening continuation A/B: baseline vs any combination of the major
//! game tries (`--game-tries`), the limit-raise acceptance (`--limit-raise`),
//! the `1♥ – 1♠` rebid tails (`--tails`), and fourth-suit forcing (`--fsf`,
//! rides `--tails`).
//!
//! Both arms run the same 2/1 system over the same deals; the only difference
//! is the selected `set_*` knobs.  Opponents are silenced (East/West always
//! pass), so this measures the *constructive* value of the treatments.  Each
//! board is bid twice, once per arm; boards whose arms reach different
//! contracts are solved double dummy once and scored with **both** scorers —
//! plain DD (`ns_score_contract`) and perfect defense (`ns_score_pd`) — per
//! the measurement playbook's bracket.
//!
//! Seed hygiene: pass `--seed "$SEED_BASE"` (fresh per experiment, shared by
//! every arm and vulnerability of that experiment).
//!
//! ```text
//! cargo run --release --example ab-major-continuations -- \
//!     --count 200000 --vulnerability none --seed "$SEED_BASE" --tails --fsf
//! ```

use clap::Parser;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::Family;
use pons::bidding::american::{
    TwoOverOneGate, set_fourth_suit_forcing, set_limit_raise_acceptance, set_major_choice_of_games,
    set_major_game_tries, set_major_rebid_tails, set_two_over_one_fit, set_two_over_one_gate,
};
use pons::scoring::final_contract;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_uncontested, report_brackets, seeded_deals};

/// Major-opening continuation A/B: baseline vs the selected treatments
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "20000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Base seed — fresh per experiment (`SEED_BASE=$(date +%s)`), shared
    /// across the experiment's arms; random when omitted
    #[arg(short, long)]
    seed: Option<u64>,

    /// Treatment: opener's major game tries after a single raise, `1M – 2M`
    /// (`set_major_game_tries`)
    #[arg(long, default_value_t = false)]
    game_tries: bool,

    /// Treatment: opener's limit-raise acceptance ladder, `1M – 3M`
    /// (`set_limit_raise_acceptance`)
    #[arg(long, default_value_t = false)]
    limit_raise: bool,

    /// Treatment: the `1♥ – 1♠` rebid tails (`set_major_rebid_tails`)
    #[arg(long, default_value_t = false)]
    tails: bool,

    /// Treatment: fourth-suit forcing, rides `--tails` (`set_fourth_suit_forcing`)
    #[arg(long, default_value_t = false)]
    fsf: bool,

    /// Treatment: the 1M-3NT choice-of-games response — 3-4 card support,
    /// exactly (4333), 12-15 HCP (`set_major_choice_of_games`)
    #[arg(long, default_value_t = false)]
    choice_of_games: bool,

    /// Treatment: the 2/1 fit leg — exactly 3-card support enters on
    /// `support_points(13..)` (`set_two_over_one_fit`)
    #[arg(long, default_value_t = false)]
    two_over_one_fit: bool,

    /// Treatment: the no-fit 2/1 gauge — points13 (baseline) | hcp13 | hcp12
    /// (`set_two_over_one_gate`)
    #[arg(long, default_value = "points13")]
    two_over_one_gate: String,

    /// The no-fit 2/1 gauge of the BASELINE arm — for gate-vs-gate
    /// head-to-heads (both arms otherwise all-off)
    #[arg(long, default_value = "points13")]
    baseline_gate: String,
}

/// Parse the `--two-over-one-gate` argument
fn parse_gate(s: &str) -> TwoOverOneGate {
    match s {
        "points13" => TwoOverOneGate::Points13,
        "hcp13" => TwoOverOneGate::Hcp13,
        "hcp12" => TwoOverOneGate::Hcp12,
        other => panic!("--two-over-one-gate must be points13|hcp13|hcp12, got {other:?}"),
    }
}

/// Set every knob at once; `treatment` false = the all-off baseline arm
fn set_knobs(args: &Args, treatment: bool) {
    set_major_game_tries(treatment && args.game_tries);
    set_limit_raise_acceptance(treatment && args.limit_raise);
    set_major_rebid_tails(treatment && args.tails);
    set_fourth_suit_forcing(treatment && args.fsf);
    set_major_choice_of_games(treatment && args.choice_of_games);
    set_two_over_one_fit(treatment && args.two_over_one_fit);
    set_two_over_one_gate(if treatment {
        parse_gate(&args.two_over_one_gate)
    } else {
        parse_gate(&args.baseline_gate)
    });
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let gate_selected = args.two_over_one_gate != "points13";
    assert!(
        args.game_tries
            || args.limit_raise
            || args.tails
            || args.fsf
            || args.choice_of_games
            || args.two_over_one_fit
            || gate_selected,
        "select at least one treatment: --game-tries / --limit-raise / --tails / --fsf / \
         --choice-of-games / --two-over-one-fit / --two-over-one-gate",
    );
    assert!(!args.fsf || args.tails, "--fsf rides --tails; enable both");
    let base = args.seed.unwrap_or_else(rand::random);
    let vul = args.vulnerability;

    // arm 0 = baseline (all off), arm 1 = the selected treatment set.  All
    // the knobs are read only at book-construction time, so each arm bakes
    // its own books; unlike the minor-continuations longer-major knob, none
    // of these is also read at classify time, so there is no per-arm re-set
    // inside the worker below.
    set_knobs(&args, false);
    let baseline = american().against(Family::NATURAL);
    set_knobs(&args, true);
    let treatment = american().against(Family::NATURAL);
    set_knobs(&args, false);
    let stances = [baseline, treatment];

    // Deals are seeded per board (base + index) so any arm of the experiment
    // replays the identical deal set; bidding is pure and parallelizes, the
    // DD solver stays on the main thread below.
    let deals = seeded_deals(base, args.count);
    let contracts: Vec<[_; 2]> = deals
        .par_iter()
        .enumerate()
        .map(|(index, deal)| {
            let dealer = Seat::ALL[index % 4];
            std::array::from_fn(|arm| {
                // All four knobs are construction-time; nothing to re-set per worker.
                let auction = bid_uncontested(&stances[arm], dealer, vul, deal);
                final_contract(&auction, dealer)
            })
        })
        .collect();

    // Only boards whose arms diverge can swing; solve those once and score
    // both brackets from the same solved tables.
    let divergent: Vec<usize> = (0..args.count)
        .filter(|&i| contracts[i][0] != contracts[i][1])
        .collect();
    let solve_deals: Vec<FullDeal> = divergent.iter().map(|&i| deals[i]).collect();
    let tables = Solver::lock().solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

    let mut treatments: Vec<String> = [
        ("game-tries", args.game_tries),
        ("limit-raise", args.limit_raise),
        ("tails", args.tails),
        ("fsf", args.fsf),
        ("choice-of-games", args.choice_of_games),
        ("two-over-one-fit", args.two_over_one_fit),
    ]
    .iter()
    .filter(|(_, on)| *on)
    .map(|&(name, _)| name.to_owned())
    .collect();
    if gate_selected {
        treatments.push(format!("two-over-one-gate={}", args.two_over_one_gate));
    }
    println!(
        "=== major-continuations A/B: {} boards, vulnerability {}, seed {}, treatment [{}] ===",
        args.count,
        vul,
        base,
        treatments.join(" "),
    );
    println!("(opponents silenced — constructive value only)");
    println!(
        "Divergent boards: {} of {} ({:.2}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );

    report_brackets(args.count, &divergent, &tables, &contracts, vul);
}
