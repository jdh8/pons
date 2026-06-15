//! AI-bidder **Side-track S.2** — the WJ (Polish Club) reference set.
//!
//! Harvests Edward Piwowar's EPBot bidding under its **WJ — *Wspólny Język* /
//! Polish Club** system (EPBot system type 2, the `WJ.bbsa` card) as ground
//! truth for the M4.3 Polish Club port and a head-start on the second corpus M5
//! needs.  Driven natively through `libEPBot.so` (no Wine — see the S.0 spike in
//! `examples/bba-oracle`; the FFI is the same one S.1's `bba-match` confirmed,
//! plus the meaning-introspection calls discovered here).
//!
//! Each board is bid out by a *table of WJ bidders* (all four seats system 2,
//! self-play), one fresh bot per decision so each call sees only the actor's
//! hand.  For every call we record the auction-so-far, the call, the actor's
//! hand, and — for the **first round** of the auction — BBA's *self-reported
//! meaning*: a short systemic label (`"Polish 1C"`, `"bidable suit"`,
//! `"preemptive"`) plus the constraint ranges it shows (point range + per-suit
//! length ranges).  These map straight onto the `Constraint` DSL the port
//! authors against.
//!
//! ## Meaning reliability — first round only
//!
//! EPBot's introspection (`epbot_get_info_meaning[_extended]`) exposes the
//! systemic meaning of a bid reliably only for the **first four calls** of an
//! auction (one round).  From position 4 on, the same array indices report
//! per-seat *hand inferences* instead of bid meanings — a structural limit of
//! the FFI surface as driven outside the BBA application.  We therefore capture
//! meanings for positions `0..4` and detect "no info" (a trivial 0–37-point
//! range) to drop the garbage; calls past the first round carry the call only.
//! The opening and first responses are a system's defining, most-distinct part
//! and exactly what the port builds first, so this is the high-value slice.
//!
//! Output: a JSONL reference set (one record per `(auction, call)`) plus a
//! versioned JSON sidecar (system, seed, git SHA, schema, counts).  The proper
//! `bidding::verify` per-auction check against the *ported* books lands with
//! M4.3 — this milestone produces the reference those checks diff against.
//!
//! EPBot ships in the `vendor/bba` git submodule (BBA is free for non-commercial
//! use and redistribution); `git submodule update --init vendor/bba` resolves the
//! default library path, or point `BBA_LIB` elsewhere:
//!
//! ```text
//! cargo run --release --example bba-wj-reference -- --count 2000 --seed 1
//! BBA_LIB=/path/to/libEPBot.so cargo run --release --example bba-wj-reference
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call, RelativeVulnerability};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Bid, FullDeal, Hand, Level, Seat, Strain, Suit};
use libloading::Library;
use pons::bidding::context::relative;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::io::Write;

const DEFAULT_LIB: &str = "vendor/bba/Native-libraries/linux/x64/libEPBot.so";

/// System index 2 = "WJ" (Wspólny Język / Polish Club), per the `WJ.bbsa` card's
/// `System type = 2` and confirmed behaviorally (an 18-balanced hand opens 1♣,
/// the Polish Club catch-all, not 1NT/1♠ as 2/1 would).
const SYSTEM_WJ: c_int = 2;

/// Calls whose first-round meaning we trust (see the module docs).
const MEANING_ROUND: usize = 4;

/// Harvest BBA's WJ (Polish Club) auctions as a reference set
#[derive(Parser)]
struct Args {
    /// Number of random boards to harvest (dealer + vul rotate per board)
    #[arg(short, long, default_value = "2000")]
    count: usize,

    /// RNG seed (for reproducibility)
    #[arg(short, long, default_value = "1")]
    seed: u64,

    /// Output JSONL path for the reference records (sidecar `.json` alongside)
    #[arg(short, long, default_value = "target/wj-reference.jsonl")]
    output: String,

    /// Print the curated textbook fixtures to stderr as they are checked
    #[arg(long, default_value_t = true)]
    show_fixtures: bool,
}

// ---------------------------------------------------------------------------
// EPBot FFI — the S.1 ABI plus the meaning-introspection calls
// ---------------------------------------------------------------------------

type CreateFn = unsafe extern "C" fn() -> *mut c_void;
type DestroyFn = unsafe extern "C" fn(*mut c_void);
type SetSystemFn = unsafe extern "C" fn(*mut c_void, c_int, c_int);
type NewHandFn =
    unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int, c_int, c_int, c_int);
type SetBidFn = unsafe extern "C" fn(*mut c_void, c_int, c_int, *const c_char);
type GetBidFn = unsafe extern "C" fn(*mut c_void) -> c_int;
// `epbot_interpret_bid(bot, position)` primes the engine's interpretation of the
// bid at `position`; `epbot_get_info_meaning[_extended](bot, position, buf, len)`
// then fills `buf` with the meaning text (a short label, resp. a structured
// range sentence).  Signatures recovered by `objdump` (the buffer form: a bounds
// check on `position`, then a string-fill into `buf` of capacity `len`).
type InterpretFn = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
type GetInfoFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_char, c_int) -> c_int;

/// EPBot's WJ bidder, driven for the harvest.
struct Wj {
    _lib: Library,
    create: CreateFn,
    destroy: DestroyFn,
    set_system: SetSystemFn,
    new_hand: NewHandFn,
    set_bid: SetBidFn,
    get_bid: GetBidFn,
    interpret: InterpretFn,
    get_meaning: GetInfoFn,
    get_meaning_ext: GetInfoFn,
}

impl Wj {
    fn load(path: &str) -> anyhow::Result<Self> {
        // SAFETY: loading a trusted native library; its initializers run here.
        let lib = unsafe { Library::new(path) }?;
        // SAFETY: each symbol has the signature confirmed in S.0/S.1 (the
        // bidding calls) or recovered here by disassembly (the info calls);
        // `*sym` copies the `Copy` function pointer, `_lib` keeps it valid.
        unsafe {
            Ok(Self {
                create: *lib.get::<CreateFn>(b"epbot_create\0")?,
                destroy: *lib.get::<DestroyFn>(b"epbot_destroy\0")?,
                set_system: *lib.get::<SetSystemFn>(b"epbot_set_system_type\0")?,
                new_hand: *lib.get::<NewHandFn>(b"epbot_new_hand\0")?,
                set_bid: *lib.get::<SetBidFn>(b"epbot_set_bid\0")?,
                get_bid: *lib.get::<GetBidFn>(b"epbot_get_bid\0")?,
                interpret: *lib.get::<InterpretFn>(b"epbot_interpret_bid\0")?,
                get_meaning: *lib.get::<GetInfoFn>(b"epbot_get_info_meaning\0")?,
                get_meaning_ext: *lib.get::<GetInfoFn>(b"epbot_get_info_meaning_extended\0")?,
                _lib: lib,
            })
        }
    }

    /// WJ's call for `hand` after `auction`, with the dealer canonicalized to
    /// position 0 (the S.1 `BbaOracle` flow): a fresh bot, all four seats set to
    /// WJ, the actor's hand dealt, the auction replayed, the call read.  Returns
    /// [`Call::Pass`] on an error code.
    fn classify(&self, hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> Call {
        let actor = (auction.len() % 4) as c_int;
        let suits = hand_to_suits(hand);
        let empty = c"".as_ptr();
        // SAFETY: a fresh bot used and destroyed within this call; argument
        // types match the confirmed ABI.
        let code = unsafe {
            let bot = (self.create)();
            if bot.is_null() {
                return Call::Pass;
            }
            for seat in 0..4 {
                (self.set_system)(bot, seat, SYSTEM_WJ);
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
        decode_call(code).unwrap_or(Call::Pass)
    }

    /// The first-round meanings EPBot attaches to `auction` under WJ: one entry
    /// per call at positions `0..MEANING_ROUND`, [`None`] where the engine shows
    /// no systemic info.  `vul`/`dealer` set the bot's vulnerability so the
    /// (vul-sensitive) preempt ranges are right.
    fn round_meanings(
        &self,
        auction: &[Call],
        vul: AbsoluteVulnerability,
        dealer: Seat,
    ) -> Vec<Option<Meaning>> {
        let n = auction.len().min(MEANING_ROUND);
        let empty = c"".as_ptr();
        // SAFETY: a fresh bot, used and destroyed here; the info calls fill our
        // own stack buffer of the capacity we pass.
        unsafe {
            let bot = (self.create)();
            if bot.is_null() {
                return vec![None; n];
            }
            for seat in 0..4 {
                (self.set_system)(bot, seat, SYSTEM_WJ);
            }
            // Canonical dealer at position 0; no hands dealt, so the meanings are
            // purely systemic (a dealt hand would leak its actual holding).
            (self.new_hand)(
                bot,
                0,
                empty,
                0,
                epbot_vulnerability(relative(vul, dealer), 0),
                0,
                0,
            );
            let meanings = (0..auction.len())
                .map(|p| {
                    (self.set_bid)(bot, (p % 4) as c_int, encode_call(auction[p]), empty);
                    (self.interpret)(bot, p as c_int);
                    (p < n).then(|| {
                        let label = self.read_info(&self.get_meaning, bot, p as c_int);
                        let ext = self.read_info(&self.get_meaning_ext, bot, p as c_int);
                        Meaning::parse(&label, &ext)
                    })
                })
                .take(n)
                .collect();
            (self.destroy)(bot);
            meanings
        }
    }

    /// Read one info string for the bid at `position` into a fresh buffer.
    ///
    /// # Safety
    /// `bot` must be a live EPBot handle and `get` one of its info getters.
    unsafe fn read_info(&self, get: &GetInfoFn, bot: *mut c_void, position: c_int) -> String {
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
}

// ---------------------------------------------------------------------------
// BBA's self-reported meaning of a call
// ---------------------------------------------------------------------------

/// What WJ shows for a bid: a short systemic label and, when present, the point
/// range and per-suit (C, D, H, S) length ranges it promises.
#[derive(Clone)]
struct Meaning {
    label: String,
    points: Option<(u8, u8)>,
    /// (min, max) cards in clubs, diamonds, hearts, spades — `None` if no info.
    lengths: Option<[(u8, u8); 4]>,
}

impl Meaning {
    /// Parse the short `label` and the structured `extended` sentence
    /// (`"… ALERT. <lo> to <hi> total points, <lo> to <hi> cards in clubs, …"`).
    /// A trivial 0–37 point range is EPBot's "no constraint shown" → no ranges.
    fn parse(label: &str, extended: &str) -> Self {
        let label = label.trim().to_string();
        // The text after "ALERT." is the range sentence; the prefix before it is
        // a formatting artifact (an alert/next-bid token) we discard.
        let body = extended.split_once("ALERT.").map(|(_, tail)| tail);
        let ranges: Vec<(u8, u8)> = body
            .into_iter()
            .flat_map(|tail| tail.split(','))
            .filter_map(parse_range)
            .collect();
        // Expect exactly five ranges: points, then clubs/diamonds/hearts/spades.
        let (points, lengths) = if ranges.len() == 5 {
            let pts = ranges[0];
            let lens = [ranges[1], ranges[2], ranges[3], ranges[4]];
            // 0..=37 points = unconstrained: EPBot's "nothing shown" sentinel.
            if pts.0 == 0 && pts.1 >= 37 {
                (None, None)
            } else {
                (Some(pts), Some(lens))
            }
        } else {
            (None, None)
        };
        Self {
            label,
            points,
            lengths,
        }
    }

    /// True when this carries nothing worth recording (no label, no ranges).
    fn is_empty(&self) -> bool {
        self.label.is_empty() && self.points.is_none()
    }
}

/// Parse the leading `"<lo> to <hi>"` of a segment like `"11 to 37 total points"`.
fn parse_range(segment: &str) -> Option<(u8, u8)> {
    let mut words = segment.split_whitespace();
    let lo: u8 = words.next()?.parse().ok()?;
    if words.next()? != "to" {
        return None;
    }
    let hi: u8 = words.next()?.parse().ok()?;
    Some((lo, hi))
}

// ---------------------------------------------------------------------------
// Shared EPBot encoding helpers (mirroring examples/bba-match)
// ---------------------------------------------------------------------------

/// The four holdings in EPBot's C, D, H, S order, newline-joined.
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

/// EPBot vulnerability code from the actor-relative vulnerability (1 = N/S,
/// 2 = E/W bits; the actor's side is N/S iff `actor` is even).
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

/// Encode a [`Call`] into EPBot's integer bid code.
fn encode_call(call: Call) -> c_int {
    match call {
        Call::Pass => 0,
        Call::Double => 1,
        Call::Redouble => 2,
        Call::Bid(bid) => 5 + (c_int::from(bid.level.get()) - 1) * 5 + strain_index(bid.strain),
    }
}

/// Decode EPBot's bid code back into a [`Call`], or [`None`] on an error code.
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

/// Strains in EPBot/[`Strain`] discriminant order (♣ ♦ ♥ ♠ NT).
const STRAINS: [Strain; 5] = [
    Strain::Clubs,
    Strain::Diamonds,
    Strain::Hearts,
    Strain::Spades,
    Strain::Notrump,
];

/// The 0..=4 index of a strain.
fn strain_index(strain: Strain) -> c_int {
    STRAINS
        .iter()
        .position(|&s| s == strain)
        .expect("every strain is in STRAINS") as c_int
}

/// The seat acting after `len` calls from `dealer`.
const fn seat_to_act(dealer: Seat, len: usize) -> Seat {
    Seat::ALL[(dealer as usize + len) % 4]
}

/// Bid out one deal with WJ in all four seats, recording each decision.
fn bid_out(wj: &Wj, dealer: Seat, vul: AbsoluteVulnerability, deal: &FullDeal) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let call = wj.classify(deal[seat], relative(vul, seat), auction.as_ref());
        // A pass keeps the auction finite even if the engine returns something
        // illegal at this point (it should not, but the harness must terminate).
        if auction.try_push(call).is_err() {
            auction.push(Call::Pass);
        }
    }
    auction
}

// ---------------------------------------------------------------------------
// Reference records
// ---------------------------------------------------------------------------

/// One `(auction, call)` reference record.
struct Record<'a> {
    /// "bulk" for the random harvest, the fixture name for a curated opening.
    kind: &'a str,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    seat: Seat,
    position: usize,
    auction: Vec<Call>,
    call: Call,
    hand: Hand,
    meaning: Option<Meaning>,
}

impl Record<'_> {
    fn to_json(&self) -> String {
        let auction: Vec<String> = self.auction.iter().map(|c| format!("{c}")).collect();
        let lengths = self.meaning.as_ref().and_then(|m| m.lengths).map(|l| {
            serde_json::json!({
                "C": [l[0].0, l[0].1],
                "D": [l[1].0, l[1].1],
                "H": [l[2].0, l[2].1],
                "S": [l[3].0, l[3].1],
            })
        });
        serde_json::json!({
            "kind": self.kind,
            "dealer": format!("{:?}", self.dealer),
            "vul": format!("{}", self.vul),
            "seat": format!("{:?}", self.seat),
            "position": self.position,
            "auction": auction,
            "call": format!("{}", self.call),
            "hand": format!("{}", self.hand),
            "meaning": self.meaning.as_ref().map_or("", |m| m.label.as_str()),
            "points": self.meaning.as_ref().and_then(|m| m.points).map(|(lo, hi)| [lo, hi]),
            "lengths": lengths,
        })
        .to_string()
    }
}

/// Turn one bid-out into its per-call records, attaching first-round meanings.
fn records_for_board<'a>(
    wj: &Wj,
    kind: &'a str,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
    auction: &Auction,
) -> Vec<Record<'a>> {
    let calls: Vec<Call> = auction.iter().copied().collect();
    let meanings = wj.round_meanings(&calls, vul, dealer);
    calls
        .iter()
        .enumerate()
        .map(|(position, &call)| {
            let seat = seat_to_act(dealer, position);
            Record {
                kind,
                dealer,
                vul,
                seat,
                position,
                auction: calls[..position].to_vec(),
                call,
                hand: deal[seat],
                meaning: meanings.get(position).cloned().flatten(),
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Curated textbook fixtures — Polish Club's defining openings
// ---------------------------------------------------------------------------

/// A canonical Polish Club opening: a name, the opener's hand (PBN `S.H.D.C`),
/// and — for the few rock-solid ones — the call WJ must produce.
struct Fixture {
    name: &'static str,
    hand: &'static str,
    /// Asserted opening (e.g. `"1♣"`); `None` = recorded + printed only.
    expect: Option<&'static str>,
}

/// The hallmark Polish Club openings.  Only the system-defining calls are hard
/// assertions (the strong/catch-all 1♣ and the 15–17 1NT); the rest are dealt to
/// EPBot and recorded as the textbook reference for the M4.3 port to match.
const FIXTURES: &[Fixture] = &[
    // The defining feature: a strong balanced hand too good for 1NT opens 1♣,
    // NOT 1NT/1-of-a-suit as 2/1 would.
    Fixture {
        name: "polish-1c-strong-balanced",
        hand: "AQ5.KJ4.KQ72.K43", // 18 balanced
        expect: Some("1♣"),
    },
    // A natural club hand also routes through the catch-all 1♣.
    Fixture {
        name: "polish-1c-clubs",
        hand: "43.K43.Q82.AKJ95", // 12, 5 clubs
        expect: Some("1♣"),
    },
    // Weak NT range is 15–17 on this card (WJ.bbsa: "1NT range 15-17 = 1").
    Fixture {
        name: "1nt-15-17",
        hand: "KJ4.AQ5.Q872.K32", // 15 balanced, 3-3-4-3
        expect: Some("1NT"),
    },
    // Limited five-card majors.
    Fixture {
        name: "1h-five-card-major",
        hand: "K3.AQ952.KJ3.842", // 12, 5 hearts
        expect: Some("1♥"),
    },
    Fixture {
        name: "1s-five-card-major",
        hand: "AQ952.K3.KJ3.842", // 12, 5 spades
        expect: Some("1♠"),
    },
    // A 2/1-style strong balanced 20–21 (in WJ also through 1♣); printed only.
    Fixture {
        name: "strong-balanced-21",
        hand: "AQ5.AKJ.KQ72.Q43", // 21 balanced, 3-3-4-3
        expect: None,
    },
    // A diamond opening (length conventions vary by card — recorded only).
    Fixture {
        name: "1d-opener",
        hand: "K3.842.AQJ95.KJ3",
        expect: None,
    },
    // A weak hand with a long suit — a preempt under any system.
    Fixture {
        name: "weak-two-spades",
        hand: "KQJ976.43.852.42", // 6 spades, ~6 HCP
        expect: None,
    },
];

/// Bid the opener's hand (none vul, first seat) and return WJ's call + meaning.
fn open(wj: &Wj, hand: Hand) -> (Call, Option<Meaning>) {
    let call = wj.classify(hand, RelativeVulnerability::NONE, &[]);
    let meanings = wj.round_meanings(&[call], AbsoluteVulnerability::NONE, Seat::North);
    (call, meanings.into_iter().next().flatten())
}

/// Run the curated fixtures: assert the hard ones, record + print them all.
/// Returns the records (tagged with each fixture's name) and the failure count.
fn run_fixtures<'a>(wj: &Wj, show: bool) -> (Vec<Record<'a>>, usize) {
    let mut records = Vec::new();
    let mut failures = 0;
    for fixture in FIXTURES {
        let hand: Hand = fixture.hand.parse().expect("a fixture hand parses");
        assert_eq!(
            hand.into_iter().count(),
            13,
            "fixture `{}` hand `{}` must hold exactly 13 cards",
            fixture.name,
            fixture.hand,
        );
        let (call, meaning) = open(wj, hand);
        let rendered = format!("{call}");
        let verdict = match fixture.expect {
            Some(want) if want != rendered => {
                failures += 1;
                format!("FAIL (expected {want})")
            }
            Some(_) => "ok".to_string(),
            None => "(recorded)".to_string(),
        };
        if show {
            let label = meaning.as_ref().map_or("", |m| m.label.as_str());
            eprintln!(
                "  {:<28} {:<12} -> {:<4} {verdict:<18} [{label}]",
                fixture.name, fixture.hand, rendered,
            );
        }
        records.push(Record {
            kind: fixture.name,
            dealer: Seat::North,
            vul: AbsoluteVulnerability::NONE,
            seat: Seat::North,
            position: 0,
            auction: Vec::new(),
            call,
            hand,
            meaning,
        });
    }
    (records, failures)
}

// ---------------------------------------------------------------------------
// Driver
// ---------------------------------------------------------------------------

/// Rotate vulnerability with the board, like a real session.
const VULS: [AbsoluteVulnerability; 4] = [
    AbsoluteVulnerability::NONE,
    AbsoluteVulnerability::NS,
    AbsoluteVulnerability::EW,
    AbsoluteVulnerability::ALL,
];

/// Best-effort current commit for the sidecar; `"unknown"` on failure.
fn git_sha() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map_or_else(|| "unknown".to_string(), |s| s.trim().to_string())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
    let wj = match Wj::load(&path) {
        Ok(wj) => wj,
        Err(error) => {
            eprintln!(
                "could not load EPBot native lib at `{path}`: {error}\n\
                 Fetch it with `git submodule update --init vendor/bba`, or set BBA_LIB."
            );
            std::process::exit(1);
        }
    };

    let file = std::fs::File::create(&args.output)?;
    let mut writer = std::io::BufWriter::new(file);

    // The curated textbook openings first: the named, asserted reference.
    eprintln!("=== Curated textbook WJ openings (Polish Club) ===");
    let (fixtures, failures) = run_fixtures(&wj, args.show_fixtures);
    let mut records = 0_u64;
    let mut with_meaning = 0_u64;
    for record in &fixtures {
        if record.meaning.as_ref().is_some_and(|m| !m.is_empty()) {
            with_meaning += 1;
        }
        writeln!(writer, "{}", record.to_json())?;
        records += 1;
    }
    if failures > 0 {
        eprintln!(
            "\n{failures} curated fixture(s) FAILED — WJ did not bid the textbook call.\n\
             (BBA is ground truth here; a failure means the fixture's expectation is wrong.)"
        );
    } else {
        eprintln!("all asserted fixtures bid their textbook call.");
    }

    // The random bulk harvest: a table of WJ bidders over `count` boards.
    let mut rng = StdRng::seed_from_u64(args.seed);
    let mut call_hist: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for index in 0..args.count {
        let dealer = Seat::ALL[index % 4];
        let vul = VULS[index % 4];
        let deal = full_deal(&mut rng);
        let auction = bid_out(&wj, dealer, vul, &deal);
        for record in records_for_board(&wj, "bulk", dealer, vul, &deal, &auction) {
            if record.meaning.as_ref().is_some_and(|m| !m.is_empty()) {
                with_meaning += 1;
            }
            *call_hist.entry(format!("{}", record.call)).or_insert(0) += 1;
            writeln!(writer, "{}", record.to_json())?;
            records += 1;
        }
    }
    writer.flush()?;

    let metadata = serde_json::json!({
        "schema": "wj-reference/v1: one JSONL record per (auction, call)",
        "fields": "kind, dealer, vul, seat, position, auction, call, hand, meaning, points, lengths",
        "system": "WJ (Wspólny Język / Polish Club), EPBot system type 2",
        "engine": "EPBot (libEPBot.so, native FFI)",
        "meaning_reliability": format!(
            "systemic label + ranges captured for the first {MEANING_ROUND} calls only; \
             later positions carry the call alone (EPBot FFI limit)"
        ),
        "git_sha": git_sha(),
        "seed": args.seed,
        "boards": args.count,
        "records": records,
        "records_with_meaning": with_meaning,
        "fixtures": FIXTURES.len(),
        "fixture_failures": failures,
    });
    let json_path = format!("{}.json", args.output.trim_end_matches(".jsonl"));
    std::fs::write(&json_path, format!("{metadata:#}\n"))?;

    let pct = |n: u64| {
        if records == 0 {
            0.0
        } else {
            100.0 * n as f64 / records as f64
        }
    };
    eprintln!(
        "\nwj-reference: {records} records from {} boards + {} fixtures → {} ({:.0}% carry a meaning).",
        args.count,
        FIXTURES.len(),
        args.output,
        pct(with_meaning),
    );
    eprintln!("sidecar: {json_path}");
    eprintln!("top calls:");
    let mut hist: Vec<(String, u64)> = call_hist.into_iter().collect();
    hist.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    for (call, count) in hist.into_iter().take(12) {
        eprintln!("  {call:>4}  {count:>8}");
    }
    if failures > 0 {
        std::process::exit(1);
    }
    Ok(())
}
