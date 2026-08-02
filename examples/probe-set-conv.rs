//! Ground truth for `epbot_set_conventions`: what it returns, what round-trips,
//! and whether its second argument is a **seat** or a **side**.
//!
//! The repo contradicts itself here.  `docs/ai-bidder/bba-kickback.md` records an
//! "FFI trap" claiming the argument is a side (0/1), that seats 2/3 return −2, and
//! that `examples/common/oracle.rs` is correct only by an aliasing accident.
//! `docs/ai-bidder/configured-net.md` claims the opposite — seat + name, with −2
//! coming from passing a convention *index* instead of a name.  Both cannot hold,
//! and the configured net pushes 135 rows per side, so the answer decides whether
//! a corpus records the configuration the teacher actually played.
//!
//! ```text
//! cargo run --release --example probe-set-conv -- cards/American.bbsa
//! ```
//!
//! Reports, in order: the return code per index for a known and an unknown name;
//! how many slots the engine really has; whether a bogus row announces itself;
//! whether the two slots hold independent systems; and every card row that fails
//! to read back what was written.
//!
//! **Measured answer (2026-08-02, `libEPBot.so`, system 0):** the argument is a
//! **side** — indices 0 and 1 answer, 2 and above return −2 from the setter *and*
//! the getter.  An unknown name returns **0** and reads back **0**, so a
//! misspelled row is invisible to the return code and only a read-back finds it.
//! The two slots are independent, so asymmetric cards are real.  Three rows of
//! `cards/American.bbsa` do not stick: our two `PONS_SCHEMA` filler names (by
//! design) and `Reverse Bergen`, which refuses to turn off.

use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};

const DEFAULT_LIB: &str = "vendor/bba/Native-libraries/linux/x64/libEPBot.so";

type CreateFn = unsafe extern "C" fn() -> *mut c_void;
type DestroyFn = unsafe extern "C" fn(*mut c_void);
type SetSystemFn = unsafe extern "C" fn(*mut c_void, c_int, c_int);
type GetConvFn = unsafe extern "C" fn(*mut c_void, c_int, *const c_char) -> c_int;
type SetConvFn = unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int) -> c_int;

/// Parse a `.bbsa` card into its system id and `name = value` toggles
fn parse(path: &str) -> anyhow::Result<(c_int, Vec<(CString, c_int)>)> {
    let text = std::fs::read_to_string(path)?;
    let mut system = None;
    let mut toggles = Vec::new();
    for line in text.lines() {
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        let (name, value) = (name.trim(), value.trim());
        let Ok(value) = value.parse::<c_int>() else {
            continue;
        };
        if name == "System type" {
            system = Some(value);
        } else {
            toggles.push((CString::new(name)?, value));
        }
    }
    Ok((
        system.ok_or_else(|| anyhow::anyhow!("card `{path}` has no `System type` header"))?,
        toggles,
    ))
}

fn main() -> anyhow::Result<()> {
    let card = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "cards/American.bbsa".to_owned());
    let (system, toggles) = parse(&card)?;

    // SAFETY: a trusted vendored library; both signatures are the ones already
    // bound in `examples/common/oracle.rs` and `probe-bba-conventions`.
    let lib = unsafe { Library::new(DEFAULT_LIB) }?;
    let (create, destroy, set_system, get_conv, set_conv) = unsafe {
        (
            *lib.get::<CreateFn>(b"epbot_create\0")?,
            *lib.get::<DestroyFn>(b"epbot_destroy\0")?,
            *lib.get::<SetSystemFn>(b"epbot_set_system_type\0")?,
            *lib.get::<GetConvFn>(b"epbot_get_conventions\0")?,
            *lib.get::<SetConvFn>(b"epbot_set_conventions\0")?,
        )
    };

    let known = CString::new("Stayman")?;
    let unknown = CString::new("Pons Queen Relay Nonsense")?;

    // SAFETY: the handle lives until `destroy`; every seat is configured exactly
    // as `BbaOracle::load` does, so what is measured is what a dump would see.
    unsafe {
        let bot = create();
        for seat in 0..4 {
            set_system(bot, seat, system);
        }

        println!("== return codes, `{card}`, system {system} ==");
        for seat in 0..6 {
            let ok = set_conv(bot, seat, known.as_ptr(), 1);
            let bad = set_conv(bot, seat, unknown.as_ptr(), 1);
            println!("  seat {seat}:  known name -> {ok:>3}    unknown name -> {bad:>3}");
        }

        println!("\n== how many slots?  write 0 and 1, read all four ==");
        let probe = CString::new("Landy")?;
        for seat in 0..4 {
            set_conv(bot, seat, probe.as_ptr(), 0);
        }
        set_conv(bot, 0, probe.as_ptr(), 1);
        let seen: Vec<c_int> = (0..4)
            .map(|seat| get_conv(bot, seat, probe.as_ptr()))
            .collect();
        println!("  after set(0, Landy, 1): get(0..4) = {seen:?}");
        println!("  (-2 on the read side too means the index is out of range, not merely unset)");

        println!("\n== does an unknown name announce itself at all? ==");
        let before = get_conv(bot, 0, unknown.as_ptr());
        let code = set_conv(bot, 0, unknown.as_ptr(), 1);
        let after = get_conv(bot, 0, unknown.as_ptr());
        println!("  set -> {code}, get before {before}, get after {after}");
        println!(
            "  verdict: a bogus row is {} by its return code",
            if code == 0 {
                "INVISIBLE -- only a read-back catches it"
            } else {
                "reported"
            }
        );

        println!("\n== set_system: two slots or four? ==");
        // Give slot 0 and slot 1 different systems; if a row's default differs
        // between them, the two slots are genuinely independent.
        set_system(bot, 0, 0);
        set_system(bot, 1, 8);
        let split: Vec<_> = toggles
            .iter()
            .filter_map(|(name, _)| {
                let (a, b) = (
                    get_conv(bot, 0, name.as_ptr()),
                    get_conv(bot, 1, name.as_ptr()),
                );
                (a != b).then(|| (name.to_string_lossy().into_owned(), a, b))
            })
            .collect();
        println!(
            "  systems (0, 8): {} of {} rows differ between slot 0 and slot 1",
            split.len(),
            toggles.len()
        );
        for (name, a, b) in split.iter().take(6) {
            println!("      {name:<44} slot0 {a}  slot1 {b}");
        }
        println!(
            "  verdict: the two slots are {}",
            if split.is_empty() {
                "NOT shown independent by this test"
            } else {
                "INDEPENDENT -- asymmetric systems are real"
            }
        );
        for seat in 0..4 {
            set_system(bot, seat, system);
        }

        println!("\n== round trip: every card row, every seat ==");
        let mut bad_return = Vec::new();
        let mut bad_read = Vec::new();
        for (name, want) in &toggles {
            for seat in 0..4 {
                let code = set_conv(bot, seat, name.as_ptr(), *want);
                if code != 0 {
                    bad_return.push((name.to_string_lossy().into_owned(), seat, code));
                }
            }
            let got = get_conv(bot, 0, name.as_ptr());
            if got != *want {
                bad_read.push((name.to_string_lossy().into_owned(), *want, got));
            }
        }

        println!("  rows: {}", toggles.len());
        println!("  non-zero returns: {}", bad_return.len());
        for (name, seat, code) in bad_return.iter().take(20) {
            println!("      {name:<44} seat {seat} -> {code}");
        }
        println!(
            "  rows that do NOT read back what was written: {}",
            bad_read.len()
        );
        for (name, want, got) in &bad_read {
            println!("      {name:<44} wrote {want}, read {got}");
        }

        destroy(bot);
    }
    Ok(())
}
