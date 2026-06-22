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
//! cargo run --release --example probe-bba-1nt            # BBA's defense over (1NT)
//! cargo run --release --example probe-bba-1nt responder  # BBA's Unusual-vs-Unusual: 1NT-(2NT)
//! ```
//!
//! The `responder` mode reads BBA's *opening-side* call after `1NT-(2NT)`, where
//! that `2NT` is BBA's own Multi-Landy both-minors overcall — i.e. how BBA plays
//! "Unusual vs Unusual" over our 1NT.  Both vulnerabilities are shown because the
//! penalty-double decision is vul-sensitive.

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

    if std::env::args().nth(1).as_deref() == Some("responder") {
        // Opening-side responder over 1NT-(2NT both minors) = "Unusual vs Unusual".
        // (label, ♠, ♥, ♦, ♣) — responder (position 2) after [1NT, (2NT)].
        let hands: &[(&str, &str, &str, &str, &str)] = &[
            ("penalize ♦ only  ", "K54", "84", "KQJT9", "732"),
            ("penalize ♣ only  ", "K54", "84", "732", "KQJT9"),
            ("penalize both min", "832", "54", "KQT9", "KQJT"),
            ("4-4 majors GF    ", "KQ87", "AQ96", "84", "732"),
            ("5♠ invitational  ", "KQT98", "A4", "843", "732"),
            ("5-5 majors       ", "KQ987", "AQ876", "8", "32"),
            ("balanced game    ", "Q97", "K94", "AQ7", "T987"),
            ("weak flat        ", "Q872", "J94", "T75", "962"),
            ("values, no stack ", "AJ7", "KQ7", "J85", "QT83"),
            ("strong 1-suit ♦  ", "A54", "K4", "KQJT9", "732"),
            ("weak both minors ", "8432", "4", "QJT9", "QJT9"),
        ];
        println!("BBA (system 0) responder over 1NT-(2NT both minors):\n");
        for &(label, s, h, d, c) in hands {
            let hand = suits(s, h, d, c);
            // SAFETY: fresh bot per probe; responder (position 2) holds `hand`;
            // replay opener's 1NT (code 9) and the both-minors 2NT (code 14).
            let call = |vul: c_int| unsafe {
                let bot = create();
                for seat in 0..4 {
                    set_system(bot, seat, 0);
                }
                new_hand(bot, 2, hand.as_ptr(), 0, vul, 0, 0);
                set_bid(bot, 0, 9, c"".as_ptr()); // 1NT by opener (position 0)
                set_bid(bot, 1, 14, c"".as_ptr()); // (2NT) both minors (position 1)
                let code = get_bid(bot);
                destroy(bot);
                decode(code)
            };
            // vul bit 1 = N/S (us, position 2 is even), bit 2 = E/W (them).
            println!(
                "  {label} ♠{s} ♥{h} ♦{d} ♣{c}  ->  NV {:<5}  they-vul {:<5}  both {}",
                call(0),
                call(2),
                call(3),
            );
        }
        return Ok(());
    }

    if std::env::args().nth(1).as_deref() == Some("runout") {
        // Is there a *delayed* penalty double of the opponents' 3♣ runout?
        // After 1NT-(2NT)-P, the advancer picks a minor (3♣ here).  We probe two
        // seats with club-stacked hands: opener reopening over 3♣, and responder's
        // delayed double after opener+overcaller pass.  `prefix` is the replayed
        // auction up to (but not including) the actor; `actor` is its seat.
        let probe = |label: &str, actor: c_int, prefix: &[c_int], s, h, d, c| {
            let hand = suits(s, h, d, c);
            // SAFETY: fresh bot; `actor` holds `hand`; replay `prefix` then read.
            let call = |vul: c_int| unsafe {
                let bot = create();
                for seat in 0..4 {
                    set_system(bot, seat, 0);
                }
                new_hand(bot, actor, hand.as_ptr(), 0, vul, 0, 0);
                for (index, &code) in prefix.iter().enumerate() {
                    set_bid(bot, (index % 4) as c_int, code, c"".as_ptr());
                }
                let code = get_bid(bot);
                destroy(bot);
                decode(code)
            };
            println!(
                "  {label} ♠{s} ♥{h} ♦{d} ♣{c}  ->  NV {:<5}  they-vul {:<5}  both {}",
                call(0),
                call(2),
                call(3),
            );
        };
        // 1NT=9, 2NT=14, Pass=0, 3♣=15.
        println!("BBA (system 0) over the opponents' 3♣ runout after 1NT-(2NT)-P-3♣:\n");
        println!("opener reopening [1NT,(2NT),P,(3♣)]:");
        probe(
            "16, club stack ",
            0,
            &[9, 14, 0, 15],
            "AQ5",
            "K72",
            "K85",
            "KJ83",
        );
        probe(
            "17, balanced   ",
            0,
            &[9, 14, 0, 15],
            "AQ5",
            "KQ7",
            "KJ86",
            "Q83",
        );
        println!("\nresponder delayed [1NT,(2NT),P,(3♣),P,P]:");
        probe(
            "penalize ♣ only",
            2,
            &[9, 14, 0, 15, 0, 0],
            "K54",
            "84",
            "732",
            "KQJT9",
        );
        probe(
            "9 HCP, ♣ stack ",
            2,
            &[9, 14, 0, 15, 0, 0],
            "J54",
            "Q84",
            "73",
            "KQJT9",
        );
        return Ok(());
    }

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
