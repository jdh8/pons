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
//! **Its baseline is not the shipped system.**  `set_knobs(.., false)` drives
//! *every* knob here to its off state, including the ones that ship on
//! (`set_two_over_one_fit`, `set_opener_third`, `set_two_over_one_force`, …), so
//! arm 0 is a stripped system and arm 1 is that system plus the treatments.  For
//! a knob whose value depends on machinery the stripping removes, this harness
//! can read a flat zero where the real routing shows a win — the two-over-one
//! slam-entry floor read 0 divergent in 2M boards here and +0.003/+0.004
//! IMPs/board against BBA.  Prefer `bba-gen` whenever the treatment interacts
//! with a shipped-on knob; this harness is for isolating a treatment *against
//! its own absence*, which is a different question.
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
    TwoOverOneGate, set_fourth_suit_forcing, set_game_backstop, set_limit_raise_acceptance,
    set_major_choice_of_games, set_major_game_tries, set_major_rebid_tails, set_opener_third,
    set_second_suit_agreement, set_two_over_one_fit, set_two_over_one_gate,
    set_xyz_invite_judgment,
};
use pons::bidding::instinct::{set_two_over_one_force, set_two_over_one_slam_strength};
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

    /// Treatment: *restore* the retired 2/1 game backstop, so uncovered
    /// game-forcing continuations answer from its three rules instead of the
    /// floor (`set_game_backstop`, shipped off)
    #[arg(long, default_value_t = false)]
    game_backstop: bool,

    /// Treatment: *drop* the floor's 2/1 game force, letting it pass below game
    /// in an established two-over-one (`set_two_over_one_force`, shipped on)
    #[arg(long, default_value_t = false)]
    no_two_over_one_force: bool,

    /// Treatment: *drop* opener's third call after trump is agreed at
    /// `1M–2r–R–3M`, so the node falls to the floor (`set_opener_third`,
    /// shipped on).  Re-audit candidate #2 — measures positive but strands
    /// every slam at the node; not shipped.
    #[arg(long, default_value_t = false)]
    no_opener_third: bool,

    /// Treatment: *drop* opener's third call after responder agrees the second
    /// suit at `1M–2r–2x–3x` (`set_second_suit_agreement`, shipped on).
    /// Constructive book re-audit candidate #1.
    #[arg(long, default_value_t = false)]
    no_second_suit_agreement: bool,

    /// Treatment: *drop* opener's judgment of the XYZ invitations that stop
    /// below game (`set_xyz_invite_judgment`, shipped on).  Constructive book
    /// re-audit candidate #3 — the most-reached one.
    #[arg(long, default_value_t = false)]
    no_xyz_invite_judgment: bool,

    /// Treatment: *drop* the two-over-one strength floor on the slam-entry gate
    /// (`set_two_over_one_slam_strength`, shipped on).  Without it the alerted
    /// `GAME_FORCE` response reads as zero and the floor never asks keycards
    /// through a 2/1.
    #[arg(long, default_value_t = false)]
    no_two_over_one_slam_strength: bool,
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
    // These two ship the other way round (backstop retired, force on), so the
    // treatment *restores* the old behaviour rather than adding a new one.
    set_game_backstop(treatment && args.game_backstop);
    set_two_over_one_force(!(treatment && args.no_two_over_one_force));
    set_opener_third(!(treatment && args.no_opener_third));
    set_second_suit_agreement(!(treatment && args.no_second_suit_agreement));
    set_xyz_invite_judgment(!(treatment && args.no_xyz_invite_judgment));
    set_two_over_one_slam_strength(!(treatment && args.no_two_over_one_slam_strength));
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
            || args.game_backstop
            || args.no_two_over_one_force
            || args.no_opener_third
            || args.no_second_suit_agreement
            || args.no_xyz_invite_judgment
            || args.no_two_over_one_slam_strength
            || gate_selected,
        "select at least one treatment: --game-tries / --limit-raise / --tails / --fsf / \
         --choice-of-games / --two-over-one-fit / --two-over-one-gate / --game-backstop / \
         --no-opener-third / --no-second-suit-agreement / --no-xyz-invite-judgment / \
         --no-two-over-one-force / --no-two-over-one-slam-strength",
    );
    assert!(!args.fsf || args.tails, "--fsf rides --tails; enable both");
    let base = args.seed.unwrap_or_else(rand::random);
    let vul = args.vulnerability;

    // arm 0 = baseline (all off), arm 1 = the selected treatment set.  Most of
    // the knobs are read only at book-construction time, so each arm bakes its
    // own books; `two_over_one_force` is the exception — a classify-time read,
    // re-set per arm inside the worker below.
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
                // Every knob but these two is construction-time and already baked
                // into `stances`.  `two_over_one_force` and
                // `two_over_one_slam_strength` are read at *classify* time, so
                // they must be re-set inside the worker or these threads would
                // only ever see the default (cf. ab-minor-continuations'
                // longer-major).
                set_two_over_one_force(!(arm == 1 && args.no_two_over_one_force));
                set_two_over_one_slam_strength(!(arm == 1 && args.no_two_over_one_slam_strength));
                let auction = bid_uncontested(&stances[arm], dealer, vul, deal);
                set_two_over_one_force(true);
                set_two_over_one_slam_strength(true);
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
        ("game-backstop", args.game_backstop),
        ("no-two-over-one-force", args.no_two_over_one_force),
        ("no-opener-third", args.no_opener_third),
        (
            "no-two-over-one-slam-strength",
            args.no_two_over_one_slam_strength,
        ),
        ("no-second-suit-agreement", args.no_second_suit_agreement),
        ("no-xyz-invite-judgment", args.no_xyz_invite_judgment),
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
