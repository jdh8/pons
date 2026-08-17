//! Fit the trick evaluator (accountant session C): ranges → double-dummy trick mean
//! and spread.
// The export sidecar's `json!` literal outgrew the default macro recursion
// limit when the survey slices joined it.
#![recursion_limit = "256"]
//!
//! Reads the corpus from `examples/dump-evaluator` — rows of
//! `[features][20 dd_tricks]`, where the features are own-hand summary plus the
//! three hidden seats' shown ranges and **no auction** — and fits a
//! **heteroscedastic Gaussian** per target: two heads, `mu` and `s = ln σ`,
//! trained by negative log-likelihood (dropping the constant `½·ln 2π`):
//!
//! ```text
//! L(t; mu, s) = s + ½·(t − mu)²·exp(−2s)
//! ```
//!
//! Each row is *one* real deal consistent with its ranges, i.e. one unbiased
//! draw from the posterior over hidden hands. Minimising this over the
//! population drives `mu` to the true conditional mean of tricks given the
//! information state and `σ` to its conditional standard deviation — the spread
//! emerges from the population without ever sampling N completions per state.
//!
//! What the Gaussian costs: it *does* assume symmetry and unbounded support,
//! and trick counts are neither — they are discrete, left-skewed on a good fit,
//! and hard-bounded at 13. The `below_mean` metric (fraction of labels strictly
//! under `mu`, nominally 50%) is the diagnostic that measures that cost.
//!
//! ```text
//! cargo run --release --bin evaluator -- --data ../target/eval-train \
//!     --test ../target/eval-test --hidden 64
//! ```
//!
//! `--hidden 0` is the honest linear baseline (~2.5k coefficients); the default
//! `64` is a two-hidden-layer MLP an order of magnitude smaller than the
//! distilled policy net. Deliberately self-contained: the policy trainer's
//! loader hard-asserts a 38-wide softmax, which this corpus does not have.

use anyhow::{Context as _, Result, bail};
use candle_core::{DType, Device, Tensor};
use candle_nn::{AdamW, Linear, Module, Optimizer, ParamsAdamW, VarBuilder, VarMap};
use clap::Parser;
use serde::Deserialize;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

/// Targets per row: 5 strains × 4 declarers.
const DD_LEN: usize = 20;
/// Output heads per target: `mu` and `ln σ`.
const HEADS: usize = 2;
/// Lower clamp on the `ln σ` head, paired with `LN_SD_MAX`. In the corpus's
/// `tricks / 13` units the pair is σ ∈ [0.087, 13] tricks — generous both ways.
/// The clamp exists to stop the classic heteroscedastic collapse, where the net
/// drives σ → 0 on the rows it finds easy and the loss runs to −∞.
// ponytail: a hard clamp zeroes the gradient at the boundary, so a head parked
// there cannot climb back off it. If that ever bites, the upgrade is a softplus
// parameterisation (σ = softplus(raw)), smooth everywhere and needing no clamp.
const LN_SD_MIN: f64 = -5.0;
/// Upper clamp on the `ln σ` head; see `LN_SD_MIN`.
const LN_SD_MAX: f64 = 0.0;
/// Feature floats the three hidden seats' range blocks occupy (the tail).
const LEN_RANGES: usize = 30;
/// One seat's unknown-range encoding: `[min, max]` pairs of `[0, 1]`.
const UNKNOWN_PAIR: [f32; 2] = [0.0, 1.0];
/// Columns the `bits` superset writes before the Phase-3 oracle tail.
const BITS_FEATURES: usize = 79;
/// The hidden-seat axis-survey oracle tail (`dump-evaluator --oracle-all`),
/// as `(start, len)` column ranges. Only the `*-oracle*` arms keep their block;
/// every other arm masks the whole tail off, so one oracle corpus trains the
/// whole ladder. Per-axis blocks cover all three hidden seats [LHO, partner,
/// RHO], suits `Suit::ASC` within a seat — see `dump-evaluator`'s
/// `ORACLE_ALL_LEN` for the cell semantics.
const ORACLE_KEYCARDS: (usize, usize) = (BITS_FEATURES, 8);
/// Per-suit `suit_hcp/10` truth ×3 seats.
const ORACLE_QUALITY: (usize, usize) = (87, 12);
/// Per-suit `len ≤ 1` bit ×3 seats.
const ORACLE_SHORTNESS: (usize, usize) = (99, 12);
/// Per-suit (ace, king) bits ×3 seats.
const ORACLE_CONTROLS: (usize, usize) = (111, 24);
/// Per-suit A/Kx/Qxx/Jxxx stopper bit ×3 seats.
const ORACLE_STOPPER: (usize, usize) = (135, 12);
/// The `dump-evaluator --auction` block: 4 most-recent calls × 40 columns
/// (bid encoding, call-kind bits, tag multi-hot, alerted bit, hashed alert).
///
/// It starts at [`BITS_FEATURES`] like the oracle tail — the two are mutually
/// exclusive on the dumper side, so columns past 79 are *either* oracle truth
/// (147-wide corpus) or the auction block (239-wide corpus). The sidecar's
/// `auction` flag is what tells the arms apart; see the guards in `main`.
const AUCTION: (usize, usize) = (BITS_FEATURES, 160);

/// The bare-call slice of the `--auction` block: per 40-column call slot, only
/// the 10 call-identity columns (7-value bid encoding + pass/X/XX bits),
/// zeroing the 21 structural tags, the alerted bit, and the 8-bucket alert
/// hash. The "hulls replace tags and alerts" arm: if `ben-calls` matches
/// `ben-auction`, the tag/alert columns were redundant given the ranges.
const AUCTION_CALLS_ONLY: [(usize, usize); 4] = [
    (BITS_FEATURES, 10),
    (BITS_FEATURES + 40, 10),
    (BITS_FEATURES + 80, 10),
    (BITS_FEATURES + 120, 10),
];
/// The `dump-evaluator --encoding shape` layout: the shape-reading research
/// superset.  A 24-float honour hand block, then three hidden-seat blocks of
/// [`SHAPE_SEAT`], then the 4×10 call tail — see the dumper's `shape_layout`
/// sidecar field.
///
/// ```text
/// 0..24        hand: 4 suits × [#spots/8, A, K, Q, J, T]   (always live)
/// 24 + 45·i    seat i of [LHO, partner, RHO]:
///     +0..8      hull length endpoints, 4 × (min, max) ÷ 13
///     +8..10     hull points (min, max) ÷ 37
///     +10..18    shape: E[len] ×4, sd[len] ×4      (the Gaussian summary)
///     +18..74    shape: P(len_s = k), suit-major, 4 × 14 bins
///     +74        shape: pinned = −ln(mass) / ln C(39,13)
/// 249..289     4 most-recent calls × 10 identity columns  (always live)
/// ```
const SHAPE_HAND: usize = 24;
/// Columns one hidden seat occupies in a `shape` corpus.
const SHAPE_SEAT: usize = 75;
/// Width of a `shape` corpus — the [`Arm`] hybrid arm's live count.
const SHAPE_FEATURES: usize = SHAPE_HAND + 3 * SHAPE_SEAT + 40;

/// Sub-blocks of one seat's [`SHAPE_SEAT`] columns, as `(offset, len)`.  The
/// arms are combinations of these, and the campaign question is whether the
/// distributional blocks can *replace* [`HULL_LEN`] rather than sit beside it:
/// keeping the endpoints keeps their non-invariance to information-free
/// re-hulling, which is the defect the encoding exists to fix.
const HULL_LEN: (usize, usize) = (0, 8);
/// The `points` endpoints — orthogonal to the shape block, which is
/// length-only, so every arm keeps them.
const HULL_POINTS: (usize, usize) = (8, 2);
/// `E[len]` and `sd[len]`: the 1:1 replacement for [`HULL_LEN`], and the
/// "Gaussian" family — round one measured it at *par* with the endpoints.
const SHAPE_MOMENTS: (usize, usize) = (10, 8);
/// The full per-suit length marginal, 4 suits × 14 bins — the non-parametric
/// family. Subsumes round one's threshold and tail masses, each of which was a
/// sum of these bins.
const SHAPE_HIST: (usize, usize) = (18, 56);
/// One column: how much the reading pins down, `−ln(mass) / ln C(39,13)`.
/// Round one never isolated it — the +0.0011 arm carried it together with the
/// thresholds and tails, so which of the three paid is exactly what round two
/// decomposes.
const SHAPE_MASS: (usize, usize) = (74, 1);

/// The `dump-evaluator --encoding points` layout: the strength-reading research
/// superset.  Same 24-float hand block and 4×10 call tail as a `shape` corpus,
/// with a narrower seat block — the shipped v4 vector plus the two new strength
/// blocks, and none of the 56-bin length marginal.
///
/// ```text
/// 0..24        hand: 4 suits × [#spots/8, A, K, Q, J, T]   (always live)
/// 24 + 24·i    seat i of [LHO, partner, RHO]:
///     +0..8      hull length endpoints, 4 × (min, max) ÷ 13
///     +8..10     hull points (min, max) ÷ 37
///     +10..18    shape: E[len] ×4, sd[len] ×4
///     +18        shape: pinned = −ln(mass) / ln C(39,13)
///     +19..21    hull raw HCP (min, max) ÷ 37   — the axis no net has read
///     +21..23    strength: E[hcp], sd[hcp]
///     +23        strength: pinned, same scale as the shape mass
/// 96..136      4 most-recent calls × 10 identity columns  (always live)
/// ```
const POINTS_HAND: usize = 24;
/// Columns one hidden seat occupies in a `points` corpus.
const POINTS_SEAT: usize = 24;
/// Width of a `points` corpus.
const POINTS_FEATURES: usize = POINTS_HAND + 3 * POINTS_SEAT + 40;

/// Sub-blocks of one seat's [`POINTS_SEAT`] columns, as `(offset, len)`.  The
/// campaign question is the same one the shape sweep asked, one axis over:
/// can the strength distribution *replace* [`P_HULL_POINTS`] rather than sit
/// beside it — and, separately, is there anything in [`P_HCP_ENDS`], which is
/// written unslacked wherever `points` is slacked and which no shipped vector
/// has ever read.
/// The hull length endpoints — the *v3* shape reading, which the shipped
/// `features_eval_v4` vector dropped for [`P_SHAPE_GAUSS`].  Every arm above
/// prices strength against the v4 baseline; the two `pts-len-*` arms price it
/// against v3, which is the vector that actually ships.
const P_HULL_LEN: (usize, usize) = (0, 8);
/// The `points` endpoints — what the strength blocks are trying to replace.
const P_HULL_POINTS: (usize, usize) = (8, 2);
/// The shipped v4 shape reading, moments and mass together.  Every arm keeps
/// it: the sweep prices strength against a fixed shape baseline.
const P_SHAPE_GAUSS: (usize, usize) = (10, 9);
/// The crisp raw-HCP endpoints.
const P_HCP_ENDS: (usize, usize) = (19, 2);
/// `E[hcp]` and `sd[hcp]`: the 1:1 replacement for [`P_HULL_POINTS`].
const P_HCP_MOMENTS: (usize, usize) = (21, 2);
/// One column: how much the reading pins strength down.
const P_HCP_MASS: (usize, usize) = (23, 1);

/// Feature width the [`Arm`] masks are written against.
///
/// ```text
/// 0..32     hand: 4 suits × [len/13, #spots/8, suit_hcp/10, A, K, Q, J, T]
/// 32        hcp/40
/// 33        upgrade/2
/// 34..79    ranges: 3 seats × 5 (min, max, width) triples
/// 79..87    oracle: partner keycards ×4 (Suit::ASC), then trump-Q ×4
/// 87..147   axis-survey oracle: quality, shortness, controls, stopper
/// 79..239   — or, on an `--auction` corpus, the auction+alert block
/// ```
///
/// Narrower corpora are fine: [`Dataset::mask_features`]' zip stops at the
/// corpus width, and a per-arm coverage check refuses an arm whose highest
/// live column the corpus does not reach.
///
/// The Phase-3 87-wide `--oracle` corpus is retired by this bump; regenerate
/// with `--oracle-all` (same walk, same seed reproduces the same auctions).
///
/// Wide enough for *either* layout — the `bits` superset's auction tail (239)
/// or a `shape` corpus (289). A mask wider than the corpus is harmless
/// ([`Dataset::mask_features`] zips against the row), and the per-arm coverage
/// check is what refuses an arm whose live columns the corpus does not reach.
const ARM_FEATURES: usize = if SHAPE_FEATURES > AUCTION.0 + AUCTION.1 {
    SHAPE_FEATURES
} else {
    AUCTION.0 + AUCTION.1
};

/// One [`Arm`]'s mask recipe — the [`Arm::spec`] row: `(name, hand-column
/// offsets within each suit's 8, keep global `hcp`, keep global `upgrade`,
/// offsets within each range triple, oracle-tail `(start, len)` ranges to
/// keep, live column count)`.
type ArmSpec = (
    &'static str,
    &'static [usize],
    bool,
    bool,
    &'static [usize],
    &'static [(usize, usize)],
    usize,
);

/// splitmix64: advance `state` and return the next word.
///
/// A few lines beat a new dependency, and reproducibility is the only thing
/// asked of it. Shared by [`seed_params`] and the per-epoch shuffle.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Deep-copy every parameter, for the best-val checkpoint.
///
/// `Var::set` writes through the existing storage, so a cloned `Tensor` would
/// alias the live parameter and be clobbered by the next optimiser step —
/// [`Tensor::copy`] is what makes the snapshot a snapshot.
fn snapshot(varmap: &VarMap) -> Result<Vec<(String, Tensor)>> {
    let data = varmap.data().lock().expect("varmap poisoned");
    data.iter()
        .map(|(name, var)| Ok((name.clone(), var.as_tensor().copy()?)))
        .collect()
}

/// Deterministically re-initialise every parameter from `seed`.
///
/// candle's CPU device rejects `set_seed`, so `VarBuilder`'s init is drawn from
/// an unseeded thread-local RNG. Overwriting the parameters afterwards is the
/// cheapest way to make a run reproducible — and it has to be reproducible,
/// because the `ln σ` head is init-sensitive (see [`Args::seed`]).
///
/// Weights are `U(-k, k)` with `k = 1/√fan_in` (PyTorch's `Linear` default);
/// biases start at zero.
/// `live_in` is how many of the `in_dim` input columns an `--arm` left live. It
/// only ever differs from `in_dim` under masking, and it matters because
/// `k = 1/√fan_in` would otherwise be drawn from the *padded* width: a 40-live
/// arm of a 79-wide corpus would start at `40/79 ≈ 0.5×` the pre-activation
/// variance the same net trained on a real 40-float corpus gets. Measured, that
/// is not cosmetic — it widens the catchment of the `ln σ` bad-init basin, and
/// the sparsest arm of the first featurization sweep fell into it at epoch 1
/// while every denser arm converged. Draw as the net the arm actually is.
fn seed_params(
    varmap: &VarMap,
    seed: u64,
    device: &Device,
    in_dim: usize,
    live_in: usize,
) -> Result<()> {
    let mut state = seed;
    let mut unit = move || (splitmix64(&mut state) >> 11) as f64 / (1u64 << 53) as f64;
    let data = varmap.data().lock().expect("varmap poisoned");
    // Sorted, so the draw order does not depend on HashMap iteration order.
    let mut names: Vec<_> = data.keys().collect();
    names.sort();
    for name in names {
        let var = &data[name];
        let tensor = match *var.dims() {
            [out, fan_in] => {
                // Only `l1` reads the (possibly padded) corpus width, so key off
                // the name. The old shape test `fan_in == in_dim` also caught
                // `l2` (hidden, hidden) and `l3` (out_dim, hidden) whenever
                // `--hidden` happened to equal `in_dim`, drawing those at
                // `1/√live_in` instead of `1/√hidden`. The assert pins the name,
                // so a rename of the export order fails loudly here rather than
                // silently switching the narrowing off.
                let first = name == "l1.weight";
                assert!(!first || fan_in == in_dim, "{name} is not the input layer");
                let effective = if first { live_in } else { fan_in };
                let k = 1.0 / (effective as f64).sqrt();
                let v: Vec<f32> = (0..out * fan_in)
                    .map(|_| ((2.0 * unit() - 1.0) * k) as f32)
                    .collect();
                Tensor::from_vec(v, (out, fan_in), device)?
            }
            [out] => Tensor::zeros(out, DType::F32, device)?,
            ref other => bail!("unexpected parameter shape {other:?} for {name}"),
        };
        var.set(&tensor)?;
    }
    Ok(())
}

/// Featurization arm: which columns of an [`ARM_FEATURES`]-wide corpus stay
/// live. Every other column is zeroed, in train and val alike.
///
/// Zeroing is equivalent to *deleting* the column for the first linear layer,
/// which is what makes this an honest ablation rather than a handicap: the
/// forward pass contributes `w·0 = 0`, and the gradient `∂L/∂w = δ·x` is `δ·0`
/// — identically zero on every row of every epoch. The weight never leaves its
/// initialisation and never touches an activation. So one corpus serves the
/// whole ladder at an *identical parameter count*, and rungs stay comparable
/// without re-dumping features per arm.
#[derive(Clone, Copy, clap::ValueEnum)]
enum Arm {
    Full,
    Baseline,
    Bits,
    BitsNohcp,
    Ben,
    BitsWidth,
    BaselineDropUpgrade,
    BaselineDropHcp,
    BaselineDropBoth,
    /// `Ben` plus the honour oracle — the interaction arm (own + partner in a
    /// few weights). Phase 3's headline.
    BenOracle,
    /// `BaselineDropBoth` plus the oracle — the control arm; `suit_hcp` alone
    /// cannot extract an ace, so at most a mild main effect is expected.
    BaselineDropBothOracle,
    /// `Ben` plus per-suit `suit_hcp` truth for the hidden seats — the quality
    /// axis of the hidden-seat survey.
    BenOracleQuality,
    /// `Ben` plus per-suit shortness bits for the hidden seats.
    BenOracleShortness,
    /// `Ben` plus per-suit (ace, king) bits for the hidden seats.
    BenOracleControls,
    /// `Ben` plus per-suit stopper bits for the hidden seats.
    BenOracleStopper,
    /// `Ben` plus the `--auction` block — the auction+alert ablation. Needs an
    /// `--auction` corpus (sidecar `auction: true`); the ranges the ben columns
    /// carry already compress the auction, so the delta over `Ben` is what the
    /// raw calls + alerts add beyond that compression.
    BenAuction,
    /// `Ben` plus only the call-identity columns of the `--auction` block —
    /// no tags, no alerts. The delta `ben-auction − ben-calls` is what the
    /// structural tags and alert columns add once the hulls and the raw calls
    /// are both on the table.
    BenCalls,
    /// **Shape corpus** (`--encoding shape`) — the control: hull endpoints and
    /// points, i.e. the shipped `features_eval_v3` vector reproduced out of the
    /// superset. 94 live columns.
    ShapeControl,
    /// Shape corpus — the **Gaussian** family: `E[len]` and `sd[len]`
    /// *replacing* the length endpoints. Same 94 live columns as
    /// `ShapeControl`, so this is a pure re-parameterisation at identical
    /// width; round one measured it at par.
    ShapeGauss,
    /// Shape corpus — the endpoints replaced by **one column per seat**, the
    /// log-mass. The cheapest distributional reading there is: not what the
    /// seat holds, only how much the auction pinned down.
    ShapeMass,
    /// Shape corpus — Gaussian summary plus the log-mass.
    ShapeGaussMass,
    /// Shape corpus — the **marginal** family: the full 14-bin per-suit length
    /// distribution replacing the endpoints, no summary beside it.
    ShapeHist,
    /// Shape corpus — marginal, its Gaussian summary, and the log-mass: the
    /// ceiling of a per-seat, per-suit reading.
    ShapeHistMass,
    /// Shape corpus — distribution *beside* the endpoints. The MARG cell: if
    /// this ties `ShapeFull`, the endpoints were redundant; if it ties
    /// `ShapeControl`, the distribution was.
    ShapeHybrid,
    /// **Points corpus** (`--encoding points`) — the control: the shipped
    /// `features_eval_v4` vector reproduced out of the superset. 97 live
    /// columns, and the number every other points arm is judged against.
    PtsControl,
    /// Points corpus — `E[hcp]` and `sd[hcp]` *replacing* the `points`
    /// endpoints. Same 97 live columns as `PtsControl`, so this is the pure
    /// re-parameterisation at identical width: the cleanest read of whether a
    /// strength distribution beats a strength interval.
    PtsGauss,
    /// Points corpus — the strength moments plus their log-mass (100).
    PtsGaussMass,
    /// Points corpus — the strength distribution *beside* the endpoints (106).
    /// The shape sweep found the length endpoints spent against exact moments;
    /// this is that test on the strength axis.
    PtsBoth,
    /// Points corpus — the `points` endpoints plus the crisp raw-HCP endpoints
    /// (103), no distribution. Isolates the one block carrying information the
    /// net has never seen, and is what says whether the whole idea is
    /// downstream of an empty axis.
    PtsHcpEnds,
    /// Points corpus — every strength block at once: both endpoint pairs, the
    /// strength moments and their mass (112). The decomposition arm: if the
    /// distribution's win over `PtsControl` is just a weaker proxy for what the
    /// crisp band carries directly, this lands on `PtsHcpEnds` rather than past
    /// it, and a serving vector needs only the two endpoint pairs.
    PtsHcpBoth,
    /// Points corpus — the **v3** baseline: hull length endpoints plus the
    /// `points` endpoints, no shape distribution (94). Every arm above prices
    /// strength on top of v4, which lost its A/B and ships off; this is the
    /// same corpus, same rows, reproducing the vector that actually ships.
    PtsLenControl,
    /// Points corpus — v3 plus the crisp raw-HCP endpoints (100). The arm the
    /// sweep should have run: it is what a serving `features_eval_v5` would be,
    /// and it is a pure mask over the corpus already on disk.
    PtsLenHcpEnds,
}

impl Arm {
    /// The arm table: `(name, hand-column offsets within each suit's 8, keep
    /// global `hcp`, keep global `upgrade`, offsets within each
    /// `(min, max, width)` range triple, oracle-tail `(start, len)` ranges to
    /// keep, live column count)`.
    ///
    /// The two globals are separate flags because they are separately suspect.
    /// `upgrade` is the *legacy* shape term (`!is_balanced + longest_two ≥ 10`)
    /// from when `point_count` was `raw_hcp + upgrade`; when this spec was
    /// drawn the default scale was `RuleOfNFloored`, so it did not reconstruct
    /// the scale the inference blocks record partner's points on (the default
    /// has since returned to `PointCount`, `raw_hcp` plus the linearised
    /// upgrade). And global `hcp` is exactly the sum of
    /// the four `suit_hcp` columns whenever those are live, which is a 4-weight
    /// reconstruction rather than the 16-weight one the coordination argument
    /// was built on.
    const fn spec(self) -> ArmSpec {
        match self {
            Self::Full => (
                "full",
                &[0, 1, 2, 3, 4, 5, 6, 7],
                true,
                true,
                &[0, 1, 2],
                &[],
                79,
            ),
            Self::Baseline => ("baseline", &[0, 2], true, true, &[0, 1], &[], 40),
            Self::Bits => ("bits", &[1, 2, 3, 4, 5, 6, 7], true, true, &[0, 1], &[], 60),
            Self::BitsNohcp => (
                "bits-nohcp",
                &[1, 3, 4, 5, 6, 7],
                true,
                true,
                &[0, 1],
                &[],
                56,
            ),
            Self::Ben => ("ben", &[1, 3, 4, 5, 6, 7], false, false, &[0, 1], &[], 54),
            Self::BitsWidth => (
                "bits-width",
                &[1, 2, 3, 4, 5, 6, 7],
                true,
                true,
                &[0, 1, 2],
                &[],
                75,
            ),
            Self::BaselineDropUpgrade => (
                "baseline-drop-upgrade",
                &[0, 2],
                true,
                false,
                &[0, 1],
                &[],
                39,
            ),
            Self::BaselineDropHcp => ("baseline-drop-hcp", &[0, 2], false, true, &[0, 1], &[], 39),
            Self::BaselineDropBoth => (
                "baseline-drop-both",
                &[0, 2],
                false,
                false,
                &[0, 1],
                &[],
                38,
            ),
            Self::BenOracle => (
                "ben-oracle",
                &[1, 3, 4, 5, 6, 7],
                false,
                false,
                &[0, 1],
                &[ORACLE_KEYCARDS],
                62,
            ),
            Self::BaselineDropBothOracle => (
                "baseline-drop-both-oracle",
                &[0, 2],
                false,
                false,
                &[0, 1],
                &[ORACLE_KEYCARDS],
                46,
            ),
            Self::BenOracleQuality => (
                "ben-oracle-quality",
                &[1, 3, 4, 5, 6, 7],
                false,
                false,
                &[0, 1],
                &[ORACLE_QUALITY],
                66,
            ),
            Self::BenOracleShortness => (
                "ben-oracle-shortness",
                &[1, 3, 4, 5, 6, 7],
                false,
                false,
                &[0, 1],
                &[ORACLE_SHORTNESS],
                66,
            ),
            Self::BenOracleControls => (
                "ben-oracle-controls",
                &[1, 3, 4, 5, 6, 7],
                false,
                false,
                &[0, 1],
                &[ORACLE_CONTROLS],
                78,
            ),
            Self::BenOracleStopper => (
                "ben-oracle-stopper",
                &[1, 3, 4, 5, 6, 7],
                false,
                false,
                &[0, 1],
                &[ORACLE_STOPPER],
                66,
            ),
            Self::BenAuction => (
                "ben-auction",
                &[1, 3, 4, 5, 6, 7],
                false,
                false,
                &[0, 1],
                &[AUCTION],
                214,
            ),
            Self::BenCalls => (
                "ben-calls",
                &[1, 3, 4, 5, 6, 7],
                false,
                false,
                &[0, 1],
                &AUCTION_CALLS_ONLY,
                94,
            ),
            // Written against the `shape` layout instead; `mask` and `name`
            // both branch on `shape_spec` before they ever reach here.
            Self::ShapeControl
            | Self::ShapeGauss
            | Self::ShapeMass
            | Self::ShapeGaussMass
            | Self::ShapeHist
            | Self::ShapeHistMass
            | Self::ShapeHybrid => panic!("shape arms have no `bits` spec"),
            // Likewise, via `points_spec`.
            Self::PtsControl
            | Self::PtsGauss
            | Self::PtsGaussMass
            | Self::PtsBoth
            | Self::PtsHcpEnds
            | Self::PtsHcpBoth
            | Self::PtsLenControl
            | Self::PtsLenHcpEnds => panic!("points arms have no `bits` spec"),
        }
    }

    /// The shape-corpus arms: `(name, per-seat sub-blocks, live column count)`.
    /// `None` for every arm written against the `bits` superset.
    const fn shape_spec(self) -> Option<(&'static str, &'static [(usize, usize)], usize)> {
        // 24 hand + 40 calls are live in every arm; the rest is 3 × per-seat.
        Some(match self {
            Self::ShapeControl => ("shape-control", &[HULL_LEN, HULL_POINTS], 94),
            Self::ShapeGauss => ("shape-gauss", &[HULL_POINTS, SHAPE_MOMENTS], 94),
            Self::ShapeMass => ("shape-mass", &[HULL_POINTS, SHAPE_MASS], 73),
            Self::ShapeGaussMass => (
                "shape-gauss-mass",
                &[HULL_POINTS, SHAPE_MOMENTS, SHAPE_MASS],
                97,
            ),
            Self::ShapeHist => ("shape-hist", &[HULL_POINTS, SHAPE_HIST], 238),
            Self::ShapeHistMass => (
                "shape-hist-mass",
                &[HULL_POINTS, SHAPE_MOMENTS, SHAPE_HIST, SHAPE_MASS],
                265,
            ),
            Self::ShapeHybrid => (
                "shape-hybrid",
                &[HULL_LEN, HULL_POINTS, SHAPE_MOMENTS, SHAPE_HIST, SHAPE_MASS],
                SHAPE_FEATURES,
            ),
            _ => return None,
        })
    }

    /// The points-corpus arms: `(name, per-seat sub-blocks, live column count)`.
    /// `None` for every arm written against another superset.
    const fn points_spec(self) -> Option<(&'static str, &'static [(usize, usize)], usize)> {
        // 24 hand + 40 calls are live in every arm; the rest is 3 × per-seat.
        Some(match self {
            Self::PtsControl => ("pts-control", &[P_HULL_POINTS, P_SHAPE_GAUSS], 97),
            Self::PtsGauss => ("pts-gauss", &[P_HCP_MOMENTS, P_SHAPE_GAUSS], 97),
            Self::PtsGaussMass => (
                "pts-gauss-mass",
                &[P_HCP_MOMENTS, P_HCP_MASS, P_SHAPE_GAUSS],
                100,
            ),
            Self::PtsBoth => (
                "pts-both",
                &[P_HULL_POINTS, P_HCP_MOMENTS, P_HCP_MASS, P_SHAPE_GAUSS],
                106,
            ),
            Self::PtsHcpEnds => (
                "pts-hcp-ends",
                &[P_HULL_POINTS, P_HCP_ENDS, P_SHAPE_GAUSS],
                103,
            ),
            Self::PtsHcpBoth => (
                "pts-hcp-both",
                &[
                    P_HULL_POINTS,
                    P_HCP_ENDS,
                    P_HCP_MOMENTS,
                    P_HCP_MASS,
                    P_SHAPE_GAUSS,
                ],
                112,
            ),
            Self::PtsLenControl => ("pts-len-control", &[P_HULL_LEN, P_HULL_POINTS], 94),
            Self::PtsLenHcpEnds => (
                "pts-len-hcp-ends",
                &[P_HULL_LEN, P_HULL_POINTS, P_HCP_ENDS],
                100,
            ),
            _ => return None,
        })
    }

    /// Which corpus layout this arm's column offsets are written against — the
    /// `--encoding` family it can serve.
    const fn layout(self) -> &'static str {
        if self.shape_spec().is_some() {
            "shape"
        } else if self.points_spec().is_some() {
            "points"
        } else {
            "bits"
        }
    }

    /// The arm's reported name, under any layout
    const fn name(self) -> &'static str {
        if let Some((name, _, _)) = self.shape_spec() {
            return name;
        }
        if let Some((name, _, _)) = self.points_spec() {
            return name;
        }
        self.spec().0
    }

    /// Per-column keep flags. Hard-asserts (in release too — a mis-mask is
    /// invisible in the results, and this runs once at startup) that the live
    /// count matches the width [`Self::spec`] documents, which is what catches a
    /// typo in an offset list.
    fn mask(self) -> [bool; ARM_FEATURES] {
        // The two superset layouts differ only in their per-seat stride and
        // total width, so one builder serves both.
        let seated = self
            .shape_spec()
            .map(|spec| (spec, SHAPE_HAND, SHAPE_SEAT, SHAPE_FEATURES))
            .or_else(|| {
                self.points_spec()
                    .map(|spec| (spec, POINTS_HAND, POINTS_SEAT, POINTS_FEATURES))
            });
        if let Some(((name, blocks, width), hand, seat_len, features)) = seated {
            let mut keep = [false; ARM_FEATURES];
            keep[..hand].fill(true);
            for seat in 0..3 {
                let base = hand + seat * seat_len;
                for &(offset, len) in blocks {
                    keep[base + offset..base + offset + len].fill(true);
                }
            }
            keep[hand + 3 * seat_len..features].fill(true);
            assert_eq!(
                keep.iter().filter(|&&k| k).count(),
                width,
                "arm {name}: live column count disagrees with its documented width"
            );
            return keep;
        }
        let (name, suit, hcp, upgrade, triple, oracle, width) = self.spec();
        let mut keep = [false; ARM_FEATURES];
        for cols in keep[..32].chunks_exact_mut(8) {
            for &o in suit {
                cols[o] = true;
            }
        }
        keep[32] = hcp; // hcp/40
        keep[33] = upgrade; // upgrade/2
        // The 15 triples are laid out uniformly, so the seat boundary every 5 of
        // them needs no special casing. Bound the slice at `BITS_FEATURES`: past
        // it is the oracle tail, which is not triple-encoded.
        for cols in keep[34..BITS_FEATURES].chunks_exact_mut(3) {
            for &o in triple {
                cols[o] = true;
            }
        }
        for &(start, len) in oracle {
            keep[start..start + len].fill(true);
        }
        assert_eq!(
            keep.iter().filter(|&&k| k).count(),
            width,
            "arm {name}: live column count disagrees with its documented width"
        );
        keep
    }
}

#[derive(Parser)]
#[command(about = "Fit the DD trick mean/spread evaluator (accountant session C)")]
struct Args {
    /// Corpus stem; reads `<stem>.f32`, `<stem>.json`, `<stem>.tags`
    #[arg(long, default_value = "../target/eval-train")]
    data: String,
    /// Held-out corpus stem (deal-disjoint by construction — a different
    /// database slice). Without it, a contiguous `--val-frac` tail is used;
    /// the dump is deal-major, so that tail is still deal-disjoint.
    #[arg(long)]
    test: Option<String>,
    /// Output stem: `<stem>.f32` + `<stem>.json` + `<stem>.fixture.json`.
    /// The default is the artifact the crate `include_bytes!`s, behind a
    /// compile-time size assert — so a run at a mismatched width silently
    /// clobbers the shipped weights and breaks the build. Point it elsewhere
    /// for sweeps.
    #[arg(long, default_value = "../src/bidding/weights/evaluator_v2")]
    weights_out: String,
    /// Hidden width of both hidden layers; `0` fits a single linear layer
    #[arg(long, default_value_t = 64)]
    hidden: usize,
    /// Training epochs
    #[arg(long, default_value_t = 60)]
    epochs: usize,
    /// AdamW learning rate
    #[arg(long, default_value_t = 1e-3)]
    lr: f64,
    /// AdamW weight decay
    #[arg(long, default_value_t = 0.0)]
    wd: f64,
    /// Minibatch size
    #[arg(long, default_value_t = 4096)]
    batch: usize,
    /// Seed for weight initialisation. Left unseeded, the `μ` head still lands
    /// within ~0.004 tricks of MAE run to run, but `ln σ` does not: two runs at
    /// identical settings came in 0.075 apart in NLL and 2.7 points apart in
    /// coverage. Anything comparing those two numbers across runs must fix this.
    #[arg(long, default_value_t = 0)]
    seed: u64,
    /// Validation fraction, taken contiguously from the end (ignored with `--test`)
    #[arg(long, default_value_t = 0.10)]
    val_frac: f64,
    /// Rows to dump as the in-crate parity fixture
    #[arg(long, default_value_t = 8)]
    fixture: usize,
    /// Ablation: overwrite every range block with the *unknown* pattern, so the
    /// net sees only its own hand. The delta against a normal run is what the
    /// auction (compressed to ranges) is worth in tricks.
    #[arg(long)]
    blank_ranges: bool,
    /// Ablation: fold the 20 per-declarer targets to 10 per-side ones, taking
    /// the better declarer of each side.
    #[arg(long)]
    collapse_side: bool,
    /// Featurization arm: zero every input column outside the named subset, so
    /// one 79-wide corpus drives the whole ladder at an identical parameter
    /// count. Requires a 79-feature corpus; omitted, nothing is masked and
    /// 40-wide corpora behave exactly as before.
    #[arg(long, value_enum)]
    arm: Option<Arm>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    // Falls back to CPU when built without `--features cuda`, so this is safe
    // unconditionally. The whole corpus lives on the device (8 GB of f32 for
    // 1M deals against the 4090's 24 GB), which is what makes the per-epoch
    // `index_select` shuffle cheap.
    let device = Device::cuda_if_available(0)?;
    eprintln!("device: {device:?}");

    let mut train = Dataset::load(&args.data)?;
    let mut val = match &args.test {
        Some(stem) => {
            let ds = Dataset::load(stem)?;
            if ds.features_len != train.features_len {
                bail!(
                    "feature width mismatch: train {} vs test {}",
                    train.features_len,
                    ds.features_len
                );
            }
            ds
        }
        None => {
            let nval = (((train.rows as f64) * args.val_frac).round() as usize)
                .clamp(1, train.rows.saturating_sub(1));
            train.split_off(train.rows - nval)
        }
    };
    if args.blank_ranges {
        // The pair-wise overwrite below would land on the wrong columns of a
        // triple-encoded range block, silently — so refuse rather than lie.
        // Keyed off the sidecar's encoding, not a width compare: `bits`
        // corpora come 79, 87, or 147 wide and all encode triples.
        if !matches!(train.meta.encoding.as_str(), "summary" | "onehot") {
            bail!(
                "--blank-ranges overwrites the last {LEN_RANGES} columns as \
                 (min, max) pairs; only `summary` and `onehot` corpora put the \
                 range block there (this one is `{}`)",
                train.meta.encoding
            );
        }
        train.blank_ranges();
        val.blank_ranges();
    }
    // Defaults to the corpus width; `--arm` narrows it, and only the first
    // layer's init scale reads it.
    let mut live_in = train.features_len;
    if let Some(arm) = args.arm {
        if train.features_len > ARM_FEATURES {
            bail!(
                "--arm masks are {ARM_FEATURES} wide; this corpus is {} wide",
                train.features_len
            );
        }
        // The two layouts share column offsets with entirely different
        // meanings, so the sidecar's encoding — not the width — is the
        // authority on which family of arms this corpus can serve.
        let corpus_layout = match train.meta.encoding.as_str() {
            "shape" => "shape",
            "points" => "points",
            _ => "bits",
        };
        if arm.layout() != corpus_layout {
            bail!(
                "--arm {} is written against the `{}` layout, but this corpus \
                 is `--encoding {}`",
                arm.name(),
                arm.layout(),
                train.meta.encoding
            );
        }
        // Columns past `BITS_FEATURES` are oracle truth on an oracle corpus but
        // the auction block on an `--auction` one — same offsets, different
        // meaning — so the sidecar flag, not the width, is the authority.
        let is_auction_arm = matches!(arm, Arm::BenAuction | Arm::BenCalls);
        if is_auction_arm && !train.meta.auction {
            bail!(
                "--arm {} needs an `--auction` corpus (sidecar `auction: true`)",
                arm.name()
            );
        }
        // `spec()` panics for the superset layouts, so the `bits` test has to
        // come first and short-circuit.
        if arm.layout() == "bits"
            && !is_auction_arm
            && train.meta.auction
            && !arm.spec().5.is_empty()
        {
            bail!(
                "--arm {} reads oracle truth columns, but this corpus's tail \
                 is the `--auction` block (sidecar `auction: true`)",
                arm.name()
            );
        }
        let keep = arm.mask();
        // `mask_features`' zip stops at the corpus width, so a narrower corpus
        // is fine only when every live column actually exists in it.
        let needed = keep.iter().rposition(|&k| k).map_or(0, |last| last + 1);
        if train.features_len < needed {
            bail!(
                "--arm {} keeps columns up to {needed}; this corpus is only {} wide",
                arm.name(),
                train.features_len
            );
        }
        eprintln!(
            "arm {}: {} of {} columns live",
            arm.name(),
            keep.iter().filter(|&&k| k).count(),
            train.features_len,
        );
        train.mask_features(&keep);
        val.mask_features(&keep);
        live_in = keep.iter().filter(|&&k| k).count();
    }
    if args.collapse_side {
        train.collapse_side();
        val.collapse_side();
    }

    let in_dim = train.features_len;
    let targets = train.target_len;
    let out_dim = targets * HEADS;
    eprintln!(
        "train {} rows / val {} rows; {in_dim} features → {out_dim} outputs \
         ({targets} targets × {HEADS} heads){}",
        train.rows,
        val.rows,
        if args.blank_ranges {
            " [ranges blanked]"
        } else {
            ""
        },
    );

    let xtrain = train.features_tensor(&device)?;
    let ytrain = train.labels_tensor(&device)?;
    let xval = val.features_tensor(&device)?;
    let yval = val.labels_tensor(&device)?;

    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let model = Net::new(in_dim, args.hidden, out_dim, vb)?;
    seed_params(&varmap, args.seed, &device, in_dim, live_in)?;
    let mut opt = AdamW::new(
        varmap.all_vars(),
        ParamsAdamW {
            lr: args.lr,
            weight_decay: args.wd,
            ..Default::default()
        },
    )?;

    // The corpus is deal-major with ~20 rows per deal, and all rows of a deal
    // share one DD label vector. Walked in order, a nominal 4096-row batch holds
    // only ~200 distinct labels replicated ~20× — and the same ~200 every epoch.
    // The σ head is fit from the spread of residuals within a batch, so that is
    // exactly the head a fixed, label-degenerate batching destabilises. Shuffle.
    let mut perm: Vec<u32> = (0..train.rows as u32).collect();
    let mut rng = args.seed ^ 0x5EED_5EED_5EED_5EED;
    let mut best: Option<(f32, Vec<(String, Tensor)>)> = None;

    for epoch in 1..=args.epochs {
        // Fisher–Yates.
        for i in (1..perm.len()).rev() {
            perm.swap(i, (splitmix64(&mut rng) % (i as u64 + 1)) as usize);
        }
        // Cosine decay to ~0; the decayed tail is what lets the σ head settle.
        let progress = (epoch - 1) as f64 / args.epochs.max(1) as f64;
        opt.set_learning_rate(args.lr * 0.5 * (1.0 + (std::f64::consts::PI * progress).cos()));

        let (mut start, mut running, mut steps) = (0usize, 0f32, 0usize);
        while start < train.rows {
            let len = args.batch.min(train.rows - start);
            let idx = Tensor::from_slice(&perm[start..start + len], len, &device)?;
            let pred = model.forward(&xtrain.index_select(&idx, 0)?)?;
            let loss = gaussian_nll(&pred, &ytrain.index_select(&idx, 0)?, targets)?;
            opt.backward_step(&loss)?;
            running += loss.to_scalar::<f32>()?;
            steps += 1;
            start += len;
        }

        // Every epoch, so the checkpoint is the true best and not the best of a
        // 5-epoch stride; a val forward pass is cheap beside the training epoch.
        let e = evaluate(&model, &xval, &yval, targets, &val.tags)?;
        let nll = e.overall.mean_nll();
        if best.as_ref().is_none_or(|(b, _)| nll < *b) {
            best = Some((nll, snapshot(&varmap)?));
        }
        if epoch == 1 || epoch % 5 == 0 || epoch == args.epochs {
            eprintln!(
                "epoch {epoch:>4}: train {:.5}  val nll {:.5}  MAE {:.3}  RMSE {:.3} tricks  \
                 coverage {:.1}% (constructive {:.1}% / contested {:.1}%)  \
                 below-mu {:.1}%  slam-MAE {:.3}  suit-game-MAE {:.3}  nt-cont-MAE {:.3}",
                running / steps as f32,
                nll,
                e.overall.mae_tricks(),
                e.overall.rmse_tricks(),
                100.0 * e.overall.coverage(),
                100.0 * e.phase[0].coverage(),
                100.0 * e.phase[1].coverage(),
                100.0 * e.overall.below_mean(),
                e.slam.mae_tricks(),
                e.suit_game.mae_tricks(),
                e.nt_contested.mae_tricks(),
            );
        }
    }

    // Ship the best epoch, not whatever the last one happened to land on.
    if let Some((nll, params)) = best {
        let data = varmap.data().lock().expect("varmap poisoned");
        for (name, tensor) in &params {
            data[name].set(tensor)?;
        }
        eprintln!("restored best-val checkpoint: nll {nll:.5}");
    }
    let final_eval = evaluate(&model, &xval, &yval, targets, &val.tags)?;
    export(&args, &varmap, &model, &xval, &train, targets, &final_eval)?;
    Ok(())
}

// ── Model ─────────────────────────────────────────────────────────────────────

/// The evaluator: one linear layer, or two ReLU hidden layers. The export order
/// (`l1.weight, l1.bias, …`) is what the in-crate hand-rolled forward pass reads.
struct Net {
    layers: Vec<Linear>,
}

impl Net {
    fn new(in_dim: usize, hidden: usize, out_dim: usize, vb: VarBuilder) -> Result<Self> {
        let layers = if hidden == 0 {
            vec![candle_nn::linear(in_dim, out_dim, vb.pp("l1"))?]
        } else {
            vec![
                candle_nn::linear(in_dim, hidden, vb.pp("l1"))?,
                candle_nn::linear(hidden, hidden, vb.pp("l2"))?,
                candle_nn::linear(hidden, out_dim, vb.pp("l3"))?,
            ]
        };
        Ok(Self { layers })
    }

    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let mut h = x.clone();
        for (i, layer) in self.layers.iter().enumerate() {
            h = layer.forward(&h)?;
            if i + 1 < self.layers.len() {
                h = h.relu()?;
            }
        }
        Ok(h)
    }

    /// Parameter names in export order.
    fn param_names(&self) -> Vec<String> {
        (1..=self.layers.len())
            .flat_map(|i| [format!("l{i}.weight"), format!("l{i}.bias")])
            .collect()
    }
}

/// Mean Gaussian negative log-likelihood, dropping the constant `½·ln 2π`:
///
/// ```text
/// L(t; mu, s) = s + ½·(t − mu)²·exp(−2s)
/// ```
///
/// Its minimiser is exactly what the evaluator wants: `mu` converges to the
/// conditional **mean** of double-dummy tricks given the information state, and
/// σ to the conditional **standard deviation** — both from single-deal
/// supervision, because each row is one unbiased draw from the posterior over
/// hidden hands, so the population supplies the spread no single row carries.
///
/// `pred` is `(batch, targets × 2)` laid out head-major — all `mu`s, then all
/// `ln σ`s — so each head is one contiguous `narrow`.
fn gaussian_nll(pred: &Tensor, target: &Tensor, targets: usize) -> Result<Tensor> {
    let mu = pred.narrow(1, 0, targets)?;
    let ln_sd = pred
        .narrow(1, targets, targets)?
        .clamp(LN_SD_MIN, LN_SD_MAX)?;
    // ½·(t − mu)²·exp(−2s): squared error weighted by the predicted precision.
    // The net can only buy a cheaper residual by paying the `s` term for it,
    // which is what stops σ from collapsing everywhere it is inconvenient.
    let quad = ((target - &mu)?.sqr()?.affine(0.5, 0.0)? * ln_sd.affine(-2.0, 0.0)?.exp()?)?;
    Ok((ln_sd + quad)?.mean_all()?)
}

/// The standard normal's upper quartile, `Φ⁻¹(0.75)`. The band `mu ± Z75·σ` is
/// the model's central 50%, so its coverage stays directly comparable to the
/// interquartile coverage the old quantile heads reported.
const Z75: f64 = 0.674_490;

/// Metrics over one slice of the held-out set.
#[derive(Default, Clone, Copy)]
struct Slice {
    nll: f64,
    abs: f64,
    sq: f64,
    inside: u64,
    below: u64,
    n: u64,
}

impl Slice {
    fn push(&mut self, nll: f64, abs: f64, inside: bool, below: bool) {
        self.nll += nll;
        self.abs += abs;
        self.sq += abs * abs;
        self.inside += u64::from(inside);
        self.below += u64::from(below);
        self.n += 1;
    }

    fn mean_nll(self) -> f32 {
        (self.nll / self.n.max(1) as f64) as f32
    }

    /// Mean-head absolute error, rescaled from the corpus's `tricks / 13`.
    fn mae_tricks(self) -> f32 {
        (13.0 * self.abs / self.n.max(1) as f64) as f32
    }

    /// Mean-head root-mean-square error, rescaled from the corpus's `tricks / 13`.
    ///
    /// This — not [`Self::mae_tricks`] — is the metric the `μ` head optimises:
    /// squared error is minimised by the conditional *mean*, absolute error by
    /// the conditional *median*. Scoring a mean head on MAE hands a systematic
    /// advantage to any predictor that aims at the median instead, so the two
    /// are reported side by side.
    fn rmse_tricks(self) -> f32 {
        (13.0 * (self.sq / self.n.max(1) as f64).sqrt()) as f32
    }

    /// Fraction of labels inside the central-50% band `mu ± Z75·σ` — nominally
    /// 50%.
    fn coverage(self) -> f32 {
        self.inside as f32 / self.n.max(1) as f32
    }

    /// Fraction of labels strictly below `mu` — the skew diagnostic. For a
    /// symmetric distribution this is 50%; double-dummy trick counts on a good
    /// fit are left-skewed and hard-bounded at 13, so a systematic departure
    /// from 50% is the Gaussian's shape assumption failing to fit the data, not
    /// a bug.
    fn below_mean(self) -> f32 {
        self.below as f32 / self.n.max(1) as f32
    }
}

/// Render a pair of named slices for the sidecar.
fn slices(names: [&str; 2], slices: &[Slice; 2]) -> serde_json::Value {
    let rows: Vec<_> = names
        .iter()
        .zip(slices)
        .map(|(name, s)| {
            serde_json::json!({
                "slice": name,
                "nll": s.mean_nll(),
                "mae_tricks": s.mae_tricks(),
                "rmse_tricks": s.rmse_tricks(),
                "coverage": s.coverage(),
                "below_mean": s.below_mean(),
                "targets": s.n,
            })
        })
        .collect();
    serde_json::Value::Array(rows)
}

/// Held-out metrics, overall and sliced by the two tag bits.
struct Eval {
    overall: Slice,
    /// Tag bit 0: `[constructive, contested]`.
    phase: [Slice; 2],
    /// Tag bit 1: the corpus's system slot. Comparable numbers here are the
    /// evidence for the system-agnostic claim — the net reads ranges, never
    /// calls, so neither book should be systematically easier.
    system: [Slice; 2],
    /// Targets whose *truth* is a slam (≥ 12 of 13 tricks) — Phase 3's kill-gate
    /// metric. The whole point of partner-honour info is bidding slams, so an
    /// oracle that does not move MAE *here* has not earned the axis it bounds.
    slam: Slice,
    /// Suit-strain targets whose truth makes game or better (≥ 10 tricks) —
    /// the shortness axis's home turf: ruffing value prices suit games.
    suit_game: Slice,
    /// NT targets on contested rows whose truth makes game or better (≥ 9
    /// tricks) — where the quality and stopper axes would earn their keep.
    nt_contested: Slice,
}

fn evaluate(model: &Net, x: &Tensor, y: &Tensor, targets: usize, tags: &[u8]) -> Result<Eval> {
    let p = model.forward(x)?.to_vec2::<f32>()?;
    let t = y.to_vec2::<f32>()?;
    let mut eval = Eval {
        overall: Slice::default(),
        phase: [Slice::default(); 2],
        system: [Slice::default(); 2],
        slam: Slice::default(),
        suit_game: Slice::default(),
        nt_contested: Slice::default(),
    };

    for (row, (pr, tr)) in p.iter().zip(&t).enumerate() {
        let tag = tags.get(row).copied().unwrap_or(0);
        for j in 0..targets {
            // Score the clamped `ln σ`, which is what training optimised and
            // what serving will read.
            let mu = f64::from(pr[j]);
            let s = f64::from(pr[targets + j]).clamp(LN_SD_MIN, LN_SD_MAX);
            let sigma = s.exp();
            let truth = f64::from(tr[j]);
            let d = truth - mu;
            let nll = s + 0.5 * d * d * (-2.0 * s).exp();
            let abs = d.abs();
            let inside = abs <= Z75 * sigma;
            let below = truth < mu;
            eval.overall.push(nll, abs, inside, below);
            eval.phase[usize::from(tag & 1)].push(nll, abs, inside, below);
            eval.system[usize::from(tag >> 1 & 1)].push(nll, abs, inside, below);
            // `truth` is tricks/13; mid-gap thresholds (11.5, 9.5, 8.5) keep
            // each slice clear of float-rounding on its trick boundary. Labels
            // are strain-major NT,S,H,D,C × 4 declarers, so `j < 4` is NT.
            if truth * 13.0 >= 11.5 {
                eval.slam.push(nll, abs, inside, below);
            }
            if j >= 4 && truth * 13.0 >= 9.5 {
                eval.suit_game.push(nll, abs, inside, below);
            }
            if j < 4 && tag & 1 == 1 && truth * 13.0 >= 8.5 {
                eval.nt_contested.push(nll, abs, inside, below);
            }
        }
    }
    Ok(eval)
}

// ── Corpus ────────────────────────────────────────────────────────────────────

/// The `dump-evaluator` sidecar fields we depend on (serde ignores the rest).
#[derive(Debug, Deserialize)]
struct Meta {
    feature_version: u32,
    features_len: usize,
    dd_len: usize,
    row_len: usize,
    rows: u64,
    seed: u64,
    #[serde(default)]
    encoding: String,
    #[serde(default)]
    git_sha: String,
    #[serde(default)]
    systems: Vec<String>,
    #[serde(default)]
    deals: String,
    /// The corpus's tail past column 79 is the `--auction` block, not oracle
    /// truth (`dump-evaluator --auction`). Old sidecars default to `false`.
    #[serde(default)]
    auction: bool,
    /// Features were extracted with both box closures folded into the hulls
    /// (`dump-evaluator --closed-hulls`); the auctions themselves are bid
    /// knob-off. Old sidecars default to `false`.
    #[serde(default)]
    closed_hulls: bool,
}

struct Dataset {
    features: Vec<f32>,
    labels: Vec<f32>,
    tags: Vec<u8>,
    rows: usize,
    features_len: usize,
    target_len: usize,
    meta: Meta,
}

impl Dataset {
    fn load(stem: &str) -> Result<Self> {
        let json_path = format!("{stem}.json");
        let f32_path = format!("{stem}.f32");
        let meta: Meta = serde_json::from_slice(
            &std::fs::read(&json_path).with_context(|| format!("reading sidecar {json_path}"))?,
        )
        .with_context(|| format!("parsing sidecar {json_path}"))?;
        // Coarse gate, not a layout check: the dumper stamps the same tag on
        // every `--encoding`, so summary, onehot and bits corpora are
        // indistinguishable here. `features_len` and `meta.encoding` are what
        // actually identify the layout. v1 corpora on disk stay readable.
        if !matches!(meta.feature_version, 1..=5) {
            bail!(
                "evaluator feature_version {} unsupported (this trainer knows 1 to 5)",
                meta.feature_version
            );
        }
        if meta.dd_len != DD_LEN || meta.row_len != meta.features_len + DD_LEN {
            bail!(
                "row layout mismatch: features_len {} + dd_len {} != row_len {}",
                meta.features_len,
                meta.dd_len,
                meta.row_len
            );
        }

        // Streamed a row at a time rather than `fs::read` into one `Vec<u8>`:
        // the split below already materialises the whole corpus as f32, so
        // slurping the bytes first costs a second full copy of it at peak. At
        // 100k deals that was 481 MB and nobody noticed; a 1M-deal `bits`
        // corpus is 7.9 GB, and this box is shared.
        let file = std::fs::File::open(&f32_path).with_context(|| format!("opening {f32_path}"))?;
        let row_bytes = meta.row_len * 4;
        let len = usize::try_from(file.metadata()?.len())?;
        if len % row_bytes != 0 {
            bail!("{f32_path} is not a whole number of {row_bytes}-byte rows");
        }
        let rows = len / row_bytes;
        if rows as u64 != meta.rows {
            bail!("{f32_path} has {rows} rows, sidecar says {}", meta.rows);
        }

        let features_len = meta.features_len;
        let mut features = Vec::with_capacity(rows * features_len);
        let mut labels = Vec::with_capacity(rows * DD_LEN);
        let mut reader = BufReader::new(file);
        let mut row = vec![0u8; row_bytes];
        for _ in 0..rows {
            reader.read_exact(&mut row)?;
            let mut floats = row
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]));
            features.extend((&mut floats).take(features_len));
            labels.extend(floats);
        }

        let tags_path = format!("{stem}.tags");
        let tags = std::fs::read(&tags_path).with_context(|| format!("reading {tags_path}"))?;
        if tags.len() != rows {
            bail!("{tags_path} has {} tags but {rows} rows", tags.len());
        }

        Ok(Self {
            features,
            labels,
            tags,
            rows,
            features_len,
            target_len: DD_LEN,
            meta,
        })
    }

    /// Split the trailing `rows - at` rows off into a second dataset. The dump
    /// is deal-major, so a contiguous tail never shares a board — and hence
    /// never shares a DD label — with what stays behind.
    fn split_off(&mut self, at: usize) -> Self {
        let tail = Self {
            features: self.features.split_off(at * self.features_len),
            labels: self.labels.split_off(at * self.target_len),
            tags: self.tags.split_off(at),
            rows: self.rows - at,
            features_len: self.features_len,
            target_len: self.target_len,
            meta: Meta {
                feature_version: self.meta.feature_version,
                features_len: self.features_len,
                dd_len: DD_LEN,
                row_len: self.meta.row_len,
                rows: (self.rows - at) as u64,
                seed: self.meta.seed,
                encoding: self.meta.encoding.clone(),
                git_sha: self.meta.git_sha.clone(),
                systems: self.meta.systems.clone(),
                deals: self.meta.deals.clone(),
                auction: self.meta.auction,
                closed_hulls: self.meta.closed_hulls,
            },
        };
        self.rows = at;
        tail
    }

    /// Ablation: pretend nothing was ever bid. The unknown *encoding* is
    /// `[0, 1]` per bound pair, not zeros — zeros would be a hand with no cards.
    fn blank_ranges(&mut self) {
        let start = self.features_len - LEN_RANGES;
        for row in self.features.chunks_exact_mut(self.features_len) {
            for pair in row[start..].chunks_exact_mut(2) {
                pair.copy_from_slice(&UNKNOWN_PAIR);
            }
        }
    }

    /// Ablation: zero every column outside `keep`, which for the first linear
    /// layer is the same as deleting it — see [`Arm`]. The zip stops at the
    /// row width, so a corpus narrower than [`ARM_FEATURES`] is safe here;
    /// callers must have checked it still reaches every *live* column.
    fn mask_features(&mut self, keep: &[bool; ARM_FEATURES]) {
        for row in self.features.chunks_exact_mut(self.features_len) {
            for (x, &k) in row.iter_mut().zip(keep) {
                if !k {
                    *x = 0.0;
                }
            }
        }
    }

    /// Ablation: 20 per-declarer targets → 10 per-side ones, keeping the better
    /// declarer of each side (right-siding stops being visible).
    fn collapse_side(&mut self) {
        let mut folded = Vec::with_capacity(self.rows * DD_LEN / 2);
        for row in self.labels.chunks_exact(DD_LEN) {
            // Each strain contributes [me, lho, partner, rho].
            for strain in row.chunks_exact(4) {
                folded.push(strain[0].max(strain[2]));
                folded.push(strain[1].max(strain[3]));
            }
        }
        self.labels = folded;
        self.target_len = DD_LEN / 2;
    }

    fn features_tensor(&self, device: &Device) -> Result<Tensor> {
        Ok(Tensor::from_slice(
            &self.features,
            (self.rows, self.features_len),
            device,
        )?)
    }

    fn labels_tensor(&self, device: &Device) -> Result<Tensor> {
        Ok(Tensor::from_slice(
            &self.labels,
            (self.rows, self.target_len),
            device,
        )?)
    }
}

// ── Export ────────────────────────────────────────────────────────────────────

fn export(
    args: &Args,
    varmap: &VarMap,
    model: &Net,
    xval: &Tensor,
    ds: &Dataset,
    targets: usize,
    eval: &Eval,
) -> Result<()> {
    let stem = &args.weights_out;
    if let Some(parent) = Path::new(stem).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let names = model.param_names();
    let f32_path = format!("{stem}.f32");
    let mut w = BufWriter::new(std::fs::File::create(&f32_path)?);
    let mut shapes = serde_json::Map::new();
    let mut total = 0usize;
    {
        let data = varmap.data().lock().expect("varmap mutex poisoned");
        for name in &names {
            let var = data
                .get(name)
                .with_context(|| format!("missing param {name}"))?;
            shapes.insert(name.clone(), serde_json::json!(var.dims()));
            for x in var.flatten_all()?.to_vec1::<f32>()? {
                w.write_all(&x.to_le_bytes())?;
                total += 1;
            }
        }
    }
    w.flush()?;

    let out_dim = targets * HEADS;
    let sidecar = serde_json::json!({
        "trainer": "pons-trainer evaluator",
        "feature_version": ds.meta.feature_version,
        "features_len": ds.features_len,
        "encoding": ds.meta.encoding,
        "targets": targets,
        "heads": ["mu", "ln_sd"],
        "ln_sd_clamp": [LN_SD_MIN, LN_SD_MAX],
        "out_dim": out_dim,
        "out_layout": format!("[{targets} × mu][{targets} × ln_sd], tricks / 13"),
        "label_order": "strain-major NT,S,H,D,C × declarer [me,lho,partner,rho]",
        "hidden": args.hidden,
        "arch": if args.hidden == 0 {
            format!("x -> Linear({}, {out_dim})", ds.features_len)
        } else {
            format!(
                "x -> Linear({}, {h}) -> relu -> Linear({h}, {h}) -> relu -> Linear({h}, {out_dim})",
                ds.features_len,
                h = args.hidden
            )
        },
        "param_order": names,
        "param_shapes": shapes,
        "param_floats": total,
        "dtype": "f32-le",
        "blank_ranges": args.blank_ranges,
        "collapse_side": args.collapse_side,
        "arm": args.arm.map(Arm::name),
        "data_deals": ds.meta.deals,
        "data_systems": ds.meta.systems,
        "data_git_sha": ds.meta.git_sha,
        "data_seed": ds.meta.seed,
        "data_closed_hulls": ds.meta.closed_hulls,
        "train_rows": ds.rows,
        "test": args.test,
        "epochs": args.epochs,
        "lr": args.lr,
        "wd": args.wd,
        "batch": args.batch,
        "seed": args.seed,
        "git_sha": git_sha(),
        "val_nll": eval.overall.mean_nll(),
        "val_mae_tricks": eval.overall.mae_tricks(),
        "val_rmse_tricks": eval.overall.rmse_tricks(),
        "val_coverage": eval.overall.coverage(),
        "val_below_mean": eval.overall.below_mean(),
        "val_slam_mae_tricks": eval.slam.mae_tricks(),
        "val_slam_rmse_tricks": eval.slam.rmse_tricks(),
        "val_slam_targets": eval.slam.n,
        "val_suit_game_mae_tricks": eval.suit_game.mae_tricks(),
        "val_suit_game_rmse_tricks": eval.suit_game.rmse_tricks(),
        "val_suit_game_targets": eval.suit_game.n,
        "val_nt_contested_mae_tricks": eval.nt_contested.mae_tricks(),
        "val_nt_contested_rmse_tricks": eval.nt_contested.rmse_tricks(),
        "val_nt_contested_targets": eval.nt_contested.n,
        "val_by_phase": slices(["constructive", "contested"], &eval.phase),
        "val_by_system": slices(
            [
                ds.meta.systems.first().map_or("system 0", String::as_str),
                ds.meta.systems.get(1).map_or("system 1", String::as_str),
            ],
            &eval.system,
        ),
    });
    std::fs::write(format!("{stem}.json"), format!("{sidecar:#}\n"))?;

    let k = args.fixture.min(xval.dim(0)?);
    if k > 0 {
        let xf = xval.narrow(0, 0, k)?;
        let fixture = serde_json::json!({
            "note": "Parity: the in-crate hand-rolled forward pass must reproduce \
                     these outputs from these features (within tolerance).",
            "feature_version": ds.meta.feature_version,
            "rows": k,
            "features": xf.to_vec2::<f32>()?,
            "outputs": model.forward(&xf)?.to_vec2::<f32>()?,
        });
        std::fs::write(format!("{stem}.fixture.json"), format!("{fixture:#}\n"))?;
    }

    eprintln!("exported {total} floats -> {f32_path} (+ .json, .fixture.json)");
    eprintln!(
        "final val: nll {:.5}  MAE {:.3}  RMSE {:.3} tricks  coverage {:.1}%  below-mu {:.1}%",
        eval.overall.mean_nll(),
        eval.overall.mae_tricks(),
        eval.overall.rmse_tricks(),
        100.0 * eval.overall.coverage(),
        100.0 * eval.overall.below_mean(),
    );
    let row = |name: &str, s: Slice| {
        eprintln!(
            "  {name:<14} nll {:.5}  MAE {:.3}  RMSE {:.3}  coverage {:.1}%  \
             below-mu {:.1}%  ({} targets)",
            s.mean_nll(),
            s.mae_tricks(),
            s.rmse_tricks(),
            100.0 * s.coverage(),
            100.0 * s.below_mean(),
            s.n,
        );
    };
    row("constructive", eval.phase[0]);
    row("contested", eval.phase[1]);
    row("slam", eval.slam);
    row("suit-game", eval.suit_game);
    row("nt-contested", eval.nt_contested);
    for (i, s) in eval.system.iter().enumerate() {
        if s.n > 0 {
            row(ds.meta.systems.get(i).map_or("system?", String::as_str), *s);
        }
    }
    Ok(())
}

/// Best-effort current commit for the sidecar; `"unknown"` on failure.
fn git_sha() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map_or_else(|| "unknown".to_string(), |s| s.trim().to_string())
}

#[cfg(test)]
mod tests;
