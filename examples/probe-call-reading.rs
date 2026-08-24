//! What does one call actually *read* as, on the surface the nets are handed?
//!
//! `probe-reading-census` counts how many of the five axes are ⊤; this prints
//! the ranges themselves for one auction, so a key off the census's worklist can
//! be inspected directly. Reads through [`Partnership::infer`] and reports the
//! `announced()` envelope — exactly what `features::push_inference` encodes.
//!
//! Each argument is an auction; the reading shown is of its **last** call, from
//! the perspective of the seat about to act.
//!
//! ```sh
//! cargo run --release --example probe-call-reading -- "2H -" "2H (X)" "2H (2N)"
//! ```
//!
//! The three above read `⊤ ⊤ ⊤ ⊤ ⊤`, `points 12..` (shape ⊤), and `⊤ ⊤ ⊤ ⊤ ⊤`
//! — even though the 2NT overcall is authored `hcp(15..=18) & balanced() &
//! stopper_in_their_suits()`. `project_authored` decodes **alerted calls only**,
//! and the natural walk reads length off a bid *suit*, so an unalerted notrump
//! overcall reaches the nets as nothing at all. Contrast `1N` (a book node:
//! `points 15..=18`, `♣ 2..=6 ♦ 2..=6 ♥ 2..=5 ♠ 2..=5`).

use clap::Parser;
use contract_bridge::auction::Call;
use contract_bridge::{AbsoluteVulnerability, Suit};
use pons::american;
use pons::bidding::context::relative;
use pons::bidding::{Envelope, Relative};

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::seat_to_act;

#[derive(Parser)]
struct Args {
    /// Auctions to read, e.g. `"2H -"` or `"2H (X)"`; the last call of each is read
    auctions: Vec<String>,

    /// Author the weak-two pass gate (`defense.weak_two_pass_gate`, default off)
    #[arg(long, default_value_t = false)]
    weak_two_pass_gate: bool,

    /// Turn on the three weak-two defense candidates at once: the wider 2NT
    /// shape, the sub-3NT jump overcall, and the Michaels cue
    #[arg(long, default_value_t = false)]
    weak_two_v2: bool,

    /// Arm the `(2♦)` diamond penalty double (`competition.two_diamond_double`)
    /// as `LEN:SUITHCP:HCP`, so its projection can be read off `1N (2D) X`
    #[arg(long)]
    ns_2d_double: Option<String>,

    /// Declare their `2♦` a Multi (`their.two_diamonds_multi`), so the N4
    /// table's values double and opener's trump double read off `1N (2D) X`
    #[arg(long, default_value_t = false)]
    their_2d_multi: bool,

    /// Minimum suit length for the floorless Multi escape
    /// (`competition.multi_weak_escape`), so its published reading can be read
    /// off `1H 1NT 2D 2S` as well as off `1N (2D) 2S`.  Absent leaves the
    /// shipped default (`Some(6)`) alone; `0` turns the escape off.
    #[arg(long)]
    ns_multi_weak_escape: Option<u8>,

    /// Read their Multi *advance* as the whole pass-or-correct ladder and
    /// claim `♥3+ & ♠3+` on its jump rungs
    /// (`reading.their_multi_advance_reading`), so `1N (2D) X (3H)` and
    /// `1N (2D) X (4D)` can be read on both arms
    #[arg(long, default_value_t = false)]
    ns_their_multi_advance_read: bool,

    /// Read our values double of their Multi at its authored `hcp(6..)`
    /// (`reading.their_multi_double_reading`), off `1N (2D) X -`
    #[arg(long, default_value_t = false)]
    ns_their_multi_double_read: bool,

    /// Play the Kokish–Kraft counter to their Multi
    /// (`competition.multi_kokish_kraft`), so its `X`, its floorless minor
    /// transfers and its delayed takeout double can be read off
    /// `1N (2D) X`, `1N (2D) 2N` and `1N (2D) - (2H) - - X`
    #[arg(long, default_value_t = false)]
    ns_multi_kokish_kraft: bool,
}

fn render(shown: &Envelope) -> String {
    let mut out = format!("points {:?}", shown.strength.points);
    for suit in Suit::ASC {
        out += &format!("  {suit} {:?}", shown.length(suit));
    }
    out
}

fn main() {
    let args = Args::parse();
    let mut agreements = pons::bidding::agreements::Agreements::default();
    agreements.defense.weak_two_pass_gate = args.weak_two_pass_gate;
    agreements.defense.weak_two_notrump_shape = args.weak_two_v2;
    agreements.defense.weak_two_jump_overcall = args.weak_two_v2;
    agreements.defense.weak_two_cue = args.weak_two_v2;
    agreements.competition.two_diamond_double = args.ns_2d_double.as_deref().map(|spec| {
        let mut parts = spec.split(':').map(|p| p.parse::<u8>().expect("a number"));
        let mut next = || parts.next().expect("LEN:SUITHCP:HCP");
        (usize::from(next()), next(), next())
    });
    agreements.decision.their.two_diamonds_multi = args.their_2d_multi;
    agreements.decision.reading.their_multi_advance_reading = args.ns_their_multi_advance_read;
    agreements.decision.reading.their_multi_double_reading = args.ns_their_multi_double_read;
    agreements.competition.multi_kokish_kraft = args.ns_multi_kokish_kraft;
    if let Some(n) = args.ns_multi_weak_escape {
        agreements.competition.multi_weak_escape = (n > 0).then_some(n);
    }
    let vul = AbsoluteVulnerability::NONE;
    let partnership = american(&agreements).bind();

    for text in &args.auctions {
        let auction: Vec<Call> = text
            .split_whitespace()
            .map(|token| {
                token
                    .strip_prefix('(')
                    .and_then(|token| token.strip_suffix(')'))
                    .unwrap_or(token)
                    .parse()
                    .expect("a call")
            })
            .collect();
        // Read from the seat about to act, so the auction's last call is RHO's.
        let rel = relative(
            vul,
            seat_to_act(contract_bridge::Seat::North, auction.len()),
        );
        let read = partnership.infer(rel, &auction);
        println!(
            "{text:<12} rho     {}",
            render(read.announced(Relative::Rho))
        );
        // Our own side's view of the same auction: a reader gated on "the 1NT is
        // ours" is silent from an opponent's seat, so a call can look vacuous
        // here and be read fine one seat over.
        println!(
            "{:<12} partner {}",
            "",
            render(read.announced(Relative::Partner))
        );
    }
}
