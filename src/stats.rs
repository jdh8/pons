use core::fmt;

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
