//! Read EPBot's **compiled-in default** convention settings for a system, and diff
//! them against a vendored `.bbsa` card.
//!
//! The `.so` never opens the cards itself (see `probe-bba-1nt`), so a card only
//! takes effect when something replays it through `epbot_set_conventions` — which
//! `load_bbsa` does for `bba-gen --our-card`, and which `dump-teacher` did *not*
//! do before the `--card` flag existed.  Any net distilled by such a run learned
//! the engine defaults, whatever those happen to be, so knowing where the
//! defaults and the card disagree is what tells you which conventional agreements
//! a distilled floor actually holds.
//!
//! ```text
//! cargo run --release --example probe-bba-conventions -- vendor/bba/21GF.bbsa
//! cargo run --release --example probe-bba-conventions -- vendor/bba/WJ.bbsa --all
//! ```
//!
//! Prints the disagreements by default (`--all` prints every toggle).  The system
//! id comes from the card's `System type` header, so the comparison is always
//! against the defaults for the system that card describes.

use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};

const DEFAULT_LIB: &str = "vendor/bba/Native-libraries/linux/x64/libEPBot.so";

type CreateFn = unsafe extern "C" fn() -> *mut c_void;
type DestroyFn = unsafe extern "C" fn(*mut c_void);
type SetSystemFn = unsafe extern "C" fn(*mut c_void, c_int, c_int);
// `epbot_get_conventions(bot, seat, name)` — the read side of the per-seat toggle.
type GetConvFn = unsafe extern "C" fn(*mut c_void, c_int, *const c_char) -> c_int;

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
    let mut args = std::env::args().skip(1);
    let card = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("usage: probe-bba-conventions CARD.bbsa [--all]"))?;
    let all = args.any(|arg| arg == "--all");
    let (system, toggles) = parse(&card)?;

    // SAFETY: loading a trusted vendored library; signatures confirmed against
    // the exported symbol table and the existing `epbot_set_conventions` binding.
    let lib = unsafe { Library::new(DEFAULT_LIB) }?;
    let (create, destroy, set_system, get_conv) = unsafe {
        (
            *lib.get::<CreateFn>(b"epbot_create\0")?,
            *lib.get::<DestroyFn>(b"epbot_destroy\0")?,
            *lib.get::<SetSystemFn>(b"epbot_set_system_type\0")?,
            *lib.get::<GetConvFn>(b"epbot_get_conventions\0")?,
        )
    };

    // SAFETY: the handle lives until `destroy`; every seat is set to `system`
    // exactly as `BbaOracle::load` does, so the defaults read are the ones a
    // teacher dump would have run under.
    unsafe {
        let bot = create();
        for seat in 0..4 {
            set_system(bot, seat, system);
        }
        let mut differ = 0;
        println!("system {system}, card {card}\n");
        println!("{:<44} {:>7} {:>5}", "convention", "default", "card");
        for (name, want) in &toggles {
            let got = get_conv(bot, 0, name.as_ptr());
            if got != *want {
                differ += 1;
            } else if !all {
                continue;
            }
            let flag = if got == *want { ' ' } else { '*' };
            println!(
                "{flag}{:<43} {got:>7} {want:>5}",
                name.to_string_lossy().as_ref()
            );
        }
        destroy(bot);
        println!(
            "\n{differ} of {} toggles differ from the card",
            toggles.len()
        );
    }
    Ok(())
}
