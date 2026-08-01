//! Probe the real EPBot engine (system 0, 2/1 GF) for its **Kickback** rule set:
//! with a fit in suit X, when is the keycard ask the cheapest bid above 4-of-X
//! (`Kickback 1430 = 1`) and when does it stay 4NT?  The trigger conditions were
//! read from the decompiled `EPBot64.dll` (`get_kolor_kickback`,
//! `interpretuj_blackwooda`, `determine_pytanie_o_asy`); this probe validates
//! them against the NativeAOT `.so` we actually drive, reading each call's
//! systemic label via `epbot_get_info_meaning` (buffer ABI recovered from the
//! deleted `bba-wj-reference` harvest, commit `7d82918`; semantics pinned from
//! the wasm shim: the index is a SEAT, whose slot holds that seat's *latest*
//! call's label, refreshed by `set_bid` itself — so we read after every call).
//!
//! Each case runs twice — `Kickback 1430` off and on — so the flag-bites control
//! is built in.  Run from the repo root (EPBot segfaults under `cargo test`
//! threads, so this is an example, main thread only):
//!
//! ```text
//! cargo run --release --example probe-bba-kickback          # smoke: ask + answer
//! ```

use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};

const DEFAULT_LIB: &str = "vendor/bba/Native-libraries/linux/x64/libEPBot.so";

type CreateFn = unsafe extern "C" fn() -> *mut c_void;
type DestroyFn = unsafe extern "C" fn(*mut c_void);
type SetSystemFn = unsafe extern "C" fn(*mut c_void, c_int, c_int);
type NewHandFn =
    unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int, c_int, c_int, c_int);
type SetBidFn = unsafe extern "C" fn(*mut c_void, c_int, c_int, *const c_char);
type GetBidFn = unsafe extern "C" fn(*mut c_void) -> c_int;
type SetConvFn = unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int) -> c_int;
type GetConvFn = unsafe extern "C" fn(*mut c_void, c_int, *const c_char) -> c_int;
type GetInfoFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_char, c_int) -> c_int;

/// Decode EPBot's bid code (0/1/2 = Pass/X/XX; bid = 5 + (level-1)*5 + strain).
fn decode(code: c_int) -> String {
    const STRAIN: [&str; 5] = ["♣", "♦", "♥", "♠", "NT"];
    match code {
        0 => "Pass".into(),
        1 => "X".into(),
        2 => "XX".into(),
        5..=39 => {
            let i = code - 5;
            format!("{}{}", i / 5 + 1, STRAIN[(i % 5) as usize])
        }
        other => format!("?{other}"),
    }
}

/// Join four suit holdings into EPBot's C\nD\nH\nS hand string (13 cards total).
fn suits(spades: &str, hearts: &str, diamonds: &str, clubs: &str) -> CString {
    let n = spades.len() + hearts.len() + diamonds.len() + clubs.len();
    assert_eq!(
        n, 13,
        "hand {spades}.{hearts}.{diamonds}.{clubs} has {n} cards"
    );
    CString::new(format!("{clubs}\n{diamonds}\n{hearts}\n{spades}")).unwrap()
}

struct Bot {
    _lib: Library,
    create: CreateFn,
    destroy: DestroyFn,
    set_system: SetSystemFn,
    new_hand: NewHandFn,
    set_bid: SetBidFn,
    get_bid: GetBidFn,
    set_conv: SetConvFn,
    get_conv: GetConvFn,
    get_meaning: GetInfoFn,
}

/// One probe's outcome: the actor's call and the systemic label of every
/// auction position including that call.
struct Probe {
    call: String,
    meanings: Vec<String>,
}

impl Bot {
    fn load() -> anyhow::Result<Self> {
        let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
        // SAFETY: loading the trusted vendored engine; `_lib` outlives the
        // copied `Copy` function pointers.
        let lib = unsafe { Library::new(&path) }?;
        unsafe {
            Ok(Self {
                create: *lib.get::<CreateFn>(b"epbot_create\0")?,
                destroy: *lib.get::<DestroyFn>(b"epbot_destroy\0")?,
                set_system: *lib.get::<SetSystemFn>(b"epbot_set_system_type\0")?,
                new_hand: *lib.get::<NewHandFn>(b"epbot_new_hand\0")?,
                set_bid: *lib.get::<SetBidFn>(b"epbot_set_bid\0")?,
                get_bid: *lib.get::<GetBidFn>(b"epbot_get_bid\0")?,
                set_conv: *lib.get::<SetConvFn>(b"epbot_set_conventions\0")?,
                get_conv: *lib.get::<GetConvFn>(b"epbot_get_conventions\0")?,
                get_meaning: *lib.get::<GetInfoFn>(b"epbot_get_info_meaning\0")?,
                _lib: lib,
            })
        }
    }

    /// `actor` holds `hand`; `prefix` is replayed (dealer at position 0), the
    /// convention overrides are applied to all four seats *after* `set_system`
    /// (which would clobber them), and the actor's call is read, appended, and
    /// interpreted along with the whole auction.
    fn probe(
        &self,
        actor: c_int,
        hand: &CString,
        prefix: &[c_int],
        convs: &[(&str, bool)],
    ) -> Probe {
        let empty = c"".as_ptr();
        // SAFETY: fresh bot per probe, destroyed before returning; the bidding
        // ABI is the long-confirmed S.1 surface, the meaning ABI the recovered
        // `bba-wj-reference` one (own stack buffer, capacity passed in bytes).
        unsafe {
            let bot = (self.create)();
            assert!(!bot.is_null(), "epbot_create returned null");
            for seat in 0..4 {
                (self.set_system)(bot, seat, 0);
            }
            // Conventions are per SIDE, not per seat: the engine's `cc` array
            // has two entries (decompile: `cc = new TYP_SYSTEM[2]`), so 2/3
            // throw (-2).  Set both sides and read each toggle back to verify.
            for &(name, on) in convs {
                let name = CString::new(name).unwrap();
                for side in 0..2 {
                    (self.set_conv)(bot, side, name.as_ptr(), on as c_int);
                    let got = (self.get_conv)(bot, side, name.as_ptr());
                    assert_eq!(got, on as c_int, "toggle did not stick on side {side}");
                }
            }
            (self.new_hand)(bot, actor, hand.as_ptr(), 0, 0, 0, 0);
            // `set_bid` interprets each call as it arrives; a seat's meaning
            // slot holds its LATEST call's label, so read right after each.
            let read_meaning = |seat: c_int| {
                let mut buf = [0_u8; 1024];
                (self.get_meaning)(bot, seat, buf.as_mut_ptr().cast::<c_char>(), 1024);
                let end = buf.iter().position(|&b| b == 0).unwrap_or(0);
                String::from_utf8_lossy(&buf[..end]).into_owned()
            };
            let mut meanings = Vec::with_capacity(prefix.len() + 1);
            for (index, &code) in prefix.iter().enumerate() {
                let seat = (index % 4) as c_int;
                (self.set_bid)(bot, seat, code, empty);
                meanings.push(read_meaning(seat));
            }
            let code = (self.get_bid)(bot);
            (self.set_bid)(bot, actor, code, empty);
            meanings.push(read_meaning(actor));
            (self.destroy)(bot);
            Probe {
                call: decode(code),
                meanings,
            }
        }
    }
}

/// One bidding situation to probe.
struct Case {
    label: &'static str,
    actor: c_int,
    prefix: &'static [c_int],
    hand: (&'static str, &'static str, &'static str, &'static str),
    convs: &'static [(&'static str, bool)],
    expect_call: &'static str,
    expect_label: Option<(usize, &'static str)>,
}

// Bid codes: Pass=0, X=1; bid = 5 + (level-1)*5 + strain (♣0 ♦1 ♥2 ♠3 NT4).
const P: c_int = 0;
const X: c_int = 1;
const B1C: c_int = 5;
const B1D: c_int = 6;
const B1H: c_int = 7;
const B1S: c_int = 8;
const B2D: c_int = 11;
const B2S: c_int = 13;
const B2N: c_int = 14;
const B3C: c_int = 15;
const B3D: c_int = 16;
const B3H: c_int = 17;
const B4C: c_int = 20;
const B4D: c_int = 21;
const B4H: c_int = 22;
const B4S: c_int = 23;
const B4N: c_int = 24;
const B5C: c_int = 25;
const B5D: c_int = 26;
const B5H: c_int = 27;
const B5S: c_int = 28;
const B5N: c_int = 29;

fn main() -> anyhow::Result<()> {
    let bot = Bot::load()?;
    let cases = [
        Case {
            label: "♦ ask labeled; answer 1 kc → step 1",
            actor: 2,
            prefix: &[B1D, P, B3D, P, B4H, P],
            hand: ("A54", "876", "QJ854", "K2"),
            convs: &[("Kickback 1430", true)],
            expect_call: "4♠",
            expect_label: Some((4, "Kickback 1430, for !D")),
        },
        Case {
            label: "answer 0 kc → step 2",
            actor: 2,
            prefix: &[B1D, P, B3D, P, B4H, P],
            hand: ("KQ4", "876", "QJ854", "Q2"),
            convs: &[("Kickback 1430", true)],
            expect_call: "4NT",
            expect_label: Some((6, "A=0/5 or 3/5")),
        },
        Case {
            label: "answer 2 kc no Q → step 3",
            actor: 2,
            prefix: &[B1D, P, B3D, P, B4H, P],
            hand: ("Q54", "876", "KJ854", "A2"),
            convs: &[("Kickback 1430", true)],
            expect_call: "5♣",
            expect_label: Some((6, "A=2")),
        },
        Case {
            label: "answer 2 kc + ♦Q → step 4",
            actor: 2,
            prefix: &[B1D, P, B3D, P, B4H, P],
            hand: ("Q54", "87", "KQJ85", "A32"),
            convs: &[("Kickback 1430", true)],
            expect_call: "5♦",
            expect_label: Some((6, "A=2")),
        },
        Case {
            label: "2 kc, no Q, 10-card fit counts as Q",
            actor: 2,
            prefix: &[B1D, P, B3D, P, B4H, P],
            hand: ("5", "876", "KJ87542", "A2"),
            convs: &[("Kickback 1430", true)],
            expect_call: "5♦",
            expect_label: Some((6, "A=2")),
        },
        Case {
            label: "control: flag off, 4♥ reads Splinter",
            actor: 2,
            prefix: &[B1D, P, B3D, P, B4H, P],
            hand: ("A54", "876", "QJ854", "K2"),
            convs: &[("Kickback 1430", false)],
            expect_call: "",
            expect_label: Some((4, "Splinter")),
        },
        Case {
            label: "0314 flavor: 1 kc → step 2",
            actor: 2,
            prefix: &[B1D, P, B3D, P, B4H, P],
            hand: ("A54", "876", "QJ854", "K2"),
            convs: &[("Kickback 0314", true)],
            expect_call: "4NT",
            expect_label: Some((4, "Kickback 0314")),
        },
        Case {
            label: "0123 flavor: 1 kc → step 2",
            actor: 2,
            prefix: &[B1D, P, B3D, P, B4H, P],
            hand: ("A54", "876", "QJ854", "K2"),
            convs: &[("Kickback 0123", true)],
            expect_call: "4NT",
            expect_label: Some((4, "Kickback 0123")),
        },
        Case {
            label: "4NT residual stays RKCB (♦ agreed, KB on)",
            actor: 2,
            prefix: &[B1D, P, B3D, P, B4N, P],
            hand: ("A54", "876", "QJ854", "K2"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: Some((4, "Blackwood")),
        },
        Case {
            label: "guard: responder showed hearts → 4♥ not an ask",
            actor: 0,
            prefix: &[B1D, P, B1H, P, B3D, P, B4H, P],
            hand: ("K5", "Q32", "AKQ63", "Q32"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "guard: opener showed hearts (1♥ then ♦ raise)",
            actor: 0,
            prefix: &[B1H, P, B2D, P, B3D, P, B4H, P],
            hand: ("87", "AQJ85", "KQ54", "Q3"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "retro/splinter collision: 2/1 then 4♥ jump",
            actor: 0,
            prefix: &[B1S, P, B2D, P, B2S, P, B4H, P],
            hand: ("AQJ854", "87", "K5", "Q32"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "cue chain: 4♣-4♦ cues then 4♥",
            actor: 2,
            prefix: &[B1D, P, B3D, P, B4C, P, B4D, P, B4H, P],
            hand: ("A54", "876", "QJ854", "K2"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "♣ ask labeled; 1 kc → 4♥",
            actor: 2,
            prefix: &[B1C, P, B3C, P, B4D, P],
            hand: ("A54", "876", "872", "QJ85"),
            convs: &[("Kickback 1430", true)],
            expect_call: "4♥",
            expect_label: Some((4, "Kickback 1430, for !C")),
        },
        Case {
            label: "0 kc → 4♠",
            actor: 2,
            prefix: &[B1C, P, B3C, P, B4D, P],
            hand: ("KQ5", "876", "Q72", "QJ85"),
            convs: &[("Kickback 1430", true)],
            expect_call: "4♠",
            expect_label: Some((6, "A=0/5 or 3/5")),
        },
        Case {
            label: "2 kc + ♣Q → 5♣ (veto below game)",
            actor: 2,
            prefix: &[B1C, P, B3C, P, B4D, P],
            hand: ("A54", "87", "87", "KQJ852"),
            convs: &[("Kickback 1430", true)],
            expect_call: "5♣",
            expect_label: Some((6, "A=2")),
        },
        Case {
            label: "4NT residual (♣ agreed, KB on)",
            actor: 2,
            prefix: &[B1C, P, B3C, P, B4N, P],
            hand: ("A54", "876", "872", "QJ85"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: Some((4, "Blackwood")),
        },
        Case {
            label: "guard: diamonds shown by responder",
            actor: 0,
            prefix: &[B1C, P, B1D, P, B3C, P, B4D, P],
            hand: ("K5", "A32", "Q32", "AQJ63"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "♥ ask labeled; 1 kc → 4NT",
            actor: 2,
            prefix: &[B1H, P, B3H, P, B4S, P],
            hand: ("876", "QJ85", "A54", "872"),
            convs: &[("Kickback 1430", true)],
            expect_call: "4NT",
            expect_label: Some((4, "Kickback 1430, for !H")),
        },
        Case {
            label: "2 kc + ♥Q → 5♥",
            actor: 2,
            prefix: &[B1H, P, B3H, P, B4S, P],
            hand: ("87", "KQ854", "A54", "872"),
            convs: &[("Kickback 1430", true)],
            expect_call: "5♥",
            expect_label: Some((6, "A=2")),
        },
        Case {
            label: "control: flag off, 4♠ over 3♥",
            actor: 2,
            prefix: &[B1H, P, B3H, P, B4S, P],
            hand: ("876", "QJ85", "A54", "872"),
            convs: &[("Kickback 1430", false)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "guard: responder showed spades first",
            actor: 0,
            prefix: &[B1H, P, B1S, P, B3H, P, B4S, P],
            hand: ("K5", "AQJ63", "Q32", "A32"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "queen ask; no ♥Q held but own 4 + probable 6 = 10-fit counts it",
            actor: 2,
            prefix: &[B1H, P, B3H, P, B4S, P, B4N, P, B5C, P],
            hand: ("876", "J985", "A54", "872"),
            convs: &[("Kickback 1430", true)],
            expect_call: "5NT",
            expect_label: Some((8, "queen")),
        },
        Case {
            label: "queen ask; ♥Q + ♣K → show king",
            actor: 2,
            prefix: &[B1H, P, B3H, P, B4S, P, B4N, P, B5C, P],
            hand: ("87", "Q985", "A54", "K872"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: Some((8, "queen")),
        },
        Case {
            label: "5♦ after the 4NT answer is NATURAL (the king ask lives at 5NT)",
            actor: 2,
            prefix: &[B1H, P, B3H, P, B4S, P, B4N, P, B5D, P],
            hand: ("K76", "QJ85", "A54", "872"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: Some((8, "bidable suit")),
        },
        Case {
            label: "Jacoby 2NT route: 4♠ fed after 1♥-2NT",
            actor: 2,
            prefix: &[B1H, P, B2N, P, B4S, P],
            hand: ("876", "QJ85", "A54", "872"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "their 1♥ overcall, raise agreed ♦: is 4♥ still the ask?",
            actor: 2,
            prefix: &[B1D, B1H, B3D, P, B4H, P],
            hand: ("A54", "87", "QJ854", "K32"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "X of the 4♥ ask: answer under the double",
            actor: 2,
            prefix: &[B1D, B1H, B3D, P, B4H, X],
            hand: ("A54", "87", "QJ854", "K32"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "5♥ over the ask: parity answers?",
            actor: 2,
            prefix: &[B1D, B1H, B3D, P, B4H, B5H],
            hand: ("A54", "87", "QJ854", "K32"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "♦ monster over limit raise, KB on",
            actor: 0,
            prefix: &[B1D, P, B3D, P],
            hand: ("AK3", "A2", "AKQ632", "K4"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "same, KB off (baseline)",
            actor: 0,
            prefix: &[B1D, P, B3D, P],
            hand: ("AK3", "A2", "AKQ632", "K4"),
            convs: &[("Kickback 1430", false)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "cue chain complete, ask due: 4♣-4♦ done, KB on",
            actor: 0,
            prefix: &[B1D, P, B3D, P, B4C, P, B4D, P],
            hand: ("AK3", "A2", "AKQ632", "K4"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "same, KB off",
            actor: 0,
            prefix: &[B1D, P, B3D, P, B4C, P, B4D, P],
            hand: ("AK3", "A2", "AKQ632", "K4"),
            convs: &[("Kickback 1430", false)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "Jacoby, opener monster, after 3♥-4♦ cue, KB on",
            actor: 0,
            prefix: &[B1H, P, B2N, P, B3H, P, B4D, P],
            hand: ("A2", "AKQJ65", "KQ4", "A3"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "same, KB off",
            actor: 0,
            prefix: &[B1H, P, B2N, P, B3H, P, B4D, P],
            hand: ("A2", "AKQJ65", "KQ4", "A3"),
            convs: &[("Kickback 1430", false)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "♣ freak over limit raise, KB on",
            actor: 0,
            prefix: &[B1C, P, B3C, P],
            hand: ("A2", "A2", "A2", "AKQJ632"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "same, KB off",
            actor: 0,
            prefix: &[B1C, P, B3C, P],
            hand: ("A2", "A2", "A2", "AKQJ632"),
            convs: &[("Kickback 1430", false)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "king ask stays 5NT (default King ask by 5NT = 1); ♠K held",
            actor: 2,
            prefix: &[B1H, P, B3H, P, B4S, P, B4N, P, B5N, P],
            hand: ("K76", "QJ85", "A54", "872"),
            convs: &[("Kickback 1430", true)],
            expect_call: "6♦",
            expect_label: Some((8, "King ask by 5NT")),
        },
        // --- `King ask by available bid` UNDER kickback ------------------
        // Hearts agreed by 3♥, kickback ask 4♠, answer 4NT (1-or-4), queen ask
        // 5♣, and partner's 5♠ shows the queen plus the ♠K.  The asker is on
        // lead to explore seven.  Off the row the king ask must climb to 5NT;
        // on it, the "2nd available bid" form is free to sit lower — the whole
        // point of relocating the ask.  Same hand both ways, so the row bites
        // or it does not.
        Case {
            label: "kickback + king-ask-by-5NT (control): asker explores seven",
            actor: 0,
            prefix: &[B1H, P, B3H, P, B4S, P, B4N, P, B5C, P, B5S, P],
            hand: ("A2", "AKJ85", "AQ2", "KQ3"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "kickback + King ask by available bid: asker explores seven",
            actor: 0,
            prefix: &[B1H, P, B3H, P, B4S, P, B4N, P, B5C, P, B5S, P],
            hand: ("A2", "AKJ85", "AQ2", "KQ3"),
            convs: &[
                ("Kickback 1430", true),
                ("King ask by available bid", true),
                ("King ask by 5NT", false),
            ],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "kickback + BOTH king-ask rows on: which one wins?",
            actor: 0,
            prefix: &[B1H, P, B3H, P, B4S, P, B4N, P, B5C, P, B5S, P],
            hand: ("A2", "AKJ85", "AQ2", "KQ3"),
            convs: &[
                ("Kickback 1430", true),
                ("King ask by available bid", true),
                ("King ask by 5NT", true),
            ],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "kickback + NEITHER king-ask row: the floor of the comparison",
            actor: 0,
            prefix: &[B1H, P, B3H, P, B4S, P, B4N, P, B5C, P, B5S, P],
            hand: ("A2", "AKJ85", "AQ2", "KQ3"),
            convs: &[
                ("Kickback 1430", true),
                ("King ask by available bid", false),
                ("King ask by 5NT", false),
            ],
            expect_call: "",
            expect_label: None,
        },
        // Diamonds agreed: kickback ask 4♥, answer 4♠ (1-or-4), queen ask
        // 5♣, partner's 5♥ = queen plus the ♥K.  Here the "2nd available bid"
        // king ask would be 5♠ — a full three steps under six of diamonds, the
        // most room the row can ever have.  If it does not fire here it does
        // not fire.
        Case {
            label: "minor kickback, 5NT row: asker explores seven",
            actor: 0,
            prefix: &[B1D, P, B3D, P, B4H, P, B4S, P, B5C, P, B5H, P],
            hand: ("A2", "AQ2", "AKJ85", "KQ3"),
            convs: &[("Kickback 1430", true), ("King ask by 5NT", true)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "minor kickback, available-bid row: asker explores seven",
            actor: 0,
            prefix: &[B1D, P, B3D, P, B4H, P, B4S, P, B5C, P, B5H, P],
            hand: ("A2", "AQ2", "AKJ85", "KQ3"),
            convs: &[
                ("Kickback 1430", true),
                ("King ask by available bid", true),
                ("King ask by 5NT", false),
            ],
            expect_call: "",
            expect_label: None,
        },
        // --- the 5NT king ladder, enumerated ---------------------------
        // Same ask, same shape, only the side kings move.  §3 recorded a single
        // point (6♦ = K=1); these four read the whole ladder off the engine so
        // the step rule is observed rather than extrapolated.
        Case {
            label: "king ladder: 0 side kings",
            actor: 2,
            prefix: &[B1H, P, B3H, P, B4S, P, B4N, P, B5N, P],
            hand: ("Q76", "QJ85", "A54", "872"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "king ladder: 1 side king (♠K)",
            actor: 2,
            prefix: &[B1H, P, B3H, P, B4S, P, B4N, P, B5N, P],
            hand: ("K76", "QJ85", "A54", "872"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "king ladder: 2 side kings (♠K ♦K)",
            actor: 2,
            prefix: &[B1H, P, B3H, P, B4S, P, B4N, P, B5N, P],
            hand: ("K76", "QJ85", "K54", "872"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "king ladder: 3 side kings (♠K ♦K ♣K)",
            actor: 2,
            prefix: &[B1H, P, B3H, P, B4S, P, B4N, P, B5N, P],
            hand: ("K76", "QJ85", "K54", "K72"),
            convs: &[("Kickback 1430", true)],
            expect_call: "",
            expect_label: None,
        },
        Case {
            label: "queen ask, 3 trumps (own 3 + probable 6 < 10) → no Q, signoff",
            actor: 2,
            prefix: &[B1H, P, B3H, P, B4S, P, B4N, P, B5C, P],
            hand: ("87", "J98", "A542", "9873"),
            convs: &[("Kickback 1430", true)],
            expect_call: "5♥",
            expect_label: Some((8, "queen")),
        },
    ];

    let mut pass = 0;
    let mut fail = 0;
    let mut exploratory = 0;
    for case in &cases {
        let (s, h, d, c) = case.hand;
        let probe = bot.probe(case.actor, &suits(s, h, d, c), case.prefix, case.convs);

        println!("\n{}  ♠{s} ♥{h} ♦{d} ♣{c}", case.label);
        if case.expect_call.is_empty() {
            exploratory += 1;
            println!(
                "  call  OBS: expected <exploratory> | observed {}",
                probe.call
            );
        } else if probe.call == case.expect_call {
            pass += 1;
            println!(
                "  call PASS: expected {} | observed {}",
                case.expect_call, probe.call
            );
        } else {
            fail += 1;
            println!(
                "  call FAIL: expected {} | observed {}",
                case.expect_call, probe.call
            );
        }

        match case.expect_label {
            Some((index, expected)) => {
                let observed = probe
                    .meanings
                    .get(index)
                    .map_or("<missing>", String::as_str);
                if observed.contains(expected) {
                    pass += 1;
                    println!(
                        "  label PASS: [{index}] expected substring {expected:?} | observed {observed:?}"
                    );
                } else {
                    fail += 1;
                    println!(
                        "  label FAIL: [{index}] expected substring {expected:?} | observed {observed:?}"
                    );
                }
            }
            None => {
                exploratory += 1;
                if let Some(index) = case.prefix.iter().rposition(|&call| call != P) {
                    let observed = probe
                        .meanings
                        .get(index)
                        .map_or("<missing>", String::as_str);
                    println!(
                        "  label OBS: no expectation | last non-pass [{index}] observed {observed:?}"
                    );
                } else {
                    println!("  label OBS: no expectation | no non-pass prefix call");
                }
            }
        }

        println!("  meanings:");
        for (index, meaning) in probe.meanings.iter().enumerate() {
            if meaning.is_empty() {
                println!("    [{index}] <empty>");
            } else {
                println!("    [{index}] {meaning}");
            }
        }
    }
    println!("\n{pass} pass / {fail} fail / {exploratory} exploratory");
    Ok(())
}
