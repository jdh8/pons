//! Hidden-seat axis reach survey: how much of each axis's slice can a
//! projection actually *reach*?
//!
//! Phase 3/4 of the BEN-projection plan established the method on keycards:
//! an oracle proves the ceiling (partner's true keycards cut the slam-slice
//! evaluator MAE 2.66 → 1.41), but a projection recovers the truth only where
//! the auction has *shown* it, so realizable gain ≈ reach-fraction × ceiling —
//! and keycards died on reach (0.54% of slam cells ⟹ ≈ 0.007 tricks).
//!
//! This probe generalizes that reach measurement to the whole hidden-seat
//! axis survey. It walks the *same* auctions [`dump-evaluator`] does — full
//! self-play over pre-solved deals, a row at every decision — and measures,
//! per axis, on the slice where that axis's oracle ceiling is read:
//!
//! - **keycards** (legacy, partner-only, unchanged): *book* = the response
//!   resolved to an authored `keycards(...)` rule; *struct* = any `5♣/♦/♥/♠`
//!   after partner's `4NT`, the floor-inclusive superset. Slice: slam cells.
//! - **quality**: *book* = a call whose winning rule's prose constrains
//!   `suit_hcp` ("HCP in …") or `top_honors` ("of the top honors in …") —
//!   Ogust answers, quality-gated preempts, trap gates; *struct* = the Ogust
//!   answer position itself (own weak two, partner's 2NT relay). Slice: NT
//!   game-or-better cells on contested rows.
//! - **shortness**: *book* = a rule pinning some suit to `≤1`; *envelope* =
//!   the live [`Stance::infer`] envelope already caps some suit of that seat
//!   at ≤ 1 — splinters project, so this is the portion *already realized* by
//!   the range features. Slice: suit-strain game-or-better cells.
//! - **controls**: *book* = a rule whose prose mentions "control" (the
//!   American book authors none — expected ≈ 0); *struct* = a strong 2♣
//!   opening. Slice: slam cells.
//! - **stopper**: *book* = a rule gated on "stopper in …"; *struct* = a
//!   2NT/3NT call after an opponent showed a suit. Slice: NT game-or-better
//!   cells on contested rows.
//!
//! For the survey axes the oracle feeds truth for all **three** hidden seats,
//! so reach is reported as the disclosed-seat fraction: of `3 × cells`
//! seat-cells, how many belong to a hidden seat that has disclosed the axis.
//!
//! ```sh
//! cargo run --release --example probe-keycard-reach -- \
//!     --deals /nfs2/jdh8/22.pdd --count 200000 --seed 1
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Bid, FullDeal, Seat, Strain, Suit};
use ddss::TrickCountTable;
use pons::bidding::context::relative;
use pons::bidding::{Family, Phase, Stance, System};
use pons::{american, dutch, gib};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use rayon::prelude::*;
use std::sync::LazyLock;

/// A slam is 12+ of 13 tricks; compare on the `/13`-normalised label with a
/// mid-gap threshold so f32 rounding at the 11↔12 boundary can't flip a cell.
const SLAM_TRICKS: f32 = 11.5 / 13.0;
/// Game or better in a suit strain: 10+ tricks, mid-gap like [`SLAM_TRICKS`].
const SUIT_GAME_TRICKS: f32 = 9.5 / 13.0;
/// Game or better in notrump: 9+ tricks, mid-gap like [`SLAM_TRICKS`].
const NT_GAME_TRICKS: f32 = 8.5 / 13.0;

/// The four absolute vulnerabilities, sampled uniformly per board (matches
/// `dump-evaluator`'s stream shape, though not its exact per-deal assignment —
/// reach is an aggregate over random dealer/vul, so per-deal seeding is fine).
const VULS: [AbsoluteVulnerability; 4] = [
    AbsoluteVulnerability::NONE,
    AbsoluteVulnerability::NS,
    AbsoluteVulnerability::EW,
    AbsoluteVulnerability::ALL,
];

// Legacy keycard accumulator column indices. Rows/slam-cells, each split
// none/book/struct; "ours" restricts slam cells to our-side declarers.
const N: usize = 12;
const ROWS: usize = 0;
const ROWS_BOOK: usize = 1;
const ROWS_STRUCT: usize = 2;
const SLAM: usize = 3;
const SLAM_BOOK: usize = 4;
const SLAM_STRUCT: usize = 5;
const OURS: usize = 6;
const OURS_BOOK: usize = 7;
const OURS_STRUCT: usize = 8;
const AUC: usize = 9;
const AUC_BOOK: usize = 10;
const AUC_STRUCT: usize = 11;

// Survey axes beyond keycards, and their two latch kinds.
const AXES: usize = 4;
const QUALITY: usize = 0;
const SHORTNESS: usize = 1;
const CONTROLS: usize = 2;
const STOPPER: usize = 3;
/// Latch kind 0: an authored rule's prose disclosed the axis.
const BOOK: usize = 0;
/// Latch kind 1: the structural superset (quality/controls/stopper) or the
/// live inference envelope (shortness).
const STRUCT: usize = 1;
const KINDS: usize = 2;

/// Survey accumulator: slice denominators plus, per axis × latch kind, the
/// disclosed-seat counts. `rows_reached`/`slice_reached` count *seat-cells* —
/// each row or slice cell contributes 0..=3, one per hidden seat that has
/// disclosed — so the denominator is 3 × the matching plain count.
#[derive(Default, Clone, Copy)]
struct Survey {
    suit_game: u64,
    nt_contested: u64,
    rows_reached: [[u64; KINDS]; AXES],
    slice_reached: [[u64; KINDS]; AXES],
}

impl Survey {
    fn merge(mut self, other: Self) -> Self {
        self.suit_game += other.suit_game;
        self.nt_contested += other.nt_contested;
        for (a, b) in self
            .rows_reached
            .iter_mut()
            .flatten()
            .zip(other.rows_reached.iter().flatten())
        {
            *a += b;
        }
        for (a, b) in self
            .slice_reached
            .iter_mut()
            .flatten()
            .zip(other.slice_reached.iter().flatten())
        {
            *a += b;
        }
        self
    }
}

/// `describe_int_range`'s rendering of `len(suit, ..=1)`, per suit — the
/// prose fingerprint of an authored shortness pin (splinters and kin).
static SHORT_NEEDLES: LazyLock<[String; 4]> =
    LazyLock::new(|| Suit::ASC.map(|suit| format!("≤1 {suit}")));

/// The winning rule's prose constrains a suit's HCP or top honors.
fn quality_book(desc: &str) -> bool {
    desc.contains("HCP in ") || desc.contains("of the top honors in ")
}

/// The winning rule's prose pins some suit to at most one card.
fn shortness_book(desc: &str) -> bool {
    SHORT_NEEDLES.iter().any(|needle| desc.contains(needle))
}

/// The winning rule's prose mentions controls (currently authored nowhere —
/// the expected ≈ 0 is itself a survey datum).
fn controls_book(desc: &str) -> bool {
    desc.contains("control")
}

/// The winning rule's prose demands a stopper.
fn stopper_book(desc: &str) -> bool {
    desc.contains("stopper in ")
}

/// The Ogust answer position: partner relayed `2NT` over this seat's own
/// two-level suit opening two calls earlier. A structural superset of the
/// authored Ogust answers (any 2-level suit opening counts).
fn ogust_answer_position(auction: &[Call]) -> bool {
    let n = auction.len();
    n >= 4
        && auction[n - 2] == Call::Bid(Bid::new(2, Strain::Notrump))
        && matches!(auction[n - 4], Call::Bid(bid)
            if bid == Bid::new(2, bid.strain) && bid.strain != Strain::Notrump)
}

/// A strong `2♣` opening: the first non-pass call of the auction.
fn strong_two_clubs_opening(auction: &[Call], call: Call) -> bool {
    call == Call::Bid(Bid::new(2, Strain::Clubs)) && auction.iter().all(|&c| c == Call::Pass)
}

/// A `2NT`/`3NT` call after an opponent of `seat` showed a suit — the
/// structural "I have their suit stopped" superset.
fn nt_after_their_suit(auction: &[Call], call: Call, dealer: usize, seat: Seat) -> bool {
    [2, 3]
        .map(|level| Call::Bid(Bid::new(level, Strain::Notrump)))
        .contains(&call)
        && auction.iter().enumerate().any(|(i, c)| {
            // Seats alternate sides in `Seat::ALL`, so index parity is side.
            (dealer + i) % 2 != (seat as usize) % 2
                && matches!(c, Call::Bid(bid) if bid.strain != Strain::Notrump)
        })
}

/// The highest-logit finite (hence legal, after masking) call, defaulting to a
/// pass so the auction always terminates. Verbatim from `dump-evaluator`.
fn argmax_legal(logits: &pons::bidding::array::Logits) -> Call {
    logits
        .iter()
        .filter(|(_, l)| l.is_finite())
        .max_by(|a, b| a.1.partial_cmp(b.1).expect("logits are never NaN"))
        .map_or(Call::Pass, |(call, _)| call)
}

/// Bid one auction under `stance` and fold its rows into `acc` (the legacy
/// keycard counters) and `survey`. `rkcb` is prebuilt so the hot loop only
/// compares `Call`s: `[0..4]` are the four `5♣/♦/♥/♠` responses, `[4]` is the
/// `4NT` ask.
#[allow(clippy::too_many_arguments)]
fn walk(
    acc: &mut [u64; N],
    survey: &mut Survey,
    stance: &Stance,
    dealer: usize,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
    table: &TrickCountTable,
    rkcb: &[Call; 5],
) {
    let mut auction = Auction::new();
    // Per-seat latch: has this seat revealed its keycards to partner?
    let mut revealed_book = [false; 4];
    let mut revealed_struct = [false; 4];
    // Survey latches, `[axis][kind][seat]`. Shortness's envelope kind is read
    // live off the inference envelope instead (it *is* the projection).
    let mut latched = [[[false; 4]; KINDS]; AXES];

    while !auction.has_ended() {
        let seat = Seat::ALL[(dealer + auction.len()) % 4];
        let hand = deal[seat];
        let rel = relative(vul, seat);

        let Some(mut logits) = stance.classify(hand, rel, &auction) else {
            // Forced pass: dump-evaluator emits no row here, so neither do we.
            auction.push(Call::Pass);
            continue;
        };
        for (call, slot) in logits.iter_mut() {
            if auction.can_push(call).is_err() {
                *slot = f32::NEG_INFINITY;
            }
        }

        // Keycard reach: whether *partner* has revealed keycards by now.
        let partner = seat.partner() as usize;
        let (rb, rs) = (revealed_book[partner], revealed_struct[partner]);
        acc[ROWS] += 1;
        acc[ROWS_BOOK] += u64::from(rb);
        acc[ROWS_STRUCT] += u64::from(rs);

        // Survey reach: disclosed-seat counts over the three hidden seats.
        let hidden = [seat.lho(), seat.partner(), seat.rho()];
        let mut n = [[0u64; KINDS]; AXES];
        for (axis, kinds) in latched.iter().enumerate() {
            for (kind, seats) in kinds.iter().enumerate() {
                n[axis][kind] = hidden.iter().filter(|&&t| seats[t as usize]).count() as u64;
            }
        }
        let inferences = stance.infer(rel, &auction);
        n[SHORTNESS][STRUCT] = [inferences.lho(), inferences.partner(), inferences.rho()]
            .iter()
            .filter(|inf| inf.lengths.iter().any(|range| range.max <= 1))
            .count() as u64;

        let contested = Phase::of(&auction) != Phase::Constructive;
        let (mut slam_cells, mut suit_game_cells, mut nt_cells) = (0u64, 0u64, 0u64);
        for (idx, &value) in gib::relativized_tricks(table, seat).iter().enumerate() {
            if value >= SLAM_TRICKS {
                slam_cells += 1;
                acc[SLAM] += 1;
                acc[SLAM_BOOK] += u64::from(rb);
                acc[SLAM_STRUCT] += u64::from(rs);
                // Declarer is the target's low 2 bits: 0 = me, 2 = partner.
                if idx % 4 == 0 || idx % 4 == 2 {
                    acc[OURS] += 1;
                    acc[OURS_BOOK] += u64::from(rb);
                    acc[OURS_STRUCT] += u64::from(rs);
                }
            }
            // Labels are strain-major NT,S,H,D,C × 4 declarers: `idx < 4` is NT.
            if idx >= 4 && value >= SUIT_GAME_TRICKS {
                suit_game_cells += 1;
            }
            if idx < 4 && contested && value >= NT_GAME_TRICKS {
                nt_cells += 1;
            }
        }
        survey.suit_game += suit_game_cells;
        survey.nt_contested += nt_cells;
        for (axis, kinds) in n.iter().enumerate() {
            let slice_cells = match axis {
                SHORTNESS => suit_game_cells,
                CONTROLS => slam_cells,
                _ => nt_cells, // QUALITY and STOPPER read NT-contested
            };
            for (kind, &count) in kinds.iter().enumerate() {
                survey.rows_reached[axis][kind] += count;
                survey.slice_reached[axis][kind] += count * slice_cells;
            }
        }

        let call = argmax_legal(&logits);
        // One attribution per decision serves every book latch: the prose of
        // the rule that actually produced the call.
        let desc = stance
            .explain_call(hand, rel, &auction, call)
            .and_then(|(_, rule)| rule)
            .map_or(String::new(), |rule| rule.description);
        let s = seat as usize;
        // A structural RKCB response: `5♣/♦/♥/♠` when partner's previous call
        // (always 2 back — partners sit 2 seats apart) was `4NT`.
        if rkcb[..4].contains(&call) && auction.iter().rev().nth(1) == Some(&rkcb[4]) {
            revealed_struct[s] = true;
            // Book-confirmed iff an authored `keycards(...)` rule served it.
            if desc.contains("keycards") {
                revealed_book[s] = true;
            }
        }
        if quality_book(&desc) {
            latched[QUALITY][BOOK][s] = true;
        }
        if shortness_book(&desc) {
            latched[SHORTNESS][BOOK][s] = true;
        }
        if controls_book(&desc) {
            latched[CONTROLS][BOOK][s] = true;
        }
        if stopper_book(&desc) {
            latched[STOPPER][BOOK][s] = true;
        }
        if ogust_answer_position(&auction) {
            latched[QUALITY][STRUCT][s] = true;
        }
        if strong_two_clubs_opening(&auction, call) {
            latched[CONTROLS][STRUCT][s] = true;
        }
        if nt_after_their_suit(&auction, call, dealer, seat) {
            latched[STOPPER][STRUCT][s] = true;
        }
        auction.push(call);
    }
    acc[AUC] += 1;
    acc[AUC_BOOK] += u64::from(revealed_book.iter().any(|&b| b));
    acc[AUC_STRUCT] += u64::from(revealed_struct.iter().any(|&b| b));
}

#[derive(Parser)]
#[command(about = "Hidden-seat axis reach survey over the evaluator's self-play walk")]
struct Args {
    /// Pre-solved deal database: binary `.pdd`
    #[arg(long)]
    deals: String,
    /// Skip this many deals before reading
    #[arg(long, default_value_t = 0)]
    skip: u64,
    /// Number of deals to bid out
    #[arg(long, default_value_t = 200_000)]
    count: usize,
    /// Seed for the per-deal dealer/vulnerability stream
    #[arg(long, default_value_t = 0)]
    seed: u64,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let deals = pons::pdd::load_slice(&args.deals, args.skip, args.count)?;
    eprintln!("axis-reach: {} deals × 2 systems", deals.len());

    let systems = [
        american().against(Family::NATURAL),
        dutch().against(Family::NATURAL),
    ];
    let rkcb = [
        Call::Bid(Bid::new(5, Strain::Clubs)),
        Call::Bid(Bid::new(5, Strain::Diamonds)),
        Call::Bid(Bid::new(5, Strain::Hearts)),
        Call::Bid(Bid::new(5, Strain::Spades)),
        Call::Bid(Bid::new(4, Strain::Notrump)),
    ];

    let (totals, survey) = deals
        .par_iter()
        .enumerate()
        .map(|(index, (deal, table))| {
            // Per-deal seeding: order-independent, so rayon is free.
            let mut rng = StdRng::seed_from_u64(args.seed.wrapping_add(index as u64));
            let dealer = rng.random_range(0..4usize);
            let vul = VULS[rng.random_range(0..4usize)];
            let mut acc = [0u64; N];
            let mut survey = Survey::default();
            for stance in &systems {
                walk(
                    &mut acc,
                    &mut survey,
                    stance,
                    dealer,
                    vul,
                    deal,
                    table,
                    &rkcb,
                );
            }
            (acc, survey)
        })
        .reduce(
            || ([0u64; N], Survey::default()),
            |(mut a, sa), (b, sb)| {
                for (x, y) in a.iter_mut().zip(b) {
                    *x += y;
                }
                (a, sa.merge(sb))
            },
        );

    #[allow(clippy::cast_precision_loss)]
    let pct = |n: u64, d: u64| {
        if d == 0 {
            0.0
        } else {
            100.0 * n as f64 / d as f64
        }
    };
    let (rows, slam, ours, auc) = (totals[ROWS], totals[SLAM], totals[OURS], totals[AUC]);
    println!(
        "=== keycard reach: {} deals, seed {} ===",
        deals.len(),
        args.seed
    );
    println!("auctions {auc}   rows {rows}   slam-cells {slam}   our-side slam-cells {ours}\n");
    println!("                          book-RKCB      struct-RKCB (floor-incl)");
    println!(
        "auctions w/ reveal   {:9} {:6.3}%   {:9} {:6.3}%",
        totals[AUC_BOOK],
        pct(totals[AUC_BOOK], auc),
        totals[AUC_STRUCT],
        pct(totals[AUC_STRUCT], auc),
    );
    println!(
        "rows readable        {:9} {:6.3}%   {:9} {:6.3}%",
        totals[ROWS_BOOK],
        pct(totals[ROWS_BOOK], rows),
        totals[ROWS_STRUCT],
        pct(totals[ROWS_STRUCT], rows),
    );
    println!(
        "SLAM cells readable  {:9} {:6.3}%   {:9} {:6.3}%",
        totals[SLAM_BOOK],
        pct(totals[SLAM_BOOK], slam),
        totals[SLAM_STRUCT],
        pct(totals[SLAM_STRUCT], slam),
    );
    println!(
        "  our-side only      {:9} {:6.3}%   {:9} {:6.3}%",
        totals[OURS_BOOK],
        pct(totals[OURS_BOOK], ours),
        totals[OURS_STRUCT],
        pct(totals[OURS_STRUCT], ours),
    );
    println!(
        "\nrealizable slam-slice gain (book reach × 1.257 ceiling) ≈ {:.3} tricks",
        pct(totals[SLAM_BOOK], slam) / 100.0 * 1.257,
    );

    println!(
        "\n=== hidden-seat axis survey (reach = disclosed-seat fraction of 3 hidden seats) ==="
    );
    println!(
        "denominators: rows {rows}   slam-cells {slam}   suit-game-cells {}   \
         nt-contested-game-cells {}\n",
        survey.suit_game, survey.nt_contested,
    );
    println!("axis        slice          latch            rows-reach    slice-reach");
    let table = [
        (
            QUALITY,
            "quality",
            "nt-contested",
            survey.nt_contested,
            "book",
            "ogust-pos",
        ),
        (
            SHORTNESS,
            "shortness",
            "suit-game",
            survey.suit_game,
            "book(≤1)",
            "envelope",
        ),
        (CONTROLS, "controls", "slam", slam, "book", "2C-opening"),
        (
            STOPPER,
            "stopper",
            "nt-contested",
            survey.nt_contested,
            "book",
            "nt-vs-suit",
        ),
    ];
    for (axis, name, slice_name, slice_denom, book_label, struct_label) in table {
        for (kind, label) in [(BOOK, book_label), (STRUCT, struct_label)] {
            println!(
                "{name:<11} {slice_name:<14} {label:<14} {:9.3}%     {:9.3}%",
                pct(survey.rows_reached[axis][kind], 3 * rows),
                pct(survey.slice_reached[axis][kind], 3 * slice_denom),
            );
        }
    }
    Ok(())
}

/// The latch predicates are prose- and shape-matches against live books; each
/// one is pinned to a scripted auction so a reworded description or changed
/// structure fails here rather than silently reading zero reach forever.
#[cfg(test)]
mod tests {
    use super::*;
    use contract_bridge::Hand;

    fn calls(strs: &[&str]) -> Vec<Call> {
        strs.iter()
            .map(|c| c.parse().expect("valid test call"))
            .collect()
    }

    /// The winning rule's prose for `call` at this point of a North-dealt
    /// auction, `""` when no authored rule serves it.
    fn describe(auction: &[&str], hand: &str, call: &str) -> String {
        let stance = american().against(Family::NATURAL);
        let auction = calls(auction);
        let hand: Hand = hand.parse().expect("valid test hand");
        let seat = Seat::ALL[auction.len() % 4];
        let rel = relative(AbsoluteVulnerability::NONE, seat);
        stance
            .explain_call(hand, rel, &auction, call.parse().expect("valid test call"))
            .and_then(|(_, rule)| rule)
            .map_or(String::new(), |rule| rule.description)
    }

    /// Opener's Ogust answer is gated on `suit_hcp` — the quality prose latch.
    #[test]
    fn ogust_answer_carries_quality_prose() {
        let desc = describe(&["2S", "P", "2NT", "P"], "KQJ982.843.75.62", "3D");
        assert!(quality_book(&desc), "quality prose not found in {desc:?}");
    }

    /// Responder's splinter pins the short suit to ≤1 in prose, and the
    /// projected envelope caps it for partner's next look.
    #[test]
    fn splinter_shows_shortness() {
        let desc = describe(&["1S", "P"], "T984.AJ43.KQ42.7", "4C");
        assert!(
            shortness_book(&desc),
            "shortness prose not found in {desc:?}"
        );

        let stance = american().against(Family::NATURAL);
        let auction = calls(&["1S", "P", "4C", "P"]);
        let rel = relative(AbsoluteVulnerability::NONE, Seat::North);
        let inferences = stance.infer(rel, &auction);
        assert!(
            inferences
                .partner()
                .lengths
                .iter()
                .any(|range| range.max <= 1),
            "splinter did not cap any suit in partner's envelope"
        );
    }

    /// Responder's direct 3NT over their overcall of our 1NT (the lebensohl
    /// node) is gated on `stopper_in`.
    #[test]
    fn direct_3nt_over_overcall_carries_stopper_prose() {
        let desc = describe(&["1NT", "2S"], "A542.84.KQ54.K93", "3NT");
        assert!(stopper_book(&desc), "stopper prose not found in {desc:?}");
    }

    /// The pure auction-shape latches: Ogust position, strong 2♣ opening, and
    /// NT bid over an opponent's shown suit.
    #[test]
    fn structural_latches_fire_on_shape() {
        assert!(ogust_answer_position(&calls(&["2S", "P", "2NT", "P"])));
        assert!(!ogust_answer_position(&calls(&["1S", "P", "2NT", "P"])));

        let two_clubs: Call = "2C".parse().expect("valid test call");
        assert!(strong_two_clubs_opening(&calls(&["P"]), two_clubs));
        assert!(!strong_two_clubs_opening(&calls(&["1D", "P"]), two_clubs));

        let three_nt: Call = "3NT".parse().expect("valid test call");
        // North deals; South (same side as North) bids over East's 1♠.
        assert!(nt_after_their_suit(
            &calls(&["1H", "1S"]),
            three_nt,
            0,
            Seat::South
        ));
        assert!(!nt_after_their_suit(
            &calls(&["1H", "P"]),
            three_nt,
            0,
            Seat::South
        ));
    }
}
