use super::array::Array;
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

/// Natural logarithm of odds
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Logit(pub f32);

impl Logit {
    /// Negative infinity, corresponding to zero probability
    pub const NEVER: Self = Self(-f32::INFINITY);

    /// The greater logit
    #[must_use] 
    pub const fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }

    /// The lesser logit
    #[must_use] 
    pub const fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }
}

impl Default for Logit {
    fn default() -> Self {
        Self::NEVER
    }
}

impl Add for Logit {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for Logit {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sub for Logit {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for Logit {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl Mul<f32> for Logit {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl MulAssign<f32> for Logit {
    fn mul_assign(&mut self, rhs: f32) {
        self.0 *= rhs;
    }
}

impl Mul<Logit> for f32 {
    type Output = Logit;

    fn mul(self, rhs: Logit) -> Self::Output {
        Logit(rhs.0 * self)
    }
}

impl Div<f32> for Logit {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl DivAssign<f32> for Logit {
    fn div_assign(&mut self, rhs: f32) {
        self.0 /= rhs;
    }
}

impl Array<Logit> {
    /// Apply softmax to the array, returning a probability distribution
    pub fn softmax(&self) -> Array<f32> {
        let max = self.values().copied().fold(Logit::NEVER, Logit::max);
        let mut result = self.map(|_, logit| (logit.0 - max.0).exp());
        let sum: f32 = result.values().copied().sum();

        result.values_mut().for_each(|value| *value /= sum);
        result
    }
}
