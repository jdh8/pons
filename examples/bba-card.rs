//! Print a system's `.bbsa` convention card, generated from the live book
//!
//! The bless path for `cards/*.bbsa`, which `tests/bba_card.rs` asserts are
//! current:
//!
//! ```sh
//! cargo run --example bba-card -- --system american >cards/American.bbsa
//! cargo run --example bba-card -- --system dutch    >cards/Dutch.bbsa
//! ```
//!
//! Needs neither EPBot nor the `bba` feature — a card is a pure function of the
//! thread-local knob state.

use pons::bidding::card::{Card, american_card, dutch_card};

fn main() {
    let mut system = "american".to_owned();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--system" => system = args.next().unwrap_or_default(),
            other => {
                eprintln!("bba-card: unexpected argument `{other}`");
                eprintln!("usage: bba-card [--system american|dutch]");
                std::process::exit(2);
            }
        }
    }
    print!("{}", card_for(&system));
}

/// The card for a named system, or a usage error naming the systems that have one
///
/// Exits rather than falling back: a system with no card generator must never be
/// disclosed under another system's card, which would misdescribe us to BBA far
/// more damagingly than disclosing nothing.
fn card_for(system: &str) -> Card {
    match system {
        "american" => american_card(&pons::bidding::agreements::Agreements::default()),
        "dutch" => dutch_card(&pons::bidding::agreements::Agreements::default()),
        other => {
            eprintln!("bba-card: no card generator for system `{other}`");
            eprintln!("           known systems: american, dutch");
            std::process::exit(2);
        }
    }
}
