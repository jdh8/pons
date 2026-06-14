//! AI-bidder **M4.3 cross-check** — our Polish Club vs BBA's WJ reference.
//!
//! The port half of the S.2 reference (`examples/bba-wj-reference`): where S.2
//! *harvested* BBA/EPBot's **WJ (Wspólny Język / Polish Club)** calls, this asks
//! whether our authored [`polish_club`][pons::bidding::polish_club] bids the same
//! way.  BBA WJ is an **informational** oracle, not the spec — the author's
//! Strawberry notes are authoritative and diverge from generic WJ on the
//! two-level (Ekren 2♣, Muiderberg 2♥/2♠, unusual 2NT) and the response
//! conventions.  So only the curated textbook fixtures are hard assertions
//! (against *our* system); the agreement numbers are reported, and divergences
//! are listed as the authoring TODO, never failed.
//!
//! Anchored on BBA: each board, BBA WJ opens the dealer's hand; on the
//! uncontested auction (LHO passes) BBA WJ also responds with partner's hand.
//! We ask our system the same two questions and tally agreement, bucketed for
//! the **overlap** openings (1♣/1♦/1♥/1♠/1NT/2♦) where the two systems should
//! line up.
//!
//! ```text
//! cargo run --release --example polish-club-reference -- --count 2000 --seed 1
//! BBA_LIB=/path/to/libEPBot.so cargo run --example polish-club-reference
//! ```

use clap::Parser;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Bid, Hand, Level, Seat, Strain, Suit};
use libloading::Library;
use pons::bidding::context::relative;
use pons::bidding::polish_club::polish_club;
use pons::bidding::{Family, Stance, System};
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::collections::HashMap;
use std::ffi::{CString, c_char, c_int, c_void};

const DEFAULT_LIB: &str = "vendor/bba/Native-libraries/linux/x64/libEPBot.so";

/// System index 2 = WJ (Wspólny Język / Polish Club).
const SYSTEM_WJ: c_int = 2;

/// Cross-check our Polish Club port against BBA's WJ reference
#[derive(Parser)]
struct Args {
    /// Number of random boards (dealer + vulnerability rotate per board)
    #[arg(short, long, default_value = "2000")]
    count: usize,

    /// RNG seed (for reproducibility)
    #[arg(short, long, default_value = "1")]
    seed: u64,

    /// How many distinct disagreement patterns to list
    #[arg(long, default_value = "20")]
    top: usize,
}

// ---------------------------------------------------------------------------
// EPBot FFI (the S.1 bidding ABI; meaning introspection is not needed here)
// ---------------------------------------------------------------------------

type CreateFn = unsafe extern "C" fn() -> *mut c_void;
type DestroyFn = unsafe extern "C" fn(*mut c_void);
type SetSystemFn = unsafe extern "C" fn(*mut c_void, c_int, c_int);
type NewHandFn =
    unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int, c_int, c_int, c_int);
type SetBidFn = unsafe extern "C" fn(*mut c_void, c_int, c_int, *const c_char);
type GetBidFn = unsafe extern "C" fn(*mut c_void) -> c_int;

/// EPBot's WJ bidder.
struct Wj {
    _lib: Library,
    create: CreateFn,
    destroy: DestroyFn,
    set_system: SetSystemFn,
    new_hand: NewHandFn,
    set_bid: SetBidFn,
    get_bid: GetBidFn,
}

impl Wj {
    fn load(path: &str) -> anyhow::Result<Self> {
        // SAFETY: loading a trusted native library; its initializers run here.
        let lib = unsafe { Library::new(path) }?;
        // SAFETY: each symbol has the ABI confirmed in S.0/S.1.
        unsafe {
            Ok(Self {
                create: *lib.get::<CreateFn>(b"epbot_create\0")?,
                destroy: *lib.get::<DestroyFn>(b"epbot_destroy\0")?,
                set_system: *lib.get::<SetSystemFn>(b"epbot_set_system_type\0")?,
                new_hand: *lib.get::<NewHandFn>(b"epbot_new_hand\0")?,
                set_bid: *lib.get::<SetBidFn>(b"epbot_set_bid\0")?,
                get_bid: *lib.get::<GetBidFn>(b"epbot_get_bid\0")?,
                _lib: lib,
            })
        }
    }

    /// WJ's call for `hand` after `auction` (a fresh bot per decision).
    fn classify(&self, hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> Call {
        let actor = (auction.len() % 4) as c_int;
        let suits = hand_to_suits(hand);
        let empty = c"".as_ptr();
        // SAFETY: a fresh bot used and destroyed within this call; argument types
        // match the confirmed ABI.
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
}

// ---------------------------------------------------------------------------
// EPBot encoding helpers (mirroring examples/bba-wj-reference)
// ---------------------------------------------------------------------------

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

fn encode_call(call: Call) -> c_int {
    match call {
        Call::Pass => 0,
        Call::Double => 1,
        Call::Redouble => 2,
        Call::Bid(bid) => 5 + (c_int::from(bid.level.get()) - 1) * 5 + strain_index(bid.strain),
    }
}

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

const STRAINS: [Strain; 5] = [
    Strain::Clubs,
    Strain::Diamonds,
    Strain::Hearts,
    Strain::Spades,
    Strain::Notrump,
];

fn strain_index(strain: Strain) -> c_int {
    STRAINS
        .iter()
        .position(|&s| s == strain)
        .expect("every strain is in STRAINS") as c_int
}

// ---------------------------------------------------------------------------
// Our side
// ---------------------------------------------------------------------------

/// Our system's highest finite-logit call, [`Call::Pass`] when it has none.
fn our_call(sys: &Stance, hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> Call {
    match sys.classify(hand, vul, auction) {
        None => Call::Pass,
        Some(logits) => (&logits.0)
            .into_iter()
            .filter(|(_, l)| l.is_finite())
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .unwrap_or(Call::Pass),
    }
}

/// The overlap openings — where Strawberry and generic WJ should line up.
fn is_overlap_opening(call: Call) -> bool {
    matches!(
        call,
        Call::Bid(Bid { level, strain })
            if level.get() == 1
                || (level.get() == 2 && strain == Strain::Diamonds)
    )
}

// ---------------------------------------------------------------------------
// Curated textbook fixtures — checked against OUR system (hard), BBA shown
// ---------------------------------------------------------------------------

/// Reused verbatim from `examples/bba-wj-reference` (S.2).
struct Fixture {
    name: &'static str,
    hand: &'static str,
    expect: Option<&'static str>,
}

const FIXTURES: &[Fixture] = &[
    Fixture {
        name: "polish-1c-strong-balanced",
        hand: "AQ5.KJ4.KQ72.K43",
        expect: Some("1♣"),
    },
    Fixture {
        name: "polish-1c-clubs",
        hand: "43.K43.Q82.AKJ95",
        expect: Some("1♣"),
    },
    Fixture {
        name: "1nt-15-17",
        hand: "KJ4.AQ5.Q872.K32",
        expect: Some("1NT"),
    },
    Fixture {
        name: "1h-five-card-major",
        hand: "K3.AQ952.KJ3.842",
        expect: Some("1♥"),
    },
    Fixture {
        name: "1s-five-card-major",
        hand: "AQ952.K3.KJ3.842",
        expect: Some("1♠"),
    },
    Fixture {
        name: "strong-balanced-21",
        hand: "AQ5.AKJ.KQ72.Q43",
        expect: Some("1♣"),
    },
    Fixture {
        name: "1d-opener",
        hand: "K3.842.AQJ95.KJ3",
        expect: Some("1♦"),
    },
    Fixture {
        name: "weak-two-spades",
        hand: "KQJ976.43.852.42",
        expect: Some("2♦"),
    },
];

/// Assert our system opens each fixture; print BBA's call alongside.  Returns
/// the number of hard-assertion failures.
fn run_fixtures(ours: &Stance, wj: &Wj) -> usize {
    eprintln!("=== Curated textbook openings (ours, hard) — BBA WJ shown ===");
    let mut failures = 0;
    for fixture in FIXTURES {
        let hand: Hand = fixture.hand.parse().expect("a fixture hand parses");
        let mine = our_call(ours, hand, RelativeVulnerability::NONE, &[]);
        let theirs = wj.classify(hand, RelativeVulnerability::NONE, &[]);
        let rendered = format!("{mine}");
        let verdict = match fixture.expect {
            Some(want) if want != rendered => {
                failures += 1;
                format!("FAIL (want {want})")
            }
            _ => "ok".to_string(),
        };
        eprintln!(
            "  {:<28} {:<17} ours {:<4} bba {:<4} {verdict}",
            fixture.name,
            fixture.hand,
            rendered,
            format!("{theirs}"),
        );
    }
    failures
}

// ---------------------------------------------------------------------------
// Agreement tally
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Tally {
    total: u64,
    agree: u64,
    /// (bba_call, our_call) → count, for the disagreements.
    diffs: HashMap<(String, String), u64>,
}

impl Tally {
    fn record(&mut self, bba: Call, ours: Call) {
        self.total += 1;
        if bba == ours {
            self.agree += 1;
        } else {
            *self
                .diffs
                .entry((format!("{bba}"), format!("{ours}")))
                .or_insert(0) += 1;
        }
    }

    fn pct(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            100.0 * self.agree as f64 / self.total as f64
        }
    }

    fn report(&self, label: &str, top: usize) {
        eprintln!(
            "{label}: {}/{} agree ({:.1}%)",
            self.agree,
            self.total,
            self.pct()
        );
        let mut diffs: Vec<_> = self.diffs.iter().collect();
        diffs.sort_by(|a, b| b.1.cmp(a.1));
        for ((bba, ours), count) in diffs.into_iter().take(top) {
            eprintln!("    bba {bba:<4} ours {ours:<4} ×{count}");
        }
    }
}

const VULS: [AbsoluteVulnerability; 4] = [
    AbsoluteVulnerability::NONE,
    AbsoluteVulnerability::NS,
    AbsoluteVulnerability::EW,
    AbsoluteVulnerability::ALL,
];

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
    let wj = match Wj::load(&path) {
        Ok(wj) => wj,
        Err(error) => {
            eprintln!(
                "could not load EPBot native lib at `{path}`: {error}\n\
                 Set BBA_LIB to the libEPBot.so path (it is proprietary + git-ignored)."
            );
            std::process::exit(1);
        }
    };
    let ours = polish_club().against(Family::NATURAL);

    let failures = run_fixtures(&ours, &wj);

    let mut openings = Tally::default();
    let mut overlap_openings = Tally::default();
    let mut responses = Tally::default();

    let mut rng = StdRng::seed_from_u64(args.seed);
    for index in 0..args.count {
        let dealer = Seat::ALL[index % 4];
        let vul = VULS[index % 4];
        let deal = full_deal(&mut rng);

        // Opening: BBA opens the dealer's hand; we answer the same question.
        let opener = deal[dealer];
        let ovul = relative(vul, dealer);
        let bba_open = wj.classify(opener, ovul, &[]);
        let our_open = our_call(&ours, opener, ovul, &[]);
        openings.record(bba_open, our_open);
        if is_overlap_opening(bba_open) {
            overlap_openings.record(bba_open, our_open);
        }

        // Uncontested first response: LHO passes, partner responds.  Only
        // meaningful when BBA actually opened a bid (not a pass).
        if let Call::Bid(_) = bba_open {
            let responder_seat = Seat::ALL[(dealer as usize + 2) % 4];
            let responder = deal[responder_seat];
            let rvul = relative(vul, responder_seat);
            let auction = [bba_open, Call::Pass];
            let bba_resp = wj.classify(responder, rvul, &auction);
            let our_resp = our_call(&ours, responder, rvul, &auction);
            responses.record(bba_resp, our_resp);
        }
    }

    eprintln!("\n=== Agreement vs BBA WJ over {} boards ===", args.count);
    openings.report("openings (all)", args.top);
    overlap_openings.report("openings (overlap 1-level + 2♦)", args.top);
    responses.report("first responses (uncontested)", args.top);
    eprintln!(
        "\nNote: Strawberry diverges from generic WJ on 2♣/2♥/2♠/2NT and the response\n\
         conventions by design; the disagreement lists above are the authoring TODO."
    );

    if failures > 0 {
        eprintln!(
            "\n{failures} curated fixture(s) FAILED — our system did not open the textbook call."
        );
        std::process::exit(1);
    }
    Ok(())
}
