use dds_bridge::{Seat, Strain};
use pons::stats::{Accumulator, HistogramRow, HistogramTable, Statistics};

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
