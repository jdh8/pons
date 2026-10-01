//! Exact mass of a reading over one hidden hand — Phase 0 of
//! `docs/exact-posterior.md`.
//!
//! `P(hidden hand ∈ union | my thirteen cards)` by counting instead of
//! sampling.  Per suit, every subset of the cards I cannot see is keyed by what
//! a box reads — length, raw HCP, and whether the holding is a
//! [`wasted`] honor; the four censuses convolve over the 560 shapes into
//! `(shape, points)` fibres, and a union's mass is the sum of the fibres some
//! box admits.  Enumerating atoms is what makes overlapping boxes free: a fibre
//! lies in some box or in none, so there is no inclusion–exclusion to pay.
//!
//! Membership is the **lenient** [`Envelope::admits`][pons::bidding::inference::Envelope::admits] — lengths and the
//! `points` gauge — because that is what [`EnvelopeUnion::contains`] and the
//! sampler test.  `probe-exact-mass` pins it.

use contract_bridge::eval::hcp;
use contract_bridge::{Hand, Holding, Suit};
use pons::bidding::constraint::wasted;
use pons::bidding::inference::EnvelopeUnion;

/// One more than the largest point count: 37 HCP plus the upgrade's 2
const POINTS: usize = 40;

/// Every hidden hand, `C(39, 13)` — whatever I hold
pub const TOTAL: u64 = 8_122_425_444;

/// Hands per (raw HCP, wasted holdings) — one suit's census at one length, or
/// the running convolution of several suits
type Joint = [[u64; 5]; 38];

/// What a box reads off one suit: its length, raw HCP, and wasted-honor flag
pub fn holding_key(holding: Holding) -> (usize, usize, usize) {
    (
        holding.len(),
        usize::from(hcp::<u8>(holding)),
        usize::from(wasted(holding)),
    )
}

/// `pons::bidding::constraint::point_count` from the per-suit keys' sums
///
/// ponytail: a copy of the default `PointScale::PointCount` (raw HCP + the
/// shape upgrade less one per wasted holding, floored at zero), because
/// `point_count_on` is `pub(crate)`.  The other scales are functions of the
/// same three arguments — make this a `match` when the counter moves in-crate.
/// `probe-exact-mass` fails if this drifts from the crate's count.
pub fn points(shape: [u8; 4], hcp: usize, wasted: usize) -> usize {
    let mut sorted = shape;
    sorted.sort_unstable();
    let balanced = sorted[0] >= 2 && sorted[1] >= 3;
    let base = usize::from(!balanced) + usize::from(sorted[2] + sorted[3] >= 10);
    hcp + base.saturating_sub(wasted)
}

/// Whether some box of `union` admits a (shape, points) fibre
fn admits(union: &EnvelopeUnion, shape: [u8; 4], points: usize) -> bool {
    union.boxes().iter().any(|envelope| {
        (envelope.lengths.iter().zip(shape)).all(|(range, len)| range.contains(len))
            // A point count is below `POINTS`, so the cast cannot truncate.
            && envelope.strength.points.contains(points as u8)
    })
}

/// [`EnvelopeUnion::contains`] rebuilt from the counter's own keys — the
/// membership the fibres are summed under, exposed so a probe can pin the two
/// against each other hand by hand
pub fn contains(union: &EnvelopeUnion, hand: Hand) -> bool {
    let keys = Suit::ASC.map(|suit| holding_key(hand[suit]));
    let (hcp, wasted) = keys.iter().fold((0, 0), |a, k| (a.0 + k.1, a.1 + k.2));
    // A suit holds at most 13 cards, so the cast cannot truncate.
    let shape = keys.map(|key| key.0 as u8);
    admits(union, shape, points(shape, hcp, wasted))
}

fn fold(joint: &Joint, suit: &[[u64; 2]; 11]) -> Joint {
    let mut out = [[0; 5]; 38];
    for (h, row) in joint.iter().enumerate() {
        for (w, &n) in row.iter().enumerate() {
            if n == 0 {
                continue;
            }
            for (dh, cell) in suit.iter().enumerate() {
                for (dw, &k) in cell.iter().enumerate() {
                    if k != 0 {
                        out[h + dh][w + dw] += n * k;
                    }
                }
            }
        }
    }
    out
}

/// The hands one hidden seat can hold beside mine, counted by (shape, points)
pub struct Counter {
    /// Suit lengths in [`Suit::ASC`] order and the number of hands at each
    /// point count; shapes my own cards rule out are absent
    fibres: Vec<([u8; 4], [u64; POINTS])>,
}

impl Counter {
    /// Count every 13-card hand drawn from the 39 cards outside `seen`
    pub fn new(seen: Hand) -> Self {
        // census[suit][length][hcp][wasted]
        let mut census = [[[[0_u64; 2]; 11]; 14]; 4];
        for (table, suit) in census.iter_mut().zip(Suit::ASC) {
            let unseen = (!seen[suit]).to_bits();
            let mut subset = unseen;
            loop {
                let (len, hcp, wasted) = holding_key(Holding::from_bits_retain(subset));
                table[len][hcp][wasted] += 1;
                if subset == 0 {
                    break;
                }
                subset = (subset - 1) & unseen;
            }
        }

        let mut unit: Joint = [[0; 5]; 38];
        unit[0][0] = 1;
        let mut fibres = Vec::with_capacity(560);
        for a in 0..=13_u8 {
            let clubs = fold(&unit, &census[0][usize::from(a)]);
            for b in 0..=13 - a {
                let minors = fold(&clubs, &census[1][usize::from(b)]);
                for c in 0..=13 - a - b {
                    let shape = [a, b, c, 13 - a - b - c];
                    let three = fold(&minors, &census[2][usize::from(c)]);
                    let joint = fold(&three, &census[3][usize::from(shape[3])]);
                    let mut by_points = [0; POINTS];
                    for (h, row) in joint.iter().enumerate() {
                        for (w, &n) in row.iter().enumerate() {
                            by_points[points(shape, h, w)] += n;
                        }
                    }
                    if by_points.iter().any(|&n| n != 0) {
                        fibres.push((shape, by_points));
                    }
                }
            }
        }
        Self { fibres }
    }

    /// The number of hidden hands some box of `union` admits
    pub fn count(&self, union: &EnvelopeUnion) -> u64 {
        (self.fibres.iter())
            .flat_map(|(shape, by_points)| {
                (by_points.iter().enumerate())
                    .filter(|&(points, &n)| n != 0 && admits(union, *shape, points))
            })
            .map(|(_, &n)| n)
            .sum()
    }

    /// `P(hidden hand ∈ union | my cards)`
    pub fn mass(&self, union: &EnvelopeUnion) -> f64 {
        self.count(union) as f64 / TOTAL as f64
    }
}
