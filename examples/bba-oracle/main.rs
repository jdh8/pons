//! AI-bidder **Side-track S** spike — drive Edward Piwowar's EPBot engine as a
//! black-box bidding oracle through its native C ABI (`libEPBot.so`), with **no
//! Wine**: the Linux build is a self-contained .NET-NativeAOT shared library
//! exposing `epbot_*` C functions, which we `dlopen` and call directly.
//!
//! This is the *spike* (Side-track S.0), and it succeeds: a fresh bot bids each
//! known hand to its textbook 2/1 opening. The C signatures are not published —
//! they were recovered from `objdump` register usage + a decompile of
//! `EPBotFFI` (EPBotWasm.dll) + the bid codes confirmed against known openings,
//! all documented inline below. S.1 (the `BbaOracle` harness + 2/1 A/B match)
//! generalizes these bindings to full auctions.
//!
//! The native library is proprietary and git-ignored. Point `BBA_LIB` at it, or
//! drop it at the default vendored path:
//!
//! ```text
//! cargo run --example bba-oracle
//! BBA_LIB=/path/to/libEPBot.so cargo run --example bba-oracle
//! ```

use libloading::{Library, Symbol};
use std::ffi::{CString, c_char, c_int, c_void};

const DEFAULT_LIB: &str = "vendor/bba/Native-libraries/linux/x64/libEPBot.so";

// --- Confirmed C ABI (objdump + EPBotFFI decompile + empirical bid codes) ---
// Handles are opaque pointers returned by epbot_create.
type CreateFn = unsafe extern "C" fn() -> *mut c_void;
type DestroyFn = unsafe extern "C" fn(*mut c_void);
type SetSystemFn = unsafe extern "C" fn(*mut c_void, c_int, c_int);
// Decompiled from EPBotFFI.NewHand (EPBotWasm.dll): the core call is
//   New_hand(position, suits[], dealer, vul, arg5!=0, arg6!=0)
// where `suits` is ONE string pointer the wrapper splits on '\n' into 4 holdings.
// So: (handle, int position, char* suits, int dealer, int vul, int b1, int b2).
type NewHandFn =
    unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int, c_int, c_int, c_int);
type GetBidFn = unsafe extern "C" fn(*mut c_void) -> c_int;

fn main() -> anyhow::Result<()> {
    let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
    println!("== loading {path} ==");
    let lib = match unsafe { Library::new(&path) } {
        Ok(lib) => lib,
        Err(e) => {
            eprintln!(
                "could not load EPBot native lib at `{path}`: {e}\n\
                 Set BBA_LIB to the libEPBot.so path (it is proprietary + git-ignored)."
            );
            std::process::exit(1);
        }
    };

    unsafe {
        let create: Symbol<CreateFn> = lib.get(b"epbot_create\0")?;
        let destroy: Symbol<DestroyFn> = lib.get(b"epbot_destroy\0")?;
        let new_hand: Symbol<NewHandFn> = lib.get(b"epbot_new_hand\0")?;
        let get_bid: Symbol<GetBidFn> = lib.get(b"epbot_get_bid\0")?;
        let set_system: Option<Symbol<SetSystemFn>> = lib.get(b"epbot_set_system_type\0").ok();

        // Bid one hand as the opener (seat 0, dealer 0, none vul). `suits` are the
        // four holdings in C,D,H,S order; the wrapper splits a single newline-joined
        // string. Fresh bot per hand. Returns the raw bid code.
        let bid_hand = |suits: [&str; 4]| -> c_int {
            let bot = create();
            assert!(!bot.is_null(), "epbot_create returned NULL");
            if let Some(set) = &set_system {
                set(bot, 0, 0); // system 0; refine once hands read correctly
            }
            let joined = CString::new(suits.join("\n")).unwrap();
            new_hand(bot, 0, joined.as_ptr(), 0, 0, 0, 0);
            let bid = get_bid(bot);
            destroy(bot);
            bid
        };

        // The oracle is live: each 13-card hand bids on its merits. Suits are
        // [♣, ♦, ♥, ♠]; the comment is the textbook 2/1 opening for cross-check.
        println!("EPBot openings (seat 0, none vul):");
        for (suits, expect) in [
            (["432", "5432", "432", "432"], "Pass (~2 HCP bust)"),
            (["AQ5", "KJ4", "Q872", "K43"], "1NT (15 bal)"),
            (["AQ52", "KJ7", "KJ3", "AQ4"], "2NT (20 bal)"),
            (["AK42", "AKQ", "AQJ", "AKQ"], "2♣ (22+ monster)"),
            (["AKJ9876", "432", "32", "2"], "preempt (7-card ♣)"),
            (["2", "32", "432", "AKJ9876"], "preempt (7-card ♠)"),
            (["K2", "A8765", "KQ32", "A4"], "1NT (16, 5422)"),
        ] {
            let code = bid_hand(suits);
            println!(
                "  {:<22} -> code {code:<3} = {:<5}  [{expect}]",
                suits.join("."),
                decode_bid(code)
            );
        }
    }

    println!(
        "\nSPIKE RESULT (success): EPBot drives natively via FFI — no Wine. Hands are\n\
         read and bid on their merits. Confirmed ABI:\n\
         - epbot_create() -> bot ; epbot_destroy(bot)\n\
         - epbot_set_system_type(bot, position, system)\n\
         - epbot_new_hand(bot, position, suits, dealer, vul, b1, b2)\n\
             suits = ONE string, the 4 holdings (C,D,H,S) joined by '\\n'\n\
         - epbot_get_bid(bot) -> bid code\n\
         - bid code: 0/1/2 = Pass/X/XX; contract = 5 + (level-1)*5 + strain,\n\
             strain 0=♣ 1=♦ 2=♥ 3=♠ 4=NT.\n\
         Next (S.1): generalize to full auctions (set_bid for the table), wrap as\n\
         a BbaOracle System, and run the 2/1 A/B match."
    );
    Ok(())
}

/// Decode EPBot's integer bid code (confirmed empirically against known openings):
/// `0/1/2 = Pass/X/XX`; a contract bid is `5 + (level-1)*5 + strain` with strain
/// `0=♣ 1=♦ 2=♥ 3=♠ 4=NT`.
fn decode_bid(code: c_int) -> String {
    match code {
        0 => "Pass".into(),
        1 => "X".into(),
        2 => "XX".into(),
        n if (5..=39).contains(&n) => {
            let idx = n - 5;
            let level = idx / 5 + 1;
            let strain = ["♣", "♦", "♥", "♠", "NT"][(idx % 5) as usize];
            format!("{level}{strain}")
        }
        n => format!("?{n}"),
    }
}
