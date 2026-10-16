//! The self-play blind leader must see the declaring side's *unalerted* calls
//! (docs/research-method-review.md §4).  Before 2026-10-16 every seat read
//! through one default-flag book, which decodes the opponents' calls at
//! `ReadingScope::Alerted` only: the candidate's natural weak two fell to the
//! natural walk (`points 5..=10`, any HCP), so a `--weak-two-hcp` band never
//! reached the leader's sampler.

use super::*;
use pons::bidding::{Envelope, Relative};

fn auction(text: &str) -> Auction {
    let mut auction = Auction::new();
    for token in text.split_whitespace() {
        let call: Call = token.parse().expect("a call");
        auction.try_push(call).expect("a legal call");
    }
    auction
}

/// What the blind leader at `table` (`[table_b, table_a]`) reads of its RHO,
/// the weak-two opener
fn opener(text: &str, readers: &[TableReader; 2], table: usize) -> Envelope {
    let inferences = readers[table]
        .lead_inputs(&auction(text), Seat::North, AbsoluteVulnerability::NONE)
        .expect("a contract")
        .2;
    *inferences.get(Relative::Rho)
}

/// North (dealer) opens: table A seats the candidate North–South, East leads.
const CANDIDATE_NS: &str = "2H P P P";
/// East opens: table B seats the candidate East–West, South leads.
const CANDIDATE_EW: &str = "P 2H P P P";

#[test]
fn leader_reads_the_candidate_weak_two_band() {
    for (lo, hi) in [(5, 10), (11, 15)] {
        let partnerships = build_partnerships(Arms::WeakTwoHcp { band: (lo, hi) });
        let readers = table_readers(&partnerships, false);
        for (text, table) in [(CANDIDATE_NS, 1), (CANDIDATE_EW, 0)] {
            let shown = opener(text, &readers, table);
            assert_eq!(
                (shown.strength.hcp.min, shown.strength.hcp.max),
                (lo, hi),
                "{text} at table {table}"
            );
            assert_eq!(shown.length(contract_bridge::Suit::Hearts).min, 6);
        }
        // The baseline pair's weak two reads off the baseline's own book.
        let baseline = opener(CANDIDATE_NS, &readers, 0);
        assert_eq!(
            (baseline.strength.points.min, baseline.strength.points.max),
            (5, 10)
        );
    }
}

/// The defect itself, pinned so `--legacy-leader-reading` keeps reproducing
/// the old numbers: the legacy leader cannot tell the bands apart.
#[test]
fn legacy_leader_is_blind_to_the_band() {
    let read = |band| {
        let partnerships = build_partnerships(Arms::WeakTwoHcp { band });
        let readers = table_readers(&partnerships, true);
        [
            opener(CANDIDATE_NS, &readers, 1),
            opener(CANDIDATE_EW, &readers, 0),
        ]
    };
    let (sane, absurd) = (read((5, 10)), read((11, 15)));
    assert_eq!(sane, absurd);
    assert_eq!(
        sane[0].strength.hcp.max, 37,
        "the natural walk caps nothing"
    );
}
