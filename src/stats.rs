use core::fmt;
use core::num::Wrapping;
use dds_bridge::contract::{Bid, Contract, Penalty, Strain};
use dds_bridge::deal::{Deal, Seat};
use dds_bridge::solver::{self, Error, StrainFlags, Vulnerability};

/// Representation of statistics on a variable
///
/// This struct is merely a pair of mean and standard deviation.  It does not
/// specify if the standard deviation is a sample or population one.  Usually,
/// [`Accumulator`] makes such distinction instead.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Statistics {
    /// Mean, a.k.a. average or expected value
    pub mean: f64,
    /// Standard deviation, a measure of dispersion
    pub sd: f64,
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
    /// The number of seen values
    pub count: usize,
    /// The mean of the seen values
    pub mean: f64,
    /// [Squared deviations from the mean](https://en.wikipedia.org/wiki/Squared_deviations_from_the_mean)
    pub sdm: f64,
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

/// Calculate average NS par score from the provided deals.
///
/// This idea is inspired by [Cuebids](https://cuebids.com/).
///
/// # Errors
///
/// A [`dds_bridge::solver::SystemError`] propagated from DDS or a
/// [`std::sync::PoisonError`]
pub fn average_ns_par(
    deals: &[Deal],
    vul: Vulnerability,
    dealer: Seat,
    n: usize,
) -> Result<(f64, Option<(Contract, Seat)>), Error> {
    const BID_VARIANTS: usize = 7 * 5;

    /// Encode a bid to an array index
    const fn encode_bid(bid: Bid) -> usize {
        (bid.level as usize - 1) * 5 + bid.strain as usize
    }

    /// Decode an array index back to the bid
    #[allow(clippy::cast_possible_truncation)]
    const fn decode_bid(code: usize) -> Bid {
        let level = (code / 5) as u8 + 1;
        let strain = Strain::ASC[code % 5];
        Bid { level, strain }
    }

    // Check at compile time that `encode_bid` and `decode_bid` cancel each other
    const _: () = {
        let mut code = 0;

        while code < BID_VARIANTS {
            let bid = decode_bid(code);
            assert!(code == encode_bid(bid));
            code += 1;
        }
    };

    // seat -> strain -> tricks -> frequency
    let histogram = solver::solve_deals(deals, StrainFlags::all())?
        .into_iter()
        .fold([[[0usize; 14]; 5]; 4], |mut hist, tricks| {
            for seat in Seat::ALL {
                for strain in Strain::ASC {
                    hist[seat as usize][strain as usize][usize::from(tricks[strain].get(seat))] +=
                        1;
                }
            }
            hist
        });

    // seat -> bid -> (score, contract)
    let scores = Seat::ALL.map(|seat| {
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

        let side = match seat {
            Seat::North | Seat::South => Vulnerability::NS,
            Seat::East | Seat::West => Vulnerability::EW,
        };

        let mut table: [_; BID_VARIANTS] = core::array::from_fn(|bid| {
            let bid = decode_bid(bid);
            let normal = Contract {
                bid,
                penalty: Penalty::None,
            };
            let doubled = Contract {
                bid,
                penalty: Penalty::Doubled,
            };
            let hist = histogram[seat as usize][bid.strain as usize];
            let normal = (score(normal, hist, vul.contains(side)), normal);
            let doubled = (score(doubled, hist, vul.contains(side)), doubled);
            normal.min(doubled)
        });

        for bid in (0..BID_VARIANTS - 1).rev() {
            table[bid] = table[bid].max(table[bid + 1]);
        }

        match seat {
            Seat::North | Seat::South => table,
            Seat::East | Seat::West => table.map(|(score, contract)| (-score, contract)),
        }
    });

    let mut par_score = 0;
    let mut par_contract: Option<(Contract, Seat)> = None;
    let mut improve_for = |seat: Seat| {
        let bid = par_contract.map_or(0, |(contract, _)| encode_bid(contract.bid));
        let (score, contract) = scores[seat as usize][bid];
        let is_improved = match seat {
            Seat::North | Seat::South => score > par_score,
            Seat::East | Seat::West => score < par_score,
        };
        if is_improved {
            par_score = score;
            par_contract.replace((contract, seat));
        }
    };
    improve_for(dealer);
    improve_for(dealer - Wrapping(1));
    improve_for(dealer - Wrapping(2));
    improve_for(dealer - Wrapping(3));
    improve_for(dealer);

    #[allow(clippy::cast_precision_loss)]
    Ok((par_score as f64 / n as f64, par_contract))
}
