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
//! Two scorers because there are two questions. **Plain double dummy**
//! ([`ns_score_contract`]) honors the penalty the auction actually produced.
//! **Perfect defense** ([`ns_score_bid`]) scores the auction as double-dummy
//! opponents would have doubled it: a contract that fails double-dummy is
//! *doubled*, a making one *undoubled*, whatever `X`/`XX` the table carried —
//! the cardplay already assumes optimal defense, so the doubling must too, or a
//! failing sacrifice prices far too cheaply; and a double no double-dummy
//! opponent would have made (of a making contract) is erased by the same rule.
//! The EV rollout in [`crate::bidding::ev_all()`] applies it to a *call*, and
//! [`ns_score_pd_tricks`] to a single-dummy trick count.
//!
//! Until 2026-09-27 a third scorer, `ns_score_pd`, kept a real `X`/`XX` on a
//! making contract.  It was retired for the pure double-dummy rule (see
//! `docs/ben-gap-campaign.md`, "the two PD scorers").

#[cfg(feature = "dd")]
use contract_bridge::AbsoluteVulnerability;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{Bid, Contract, Penalty, Seat};
#[cfg(feature = "dd")]
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
#[cfg(feature = "dd")]
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
#[cfg(feature = "dd")]
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
#[cfg(feature = "dd")]
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

/// Perfect-defense NS score of a reached contract (0 for a pass-out): scored
/// **doubled if it fails double-dummy, undoubled if it makes**, whatever
/// `X`/`XX` the auction carried — the contract's table penalty is ignored by
/// design.
///
/// This is the double-dummy-bidding rule: in a double-dummy model the
/// opponents always hold the red card, so a failing overbid must be priced
/// doubled (an opponent who *cannot* double is never the case at a real table),
/// while a making contract is never doubled (that only helps declarer, and a
/// double-dummy opponent would not have offered it).  The rule is symmetric —
/// it doubles either side's failing contract and erases either side's double of
/// a making one — so it sharpens both our overbids and our defense of theirs.
/// The **PD** column of every duplicate A/B, the EV rollout in
/// [`crate::bidding::ev_all()`] and the contract-choice probes all use it.
///
/// [`stats::average_ns_par`][crate::stats::average_ns_par] makes the same
/// assumption for par scoring (there as `min(undoubled, doubled)` on the
/// expected score); this is its per-deal analogue.
#[cfg(feature = "dd")]
#[must_use]
pub fn ns_score_bid(
    result: Option<(Contract, Seat)>,
    table: &TrickCountTable,
    vul: AbsoluteVulnerability,
) -> i64 {
    let Some((contract, declarer)) = result else {
        return 0;
    };
    let penalty = if fails_dd(contract.bid, declarer, table) {
        Penalty::Doubled
    } else {
        Penalty::Undoubled
    };
    ns_score_with(contract.bid, declarer, penalty, table, vul)
}

/// Signed-for-NS score of a reached contract given declarer's *actual* tricks
///
/// The pricing tail of the single-dummy scorers
/// ([`single_dummy_lead_tricks`][crate::single_dummy_lead_tricks],
/// [`single_dummy_playout`][crate::single_dummy_playout]): those return a trick
/// count on the actual deal, and this scores it — the contract as it stands
/// (its auction penalty), at the declaring side's vulnerability, signed for
/// North/South.  The single-dummy sibling of [`ns_score_contract`], which
/// reads its tricks from a double-dummy table instead.
#[cfg(feature = "dd")]
#[must_use]
pub fn ns_score_tricks(
    contract: Contract,
    declarer: Seat,
    tricks: u8,
    vul: AbsoluteVulnerability,
) -> i64 {
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

/// Perfect-defense NS score of a reached contract given declarer's *actual*
/// tricks: the single-dummy analogue of [`ns_score_bid`].
///
/// Like [`ns_score_tricks`] it prices an explicit trick count on the real deal,
/// but under the double-dummy-bidding rule: a contract that **fails on those
/// tricks** is scored doubled and one that makes undoubled, whatever `X`/`XX`
/// the table carried.  This layers the perfect-defense downside onto the
/// realistic single-dummy lead: concealment still earns its extra makes, but
/// the games that fail anyway pay the doubled penalty a real opponent would
/// exact.  The SD arbiter for a game-reaching treatment — plain single-dummy
/// relaxes only the defenders' lead and never punishes the failures, so it
/// flatters aggression.
#[cfg(feature = "dd")]
#[must_use]
pub fn ns_score_pd_tricks(
    contract: Contract,
    declarer: Seat,
    tricks: u8,
    vul: AbsoluteVulnerability,
) -> i64 {
    let fails = u32::from(tricks) < 6 + u32::from(contract.bid.level.get());
    let penalty = if fails {
        Penalty::Doubled
    } else {
        Penalty::Undoubled
    };
    ns_score_tricks(
        Contract {
            bid: contract.bid,
            penalty,
        },
        declarer,
        tricks,
        vul,
    )
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
