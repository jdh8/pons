//! Opener's third call after responder sets trump at `1M - 2r - R - 3M`
//!
//! Gated by [`set_opener_third`].  Read at book-construction time.  **On by
//! default** — but see the caveat, it is a deletion candidate blocked on a
//! floor capability, not a settled node.

use super::*;
use crate::bidding::american::slam;

std::thread_local! {
    /// Whether opener authors a third-call table after trump is agreed at
    /// `1M - 2r - R - 3M`.  On by default; the deletion measures positive but
    /// strands every slam at this node, see [`set_opener_third`].
    static OPENER_THIRD: Cell<bool> = const { Cell::new(true) };
}

/// Toggle opener's third call after responder sets trump: `1M - 2r - R - 3M`
///
/// Read at book-construction time.  **On by default** — but see the caveat, it
/// is a deletion candidate blocked on a floor capability, not a settled node.
///
/// Two rules — 4NT RKCB on `points(15..)`, else an unconditional `4M` — the
/// retired game backstop's signature: a raw point threshold, no shape or
/// control term, and every cue-bid and five-level call at −∞ at depth 4.
///
/// Deleting it *measures* **+0.437/+0.527 plain, +0.524/+0.637 PD** IMPs per
/// divergent board NV/vul (`ab-major-continuations`, 2,000,000 boards per arm
/// per vulnerability, seed 1784484826, 971 divergent = 0.05%) — +0.0002/+0.0003
/// per board, the same sign on all four arms.
///
/// **It is not shipped anyway.** With the node gone the floor never asks
/// keycards here at all: it signs off in `4M` on a 26-count opposite a
/// game-forcing two-over-one, so slam becomes unreachable at this node. That is
/// the backstop lesson again — deleting a node deletes the invariant it held by
/// omission, and here the invariant is "opener can still try for slam". A
/// +0.0003 IMPs/board gain does not buy a total capability loss.
///
/// The architecturally correct fix, if this is ever resumed, is the
/// [`set_two_over_one_force`][crate::bidding::instinct::set_two_over_one_force]
/// pattern: delete the node *and* teach `instinct()` to ask keycards on a
/// controls-and-fit test at an agreed-trump game force, which should beat both
/// arms. Only the raw point threshold is obviously wrong; the ask itself is
/// load-bearing.
///
/// The RKCB answer rows (`slam::rkcb_rows`) are independent of this knob.
pub fn set_opener_third(on: bool) {
    OPENER_THIRD.with(|cell| cell.set(on));
}

pub(super) fn opener_third_enabled() -> bool {
    OPENER_THIRD.with(Cell::get)
}

/// Opener's third call after `1M - 2r - R - 3M`
///
/// Once trump has been set at three of the major, opener shows strength:
/// the 4NT key card ask on extras or a sign-off at four of the major.
///
/// No [`Pass`][Call::Pass] rule.
fn opener_third(major: Suit) -> Rules {
    let major_strain = Strain::from(major);
    Rules::new()
        .rule(call(4, Strain::Notrump), 100, points(15..))
        .alert(slam::RKCB)
        .rule(call(4, major_strain), 50, hcp(0..))
}

/// Opener's third-call table after responder agrees the opening major
pub(crate) fn opener_third_continuations() -> Package {
    Package {
        name: "two-over-one-opener-third",
        gate: |agreements| agreements.build.game_force.opener_third,
        entries: |_| {
            let mut entries = Vec::new();
            for major in [Suit::Spades, Suit::Hearts] {
                for resp in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
                    if Strain::from(resp) >= Strain::from(major) {
                        continue;
                    }
                    let prefix = format!(
                        "P* {} - {} -",
                        call(1, Strain::from(major)),
                        call(2, Strain::from(resp)),
                    );
                    let three_major = Bid::new(3, Strain::from(major));
                    for rebid_call in distinct_calls(&opener_rebid(major, resp)) {
                        if let Call::Bid(rebid_bid) = rebid_call
                            && rebid_bid < three_major
                        {
                            entries.extend(rows_of(
                                Pattern::node(&format!("{prefix} {rebid_call} - {three_major} -")),
                                opener_third(major),
                            ));
                        }
                    }
                }
            }
            entries
        },
    }
}
