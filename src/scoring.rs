//! Scoring a bid-out board
//!
//! Per-board primitives connecting the three stages of a simulated board:
//! a completed [`Auction`] yields the [`final_contract`], a double-dummy
//! [`TrickCountTable`] prices it — as either [`ns_score_contract`] (plain DD,
//! the contract's *actual* penalty) or [`ns_score_bid`] (perfect-defense
//! doubling, for evaluating a *call*) — and a score difference between two
//! tables converts to [`imps`].  Promoted from the `instinct-floor` example so
//! every simulation harness shares one scorer.
//!
//! Two scorers because there are two questions. **Scoring a reached contract**
//! (a duplicate A/B result) honors the penalty the auction actually produced —
//! that is [`ns_score_contract`], plain double-dummy. **Evaluating a call**
//! (the EV rollout in [`crate::bidding::ev()`], a contract-choice probe) assumes
//! perfect-defense doubling: a contract that fails double-dummy is scored
//! *doubled*, a making one *undoubled*, regardless of the auction — because the
//! cardplay already assumes optimal defense, so the doubling must too, or a
//! failing sacrifice prices far too cheaply.  That is [`ns_score_bid`], which
//! takes a [`Bid`] (not a [`Contract`]) precisely because it derives the
//! penalty itself.
//!
//! A third scorer, [`ns_score_pd`], bridges the two: it scores a *settled*
//! contract under perfect defense but **carries the actual `X`/`XX`** (which
//! cannot be taken back), so it is the right scorer for an A/B where a side may
//! *defend* by passing — putting real doubled contracts on the table.

use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Bid, Contract, Penalty, Seat};
use ddss::TrickCountTable;

/// The seat acting at `index` calls after `dealer`
const fn seat_at(dealer: Seat, index: usize) -> Seat {
    Seat::ALL[(dealer as usize + index) % 4]
}

/// The final contract and absolute declarer, or [`None`] for a pass-out
///
/// The contract is the last bid with any doubles after it; the declarer is
/// the first player on the declaring side to have bid its strain, located
/// with [`Auction::declarer`] and converted to an absolute seat from
/// `dealer`.
#[must_use]
pub fn final_contract(auction: &Auction, dealer: Seat) -> Option<(Contract, Seat)> {
    let mut last_bid: Option<Bid> = None;
    let mut penalty = Penalty::Undoubled;

    for &call in auction {
        match call {
            Call::Bid(bid) => {
                last_bid = Some(bid);
                penalty = Penalty::Undoubled;
            }
            Call::Double => penalty = Penalty::Doubled,
            Call::Redouble => penalty = Penalty::Redoubled,
            Call::Pass => {}
        }
    }

    let bid = last_bid?;
    let index = auction.declarer()?;
    Some((Contract { bid, penalty }, seat_at(dealer, index)))
}

/// Signed-for-NS double-dummy score of `bid` played by `declarer` with the given
/// `penalty`; the shared tail of both public scorers.
fn ns_score_with(
    bid: Bid,
    declarer: Seat,
    penalty: Penalty,
    table: &TrickCountTable,
    vul: AbsoluteVulnerability,
) -> i64 {
    let tricks = u8::from(table[bid.strain].get(declarer));
    let declarer_vul = vul.contains(match declarer {
        Seat::North | Seat::South => AbsoluteVulnerability::NS,
        Seat::East | Seat::West => AbsoluteVulnerability::EW,
    });
    let score = i64::from(Contract { bid, penalty }.score(tricks, declarer_vul));
    match declarer {
        Seat::North | Seat::South => score,
        Seat::East | Seat::West => -score,
    }
}

/// Whether `bid` fails double-dummy when played by `declarer` (tricks short of
/// the book-plus-level needed)
fn fails_dd(bid: Bid, declarer: Seat, table: &TrickCountTable) -> bool {
    let tricks = u8::from(table[bid.strain].get(declarer));
    u32::from(tricks) < 6 + u32::from(bid.level.get())
}

/// Plain double-dummy NS score of a reached contract (0 for a pass-out): the
/// contract's *actual* penalty, scored at the declaring side's vulnerability and
/// signed for North/South (positive is good for NS).
///
/// This is the scorer for a **duplicate A/B result** — the contract was bid and
/// (re)doubled in the simulation, so it is priced exactly as it stands, with no
/// synthetic doubling.  Takes the [`Option`] straight from [`final_contract`].
/// To evaluate a *call* against perfect defense instead, use [`ns_score_bid`].
#[must_use]
pub fn ns_score_contract(
    result: Option<(Contract, Seat)>,
    table: &TrickCountTable,
    vul: AbsoluteVulnerability,
) -> i64 {
    let Some((contract, declarer)) = result else {
        return 0;
    };
    ns_score_with(contract.bid, declarer, contract.penalty, table, vul)
}

/// Perfect-defense NS score of a `bid` played by `declarer` (0 for a pass-out):
/// the contract is scored **doubled if it fails double-dummy, undoubled if it
/// makes**, regardless of any auction penalty — hence a [`Bid`], not a
/// [`Contract`].
///
/// This is the scorer for **evaluating a call**: in a double-dummy model the
/// opponents always hold the red card, so a failing overbid must be priced
/// doubled (an opponent who *cannot* double is never the case at a real table),
/// while a making contract is never doubled (that only helps declarer).  The
/// rule is symmetric — it doubles either side's failing contract — so it sharpens
/// both our overbids and our defense of theirs.  Used by the EV rollout in
/// [`crate::bidding::ev()`] and contract-choice probes.
///
/// [`stats::average_ns_par`][crate::stats::average_ns_par] makes the same
/// assumption for par scoring (there as `min(undoubled, doubled)` on the
/// expected score); this is its per-deal analogue.
#[must_use]
pub fn ns_score_bid(
    result: Option<(Bid, Seat)>,
    table: &TrickCountTable,
    vul: AbsoluteVulnerability,
) -> i64 {
    let Some((bid, declarer)) = result else {
        return 0;
    };
    let penalty = if fails_dd(bid, declarer, table) {
        Penalty::Doubled
    } else {
        Penalty::Undoubled
    };
    ns_score_with(bid, declarer, penalty, table, vul)
}

/// Perfect-defense NS score of a *settled* contract, **carrying its actual
/// double/redouble**: like [`ns_score_bid`] it doubles a contract that fails
/// double-dummy, but a double or redouble already on the table is locked in and
/// kept even when the contract makes.
///
/// This is the scorer for "pass = play the top bid": when an auction settles, the
/// contract on the table is played with whatever penalty it actually carries —
/// `X`/`XX` cannot be taken back, so a doubled contract that *makes* keeps its
/// bonus.  Perfect defense only ever *adds* a double (to a failing **undoubled**
/// contract), never removes one — the penalty is therefore the more severe of the
/// table penalty and the fails-double-dummy floor.  Use this (not
/// [`ns_score_bid`]) to score a duplicate A/B once a side may *defend* by passing,
/// which puts real doubled contracts on the table.
///
/// [`Penalty`] is not `Ord`, so the floor is spelled out: a failing undoubled
/// contract becomes [`Penalty::Doubled`]; an already doubled/redoubled one keeps
/// its (more severe) penalty; a making contract keeps the table penalty verbatim.
#[must_use]
pub fn ns_score_pd(
    result: Option<(Contract, Seat)>,
    table: &TrickCountTable,
    vul: AbsoluteVulnerability,
) -> i64 {
    let Some((contract, declarer)) = result else {
        return 0;
    };
    let penalty = if fails_dd(contract.bid, declarer, table) {
        match contract.penalty {
            Penalty::Undoubled => Penalty::Doubled,
            doubled => doubled,
        }
    } else {
        contract.penalty
    };
    ns_score_with(contract.bid, declarer, penalty, table, vul)
}

/// Upper bounds (exclusive) of the point difference for 0, 1, 2, … IMPs
const IMP_BOUNDS: [i64; 24] = [
    20, 50, 90, 130, 170, 220, 270, 320, 370, 430, 500, 600, 750, 900, 1100, 1300, 1500, 1750,
    2000, 2250, 2500, 3000, 3500, 4000,
];

/// Convert a point difference to International Match Points
///
/// The standard WBF scale: ±20 points is the first IMP, ±4000 caps at 24.
/// The sign of the difference is preserved.
// ponytail: the `try_from` cannot fail — `magnitude` counts entries of a
// fixed 24-element array, so it is always in `0..=24` and fits an `i64`.
#[allow(clippy::missing_panics_doc)]
#[must_use]
pub fn imps(diff: i64) -> i64 {
    let magnitude = IMP_BOUNDS
        .iter()
        .take_while(|&&bound| diff.abs() >= bound)
        .count();
    i64::try_from(magnitude).expect("at most 24 IMPs") * diff.signum()
}
