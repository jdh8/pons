//! Interactive bidding-practice tool: bid one seat on random deals, get feedback.
//!
//! A human bids one seat; pons bots bid the other seats (or just partner in
//! `--bots 1` mode).  After the auction ends the tool reveals all four hands,
//! shows the auction grid, and gives a double-dummy verdict on the final
//! contract plus par information.
//!
//! ```text
//! cargo run --example practice-bidding -- --seat south --count 5
//! cargo run --example practice-bidding -- --bots 1 --min-hcp 12 --count 3
//! ```

#![allow(clippy::cast_precision_loss)]
#![allow(clippy::too_many_lines)]

use std::fmt::Write as _;
use std::io::{self, BufRead, Write as _};

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::{fill_deals, full_deal};
use contract_bridge::eval::{self, HandEvaluator as _, SimpleEvaluator};
use contract_bridge::{AbsoluteVulnerability, Builder, Contract, FullDeal, Seat, Strain, Suit};
use ddss::{
    NonEmptyStrainFlags, Solver, StrainFlags, TrickCountTable, Vulnerability, calculate_par,
};
use pons::bidding::{Pair, Table};
use pons::scoring::{final_contract, ns_score};
use pons::two_over_one;
#[cfg(feature = "neural-floor")]
use pons::two_over_one_neural_search;

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

fn parse_bots(s: &str) -> Result<u8, String> {
    match s {
        "1" => Ok(1),
        "3" => Ok(3),
        _ => Err(format!("bots must be 1 or 3, got {s}")),
    }
}

/// Interactive contract-bridge bidding practice
///
/// Deals random hands, bids three seats with the 2/1 system, and lets you
/// bid one seat.  After the auction you see all four hands, the auction grid,
/// and a double-dummy verdict.
#[derive(Parser)]
struct Args {
    /// Number of bots: 1 (partner only, opponents silent) or 3 (all three)
    #[arg(long, default_value = "3", value_parser = parse_bots)]
    bots: u8,

    /// Your seat: north, east, south, west
    #[arg(long, default_value = "south")]
    seat: Seat,

    /// Fixed dealer; if omitted, dealer rotates N→E→S→W per board
    #[arg(long)]
    dealer: Option<Seat>,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Number of boards to play; if omitted, play until quit or EOF
    #[arg(short, long)]
    count: Option<usize>,

    /// Minimum HCP for your hand
    #[arg(long)]
    min_hcp: Option<u8>,

    /// Maximum HCP for your hand
    #[arg(long)]
    max_hcp: Option<u8>,

    /// Minimum longest-suit length for your hand
    #[arg(long)]
    min_suit: Option<usize>,

    /// Rejection-sampling cap per board; falls back to the last deal on overflow
    #[arg(long, default_value = "10000")]
    max_attempts: usize,

    /// Reshuffled-opponent simulations for the verdict (0 disables simulation)
    #[arg(long, default_value = "1000")]
    simulations: usize,

    /// Bidding floor for the bots: neural-search (default, needs
    /// `--features neural-floor`) or instinct
    #[arg(long, default_value = DEFAULT_FLOOR)]
    floor: Floor,
}

/// Default floor: the learned net when it's compiled in, else the instinct ladder.
#[cfg(feature = "neural-floor")]
const DEFAULT_FLOOR: &str = "neural-search";
#[cfg(not(feature = "neural-floor"))]
const DEFAULT_FLOOR: &str = "instinct";

/// Which bidding floor the bots (and the "Bot's opinion" feedback) use
#[derive(Clone, Copy, clap::ValueEnum)]
enum Floor {
    /// Deterministic instinct ladder (baseline)
    Instinct,
    /// Distilled search-target neural floor (AI-bidder M3.2)
    #[cfg(feature = "neural-floor")]
    NeuralSearch,
}

/// Build a fresh 2/1 pair for the chosen floor
fn build_pair(floor: Floor) -> Pair {
    match floor {
        Floor::Instinct => two_over_one(),
        #[cfg(feature = "neural-floor")]
        Floor::NeuralSearch => two_over_one_neural_search(),
    }
}

// ---------------------------------------------------------------------------
// Helpers copied / adapted from instinct-floor
// ---------------------------------------------------------------------------

/// The seat acting after `len` calls from `dealer`
const fn seat_to_act(dealer: Seat, len: usize) -> Seat {
    Seat::ALL[(dealer as usize + len) % 4]
}

/// Signed human-side score: positive means good for the human's side
fn human_side_score(
    contract: Contract,
    declarer: Seat,
    table: &TrickCountTable,
    vul: AbsoluteVulnerability,
    human_seat: Seat,
) -> i64 {
    let ns = ns_score(Some((contract, declarer)), table, vul);
    match human_seat {
        Seat::North | Seat::South => ns,
        Seat::East | Seat::West => -ns,
    }
}

/// Convert a `Strain` to the matching single-strain `NonEmptyStrainFlags`
fn strain_flags(strain: Strain) -> NonEmptyStrainFlags {
    let flag = match strain {
        Strain::Clubs => StrainFlags::CLUBS,
        Strain::Diamonds => StrainFlags::DIAMONDS,
        Strain::Hearts => StrainFlags::HEARTS,
        Strain::Spades => StrainFlags::SPADES,
        Strain::Notrump => StrainFlags::NOTRUMP,
    };
    NonEmptyStrainFlags::new(flag).expect("single-strain flag is never empty")
}

/// Convert `AbsoluteVulnerability` to `ddss::Vulnerability` for `calculate_par`
fn to_ddss_vul(vul: AbsoluteVulnerability) -> Vulnerability {
    let mut v = Vulnerability::empty();
    if vul.contains(AbsoluteVulnerability::NS) {
        v |= Vulnerability::NS;
    }
    if vul.contains(AbsoluteVulnerability::EW) {
        v |= Vulnerability::EW;
    }
    v
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

fn print_board_all(deal: &FullDeal, dealer: Seat, vul: AbsoluteVulnerability, index: usize) {
    println!(
        "\nBoard {index}: dealer {}, vulnerability {vul}",
        dealer.letter(),
    );
    for seat in Seat::ALL {
        println!("  {}  {}", seat.letter(), deal[seat]);
    }
}

fn print_auction(auction: &[Call], dealer: Seat) {
    println!("  {:>6}{:>6}{:>6}{:>6}", "North", "East", "South", "West");

    // Leading blanks place the dealer's first call in the right column.
    let mut cells: Vec<String> = vec![String::new(); dealer as usize];
    cells.extend(auction.iter().map(|call| format!("{call}")));

    for row in cells.chunks(4) {
        let mut line = String::new();
        for cell in row {
            write!(line, "{cell:>6}").expect("writing to String never fails");
        }
        println!("  {line}");
    }
}

// ---------------------------------------------------------------------------
// Builder extension to set a seat by variable
// ---------------------------------------------------------------------------

/// Extension trait to set a [`Builder`] seat by a runtime [`Seat`] value
trait BuilderExt {
    fn set_seat(self, seat: Seat, hand: contract_bridge::Hand) -> Self;
}

impl BuilderExt for Builder {
    fn set_seat(self, seat: Seat, hand: contract_bridge::Hand) -> Self {
        match seat {
            Seat::North => self.north(hand),
            Seat::East => self.east(hand),
            Seat::South => self.south(hand),
            Seat::West => self.west(hand),
        }
    }
}

// ---------------------------------------------------------------------------
// Per-board double-dummy verdict
// ---------------------------------------------------------------------------

fn print_verdict(
    result: Option<(Contract, Seat)>,
    deal: &FullDeal,
    args: &Args,
    dealer: Seat,
    rng: &mut impl rand::Rng,
) {
    // Actual layout — always solve all strains (needed for par even on pass-out)
    let actual_tables = Solver::lock().solve_deals(&[*deal], NonEmptyStrainFlags::ALL);
    let actual_table = &actual_tables[0];

    if let Some((contract, declarer)) = result {
        let tricks = u8::from(actual_table[contract.bid.strain].get(declarer));
        let score = human_side_score(
            contract,
            declarer,
            actual_table,
            args.vulnerability,
            args.seat,
        );
        println!(
            "DD verdict: {tricks} tricks for {}, score {} from your side",
            declarer.letter(),
            score,
        );
    }

    // Par score (always print), signed from the human's side like the verdict
    let par = calculate_par(*actual_table, to_ddss_vul(args.vulnerability), dealer);
    let human_is_ns = matches!(args.seat, Seat::North | Seat::South);
    let par_score = i64::from(par.score) * if human_is_ns { 1 } else { -1 };
    if par.contracts.is_empty() {
        println!("Par from your side: 0 (passed out)");
    } else {
        let par_desc: Vec<String> = par
            .contracts
            .iter()
            .map(|pc| format!("{}{}", pc.contract, pc.declarer.letter()))
            .collect();
        println!("Par from your side: {par_score} ({})", par_desc.join(" / "));
    }

    // Reshuffled-opponent simulations (only when a contract was reached)
    if args.simulations > 0
        && let Some((contract, declarer)) = result
    {
        let human = args.seat;
        let partner = human.partner();
        // Fix the human's and partner's actual hands; reshuffle the other two
        let partial = Builder::new()
            .set_seat(human, deal[human])
            .set_seat(partner, deal[partner])
            .build_partial()
            .expect("two disjoint 13-card hands are a valid partial deal");

        let sim_deals: Vec<FullDeal> = fill_deals(rng, partial).take(args.simulations).collect();
        let flags = strain_flags(contract.bid.strain);
        let sim_tables = Solver::lock().solve_deals(&sim_deals, flags);

        let mut makes: usize = 0;
        let mut score_sum: i64 = 0;
        let mut tricks_min = u8::MAX;
        let mut tricks_sum: u64 = 0;
        let mut tricks_max: u8 = 0;

        for sim_table in &sim_tables {
            let t = u8::from(sim_table[contract.bid.strain].get(declarer));
            if t >= contract.bid.level.get() + 6 {
                makes += 1;
            }
            score_sum += human_side_score(contract, declarer, sim_table, args.vulnerability, human);
            tricks_sum += u64::from(t);
            tricks_min = tricks_min.min(t);
            tricks_max = tricks_max.max(t);
        }

        let n = sim_tables.len();
        if n > 0 {
            let make_pct = 100.0 * makes as f64 / n as f64;
            let mean_score = score_sum as f64 / n as f64;
            let mean_tricks = tricks_sum as f64 / n as f64;
            println!(
                "Simulation ({n} deals): makes {make_pct:.0}%, \
                 mean score {mean_score:+.0}, \
                 tricks {tricks_min}/{mean_tricks:.1}/{tricks_max}",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------

fn main() {
    let args = Args::parse();
    let mut rng = rand::rng();
    let hcp_eval = SimpleEvaluator(eval::hcp::<u8>);

    // Session statistics
    let mut boards_completed: usize = 0;
    let mut human_calls_total: usize = 0;
    let mut human_agree: usize = 0;

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    let board_limit = args.count.unwrap_or(usize::MAX);

    'board: for board_index in 1..=board_limit {
        // Dealer rotates per board unless fixed
        let dealer = args.dealer.unwrap_or(Seat::ALL[(board_index - 1) % 4]);

        // Rejection-sample until the human's hand satisfies constraints
        let deal: FullDeal = {
            let mut candidate = full_deal(&mut rng);
            let mut attempts = 1;
            loop {
                let hand = candidate[args.seat];
                let hcp = hcp_eval.eval(hand);
                let longest = Suit::ASC
                    .into_iter()
                    .map(|s| hand[s].len())
                    .max()
                    .unwrap_or(0);
                let ok = args.min_hcp.is_none_or(|m| hcp >= m)
                    && args.max_hcp.is_none_or(|m| hcp <= m)
                    && args.min_suit.is_none_or(|m| longest >= m);
                if ok {
                    break candidate;
                }
                if attempts >= args.max_attempts {
                    eprintln!(
                        "Warning: could not satisfy hand constraints after {attempts} attempts; \
                         using last deal."
                    );
                    break candidate;
                }
                candidate = full_deal(&mut rng);
                attempts += 1;
            }
        };

        // Print header: board info and human's hand only
        let human_hand = deal[args.seat];
        let hcp = hcp_eval.eval(human_hand);
        println!(
            "\nBoard {board_index}: dealer {}, vulnerability {}",
            dealer.letter(),
            args.vulnerability,
        );
        println!(
            "Your hand ({}): {}  [{hcp} HCP]",
            args.seat.letter(),
            human_hand,
        );

        // Build the table fresh per board so dealer/vul are correct
        let ns = build_pair(args.floor);
        let ew = build_pair(args.floor);
        let table = Table::of_pairs(&ns, &ew, dealer, args.vulnerability);

        let mut auction = Auction::new();
        let mut quit_session = false;

        while !auction.has_ended() {
            let seat = seat_to_act(dealer, auction.len());

            if seat == args.seat {
                // --- Human's turn ---
                // Snapshot the bot's opinion BEFORE pushing the human's call.
                // (classify reads auction.len() to determine which seat acts,
                //  so it must run before we extend the auction.)
                let bot_logits = table.classify(human_hand, &auction);

                // Build the bot's ranked top-3 legal calls (finite logit only)
                let top3: Vec<(Call, f32)> = if let Some(logits) = bot_logits.as_ref() {
                    let softmax = logits.softmax();
                    let mut scored: Vec<(Call, f32)> = logits
                        .iter()
                        .filter(|&(_, &logit)| logit.is_finite())
                        .filter(|(call, _)| auction.can_push(*call).is_ok())
                        .map(|(call, &logit)| (call, logit))
                        .collect();
                    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("logits never NaN"));
                    scored
                        .into_iter()
                        .take(3)
                        .map(|(call, _)| {
                            let prob = softmax.as_ref().map_or(0.0, |sm| *sm.get(call));
                            (call, prob)
                        })
                        .collect()
                } else {
                    vec![]
                };

                let bot_top1 = top3.first().map(|(c, _)| *c);

                // Prompt and read the human's call, looping on bad input
                let human_call = loop {
                    print!("Your call (e.g. 1H, P, X) [{seat}]: ");
                    let _ = io::stdout().flush();

                    let Some(Ok(line)) = lines.next() else {
                        // EOF → quit the session
                        quit_session = true;
                        break Call::Pass; // value unused; we break 'board below
                    };

                    let trimmed = line.trim();
                    if trimmed.eq_ignore_ascii_case("q") || trimmed.eq_ignore_ascii_case("quit") {
                        quit_session = true;
                        break Call::Pass; // value unused
                    }

                    match trimmed.parse::<Call>() {
                        Err(e) => println!("  Parse error: {e}. Try again."),
                        Ok(call) => {
                            if let Err(e) = auction.can_push(call) {
                                println!("  Illegal call: {e}. Try again.");
                            } else {
                                break call;
                            }
                        }
                    }
                };

                if quit_session {
                    break;
                }

                auction.push(human_call);
                human_calls_total += 1;

                // Print the bot's opinion
                if top3.is_empty() {
                    // Off-book position
                    println!("  [Book has no opinion; bot would pass]");
                    if human_call == Call::Pass {
                        human_agree += 1;
                    }
                } else {
                    let agreed = bot_top1 == Some(human_call);
                    if agreed {
                        human_agree += 1;
                    }
                    println!("  Bot's opinion:");
                    for (i, (call, prob)) in top3.iter().enumerate() {
                        let marker = if *call == human_call && i == 0 {
                            " ✓"
                        } else if *call == human_call {
                            " <"
                        } else {
                            ""
                        };
                        println!("    {}: {call} ({:.0}%){marker}", i + 1, 100.0 * prob);
                    }
                    if !agreed && let Some(top) = bot_top1 {
                        println!("  Bot's top pick was {top}; you chose {human_call}.");
                    }
                }
            } else {
                // --- Bot or silent seat ---
                let is_partner = seat == args.seat.partner();
                let call = if args.bots == 3 || is_partner {
                    // Active bot: use the system
                    table.next_call(deal[seat], &auction)
                } else {
                    // Silent opponent: always pass
                    Call::Pass
                };
                auction.push(call);
                println!("  {}: {call}", seat.letter());
            }
        }

        // If quit was requested during bidding, print session summary and stop
        if quit_session {
            break 'board;
        }

        boards_completed += 1;

        // -----------------------------------------------------------------------
        // Reveal
        // -----------------------------------------------------------------------
        print_board_all(&deal, dealer, args.vulnerability, board_index);
        println!("Auction:");
        let calls: Vec<Call> = auction.iter().copied().collect();
        print_auction(&calls, dealer);

        let result = final_contract(&auction, dealer);
        match result {
            Some((contract, declarer)) => {
                println!("Contract: {contract} by {}", declarer.letter());
            }
            None => println!("Contract: Passed out"),
        }

        print_verdict(result, &deal, &args, dealer, &mut rng);
        println!();
    }

    // -----------------------------------------------------------------------
    // Session summary
    // -----------------------------------------------------------------------
    let agree_pct = if human_calls_total > 0 {
        100.0 * human_agree as f64 / human_calls_total as f64
    } else {
        0.0
    };
    println!("=== Session summary ===");
    println!("Boards completed: {boards_completed}");
    println!("Your calls: {human_calls_total}");
    println!("Agreement with bot's top pick: {human_agree}/{human_calls_total} ({agree_pct:.0}%)");
}
