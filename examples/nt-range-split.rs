//! Does double-dummy simulation see 1NT-response games the HCP book misses?
//!
//! A rule that *drives simulation*: split the 1NT opener's shown range into a
//! lower and an upper half, deal layouts from each, and ask double-dummy whether
//! some game is good opposite a maximum but not a minimum — the very meaning of
//! an invitation (AI-bidder; see `docs/ai-bidder/`).
//!
//! The opener is North (`1NT`–`Pass`–?, South to respond, uncontested).  For each
//! sampled responder hand:
//!
//! - **Oracle** — over openers sampled from each half ([`sample_layouts`] with the
//!   opener's points [`narrowed`][pons::bidding::Inferences::narrowed_points] to that
//!   half), score the best NS *game* against the best NS *partscore* double-dummy.
//!   Game good opposite both halves → **FG**, opposite the upper half only → **INV**,
//!   neither → **PASS**.  This is steps 1–3 of the proposal, raw DD on a fixed
//!   contract (perfect-defense doubling, as the EV evaluator uses).
//! - **Book** — bid the full auction out under [`american`] from `1NT`–`Pass` over
//!   the *same* sampled openers; the fraction that lands in game gives the book the
//!   same FG/INV/PASS verdict.  Bidding out (rather than reading responder's single
//!   call) routes a 4-card-major hand through `2♣` Stayman and a 5-carder through a
//!   transfer automatically — the point of the comparison is whether that routing
//!   reaches the game the oracle sees.
//!
//! The output is a confusion matrix (oracle × book) per vulnerability, a
//! disagreement breakdown by responder HCP band, and the list of *under-reaches* —
//! hands the oracle rates above what the book reaches.  Two kinds surface: the book
//! cannot accept a 1NT–2NT invitation (so every invitational hand plays partscore —
//! a book-completeness gap that empties the whole INV column), and a smaller tail of
//! shapely hands whose distributional game the HCP book passes.  If the book — or the
//! search bidder — already reaches these, a bespoke simulation-driven rule is redundant.
//!
//! ```text
//! cargo run --release --example nt-range-split -- 500 128
//! ```
//! Args (all optional, positional): `hands` (default 200), `layouts` per half
//! (default 80), `seed` (default 0).  Heavy at large `hands` — run via
//! `scripts/idle-run.sh`.

use contract_bridge::auction::{Auction, Call, RelativeVulnerability};
use contract_bridge::eval::{self, HandEvaluator, SimpleEvaluator};
use contract_bridge::{
    AbsoluteVulnerability, Bid, Contract, FullDeal, Hand, Penalty, Seat, Strain, Suit,
};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::bidding::{Context, Inferences, Range, Relative, american, sample_layouts};
use pons::scoring::{final_contract, ns_score_doubling_failures};
use pons::{Pair, Table};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// NS game contracts realistic from a 1NT auction.
// ponytail: minor-suit games (5♣/5♦) omitted — rare from 1NT; widen GAME if a
// long-minor responder ever matters.
const GAME: &[(u8, Strain)] = &[
    (3, Strain::Notrump),
    (4, Strain::Hearts),
    (4, Strain::Spades),
];

/// NS partscore alternatives (the places to stop below game).
const PART: &[(u8, Strain)] = &[
    (1, Strain::Notrump),
    (2, Strain::Notrump),
    (2, Strain::Hearts),
    (3, Strain::Hearts),
    (2, Strain::Spades),
    (3, Strain::Spades),
];

/// Responder's raw high-card points — the scale the user's bands (`≤7`, `8–9`,
/// `10+`) are stated on, so the report groups by it rather than by the upgraded
/// `point_count` the sampler matches openers against.
fn raw_hcp(hand: Hand) -> u8 {
    SimpleEvaluator(eval::hcp::<u8>).eval(hand)
}

/// Length of the responder's longest suit (5+ ⇒ "shapely").
fn longest(hand: Hand) -> usize {
    Suit::ASC
        .into_iter()
        .map(|suit| hand[suit].len())
        .max()
        .unwrap_or(0)
}

/// Whether a bid is game-level or higher (scores a game bonus).
fn is_game_or_better(bid: Bid) -> bool {
    let level = bid.level.get();
    match bid.strain {
        Strain::Notrump => level >= 3,
        Strain::Spades | Strain::Hearts => level >= 4,
        Strain::Diamonds | Strain::Clubs => level >= 5,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Pass,
    Inv,
    Fg,
}

impl Verdict {
    const ALL: [Verdict; 3] = [Verdict::Pass, Verdict::Inv, Verdict::Fg];
    fn label(self) -> &'static str {
        match self {
            Verdict::Pass => "PASS",
            Verdict::Inv => "INV ",
            Verdict::Fg => "FG  ",
        }
    }
}

/// FG iff game good opposite both halves, INV iff opposite the upper only, else
/// PASS.  `good_lower && !good_upper` (good vs a minimum but not a maximum) is an
/// anomaly; fold it into FG (game is good somewhere in range).
fn verdict(good_lower: bool, good_upper: bool) -> Verdict {
    match (good_lower, good_upper) {
        (true, true) => Verdict::Fg,
        (false, true) => Verdict::Inv,
        (true, false) => Verdict::Fg, // anomaly: good vs a minimum but not a maximum
        (false, false) => Verdict::Pass,
    }
}

/// Total DD score (summed over the half's deals) of one contract by one declarer,
/// perfect-defense doubled — the EV evaluator's per-deal scorer.
fn total_score(
    level: u8,
    strain: Strain,
    declarer: Seat,
    tables: &[ddss::TrickCountTable],
    vul: AbsoluteVulnerability,
) -> i64 {
    let contract = Contract {
        bid: Bid::new(level, strain),
        penalty: Penalty::Undoubled,
    };
    tables
        .iter()
        .map(|table| ns_score_doubling_failures(Some((contract, declarer)), table, vul))
        .sum()
}

/// Whether the best NS game out-scores the best NS partscore over a half's deals
/// (max over the two NS declarers — an optimistic double-dummy bound).
fn game_is_good(tables: &[ddss::TrickCountTable], vul: AbsoluteVulnerability) -> bool {
    let best = |set: &[(u8, Strain)]| {
        set.iter()
            .flat_map(|&(level, strain)| {
                [Seat::North, Seat::South]
                    .map(|declarer| total_score(level, strain, declarer, tables, vul))
            })
            .max()
            .unwrap_or(i64::MIN)
    };
    best(GAME) > best(PART)
}

/// Fraction of the half's deals where the book lands in a game contract.
fn reaches_game_frac(
    table: &Table<impl pons::System, impl pons::System>,
    deals: &[FullDeal],
    seed: &Auction,
    dealer: Seat,
) -> f64 {
    let reached = deals
        .iter()
        .filter(|deal| {
            let auction = table.bid_out_from(deal, seed.clone());
            matches!(final_contract(&auction, dealer), Some((c, _)) if is_game_or_better(c.bid))
        })
        .count();
    reached as f64 / deals.len() as f64
}

/// HCP band a responder hand falls in, for the by-band breakdown.
fn band(hcp: u8) -> &'static str {
    match hcp {
        0..=5 => "0-5",
        6..=7 => "6-7",
        8..=9 => "8-9",
        10..=11 => "10-11",
        _ => "12+",
    }
}

fn main() {
    // Load-bearing branch self-check (deterministic).
    assert!(is_game_or_better(Bid::new(3, Strain::Notrump)));
    assert!(!is_game_or_better(Bid::new(2, Strain::Notrump)));
    assert!(is_game_or_better(Bid::new(4, Strain::Hearts)));
    assert!(!is_game_or_better(Bid::new(3, Strain::Spades)));

    let mut argv = std::env::args().skip(1);
    let hands: usize = argv.next().and_then(|s| s.parse().ok()).unwrap_or(200);
    let layouts: usize = argv.next().and_then(|s| s.parse().ok()).unwrap_or(80);
    let seed: u64 = argv.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let min_layouts = layouts / 2;

    // 1NT (North) – Pass (East) – South to respond, uncontested.
    let prior = [Call::Bid(Bid::new(1, Strain::Notrump)), Call::Pass];
    let context = Context::new(RelativeVulnerability::NONE, &prior);
    let inf = Inferences::read(&context);
    let full = inf.partner().points; // the opener (Relative::Partner)
    assert!(full.max > full.min, "1NT shows a splittable point range");
    let mid = (full.min + full.max) / 2;
    let lower = inf.narrowed_points(Relative::Partner, Range::new(full.min, mid));
    let upper = inf.narrowed_points(Relative::Partner, Range::new(mid + 1, full.max));

    let mut seed_auction = Auction::new();
    seed_auction
        .try_extend(prior.iter().copied())
        .expect("1NT–Pass is a legal prior auction");
    let dealer = Seat::North;
    let book: Pair = american();

    let vuls = [
        ("none", AbsoluteVulnerability::NONE),
        ("both", AbsoluteVulnerability::ALL),
    ];
    // confusion[vul][oracle][book] and per-band [vul][band] = (total, disagree)
    let mut confusion = [[[0u32; 3]; 3]; 2];
    let mut by_band: std::collections::BTreeMap<(usize, &'static str), (u32, u32)> =
        Default::default();
    // Under-reaches: the oracle rates the hand higher than the book reaches.
    let mut under_reach: Vec<(usize, Hand, u8, bool, Verdict, Verdict)> = Vec::new();
    let mut rng = StdRng::seed_from_u64(seed);
    let mut skipped = 0u32;

    for _ in 0..hands {
        let responder = contract_bridge::deck::full_deal(&mut rng)[Seat::South];

        let deals_lo = sample_layouts(responder, Seat::South, &lower, &mut rng, layouts);
        let deals_hi = sample_layouts(responder, Seat::South, &upper, &mut rng, layouts);
        if deals_lo.len() < min_layouts || deals_hi.len() < min_layouts {
            skipped += 1;
            continue;
        }
        let tables_lo = Solver::lock().solve_deals(&deals_lo, NonEmptyStrainFlags::ALL);
        let tables_hi = Solver::lock().solve_deals(&deals_hi, NonEmptyStrainFlags::ALL);

        let hcp = raw_hcp(responder);
        let shapely = longest(responder) >= 5;

        for (vi, &(_, vul)) in vuls.iter().enumerate() {
            let oracle = verdict(game_is_good(&tables_lo, vul), game_is_good(&tables_hi, vul));

            let table = Table::of_pairs(&book, &book, dealer, vul);
            let frac_lo = reaches_game_frac(&table, &deals_lo, &seed_auction, dealer);
            let frac_hi = reaches_game_frac(&table, &deals_hi, &seed_auction, dealer);
            let booked = verdict(frac_lo >= 0.5, frac_hi >= 0.5);

            confusion[vi][oracle as usize][booked as usize] += 1;
            let entry = by_band.entry((vi, band(hcp))).or_default();
            entry.0 += 1;
            if oracle != booked {
                entry.1 += 1;
            }
            if (oracle as usize) > (booked as usize) {
                under_reach.push((vi, responder, hcp, shapely, oracle, booked));
            }
        }
    }

    let scored = hands as u32 - skipped;
    println!(
        "=== 1NT range-split: {scored} responder hands ({skipped} skipped), \
         {layouts} layouts/half, opener {}-{} split at {mid} (lower {}-{}, upper {}-{}) ===",
        full.min,
        full.max,
        full.min,
        mid,
        mid + 1,
        full.max
    );

    for (vi, (name, _)) in vuls.iter().enumerate() {
        println!("\n-- vulnerability {name}: oracle (rows) × book (cols) --");
        println!("       {:>5} {:>5} {:>5}", "PASS", "INV", "FG");
        let mut disagree = 0u32;
        let mut total = 0u32;
        for o in Verdict::ALL {
            let c = &confusion[vi][o as usize];
            println!("  {} {:>5} {:>5} {:>5}", o.label(), c[0], c[1], c[2]);
            for b in Verdict::ALL {
                let n = confusion[vi][o as usize][b as usize];
                total += n;
                if o != b {
                    disagree += n;
                }
            }
        }
        let rate = 100.0 * f64::from(disagree) / f64::from(total.max(1));
        println!("  disagreement: {disagree}/{total} ({rate:.1}%)");
    }

    println!("\n-- disagreement by responder HCP band --");
    for ((vi, b), (tot, dis)) in &by_band {
        let rate = 100.0 * f64::from(*dis) / f64::from((*tot).max(1));
        println!(
            "  {:<4} {:<6} {dis:4}/{tot:<4} ({rate:.1}%)",
            vuls[*vi].0, b
        );
    }

    println!(
        "\n-- under-reach: oracle rates the hand above what the book reaches ({} cases) --",
        under_reach.len()
    );
    for (vi, hand, hcp, shapely, oracle, booked) in under_reach.iter().take(40) {
        println!(
            "  {:<4} {hcp:2} HCP {}  oracle={} book={}  {hand}",
            vuls[*vi].0,
            if *shapely { "shapely" } else { "flat   " },
            oracle.label().trim(),
            booked.label().trim(),
        );
    }
    if under_reach.len() > 40 {
        println!("  … and {} more", under_reach.len() - 40);
    }
}
