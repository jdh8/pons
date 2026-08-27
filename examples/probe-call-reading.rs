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

    /// Declare their `2♣` a Landy (`their.two_clubs_landy`), so the N1j
    /// counter's calls read off `1N (2C) X`, `1N (2C) 2N` and their
    /// continuations
    #[arg(long, default_value_t = false)]
    their_2c_landy: bool,

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

    /// Fall back to the v7 subtree against their Multi, disabling the
    /// shipped Kokish–Kraft counter (`competition.multi_kokish_kraft`) — so
    /// the pre-N4-KK readings of `1N (2D) X`, `1N (2D) 2N` and
    /// `1N (2D) - (2H) - - X` can be compared against it
    #[arg(long, default_value_t = false)]
    no_ns_multi_kokish_kraft: bool,

    /// The `points` floor of the `4m` slam try above a completed Kokish–Kraft
    /// minor transfer: a `points` floor (default `15`), or `off` — `13` is
    /// `landy_bba_transfer_rebid`'s own rung, `15` the narrow arm
    ///
    /// Also authors opener's answer (`4NT` RKCB on a maximum, else `5m`) and,
    /// on the same switch, the shortness `4m` when they compete over the
    /// completion (§N4-KK residues 3 and 6, `docs/minor-transfer-slam.md`).
    /// Needs the Kokish–Kraft counter and their declared `(2♦)` Multi to do
    /// anything.
    #[arg(long, default_value = "15", value_name = "off|13|15")]
    ns_multi_minor_slam_try: String,

    /// Withhold the Kokish–Kraft doubler's **natural bid of the other major**
    /// once their pass-or-correct resolves theirs
    ///
    /// Turns *off* the rung shipped default-on 2026-08-26
    /// (`competition.multi_doubler_major`): `2♠` over their `(2♥)`, `3♥` over
    /// their `(2♠)`, on four-plus of the other major at weight 100 — below
    /// every existing rung, so it fires only on the hands that would otherwise
    /// pass out their resolved partscore.  Withheld from `X (2♥) - (2♠)`,
    /// where opener's pass has already denied four hearts.  Opener answers with
    /// game from the top of the range, the invitational raise where there is
    /// room, else a pass.  This is the control arm of
    /// `scripts/ab-2d-multi-doubler.sh`.
    #[arg(long, default_value_t = false)]
    no_ns_multi_doubler_major: bool,
    /// Split responder's `P`/`X` over their `(2♦)` Multi by **information**:
    /// `X` = game values, or an invitation with a four-card major
    ///
    /// `hcp(10..) | (hcp(8..=9) & (len(♥, 4..) | len(♠, 4..)))` in place of
    /// Kokish–Kraft's flat `hcp 8+`.  The 8–9-no-major hands take the neutral
    /// pass instead, where the delayed `2NT` becomes a live invitation (opener
    /// accepts on `hcp 16+`), and the doubler's natural other major becomes
    /// required at weight 148 — above the natural `2NT` — on three of the four
    /// resolved paths.  Implies `--ns-multi-doubler-major`'s rung and
    /// re-weights it; the two emit one rung.  Needs the Kokish–Kraft counter
    /// and their declared `(2♦)` Multi to do anything.
    #[arg(long, default_value_t = false)]
    ns_multi_px_split: bool,

    /// The `4m` slam try above a completed **Puppet** minor transfer
    /// (`1NT - 2♠`→♣, `1NT - 2NT`→♦): a `points` floor (default `13`), or `off`
    ///
    /// The shipped constructive twin of
    /// `--ns-multi-minor-slam-try`.  Authors the rung in all four Puppet seats
    /// plus opener's answer (`4NT` RKCB on `size_ask_accept_floor`, else `5m`).
    /// The European arm is an opponent model and never carries it.
    #[arg(long, default_value = "13", value_name = "off|POINTS")]
    ns_minor_transfer_slam_try: String,

    /// Leave opener's N1j Landy `4m` slam try to the floor instead of using
    /// the shipped authored answer (`1NT (2♣) 2NT - 3♣ - 4♣ -`)
    ///
    /// The rung itself has shipped since N1; this restores the former
    /// floor-owned seat above it — and the floor can never keycard in a
    /// disturbed auction (`docs/minor-transfer-slam.md`).
    #[arg(long, default_value_t = false)]
    no_ns_landy_minor_slam_answer: bool,

    /// Author opener's notrump out over the N1j Landy doubler's advanced major
    /// (`1NT (2♣) X (2♥)`, `1NT (2♣) X (2♠)`)
    ///
    /// `competition.landy_doubler_notrump`, default **off**: `3NT` on
    /// `hcp(16..) & stopper_in(their major)` where the seat is the floor's
    /// today and it passes.  The treatment arm of
    /// `scripts/ab-landy-doubler-nt.sh`.
    #[arg(long, default_value_t = false)]
    ns_landy_doubler_notrump: bool,
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
    agreements.decision.their.two_clubs_landy = args.their_2c_landy;
    agreements.decision.reading.their_multi_advance_reading = args.ns_their_multi_advance_read;
    agreements.decision.reading.their_multi_double_reading = args.ns_their_multi_double_read;
    agreements.competition.multi_kokish_kraft = !args.no_ns_multi_kokish_kraft;
    agreements.competition.multi_minor_slam_try = match args.ns_multi_minor_slam_try.as_str() {
        "off" => None,
        n => Some(
            n.parse()
                .expect("--ns-multi-minor-slam-try must be off or a points floor"),
        ),
    };
    agreements.competition.multi_doubler_major = !args.no_ns_multi_doubler_major;
    agreements.competition.multi_px_split = args.ns_multi_px_split;
    agreements.notrump.minor_transfer_slam_try = match args.ns_minor_transfer_slam_try.as_str() {
        "off" => None,
        n => Some(
            n.parse()
                .expect("--ns-minor-transfer-slam-try must be off or a points floor"),
        ),
    };
    agreements.competition.landy_minor_slam_answer = !args.no_ns_landy_minor_slam_answer;
    agreements.competition.landy_doubler_notrump = args.ns_landy_doubler_notrump;
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
