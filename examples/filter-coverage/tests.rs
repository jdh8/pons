use super::*;
use common::Board;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};

fn auction(text: &str) -> Auction {
    let mut auction = Auction::new();
    for token in text.split_whitespace() {
        let call: Call = token.parse().expect("a call");
        auction.try_push(call).expect("a legal call");
    }
    auction
}

/// North is the only 1NT candidate; its LHO (East) is Landy-shaped.
const LANDY_LHO: &str = "N:AKQ2.K32.K32.432 JT987.AQJT.A5.65 654.987.QJT9.KQJ 3.654.8764.AT987";
/// The same hands with East and West swapped: the Landy hand is RHO.
const LANDY_RHO: &str = "N:AKQ2.K32.K32.432 3.654.8764.AT987 654.987.QJT9.KQJ JT987.AQJT.A5.65";

fn dump(boards: &[(&str, &str)], gen_args: &[&str]) -> Dump {
    Dump {
        our_label: "us".into(),
        their_label: "them".into(),
        vulnerability: AbsoluteVulnerability::NONE,
        seed: Some(1),
        gen_args: gen_args.iter().map(|&a| a.into()).collect(),
        declared_european_minors: None,
        boards: boards
            .iter()
            .map(|&(deal, calls)| Board {
                deal: deal.parse::<FullDeal>().unwrap(),
                dealer: Seat::North,
                table_a: auction(calls),
                table_b: Auction::new(),
            })
            .collect(),
    }
}

/// Board 0 moves the contract on a Landy-accepted deal, board 1 moves only
/// the auction on a Landy-rejected one, board 2 does not diverge.
#[test]
fn splits_the_divergent_set_by_scan() {
    let on = dump(
        &[
            (LANDY_LHO, "1NT P P P"),
            (LANDY_RHO, "1C P 1D P 2D P P P"),
            (LANDY_RHO, "1NT P P P"),
        ],
        &[],
    );
    let off = dump(
        &[
            (LANDY_LHO, "1NT 2C P P P"),
            (LANDY_RHO, "P P 1D P 2D P P P"),
            (LANDY_RHO, "1NT P P P"),
        ],
        &[],
    );
    let landy = Scan {
        filter: Filter::Landy,
        wide: false,
    };
    let one_nt = Scan {
        filter: Filter::OneNt,
        wide: true,
    };
    let report = coverage(&on, &off, &[landy, one_nt]).unwrap();
    assert_eq!(report.boards, 3);
    assert_eq!(report.auction_divergent, [0, 1]);
    assert_eq!(report.contract_divergent, [true, false]);
    assert_eq!(
        report.rows[0],
        (
            landy,
            Row {
                accepted: 1,
                auction_accepted: 1,
                auction_rejected: vec![1],
                contract_accepted: 1,
                contract_rejected: 0,
            }
        )
    );
    assert_eq!(report.rows[1].1.accepted, 3);
    assert!(report.rows[1].1.auction_rejected.is_empty());
}

#[test]
fn refuses_unpaired_or_filtered_arms() {
    let one = dump(&[(LANDY_LHO, "1NT P P P")], &[]);
    let other = dump(&[(LANDY_RHO, "1NT P P P")], &[]);
    assert!(coverage(&one, &other, &Scan::all()).is_err());

    let filtered = dump(
        &[(LANDY_LHO, "1NT P P P")],
        &["--count", "1", "--filter-landy"],
    );
    assert!(check_unfiltered(&filtered, "f.json").is_err());
    assert!(check_unfiltered(&one, "u.json").is_ok());
}

/// Strict and wide rows for the three scans `--wide-1nt` changes, one row
/// each for the two that are always wide.
#[test]
fn every_flag_has_its_rows() {
    let labels: Vec<String> = Scan::all().into_iter().map(Scan::label).collect();
    assert_eq!(
        labels,
        [
            "--filter-1nt",
            "--filter-1nt --wide-1nt",
            "--filter-landy",
            "--filter-landy --wide-1nt",
            "--filter-preempt",
            "--filter-preempt --wide-1nt",
            "--filter-multi-slam",
            "--filter-lebensohl-raise",
        ]
    );
}
