use dds_bridge::solver::Vulnerability;
use dds_bridge::{Seat, Strain};
use pons::stats::{HistogramRow, HistogramTable, average_ns_par};
use proptest::prelude::*;

fn seat() -> impl Strategy<Value = Seat> {
    prop_oneof![
        Just(Seat::North),
        Just(Seat::East),
        Just(Seat::South),
        Just(Seat::West),
    ]
}

fn vulnerability() -> impl Strategy<Value = Vulnerability> {
    prop_oneof![
        Just(Vulnerability::NONE),
        Just(Vulnerability::NS),
        Just(Vulnerability::EW),
        Just(Vulnerability::ALL),
    ]
}

/// Build a valid [`HistogramTable`] respecting the invariant that every
/// `(seat, strain)` row has the same total count.  Pick a shared total `n`,
/// then for each of the 20 rows distribute `n` across the 14 trick buckets by
/// drawing 14 u32 weights and rounding; the final bucket absorbs leftovers.
fn histogram() -> impl Strategy<Value = HistogramTable> {
    (
        0usize..8,
        prop::collection::vec(prop::collection::vec(1u32..100, 14), 20),
    )
        .prop_map(|(n, weights)| {
            let mut table = HistogramTable::new();
            if n == 0 {
                return table;
            }
            for (i, seat) in [Seat::North, Seat::East, Seat::South, Seat::West]
                .into_iter()
                .enumerate()
            {
                for (j, strain) in Strain::ASC.into_iter().enumerate() {
                    let ws = &weights[i * 5 + j];
                    let total: u32 = ws.iter().sum();
                    let mut bucket = [0usize; 14];
                    let mut assigned = 0;
                    for k in 0..13 {
                        let share = (u64::from(ws[k]) * n as u64 / u64::from(total)) as usize;
                        bucket[k] = share;
                        assigned += share;
                    }
                    bucket[13] = n - assigned;
                    table[seat][strain] = bucket;
                }
            }
            table
        })
}

proptest! {
    #[test]
    fn average_ns_par_never_panics(
        hist in histogram(),
        vul in vulnerability(),
        dealer in seat(),
    ) {
        let _ = average_ns_par(hist, vul, dealer);
    }

    #[test]
    fn average_ns_par_some_iff_nonempty(
        hist in histogram(),
        vul in vulnerability(),
        dealer in seat(),
    ) {
        let empty = hist.count().is_none();
        let par = average_ns_par(hist, vul, dealer);
        prop_assert_eq!(par.is_none(), empty);
    }

    #[test]
    fn histogram_row_count_matches_table_count(hist in histogram()) {
        let table_count = hist.count();
        for seat in [Seat::North, Seat::East, Seat::South, Seat::West] {
            let row: HistogramRow = hist[seat];
            prop_assert_eq!(row.count(), table_count);
        }
    }
}
