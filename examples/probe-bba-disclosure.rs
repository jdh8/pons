//! Does EPBot's *bidding* depend on what it believes the **opponents** play?
//!
//! `cards/American.bbsa` exists to declare our 2/1 to the BBA seats, but nothing
//! wires it: [`BbaOracle::with_bot`] configures all four seats to one system and
//! applies one convention set to all four, so BBA has never been told what pons
//! plays.  Before auditing 258 toggles it is worth knowing whether any
//! disclosure channel moves a single call.  Three candidates:
//!
//! 1. **Per-seat system** — `epbot_set_system_type(bot, seat, system)` on the
//!    opponents' seats only, actor's own seats left on 2/1.
//! 2. **Per-seat conventions** — `epbot_set_conventions(bot, seat, name, on)` on
//!    the opponents' seats only.
//! 3. **`epbot_set_opponent_type`** — the scalar behind the card's trailing
//!    `Opponent type = 0` row, recorded in `docs/ai-bidder/bba-floor.md` as
//!    unbound because its `systemNumber` argument is ambiguous.  Arity is
//!    discovered empirically here via the getter.
//!
//! The actor is seat 1 facing an opening from seat 0 (its RHO), on auctions whose
//! *meaning* is system-dependent (a `2♦` that is weak-natural or Multi; a `1NT`
//! that is 15-17 or 12-14).  If the actor's call is invariant across every axis,
//! the card-as-disclosure premise is dead and only the toggles that change our
//! *own* seats can matter.
//!
//! ```text
//! cargo run --release --example probe-bba-disclosure
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
// Arity unknown — see the module docs.  Passing two integers is safe under the
// SysV ABI whether the callee reads one or both.
type SetOppFn = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> c_int;
type GetOppFn = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;

/// The actor's seat: seat 0 deals and opens, so seat 1 acts over its RHO.
const ACTOR: c_int = 1;
/// The actor's opponents — the seats a disclosure channel would reconfigure.
const THEM: [c_int; 2] = [0, 2];
const SYSTEM_2_OVER_1: c_int = 0;

/// A hand in EPBot's form: clubs, diamonds, hearts, spades, high-to-low, `T` for ten
struct Probe {
    label: &'static str,
    hand: &'static str,
    /// The auction before the actor's turn, as EPBot bid codes
    opening: c_int,
    opening_label: &'static str,
}

/// `5 + (level - 1) * 5 + strain`, strain `0 = ♣ … 4 = NT`
const fn bid(level: c_int, strain: c_int) -> c_int {
    5 + (level - 1) * 5 + strain
}

const PROBES: &[Probe] = &[
    Probe {
        label: "balanced 16",
        hand: "A93\nQJ7\nK52\nAKQ4",
        opening: bid(2, 1),
        opening_label: "2D",
    },
    Probe {
        label: "majors 5-5",
        hand: "52\n4\nAQ976\nAKJ83",
        opening: bid(2, 1),
        opening_label: "2D",
    },
    Probe {
        label: "long diamonds",
        hand: "J2\nAKQ8765\nK3\nA4",
        opening: bid(2, 1),
        opening_label: "2D",
    },
    Probe {
        label: "balanced 16",
        hand: "A93\nQJ7\nK52\nAKQ4",
        opening: bid(1, 4),
        opening_label: "1NT",
    },
    Probe {
        label: "majors 5-5",
        hand: "52\n4\nAQ976\nAKJ83",
        opening: bid(1, 4),
        opening_label: "1NT",
    },
    Probe {
        label: "flat 10",
        hand: "Q82\nK963\nT54\nJ87",
        opening: bid(1, 4),
        opening_label: "1NT",
    },
];

/// Render an EPBot bid code
fn show(code: c_int) -> String {
    match code {
        0 => "Pass".into(),
        1 => "X".into(),
        2 => "XX".into(),
        5..=39 => {
            let index = code - 5;
            let level = index / 5 + 1;
            let strain = ["C", "D", "H", "S", "NT"][(index % 5) as usize];
            format!("{level}{strain}")
        }
        other => format!("?{other}"),
    }
}

struct Epbot {
    create: CreateFn,
    destroy: DestroyFn,
    set_system: SetSystemFn,
    new_hand: NewHandFn,
    set_bid: SetBidFn,
    get_bid: GetBidFn,
    set_conv: SetConvFn,
    set_opp: SetOppFn,
    get_opp: GetOppFn,
}

impl Epbot {
    /// One decision on a fresh bot; `configure` runs after the seats are set to
    /// 2/1 and before the hand is dealt.
    fn ask(&self, probe: &Probe, configure: impl FnOnce(*mut c_void)) -> c_int {
        let suits = CString::new(probe.hand).expect("a literal hand has no NUL");
        let empty = c"".as_ptr();
        // SAFETY: a fresh bot used and destroyed within this call; all argument
        // types match the ABI confirmed in `examples/common/oracle.rs`.
        unsafe {
            let bot = (self.create)();
            assert!(!bot.is_null(), "EPBot failed to allocate a bot");
            for seat in 0..4 {
                (self.set_system)(bot, seat, SYSTEM_2_OVER_1);
            }
            configure(bot);
            (self.new_hand)(bot, ACTOR, suits.as_ptr(), 0, 0, 0, 0);
            (self.set_bid)(bot, 0, probe.opening, empty);
            let code = (self.get_bid)(bot);
            (self.destroy)(bot);
            code
        }
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_LIB.into());
    // SAFETY: loading the vendored EPBot shared object and binding symbols whose
    // signatures are confirmed in `examples/common/oracle.rs` and by `nm -D`.
    let (epbot, _lib) = unsafe {
        let lib = Library::new(&path)?;
        let epbot = Epbot {
            create: *lib.get(b"epbot_create\0")?,
            destroy: *lib.get(b"epbot_destroy\0")?,
            set_system: *lib.get(b"epbot_set_system_type\0")?,
            new_hand: *lib.get(b"epbot_new_hand\0")?,
            set_bid: *lib.get(b"epbot_set_bid\0")?,
            get_bid: *lib.get(b"epbot_get_bid\0")?,
            set_conv: *lib.get(b"epbot_set_conventions\0")?,
            set_opp: *lib.get(b"epbot_set_opponent_type\0")?,
            get_opp: *lib.get(b"epbot_get_opponent_type\0")?,
        };
        (epbot, lib)
    };

    // --- Baseline: every seat on 2/1, nothing disclosed ---
    eprintln!("checkpoint: symbols bound");
    let baseline: Vec<c_int> = PROBES.iter().map(|p| epbot.ask(p, |_| {})).collect();
    eprintln!("checkpoint: baseline done");

    // --- The systems to reconfigure the opponents' seats with ---
    //
    // `epbot_system_name` segfaults under this binding (its out-parameter is not
    // the `(bot, index) -> *const c_char` shape `nm` suggests), so the ids stand
    // unnamed; `set_system_type` clamps or ignores an out-of-range id.
    let systems: Vec<(c_int, String)> = (0..12).map(|id| (id, format!("system {id}"))).collect();

    println!("\n== baseline (all four seats 2/1) ==");
    for (probe, &code) in PROBES.iter().zip(&baseline) {
        println!(
            "  ({}) {:<14} -> {}",
            probe.opening_label,
            probe.label,
            show(code)
        );
    }

    // --- Axis 1: the opponents' seats play a different system ---
    println!("\n== axis 1: set_system_type on THEIR seats only ==");
    let mut axis1_moved = 0;
    for (id, name) in &systems {
        if *id == SYSTEM_2_OVER_1 {
            continue;
        }
        let calls: Vec<c_int> = PROBES
            .iter()
            .map(|p| {
                epbot.ask(p, |bot| {
                    for seat in THEM {
                        // SAFETY: `bot` is live inside `ask`.
                        unsafe { (epbot.set_system)(bot, seat, *id) };
                    }
                })
            })
            .collect();
        let diffs = calls.iter().zip(&baseline).filter(|(a, b)| a != b).count();
        axis1_moved += diffs;
        if diffs > 0 {
            println!("  {id:2} {name:<40} {diffs}/{} changed", PROBES.len());
            for ((probe, &now), &was) in PROBES.iter().zip(&calls).zip(&baseline) {
                if now != was {
                    println!(
                        "       ({}) {:<14} {} -> {}",
                        probe.opening_label,
                        probe.label,
                        show(was),
                        show(now)
                    );
                }
            }
        }
    }
    println!("  total changed calls: {axis1_moved}");

    // --- Axis 2: one convention toggled on the opponents' seats only ---
    println!("\n== axis 2: set_conventions on THEIR seats only ==");
    let mut axis2_moved = 0;
    for name in [
        "Multi",
        "Weak natural 2D",
        "1NT opening range 12-14",
        "Landy",
    ] {
        let conv = CString::new(name).expect("a literal name has no NUL");
        for on in [0, 1] {
            let calls: Vec<c_int> = PROBES
                .iter()
                .map(|p| {
                    epbot.ask(p, |bot| {
                        for seat in THEM {
                            // SAFETY: `bot` is live inside `ask`.
                            unsafe { (epbot.set_conv)(bot, seat, conv.as_ptr(), on) };
                        }
                    })
                })
                .collect();
            let diffs = calls.iter().zip(&baseline).filter(|(a, b)| a != b).count();
            axis2_moved += diffs;
            println!("  {name:<26} = {on}  {diffs}/{} changed", PROBES.len());
            for ((probe, &now), &was) in PROBES.iter().zip(&calls).zip(&baseline) {
                if now != was {
                    println!(
                        "       ({}) {:<14} {} -> {}",
                        probe.opening_label,
                        probe.label,
                        show(was),
                        show(now)
                    );
                }
            }
        }
    }
    println!("  total changed calls: {axis2_moved}");

    // --- Axis 3: the `Opponent type` scalar, arity discovered by round trip ---
    println!("\n== axis 3: epbot_set_opponent_type ==");
    // SAFETY: a live bot; extra integer arguments are harmless under SysV.
    unsafe {
        let bot = (epbot.create)();
        for value in [0, 1, 2, 3] {
            (epbot.set_opp)(bot, value, value);
            let readback: Vec<c_int> = (0..4).map(|arg| (epbot.get_opp)(bot, arg)).collect();
            println!("  set({value}, {value}) -> get(0..4) = {readback:?}");
        }
        (epbot.destroy)(bot);
    }
    let mut axis3_moved = 0;
    for value in 1..8 {
        let calls: Vec<c_int> = PROBES
            .iter()
            .map(|p| {
                epbot.ask(p, |bot| {
                    // SAFETY: `bot` is live inside `ask`.
                    unsafe { (epbot.set_opp)(bot, value, value) };
                })
            })
            .collect();
        let diffs = calls.iter().zip(&baseline).filter(|(a, b)| a != b).count();
        axis3_moved += diffs;
        if diffs > 0 {
            println!("  opponent type {value}: {diffs}/{} changed", PROBES.len());
            for ((probe, &now), &was) in PROBES.iter().zip(&calls).zip(&baseline) {
                if now != was {
                    println!(
                        "       ({}) {:<14} {} -> {}",
                        probe.opening_label,
                        probe.label,
                        show(was),
                        show(now)
                    );
                }
            }
        }
    }
    println!("  total changed calls: {axis3_moved}");

    println!("\nVERDICT: axis1={axis1_moved} axis2={axis2_moved} axis3={axis3_moved}");
    Ok(())
}
