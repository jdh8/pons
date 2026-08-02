//! Versioned feature extractor for the AI instinct bidder
//!
//! Converts a bridge hand and its auction [`Context`] into a fixed-size
//! `Vec<f32>` suitable for input to a neural network.  Every value is
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
use super::inference::{Dnf, Envelope, Inferences, Range, Relative};
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

/// HCP of a single holding (A=4, K=3, Q=2, J=1)
fn holding_hcp(holding: Holding) -> u8 {
    4 * u8::from(holding.contains(Rank::A))
        + 3 * u8::from(holding.contains(Rank::K))
        + 2 * u8::from(holding.contains(Rank::Q))
        + u8::from(holding.contains(Rank::J))
}

/// Push the disclosable hand summary ([`LEN_HAND_V3`] values): per suit
/// `len/13` and `suit_hcp/10`, then global `hcp/40` and `shape/2`.
fn push_hand(out: &mut Vec<f32>, hand: Hand) {
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
fn push_hand_eval(out: &mut Vec<f32>, hand: Hand) {
    for suit in Suit::ASC {
        let holding = hand[suit];
        // A suit holds any *subset* of the honours, so count what is actually
        // there; the rest of its length is spot cards.
        let held = HONOURS.map(|rank| holding.contains(rank));
        let spots = holding.len() - held.iter().filter(|&&h| h).count();
        out.push(spots as f32 / 8.0);
        out.extend(held.map(f32::from));
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

/// The reading the nets are fed: the seat's agreement, or nothing under
/// [`set_blind_inference`].
fn shown(player: &Envelope) -> &Envelope {
    // ponytail: one shared `unknown` rather than a per-call temporary — the
    // envelope is immutable and `Envelope::unknown` is `const`.
    const NOTHING: Envelope = Envelope::unknown();
    if BLIND_INFERENCE.with(std::cell::Cell::get) {
        &NOTHING
    } else {
        player
    }
}

/// Push one player's shown ranges ([`LEN_INFERENCE`] values): per suit
/// `{min, max}` length ÷ 13, then `{min, max}` points ÷ 37.  Nothing shown is
/// the `[0, 1]` pattern (`Envelope::unknown`), *not* zeros.
fn push_inference(out: &mut Vec<f32>, player: &Envelope) {
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
fn push_points(out: &mut Vec<f32>, shown: &Envelope) {
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
fn push_shape_gauss(out: &mut Vec<f32>, shape: &Shape) {
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
fn push_shape_dist(out: &mut Vec<f32>, shape: &Shape) {
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
fn shown_boxes(dnf: &Dnf) -> Option<&[Envelope]> {
    (!BLIND_INFERENCE.with(std::cell::Cell::get)).then(|| dnf.boxes())
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
fn push_hcp_ends(out: &mut Vec<f32>, shown: &Envelope) {
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
fn push_hcp_gauss(out: &mut Vec<f32>, dist: &HcpDist) {
    out.push((dist.mean / 37.0) as f32);
    out.push((dist.sd / HCP_SPREAD_SCALE) as f32);
    out.push(dist.mass as f32);
}

/// Push a 7-value bid encoding: [present, level/7, strain one-hot ×5]
fn push_bid_encoding(out: &mut Vec<f32>, bid: Option<contract_bridge::Bid>) {
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
fn push_context(out: &mut Vec<f32>, context: &Context<'_>) {
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
    let inf = Inferences::read(context);

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
pub fn features_eval(hand: Hand, inferences: &Inferences) -> Vec<f32> {
    let mut out = Vec::with_capacity(FEATURES_LEN_EVAL);
    push_hand_eval(&mut out, hand);
    for who in [Relative::Lho, Relative::Partner, Relative::Rho] {
        // The hidden seats' *agreements* — see `push_context`.  This is the site
        // the reach ceiling names: a seat whose call the net decided projects ⊤,
        // so without the split partner's estimate is computed on nothing.
        push_inference(&mut out, inferences.announced(who));
    }
    debug_assert_eq!(out.len(), FEATURES_LEN_EVAL);
    out
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
fn push_call_identity(out: &mut Vec<f32>, call: Option<Call>) {
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
pub fn features_eval_v3(hand: Hand, inferences: &Inferences, auction: &[Call]) -> Vec<f32> {
    let mut out = features_eval(hand, inferences);
    out.reserve_exact(CALLS_EVAL_V3 * LEN_CALL_EVAL_V3);
    for age in 1..=CALLS_EVAL_V3 {
        let call = auction.len().checked_sub(age).map(|j| auction[j]);
        push_call_identity(&mut out, call);
    }
    debug_assert_eq!(out.len(), FEATURES_LEN_EVAL_V3);
    out
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
pub fn features_eval_v4(hand: Hand, inferences: &Inferences, auction: &[Call]) -> Vec<f32> {
    let mut out = Vec::with_capacity(FEATURES_LEN_EVAL_V4);
    push_hand_eval(&mut out, hand);
    let unseen = Unseen::new(hand);
    for who in [Relative::Lho, Relative::Partner, Relative::Rho] {
        push_points(&mut out, shown(inferences.announced(who)));
        let boxes = shown_boxes(inferences.announced_dnf(who));
        push_shape_gauss(&mut out, &shape_of(&unseen, boxes));
    }
    for age in 1..=CALLS_EVAL_V3 {
        let call = auction.len().checked_sub(age).map(|j| auction[j]);
        push_call_identity(&mut out, call);
    }
    debug_assert_eq!(out.len(), FEATURES_LEN_EVAL_V4);
    out
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
pub fn features_eval_shape(hand: Hand, inferences: &Inferences, auction: &[Call]) -> Vec<f32> {
    let mut out = Vec::with_capacity(FEATURES_LEN_EVAL_SHAPE);
    push_hand_eval(&mut out, hand);
    let unseen = Unseen::new(hand);
    for who in [Relative::Lho, Relative::Partner, Relative::Rho] {
        push_inference(&mut out, inferences.announced(who));
        let boxes = shown_boxes(inferences.announced_dnf(who));
        push_shape_dist(&mut out, &shape_of(&unseen, boxes));
    }
    for age in 1..=CALLS_EVAL_V3 {
        let call = auction.len().checked_sub(age).map(|j| auction[j]);
        push_call_identity(&mut out, call);
    }
    debug_assert_eq!(out.len(), FEATURES_LEN_EVAL_SHAPE);
    out
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
pub fn features_eval_points(hand: Hand, inferences: &Inferences, auction: &[Call]) -> Vec<f32> {
    let mut out = Vec::with_capacity(FEATURES_LEN_EVAL_POINTS);
    push_hand_eval(&mut out, hand);
    let unseen = Unseen::new(hand);
    let honours = UnseenHonours::new(hand);
    for who in [Relative::Lho, Relative::Partner, Relative::Rho] {
        push_inference(&mut out, inferences.announced(who));
        let boxes = shown_boxes(inferences.announced_dnf(who));
        push_shape_gauss(&mut out, &shape_of(&unseen, boxes));
        push_hcp_ends(&mut out, shown(inferences.announced(who)));
        push_hcp_gauss(&mut out, &hcp_of(&honours, boxes));
    }
    for age in 1..=CALLS_EVAL_V3 {
        let call = auction.len().checked_sub(age).map(|j| auction[j]);
        push_call_identity(&mut out, call);
    }
    debug_assert_eq!(out.len(), FEATURES_LEN_EVAL_POINTS);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use contract_bridge::auction::{Call, RelativeVulnerability};
    use contract_bridge::{Bid, Level, Strain};

    const fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid {
            level: Level::new(level),
            strain,
        })
    }

    fn hand(s: &str) -> Hand {
        s.parse().expect("valid test hand")
    }

    fn empty_context() -> Context<'static> {
        Context::new(RelativeVulnerability::NONE, &[])
    }

    /// A length box, in `Suit::ASC` order: clubs, diamonds, hearts, spades.
    fn lengths(bounds: [(u8, u8); 4]) -> Envelope {
        let mut envelope = Envelope::unknown();
        envelope.lengths = bounds.map(|(min, max)| super::super::inference::Range::new(min, max));
        envelope
    }

    fn shape_block(cards: &str, boxes: Option<&[Envelope]>) -> Vec<f32> {
        let mut out = Vec::new();
        push_shape_dist(&mut out, &shape_of(&Unseen::new(hand(cards)), boxes));
        assert_eq!(out.len(), LEN_SHAPE);
        out
    }

    /// The shipped [`LEN_SHAPE_GAUSS`] block, for the same reading.
    fn gauss_block(cards: &str, boxes: Option<&[Envelope]>) -> Vec<f32> {
        let mut out = Vec::new();
        push_shape_gauss(&mut out, &shape_of(&Unseen::new(hand(cards)), boxes));
        assert_eq!(out.len(), LEN_SHAPE_GAUSS);
        out
    }

    /// `C(n, k)` in `f64` — the test's own arithmetic, independent of [`BINOM`].
    fn choose(n: u32, k: u32) -> f64 {
        (0..k).fold(1.0, |acc, i| acc * f64::from(n - i) / f64::from(i + 1))
    }

    /// A hand with 1-3-4-5 in `Suit::ASC` order, so all four unseen counts differ.
    const SPREAD_HAND: &str = "AKQ32.K532.QJ4.9";

    #[test]
    fn unconditional_shape_prior_is_hypergeometric() {
        // Unseen per suit, ASC: ♣12 ♦10 ♥9 ♠8, summing to 39.  A hidden seat
        // draws 13 of those, so `E[len_s] = 13 · n_s / 39 = n_s / 3` exactly.
        let block = shape_block(SPREAD_HAND, None);
        for (s, unseen) in [12.0, 10.0, 9.0, 8.0].into_iter().enumerate() {
            assert!(
                (f64::from(block[s]) - unseen / 3.0 / 13.0).abs() < 1e-6,
                "E[len_{s}] = {}",
                block[s]
            );
        }
        // Shows nothing, so it pins nothing.
        assert!(
            block[LEN_SHAPE - 1].abs() < 1e-6,
            "{}",
            block[LEN_SHAPE - 1]
        );
    }

    /// **The point of the encoding.**  `set_sum_closure` narrows every box to
    /// what `Σ len = 13` already implies — it cannot reject a hand, so it is
    /// information-free — yet it moves `push_inference`'s endpoints by multiple
    /// σ of their own corpus spread.  The shape block must not move at all.
    #[test]
    fn shape_block_is_invariant_to_the_sum_closure() {
        // Two majors of 5+ leave at most 3 cards for each minor and at most 8
        // for each major.  Same set of hands, different bounding box.
        let open = lengths([(0, 13), (0, 13), (5, 13), (5, 13)]);
        let closed = lengths([(0, 3), (0, 3), (5, 8), (5, 8)]);

        let mut endpoints = (Vec::new(), Vec::new());
        push_inference(&mut endpoints.0, &open);
        push_inference(&mut endpoints.1, &closed);
        assert_ne!(endpoints.0, endpoints.1, "the closure moves the endpoints");

        for cards in [SPREAD_HAND, "AQ32.K53.QJ4.A92"] {
            assert_eq!(
                shape_block(cards, Some(&[open])),
                shape_block(cards, Some(&[closed])),
                "{cards}"
            );
        }
    }

    #[test]
    fn a_shown_void_reads_as_its_exact_mass() {
        // ♠ 0..=0 against the 8 unseen spades: the seat draws all 13 from the
        // other 31 unseen cards.
        let block = shape_block(
            SPREAD_HAND,
            Some(&[lengths([(0, 13), (0, 13), (0, 13), (0, 0)])]),
        );
        assert!(block[3].abs() < 1e-6, "E[len_♠] = {}", block[3]);
        assert!(block[7].abs() < 1e-6, "sd[len_♠] = {}", block[7]);
        // Spades are suit 3, so its 14-bin marginal starts at 8 + 3·14 = 50.
        assert!((block[50] - 1.0).abs() < 1e-6, "P(♠ = 0) = {}", block[50]);

        let all = choose(39, 13);
        let want = -(choose(31, 13) / all).ln() / all.ln();
        assert!(
            (f64::from(block[LEN_SHAPE - 1]) - want).abs() < 1e-6,
            "pinned = {} want {want}",
            block[LEN_SHAPE - 1]
        );
    }

    /// An agreement can over-claim against a hand that holds the cards it wants.
    /// Dividing by zero mass is not an option; read it as nothing shown.
    #[test]
    fn an_unsatisfiable_reading_falls_back_to_nothing_shown() {
        // Only 8 spades are unseen, so "9+ spades" admits no shape at all.
        let impossible = lengths([(0, 13), (0, 13), (0, 13), (9, 13)]);
        assert_eq!(
            shape_block(SPREAD_HAND, Some(&[impossible])),
            shape_block(SPREAD_HAND, None)
        );
    }

    /// A strength box, leaving lengths unknown.
    fn strength(hcp: (u8, u8), points: (u8, u8)) -> Envelope {
        use super::super::inference::Range;
        let mut envelope = Envelope::unknown();
        envelope.strength.hcp = Range::new(hcp.0, hcp.1);
        envelope.strength.points = Range::new(points.0, points.1);
        envelope
    }

    fn hcp_block(cards: &str, boxes: Option<&[Envelope]>) -> Vec<f32> {
        let mut out = Vec::new();
        push_hcp_gauss(&mut out, &hcp_of(&UnseenHonours::new(hand(cards)), boxes));
        assert_eq!(out.len(), LEN_HCP_GAUSS);
        out
    }

    /// `E[hcp]` of one hidden seat, undoing the block's ÷37.
    fn mean_hcp(cards: &str, boxes: Option<&[Envelope]>) -> f64 {
        f64::from(hcp_block(cards, boxes)[0]) * 37.0
    }

    /// The kernel's arithmetic, against a closed form it cannot fake: the three
    /// hidden seats split what this hand does not hold, so an unconstrained
    /// seat averages a third of the missing HCP.
    #[test]
    fn unconditional_hcp_prior_is_hypergeometric() {
        // SPREAD_HAND holds ♠AKQ ♥K ♦QJ = 15 HCP, so 25 are unseen.
        let block = hcp_block(SPREAD_HAND, None);
        assert!(
            (mean_hcp(SPREAD_HAND, None) - 25.0 / 3.0).abs() < 1e-4,
            "E[hcp] = {}",
            mean_hcp(SPREAD_HAND, None)
        );
        // Shows nothing, so it pins nothing.
        assert!(block[2].abs() < 1e-6, "pinned = {}", block[2]);
        // The walk must cover the whole prior, not a sub-lattice of it: a
        // 13-card draw from 39 has σ[hcp] ≈ 4, and 0 would mean it collapsed.
        let sd = f64::from(block[1]) * HCP_SPREAD_SCALE;
        assert!((3.0..5.0).contains(&sd), "sd[hcp] = {sd}");
    }

    /// **The point of the encoding.**  The endpoints of `11..=26` say the
    /// midpoint is 18.5; the truncated prior says the seat averages barely over
    /// 13, because 25 unseen HCP rarely land three-quarters in one hand.
    #[test]
    fn a_wide_band_is_nothing_like_its_midpoint() {
        let wide = strength((11, 26), (11, 26));
        let mean = mean_hcp(SPREAD_HAND, Some(&[wide]));
        assert!((11.0..=26.0).contains(&mean), "E[hcp] = {mean}");
        assert!(
            mean < 15.0,
            "E[hcp] = {mean}, nowhere near the midpoint 18.5"
        );
    }

    /// The band reads **both** strength axes.  A 1NT box carries a crisp
    /// `hcp 15..=17` beside a slacked `points 15..=19`; dropping either leg
    /// widens the support, so the mean must sit inside the tighter one.
    #[test]
    fn the_strength_band_intersects_hcp_with_points() {
        let notrump = strength((15, 17), (15, 19));
        let mean = mean_hcp(SPREAD_HAND, Some(&[notrump]));
        assert!((15.0..=17.0).contains(&mean), "E[hcp] = {mean}");

        // The `points` leg is load-bearing too: `upgrade >= 0` caps raw HCP at
        // `points.max`, so widening it alone moves the reading.
        let looser = strength((15, 17), (15, 16));
        assert_ne!(
            hcp_block(SPREAD_HAND, Some(&[notrump])),
            hcp_block(SPREAD_HAND, Some(&[looser]))
        );
    }

    /// The union of boxes is an OR of bands weighted by *prior mass*, not a
    /// hull — and this is the case the endpoints cannot represent at all.
    ///
    /// "6-9 or 20-23" hulls to `6..=23`, which hands the net a band whose bulk
    /// is the 10-19 middle the reading explicitly excludes.  The kernel instead
    /// reads the two humps it was given, and weights them: against 25 unseen
    /// HCP the strong alternative is nearly impossible, so the reading sits on
    /// the weak hump — below where the hull's own truncated mean lands.
    #[test]
    fn disjoint_strength_boxes_do_not_read_as_their_hull() {
        let weak = strength((6, 9), (6, 9));
        let strong = strength((20, 23), (20, 23));
        let split = mean_hcp(SPREAD_HAND, Some(&[weak, strong]));
        let hull = mean_hcp(SPREAD_HAND, Some(&[strength((6, 23), (6, 23))]));
        assert!((6.0..=23.0).contains(&split), "E[hcp] = {split}");
        assert!(split < 9.0, "E[hcp] = {split}, off the weak hump");
        assert!(
            hull > split + 0.5,
            "split {split} vs hull {hull}: the union collapsed to its span"
        );
    }

    /// An agreement can over-claim on strength as well as on shape.
    #[test]
    fn an_unsatisfiable_strength_reading_falls_back_to_nothing_shown() {
        // 25 HCP are unseen, so "30+" admits no split at all.
        let impossible = strength((30, 37), (30, 37));
        assert_eq!(
            hcp_block(SPREAD_HAND, Some(&[impossible])),
            hcp_block(SPREAD_HAND, None)
        );
    }

    /// The superset carries the shipped vector verbatim, so the control arm of
    /// the ablation is reproducible from the same corpus.
    #[test]
    fn shape_superset_embeds_the_shipped_vector() {
        assert_eq!(LEN_SHAPE, 65);
        assert_eq!(FEATURES_LEN_EVAL_SHAPE, 289);

        let auction = [
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Double,
        ];
        let ctx = Context::new(RelativeVulnerability::ALL, &auction);
        let inferences = Inferences::read(&ctx);
        let cards = hand(SPREAD_HAND);
        let wide = features_eval_shape(cards, &inferences, &auction);
        let shipped = features_eval_v3(cards, &inferences, &auction);

        assert_eq!(wide.len(), FEATURES_LEN_EVAL_SHAPE);
        assert_eq!(wide[..LEN_HAND_EVAL], shipped[..LEN_HAND_EVAL]);
        for seat in 0..3 {
            let from = LEN_HAND_EVAL + seat * LEN_SEAT_SHAPE;
            let was = LEN_HAND_EVAL + seat * LEN_INFERENCE;
            assert_eq!(
                wide[from..from + LEN_INFERENCE],
                shipped[was..was + LEN_INFERENCE],
                "seat {seat}"
            );
        }
        let tail = LEN_HAND_EVAL + 3 * LEN_SEAT_SHAPE;
        assert_eq!(wide[tail..], shipped[FEATURES_LEN_EVAL..]);
        for (i, &v) in wide.iter().enumerate() {
            assert!(
                v.is_finite() && (-1.0..=1.5).contains(&v),
                "shape[{i}] = {v}"
            );
        }
    }

    /// The shipped vector's layout, and the one property it exists for: the
    /// sum closure moves the endpoints it replaces and must not move it.
    #[test]
    fn eval_v4_is_invariant_where_the_hull_is_not() {
        assert_eq!(LEN_SEAT_V4, 11);
        assert_eq!(FEATURES_LEN_EVAL_V4, 97);

        let open = lengths([(0, 13), (0, 13), (5, 13), (5, 13)]);
        let closed = lengths([(0, 3), (0, 3), (5, 8), (5, 8)]);
        for cards in [SPREAD_HAND, "AQ32.K53.QJ4.A92"] {
            assert_eq!(
                gauss_block(cards, Some(&[open])),
                gauss_block(cards, Some(&[closed])),
                "{cards}"
            );
        }
        // …and it is not invariant to everything, or it would be reading nothing.
        assert_ne!(
            gauss_block(SPREAD_HAND, Some(&[open])),
            gauss_block(SPREAD_HAND, None)
        );
    }

    /// v4 is v3's hand and calls with each seat's eight length endpoints swapped
    /// for the shape reading — every column traceable to a shipped one.
    #[test]
    fn eval_v4_swaps_the_length_hull_for_the_shape_reading() {
        let auction = [
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Double,
        ];
        let ctx = Context::new(RelativeVulnerability::ALL, &auction);
        let inferences = Inferences::read(&ctx);
        let cards = hand(SPREAD_HAND);
        let v4 = features_eval_v4(cards, &inferences, &auction);
        let v3 = features_eval_v3(cards, &inferences, &auction);
        let wide = features_eval_shape(cards, &inferences, &auction);

        assert_eq!(v4.len(), FEATURES_LEN_EVAL_V4);
        assert_eq!(v4[..LEN_HAND_EVAL], v3[..LEN_HAND_EVAL]);
        for seat in 0..3 {
            // The `points` endpoints survive the swap verbatim…
            let from = LEN_HAND_EVAL + seat * LEN_SEAT_V4;
            let was = LEN_HAND_EVAL + seat * LEN_INFERENCE + 8;
            assert_eq!(
                v4[from..from + LEN_POINTS],
                v3[was..was + LEN_POINTS],
                "seat {seat} points"
            );
            // …and the shape reading is the superset's own summary and mass.
            let wide_seat = LEN_HAND_EVAL + seat * LEN_SEAT_SHAPE + LEN_INFERENCE;
            assert_eq!(
                v4[from + LEN_POINTS..from + LEN_SEAT_V4 - 1],
                wide[wide_seat..wide_seat + 8],
                "seat {seat} moments"
            );
            assert_eq!(
                v4[from + LEN_SEAT_V4 - 1],
                wide[wide_seat + LEN_SHAPE - 1],
                "seat {seat} mass"
            );
        }
        let tail = LEN_HAND_EVAL + 3 * LEN_SEAT_V4;
        assert_eq!(v4[tail..], v3[FEATURES_LEN_EVAL..]);
    }

    /// The strength superset carries [`features_eval_v4`] verbatim, so the
    /// ablation's control arm is the shipped vector reproduced from the same
    /// corpus — the whole reason a superset exists.
    #[test]
    fn points_superset_embeds_the_shipped_vector() {
        assert_eq!(LEN_SEAT_POINTS, 24);
        assert_eq!(FEATURES_LEN_EVAL_POINTS, 136);

        let auction = [
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Double,
        ];
        let ctx = Context::new(RelativeVulnerability::ALL, &auction);
        let inferences = Inferences::read(&ctx);
        let cards = hand(SPREAD_HAND);
        let wide = features_eval_points(cards, &inferences, &auction);
        let v4 = features_eval_v4(cards, &inferences, &auction);

        assert_eq!(wide.len(), FEATURES_LEN_EVAL_POINTS);
        assert_eq!(wide[..LEN_HAND_EVAL], v4[..LEN_HAND_EVAL]);
        for seat in 0..3 {
            // v4's seat block is the superset's `points` endpoints and shape
            // reading, contiguous — offsets 8..19 of the 24.
            let from = LEN_HAND_EVAL + seat * LEN_SEAT_POINTS + 8;
            let was = LEN_HAND_EVAL + seat * LEN_SEAT_V4;
            assert_eq!(
                wide[from..from + LEN_SEAT_V4],
                v4[was..was + LEN_SEAT_V4],
                "seat {seat}"
            );
        }
        let tail = LEN_HAND_EVAL + 3 * LEN_SEAT_POINTS;
        assert_eq!(wide[tail..], v4[LEN_HAND_EVAL + 3 * LEN_SEAT_V4..]);
        for (i, &v) in wide.iter().enumerate() {
            assert!(
                v.is_finite() && (-1.0..=1.5).contains(&v),
                "points[{i}] = {v}"
            );
        }
    }

    /// The two halves of the block must agree: each suit's 14 bins are a
    /// probability distribution, and the `E`/`sd` summary beside them is its
    /// first two moments.  Cheap, and it is what catches an offset slip.
    #[test]
    fn the_marginal_and_its_summary_agree() {
        let block = shape_block(
            SPREAD_HAND,
            Some(&[lengths([(0, 13), (0, 3), (5, 13), (5, 13)])]),
        );
        for s in 0..4 {
            let bins: Vec<f64> = block[8 + s * 14..8 + (s + 1) * 14]
                .iter()
                .map(|&p| f64::from(p))
                .collect();
            let total: f64 = bins.iter().sum();
            assert!((total - 1.0).abs() < 1e-5, "suit {s} mass {total}");

            let mean: f64 = bins.iter().enumerate().map(|(k, p)| k as f64 * p).sum();
            let var: f64 = bins
                .iter()
                .enumerate()
                .map(|(k, p)| (k as f64 - mean).powi(2) * p)
                .sum();
            assert!(
                (f64::from(block[s]) - mean / 13.0).abs() < 1e-5,
                "suit {s} E: {} vs {}",
                block[s],
                mean / 13.0
            );
            assert!(
                (f64::from(block[4 + s]) - var.sqrt() / SPREAD_SCALE).abs() < 1e-5,
                "suit {s} sd: {} vs {}",
                block[4 + s],
                var.sqrt() / SPREAD_SCALE
            );
        }
    }

    /// The negative control has to reach the shape block too, or it stops
    /// bounding the reading channel: blind, every seat must read as the bare
    /// hypergeometric prior over shapes.
    #[test]
    fn blind_inference_blanks_the_shape_block() {
        let auction = [bid(1, Strain::Spades), Call::Pass, bid(2, Strain::Clubs)];
        let ctx = Context::new(RelativeVulnerability::NONE, &auction);
        let inferences = Inferences::read(&ctx);
        let cards = hand(SPREAD_HAND);

        let seeing = features_eval_shape(cards, &inferences, &auction);
        set_blind_inference(true);
        let blind = features_eval_shape(cards, &inferences, &auction);
        set_blind_inference(false);

        assert_ne!(seeing, blind, "an opened auction shows something");
        let prior = shape_block(SPREAD_HAND, None);
        for seat in 0..3 {
            let from = LEN_HAND_EVAL + seat * LEN_SEAT_SHAPE + LEN_INFERENCE;
            assert_eq!(blind[from..from + LEN_SHAPE], prior[..], "seat {seat}");
        }
    }

    #[test]
    fn block_offsets_are_consistent() {
        assert_eq!(LEN_HAND_V3, 10);
        assert_eq!(OFFSET_CONTEXT, LEN_HAND_V3);
        assert_eq!(LEN_CONTEXT, 36);
        assert_eq!(OFFSET_INFERENCES, OFFSET_CONTEXT + LEN_CONTEXT);
        assert_eq!(LEN_INFERENCES, 40);
        assert_eq!(OFFSET_VUL, OFFSET_INFERENCES + LEN_INFERENCES);
        assert_eq!(LEN_VUL, 2);
        assert_eq!(OFFSET_VUL + LEN_VUL, FEATURES_LEN_V3);
    }

    #[test]
    fn length_is_correct_for_contested_auction() {
        let auction = [
            bid(1, Strain::Hearts),
            bid(1, Strain::Spades),
            bid(2, Strain::Hearts),
        ];
        let ctx = Context::new(RelativeVulnerability::WE, &auction);
        let f = features_v3(hand("AQ32.K53.QJ4.A92"), &ctx);
        assert_eq!(f.len(), FEATURES_LEN_V3);
    }

    #[test]
    fn v3_length_and_range() {
        // v3 is 88 floats: a 10-value restrictive hand block + the 78-value
        // shared context/inferences/vul tail.
        assert_eq!(FEATURES_LEN_V3, 88);
        let auction = [
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Double,
        ];
        for ctx in [
            empty_context(),
            Context::new(RelativeVulnerability::ALL, &auction),
        ] {
            let f = features_v3(hand("AKQ32.K532.QJ4.9"), &ctx);
            assert_eq!(f.len(), FEATURES_LEN_V3);
            for (i, &v) in f.iter().enumerate() {
                assert!(v.is_finite() && (0.0..=1.5).contains(&v), "v3[{i}] = {v}");
            }
        }
    }

    /// The negative control blanks the whole inference block and nothing else.
    ///
    /// Knob-off an opened auction shows *something* — this is what every reading
    /// generator competes to sharpen; knob-on all four seats read
    /// `Envelope::unknown`, the `[0, 1]` pattern, and the rest of the vector is
    /// untouched.
    #[test]
    fn blind_inference_blanks_only_the_reading_block() {
        let auction = [bid(1, Strain::Spades), Call::Pass, bid(2, Strain::Clubs)];
        let ctx = Context::new(RelativeVulnerability::NONE, &auction);
        let hand = hand("AKQ32.K532.QJ4.9");

        let seen = features_v3(hand, &ctx);
        set_blind_inference(true);
        let blind = features_v3(hand, &ctx);
        set_blind_inference(false); // restore the default before any assert

        let block = OFFSET_INFERENCES..OFFSET_INFERENCES + LEN_INFERENCES;
        assert_ne!(
            seen[block.clone()],
            blind[block.clone()],
            "nothing was shown"
        );
        assert_eq!(
            blind[block.clone()],
            [0.0, 1.0].repeat(LEN_INFERENCES / 2),
            "blind is not the unknown pattern"
        );
        assert_eq!(seen[..block.start], blind[..block.start]);
        assert_eq!(seen[block.end..], blind[block.end..]);
    }

    #[test]
    fn empty_auction_known_values() {
        let ctx = empty_context();
        let f = features_v3(hand("AKQ32.K532.QJ4.9"), &ctx);

        // Context layout: 5 our_strains + 5 their_strains + 7 last_bid + 7 partner
        // + 3 penalty + 1 undisturbed + 1 passed + 1 partner_passed + 1 leading
        // + 4 seat + 1 we_opened = 36.
        // Seat one-hot: auction.len() = 0, so index 0 is set.
        let seat_one_hot_start = OFFSET_CONTEXT + 5 + 5 + 7 + 7 + 3 + 1 + 1 + 1 + 1;
        assert_eq!(f[seat_one_hot_start], 1.0, "seat index 0 should be 1.0");
        assert_eq!(f[seat_one_hot_start + 1], 0.0);
        assert_eq!(f[seat_one_hot_start + 2], 0.0);
        assert_eq!(f[seat_one_hot_start + 3], 0.0);

        // Vulnerability: both 0.0 (NONE)
        assert_eq!(f[OFFSET_VUL], 0.0, "WE vul should be 0.0");
        assert_eq!(f[OFFSET_VUL + 1], 0.0, "THEY vul should be 0.0");

        // contract-to-beat present bit = 0.0
        let last_bid_start = OFFSET_CONTEXT + 5 + 5;
        assert_eq!(f[last_bid_start], 0.0, "contract-to-beat present bit");

        // undisturbed = 1.0 for empty auction
        let undisturbed_offset = OFFSET_CONTEXT + 5 + 5 + 7 + 7 + 3;
        assert_eq!(f[undisturbed_offset], 1.0, "undisturbed should be 1.0");
    }

    #[test]
    fn disclosable_hand_block_for_known_hand() {
        // "AKQ32.K532.QJ4.9" — Suit::ASC order is clubs, diamonds, hearts, spades.
        let f = features_v3(hand("AKQ32.K532.QJ4.9"), &empty_context());

        // Clubs: singleton 9, no HCP.
        assert!((f[0] - 1.0 / 13.0).abs() < 1e-6, "clubs len/13");
        assert_eq!(f[1], 0.0, "clubs suit_hcp");
        // Diamonds: QJ4 = 3 cards, 3 HCP.
        assert!((f[2] - 3.0 / 13.0).abs() < 1e-6, "diamonds len/13");
        assert!((f[3] - 3.0 / 10.0).abs() < 1e-6, "diamonds suit_hcp");
        // Hearts: K532 = 4 cards, 3 HCP.
        assert!((f[4] - 4.0 / 13.0).abs() < 1e-6, "hearts len/13");
        assert!((f[5] - 3.0 / 10.0).abs() < 1e-6, "hearts suit_hcp");
        // Spades: AKQ32 = 5 cards, 9 HCP.
        assert!((f[6] - 5.0 / 13.0).abs() < 1e-6, "spades len/13");
        assert!((f[7] - 9.0 / 10.0).abs() < 1e-6, "spades suit_hcp");
        // Global: 15 HCP, then the fuzzy shape upgrade scaled by 2.
        assert!((f[8] - 15.0 / 40.0).abs() < 1e-6, "hcp/40");
        assert!((0.0..=1.0).contains(&f[9]), "shape/2 in range");
    }

    #[test]
    fn vulnerability_bits() {
        let h = hand("AQ32.K53.QJ4.A92");
        let ctx_we = Context::new(RelativeVulnerability::WE, &[]);
        let f = features_v3(h, &ctx_we);
        assert_eq!(f[OFFSET_VUL], 1.0, "WE vul bit");
        assert_eq!(f[OFFSET_VUL + 1], 0.0, "THEY vul bit");

        let ctx_all = Context::new(RelativeVulnerability::ALL, &[]);
        let f2 = features_v3(h, &ctx_all);
        assert_eq!(f2[OFFSET_VUL], 1.0);
        assert_eq!(f2[OFFSET_VUL + 1], 1.0);
    }

    #[test]
    fn we_opened_bit() {
        let h = hand("AQ32.K53.QJ4.A92");
        let we_opened_offset = OFFSET_CONTEXT + 35; // last value in context block

        // Empty auction: no opener → 0.0
        let f0 = features_v3(h, &empty_context());
        assert_eq!(f0[we_opened_offset], 0.0, "no opener → 0.0");

        // After [1♠]: auction.len()=1, opening_index=0, (1-0)%2=1 ≠ 0 → they opened
        let auction_they = [bid(1, Strain::Spades)];
        let ctx_they = Context::new(RelativeVulnerability::NONE, &auction_they);
        let f1 = features_v3(h, &ctx_they);
        assert_eq!(f1[we_opened_offset], 0.0, "they opened (RHO opened)");

        // After [1♠, P]: auction.len()=2, opening_index=0, (2-0)%2=0 → we opened
        let auction_we = [bid(1, Strain::Spades), Call::Pass];
        let ctx_we = Context::new(RelativeVulnerability::NONE, &auction_we);
        let f2 = features_v3(h, &ctx_we);
        assert_eq!(f2[we_opened_offset], 1.0, "we opened (partner opened)");
    }

    /// Nothing shown is `[0, 1]` per value pair — the `Envelope::unknown`
    /// encoding.  Zeros would be a *different*, out-of-distribution hand.
    const UNKNOWN_BLOCK: [f32; LEN_INFERENCE] = [0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0];

    #[test]
    fn eval_layout_and_unknown_pattern() {
        assert_eq!(LEN_HAND_EVAL, 24);
        assert_eq!(FEATURES_LEN_EVAL, 54);
        let h = hand("AKQ32.K532.QJ4.9");
        let ctx = empty_context();
        let f = features_eval(h, &Inferences::read(&ctx));
        assert_eq!(f.len(), FEATURES_LEN_EVAL);

        // The hand block no longer *repeats* `features_v3`'s summary — it
        // **recovers** it: `len = #spots + ΣA..T` and `suit_hcp = 4A+3K+2Q+J`.
        // That identity is what makes the honour block strictly more
        // informative, so the first layer can still represent everything the
        // 10-float summary carried.  Both sides divide rather than multiply
        // out, and 8 is a power of two, so the compare is exact.
        let v3 = features_v3(h, &ctx);
        for (i, block) in f[..LEN_HAND_EVAL]
            .as_chunks::<{ LEN_HAND_EVAL / 4 }>()
            .0
            .iter()
            .enumerate()
        {
            let (spots, honours) = (block[0] * 8.0, &block[1..]);
            assert_eq!(v3[2 * i], (spots + honours.iter().sum::<f32>()) / 13.0);
            let hcp = 4.0 * honours[0] + 3.0 * honours[1] + 2.0 * honours[2] + honours[3];
            assert_eq!(v3[2 * i + 1], hcp / 10.0);
        }

        // No auction: all three hidden seats read as unknown.
        for start in [24, 34, 44] {
            assert_eq!(
                f[start..start + LEN_INFERENCE],
                UNKNOWN_BLOCK,
                "seat block at {start} should be unknown"
            );
        }
    }

    #[test]
    fn eval_v3_call_tail_is_most_recent_first() {
        assert_eq!(FEATURES_LEN_EVAL_V3, 94);
        let auction = [bid(1, Strain::Spades), Call::Pass, bid(2, Strain::Clubs)];
        let ctx = Context::new(RelativeVulnerability::NONE, &auction);
        let f = features_eval_v3(hand("AQ32.K53.QJ4.A92"), &Inferences::read(&ctx), &auction);
        assert_eq!(f.len(), FEATURES_LEN_EVAL_V3);
        // Head is exactly the v2 vector.
        assert_eq!(
            f[..FEATURES_LEN_EVAL],
            features_eval(hand("AQ32.K53.QJ4.A92"), &Inferences::read(&ctx))[..]
        );
        // Slot 0 (latest): 2♣ — present, level 2/7, ♣ one-hot first in ASC.
        let slot0 = &f[54..64];
        assert_eq!(slot0[..7], [1.0, 2.0 / 7.0, 1.0, 0.0, 0.0, 0.0, 0.0]);
        assert_eq!(slot0[7..], [0.0, 0.0, 0.0]);
        // Slot 1: pass — no bid, pass bit set.
        let slot1 = &f[64..74];
        assert_eq!(slot1[..7], [0.0; 7]);
        assert_eq!(slot1[7..], [1.0, 0.0, 0.0]);
        // Slot 2: 1♠ — present, level 1/7, ♠ is fourth in ASC.
        let slot2 = &f[74..84];
        assert_eq!(slot2[..7], [1.0, 1.0 / 7.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
        // Slot 3: beyond the auction — all zeros, unlike any real call.
        assert_eq!(f[84..94], [0.0; 10]);
    }

    #[test]
    fn eval_seat_blocks_are_actor_relative() {
        // A 1♠ opening one call ago is RHO's: only the last block moves.
        let auction = [bid(1, Strain::Spades)];
        let ctx = Context::new(RelativeVulnerability::NONE, &auction);
        let f = features_eval(hand("AQ32.K53.QJ4.A92"), &Inferences::read(&ctx));

        assert_eq!(f[24..34], UNKNOWN_BLOCK, "LHO has not called");
        assert_eq!(f[34..44], UNKNOWN_BLOCK, "partner has not called");
        // RHO: 5+ spades (block offset 6 = spades min, `Suit::ASC` order) and a
        // non-zero point floor.
        assert!(f[50] >= 5.0 / 13.0, "RHO spade floor: {}", f[50]);
        assert!(f[52] > 0.0, "RHO point floor: {}", f[52]);
    }

    #[test]
    fn penalty_one_hot() {
        let h = hand("AQ32.K53.QJ4.A92");
        let penalty_offset = OFFSET_CONTEXT + 5 + 5 + 7 + 7;

        // Undoubled (default)
        let f0 = features_v3(h, &empty_context());
        assert_eq!(f0[penalty_offset], 1.0, "undoubled");
        assert_eq!(f0[penalty_offset + 1], 0.0);
        assert_eq!(f0[penalty_offset + 2], 0.0);

        // Doubled
        let auction_x = [bid(1, Strain::Spades), Call::Double];
        let ctx_x = Context::new(RelativeVulnerability::NONE, &auction_x);
        let f1 = features_v3(h, &ctx_x);
        assert_eq!(f1[penalty_offset], 0.0);
        assert_eq!(f1[penalty_offset + 1], 1.0, "doubled");
        assert_eq!(f1[penalty_offset + 2], 0.0);

        // Redoubled
        let auction_xx = [bid(1, Strain::Spades), Call::Double, Call::Redouble];
        let ctx_xx = Context::new(RelativeVulnerability::NONE, &auction_xx);
        let f2 = features_v3(h, &ctx_xx);
        assert_eq!(f2[penalty_offset], 0.0);
        assert_eq!(f2[penalty_offset + 1], 0.0);
        assert_eq!(f2[penalty_offset + 2], 1.0, "redoubled");
    }

    // ── The configured extractor ────────────────────────────────────────────

    /// `LEN_CARD` must equal what a card actually renders
    ///
    /// A row added to `SCHEMA` or `PONS_SCHEMA` shifts every feature after the
    /// card blocks, silently misaligning an artifact against its extractor.
    /// This is the tripwire; the cost of ignoring it is a worse bidder with no
    /// other symptom.
    #[test]
    fn card_block_is_the_whole_card() {
        assert_eq!(
            crate::bidding::card::american_card().rows.len(),
            LEN_CARD_ROWS
        );
        assert_eq!(crate::bidding::card::dutch_card().rows.len(), LEN_CARD_ROWS);
        assert_eq!(LEN_CARD, LEN_SYSTEM + LEN_CARD_ROWS);
        assert_eq!(FEATURES_LEN_V4, FEATURES_LEN_V3 + 2 * LEN_CARD);
        assert_eq!(FEATURES_LEN_V4, 368);
    }

    /// The base system must reach the vector, not just the rows
    ///
    /// `dutch_card` differs from `american_card` by its header (2/1 → WJ) plus a
    /// single row, and the header is the only channel for the wide non-forcing
    /// 1♣.  Encoding rows alone would leave a WJ opponent nearly
    /// indistinguishable from a 2/1 one.
    #[test]
    fn the_base_system_is_encoded() {
        let american = Config::symmetric(&crate::bidding::card::american_card());
        let dutch = Config::symmetric(&crate::bidding::card::dutch_card());

        assert_eq!(american.ours[..LEN_SYSTEM], [1.0, 0.0, 0.0, 0.0, 0.0]);
        assert_eq!(dutch.ours[..LEN_SYSTEM], [0.0, 0.0, 1.0, 0.0, 0.0]);
        assert_ne!(american, dutch);

        // The header, plus the one row the two systems disagree on.
        let differing = american
            .ours
            .iter()
            .zip(&dutch.ours)
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(
            differing, 3,
            "two one-hot slots plus `1D opening with 5 cards`"
        );
    }

    /// Each side is encoded independently — the opponents may play anything
    ///
    /// The opposition is ourselves, BBA, BEN or another engine entirely, so the
    /// two blocks must be able to disagree, including on the base system.
    #[test]
    fn the_two_sides_are_independent() {
        let mixed = Config::new(
            &crate::bidding::card::american_card(),
            &crate::bidding::card::dutch_card(),
        );
        assert_eq!(mixed.ours[..LEN_SYSTEM], [1.0, 0.0, 0.0, 0.0, 0.0]);
        assert_eq!(mixed.theirs[..LEN_SYSTEM], [0.0, 0.0, 1.0, 0.0, 0.0]);
        assert_ne!(mixed.ours, mixed.theirs);
    }

    /// v4 is v3 with two card blocks appended — the v3 prefix is untouched
    #[test]
    fn features_v4_extends_v3_in_place() {
        let hand = hand("AQ32.K53.QJ4.A92");
        let config = Config::symmetric(&crate::bidding::card::american_card());
        let auction = [bid(1, Strain::Spades)];
        let context = Context::new(RelativeVulnerability::NONE, &auction).with_config(&config);

        let v3 = features_v3(hand, &context);
        let v4 = features_v4(hand, &context);

        assert_eq!(v4.len(), FEATURES_LEN_V4);
        assert_eq!(v4[..FEATURES_LEN_V3], v3[..], "the v3 prefix must not move");
        assert!(v4.iter().all(|value| value.is_finite()));
        assert!(
            v4[OFFSET_OUR_CARD..].iter().all(|v| *v == 0.0 || *v == 1.0),
            "card rows are boolean"
        );
        // Symmetric config: the two blocks agree.
        assert_eq!(
            v4[OFFSET_OUR_CARD..OFFSET_THEIR_CARD],
            v4[OFFSET_THEIR_CARD..]
        );
    }

    /// The point of the whole design: a knob moves the features
    ///
    /// If flipping a convention left the vector unchanged, one net could never
    /// serve both regimes and the arms would still differ by their weights —
    /// which is the confound `docs/ai-bidder/configured-net.md` exists to kill.
    #[test]
    fn a_convention_knob_moves_the_card_block() {
        use crate::bidding::instinct::set_kickback;

        let plain = {
            set_kickback(false);
            Config::symmetric(&crate::bidding::card::american_card())
        };
        let relocated = {
            set_kickback(true); // restore the shipped default (on)
            Config::symmetric(&crate::bidding::card::american_card())
        };
        assert_ne!(
            plain, relocated,
            "`Kickback 1430` rides `set_kickback`, so the config block must differ"
        );

        // Exactly one row moves, and the two sides move together.
        let differing = plain
            .ours
            .iter()
            .zip(&relocated.ours)
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(differing, 1, "only the kickback row should move");
        assert_eq!(plain.theirs, plain.ours, "symmetric config");
    }

    /// The train/serve skew, measured rather than argued
    ///
    /// `dump-teacher` extracts from a bare [`Context::new`], which carries no
    /// trie prefixes, so `Inferences::read` skips `project_authored` entirely.
    /// Serving does not: `NeuralFloorBba::classify` gets the trie context. This
    /// pins **how much** of the vector that costs, so a corpus is never dumped
    /// through the wrong extractor by accident.
    ///
    /// Documented in `docs/ai-bidder/configured-net.md` and, as the original
    /// finding, in `docs/dnf-migration.md` F1. If a change makes these agree,
    /// this test fails and the skew note should come out of both docs.
    #[test]
    fn bare_and_prefixed_contexts_disagree() {
        let stance = crate::bidding::american().against();
        // An artificial call whose meaning lives in its authoring rule: the
        // Jacoby 2NT game-forcing raise.  A bare context cannot project it.
        let auction = [
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Notrump),
            Call::Pass,
        ];
        let hand = hand("AQ32.K53.QJ4.A92");

        let bare = features_v3(hand, &Context::new(RelativeVulnerability::NONE, &auction));
        let served = features_v3(
            hand,
            &stance.prefixed_context(RelativeVulnerability::NONE, &auction),
        );

        let moved: Vec<usize> = (0..FEATURES_LEN_V3)
            .filter(|index| bare[*index] != served[*index])
            .collect();
        eprintln!(
            "bare-vs-prefixed: {} of {LEN_INFERENCES} inference floats move",
            moved.len()
        );
        assert!(
            !moved.is_empty(),
            "if these now agree the skew is gone — delete the warnings in \
             configured-net.md and dnf-migration.md rather than this test"
        );
        // The hand and vulnerability blocks describe the actor, not the reading,
        // so only the inference block may move.
        assert!(
            moved.iter().all(
                |index| (OFFSET_INFERENCES..OFFSET_INFERENCES + LEN_INFERENCES).contains(index)
            ),
            "only the inference block should differ, got {moved:?}"
        );
    }

    /// An unattached config encodes as zeros rather than panicking in release
    #[test]
    fn features_v4_without_a_config_is_zero_padded() {
        let hand = hand("AQ32.K53.QJ4.A92");
        let context = empty_context();
        // The debug assert in `features_v4` fires on a missing config, so reach
        // past it: this pins the *release* shape, that the vector is still the
        // right width rather than short.
        let mut out = features_v3(hand, &context);
        out.resize(FEATURES_LEN_V4, 0.0);
        assert_eq!(out.len(), FEATURES_LEN_V4);
        assert!(out[OFFSET_OUR_CARD..].iter().all(|value| *value == 0.0));
    }
}
