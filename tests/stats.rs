use dds_bridge::solver::Vulnerability;
use dds_bridge::{Bid, Contract, Level, Penalty, Seat, Strain};
use pons::stats::{Accumulator, HistogramRow, HistogramTable, Statistics, average_ns_par};

#[test]
fn test_statistics_new() {
    let s = Statistics::new(3.0, 1.5);
    assert_eq!(s.mean(), 3.0);
    assert_eq!(s.sd(), 1.5);
}

#[test]
fn test_statistics_default() {
    let s = Statistics::default();
    assert_eq!(s.mean(), 0.0);
    assert_eq!(s.sd(), 0.0);
}

#[test]
fn test_statistics_display() {
    let s = Statistics::new(3.0, 1.5);
    let display = format!("{s}");
    assert!(display.contains("3"));
    assert!(display.contains("±"));
    assert!(display.contains("1.5"));
}

#[test]
fn test_accumulator_new() {
    let acc = Accumulator::new();
    assert_eq!(acc.count(), 0);
    assert_eq!(acc.mean(), 0.0);
    assert_eq!(acc.sdm(), 0.0);
}

#[test]
fn test_accumulator_default() {
    assert_eq!(Accumulator::default(), Accumulator::new());
}

#[test]
fn test_accumulator_single_value() {
    let mut acc = Accumulator::new();
    acc.push(5.0);
    assert_eq!(acc.count(), 1);
    assert_eq!(acc.mean(), 5.0);
    assert_eq!(acc.sdm(), 0.0);
}

#[test]
fn test_accumulator_two_values() {
    let mut acc = Accumulator::new();
    acc.push(2.0);
    acc.push(4.0);
    assert_eq!(acc.count(), 2);
    assert_eq!(acc.mean(), 3.0);
    // sdm = (2-3)^2 + (4-3)^2 = 2
    assert!((acc.sdm() - 2.0).abs() < 1e-10);
}

#[test]
fn test_accumulator_population() {
    let mut acc = Accumulator::new();
    acc.push(2.0);
    acc.push(4.0);
    let pop = acc.population();
    assert_eq!(pop.mean(), 3.0);
    // population sd = sqrt(2/2) = 1
    assert!((pop.sd() - 1.0).abs() < 1e-10);
}

#[test]
fn test_accumulator_sample() {
    let mut acc = Accumulator::new();
    acc.push(2.0);
    acc.push(4.0);
    let samp = acc.sample();
    assert_eq!(samp.mean(), 3.0);
    // sample sd = sqrt(2/1) = sqrt(2)
    assert!((samp.sd() - std::f64::consts::SQRT_2).abs() < 1e-10);
}

#[test]
fn test_accumulator_empty_population() {
    let acc = Accumulator::new();
    let pop = acc.population();
    assert!(pop.mean().is_nan());
    assert!(pop.sd().is_nan());
}

#[test]
fn test_accumulator_empty_sample() {
    let acc = Accumulator::new();
    let samp = acc.sample();
    assert!(samp.mean().is_nan());
    assert!(samp.sd().is_nan());
}

#[test]
fn test_accumulator_single_sample_sd_nan() {
    let mut acc = Accumulator::new();
    acc.push(5.0);
    let samp = acc.sample();
    assert_eq!(samp.mean(), 5.0);
    // sd undefined for n=1 sample (0/0 = NaN)
    assert!(samp.sd().is_nan());
}

#[test]
fn test_accumulator_many_values() {
    let mut acc = Accumulator::new();
    for i in 1..=100 {
        acc.push(i as f64);
    }
    assert_eq!(acc.count(), 100);
    assert!((acc.mean() - 50.5).abs() < 1e-10);
}

#[test]
fn test_histogram_row_new() {
    let row = HistogramRow::new();
    assert!(row.count().is_none());
}

#[test]
fn test_histogram_row_index() {
    let mut row = HistogramRow::new();
    row[Strain::Notrump][7] = 5;
    assert_eq!(row[Strain::Notrump][7], 5);
    assert_eq!(row[Strain::Clubs][7], 0);
}

#[test]
fn test_histogram_row_count() {
    let mut row = HistogramRow::new();
    row[Strain::Clubs][6] = 3;
    assert_eq!(row.count().map(|n| n.get()), Some(3));
}

#[test]
fn test_histogram_table_new() {
    let table = HistogramTable::new();
    assert!(table.count().is_none());
}

#[test]
fn test_histogram_table_index() {
    let mut table = HistogramTable::new();
    table[Seat::North][Strain::Clubs][7] = 10;
    assert_eq!(table[Seat::North][Strain::Clubs][7], 10);
    assert_eq!(table[Seat::South][Strain::Clubs][7], 0);
}

// ===== average_ns_par =====

fn bid_of(level: u8, strain: Strain) -> Bid {
    Bid {
        level: Level::new(level),
        strain,
    }
}

/// Histogram with one entry per `(seat, strain)` at the given trick count.
fn uniform_hist(tricks: u8) -> HistogramTable {
    let mut hist = HistogramTable::new();
    for seat in Seat::ALL {
        for strain in Strain::ASC {
            hist[seat][strain][usize::from(tricks)] = 1;
        }
    }
    hist
}

#[test]
fn test_par_empty_histogram_returns_none() {
    let hist = HistogramTable::new();
    assert_eq!(average_ns_par(hist, Vulnerability::NONE, Seat::North), None);
}

#[test]
fn test_par_pass_out_when_every_contract_loses() {
    // Six tricks for everyone in every strain: any contract goes down at
    // least one trick, so neither side wants to bid.
    let hist = uniform_hist(6);
    let par = average_ns_par(hist, Vulnerability::NONE, Seat::North).unwrap();
    assert_eq!(par.score, 0.0);
    assert_eq!(par.contract, None);
}

#[test]
fn test_par_pass_out_independent_of_dealer() {
    let hist = uniform_hist(6);
    for dealer in Seat::ALL {
        let par = average_ns_par(hist, Vulnerability::NONE, dealer).unwrap();
        assert_eq!(par.score, 0.0, "dealer {dealer:?}");
        assert_eq!(par.contract, None, "dealer {dealer:?}");
    }
}

#[test]
fn test_par_ns_partial_1nt() {
    // NS makes 1NT (7 tricks); EW takes 6 in NT and everyone takes 6 in
    // every other strain — so EW cannot profitably compete.
    let mut hist = uniform_hist(6);
    hist[Seat::North][Strain::Notrump] = [0; 14];
    hist[Seat::North][Strain::Notrump][7] = 1;
    hist[Seat::South][Strain::Notrump] = [0; 14];
    hist[Seat::South][Strain::Notrump][7] = 1;

    let par = average_ns_par(hist, Vulnerability::NONE, Seat::North).unwrap();

    let one_nt = Contract {
        bid: bid_of(1, Strain::Notrump),
        penalty: Penalty::Undoubled,
    };
    let expected = f64::from(one_nt.score(7, false));

    assert_eq!(par.score, expected);
    let (contract, declarer) = par.contract.expect("expected a par contract");
    assert_eq!(contract, one_nt);
    assert!(matches!(declarer, Seat::North | Seat::South));
}

#[test]
fn test_par_ns_game_4h_vul() {
    // NS takes 10 tricks in hearts, all else stays at 6 tricks.
    let mut hist = uniform_hist(6);
    hist[Seat::North][Strain::Hearts] = [0; 14];
    hist[Seat::North][Strain::Hearts][10] = 1;
    hist[Seat::South][Strain::Hearts] = [0; 14];
    hist[Seat::South][Strain::Hearts][10] = 1;

    let par = average_ns_par(hist, Vulnerability::NS, Seat::North).unwrap();

    let four_h = Contract {
        bid: bid_of(4, Strain::Hearts),
        penalty: Penalty::Undoubled,
    };
    let expected = f64::from(four_h.score(10, true));

    assert_eq!(par.score, expected);
    let (contract, declarer) = par.contract.expect("expected a par contract");
    assert_eq!(contract, four_h);
    assert!(matches!(declarer, Seat::North | Seat::South));
}

#[test]
fn test_par_ew_sacrifice_against_vulnerable_game() {
    // NS vulnerable, EW non-vulnerable. NS makes 4H (+620), but EW can
    // sacrifice in 4S taking 9 tricks (down 1 doubled NV = -100 EW = +100 NS).
    let mut hist = uniform_hist(6);
    hist[Seat::North][Strain::Hearts] = [0; 14];
    hist[Seat::North][Strain::Hearts][10] = 1;
    hist[Seat::South][Strain::Hearts] = [0; 14];
    hist[Seat::South][Strain::Hearts][10] = 1;
    hist[Seat::East][Strain::Spades] = [0; 14];
    hist[Seat::East][Strain::Spades][9] = 1;
    hist[Seat::West][Strain::Spades] = [0; 14];
    hist[Seat::West][Strain::Spades][9] = 1;

    let par = average_ns_par(hist, Vulnerability::NS, Seat::North).unwrap();

    let four_sx = Contract {
        bid: bid_of(4, Strain::Spades),
        penalty: Penalty::Doubled,
    };
    // EW going down 1 doubled NV gives NS the absolute value of that score.
    let expected = -f64::from(four_sx.score(9, false));

    assert_eq!(par.score, expected);
    let (contract, declarer) = par.contract.expect("expected a par contract");
    assert_eq!(contract, four_sx);
    assert!(matches!(declarer, Seat::East | Seat::West));
}

#[test]
fn test_par_count_averages_across_deals() {
    // Two deals: deal A pass-out, deal B 1NT making by NS. Expected NS par
    // average = (0 + 90) / 2 = 45.
    let mut hist = uniform_hist(6); // first deal: pass-out
    // Second deal contribution: bump NS NT entry from 6 → also count one at 7.
    hist[Seat::North][Strain::Notrump][7] = 1;
    hist[Seat::South][Strain::Notrump][7] = 1;
    // Other strains for the second deal: mark 6 again so each (seat, strain)
    // has count 2 — preserves the table-wide invariant of equal counts.
    for seat in Seat::ALL {
        for strain in Strain::ASC {
            if !(matches!(seat, Seat::North | Seat::South) && strain == Strain::Notrump) {
                hist[seat][strain][6] = 2;
            }
        }
    }
    // Set NT for EW to count 2 at 6 tricks too (one per deal).
    hist[Seat::East][Strain::Notrump][6] = 2;
    hist[Seat::West][Strain::Notrump][6] = 2;

    // Now hist[NS][NT] has [..., 6→1, 7→1, ...] = 2 entries; everywhere
    // else has count 2 at index 6. Total deals = 2.
    assert_eq!(hist.count().map(|n| n.get()), Some(2));

    let par = average_ns_par(hist, Vulnerability::NONE, Seat::North).unwrap();

    // The algorithm sums per-deal scores for a single (bid, penalty) and
    // picks the penalty minimising the result for declarer (the one
    // opponents would choose); the par is that sum divided by count:
    //   undoubled: score(1NT, 7) + score(1NT, 6) = 90 + (-50) = 40
    //   doubled  : score(1NTx, 7) + score(1NTx, 6) = 180 + (-100) = 80
    //   min → 40, divided by 2 → 20.
    let one_nt = Contract {
        bid: bid_of(1, Strain::Notrump),
        penalty: Penalty::Undoubled,
    };
    let one_ntx = Contract {
        bid: bid_of(1, Strain::Notrump),
        penalty: Penalty::Doubled,
    };
    let undoubled = i64::from(one_nt.score(7, false)) + i64::from(one_nt.score(6, false));
    let doubled = i64::from(one_ntx.score(7, false)) + i64::from(one_ntx.score(6, false));
    let expected = undoubled.min(doubled) as f64 / 2.0;

    let (contract, _) = par.contract.expect("expected a par contract");
    assert_eq!(contract, one_nt);
    assert!(
        (par.score - expected).abs() < 1e-9,
        "got {} vs {expected}",
        par.score
    );
}
