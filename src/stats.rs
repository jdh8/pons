use super::bidding::array::Array;
use super::bidding::Call;
use core::fmt;
use core::ops::{Index, IndexMut};
use dds_bridge::solver::{self, SystemError, Vulnerability};
use dds_bridge::{Contract, Penalty, Seat, Strain};
use std::num::NonZeroUsize;

/// Representation of statistics on a variable
///
/// This struct is merely a pair of mean and standard deviation.  It does not
/// specify if the standard deviation is a sample or population one.  Usually,
/// [`Accumulator`] makes such distinction instead.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Statistics {
    mean: f64,
    sd: f64,
}

impl Statistics {
    /// Construct a new statistics with the given mean and standard deviation
    #[must_use]
    pub const fn new(mean: f64, sd: f64) -> Self {
        Self { mean, sd }
    }

    /// Mean, a.k.a. average or expected value
    #[must_use]
    pub const fn mean(self) -> f64 {
        self.mean
    }

    /// Standard deviation, a measure of dispersion
    #[must_use]
    pub const fn sd(self) -> f64 {
        self.sd
    }
}

impl fmt::Display for Statistics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.mean.fmt(f)?;
        " ± ".fmt(f)?;
        self.sd.fmt(f)
    }
}

/// Accumulator for computing mean and standard deviation
///
/// This accumulator uses constant space while keeping numerical stability.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Accumulator {
    count: usize,
    mean: f64,
    sdm: f64,
}

impl Accumulator {
    /// Construct a new accumulator
    #[must_use]
    pub const fn new() -> Self {
        Self {
            count: 0,
            mean: 0.0,
            sdm: 0.0,
        }
    }

    /// The number of seen values
    #[must_use]
    pub const fn count(self) -> usize {
        self.count
    }

    /// The mean of the seen values
    #[must_use]
    pub const fn mean(self) -> f64 {
        self.mean
    }

    /// [Squared deviations from the mean](https://en.wikipedia.org/wiki/Squared_deviations_from_the_mean)
    #[must_use]
    pub const fn sdm(self) -> f64 {
        self.sdm
    }

    /// Update the accumulator with a new value
    #[allow(clippy::cast_precision_loss)]
    pub fn push(&mut self, x: f64) {
        let delta = x - self.mean;
        self.count += 1;
        self.mean += delta / self.count as f64;
        self.sdm += delta * (x - self.mean);
    }

    /// Compute population mean and standard deviation
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn population(self) -> Statistics {
        Statistics {
            mean: if self.count == 0 { f64::NAN } else { self.mean },
            sd: (self.sdm / self.count as f64).sqrt(),
        }
    }

    /// Compute sample mean and standard deviation
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn sample(self) -> Statistics {
        Statistics {
            mean: if self.count == 0 { f64::NAN } else { self.mean },
            sd: (self.sdm / (self.count.max(1) - 1) as f64).sqrt(),
        }
    }
}

/// Histograms of tricks taken by a seat in all strains
///
/// Each strain either contains no data or the same nonzero number of entries.
/// This invariant is not enforced by the type system, but it is expected to be
/// upheld by the code.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct HistogramRow([[usize; 14]; 5]);

impl HistogramRow {
    /// Constant default constructor
    #[must_use]
    pub const fn new() -> Self {
        Self([[0; 14]; 5])
    }

    /// Count the total number of entries in the histogram
    #[must_use]
    pub fn count(&self) -> usize {
        self.0
            .into_iter()
            .find_map(|hist| NonZeroUsize::new(hist.into_iter().sum()))
            .map_or(0, NonZeroUsize::get)
    }
}

impl Index<Strain> for HistogramRow {
    type Output = [usize; 14];

    fn index(&self, index: Strain) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl IndexMut<Strain> for HistogramRow {
    fn index_mut(&mut self, index: Strain) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}

/// Histograms of tricks taken by all seats in all strains
///
/// Each seat contains the same number of entries, which is the total number of
/// solved deals.  This invariant is not enforced by the type system, but it is
/// expected to be upheld by the code.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct HistogramTable([HistogramRow; 4]);

impl HistogramTable {
    /// Constant default constructor
    #[must_use]
    pub const fn new() -> Self {
        Self([HistogramRow::new(); 4])
    }

    /// Count the total number of entries in the histogram
    #[must_use]
    pub fn count(self) -> usize {
        self.0[0].count()
    }
}

impl Index<Seat> for HistogramTable {
    type Output = HistogramRow;

    fn index(&self, index: Seat) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl IndexMut<Seat> for HistogramTable {
    fn index_mut(&mut self, index: Seat) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}

impl FromIterator<solver::TricksTable> for HistogramTable {
    fn from_iter<I: IntoIterator<Item = solver::TricksTable>>(iter: I) -> Self {
        iter.into_iter().fold(Self::new(), |mut hist, tricks| {
            for seat in Seat::ALL {
                for strain in Strain::ASC {
                    hist[seat][strain][usize::from(tricks[strain].get(seat))] += 1;
                }
            }
            hist
        })
    }
}

/// Calculate average NS par score from the solved deals.
///
/// This idea is inspired by [Cuebids](https://cuebids.com/).
///
/// # Errors
///
/// A [`dds_bridge::solver::SystemError`] propagated from DDS or a
/// [`std::sync::PoisonError`]
pub fn average_ns_par(
    histogram: HistogramTable,
    vul: Vulnerability,
    dealer: Seat,
) -> Result<(f64, Option<(Contract, Seat)>), SystemError> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    const fn score(contract: Contract, hist: [usize; 14], vul: bool) -> i64 {
        let mut sum = 0;
        let mut tricks = 0;

        while tricks <= 13 {
            sum += (hist[tricks] as i64) * contract.score(tricks as u8, vul) as i64;
            tricks += 1;
        }
        sum
    }

    // seat -> bid -> (score, contract)
    let scores = Seat::ALL.map(|seat| {
        let side = match seat {
            Seat::North | Seat::South => Vulnerability::NS,
            Seat::East | Seat::West => Vulnerability::EW,
        };

        let mut array = Array::from_fn(|call| match call {
            Call::Bid(bid) => {
                let normal = Contract {
                    bid,
                    penalty: Penalty::Undoubled,
                };
                let doubled = Contract {
                    bid,
                    penalty: Penalty::Doubled,
                };
                let hist = histogram[seat][bid.strain];
                let normal = (score(normal, hist, vul.contains(side)), Some(normal));
                let doubled = (score(doubled, hist, vul.contains(side)), Some(doubled));
                normal.min(doubled)
            }
            _ => (0, None),
        });

        let slice = &mut array[..];
        for i in (0..slice.len() - 1).rev() {
            slice[i] = slice[i].max(slice[i + 1]);
        }

        match seat {
            Seat::North | Seat::South => array,
            Seat::East | Seat::West => array.map(|_, (score, contract)| (-score, contract)),
        }
    });

    let mut par_score = 0;
    let mut par_contract: Option<(Contract, Seat)> = None;

    let mut improve_for = |seat: Seat| {
        let call = par_contract.map_or(Call::Pass, |(contract, _)| contract.bid.into());
        if let (score, Some(contract)) = scores[seat as usize][call]
            && match seat {
                Seat::North | Seat::South => score > par_score,
                Seat::East | Seat::West => score < par_score,
            }
        {
            par_score = score;
            par_contract.replace((contract, seat));
        }
    };
    improve_for(dealer);
    improve_for(dealer.rho());
    improve_for(dealer.partner());
    improve_for(dealer.lho());
    improve_for(dealer);

    #[allow(clippy::cast_precision_loss)]
    Ok((par_score as f64 / (histogram.count() as f64), par_contract))
}
