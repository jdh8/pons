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
use contract_bridge::auction::{Auction, Call, RelativeVulnerability};
use contract_bridge::deck::full_deal;
use contract_bridge::eval::hcp as holding_hcp;
use contract_bridge::{AbsoluteVulnerability, Bid, FullDeal, Hand, Level, Seat, Strain, Suit};
use libloading::Library;
use pons::american;
use pons::bidding::american::DoubleShape;
use pons::bidding::array::{Array, Logits};
use pons::bidding::context::relative;
use pons::bidding::{Family, System};
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::ffi::{CString, c_char, c_int, c_void};

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Board, Dump};

const DEFAULT_LIB: &str = "vendor/bba/Native-libraries/linux/x64/libEPBot.so";

/// System index 0 = "2/1GF - 2/1 Game Force" (verified via `epbot_system_name`)
const SYSTEM_2_OVER_1: c_int = 0;

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

    /// Which of our authored systems to seat: `american` (default) or
    /// `neural-v3` (the restrictive disclosable distilled floor; requires the
    /// `neural-floor` feature).  Ignored when `--our-system` selects an EPBot card.
    #[arg(long, default_value = "american")]
    our_floor: String,

    /// Force a named BBA convention on/off on *our* side (repeatable), e.g.
    /// `--our-conv "Rubensohl after 1m=1"`.  Only meaningful with `--our-system`.
    #[arg(long = "our-conv", value_parser = parse_override, value_name = "NAME=0|1")]
    our_conv: Vec<(CString, c_int)>,

    /// Force a named BBA convention on/off on *their* side (repeatable), e.g.
    /// `--their-conv "Rubensohl after 1m=0"`.  Pair with `--our-conv` to isolate
    /// one toggle in a BBA-vs-BBA A/B.
    #[arg(long = "their-conv", value_parser = parse_override, value_name = "NAME=0|1")]
    their_conv: Vec<(CString, c_int)>,

    /// Only keep deals with a balanced 15-17 HCP hand somewhere (a 1NT-opener
    /// candidate), to raise the yield of 1NT boards.  Cheap shape gate, no
    /// bidding; `--count` then means *kept* boards.
    #[arg(long, default_value_t = false)]
    filter_1nt: bool,

    /// Enable our Unusual-vs-Unusual structure over 1NT-(2NT) — BBA overcalls our
    /// 1NT with a both-minors 2NT (Multi-Landy), so this is the live test.  Sets
    /// the responder structure + the encircling chase at the given floors.
    #[arg(long, default_value_t = false)]
    uvu: bool,

    /// Responder's penalty-double HCP floor for `--uvu`
    #[arg(long, default_value_t = 9)]
    uvu_x_floor: u8,

    /// Responder's INV+ cue-bid points floor for `--uvu`
    #[arg(long, default_value_t = 8)]
    uvu_cue_floor: u8,

    /// Deal seed for reproducible boards (pairs an `--uvu` on/off comparison so
    /// the boards UvU does not touch are identical and cancel); unset = random
    #[arg(long)]
    seed: Option<u64>,

    /// Read a `(2♦)` overcall of our 1NT as a Multi (an unknown major) and use our
    /// Multi counter-defense.  BBA's 2/1 card overcalls 1NT with Multi-Landy, whose
    /// 2♦ *is* a Multi, so this is the live test — pair with
    /// `--their-conv "Multi-Landy=1"` to be sure BBA bids it.
    #[arg(long, default_value_t = false)]
    defense_2d_multi: bool,

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

    /// Disable our continuations after the opponents contest our 2♣ Stayman
    /// (`1NT-(P)-2♣-(X)`/`-(2♦/2♥/2♠)`); on by default.  Off-switch for the A/B.
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

    /// Author our defense to the opponents' 2♣ Stayman (`(1NT)-P-(2♣)`): X =
    /// lead-directing clubs, natural overcalls, strong 3♣ (default off; opt-in A/B).
    #[arg(long, default_value_t = false)]
    ns_defense_to_their_stayman: bool,

    /// Author our continuations after the opponents contest our Jacoby transfer
    /// (`1NT-(P)-2♦/2♥-(X)`/`-(overcall)`); default off (opt-in A/B — DD-negative).
    #[arg(long, default_value_t = false)]
    ns_comp_over_transfer: bool,

    /// Author opener's jump super-accept of a Jacoby transfer (four-card support +
    /// a maximum); default off (opt-in A/B — DD wash).
    #[arg(long, default_value_t = false)]
    ns_transfer_super_accept: bool,

    /// Disable responder's post-transfer single-suited slam try (`1NT–2♦–2♥–3♠` /
    /// `1NT–2♥–2♠–3♥`, a 5-card-major RKCB slam try); on by default.
    #[arg(long, default_value_t = false)]
    no_ns_transfer_slam_try: bool,

    /// Disable the Texas + responder-RKCB slam drive for 6-card-major hands
    /// (restores the opener-decides direct `1NT–4♥/4♠` at 15-18); on by default.
    #[arg(long, default_value_t = false)]
    no_ns_texas_slam_drive: bool,

    /// Disable garbage (drop-dead) Stayman: a weak 2♣ to escape 1NT, passing
    /// opener's 2♦/2♥/2♠; on by default.  Off-switch for the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_garbage_stayman: bool,

    /// Disable opener's max-only right-siding relay over 1NT-2♣ with both four-card
    /// majors (2NT = 16-17; responder names a major via 3♣/3♦, opener completes and
    /// declares); on by default.  Off-switch for the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_stayman_both_majors: bool,

    /// Disable opener's max five-card-major jump over 1NT-2♣ (3♥/3♠); on by
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

    /// Author our defense to the opponents' Jacoby transfers (`(1NT)-P-(2♦/2♥)`):
    /// X = lead-directing the bid suit, Michaels cue, natural overcalls (default
    /// off; opt-in A/B).
    #[arg(long, default_value_t = false)]
    ns_transfer_defense: bool,

    /// Turn OFF our continuations after the opponents contest our two-way 2♠ minor
    /// response (`1NT-(P)-2♠-(X)`/`-(overcall)`); default on (the A/B off-switch —
    /// measured +4.80 IMPs/fired plain, +5.63 PD).
    #[arg(long, default_value_t = false)]
    no_ns_comp_over_minor_transfer: bool,

    /// Author our defense to the opponents' two-way 2♠ minor response
    /// (`(1NT)-P-(2♠)`): X = lead-directing spades, 2NT/3♣ two-suiters, natural
    /// overcalls (default off; opt-in A/B).
    #[arg(long, default_value_t = false)]
    ns_minor_transfer_defense: bool,

    /// Turn OFF our continuations after the opponents contest our 2NT diamond
    /// transfer (`1NT-(P)-2NT-(X)`/`-(overcall)`); default on (the A/B off-switch —
    /// measured a plain-DD wash +0.24/fired, +3.40 PD).
    #[arg(long, default_value_t = false)]
    no_ns_comp_over_diamond_transfer: bool,

    /// Author our defense to the opponents' 2NT diamond transfer
    /// (`(1NT)-P-(2NT)`): X = lead-directing diamonds, 3♦ cue = both majors,
    /// natural overcalls (default off; opt-in A/B).
    #[arg(long, default_value_t = false)]
    ns_diamond_transfer_defense: bool,

    /// Stayman-defense natural-overcall `MIN_LEN:POINTS_FLOOR` (default 6:12); the
    /// A/B search knob for the 2♦/2♥/2♠ length + strength (no effect unless
    /// `--ns-defense-to-their-stayman`).
    #[arg(long, default_value = "6:12")]
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

    /// Logit weight of our natural penalty double of their 1NT (default 1.3, above
    /// the 1.0 suit overcall).
    #[arg(long, default_value_t = 1.3)]
    ns_double_weight: f32,

    /// Extend our 1NT defense to the balancing seat (1NT) P P ? (default off).
    #[arg(long, default_value_t = false)]
    ns_balancing: bool,

    /// Replace our natural 1NT defense with conventional DONT (default off):
    /// one-suiter X, 2♣ = clubs + a higher major, 2♦ = diamonds + a major, 2♥ =
    /// both majors, 2♠ natural, 2NT = both minors.
    #[arg(long, default_value_t = false)]
    ns_dont: bool,

    /// DONT one-suiter minimum length for the `X`/`2♠` (default 5; set 6 to insist
    /// only with a six-card suit). Only with `--ns-dont`.
    #[arg(long, default_value_t = 5)]
    ns_dont_one_suiter_min: u8,

    /// Let DONT two-suiters (`2♣`/`2♦`/`2♥`) accept a flat 4-4 (default off = 5-4+).
    /// Only with `--ns-dont`.
    #[arg(long, default_value_t = false)]
    ns_dont_four_four: bool,

    /// Overlay Landy on our natural 1NT defense (default off): `2♣` = both majors
    /// (≥5-4), `2NT` = both minors, on the given `points` band `LO:HI`, replacing the
    /// natural `2♣` club overcall (penalty X + natural `2♦`/`2♥`/`2♠` stay).  Pair with
    /// `--advertise-landy` so BBA reads our `2♣` as both majors and the rest natural.
    #[arg(long)]
    ns_landy: Option<String>,

    /// Replace our 1NT defense with our own Woolsey "Multi-Landy" (default off):
    /// X = 4-card major + longer minor, 2♣ = both majors, 2♦ = Multi, 2♥/2♠ =
    /// Muiderberg.  Run WITHOUT `--advertise-natural` (BBA reads it via Multi-Landy).
    #[arg(long, default_value_t = false)]
    ns_woolsey: bool,

    /// Woolsey suit-overcall (2♣/2♦/2♥/2♠) points band LO:HI (default 8:19). Only
    /// with `--ns-woolsey`.
    #[arg(long, default_value = "8:19")]
    ns_woolsey_range: String,

    /// `points` floor for our Woolsey takeout X (default 12). Only with `--ns-woolsey`.
    #[arg(long, default_value_t = 12)]
    ns_woolsey_x_floor: u8,

    /// Disable the penalty-double latch (default on): after our natural penalty X of
    /// BBA's 1NT, our later doubles read as penalty instead of takeout.
    #[arg(long, default_value_t = false)]
    no_ns_penalty_latch: bool,

    /// Restore the doubler's constructive pulls of its own penalty X of BBA's 1NT
    /// (default off = pulls suppressed): with this set, a latched doubler may again
    /// "compete" to 2NT/3NT/a major over the opponents' escape instead of defending.
    #[arg(long, default_value_t = false)]
    ns_allow_pull: bool,

    /// Disable the advancer's runout from BBA's redoubled penalty X (default on):
    /// after `[1NT, X, XX]`, a weak advancer sits for `1NTxx` instead of escaping to
    /// its long suit.
    #[arg(long, default_value_t = false)]
    no_ns_xx_runout: bool,

    /// Disable the *doubler's* runout once BBA's redoubled penalty X runs back around
    /// (default on): after `[1NT, X, XX, P, P]`, a 15+ doubler with a five-plus suit
    /// escapes to it instead of defending `1NTxx`.
    #[arg(long, default_value_t = false)]
    no_ns_doubler_run: bool,

    /// Our side NEVER competes over BBA's 1NT (default off): authors only Pass at
    /// every seat, the truest "do nothing" baseline.  Overrides every other defense knob.
    #[arg(long, default_value_t = false)]
    ns_always_pass: bool,

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

    /// Disable reading per-call alerts as artificial (default on) to A/B the
    /// alert-reading defense switch — how our floor reads alerted artificial calls.
    #[arg(long, default_value_t = false)]
    no_alert_reading: bool,

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

    /// Opener corrects partner's choice-of-games 3NT to 4M with a known
    /// eight-card major fit.  Single-dummy-sound but double-dummy-negative
    /// (−0.037 IMPs/board), so opt-in and off by default.
    #[arg(long, default_value_t = false)]
    ns_correct_3nt_to_major: bool,
}

/// Parse a `NAME=0|1` convention override for `--our-conv` / `--their-conv`
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

/// EPBot system label for the indices we use (the pinned `vendor/bba` build)
fn system_label(system: c_int) -> &'static str {
    match system {
        0 => "2/1 Game Force",
        2 => "WJ (Polish Club)",
        _ => "EPBot system",
    }
}

/// Render convention overrides for a side's label, e.g. ` [Rubensohl after 1m=1]`
fn label_overrides(overrides: &[(CString, c_int)]) -> String {
    overrides
        .iter()
        .map(|(name, value)| format!(" [{}={value}]", name.to_string_lossy()))
        .collect()
}

// ---------------------------------------------------------------------------
// The BBA oracle: EPBot driven as a pons `System`
// ---------------------------------------------------------------------------

// Confirmed C ABI (objdump + EPBotFFI decompile + empirical bid codes); the
// S.0 spike documents the discovery.  Handles are opaque pointers.
type CreateFn = unsafe extern "C" fn() -> *mut c_void;
type DestroyFn = unsafe extern "C" fn(*mut c_void);
type SetSystemFn = unsafe extern "C" fn(*mut c_void, c_int, c_int);
type NewHandFn =
    unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int, c_int, c_int, c_int);
// `epbot_set_bid(bot, position, bid, meaning)` — the 4th arg is the bid's
// meaning string (decompiled from EPBotFFI.SetBid); an empty string is fine,
// EPBot interprets each bid itself from the configured system.
type SetBidFn = unsafe extern "C" fn(*mut c_void, c_int, c_int, *const c_char);
type GetBidFn = unsafe extern "C" fn(*mut c_void) -> c_int;
// `epbot_set_conventions(bot, seat, name, on)` — per-seat convention toggle.
type SetConvFn = unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int) -> c_int;

/// EPBot 2/1 bidder behind pons's [`System`] trait.
///
/// Each [`System::classify`] call drives a *fresh* bot: it configures all four
/// seats to the chosen system, deals the actor's hand, replays the auction so
/// far with `set_bid`, and reads the actor's call with `get_bid`.  A fresh bot
/// per decision keeps `classify` a pure, stateless function of its arguments.
///
/// Cached raw function pointers (copied out of the [`Library`]) avoid a `dlsym`
/// per call; `_lib` is held so the pointers stay valid for the oracle's life.
struct BbaOracle {
    _lib: Library,
    create: CreateFn,
    destroy: DestroyFn,
    set_system: SetSystemFn,
    new_hand: NewHandFn,
    set_bid: SetBidFn,
    get_bid: GetBidFn,
    set_conv: SetConvFn,
    system: c_int,
    /// Named conventions forced to a value on all four seats of every fresh bot,
    /// applied after `set_system` (which loads the system's defaults).
    overrides: Vec<(CString, c_int)>,
}

impl BbaOracle {
    /// Load the EPBot library and bind the `epbot_*` symbols
    fn load(path: &str, system: c_int, overrides: Vec<(CString, c_int)>) -> anyhow::Result<Self> {
        // SAFETY: loading a trusted native library; its initializers run here.
        let lib = unsafe { Library::new(path) }?;
        // SAFETY: each symbol has the signature confirmed in the S.0 spike;
        // `*sym` copies the function pointer (it is `Copy` and does not borrow
        // the library, which we keep alive in `_lib`).
        unsafe {
            Ok(Self {
                create: *lib.get::<CreateFn>(b"epbot_create\0")?,
                destroy: *lib.get::<DestroyFn>(b"epbot_destroy\0")?,
                set_system: *lib.get::<SetSystemFn>(b"epbot_set_system_type\0")?,
                new_hand: *lib.get::<NewHandFn>(b"epbot_new_hand\0")?,
                set_bid: *lib.get::<SetBidFn>(b"epbot_set_bid\0")?,
                get_bid: *lib.get::<GetBidFn>(b"epbot_get_bid\0")?,
                set_conv: *lib.get::<SetConvFn>(b"epbot_set_conventions\0")?,
                _lib: lib,
                system,
                overrides,
            })
        }
    }
}

impl System for BbaOracle {
    fn classify(&self, hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> Option<Logits> {
        // Canonicalize the dealer to position 0: the actor is the seat that has
        // bid `auction.len()` times after the dealer.  The relative seat and the
        // favorable/unfavorable vulnerability are preserved by the replayed calls
        // and the mapping below, so the bid is identical to the true seating.
        let actor = (auction.len() % 4) as c_int;
        let suits = hand_to_suits(hand);
        let empty = c"".as_ptr();

        // SAFETY: a fresh bot used and destroyed within this call; all argument
        // types match the confirmed ABI.
        let code = unsafe {
            let bot = (self.create)();
            if bot.is_null() {
                return None;
            }
            for seat in 0..4 {
                (self.set_system)(bot, seat, self.system);
            }
            // Force any isolated convention(s) AFTER set_system loads defaults.
            for (name, value) in &self.overrides {
                for seat in 0..4 {
                    (self.set_conv)(bot, seat, name.as_ptr(), *value);
                }
            }
            (self.new_hand)(
                bot,
                actor,
                suits.as_ptr(),
                0,
                epbot_vulnerability(vul, actor),
                0,
                0,
            );
            for (index, &call) in auction.iter().enumerate() {
                (self.set_bid)(bot, (index % 4) as c_int, encode_call(call), empty);
            }
            let code = (self.get_bid)(bot);
            (self.destroy)(bot);
            code
        };

        decode_call(code).map(one_hot)
    }
}

/// The four holdings in EPBot's C,D,H,S order, newline-joined
///
/// [`Holding`][contract_bridge::Holding]'s `Display` renders ranks high-to-low
/// using `T` for the ten — exactly EPBot's canonical form.  EPBot counts
/// characters as cards, so every hand must be exactly 13; `full_deal` guarantees
/// that.  A void suit is an empty segment, which EPBot reads as zero cards.
fn hand_to_suits(hand: Hand) -> CString {
    use core::fmt::Write;
    let mut suits = String::with_capacity(20);
    for (index, suit) in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
        .into_iter()
        .enumerate()
    {
        if index > 0 {
            suits.push('\n');
        }
        write!(suits, "{}", hand[suit]).expect("writing to a String never fails");
    }
    CString::new(suits).expect("a holding string never contains a NUL byte")
}

/// EPBot vulnerability code from the actor-relative vulnerability
///
/// EPBot seats even (0, 2) are North/South and odd (1, 3) East/West; its
/// vulnerability bits are 1 = N/S, 2 = E/W.  With the dealer canonicalized to
/// position 0, the actor's side is N/S iff `actor` is even.
fn epbot_vulnerability(vul: RelativeVulnerability, actor: c_int) -> c_int {
    let we = vul.contains(RelativeVulnerability::WE);
    let they = vul.contains(RelativeVulnerability::THEY);
    let (ns, ew) = if actor % 2 == 0 {
        (we, they)
    } else {
        (they, we)
    };
    c_int::from(ns) | (c_int::from(ew) << 1)
}

/// Encode a [`Call`] into EPBot's integer bid code
///
/// `0/1/2 = Pass/X/XX`; a bid is `5 + (level - 1) * 5 + strain` with strain
/// `0 = ♣ … 4 = NT`, matching [`Strain`]'s discriminant order.
fn encode_call(call: Call) -> c_int {
    match call {
        Call::Pass => 0,
        Call::Double => 1,
        Call::Redouble => 2,
        Call::Bid(bid) => 5 + (c_int::from(bid.level.get()) - 1) * 5 + strain_index(bid.strain),
    }
}

/// Decode EPBot's bid code back into a [`Call`], or [`None`] on an error code
fn decode_call(code: c_int) -> Option<Call> {
    match code {
        0 => Some(Call::Pass),
        1 => Some(Call::Double),
        2 => Some(Call::Redouble),
        5..=39 => {
            let index = code - 5;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let level = Level::new((index / 5 + 1) as u8);
            Some(Call::Bid(Bid {
                level,
                strain: STRAINS[(index % 5) as usize],
            }))
        }
        _ => None,
    }
}

/// Strains in EPBot/[`Strain`] discriminant order (♣ ♦ ♥ ♠ NT)
const STRAINS: [Strain; 5] = [
    Strain::Clubs,
    Strain::Diamonds,
    Strain::Hearts,
    Strain::Spades,
    Strain::Notrump,
];

/// The 0..=4 index of a strain
fn strain_index(strain: Strain) -> c_int {
    STRAINS
        .iter()
        .position(|&s| s == strain)
        .expect("every strain is in STRAINS") as c_int
}

/// One-hot logits: the chosen call finite, everything else impossible
fn one_hot(call: Call) -> Logits {
    Logits(Array::from_fn(|c| {
        if c == call { 0.0 } else { f32::NEG_INFINITY }
    }))
}

// ---------------------------------------------------------------------------
// Driving the match (mirrors examples/instinct-floor)
// ---------------------------------------------------------------------------

/// The seat acting after `len` calls from `dealer`
const fn seat_to_act(dealer: Seat, len: usize) -> Seat {
    Seat::ALL[(dealer as usize + len) % 4]
}

/// The highest-logit *legal* call, defaulting to a pass
fn next_call(
    system: &dyn System,
    hand: Hand,
    seat: Seat,
    vul: AbsoluteVulnerability,
    auction: &Auction,
) -> Call {
    let Some(logits) = system.classify(hand, relative(vul, seat), auction) else {
        return Call::Pass;
    };
    let mut scored: Vec<(Call, f32)> = logits
        .iter()
        .map(|(call, &logit)| (call, logit))
        .filter(|&(_, logit)| logit.is_finite())
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("logits are never NaN"));
    scored
        .into_iter()
        .map(|(call, _)| call)
        .find(|&call| auction.can_push(call).is_ok())
        .unwrap_or(Call::Pass)
}

/// Bid out one deal with our pair on `ours_is_ns`'s side, BBA on the other
fn bid_out(
    ours: &dyn System,
    bba: &dyn System,
    ours_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let system = if seat_is_ns == ours_is_ns { ours } else { bba };
        auction.push(next_call(system, deal[seat], seat, vul, &auction));
    }
    auction
}

// ---------------------------------------------------------------------------
// The 1NT pre-filters that shape which boards are generated
// ---------------------------------------------------------------------------

/// Total HCP of a hand
fn hand_hcp(hand: Hand) -> u8 {
    Suit::ASC.iter().map(|&s| holding_hcp::<u8>(hand[s])).sum()
}

/// Balanced (no singleton/void, at most one doubleton) with 15-17 HCP — a strict
/// 1NT-opener gate for the cheap `--filter-1nt` pre-filter.
fn is_1nt_opener(hand: Hand) -> bool {
    let len = Suit::ASC.map(|s| hand[s].len());
    let balanced = len.iter().all(|&l| l >= 2) && len.iter().filter(|&&l| l == 2).count() <= 1;
    balanced && (15..=17).contains(&hand_hcp(hand))
}

/// If this auction's *opening* call is 1NT, its index and whether the opener is
/// North/South.  The opening requirement (all prior calls passes) excludes a
/// `1♣-P-1NT` rebid — we want 1NT *openings* only.  Used by `--isolate-defense`
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

#[allow(clippy::too_many_lines)]
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
    let bba = match BbaOracle::load(&path, args.system, args.their_conv.clone()) {
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
        let mut conv = args.their_conv.clone();
        // Disclose our defense by setting how the opponent bot reads us: drop every
        // 1NT-defense convention, then (for Landy) re-enable just `Landy` so our `2♣`
        // reads as both majors and the rest natural.
        for name in ["Multi-Landy", "Cappelletti", "Landy"] {
            let on = (name == "Landy" && args.advertise_landy) as c_int;
            conv.push((CString::new(name).expect("a literal name has no NUL"), on));
        }
        Some(BbaOracle::load(&path, args.system, conv)?)
    } else {
        None
    };
    // Our side: the authored floor by default, or a second EPBot card when
    // `--our-system` is given (the BBA-vs-BBA experiment).
    if args.uvu {
        pons::bidding::american::set_uvu(true);
        pons::bidding::american::set_uvu_x_floor(args.uvu_x_floor);
        pons::bidding::american::set_uvu_cue_floor(args.uvu_cue_floor);
        pons::bidding::instinct::set_uvu_encircle(true);
    }
    pons::bidding::american::set_defense_to_2d_multi(args.defense_2d_multi);
    pons::bidding::instinct::set_settle_floor(!args.no_settle_floor);
    pons::bidding::instinct::set_nt_responder_game_floor(args.ns_nt_responder_game_floor);
    pons::bidding::instinct::set_suppress_nt_game_force_over_double(
        !args.no_ns_suppress_nt_gf_over_double,
    );
    pons::bidding::instinct::set_gambling_3nt_over_double(args.ns_gambling_3nt);
    pons::bidding::instinct::set_gambling_3nt_top_honors(args.ns_gambling_3nt_top_honors);
    pons::bidding::instinct::set_gambling_3nt_require_ace(!args.no_ns_gambling_3nt_ace);
    pons::bidding::instinct::set_preempt_4m_over_double(args.ns_preempt_4m);
    pons::bidding::instinct::set_preempt_4m_top_honors(args.ns_preempt_4m_top_honors);
    pons::bidding::instinct::set_preempt_4m_require_ace(!args.no_ns_preempt_4m_ace);
    pons::bidding::instinct::set_correct_3nt_to_major(args.ns_correct_3nt_to_major);
    pons::bidding::set_alert_reading(!args.no_alert_reading);
    pons::bidding::instinct::set_penalty_latch(!args.no_ns_penalty_latch);
    pons::bidding::instinct::set_penalty_no_pull(!args.ns_allow_pull);
    pons::bidding::instinct::set_advancer_xx_runout(!args.no_ns_xx_runout);
    pons::bidding::instinct::set_doubler_xx_runout(!args.no_ns_doubler_run);
    pons::bidding::set_fallback_projection(!args.no_ns_fallback_projection);
    pons::bidding::american::set_open_one_notrump(!args.no_our_1nt);
    pons::bidding::american::set_one_notrump_fifths(args.nt_fifths);
    pons::bidding::american::set_natural_double_shape(match args.ns_double_shape.as_str() {
        "any" => DoubleShape::Any,
        "semi" => DoubleShape::SemiBalanced,
        "balanced" => DoubleShape::Balanced,
        other => anyhow::bail!("--ns-double-shape must be any|semi|balanced, got {other:?}"),
    });
    pons::bidding::american::set_natural_double_floor(args.ns_double_floor);
    pons::bidding::american::set_natural_double_weight(args.ns_double_weight);
    pons::bidding::american::set_notrump_balancing(args.ns_balancing);
    let (oc_lo, oc_hi) = args
        .ns_overcall
        .split_once(':')
        .and_then(|(lo, hi)| Some((lo.parse::<u8>().ok()?, hi.parse::<u8>().ok()?)))
        .ok_or_else(|| {
            anyhow::anyhow!("--ns-overcall must be LO:HI, got {:?}", args.ns_overcall)
        })?;
    pons::bidding::american::set_natural_overcall_points(oc_lo, oc_hi);
    pons::bidding::american::set_competition_over_stayman(!args.no_ns_comp_over_stayman);
    pons::bidding::american::set_competitive_4333(match args.ns_competitive_4333.as_str() {
        "allow" => pons::bidding::american::Competitive4333::Allow,
        "suppress" => pons::bidding::american::Competitive4333::Suppress,
        "suppress-stopper" => pons::bidding::american::Competitive4333::SuppressWithStopper,
        other => {
            anyhow::bail!(
                "--ns-competitive-4333 must be allow|suppress|suppress-stopper, got {other:?}"
            )
        }
    });
    pons::bidding::american::set_stayman_defense(args.ns_defense_to_their_stayman);
    pons::bidding::american::set_competition_over_transfer(args.ns_comp_over_transfer);
    pons::bidding::american::set_transfer_super_accept(args.ns_transfer_super_accept);
    pons::bidding::american::set_transfer_slam_try(!args.no_ns_transfer_slam_try);
    pons::bidding::american::set_texas_slam_drive(!args.no_ns_texas_slam_drive);
    pons::bidding::american::set_garbage_stayman(!args.no_ns_garbage_stayman);
    pons::bidding::american::set_stayman_both_majors(!args.no_ns_stayman_both_majors);
    pons::bidding::american::set_stayman_5card_max(!args.no_ns_stayman_5card_max);
    pons::bidding::american::set_invitational_5card_majors(!args.no_ns_invitational_5card_majors);
    pons::bidding::american::set_crawling_stayman(!args.no_ns_crawling_stayman);
    pons::bidding::american::set_texas_game_floor(args.ns_texas_game_floor);
    pons::bidding::american::set_sixcard_invite_floor(args.ns_sixcard_invite_floor);
    pons::bidding::american::set_sixcard_accept_floor(args.ns_sixcard_accept_floor);
    pons::bidding::american::set_transfer_defense(args.ns_transfer_defense);
    pons::bidding::american::set_competition_over_minor_transfer(
        !args.no_ns_comp_over_minor_transfer,
    );
    pons::bidding::american::set_minor_transfer_defense(args.ns_minor_transfer_defense);
    pons::bidding::american::set_competition_over_diamond_transfer(
        !args.no_ns_comp_over_diamond_transfer,
    );
    pons::bidding::american::set_diamond_transfer_defense(args.ns_diamond_transfer_defense);
    {
        let (lo, hi) = args
            .ns_staydef_overcall
            .split_once(':')
            .and_then(|(l, f)| Some((l.parse::<usize>().ok()?, f.parse::<u8>().ok()?)))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "--ns-staydef-overcall must be LEN:FLOOR, got {:?}",
                    args.ns_staydef_overcall
                )
            })?;
        pons::bidding::american::set_stayman_defense_overcall(lo, hi);
    }
    pons::bidding::american::set_direct_dont(args.ns_dont);
    if args.ns_dont {
        pons::bidding::american::set_landy(None);
        pons::bidding::american::set_unusual_notrump_defense(Some((8, 14)));
        pons::bidding::american::set_direct_dont_one_suiter_min(args.ns_dont_one_suiter_min);
        pons::bidding::american::set_direct_dont_four_four(args.ns_dont_four_four);
    }
    if let Some(spec) = &args.ns_landy {
        let (lo, hi) = spec
            .split_once(':')
            .and_then(|(lo, hi)| Some((lo.parse::<u8>().ok()?, hi.parse::<u8>().ok()?)))
            .ok_or_else(|| anyhow::anyhow!("--ns-landy must be LO:HI, got {spec:?}"))?;
        pons::bidding::american::set_landy(Some((lo, hi)));
    }
    pons::bidding::american::set_woolsey(args.ns_woolsey);
    if args.ns_woolsey {
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
        pons::bidding::american::set_woolsey_points(wlo, whi);
        pons::bidding::american::set_woolsey_double_floor(args.ns_woolsey_x_floor);
        pons::bidding::american::set_natural_defense(false);
        pons::bidding::american::set_landy(None);
        pons::bidding::american::set_direct_dont(false);
    }
    pons::bidding::american::set_always_pass_defense(args.ns_always_pass);
    let our_floor = match args.our_floor.as_str() {
        "american" => american().against(Family::NATURAL),
        #[cfg(feature = "neural-floor")]
        "neural-v3" => pons::american_neural_v3().against(Family::NATURAL),
        other => anyhow::bail!(
            "--our-floor must be american{}, got {other:?}",
            if cfg!(feature = "neural-floor") {
                " or neural-v3"
            } else {
                " (neural-v3 needs --features neural-floor)"
            }
        ),
    };
    let our_oracle = match args.our_system {
        Some(system) => Some(BbaOracle::load(&path, system, args.our_conv.clone())?),
        None => None,
    };
    let ours: &dyn System = match &our_oracle {
        Some(oracle) => oracle,
        None => &our_floor,
    };
    let opponent: &dyn System = match &bba_vs_natural {
        Some(oracle) => oracle,
        None => &bba,
    };
    let our_label = match args.our_system {
        Some(system) => format!(
            "BBA {}{}",
            system_label(system),
            label_overrides(&args.our_conv)
        ),
        None => format!("our {} floor", args.our_floor),
    };
    let their_label = format!(
        "BBA {}{}",
        system_label(args.system),
        label_overrides(&args.their_conv)
    );
    let isolate_opening = args.isolate_opening.as_str();
    anyhow::ensure!(
        matches!(isolate_opening, "off" | "bba" | "pons"),
        "--isolate-opening must be off, bba, or pons"
    );
    anyhow::ensure!(
        !(args.isolate_defense && isolate_opening != "off"),
        "--isolate-defense and --isolate-opening are mutually exclusive"
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
        if args.filter_1nt && !Seat::ALL.iter().any(|&seat| is_1nt_opener(deal[seat])) {
            continue;
        }
        let dealer = Seat::ALL[boards.len() % 4];
        // For `--isolate-opening pons` the defender is ours at *both* tables, so our
        // N/S opens against our own defense at table A; otherwise BBA defends.
        let defender_a: &dyn System = if isolate_opening == "pons" {
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
    Ok(())
}
