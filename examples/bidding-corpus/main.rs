//! Deterministically regenerate the frozen bidding-performance corpus.
//!
//! This is data generation, not an A/B. It prints TSV to stdout; review and
//! replace `benches/fixtures/bidding-performance.tsv` explicitly.

use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat};
use pons::bidding::book::Phase;
use pons::bidding::context::relative;
use pons::bidding::{Stance, System};
use pons::{american, american_instinct};
use rand::SeedableRng;
use rand::rngs::StdRng;

#[path = "../../benches/support/mod.rs"]
mod support;
use support::{Category, DepthBin, Origin, format_auction, format_vulnerability};

#[path = "../common/oracle/mod.rs"]
#[allow(dead_code)]
mod oracle;
use oracle::{BbaOracle, DEFAULT_LIB, SYSTEM_2_OVER_1};

const SEED: u64 = 1;
const TARGETS: [(DepthBin, usize); 4] = [
    (DepthBin::Two, 2),
    (DepthBin::Four, 4),
    (DepthBin::Eight, 8),
    (DepthBin::Twelve, 12),
];

#[derive(Clone)]
struct Row {
    origin: Origin,
    bin: DepthBin,
    category: Category,
    vul: contract_bridge::auction::RelativeVulnerability,
    hand: Hand,
    auction: Vec<Call>,
}

fn rkcb_historical_decode_row() -> Row {
    Row {
        origin: Origin::Pons,
        bin: DepthBin::Twelve,
        category: Category::RkcbSlamTail,
        vul: contract_bridge::auction::RelativeVulnerability::NONE,
        hand: "KQ.9.AQ9875.T764".parse().expect("valid frozen RKCB hand"),
        auction: support::parse_auction("1D P 1H 1S P 3S 4D P 4N P 5H X")
            .expect("valid frozen RKCB auction"),
    }
}

fn seat_to_act(dealer: Seat, len: usize) -> Seat {
    Seat::ALL[(dealer as usize + len) % 4]
}

fn next_call(
    system: &dyn System,
    hand: Hand,
    vul: contract_bridge::auction::RelativeVulnerability,
    auction: &Auction,
) -> Call {
    let logits = system
        .classify(hand, vul, auction)
        .expect("a corpus engine must classify every harvested prefix");
    let mut scored: Vec<_> = logits
        .iter()
        .filter(|(_, score)| score.is_finite())
        .map(|(call, score)| (call, *score))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("finite logits are never NaN"));
    scored
        .into_iter()
        .map(|(call, _)| call)
        .find(|&call| auction.can_push(call).is_ok())
        .unwrap_or(Call::Pass)
}

fn harvest(origin: Origin, system: &dyn System, rows: &mut Vec<Row>) {
    let mut counts = [0_usize; 4];
    for row in rows.iter().filter(|row| row.origin == origin) {
        let index = TARGETS.iter().position(|&(bin, _)| bin == row.bin).unwrap();
        counts[index] += 1;
    }
    for board in 0_u64..200_000 {
        if counts.iter().all(|&count| count == support::PER_ORIGIN_BIN) {
            return;
        }
        let deal = full_deal(&mut StdRng::seed_from_u64(SEED.wrapping_add(board)));
        let dealer = Seat::ALL[board as usize % 4];
        let vul = [
            AbsoluteVulnerability::NONE,
            AbsoluteVulnerability::NS,
            AbsoluteVulnerability::EW,
            AbsoluteVulnerability::ALL,
        ][board as usize / 4 % 4];
        harvest_deal(origin, system, &deal, dealer, vul, rows, &mut counts);
    }
    panic!("failed to fill {origin:?} cells: {counts:?}");
}

fn same_logits(one: &pons::bidding::array::Logits, two: &pons::bidding::array::Logits) -> bool {
    one.iter()
        .zip(two.iter())
        .all(|((one_call, one), (two_call, two))| {
            one_call == two_call && one.to_bits() == two.to_bits()
        })
}

fn annotate_categories(stance: &Stance, deterministic: &Stance, rows: &mut [Row]) {
    fn mark(rows: &mut [Row], category: Category, predicate: impl Fn(&Row) -> bool) {
        let row = rows
            .iter_mut()
            .find(|row| row.category == Category::Representative && predicate(row))
            .unwrap_or_else(|| panic!("no harvested position covers {}", category.as_str()));
        row.category = category;
    }
    let is_pons = |row: &Row| row.origin == Origin::Pons;
    let provenance = |row: &Row| {
        stance
            .classify_with_provenance(row.hand, row.vul, &row.auction)
            .expect("the shipped floor is total")
            .1
    };
    let is_floor = |row: &Row| {
        let provenance = provenance(row);
        provenance.depth == 0 && provenance.fallback.is_some()
    };
    let matches_instinct = |row: &Row| {
        let pons = stance
            .classify(row.hand, row.vul, &row.auction)
            .expect("Pons logits");
        let instinct = deterministic
            .classify(row.hand, row.vul, &row.auction)
            .expect("instinct logits");
        same_logits(&pons, &instinct)
    };
    let has_four_nt = |row: &Row| {
        row.auction.iter().any(|call| {
            matches!(call, Call::Bid(bid) if bid.level.get() == 4 && bid.strain == contract_bridge::Strain::Notrump)
        })
    };

    mark(rows, Category::AuthoredShallow, |row| {
        is_pons(row)
            && matches!(row.bin, DepthBin::Two | DepthBin::Four)
            && provenance(row).depth > 0
            && provenance(row).fallback.is_none()
    });
    mark(rows, Category::AuthoredDeep, |row| {
        is_pons(row)
            && matches!(row.bin, DepthBin::Eight | DepthBin::Twelve)
            && provenance(row).depth > 0
            && provenance(row).fallback.is_none()
    });
    mark(rows, Category::ConstructiveFloor, |row| {
        is_pons(row) && Phase::of(&row.auction) == Phase::Constructive && is_floor(row)
    });
    // The old label selected any depth-12 row containing 4NT; its frozen row
    // had RHO make that call and exercised no keycard machinery.  Keep the
    // same cell and row count, but replace that reserved slot with the audited
    // off-book position where we asked, partner answered 1430, and the floor
    // must decode the answer using its pre-answer historical reading.
    let rkcb = rows
        .iter_mut()
        .find(|row| {
            row.category == Category::Representative
                && is_pons(row)
                && row.bin == DepthBin::Twelve
                && has_four_nt(row)
        })
        .expect("a depth-12 Pons slot is available for the targeted RKCB case");
    *rkcb = rkcb_historical_decode_row();
    mark(rows, Category::ForcedInstinctFloor, |row| {
        is_pons(row) && is_floor(row) && !has_four_nt(row) && matches_instinct(row)
    });
    mark(rows, Category::NeuralFloor, |row| {
        is_pons(row)
            && is_floor(row)
            && Phase::of(&row.auction) != Phase::Constructive
            && !matches_instinct(row)
    });
    mark(rows, Category::SystemsOn, |row| {
        is_pons(row) && provenance(row).rebases > 0
    });
    mark(rows, Category::Fallback, |row| {
        is_pons(row)
            && provenance(row).depth > 0
            && provenance(row).fallback.is_some()
            && provenance(row).rebases == 0
    });
}

fn harvest_deal(
    origin: Origin,
    system: &dyn System,
    deal: &FullDeal,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    rows: &mut Vec<Row>,
    counts: &mut [usize; 4],
) {
    let mut auction = Auction::new();
    while !auction.has_ended() && auction.len() <= 12 {
        for (index, &(bin, depth)) in TARGETS.iter().enumerate() {
            if auction.len() == depth && counts[index] < support::PER_ORIGIN_BIN {
                let seat = seat_to_act(dealer, auction.len());
                let candidate = Row {
                    origin,
                    bin,
                    category: Category::Representative,
                    vul: relative(vul, seat),
                    hand: deal[seat],
                    auction: auction.iter().copied().collect(),
                };
                let duplicate = rows.iter().any(|row| {
                    row.origin == candidate.origin
                        && row.bin == candidate.bin
                        && row.vul == candidate.vul
                        && row.hand == candidate.hand
                        && row.auction == candidate.auction
                });
                if !duplicate {
                    rows.push(candidate);
                    counts[index] += 1;
                }
            }
        }
        if auction.len() == 12 {
            break;
        }
        let seat = seat_to_act(dealer, auction.len());
        auction.push(next_call(system, deal[seat], relative(vul, seat), &auction));
    }
}

fn main() -> anyhow::Result<()> {
    // NativeAOT EPBot stays on this main thread throughout.
    let stance: Stance = american().against();
    let deterministic = american_instinct().against();
    let bba = BbaOracle::load(DEFAULT_LIB, SYSTEM_2_OVER_1, Vec::new())?;
    let mut rows = Vec::with_capacity(support::POSITION_COUNT);
    harvest(Origin::Pons, &stance, &mut rows);
    harvest(Origin::Bba, &bba, &mut rows);
    annotate_categories(&stance, &deterministic, &mut rows);
    rows.sort_by_key(|row| (row.origin, row.bin, row.category));

    println!("# pons bidding performance corpus v1");
    println!("# seed={SEED}; exact cells: 64 Pons + 64 BBA per depth 2/4/8/12");
    println!("# id\torigin\tdepth\tcategory\trelative-vulnerability\thand\tauction");
    for (id, row) in rows.iter().enumerate() {
        println!(
            "{id}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.origin.as_str(),
            row.bin.as_str(),
            row.category.as_str(),
            format_vulnerability(row.vul),
            row.hand,
            format_auction(&row.auction),
        );
    }
    Ok(())
}
