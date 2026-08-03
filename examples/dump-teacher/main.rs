//! Teacher dump (AI-bidder M0.4)
//!
//! Bids out boards — random, or every deal in a GIB file via `--deals` — with
//! the *teacher* system (`american()`, or the vendored EPBot 2/1 oracle via
//! `--teacher bba`) and records, at every decision point, a training row of
//! `(features, teacher_softmax)`:
//!
//! - **features** — the restrictive *disclosable-only* v3 vector for the hand
//!   to act ([`features_v3`][pons::bidding::features::features_v3]): 88 floats
//!   of hand summary, context, inferences and vulnerability, with no
//!   card-specific values. This is the extractor the shipped floor uses.
//! - **teacher_softmax** — the teacher's `Logits` at that node, masked to the
//!   *legal* calls and pushed through `softmax`, giving a 38-way distribution
//!   over calls. Matching the full distribution (not just the argmax) is what
//!   makes distillation transfer the teacher's near-misses and mixtures.
//!
//! Output is a flat little-endian `f32` file — one row of `features_len + 38`
//! floats, plus 20 more when `--deals` supplies a GIB file: that board's cached
//! double-dummy table, re-oriented to the acting seat
//! ([`gib::relativized_tricks`]), as a free regression target alongside the
//! policy. Plus a JSON sidecar pinning the feature version, teacher, seed,
//! counts, and `dd_len` (a distilled model is meaningless without its exact
//! feature extractor; they version together), and a sibling `.tags` file of one
//! `u8` per row (`1` = contested-phase decision, `0` = constructive) so the
//! trainer can report held-out agreement split by phase. The Rust/candle trainer
//! reads the `.f32` with a trivial loader.
//!
//! ```text
//! cargo run --release --example dump-teacher -- --boards 100000 --seed 1
//! ```
//!
//! The auction is advanced by the teacher's own legal argmax, so the visited
//! states are the ones the teacher actually reaches. Contested/off-book
//! oversampling beyond what random boards yield is left to M1 data prep; this
//! reports the contested fraction so we know what we have.

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use ddss::TrickCountTable;
use pons::bidding::Stance;
use pons::bidding::card::{american_card, dutch_card};
use pons::bidding::context::{Context, relative};
use pons::bidding::features::{
    Config, FEATURES_LEN_V3, FEATURES_LEN_V4, FEATURES_VERSION_V3, FEATURES_VERSION_V4,
    features_v3, features_v4,
};
use pons::bidding::{Phase, System};
use pons::gib;
use pons::{american, american_instinct, dutch, dutch_instinct};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::io::{BufWriter, Write};
use std::os::raw::c_int;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::oracle::{BbaOracle, ConventionCard, DEFAULT_LIB, SYSTEM_2_OVER_1, load_bbsa};
use common::slam_ish;

/// Number of calls in a `Logits` array (the softmax width).
const SOFTMAX_LEN: usize = 38;

#[derive(Parser)]
#[command(about = "Dump (features, teacher_softmax) training rows from american()")]
struct Args {
    /// Number of boards to bid out
    ///
    /// Without `--deals` this is how many random boards to draw (default 5000).
    /// With `--deals` it caps the window read from the file; omit it to take
    /// every deal from `--skip` to the end, which is what a bare `--deals` has
    /// always meant.
    #[arg(long)]
    boards: Option<usize>,
    /// RNG seed (for reproducibility)
    #[arg(long, default_value_t = 0)]
    seed: u64,
    /// Optional GIB deal file (e.g. sol100000.txt): bid out every deal in it
    /// instead of random boards. Each line is `<PBN, West-first>:<20 hex DD>`;
    /// the cached DD table becomes a 20-float per-row regression target (random
    /// boards have no free DD, so they omit it). Dealer and vulnerability are
    /// still drawn from the seeded RNG per board.
    #[arg(long)]
    deals: Option<String>,
    /// Skip this many deals at the front of `--deals` before reading
    ///
    /// The banks at `/nfs2/jdh8/pons/` are read with `pdd::load_slice`, so a
    /// corpus draw never pulls a 2 GB file into memory whole.  Pair with
    /// `--boards` to bound the draw: `--skip 1000000 --boards 250000`.
    ///
    /// Training draws do **not** advance the never-replay cursor (they may
    /// overlap each other), but they must be recorded — a slice used to train a
    /// net must never later be used to score it.  See
    /// [docs/pdd-bank-ledger.md](../../docs/pdd-bank-ledger.md).
    #[arg(long, default_value_t = 0)]
    skip: u64,
    /// Output path stem; writes `<out>.f32` and `<out>.json`
    #[arg(long, default_value = "target/teacher-data")]
    out: String,
    /// Teacher to distil: `american` (the pure-Rust 2/1 floor, default) or `bba`
    /// (the vendored EPBot 2/1 oracle). `bba` bids through a single-threaded FFI
    /// bot per decision, so that dump is BBA-bidding-bound — tractable, not
    /// instant. Override the `.so` with `BBA_LIB`.
    #[arg(long, default_value = "american")]
    teacher: String,
    /// `--teacher bba` only: a `.bbsa` convention card (e.g.
    /// `vendor/bba/WJ.bbsa`) pinning the teacher's system *and* every one of its
    /// named conventions, engine defaults included. Without it the teacher is
    /// EPBot's 2/1 with whatever the engine defaults to — fine for the 2/1 nets,
    /// but "BBA system 2" alone does not pin `Multi` / `Polish two suiters`, so
    /// a WJ net must name its card. Recorded in the JSON sidecar: a distilled
    /// net is identified by its extractor *and* its teacher configuration.
    #[arg(long)]
    card: Option<String>,
    /// `--teacher bba` only: force one named convention on/off (repeatable),
    /// applied *after* the system's defaults and after `--card`, e.g.
    /// `--conv "Kickback 1430=1"`.  A card pins everything at once, which makes
    /// it useless for isolating a single toggle against a net trained on engine
    /// defaults; this keeps every other convention exactly where it was so the
    /// retrained twin differs from its baseline in one convention only.
    /// Recorded in the JSON sidecar beside `card`.
    #[arg(long = "conv", value_parser = parse_override, value_name = "NAME=0|1")]
    conv: Vec<(CString, c_int)>,
    /// Extract features with **our** kickback recognizer armed
    /// ([`set_kickback`]).  Pair it with `--conv "Kickback 1430=1"`: the `conv`
    /// flag makes the *teacher* play kickback, this one makes our extractor
    /// read the resulting auctions the way serving will.
    ///
    /// It matters because 40 of `features_v3`'s 88 floats come from
    /// `Inferences::read`, and the recognizer is what decides whether a
    /// relocated 4♥ reads as a diamond ask or as natural hearts.  Distilling a
    /// kickback teacher through a kickback-blind extractor trains the net on
    /// readings it will never be served — the same out-of-distribution trap the
    /// `evaluator_v3_exclusion` twin was regenerated to escape.
    #[arg(long)]
    kickback: bool,
    /// Alternate the kickback regime **per board**: even boards get the plain
    /// teacher and a kickback-blind extractor, odd boards get `--conv` and an
    /// armed one.  Requires `--conv`.
    ///
    /// One net then covers both systems instead of a knob choosing between two.
    /// It can, because the regime is *in the features*: forty of the eighty-eight
    /// come from `Inferences::read`, and the ranges a 4♥ carries differ between
    /// "six-plus hearts" and "a keycard count", so the net reads which system it
    /// is in off the auction rather than being told.
    ///
    /// Alternating **inside one dump** rather than concatenating two is what
    /// keeps the trainer's validation split honest: it takes the tail
    /// contiguously, so stitched corpora would validate entirely on whichever
    /// regime landed last, while interleaving by board leaves the tail
    /// board-disjoint *and* mixed.
    #[arg(long)]
    mix_kickback: bool,
    /// Emit the **configured** vector [`features_v4`] instead of `features_v3`
    ///
    /// 368 floats rather than 88: the v3 vector, then both partnerships'
    /// convention cards.  The point is that an A/B arm can then differ by a card
    /// row instead of by a separately trained artifact — see
    /// [docs/ai-bidder/configured-net.md](../../docs/ai-bidder/configured-net.md).
    ///
    /// Opt-in, so every existing v3 corpus recipe keeps its exact meaning.
    #[arg(long)]
    configured: bool,
    /// `--configured` only: which system *we* are declared to play
    #[arg(long, default_value = "american", value_name = "american|dutch")]
    system: String,
    /// `--configured` only: extract from a **bare** context, as v3 dumps do
    ///
    /// `--configured` otherwise builds the prefix-bearing context serving uses,
    /// because a bare one skips `project_authored` entirely — a train/serve skew
    /// measured at 3 of 40 inference floats.  This flag exists to reproduce the
    /// old behaviour for comparison, not to dump a corpus with.
    #[arg(long)]
    bare_context: bool,
    /// `--configured` only: which system the **opponents** are declared to play
    ///
    /// Defaults to ours, mirroring how `BbaOracle` treats undeclared opponents
    /// and how `Context::their_system` models them.  Naming a different one
    /// gives the cross-system cell.
    #[arg(long, value_name = "american|dutch")]
    their_system: Option<String>,
    /// `--configured` only: a table configuration to interleave, `OURS/THEIRS`
    ///
    /// Repeatable; boards rotate through the list so the trainer's contiguous
    /// validation tail stays representative of every cell.  Omit for the six of
    /// `docs/ai-bidder/configured-net.md`; `--system`/`--their-system` are
    /// ignored once any `--cell` is given.
    ///
    /// A mixed table emits *both* asymmetric cells, because a row is written
    /// from the acting seat's view — `--cell a-on/a-off` covers
    /// `(ours=on, theirs=off)` and its mirror in one dump.
    #[arg(long = "cell", value_parser = parse_cell, value_name = "OURS/THEIRS")]
    cells: Vec<(SideConfig, SideConfig)>,
    /// Bid every board in *every* `--cell` instead of rotating one per board
    ///
    /// The corpus unit is a row, and what the net has to learn is that a card
    /// slot changes the target.  Rotating cells across boards leaves that to be
    /// inferred across different deals; replaying one deal through both cells
    /// puts the two rows side by side, identical in all 368 features but the
    /// card slot, with different targets.  For a rare bit — `Kickback 1430`
    /// decides about one board in 600 — that matched pair is the difference
    /// between a learnable signal and noise.
    ///
    /// Costs one full auction per extra cell, so pair it with an enriched draw
    /// rather than turning it on over the uniform bulk.
    #[arg(long)]
    replay: bool,
    /// Keep only deals that pass a raw-hand slam-ish test, `HCP:FIT`
    ///
    /// `--enrich 28:9` keeps a deal only if 28+ combined HCP and a 9+ card fit
    /// in a *non-spade* suit both appear — each for **some** partnership,
    /// measured independently, so the two axes may split across the table.
    /// Both are read off the raw hands ([`slam_ish`][common::slam_ish]),
    /// before the bidder, so acceptance cannot depend on — and therefore
    /// cannot bias — what the bidder does.
    ///
    /// This is how the enriched slice of the mixture corpus is drawn.  Deals
    /// are cheap and bidding is not, so a rejected deal costs a shuffle; see
    /// `probe-kickback-yield` for the accept/lift/cost table the thresholds are
    /// chosen from.  Evaluation must stay on unfiltered deals: an oversampled
    /// slice trains, it never scores.
    #[arg(long, value_parser = parse_enrich, value_name = "HCP:FIT")]
    enrich: Option<(u8, u8)>,
}

/// `HCP:FIT`, the raw-hand acceptance thresholds of an enriched draw.
fn parse_enrich(spec: &str) -> Result<(u8, u8), String> {
    let (points, fit) = spec
        .split_once(':')
        .ok_or_else(|| format!("expected HCP:FIT, got {spec:?}"))?;
    Ok((
        points.parse().map_err(|_| format!("bad HCP {points:?}"))?,
        fit.parse().map_err(|_| format!("bad fit {fit:?}"))?,
    ))
}

/// What one partnership is declared to play: a base system and a convention
///
/// The two axes that exist today.  `SCHEMA` can express roughly 26 of the 222
/// `set_*` knobs, and a corpus may only vary configuration the card can express
/// — otherwise two cells collide into identical vectors with contradictory
/// targets, which is the mixed-net failure this whole design exists to fix.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct SideConfig {
    dutch: bool,
    kickback: bool,
}

impl SideConfig {
    fn label(self) -> String {
        format!(
            "{}-{}",
            if self.dutch { "dutch" } else { "american" },
            if self.kickback { "on" } else { "off" }
        )
    }

    fn system(self) -> &'static str {
        if self.dutch { "dutch" } else { "american" }
    }
}

/// `a-on`, `d-off`, `american-on`, `dutch-off` — a side's declared system
fn parse_side(spec: &str) -> Result<SideConfig, String> {
    let (system, kickback) = spec
        .rsplit_once('-')
        .ok_or("expected SYSTEM-on|off, e.g. `a-on` or `dutch-off`")?;
    let dutch = match system {
        "a" | "american" => false,
        "d" | "dutch" => true,
        other => return Err(format!("system must be a|american|d|dutch, got {other:?}")),
    };
    let kickback = match kickback {
        "on" => true,
        "off" => false,
        other => return Err(format!("kickback must be on|off, got {other:?}")),
    };
    Ok(SideConfig { dutch, kickback })
}

/// `OURS/THEIRS`, one table's seating — e.g. `a-on/a-off` for the mixed table
fn parse_cell(spec: &str) -> Result<(SideConfig, SideConfig), String> {
    let (ours, theirs) = spec
        .split_once('/')
        .ok_or("expected OURS/THEIRS, e.g. `a-on/a-off`")?;
    Ok((parse_side(ours)?, parse_side(theirs)?))
}

/// The six table configurations of `docs/ai-bidder/configured-net.md`
///
/// Eight distinct *ordered* cells, because a row is written from the acting
/// seat's view and a mixed table therefore emits both asymmetric cells at once.
/// 1–3 are what the two gates need; 4–6 exist because kickback alone decides
/// ~0.05% of boards and cannot train the config block on its own, while the
/// base system moves nearly every auction.
const DEFAULT_CELLS: [(SideConfig, SideConfig); 6] = [
    (A_OFF, A_OFF),
    (A_ON, A_ON),
    (A_ON, A_OFF),
    (D_OFF, D_OFF),
    (D_ON, D_ON),
    (A_OFF, D_OFF),
];
const A_OFF: SideConfig = SideConfig {
    dutch: false,
    kickback: false,
};
const A_ON: SideConfig = SideConfig {
    dutch: false,
    kickback: true,
};
const D_OFF: SideConfig = SideConfig {
    dutch: true,
    kickback: false,
};
const D_ON: SideConfig = SideConfig {
    dutch: true,
    kickback: true,
};

/// A generated card as EPBot overrides, so a teacher plays what we disclose
fn to_convention_card(card: &pons::bidding::card::Card) -> anyhow::Result<ConventionCard> {
    let toggles = card
        .rows
        .iter()
        .map(|(name, value)| Ok((CString::new(*name)?, c_int::from(*value != 0))))
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(ConventionCard {
        system: card.system,
        toggles,
    })
}

/// Our card for a named system, rendered off the **live** knob state
///
/// Must be called after the knobs for this cell are set: `american_card()` reads
/// them, which is precisely what keeps the card, the code and the net in sync.
fn card_for(system: &str) -> anyhow::Result<pons::bidding::card::Card> {
    Ok(match system {
        "american" => american_card(),
        "dutch" => dutch_card(),
        other => anyhow::bail!("--system must be american|dutch, got {other:?}"),
    })
}

/// `NAME=0|1`, as `bba-gen` spells it.
fn parse_override(spec: &str) -> Result<(CString, c_int), String> {
    let (name, value) = spec
        .rsplit_once('=')
        .ok_or("expected NAME=0|1 (e.g. \"Kickback 1430=1\")")?;
    let on = match value.trim() {
        "0" => 0,
        "1" => 1,
        other => return Err(format!("value must be 0 or 1, got `{other}`")),
    };
    let name = CString::new(name.trim()).map_err(|_| "name has an interior NUL".to_string())?;
    Ok((name, on))
}

/// The four absolute vulnerabilities, sampled uniformly per board.
const VULS: [AbsoluteVulnerability; 4] = [
    AbsoluteVulnerability::NONE,
    AbsoluteVulnerability::NS,
    AbsoluteVulnerability::EW,
    AbsoluteVulnerability::ALL,
];

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let (feature_version, features_len) = if args.configured {
        (FEATURES_VERSION_V4, FEATURES_LEN_V4)
    } else {
        (FEATURES_VERSION_V3, FEATURES_LEN_V3)
    };
    // DD label only exists when deals come from a GIB file (cached, no solving).
    let dd_len = if args.deals.is_some() { 20 } else { 0 };
    let row_len = features_len + SOFTMAX_LEN + dd_len;
    // Both sides play the same system; the classifier handles whichever seat is
    // to act (vulnerability passed in relative). `american()` routes by phase
    // through a Stance; `bba` is the vendored EPBot 2/1 oracle — a fresh
    // single-threaded FFI bot per decision.
    if args.mix_kickback && args.conv.is_empty() {
        anyhow::bail!("--mix-kickback needs --conv to say what the ON regime is");
    }
    // `--replay` says "bid every board at every table"; with no table list there
    // is nothing to replay across, and silently dumping one copy would look like
    // a matched-pair corpus without being one.
    if args.replay && args.cells.is_empty() {
        anyhow::bail!("--replay needs at least one --cell to replay across");
    }
    // The regimes a board can be dumped under.  One entry normally; two when
    // mixing, selected by board parity so the corpus interleaves.
    let regimes: Vec<bool> = if args.mix_kickback {
        vec![false, true]
    } else {
        vec![args.kickback]
    };
    let build_teacher = |with_conv: bool| -> anyhow::Result<Box<dyn System>> {
        Ok(match args.teacher.as_str() {
            // `--system` selects the teacher, not merely the disclosed card.
            // Letting them drift apart writes rows labelled with a system the
            // teacher was not playing -- the mislabeling that `verify_card`
            // guards against for BBA, one level up and just as invisible.
            "american" if args.system == "dutch" => Box::new(dutch_instinct().against()),
            "american" => Box::new(american_instinct().against()),
            "bba" => {
                let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
                let card = args.card.as_deref().map(load_bbsa).transpose()?;
                let (system, mut toggles) = match card {
                    Some(card) => (card.system, card.toggles),
                    // `--configured --system dutch` means the teacher plays WJ;
                    // without this the corpus would claim WJ over 2/1 bidding.
                    None if args.configured => (card_for(&args.system)?.system, Vec::new()),
                    None => (SYSTEM_2_OVER_1, Vec::new()),
                };
                // Singles win over the card, exactly as `bba-gen` applies them.
                if with_conv {
                    toggles.extend(args.conv.iter().cloned());
                }
                Box::new(BbaOracle::load(&path, system, toggles)?)
            }
            other => anyhow::bail!("--teacher must be american|bba, got {other:?}"),
        })
    };
    // One teacher per regime: mixing needs the plain engine *and* the one
    // playing the convention, because the target has to change with the reading.
    let teachers: Vec<Box<dyn System>> = if args.mix_kickback {
        vec![build_teacher(false)?, build_teacher(true)?]
    } else {
        vec![build_teacher(!args.conv.is_empty())?]
    };

    // ── Configured mode: the table configurations, and one set of artifacts
    // per *ordered* side pair.  Built once here rather than per board: a Stance
    // bakes in rule presence at construction, so each must be built with its own
    // cell's knobs armed.
    let cells: Vec<(SideConfig, SideConfig)> = if !args.cells.is_empty() {
        args.cells.clone()
    } else if args.configured {
        DEFAULT_CELLS.to_vec()
    } else {
        Vec::new()
    };
    if !args.cells.is_empty() && !args.conv.is_empty() {
        anyhow::bail!(
            "--cell and --conv both configure the teacher; a cell's card already \
             pins every row, so a single override would silently disagree with it"
        );
    }
    if !args.cells.is_empty() && !args.configured {
        anyhow::bail!(
            "--cell rotates table configurations, which only the configured (v4) \
             extractor can tell apart; identical v3 features with per-cell targets \
             are the mixed-net corpus this flag exists to prevent — pass \
             --configured or drop --cell"
        );
    }
    let mut sides: Vec<SideConfig> = cells.iter().flat_map(|(a, b)| [*a, *b]).collect();
    sides.sort_by_key(|side| (side.dutch, side.kickback));
    sides.dedup();
    // Per side: its card and the stance that reads its auctions.
    let mut per_side: BTreeMap<String, (pons::bidding::card::Card, Stance)> = BTreeMap::new();
    for side in &sides {
        pons::bidding::instinct::set_kickback(side.kickback);
        let card = card_for(side.system())?;
        let stance = if side.dutch {
            dutch().against()
        } else {
            american().against()
        };
        per_side.insert(side.label(), (card, stance));
    }
    // Per ordered pair: the feature-side config, and the teacher that plays it.
    let mut per_pair: BTreeMap<(String, String), (Config, Box<dyn System>)> = BTreeMap::new();
    for (a, b) in cells.iter().flat_map(|(a, b)| [(*a, *b), (*b, *a)]) {
        let key = (a.label(), b.label());
        if per_pair.contains_key(&key) {
            continue;
        }
        let ours = per_side[&a.label()].0.clone();
        let theirs = per_side[&b.label()].0.clone();
        let config = Config::new(&ours, &theirs);
        pons::bidding::instinct::set_kickback(a.kickback);
        let teacher: Box<dyn System> = match args.teacher.as_str() {
            "american" if a.dutch => Box::new(dutch_instinct().against()),
            "american" => Box::new(american_instinct().against()),
            "bba" => {
                let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
                let ours = to_convention_card(&ours)?;
                let theirs = to_convention_card(&theirs)?;
                Box::new(
                    BbaOracle::load(&path, ours.system, ours.toggles)?.with_opponents(Some(theirs)),
                )
            }
            other => anyhow::bail!("--teacher must be american|bba, got {other:?}"),
        };
        per_pair.insert(key, (config, teacher));
    }
    let mut rng = StdRng::seed_from_u64(args.seed);

    let f32_path = format!("{}.f32", args.out);
    let json_path = format!("{}.json", args.out);
    let tags_path = format!("{}.tags", args.out);
    let mut writer = BufWriter::new(std::fs::File::create(&f32_path)?);
    let mut tags_writer = BufWriter::new(std::fs::File::create(&tags_path)?);

    let mut rows = 0u64;
    let mut contested = 0u64;
    let mut forced_pass = 0u64; // decisions the teacher had no logits for
    let mut rejected = 0u64; // deals `--enrich` turned away before the bidder
    let mut call_hist: BTreeMap<String, u64> = BTreeMap::new();
    let mut row = vec![0f32; row_len];

    // Deal source: every deal + cached DD table in `--deals` (the 100K GIB
    // file), else random boards (no DD). Dealer/vulnerability come from the
    // seeded RNG either way.
    let file_deals: Vec<(FullDeal, TrickCountTable)> = match &args.deals {
        // `boards` unset means "to the end of the file", the historical meaning
        // of a bare `--deals`; set, it bounds the window so a bank draw stays
        // the size it says it is.
        Some(path) => load_deals(path, args.skip, args.boards.unwrap_or(0))?,
        None => Vec::new(),
    };
    let n_boards = if args.deals.is_some() {
        file_deals.len()
    } else {
        args.boards.unwrap_or(5000)
    };
    let mut file_iter = file_deals.iter().copied();

    for board in 0..n_boards {
        // Which regime this board is dumped under.  Constant unless mixing, in
        // which case parity alternates so the corpus interleaves and the
        // trainer's contiguous tail stays representative of both systems.
        let regime = board % regimes.len();
        // Arm the recognizer before a single row of this board is extracted:
        // `features_v3` reads `Inferences`, so this decides what the corpus
        // *says* a relocated ask is — and it must agree with the teacher that
        // produced the target.
        pons::bidding::instinct::set_kickback(regimes[regime]);
        // The card is rendered *after* the knobs are armed, which is what keeps
        // card, code and net in sync: `american_card()` reads the same knobs the
        // rules do.  Rebuilt per board only because `regime` can alternate; it is
        // 140 floats a side, not a hot cost.
        let config = (args.configured && cells.is_empty())
            .then(|| -> anyhow::Result<Config> {
                let ours = card_for(&args.system)?;
                let theirs = match &args.their_system {
                    Some(system) => card_for(system)?,
                    None => ours.clone(),
                };
                Ok(Config::new(&ours, &theirs))
            })
            .transpose()?;
        // The stance that *reads* the auction.  Built after the knobs are armed,
        // for the same reason the card is: rule presence is decided at build.
        let reader: Option<Stance> = (args.configured && cells.is_empty() && !args.bare_context)
            .then(|| -> anyhow::Result<Stance> {
                Ok(match args.system.as_str() {
                    "dutch" => dutch().against(),
                    _ => american().against(),
                })
            })
            .transpose()?;
        let teacher = teachers[regime].as_ref();
        // The table(s) this board is played at.  The dealer's side plays
        // `cell.0`, so a random dealer also randomises which physical side holds
        // which configuration.  Under `--replay` the board is bid at every table
        // instead of one, so the rows that differ are matched pairs.
        let tables: Vec<Option<(SideConfig, SideConfig)>> = if cells.is_empty() {
            vec![None]
        } else if args.replay {
            cells.iter().copied().map(Some).collect()
        } else {
            vec![Some(cells[board % cells.len()])]
        };

        // File deals (with their DD table) when `--deals` is set, else a fresh
        // random board with no table.
        let (deal, table) = match file_iter.next() {
            Some((deal, table)) => (deal, Some(table)),
            None => (full_deal(&mut rng), None),
        };
        let dealer = rng.random_range(0..4usize);
        let vul = VULS[rng.random_range(0..4usize)];

        // The enriched draw's acceptance test.  Raw hands only, and applied
        // before a single call is classified, so a kept deal is bid exactly as
        // an unfiltered one would be — the slice changes *which* deals reach
        // the teacher, never what the teacher says about them.
        if let Some((points, fit)) = args.enrich {
            let (have_points, have_fit) = slam_ish(&deal);
            if have_points < points || have_fit < fit {
                rejected += 1;
                continue;
            }
        }

        for cell in tables {
            let mut auction = Auction::new();
            while !auction.has_ended() {
                let seat = Seat::ALL[(dealer + auction.len()) % 4];
                let hand = deal[seat];
                let rel = relative(vul, seat);
                // Which side is acting.  Sides are seat parity, and the dealer is
                // side 0, so this is what decides whose card is "ours" on this row.
                let acting = cell.map(|(dealers_side, others)| {
                    if auction.len().is_multiple_of(2) {
                        (dealers_side, others)
                    } else {
                        (others, dealers_side)
                    }
                });
                // Arm the recognizer for the *acting* side before anything reads or
                // classifies: in a mixed table the two sides disagree, and the row
                // must be extracted under the configuration that produced it.
                if let Some((ours, _)) = acting {
                    pons::bidding::instinct::set_kickback(ours.kickback);
                }
                let cell_artifacts =
                    acting.map(|(ours, theirs)| &per_pair[&(ours.label(), theirs.label())]);
                let teacher = match cell_artifacts {
                    Some((_, teacher)) => teacher.as_ref(),
                    None => teacher,
                };

                let Some(mut logits) = teacher.classify(hand, rel, &auction) else {
                    forced_pass += 1;
                    auction.push(Call::Pass);
                    continue;
                };

                // Mask illegal calls; the teacher target is over legal calls only.
                for (call, slot) in logits.iter_mut() {
                    if auction.can_push(call).is_err() {
                        *slot = f32::NEG_INFINITY;
                    }
                }
                let Some(softmax) = logits.softmax() else {
                    forced_pass += 1;
                    auction.push(Call::Pass);
                    continue;
                };

                // Record the row: features ++ softmax (++ DD label when present).
                // Prefixed when configured: the same context serving builds, so the
                // authored-projection overlay is applied at dump time too.
                let acting_reader = match acting {
                    Some((ours, _)) if !args.bare_context => Some(&per_side[&ours.label()].1),
                    _ => reader.as_ref(),
                };
                let mut context = match acting_reader {
                    Some(stance) => stance.prefixed_context(rel, &auction),
                    None => Context::new(rel, &auction),
                };
                if let Some(config) = cell_artifacts.map(|(config, _)| config).or(config.as_ref()) {
                    context = context.with_config(config);
                }
                let feats = if args.configured {
                    features_v4(hand, &context)
                } else {
                    features_v3(hand, &context)
                };
                row[..features_len].copy_from_slice(&feats);
                row[features_len..features_len + SOFTMAX_LEN].copy_from_slice(&softmax[..]);
                if let Some(table) = &table {
                    row[features_len + SOFTMAX_LEN..]
                        .copy_from_slice(&gib::relativized_tricks(table, seat));
                }
                for value in &row {
                    writer.write_all(&value.to_le_bytes())?;
                }
                let contested_row = Phase::of(&auction) != Phase::Constructive;
                tags_writer.write_all(&[u8::from(contested_row)])?;
                rows += 1;
                if contested_row {
                    contested += 1;
                }

                // Advance the auction by the teacher's legal argmax.
                let next = argmax_legal(&logits);
                *call_hist.entry(format!("{next}")).or_insert(0) += 1;
                auction.push(next);
            }
        }
    }
    writer.flush()?;
    tags_writer.flush()?;

    let git_sha = git_sha();
    let metadata = serde_json::json!({
        "feature_version": feature_version,
        "features_len": features_len,
        "softmax_len": SOFTMAX_LEN,
        "dd_len": dd_len,
        "row_len": row_len,
        "row_bytes": row_len * 4,
        "dtype": "f32-le",
        "layout": if dd_len > 0 {
            format!("row = [{features_len} features][{SOFTMAX_LEN} teacher_softmax][{dd_len} dd_tricks]")
        } else {
            format!("row = [{features_len} features][{SOFTMAX_LEN} teacher_softmax]")
        },
        "tags": "sibling .tags file: one u8 per row, 1 = contested phase, 0 = constructive",
        "teacher": &args.teacher,
        "card": args.card.as_deref().unwrap_or("engine defaults"),
        // A distilled net is identified by its extractor *and* its teacher
        // configuration, so a forced toggle belongs beside the card.
        "conv": args
            .conv
            .iter()
            .map(|(name, on)| format!("{}={on}", name.to_string_lossy()))
            .collect::<Vec<_>>(),
        // The extractor's own regime, not the teacher's — a net is identified
        // by both.  Under `--mix-kickback` the corpus carries both, alternating
        // by board, and the net learns to tell them apart from the readings.
        "our_kickback": args.kickback,
        "mix_kickback": args.mix_kickback,
        "configured": args.configured,
        "replay": args.replay,
        "enrich": args.enrich.map(|(points, fit)| format!("{points}:{fit}")),
        "enrich_rejected": rejected,
        "cells": cells
            .iter()
            .map(|(a, b)| format!("{}/{}", a.label(), b.label()))
            .collect::<Vec<_>>(),
        "context": if args.configured && !args.bare_context { "prefixed" } else { "bare" },
        "our_system": args.configured.then(|| args.system.clone()),
        "their_system": args
            .configured
            .then(|| args.their_system.clone().unwrap_or_else(|| args.system.clone())),
        "skip": args.skip,
        "deals": args.deals.as_deref().unwrap_or("random"),
        "git_sha": git_sha,
        "seed": args.seed,
        "boards": n_boards,
        "rows": rows,
        "contested_rows": contested,
        "forced_pass_decisions": forced_pass,
    });
    std::fs::write(&json_path, format!("{metadata:#}\n"))?;

    let pct = |n: u64| {
        if rows == 0 {
            0.0
        } else {
            100.0 * n as f64 / rows as f64
        }
    };
    eprintln!(
        "teacher-dump: {rows} rows (feature v{feature_version}, {features_len} features) \
         from {n_boards} boards → {f32_path} ({:.1} MB), \
         {contested} contested ({:.0}%), {forced_pass} forced passes.",
        (rows as usize * row_len * 4) as f64 / 1e6,
        pct(contested),
    );
    if let Some((points, fit)) = args.enrich {
        let kept = n_boards as u64 - rejected;
        eprintln!(
            "teacher-dump: --enrich {points}:{fit} kept {kept} of {n_boards} deals \
             ({:.2}%); rejected deals never reached the bidder.",
            100.0 * kept as f64 / n_boards.max(1) as f64,
        );
    }
    eprintln!("top advancing calls:");
    let mut hist: Vec<(String, u64)> = call_hist.into_iter().collect();
    hist.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    for (call, count) in hist.into_iter().take(12) {
        eprintln!("  {call:>4}  {count:>8}  ({:.1}%)", pct(count));
    }
    Ok(())
}

/// The highest-logit finite (hence legal, after masking) call, defaulting to a
/// pass so the auction always terminates.
fn argmax_legal(logits: &pons::bidding::array::Logits) -> Call {
    logits
        .iter()
        .filter(|(_, l)| l.is_finite())
        .max_by(|a, b| a.1.partial_cmp(b.1).expect("logits are never NaN"))
        .map_or(Call::Pass, |(call, _)| call)
}

/// Load deals and their cached double-dummy tables from a solution file in
/// either format (GIB text like `sol100000.txt`, or binary `.pdd`).
///
/// `skip`/`count` read a window rather than the whole file: `24.pdd` is 2 GB and
/// the standing rule is to draw a corpus from the banks without pulling one into
/// memory entire.  `count` of 0 means "the rest of the file", which is what a
/// bare `--deals` on a small GIB file has always meant.
fn load_deals(
    path: &str,
    skip: u64,
    count: usize,
) -> std::io::Result<Vec<(FullDeal, TrickCountTable)>> {
    let deals = if skip == 0 && count == 0 {
        pons::pdd::load(path)?
    } else if count == 0 {
        // "The rest of the file": `load_slice` caps by rows, so ask for more
        // rows than any bank can hold (2^40 rows ≈ 57 TB of `.pdd`).  Without
        // this arm a `--skip` with no `--boards` passed the 0 through and
        // silently loaded nothing — while printing "asked all".
        pons::pdd::load_slice(path, skip, 1 << 40)?
    } else {
        pons::pdd::load_slice(path, skip, count)?
    };
    eprintln!(
        "teacher-dump: loaded {} deals from {path} (skip {skip}, asked {})",
        deals.len(),
        if count == 0 {
            "all".to_owned()
        } else {
            count.to_string()
        },
    );
    Ok(deals)
}

/// Best-effort current commit, for the metadata sidecar; `"unknown"` on failure.
fn git_sha() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map_or_else(|| "unknown".to_string(), |s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use contract_bridge::{Seat, Suit};

    #[test]
    fn enrich_thresholds_parse_as_hcp_then_fit() {
        assert_eq!(parse_enrich("28:9"), Ok((28, 9)));
        assert!(parse_enrich("28").is_err(), "no separator");
        assert!(parse_enrich("28:x").is_err(), "fit is not a number");
    }

    /// The raw-hand test must ignore spades: a spade ask is 4NT under either
    /// card, so a spade fit carries no configuration signal and accepting on
    /// one would spend the whole enriched slice on deals that cannot diverge.
    #[test]
    fn the_raw_hand_test_ignores_spades() {
        // North holds all thirteen spades and all his side's points; the best
        // *non*-spade fit at the table is E-W's nine (diamonds, and clubs).
        let deal: FullDeal = "N:AKQJT98765432... .98765.T987.T987 \
                              .AKQJT.AKQJ.AKQJ .432.65432.65432"
            .parse()
            .expect("a PBN deal, North first");
        assert_eq!(deal[Seat::North][Suit::Spades].len(), 13, "a 13-card fit");
        assert_eq!(
            slam_ish(&deal),
            (40, 9),
            "all forty points to N-S, and the fit is E-W's nine — not the spade thirteen",
        );
    }
}
