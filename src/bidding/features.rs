//! Versioned feature extractor for the AI instinct bidder
//!
//! Converts a bridge hand and its auction [`Context`] into a fixed-size
//! feature vector suitable for input to a neural network.  Every value is
//! normalised so that the expected range is roughly `[0.0, 1.0]`; the exact
//! layout is pinned by [`FEATURES_VERSION_V3`] so that a model trained on one
//! version cannot be accidentally loaded under another.
//!
//! # Layout (version 3 — the restrictive, fully disclosable vector)
//!
//! | Block                | Start | Len |
//! |----------------------|-------|-----|
//! | Disclosable hand     |     0 |  10 |
//! | Context              |    10 |  36 |
//! | Inferences           |    46 |  40 |
//! | Vulnerability        |    86 |   2 |
//! | **Total**            |       | **88** |

use super::card::Card;
use super::context::Context;
use super::inference::{Envelope, EnvelopeUnion, Inferences, Range, Relative};
use crate::bidding::constraint::{upgrade, upgrade_ceiling};
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::eval::{self, HandEvaluator, SimpleEvaluator};
use contract_bridge::{Hand, Holding, Penalty, Rank, Strain, Suit};

/// Layout version tag for the restrictive *disclosable* extractor [`features_v3`]
pub const FEATURES_VERSION_V3: u32 = 3;

/// Length of the restrictive hand block in [`features_v3`]: 4 suits ×
/// `{len, suit_hcp}` (8) plus global `{hcp, shape}` (2).
pub const LEN_HAND_V3: usize = 10;

/// Number of `f32` values returned by [`features_v3`]: a disclosable-only hand
/// summary ([`LEN_HAND_V3`]) plus the shared context/inferences/vulnerability
/// blocks.
pub const FEATURES_LEN_V3: usize = LEN_HAND_V3 + LEN_CONTEXT + LEN_INFERENCES + LEN_VUL;

// ── Block offsets (used in tests and as documentation) ──────────────────────

/// Offset of the context block (36 values)
pub const OFFSET_CONTEXT: usize = LEN_HAND_V3;
/// Length of the context block
pub const LEN_CONTEXT: usize = 36;

/// Offset of the inferences block (40 values)
pub const OFFSET_INFERENCES: usize = OFFSET_CONTEXT + LEN_CONTEXT;
/// Values one player's shown ranges contribute: 4 suits × `{min, max}` length
/// plus `{min, max}` points.
pub const LEN_INFERENCE: usize = 8 + LEN_POINTS;
/// Values one player's shown `points` range contributes: `{min, max}` ÷ 37.
pub const LEN_POINTS: usize = 2;
/// Length of the inferences block (all four seats)
pub const LEN_INFERENCES: usize = 4 * LEN_INFERENCE;

/// Offset of the vulnerability block (2 values)
pub const OFFSET_VUL: usize = OFFSET_INFERENCES + LEN_INFERENCES;
/// Length of the vulnerability block
pub const LEN_VUL: usize = 2;

// ── Private helpers ───────────────────────────────────────────────────────────

/// Minimal append-only target shared by heap-backed policy features and the
/// evaluator's fixed stack buffers.
trait FeatureSink {
    fn push(&mut self, value: f32);
}

impl FeatureSink for Vec<f32> {
    fn push(&mut self, value: f32) {
        Vec::push(self, value);
    }
}

/// Bounds-checked builder for one statically sized evaluator feature vector.
struct FixedFeatures<const N: usize> {
    values: [f32; N],
    len: usize,
}

impl<const N: usize> FixedFeatures<N> {
    const fn new() -> Self {
        Self {
            values: [0.0; N],
            len: 0,
        }
    }

    fn finish(self) -> [f32; N] {
        assert_eq!(self.len, N, "evaluator feature width");
        self.values
    }
}

impl<const N: usize> FeatureSink for FixedFeatures<N> {
    fn push(&mut self, value: f32) {
        let slot = self
            .values
            .get_mut(self.len)
            .expect("evaluator feature extractor exceeded its fixed width");
        *slot = value;
        self.len += 1;
    }
}

/// HCP of a single holding (A=4, K=3, Q=2, J=1)
fn holding_hcp(holding: Holding) -> u8 {
    4 * u8::from(holding.contains(Rank::A))
        + 3 * u8::from(holding.contains(Rank::K))
        + 2 * u8::from(holding.contains(Rank::Q))
        + u8::from(holding.contains(Rank::J))
}

/// Push the disclosable hand summary ([`LEN_HAND_V3`] values): per suit
/// `len/13` and `suit_hcp/10`, then global `hcp/40` and `shape/2`.
fn push_hand(out: &mut impl FeatureSink, hand: Hand) {
    // Per suit: length and suit HCP only — no rank/honor/stopper card detail.
    for suit in Suit::ASC {
        let holding = hand[suit];
        out.push(holding.len() as f32 / 13.0);
        out.push(holding_hcp(holding) as f32 / 10.0);
    }

    // Global strength: HCP and shape (= points − HCP = the fuzzy upgrade, 0–2).
    let hcp = SimpleEvaluator(eval::hcp::<u8>).eval(hand);
    out.push(hcp as f32 / 40.0);
    out.push(upgrade(hand) as f32 / 2.0);
}

/// The five honours [`push_hand_eval`] flags per suit.  Everything else in a
/// suit is a spot card — ranks 2..9, hence the divisor 8 rather than 13.
const HONOURS: [Rank; 5] = [Rank::A, Rank::K, Rank::Q, Rank::J, Rank::T];

/// Push the trick-evaluator hand block ([`LEN_HAND_EVAL`] values): per suit
/// `#spots/8` then one bit each for A, K, Q, J, T.
///
/// This is [`push_hand`]'s granular counterpart, and it is strictly more
/// informative: `len = #spots + ΣA..T` and `suit_hcp = 4A + 3K + 2Q + J` are
/// both one weight away, so the first layer can recover the summary exactly.
/// The globals `hcp` and `shape` are dropped for the same reason — measured
/// free, because `hcp` is a fixed dot product of the sixteen honour bits.
fn push_hand_eval(out: &mut impl FeatureSink, hand: Hand) {
    for suit in Suit::ASC {
        let holding = hand[suit];
        // A suit holds any *subset* of the honours, so count what is actually
        // there; the rest of its length is spot cards.
        let held = HONOURS.map(|rank| holding.contains(rank));
        let spots = holding.len() - held.iter().filter(|&&h| h).count();
        out.push(spots as f32 / 8.0);
        for honour in held {
            out.push(f32::from(honour));
        }
    }
}

std::thread_local! {
    /// Whether the inference blocks are blanked to `Envelope::unknown` (see
    /// [`set_blind_inference`]).  Off by default.
    static BLIND_INFERENCE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Blank every inference block the nets see — the reading program's *negative control*
///
/// Every generator of readings (authored `project`, the agreement overlay behind
/// [`set_announced_reading`][super::inference::set_announced_reading], and any
/// future sampled projection) competes for one prize: the IMPs that flow from
/// the nets reasoning about what the other three seats have shown.  Tightening a
/// reading measures the *derivative* of that prize and lands in the noise.  This
/// knob measures its **level** — on, all four seats read as `Envelope::unknown`
/// and the nets reason from the auction alone.
///
/// The A/B against the shipped default is therefore a ceiling on the whole
/// program: no reading, however well generated, can be worth more than what
/// deleting every reading costs.  Nothing else consumes this — the sampler's
/// containment test, `admits`, and the opening-lead sampling all read the
/// [`Inferences`] directly and are untouched.
///
/// Diagnostic only; never ship it on.  Read at extraction time, per-thread.
#[doc(hidden)]
pub fn set_blind_inference(on: bool) {
    BLIND_INFERENCE.with(|cell| cell.set(on));
}

/// Whether evaluator feature extraction is blind to auction inferences
pub(crate) fn blind_inference() -> bool {
    BLIND_INFERENCE.with(std::cell::Cell::get)
}

/// The reading the nets are fed: the seat's agreement, or nothing under
/// [`set_blind_inference`].
fn shown(player: &Envelope) -> &Envelope {
    // ponytail: one shared `unknown` rather than a per-call temporary — the
    // envelope is immutable and `Envelope::unknown` is `const`.
    const NOTHING: Envelope = Envelope::unknown();
    if blind_inference() { &NOTHING } else { player }
}

/// Push one player's shown ranges ([`LEN_INFERENCE`] values): per suit
/// `{min, max}` length ÷ 13, then `{min, max}` points ÷ 37.  Nothing shown is
/// the `[0, 1]` pattern (`Envelope::unknown`), *not* zeros.
fn push_inference(out: &mut impl FeatureSink, player: &Envelope) {
    let player = shown(player);
    for suit in Suit::ASC {
        let range = player.length(suit);
        out.push(range.min as f32 / 13.0);
        out.push(range.max as f32 / 13.0);
    }
    push_points(out, player);
}

/// Push one player's shown `points` range ([`LEN_POINTS`] values) — the half of
/// [`push_inference`] the shape distribution does *not* replace, because
/// `points` couples to shape only weakly, through [`upgrade`].
///
/// Takes the [`shown`] envelope, not the raw one: [`push_inference`] has already
/// resolved the blind knob by the time it delegates here.
fn push_points(out: &mut impl FeatureSink, shown: &Envelope) {
    let points = net_points(shown);
    out.push(points.min as f32 / 37.0);
    out.push(points.max as f32 / 37.0);
}

/// The points hull served to the nets: the legacy axis with every per-suit
/// support promise folded back in
///
/// The shipped nets were trained on corpora where a fit-showing raise wrote
/// its support-scale band verbatim onto the legacy `points` axis.  The
/// reader now keeps that axis sound (`support_band_to_points` — a 4-point
/// shapely raise sits inside the box), which would hand a trained net an
/// off-distribution widening at every raise auction — the pass-exclusion
/// lesson: a reading change the net consumes is a retrain, not a free edit.
/// Folding the slots back in reconstructs the training-time hull exactly at
/// those nodes and is a no-op elsewhere (an unnarrowed slot is ⊤, and
/// `canonicalize` seeds slot floors only from the raw-HCP floor the `points`
/// axis already carries).  The next feature version retires this fold by
/// serving the support slots as their own columns.
fn net_points(shown: &Envelope) -> Range {
    (shown.strength.support_points.iter())
        .fold(shown.strength.points, |acc, slot| acc.intersect(*slot))
}

// ── The shape-distribution reading ────────────────────────────────────────────

/// Binomial coefficients `C(n, k)` for `n, k ≤ 13` — Pascal's triangle, with
/// the impossible `k > n` entries left at zero so a lookup can be unguarded.
const BINOM: [[u32; 14]; 14] = {
    let mut table = [[0_u32; 14]; 14];
    let mut n = 0;
    while n < 14 {
        table[n][0] = 1;
        let mut k = 1;
        while k <= n {
            table[n][k] = table[n - 1][k - 1] + table[n - 1][k];
            k += 1;
        }
        n += 1;
    }
    table
};

/// Divisor that brings a length standard deviation into roughly `[0, 1]`.  A
/// suit length's unconditional σ is ≈1.8 and the widest a reading can make it
/// is a 0/13 barbell, so 4 covers the realistic span.
const SPREAD_SCALE: f64 = 4.0;

/// Values one hidden seat's **shipped** shape reading contributes: `E[len]` and
/// `sd[len]` per suit, then the log-mass column.
///
/// This is the round-two `gauss-mass` arm, and the width is the point.  It
/// replaces the eight length endpoints with nine columns at **exact NLL par**
/// (+0.00004 against a 94-column control on 8.15M rows) while being invariant to
/// information-preserving re-hulling, which the endpoints are not.
pub const LEN_SHAPE_GAUSS: usize = 8 + 1;

/// Values one hidden seat's shape distribution contributes to the ablation
/// superset: [`LEN_SHAPE_GAUSS`] plus the full per-suit length marginal
/// `P(len = k)` for `k = 0..=13` (56).
///
/// Round two of the ablation replaced round one's hand-picked functionals with
/// the marginal itself.  Measured: the six `Cov[len_s, len_t]` off-diagonals
/// were worth **0.00001** — the joint term a shape distribution is supposed to
/// buy, and it bought nothing — while the histogram is worth **+0.0008** over
/// the Gaussian summary, barely past the 0.0006 seed spread and costing 168
/// extra columns per row for it.  Hence [`features_eval_v4`] ships the Gaussian
/// and this stays research-only; see docs/ai-bidder/evaluator-net.md.
pub const LEN_SHAPE: usize = LEN_SHAPE_GAUSS + 4 * 14;

/// Per-suit weights over the 39 cards this hand does not hold: `w[s][k]` is the
/// number of ways one hidden seat holds exactly `k` cards of suit `s`, namely
/// `C(13 − my_len_s, k)`.
///
/// Built once per hand and shared by all three hidden seats — only the
/// membership mask differs between them.
struct Unseen([[f64; 14]; 4]);

impl Unseen {
    fn new(hand: Hand) -> Self {
        Self(std::array::from_fn(|s| {
            let unseen = 13 - hand[Suit::ASC[s]].len();
            std::array::from_fn(|k| f64::from(BINOM[unseen][k]))
        }))
    }
}

/// Weighted sums over the shape lattice, before normalisation
#[derive(Default)]
struct Moments {
    /// Total weight over all 560 shapes — always `C(39, 13)`, by Vandermonde
    all: f64,
    /// Total weight over the shapes the reading admits
    hit: f64,
    /// `Σ w·len_s`
    sum: [f64; 4],
    /// `Σ w·len_s²`
    square: [f64; 4],
    /// `Σ w` over each `(suit, length)` cell — the per-suit length marginal
    histogram: [[f64; 14]; 4],
}

/// Walk all 560 shapes — every 4-tuple of suit lengths summing to 13 — and
/// accumulate the hypergeometric weight of those the reading admits.
///
/// `boxes` is `None` for a reading that shows nothing.  Enumerating the *atoms*
/// is what makes a union of boxes free here: a shape either lies in some box or
/// in none, so there is no inclusion–exclusion to pay and no overlap to cap.
fn walk_shapes(unseen: &Unseen, boxes: Option<&[Envelope]>) -> Moments {
    let w = &unseen.0;
    let mut m = Moments::default();
    for a in 0..=13 {
        for b in 0..=13 - a {
            for c in 0..=13 - a - b {
                let d = 13 - a - b - c;
                let weight = w[0][a] * w[1][b] * w[2][c] * w[3][d];
                if weight == 0.0 {
                    continue;
                }
                m.all += weight;
                let shape = [a as u8, b as u8, c as u8, d as u8];
                let admitted = boxes.is_none_or(|boxes| {
                    boxes.iter().any(|envelope| {
                        envelope
                            .lengths
                            .iter()
                            .zip(shape)
                            .all(|(range, len)| range.contains(len))
                    })
                });
                if !admitted {
                    continue;
                }
                m.hit += weight;
                for (s, &len) in shape.iter().enumerate() {
                    m.histogram[s][usize::from(len)] += weight;
                    let len = f64::from(len);
                    m.sum[s] += weight * len;
                    m.square[s] += weight * len * len;
                }
            }
        }
    }
    m
}

/// One hidden seat's shape distribution, normalised — the shared core of the
/// shipped [`push_shape_gauss`] block and the [`push_shape_dist`] superset.
struct Shape {
    /// `E[len_s]`, in cards
    mean: [f64; 4],
    /// `sd[len_s]`, in cards
    sd: [f64; 4],
    /// `P(len_s = k)`
    histogram: [[f64; 14]; 4],
    /// How much the reading pins the seat down, in `[0, 1]`: 0 when it admits
    /// every shape, 1 when it admits a single hand.  The total weight is
    /// `C(39, 13)` for every hand, so the scale is fixed rather than
    /// hand-dependent.
    mass: f64,
}

/// Walk the lattice once and normalise.
fn shape_of(unseen: &Unseen, boxes: Option<&[Envelope]>) -> Shape {
    let m = walk_shapes(unseen, boxes);
    // A sound reading contains the truth, so it cannot exclude every shape —
    // but `announced` carries an *agreement*, which can over-claim against a
    // hand that saw a card the agreement did not expect.  Read that as "nothing
    // shown" rather than dividing by zero.
    let m = if m.hit > 0.0 {
        m
    } else {
        debug_assert!(boxes.is_some(), "the unconditional walk always has mass");
        walk_shapes(unseen, None)
    };
    let inv = 1.0 / m.hit;
    let mean = m.sum.map(|sum| sum * inv);
    Shape {
        mean,
        sd: std::array::from_fn(|s| (m.square[s] * inv - mean[s] * mean[s]).max(0.0).sqrt()),
        histogram: m.histogram.map(|row| row.map(|cell| cell * inv)),
        mass: -(m.hit * (1.0 / m.all)).ln() / m.all.ln(),
    }
}

/// Push one hidden seat's **shape reading** ([`LEN_SHAPE_GAUSS`] values) — the
/// distributional twin of [`push_inference`]'s length endpoints, and what
/// [`features_eval_v4`] ships in their place.
///
/// The hull encoding is not invariant to information-preserving
/// re-representation: `♥5..13` and `♥5..8` are the same claim (`E[len] = 5.36`
/// and the same mass to four digits — nine-plus suits are 0.04% of the 5+ mass)
/// yet `max/13` moves 1.00 → 0.62.  A closure that provably rejects no hand
/// therefore displaces the net's inputs by multiple σ, which is why every
/// hull-tightening chop has had to buy a retrain before it could be judged on
/// merit.  Conditioning on the *distribution* the reading describes removes that
/// by construction: re-hulling a box without moving mass leaves every column
/// here unchanged.
///
/// Conditions on lengths only — `points` couples to shape weakly through
/// [`upgrade`] and its two endpoint columns stay beside this block — and on one
/// seat at a time, marginalising over the other two hidden hands exactly as the
/// hull it replaces does.
fn push_shape_gauss(out: &mut impl FeatureSink, shape: &Shape) {
    for value in shape.mean {
        out.push((value / 13.0) as f32);
    }
    for value in shape.sd {
        out.push((value / SPREAD_SCALE) as f32);
    }
    out.push(shape.mass as f32);
}

/// Push one hidden seat's **shape distribution** ([`LEN_SHAPE`] values) — the
/// distributional twin of [`push_inference`]'s bounding box.
///
/// The hull encoding is not invariant to information-preserving
/// re-representation: `♥5..13` and `♥5..8` are the same claim (`E[len] = 5.36`
/// and the same mass to four digits — nine-plus suits are 0.04% of the 5+ mass)
/// yet `max/13` moves 1.00 → 0.62.  A closure that provably rejects no hand
/// therefore displaces the net's inputs by multiple σ, which is why every
/// hull-tightening chop has had to buy a retrain before it could be judged on
/// merit.  Conditioning on the *distribution* the reading describes removes that
/// by construction: re-hulling a box without moving mass leaves every column
/// here unchanged.
///
/// Conditions on lengths only — `points` couples to shape weakly through
/// [`upgrade`] and its two endpoint columns stay beside this block — and on one
/// seat at a time, marginalising over the other two hidden hands exactly as the
/// hull it replaces does.  The marginal is per suit, so genuinely *joint*
/// structure (5-4 in the majors) survives only through `Σ len = 13`; round one
/// priced the explicit covariances at 0.00001 and they are not carried.
///
/// Layout is [`push_shape_gauss`]'s summary, the marginal, then the log-mass —
/// the mass column stays last so the ablation's block offsets keep their
/// meaning.
fn push_shape_dist(out: &mut impl FeatureSink, shape: &Shape) {
    for value in shape.mean {
        out.push((value / 13.0) as f32);
    }
    for value in shape.sd {
        out.push((value / SPREAD_SCALE) as f32);
    }
    for row in shape.histogram {
        for cell in row {
            out.push(cell as f32);
        }
    }
    out.push(shape.mass as f32);
}

/// The boxes the nets are fed: the seat's agreement union, or nothing under
/// [`set_blind_inference`].  `None` means "shows nothing", the ⊤ reading.
fn shown_boxes(union: &EnvelopeUnion) -> Option<&[Envelope]> {
    (!BLIND_INFERENCE.with(std::cell::Cell::get)).then(|| union.boxes())
}

// ── The HCP-distribution reading ──────────────────────────────────────────────

/// Binomial coefficients `C(n, k)` for `n ≤ 39`, `k ≤ 13` — Pascal's triangle
/// again, wide enough for the honour walk.
///
/// [`BINOM`] cannot serve it twice over: it stops at `n = 13`, while the
/// non-honour factor runs to `C(39, 13) ≈ 8.1e9`, which overflows the `u32` it
/// stores.  `k > n` entries stay zero so a lookup can be unguarded.
const BINOM_39: [[u64; 14]; 40] = {
    let mut table = [[0_u64; 14]; 40];
    let mut n = 0;
    while n < 40 {
        table[n][0] = 1;
        let mut k = 1;
        while k < 14 {
            table[n][k] = if n == 0 {
                0
            } else {
                table[n - 1][k - 1] + table[n - 1][k]
            };
            k += 1;
        }
        n += 1;
    }
    table
};

/// Divisor that brings an HCP standard deviation into roughly `[0, 1]`.  A
/// hidden hand's unconditional σ is ≈4 and the widest a reading can leave it is
/// a barbell across the band, so 8 covers the realistic span.
const HCP_SPREAD_SCALE: f64 = 8.0;

/// Values one hidden seat's raw-HCP endpoints contribute: `{min, max}` ÷ 37
pub const LEN_HCP_ENDS: usize = 2;

/// Values one hidden seat's strength reading contributes: `E[hcp]`, `sd[hcp]`,
/// then the log-mass column
pub const LEN_HCP_GAUSS: usize = 3;

/// The most HCP thirteen cards can hold — AAAA KKKK QQQQ J.  Also the width of
/// the [`walk_hcp`] admission mask, which is why it must stay under 64.
const HCP_CAP: u8 = 37;

/// What each honour class is worth, in [`UnseenHonours`] order
const HONOUR_HCP: [Rank; 4] = [Rank::A, Rank::K, Rank::Q, Rank::J];

/// Ways one hidden seat can hold each honour class — the honour analogue of
/// [`Unseen`], and the same trick: `counts[c]` is how many of class `c` this
/// hand does not hold, `classes[c][n]` is `C(counts[c], n)`, and `rest[k]` is
/// `C(unseen non-honours, k)`.
///
/// Built once per hand and shared by all three hidden seats — only the
/// admission mask differs between them.
struct UnseenHonours {
    counts: [u8; 4],
    classes: [[f64; 5]; 4],
    rest: [f64; 14],
}

impl UnseenHonours {
    fn new(hand: Hand) -> Self {
        let counts = HONOUR_HCP.map(|rank| {
            let held = Suit::ASC
                .iter()
                .filter(|&&suit| hand[suit].contains(rank))
                .count();
            // Four of each rank exist, so what this hand lacks is unseen.
            4 - held as u8
        });
        let honours: u8 = counts.iter().sum();
        let rest = usize::from(39 - honours);
        Self {
            counts,
            classes: counts.map(|n| std::array::from_fn(|k| BINOM_39[usize::from(n)][k] as f64)),
            rest: std::array::from_fn(|k| BINOM_39[rest][k] as f64),
        }
    }
}

/// Bits `lo..=hi` of a `u64`, empty when the band is
fn band_mask(lo: u8, hi: u8) -> u64 {
    let hi = hi.min(HCP_CAP);
    if lo > hi {
        return 0;
    }
    let width = u32::from(hi - lo) + 1;
    (u64::MAX >> (64 - width)) << lo
}

/// Bitmask over `0..=HCP_CAP` of the raw-HCP values one box admits
///
/// Reads **both** whole-hand strength axes, which is where the information the
/// hull cannot carry actually is.  `strength.hcp` is the crisp raw band an
/// HCP-gated rule writes — a 1NT box holds 15..17 there while the `points` leg
/// beside it is slacked to 15..19 — and no shipped vector has ever read it.
/// `points = hcp + upgrade` with `upgrade` in `0..=ceiling`, so the points leg
/// bounds raw HCP too; [`upgrade_ceiling`] supplies the ceiling *box-locally*,
/// which is 0 for a box whose lengths force balanced, so a 1NT box pins raw HCP
/// from both ends.  It returns `None` under rule-of-N+8, whose count can fall
/// below raw HCP; the leg is then dropped, which is sound because wider is.
fn box_hcp_mask(envelope: &Envelope) -> u64 {
    let hcp = envelope.strength.hcp;
    let (mut lo, mut hi) = (hcp.min, hcp.max);
    if let Some(ceiling) = upgrade_ceiling(&envelope.lengths) {
        let points = envelope.strength.points;
        lo = lo.max(points.min.saturating_sub(ceiling));
        hi = hi.min(points.max);
    }
    band_mask(lo, hi)
}

/// Weighted sums over the HCP lattice, before normalisation
#[derive(Default)]
struct HcpMoments {
    /// Total weight over every honour split — always `C(39, 13)`, by Vandermonde
    all: f64,
    /// Total weight over the splits the reading admits
    hit: f64,
    /// `Σ w·hcp`
    sum: f64,
    /// `Σ w·hcp²`
    square: f64,
}

/// Walk every `(A, K, Q, J)` count the unseen honours allow — at most 625 atoms
/// — and accumulate the hypergeometric weight of those the reading admits.
///
/// Enumerating atoms makes a union of boxes free for the same reason it does in
/// [`walk_shapes`]: a split lies in some box or in none, so there is no
/// inclusion–exclusion to pay.  Here it collapses further — raw HCP is a single
/// scalar axis, so the whole union is a 38-bit mask and admission is a shift.
///
/// ponytail: this is the *marginal* HCP walk. It reads each box's strength
/// axes and ignores its lengths, exactly as `walk_shapes` reads lengths and
/// ignores strength, so a box that couples the two ("weak with six spades")
/// contributes its strength leg and its shape leg independently. The joint walk
/// is a per-suit honour lattice (2⁴ states per suit, 65536 atoms) that could
/// also read `suit_hcp`; it costs ~100× this one and is the upgrade path if the
/// marginal measures interesting.
fn walk_hcp(unseen: &UnseenHonours, admitted: u64) -> HcpMoments {
    let mut m = HcpMoments::default();
    for a in 0..=unseen.counts[0] {
        for k in 0..=unseen.counts[1] {
            for q in 0..=unseen.counts[2] {
                for j in 0..=unseen.counts[3] {
                    let held = a + k + q + j;
                    if held > 13 {
                        continue;
                    }
                    let weight = unseen.classes[0][usize::from(a)]
                        * unseen.classes[1][usize::from(k)]
                        * unseen.classes[2][usize::from(q)]
                        * unseen.classes[3][usize::from(j)]
                        * unseen.rest[usize::from(13 - held)];
                    if weight == 0.0 {
                        continue;
                    }
                    m.all += weight;
                    // Thirteen cards cap this at 37, so the shift is in range.
                    let hcp = 4 * a + 3 * k + 2 * q + j;
                    debug_assert!(hcp <= HCP_CAP, "{hcp} HCP in thirteen cards");
                    if admitted >> hcp & 1 == 0 {
                        continue;
                    }
                    m.hit += weight;
                    let hcp = f64::from(hcp);
                    m.sum += weight * hcp;
                    m.square += weight * hcp * hcp;
                }
            }
        }
    }
    m
}

/// One hidden seat's raw-HCP distribution, normalised
struct HcpDist {
    /// `E[hcp]`, in points
    mean: f64,
    /// `sd[hcp]`, in points
    sd: f64,
    /// How much the reading pins the seat's strength down, in `[0, 1]` — the
    /// strength twin of [`Shape::mass`], on the same `C(39, 13)` scale
    mass: f64,
}

/// Walk the lattice once and normalise.
fn hcp_of(unseen: &UnseenHonours, boxes: Option<&[Envelope]>) -> HcpDist {
    let admitted = boxes.map_or(u64::MAX, |boxes| {
        boxes
            .iter()
            .fold(0, |mask, envelope| mask | box_hcp_mask(envelope))
    });
    let m = walk_hcp(unseen, admitted);
    // Same guard as `shape_of`: an *agreement* can over-claim against a hand
    // that saw a card it did not expect, and that reads as "shows nothing"
    // rather than dividing by zero.
    let m = if m.hit > 0.0 {
        m
    } else {
        walk_hcp(unseen, u64::MAX)
    };
    let inv = 1.0 / m.hit;
    let mean = m.sum * inv;
    HcpDist {
        mean,
        sd: (m.square * inv - mean * mean).max(0.0).sqrt(),
        mass: -(m.hit * (1.0 / m.all)).ln() / m.all.ln(),
    }
}

/// Push one hidden seat's **raw-HCP endpoints** ([`LEN_HCP_ENDS`] values) — the
/// `strength.hcp` axis, `{min, max}` ÷ 37.
///
/// The axis every net has ignored.  It is written unslacked wherever `points`
/// is written slacked, so it is strictly sharper on exactly the hands the
/// notrump gates care about; the ablation's `pts-hcp-ends` arm is what prices
/// it against the [`push_hcp_gauss`] block that consumes the same axis.
///
/// Takes the [`shown`] envelope, like [`push_points`].
fn push_hcp_ends(out: &mut impl FeatureSink, shown: &Envelope) {
    out.push(shown.strength.hcp.min as f32 / 37.0);
    out.push(shown.strength.hcp.max as f32 / 37.0);
}

/// Push one hidden seat's **strength reading** ([`LEN_HCP_GAUSS`] values) — the
/// distributional twin of [`push_points`]'s two endpoints.
///
/// The shape kernel's argument, on the axis where it should bite harder.  A
/// length reading is narrow (`♠5..13`, `♥0..2`) and the prior across it is
/// flat enough that `{min, max}` is nearly sufficient — which is why the shape
/// Gaussian measured par.  A strength reading is wide (`0..37` unshown,
/// `11..26` after a 2/1) and the prior across it is sharply peaked, so the
/// endpoints discard much more: the truncated mean of `11..26` sits nowhere
/// near its midpoint, and where it does sit depends on how many honours this
/// hand is holding.
///
/// Conditions on the strength axes only, and on one seat at a time,
/// marginalising over the other two hidden hands exactly as the endpoints it
/// replaces do.
fn push_hcp_gauss(out: &mut impl FeatureSink, dist: &HcpDist) {
    out.push((dist.mean / 37.0) as f32);
    out.push((dist.sd / HCP_SPREAD_SCALE) as f32);
    out.push(dist.mass as f32);
}

/// Push a 7-value bid encoding: [present, level/7, strain one-hot ×5]
fn push_bid_encoding(out: &mut impl FeatureSink, bid: Option<contract_bridge::Bid>) {
    match bid {
        None => {
            out.push(0.0); // present
            out.push(0.0); // level/7
            for _ in Strain::ASC {
                out.push(0.0);
            }
        }
        Some(b) => {
            out.push(1.0); // present
            out.push(b.level.get() as f32 / 7.0);
            for strain in Strain::ASC {
                out.push(f32::from(b.strain == strain));
            }
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Push the auction-context, inferences, and vulnerability blocks (36 + 40 + 2
/// = 78 values) — the disclosable, hand-shape-independent tail of
/// [`features_v3`].
///
/// Everything here is derivable from the *public* auction and the partnership's
/// disclosed agreements (the [`Inferences`] ranges), so it stays in the
/// restrictive v3 vector unchanged.
fn push_context(out: &mut impl FeatureSink, context: &Context<'_>) {
    // ── Context (36 values) ─────────────────────────────────────────────────

    // our_strains: 5 bits
    for strain in Strain::ASC {
        out.push(f32::from(context.we_bid(strain)));
    }

    // their_strains: 5 bits
    for strain in Strain::ASC {
        out.push(f32::from(context.they_bid(strain)));
    }

    // contract-to-beat: 7 values
    push_bid_encoding(out, context.last_bid());

    // partner's last bid: 7 values
    push_bid_encoding(out, context.partner_last_bid());

    // penalty one-hot: 3 values [Undoubled, Doubled, Redoubled]
    let penalty = context.penalty();
    out.push(f32::from(penalty == Penalty::Undoubled));
    out.push(f32::from(penalty == Penalty::Doubled));
    out.push(f32::from(penalty == Penalty::Redoubled));

    // undisturbed, passed_hand, partner_passed_hand: 3 values
    out.push(f32::from(context.undisturbed()));
    out.push(f32::from(context.passed_hand()));
    out.push(f32::from(context.partner_passed_hand()));

    // leading_passes (capped at 3): 1 value
    out.push((context.leading_passes().min(3) as f32) / 3.0);

    // seat one-hot (4 values): index = auction.len() % 4 (seat relative to dealer)
    let seat_idx = context.auction().len() % 4;
    for i in 0..4 {
        out.push(f32::from(i == seat_idx));
    }

    // we-opened bit: 1 value
    out.push(f32::from(context.we_opened()));

    // ── Inferences (40 values) ──────────────────────────────────────────────
    let inf = context.inferences();

    for who in [
        Relative::Me,
        Relative::Lho,
        Relative::Partner,
        Relative::Rho,
    ] {
        // The *agreement*, not the sound projection: a call is explained by what
        // the partnership announces, and that is what a reasoning seat should be
        // fed.  Identical to `get` unless `set_announced_reading` is on and some
        // rule split the two — a net-decided call, whose sound projection is ⊤.
        push_inference(out, inf.announced(who));
    }

    // ── Vulnerability (2 values) ────────────────────────────────────────────
    let v = context.vul();
    out.push(f32::from(v.contains(RelativeVulnerability::WE)));
    out.push(f32::from(v.contains(RelativeVulnerability::THEY)));
}

/// Extract the **restrictive, fully disclosable** feature vector (AI-bidder v3)
///
/// Bridge ethics require full disclosure: a call is explained to opponents by
/// the partnership's *agreement*, never by the bidder's specific cards.
/// Agreements are defined over summary abstractions — so this extractor drops
/// every card-specific value (per-suit rank bits, top-honor count, stopper bit)
/// and keeps only what a bidder could disclose:
///
/// - per suit (4 × 2): `len/13`, `suit_hcp/10` (suit quality);
/// - global (2): `hcp/40`, `shape/2` where `shape = points − hcp` is the
///   crate's fuzzy distribution [`upgrade`] (0–2; the detailed shape is already
///   carried by the four suit lengths);
/// - the shared context, inferences, and vulnerability blocks (the
///   `push_context` tail) — all derived from the public auction and the
///   disclosed agreement ranges.
///
/// Seat (relative to dealer) and relative vulnerability are already inside those
/// shared blocks, so they are not repeated here.  Returns exactly
/// [`FEATURES_LEN_V3`] finite values normalised to roughly `[0.0, 1.0]`.
#[must_use]
pub fn features_v3(hand: Hand, context: &Context<'_>) -> Vec<f32> {
    let mut out = Vec::with_capacity(FEATURES_LEN_V3);

    // ── Restrictive hand block (10 values) ──────────────────────────────────
    push_hand(&mut out, hand);
    debug_assert_eq!(out.len(), LEN_HAND_V3);

    // ── Shared context / inferences / vulnerability (78 values) ─────────────
    push_context(&mut out, context);

    debug_assert_eq!(out.len(), FEATURES_LEN_V3);
    out
}

// ── The configured extractor (docs/ai-bidder/configured-net.md) ──────────────

/// Layout version tag for the configured extractor [`features_v4`]
pub const FEATURES_VERSION_V4: u32 = 4;

/// Convention rows on one side's card: `SCHEMA` (133) plus `PONS_SCHEMA` (2)
///
/// Pinned by `card_block_is_the_whole_card`, so a row added to either schema
/// fails a test rather than silently shifting every feature after it — a
/// mismatch here would misalign an artifact against its extractor with no
/// symptom other than worse bidding.
pub const LEN_CARD_ROWS: usize = 135;

/// EPBot's base systems, one-hot: 2/1 GF, SAYC, WJ, Precision, Acol
///
/// **Not decoration, and not derivable from the rows.** `Card::system` is the
/// only channel for facts no row expresses — `dutch_card` differs from
/// `american_card` by this header plus a single row, yet the header is carrying
/// the entire wide non-forcing 1♣. Encoding the rows alone would have made a WJ
/// opponent nearly indistinguishable from a 2/1 one, which is precisely the
/// blindness this whole extractor exists to remove.
pub const LEN_SYSTEM: usize = 5;

/// One side's whole declared system: its base system, then its convention rows
pub const LEN_CARD: usize = LEN_SYSTEM + LEN_CARD_ROWS;

/// Number of `f32` values returned by [`features_v4`]: every value
/// [`features_v3`] produces, then both partnerships' convention cards.
pub const FEATURES_LEN_V4: usize = FEATURES_LEN_V3 + 2 * LEN_CARD;

/// Offset of our own card block in [`features_v4`]
pub const OFFSET_OUR_CARD: usize = FEATURES_LEN_V3;

/// Offset of the opponents' card block in [`features_v4`]
pub const OFFSET_THEIR_CARD: usize = OFFSET_OUR_CARD + LEN_CARD;

/// Both partnerships' convention cards, encoded once per configuration cell
///
/// This is what makes a net *configured*: the system is an input, so one
/// artifact serves every regime and an A/B arm differs by a row of this rather
/// than by a separately trained net.  Without it, two arms differ by both the
/// convention and the weights fitted to it, and nothing can separate them —
/// `docs/ai-bidder/configured-net.md` has the measurement that motivated this.
///
/// **Both sides, because a mixed table is the normal case in an A/B.** The arms
/// play each other, so at every table one side relocates its asks and the other
/// does not; a net blind to the opposition's card is out of distribution on
/// exactly the boards the measurement is about.
///
/// Encoded once per cell and attached to a [`Context`] by reference, so the
/// per-decision path neither allocates nor consults ambient knob state.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    ours: [f32; LEN_CARD],
    theirs: [f32; LEN_CARD],
}

impl Config {
    /// Encode what each side is declared to play
    ///
    /// # Panics
    ///
    /// If either card's row count is not [`LEN_CARD`].
    #[must_use]
    pub fn new(ours: &Card, theirs: &Card) -> Self {
        Self {
            ours: encode_card(ours),
            theirs: encode_card(theirs),
        }
    }

    /// Both sides declared to play the same card
    ///
    /// The default reading everywhere else in the crate: [`Context`]'s
    /// `their_system` and `BbaOracle`'s undeclared opponents both model the
    /// opposition as playing our own system.
    #[must_use]
    pub fn symmetric(card: &Card) -> Self {
        Self::new(card, card)
    }
}

/// One card as `0.0`/`1.0`: the base system one-hot, then the rows in schema order
///
/// Every row is boolean (`american_row` returns only `0` or `1`), so the values
/// are already in the `[0.0, 1.0]` the rest of the vector uses.
///
/// A system id outside `0..LEN_SYSTEM` leaves the one-hot all-zero rather than
/// panicking: an unknown base system is genuinely "none of these five", and a
/// foreign `.bbsa` is untrusted input.
fn encode_card(card: &Card) -> [f32; LEN_CARD] {
    assert_eq!(
        card.rows.len(),
        LEN_CARD_ROWS,
        "a card carries {LEN_CARD_ROWS} rows; a schema change must move \
         `LEN_CARD_ROWS` and retrain, since every later feature shifts"
    );
    let mut out = [0.0; LEN_CARD];
    if let Ok(system) = usize::try_from(card.system)
        && system < LEN_SYSTEM
    {
        out[system] = 1.0;
    }
    for (slot, (_, value)) in out[LEN_SYSTEM..].iter_mut().zip(&card.rows) {
        *slot = if *value == 0 { 0.0 } else { 1.0 };
    }
    out
}

/// [`features_v3`] plus both partnerships' convention cards
///
/// Returns exactly [`FEATURES_LEN_V4`] finite values. The card blocks are
/// verbatim `0.0`/`1.0` rows, so the feature vector *is* the disclosure — a
/// configured net cannot learn from an agreement it would not show an opponent.
///
/// Most rows are constant within any one corpus and train to a weight of
/// roughly zero. That is deliberate: pruning to the varying rows would make an
/// artifact's meaning depend on the corpus that produced it, and would reopen
/// the width question every campaign. **A v4 net is only responsive along the
/// axes its corpus actually varied** — the generality is in the plumbing, not
/// automatically in the weights.
///
/// With no [`Config`] attached the card blocks are zero, which is why
/// [`Context::with_config`] belongs on every dump and serving path.
#[must_use]
pub fn features_v4(hand: Hand, context: &Context<'_>) -> Vec<f32> {
    let mut out = features_v3(hand, context);
    out.reserve_exact(2 * LEN_CARD);
    // ponytail: an absent config encodes as zeros, which is indistinguishable
    // from a side that genuinely plays no conventions.  Harmless while every
    // caller attaches one (the assert below catches a miss in tests); if an
    // undeclared side ever becomes a real state, give it its own flag column
    // rather than overloading the all-zero card.
    debug_assert!(
        context.config().is_some(),
        "features_v4 wants a Config attached; see Context::with_config"
    );
    match context.config() {
        Some(config) => {
            out.extend_from_slice(&config.ours);
            out.extend_from_slice(&config.theirs);
        }
        None => out.resize(FEATURES_LEN_V4, 0.0),
    }
    debug_assert_eq!(out.len(), FEATURES_LEN_V4);
    out
}

// ── The trick-evaluator extractor (bilans session C) ─────────────────────────

/// Layout version tag for the trick-evaluator extractor [`features_eval`]
///
/// Version 2 replaced the disclosable `{len, suit_hcp}` hand summary with the
/// per-suit honour decomposition — the `ben` arm of the featurization sweep.
pub const FEATURES_VERSION_EVAL: u32 = 2;

/// Length of the trick-evaluator hand block: 4 suits × `{#spots, A, K, Q, J, T}`
pub const LEN_HAND_EVAL: usize = 24;

/// Number of `f32` values returned by [`features_eval`]: the honour-granular
/// hand block ([`LEN_HAND_EVAL`]) plus the three *hidden* seats' range blocks.
pub const FEATURES_LEN_EVAL: usize = LEN_HAND_EVAL + 3 * LEN_INFERENCE;

fn push_eval_base(out: &mut impl FeatureSink, hand: Hand, inferences: &Inferences) {
    push_hand_eval(out, hand);
    for who in [Relative::Lho, Relative::Partner, Relative::Rho] {
        push_inference(out, inferences.announced(who));
    }
}

/// Extract the **trick-evaluator** feature vector: own hand plus what the
/// three hidden seats have shown.
///
/// The question this vector poses is *physics*, not system: given my cards and
/// range envelopes on the other three hands, how many double-dummy tricks does
/// each declarer take in each strain?  So it deliberately carries **no auction,
/// no seat, and no vulnerability** — the auction enters only through the
/// [`Inferences`] the book already distilled from it, and vulnerability belongs
/// to the expected-score arithmetic downstream, not to the trick count.
///
/// That omission is what makes an evaluator trained on this vector
/// *bidding-system agnostic*: corpora generated under different books describe
/// the same physics and pool into one training set.
///
/// | Block           | Start | Len |
/// |-----------------|-------|-----|
/// | Own hand        |     0 |  24 |
/// | LHO ranges      |    24 |  10 |
/// | Partner ranges  |    34 |  10 |
/// | RHO ranges      |    44 |  10 |
/// | **Total**       |       | **54** |
///
/// The own-hand block is a per-suit honour decomposition, *not*
/// [`features_v3`]'s summary: the evaluator is never disclosed to opponents, so
/// the ethics constraint that shapes the policy vector does not bind here.
/// Texture therefore survives down to honour granularity — AJx and KQx are both
/// 5 HCP in three cards and now read differently — though spot-card *identity*
/// below the ten still does not (T9x and T2x read alike), and the net absorbs
/// that residue as spread.  Get the `Inferences` from
/// [`Stance::infer`][super::Stance::infer], never from a bare [`Context`] — the
/// trie-prefixed reading is what decodes conventional calls off their authoring
/// rules.
#[must_use]
pub fn features_eval(hand: Hand, inferences: &Inferences) -> [f32; FEATURES_LEN_EVAL] {
    let mut out = FixedFeatures::new();
    push_eval_base(&mut out, hand, inferences);
    out.finish()
}

// ── The trick-evaluator extractor, v3: + the raw call tail ────────────────────

/// Calls of auction history in the [`features_eval_v3`] tail, most recent
/// first.  Four matches the window the NLL ablation measured; the hulls carry
/// the older history in compressed form.
pub const CALLS_EVAL_V3: usize = 4;

/// Width of one call-identity slot: the 7-value bid encoding plus one bit each
/// for pass, double, and redouble.
pub const LEN_CALL_EVAL_V3: usize = 10;

/// Number of `f32` values returned by [`features_eval_v3`]
pub const FEATURES_LEN_EVAL_V3: usize = FEATURES_LEN_EVAL + CALLS_EVAL_V3 * LEN_CALL_EVAL_V3;

/// Push one call-identity slot ([`LEN_CALL_EVAL_V3`] values).  `None` — the
/// auction is shorter than the window — is all zeros, distinguishable from
/// every real call because a real call sets `present` or a call-kind bit.
fn push_call_identity(out: &mut impl FeatureSink, call: Option<Call>) {
    push_bid_encoding(
        out,
        match call {
            Some(Call::Bid(bid)) => Some(bid),
            _ => None,
        },
    );
    out.push(f32::from(call == Some(Call::Pass)));
    out.push(f32::from(call == Some(Call::Double)));
    out.push(f32::from(call == Some(Call::Redouble)));
}

/// [`features_eval`] plus the identities of the last [`CALLS_EVAL_V3`] calls,
/// most recent first.
///
/// This overturns v2's "no auction, ever" commitment, and it was overturned by
/// measurement: the 2026-07-27 NLL ablation priced the raw call tail at
/// **0.042 NLL / 0.053 tricks of MAE** over the hull-only vector — the
/// largest featurization delta on record, concentrated exactly where the
/// ⊤-census says the readings starve (contested, slam) — and bare call
/// identity carries 90% of it (tags and alerts, the rest of the measured
/// block, are *not* included: +0.004 was not worth coupling the vector to how
/// rules are authored).  See docs/ai-bidder/evaluator-net.md §auction-input
/// ablation, including what the win costs: a v3 corpus is coupled to the
/// bidding system that generated it, so corpora only pool across systems whose
/// auctions were dumped alongside, and every routing change owes the twin
/// protocol.
///
/// | Block             | Start | Len |
/// |-------------------|-------|-----|
/// | [`features_eval`] |     0 |  54 |
/// | Call −1 (latest)  |    54 |  10 |
/// | Call −2           |    64 |  10 |
/// | Call −3           |    74 |  10 |
/// | Call −4           |    84 |  10 |
/// | **Total**         |       | **94** |
#[must_use]
pub fn features_eval_v3(
    hand: Hand,
    inferences: &Inferences,
    auction: &[Call],
) -> [f32; FEATURES_LEN_EVAL_V3] {
    let mut out = FixedFeatures::new();
    push_eval_base(&mut out, hand, inferences);
    for age in 1..=CALLS_EVAL_V3 {
        let call = auction.len().checked_sub(age).map(|j| auction[j]);
        push_call_identity(&mut out, call);
    }
    out.finish()
}

// ── The shape-reading evaluator vector (v4) ───────────────────────────────────

/// Feature version tag of [`features_eval_v4`]
pub const FEATURES_VERSION_EVAL_V4: u32 = 4;

/// Width of one hidden seat's block in [`features_eval_v4`]: the two `points`
/// endpoints, then the [`LEN_SHAPE_GAUSS`] shape reading that replaces the eight
/// length endpoints.
pub const LEN_SEAT_V4: usize = LEN_POINTS + LEN_SHAPE_GAUSS;

/// Number of `f32` values returned by [`features_eval_v4`]
pub const FEATURES_LEN_EVAL_V4: usize =
    LEN_HAND_EVAL + 3 * LEN_SEAT_V4 + CALLS_EVAL_V3 * LEN_CALL_EVAL_V3;

/// [`features_eval_v3`] with each hidden seat's **length hull replaced by its
/// shape distribution** — the shipped shape-reading vector.
///
/// Three columns wider than v3 and, on the round-two ablation, exactly as good:
/// `gauss-mass` scored −1.54562 against the 94-column control's −1.54558 on
/// 8.15M rows, inside a 0.0006 seed spread.  The NLL case for this vector is
/// *nil*, and that is the expected result — MASS already showed the fitted
/// `(μ, σ)` at a hull row is the union-conditional the net learned empirically,
/// so handing it the same information in a different parameterization cannot
/// pay.  What is bought is **invariance**: under `set_sum_closure`, a
/// provably-rejection-free tightening, the endpoint columns move at 81.17% of
/// nodes by up to 4.19σ while these columns move at 0.11% by up to 0.07σ — and
/// that 0.11% is where the reading genuinely changed.  Every future
/// hull-tightening chop can then be judged on merit instead of buying a retrain.
///
/// Two findings from the same sweep are load-bearing on the layout.  The
/// log-mass column is worth +0.0007 *beside* a length reading but −0.023 as a
/// substitute for one, so it is a modifier, not a replacement.  And the
/// endpoints are **spent**: given this block, re-adding all eight of them moved
/// the NLL by 0.00001, which inverts the MARG/MASS verdict — those campaigns
/// measured *estimated* marginals beside the endpoints, and against exact ones
/// it is the endpoints that are redundant.
///
/// | Block                       | Start | Len |
/// |-----------------------------|-------|-----|
/// | Own hand                    |     0 |  24 |
/// | LHO points + shape          |    24 |  11 |
/// | Partner points + shape      |    35 |  11 |
/// | RHO points + shape          |    46 |  11 |
/// | Calls −1 … −4               |    57 |  40 |
/// | **Total**                   |       | **97** |
#[must_use]
pub fn features_eval_v4(
    hand: Hand,
    inferences: &Inferences,
    auction: &[Call],
) -> [f32; FEATURES_LEN_EVAL_V4] {
    let mut out = FixedFeatures::new();
    push_hand_eval(&mut out, hand);
    let unseen = Unseen::new(hand);
    for who in [Relative::Lho, Relative::Partner, Relative::Rho] {
        push_points(&mut out, shown(inferences.announced(who)));
        let boxes = shown_boxes(inferences.announced_union(who));
        push_shape_gauss(&mut out, &shape_of(&unseen, boxes));
    }
    for age in 1..=CALLS_EVAL_V3 {
        let call = auction.len().checked_sub(age).map(|j| auction[j]);
        push_call_identity(&mut out, call);
    }
    out.finish()
}

// ── The shape-distribution research superset ──────────────────────────────────

/// Width of one hidden seat's block in [`features_eval_shape`]: the hull
/// [`LEN_INFERENCE`] hull endpoints, then the [`LEN_SHAPE`] shape distribution.
pub const LEN_SEAT_SHAPE: usize = LEN_INFERENCE + LEN_SHAPE;

/// Number of `f32` values returned by [`features_eval_shape`]
pub const FEATURES_LEN_EVAL_SHAPE: usize =
    LEN_HAND_EVAL + 3 * LEN_SEAT_SHAPE + CALLS_EVAL_V3 * LEN_CALL_EVAL_V3;

/// The **research superset** behind the shape-reading ablation: everything
/// [`features_eval_v3`] carries, plus each hidden seat's shape distribution.
///
/// Dumped once; the trainer's `--arm` masks the columns each arm does not want,
/// so every arm sees the same rows in the same batch order and only differences
/// *within* one sweep mean anything.  Keeping the endpoints alongside is what
/// lets the control arm reproduce the shipped vector out of the same corpus —
/// and what lets a hybrid arm re-test the MARG finding, that distributional
/// columns *beside* the endpoints buy nothing.  The shipped encoding will be a
/// **replacement**: retaining the endpoints retains their non-invariance.
///
/// | Block                          | Start | Len |
/// |--------------------------------|-------|-----|
/// | Own hand                       |     0 |  24 |
/// | LHO endpoints + shape          |    24 |  75 |
/// | Partner endpoints + shape      |    99 |  75 |
/// | RHO endpoints + shape          |   174 |  75 |
/// | Calls −1 … −4                  |   249 |  40 |
/// | **Total**                      |       | **289** |
#[must_use]
pub fn features_eval_shape(
    hand: Hand,
    inferences: &Inferences,
    auction: &[Call],
) -> [f32; FEATURES_LEN_EVAL_SHAPE] {
    let mut out = FixedFeatures::new();
    push_hand_eval(&mut out, hand);
    let unseen = Unseen::new(hand);
    for who in [Relative::Lho, Relative::Partner, Relative::Rho] {
        push_inference(&mut out, inferences.announced(who));
        let boxes = shown_boxes(inferences.announced_union(who));
        push_shape_dist(&mut out, &shape_of(&unseen, boxes));
    }
    for age in 1..=CALLS_EVAL_V3 {
        let call = auction.len().checked_sub(age).map(|j| auction[j]);
        push_call_identity(&mut out, call);
    }
    out.finish()
}

// ── The strength-reading research superset ────────────────────────────────────

/// Width of one hidden seat's block in [`features_eval_points`]: the hull
/// [`LEN_INFERENCE`] endpoints, the shipped [`LEN_SHAPE_GAUSS`] shape reading,
/// then the two new strength blocks.
pub const LEN_SEAT_POINTS: usize = LEN_INFERENCE + LEN_SHAPE_GAUSS + LEN_HCP_ENDS + LEN_HCP_GAUSS;

/// Number of `f32` values returned by [`features_eval_points`]
pub const FEATURES_LEN_EVAL_POINTS: usize =
    LEN_HAND_EVAL + 3 * LEN_SEAT_POINTS + CALLS_EVAL_V3 * LEN_CALL_EVAL_V3;

/// The **research superset** behind the strength-reading ablation: everything
/// [`features_eval_v4`] carries, plus each hidden seat's raw-HCP endpoints and
/// its strength distribution.
///
/// Dumped once; the trainer's `--arm` masks the columns each arm does not want,
/// so every arm sees the same rows in the same batch order and only differences
/// *within* one sweep mean anything.  Carrying the shipped blocks alongside is
/// what lets the control arm reproduce [`features_eval_v4`] out of this corpus,
/// and carrying the `points` endpoints beside the strength distribution is what
/// re-tests the MARG finding one axis over — the shape sweep found the length
/// endpoints **spent** against exact moments (re-adding all eight moved the NLL
/// by 0.00001), and whether that transfers to strength is the question.
///
/// Two blocks are new, and they answer different questions.  The strength
/// distribution is the re-parameterization, whose honest prior is *par*:
/// truncated moments are a deterministic function of the endpoints and this
/// hand's honours, all of which the net already has.  The raw-HCP endpoints are
/// what carries information no net has seen — the crisp `strength.hcp` axis,
/// written unslacked wherever `points` is slacked.
///
/// | Block                                   | Start | Len |
/// |-----------------------------------------|-------|-----|
/// | Own hand                                |     0 |  24 |
/// | LHO endpoints + shape + strength        |    24 |  24 |
/// | Partner endpoints + shape + strength    |    48 |  24 |
/// | RHO endpoints + shape + strength        |    72 |  24 |
/// | Calls −1 … −4                           |    96 |  40 |
/// | **Total**                               |       | **136** |
///
/// One seat's 24, in order: 8 length endpoints, 2 `points` endpoints, 8 shape
/// moments, 1 shape mass, 2 `hcp` endpoints, 2 strength moments, 1 strength
/// mass.
#[must_use]
pub fn features_eval_points(
    hand: Hand,
    inferences: &Inferences,
    auction: &[Call],
) -> [f32; FEATURES_LEN_EVAL_POINTS] {
    let mut out = FixedFeatures::new();
    push_hand_eval(&mut out, hand);
    let unseen = Unseen::new(hand);
    let honours = UnseenHonours::new(hand);
    for who in [Relative::Lho, Relative::Partner, Relative::Rho] {
        push_inference(&mut out, inferences.announced(who));
        let boxes = shown_boxes(inferences.announced_union(who));
        push_shape_gauss(&mut out, &shape_of(&unseen, boxes));
        push_hcp_ends(&mut out, shown(inferences.announced(who)));
        push_hcp_gauss(&mut out, &hcp_of(&honours, boxes));
    }
    for age in 1..=CALLS_EVAL_V3 {
        let call = auction.len().checked_sub(age).map(|j| auction[j]);
        push_call_identity(&mut out, call);
    }
    out.finish()
}

#[cfg(test)]
mod tests;
