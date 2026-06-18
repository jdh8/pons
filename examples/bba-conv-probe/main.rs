//! Side-track S spike — confirm EPBot's *named-convention* ABI so we can flip a
//! single toggle (e.g. "Rubensohl after 1m") for a clean BBA-vs-BBA A/B.
//!
//! The FFI's `epbot_set_system_type` only selects whole systems; it cannot
//! isolate one convention.  But the engine also exports a named-convention API.
//! Signatures recovered from `objdump` (register usage) of `libEPBot.so`:
//!   epbot_convention_index(bot, char* name) -> int        ; -1 on bad bot
//!   epbot_get_conventions(bot, int idx, char* name) -> int ; 0/1, -1 on bad bot
//!   epbot_set_conventions(bot, int idx, char* name, int on) -> int
//!
//! GROUND-TRUTH CHECK: after `set_system_type(_, 0)` (= "2/1 Game Force"), read
//! every boolean toggle named in `vendor/bba/21GF.bbsa` back through this API and
//! compare to the file.  If they match, the addressing is right and we can trust
//! the flip.  Then flip "Rubensohl after 1m" off, re-read, and bid one diagnostic
//! responder auction on/off to *see* the convention change behaviour.

use libloading::{Library, Symbol};
use std::ffi::{CString, c_char, c_int, c_void};

const DEFAULT_LIB: &str = "vendor/bba/Native-libraries/linux/x64/libEPBot.so";
const SYSTEM_2_OVER_1: c_int = 0;
const TOGGLE: &str = "Rubensohl after 1m";

type CreateFn = unsafe extern "C" fn() -> *mut c_void;
type DestroyFn = unsafe extern "C" fn(*mut c_void);
type SetSystemFn = unsafe extern "C" fn(*mut c_void, c_int, c_int);
type ConvIndexFn = unsafe extern "C" fn(*mut c_void, *const c_char) -> c_int;
type GetConvFn = unsafe extern "C" fn(*mut c_void, c_int, *const c_char) -> c_int;
type SetConvFn = unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int) -> c_int;
type NewHandFn =
    unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int, c_int, c_int, c_int);
type SetBidFn = unsafe extern "C" fn(*mut c_void, c_int, c_int, *const c_char);
type GetBidFn = unsafe extern "C" fn(*mut c_void) -> c_int;

fn main() -> anyhow::Result<()> {
    let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
    let lib = unsafe { Library::new(&path) }?;
    let (create, destroy, set_system, conv_index, get_conv, set_conv, new_hand, set_bid, get_bid) = unsafe {
        (
            *lib.get::<CreateFn>(b"epbot_create\0")?,
            *lib.get::<DestroyFn>(b"epbot_destroy\0")?,
            *lib.get::<SetSystemFn>(b"epbot_set_system_type\0")?,
            *lib.get::<ConvIndexFn>(b"epbot_convention_index\0")?,
            *lib.get::<GetConvFn>(b"epbot_get_conventions\0")?,
            *lib.get::<SetConvFn>(b"epbot_set_conventions\0")?,
            *lib.get::<NewHandFn>(b"epbot_new_hand\0")?,
            *lib.get::<SetBidFn>(b"epbot_set_bid\0")?,
            *lib.get::<GetBidFn>(b"epbot_get_bid\0")?,
        )
    };

    let cstr = |s: &str| CString::new(s).unwrap();
    // Conventions are addressed per-SEAT + name, mirroring set_system_type:
    //   get_conventions(bot, seat, name) -> 0/1 ; set_conventions(bot, seat, name, on)
    let get_seat = |bot: *mut c_void, seat: c_int, name: &str| -> c_int {
        unsafe { get_conv(bot, seat, cstr(name).as_ptr()) }
    };
    let set_seat = |bot: *mut c_void, seat: c_int, name: &str, on: c_int| {
        unsafe { set_conv(bot, seat, cstr(name).as_ptr(), on) };
    };
    let _ = conv_index; // name->index mapping; unused now that get/set take the name

    // --- Ground-truth check against 21GF.bbsa boolean toggles (seat 0) ---
    let bbsa = std::fs::read_to_string("vendor/bba/21GF.bbsa")?;
    let bot = unsafe { create() };
    assert!(!bot.is_null());
    unsafe { set_system(bot, 0, SYSTEM_2_OVER_1) };

    let (mut ok, mut bad, mut skip) = (0u32, 0u32, 0u32);
    let mut mismatches = Vec::new();
    for line in bbsa.lines() {
        let Some((name, val)) = line.rsplit_once(" = ") else {
            continue;
        };
        let name = name.trim();
        let want = match val.trim() {
            "0" => 0,
            "1" => 1,
            _ => continue, // non-boolean parameter line
        };
        let got = get_seat(bot, 0, name);
        if got < 0 {
            skip += 1; // not a recognized per-seat convention (a system parameter)
            continue;
        }
        if got == want {
            ok += 1;
        } else {
            bad += 1;
            mismatches.push(format!("  {name}: file={want} engine={got}"));
        }
    }
    println!("== ground-truth vs 21GF.bbsa (seat 0, system 0) ==");
    println!("  matched={ok}  mismatched={bad}  skipped(not a convention)={skip}");
    for m in mismatches.iter().take(20) {
        println!("{m}");
    }

    // --- Confirm set_conventions actually WRITES: default -> on -> off ---
    let before = get_seat(bot, 0, TOGGLE);
    set_seat(bot, 0, TOGGLE, 1);
    let on = get_seat(bot, 0, TOGGLE);
    set_seat(bot, 0, TOGGLE, 0);
    let off = get_seat(bot, 0, TOGGLE);
    println!("\n== '{TOGGLE}' set-read round-trip (seat 0) ==");
    println!("  system-0 default = {before}  (engine baseline; 21GF.bbsa file = 1)");
    println!("  after set ON     = {on}   (expect 1)");
    println!("  after set OFF    = {off}   (expect 0)");
    unsafe { destroy(bot) };

    // --- Does get_bid actually consult the per-seat flag? ---
    // For each (convention, auction, responder hand): compute the actor's call
    // with the convention ON vs OFF (set on our seats 0 & 2).  Bid codes:
    // Pass=0; 1C=5,1D=6,1H=7,1S=8,1NT=9.  Each holding is char-counted, 13 total.
    // "Stayman"/"Texas" are CONTROLS known to change bidding; if they diverge but
    // the target does not, the target is inert in the FFI engine, not a pipeline bug.
    let trial = |conv: &str, auction: &[c_int], suits: [&str; 4]| -> (c_int, c_int) {
        let actor = auction.len() as c_int;
        let bid = |on: c_int| -> c_int {
            let bot = unsafe { create() };
            unsafe {
                for seat in 0..4 {
                    set_system(bot, seat, SYSTEM_2_OVER_1);
                }
                set_seat(bot, 0, conv, on); // opener
                set_seat(bot, 2, conv, on); // responder (the actor)
                let joined = cstr(&suits.join("\n"));
                new_hand(bot, actor, joined.as_ptr(), 0, 0, 0, 0);
                let empty = cstr("");
                for (i, &code) in auction.iter().enumerate() {
                    set_bid(bot, (i % 4) as c_int, code, empty.as_ptr());
                }
                let code = get_bid(bot);
                destroy(bot);
                code
            }
        };
        (bid(1), bid(0))
    };

    println!("\n== get_bid with convention ON vs OFF (<< = diverges) ==");
    // bid codes: 2C=10,2D=11,2H=12,2S=13,2NT=14
    let trials: [(&str, &str, &[c_int], [&str; 4]); 10] = [
        // CONTROLS — known live conventions
        (
            "Stayman",
            "1N-(P)-? 4-4 maj 13",
            &[9, 0],
            ["32", "K32", "KJ32", "AQ32"],
        ),
        (
            "Texas",
            "1N-(P)-? 6S 10",
            &[9, 0],
            ["432", "K2", "32", "AKJ982"],
        ),
        (
            "Jacoby 2NT",
            "1H-(P)-? GF 4H raise",
            &[7, 0],
            ["A32", "K32", "KQ32", "A2"],
        ),
        // TARGET — Rubensohl after 1m, 1-level overcall
        (
            TOGGLE,
            "1D-(1S)-? GF 6C",
            &[6, 8],
            ["AQJ987", "42", "KJ4", "A2"],
        ),
        (
            TOGGLE,
            "1D-(1S)-? bal 12 inv",
            &[6, 8],
            ["KJ4", "Q32", "K54", "QJ32"],
        ),
        (
            TOGGLE,
            "1C-(1S)-? bal 12 inv",
            &[5, 8],
            ["KJ4", "Q32", "K54", "QJ32"],
        ),
        // TARGET — 2-level overcall (the literature's canonical trigger)
        (
            TOGGLE,
            "1D-(2C)-? GF 6H",
            &[6, 10],
            ["43", "K2", "AKQJ87", "A2"],
        ),
        (
            TOGGLE,
            "1D-(2H)-? GF 6C",
            &[6, 12],
            ["AKQ987", "K2", "43", "A32"],
        ),
        (
            TOGGLE,
            "1D-(2H)-? GF D-raise",
            &[6, 12],
            ["A2", "KQ982", "43", "AJ3"],
        ),
        (
            TOGGLE,
            "1C-(2H)-? GF 6D",
            &[5, 12],
            ["A2", "AKQ987", "43", "KJ3"],
        ),
    ];
    for (conv, label, auction, suits) in trials {
        let (on, off) = trial(conv, auction, suits);
        let mark = if on == off { "  " } else { "<<" };
        println!(
            "  {label:<22} [{conv:<19}] ON={:<4} OFF={:<4} {mark}",
            decode(on),
            decode(off)
        );
    }
    Ok(())
}

fn decode(code: c_int) -> String {
    match code {
        0 => "Pass".into(),
        1 => "X".into(),
        2 => "XX".into(),
        n if (5..=39).contains(&n) => {
            let idx = n - 5;
            format!(
                "{}{}",
                idx / 5 + 1,
                ["C", "D", "H", "S", "NT"][(idx % 5) as usize]
            )
        }
        n => format!("?{n}"),
    }
}
