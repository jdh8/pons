//! How often does the RKCB answerer overrule the asker's placement?
//!
//! `slam::rkcb_rows` authors nothing for the answerer after the asker places
//! the contract at five or six of trump, so that seat falls to the floor.
//! `slam::rkcb_answerer_rows` authors the pass, but only the forcing-NT
//! jump-shift lane wires it in.  This census bids uncontested self-play deals
//! with the default system and, at every `4NT - 5x - 5T/6T` window, records
//! the answerer's next call, keyed by the auction up to the ask, then prices
//! two counterfactuals double dummy: the answerer passing the signoff
//! (`rkcb_answerer_rows` wired everywhere) and the asker bidding six itself
//! (the `asker_after_5d` / `asker_after_5h` fix, see `asker_fix`).
//!
//! ```sh
//! cargo run --release --example probe-rkcb-answerer -- -c 400000
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Bid, Hand, Rank, Seat, Strain, Suit};
use pons::american;
use pons::scoring::final_contract;
use rayon::prelude::*;
use std::collections::BTreeMap;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{
    Reached, auction_key, bid_uncontested, hand_hcp, report_brackets, seat_to_act, seeded_deals,
};
use pons::bidding::context::relative;

#[derive(Parser)]
struct Args {
    /// Deals to bid
    #[arg(short, long, default_value = "400000")]
    count: usize,

    /// Seed base; random when omitted
    #[arg(short, long)]
    seed: Option<u64>,
}

/// `(lane, tail, answerer out-points asker, answerer at an exact book node,
/// deal index, [reached, reached had the answerer passed])` at each window
/// whose placement is an unalerted call from an exact book node — the asker's
/// signoff out of `rkcb_rows`, not a relay or a floor call
type Hit = (
    String,
    String,
    bool,
    bool,
    usize,
    [Reached; 2],
    Seat,
    Option<[Reached; 2]>,
);

/// Keycards: the four aces and the trump king
fn keycards(hand: Hand, trump: Suit) -> usize {
    Suit::ASC
        .iter()
        .filter(|&&s| hand[s].contains(Rank::A))
        .count()
        + usize::from(hand[trump].contains(Rank::K))
}

/// The asker-table fix under test: over `5♦` one keycard reads partner for
/// three, and over `5♥` two keycards with the trump queen make four plus the
/// queen — both bid six instead of signing off
fn asker_fix(answer: Bid, hand: Hand, trump: Suit) -> bool {
    match answer.strain {
        Strain::Diamonds => keycards(hand, trump) == 1,
        Strain::Hearts => keycards(hand, trump) == 2 && hand[trump].contains(Rank::Q),
        _ => false,
    }
}

/// `calls` then pass it out
fn passed_out(calls: &[Call]) -> Auction {
    let mut auction = Auction::new();
    for &call in calls.iter().chain(&[Call::Pass; 3]) {
        if auction.has_ended() {
            break;
        }
        auction.push(call);
    }
    auction
}

fn main() {
    let args = Args::parse();
    let base = args.seed.unwrap_or_else(rand::random);
    let vul = AbsoluteVulnerability::NONE;
    let partnership = american(&pons::bidding::agreements::Agreements::default()).bind();

    let deals = seeded_deals(base, args.count);
    let hits: Vec<Hit> = deals
        .par_iter()
        .enumerate()
        .flat_map_iter(|(index, deal)| {
            let dealer = Seat::ALL[index % 4];
            let auction = bid_uncontested(&partnership, dealer, vul, deal);
            let calls: Vec<Call> = auction.iter().copied().collect();
            let mut hits = Vec::new();
            for i in 0..calls.len().saturating_sub(6) {
                let (Call::Bid(ask), Call::Bid(answer), Call::Bid(placement)) =
                    (calls[i], calls[i + 2], calls[i + 4])
                else {
                    continue;
                };
                let window = ask.level.get() == 4
                    && ask.strain == Strain::Notrump
                    && answer.level.get() == 5
                    && answer.strain != Strain::Notrump
                    && matches!(placement.level.get(), 5 | 6)
                    && placement.strain != Strain::Notrump
                    && [1, 3, 5].iter().all(|&k| calls[i + k] == Call::Pass);
                if !window {
                    continue;
                }
                let (asker_seat, answerer_seat) =
                    (seat_to_act(dealer, i), seat_to_act(dealer, i + 2));
                let (asker, answerer) = (deal[asker_seat], deal[answerer_seat]);
                let signoff = partnership
                    .explain_call(
                        asker,
                        relative(vul, asker_seat),
                        &calls[..i + 4],
                        calls[i + 4],
                    )
                    .is_some_and(|(prov, rule)| {
                        prov.fallback.is_none() && rule.is_some_and(|r| r.alert.is_none())
                    });
                if !signoff {
                    continue;
                }
                let exact = partnership
                    .explain_call(
                        answerer,
                        relative(vul, answerer_seat),
                        &calls[..i + 6],
                        calls[i + 6],
                    )
                    .is_some_and(|(prov, _)| prov.fallback.is_none());
                let passed = passed_out(&calls[..i + 6]);
                let fix = Suit::try_from(placement.strain)
                    .ok()
                    .filter(|&trump| placement.level.get() == 5 && asker_fix(answer, asker, trump))
                    .map(|_| {
                        let mut six = calls[..i + 4].to_vec();
                        six.push(Call::Bid(Bid::new(6, placement.strain)));
                        [
                            final_contract(&auction, dealer),
                            final_contract(&passed_out(&six), dealer),
                        ]
                    });
                hits.push((
                    auction_key(&calls[..=i]),
                    auction_key(&calls[i + 2..=i + 6]),
                    hand_hcp(answerer) > hand_hcp(asker),
                    exact,
                    index,
                    [
                        final_contract(&auction, dealer),
                        final_contract(&passed, dealer),
                    ],
                    asker_seat,
                    fix,
                ));
            }
            hits
        })
        .collect();

    let windows = hits.len();
    let overrules: Vec<&Hit> = hits.iter().filter(|h| !h.1.ends_with('-')).collect();
    let stronger = hits.iter().filter(|h| h.2).count();
    let exact = hits.iter().filter(|h| h.3).count();
    println!(
        "=== RKCB answerer census: {} deals, seed {base} ===",
        args.count
    );
    println!("book signoff windows   {windows:8}");
    println!("answerer at book node  {exact:8}");
    println!("answerer out-points    {stronger:8}");
    println!("answerer overrules     {:8}", overrules.len());

    let mut lanes: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut tails: BTreeMap<&str, usize> = BTreeMap::new();
    for h in &hits {
        let e = lanes.entry(&h.0).or_default();
        e.0 += 1;
        if !h.1.ends_with('-') {
            e.1 += 1;
            *tails.entry(&h.1).or_default() += 1;
        }
    }
    let mut lanes: Vec<_> = lanes.into_iter().collect();
    lanes.sort_by_key(|&(_, (n, o))| std::cmp::Reverse((o, n)));
    println!("\n--- lanes (windows, overrules) ---");
    for (lane, (n, o)) in lanes.iter().take(30) {
        println!("{n:6} {o:6}  {lane}");
    }
    println!("\n--- overruling tails ---");
    let mut tails: Vec<_> = tails.into_iter().collect();
    tails.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    for (tail, n) in tails.iter().take(20) {
        println!("{n:6}  {tail}");
    }

    // ponytail: first overruling window per board only; a later one cannot occur
    // once the pass ends the auction.
    let mut seen = std::collections::HashSet::new();
    let divergent_hits: Vec<&&Hit> = overrules
        .iter()
        .filter(|h| h.5[0] != h.5[1] && seen.insert(h.4))
        .collect();
    let (deals_off, contracts): (Vec<_>, Vec<_>) =
        divergent_hits.iter().map(|h| (deals[h.4], h.5)).unzip();
    let divergent: Vec<usize> = (0..contracts.len()).collect();
    let tables = ddss::Solver::lock(None).solve_deals(&deals_off, ddss::NonEmptyStrainFlags::ALL);
    println!(
        "\n--- DD value of the pass (on = pass), {} divergent ---",
        divergent.len()
    );
    report_brackets(args.count, &divergent, &tables, &contracts, vul);

    let mut by_tail: BTreeMap<&str, [i64; 3]> = BTreeMap::new();
    for ((h, table), [off, on]) in divergent_hits.iter().zip(&tables).zip(&contracts) {
        let e = by_tail.entry(&h.1).or_default();
        e[0] += 1;
        e[1] += pons::scoring::imps(
            pons::scoring::ns_score_contract(*on, table, vul)
                - pons::scoring::ns_score_contract(*off, table, vul),
        );
        e[2] += pons::scoring::imps(
            pons::scoring::ns_score_pd(*on, table, vul)
                - pons::scoring::ns_score_pd(*off, table, vul),
        );
    }
    println!("\n--- sample 5♥ - 5♠ - 6♠ boards: lane | asker | answerer | tricks ---");
    for (h, table) in divergent_hits
        .iter()
        .zip(&tables)
        .filter(|(h, _)| h.1 == "5♥ - 5♠ - 6♠")
        .take(12)
    {
        let (_, declarer) = h.5[1].expect("placed");
        let (asker, deal) = (h.6, deals[h.4]);
        let answerer = seat_to_act(asker, 2);
        println!(
            "{:28} | {} | {} | {}",
            h.0,
            deal[asker],
            deal[answerer],
            u8::from(table[Strain::Spades].get(declarer))
        );
    }
    println!("\n--- per tail: boards, plain IMPs, PD IMPs (on = pass) ---");
    for (tail, [n, plain, pd]) in by_tail {
        println!("{n:6} {plain:+7} {pd:+7}  {tail}");
    }

    let fixes: Vec<&Hit> = hits
        .iter()
        .filter(|h| h.7.is_some_and(|[a, b]| a != b))
        .collect();
    let (deals_fix, contracts_fix): (Vec<_>, Vec<_>) = fixes
        .iter()
        .map(|h| (deals[h.4], h.7.expect("filtered")))
        .unzip();
    let fired = hits.iter().filter(|h| h.7.is_some()).count();
    let tables = ddss::Solver::lock(None).solve_deals(&deals_fix, ddss::NonEmptyStrainFlags::ALL);
    println!(
        "\n--- DD value of the asker fix (on = six): {fired} fired, {} divergent ---",
        fixes.len()
    );
    report_brackets(
        args.count,
        &(0..fixes.len()).collect::<Vec<_>>(),
        &tables,
        &contracts_fix,
        vul,
    );
    for answer in ["5♦", "5♥"] {
        let idx: Vec<usize> = (0..fixes.len())
            .filter(|&k| fixes[k].1.starts_with(answer))
            .collect();
        let sub: Vec<_> = idx.iter().map(|&k| contracts_fix[k]).collect();
        let tab: Vec<_> = idx.iter().map(|&k| tables[k]).collect();
        println!("  over {answer}: {} divergent", idx.len());
        report_brackets(
            args.count,
            &(0..idx.len()).collect::<Vec<_>>(),
            &tab,
            &sub,
            vul,
        );
    }
}
