//! Probe the real EPBot engine (system 0, 2/1 GF — the card `bba-match` uses) for
//! its direct-seat call over a (1NT) opening, across a set of crafted archetype
//! hands.  This reads BBA's *actual* convention from real hands, the only reliable
//! way: the `.so` ignores the `vendor/bba/*.bbsa` cards entirely (strace shows it
//! opens no data file — those drive `BBA.exe`, not the FFI), so the compiled-in
//! system can disagree with the config (here it plays **Multi-Landy**, while
//! `21GF.bbsa` labels the card `Cappelletti=1`).  The same `create → set_system →
//! new_hand → set_bid → get_bid` recipe verifies any BBA convention; edit `hands`
//! (and the replayed auction) for a different position.  Run from the repo root:
//!
//! ```text
//! cargo run --release --example probe-bba-1nt
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

/// Decode EPBot's bid code (0/1/2 = Pass/X/XX; bid = 5 + (level-1)*5 + strain)
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

/// Join four suit holdings into EPBot's C\nD\nH\nS hand string (13 cards total)
fn suits(spades: &str, hearts: &str, diamonds: &str, clubs: &str) -> CString {
    let n = spades.len() + hearts.len() + diamonds.len() + clubs.len();
    assert_eq!(
        n, 13,
        "hand {spades}.{hearts}.{diamonds}.{clubs} has {n} cards"
    );
    CString::new(format!("{clubs}\n{diamonds}\n{hearts}\n{spades}")).unwrap()
}

fn main() -> anyhow::Result<()> {
    let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
    let lib = unsafe { Library::new(&path) }?;
    let (create, destroy, set_system, new_hand, set_bid, get_bid) = unsafe {
        (
            *lib.get::<CreateFn>(b"epbot_create\0")?,
            *lib.get::<DestroyFn>(b"epbot_destroy\0")?,
            *lib.get::<SetSystemFn>(b"epbot_set_system_type\0")?,
            *lib.get::<NewHandFn>(b"epbot_new_hand\0")?,
            *lib.get::<SetBidFn>(b"epbot_set_bid\0")?,
            *lib.get::<GetBidFn>(b"epbot_get_bid\0")?,
        )
    };

    // (label, spades, hearts, diamonds, clubs) — defender (position 1) over 1NT.
    let hands: &[(&str, &str, &str, &str, &str)] = &[
        ("6♠ one-suiter   ", "AQJ976", "K5", "84", "732"),
        ("6♥ one-suiter   ", "K5", "AQJ976", "84", "732"),
        ("6♦ one-suiter   ", "K5", "84", "AQJ976", "732"),
        ("6♣ one-suiter   ", "K5", "84", "732", "AQJ976"),
        ("5♠ only (5332)  ", "AQJ97", "K5", "Q84", "732"),
        ("5♥ only (5332)  ", "K5", "AQJ97", "Q84", "732"),
        ("5-5 majors      ", "AQ976", "KJ876", "8", "43"),
        ("5-5 minors      ", "8", "43", "AQ976", "KJ876"),
        ("5♥-5♦ H+minor   ", "8", "AQ976", "KJ876", "43"),
        ("5♥-5♣ H+minor   ", "8", "AQ976", "43", "KJ876"),
        ("5♠-5♦ S+minor   ", "AQ976", "8", "KJ876", "43"),
        ("5♠-5♣ S+minor   ", "AQ976", "8", "43", "KJ876"),
        ("5♥-4♦ H+minor   ", "K5", "AQ976", "KJ83", "84"),
        ("5♠-4♥ majors    ", "AQ976", "KJ87", "84", "32"),
        ("4♠-5♥ majors    ", "KJ87", "AQ976", "84", "32"),
        ("4♠-4♥ majors    ", "KJ87", "AQ96", "843", "32"),
        ("5♠-4♣ S+minor   ", "AQ976", "K5", "84", "KJ87"),
        ("5♥-4♣ H+minor   ", "K5", "AQ976", "84", "KJ87"),
        ("balanced 17 HCP ", "AQ5", "KQ7", "KJ86", "Q83"),
        ("balanced 19 HCP ", "AQ5", "KQ7", "AJ86", "KJ3"),
        ("balanced 22 HCP ", "AKQ", "AQ7", "AJ86", "KJ3"),
        ("balanced 13 HCP ", "AQJ", "K72", "J986", "Q83"),
        ("flat 8 HCP      ", "Q872", "K72", "J96", "Q83"),
    ];

    println!("BBA (EPBot system 0, 2/1 GF) direct call over (1NT):\n");
    for &(label, s, h, d, c) in hands {
        let hand = suits(s, h, d, c);
        // SAFETY: fresh bot per probe; all four seats set to system 0; the
        // defender (position 1) holds `hand`; replay the opener's 1NT (code 9).
        let code = unsafe {
            let bot = create();
            for seat in 0..4 {
                set_system(bot, seat, 0);
            }
            new_hand(bot, 1, hand.as_ptr(), 0, 0, 0, 0);
            set_bid(bot, 0, 9, c"".as_ptr()); // opener's 1NT at position 0
            let code = get_bid(bot);
            destroy(bot);
            code
        };
        println!("  {label} ♠{s} ♥{h} ♦{d} ♣{c}  ->  {}", decode(code));
    }
    Ok(())
}
