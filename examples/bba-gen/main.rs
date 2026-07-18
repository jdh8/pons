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
use pons::american;
use pons::bidding::american::DoubleShape;
use pons::bidding::{Family, System};
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::ffi::{CString, c_int};

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::oracle::{BbaOracle, DEFAULT_LIB, SYSTEM_2_OVER_1, bid_out, load_bbsa};
use common::{Board, Dump, hand_hcp, seat_to_act};

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

    /// 1NT opening shape policy for our pair: wide6322 (default, the shipped
    /// 5422+6322-minor shape) | wide (superseded 5422-minor baseline) | classic
    /// (balanced-only baseline).  The ablation handle for the wide-1NT redesign
    /// against the BBA opponent; the default matches `american()`.
    #[arg(long, default_value = "wide6322", value_name = "wide6322|wide|classic")]
    nt_shape: String,

    /// Disable Rule-of-20 light openings (restores the 12+-only opener that
    /// passes sound 10-11 counts); on by default.  Off-switch for the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_rule_of_20: bool,

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

    /// Disable responder's game-forcing structure after the spade transfer
    /// (`1NT–2♥–2♠`: natural 5-5 `3♥` slam try, `3♣`/`3♦` minors, `4♣`/`4♦`/`4♥`
    /// splinters, quantitative `4NT`); on by default.
    #[arg(long, default_value_t = false)]
    no_ns_transfer_gf_majors: bool,

    /// Within the GF-majors structure (Arm B), reserve `3♣`/`3♦` for slam tries and
    /// route minimum game-forces into the choice-of-games `3NT`; default off.
    #[arg(long, default_value_t = false)]
    ns_minor_min_to_3nt: bool,

    /// Disable the GF structure's heart-transfer mirror (`1NT–2♦–2♥`: `3♣`/`3♦` minors,
    /// `3♠`/`4♣`/`4♦` splinters, quantitative `4NT`); on by default (with the master
    /// GF-majors structure).
    #[arg(long, default_value_t = false)]
    no_ns_transfer_gf_hearts: bool,

    /// Disable responder's post-transfer single-suited slam try (`1NT–2♦–2♥–3♠` /
    /// `1NT–2♥–2♠–3♥`, a 5-card-major RKCB slam try); on by default.
    #[arg(long, default_value_t = false)]
    no_ns_transfer_slam_try: bool,

    /// Disable the Texas + responder-RKCB slam drive for 6-card-major hands
    /// (restores the opener-decides direct `1NT–4♥/4♠` at 15-18); on by default.
    #[arg(long, default_value_t = false)]
    no_ns_texas_slam_drive: bool,

    /// Disable the plain-4NT minor-suit keycard (strong-2♣ minor raise and
    /// inverted-minor `1m–2m–3NT–4NT`; restores the pre-keycard blind 6m jump /
    /// 3NT top-out); on by default.  Off-switch for the A7 re-measure.
    #[arg(long, default_value_t = false)]
    no_ns_minor_keycard: bool,

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

    /// Disable responder's continuation after opener's 3OM-slam-try cue
    /// (`1NT-2♣-2M-3OM-4x`): on, responder keycards or signs off in the major game
    /// instead of passing the cue out below game; on by default. Off-switch for the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_stayman_cue_continuation: bool,

    /// Disable the longer-major discipline for minor-opening responses (1♠ on
    /// longer spades or 5-5, 1♥ up the line only on 4-4, with the M6.4
    /// classifier reading to match); on by default (the established American
    /// treatment — see `set_longer_major_response`). Off-switch for the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_longer_major_response: bool,

    /// Disable the up-the-line completion of the natural minor tree (the
    /// 1♣-1♦ response, opener's 1♠ rebid over 1m-1♥, opener's natural 2♣
    /// after 1♣-1♦); on by default, shipped jointly with XYZ. Off-switch for
    /// the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_up_the_line: bool,

    /// Disable the 1M-3NT choice-of-games response (3-4 card support, exactly
    /// (4333), 12-15 HCP; opener passes balanced, corrects to 4M with shape);
    /// on by default. Off-switch for the A/B (see `set_major_choice_of_games`).
    #[arg(long, default_value_t = false)]
    no_ns_major_choice_of_games: bool,

    /// Disable the fit leg of the major 2/1 game force (exactly 3-card
    /// support enters on `support_points(13..)` — the 2/1 as a preparation
    /// for 4M); on by default. Off-switch for the A/B (see
    /// `set_two_over_one_fit`).
    #[arg(long, default_value_t = false)]
    no_ns_two_over_one_fit: bool,

    /// The no-fit gauge of the major 2/1 game force:
    /// hcp13 (shipped default) | hcp12 | points13 (the legacy gate)
    /// (see `set_two_over_one_gate`).
    #[arg(long, default_value = "hcp13")]
    ns_two_over_one_gate: String,

    /// Disable the XYZ two-way checkback after three one-level bids (2♣
    /// puppets 2♦ for sign-off or invite, 2♦ game-forces); on by default.
    /// Off-switch for the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_xyz: bool,

    /// Enable New Minor Forcing in place of XYZ on the four `1m-1M-1NT` slots
    /// (opt-in, off by default): responder's two-of-the-new-minor is an
    /// invitational-or-better checkback promising a five-card major.
    #[arg(long, default_value_t = false)]
    ns_new_minor_forcing: bool,

    /// Author opener's major game tries after a single raise (`1M – 2M`): a
    /// long-suit try, the general re-raise, or a keycard-asking maximum
    /// (shipped default-on; see `set_major_game_tries`).
    #[arg(long, default_value_t = false)]
    no_ns_major_game_tries: bool,

    /// Disable opener's limit-raise acceptance ladder after `1M – 3M`
    /// (shipped default-on; see `set_limit_raise_acceptance`).
    #[arg(long, default_value_t = false)]
    no_ns_limit_raise_acceptance: bool,

    /// Disable opener's answer to partner's cue-raise (`1M – (ovc) – cue – P`)
    /// (shipped default-on; see `set_cue_raise_answer`).
    #[arg(long, default_value_t = false)]
    no_ns_cue_raise_answer: bool,

    /// Disable opener's answer to a *minor*-opening cue-raise
    /// (`1m – (ovc) – cue – P`) (default-on; see `set_cue_minor_raise_answer`).
    #[arg(long, default_value_t = false)]
    no_ns_cue_minor_raise_answer: bool,

    /// Disable responder's structure over their two-suiters over our 1M — UvU
    /// over their both-minors `(2NT)`, the raise structure over their Michaels
    /// cue, and the two-suiter inference reading (shipped default-on; see
    /// `set_uvu_over_majors`).
    #[arg(long, default_value_t = false)]
    no_ns_uvu_over_majors: bool,

    /// Author our contested weak twos — business XX + systems-on Ogust over
    /// their double, Ogust-when-legal / values-X / preemptive raises over
    /// their overcall (default off; see `set_weak_two_competition`).
    #[arg(long, default_value_t = false)]
    ns_weak_two_comp: bool,

    /// Disable our contested strong 2♣ — systems-on over their double,
    /// natural GF / values-X / waiting-pass + forced reopening over their
    /// overcall (shipped default-on; see `set_strong_two_competition`).
    #[arg(long, default_value_t = false)]
    no_ns_strong_two_comp: bool,

    /// Disable opener's support double/redouble on `1♥ – (P) – 1♠` (shipped
    /// default-on; see `set_major_support_double`).
    #[arg(long, default_value_t = false)]
    no_ns_major_support_double: bool,

    /// Author responder's natural free bids over an overcall — 1-level new
    /// suit 5+ & 6+, 2-level non-jump 5+ & 10+, 1NT/2NT with a stopper
    /// (default off; implied by --ns-negative-double-shape modern|cachalot;
    /// see `set_free_bids`).
    #[arg(long, default_value_t = false)]
    ns_free_bids: bool,

    /// Minimum points/HCP for the 1-level free bids (default 6; sweep to 8+ to
    /// trim the free-bid family's vulnerable-PD leak; see `set_free_bid_floor`).
    #[arg(long, default_value_t = 6)]
    ns_free_bid_floor: u8,

    /// Minimum HCP for the free 1NT (`1X (1Y) 1NT`), decoupled from the suit
    /// floor above (default 6; see `set_free_1nt_floor`).
    #[arg(long, default_value_t = 6)]
    ns_free_1nt_floor: u8,

    /// Gate the vulnerable free bids on quality: a vulnerable 1-level new suit
    /// needs two of the top three honors, and the free 1NT is not authored
    /// vulnerable (default off; see `set_free_bid_quality`).
    #[arg(long, default_value_t = false)]
    ns_free_bid_quality: bool,

    /// The negative-double school over our minor openings:
    /// modern (shipped default) | both-majors | cachalot | sputnik
    /// (see `set_negative_double_shape`; all but both-majors imply the free
    /// bids and opener's forcing answers to them).
    #[arg(long, default_value = "modern")]
    ns_negative_double_shape: String,

    /// Responder's non-jump 2-level new suit over their overcall:
    /// forcing (shipped default — forcing one round) | negative (classic NFB:
    /// non-forcing 5-11 with a 6+ suit or strong 5-carder; stronger long-suit
    /// hands double then bid, forcing to game) | transfer (2-level slots swap
    /// and opener completes; see `set_free_bid_style`).
    #[arg(long, default_value = "forcing")]
    ns_free_bid_style: String,

    /// Author responder's structure over their jump / 3-level overcalls
    /// (2NT < bid ≤ 3♠): negative X through 3♠, forcing new suits, 3NT with a
    /// stopper (default off; see `set_high_overcall_responses`).
    #[arg(long, default_value_t = false)]
    ns_high_overcall: bool,

    /// Re-enable our takeout double on a flat 4-3-3-3 weaker than a 1NT opening
    /// (12–14 HCP flat 4333) — the default suppresses it and routes to Pass
    /// (shipped default-on; see `set_suppress_flat_4333_takeout`).
    #[arg(long, default_value_t = false)]
    no_ns_suppress_flat_4333_takeout: bool,

    /// Re-enable our takeout double on a weak `5-3-3-2` (12–13 HCP) — the default
    /// routes it to a natural overcall of the five-card suit (a 5-3-3-2 has no
    /// 4-card major, so the double cannot find a fit; shipped default-on; see
    /// `set_suppress_5332_takeout`).
    #[arg(long, default_value_t = false)]
    no_ns_suppress_5332_takeout: bool,

    /// Route a weak `4-4-3-2` (12–13 HCP) to Pass when the opponents opened a
    /// **major** (opt-in; see `set_suppress_4432_vs_major`).
    #[arg(long, default_value_t = false)]
    ns_suppress_4432_vs_major: bool,

    /// Route a weak `4-4-3-2` (12–13 HCP) to Pass when the opponents opened a
    /// **minor** (opt-in; see `set_suppress_4432_vs_minor`).
    #[arg(long, default_value_t = false)]
    ns_suppress_4432_vs_minor: bool,

    /// Re-enable our takeout double on a hand with an unbid five-card **major** —
    /// the default routes it to a natural overcall of the major (show the suit
    /// rather than double into partner's short suit; shipped default-on; see
    /// `set_suppress_5card_major_takeout`).
    #[arg(long, default_value_t = false)]
    no_ns_suppress_5card_major_takeout: bool,

    /// Disable the **rich advance** of partner's takeout double of a one-opening
    /// (`(1t)–X–(P)–?`) — revert to the flat advance without the cue + notrump
    /// invite/force ladder (shipped default-on; see `set_rich_advance_double`).
    #[arg(long, default_value_t = false)]
    no_ns_rich_advance: bool,

    /// Add the **jump-cue Rubens transfer** layer on top of the rich advance (a
    /// transfer to a 5+ unbid major; no-op unless `--ns-rich-advance`; opt-in,
    /// see `set_advance_rubens`).
    #[arg(long, default_value_t = false)]
    ns_advance_rubens: bool,

    /// Disable the advancer's **invitational minor jump** on the rich advance — a
    /// three-level minor jump = 5+ one-suiter, 10–12, denying a 4-card unbid major
    /// (with the doubler's stopper-ask cue continuation) — revert that rung to the
    /// floor (shipped default-on; no-op unless `--ns-rich-advance`; see
    /// `set_advance_minor_jump`).
    #[arg(long, default_value_t = false)]
    no_ns_advance_minor_jump: bool,

    /// Disable the **doubler's accept/decline of the advancer's `2NT` invite** on
    /// the rich advance (Pass = decline, 3NT = accept to play, new 5-card major =
    /// game-forcing) — revert to the floor, which passes `2NT` even holding a game
    /// (shipped default-on; no-op unless the rich advance is on; see
    /// `set_advance_2nt_continuation`).
    #[arg(long, default_value_t = false)]
    no_ns_advance_2nt_continuation: bool,

    /// Advance partner's takeout double with the **highest-ranking** eligible
    /// suit rather than the **longest** (higher-ranking on a tie); also governs
    /// the rich advance's weak natural and forced-suit picks (shipped default-on
    /// = longest; see `set_longest_first_advance`).
    #[arg(long, default_value_t = false)]
    no_ns_longest_advance: bool,

    /// Disable opener's balanced `1NT` rebid after `1m – 1M` — revert a balanced
    /// 12–14 with a five-card minor to the natural `2m` (shipped default-on; see
    /// `set_balanced_1nt_rebid`).
    #[arg(long, default_value_t = false)]
    no_ns_balanced_1nt_rebid: bool,

    /// Disable opener's strength-showing rebid ladder after a minor opening and a
    /// one-level response — revert jump-rebid / reverse / jump-shift to the
    /// minimum natural rebid (shipped default-on; see `set_opener_extras_ladder`).
    /// BBA-gap bucket #3.
    #[arg(long, default_value_t = false)]
    no_ns_opener_extras_ladder: bool,

    /// Disable opener's major jump-rebid rung (`1♥ – 1♠ – 3♥`, `1M – 1NT – 3M`)
    /// on a six-card major with 16+ and responder's continuation over it — the
    /// major-opening half of the extras ladder (shipped default-on; see
    /// `set_opener_major_jump_rebid`).
    #[arg(long, default_value_t = false)]
    no_ns_opener_major_jump_rebid: bool,

    /// Disable opener's third-call table after responder raises opener's second
    /// suit in a 2/1 auction (`1M – 2r – 2x – 3x`) — revert that node to the game
    /// backstop (shipped default-on; see `set_second_suit_agreement`).
    #[arg(long, default_value_t = false)]
    no_ns_second_suit_agreement: bool,

    /// Disable the competitive long-suit rebid — opener's/overcaller's rebid of a
    /// 6+ suit in competition (2-level any, 3-level needs 7 cards or a good six)
    /// instead of a forced takeout double (shipped default-on; see
    /// `set_competitive_rebid`).
    #[arg(long, default_value_t = false)]
    no_ns_competitive_rebid: bool,

    /// Disable opener's balanced-18-19 notrump actions in a `1X (1Y) …` auction
    /// the floor otherwise passes out: reopening 1NT, 3NT over responder's free
    /// 1NT, and responder's raise (default-on; see `set_reopening_notrump`).
    #[arg(long, default_value_t = false)]
    no_ns_reopening_notrump: bool,

    /// Disable the rein on a minimum takeout doubler that over-raises partner's
    /// forced advance of our double into a doubled game (default-on; see
    /// `set_rein_advance_raise`).
    #[arg(long, default_value_t = false)]
    no_ns_rein_advance_raise: bool,

    /// Disable opener's authored raise of a Cachalot X transfer when LHO
    /// competes over it (default-on; Cachalot only; see
    /// `set_cachalot_contested_x`).
    #[arg(long, default_value_t = false)]
    no_ns_cachalot_contested_x: bool,

    /// Disable responder's structure over their takeout double of our 1-suit
    /// opening: Jordan/Truscott 2NT, value XX, preemptive jump-raise flip,
    /// weak NF 2-level suits (shipped default-on; see `set_jordan_truscott`).
    #[arg(long, default_value_t = false)]
    no_ns_jordan_truscott: bool,

    /// Disable systems-on over their double of our splinter — revert to letting
    /// opener's rebid fall to the floor, which passes the doubled game force
    /// (shipped default-on; see `set_splinter_doubled`).
    #[arg(long, default_value_t = false)]
    no_ns_splinter_doubled: bool,

    /// Disable the major-rebid-tails adjunct — the full continuations after
    /// `1♥ – 1♠` below opener's `2♠`/`3♠` raise, `2♥` rebid, and `2♣`/`2♦`
    /// minor rebid (shipped default-on; see `set_major_rebid_tails`).
    #[arg(long, default_value_t = false)]
    no_ns_major_rebid_tails: bool,

    /// Disable fourth-suit-forcing — at `1♥ – 1♠ – 2♣`, responder's `2♦`
    /// reverts to the natural-tail reading (shipped default-on; rides the
    /// tails adjunct, so `--no-ns-major-rebid-tails` also silences it — see
    /// `set_fourth_suit_forcing`).
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

    /// Support gate on our 12+ takeout double of a suit / weak-two opening:
    /// off | lenient | strict (default, matches shipped `american()`).
    #[arg(long, default_value = "strict")]
    ns_takeout_support: String,

    /// Discipline our natural suit-overcall bands (1-level 8–17, 2-level 11–17)
    /// instead of the flat 8–16: on (default, matches shipped `american()`) | off.
    #[arg(long, default_value = "on")]
    ns_overcall_discipline: String,

    /// Disable a passed hand's lighter (9+ not 11+) disciplined 2-level overcall
    /// (folded into base default-on in the A5 pass; see `set_passed_hand_overcall`).
    #[arg(long, default_value_t = false)]
    no_ns_passed_hand_overcall: bool,

    /// Demand 15+ for the 2-level minor overcall (2♣/2♦ below their suit) instead
    /// of the disciplined 11+; off by default (A/B candidate — the anchor bleeds
    /// on these across every band, sd-lead confirms the loss is real).
    #[arg(long, default_value_t = false)]
    ns_two_level_minor_overcall_tight: bool,

    /// Bar a five-card major from the natural 1NT overcall (overcall the major
    /// instead, to find the fit); off by default (A/B candidate — buried majors
    /// miss the major game).
    #[arg(long, default_value_t = false)]
    ns_nt_overcall_no_major: bool,

    /// Disable systems-on advances after our 1NT overcall: on, the advancer plays
    /// the full opening-1NT structure (Stayman/transfers/Smolen) grafted below
    /// `[1t,1NT]`, finding and right-siding major fits; on by default. Off-switch
    /// for the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_nt_overcall_systems_on: bool,

    /// Gladiator advances after our 1NT overcall of their *major* (replaces the
    /// opening-1NT graft over majors only): 2♣ weak relay, cue-of-major Stayman
    /// for the unbid major, natural INV, splinter/Leaping-Michaels. Off by default
    /// (A/B candidate — the major graft washes plain/PD, wins only on sd-lead).
    #[arg(long, default_value_t = false)]
    ns_nt_overcall_gladiator: bool,

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

    /// Replace our natural 1NT defense with Meckwell (default off): two-way X =
    /// single 6+ minor OR both majors, 2♣ = clubs + a major, 2♦ = diamonds + a major,
    /// 2♥/2♠ = natural single-suiters, 2NT = both minors.  Run WITHOUT
    /// `--advertise-natural` (BBA reads it via its DONT convention).
    #[arg(long, default_value_t = false)]
    ns_meckwell: bool,

    /// Probe: let Meckwell's `2♣`/`2♦` accept a flat 4-4 (default off = 5-4+). Only
    /// with `--ns-meckwell`.
    #[arg(long, default_value_t = false)]
    ns_meckwell_minor_major_44: bool,

    /// Probe: let Meckwell's both-majors `X` accept a flat 4-4 (default on = 4-4; set
    /// false-ish by passing `--ns-meckwell-x-five-four` for 5-4+). Only with
    /// `--ns-meckwell`.
    #[arg(long, default_value_t = false)]
    ns_meckwell_x_five_four: bool,

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

    /// Disable Rubens advances of partner's simple overcall (default on): the
    /// transfers/cue-raise revert to natural raises plus a natural two-level
    /// new-suit advance — the natural-advances baseline for the A/B.
    #[arg(long, default_value_t = false)]
    no_ns_rubens: bool,

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

    /// Suppress opener correcting partner's choice-of-games 3NT to 4M with a known
    /// eight-card major fit.  Gated on undisturbed + a ruffing doubleton it wins
    /// +0.0062 IMPs/board plain / +0.0068 PD (two seeds), so it is on by default.
    #[arg(long, default_value_t = false)]
    no_ns_correct_3nt_to_major: bool,
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

// The BBA oracle (`BbaOracle`) and the `&dyn System` match drivers
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
    pons::bidding::instinct::set_correct_3nt_to_major(!args.no_ns_correct_3nt_to_major);
    pons::bidding::set_alert_reading(!args.no_alert_reading);
    pons::bidding::instinct::set_penalty_latch(!args.no_ns_penalty_latch);
    pons::bidding::instinct::set_penalty_no_pull(!args.ns_allow_pull);
    pons::bidding::instinct::set_advancer_xx_runout(!args.no_ns_xx_runout);
    pons::bidding::instinct::set_doubler_xx_runout(!args.no_ns_doubler_run);
    pons::bidding::instinct::set_rubens_advances(!args.no_ns_rubens);
    pons::bidding::set_rubens_transfer_reading(!args.no_ns_rubens_reading);
    pons::bidding::instinct::set_floor_rkcb(!args.no_ns_floor_rkcb);
    pons::bidding::set_control_bid_reading(!args.no_ns_control_bid_reading);
    pons::bidding::set_cue_reading(!args.no_ns_cue_reading);
    pons::bidding::set_length_soundness(!args.no_ns_length_soundness);
    pons::bidding::set_table_alert_reading(!args.no_ns_table_alert_reading);
    pons::bidding::set_pass_reading(!args.no_ns_pass_reading);
    pons::bidding::american::set_transfer_longer_major(!args.no_ns_transfer_longer);
    pons::bidding::set_fallback_projection(!args.no_ns_fallback_projection);
    pons::bidding::american::set_open_one_notrump(!args.no_our_1nt);
    pons::bidding::american::set_one_notrump_fifths(args.nt_fifths);
    pons::bidding::american::set_rule_of_20(!args.no_ns_rule_of_20);
    pons::bidding::american::set_natural_double_shape(match args.ns_double_shape.as_str() {
        "any" => DoubleShape::Any,
        "semi" => DoubleShape::SemiBalanced,
        "balanced" => DoubleShape::Balanced,
        other => anyhow::bail!("--ns-double-shape must be any|semi|balanced, got {other:?}"),
    });
    pons::bidding::american::set_natural_double_floor(args.ns_double_floor);
    pons::bidding::american::set_natural_double_weight(args.ns_double_weight);
    pons::bidding::american::set_takeout_support(match args.ns_takeout_support.as_str() {
        "off" => pons::bidding::american::TakeoutSupport::Off,
        "lenient" => pons::bidding::american::TakeoutSupport::Lenient,
        "strict" => pons::bidding::american::TakeoutSupport::Strict,
        other => anyhow::bail!("--ns-takeout-support must be off|lenient|strict, got {other:?}"),
    });
    pons::bidding::american::set_overcall_discipline(match args.ns_overcall_discipline.as_str() {
        "on" => true,
        "off" => false,
        other => anyhow::bail!("--ns-overcall-discipline must be on|off, got {other:?}"),
    });
    pons::bidding::american::set_passed_hand_overcall(!args.no_ns_passed_hand_overcall);
    pons::bidding::american::set_two_level_minor_overcall_tight(
        args.ns_two_level_minor_overcall_tight,
    );
    pons::bidding::american::set_nt_overcall_no_major(args.ns_nt_overcall_no_major);
    pons::bidding::american::set_nt_overcall_systems_on(!args.no_ns_nt_overcall_systems_on);
    pons::bidding::american::set_nt_overcall_gladiator(args.ns_nt_overcall_gladiator);
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
    pons::bidding::american::set_minor_keycard(!args.no_ns_minor_keycard);
    pons::bidding::american::set_transfer_gf_majors(!args.no_ns_transfer_gf_majors);
    pons::bidding::american::set_minor_min_to_3nt(args.ns_minor_min_to_3nt);
    pons::bidding::american::set_transfer_gf_hearts(!args.no_ns_transfer_gf_hearts);
    pons::bidding::american::set_garbage_stayman(!args.no_ns_garbage_stayman);
    pons::bidding::american::set_stayman_both_majors(!args.no_ns_stayman_both_majors);
    pons::bidding::american::set_stayman_5card_max(!args.no_ns_stayman_5card_max);
    pons::bidding::american::set_invitational_5card_majors(!args.no_ns_invitational_5card_majors);
    pons::bidding::american::set_crawling_stayman(!args.no_ns_crawling_stayman);
    pons::bidding::american::set_stayman_cue_continuation(!args.no_ns_stayman_cue_continuation);
    pons::bidding::american::set_longer_major_response(!args.no_ns_longer_major_response);
    pons::bidding::american::set_up_the_line(!args.no_ns_up_the_line);
    pons::bidding::american::set_major_choice_of_games(!args.no_ns_major_choice_of_games);
    pons::bidding::american::set_two_over_one_fit(!args.no_ns_two_over_one_fit);
    pons::bidding::american::set_two_over_one_gate(match args.ns_two_over_one_gate.as_str() {
        "points13" => pons::bidding::american::TwoOverOneGate::Points13,
        "hcp13" => pons::bidding::american::TwoOverOneGate::Hcp13,
        "hcp12" => pons::bidding::american::TwoOverOneGate::Hcp12,
        other => {
            anyhow::bail!("--ns-two-over-one-gate must be points13|hcp13|hcp12, got {other:?}")
        }
    });
    pons::bidding::american::set_xyz(!args.no_ns_xyz);
    pons::bidding::american::set_new_minor_forcing(args.ns_new_minor_forcing);
    pons::bidding::american::set_major_game_tries(!args.no_ns_major_game_tries);
    pons::bidding::american::set_limit_raise_acceptance(!args.no_ns_limit_raise_acceptance);
    pons::bidding::american::set_cue_raise_answer(!args.no_ns_cue_raise_answer);
    pons::bidding::american::set_cue_minor_raise_answer(!args.no_ns_cue_minor_raise_answer);
    pons::bidding::american::set_uvu_over_majors(!args.no_ns_uvu_over_majors);
    pons::bidding::american::set_weak_two_competition(args.ns_weak_two_comp);
    pons::bidding::american::set_strong_two_competition(!args.no_ns_strong_two_comp);
    pons::bidding::american::set_major_support_double(!args.no_ns_major_support_double);
    pons::bidding::american::set_free_bids(args.ns_free_bids);
    pons::bidding::american::set_free_bid_floor(args.ns_free_bid_floor);
    pons::bidding::american::set_free_1nt_floor(args.ns_free_1nt_floor);
    pons::bidding::american::set_free_bid_quality(args.ns_free_bid_quality);
    pons::bidding::american::set_negative_double_shape(
        match args.ns_negative_double_shape.as_str() {
            "both-majors" => pons::bidding::american::NegativeDoubleShape::BothMajors,
            "modern" => pons::bidding::american::NegativeDoubleShape::Modern,
            "cachalot" => pons::bidding::american::NegativeDoubleShape::Cachalot,
            "sputnik" => pons::bidding::american::NegativeDoubleShape::Sputnik,
            other => anyhow::bail!(
                "--ns-negative-double-shape must be both-majors|modern|cachalot|sputnik, got {other:?}"
            ),
        },
    );
    pons::bidding::american::set_free_bid_style(match args.ns_free_bid_style.as_str() {
        "forcing" => pons::bidding::american::FreeBidStyle::Forcing,
        "negative" => pons::bidding::american::FreeBidStyle::Negative,
        "transfer" => pons::bidding::american::FreeBidStyle::Transfer,
        other => {
            anyhow::bail!("--ns-free-bid-style must be forcing|negative|transfer, got {other:?}")
        }
    });
    pons::bidding::american::set_high_overcall_responses(args.ns_high_overcall);
    pons::bidding::constraint::set_suppress_flat_4333_takeout(
        !args.no_ns_suppress_flat_4333_takeout,
    );
    pons::bidding::constraint::set_suppress_5332_takeout(!args.no_ns_suppress_5332_takeout);
    pons::bidding::constraint::set_suppress_4432_vs_major(args.ns_suppress_4432_vs_major);
    pons::bidding::constraint::set_suppress_4432_vs_minor(args.ns_suppress_4432_vs_minor);
    pons::bidding::constraint::set_suppress_5card_major_takeout(
        !args.no_ns_suppress_5card_major_takeout,
    );
    pons::bidding::american::set_rich_advance_double(!args.no_ns_rich_advance);
    pons::bidding::american::set_advance_rubens(args.ns_advance_rubens);
    pons::bidding::american::set_advance_minor_jump(!args.no_ns_advance_minor_jump);
    pons::bidding::american::set_advance_2nt_continuation(!args.no_ns_advance_2nt_continuation);
    pons::bidding::american::set_longest_first_advance(!args.no_ns_longest_advance);
    pons::bidding::american::set_cachalot_contested_x(!args.no_ns_cachalot_contested_x);
    pons::bidding::american::set_balanced_1nt_rebid(!args.no_ns_balanced_1nt_rebid);
    pons::bidding::american::set_opener_extras_ladder(!args.no_ns_opener_extras_ladder);
    pons::bidding::american::set_opener_major_jump_rebid(!args.no_ns_opener_major_jump_rebid);
    pons::bidding::american::set_second_suit_agreement(!args.no_ns_second_suit_agreement);
    pons::bidding::instinct::set_competitive_rebid(!args.no_ns_competitive_rebid);
    pons::bidding::instinct::set_reopening_notrump(!args.no_ns_reopening_notrump);
    pons::bidding::instinct::set_rein_advance_raise(!args.no_ns_rein_advance_raise);
    pons::bidding::american::set_jordan_truscott(!args.no_ns_jordan_truscott);
    pons::bidding::american::set_splinter_doubled(!args.no_ns_splinter_doubled);
    pons::bidding::american::set_major_rebid_tails(!args.no_ns_major_rebid_tails);
    pons::bidding::american::set_fourth_suit_forcing(!args.no_ns_fourth_suit_forcing);
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
    pons::bidding::american::set_meckwell(args.ns_meckwell);
    if args.ns_meckwell {
        pons::bidding::american::set_natural_defense(false);
        pons::bidding::american::set_landy(None);
        pons::bidding::american::set_direct_dont(false);
        pons::bidding::american::set_unusual_notrump_defense(Some((8, 14)));
        pons::bidding::american::set_meckwell_minor_major_44(args.ns_meckwell_minor_major_44);
        pons::bidding::american::set_meckwell_x_four_four(!args.ns_meckwell_x_five_four);
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
        "american" => {
            use pons::bidding::american::{american_classic, american_wide};
            let pair = match args.nt_shape.as_str() {
                // `american()` now ships Wide6322 (the shipped default); `wide`
                // is the superseded 5422-minor baseline, `classic` balanced-only.
                "wide6322" => american(),
                "wide" => american_wide(),
                "classic" => american_classic(),
                other => anyhow::bail!("--nt-shape must be wide|classic|wide6322, got {other:?}"),
            };
            pair.against(Family::NATURAL)
        }
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
    let our_oracle = match our_system {
        Some(system) => Some(BbaOracle::load(&path, system, our_conv.clone())?),
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
    // Labels name the card file rather than spelling out its ~257 toggles;
    // explicit `--*-conv` singles still render individually.
    let our_label = match our_system {
        Some(system) => format!(
            "BBA {}{}{}",
            system_label(system),
            label_card(&args.our_card),
            label_overrides(&args.our_conv)
        ),
        None => format!("our {} floor", args.our_floor),
    };
    let their_label = format!(
        "BBA {}{}{}",
        system_label(args.system),
        label_card(&args.their_card),
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
    Ok(())
}
