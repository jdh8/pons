//! Measure the doubled-1NT runout: an A/B duplicate match.
//!
//! When our 1NT is doubled, the [instinct floor][pons::bidding::instinct]
//! normally has nothing to say and responder passes — sitting for an
//! effectively-penalty double on a hand that may be broke.  The runout
//! ([`set_one_nt_runout`][pons::bidding::instinct::set_one_nt_runout]) lets a
//! weak responder escape to its longest five-plus-card suit instead.  Is that
//! worth points?
//!
//! Each board is bid twice, duplicate style: at table A the feature pair sits
//! North/South against a pair without it; at table B the teams swap seats.
//! Both pairs play the very same books — the per-call thread-local flip serves
//! both from one stance.  Boards whose two auctions reach different contracts
//! are scored double dummy ([`ns_score_contract`], the actual penalty as bid),
//! and the swing is credited to the feature team.
//!
//! `--compare` selects the feature under test: `runout` (the whole runout vs the
//! passing floor — the default), `escape-stack` / `escape-values` (the penalty
//! double of the opponents' escape), or `minors5` / `direct` (the 2NT shape
//! variants).  Every axis but `runout` holds the base runout on for both sides
//! and flips only its sub-feature, isolating the marginal value.
//!
//! The doubled-1NT direct-game axes (gambling 3NT on a long minor, preemptive 4M
//! on a long major) test the user's claims: `gambling-len` / `preempt4m` vs the
//! suppress baseline (claim 3, bid the long-suit game), `gambling-semisolid` vs
//! `gambling-len` (does suit quality help?), `gambling-ace` vs `gambling-semisolid`
//! (claim 4, the ace), and `gambling` for the whole package.  Score with both
//! `--score plain` and `--score pd` — a plain win that the perfect-defense scorer
//! reverses is a doubling artifact (the obstruction value is DD-invisible).  Claim 1
//! (XX catches all strong balanced) is a `--coverage` tally, not a swing.
//!
//! ```text
//! cargo run --release --example ab-one-nt-runout -- --compare gambling-ace --score pd --filter-1nt --count 3000000
//! cargo run --release --example ab-one-nt-runout -- --compare gambling --filter-1nt --count 100000 --show 8
//! cargo run --release --example ab-one-nt-runout -- --coverage --filter-1nt --count 3000000
//! ```

use clap::{Parser, ValueEnum};
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Bid, FullDeal, Hand, Seat, Strain, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::instinct::{
    Unusual2nt, set_gambling_3nt_over_double, set_gambling_3nt_require_ace,
    set_gambling_3nt_top_honors, set_one_nt_runout, set_one_nt_runout_universal,
    set_penalize_escape_stack, set_penalize_escape_values, set_preempt_4m_over_double,
    set_preempt_4m_require_ace, set_preempt_4m_top_honors, set_runout_xx_min, set_unusual_2nt,
};
use pons::bidding::{Family, Stance};
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Board, hand_hcp, next_call, seat_to_act};

/// Which runout feature the two tables differ on
#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
enum Compare {
    /// The whole runout vs the natural floor (passing) — the original measure
    Runout,
    /// Penalty double of their escape on a trump stack (runout on both sides)
    EscapeStack,
    /// Penalty double of their escape on values after a business XX (likewise)
    EscapeValues,
    /// Responder's 2NT extended to five-five minors (`Unusual2nt::FiveFiveAdd`)
    Minors5,
    /// No 2NT relay: a four-four bust runs direct (`Unusual2nt::Direct`)
    Direct,
    /// Gambling 3NT on a 6+ minor, length only (no quality/ace gate) vs suppress.
    /// Claim 3 for the minors: is bidding the long-suit game worth more than
    /// sitting for XX / escaping?
    GamblingLen,
    /// Gambling 3NT semi-solid (top-honors 2) vs length only — does suit quality
    /// help?  Both sides gamble; the feature side adds the quality gate.
    GamblingSemisolid,
    /// Gambling 3NT with an outside ace vs semi-solid without — claim 4, the
    /// single-gate flip that isolates the ace requirement.
    GamblingAce,
    /// Length-only 4M on any 6+ major vs suppress — the major mirror of
    /// `gambling-len` (no quality/ace gate).  Expected DD-negative.
    Preempt4mLen,
    /// Quality 4M (semi-solid, trump ace) vs suppress — does the same quality gate
    /// that rescues 3NT also rescue the long-major game?
    Preempt4mQuality,
    /// Semi-solid + suit-ace 3NT *alone* (no 4M) vs suppress — the isolated
    /// "is the long-minor gamble, done right, worth more than XX/escape?" ship test.
    Gambling3nt,
    /// The whole package (semi-solid + suit-ace 3NT, quality 4M) vs suppress — the
    /// net ship candidate.
    Gambling,
}

/// Which double-dummy scorer prices the divergent boards
#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
enum Score {
    /// Plain DD: the reached contract at its actual penalty (the duplicate result)
    Plain,
    /// Perfect-defense: a contract that fails DD is doubled, carrying any real
    /// X/XX already on the table — the right scorer once a side may defend by passing
    Pd,
}

/// Measure the doubled-1NT runout: an A/B duplicate match
#[derive(Parser)]
struct Args {
    /// Which feature to A/B between the two tables
    #[arg(long, value_enum, default_value_t = Compare::Runout)]
    compare: Compare,

    /// Which double-dummy scorer prices the swing (plain DD or perfect-defense)
    #[arg(long, value_enum, default_value_t = Score::Plain)]
    score: Score,

    /// Only keep deals with a balanced 15-17 hand somewhere (a 1NT-opener
    /// candidate) — raises the doubled-1NT fire density ~6-10x.  `--count` then
    /// means *kept* boards.
    #[arg(long, default_value_t = false)]
    filter_1nt: bool,

    /// Coverage mode (claim 1): bid each deal once with the full gambling package
    /// on and tally responder's call over a double of our 1NT, by shape and HCP —
    /// no A/B swing.  Confirms every strong *balanced* hand lands on XX.
    #[arg(long, default_value_t = false)]
    coverage: bool,

    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "20000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Deal seed (reproducible boards)
    #[arg(long, default_value = "0")]
    seed: u64,

    /// HCP floor for responder's XX = to-play (raise to disable XX entirely)
    #[arg(long, default_value = "7")]
    xx_min: u8,

    /// Restrict the runout to responder's direct seat (no opener escape / SOS)
    #[arg(long)]
    no_universal: bool,

    /// Print this many divergent boards (auction + contracts) for inspection
    #[arg(long, default_value = "0")]
    show: usize,
}

/// Bid out one deal, flipping the measured feature per acting side
///
/// The thread-locals are set just before each classification, so this is safe
/// under rayon: the worker sets and reads them on its own thread.  For the
/// `Runout` axis the base runout itself toggles per side (the original measure);
/// for every other axis the base runout is on for both sides and only the named
/// sub-feature flips, isolating its marginal value.
fn bid_out(
    stance: &Stance,
    args: &Args,
    feature_is_ns: bool,
    dealer: Seat,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let on = seat_is_ns == feature_is_ns;

        set_runout_xx_min(args.xx_min);
        set_one_nt_runout_universal(!args.no_universal);
        // Baseline: runout on (except the Runout axis flips it), sub-features off.
        set_one_nt_runout(args.compare != Compare::Runout || on);
        set_unusual_2nt(Unusual2nt::FourFour);
        set_penalize_escape_stack(false);
        set_penalize_escape_values(false);
        // Flip only the measured sub-feature on the feature side.
        match args.compare {
            Compare::EscapeStack => set_penalize_escape_stack(on),
            Compare::EscapeValues => set_penalize_escape_values(on),
            Compare::Minors5 if on => set_unusual_2nt(Unusual2nt::FiveFiveAdd),
            Compare::Direct if on => set_unusual_2nt(Unusual2nt::Direct),
            _ => {}
        }

        // Gambling 3NT / preemptive 4M over a double of our 1NT: configure the
        // feature side (`on`) and its control as `(3NT armed, top-honor floor,
        // outside ace, 4M armed)`.  Every non-gambling axis leaves the package off
        // — the shipped suppress baseline.
        let (g3_on, g3_honors, g3_ace, p4_on, p4_honors, p4_ace) = match (args.compare, on) {
            // 3NT long-minor gamble arms (preempt 4M off both sides)
            (Compare::GamblingLen, true) => (true, 0, false, false, 0, false),
            (Compare::GamblingSemisolid, true) => (true, 2, false, false, 0, false),
            (Compare::GamblingSemisolid, false) => (true, 0, false, false, 0, false),
            (Compare::GamblingAce, true) => (true, 2, true, false, 0, false),
            (Compare::GamblingAce, false) => (true, 2, false, false, 0, false),
            (Compare::Gambling3nt, true) => (true, 2, true, false, 0, false),
            // 4M long-major arms (3NT off both sides)
            (Compare::Preempt4mLen, true) => (false, 2, true, true, 0, false),
            (Compare::Preempt4mQuality, true) => (false, 2, true, true, 2, true),
            // full package
            (Compare::Gambling, true) => (true, 2, true, true, 2, true),
            // every baseline / non-gambling axis: suppress (both games off)
            _ => (false, 2, true, false, 2, true),
        };
        set_gambling_3nt_over_double(g3_on);
        set_gambling_3nt_top_honors(g3_honors);
        set_gambling_3nt_require_ace(g3_ace);
        set_preempt_4m_over_double(p4_on);
        set_preempt_4m_top_honors(p4_honors);
        set_preempt_4m_require_ace(p4_ace);

        auction.push(next_call(
            stance,
            deal[seat],
            dealer,
            args.vulnerability,
            &auction,
        ));
    }
    auction
}

/// Balanced shape: no void or singleton, at most one doubleton (4333/4432/5332)
fn is_balanced(hand: Hand) -> bool {
    let len = Suit::ASC.map(|s| hand[s].len());
    len.iter().all(|&l| l >= 2) && len.iter().filter(|&&l| l == 2).count() <= 1
}

/// A 1NT-opener candidate: balanced 15-17 (the `--filter-1nt` gate)
fn is_1nt_opener(hand: Hand) -> bool {
    is_balanced(hand) && (15..=17).contains(&hand_hcp(hand))
}

/// If our side opened 1NT (all prior calls passes) and the next hand doubled it,
/// responder's call (the `[1NT, (X), ?]` action) and seat.
fn responder_over_double(auction: &Auction, dealer: Seat) -> Option<(Call, Seat)> {
    let one_nt = Call::Bid(Bid::new(1, Strain::Notrump));
    let calls: Vec<Call> = auction.iter().copied().collect();
    let nt = calls.iter().position(|&call| call == one_nt)?;
    if calls[..nt].iter().any(|&call| call != Call::Pass)
        || calls.get(nt + 1) != Some(&Call::Double)
    {
        return None;
    }
    let responder_call = *calls.get(nt + 2)?;
    Some((responder_call, seat_to_act(dealer, nt + 2)))
}

/// Bid one deal with the full gambling package on for every seat (coverage mode)
fn bid_coverage(stance: &Stance, args: &Args, dealer: Seat, deal: &FullDeal) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        set_runout_xx_min(args.xx_min);
        set_one_nt_runout(true);
        set_one_nt_runout_universal(!args.no_universal);
        set_unusual_2nt(Unusual2nt::FourFour);
        set_penalize_escape_stack(false);
        set_penalize_escape_values(false);
        set_gambling_3nt_over_double(true);
        set_gambling_3nt_top_honors(2);
        set_gambling_3nt_require_ace(true);
        set_preempt_4m_over_double(true);
        auction.push(next_call(
            stance,
            deal[seat],
            dealer,
            args.vulnerability,
            &auction,
        ));
    }
    auction
}

/// Coverage (claim 1): tally responder's action over a double of our 1NT, by shape
/// and HCP, with the full gambling package armed.  Every strong *balanced* hand
/// should land on the business redouble — none should leak to the gamble.
#[allow(clippy::cast_precision_loss)]
fn run_coverage(stance: &Stance, args: &Args, deals: &[(Seat, FullDeal)]) {
    let rows: Vec<(bool, u8, Call)> = deals
        .par_iter()
        .filter_map(|&(dealer, deal)| {
            let auction = bid_coverage(stance, args, dealer, &deal);
            responder_over_double(&auction, dealer)
                .map(|(call, seat)| (is_balanced(deal[seat]), hand_hcp(deal[seat]), call))
        })
        .collect();

    let gamble = Call::Bid(Bid::new(3, Strain::Notrump));
    let xx = Call::Redouble;
    println!(
        "=== Coverage: responder over a double of our 1NT — {} fired of {} deals ===",
        rows.len(),
        args.count,
    );
    for (lo, hi, label) in [
        (0u8, 6u8, "0-6"),
        (7, 9, "7-9"),
        (10, 12, "10-12"),
        (13, 40, "13+"),
    ] {
        let bucket: Vec<Call> = rows
            .iter()
            .filter(|&&(bal, hcp, _)| bal && (lo..=hi).contains(&hcp))
            .map(|&(_, _, call)| call)
            .collect();
        if bucket.is_empty() {
            continue;
        }
        let xx_n = bucket.iter().filter(|&&call| call == xx).count();
        let g_n = bucket.iter().filter(|&&call| call == gamble).count();
        println!(
            "balanced {label:>5} HCP: {:6} hands, {xx_n:6} XX ({:5.1}%), {g_n} gambling-3NT",
            bucket.len(),
            100.0 * xx_n as f64 / bucket.len() as f64,
        );
    }
    let leaks = rows
        .iter()
        .filter(|&&(bal, hcp, call)| bal && hcp >= 7 && call == gamble)
        .count();
    println!("Claim 1 — strong balanced hands that gambled instead of XX: {leaks}");
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let stance = american().against(Family::NATURAL);

    // Deal sequentially (seeded, reproducible); bid both tables in parallel.  With
    // --filter-1nt keep only deals holding a 1NT-opener candidate, to raise the
    // doubled-1NT fire density (--count then means *kept* boards).
    let mut rng = StdRng::seed_from_u64(args.seed);
    let mut deals: Vec<(Seat, FullDeal)> = Vec::with_capacity(args.count);
    while deals.len() < args.count {
        let deal = full_deal(&mut rng);
        if !args.filter_1nt || Seat::ALL.iter().any(|&seat| is_1nt_opener(deal[seat])) {
            deals.push((Seat::ALL[deals.len() % 4], deal));
        }
    }

    if args.coverage {
        run_coverage(&stance, &args, &deals);
        return;
    }

    let boards: Vec<Board> = deals
        .par_iter()
        .map(|&(dealer, deal)| Board {
            deal,
            dealer,
            table_a: bid_out(&stance, &args, true, dealer, &deal),
            table_b: bid_out(&stance, &args, false, dealer, &deal),
        })
        .collect();

    // Only boards whose tables reach different contracts can swing; solve those
    // double dummy (on the main thread) and credit the swing to the runout team
    // (NS at table A, EW at table B).
    let contracts: Vec<_> = boards
        .iter()
        .map(|board| {
            (
                final_contract(&board.table_a, board.dealer),
                final_contract(&board.table_b, board.dealer),
            )
        })
        .collect();
    let divergent: Vec<usize> = (0..boards.len())
        .filter(|&index| contracts[index].0 != contracts[index].1)
        .collect();
    let deals: Vec<FullDeal> = divergent.iter().map(|&index| boards[index].deal).collect();
    let tables = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);

    let scorer = match args.score {
        Score::Plain => ns_score_contract,
        Score::Pd => ns_score_pd,
    };
    let mut total_points = 0i64;
    let mut total_imps = 0i64;
    let mut shown = 0;
    for (&index, table) in divergent.iter().zip(tables.iter()) {
        let (contract_a, contract_b) = contracts[index];
        let swing = scorer(contract_a, table, args.vulnerability)
            - scorer(contract_b, table, args.vulnerability);
        total_points += swing;
        total_imps += imps(swing);

        if shown < args.show {
            shown += 1;
            let board = &boards[index];
            let calls: Vec<Call> = board.table_a.iter().copied().collect();
            // The gambler's hand (responder over the double) — the "find some hands"
            // payload: the actual holdings that bid 1NT-(X)-3NT/4M.
            let responder = responder_over_double(&board.table_a, board.dealer)
                .map(|(_, seat)| {
                    format!(
                        "  resp {seat:?} {} ({} HCP)",
                        board.deal[seat],
                        hand_hcp(board.deal[seat])
                    )
                })
                .unwrap_or_default();
            println!(
                "[{shown}] dealer {:?}  A {calls:?} -> {contract_a:?}  vs  B -> {contract_b:?}  (swing {swing:+}){responder}",
                board.dealer,
            );
        }
    }

    println!(
        "=== Doubled-1NT runout A/B: compare {:?}, score {:?}, {} boards, vulnerability {}, xx-min {}, universal {} ===",
        args.compare, args.score, args.count, args.vulnerability, args.xx_min, !args.no_universal,
    );
    println!(
        "Divergent boards: {} of {} ({:.2}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );
    println!(
        "Runout team: {total_points:+} points, {total_imps:+} IMPs ({:+.3} IMPs/board, {:+.3} IMPs/divergent)",
        total_imps as f64 / args.count.max(1) as f64,
        total_imps as f64 / divergent.len().max(1) as f64,
    );
}
