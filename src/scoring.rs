//! Scoring a bid-out board
//!
//! Per-board primitives connecting the three stages of a simulated board:
//! a completed [`Auction`] yields the [`final_contract`], a double-dummy
//! [`TrickCountTable`] prices it as a signed [`ns_score`], and a score
//! difference between two tables converts to [`imps`].  Promoted from the
//! `instinct-floor` example so every simulation harness shares one scorer.

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

    for &call in auction.iter() {
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

/// Double-dummy NS score of a final contract (0 for a pass-out)
///
/// Looks the declarer's tricks up in the solved `table`, scores the contract
/// at the declaring side's vulnerability, and signs the result for
/// North/South: positive is good for NS.  Takes the [`Option`] straight from
/// [`final_contract`] so a passed-out board scores 0.
#[must_use]
pub fn ns_score(
    result: Option<(Contract, Seat)>,
    table: &TrickCountTable,
    vul: AbsoluteVulnerability,
) -> i64 {
    let Some((contract, declarer)) = result else {
        return 0;
    };
    let tricks = u8::from(table[contract.bid.strain].get(declarer));
    let declarer_vul = vul.contains(match declarer {
        Seat::North | Seat::South => AbsoluteVulnerability::NS,
        Seat::East | Seat::West => AbsoluteVulnerability::EW,
    });
    let score = i64::from(contract.score(tricks, declarer_vul));
    match declarer {
        Seat::North | Seat::South => score,
        Seat::East | Seat::West => -score,
    }
}

/// NS score assuming **perfect-defense doubling**: a contract that fails
/// double-dummy is scored *doubled*; a making contract keeps its auction
/// penalty.
///
/// The cardplay rollout already assumes optimal defense, yet leaves the
/// *doubling* decision to a weak continuation policy that under-doubles — so a
/// failing sacrifice looks far cheaper than it would at a table where the
/// defenders double a contract that is going down.  This applies the same
/// perfect-defense assumption to the penalty: the side that is *not* declaring
/// doubles iff the contract fails.  Symmetric (it doubles either side's failing
/// contract), so it sharpens both our overbids and our defense of theirs, and it
/// never rewards doubling a making contract.
///
/// [`stats::average_ns_par`][crate::stats::average_ns_par] makes the same
/// assumption for par scoring (there as `min(undoubled, doubled)` on the
/// expected score); this is its per-deal analogue.
#[must_use]
pub fn ns_score_doubling_failures(
    result: Option<(Contract, Seat)>,
    table: &TrickCountTable,
    vul: AbsoluteVulnerability,
) -> i64 {
    let Some((contract, declarer)) = result else {
        return 0;
    };
    let tricks = u8::from(table[contract.bid.strain].get(declarer));
    let failing = u32::from(tricks) < 6 + u32::from(contract.bid.level.get());
    let penalty = if failing {
        Penalty::Doubled
    } else {
        contract.penalty
    };
    let contract = Contract {
        bid: contract.bid,
        penalty,
    };
    let declarer_vul = vul.contains(match declarer {
        Seat::North | Seat::South => AbsoluteVulnerability::NS,
        Seat::East | Seat::West => AbsoluteVulnerability::EW,
    });
    let score = i64::from(contract.score(tricks, declarer_vul));
    match declarer {
        Seat::North | Seat::South => score,
        Seat::East | Seat::West => -score,
    }
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
#[must_use]
pub fn imps(diff: i64) -> i64 {
    let magnitude = IMP_BOUNDS
        .iter()
        .take_while(|&&bound| diff.abs() >= bound)
        .count();
    i64::try_from(magnitude).expect("at most 24 IMPs") * diff.signum()
}
