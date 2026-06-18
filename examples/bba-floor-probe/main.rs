//! AI-bidder study probe — **how EPBot bids where authoring runs out**.
//!
//! Companion to `docs/ai-bidder/bba-floor.md`.  `MB.TXT` shows BBA's authored
//! rules are shallow (specific literal auctions vanish past ~depth 5) and the
//! deep tail is all *generic/parametric* templates.  This probe confirms the
//! live engine matches that picture: it drives EPBot down **deliberately deep,
//! off-book auctions** (no specific node can plausibly exist that deep) and
//! reads back the engine's own bid *and its self-description* via the
//! introspection ABI.  Two signatures tell programmatic-floor from dumb-lookup:
//!
//! * **Programmatic floor** — the call tracks the actor's hand (more values →
//!   higher action) and the engine still *describes* its bid, even with no
//!   spelled-out node → it computed the bid from a parametric/generic rule.
//! * **Ad-hoc lookup** — the call flattens / degenerates to Pass and the
//!   meaning goes blank on a miss.
//!
//! Throwaway: not wired into CI; delete with the report's scratch artifacts.
//! ```text
//! cargo run --release --example bba-floor-probe
//! ```

use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Level, Strain, Suit, eval};
use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};

const DEFAULT_LIB: &str = "vendor/bba/Native-libraries/linux/x64/libEPBot.so";
const SYSTEM_2_OVER_1: c_int = 0;
const SUITS: [Suit; 4] = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];
const STRAINS: [Strain; 5] = [
    Strain::Clubs,
    Strain::Diamonds,
    Strain::Hearts,
    Strain::Spades,
    Strain::Notrump,
];

// Confirmed C ABI (see examples/bba-match + the removed bba-wj-reference spike).
type CreateFn = unsafe extern "C" fn() -> *mut c_void;
type DestroyFn = unsafe extern "C" fn(*mut c_void);
type SetSystemFn = unsafe extern "C" fn(*mut c_void, c_int, c_int);
type NewHandFn =
    unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int, c_int, c_int, c_int);
type SetBidFn = unsafe extern "C" fn(*mut c_void, c_int, c_int, *const c_char);
type GetBidFn = unsafe extern "C" fn(*mut c_void) -> c_int;
// `epbot_interpret_bid(bot, position)` primes the engine's reading of the bid at
// `position`; `epbot_get_info_meaning[_extended](bot, position, buf, len)` then
// fills `buf` with the label / structured range sentence.
type InterpretFn = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
type GetInfoFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_char, c_int) -> c_int;

struct Bba {
    _lib: Library,
    create: CreateFn,
    destroy: DestroyFn,
    set_system: SetSystemFn,
    new_hand: NewHandFn,
    set_bid: SetBidFn,
    get_bid: GetBidFn,
    interpret: InterpretFn,
    meaning: GetInfoFn,
    meaning_ext: GetInfoFn,
}

impl Bba {
    fn load(path: &str) -> anyhow::Result<Self> {
        // SAFETY: trusted native library; signatures confirmed in S.0/S.2.
        let lib = unsafe { Library::new(path) }?;
        unsafe {
            Ok(Self {
                create: *lib.get::<CreateFn>(b"epbot_create\0")?,
                destroy: *lib.get::<DestroyFn>(b"epbot_destroy\0")?,
                set_system: *lib.get::<SetSystemFn>(b"epbot_set_system_type\0")?,
                new_hand: *lib.get::<NewHandFn>(b"epbot_new_hand\0")?,
                set_bid: *lib.get::<SetBidFn>(b"epbot_set_bid\0")?,
                get_bid: *lib.get::<GetBidFn>(b"epbot_get_bid\0")?,
                interpret: *lib.get::<InterpretFn>(b"epbot_interpret_bid\0")?,
                meaning: *lib.get::<GetInfoFn>(b"epbot_get_info_meaning\0")?,
                meaning_ext: *lib.get::<GetInfoFn>(b"epbot_get_info_meaning_extended\0")?,
                _lib: lib,
            })
        }
    }

    /// Drive a fresh bot through `auction` and return the actor's call plus the
    /// engine's own meaning / extended-meaning for that call.
    fn probe(&self, hand: Hand, auction: &[Call]) -> (Option<Call>, String, String) {
        let actor = (auction.len() % 4) as c_int;
        let suits = hand_to_suits(hand);
        let empty = c"".as_ptr();
        // SAFETY: a fresh bot used and destroyed within this call.
        unsafe {
            let bot = (self.create)();
            if bot.is_null() {
                return (None, String::new(), String::new());
            }
            for seat in 0..4 {
                (self.set_system)(bot, seat, SYSTEM_2_OVER_1);
            }
            (self.new_hand)(bot, actor, suits.as_ptr(), 0, 0, 0, 0);
            for (i, &call) in auction.iter().enumerate() {
                (self.set_bid)(bot, (i % 4) as c_int, encode_call(call), empty);
            }
            let code = (self.get_bid)(bot);
            // Register the engine's own call, then read how it describes it.
            (self.set_bid)(bot, actor, code, empty);
            (self.interpret)(bot, actor);
            let m = read_info(&self.meaning, bot, actor);
            let mx = read_info(&self.meaning_ext, bot, actor);
            (self.destroy)(bot);
            (decode_call(code), m, mx)
        }
    }
}

/// Read one info string for the bid at `position` into a fresh buffer.
unsafe fn read_info(get: &GetInfoFn, bot: *mut c_void, position: c_int) -> String {
    let mut buf = vec![0_u8; 1024];
    unsafe {
        get(
            bot,
            position,
            buf.as_mut_ptr().cast::<c_char>(),
            buf.len() as c_int,
        );
        CStr::from_ptr(buf.as_ptr().cast::<c_char>())
            .to_string_lossy()
            .into_owned()
    }
}

fn hand_to_suits(hand: Hand) -> CString {
    use core::fmt::Write;
    let mut s = String::with_capacity(20);
    for (i, suit) in SUITS.into_iter().enumerate() {
        if i > 0 {
            s.push('\n');
        }
        write!(s, "{}", hand[suit]).unwrap();
    }
    CString::new(s).unwrap()
}

fn hcp(hand: Hand) -> u8 {
    SUITS.iter().map(|&s| eval::hcp::<u8>(hand[s])).sum()
}

fn shape(hand: Hand) -> String {
    // spades-hearts-diamonds-clubs, the usual quoting order
    format!(
        "{}{}{}{}",
        hand[Suit::Spades].len(),
        hand[Suit::Hearts].len(),
        hand[Suit::Diamonds].len(),
        hand[Suit::Clubs].len()
    )
}

fn encode_call(call: Call) -> c_int {
    match call {
        Call::Pass => 0,
        Call::Double => 1,
        Call::Redouble => 2,
        Call::Bid(bid) => {
            let strain = STRAINS.iter().position(|&s| s == bid.strain).unwrap() as c_int;
            5 + (c_int::from(bid.level.get()) - 1) * 5 + strain
        }
    }
}

fn decode_call(code: c_int) -> Option<Call> {
    match code {
        0 => Some(Call::Pass),
        1 => Some(Call::Double),
        2 => Some(Call::Redouble),
        5..=39 => {
            let i = (code - 5) as u8;
            Some(Call::Bid(Bid {
                level: Level::new(i / 5 + 1),
                strain: STRAINS[(i % 5) as usize],
            }))
        }
        _ => None,
    }
}

/// Render a [`Call`] compactly (e.g. `4H`, `P`, `X`).
fn show(call: Option<Call>) -> String {
    match call {
        None => "??".into(),
        Some(Call::Pass) => "P".into(),
        Some(Call::Double) => "X".into(),
        Some(Call::Redouble) => "XX".into(),
        Some(Call::Bid(b)) => {
            let s = match b.strain {
                Strain::Clubs => "C",
                Strain::Diamonds => "D",
                Strain::Hearts => "H",
                Strain::Spades => "S",
                Strain::Notrump => "NT",
            };
            format!("{}{s}", b.level.get())
        }
    }
}

/// Parse a space-separated call list like `1C P 1H 1S 2H 2S 3H 3S`.
fn auction(s: &str) -> Vec<Call> {
    s.split_whitespace()
        .map(|t| match t {
            "P" => Call::Pass,
            "X" => Call::Double,
            "XX" => Call::Redouble,
            _ => {
                let level = Level::new(t[..1].parse().unwrap());
                let strain = t[1..].parse::<Strain>().unwrap();
                Call::Bid(Bid { level, strain })
            }
        })
        .collect()
}

fn main() -> anyhow::Result<()> {
    let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
    let bba = Bba::load(&path)?;

    // -- Probe 1: HCP sweep on a DEEP competitive auction (8 calls in) ---------
    // N opened 1C, partner showed hearts, both sides jostled to the 3-level.
    // No specific MB.TXT node lives at depth 8; whatever N bids here is the
    // floor.  Hands fixed at 2=5=2=4 (heart support + club length, consistent
    // with the 1C open), HCP swept low→high.  A programmatic floor escalates
    // P → competitive raise → game as the hand improves.
    let seq1 = auction("1C P 1H 1S 2H 2S 3H 3S");
    let sweep = [
        "J2.T9542.J2.9876",
        "Q2.QT954.Q2.K876",
        "K2.KJ954.K2.A876",
        "A2.AQ954.K2.AQ76",
        "AK.AKJ54.A2.AK76",
    ];
    println!(
        "== Probe 1: HCP sweep, DEEP off-book auction (depth {}) ==",
        seq1.len()
    );
    println!("auction: 1C P 1H 1S 2H 2S 3H 3S  — N (shape 2=5=2=4) to call\n");
    println!(
        "{:>4}  {:>5}  {:>4}   EPBot's meaning for its own call",
        "hcp", "shape", "bid"
    );
    for h in sweep {
        let hand: Hand = h.parse()?;
        let (call, m, _mx) = bba.probe(hand, &seq1);
        println!(
            "{:>4}  {:>5}  {:>4}   {}",
            hcp(hand),
            shape(hand),
            show(call),
            m.trim()
        );
    }

    // -- Probe 2: same hand, increasing auction depth --------------------------
    // One fixed strong-ish hand; lengthen the auction prefix call by call.  If
    // the engine still names a sensible, described bid as depth grows past where
    // specific nodes exist, the floor is carrying it.
    let hand: Hand = "KQ2.AQ954.K2.A76".parse()?;
    println!(
        "\n== Probe 2: fixed hand ({} HCP, {}), growing auction depth ==",
        hcp(hand),
        shape(hand)
    );
    println!("hand: ♠KQ2 ♥AQ954 ♦K2 ♣A76\n");
    let prefixes = [
        "1H 1S 2S",
        "1H 1S 2S P P 3S",
        "1H 1S 2S P P 3S P P 4S",
        "1H 1S 2S P P 3S P P 4S P P 5S",
    ];
    println!("{:>5}  {:>4}   EPBot's meaning + extended", "depth", "bid");
    for p in prefixes {
        let seq = auction(p);
        let (call, m, mx) = bba.probe(hand, &seq);
        println!(
            "{:>5}  {:>4}   {} | {}",
            seq.len(),
            show(call),
            m.trim(),
            mx.trim()
        );
    }

    Ok(())
}
