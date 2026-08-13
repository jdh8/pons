//! Distill BBA/EPBot's Multi-Landy defense to a 1NT opening into the constraint
//! DSL by **sample-and-probe**: deal random actor hands, drive the real EPBot
//! engine for a fixed `(seat, auction)`, bucket each hand by the call it returns,
//! and summarise every bucket in DSL vocabulary (HCP / suit length / balanced).
//!
//! BBA's compiled 2/1 card answers a 1NT opening with **Multi-Landy**, whose
//! `2♦` is the *Multi* (an unknown single-suited major) and whose `2♥`/`2♠` are
//! *Muiderberg* (5+ major, 4+ minor).  Several `--mode`s read the seats of that
//! structure.  The `.so` ignores `vendor/bba/*.bbsa`, so this real-hand probe is
//! the only reliable read (see `probe-bba-1nt`); we force `Multi-Landy=1` (and
//! `Cappelletti=0`) on all seats so BBA both *bids* and *interprets* the calls:
//!
//! ```text
//! cargo run --release --example probe-bba-constraints -- --mode multi    # direct call over (1NT): X/2♣/2♦/2♥/2♠
//! cargo run --release --example probe-bba-constraints -- --mode advance  # advancer over the 2♦ Multi
//! cargo run --release --example probe-bba-constraints -- --mode muider-h # advancer over the 2♥ Muiderberg
//! cargo run --release --example probe-bba-constraints -- --mode muider-s # advancer over the 2♠ Muiderberg
//! cargo run --release --example probe-bba-constraints -- --mode rebid-d  # 2♦-overcaller's rebid (Multi → which major)
//! cargo run --release --example probe-bba-constraints -- --mode rebid-h  # 2♥-overcaller's rebid after the 2NT ask
//! cargo run --release --example probe-bba-constraints -- --mode rebid-s  # 2♠-overcaller's rebid after the 2NT ask
//! cargo run --release --example probe-bba-constraints -- --mode counter --vul none,both  # our-side counter-defense
//! cargo run --release --example probe-bba-constraints -- --mode weak2-h  # opener's rebid over 2♥ - 2NT -
//! cargo run --release --example probe-bba-constraints -- --mode weak2-h --conv Ogust=1  # ...with BBA's Ogust on
//! cargo run --release --example probe-bba-constraints -- --mode nt-resp --conv "1N-3M splinter"=1  # responses to BBA's own 1NT
//! cargo run --release --example probe-bba-constraints -- --mode nt-3h --conv "1N-3M splinter"=1    # opener over 1NT - 3♥ -
//! cargo run --release --example probe-bba-constraints -- --mode nt-3c-3d --conv "1N-3C transfer to diamonds"=1 --conv "1N-3C Puppet Stayman"=0  # responder over 1NT - 3♣ - 3♦ -
//! cargo run --release --example probe-bba-constraints -- --mode nt-2s-3c --conv "1N-2S transfer to clubs"=1  # responder over 1NT - 2♠ - 3♣ -
//! cargo run --release --example probe-bba-constraints -- --mode def1-s   # direct defense to (1♠)
//! cargo run --release --example probe-bba-constraints -- --mode o4 --vul none,we,they,both  # fit the two-level overcall quality gate
//! ```
//!
//! The `weak2-*` modes read a node we author as **Ogust** and BBA does not: its
//! 2/1 default is `Ogust = 0` (confirmed against the live engine, since the `.so`
//! ignores the cards).  `libEPBot.so` does carry a full Ogust implementation
//! (`odzywka_OGUST`, and an `_interpretacja` for both sides), so `--conv Ogust=1`
//! reads the same node with it enabled — the two runs together decide whether to
//! match BBA's ladder or to teach BBA ours.
//!
//! Each `sketch:` line is a *candidate* constraint to verify and hand-author,
//! not a proof of BBA's internal logic.

#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use anyhow::{Result, bail};
use clap::Parser;
use contract_bridge::AbsoluteVulnerability;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::fill_deals;
use contract_bridge::eval::{self, HandEvaluator, SimpleEvaluator};
use contract_bridge::{Bid, Strain};
use contract_bridge::{Builder, Hand, Holding, Rank, Seat, Suit};
use libloading::Library;
use pons::american;
use pons::bidding::Partnership;
use pons::bidding::constraint::{point_count, support_point_count_in};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::{IndexedRandom, SliceRandom};
use std::collections::BTreeMap;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fmt::Write as _;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::next_call;

const DEFAULT_LIB: &str = "vendor/bba/Native-libraries/linux/x64/libEPBot.so";

// EPBot bid codes: Pass = 0, X = 1; a bid is 5 + (level-1)*5 + strain (♣..NT).
const PASS: c_int = 0;
const ONE_C: c_int = 5; // 5 + 0*5 + 0
const ONE_D: c_int = 6; // 5 + 0*5 + 1
const ONE_H: c_int = 7; // 5 + 0*5 + 2
const ONE_S: c_int = 8; // 5 + 0*5 + 3
const ONE_NT: c_int = 9; // 5 + 0*5 + 4
const TWO_C: c_int = 10; // 5 + 1*5 + 0
const TWO_D: c_int = 11; // 5 + 1*5 + 1
const TWO_H: c_int = 12; // 5 + 1*5 + 2
const TWO_S: c_int = 13; // 5 + 1*5 + 3
const TWO_NT: c_int = 14; // 5 + 1*5 + 4
const THREE_C: c_int = 15; // 5 + 2*5 + 0
const THREE_D: c_int = 16; // 5 + 2*5 + 1
const THREE_H: c_int = 17; // 5 + 2*5 + 2
const THREE_S: c_int = 18; // 5 + 2*5 + 3

type CreateFn = unsafe extern "C" fn() -> *mut c_void;
type DestroyFn = unsafe extern "C" fn(*mut c_void);
type SetSystemFn = unsafe extern "C" fn(*mut c_void, c_int, c_int);
type NewHandFn =
    unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int, c_int, c_int, c_int);
type SetBidFn = unsafe extern "C" fn(*mut c_void, c_int, c_int, *const c_char);
type GetBidFn = unsafe extern "C" fn(*mut c_void) -> c_int;
type SetConvFn = unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int) -> c_int;

/// Cached EPBot entry points (copied out of the [`Library`], which we keep alive)
struct Bba {
    _lib: Library,
    create: CreateFn,
    destroy: DestroyFn,
    set_system: SetSystemFn,
    new_hand: NewHandFn,
    set_bid: SetBidFn,
    get_bid: GetBidFn,
    set_conv: SetConvFn,
    /// Named conventions forced on all four seats after `set_system`.
    overrides: Vec<(CString, c_int)>,
}

impl Bba {
    fn load(path: &str, overrides: Vec<(CString, c_int)>) -> Result<Self> {
        // SAFETY: loading a trusted native library; symbol signatures match the
        // ABI confirmed by `bba-match`/`probe-bba-1nt`.
        let lib = unsafe { Library::new(path) }?;
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
                overrides,
            })
        }
    }

    /// The call `actor` makes (system 0, 2/1 GF) holding `hand` after `prefix` is
    /// replayed from a dealer canonicalized to seat 0.  A fresh bot per call keeps
    /// this a pure function of its arguments.
    fn call(&self, actor: c_int, prefix: &[c_int], hand: Hand, vul: c_int) -> c_int {
        let suits = hand_to_suits(hand);
        let empty = c"".as_ptr();
        // SAFETY: a fresh bot used and destroyed here; all argument types match
        // the confirmed ABI; `suits` outlives the `new_hand` call.
        unsafe {
            let bot = (self.create)();
            assert!(!bot.is_null(), "epbot_create returned null");
            for seat in 0..4 {
                (self.set_system)(bot, seat, 0);
            }
            for (name, value) in &self.overrides {
                for seat in 0..4 {
                    (self.set_conv)(bot, seat, name.as_ptr(), *value);
                }
            }
            (self.new_hand)(bot, actor, suits.as_ptr(), 0, vul, 0, 0);
            for (index, &code) in prefix.iter().enumerate() {
                (self.set_bid)(bot, (index % 4) as c_int, code, empty);
            }
            let code = (self.get_bid)(bot);
            (self.destroy)(bot);
            code
        }
    }
}

/// The four holdings in EPBot's C,D,H,S order, newline-joined (13 cards).  See
/// `bba-match::hand_to_suits` — `Holding`'s `Display` is EPBot's canonical form.
fn hand_to_suits(hand: Hand) -> CString {
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

/// The strains in EPBot's code order (♣ ♦ ♥ ♠ NT), for code ↔ [`Call`]
const STRAINS: [Strain; 5] = [
    Strain::Clubs,
    Strain::Diamonds,
    Strain::Hearts,
    Strain::Spades,
    Strain::Notrump,
];

/// An EPBot bid code as a [`Call`], so the `--ours` arm can replay the same node
/// through our own bidder and be bucketed by the same code.
fn code_to_call(code: c_int) -> Option<Call> {
    match code {
        0 => Some(Call::Pass),
        1 => Some(Call::Double),
        2 => Some(Call::Redouble),
        5..=39 => {
            let i = (code - 5) as u8;
            Some(Call::Bid(Bid::new(i / 5 + 1, STRAINS[(i % 5) as usize])))
        }
        _ => None,
    }
}

/// The inverse of [`code_to_call`] — our bidder's call as an EPBot code
fn call_to_code(call: Call) -> c_int {
    match call {
        Call::Pass => 0,
        Call::Double => 1,
        Call::Redouble => 2,
        Call::Bid(bid) => {
            let strain = STRAINS
                .iter()
                .position(|&s| s == bid.strain)
                .expect("every strain is in STRAINS");
            5 + (c_int::from(bid.level.get()) - 1) * 5 + strain as c_int
        }
    }
}

/// Our own bidder's call at the probed node — the `--ours` arm
///
/// BBA's buckets answer *what the opponents mean by this call*; ours answer
/// *what we actually do with it*, which is the other half of whether a reading
/// that only fits our own agreement should be side-gated or dropped outright.
/// The book rule may be `-∞` for a hand and the floor still choose the call, so
/// this replays the whole bidder, not the rule (docs/ai-bidder/sampled-projection.md).
fn our_call(
    partnership: &Partnership,
    prefix: &[c_int],
    hand: Hand,
    vul: AbsoluteVulnerability,
) -> Call {
    let mut auction = Auction::new();
    for &code in prefix {
        let call = code_to_call(code).expect("probe prefixes are legal calls");
        auction.push(call);
    }
    next_call(partnership, hand, Seat::North, vul, &auction)
}

/// Decode an EPBot bid code into a label, or `None` for an error/illegal code
fn decode(code: c_int) -> Option<String> {
    const STRAIN: [&str; 5] = ["♣", "♦", "♥", "♠", "NT"];
    match code {
        0 => Some("Pass".into()),
        1 => Some("X".into()),
        2 => Some("XX".into()),
        5..=39 => {
            let i = code - 5;
            Some(format!("{}{}", i / 5 + 1, STRAIN[(i % 5) as usize]))
        }
        _ => None,
    }
}

/// The EPBot vulnerability code (bit 1 = N/S, bit 2 = E/W) for `we`/`they`
/// relative to `actor`; even seats are N/S.  Mirrors `bba-match::epbot_vulnerability`.
fn vul_code(token: &str, actor: c_int) -> Result<c_int> {
    let (we, they) = match token {
        "none" => (false, false),
        "we" => (true, false),
        "they" => (false, true),
        "both" => (true, true),
        other => bail!("--vul must be none|we|they|both, got {other:?}"),
    };
    let (ns, ew) = if actor % 2 == 0 {
        (we, they)
    } else {
        (they, we)
    };
    Ok(c_int::from(ns) | (c_int::from(ew) << 1))
}

/// Per-call accumulator: every probe hand BBA mapped to this call.
#[derive(Default)]
struct Bucket {
    hcp: Vec<u8>,
    /// Suit lengths in [`Suit::ASC`] order (♣ ♦ ♥ ♠).
    len: [Vec<u8>; 4],
    balanced: usize,
    /// A/K/Q count in the mode's trump suit; empty unless the mode names one.
    /// An Ogust-style ladder splits on suit *quality*, which neither the HCP
    /// nor the length columns can show.
    tops: Vec<u8>,
    /// Rival definitions of the same "good suit", to find which one BBA uses:
    /// A/K/Q/J count, A/K/Q/J/10 count, and plain HCP inside the trump suit.
    tops4: Vec<u8>,
    tops5: Vec<u8>,
    trump_hcp: Vec<u8>,
    /// Support points in the mode's *support* suit, and the legacy `points`
    /// image; empty unless the mode names one.  The `ucb-*` modes need these:
    /// `rubens_reading`'s cue block asserts a floor on that scale, not on HCP.
    sp: Vec<u8>,
    points: Vec<u8>,
}

#[derive(Parser)]
#[command(about = "Distill BBA's Multi-Landy 2♦ defense (and its counter) into the DSL")]
struct Args {
    /// Which seat to probe: multi (2♦ overcaller) | advance (advancer relay) | counter (our-side defense)
    #[arg(long, default_value = "multi")]
    mode: String,

    /// Comma-separated vulnerabilities to report: none,we,they,both
    #[arg(long, default_value = "none")]
    vul: String,

    /// Probe hands per vulnerability
    #[arg(long, default_value_t = 5000)]
    samples: usize,

    /// Two-sided percentile trim for the reported HCP range (0.05 = 5th–95th)
    #[arg(long, default_value_t = 0.05)]
    trim: f64,

    /// Skip calls chosen on fewer than this fraction of probe hands
    #[arg(long, default_value_t = 0.01)]
    min_share: f64,

    /// RNG seed (fixed by default; the same hands are reused across vulnerabilities)
    #[arg(long, default_value_t = 0)]
    seed: u64,

    /// Force a named convention: NAME=0|1 (repeatable). Default: Multi-Landy=1, Cappelletti=0
    #[arg(long = "conv")]
    conv: Vec<String>,

    /// Probe **our own** bidder at the same node instead of BBA's (`ucb-*` only)
    #[arg(long)]
    ours: bool,

    /// Inclusive whole-hand HCP range for the O4 controlled grid (LO:HI)
    #[arg(long, default_value = "6:18")]
    o4_hcp: String,

    /// Independently generated controlled hands per O4 grid cell
    #[arg(long, default_value_t = 2)]
    o4_replicates: usize,

    /// Held-out random one-suit hands per O4 (opening, candidate-suit) pair
    #[arg(long, default_value_t = 1000)]
    o4_holdout: usize,

    /// Optional output file (default: stdout)
    #[arg(long)]
    out: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if !(0.0..0.5).contains(&args.trim) {
        bail!("--trim must be in [0.0, 0.5)");
    }

    if args.mode == "o4" {
        return run_o4(&args);
    }

    // (actor seat, replayed prefix, self-consistency filter, heading) — dealer is
    // canonicalized to seat 0.  `filter` is the direct-seat call a probe hand must
    // make over (1NT) to be kept; `None` accepts every hand (the advancer's hand is
    // unconstrained, so its modes need no filter).  The `rebid-*` modes read the
    // overcaller's own seat, so they keep only hands BBA would actually overcall.
    let (actor, prefix, filter, what): (c_int, &[c_int], Option<c_int>, &str) = match args
        .mode
        .as_str()
    {
        // Dealer opening, nothing replayed.  Reads what the `1NT opening *`
        // rows of a `.bbsa` card actually do — `1NT opening natural` and
        // `1NT opening NT style` move BBA's calls when disclosed
        // (docs/ai-bidder/bba-disclosure-sweep.md) but their meaning is not
        // documented anywhere, and the card claims `natural = 0` for a system
        // whose 1NT is a natural 15-17.
        "open" => (
            0,
            &[],
            None,
            "BBA's opening call in first seat — the 1NT bucket is the one to read",
        ),
        "def1-c" => (
            1,
            &[ONE_C],
            None,
            "BBA direct seat over (1♣) — X / 1NT / natural overcalls / cue bids",
        ),
        "def1-d" => (
            1,
            &[ONE_D],
            None,
            "BBA direct seat over (1♦) — X / 1NT / natural overcalls / cue bids",
        ),
        "def1-h" => (
            1,
            &[ONE_H],
            None,
            "BBA direct seat over (1♥) — X / 1NT / natural overcalls / cue bids",
        ),
        "def1-s" => (
            1,
            &[ONE_S],
            None,
            "BBA direct seat over (1♠) — X / 1NT / natural overcalls / cue bids",
        ),
        "multi" => (
            1,
            &[ONE_NT],
            None,
            "BBA's direct call over (1NT) — the 2♦ bucket is the Multi",
        ),
        "advance" => (
            3,
            &[ONE_NT, TWO_D, PASS],
            None,
            "BBA advancer over (1NT) 2♦ - — the pass-or-correct relay",
        ),
        "counter" => (
            2,
            &[ONE_NT, TWO_D],
            None,
            "BBA responder's counter-defense over 1NT (2♦)",
        ),
        "muider-h" => (
            3,
            &[ONE_NT, TWO_H, PASS],
            None,
            "BBA advancer over (1NT) 2♥ - — the Muiderberg advance (2NT/3♣/3♦ asks)",
        ),
        "muider-s" => (
            3,
            &[ONE_NT, TWO_S, PASS],
            None,
            "BBA advancer over (1NT) 2♠ - — the Muiderberg advance (2NT/3♣/3♦ asks)",
        ),
        "rebid-d" => (
            1,
            &[ONE_NT, TWO_D, PASS, TWO_H, PASS],
            Some(TWO_D),
            "BBA 2♦-overcaller's rebid over (1NT) 2♦ - 2♥ - — Pass=hearts, 2♠=spades",
        ),
        "rebid-d2s" => (
            1,
            &[ONE_NT, TWO_D, PASS, TWO_S, PASS],
            Some(TWO_D),
            "BBA 2♦-overcaller's rebid over (1NT) 2♦ - 2♠ - — what the 2♠ advance forces",
        ),
        "rebid-h" => (
            1,
            &[ONE_NT, TWO_H, PASS, TWO_NT, PASS],
            Some(TWO_H),
            "BBA 2♥-overcaller's rebid over (1NT) 2♥ - 2NT - — what the 2NT ask wants",
        ),
        "rebid-s" => (
            1,
            &[ONE_NT, TWO_S, PASS, TWO_NT, PASS],
            Some(TWO_S),
            "BBA 2♠-overcaller's rebid over (1NT) 2♠ - 2NT - — what the 2NT ask wants",
        ),
        // Defense to the opponents' 1NT *response* (Stayman / Jacoby transfers),
        // 4th seat at `(1NT) - (2x)`. No filter — the 4th-seat hand is unconstrained.
        "stayman" => (
            3,
            &[ONE_NT, PASS, TWO_C],
            None,
            "BBA 4th-seat over (1NT) - (2♣) Stayman — X=clubs, natural, no 2NT",
        ),
        "xfer-h" => (
            3,
            &[ONE_NT, PASS, TWO_D],
            None,
            "BBA 4th-seat over (1NT) - (2♦), a transfer to hearts — X=diamonds, 2♥ cue=spades+minor",
        ),
        "xfer-s" => (
            3,
            &[ONE_NT, PASS, TWO_H],
            None,
            "BBA 4th-seat over (1NT) - (2♥), a transfer to spades — X=hearts, 2♠ cue=hearts+minor",
        ),
        // Opener's rebid after our own weak two is asked with 2NT.  We author
        // Ogust here; BBA's 2/1 default has `Ogust = 0` (verified against the live
        // engine by `bba-conv-probe`, not the card — the `.so` ignores the file),
        // so these read what its ladder means instead.  Pair with `--conv Ogust=1`
        // to read the same node with BBA's own Ogust switched on.
        "weak2-d" => (
            0,
            &[TWO_D, PASS, TWO_NT, PASS],
            Some(TWO_D),
            "BBA opener's rebid over 2♦ - 2NT - — what the 2NT ask wants",
        ),
        "weak2-h" => (
            0,
            &[TWO_H, PASS, TWO_NT, PASS],
            Some(TWO_H),
            "BBA opener's rebid over 2♥ - 2NT - — what the 2NT ask wants",
        ),
        "weak2-s" => (
            0,
            &[TWO_S, PASS, TWO_NT, PASS],
            Some(TWO_S),
            "BBA opener's rebid over 2♠ - 2NT - — what the 2NT ask wants",
        ),
        // Direct seat over THEIR weak two — the whole toolkit at once: the 2NT
        // overcall's range and shape, the takeout double's floor, the natural
        // overcalls, whether a jump is authored, and what the cue means.
        "def2-d" => (
            1,
            &[TWO_D],
            None,
            "BBA direct seat over (2♦) — X / 2NT / natural overcalls / 3♦ cue",
        ),
        "def2-h" => (
            1,
            &[TWO_H],
            None,
            "BBA direct seat over (2♥) — X / 2NT / natural overcalls / 3♥ cue",
        ),
        "def2-s" => (
            1,
            &[TWO_S],
            None,
            "BBA direct seat over (2♠) — X / 2NT / natural overcalls / 3♠ cue",
        ),
        // Constructive: BBA's *own* 1NT structure.  `1N-3M splinter` is on in the
        // stock 21GF card and off in ours, and the `.so` ignores both — pass
        // `--conv "1N-3M splinter"=0|1` to read the slot either way.
        "nt-resp" => (
            2,
            &[ONE_NT, PASS],
            None,
            "BBA responder over its own 1NT - — the 3♥/3♠ buckets are the splinter",
        ),
        "nt-3h" => (
            0,
            &[ONE_NT, PASS, THREE_H, PASS],
            Some(ONE_NT),
            "BBA opener over 1NT - 3♥ - — a natural read raises hearts, a splinter never does",
        ),
        "nt-3s" => (
            0,
            &[ONE_NT, PASS, THREE_S, PASS],
            Some(ONE_NT),
            "BBA opener over 1NT - 3♠ - — a natural read raises spades, a splinter never does",
        ),
        // Opener's answer to the European diamond transfer.  Our
        // `european_three_club_answer` completes to `3♦` unconditionally
        // (`hcp(0..)`); a second bucket here would mean a super-accept we do
        // not model.
        "nt-3c" => (
            0,
            &[ONE_NT, PASS, THREE_C, PASS],
            Some(ONE_NT),
            "BBA opener over 1NT - 3♣ - — a single 3♦ bucket means the completion is unconditional",
        ),
        // The European minor scheme's **club** lane, one round on: `2♠` is the
        // transfer and `3♣` the completion, so this reads **responder's** rebid
        // — the node `european_two_spade_rebid` authors `3♦`/`3♥`/`3♠`
        // splinters at.  Needs `1N-2S transfer to clubs`=1, or the `2♠` filter
        // accepts nothing.
        "nt-2s-3c" => (
            2,
            &[ONE_NT, PASS, TWO_S, PASS, THREE_C, PASS],
            Some(TWO_S),
            "BBA responder over 1NT - 2♠ - 3♣ - — a 3♦/3♥/3♠ bucket is the splinter",
        ),
        // The European minor scheme's diamond lane, one round on: `3♣` is the
        // transfer and `3♦` the completion, so this reads **responder's** rebid.
        // A `3♥`/`3♠` bucket here is a splinter — the call our `european.rs`
        // does not author.  Needs `1N-3C transfer to diamonds`=1 with
        // `1N-3C Puppet Stayman`=0, or the `3♣` filter accepts nothing.
        "nt-3c-3d" => (
            2,
            &[ONE_NT, PASS, THREE_C, PASS, THREE_D, PASS],
            Some(THREE_C),
            "BBA responder over 1NT - 3♣ - 3♦ - — a 3♥/3♠ bucket is the splinter",
        ),
        // The **unassuming cue-raise**: BBA's advancer over our 1-of-a-suit
        // opening and its partner's simple *two-level* overcall.  This is the
        // one node where `rubens_reading` records a floor on an **opponent**
        // (three-plus cards in partner's overcall, support band `10..`) with no
        // side gate, unlike the Rubens transfer beside it.  A `2X` bucket whose
        // hands fall outside that floor is a box that excludes the truth — a
        // wrong prior, not a loose one (see `probe-reading-sound`).  The cue
        // needs `level == 2`, i.e. the overcall suit *below* opener's, so these
        // three auctions are the whole shape of it.
        "ucb-sd" => (
            3,
            &[ONE_S, TWO_D, PASS],
            None,
            "BBA advancer over (1♠) 2♦ - — the 2♠ bucket is the unassuming cue-raise",
        ),
        "ucb-sc" => (
            3,
            &[ONE_S, TWO_C, PASS],
            None,
            "BBA advancer over (1♠) 2♣ - — the 2♠ bucket is the unassuming cue-raise",
        ),
        "ucb-dc" => (
            3,
            &[ONE_D, TWO_C, PASS],
            None,
            "BBA advancer over (1♦) 2♣ - — the 2♦ bucket is the unassuming cue-raise",
        ),
        // The one *major* the cue can ever be about: the overcall must rank
        // below opener's suit, so ♠ is never a two-level simple overcall and
        // `1♠ (2♥)` is the whole major case.  Support for a major is worth more
        // than support for a minor — and the cue over a minor is half a stopper
        // ask — so the two are not the same measurement.
        "ucb-sh" => (
            3,
            &[ONE_S, TWO_H, PASS],
            None,
            "BBA advancer over (1♠) 2♥ - — the 2♠ bucket is the unassuming cue-raise (major)",
        ),
        // The *one-level* overcall: `(1♣) 1♥` routes to the Rubens **transfer**
        // branch instead, where `2♦` (the transfer into partner's suit) is the
        // limit-plus raise and carries the same `len(Y) >= 3, points >= 10`
        // claim.  That reading is already our-side-only, so `--ours` is the arm
        // that matters: it asks whether our own floor makes the transfer with
        // hands the authored rule rejects.
        "rub-ch" => (
            3,
            &[ONE_C, ONE_H, PASS],
            None,
            "advancer over (1♣) 1♥ - — the 2♦ bucket is the transfer into partner's hearts",
        ),
        other => bail!(
            "--mode must be open|def1-c|def1-d|def1-h|def1-s|multi|advance|counter|muider-h|muider-s|rebid-d|rebid-h|rebid-s|stayman|xfer-h|xfer-s|weak2-d|weak2-h|weak2-s|def2-d|def2-h|def2-s|nt-resp|nt-3h|nt-3s|nt-3c|nt-3c-3d|nt-2s-3c|ucb-sd|ucb-sc|ucb-dc|ucb-sh|rub-ch|o4, got {other:?}"
        ),
    };

    // The weak-two modes probe the OPENER, so their self-consistency filter replays
    // an empty prefix (the opening itself), not (1NT); `trump` names the suit whose
    // top-honor count to report, since an Ogust ladder splits on suit quality.
    let (filter_prefix, trump): (&[c_int], Option<Suit>) = match args.mode.as_str() {
        "weak2-d" => (&[], Some(Suit::Diamonds)),
        "weak2-h" => (&[], Some(Suit::Hearts)),
        "weak2-s" => (&[], Some(Suit::Spades)),
        // The `def2-*` modes read the DEFENDER, so `trump` names *their* suit:
        // the top-honor column is then the stopper the 2NT overcall wants.
        "def2-d" => (&[], Some(Suit::Diamonds)),
        "def2-h" => (&[], Some(Suit::Hearts)),
        "def2-s" => (&[], Some(Suit::Spades)),
        // The `nt-3*` modes probe the OPENER, so their filter is "a hand BBA
        // opens 1NT" — an empty prefix, exactly like `weak2-*`.
        "nt-3h" => (&[], Some(Suit::Hearts)),
        "nt-3s" => (&[], Some(Suit::Spades)),
        "nt-3c" => (&[], Some(Suit::Diamonds)),
        // `nt-3c-3d` probes RESPONDER, so the filter replays the transfer
        // itself: keep only hands BBA bids `3♣` with over `1NT - `.  `trump`
        // is diamonds — the suit a splinter would be agreeing.
        "nt-3c-3d" => (&[ONE_NT, PASS], Some(Suit::Diamonds)),
        // Likewise for the club lane: keep only hands BBA bids `2♠` with, and
        // report top honours in clubs — the suit its splinters would agree.
        "nt-2s-3c" => (&[ONE_NT, PASS], Some(Suit::Clubs)),
        _ => (&[ONE_NT], None),
    };

    // Partner's overcall suit `Y` for the `ucb-*` modes — the suit
    // `rubens_reading`'s cue block asserts `len(Y) >= 3` and `SP(Y) >= 10` in.
    // Suit quality is not what the floor claims, so these modes take the support
    // columns instead of the honour histograms.
    let support = match args.mode.as_str() {
        "ucb-sd" => Some(Suit::Diamonds),
        "ucb-sc" | "ucb-dc" => Some(Suit::Clubs),
        "ucb-sh" | "rub-ch" => Some(Suit::Hearts),
        _ => None,
    };

    if args.ours && support.is_none() {
        bail!("--ours is only meaningful for the ucb-* modes");
    }
    let ours = args
        .ours
        .then(|| american(&pons::bidding::agreements::Agreements::default()).bind());

    // Direct one-suit probes match `bba-gen`'s stock 2/1 configuration.  The
    // Multi-Landy defaults exist only to make the historical 1NT modes
    // self-contained; they are irrelevant here and need not be forced.
    let overrides = if args.conv.is_empty() && args.mode.starts_with("def1-") {
        Vec::new()
    } else {
        parse_conv(&args.conv)?
    };
    let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
    let bba = Bba::load(&path, overrides)?;

    let mut report = String::new();
    let _ = writeln!(
        report,
        "# BBA Multi-Landy probe — mode `{}`\n\n{what}\nsystem: 0 (2/1 GF)  conventions: {}\nsamples per vul: {}\n",
        args.mode,
        conv_summary(&bba.overrides),
        args.samples,
    );
    let _ = writeln!(
        report,
        "Hands summarised in DSL vocabulary; `sketch` is a *candidate* to verify, not a proof.\n"
    );

    for token in args.vul.split(',').map(str::trim) {
        let vul = vul_code(token, actor)?;
        // Our bidder takes the absolute vulnerability; the actor sits West with
        // the dealer canonicalized to North, so "we" is E/W.
        let vul_abs = match token {
            "none" => AbsoluteVulnerability::NONE,
            "both" => AbsoluteVulnerability::ALL,
            "we" => AbsoluteVulnerability::EW,
            "they" => AbsoluteVulnerability::NS,
            other => bail!("--vul must be none|we|they|both, got {other:?}"),
        };
        let buckets = run(
            &bba,
            actor,
            prefix,
            filter,
            filter_prefix,
            trump,
            support,
            ours.as_ref().map(|partnership| (partnership, vul_abs)),
            vul,
            args.samples,
            args.seed,
        );
        render_vul(&mut report, token, &buckets, &args, support);
    }

    if let Some(out) = &args.out {
        std::fs::write(out, &report)?;
        eprintln!("wrote {out}");
    } else {
        print!("{report}");
    }
    Ok(())
}

/// Deal `samples` random hands, drive BBA, and bucket by its returned call.
// ponytail: a probe harness — a params struct would be more ceremony than the
// one call site is worth.  Bundle them if a third caller ever appears.
#[allow(clippy::too_many_arguments)]
fn run(
    bba: &Bba,
    actor: c_int,
    prefix: &[c_int],
    filter: Option<c_int>,
    filter_prefix: &[c_int],
    trump: Option<Suit>,
    support: Option<Suit>,
    ours: Option<(&Partnership, AbsoluteVulnerability)>,
    vul: c_int,
    samples: usize,
    seed: u64,
) -> BTreeMap<c_int, Bucket> {
    let mut rng = StdRng::seed_from_u64(seed);
    let empty = Builder::new()
        .build_partial()
        .expect("an empty builder is a valid (all-unknown) partial deal");

    let mut buckets: BTreeMap<c_int, Bucket> = BTreeMap::new();
    for deal in fill_deals(&mut rng, empty).take(samples) {
        let hand = deal[Seat::North];
        // ponytail: rebid modes read the bidder's OWN seat, so a uniform random hand
        // is wrong — most never make the call.  Keep only hands whose call over
        // `filter_prefix` is the studied one; otherwise the rebid is from a hand that
        // never bid it.  (`filter_prefix` is (1NT) for the overcall modes and empty
        // for the weak-two modes, which probe the opener.)  Rejection is exact here.
        if let Some(want) = filter
            && bba.call(actor, filter_prefix, hand, vul) != want
        {
            continue;
        }
        let code = match ours {
            Some((partnership, vul_abs)) => {
                call_to_code(our_call(partnership, prefix, hand, vul_abs))
            }
            None => bba.call(actor, prefix, hand, vul),
        };
        if decode(code).is_none() {
            continue; // EPBot error/illegal code — drop it
        }
        let entry = buckets.entry(code).or_default();
        entry.hcp.push(hcp(hand));
        let lengths = Suit::ASC.map(|suit| hand[suit].len() as u8);
        for (slot, &l) in entry.len.iter_mut().zip(&lengths) {
            slot.push(l);
        }
        if is_balanced(lengths) {
            entry.balanced += 1;
        }
        if let Some(suit) = trump {
            let holding = hand[suit];
            let count =
                |ranks: &[Rank]| ranks.iter().filter(|&&r| holding.contains(r)).count() as u8;
            entry.tops.push(count(&[Rank::A, Rank::K, Rank::Q]));
            entry
                .tops4
                .push(count(&[Rank::A, Rank::K, Rank::Q, Rank::J]));
            entry
                .tops5
                .push(count(&[Rank::A, Rank::K, Rank::Q, Rank::J, Rank::T]));
            let thcp = [(Rank::A, 4), (Rank::K, 3), (Rank::Q, 2), (Rank::J, 1)]
                .into_iter()
                .filter(|&(r, _)| holding.contains(r))
                .map(|(_, v)| v)
                .sum();
            entry.trump_hcp.push(thcp);
        }
        if let Some(suit) = support {
            entry.sp.push(support_point_count_in(hand, suit));
            entry.points.push(point_count(hand));
        }
    }
    buckets
}

/// One natural two-level overcall that is cheapest after a one-suit opening.
/// There is no O4 candidate over 1♣: every unbid suit is still available at
/// the one level, while (1♣) 2♦ belongs to the separate weak-jump item O3.
#[derive(Clone, Copy)]
struct O4Pair {
    opening: Suit,
    opening_code: c_int,
    candidate: Suit,
    candidate_code: c_int,
}

const O4_PAIRS: [O4Pair; 6] = [
    O4Pair {
        opening: Suit::Diamonds,
        opening_code: ONE_D,
        candidate: Suit::Clubs,
        candidate_code: TWO_C,
    },
    O4Pair {
        opening: Suit::Hearts,
        opening_code: ONE_H,
        candidate: Suit::Clubs,
        candidate_code: TWO_C,
    },
    O4Pair {
        opening: Suit::Hearts,
        opening_code: ONE_H,
        candidate: Suit::Diamonds,
        candidate_code: TWO_D,
    },
    O4Pair {
        opening: Suit::Spades,
        opening_code: ONE_S,
        candidate: Suit::Clubs,
        candidate_code: TWO_C,
    },
    O4Pair {
        opening: Suit::Spades,
        opening_code: ONE_S,
        candidate: Suit::Diamonds,
        candidate_code: TWO_D,
    },
    O4Pair {
        opening: Suit::Spades,
        opening_code: ONE_S,
        candidate: Suit::Hearts,
        candidate_code: TWO_H,
    },
];

const O4_HONORS: [(Rank, u8); 5] = [
    (Rank::A, 4),
    (Rank::K, 3),
    (Rank::Q, 2),
    (Rank::J, 1),
    (Rank::T, 0),
];

#[derive(Clone, Copy)]
struct O4Row {
    points: u8,
    suit_hcp: u8,
    len: u8,
    mask: u8,
    vulnerable: bool,
    pair: usize,
    vul: usize,
    bid: bool,
}

#[derive(Default)]
struct O4Sample {
    rows: Vec<O4Row>,
    calls: usize,
    alternatives: BTreeMap<c_int, usize>,
}

impl O4Sample {
    fn observe(
        &mut self,
        bba: &Bba,
        pair_index: usize,
        vul_index: usize,
        vul: c_int,
        vulnerable: bool,
        hand: Hand,
    ) {
        let pair = O4_PAIRS[pair_index];
        let code = bba.call(1, &[pair.opening_code], hand, vul);
        self.calls += 1;
        let bid = if code == pair.candidate_code {
            true
        } else if code == PASS {
            false
        } else {
            *self.alternatives.entry(code).or_default() += 1;
            return;
        };
        self.rows.push(O4Row {
            points: point_count(hand),
            suit_hcp: holding_hcp(hand[pair.candidate]),
            len: hand[pair.candidate].len() as u8,
            mask: honor_mask(hand[pair.candidate]),
            vulnerable,
            pair: pair_index,
            vul: vul_index,
            bid,
        });
    }
}

/// All holdings of up to four cards, indexed by `[length][HCP]`, for exact
/// construction of the three side suits in the controlled O4 grid.
struct HoldingPools(Vec<Vec<Vec<Holding>>>);

impl HoldingPools {
    fn new() -> Self {
        let mut pools = vec![vec![Vec::new(); 11]; 5];
        for bits in 0_u16..(1 << 13) {
            let holding = Holding::from_bits_retain(bits << 2);
            let len = holding.len();
            if len <= 4 {
                pools[len][holding_hcp(holding) as usize].push(holding);
            }
        }
        Self(pools)
    }

    fn get(&self, len: u8, hcp: u8) -> &[Holding] {
        &self.0[len as usize][hcp as usize]
    }
}

#[derive(Clone, Copy, Debug)]
enum O4ModelKind {
    SuitHcp,
    SuitHcpLength,
    HonorMaskLength,
}

impl O4ModelKind {
    const ALL: [Self; 3] = [Self::SuitHcp, Self::SuitHcpLength, Self::HonorMaskLength];

    const fn name(self) -> &'static str {
        match self {
            Self::SuitHcp => "raw suit HCP",
            Self::SuitHcpLength => "suit HCP + length",
            Self::HonorMaskLength => "AKQJT mask + length",
        }
    }

    const fn keys(self) -> usize {
        match self {
            Self::SuitHcp => 2 * 11,
            Self::SuitHcpLength => 2 * 2 * 11,
            Self::HonorMaskLength => 2 * 2 * 32,
        }
    }

    const fn key(self, row: O4Row) -> usize {
        let vul = row.vulnerable as usize;
        let len = (row.len - 5) as usize;
        match self {
            Self::SuitHcp => vul * 11 + row.suit_hcp as usize,
            Self::SuitHcpLength => (vul * 2 + len) * 11 + row.suit_hcp as usize,
            Self::HonorMaskLength => (vul * 2 + len) * 32 + row.mask as usize,
        }
    }
}

#[derive(Clone)]
struct O4Model {
    kind: O4ModelKind,
    thresholds: Vec<u8>,
}

impl O4Model {
    fn predicts(&self, row: O4Row) -> bool {
        row.points >= self.thresholds[self.kind.key(row)]
    }
}

#[derive(Clone, Copy, Default)]
struct Confusion {
    tp: usize,
    tn: usize,
    fp: usize,
    fn_: usize,
}

impl Confusion {
    fn push(&mut self, truth: bool, predicted: bool) {
        match (truth, predicted) {
            (true, true) => self.tp += 1,
            (false, false) => self.tn += 1,
            (false, true) => self.fp += 1,
            (true, false) => self.fn_ += 1,
        }
    }

    fn balanced_accuracy(self) -> f64 {
        let positive = self.tp + self.fn_;
        let negative = self.tn + self.fp;
        if positive == 0 || negative == 0 {
            return f64::NAN;
        }
        0.5 * (self.tp as f64 / positive as f64 + self.tn as f64 / negative as f64)
    }
}

/// Run the O4 model-selection probe.  X, 1NT, and calls in another strain do
/// not expose whether BBA's hidden natural-overcall gate accepted the hand, so
/// the fit is deliberately conditional on the observable candidate-vs-Pass
/// boundary and reports that coverage next to the score.
fn run_o4(args: &Args) -> Result<()> {
    if args.ours {
        bail!("--ours is not meaningful for --mode o4");
    }
    if args.o4_replicates == 0 || args.o4_holdout == 0 {
        bail!("--o4-replicates and --o4-holdout must be positive");
    }
    let (hcp_lo, hcp_hi) = parse_o4_hcp(&args.o4_hcp)?;
    let vuls: Vec<(&str, c_int, bool)> = args
        .vul
        .split(',')
        .map(str::trim)
        .map(|token| Ok((token, vul_code(token, 1)?, matches!(token, "we" | "both"))))
        .collect::<Result<_>>()?;
    if vuls.is_empty() {
        bail!("--vul must contain at least one vulnerability");
    }

    let overrides = if args.conv.is_empty() {
        Vec::new()
    } else {
        parse_conv(&args.conv)?
    };
    let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
    let bba = Bba::load(&path, overrides)?;
    let pools = HoldingPools::new();
    let mut rng = StdRng::seed_from_u64(args.seed);
    let mut controlled = O4Sample::default();

    for (pair, pair_spec) in O4_PAIRS.iter().enumerate() {
        for len in [5_u8, 6] {
            for mask in 0_u8..32 {
                let suit_hcp = mask_hcp(mask);
                for total_hcp in hcp_lo.max(suit_hcp)..=hcp_hi {
                    for _ in 0..args.o4_replicates {
                        let hand = controlled_o4_hand(
                            &mut rng,
                            &pools,
                            pair_spec.candidate,
                            len,
                            mask,
                            total_hcp,
                        )
                        .expect("every feasible O4 HCP grid cell has a side-suit holding");
                        assert_eq!(hcp(hand), total_hcp);
                        assert_eq!(hand[pair_spec.candidate].len(), len as usize);
                        assert_eq!(honor_mask(hand[pair_spec.candidate]), mask);
                        for (vul_index, &(_, vul, vulnerable)) in vuls.iter().enumerate() {
                            controlled.observe(&bba, pair, vul_index, vul, vulnerable, hand);
                        }
                    }
                }
            }
        }
    }

    // A separate stream keeps model selection reproducible while ensuring the
    // random validation hands were never used to fit a threshold.
    let mut holdout_rng = StdRng::seed_from_u64(args.seed ^ 0x0f04_5eed_d15c_01a7);
    let mut holdout = O4Sample::default();
    for (pair, pair_spec) in O4_PAIRS.iter().enumerate() {
        let hands = random_o4_hands(
            &mut holdout_rng,
            pair_spec.candidate,
            hcp_lo,
            hcp_hi,
            args.o4_holdout,
        );
        for hand in hands {
            for (vul_index, &(_, vul, vulnerable)) in vuls.iter().enumerate() {
                holdout.observe(&bba, pair, vul_index, vul, vulnerable, hand);
            }
        }
    }

    if controlled.rows.iter().all(|row| row.bid)
        || controlled.rows.iter().all(|row| !row.bid)
        || holdout.rows.iter().all(|row| row.bid)
        || holdout.rows.iter().all(|row| !row.bid)
    {
        bail!("O4 candidate-vs-Pass sample needs both classes in train and holdout");
    }

    let models: Vec<O4Model> = O4ModelKind::ALL
        .into_iter()
        .map(|kind| fit_o4_model(kind, &controlled.rows))
        .collect();
    let heldout_scores: Vec<Confusion> = models
        .iter()
        .map(|model| score_o4(model, &holdout.rows))
        .collect();
    let best = heldout_scores
        .iter()
        .map(|score| score.balanced_accuracy())
        .fold(f64::NEG_INFINITY, f64::max);
    let selected = models.iter().zip(&heldout_scores).find(|(model, score)| {
        score.balanced_accuracy() >= 0.95
            && score.balanced_accuracy() + 0.005 >= best
            && o4_monotonicity_violations(model) == 0
    });

    let mut report = String::new();
    let _ = writeln!(
        report,
        "# O4 live-BBA two-level-overcall model selection\n\n\
         system: 0 (2/1 GF), conventions: {}\n\
         seed: {}  controlled HCP: {hcp_lo}–{hcp_hi}  replicates: {}  \
         held-out hands/pair: {}\n\
         vulnerabilities: {}\n",
        conv_summary(&bba.overrides),
        args.seed,
        args.o4_replicates,
        args.o4_holdout,
        vuls.iter()
            .map(|(name, _, _)| *name)
            .collect::<Vec<_>>()
            .join(","),
    );
    let pairs = O4_PAIRS
        .iter()
        .map(|pair| format!("(1{}) 2{}", pair.opening, pair.candidate))
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(report, "candidate pairs: {pairs}");
    let _ = writeln!(
        report,
        "label: exact candidate natural overcall vs Pass. X, 1NT, and another \
         strain are excluded because those higher-precedence routes hide whether \
         the natural-overcall gate accepted; coverage is reported so the conditional \
         accuracy cannot be mistaken for whole-node accuracy.\n"
    );
    render_o4_sample(&mut report, "controlled fit", &controlled);
    render_o4_sample(&mut report, "held-out random", &holdout);

    let _ = writeln!(
        report,
        "## Model comparison\n\n| model | fit BA | held-out BA | sensitivity | specificity | worst pair/vul BA | monotonicity |\n| --- | ---: | ---: | ---: | ---: | ---: | ---: |"
    );
    for (model, heldout_score) in models.iter().zip(&heldout_scores) {
        let fit = score_o4(model, &controlled.rows);
        let positive = heldout_score.tp + heldout_score.fn_;
        let negative = heldout_score.tn + heldout_score.fp;
        let sensitivity = heldout_score.tp as f64 / positive as f64;
        let specificity = heldout_score.tn as f64 / negative as f64;
        let worst = worst_o4_cell(model, &holdout.rows, O4_PAIRS.len(), vuls.len());
        let _ = writeln!(
            report,
            "| {} | {:.2}% | {:.2}% | {:.2}% | {:.2}% | {:.2}% | {} |",
            model.kind.name(),
            100.0 * fit.balanced_accuracy(),
            100.0 * heldout_score.balanced_accuracy(),
            100.0 * sensitivity,
            100.0 * specificity,
            100.0 * worst,
            o4_monotonicity_violations(model),
        );
    }

    match selected {
        Some((model, score)) => {
            let _ = writeln!(
                report,
                "\n**QUALIFIES: `{}`** — {:.2}% held-out balanced accuracy; \
                 simplest model within 0.5 percentage points of the best ({:.2}%), \
                 with zero monotonicity violations.\n",
                model.kind.name(),
                100.0 * score.balanced_accuracy(),
                100.0 * best,
            );
            render_o4_thresholds(&mut report, model);
        }
        None => {
            let _ = writeln!(
                report,
                "\n**NO MODEL QUALIFIES** — best held-out balanced accuracy {:.2}%; \
                 the gate requires at least 95%, within 0.5 percentage points \
                 of best, and zero monotonicity violations.",
                100.0 * best,
            );
        }
    }

    if let Some(out) = &args.out {
        std::fs::write(out, &report)?;
        eprintln!("wrote {out}");
    } else {
        print!("{report}");
    }
    Ok(())
}

fn parse_o4_hcp(text: &str) -> Result<(u8, u8)> {
    let (lo, hi) = text
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("--o4-hcp must be LO:HI, got {text:?}"))?;
    let (lo, hi): (u8, u8) = (lo.parse()?, hi.parse()?);
    if lo > hi || hi > 37 {
        bail!("--o4-hcp must satisfy 0 <= LO <= HI <= 37");
    }
    Ok((lo, hi))
}

fn controlled_o4_hand(
    rng: &mut StdRng,
    pools: &HoldingPools,
    suit: Suit,
    len: u8,
    mask: u8,
    total_hcp: u8,
) -> Option<Hand> {
    let target = target_holding(rng, len, mask);
    let outside_hcp = total_hcp.checked_sub(holding_hcp(target))?;
    let mut side_lengths = if len == 5 { [4, 2, 2] } else { [3, 2, 2] };
    side_lengths.shuffle(rng);
    let side_suits: Vec<Suit> = Suit::ASC.into_iter().filter(|&s| s != suit).collect();

    let mut partitions = Vec::new();
    for a in 0..=10_u8 {
        for b in 0..=10_u8 {
            let Some(c) = outside_hcp.checked_sub(a + b) else {
                continue;
            };
            if c <= 10
                && !pools.get(side_lengths[0], a).is_empty()
                && !pools.get(side_lengths[1], b).is_empty()
                && !pools.get(side_lengths[2], c).is_empty()
            {
                partitions.push([a, b, c]);
            }
        }
    }
    let partition = *partitions.choose(rng)?;
    let mut hand = Hand::EMPTY;
    hand[suit] = target;
    for i in 0..3 {
        hand[side_suits[i]] = *pools
            .get(side_lengths[i], partition[i])
            .choose(rng)
            .unwrap_or_else(|| unreachable!());
    }
    Some(hand)
}

fn target_holding(rng: &mut StdRng, len: u8, mask: u8) -> Holding {
    let mut holding = Holding::EMPTY;
    for (index, &(rank, _)) in O4_HONORS.iter().enumerate() {
        if mask & (1 << index) != 0 {
            holding.insert(rank);
        }
    }
    let mut spots: Vec<Rank> = (2..=9).map(Rank::new).collect();
    spots.shuffle(rng);
    for rank in spots.into_iter().take(len as usize - holding.len()) {
        holding.insert(rank);
    }
    holding
}

fn random_o4_hands(
    rng: &mut StdRng,
    suit: Suit,
    hcp_lo: u8,
    hcp_hi: u8,
    count: usize,
) -> Vec<Hand> {
    let empty = Builder::new()
        .build_partial()
        .expect("an empty builder is a valid partial deal");
    fill_deals(rng, empty)
        .map(|deal| deal[Seat::North])
        .filter(|&hand| {
            let target = hand[suit].len();
            (target == 5 || target == 6)
                && Suit::ASC
                    .into_iter()
                    .filter(|&other| other != suit)
                    .all(|other| hand[other].len() <= 4)
                && !is_balanced(Suit::ASC.map(|s| hand[s].len() as u8))
                && (hcp_lo..=hcp_hi).contains(&hcp(hand))
        })
        .take(count)
        .collect()
}

fn holding_hcp(holding: Holding) -> u8 {
    O4_HONORS
        .iter()
        .filter(|&&(rank, _)| holding.contains(rank))
        .map(|&(_, value)| value)
        .sum()
}

fn honor_mask(holding: Holding) -> u8 {
    O4_HONORS
        .iter()
        .enumerate()
        .fold(0, |mask, (i, &(rank, _))| {
            mask | (u8::from(holding.contains(rank)) << i)
        })
}

fn mask_hcp(mask: u8) -> u8 {
    O4_HONORS
        .iter()
        .enumerate()
        .filter(|&(i, _)| mask & (1 << i) != 0)
        .map(|(_, &(_, value))| value)
        .sum()
}

fn fit_o4_model(kind: O4ModelKind, rows: &[O4Row]) -> O4Model {
    let positive = rows.iter().filter(|row| row.bid).count();
    let negative = rows.len() - positive;
    let points_lo = rows.iter().map(|row| row.points).min().unwrap_or(0);
    let points_hi = rows.iter().map(|row| row.points).max().unwrap_or(37);
    let mut buckets = vec![Vec::new(); kind.keys()];
    for &row in rows {
        buckets[kind.key(row)].push(row);
    }
    let mut thresholds = vec![points_hi.saturating_add(1); kind.keys()];
    for (key, bucket) in buckets.iter().enumerate() {
        let mut best = f64::NEG_INFINITY;
        for threshold in points_lo..=points_hi.saturating_add(1) {
            let utility: f64 = bucket
                .iter()
                .map(|row| match (row.bid, row.points >= threshold) {
                    (true, true) => 1.0 / positive as f64,
                    (false, false) => 1.0 / negative as f64,
                    _ => 0.0,
                })
                .sum();
            if utility > best || (utility == best && threshold > thresholds[key]) {
                best = utility;
                thresholds[key] = threshold;
            }
        }
    }

    let raw = O4Model { kind, thresholds };
    let mut candidates = Vec::new();
    if o4_monotonicity_violations(&raw) == 0 {
        candidates.push(raw.clone());
    }
    candidates.push(close_o4_model(raw.clone(), true));
    candidates.push(close_o4_model(raw, false));
    candidates
        .into_iter()
        .max_by(|a, b| {
            score_o4(a, rows)
                .balanced_accuracy()
                .total_cmp(&score_o4(b, rows).balanced_accuracy())
        })
        .unwrap_or_else(|| unreachable!())
}

/// Monotone closure in either conservative (raise the weaker threshold) or
/// permissive (lower the stronger threshold) direction.  Vulnerability follows
/// the same bridge order: vulnerable may demand more, never less.
fn close_o4_model(mut model: O4Model, conservative: bool) -> O4Model {
    let features = o4_features(model.kind);
    loop {
        let mut changed = false;
        for (weak_key, &weak) in features.iter().enumerate() {
            for (strong_key, &strong) in features.iter().enumerate() {
                if weak.vulnerable == strong.vulnerable
                    && o4_dominates(model.kind, strong, weak)
                    && model.thresholds[strong_key] > model.thresholds[weak_key]
                {
                    if conservative {
                        model.thresholds[weak_key] = model.thresholds[strong_key];
                    } else {
                        model.thresholds[strong_key] = model.thresholds[weak_key];
                    }
                    changed = true;
                }
                if !weak.vulnerable
                    && strong.vulnerable
                    && o4_same_quality(model.kind, strong, weak)
                    && model.thresholds[strong_key] < model.thresholds[weak_key]
                {
                    if conservative {
                        model.thresholds[strong_key] = model.thresholds[weak_key];
                    } else {
                        model.thresholds[weak_key] = model.thresholds[strong_key];
                    }
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    model
}

#[derive(Clone, Copy)]
struct O4Feature {
    vulnerable: bool,
    len: u8,
    suit_hcp: u8,
    mask: u8,
}

fn o4_features(kind: O4ModelKind) -> Vec<O4Feature> {
    let mut features = Vec::with_capacity(kind.keys());
    for vulnerable in [false, true] {
        match kind {
            O4ModelKind::SuitHcp => {
                for suit_hcp in 0..=10 {
                    features.push(O4Feature {
                        vulnerable,
                        len: 5,
                        suit_hcp,
                        mask: 0,
                    });
                }
            }
            O4ModelKind::SuitHcpLength => {
                for len in [5, 6] {
                    for suit_hcp in 0..=10 {
                        features.push(O4Feature {
                            vulnerable,
                            len,
                            suit_hcp,
                            mask: 0,
                        });
                    }
                }
            }
            O4ModelKind::HonorMaskLength => {
                for len in [5, 6] {
                    for mask in 0..32 {
                        features.push(O4Feature {
                            vulnerable,
                            len,
                            suit_hcp: mask_hcp(mask),
                            mask,
                        });
                    }
                }
            }
        }
    }
    features
}

fn o4_dominates(kind: O4ModelKind, strong: O4Feature, weak: O4Feature) -> bool {
    match kind {
        O4ModelKind::SuitHcp => strong.suit_hcp >= weak.suit_hcp,
        O4ModelKind::SuitHcpLength => strong.suit_hcp >= weak.suit_hcp && strong.len >= weak.len,
        O4ModelKind::HonorMaskLength => {
            strong.len >= weak.len && mask_dominates(strong.mask, weak.mask)
        }
    }
}

fn mask_dominates(strong: u8, weak: u8) -> bool {
    let mut strong_seen = 0;
    let mut weak_seen = 0;
    for i in 0..5 {
        strong_seen += usize::from(strong & (1 << i) != 0);
        weak_seen += usize::from(weak & (1 << i) != 0);
        if strong_seen < weak_seen {
            return false;
        }
    }
    true
}

fn o4_same_quality(kind: O4ModelKind, a: O4Feature, b: O4Feature) -> bool {
    match kind {
        O4ModelKind::SuitHcp => a.suit_hcp == b.suit_hcp,
        O4ModelKind::SuitHcpLength => a.suit_hcp == b.suit_hcp && a.len == b.len,
        O4ModelKind::HonorMaskLength => a.mask == b.mask && a.len == b.len,
    }
}

fn o4_monotonicity_violations(model: &O4Model) -> usize {
    let features = o4_features(model.kind);
    let mut violations = 0;
    for (weak_key, &weak) in features.iter().enumerate() {
        for (strong_key, &strong) in features.iter().enumerate() {
            if weak.vulnerable == strong.vulnerable
                && o4_dominates(model.kind, strong, weak)
                && model.thresholds[strong_key] > model.thresholds[weak_key]
            {
                violations += 1;
            }
            if !weak.vulnerable
                && strong.vulnerable
                && o4_same_quality(model.kind, strong, weak)
                && model.thresholds[strong_key] < model.thresholds[weak_key]
            {
                violations += 1;
            }
        }
    }
    violations
}

fn score_o4(model: &O4Model, rows: &[O4Row]) -> Confusion {
    let mut score = Confusion::default();
    for &row in rows {
        score.push(row.bid, model.predicts(row));
    }
    score
}

fn worst_o4_cell(model: &O4Model, rows: &[O4Row], pairs: usize, vuls: usize) -> f64 {
    let mut worst: f64 = 1.0;
    for pair in 0..pairs {
        for vul in 0..vuls {
            let cell: Vec<O4Row> = rows
                .iter()
                .copied()
                .filter(|row| row.pair == pair && row.vul == vul)
                .collect();
            let score = score_o4(model, &cell).balanced_accuracy();
            if score.is_finite() {
                worst = worst.min(score);
            }
        }
    }
    worst
}

fn render_o4_sample(report: &mut String, name: &str, sample: &O4Sample) {
    let positives = sample.rows.iter().filter(|row| row.bid).count();
    let negatives = sample.rows.len() - positives;
    let coverage = sample.rows.len() as f64 / sample.calls as f64;
    let alternatives = sample
        .alternatives
        .iter()
        .map(|(&code, &n)| format!("{}:{n}", decode(code).unwrap_or_else(|| code.to_string())))
        .collect::<Vec<_>>()
        .join(" ");
    let _ = writeln!(
        report,
        "- {name}: {} calls; {} labeled ({:.1}% coverage; bid {positives}, Pass {negatives}); alternate routes: {}",
        sample.calls,
        sample.rows.len(),
        100.0 * coverage,
        if alternatives.is_empty() {
            "none"
        } else {
            &alternatives
        },
    );
}

fn render_o4_thresholds(report: &mut String, model: &O4Model) {
    let _ = writeln!(
        report,
        "## Selected thresholds\n\nEach cell is the minimum whole-hand `points` for the natural overcall; a value above the probed point range means no bid."
    );
    let features = o4_features(model.kind);
    for vulnerable in [false, true] {
        let _ = writeln!(
            report,
            "\n- {}:",
            if vulnerable {
                "vulnerable"
            } else {
                "not vulnerable"
            }
        );
        match model.kind {
            O4ModelKind::SuitHcp => {
                let cells = features
                    .iter()
                    .enumerate()
                    .filter(|(_, feature)| feature.vulnerable == vulnerable)
                    .map(|(key, feature)| format!("{}:{}", feature.suit_hcp, model.thresholds[key]))
                    .collect::<Vec<_>>()
                    .join(" ");
                let _ = writeln!(report, "  - suit-HCP:min-points {cells}");
            }
            O4ModelKind::SuitHcpLength => {
                for len in [5, 6] {
                    let cells = features
                        .iter()
                        .enumerate()
                        .filter(|(_, feature)| {
                            feature.vulnerable == vulnerable && feature.len == len
                        })
                        .map(|(key, feature)| {
                            format!("{}:{}", feature.suit_hcp, model.thresholds[key])
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    let _ = writeln!(report, "  - {len} cards, suit-HCP:min-points {cells}");
                }
            }
            O4ModelKind::HonorMaskLength => {
                for len in [5, 6] {
                    let cells = features
                        .iter()
                        .enumerate()
                        .filter(|(_, feature)| {
                            feature.vulnerable == vulnerable && feature.len == len
                        })
                        .map(|(key, feature)| {
                            format!("{}:{}", mask_label(feature.mask), model.thresholds[key])
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    let _ = writeln!(report, "  - {len} cards, mask:min-points {cells}");
                }
            }
        }
    }
}

fn mask_label(mask: u8) -> String {
    let label: String = O4_HONORS
        .iter()
        .enumerate()
        .filter_map(|(i, &(rank, _))| (mask & (1 << i) != 0).then_some(rank.letter()))
        .collect();
    if label.is_empty() { "-".into() } else { label }
}

/// One report section per vulnerability: the per-call buckets in DSL vocabulary.
fn render_vul(
    report: &mut String,
    vul: &str,
    buckets: &BTreeMap<c_int, Bucket>,
    args: &Args,
    support: Option<Suit>,
) {
    let probed: usize = buckets.values().map(|b| b.hcp.len()).sum();
    let _ = writeln!(report, "## vul: {vul}   (n={probed})\n");
    if probed == 0 {
        let _ = writeln!(report, "_no probe hands produced a legal call_\n");
        return;
    }

    let mut by_share: Vec<(&c_int, &Bucket)> = buckets.iter().collect();
    by_share.sort_by_key(|(_, b)| std::cmp::Reverse(b.hcp.len()));

    for (code, bucket) in by_share {
        let n = bucket.hcp.len();
        let share = n as f64 / probed as f64;
        if share < args.min_share {
            continue;
        }
        let call = decode(*code).expect("error codes were dropped in `run`");

        let mut hcp = bucket.hcp.clone();
        hcp.sort_unstable();
        let hcp_lo = pct(&hcp, args.trim);
        let hcp_hi = pct(&hcp, 1.0 - args.trim);

        let _ = writeln!(report, "### {call}   (chosen {:.1}%, n={n})", 100.0 * share);
        let _ = writeln!(
            report,
            "- hcp: {hcp_lo}–{hcp_hi} (median {})",
            pct(&hcp, 0.5)
        );
        if !bucket.tops.is_empty() {
            let good = bucket.tops.iter().filter(|&&t| t >= 2).count();
            // Sum in u32: 3 honors × a few hundred hands overflows u8, and release
            // builds wrap silently (a 2.02 mean printed as 0.01).
            let total: u32 = bucket.tops.iter().copied().map(u32::from).sum();
            let mean = f64::from(total) / bucket.tops.len() as f64;
            let _ = writeln!(
                report,
                "- trump A/K/Q: mean {mean:.2}, two-plus {:.0}% (Ogust \"good suit\")",
                100.0 * good as f64 / bucket.tops.len() as f64
            );
            // Histograms of rival "good suit" predicates.  A predicate BBA actually
            // uses separates the good/bad rungs with NO overlap, so read these as
            // "which row has an empty cell exactly where the other rung is full".
            for (label, col) in [
                ("A/K/Q  ", &bucket.tops),
                ("A/K/Q/J", &bucket.tops4),
                ("+ten   ", &bucket.tops5),
                ("trumpHC", &bucket.trump_hcp),
            ] {
                let hi = col.iter().copied().max().unwrap_or(0);
                let cells: Vec<String> = (0..=hi)
                    .map(|v| format!("{v}:{}", col.iter().filter(|&&x| x == v).count()))
                    .collect();
                let _ = writeln!(report, "  - {label} {}", cells.join(" "));
            }
        }

        // What `rubens_reading`'s cue block would assert about this hand if the
        // call were the cue.  Columns are pushed in lockstep, so index `k` is one
        // hand across all of them; the violation rate is this block's
        // box-excludes-the-truth rate, to be read against the ambient
        // opponent-reading rate of docs/deviation-panel.md (LHO/RHO 8.2/8.3%,
        // partner 3.3%).
        if let Some(y) = support
            && !bucket.sp.is_empty()
        {
            let len_y = &bucket.len[y as usize];
            let mut sp = bucket.sp.clone();
            sp.sort_unstable();
            let _ = writeln!(
                report,
                "- support points in {y:?}: {}–{} (median {})",
                pct(&sp, 0.10),
                pct(&sp, 0.90),
                pct(&sp, 0.5)
            );
            let short = (0..n).filter(|&k| len_y[k] < 3).count();
            let weak = (0..n).filter(|&k| bucket.sp[k] < 10).count();
            // The legacy-axis image the block also publishes: `support_band_to_points`
            // maps the `10..` band to `5..` on the plain `points` scale.
            let thin = (0..n).filter(|&k| bucket.points[k] < 5).count();
            let bad = (0..n)
                .filter(|&k| len_y[k] < 3 || bucket.sp[k] < 10)
                .count();
            // Where the band would have to sit to be sound: the floor is the one
            // free parameter, so price the candidates rather than guess.
            let band: Vec<String> = [8, 9, 10, 11]
                .into_iter()
                .map(|floor| {
                    let miss = bucket.sp.iter().filter(|&&v| v < floor).count();
                    format!("{floor}:{:.1}%", 100.0 * miss as f64 / n as f64)
                })
                .collect();
            let share = |c: usize| 100.0 * c as f64 / n as f64;
            let _ = writeln!(
                report,
                "- **cue floor `len({y:?}) >= 3 & SP({y:?}) >= 10` violated {:.1}%** \
                 (short {:.1}%, weak {:.1}%; legacy `points >= 5` violated {:.1}%)",
                share(bad),
                share(short),
                share(weak),
                share(thin)
            );
            let _ = writeln!(report, "  - SP band sweep (violated): {}", band.join("  "));
        }

        let mut clauses = vec![format!("hcp({hcp_lo}..={hcp_hi})")];
        for (i, suit) in Suit::ASC.into_iter().enumerate() {
            let mut col = bucket.len[i].clone();
            col.sort_unstable();
            let (lo, mid, hi) = (pct(&col, 0.10), pct(&col, 0.5), pct(&col, 0.90));
            let _ = writeln!(report, "- {suit:?}: {lo}–{hi} (median {mid})");
            if lo >= 4 {
                clauses.push(format!("len({suit:?}, {lo}..)"));
            }
        }

        // Per-hand longest major / minor (columns are pushed in lockstep, so index
        // `k` is the same hand across all four).  These read the "6+ major" (Multi)
        // and "5-4 majors" answers directly, where the per-suit columns smear.
        let mut major: Vec<u8> = (0..n)
            .map(|k| bucket.len[2][k].max(bucket.len[3][k]))
            .collect();
        let mut minor: Vec<u8> = (0..n)
            .map(|k| bucket.len[0][k].max(bucket.len[1][k]))
            .collect();
        major.sort_unstable();
        minor.sort_unstable();
        let _ = writeln!(
            report,
            "- longest major: {}–{} (median {})",
            pct(&major, 0.10),
            pct(&major, 0.90),
            pct(&major, 0.5)
        );
        let _ = writeln!(
            report,
            "- longest minor: {}–{} (median {})",
            pct(&minor, 0.10),
            pct(&minor, 0.90),
            pct(&minor, 0.5)
        );

        let bal = bucket.balanced as f64 / n as f64;
        let _ = writeln!(report, "- balanced: {:.0}%", 100.0 * bal);
        if bal > 0.8 {
            clauses.push("balanced()".to_string());
        }
        let _ = writeln!(report, "- sketch: {}\n", clauses.join(" & "));
    }

    // Cheap invariant: HCP can never escape its range if the eval wiring is sound.
    assert!(
        buckets.values().flat_map(|b| &b.hcp).all(|&h| h <= 37),
        "HCP out of range — eval wiring is wrong"
    );
}

/// HCP via the simple evaluator (matches the DSL's `hcp(..)`).
fn hcp(hand: Hand) -> u8 {
    SimpleEvaluator(eval::hcp::<u8>).eval(hand)
}

/// Balanced: every suit ≥ 2 and at most one doubleton.
fn is_balanced(len: [u8; 4]) -> bool {
    len.iter().all(|&l| l >= 2) && len.iter().filter(|&&l| l == 2).count() <= 1
}

/// Value at quantile `q` of a pre-sorted slice (nearest-rank).
fn pct(sorted: &[u8], q: f64) -> u8 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() - 1) as f64 * q).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Parse `--conv NAME=0|1` flags; default forces Multi-Landy on, Cappelletti off.
fn parse_conv(flags: &[String]) -> Result<Vec<(CString, c_int)>> {
    if flags.is_empty() {
        return Ok(vec![
            (CString::new("Multi-Landy").unwrap(), 1),
            (CString::new("Cappelletti").unwrap(), 0),
        ]);
    }
    flags
        .iter()
        .map(|flag| {
            let (name, value) = flag
                .split_once('=')
                .ok_or_else(|| anyhow::anyhow!("--conv must be NAME=0|1, got {flag:?}"))?;
            let on: c_int = value.trim().parse()?;
            Ok((CString::new(name.trim())?, on))
        })
        .collect()
}

fn conv_summary(overrides: &[(CString, c_int)]) -> String {
    if overrides.is_empty() {
        return "none".into();
    }
    overrides
        .iter()
        .map(|(name, value)| format!("{}={value}", name.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(", ")
}
