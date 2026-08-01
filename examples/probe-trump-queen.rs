//! How often does a small slam make without the trump queen, by fit length?
//!
//! The queen relay's whole cost is the hands it *stops*: four keycards, the
//! queen denied, so the asker settles for five.  Whether that is right depends
//! on a number nobody in this repo had measured — **how often twelve tricks are
//! there anyway** — and that number is a function of the trump fit, because a
//! longer fit finds the queen more often (drop, finesse, or the suit never
//! having to behave at all).
//!
//! This probe deals hands, ignores the bidder entirely, and asks the solver
//! directly.  For every deal, every trump suit and both partnerships:
//!
//! - the side's **combined trump length** (8, 9, 10, 11+),
//! - whether the side holds the **trump queen**,
//! - the side's **keycard count** (four aces plus the trump king),
//!
//! and then, for the sides that have a fit and the keycards to be asking at all,
//! whether **six of that trump makes double-dummy** by the better declarer.
//!
//! The cell that decides the doctrine is *queen missing, four keycards*: if six
//! makes there far more often than it fails, "queen denied → stop at five" is
//! costing tricks and the relay should only require the queen on the shorter
//! fits.
//!
//! **Double-dummy overstates every row**, and by a known mechanism: the solver
//! never misguesses a two-way queen, so it converts a real 50% guess into a
//! certainty whenever either line works.  Read the DD make rate as an **upper
//! bound** on what a table would score, tightest on the long fits (where the
//! queen drops and no guess is involved) and loosest at eight cards.
//!
//! ```text
//! cargo run --release --example probe-trump-queen -- --count 200000
//! ```

use clap::Parser;
use contract_bridge::{Builder, Rank, Seat, Strain, Suit};
use ddss::{NonEmptyStrainFlags, Solver};

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::seeded_deals;

/// Measure how often six makes without the trump queen, by combined fit length
#[derive(Parser)]
struct Args {
    /// Number of deals to sample
    #[arg(short, long, default_value = "100000")]
    count: usize,

    /// Deal seed base
    #[arg(long, default_value = "0")]
    seed: u64,

    /// Solver batch size (deals per `solve_deals` call)
    #[arg(long, default_value = "2048")]
    batch: usize,

    /// Only count sides holding at least this many combined HCP — the band
    /// where a keycard ask actually happens.  Random deals answer a different
    /// question (most 4-keycard sides have no business near a slam).
    #[arg(long, default_value = "30")]
    min_hcp: u8,

    /// Combined-HCP floor for the *grand* table.  Seven is a different band
    /// from six, and counting grands over the small-slam population answers a
    /// question nobody asks at the table.
    #[arg(long, default_value = "33")]
    grand_hcp: u8,
}

/// High-card points of a hand, the standard 4/3/2/1
fn hcp(hand: contract_bridge::Hand) -> u8 {
    Suit::ASC
        .into_iter()
        .map(|suit| {
            let holding = hand[suit];
            u8::from(holding.contains(Rank::A)) * 4
                + u8::from(holding.contains(Rank::K)) * 3
                + u8::from(holding.contains(Rank::Q)) * 2
                + u8::from(holding.contains(Rank::J))
        })
        .sum()
}

/// One (fit length, has-queen, keycards) cell
///
/// `made` is the double-dummy make rate.  For the queen-missing cells the two
/// swap counters split those makes by whether the *location* of the queen
/// decided the hand: `robust` makes with the queen in either defender's hand,
/// `guessed` makes with it in one and fails with it in the other.  A real
/// declarer wins the second class about half the time, so
/// `robust + guessed / 2` is the de-biased estimate and `made` the DD upper
/// bound.
#[derive(Clone, Copy, Default)]
struct Cell {
    seen: u64,
    made: u64,
    robust: u64,
    guessed: u64,
    swapped: u64,
    /// Makes in the *original* world, restricted to the swap-eligible subset —
    /// the only honest comparison base for [`Cell::honest`]
    eligible_made: u64,
}

impl Cell {
    fn rate(self) -> f64 {
        if self.seen == 0 {
            0.0
        } else {
            self.made as f64 / self.seen as f64
        }
    }

    /// Double-dummy make rate over the swap-eligible subset — the base
    /// [`Cell::honest`] must be read against.  Not the same as [`Cell::rate`]:
    /// eligibility needs the other defender to hold a trump to trade, which
    /// quietly drops the 4-0 splits where six usually fails.
    fn eligible_rate(self) -> f64 {
        if self.swapped == 0 {
            0.0
        } else {
            self.eligible_made as f64 / self.swapped as f64
        }
    }

    /// The de-biased make rate over the same subset: a queen whose *location*
    /// decides the contract is a real 50/50 guess at the table, not the
    /// certainty DD scores it as.  The gap to [`Cell::eligible_rate`] is the
    /// share of DD's makes that were really coin flips.
    fn honest(self) -> f64 {
        if self.swapped == 0 {
            return 0.0;
        }
        (self.robust as f64 + self.guessed as f64 / 2.0) / self.swapped as f64
    }

    /// Half-width of the 95% interval on the make rate
    fn ci(self) -> f64 {
        if self.seen == 0 {
            return 0.0;
        }
        let p = self.rate();
        1.96 * (p * (1.0 - p) / self.seen as f64).sqrt()
    }
}

/// Fit-length buckets: 8, 9, 10, 11-or-more
const LENGTHS: [&str; 4] = ["8", "9", "10", "11+"];

/// Keycard buckets we report: exactly four, and all five
const KEYCARDS: [usize; 2] = [4, 5];

/// The two opponents of the partnership containing `seat`
const fn defenders(seat: Seat) -> (Seat, Seat) {
    match seat {
        Seat::North | Seat::South => (Seat::East, Seat::West),
        Seat::East | Seat::West => (Seat::North, Seat::South),
    }
}

fn bucket(len: usize) -> Option<usize> {
    match len {
        8 => Some(0),
        9 => Some(1),
        10 => Some(2),
        11..=13 => Some(3),
        _ => None,
    }
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let deals = seeded_deals(args.seed, args.count);

    // [length bucket][has queen][keycard bucket]
    let mut cells = [[[Cell::default(); 2]; 2]; 4];

    // The grand table: five keycards only, split by whether the trump suit is
    // *settled* (queen held, or a proven 10-card fit) and by how many side
    // kings the partnership holds.  This is the one number the relay's grand
    // gate rides on — `kings_outside(trump, N..)` — and N is what it answers.
    // Only `seen` and `made` are used; the swap columns price a trump-queen
    // guess, which a settled suit no longer has.
    let mut grands = [[Cell::default(); 4]; 2];

    // Queen-missing sides that qualify get a second, queen-swapped solve; the
    // work list is collected on the first pass and solved in one batch.
    struct Swap {
        len: usize,
        kc: usize,
        deal: contract_bridge::FullDeal,
        strain: Strain,
        one: Seat,
        two: Seat,
        made_original: bool,
    }
    let mut swaps: Vec<Swap> = Vec::new();

    for chunk in deals.chunks(args.batch) {
        let tables = Solver::lock(None).solve_deals(chunk, NonEmptyStrainFlags::ALL);
        for (deal, table) in chunk.iter().zip(tables.iter()) {
            for trump in Suit::ASC {
                let strain = Strain::from(trump);
                for &(one, two) in &[(Seat::North, Seat::South), (Seat::East, Seat::West)] {
                    let (a, b) = (deal[one], deal[two]);
                    let Some(len) = bucket(a[trump].len() + b[trump].len()) else {
                        continue;
                    };
                    if hcp(a) + hcp(b) < args.min_hcp {
                        continue;
                    }
                    let queen =
                        usize::from(a[trump].contains(Rank::Q) || b[trump].contains(Rank::Q));
                    // Keycards are a property of the *side*: four aces plus the
                    // trump king, counted across both hands exactly as the 1430
                    // ladder's arithmetic resolves them.
                    let aces = Suit::ASC
                        .into_iter()
                        .filter(|&s| a[s].contains(Rank::A) || b[s].contains(Rank::A))
                        .count();
                    let king =
                        usize::from(a[trump].contains(Rank::K) || b[trump].contains(Rank::K));
                    let keycards = aces + king;
                    let Some(kc) = KEYCARDS.iter().position(|&k| k == keycards) else {
                        continue;
                    };
                    // The better of the two declarers — the asker places the
                    // contract knowing which hand should play it.
                    let tricks =
                        u8::from(table[strain].get(one)).max(u8::from(table[strain].get(two)));
                    let cell = &mut cells[len][queen][kc];
                    cell.seen += 1;
                    cell.made += u64::from(tricks >= 12);

                    if keycards == 5 && hcp(a) + hcp(b) >= args.grand_hcp {
                        let kings = Suit::ASC
                            .into_iter()
                            .filter(|&s| {
                                s != trump && (a[s].contains(Rank::K) || b[s].contains(Rank::K))
                            })
                            .count();
                        // "Settled" is the relay's own test: a real queen, or
                        // ten trumps that make one unnecessary.
                        let grand = &mut grands[usize::from(queen == 1 || len >= 2)][kings];
                        grand.seen += 1;
                        grand.made += u64::from(tricks >= 13);
                    }

                    // Queen-missing: rebuild the deal with the trump queen in
                    // the *other* defender's hand, traded for their lowest
                    // trump, and queue it.  If that defender is void in trumps
                    // there is nothing to trade and no guess to price.
                    if queen == 0 {
                        let (x, y) = defenders(one);
                        let holder = if deal[x][trump].contains(Rank::Q) {
                            x
                        } else {
                            y
                        };
                        let other = if holder == x { y } else { x };
                        if let Some(low) = deal[other][trump].iter().next_back() {
                            let mut builder = Builder::from(*deal);
                            builder[holder][trump].remove(Rank::Q);
                            builder[holder][trump].insert(low);
                            builder[other][trump].remove(low);
                            builder[other][trump].insert(Rank::Q);
                            if let Ok(deal) = builder.build_full() {
                                swaps.push(Swap {
                                    len,
                                    kc,
                                    deal,
                                    strain,
                                    one,
                                    two,
                                    made_original: tricks >= 12,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Second pass: the queen-swapped worlds.  A side whose six makes with the
    // queen in *either* defender's hand never had a guess to lose; one that
    // makes in only one world did, and wins it about half the time at a table.
    for chunk in swaps.chunks(args.batch) {
        let batch: Vec<contract_bridge::FullDeal> = chunk.iter().map(|s| s.deal).collect();
        let tables = Solver::lock(None).solve_deals(&batch, NonEmptyStrainFlags::ALL);
        for (swap, table) in chunk.iter().zip(tables.iter()) {
            let tricks = u8::from(table[swap.strain].get(swap.one))
                .max(u8::from(table[swap.strain].get(swap.two)));
            let cell = &mut cells[swap.len][0][swap.kc];
            cell.swapped += 1;
            cell.eligible_made += u64::from(swap.made_original);
            match (swap.made_original, tricks >= 12) {
                (true, true) => cell.robust += 1,
                (true, false) | (false, true) => cell.guessed += 1,
                (false, false) => {}
            }
        }
    }
    println!(
        "\n=== Six of trumps, double-dummy make rate by fit length ({} deals, seed {}) ===",
        args.count, args.seed
    );
    println!("(DD never misguesses the queen, so every rate is an upper bound.)\n");
    for (&kc, kci) in KEYCARDS.iter().zip(0..) {
        println!("-- {kc} keycards --");
        println!(
            "{:>5}  {:>21}  {:>19}  {:>21}  {:>8}",
            "fit", "queen missing (DD)", "swap-eligible DD→SD", "queen held (DD)", "gap"
        );
        for (li, name) in LENGTHS.iter().enumerate() {
            let without = cells[li][0][kci];
            let with = cells[li][1][kci];
            println!(
                "{name:>5}  {:>6.1}% ± {:>4.1} ({:>6})  {:>7.1}% → {:>6.1}%  {:>6.1}% ± {:>4.1} ({:>6})  {:>+7.1}pp",
                100.0 * without.rate(),
                100.0 * without.ci(),
                without.seen,
                100.0 * without.eligible_rate(),
                100.0 * without.honest(),
                100.0 * with.rate(),
                100.0 * with.ci(),
                with.seen,
                100.0 * (with.rate() - without.rate()),
            );
        }
        println!();
    }

    println!(
        "=== Seven of trumps, five keycards, {}+ combined HCP ===",
        args.grand_hcp
    );
    println!("(Grand breaks even near 56-58% at IMPs; DD is an upper bound here too.)\n");
    println!(
        "{:>10}  {:>26}  {:>26}",
        "side kings", "trump settled (Q or 10)", "trump unsettled"
    );
    for (kings, (&settled, &unsettled)) in grands[1].iter().zip(&grands[0]).enumerate() {
        println!(
            "{kings:>10}  {:>7.1}% ± {:>4.1} ({:>6})  {:>7.1}% ± {:>4.1} ({:>6})",
            100.0 * settled.rate(),
            100.0 * settled.ci(),
            settled.seen,
            100.0 * unsettled.rate(),
            100.0 * unsettled.ci(),
            unsettled.seen,
        );
    }
}
