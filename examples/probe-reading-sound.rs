//! Does a reading of a **hidden** seat actually contain that seat's hand?
//!
//! [`Inferences::read`]'s module doc promises soundness over tightness: *"a hand
//! that actually made these calls always falls within every shown range"*. The
//! same doc then says the meanings encoded are [`american`]'s. Both halves are
//! load-bearing, and together they leave a hole: at our own two seats the
//! guarantee is *derived* — partner really did bid our system — but LHO and RHO
//! are bidding something else, so for them it is merely *assumed*.
//!
//! Every consumer of a reading rests on that assumption. The evaluator net is
//! handed each hidden seat's box as features; the sampler rejects worlds outside
//! it. A box that excludes the truth is not a loose prior, it is a wrong one,
//! and no amount of extra columns describing it helps.
//!
//! So: bid a mixed table with BBA at the opponent seats, and at every decision
//! node of *our* seats test two predicates per hidden seat: [`Inferences::admits`]
//! (the strict *table* reading every in-crate consumer sits on) and
//! `announced_union(who).contains(true_hand)` (the lenient *disclosure* overlay,
//! the recorded baseline).  Partner exclusions are additionally bucketed by the
//! auction prefix through partner's call — the repair worklist of
//! `docs/reading-drift-handoff.md`.
//!
//! - **partner ≈ 0%** is the self-check. The membership test and the harness
//!   are only trustworthy if our own side comes out clean; a material rate there
//!   is a bug in the reading, not a finding about opponents.
//! - **LHO / RHO** is the measurement. Whatever it is above partner's rate is
//!   the price of reading a foreign system with our own meanings.
//!
//! Note [`features::shape_of`] already falls back when a reading admits *zero*
//! shapes, so total exclusion is survivable; a box that is merely wrong is not
//! caught anywhere, which is what this counts.
//!
//! No double-dummy, no solver. EPBot is not thread-safe here, so this is serial.
//!
//! ```sh
//! cargo run --release --example probe-reading-sound -- -c 2000
//! ```
//!
//! [`american`]: pons::american

use clap::Parser;
use contract_bridge::auction::Call;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use pons::american;
use pons::bidding::context::relative;
use pons::bidding::{Bidder, Partnership, Relative};
use std::collections::HashMap;
use std::ffi::{CString, c_int};

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::oracle::{BbaOracle, DEFAULT_LIB, EpbotCard, SYSTEM_2_OVER_1, bid_out, load_bbsa};
use common::{auction_key, deviant_floor, seat_to_act, seeded_deals};

/// The hidden seats, in report order, with how far back each one last acted
/// The opponent seats say "(them)" rather than "(BBA)": `--their-floor` seats a
/// perturbed pons book there instead.
const HIDDEN: [(&str, Relative, usize); 3] = [
    ("LHO (them)", Relative::Lho, 3),
    ("partner (ours)", Relative::Partner, 2),
    ("RHO (them)", Relative::Rho, 1),
];

// ponytail: the worklist buckets partner only — every partner offender maps to
// a node we author, so the whole list is actionable; opponent drift stays a
// tracked scalar.  Bucket LHO/RHO too if the defensive book ever gets a pass.

#[derive(Parser)]
struct Args {
    /// Deals to bid
    #[arg(short, long, default_value = "2000")]
    count: usize,

    /// Seed base; random when omitted
    #[arg(short, long)]
    seed: Option<u64>,

    /// Offending keys to list
    #[arg(long, default_value = "20")]
    top: usize,

    /// Do not disclose our card to BBA (the `bba-gen --disclose off` arm)
    #[arg(long)]
    no_disclose: bool,

    /// BBA system index to seat at the opponent seats (default 2/1)
    #[arg(long)]
    system: Option<c_int>,

    /// A vendored `.bbsa` card for the opponents; its `System type` header
    /// must agree with `--system` when both are given
    #[arg(long, value_name = "FILE.bbsa")]
    their_card: Option<String>,

    /// Single convention override on top of the opponents' card, repeatable
    #[arg(long = "their-conv", value_name = "NAME=0|1")]
    their_conv: Vec<String>,

    /// Seat a **pons** book as the opponents instead of EPBot — the deviation
    /// panel's B/C axes.  Same names as `bba-gen --their-floor`; the exclusion
    /// rate then measures deviation from our *own* card.
    #[arg(long, value_name = "NAME")]
    their_floor: Option<String>,

    /// Antisymmetric strength dial for `--their-floor` (see `bba-gen`)
    #[arg(long, default_value_t = 0, value_name = "POINTS")]
    their_dial: u8,

    /// `--their-floor` overcalls on a good four-card suit
    #[arg(long)]
    their_overcall_four_card: bool,

    /// `--their-floor` opens 1NT off-shape
    #[arg(long)]
    their_offshape_1nt: bool,

    /// `--their-floor` opens undisciplined weak twos
    #[arg(long)]
    their_wild_weak_two: bool,

    /// Override how much of the authored book our seats read.  Unset uses the
    /// engine default (`all` since Phase 2).
    #[arg(long, value_enum)]
    ns_reading_scope: Option<common::ReadingScopeArg>,

    /// Our seats read each made call's strength **ceilings**, not just its floors
    /// (`ReadingProfile::strength_ceilings` — Phase 1 of
    /// `docs/authored-reading-handoff.md`).  Tightening a box can only *lose*
    /// soundness, so this sweep is the ceilings' soundness gate.  **Engine
    /// default ON since 2026-08-16**; pass `false` for the pre-ceilings
    /// baseline.  Unset = the engine default.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    ns_strength_ceilings: Option<bool>,

    /// Our seats close `hcp` against `points` through the shape upgrade
    /// (`ReadingProfile::upgrade_closure` — C2 of `docs/dnf-migration.md`).  It
    /// only ever *tightens* a box, so this sweep is its soundness gate too.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    ns_upgrade_closure: Option<bool>,
}

/// Readings taken, and how many excluded the truth — on both predicates
///
/// `bad` tests [`Inferences::admits`], the *table* reading every in-crate
/// consumer (sampler, nets, inference-aware floor) actually sits on, and
/// the one the `readings_admit_the_bidder` sweep enforces.  `bad_announced`
/// tests `announced_union().contains`, the lenient *disclosure* overlay — the
/// predicate of the recorded 8.2/3.3/8.3% baseline (`docs/deviation-panel.md`).
/// The delta between them is itself a finding: an announce-vs-reading gap.
#[derive(Default, Clone, Copy)]
struct Cell {
    readings: u64,
    bad: u64,
    bad_announced: u64,
}

impl Cell {
    fn add(&mut self, admits: bool, announced: bool) {
        self.readings += 1;
        self.bad += u64::from(!admits);
        self.bad_announced += u64::from(!announced);
    }

    fn pct(self, bad: u64) -> f64 {
        if self.readings == 0 {
            return 0.0;
        }
        100.0 * bad as f64 / self.readings as f64
    }
}

/// Parse a `NAME=0|1` convention override for `--their-conv`
fn parse_override(spec: &str) -> anyhow::Result<(CString, c_int)> {
    let (name, value) = spec
        .rsplit_once('=')
        .ok_or_else(|| anyhow::anyhow!("expected NAME=0|1, got {spec:?}"))?;
    let on = match value.trim() {
        "0" => 0,
        "1" => 1,
        other => anyhow::bail!("value must be 0 or 1, got `{other}`"),
    };
    Ok((CString::new(name.trim())?, on))
}

/// Our generated card, so BBA reads our calls the way `bba-gen` has it read them
fn our_card() -> EpbotCard {
    let card =
        pons::bidding::card::american_card(&pons::bidding::agreements::Agreements::default());
    EpbotCard {
        system: card.system,
        toggles: card
            .rows
            .iter()
            .map(|(name, value)| {
                (
                    std::ffi::CString::new(*name).expect("a card row name has no NUL"),
                    *value,
                )
            })
            .collect(),
    }
}

/// Every decision node of one bid-out deal, charged into `seats` and `keys`
fn census(
    partnership: &Partnership,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
    auction: &[Call],
    seats: &mut [Cell; 3],
    keys: &mut HashMap<String, Cell>,
) {
    for cut in 1..auction.len() {
        let seat = seat_to_act(dealer, cut);
        // Only our own seats: a reading taken at an EW seat is one no consumer
        // in the crate ever takes.
        if !matches!(seat, Seat::North | Seat::South) {
            continue;
        }
        let read = partnership.infer(relative(vul, seat), &auction[..cut]);
        for (slot, (_, who, back)) in HIDDEN.into_iter().enumerate() {
            let Some(last) = cut.checked_sub(back) else {
                continue; // the seat has not called yet
            };
            // `back` calls ago is `back` seats counter-clockwise from the actor.
            let hand = deal[Seat::ALL[(seat as usize + 4 - back) % 4]];
            let admits = read.admits(who, hand);
            let announced = read.announced_union(who).contains(hand);
            seats[slot].add(admits, announced);
            if who == Relative::Partner {
                keys.entry(auction_key(&auction[..=last]))
                    .or_default()
                    .add(admits, announced);
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let base = args.seed.unwrap_or_else(rand::random);
    let vul = AbsoluteVulnerability::NONE;
    let mut agreements = pons::bidding::agreements::Agreements::default();
    if let Some(scope) = args.ns_reading_scope {
        agreements.decision.reading.scope = scope.into();
    }
    if let Some(v) = args.ns_strength_ceilings {
        agreements.decision.reading.strength_ceilings = v;
    }
    if let Some(v) = args.ns_upgrade_closure {
        agreements.decision.reading.upgrade_closure = v;
    }
    let partnership = american(&agreements).bind();

    // The opponents: a perturbed pons book (deviation panel axes B/C) or, by
    // default, EPBot on whichever card `--system`/`--their-card` names (axis A).
    let their_floor = match &args.their_floor {
        Some(name) => Some(deviant_floor(
            name,
            // Our seat is bare `american()`, so that is the card they face.
            &pons::bidding::card::american_card(&agreements),
            &agreements,
            args.their_dial,
            args.their_overcall_four_card,
            args.their_offshape_1nt,
            args.their_wild_weak_two,
        )?),
        None => None,
    };
    let (system, mut their_conv) = match &args.their_card {
        Some(file) => {
            let card = load_bbsa(file)?;
            if let Some(system) = args.system {
                anyhow::ensure!(
                    card.system == system,
                    "`{file}` is system {}, but --system says {system}",
                    card.system,
                );
            }
            (card.system, card.toggles)
        }
        None => (args.system.unwrap_or(SYSTEM_2_OVER_1), Vec::new()),
    };
    for spec in &args.their_conv {
        their_conv.push(parse_override(spec)?);
    }

    let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
    let bba = match &their_floor {
        Some(_) => None,
        None => Some(
            BbaOracle::load(&path, system, their_conv)
                .map_err(|error| {
                    anyhow::anyhow!(
                        "could not load EPBot native lib at `{path}`: {error}\n\
                         Fetch it with `git submodule update --init vendor/bba`, or set BBA_LIB."
                    )
                })?
                .with_opponents((!args.no_disclose).then(our_card)),
        ),
    };
    let opponent: &dyn Bidder = match (&their_floor, &bba) {
        (Some(book), _) => book,
        (None, Some(oracle)) => oracle,
        (None, None) => unreachable!("one of the two is always built"),
    };

    let mut seats = [Cell::default(); 3];
    let mut keys: HashMap<String, Cell> = HashMap::new();
    for (board, deal) in seeded_deals(base, args.count).iter().enumerate() {
        let dealer = Seat::ALL[board % 4];
        let auction = bid_out(&partnership, opponent, true, dealer, vul, deal);
        census(
            &partnership,
            dealer,
            vul,
            deal,
            &auction,
            &mut seats,
            &mut keys,
        );
    }

    println!("boards {}  seed {base}", args.count);
    println!(
        "disclosure {}  reading-scope {:?}\n",
        if args.no_disclose { "off" } else { "generated" },
        agreements.decision.reading.scope,
    );
    println!(
        "{:<16} {:>10} {:>10} {:>9} {:>10} {:>9}",
        "seat", "readings", "admits✗", "%", "announce✗", "%"
    );
    for (slot, (label, _, _)) in HIDDEN.into_iter().enumerate() {
        let cell = seats[slot];
        println!(
            "{label:<16} {:>10} {:>10} {:>8.3}% {:>10} {:>8.3}%",
            cell.readings,
            cell.bad,
            cell.pct(cell.bad),
            cell.bad_announced,
            cell.pct(cell.bad_announced),
        );
    }

    let mut keys: Vec<_> = keys.into_iter().collect();
    let n = keys.len();
    keys.sort_unstable_by(|a, b| b.1.bad.cmp(&a.1.bad).then_with(|| a.0.cmp(&b.0)));
    println!(
        "\npartner worklist: top {} nodes by admits-excluded readings (of {n} distinct)\n",
        args.top.min(n),
    );
    println!(
        "{:>10} {:>10} {:>9} {:>10}  node (auction through partner's call)",
        "admits✗", "readings", "%", "announce✗"
    );
    for (key, cell) in keys.iter().take(args.top) {
        println!(
            "{:>10} {:>10} {:>8.2}% {:>10}  {key}",
            cell.bad,
            cell.readings,
            cell.pct(cell.bad),
            cell.bad_announced,
        );
    }

    // The same nodes ranked by *rate*.  Ranking by count alone hid the
    // side-blind 1NT-overcall strip — ~90% wrong across a few dozen readings,
    // so it sat below `1♦` at 1.5% of 4,056 — and a node that is nearly always
    // wrong is the interesting one however rarely it is reached.  The floor
    // keeps single-digit nodes, whose rate is noise, out of the top slots.
    const RATE_FLOOR: u64 = 10;
    keys.retain(|(_, cell)| cell.readings >= RATE_FLOOR);
    let rated = keys.len();
    keys.sort_unstable_by(|a, b| {
        (b.1.pct(b.1.bad))
            .total_cmp(&a.1.pct(a.1.bad))
            .then_with(|| b.1.bad.cmp(&a.1.bad))
            .then_with(|| a.0.cmp(&b.0))
    });
    println!(
        "\npartner worklist: top {} nodes by excluded *rate* (of {rated} with ≥ {RATE_FLOOR} readings)\n",
        args.top.min(rated),
    );
    println!(
        "{:>9} {:>10} {:>10} {:>10}  node (auction through partner's call)",
        "%", "admits✗", "readings", "announce✗"
    );
    for (key, cell) in keys.iter().take(args.top) {
        println!(
            "{:>8.2}% {:>10} {:>10} {:>10}  {key}",
            cell.pct(cell.bad),
            cell.bad,
            cell.readings,
            cell.bad_announced,
        );
    }
    Ok(())
}
