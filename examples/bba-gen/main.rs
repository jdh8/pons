//! AI-bidder **Side-track S.1** — the external eval anchor, *generation* half.
//!
//! Bids a duplicate A/B match of our deterministic [`american`] floor against
//! **BBA's own 2/1 Game Force card**, driven natively through EPBot's C ABI
//! (`libEPBot.so`, no Wine).  Each board is bid twice — our pair North/South at
//! table A, East/West at table B — and the two auctions are written out as a
//! `Dump` of [`Board`]s.  **No double-dummy, no scoring**: the EPBot bidding is
//! single-threaded by design (a fresh native bot per decision, FFI thread-safety
//! not assumed), so this half is CPU-light and latency-bound — run it on one
//! thread alongside a saturating self-play sweep, and hand the boards to
//! [`bba-score`](../bba-score/main.rs) for the parallel DD scoring.  Caching the
//! boards also lets a tuning loop re-score them many ways (plain vs PD) without
//! paying the slow FFI bidding again.
//!
//! To use every core anyway, parallelize across **processes** (not threads — the
//! FFI is thread-unsafe): `scripts/bba-gen-parallel.sh` runs one shard per core
//! with a distinct `--seed`, and `bba-score` merges the shard files back into one
//! match.  Each process gets its own address space, `.so`, and thread-locals, so
//! there is no shared state to race on.
//!
//! ```text
//! # pipe the whole match through in one line (today's one-shot behaviour)
//! cargo run --release --features serde --example bba-gen -- --count 1000 \
//!   | cargo run --release --features serde --example bba-score
//! # or cache the boards, then score them several ways
//! cargo run --release --features serde --example bba-gen -- --count 6000 \
//!   --isolate-defense -o boards.json
//! cargo run --release --features serde --example bba-score -- boards.json --score pd
//! ```
//!
//! EPBot ships in the `vendor/bba` git submodule; `git submodule update --init
//! vendor/bba` resolves the default library path, or point `BBA_LIB` elsewhere.
//! `--our-system <index>` swaps our side for a *second* EPBot card (BBA-vs-BBA).

use clap::Parser;
use contract_bridge::auction::Call;
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Bid, Hand, Seat, Strain, Suit};
use pons::bidding::Bidder;
use pons::bidding::agreements::Agreements;
use pons::bidding::american::DoubleShape;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::ffi::{CString, c_int};

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::oracle::{BbaOracle, DEFAULT_LIB, EpbotCard, SYSTEM_2_OVER_1, bid_out, load_bbsa};
use common::{
    Board, Dump, NtDefenseArg, ReadingScopeArg, blinded, deviant_floor, floor_card, hand_hcp,
    seat_floor, seat_floor_vs, seat_to_act,
};

/// Bid our 2/1 floor against BBA's 2/1 and write the boards (the generation half
/// of the A/B duplicate match; `bba-score` scores them)
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "200")]
    count: usize,

    /// Write the bid boards as JSON here; default is stdout (pipe into
    /// `bba-score`, or save to re-score many ways without re-bidding)
    #[arg(short, long)]
    output: Option<String>,

    /// Vulnerability the boards are bid at: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// EPBot system index for *their* side (0 = 2/1 Game Force, 2 = WJ)
    #[arg(short, long, default_value_t = SYSTEM_2_OVER_1)]
    system: c_int,

    /// Drive *our* side with EPBot at this system index too (BBA-vs-BBA
    /// experiment); unset = our authored `american` floor
    #[arg(long)]
    our_system: Option<c_int>,

    /// Which of our authored systems to seat: `american` (default), `dutch`
    /// (the wide-1♣ champion candidate), the deterministic `american-instinct` /
    /// `dutch-instinct` pre-swap baselines, `bba-constructive` (`american` with
    /// the BBA net flooring the constructive book too), `neural-v3` (the
    /// restrictive disclosable distilled floor; requires the `neural-floor`
    /// feature), or the explicit `american-v6` / `dutch-v6` aliases of the
    /// shipped defaults. Ignored when `--our-system` selects
    /// an EPBot card.
    #[arg(long, default_value = "american")]
    our_floor: String,

    /// Tell our net the card the **opponents actually hold**, instead of our own
    ///
    /// Phase 2a of docs/declarative-rows.md.  Off (the default) our partnership is
    /// built by `american()`/`dutch()`, whose `Config::symmetric` declares that
    /// the opposition plays our card — against EPBot, simply false.  On, the
    /// opponents' half of the config is read back off their own bot
    /// (`BbaOracle::card`), so this is a **bidding change**: it moves the net's
    /// inputs on every board and wants a full A/B, not a spot check.
    ///
    /// Books are untouched either way — this is the floor channel alone.
    /// Requires `--our-floor american|dutch` (the two with a net to declare to).
    /// With `--their-floor` the opponents are a pons book, so their card is the
    /// one that name generates — the american-vs-dutch mixed table; the
    /// `--their-dial`/`--their-*` perturbations are refused, since no card row
    /// expresses them.
    #[arg(long)]
    declare_opponents: bool,

    /// Read the opponents' calls off **their** books, not ours — the reading
    /// half of a declared opponent (`Partnership::with_opponents`, rows Phase 2b)
    ///
    /// The reader's twin of `--declare-opponents`, and deliberately separate:
    /// that one moves the net's card inputs, this one moves the deterministic
    /// reading of their alerted calls and passes.  Requires `--their-floor` —
    /// a pons book is the only opponent we have books *for*, so this cannot be
    /// run against EPBot.
    ///
    /// Asymmetric on purpose: only our side reads better, so a paired A/B
    /// attributes its IMPs to our decisions alone.
    #[arg(long)]
    declare_their_book: bool,

    /// Let both pons partnerships read the other's books and pinned profile
    ///
    /// The reciprocal form of `--declare-their-book`, for an honest mixed-card
    /// self-play table. Requires `--their-floor`; external bots have no pons
    /// book to attach.
    #[arg(long, requires = "their_floor", conflicts_with = "declare_their_book")]
    declare_books_mutually: bool,

    /// Seat a **pons** book as the opponents instead of EPBot — the deviation
    /// panel's B/C axes (docs/deviation-panel.md).  Takes the same names as
    /// `--our-floor`; the `--their-dial` / `--their-overcall-four-card` /
    /// `--their-offshape-1nt` / `--their-wild-weak-two` knobs then perturb
    /// *that* book only, simulating a natural bidder who deviates from our
    /// card.  Mutually exclusive with `--isolate-*` and `--our-system`.
    #[arg(long, value_name = "NAME")]
    their_floor: Option<String>,

    /// Antisymmetric strength dial for `--their-floor`: their openings and
    /// overcalls are this many points lighter, their responses and advances
    /// the same amount heavier (pair calibration preserved).  0 = off.
    #[arg(long, default_value_t = 0, value_name = "POINTS")]
    their_dial: u8,

    /// `--their-floor` overcalls on a good four-card suit
    #[arg(long)]
    their_overcall_four_card: bool,

    /// `--their-floor` opens 1NT off-shape
    #[arg(long)]
    their_offshape_1nt: bool,

    /// `--their-floor` opens undisciplined (five-card, wide-range) weak twos
    #[arg(long)]
    their_wild_weak_two: bool,

    /// The opponent seat's **own** `--ns-*` flags, as one quoted string, e.g.
    /// `--their-ns "--no-ns-garbage-stayman"`
    ///
    /// Parsed as a second `bba-gen` command line and applied with the same
    /// `arm_knobs` that arms ours, so the whole `--ns-*` surface is available
    /// to the opposing seat without a mirrored flag per knob.  Requires
    /// `--their-floor` (only a pons book has knobs), and the opponent's card is
    /// read under these, so `--declare-opponents` discloses what they really
    /// play.
    #[arg(long, value_name = "FLAGS", allow_hyphen_values = true)]
    their_ns: Option<String>,

    /// Declare the opponents as playing *this* command line rather than what
    /// `--their-ns` actually gives them — the wrong-declared arm
    ///
    /// Separates "reading them correctly helps" from "reading them as anything
    /// other than ourselves helps": their book is unchanged, only the card our
    /// net is handed moves.  Requires `--declare-opponents`.
    #[arg(long, value_name = "FLAGS", allow_hyphen_values = true)]
    declare_as: Option<String>,

    /// Blank the two opponent seats' readings on **our** side — the `blind`
    /// arm of the deviation panel.  The paired `seen − blind` score is what
    /// reading their calls is worth against one perturbed opponent.
    #[arg(long)]
    ns_blind_opponent_reading: bool,

    /// Force a named BBA convention on/off on *our* side (repeatable), e.g.
    /// `--our-conv "Rubensohl after 1m=1"`.  Only meaningful with `--our-system`.
    #[arg(long = "our-conv", value_parser = parse_override, value_name = "NAME=0|1")]
    our_conv: Vec<(CString, c_int)>,

    /// Force a named BBA convention on/off on *their* side (repeatable), e.g.
    /// `--their-conv "Rubensohl after 1m=0"`.  Pair with `--our-conv` to isolate
    /// one toggle in a BBA-vs-BBA A/B.
    #[arg(long = "their-conv", value_parser = parse_override, value_name = "NAME=0|1")]
    their_conv: Vec<(CString, c_int)>,

    /// Load a full `.bbsa` convention card for *our* side (implies
    /// `--our-system` from the card's `System type` header); `--our-conv`
    /// singles apply on top.  E.g. BEN's declared card
    /// `vendor/ben/BEN-21GF.bbsa`.
    #[arg(long = "our-card", value_name = "FILE.bbsa")]
    our_card: Option<String>,

    /// Load a full `.bbsa` convention card for *their* side; its `System type`
    /// must match `--system`, and `--their-conv` singles apply on top.  Use
    /// `--their-card vendor/ben/BEN-21GF.bbsa` so the exploit guard plays
    /// BEN's declared system rather than stock BBA defaults.
    #[arg(long = "their-card", value_name = "FILE.bbsa")]
    their_card: Option<String>,

    /// Declare *our* system to the BBA seats, so they read our calls correctly:
    /// `off`, `generated`, or a `FILE.bbsa` path.
    ///
    /// `generated` builds the card from the live system — `--our-floor`'s system
    /// read through every `--ns-*` knob this run set — so an arm that flips a
    /// knob discloses what it actually plays (see [`pons::bidding::card`]).  A
    /// path discloses that file verbatim instead; `--disclose-conv` singles
    /// apply on top of either.
    ///
    /// Not to be confused with `--our-card`, which configures a *separate* BBA
    /// oracle to play our side in a BBA-vs-BBA A/B.  This one leaves our side as
    /// pons and changes only what BBA believes pons plays, via the per-seat
    /// convention setters on the seats we occupy.
    ///
    /// `generated` by default: a BBA that takes us for a BBA misreads our
    /// conventions, and a gap measured that way is partly its confusion rather
    /// than our bidding.  It costs IMPs — BBA defends better when it knows what
    /// our calls mean — which is the price of a fair fight, not a regression.
    /// Pass `off` to reproduce the blind series measured before 2026-07-28.
    #[arg(
        long = "disclose",
        value_name = "off|generated|FILE.bbsa",
        default_value = "generated"
    )]
    disclose: String,

    /// Override one disclosed convention row, e.g.
    /// `--disclose-conv "1N-3M splinter=0"`; repeatable, applied after
    /// `--disclose`.
    ///
    /// The escape hatch for rows the generator holds constant — a treatment with
    /// no knob, or a row whose BBA semantics we have not pinned down.
    #[arg(long = "disclose-conv", value_parser = parse_override, value_name = "NAME=0|1")]
    disclose_conv: Vec<(CString, c_int)>,

    /// Only keep deals with a balanced 15-17 HCP hand somewhere (a 1NT-opener
    /// candidate), to raise the yield of 1NT boards.  Cheap shape gate, no
    /// bidding; `--count` then means *kept* boards.
    #[arg(long, default_value_t = false)]
    filter_1nt: bool,

    /// `--filter-1nt`, and additionally require that candidate's **LHO** to
    /// hold a Landy-shaped hand (5-4+ majors, 8+ HCP) — the seat that would
    /// overcall `2♣` over our 1NT.
    ///
    /// LHO, not RHO: bidding moves clockwise, so the hand that acts *over* a
    /// 1NT opening is the opener's left-hand opponent.  Until 2026-08-19 this
    /// paired on `rho()`, which is the seat that acts one call *before* the
    /// opener — a Landy-shaped hand there opens the bidding itself, so our
    /// candidate never opened 1NT and, when it did (dealer = the candidate),
    /// the Landy hand was in the balancing seat.  Measured on 2,000 accepted
    /// boards at seed 424242: the direct `1NT (2♣)` lane was **0.00%** of
    /// accepted boards under `rho()` — *below* the 0.50% of a plain
    /// `--filter-1nt` run — against **17.8%** under `lho()`.
    ///
    /// The contested-`(2♣)` lane fires on ~0.1% of unfiltered boards,
    /// which is below the resolution of any band sweep (docs/one-notrump-competitive.md).
    /// Pairing the two raw-hand tests raises the yield by orders of magnitude
    /// for pure scan cost — still no bidding, no FFI, no solver.  Deliberately
    /// **looser** than either `convention_points` band (9-18 / 8-19) and
    /// uncapped above: the overcaller here is BBA, whose band we do not
    /// control, and a gate tighter than the trigger would bias the accepted
    /// set rather than merely shrink it.
    ///
    /// Still enrichment, not isolation: the balanced seat may never open 1NT
    /// and the Landy-shaped seat may never overcall, so the headline stays
    /// IMPs per *accepted* deal, rescaled by trigger density.  Changing the
    /// acceptance predicate changes the accepted set, so arms under this flag
    /// pair only with each other — never with a `--filter-1nt` run at the same
    /// seed.
    #[arg(long, default_value_t = false)]
    filter_landy: bool,

    /// `--filter-1nt`, and additionally require that candidate's **LHO** to
    /// hold a preempt-shaped hand (a seven-card suit, no more than 12 HCP) —
    /// the seat that would overcall our 1NT at the three level.
    ///
    /// The twin of [`Args::filter_landy`] for the `1NT (3x)` lane
    /// (docs/one-notrump-competitive.md §N3), whose four buckets together are
    /// 410 boards out of 204,800 per vulnerability in the anchor — 0.2%, below
    /// the resolution of any per-suit read.  Measured on 2,000 accepted boards
    /// at seed 424242: **14.8%** of them reach `1NT (3x)` against 0.60% under
    /// a plain `--filter-1nt` run, for 179 scanned deals per accepted board.  BBA's three-level overcalls of a
    /// 1NT opening are natural **seven-card** preempts at `hcp 4–10`
    /// (docs/ai-bidder/bba-1nt-counter-defense.md), and the cap here is
    /// deliberately above that band for the same reason `--filter-landy` is
    /// looser than `convention_points`: this gates a *scan*, not a call, and
    /// must not reject a hand the opponents' own band would preempt on.
    ///
    /// Enrichment, not isolation: the balanced seat may never open 1NT and the
    /// preempt-shaped seat may never overcall, so the headline stays IMPs per
    /// *accepted* deal.  Changing the acceptance predicate changes the accepted
    /// set, so arms under this flag pair **only with each other** — never with
    /// a `--filter-1nt` or `--filter-landy` run at the same seed.
    #[arg(long, default_value_t = false)]
    filter_preempt: bool,

    /// Disable our Unusual-vs-Unusual structure over 1NT (2NT) — BBA overcalls our
    /// 1NT with a both-minors 2NT (Multi-Landy), so this is the live test.  On by
    /// default (it ships): the responder structure + the encircling chase.
    #[arg(long, default_value_t = false)]
    no_uvu: bool,

    /// Responder's penalty-double HCP floor for UvU
    #[arg(long, default_value_t = 9)]
    uvu_x_floor: u8,

    /// Responder's INV+ cue-bid points floor for UvU
    #[arg(long, default_value_t = 8)]
    uvu_cue_floor: u8,

    /// Deal seed for reproducible boards (pairs a `--no-uvu` on/off comparison so
    /// the boards UvU does not touch are identical and cancel); unset = random
    #[arg(long)]
    seed: Option<u64>,

    /// Redefine responder's double of a `(2♦)` overcall of our 1NT as a diamond
    /// penalty double, `LEN:SUITHCP:HCP` (e.g. `5:4:9` — five-plus diamonds, four
    /// HCP in the suit, nine overall).  Unset keeps the shipped cooperative
    /// double, whose 2-3 diamond gate names a suit BBA's Multi `2♦` never holds.
    #[arg(long)]
    ns_2d_double: Option<String>,

    /// Override the derived reading of their `2♣` overcall of our 1NT:
    /// `true`/bare = Landy (both majors, engage the counter-defense),
    /// `false` = natural (keep the systems-on rebase).  Unset, the reading is
    /// **derived from their declaration** (`their_2c_landy`): explicit
    /// `--their-conv`/`--their-card` Landy-family rows are honored at face
    /// value, and with no declaration the 2/1 reference defaults to Landy —
    /// its measured behavior (551-board census) — because its own card
    /// mis-declares (21GF.bbsa: Cappelletti=1, Landy=0, Multi-Landy=0).
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    their_2c_landy: Option<bool>,

    /// Override the derived reading of their `2♦` overcall of our 1NT:
    /// `true`/bare = Multi (one unknown six-card major, engage the N4
    /// Multi table), `false` = natural diamonds (the Transfer-Lebensohl
    /// leg).  Unset, the reading is **derived from their declaration**
    /// (`their_2d_multi`): an explicit `Multi-Landy` row is honored at face
    /// value, and with no declaration the reading stays *undeclared/natural*.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    their_2d_multi: Option<bool>,

    /// Read the opponents' disclosed Multi `2♦` as the exact union `6+♥ |
    /// 6+♠`, suppressing the natural-diamond and first pass-or-correct
    /// readings.  Unset tracks the shipped engine default (on); pass `false`
    /// for the pre-reader arm.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    ns_their_multi_read: Option<bool>,

    /// Read their Multi *advance* as the whole pass-or-correct ladder
    /// (`2♥/2♠/3♥/3♠/4♣/4♦/4♥/4♠`), suppressing the phantom suit each names,
    /// and claim `♥3+ & ♠3+` on the jump rungs (default off; opt-in A/B).
    /// Requires `--ns-their-multi-read`.
    #[arg(long)]
    ns_their_multi_advance_read: bool,

    /// Read our values double of their declared Multi at its authored
    /// `hcp(6..)` floor instead of the generic `DoubleStyle` 8+
    /// (default off; opt-in A/B).
    #[arg(long)]
    ns_their_multi_double_read: bool,

    /// Add the GF minor cues to the Landy counter (`2♥` = 5+ clubs, `2♠` = 5+
    /// diamonds, both game-forcing) — the third arm of the Landy A/B.  Does
    /// nothing without --defense-2c-landy.
    #[arg(long, default_value_t = false)]
    defense_2c_landy_cues: bool,

    /// N1c: re-rung the Landy counter's minors around a club transfer (`2NT` =
    /// weak 6+ clubs, `3♣`/`3♦` = INV 6+, `4♣`/`4♦` = slam try).  Implies
    /// --defense-2c-landy-cues; does nothing without --their-2c-landy.
    /// **Engine default ON since 2026-08-14** (the stack's win|win); pass
    /// `false` for the pre-stack arm.  Unset = the engine default.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    defense_2c_landy_transfer: Option<bool>,

    /// N1d: raise the Landy cues' floor to `points(10..)`, returning the 8-9
    /// five-card-minor hands to the values double.  Implies
    /// --defense-2c-landy-transfer; does nothing without --their-2c-landy.
    /// **Engine default ON since 2026-08-14**; pass `false` for the pre-N1d
    /// arm.  Unset = the engine default.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    defense_2c_landy_cue_floor: Option<bool>,

    /// N1h: price the Landy counter's minor rungs a point lower — the `2♥`/`2♠`
    /// cues to `points(9..)`, the `3♣`/`3♦` invitational six-carders to
    /// `points(7..=8)`.  Implies --defense-2c-landy-transfer; does nothing
    /// without --their-2c-landy.  Opt-in, pending its A/B.
    #[arg(long, default_value_t = false)]
    defense_2c_landy_low_minors: bool,

    /// N1i: grade the Landy counter's minor rungs on `hcp` instead of
    /// `points` — cue `hcp(9..)`, `3♣`/`3♦` INV `hcp(7..=8)`, weak `2♦` and
    /// the `2NT` club transfer `hcp(..=6)`.  Implies
    /// --defense-2c-landy-transfer and supersedes --defense-2c-landy-low-minors;
    /// does nothing without --their-2c-landy.  Opt-in, pending its A/B.
    #[arg(long, default_value_t = false)]
    defense_2c_landy_hcp_rungs: bool,

    /// N1e: answer a Landy cue in notrump on doubleton support (the notrump
    /// rungs become "both majors stopped, or ≤2-card support"; raises and asks
    /// promise 3+).  Implies --defense-2c-landy-transfer; does nothing without
    /// --their-2c-landy.  **Engine default ON since 2026-08-14**; pass `false`
    /// for the pre-N1e arm.  Unset = the engine default.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    defense_2c_landy_fit_answers: Option<bool>,

    /// N1f: author the Landy counter's interfered tails (their X of a cue/ask
    /// answered as if undoubled via the systems-on rebase, their raise over a
    /// cue answered by the compressed ladder, the doubled transfer still
    /// completed).  Implies --defense-2c-landy-transfer; does nothing without
    /// --their-2c-landy.  **Engine default ON since 2026-08-14**; pass `false`
    /// for the pre-N1f arm.  Unset = the engine default.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    defense_2c_landy_competition: Option<bool>,

    /// N1j: play the BBA-ladder Landy counter — responder's whole table over
    /// their `2♣` re-shaped to the anchor's own counter structure (notrump
    /// ladder + wide 6+ minor transfers, `2NT`→♣ / `3♣`→♦), keeping the
    /// values X verbatim and adding the GF both-minors takeout/splinter
    /// family on `2♥`/`2♠` and `3♥`/`3♠`.  The N1b–N1i stack knobs are inert
    /// under it.  Does nothing without --their-2c-landy.  **Engine default ON
    /// since 2026-08-15** (non-inferiority ship, zero CI-clear negatives);
    /// pass `false` for the pre-N1j stack arm.  Unset = the engine default.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    defense_2c_landy_bba: Option<bool>,

    /// N1j's third arm: cap the BBA ladder's weak natural `2♦` at `hcp(..=6)`
    /// (the N1i `2♦ → Pass` lead, isolated — the dropped 7-9 point hands
    /// pass).  Read only under --defense-2c-landy-bba.  **Engine default ON
    /// since 2026-08-15** (standard gate: plain wash | PD win, 0 foreign);
    /// pass `false` for the uncapped ladder.  Unset = the engine default.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    defense_2c_landy_weak_2d_cap: Option<bool>,

    /// Suppress our *own* 1NT opening (those 15-17 balanced hands open a minor),
    /// so every 1NT in the match is BBA's and our pair is purely the defender.
    #[arg(long, default_value_t = false)]
    no_our_1nt: bool,

    /// Turn OFF decoding fallback-authored conventions (contested transfers, Leaping
    /// Michaels, the Lebensohl cue) in the floor's projection, leaving only
    /// exact-node calls; on by default (the A/B off-switch — measured plain +0.0006,
    /// PD +0.0014 IMPs/board, both CIs exclude 0).
    #[arg(long, default_value_t = false)]
    no_ns_fallback_projection: bool,

    /// Turn OFF the envelope-union reading for our side (`ReadingProfile::envelope_union`,
    /// crate default ON since chop F2b — docs/dnf-migration.md): fall back to
    /// hulling every disjunction to its bounding box, the legacy reading and
    /// the F2b A/B's off arm.
    #[arg(long, default_value_t = false)]
    no_ns_envelope_union: bool,

    /// Also give the strength gauges membership teeth for our side
    /// (`ReadingProfile::gauge_membership`, crate default off): samplers reject hands
    /// outside the raw-HCP / support-points bands.  Measured WASH on sd-lead;
    /// independent kill-switch. It reads envelope-union boxes, so it wants that
    /// reading live — which is the default, i.e. pass no `--no-ns-envelope-union`.
    #[arg(long, default_value_t = false)]
    ns_gauge_membership: bool,

    /// Narrow our side's read suit lengths by `Σ len = 13` (`ReadingProfile::sum_closure`,
    /// crate default off): a both-majors box stops claiming 13 spades.  Exact
    /// and membership-inert, so only hulls and term counts move.  DNF-ledger
    /// chop C1 (docs/dnf-migration.md).
    #[arg(long, default_value_t = false)]
    ns_sum_closure: bool,

    /// Turn OFF the learned accountant floor for our side
    /// (`InstinctProfile::accountant_floor`, crate default on): the converted game and
    /// slam gates fall back to authored point arithmetic instead of asking the
    /// `evaluator_v2` net.  The isolation arm for any reading change suspected
    /// of putting the evaluator out of distribution (chop F1's mechanism) —
    /// run it on BOTH arms of the pair, so the delta that survives is the one
    /// the *authored* gates see.
    #[arg(long, default_value_t = false)]
    no_ns_accountant: bool,

    /// Collar the accountant net for our side instead of letting it replace the
    /// point arithmetic (`InstinctProfile::net_collar`, crate default off).  The
    /// shipped wiring masks the authored gate off and hands the net the whole
    /// criterion; with this on the arithmetic decides and the net rules on it in
    /// one direction only — accelerating at game (reaching at most 2 points
    /// below the threshold) and vetoing at slam, the split `break_even`'s own
    /// key implies.  Pair it with `--no-ns-accountant` on neither arm: this is a
    /// treatment of the net's licence, not of the net's presence.
    #[arg(long, default_value_t = false)]
    ns_net_collar: bool,

    /// Turn OFF the contested game-level pricing for our side
    /// (`InstinctProfile::competitive_accountant`, crate default on — shipped
    /// 2026-08-12, plain +0.0088/+0.0140 by vul with PD a wash).  On, when they
    /// buy a game or higher and a strain of ours is already named, the floor's
    /// judgement logits are repriced against the score table — the candidate
    /// bid, the penalty double and Pass, all three from the same forward pass
    /// the constructive gates read.  Demotions only; off restores the unpriced
    /// judgement logits, the `off` arm of `scripts/ab-competitive-accountant.sh`.
    #[arg(long, default_value_t = false)]
    no_ns_competitive_accountant: bool,

    /// Turn OFF the v3 calls-tail evaluator for our side
    /// (`DecisionProfile::eval_auction`, crate default on — shipped 2026-07-27,
    /// `win | win`, plain +0.018/+0.028 by vul).  The accountant game/slam gates
    /// read trick estimates from the v3 artifact, whose input is the hull
    /// vector plus the last four call identities — the 0.038-NLL win of the
    /// auction-input ablation, served. Only honoured in the envelope-union reading
    /// regime the twin was trained on (`--no-ns-envelope-union` makes it inert); off,
    /// the hull-only `evaluator_v2_dnf` serves as before.
    #[arg(long, default_value_t = false)]
    no_ns_eval_auction: bool,

    /// Serve the v4 shape-reading evaluator for our side
    /// (`DecisionProfile::eval_shape`, crate default off).  Replaces each hidden
    /// seat's four suit-length `{min, max}` pairs with its shape distribution
    /// — `E[len]`, `sd[len]` per suit plus a log-mass column — over the
    /// 560-shape lattice.  NLL-par with the v3 hull vector by construction;
    /// what it buys is invariance — pairing it with `--ns-sum-closure` recovers
    /// +0.018 plain / +0.028 PD of that lossless re-hull's cost, but on its own
    /// it measured −0.0037 plain / −0.0034 PD, so it stays off and is kept as
    /// the reference invariant reading.  Supersedes
    /// `--no-ns-eval-auction` (v4 carries the calls tail) and is only honoured
    /// in the envelope-union reading regime it was trained on.
    #[arg(long, default_value_t = false)]
    ns_eval_shape: bool,

    /// Split disclosure from projection for our side
    /// (`ReadingProfile::announced`, crate default off).  A call decided
    /// by the evaluator net projects ⊤ and always will — a net accepts hands no
    /// box contains — so it reads as nothing.  On, every rule also contributes
    /// an *agreement* overlay, which the nets' feature vectors consume while the
    /// sampler keeps the sound projection.  Today one site takes the split: the
    /// floor's 4NT RKCB ask, the one converted milestone that is not a final
    /// contract (`instinct::RKCB_ASK_ANNOUNCE`).  Reading-only — no criterion
    /// moves, so a divergence here is the nets reacting to a tighter box.
    #[arg(long, default_value_t = false)]
    ns_announced_reading: bool,

    /// Read the opponents' *declared* Landy `2♣` over our `1NT` as both majors
    /// (4-4+), and their advances as preferences, instead of the natural
    /// walk's 5+ clubs and 8+.  The read-side half of the `two_clubs_landy`
    /// disclosure — a no-op unless that declaration is in force
    /// (`--their-2c-landy` or the derived default).  **Engine default ON
    /// since the 2026-08-14 N1g ship**; the pre-ship arm is
    /// `--ns-their-landy-read false`.  Unset = the engine default.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    ns_their_landy_read: Option<bool>,

    /// Alert the forced `3♣` completion of a Lebensohl `2NT` relay, so the
    /// reading suppresses the natural walk's club holding on a puppet (shared
    /// by plain/transfer Lebensohl, advance-sohl, and the N1c club transfer).
    /// **Engine default ON since the 2026-08-14 ship** (pooled 3 seeds:
    /// vul plain/PD CI-clear, NV positive); the pre-ship arm is
    /// `--ns-completion-alerts false`.  Unset = the engine default.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    ns_completion_alerts: Option<bool>,

    /// How much of the authored book our projection decodes
    /// (`ReadingProfile::scope`, crate default `all` since 2026-08-16;
    /// `alerted` is the pre-Phase-2 arm and the frozen nets' view).
    ///
    /// `alerted` decodes a call when its rule alerts it; `none` is the
    /// pre-alert arm, where a strength-showing artificial reads as a natural
    /// suit; `all` also projects **unalerted** authored calls, whose rules —
    /// `len(♦, 5..) & points(10..)` — otherwise contribute nothing while the
    /// natural walk's guess from auction shape stands unchecked beside them.
    /// Under `all` the rule's own union is intersected into the reading
    /// *without* suppressing the walk.  Reading-only for the authored layer,
    /// but not bid-inert: the nets eat inference features, so expect
    /// divergence.  See `docs/reading-drift-handoff.md`.
    ///
    /// One flag because the two bools this replaced had four cells for three
    /// partnerships — the natural half short-circuited the alerted one.
    #[arg(long, value_enum)]
    ns_reading_scope: Option<ReadingScopeArg>,

    /// Blank every reading our nets see (`DecisionProfile::blind_inference`, crate
    /// default off — diagnostic, never ship it on).  The reading program's
    /// negative control: each generator of readings tightens a box and measures
    /// the *derivative*, which keeps landing in the noise.  This deletes the
    /// boxes outright, so the arm's loss is a **ceiling** on what any generator
    /// can be worth.  Only the nets' feature vectors go blind — the sampler's
    /// containment test, `admits`, and the opening-lead sampling read the
    /// `Inferences` directly and are untouched.
    #[arg(long, default_value_t = false)]
    ns_blind_inference: bool,

    /// Read our side's made bids with sibling-gate exclusion
    /// (`ReadingProfile::bid_exclusion`, crate default on).  The made-bid
    /// selection is argmax over `weight/100 + eval`, so a bid made through one
    /// rule proves the hand
    /// outside every sibling rule on another call whose weight strictly beats
    /// it.  The book's non-Pass `hcp(0..)` catch-alls then read what the
    /// heavier tiers denied instead of nothing at all (Jacoby `1M - 2NT - 4M`
    /// stops reading opener as `points 16..21` off the natural walk).
    /// Phase 4 of docs/authored-reading-handoff.md.  **Engine default ON since
    /// 2026-08-17** (A/B wash on both scorers, three seeds); pass `false` for
    /// the pre-fold arm.  Unset = the engine default.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    ns_bid_exclusion: Option<bool>,

    /// Probe our partnership's behavior over this many self-play boards at startup
    /// and read with the probed boxes on (`Partnership::probe` +
    /// `ReadingProfile::probed`, crate default off).  Fixed probe seed, so every
    /// shard of an arm carries the identical probed map.  0 = off.
    #[arg(long, default_value_t = 0)]
    ns_probe: usize,

    /// Serve the probed boxes through the vacuous-scoped fold instead of the
    /// full one (`ReadingProfile::probed_vacuous`, crate default off): own-side
    /// calls only, and only onto axes the symbolic reading left fully open —
    /// the coverage slice of the probed reading, without the tightening that
    /// refuted the full fold.  Requires `--ns-probe`.
    #[arg(long, default_value_t = false, requires = "ns_probe")]
    ns_probe_vacuous: bool,

    /// Close our side's read `hcp` against `points` through the shape upgrade
    /// (`ReadingProfile::upgrade_closure`, crate default **on** since
    /// 2026-08-16): balanced hands never upgrade, so a balanced box reads
    /// `points == hcp` instead of carrying the scale's global slack.  DNF-ledger
    /// chop C2; the pre-ship arm is `--ns-upgrade-closure false`.  Unset = the
    /// engine default.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    ns_upgrade_closure: Option<bool>,

    /// Our side reads every made call for its strength **ceilings**, not just its
    /// floors (`ReadingProfile::strength_ceilings`, crate default on): `points`,
    /// `hcp` and `support_points` each project their two-sided band instead of
    /// `floor..=37`, so a weak sign-off stops reading as *unlimited* — the N2
    /// `2NT` relay gates on `points(..=8)` and opener blasts `3NT` opposite it
    /// anyway, because nothing downstream ever saw the eight.  Reading, not
    /// disclosure: the alerts and the `.bbsa` cards are untouched, and under the
    /// shipped `ReadingScope::Alerted` the ceilings reach **alerted** calls only.
    ///
    /// It tightens how we read *their* calls too, and there our meanings are an
    /// approximation of a system they are not playing: `probe-reading-sound`
    /// measured LHO/RHO exclusions up 0.41/0.37pp against our own side's rate
    /// going *down* (40k boards, seed 20260816).  Phase 1 of
    /// docs/authored-reading-handoff.md; `--ns-legacy-view` is its nets-side
    /// hedge.  **Engine default ON since 2026-08-16**; pass `false` for the
    /// pre-ceilings arm.  Unset = the engine default.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    ns_strength_ceilings: Option<bool>,

    /// The instinct floor forces to game off any three-level suit bid over our
    /// strong notrump, which cannot tell a game force from a Lebensohl sign-off.
    /// On, the force also requires partner's read `points` ceiling to reach ten.
    /// **Engine default ON since 2026-08-16**; pass `false` for the
    /// shape-only-force arm.  Unset = the engine default.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    ns_forcing_ceiling_read: Option<bool>,

    /// Value the floor's notrump milestones on raw HCP — our own hand plus
    /// partner's crisp `hcp` gauge — instead of the length-upgraded
    /// `point_count`, whose long-suit bonus is a ruffing value worth nothing in
    /// notrump.  Rides `hcp_floor()`, so it only bites where the ceilings
    /// populate that gauge.  **Engine default OFF**; unset = the engine default.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    ns_nt_hcp: Option<bool>,

    /// In the fit-sum game gate, take partner's shown strength from the
    /// dedicated `support_points` gauge instead of the length-scale `points`.
    /// The slot min is at most `shown_floor()`, so this gate gets *more*
    /// conservative.  **Engine default OFF**; unset = the engine default.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    ns_fit_sum_support: Option<bool>,

    /// Open the strong 2NT (20-21) on the wide-minor shape `{M 2..=4, m 2..=6}`
    /// for our side (`ReadingProfile::two_notrump_wide`, crate default off): drops the
    /// 5M(332) balanced hands (they open one-of-a-major) and adds the wide
    /// minors (5m422/6m322).  DNF-ledger chop G0 (docs/dnf-migration.md).
    #[arg(long, default_value_t = false)]
    ns_two_nt_wide: bool,

    /// Cleanly isolate our DEFENSE to BBA's 1NT.  Keep only boards where BBA (E/W)
    /// opens 1NT and our pair (N/S) defends, and bid table B as an ALL-BBA
    /// reference — same BBA opener and responses, only the defender differs (ours
    /// vs BBA).  The swing is then pure defense quality.  `--count` means kept
    /// (we-defend) boards.
    #[arg(long, default_value_t = false)]
    isolate_defense: bool,

    /// Cleanly isolate our 1NT OPENING (mirror of `--isolate-defense`).  Keep only
    /// boards where our pair (N/S) opens 1NT, and hold the DEFENDER constant across
    /// both arms so the swing is pure opening quality (ours vs BBA).  `bba` = BBA
    /// defends both arms (table B is the all-BBA reference); `pons` = our defense
    /// both arms (table A all-pons, table B BBA-opens / we-defend); `off` = disabled.
    /// `--count` means kept (we-open) boards.
    #[arg(long, default_value = "off", value_name = "off|bba|pons")]
    isolate_opening: String,

    /// Restore the legacy fifths gauge for our 1NT opening (default = plain HCP
    /// 15-17).
    #[arg(long, default_value_t = false)]
    nt_fifths: bool,

    /// Open a Multi `2♦` — **Dutch only** (`--our-floor dutch`), replacing all
    /// three natural weak twos with one artificial 4-10 six-card-major `2♦!`
    /// (default off; see `opening.multi_two_diamonds`).  Inert under
    /// `--our-floor american`, which never compiles the package.
    #[arg(long, default_value_t = false)]
    ns_multi_2d: bool,

    /// Play BBA's verbatim Multi `2♦` book rather than the champion structure
    /// — read only under `--ns-multi-2d` (champion is the default; see
    /// `opening.multi_two_diamonds_champion`).  Anchor runs pin this.
    #[arg(long, default_value_t = false)]
    no_ns_multi_2d_champion: bool,

    /// Disable our continuations after the opponents contest our 2♣ Stayman
    /// (`1NT - 2♣ (X)` / `1NT - 2♣ (2♦/2♥/2♠)`); on by default. Off-switch for the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_comp_over_stayman: bool,

    /// How a flat 4-3-3-3 cue-Staymans when the opponents overcall our 1NT:
    /// `suppress` (default — never cue, the A/B winner), `allow` (the old
    /// baseline), or `suppress-stopper` (suppress only with a stopper).
    #[arg(
        long,
        default_value = "suppress",
        value_name = "suppress|allow|suppress-stopper"
    )]
    ns_competitive_4333: String,

    /// Responder's `3♠` stopper ask after their Multi corrects to spades:
    /// `off` (shipped default), `search` (responder searches after opener names
    /// a side suit), or `place` (opener places the game immediately).
    #[arg(long, default_value = "off", value_name = "off|search|place")]
    ns_multi_stopper_ask: String,

    /// Minimum suit length responder may escape their declared `(2♦)` Multi on
    /// with no HCP floor: `6` (shipped default), `5` (refuted as a default,
    /// opt-in), or `off` for the pre-2026-08-22 floored lane.  Also authors
    /// the escape's interfered tail (`1NT (2♦) 2M (X/2♠/2NT/3x)`).
    #[arg(long, default_value = "6", value_name = "off|6|5")]
    ns_multi_weak_escape: String,

    /// Author opener's balancing double at `1NT (2♦) - (2M)` — five cards in
    /// the major they named, penalty; else pass (default off; opt-in A/B).
    #[arg(long)]
    ns_multi_balance: bool,

    /// Fall back to the v7 subtree against their declared `(2♦)` Multi
    ///
    /// Turns *off* the Kokish–Kraft whole-table counter, shipped default-on
    /// 2026-08-25 (`competition.multi_kokish_kraft`), which is what gives
    /// responder `X` invitational-plus with no shape promise, a neutral pass
    /// with its own delayed takeout double, floorless `2NT`→♣ / `3♣`→♦
    /// transfers, `3♠` both minors GF, a penalty repeated double, and the
    /// uncontested direct `4M` slam-try tier.  v7 instead has the weak `2NT`
    /// relay, `3♣` Stayman, a `hcp 6+` double and one takeout second double.
    /// This is the control arm of `scripts/ab-2d-multi-kk.sh`.
    #[arg(long, default_value_t = false)]
    no_ns_multi_kokish_kraft: bool,

    /// The `points` floor of the `4m` slam try above a completed Kokish–Kraft
    /// minor transfer: a `points` floor (default `15`), or `off` — `13` is
    /// `landy_bba_transfer_rebid`'s own rung, `15` the narrow arm
    ///
    /// Also authors opener's answer (`4NT` RKCB on a maximum, else `5m`) and,
    /// on the same switch, the shortness `4m` when they compete over the
    /// completion (§N4-KK residues 3 and 6, `docs/minor-transfer-slam.md`).
    /// Needs the Kokish–Kraft counter and their declared `(2♦)` Multi to do
    /// anything.
    #[arg(long, default_value = "15", value_name = "off|13|15")]
    ns_multi_minor_slam_try: String,

    /// Give the Kokish–Kraft doubler a **natural bid of the other major** once
    /// their pass-or-correct resolves theirs (§N4-KK residue 4)
    ///
    /// `2♠` over their `(2♥)`, `3♥` over their `(2♠)`, on four-plus of the
    /// other major at weight 100 — below every existing rung, so it fires only
    /// on the hands that pass today.  Withheld from `X (2♥) - (2♠)`, where
    /// opener's pass has already denied four hearts.  Opener answers with game
    /// from the top of the range, the invitational raise where there is room,
    /// else a pass.  Needs the Kokish–Kraft counter and their declared `(2♦)`
    /// Multi to do anything.
    #[arg(long, default_value_t = false)]
    ns_multi_doubler_major: bool,

    /// The `4m` slam try above a completed **Puppet** minor transfer
    /// (`1NT - 2♠`→♣, `1NT - 2NT`→♦): a `points` floor (default `13`), or `off`
    ///
    /// The shipped constructive twin of
    /// `--ns-multi-minor-slam-try`.  Authors the rung in all four Puppet seats
    /// plus opener's answer (`4NT` RKCB on `size_ask_accept_floor`, else `5m`).
    /// The European arm is an opponent model and never carries it.
    #[arg(long, default_value = "13", value_name = "off|POINTS")]
    ns_minor_transfer_slam_try: String,

    /// Restore the six-card-only `4♦` slam-try gate after Puppet `2NT - 3♦`
    #[arg(long, default_value_t = false)]
    no_ns_minor_transfer_slam_fit: bool,

    /// Leave opener's N1j Landy `4m` slam try to the floor instead of using
    /// the shipped authored answer (`1NT (2♣) 2NT - 3♣ - 4♣ -`)
    ///
    /// The rung itself has shipped since N1; this restores the former
    /// floor-owned seat above it — and the floor can never keycard in a
    /// disturbed auction (`docs/minor-transfer-slam.md`).
    #[arg(long, default_value_t = false)]
    no_ns_landy_minor_slam_answer: bool,

    /// Author our defense to the opponents' 2♣ Stayman (`(1NT) - (2♣)`): X =
    /// lead-directing clubs, natural overcalls, strong 3♣ (default off; opt-in A/B).
    #[arg(long, default_value_t = false)]
    ns_defense_to_their_stayman: bool,

    /// Author our continuations after the opponents contest our Jacoby transfer
    /// (`1NT - 2♦/2♥ (X)` / `1NT - 2♦/2♥ (overcall)`); default off (opt-in A/B — DD-negative).
    #[arg(long, default_value_t = false)]
    ns_comp_over_transfer: bool,

    /// Author opener's jump super-accept of a Jacoby transfer (four-card support +
    /// a maximum); default off (opt-in A/B — DD wash).
    #[arg(long, default_value_t = false)]
    ns_transfer_super_accept: bool,

    /// Disable responder's game-forcing structure after the spade transfer
    /// (`1NT - 2♥ - 2♠`: natural 5-5 `3♥` slam try, `3♣`/`3♦` minors, `4♣`/`4♦`/`4♥`
    /// splinters, quantitative `4NT`); on by default.
    #[arg(long, default_value_t = false)]
    no_ns_transfer_gf_majors: bool,

    /// Within the GF-majors structure (Arm B), reserve `3♣`/`3♦` for slam tries and
    /// route minimum game-forces into the choice-of-games `3NT`; default off.
    #[arg(long, default_value_t = false)]
    ns_minor_min_to_3nt: bool,

    /// Disable the GF structure's heart-transfer mirror (`1NT - 2♦ - 2♥`: `3♣`/`3♦` minors,
    /// `3♠`/`4♣`/`4♦` splinters, quantitative `4NT`); on by default (with the master
    /// GF-majors structure).
    #[arg(long, default_value_t = false)]
    no_ns_transfer_gf_hearts: bool,

    /// Disable responder's post-transfer single-suited slam try (`1NT - 2♦ - 2♥ - 3♠` /
    /// `1NT - 2♥ - 2♠ - 3♥`, a 5-card-major RKCB slam try); on by default.
    #[arg(long, default_value_t = false)]
    no_ns_transfer_slam_try: bool,

    /// Disable the Texas + responder-RKCB slam drive for 6-card-major hands
    /// (restores the opener-decides direct `1NT - 4♥/4♠` at 15-18); on by default.
    #[arg(long, default_value_t = false)]
    no_ns_texas_slam_drive: bool,

    /// Disable garbage (drop-dead) Stayman: a weak 2♣ to escape 1NT, passing
    /// opener's 2♦/2♥/2♠; on by default.  Off-switch for the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_garbage_stayman: bool,

    /// Disable opener's max-only right-siding relay over 1NT - 2♣ with both four-card
    /// majors (2NT = 16-17; responder names a major via 3♣/3♦, opener completes and
    /// declares); on by default.  Off-switch for the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_stayman_both_majors: bool,

    /// Disable opener's max five-card-major jump over 1NT - 2♣ (3♥/3♠); on by
    /// default.  Off-switch for the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_stayman_5card_max: bool,

    /// Disable the invitational 5-4-majors structure after 1NT (5♠4♥ Staymans and
    /// rebids 2♠; 5♥4♠ transfers to hearts and rebids 2NT/2♠); on by default.
    /// Off-switch for the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_invitational_5card_majors: bool,

    /// Disable Crawling Stayman (superset of garbage: 4-4 majors short in diamonds
    /// — 4414/4405 — bid 2♣ and crawl opener's 2♦ to 2♥, pass-or-correct); on by
    /// default. Off-switch for the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_crawling_stayman: bool,

    /// Disable responder's continuation after opener's 3OM-slam-try cue
    /// (`1NT - 2♣ - 2M - 3OM - 4x`): on, responder keycards or signs off in the major game
    /// instead of passing the cue out below game; on by default. Off-switch for the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_stayman_cue_continuation: bool,

    /// Disable the longer-major discipline for minor-opening responses (1♠ on
    /// longer spades or 5-5, 1♥ up the line only on 4-4, with the M6.4
    /// classifier reading to match); on by default (the established American
    /// treatment — see `ReadingProfile::longer_major_response`). Off-switch for the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_longer_major_response: bool,

    /// Disable the up-the-line completion of the natural minor tree (the
    /// 1♣ - 1♦ response, opener's 1♠ rebid over 1m - 1♥, opener's natural 2♣
    /// after 1♣ - 1♦); on by default, shipped jointly with XYZ. Off-switch for
    /// the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_up_the_line: bool,

    /// Disable the 1M - 3NT choice-of-games response (3-4 card support, exactly
    /// (4333), 12-15 HCP; opener passes balanced, corrects to 4M with shape);
    /// on by default. Off-switch for the A/B (see `response.major_choice_of_games`).
    #[arg(long, default_value_t = false)]
    no_ns_major_choice_of_games: bool,

    /// Disable the fit leg of the major 2/1 game force (exactly 3-card
    /// support enters on `support_points(13..)` — the 2/1 as a preparation
    /// for 4M); on by default. Off-switch for the A/B (see
    /// `response.two_over_one_fit`).
    #[arg(long, default_value_t = false)]
    no_ns_two_over_one_fit: bool,

    /// The no-fit gauge of the major 2/1 game force:
    /// points13 (shipped default since `39a5eb6`) | hcp13 (superseded) | hcp12
    /// (see `response.two_over_one_gate`).  Must track the crate default: an
    /// unflagged run is the shipped system, and `bba-decompose` replays on
    /// crate defaults, so a stale value here shows up as a replay-verification
    /// miss and silently measures a system we do not ship.
    #[arg(long, default_value = "points13")]
    ns_two_over_one_gate: String,

    /// Natural per-call 2/1 suit lengths: `1♠ - 2♥` promises 5+ hearts, `1♠ - 2♣`
    /// allows 3+ clubs (the cheapest 2/1 catch-all); every other 2/1 keeps 4+.
    /// Off by default (uniform 4+) — on-switch for the A/B (see
    /// `response.two_over_one_natural_lengths`).
    #[arg(long, default_value_t = false)]
    ns_two_over_one_natural_lengths: bool,

    /// Lighten `1♠ - 2♥` by one HCP on its no-fit leg (`hcp(12..)` at the default
    /// hcp13 gate), the five-card major worth a shade-light game force. Off by
    /// default — on-switch for the A/B (see `response.two_over_one_major_discount`).
    #[arg(long, default_value_t = false)]
    ns_two_over_one_major_discount: bool,

    /// Disable the XYZ two-way checkback after three one-level bids (2♣
    /// puppets 2♦ for sign-off or invite, 2♦ game-forces); on by default.
    /// Off-switch for the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_xyz: bool,

    /// Enable New Minor Forcing in place of XYZ on the four `1m - 1M - 1NT` slots
    /// (opt-in, off by default): responder's two-of-the-new-minor is an
    /// invitational-or-better checkback promising a five-card major.
    #[arg(long, default_value_t = false)]
    ns_new_minor_forcing: bool,

    /// Author opener's major game tries after a single raise (`1M - 2M`): a
    /// long-suit try, the general re-raise, or a keycard-asking maximum
    /// (shipped default-on; see `response.major_game_tries`).
    #[arg(long, default_value_t = false)]
    no_ns_major_game_tries: bool,

    /// Disable opener's limit-raise acceptance ladder after `1M - 3M`
    /// (shipped default-on; see `response.limit_raise_acceptance`).
    #[arg(long, default_value_t = false)]
    no_ns_limit_raise_acceptance: bool,

    /// Disable opener's answer to partner's cue-raise (`1M (ovc) cue -`)
    /// (shipped default-on; see `competition.cue_raise_answer`).
    #[arg(long, default_value_t = false)]
    no_ns_cue_raise_answer: bool,

    /// Disable opener's answer to a *minor*-opening cue-raise
    /// (`1m (ovc) cue -`) (default-on; see `competition.cue_minor_raise_answer`).
    #[arg(long, default_value_t = false)]
    no_ns_cue_minor_raise_answer: bool,

    /// Disable responder's structure over their two-suiters over our 1M — UvU
    /// over their both-minors `(2NT)`, the raise structure over their Michaels
    /// cue, and the two-suiter inference reading (shipped default-on; see
    /// `competition.uvu_over_majors`).
    #[arg(long, default_value_t = false)]
    no_ns_uvu_over_majors: bool,

    /// Author responder's structure over their both-majors Michaels cue of our
    /// minor (`1♣ (2♣)` / `1♦ (2♦)`) — cues split limit-raise/GF, X = values
    /// with a punishable major (default off; unmeasurable vs BBA, whose live
    /// engine never bids the cue; see `competition.uvu_over_minors`).
    #[arg(long, default_value_t = false)]
    ns_uvu_over_minors: bool,

    /// Author our contested weak twos — business XX + systems-on Ogust over
    /// their double, Ogust-when-legal / values-X / preemptive raises over
    /// their overcall (default off; see `competition.weak_two_competition`).
    #[arg(long, default_value_t = false)]
    ns_weak_two_comp: bool,

    /// Disable our contested strong 2♣ — systems-on over their double,
    /// natural GF / values-X / waiting-pass + forced reopening over their
    /// overcall (shipped default-on; see `competition.strong_two_competition`).
    #[arg(long, default_value_t = false)]
    no_ns_strong_two_comp: bool,

    /// Disable opener's support double/redouble on `1♥ - 1♠` (shipped
    /// default-on; see `competition.major_support_double`).
    #[arg(long, default_value_t = false)]
    no_ns_major_support_double: bool,

    /// Author responder's natural free bids over an overcall — 1-level new
    /// suit 5+ & 6+, 2-level non-jump 5+ & 10+, 1NT/2NT with a stopper
    /// (default off; implied by --ns-negative-double-shape modern|cachalot;
    /// see `competition.free_bids`).
    #[arg(long, default_value_t = false)]
    ns_free_bids: bool,

    /// Minimum points/HCP for the 1-level free bids (default 6; sweep to 8+ to
    /// trim the free-bid family's vulnerable-PD leak; see `competition.free_bid_floor`).
    #[arg(long, default_value_t = 6)]
    ns_free_bid_floor: u8,

    /// Minimum HCP for the free 1NT (`1X (1Y) 1NT`), decoupled from the suit
    /// floor above (default 6; see `competition.free_1nt_floor`).
    #[arg(long, default_value_t = 6)]
    ns_free_1nt_floor: u8,

    /// Gate the vulnerable free bids on quality: a vulnerable 1-level new suit
    /// needs two of the top three honors, and the free 1NT is not authored
    /// vulnerable (default off; see `competition.free_bid_quality`).
    #[arg(long, default_value_t = false)]
    ns_free_bid_quality: bool,

    /// The negative-double school over our minor openings:
    /// modern (shipped default) | both-majors | cachalot | sputnik
    /// (see `competition.negative_double_shape`; all but both-majors imply the free
    /// bids and opener's forcing answers to them).
    #[arg(long, default_value = "modern")]
    ns_negative_double_shape: String,

    /// Responder's non-jump 2-level new suit over their overcall:
    /// forcing (shipped default — forcing one round) | negative (classic NFB:
    /// non-forcing 5-11 with a 6+ suit or strong 5-carder; stronger long-suit
    /// hands double then bid, forcing to game) | transfer (2-level slots swap
    /// and opener completes; see `competition.free_bid_style`).
    #[arg(long, default_value = "forcing")]
    ns_free_bid_style: String,

    /// Author responder's structure over their jump / 3-level overcalls
    /// (2NT < bid ≤ 3♠): negative X through 3♠, forcing new suits, 3NT with a
    /// stopper (default off; see `competition.high_overcall_responses`).
    #[arg(long, default_value_t = false)]
    ns_high_overcall: bool,

    /// Author responder's structure over their three-level overcall of our
    /// `1NT` (`(3♣)`–`(3♠)`, natural seven-card preempts): forcing three-level
    /// suits, natural four-level bids, takeout `X`, `3NT`, and opener's one
    /// answer to each.  **Engine default ON since 2026-08-18**; pass `false`
    /// for the pre-ship arm.  Unset = the engine default; see
    /// `competition.nt_high_overcall_responses`.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    ns_nt_high_overcall: Option<bool>,

    /// Require a stopper for responder's direct `3NT` over their *three-level*
    /// overcall of our `1NT` — the three-level table's own copy of
    /// --ns-direct-3nt-stopper, which it cannot share (the paired arm that
    /// dropped the shared bit won this lane and lost the takeout-double advance
    /// the shared bit also governs).  **Engine default OFF since 2026-08-18**
    /// — no stopper needed, because partner opened 1NT; pass `true` for the
    /// pre-flip arm.  Unset = the engine default.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    ns_nt_high_overcall_3nt_stopper: Option<bool>,

    /// Play transfers over their `(3♣)` overcall of our `1NT` (`3♦`/`3♥` to
    /// the majors, INV+ and completed at game; `3♠` to diamonds).  Implies
    /// --ns-nt-high-overcall; does nothing without it.  Unset = the engine
    /// default (off while the A/B runs); see `competition.nt_3c_transfers`.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    ns_nt_3c_transfers: Option<bool>,

    /// Opener answers responder's takeout double of their three-level overcall
    /// in the shown major at its **cheapest legal** level, even when that is
    /// the four level — only their `(3♠)` has one.  Implies
    /// --ns-nt-high-overcall; does nothing without it.  **Engine default ON
    /// since 2026-08-19**; pass `false` for the pre-ship arm.  Unset = the
    /// engine default; see `competition.nt_high_overcall_x_major_at_four`.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    ns_nt_high_overcall_x_major_at_four: Option<bool>,

    /// Opener may leave in responder's takeout double of their three-level
    /// overcall, converting it to penalty, holding FOUR cards in their suit.
    /// Implies --ns-nt-high-overcall; does nothing without it.  **Engine
    /// default ON since 2026-08-20**; pass `false` for the pre-ship arm.
    /// Unset = the engine default; see
    /// `competition.nt_high_overcall_x_leave_in`.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    ns_nt_high_overcall_x_leave_in: Option<bool>,

    /// Extend that leave-in to three cards headed by two of the top three
    /// honors, i.e. the full v2 candidate gate rather than the length half.
    /// Implies --ns-nt-high-overcall-x-leave-in; does nothing without it.
    /// Unset = the engine default (off); see
    /// `competition.nt_high_overcall_x_leave_in_three`.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    ns_nt_high_overcall_x_leave_in_three: Option<bool>,

    /// Require a stopper for responder's *direct* `3NT` over their overcall of
    /// our `1NT`.  Unset tracks the shipped engine default (on); pass `false`
    /// for the no-gate arm ("partner can hold the stopper").  Shared by the
    /// two-level Lebensohl lane and the three-level table; see
    /// `competition.direct_3nt_stopper`.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    ns_direct_3nt_stopper: Option<bool>,

    /// Re-enable our takeout double on a flat 4-3-3-3 weaker than a 1NT opening
    /// (12–14 HCP flat 4333) — the default suppresses it and routes to Pass
    /// (shipped default-on; see `DefenseKnobs::suppress_flat_4333_takeout`).
    #[arg(long, default_value_t = false)]
    no_ns_suppress_flat_4333_takeout: bool,

    /// Re-enable our takeout double on a weak `5-3-3-2` (12–13 HCP) — the default
    /// routes it to a natural overcall of the five-card suit (a 5-3-3-2 has no
    /// 4-card major, so the double cannot find a fit; shipped default-on; see
    /// `DefenseKnobs::suppress_5332_takeout`).
    #[arg(long, default_value_t = false)]
    no_ns_suppress_5332_takeout: bool,

    /// Route a weak `4-4-3-2` (12–13 HCP) to Pass when the opponents opened a
    /// **major** (opt-in; see `DefenseKnobs::suppress_4432_vs_major`).
    #[arg(long, default_value_t = false)]
    ns_suppress_4432_vs_major: bool,

    /// Route a weak `4-4-3-2` (12–13 HCP) to Pass when the opponents opened a
    /// **minor** (opt-in; see `DefenseKnobs::suppress_4432_vs_minor`).
    #[arg(long, default_value_t = false)]
    ns_suppress_4432_vs_minor: bool,

    /// Re-enable our takeout double on a hand with an unbid five-card **major** —
    /// the default routes it to a natural overcall of the major (show the suit
    /// rather than double into partner's short suit; shipped default-on; see
    /// `DefenseKnobs::suppress_5card_major_takeout`).
    #[arg(long, default_value_t = false)]
    no_ns_suppress_5card_major_takeout: bool,

    /// Bar our takeout double on a hand with an unbid **six-card minor** — the
    /// same argument as the five-card major, one card further (opt-in; see
    /// `DefenseKnobs::suppress_long_minor_takeout`).
    #[arg(long, default_value_t = false)]
    ns_suppress_long_minor_takeout: bool,

    /// Let the strong takeout double reach one HCP lower for hands that would
    /// otherwise have to overcall at the **two** level, and author the doubler's
    /// rebids over a minimum advance (opt-in; see
    /// `DefenseKnobs::defensive_seam_split`).
    #[arg(long, default_value_t = false)]
    ns_defensive_seam_split: bool,

    /// Disable the **rich advance** of partner's takeout double of a one-opening
    /// (`(1t) X - ?`) — revert to the flat advance without the cue + notrump
    /// invite/force ladder (shipped default-on; see `defense.rich_advance_double_enabled`).
    #[arg(long, default_value_t = false)]
    no_ns_rich_advance: bool,

    /// Add the **jump-cue Rubens transfer** layer on top of the rich advance (a
    /// transfer to a 5+ unbid major; no-op unless `--ns-rich-advance`; opt-in,
    /// see `defense.advance_rubens_enabled`).
    #[arg(long, default_value_t = false)]
    ns_advance_rubens: bool,

    /// Disable the advancer's **invitational minor jump** on the rich advance — a
    /// three-level minor jump = 5+ one-suiter, 10–12, denying a 4-card unbid major
    /// (with the doubler's stopper-ask cue continuation) — revert that rung to the
    /// floor (shipped default-on; no-op unless `--ns-rich-advance`; see
    /// `defense.advance_minor_jump_enabled`).
    #[arg(long, default_value_t = false)]
    no_ns_advance_minor_jump: bool,

    /// Disable the **doubler's accept/decline of the advancer's `2NT` invite** on
    /// the rich advance (Pass = decline, 3NT = accept to play, new 5-card major =
    /// game-forcing) — revert to the floor, which passes `2NT` even holding a game
    /// (shipped default-on; no-op unless the rich advance is on; see
    /// `defense.advance_2nt_continuation_enabled`).
    #[arg(long, default_value_t = false)]
    no_ns_advance_2nt_continuation: bool,

    /// Advance partner's takeout double with the **highest-ranking** eligible
    /// suit rather than the **longest** (higher-ranking on a tie); also governs
    /// the rich advance's weak natural and forced-suit picks (shipped default-on
    /// = longest; see `defense.longest_first_advance_enabled`).
    #[arg(long, default_value_t = false)]
    no_ns_longest_advance: bool,

    /// The advancer's **weak** penalty pass of partner's takeout double yields
    /// to a 4+ unbid major: below the 10+ cue band the hand bids the ladder
    /// instead of sitting; strong sits stand (default off; see
    /// `defense.advance_pass_yield_major_enabled`).
    #[arg(long, default_value_t = false)]
    ns_advance_pass_yield: bool,

    /// Swap the advancer's 4-card penalty-pass quality gate from two top
    /// honors to a per-suit HCP floor N — 5 admits exactly AJxx, 6 instead
    /// drops bare KQxx — rich advance only (default: the honor gate; see
    /// `defense.advance_sit_hcp_gate`).
    #[arg(long, value_name = "N")]
    ns_advance_sit_hcp: Option<u8>,

    /// Disable opener's balanced `1NT` rebid after `1m - 1M` — revert a balanced
    /// 12–14 with a five-card minor to the natural `2m` (shipped default-on; see
    /// `rebid.balanced_1nt_rebid`).
    #[arg(long, default_value_t = false)]
    no_ns_balanced_1nt_rebid: bool,

    /// Disable opener's strength-showing rebid ladder after a minor opening and a
    /// one-level response — revert jump-rebid / reverse / jump-shift to the
    /// minimum natural rebid (shipped default-on; see `ReadingProfile::opener_extras_ladder`).
    /// BBA-gap bucket #3.
    #[arg(long, default_value_t = false)]
    no_ns_opener_extras_ladder: bool,

    /// Disable opener's major jump-rebid rung (`1♥ - 1♠ - 3♥`, `1M - 1NT - 3M`)
    /// on a six-card major with 16+ and responder's continuation over it — the
    /// major-opening half of the extras ladder (shipped default-on; see
    /// `ReadingProfile::opener_major_jump_rebid`).
    #[arg(long, default_value_t = false)]
    no_ns_opener_major_jump_rebid: bool,

    /// Disable opener's third-call table after responder raises opener's second
    /// suit in a 2/1 auction (`1M - 2r - 2x - 3x`) — drop that node to the floor
    /// (shipped default-on; see `game_force.second_suit_agreement`).  Constructive
    /// book re-audit candidate #1: two rules, `points(15..)` RKCB else an
    /// unconditional sign-off, the retired backstop's signature.
    #[arg(long, default_value_t = false)]
    no_ns_second_suit_agreement: bool,

    /// Disable opener's third call after trump is agreed at `1M - 2r - R - 3M`
    /// — drop that node to the floor (shipped default-on; see
    /// `game_force.opener_third`).  Re-audit candidate #2: the deletion measures
    /// +0.437/+0.527 plain per divergent but leaves the floor unable to ask
    /// keycards at all here, so it is *not* shipped.
    #[arg(long, default_value_t = false)]
    no_ns_opener_third: bool,

    /// Restore the retired 2/1 game backstop — the three-rule table (4♥/4♠/3NT)
    /// that used to answer every game-forcing continuation the authored rounds
    /// miss.  Shipped off since 2026-07-20; those nodes now fall to the
    /// BBA-distilled floor (see `game_force.game_backstop`).
    #[arg(long, default_value_t = false)]
    ns_game_backstop: bool,

    /// Drop the floor's 2/1 game force, letting it pass below game in an
    /// established two-over-one.  The authored book holds the force by omission;
    /// the floor needs telling, and without this it abandoned partner's 2/1 on
    /// 24% of the boards the backstop deletion touched (shipped default-on; see
    /// `DecisionProfile::two_over_one_force`).
    #[arg(long, default_value_t = false)]
    no_ns_two_over_one_force: bool,

    /// Stop flooring partner's shown strength at the 13 points a two-over-one
    /// promised, on the floor's slam-entry gate.  The 2/1 response is alerted, so
    /// its natural reading is suppressed and the rule's `points(13..)` gate
    /// projects to no high-card floor at all — partner reads as *zero* through
    /// the whole game force and the floor can never reach the slam-entry
    /// threshold.  Shipped default-on; see `InstinctProfile::two_over_one_slam_strength`.
    #[arg(long, default_value_t = false)]
    no_ns_two_over_one_slam_strength: bool,

    /// Disable the competitive long-suit rebid — opener's/overcaller's rebid of a
    /// 6+ suit in competition (2-level any, 3-level needs 7 cards or a good six)
    /// instead of a forced takeout double (shipped default-on; see
    /// `InstinctKnobs::competitive_rebid`).
    #[arg(long, default_value_t = false)]
    no_ns_competitive_rebid: bool,

    /// Disable opener's balanced-18-19 notrump actions in a `1X (1Y) …` auction
    /// the floor otherwise passes out: reopening 1NT, 3NT over responder's free
    /// 1NT, and responder's raise (default-on; see `InstinctKnobs::reopening_notrump`).
    #[arg(long, default_value_t = false)]
    no_ns_reopening_notrump: bool,

    /// Disable the rein on a minimum takeout doubler that over-raises partner's
    /// forced advance of our double into a doubled game (default-on; see
    /// `InstinctProfile::rein_advance_raise`).
    #[arg(long, default_value_t = false)]
    no_ns_rein_advance_raise: bool,

    /// Disable opener's authored raise of a Cachalot X transfer when LHO
    /// competes over it (default-on; Cachalot only; see
    /// `competition.cachalot_contested_x`).
    #[arg(long, default_value_t = false)]
    no_ns_cachalot_contested_x: bool,

    /// Disable responder's structure over their takeout double of our 1-suit
    /// opening: Jordan/Truscott 2NT, value XX, preemptive jump-raise flip,
    /// weak NF 2-level suits (shipped default-on; see `competition.jordan_truscott`).
    #[arg(long, default_value_t = false)]
    no_ns_jordan_truscott: bool,

    /// Disable systems-on over their double of our splinter — revert to letting
    /// opener's rebid fall to the floor, which passes the doubled game force
    /// (shipped default-on; see `competition.splinter_doubled`).
    #[arg(long, default_value_t = false)]
    no_ns_splinter_doubled: bool,

    /// Disable the major-rebid-tails adjunct — the full continuations after
    /// `1♥ - 1♠` below opener's `2♠`/`3♠` raise, `2♥` rebid, and `2♣`/`2♦`
    /// minor rebid (shipped default-on; see `rebid.major_rebid_tails`).
    #[arg(long, default_value_t = false)]
    no_ns_major_rebid_tails: bool,

    /// Disable fourth-suit-forcing — at `1♥ - 1♠ - 2♣`, responder's `2♦`
    /// reverts to the natural-tail reading (shipped default-on; rides the
    /// tails adjunct, so `--no-ns-major-rebid-tails` also silences it — see
    /// `rebid.fourth_suit_forcing`).
    #[arg(long, default_value_t = false)]
    no_ns_fourth_suit_forcing: bool,

    /// point_count + trump length floor at which a 6-card-major responder blasts
    /// game via South African Texas (4♣/4♦) instead of transferring at the two
    /// level; default 14 (a 6-bagger needs 8 points, lowered from the inherited
    /// raw-HCP 9).
    #[arg(long, default_value_t = 14)]
    ns_texas_game_floor: u8,

    /// point_count + trump length floor at which a 6-card-major responder *invites*
    /// game (transfer, then jump to 3M) instead of resting in the two-level
    /// partscore; default 13 (on).  Raise to the blast floor (14) to empty the
    /// invitational band and turn the invite off.
    #[arg(long, default_value_t = 13)]
    ns_sixcard_invite_floor: u8,

    /// point_count + trump length at which opener accepts the six-card-major invite
    /// (…3M → 4M), else passes 3M; default 18.
    #[arg(long, default_value_t = 18)]
    ns_sixcard_accept_floor: u8,

    /// Author our defense to the opponents' Jacoby transfers (`(1NT) - (2♦/2♥)`):
    /// X = lead-directing the bid suit, Michaels cue, natural overcalls (default
    /// off; opt-in A/B).
    #[arg(long, default_value_t = false)]
    ns_transfer_defense: bool,

    /// Turn OFF our continuations after the opponents contest our two-way 2♠ minor
    /// response (`1NT - 2♠ (X)` / `1NT - 2♠ (overcall)`); default on (the A/B off-switch —
    /// measured +4.80 IMPs/fired plain, +5.63 PD).
    #[arg(long, default_value_t = false)]
    no_ns_comp_over_minor_transfer: bool,

    /// Author our defense to the opponents' two-way 2♠ minor response
    /// (`(1NT) - (2♠)`): X = lead-directing spades, 2NT/3♣ two-suiters, natural
    /// overcalls (default off; opt-in A/B).
    #[arg(long, default_value_t = false)]
    ns_minor_transfer_defense: bool,

    /// Turn OFF our continuations after the opponents contest our 2NT diamond
    /// transfer (`1NT - 2NT (X)` / `1NT - 2NT (overcall)`); default on (the A/B off-switch —
    /// measured a plain-DD wash +0.24/fired, +3.40 PD).
    #[arg(long, default_value_t = false)]
    no_ns_comp_over_diamond_transfer: bool,

    /// Author our defense to the opponents' 2NT diamond transfer
    /// (`(1NT) - (2NT)`): X = lead-directing diamonds, 3♦ cue = both majors,
    /// natural overcalls (default off; opt-in A/B).
    #[arg(long, default_value_t = false)]
    ns_diamond_transfer_defense: bool,

    /// Stayman-defense natural-overcall `MIN_LEN:POINTS_FLOOR` (default 6:14, the
    /// crate default); the A/B search knob for the 2♦/2♥/2♠ length + strength (no
    /// effect unless `--ns-defense-to-their-stayman`).
    #[arg(long, default_value = "6:14")]
    ns_staydef_overcall: String,

    /// Shape gate for our natural penalty double of their 1NT: balanced (default,
    /// matches the shipped `american()`) | semi | any.
    #[arg(long, default_value = "balanced")]
    ns_double_shape: String,

    /// HCP floor for our natural penalty double of their 1NT (default 15).
    #[arg(long, default_value_t = 15)]
    ns_double_floor: u8,

    /// Inclusive `points` range LO:HI for our natural two-level suit overcall of
    /// their 1NT (default 8:14).
    #[arg(long, default_value = "8:14")]
    ns_overcall: String,

    /// Logit weight of our natural penalty double of their 1NT, in centinats
    /// (default 130, above the 100 suit overcall).
    #[arg(long, default_value_t = 130)]
    ns_double_weight: i16,

    /// Support gate on our 12+ takeout double of a suit / weak-two opening:
    /// off | lenient | strict (default, matches shipped `american()`).
    #[arg(long, default_value = "strict")]
    ns_takeout_support: String,

    /// Discipline our natural suit-overcall bands (1-level 8–17, 2-level 11–17)
    /// instead of the flat 8–16: on (default, matches shipped `american()`) | off.
    #[arg(long, default_value = "on")]
    ns_overcall_discipline: String,

    /// Disable the shipped direct weak jump overcall: with an exactly six-card
    /// major, 8+ points, and at most 11 HCP, the default bids 2M instead of the
    /// simple 1M overcall.
    #[arg(long, default_value_t = false)]
    no_ns_direct_weak_jump_overcall: bool,

    /// Disable a passed hand's lighter (9+ not 11+) disciplined 2-level overcall
    /// (folded into base default-on in the A5 pass; see `defense.passed_hand_overcall`).
    #[arg(long, default_value_t = false)]
    no_ns_passed_hand_overcall: bool,

    /// Demand 15+ for the 2-level minor overcall (2♣/2♦ below their suit) instead
    /// of the disciplined 11+; off by default (A/B candidate — the anchor bleeds
    /// on these across every band, sd-lead confirms the loss is real).
    #[arg(long, default_value_t = false)]
    ns_two_level_minor_overcall_tight: bool,

    /// Bar an unbid five-card major from the natural 1NT overcall (overcall the
    /// major instead, to find the fit); off by default.  A five-card opener's
    /// major remains eligible, matching BBA's direct-overcall box.
    #[arg(long, default_value_t = false)]
    ns_nt_overcall_no_major: bool,

    /// Disable the shipped exact-six 8+ points / at-most-11-HCP `(1♣) 2♦`
    /// weak jump overcall.
    #[arg(long, default_value_t = false)]
    no_ns_direct_minor_weak_jump_overcall: bool,

    /// Disable systems-on advances after our 1NT overcall: on, the advancer plays
    /// the full opening-1NT structure (Stayman/transfers/Smolen) grafted below
    /// `(1t) 1NT`, finding and right-siding major fits; on by default. Off-switch
    /// for the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_nt_overcall_systems_on: bool,

    /// Gladiator advances after our 1NT overcall of their *major* (replaces the
    /// opening-1NT graft over majors only): 2♣ weak relay, cue-of-major Stayman
    /// for the unbid major, natural INV, splinter/Leaping-Michaels. Off by default
    /// (A/B candidate — the major graft washes plain/PD, wins only on sd-lead).
    #[arg(long, default_value_t = false)]
    ns_nt_overcall_gladiator: bool,

    /// Extend our 1NT defense to the balancing seat `(1NT) - - ?` (default off).
    #[arg(long, default_value_t = false)]
    ns_balancing: bool,

    /// Raw-HCP floor under the natural two-level overcall of their 1NT, on top of
    /// the `points(8..=14)` band (M1; **8 shipped**, `0` restores the old overcall,
    /// `9` is the wider cut and still unmeasured).
    #[arg(long, default_value_t = 8)]
    ns_nt_overcall_hcp_floor: u8,

    /// Leave `(1NT) 2x - ?` to the instinct floor instead of the authored advance
    /// (M2; the advance is **shipped on**, so this is the control arm's flag).
    #[arg(long, default_value_t = false)]
    no_ns_nt_overcall_advance: bool,

    /// Which mutually-exclusive defense our side plays over BBA's 1NT (default
    /// `natural`).
    ///
    /// `natural` = penalty X + the four natural two-level overcalls; `direct-dont`
    /// = one-suiter X, 2♣/2♦/2♥ two-suiters, 2♠ natural, 2NT both minors;
    /// `meckwell` = two-way X, 2♣/2♦ minor+major, natural majors; `woolsey` =
    /// Multi-Landy; `direct-landy` = both-majors takeout X; `always-pass` = we
    /// never compete (the truest do-nothing baseline); `off` = author nothing
    /// and let the bare instinct floor have the seat.
    ///
    /// One flag because the systems are one choice: the engine holds them in a
    /// single `Cell<NotrumpDefense>`, and the bool-per-system CLI this replaced
    /// accepted `--ns-dont --ns-meckwell --ns-woolsey` together and silently
    /// resolved it by whichever `set_*` call ran last.  Run `meckwell`/`woolsey`
    /// WITHOUT `--advertise-natural` (BBA reads them via its own DONT /
    /// Multi-Landy rows).
    #[arg(long, value_enum, default_value = "natural")]
    ns_notrump_defense: NtDefenseArg,

    /// Play (and read) the **European** 1NT minor scheme instead of Puppet:
    /// `2♠` = clubs, `2NT` = balanced invite, `3♣` = diamonds (default off).
    ///
    /// A two-valued knob, so a bool rather than a `--ns-*` value enum. Its real
    /// use is `--their-ns`: EPBot's stock system-0 defaults *already* carry this
    /// scheme (`probe-bba-conventions`), so an opponent left at Puppet is being
    /// misread. See `docs/ai-bidder/bba-1nt-minors.md`.
    #[arg(long, default_value_t = false)]
    ns_european_minors: bool,

    /// DONT one-suiter minimum length for the `X`/`2♠` (default 5; set 6 to insist
    /// only with a six-card suit). Only with `--ns-notrump-defense direct-dont`.
    #[arg(long, default_value_t = 5)]
    ns_dont_one_suiter_min: u8,

    /// Let DONT two-suiters (`2♣`/`2♦`/`2♥`) accept a flat 4-4 (default off = 5-4+).
    /// Only with `--ns-notrump-defense direct-dont`.
    #[arg(long, default_value_t = false)]
    ns_dont_four_four: bool,

    /// Probe: let Meckwell's `2♣`/`2♦` accept a flat 4-4 (default off = 5-4+). Only
    /// with `--ns-notrump-defense meckwell`.
    #[arg(long, default_value_t = false)]
    ns_meckwell_minor_major_44: bool,

    /// Probe: let Meckwell's both-majors `X` accept a flat 4-4 (default on = 4-4; set
    /// false-ish by passing `--ns-meckwell-x-five-four` for 5-4+). Only with
    /// `--ns-notrump-defense meckwell`.
    #[arg(long, default_value_t = false)]
    ns_meckwell_x_five_four: bool,

    /// Overlay Landy on our natural 1NT defense (default off): `2♣` = both majors
    /// (≥5-4), `2NT` = both minors, on the given `points` band `LO:HI`, replacing the
    /// natural `2♣` club overcall (penalty X + natural `2♦`/`2♥`/`2♠` stay).  Pair with
    /// `--advertise-landy` so BBA reads our `2♣` as both majors and the rest natural.
    ///
    /// An **overlay, not a system** — hence its own flag rather than a
    /// `--ns-notrump-defense` variant.  The engine keeps it in a separate cell and
    /// honours it only under `natural`/`off`, so pairing it with a conventional
    /// family leaves it inert rather than fighting for `2♣`.
    #[arg(long)]
    ns_landy: Option<String>,

    /// Woolsey suit-overcall (2♣/2♦/2♥/2♠) points band LO:HI (default 8:19). Only
    /// with `--ns-notrump-defense woolsey`.
    #[arg(long, default_value = "8:19")]
    ns_woolsey_range: String,

    /// `points` floor for our Woolsey takeout X (default 12). Only with
    /// `--ns-notrump-defense woolsey`.
    #[arg(long, default_value_t = 12)]
    ns_woolsey_x_floor: u8,

    /// Disable the penalty-double latch (default on): after our natural penalty X of
    /// BBA's 1NT, our later doubles read as penalty instead of takeout.
    #[arg(long, default_value_t = false)]
    no_ns_penalty_latch: bool,

    /// Arm the **generalized** pass/double-inversion latch (default off): any
    /// penalty-oriented double we made, or any pass of ours that left partner's
    /// double in, makes our later doubles read and bid as penalty.  The treatment
    /// arm of the PDI A/B (docs/pdi.md).
    #[arg(long, default_value_t = false)]
    ns_pdi_latch: bool,

    /// Recompute each seat's sound hull from its union after the walk (default
    /// off): a post-walk union that the finished walk collapses then narrows what
    /// the book, `instinct()` and the floor's feature block read, not just the
    /// sampler.  The treatment arm of the union-hull pre-count (docs/pdi.md,
    /// follow-on 1).
    #[arg(long, default_value_t = false)]
    ns_union_hull: bool,

    /// Restore the doubler's constructive pulls of its own penalty X of BBA's 1NT
    /// (default off = pulls suppressed): with this set, a latched doubler may again
    /// "compete" to 2NT/3NT/a major over the opponents' escape instead of defending.
    #[arg(long, default_value_t = false)]
    ns_allow_pull: bool,

    /// Disable the advancer's runout from BBA's redoubled penalty X (default on):
    /// after `(1NT) X (XX)`, a weak advancer sits for `1NTxx` instead of escaping to
    /// its long suit.
    #[arg(long, default_value_t = false)]
    no_ns_xx_runout: bool,

    /// Disable the *doubler's* runout once BBA's redoubled penalty X runs back around
    /// (default on): after `(1NT) X (XX) - -`, a 15+ doubler with a five-plus suit
    /// escapes to it instead of defending `1NTxx`.
    #[arg(long, default_value_t = false)]
    no_ns_doubler_run: bool,

    /// Enable Rubens advances of partner's simple overcall (**default off**
    /// since the layer lost its A/B, `scripts/ab-rubens.sh`): the natural
    /// raises plus natural two-level new-suit advance become the transfer
    /// ladder and the two-level cue-raise.  Also un-silences `rubens_reading`.
    #[arg(long, default_value_t = false)]
    ns_rubens: bool,

    /// Disable recording the one-level Rubens transfers' meaning (default on):
    /// the transfers revert to suppress-only, the overcaller blind to the shown
    /// support/length and strength — the reading-attribution A/B arm.
    #[arg(long, default_value_t = false)]
    no_ns_rubens_reading: bool,

    /// Disable the floor's RKCB 1430 (default on, M6.4): the floor reverts to
    /// the direct milestone slams (6/7 of the fit at 33/37 combined) with no
    /// keycard ask — the pre-M6.4 baseline for the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_floor_rkcb: bool,

    /// Carve RKCB back to agreed majors, in **both** layers (default off, i.e.
    /// it reaches minors too): the floor's `keycard_trump` carve and the book's
    /// two minor vehicles are one agreement and one knob.  The pre-2026-08
    /// baseline, which arm B of the kickback A/B beat by +0.0039/board vul none
    /// and +0.0050 vul both, and which the book half beat by +5.41/+7.05
    /// IMPs/divergent.
    #[arg(long, default_value_t = false)]
    no_ns_rkcb_minors: bool,

    /// The keycard ask's relocation stance, `ReadingProfile::rkcb_variant` (opt-in, as in
    /// the crate): `redwood` relocates the minor asks only — 4♦ asks in clubs
    /// and 4♥ in diamonds, the majors keep plain 4NT — and `kickback` adds 4♠
    /// asking in hearts, so every 1430 answer lands at or below five of trump.
    /// The full ladder measured a loss under the configured net (see
    /// `RkcbVariant::Kickback`), so the default arm is plain 4NT.  Either
    /// relocation implies the minors' reach whatever `--no-ns-rkcb-minors`
    /// says.  Read at *build* time as well as classify time, so the flag must
    /// be parsed before the system is constructed.  (Replaces the old
    /// `--ns-kickback`/`--ns-redwood` bool pair, which silently accepted both
    /// at once.)
    #[arg(long, value_enum, default_value = "plain")]
    ns_rkcb: RkcbArg,

    /// Disable the longer-major transfer discipline (default on): the Jacoby
    /// transfer guards revert to the legacy tie (a 6♠5♥ hand could transfer to
    /// hearts; 3♦ fired on any 5-5+) — the A/B baseline arm.
    #[arg(long, default_value_t = false)]
    no_ns_transfer_longer: bool,

    /// Disable the control-bid reading of high new-suit bids (default on,
    /// M6.4): a four-plus-level new suit reverts to the pre-M6.4 reading
    /// (double jumps skipped) and the return-to-trump signoff never fires.
    #[arg(long, default_value_t = false)]
    no_ns_control_bid_reading: bool,

    /// Disable the cue reading of the natural walk (shipped default-on
    /// 2026-07-18, bid-inert): a bid of a suit only the opponents have
    /// naturally shown is a cue, never a holding.
    #[arg(long, default_value_t = false)]
    no_ns_cue_reading: bool,

    /// Disable sound natural length floors (shipped default-on 2026-07-18:
    /// plain wash + PD win on both references): opener's immediate two-level
    /// rebid of the opened suit reads 5+ not 6+, an agreed-suit re-raise adds
    /// no length, and a doubler's later jump is never a weak six-card jump.
    #[arg(long, default_value_t = false)]
    no_ns_length_soundness: bool,

    /// Disable table-wide alert reading (shipped default-on 2026-07-18,
    /// bid-inert): the opponents' alerted calls decode off their authoring
    /// rules — modeling them as playing our books, an approximation against
    /// BBA — instead of falling to the natural walk.
    #[arg(long, default_value_t = false)]
    no_ns_table_alert_reading: bool,

    /// Disable the pass reading (shipped default-on 2026-07-18, bid-inert):
    /// each pass at an authored node reads as its table's own Pass gate — the
    /// negative inference of declining every other call (no-open ≤ 11 points,
    /// silent responder ≤ 5 HCP, direct seat ≤ 17 HCP).  Opponents' passes
    /// also need table-wide alert reading on.
    #[arg(long, default_value_t = false)]
    no_ns_pass_reading: bool,

    /// Document the shape-free 17+ tier's complement (`points(..17)`) on the
    /// direct-seat pass over their weak two, so it projects a band instead of ⊤
    /// on all five axes the nets read (default off — REFUTED, plain DD −0.0028
    /// ± 0.0017 NV; see `defense.weak_two_pass_gate`).
    #[arg(long, default_value_t = false)]
    ns_weak_two_pass_gate: bool,

    /// Widen our 2NT overcall of their weak two from strict `balanced()`
    /// (4333/4432/5332 only) to 2-4 majors / 2-6 minors, so a 6322 with a
    /// stopper and a long minor can bid it (default off; see
    /// `defense.weak_two_notrump_shape`).
    #[arg(long, default_value_t = false)]
    ns_weak_two_notrump_shape: bool,

    /// Author our jump in a new suit below 3NT over their weak two — 6+ cards
    /// and three more points than the natural overcall (default off; see
    /// `defense.weak_two_jump_overcall`).
    #[arg(long, default_value_t = false)]
    ns_weak_two_jump_overcall: bool,

    /// Author our direct cue of their *major* weak two as Michaels — the other
    /// major plus a minor, 5-5 (default off; see `defense.weak_two_cue`).
    #[arg(long, default_value_t = false)]
    ns_weak_two_cue: bool,

    /// Author advancer's Gladiator structure over our 2NT overcall of their
    /// weak two in a major — 3♣ relay, cue = Stayman, 3♦+ game-forcing
    /// (default off; see `defense.weak_two_notrump_advances_enabled`).
    #[arg(long, default_value_t = false)]
    ns_weak_two_nt_advances: bool,

    /// Turn OFF the *vulnerable* discipline on our suit overcall of their weak
    /// two (`defense.weak_two_overcall_discipline`, crate default ON): let the flat
    /// band apply at every vulnerability instead of demanding 12–17 at the two
    /// level and 15–17 at the three.  This is the A/B off-arm.
    #[arg(long, default_value_t = false)]
    no_ns_weak_two_overcall_discipline: bool,

    /// Inclusive `hcp` band `LO:HI` of our 2NT overcall of their weak two
    /// (default 16:17; BBA's own bucket is 15–17)
    #[arg(long, default_value = "16:17")]
    ns_weak_two_nt_points: String,

    /// Inclusive `points` bands `LO2:HI2:LO3:HI3` of our natural suit overcall
    /// of their weak two, split by the level it lands on (default 10:16:10:16 —
    /// the shipped flat band at both levels)
    #[arg(long, default_value = "10:16:10:16")]
    ns_weak_two_overcall: String,

    /// Advertise that our defense to BBA's 1NT is natural.  At *our* table only the
    /// opponent bot's 1NT-defense conventions are disabled (`Multi-Landy`/
    /// `Cappelletti`/`Landy` off), so BBA reads our two-level overcalls as natural.
    /// The all-BBA reference table keeps BBA's genuine Multi-Landy.
    #[arg(long, default_value_t = false)]
    advertise_natural: bool,

    /// Advertise that our defense to BBA's 1NT is **Landy** (pairs with `--ns-landy`).
    /// At *our* table the opponent bot keeps `Landy` on and `Multi-Landy`/`Cappelletti`
    /// off, so BBA reads our `2♣` as both majors and our `2♦`/`2♥`/`2♠` as natural — the
    /// honest disclosure of the Landy overlay (vs `--advertise-natural`, which would
    /// misread `2♣` as clubs).  Mutually exclusive with `--advertise-natural`.
    #[arg(long, default_value_t = false)]
    advertise_landy: bool,

    /// Disable the settle floor ("pass = play the top bid" over a takeout double,
    /// default on) to A/B the floor change's effect on defense.
    #[arg(long, default_value_t = false)]
    no_settle_floor: bool,

    /// HCP floor at which a strong-1NT responder forces game off the floor in an
    /// undisturbed auction (default 9, closing the post-transfer seam where a
    /// 9-count five-card-major game force transfers then stalls). Set 10 to restore
    /// the old floor for the A/B.
    #[arg(long, default_value_t = 9)]
    ns_nt_responder_game_floor: u8,

    /// Re-enable responder's 3NT game force over a double of our 1NT (off by
    /// default — we defend the unlimited business XX / escape a long suit instead).
    /// Off-switch for the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_suppress_nt_gf_over_double: bool,

    /// Author responder's gambling 3NT over a double of our 1NT — a long (6+)
    /// minor, semi-solid, with an outside ace.  Opt-in A/B knob, off by default.
    #[arg(long, default_value_t = false)]
    ns_gambling_3nt: bool,

    /// Semi-solid top-honor floor for the gambling 3NT's minor (`0` = length only).
    #[arg(long, default_value_t = 2)]
    ns_gambling_3nt_top_honors: u8,

    /// Drop the outside-ace requirement on the gambling 3NT (A/B the ace gate).
    #[arg(long, default_value_t = false)]
    no_ns_gambling_3nt_ace: bool,

    /// Author responder's preemptive 4M over a double of our 1NT — a quality long
    /// (6+) major (semi-solid, trump ace) plus a modest hand.  Opt-in, off by default.
    #[arg(long, default_value_t = false)]
    ns_preempt_4m: bool,

    /// Semi-solid top-honor floor for the preemptive 4M's major (`0` = length only).
    #[arg(long, default_value_t = 2)]
    ns_preempt_4m_top_honors: u8,

    /// Drop the trump-ace requirement on the preemptive 4M (A/B the ace gate).
    #[arg(long, default_value_t = false)]
    no_ns_preempt_4m_ace: bool,

    /// Suppress opener correcting partner's choice-of-games 3NT to 4M with a known
    /// eight-card major fit.  Gated on undisturbed + a ruffing doubleton it wins
    /// +0.0062 IMPs/board plain / +0.0068 PD (two seeds), so it is on by default.
    #[arg(long, default_value_t = false)]
    no_ns_correct_3nt_to_major: bool,
}

/// Parse a `NAME=0|1` convention override for `--our-conv` / `--their-conv`
/// CLI face of [`pons::bidding::instinct::RkcbVariant`] — clap's `ValueEnum`
/// cannot be derived for the foreign type.
#[derive(Clone, Copy, clap::ValueEnum)]
enum RkcbArg {
    Plain,
    Redwood,
    Kickback,
}

impl From<RkcbArg> for pons::bidding::instinct::RkcbVariant {
    fn from(arg: RkcbArg) -> Self {
        match arg {
            RkcbArg::Plain => Self::Plain,
            RkcbArg::Redwood => Self::Redwood,
            RkcbArg::Kickback => Self::Kickback,
        }
    }
}

fn parse_override(spec: &str) -> Result<(CString, c_int), String> {
    let (name, value) = spec
        .rsplit_once('=')
        .ok_or("expected NAME=0|1 (e.g. \"Rubensohl after 1m=1\")")?;
    let on = match value.trim() {
        "0" => 0,
        "1" => 1,
        other => return Err(format!("value must be 0 or 1, got `{other}`")),
    };
    let name = CString::new(name.trim()).map_err(|_| "name has an interior NUL".to_string())?;
    Ok((name, on))
}

/// What to declare to the BBA seats, per `--disclose` and `--disclose-conv`
///
/// Call **after** every `--ns-*` knob is written into `armed`: a generated card
/// is a function of that [`Agreements`] value, so building it early would
/// describe a system this run then reconfigures. The whole armed value flows
/// into the generator — card rows read `decision.reading.*` and `rebid.*` too,
/// so rebuilding from `Agreements::default()` here would disclose an arm's
/// *default* Landy/splinter/Stayman/NMF/Kickback rows whatever the arm plays
/// (the fe3a35e regression: the thread-local era's `Agreements::current()`
/// carried those halves implicitly, and the field migration dropped them).
///
/// A `--our-floor` with no card generator is a hard error rather than a fall
/// back to American's card or to silence — disclosing the wrong card
/// misdescribes us to BBA far more damagingly than disclosing nothing, and
/// silently reverting to blind would make the two arms of a cross-system A/B
/// incomparable.
fn disclosure(args: &Args, armed: &Agreements) -> anyhow::Result<Option<EpbotCard>> {
    let mut card = match args.disclose.as_str() {
        "off" => return Ok(None),
        // A BBA-vs-BBA arm does not play our authored system at all, so the
        // generated card would describe a system nobody at the table is
        // bidding.  Leave those seats at their EPBot defaults, which is what
        // every such A/B measured before disclosure defaulted on.
        "generated" if args.our_system.is_some() => return Ok(None),
        "generated" => {
            // The floor names the system; `-book`/`-instinct`/`-floor` variants
            // differ only in the floor, which no card row can express.
            let card = match args.our_floor.split('-').next().unwrap_or_default() {
                "american" => pons::bidding::card::american_card(armed),
                "dutch" => pons::bidding::card::dutch_card(armed),
                other => anyhow::bail!(
                    "--disclose generated: no card generator for system `{other}` \
                     (known: american, dutch).  Write one in `src/bidding/card.rs` \
                     rather than disclosing another system's card."
                ),
            };
            EpbotCard {
                system: card.system,
                toggles: card
                    .rows
                    .iter()
                    .map(|(name, value)| {
                        (
                            CString::new(*name).expect("a schema name has no NUL"),
                            *value as c_int,
                        )
                    })
                    .collect(),
            }
        }
        file => load_bbsa(file)?,
    };
    card.toggles.extend(args.disclose_conv.iter().cloned());
    Ok(Some(card))
}

/// EPBot system label for the indices we use (the pinned `vendor/bba` build)
fn system_label(system: c_int) -> &'static str {
    match system {
        0 => "2/1 Game Force",
        2 => "WJ (Polish Club)",
        _ => "EPBot system",
    }
}

/// Render a loaded `.bbsa` card for a side's label, e.g. ` [card: BEN-21GF.bbsa]`
fn label_card(card: &Option<String>) -> String {
    card.as_deref()
        .map(|file| format!(" [card: {file}]"))
        .unwrap_or_default()
}

/// Render convention overrides for a side's label, e.g. ` [Rubensohl after 1m=1]`
fn label_overrides(overrides: &[(CString, c_int)]) -> String {
    overrides
        .iter()
        .map(|(name, value)| format!(" [{}={value}]", name.to_string_lossy()))
        .collect()
}

// The BBA oracle (`BbaOracle`) and the `&dyn Bidder` match drivers
// (`next_call`/`bid_out`) now live in `common::oracle`, shared with `ben-gen`.

// ---------------------------------------------------------------------------
// The 1NT pre-filters that shape which boards are generated
// ---------------------------------------------------------------------------

/// Balanced (no singleton/void, at most one doubleton) with 15-17 HCP — a strict
/// 1NT-opener gate for the cheap `--filter-1nt` pre-filter.
fn is_1nt_opener(hand: Hand) -> bool {
    let len = Suit::ASC.map(|s| hand[s].len());
    let balanced = len.iter().all(|&l| l >= 2) && len.iter().filter(|&&l| l == 2).count() <= 1;
    balanced && (15..=17).contains(&hand_hcp(hand))
}

/// Landy-shaped: 4+ in both majors with the longer 5+, and overcall values —
/// the raw-hand twin of `landy_2c`'s `five_four(♥, ♠) & points(..)`
/// (`src/bidding/american/defense/nt_landy.rs`), for `--filter-landy`.
///
/// HCP rather than upgraded points, and 8+ uncapped, because this gates a
/// *scan*, not a call: it must not reject a hand the opponents' own band would
/// overcall on.  See `--filter-landy` for why loose is the safe direction.
fn is_landy_shaped(hand: Hand) -> bool {
    let (hearts, spades) = (hand[Suit::Hearts].len(), hand[Suit::Spades].len());
    hearts.min(spades) >= 4 && hearts.max(spades) >= 5 && hand_hcp(hand) >= 8
}

/// Preempt-shaped: a seven-card suit and no more than 12 HCP — the raw-hand
/// twin of BBA's natural three-level overcall of a 1NT opening, for
/// `--filter-preempt`.
///
/// HCP rather than upgraded points, and no floor, because this gates a *scan*,
/// not a call.  See `--filter-preempt` for why loose is the safe direction.
fn is_preempt_shaped(hand: Hand) -> bool {
    Suit::ASC.iter().any(|&suit| hand[suit].len() >= 7) && hand_hcp(hand) <= 12
}

/// If this auction's *opening* call is 1NT, its index and whether the opener is
/// North/South.  The opening requirement (all prior calls passes) excludes a
/// `1♣ - 1NT` rebid — we want 1NT *openings* only. Used by `--isolate-defense`
/// to keep only BBA-opens-1NT / we-defend boards.
fn opening_1nt(auction: &[Call], dealer: Seat) -> Option<(usize, bool)> {
    let one_nt = Call::Bid(Bid::new(1, Strain::Notrump));
    let index = auction.iter().position(|&call| call == one_nt)?;
    if auction[..index].iter().any(|&call| call != Call::Pass) {
        return None;
    }
    let opener_ns = matches!(seat_to_act(dealer, index), Seat::North | Seat::South);
    Some((index, opener_ns))
}

/// Parse a `--their-ns` / `--declare-as` string as a second `bba-gen` command
/// line
///
/// Whitespace-split, no quoting: every `--ns-*` flag is a bare word or a
/// `LO:HI`-style value, so a shell-grade splitter would buy nothing.  Nesting is
/// refused — one level of "and the opponents play…" is the whole feature.
fn seat_args(spec: &str) -> anyhow::Result<Args> {
    let args = Args::try_parse_from(core::iter::once("bba-gen").chain(spec.split_whitespace()))?;
    anyhow::ensure!(
        args.their_ns.is_none() && args.declare_as.is_none(),
        "--their-ns / --declare-as do not nest"
    );
    Ok(args)
}

/// Arm every knob one seat's `--ns-*` (and friends) describe
///
/// Factored out of `main` so a *second* seat can be armed the same way:
/// `--their-ns` parses its own `Args` and this applies it, then `main`
/// re-applies its own to restore.  Every knob here is read at book-build
/// time, so each returned value carries one seat's complete arm.
///
/// Returns the [`Agreements`] this seat plays. A build must be handed *this*,
/// not a fresh `Agreements::default()`.
#[allow(clippy::too_many_lines)]
/// Derive whether their `2♣` overcall of our 1NT shows both majors — the
/// disclosure that engages our Landy counter (`Agreements::their`)
///
/// Not a knob of ours: what their `2♣` means is a fact about the opponents,
/// so it is read off their declaration.  Precedence:
///
/// 1. `--their-2c-landy [true|false]` — explicit operator override.
/// 2. An explicit declaration: any of the 1NT-defense rows (`Multi-Landy`,
///    `Landy`, `Cappelletti`) present in `--their-card`/`--their-conv` is
///    played to at **face value** — both-majors rows engage the counter, a
///    declared Cappelletti or no-Landy set reads natural.  A bot that bids
///    Landy behind a declared no-Landy card commits *its* infraction, not
///    ours.
/// 3. No declaration: the 2/1 reference's **measured behavior** — Woolsey
///    Multi-Landy (the 551-board census, docs/one-notrump-competitive.md).
///    Its own card cannot serve here: `21GF.bbsa` declares `Cappelletti=1,
///    Landy=0, Multi-Landy=0` while the engine bids Multi-Landy regardless,
///    so the census outranks the card.  Other `--system`s have no census and
///    default natural.
fn their_2c_landy(args: &Args) -> anyhow::Result<bool> {
    if let Some(forced) = args.their_2c_landy {
        return Ok(forced);
    }
    // The effective declaration: card toggles first, `--their-conv` singles
    // on top (same precedence as the oracle load in `main`), so a reversed
    // search sees the strongest override first.
    let mut declared = match &args.their_card {
        Some(file) => load_bbsa(file)?.toggles,
        None => Vec::new(),
    };
    declared.extend(args.their_conv.iter().cloned());
    let row = |name: &[u8]| {
        declared
            .iter()
            .rev()
            .find(|(n, _)| n.as_bytes() == name)
            .map(|&(_, v)| v != 0)
    };
    let rows = [row(b"Multi-Landy"), row(b"Landy"), row(b"Cappelletti")];
    Ok(if rows.iter().all(Option::is_none) {
        args.system == SYSTEM_2_OVER_1
    } else {
        rows[0].unwrap_or(false) || rows[1].unwrap_or(false)
    })
}

/// Derive whether their `2♦` overcall of our 1NT is a Multi — the disclosure
/// that engages the N4 Multi table (`Agreements::their`)
///
/// Same channel and precedence as [`their_2c_landy`], and since N4 shipped
/// (v7, 2026-08-15, docs/one-notrump-competitive.md §N4) the same census
/// default at the bottom: BBA's 2/1 reference bids the Multi
/// (`docs/ai-bidder/bba-multi-2d.md`) whatever its card says.  The pre-ship
/// arm is spelled `--their-2d-multi false`.
///
/// 1. `--their-2d-multi [true|false]` — explicit operator override.
/// 2. An explicit `Multi-Landy` row in `--their-card`/`--their-conv`, at
///    face value (the only row of the family whose `2♦` is a Multi); a
///    declared family without it (`Landy`/`Cappelletti` rows only) is a
///    declared no-Multi, read as such.
/// 3. No declaration at all: the 2/1 reference's measured behavior.
fn their_2d_multi(args: &Args) -> anyhow::Result<bool> {
    if let Some(forced) = args.their_2d_multi {
        return Ok(forced);
    }
    let mut declared = match &args.their_card {
        Some(file) => load_bbsa(file)?.toggles,
        None => Vec::new(),
    };
    declared.extend(args.their_conv.iter().cloned());
    let row = |name: &[u8]| {
        declared
            .iter()
            .rev()
            .find(|(n, _)| n.as_bytes() == name)
            .map(|&(_, v)| v != 0)
    };
    let rows = [row(b"Multi-Landy"), row(b"Landy"), row(b"Cappelletti")];
    Ok(if rows.iter().all(Option::is_none) {
        args.system == SYSTEM_2_OVER_1
    } else {
        rows[0].unwrap_or(false)
    })
}

fn arm_knobs(args: &Args) -> anyhow::Result<Agreements> {
    // Our side: the authored floor by default, or a second EPBot card when
    // `--our-system` is given (the BBA-vs-BBA experiment).
    // Written on both arms, not just when on: every returned value is a
    // complete, independently buildable seat configuration.
    let mut agreements = Agreements::default();
    agreements.instinct.doubler_xx_runout = !args.no_ns_doubler_run;
    agreements.decision.eval_auction = !args.no_ns_eval_auction;
    agreements.decision.eval_shape = args.ns_eval_shape;
    agreements.decision.blind_inference = args.ns_blind_inference;
    let (oc_lo, oc_hi) = args
        .ns_overcall
        .split_once(':')
        .and_then(|(lo, hi)| Some((lo.parse::<u8>().ok()?, hi.parse::<u8>().ok()?)))
        .ok_or_else(|| {
            anyhow::anyhow!("--ns-overcall must be LO:HI, got {:?}", args.ns_overcall)
        })?;
    agreements.decision.transfer_gf_majors = !args.no_ns_transfer_gf_majors;
    agreements.decision.transfer_gf_hearts = !args.no_ns_transfer_gf_hearts;
    agreements.defense.suppress_flat_4333_takeout = !args.no_ns_suppress_flat_4333_takeout;
    agreements.defense.suppress_5332_takeout = !args.no_ns_suppress_5332_takeout;
    agreements.defense.suppress_4432_vs_major = args.ns_suppress_4432_vs_major;
    agreements.defense.suppress_4432_vs_minor = args.ns_suppress_4432_vs_minor;
    agreements.defense.suppress_5card_major_takeout = !args.no_ns_suppress_5card_major_takeout;
    agreements.defense.suppress_long_minor_takeout = args.ns_suppress_long_minor_takeout;
    agreements.defense.defensive_seam_split = args.ns_defensive_seam_split;
    agreements.decision.two_over_one_force = !args.no_ns_two_over_one_force;
    agreements.instinct.competitive_rebid = !args.no_ns_competitive_rebid;
    agreements.instinct.reopening_notrump = !args.no_ns_reopening_notrump;
    // One system, one write — the payloads then apply to whichever family owns
    // them.  No forced-off block: the cell holds exactly one variant, so
    // selecting a family already deselects the rest.
    let ns_defense = pons::bidding::american::NotrumpDefense::from(args.ns_notrump_defense);
    let mut woolsey_range = None;
    if ns_defense == pons::bidding::american::NotrumpDefense::Woolsey {
        let (wlo, whi) = args
            .ns_woolsey_range
            .split_once(':')
            .and_then(|(lo, hi)| Some((lo.parse::<u8>().ok()?, hi.parse::<u8>().ok()?)))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "--ns-woolsey-range must be LO:HI, got {:?}",
                    args.ns_woolsey_range
                )
            })?;
        woolsey_range = Some((wlo, whi));
    }
    // The Landy overlay rides its own field and the engine honours it only
    // under `natural`/`off`, so it needs no ordering against the family above.
    let landy_range = if let Some(spec) = &args.ns_landy {
        let (lo, hi) = spec
            .split_once(':')
            .and_then(|(lo, hi)| Some((lo.parse::<u8>().ok()?, hi.parse::<u8>().ok()?)))
            .ok_or_else(|| anyhow::anyhow!("--ns-landy must be LO:HI, got {spec:?}"))?;
        Some((lo, hi))
    } else {
        None
    };
    // Disclosure last: every `--ns-*` knob above moves the system, and a
    // generated card reads them.  Built here rather than beside the oracle so
    // the card cannot describe a system the run then reconfigures.
    agreements.decision.reading.penalty_latch = !args.no_ns_penalty_latch;
    agreements.decision.reading.pdi_latch = args.ns_pdi_latch;
    agreements.decision.reading.union_hull = args.ns_union_hull;
    agreements.decision.reading.rubens_advances = args.ns_rubens;
    agreements.decision.reading.floor_rkcb = !args.no_ns_floor_rkcb;
    agreements.decision.reading.rkcb_variant = args.ns_rkcb.into();
    agreements.decision.reading.two_notrump_wide = args.ns_two_nt_wide;
    agreements.decision.reading.natural_double_floor = args.ns_double_floor;
    agreements.decision.reading.nt_overcall_systems_on = !args.no_ns_nt_overcall_systems_on;
    agreements.decision.reading.nt_overcall_gladiator = args.ns_nt_overcall_gladiator;
    agreements.decision.reading.natural_overcall_points = (oc_lo, oc_hi);
    agreements.decision.reading.garbage_stayman = !args.no_ns_garbage_stayman;
    agreements.decision.reading.crawling_stayman = !args.no_ns_crawling_stayman;
    agreements.decision.reading.longer_major_response = !args.no_ns_longer_major_response;
    agreements.decision.reading.xyz = !args.no_ns_xyz;
    agreements.decision.reading.opener_extras_ladder = !args.no_ns_opener_extras_ladder;
    agreements.decision.reading.opener_major_jump_rebid = !args.no_ns_opener_major_jump_rebid;
    agreements.decision.reading.notrump_defense = ns_defense;
    if args.ns_european_minors {
        agreements.decision.reading.notrump_minors = pons::bidding::american::EUROPEAN;
    }
    if let Some(range) = woolsey_range {
        agreements.decision.reading.convention_points = range;
        agreements.decision.reading.woolsey_double_floor = args.ns_woolsey_x_floor;
    }
    agreements.decision.reading.landy = landy_range.is_some();
    if let Some(range) = landy_range {
        // Landy and Woolsey share one strength band, so `--ns-landy LO:HI`
        // names it; passed after `--ns-woolsey-range`, it wins, as it did
        // when `set_landy` wrote the Woolsey cell.
        agreements.decision.reading.convention_points = range;
    }
    agreements.decision.reading.rubens_transfer = !args.no_ns_rubens_reading;
    agreements.decision.reading.control_bid = !args.no_ns_control_bid_reading;
    agreements.decision.reading.cue = !args.no_ns_cue_reading;
    agreements.decision.reading.length_soundness = !args.no_ns_length_soundness;
    agreements.decision.reading.table_alerts = !args.no_ns_table_alert_reading;
    agreements.decision.reading.pass = !args.no_ns_pass_reading;
    agreements.decision.reading.fallback_projection = !args.no_ns_fallback_projection;
    agreements.decision.reading.envelope_union = !args.no_ns_envelope_union;
    agreements.decision.reading.gauge_membership = args.ns_gauge_membership;
    agreements.decision.reading.announced = args.ns_announced_reading;
    if let Some(v) = args.ns_their_landy_read {
        agreements.decision.reading.their_landy_reading = v;
    }
    if let Some(scope) = args.ns_reading_scope {
        agreements.decision.reading.scope = scope.into();
    }
    agreements.decision.reading.sum_closure = args.ns_sum_closure;
    if let Some(v) = args.ns_upgrade_closure {
        agreements.decision.reading.upgrade_closure = v;
    }
    if let Some(v) = args.ns_strength_ceilings {
        agreements.decision.reading.strength_ceilings = v;
    }
    if let Some(v) = args.ns_bid_exclusion {
        agreements.decision.reading.bid_exclusion = v;
    }
    if let Some(v) = args.ns_forcing_ceiling_read {
        agreements.decision.instinct.forcing_ceiling_read = v;
    }
    if let Some(v) = args.ns_nt_hcp {
        agreements.decision.instinct.nt_hcp_read = v;
    }
    if let Some(v) = args.ns_fit_sum_support {
        agreements.decision.instinct.fit_sum_support_read = v;
    }
    agreements.decision.instinct.uvu_encircle = !args.no_uvu;
    agreements.decision.instinct.settle_floor = !args.no_settle_floor;
    agreements.decision.instinct.nt_responder_game_floor = args.ns_nt_responder_game_floor;
    agreements.decision.instinct.suppress_nt_gf_over_double =
        !args.no_ns_suppress_nt_gf_over_double;
    agreements.decision.instinct.gambling_3nt_over_double = args.ns_gambling_3nt;
    agreements.decision.instinct.gambling_3nt_top_honors = args.ns_gambling_3nt_top_honors;
    agreements.decision.instinct.gambling_3nt_require_ace = !args.no_ns_gambling_3nt_ace;
    agreements.decision.instinct.preempt_4m_over_double = args.ns_preempt_4m;
    agreements.decision.instinct.preempt_4m_top_honors = args.ns_preempt_4m_top_honors;
    agreements.decision.instinct.preempt_4m_require_ace = !args.no_ns_preempt_4m_ace;
    agreements.decision.instinct.correct_3nt_to_major = !args.no_ns_correct_3nt_to_major;
    agreements.decision.instinct.penalty_no_pull = !args.ns_allow_pull;
    agreements.decision.instinct.advancer_xx_runout = !args.no_ns_xx_runout;
    agreements.decision.instinct.keycard_minors = !args.no_ns_rkcb_minors;
    agreements.decision.instinct.accountant_floor = !args.no_ns_accountant;
    agreements.decision.instinct.net_collar = args.ns_net_collar;
    agreements.decision.instinct.competitive_accountant = !args.no_ns_competitive_accountant;
    agreements.decision.instinct.two_over_one_slam_strength =
        !args.no_ns_two_over_one_slam_strength;
    agreements.decision.instinct.rein_advance_raise = !args.no_ns_rein_advance_raise;
    // The competitive book.  `--uvu-x-floor` / `--uvu-cue-floor` still ride
    // inside the UvU gate, as they did as cells; the difference is that a
    // gated-off floor now stays at the shipped default instead of keeping
    // whatever the last-armed seat left on the thread.
    agreements.competition.uvu = !args.no_uvu;
    if !args.no_uvu {
        agreements.competition.uvu_x_floor = args.uvu_x_floor;
        agreements.competition.uvu_cue_floor = args.uvu_cue_floor;
    }
    agreements.competition.two_diamond_double = args
        .ns_2d_double
        .as_deref()
        .map(|spec| {
            let mut parts = spec.split(':');
            let mut next = || parts.next().and_then(|p| p.parse::<u8>().ok());
            let gate = (next()?, next()?, next()?);
            parts.next().is_none().then_some(gate)
        })
        .map(|gate| {
            gate.map(|(len, suit_hcp, floor)| (usize::from(len), suit_hcp, floor))
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "--ns-2d-double must be LEN:SUITHCP:HCP, got {:?}",
                        args.ns_2d_double
                    )
                })
        })
        .transpose()?;
    agreements.decision.their.two_clubs_landy = their_2c_landy(args)?;
    agreements.decision.their.two_diamonds_multi = their_2d_multi(args)?;
    if let Some(read) = args.ns_their_multi_read {
        agreements.decision.reading.their_multi_reading = read;
    }
    agreements.decision.reading.their_multi_advance_reading = args.ns_their_multi_advance_read;
    agreements.decision.reading.their_multi_double_reading = args.ns_their_multi_double_read;
    agreements.competition.defense_2c_landy_cues = args.defense_2c_landy_cues;
    agreements.competition.defense_2c_landy_low_minors = args.defense_2c_landy_low_minors;
    agreements.competition.defense_2c_landy_hcp_rungs = args.defense_2c_landy_hcp_rungs;
    // The stack knobs are engine-default ON (2026-08-14): only an explicit
    // flag overrides, so a plain vs-BBA run generates the shipped stack and a
    // pre-ship arm is spelled `--defense-2c-landy-<knob> false`.
    if let Some(v) = args.defense_2c_landy_transfer {
        agreements.competition.defense_2c_landy_transfer = v;
    }
    if let Some(v) = args.defense_2c_landy_cue_floor {
        agreements.competition.defense_2c_landy_cue_floor = v;
    }
    if let Some(v) = args.defense_2c_landy_fit_answers {
        agreements.competition.defense_2c_landy_fit_answers = v;
    }
    if let Some(v) = args.defense_2c_landy_competition {
        agreements.competition.defense_2c_landy_competition = v;
    }
    if let Some(v) = args.defense_2c_landy_bba {
        agreements.competition.defense_2c_landy_bba = v;
    }
    if let Some(v) = args.defense_2c_landy_weak_2d_cap {
        agreements.competition.defense_2c_landy_weak_2d_cap = v;
    }
    if let Some(v) = args.ns_completion_alerts {
        agreements.decision.reading.completion_alerts = v;
    }
    agreements.competition.competition_over_stayman = !args.no_ns_comp_over_stayman;
    agreements.competition.competitive_4333 = match args.ns_competitive_4333.as_str() {
        "allow" => pons::bidding::american::Competitive4333::Allow,
        "suppress" => pons::bidding::american::Competitive4333::Suppress,
        "suppress-stopper" => pons::bidding::american::Competitive4333::SuppressWithStopper,
        other => {
            anyhow::bail!(
                "--ns-competitive-4333 must be allow|suppress|suppress-stopper, got {other:?}"
            )
        }
    };
    agreements.competition.multi_stopper_ask = match args.ns_multi_stopper_ask.as_str() {
        "off" => pons::bidding::american::MultiStopperAsk::Off,
        "search" => pons::bidding::american::MultiStopperAsk::FitSearch,
        "place" => pons::bidding::american::MultiStopperAsk::OpenerPlaces,
        other => anyhow::bail!("--ns-multi-stopper-ask must be off|search|place, got {other:?}"),
    };
    agreements.competition.multi_weak_escape =
        match args.ns_multi_weak_escape.as_str() {
            "off" => None,
            n => Some(n.parse().map_err(|_| {
                anyhow::anyhow!("--ns-multi-weak-escape must be off|6|5, got {n:?}")
            })?),
        };
    agreements.competition.multi_balance = args.ns_multi_balance;
    agreements.competition.multi_kokish_kraft = !args.no_ns_multi_kokish_kraft;
    agreements.competition.multi_minor_slam_try = match args.ns_multi_minor_slam_try.as_str() {
        "off" => None,
        n => Some(
            n.parse()
                .expect("--ns-multi-minor-slam-try must be off or a points floor"),
        ),
    };
    agreements.competition.multi_doubler_major = args.ns_multi_doubler_major;
    agreements.notrump.minor_transfer_slam_try = match args.ns_minor_transfer_slam_try.as_str() {
        "off" => None,
        n => Some(n.parse().map_err(|_| {
            anyhow::anyhow!("--ns-minor-transfer-slam-try must be off|POINTS, got {n:?}")
        })?),
    };
    agreements.notrump.minor_transfer_slam_fit = !args.no_ns_minor_transfer_slam_fit;
    agreements.competition.landy_minor_slam_answer = !args.no_ns_landy_minor_slam_answer;
    agreements.competition.competition_over_transfer = args.ns_comp_over_transfer;
    agreements.competition.cue_raise_answer = !args.no_ns_cue_raise_answer;
    agreements.competition.cue_minor_raise_answer = !args.no_ns_cue_minor_raise_answer;
    agreements.competition.uvu_over_majors = !args.no_ns_uvu_over_majors;
    agreements.competition.uvu_over_minors = args.ns_uvu_over_minors;
    agreements.competition.weak_two_competition = args.ns_weak_two_comp;
    agreements.competition.strong_two_competition = !args.no_ns_strong_two_comp;
    agreements.competition.major_support_double = !args.no_ns_major_support_double;
    agreements.competition.free_bids = args.ns_free_bids;
    agreements.competition.free_bid_floor = args.ns_free_bid_floor;
    agreements.competition.free_1nt_floor = args.ns_free_1nt_floor;
    agreements.competition.free_bid_quality = args.ns_free_bid_quality;
    agreements.competition.negative_double_shape = match args.ns_negative_double_shape.as_str() {
        "both-majors" => pons::bidding::american::NegativeDoubleShape::BothMajors,
        "modern" => pons::bidding::american::NegativeDoubleShape::Modern,
        "cachalot" => pons::bidding::american::NegativeDoubleShape::Cachalot,
        "sputnik" => pons::bidding::american::NegativeDoubleShape::Sputnik,
        other => anyhow::bail!(
            "--ns-negative-double-shape must be both-majors|modern|cachalot|sputnik, got {other:?}"
        ),
    };
    agreements.competition.free_bid_style = match args.ns_free_bid_style.as_str() {
        "forcing" => pons::bidding::american::FreeBidStyle::Forcing,
        "negative" => pons::bidding::american::FreeBidStyle::Negative,
        "transfer" => pons::bidding::american::FreeBidStyle::Transfer,
        other => {
            anyhow::bail!("--ns-free-bid-style must be forcing|negative|transfer, got {other:?}")
        }
    };
    agreements.competition.high_overcall_responses = args.ns_high_overcall;
    if let Some(v) = args.ns_nt_high_overcall {
        agreements.competition.nt_high_overcall_responses = v;
    }
    if let Some(v) = args.ns_nt_high_overcall_3nt_stopper {
        agreements.competition.nt_high_overcall_3nt_stopper = v;
    }
    if let Some(v) = args.ns_nt_3c_transfers {
        agreements.competition.nt_3c_transfers = v;
    }
    if let Some(v) = args.ns_nt_high_overcall_x_major_at_four {
        agreements.competition.nt_high_overcall_x_major_at_four = v;
    }
    if let Some(v) = args.ns_nt_high_overcall_x_leave_in {
        agreements.competition.nt_high_overcall_x_leave_in = v;
    }
    if let Some(v) = args.ns_nt_high_overcall_x_leave_in_three {
        agreements.competition.nt_high_overcall_x_leave_in_three = v;
    }
    if let Some(v) = args.ns_direct_3nt_stopper {
        agreements.competition.direct_3nt_stopper = v;
    }
    agreements.competition.cachalot_contested_x = !args.no_ns_cachalot_contested_x;
    agreements.competition.jordan_truscott = !args.no_ns_jordan_truscott;
    agreements.competition.splinter_doubled = !args.no_ns_splinter_doubled;
    agreements.competition.competition_over_minor_transfer = !args.no_ns_comp_over_minor_transfer;
    agreements.competition.competition_over_diamond_transfer =
        !args.no_ns_comp_over_diamond_transfer;
    // The defensive book — what we play when they open.  Every field is
    // assigned, never merely set, for the same "arm, build, re-arm" reason the
    // cells were; the per-family payloads at the end now stay at the shipped
    // default when their family is not selected, instead of keeping whatever
    // the last-armed seat left on the thread.
    agreements.defense.weak_two_pass_gate = args.ns_weak_two_pass_gate;
    agreements.defense.weak_two_notrump_shape = args.ns_weak_two_notrump_shape;
    agreements.defense.weak_two_jump_overcall = args.ns_weak_two_jump_overcall;
    agreements.defense.weak_two_cue = args.ns_weak_two_cue;
    agreements.defense.weak_two_notrump_advances_enabled = args.ns_weak_two_nt_advances;
    agreements.defense.weak_two_overcall_discipline = !args.no_ns_weak_two_overcall_discipline;
    agreements.defense.weak_two_notrump_points = args
        .ns_weak_two_nt_points
        .split_once(':')
        .and_then(|(l, h)| Some((l.parse::<u8>().ok()?, h.parse::<u8>().ok()?)))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "--ns-weak-two-nt-points must be LO:HI, got {:?}",
                args.ns_weak_two_nt_points
            )
        })?;
    agreements.defense.weak_two_overcall_points = {
        let band: Vec<u8> = args
            .ns_weak_two_overcall
            .split(':')
            .map(|n| n.parse::<u8>())
            .collect::<Result<_, _>>()
            .map_err(|_| {
                anyhow::anyhow!(
                    "--ns-weak-two-overcall must be LO2:HI2:LO3:HI3, got {:?}",
                    args.ns_weak_two_overcall
                )
            })?;
        let [two_lo, two_hi, three_lo, three_hi]: [u8; 4] = band[..].try_into().map_err(|_| {
            anyhow::anyhow!(
                "--ns-weak-two-overcall must be LO2:HI2:LO3:HI3, got {:?}",
                args.ns_weak_two_overcall
            )
        })?;
        (two_lo, two_hi, three_lo, three_hi)
    };
    agreements.defense.natural_double_shape = match args.ns_double_shape.as_str() {
        "any" => DoubleShape::Any,
        "semi" => DoubleShape::SemiBalanced,
        "balanced" => DoubleShape::Balanced,
        other => anyhow::bail!("--ns-double-shape must be any|semi|balanced, got {other:?}"),
    };
    agreements.defense.natural_double_weight = args.ns_double_weight;
    agreements.defense.takeout_support = match args.ns_takeout_support.as_str() {
        "off" => pons::bidding::american::TakeoutSupport::Off,
        "lenient" => pons::bidding::american::TakeoutSupport::Lenient,
        "strict" => pons::bidding::american::TakeoutSupport::Strict,
        other => anyhow::bail!("--ns-takeout-support must be off|lenient|strict, got {other:?}"),
    };
    agreements.defense.overcall_discipline = match args.ns_overcall_discipline.as_str() {
        "on" => true,
        "off" => false,
        other => anyhow::bail!("--ns-overcall-discipline must be on|off, got {other:?}"),
    };
    agreements.defense.direct_weak_jump_overcall = !args.no_ns_direct_weak_jump_overcall;
    agreements.defense.passed_hand_overcall = !args.no_ns_passed_hand_overcall;
    agreements.defense.two_level_minor_overcall_tight = args.ns_two_level_minor_overcall_tight;
    agreements.defense.nt_overcall_no_major = args.ns_nt_overcall_no_major;
    agreements.defense.direct_minor_weak_jump_overcall =
        !args.no_ns_direct_minor_weak_jump_overcall;
    agreements.defense.notrump_balancing_enabled = args.ns_balancing;
    agreements.defense.natural_overcall_hcp_floor = args.ns_nt_overcall_hcp_floor;
    agreements.defense.natural_overcall_advance_enabled = !args.no_ns_nt_overcall_advance;
    agreements.defense.stayman_defense_enabled = args.ns_defense_to_their_stayman;
    agreements.defense.rich_advance_double_enabled = !args.no_ns_rich_advance;
    agreements.defense.advance_rubens_enabled = args.ns_advance_rubens;
    agreements.defense.advance_minor_jump_enabled = !args.no_ns_advance_minor_jump;
    agreements.defense.advance_2nt_continuation_enabled = !args.no_ns_advance_2nt_continuation;
    agreements.defense.longest_first_advance_enabled = !args.no_ns_longest_advance;
    agreements.defense.advance_pass_yield_major_enabled = args.ns_advance_pass_yield;
    agreements.defense.advance_sit_hcp_gate = args.ns_advance_sit_hcp;
    agreements.defense.transfer_defense_enabled = args.ns_transfer_defense;
    agreements.defense.minor_transfer_defense_enabled = args.ns_minor_transfer_defense;
    agreements.defense.diamond_transfer_defense_enabled = args.ns_diamond_transfer_defense;
    agreements.defense.stayman_defense_overcall = args
        .ns_staydef_overcall
        .split_once(':')
        .and_then(|(l, f)| Some((l.parse::<usize>().ok()?, f.parse::<u8>().ok()?)))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "--ns-staydef-overcall must be LEN:FLOOR, got {:?}",
                args.ns_staydef_overcall
            )
        })?;
    // Written on every family, not only the two that widen it: one value serves
    // both the conventional families and the natural overlay, so leaving it
    // alone would carry a widening into a family that never asked for one.
    // `(8, 13)` is the crate default.
    agreements.defense.unusual_notrump_range = Some(
        if matches!(
            ns_defense,
            pons::bidding::american::NotrumpDefense::DirectDont
                | pons::bidding::american::NotrumpDefense::Meckwell
        ) {
            (8, 14)
        } else {
            (8, 13)
        },
    );
    match ns_defense {
        pons::bidding::american::NotrumpDefense::DirectDont => {
            agreements.defense.direct_dont_one_suiter_min = args.ns_dont_one_suiter_min;
            agreements.defense.direct_dont_four_four = args.ns_dont_four_four;
        }
        pons::bidding::american::NotrumpDefense::Meckwell => {
            agreements.defense.meckwell_minor_major_44 = args.ns_meckwell_minor_major_44;
            agreements.defense.meckwell_x_four_four = !args.ns_meckwell_x_five_four;
        }
        _ => {}
    }
    agreements.opening.open_one_notrump = !args.no_our_1nt;
    agreements.opening.one_notrump_fifths = args.nt_fifths;
    agreements.opening.multi_two_diamonds = args.ns_multi_2d;
    agreements.opening.multi_two_diamonds_champion = !args.no_ns_multi_2d_champion;
    agreements.response.up_the_line = !args.no_ns_up_the_line;
    agreements.response.major_choice_of_games = !args.no_ns_major_choice_of_games;
    agreements.response.two_over_one_fit = !args.no_ns_two_over_one_fit;
    agreements.response.two_over_one_gate = match args.ns_two_over_one_gate.as_str() {
        "points13" => pons::bidding::american::TwoOverOneGate::Points13,
        "hcp13" => pons::bidding::american::TwoOverOneGate::Hcp13,
        "hcp12" => pons::bidding::american::TwoOverOneGate::Hcp12,
        other => {
            anyhow::bail!("--ns-two-over-one-gate must be points13|hcp13|hcp12, got {other:?}")
        }
    };
    agreements.response.two_over_one_natural_lengths = args.ns_two_over_one_natural_lengths;
    agreements.response.two_over_one_major_discount = args.ns_two_over_one_major_discount;
    agreements.response.major_game_tries = !args.no_ns_major_game_tries;
    agreements.response.limit_raise_acceptance = !args.no_ns_limit_raise_acceptance;
    agreements.rebid.new_minor_forcing = args.ns_new_minor_forcing;
    agreements.rebid.balanced_1nt_rebid = !args.no_ns_balanced_1nt_rebid;
    agreements.rebid.major_rebid_tails = !args.no_ns_major_rebid_tails;
    agreements.rebid.fourth_suit_forcing = !args.no_ns_fourth_suit_forcing;
    agreements.game_force.second_suit_agreement = !args.no_ns_second_suit_agreement;
    agreements.game_force.opener_third = !args.no_ns_opener_third;
    agreements.game_force.game_backstop = args.ns_game_backstop;
    agreements.notrump.transfer_longer_major = !args.no_ns_transfer_longer;
    agreements.notrump.transfer_super_accept = args.ns_transfer_super_accept;
    agreements.notrump.transfer_slam_try = !args.no_ns_transfer_slam_try;
    agreements.notrump.texas_slam_drive = !args.no_ns_texas_slam_drive;
    agreements.notrump.minor_min_to_3nt = args.ns_minor_min_to_3nt;
    agreements.notrump.stayman_both_majors = !args.no_ns_stayman_both_majors;
    agreements.notrump.stayman_5card_max = !args.no_ns_stayman_5card_max;
    agreements.notrump.invitational_5card_majors = !args.no_ns_invitational_5card_majors;
    agreements.notrump.stayman_cue_continuation = !args.no_ns_stayman_cue_continuation;
    agreements.notrump.texas_game_floor = args.ns_texas_game_floor;
    agreements.notrump.sixcard_invite_floor = args.ns_sixcard_invite_floor;
    agreements.notrump.sixcard_accept_floor = args.ns_sixcard_accept_floor;
    Ok(agreements)
}

#[allow(clippy::too_many_lines)]
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
    // A full `.bbsa` card expands to convention overrides applied before the
    // explicit `--*-conv` singles, so singles override the card.
    let their_conv = match &args.their_card {
        Some(file) => {
            let card = load_bbsa(file)?;
            anyhow::ensure!(
                card.system == args.system,
                "`{file}` is system {}; pass `--system {}` to match",
                card.system,
                card.system,
            );
            let mut conv = card.toggles;
            conv.extend(args.their_conv.iter().cloned());
            conv
        }
        None => args.their_conv.clone(),
    };
    let (our_system, our_conv) = match &args.our_card {
        Some(file) => {
            let card = load_bbsa(file)?;
            if let Some(system) = args.our_system {
                anyhow::ensure!(
                    card.system == system,
                    "`{file}` is system {}, but --our-system says {system}",
                    card.system,
                );
            }
            let mut conv = card.toggles;
            conv.extend(args.our_conv.iter().cloned());
            (Some(card.system), conv)
        }
        None => (args.our_system, args.our_conv.clone()),
    };
    let bba = match BbaOracle::load(&path, args.system, their_conv.clone()) {
        Ok(bba) => bba,
        Err(error) => {
            eprintln!(
                "could not load EPBot native lib at `{path}`: {error}\n\
                 Fetch it with `git submodule update --init vendor/bba`, or set BBA_LIB."
            );
            std::process::exit(1);
        }
    };
    // When advertising natural, the opponent bot at *our* table reads our 1NT
    // overcalls naturally: disable its 1NT-defense conventions on top of
    // `--their-conv`.  Used only where `ours` defends; the all-BBA reference keeps
    // the plain `bba` (BBA's genuine Multi-Landy).
    anyhow::ensure!(
        !(args.advertise_natural && args.advertise_landy),
        "--advertise-natural and --advertise-landy are mutually exclusive"
    );
    let bba_vs_natural = if args.advertise_natural || args.advertise_landy {
        let mut conv = their_conv.clone();
        // Disclose our defense by setting how the opponent bot reads us: drop every
        // 1NT-defense convention, then (for Landy) re-enable just `Landy` so our `2♣`
        // reads as both majors and the rest natural.
        for name in ["Multi-Landy", "Cappelletti", "Landy"] {
            let on = (name == "Landy" && args.advertise_landy) as c_int;
            conv.push((CString::new(name).expect("a literal name has no NUL"), on));
        }
        // ⚠ Known gap (found 2026-08-15): this oracle never receives
        // `.with_opponents(disclosure)`, so beyond the three rows above it
        // falls back to modelling us as playing its own system (the
        // undeclared-opponents default in `oracle/mod.rs`).  Handing it the
        // disclosure card is NOT a safe one-line fix: the generated card
        // carries its own Landy-family rows, and the push order against these
        // load-time rows is unverified — a card push after load would clobber
        // the advertisement and silently break `--advertise-landy`.  Priced
        // as a known mis-model of the advertise-* lanes; fixing it needs a
        // probe of EPBot's row-push order first.
        Some(BbaOracle::load(&path, args.system, conv)?)
    } else {
        None
    };
    let agreements = arm_knobs(&args)?;
    let bba = bba.with_opponents(disclosure(&args, &agreements)?);
    // Read under *our* armed knobs, before the opponent seat borrows the
    // thread: this is the card whoever faces `--their-floor` is playing.
    let our_card = floor_card(&args.our_floor, &agreements)?;
    // The opponent seat's own command line, and the one we *say* it is playing.
    let their_ns = args.their_ns.as_deref().map(seat_args).transpose()?;
    let declared_as = args.declare_as.as_deref().map(seat_args).transpose()?;
    anyhow::ensure!(
        their_ns.is_none() || args.their_floor.is_some(),
        "--their-ns needs --their-floor: EPBot has no pons knobs to arm"
    );
    anyhow::ensure!(
        declared_as.is_none() || args.declare_opponents,
        "--declare-as moves the card handed to our net; it needs --declare-opponents"
    );
    /// Build one seat's agreements and run `read` under that value
    fn under<T>(
        seat: Option<&Args>,
        restore: &Args,
        read: impl FnOnce(Agreements) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        match seat {
            // No second seat: preserve the historical subset that this branch
            // captured from ambient cells, plus the competition/defense values
            // it already pasted explicitly.  Other areas remain shipped
            // defaults exactly as before.
            None => {
                let armed = arm_knobs(restore)?;
                let mut ambient = Agreements::default();
                ambient.decision.eval_auction = armed.decision.eval_auction;
                ambient.decision.eval_shape = armed.decision.eval_shape;
                ambient.decision.blind_inference = armed.decision.blind_inference;
                ambient.decision.two_over_one_force = armed.decision.two_over_one_force;
                ambient.decision.transfer_gf_majors = armed.decision.transfer_gf_majors;
                ambient.decision.transfer_gf_hearts = armed.decision.transfer_gf_hearts;
                ambient.competition = armed.competition;
                ambient.defense = armed.defense;
                ambient.instinct = armed.instinct;
                read(ambient)
            }
            Some(seat) => {
                let agreements = arm_knobs(seat)?;
                read(agreements)
            }
        }
    }
    // Our partnership reads the live knobs for both its rules and its own card, so it
    // is built here, under the same "every `--ns-*` first" rule as `disclosure`.
    // The opponents' card is a property of *their* engine, so it comes off the
    // oracle actually seated opposite us — `--advertise-natural` swaps that
    // oracle for one with the 1NT defenses dropped, and the net should be told
    // about the seat it faces, not the one it does not.
    let mut our_floor = if args.declare_opponents {
        anyhow::ensure!(
            args.our_system.is_none(),
            "--declare-opponents declares them to *our net*; --our-system \
             replaces our side with EPBot, which has no net to declare to"
        );
        let theirs = match &args.their_floor {
            // A pons opponent declares the card its own name generates — the
            // american-vs-dutch mixed table, and (read under `--their-ns`) any
            // agreement the knobs can express.  What is refused is the
            // *inexpressible* half of the deviation panel: no card row carries
            // `--their-dial`'s strength shift, a four-card overcall style, or
            // undisciplined weak twos, so declaring those would claim a system
            // they are not playing — the one error the net cannot see.
            // `--their-offshape-1nt` is expressible (two 1NT shape rows), so it
            // rides `--their-ns`'s transaction below instead of being refused.
            Some(name) => {
                anyhow::ensure!(
                    args.their_dial == 0
                        && !args.their_overcall_four_card
                        && !args.their_wild_weak_two,
                    "--declare-opponents cannot describe a deviant --their-floor: \
                     no card row expresses --their-dial / --their-overcall-four-card \
                     / --their-wild-weak-two"
                );
                // A `--declare-as` arm replaces the reading wholesale, so the
                // real deviation rides only the honest one.  No `--ns-*` knob
                // touches the off-shape cell, so resetting it to the crate
                // default is exact.
                let offshape = declared_as.is_none() && args.their_offshape_1nt;
                under(
                    declared_as.as_ref().or(their_ns.as_ref()),
                    &args,
                    |mut theirs| {
                        theirs.opening.one_notrump_offshape = offshape;
                        floor_card(name, &theirs)
                    },
                )?
            }
            None => bba_vs_natural.as_ref().unwrap_or(&bba).card(),
        };
        seat_floor_vs(&args.our_floor, &theirs, &agreements)?
    } else {
        seat_floor(&args.our_floor, &agreements)?
    };
    if args.ns_probe > 0 {
        // Fixed seed: every shard of an arm probes the identical map, so the
        // arm's readings are consistent across its shards.
        // Armed before the probe so its fixed-point iteration serves through
        // the same fold consumption will (see `Partnership::probe`).  Both settings
        // are pinned into the partnership at build and are only knowable after it,
        // so each moves on this partnership's own copy.
        our_floor.profile_mut().reading.probed_vacuous = args.ns_probe_vacuous;
        let report = our_floor.probe(args.ns_probe, 0x9B0BE);
        eprintln!(
            "probed {} boards: {} keys, {} drifted",
            args.ns_probe, report.keys, report.drifted
        );
        if !args.ns_probe_vacuous {
            our_floor.profile_mut().reading.probed = true;
        }
    }
    let our_floor = our_floor;
    // The deviation panel's opponent: a second pons book built under the
    // `--their-*` deviation knobs (and its own `--ns-*`), all reset afterwards
    // so only this book carries them — they are read at book construction, the
    // same discipline `--ns-*` follows above.
    let their_floor = match &args.their_floor {
        Some(name) => {
            // `--ns-probe` above moved the probed overlay only on
            // `our_floor`'s pinned profile. Their book never probed, so its
            // separately built agreements retain the shipped-off settings.
            let built = under(their_ns.as_ref(), &args, |theirs| {
                deviant_floor(
                    name,
                    &our_card,
                    &theirs,
                    args.their_dial,
                    args.their_overcall_four_card,
                    args.their_offshape_1nt,
                    args.their_wild_weak_two,
                )
            })?;
            Some(built)
        }
        None => None,
    };
    // The reading channel (Phase 2b), attached here because it needs the
    // opponents' *partnership*, which only exists once their deviation knobs have
    // been applied and reset.  Probing above ran on our own books, which is
    // right: the probe measures what we bid, and our bidding table is what
    // this leaves alone.
    let (our_floor, their_floor) = if args.declare_books_mutually {
        let Some(them) = their_floor else {
            anyhow::bail!("--declare-books-mutually needs --their-floor");
        };
        let ours = our_floor;
        (
            ours.clone().with_opponents(&them),
            Some(them.with_opponents(&ours)),
        )
    } else if args.declare_their_book {
        let Some(them) = their_floor.as_ref() else {
            anyhow::bail!(
                "--declare-their-book reads their calls off their books, so it \
                 needs a pons --their-floor to read"
            );
        };
        (our_floor.with_opponents(them), their_floor)
    } else {
        (our_floor, their_floor)
    };
    // Blind arm of the deviation panel: our side reads with the two opponent
    // seats' readings blanked.  The setting is pinned into a partnership, so this is
    // a copy of our floor with one field moved rather than a global set — the
    // pons book seated opposite us keeps its own readings.  (A BBA oracle has no
    // pons readings to blank, so the flag only reaches the floor.)
    let our_floor = if args.ns_blind_opponent_reading {
        blinded(&our_floor)
    } else {
        our_floor
    };
    let our_oracle = match our_system {
        Some(system) => Some(BbaOracle::load(&path, system, our_conv.clone())?),
        None => None,
    };
    let ours: &dyn Bidder = match &our_oracle {
        Some(oracle) => oracle,
        None => &our_floor,
    };
    let opponent: &dyn Bidder = match (&their_floor, &bba_vs_natural) {
        (Some(deviant), _) => deviant,
        (None, Some(oracle)) => oracle,
        (None, None) => &bba,
    };
    // Labels name the card file rather than spelling out its ~257 toggles;
    // explicit `--*-conv` singles still render individually.
    let our_label = match our_system {
        Some(system) => format!(
            "BBA {}{}{}",
            system_label(system),
            label_card(&args.our_card),
            label_overrides(&args.our_conv)
        ),
        None => {
            let mut label = format!("our {} floor", args.our_floor);
            if args.declare_opponents {
                label += " [declared opponents]";
            }
            if args.declare_their_book {
                label += " [their books]";
            }
            if args.declare_books_mutually {
                label += " [mutual books]";
            }
            if let Some(spec) = &args.declare_as {
                label += &format!(" [declared as {spec}]");
            }
            label
        }
    };
    let their_label = if let Some(name) = &args.their_floor {
        let mut label = format!("their {name} floor");
        if args.their_dial != 0 {
            label += &format!(" [dial {}]", args.their_dial);
        }
        for (on, tag) in [
            (args.their_overcall_four_card, "4-card overcalls"),
            (args.their_offshape_1nt, "off-shape 1NT"),
            (args.their_wild_weak_two, "wild weak twos"),
        ] {
            if on {
                label += &format!(" [{tag}]");
            }
        }
        if let Some(spec) = &args.their_ns {
            label += &format!(" [{spec}]");
        }
        label
    } else {
        format!(
            "BBA {}{}{}{}",
            system_label(args.system),
            label_card(&args.their_card),
            label_overrides(&args.their_conv),
            // Disclosure configures BBA's *view of us*, so it belongs on their label.
            match args.disclose.as_str() {
                "off" => String::new(),
                told => format!(" [told: {told}]{}", label_overrides(&args.disclose_conv)),
            },
        )
    };
    let isolate_opening = args.isolate_opening.as_str();
    anyhow::ensure!(
        matches!(isolate_opening, "off" | "bba" | "pons"),
        "--isolate-opening must be off, bba, or pons"
    );
    anyhow::ensure!(
        !(args.isolate_defense && isolate_opening != "off"),
        "--isolate-defense and --isolate-opening are mutually exclusive"
    );
    // The isolation modes and `--our-system` both seat BBA where the deviant
    // book would go, so the comparison they name no longer exists.
    anyhow::ensure!(
        args.their_floor.is_none()
            || (isolate_opening == "off" && !args.isolate_defense && our_system.is_none()),
        "--their-floor is incompatible with --isolate-opening/--isolate-defense/--our-system"
    );
    anyhow::ensure!(
        args.their_floor.is_some()
            || (args.their_dial == 0
                && !args.their_overcall_four_card
                && !args.their_offshape_1nt
                && !args.their_wild_weak_two),
        "the --their-dial/--their-* deviation knobs need --their-floor"
    );

    let seed = args.seed.unwrap_or_else(rand::random);
    let mut rng = StdRng::seed_from_u64(seed);

    // Bid every board at both tables, dealer rotating per board.  Sequential by
    // design: each EPBot decision creates/destroys a native bot through the FFI,
    // which we do not assume is thread-safe.  With --filter-1nt, keep dealing
    // until `count` deals carry a 1NT-opener candidate.
    let mut boards: Vec<Board> = Vec::with_capacity(args.count);
    let mut scanned = 0usize;
    while boards.len() < args.count {
        let deal = full_deal(&mut rng);
        scanned += 1;
        // One paired scan, not two independent ones: `--filter-landy` must
        // find a single seat that is both the 1NT candidate *and* has a
        // Landy-shaped LHO, or it would accept deals where two unrelated seats
        // satisfy the halves.  LHO is dealer-independent table geometry, which
        // matters because `dealer` is only assigned below.
        if args.filter_landy
            && !Seat::ALL
                .iter()
                .any(|&seat| is_1nt_opener(deal[seat]) && is_landy_shaped(deal[seat.lho()]))
        {
            continue;
        }
        // Same paired scan as `--filter-landy`: one seat must be both the 1NT
        // candidate and hold the preempt-shaped LHO.
        if args.filter_preempt
            && !Seat::ALL
                .iter()
                .any(|&seat| is_1nt_opener(deal[seat]) && is_preempt_shaped(deal[seat.lho()]))
        {
            continue;
        }
        if args.filter_1nt && !Seat::ALL.iter().any(|&seat| is_1nt_opener(deal[seat])) {
            continue;
        }
        let dealer = Seat::ALL[boards.len() % 4];
        // For `--isolate-opening pons` the defender is ours at *both* tables, so our
        // N/S opens against our own defense at table A; otherwise BBA defends.
        let defender_a: &dyn Bidder = if isolate_opening == "pons" {
            ours
        } else {
            opponent
        };
        let table_a = bid_out(ours, defender_a, true, dealer, args.vulnerability, &deal);
        // Opening-isolation modes keep only boards where our N/S actually opened 1NT.
        if isolate_opening != "off" && !matches!(opening_1nt(&table_a, dealer), Some((_, true))) {
            continue;
        }
        let table_b = match isolate_opening {
            // BBA opens 1NT at table B; the defender matches table A (BBA / pons), so
            // the only thing that varies is the opener.  The swing is pure opening.
            "bba" => bid_out(&bba, &bba, true, dealer, args.vulnerability, &deal),
            "pons" => bid_out(&bba, ours, true, dealer, args.vulnerability, &deal),
            _ if args.isolate_defense => {
                // Keep only boards where BBA (E/W) opened 1NT and our N/S defended,
                // and compare against an all-BBA table: same BBA opener + responses,
                // only the defender differs.  The swing is then pure defense quality.
                if !matches!(opening_1nt(&table_a, dealer), Some((_, false))) {
                    continue;
                }
                bid_out(&bba, &bba, true, dealer, args.vulnerability, &deal)
            }
            _ => bid_out(ours, opponent, false, dealer, args.vulnerability, &deal),
        };
        boards.push(Board {
            deal,
            dealer,
            table_a,
            table_b,
        });
    }

    let dump = Dump {
        our_label,
        their_label,
        vulnerability: args.vulnerability,
        seed: Some(seed),
        gen_args: std::env::args().skip(1).collect(),
        boards,
    };
    match args.output.as_deref() {
        Some(path) => {
            serde_json::to_writer(std::io::BufWriter::new(std::fs::File::create(path)?), &dump)?
        }
        None => serde_json::to_writer(std::io::stdout().lock(), &dump)?,
    }
    eprintln!(
        "bba-gen: {} (us) vs {} (them), vulnerability {} — wrote {} boards ({scanned} scanned){}",
        dump.our_label,
        dump.their_label,
        dump.vulnerability,
        dump.boards.len(),
        match args.output.as_deref() {
            Some(path) => format!(" to {path}"),
            None => " to stdout".into(),
        },
    );
    // One shard is one process, so an arm's attribution is the sum of these
    // lines over its shards.  Silent only when the gate is off
    // (`--no-ns-competitive-accountant`) or nothing tripped it.
    let [vetoes, doubles, passes] = pons::bidding::instinct::competitive_counts();
    if vetoes | doubles | passes != 0 {
        eprintln!(
            "bba-gen: competitive accountant fired — {vetoes} bid vetoes, {doubles} double masks, {passes} pass demotions"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Args, arm_knobs};
    use clap::Parser;
    use pons::bidding::agreements::Agreements;

    /// Default CLI arms the shipped system **plus the vs-BBA disclosure
    /// corrections** — `bba-decompose`'s replay contract.
    ///
    /// The decompose replays a dump through the same
    /// [`vs_bba_agreements`][super::common::vs_bba_agreements]-corrected
    /// defaults and demands 100% bit-reproduction, so every board it
    /// attributes to a bucket assumes this equality. When it broke
    /// (2026-08-10 anchor, 69 and 66 mismatched calls of ~2.1M) nothing
    /// failed loudly: the replay-verified fraction just slipped below 100%
    /// in a report line.  The only sanctioned divergence from
    /// `Agreements::default()` is the disclosure channel (`their`), derived
    /// from the opponent this harness hardwires — keep the shared transform
    /// and `their_2c_landy`'s no-declaration arm in lockstep.
    #[test]
    fn default_args_arm_the_shipped_system() {
        let armed = arm_knobs(&Args::parse_from(["bba-gen"])).unwrap();
        let shipped = super::common::vs_bba_agreements(Agreements::default());
        if armed != shipped {
            let (a, s) = (format!("{armed:#?}"), format!("{shipped:#?}"));
            let diff: Vec<_> = a
                .lines()
                .zip(s.lines())
                .filter(|(x, y)| x != y)
                .map(|(x, y)| format!("armed{x}\n  shipped{y}"))
                .collect();
            panic!(
                "--ns-* defaults drifted from the shipped system:\n{}",
                diff.join("\n")
            );
        }
    }
}
