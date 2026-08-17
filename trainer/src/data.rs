//! Loader for the teacher dump produced by `examples/teacher-dump`.
//!
//! The dump is a flat little-endian `f32` file of `ROW_LEN`-float rows
//! (`[160 features][38 teacher_softmax]`) plus a JSON sidecar pinning the
//! feature version, seed, and counts, and a sibling `.tags` file of one `u8`
//! per row (`1` = contested-phase decision, `0` = constructive). The constants
//! below mirror `pons::bidding::features` and `bidding::array`; they are
//! asserted against the sidecar so a layout/version drift fails loudly here
//! rather than silently training on garbage.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::Path;

/// Softmax width = number of distinct calls (`bidding::array::CALL_VARIANTS`).
pub const SOFTMAX_LEN: usize = 38;
/// Feature-spec versions this trainer understands
/// (`pons::bidding::features`): v1 is the 160-float vector, v2 adds the tag
/// block, v3 is the restrictive disclosable-only vector (88 floats), v4 appends
/// the two 140-wide convention cards (368 floats), v5 replaces the cards with
/// the two 28-wide compact-config blocks (144 floats), and v6 widens each
/// reading with four suit-specific support-point ranges (176 floats). The actual
/// `features_len` is read from the dump sidecar and the model input is sized
/// from it, so every supported version trains unchanged.
pub const SUPPORTED_FEATURE_VERSIONS: [u32; 6] = [1, 2, 3, 4, 5, 6];

/// Fields of the teacher-dump JSON sidecar that we care about (serde ignores
/// the rest).
#[derive(Debug, Clone, Deserialize)]
pub struct Meta {
    pub feature_version: u32,
    pub features_len: usize,
    pub softmax_len: usize,
    /// Trailing double-dummy regression target per row (20 when the dump was
    /// fed a GIB file, 0 otherwise). `#[serde(default)]` keeps pre-DD dumps loadable.
    #[serde(default)]
    pub dd_len: usize,
    pub row_len: usize,
    pub seed: u64,
    pub rows: u64,
    pub contested_rows: u64,
    #[serde(default)]
    pub git_sha: String,
    #[serde(default)]
    pub teacher: String,
    /// Convention card the teacher was configured from (`dump-teacher --card`),
    /// empty when it ran on the engine's compiled-in defaults.  Carried into the
    /// weights sidecar: `teacher: "bba"` alone does not pin a system, and a net
    /// distilled from the wrong card is silently the wrong net.
    #[serde(default)]
    pub card: String,
    /// Single conventions forced on top of the card or the engine defaults
    /// (`dump-teacher --conv "Kickback 1430=1"`).  A card pins every convention
    /// at once and so cannot express "defaults, except this one"; without this
    /// field a twin distilled from a kickback-playing teacher is
    /// indistinguishable from the plain net, because the corpus that would say
    /// so is deliberately never committed.
    #[serde(default)]
    pub conv: Vec<String>,
    /// Whether *our* extractor read the auctions with kickback armed
    /// (`dump-teacher --kickback`).  Forty of the eighty-eight v3 features come
    /// from `Inferences::read`, so this belongs to the feature spec rather than
    /// to the teacher: the same rows mean different things without it.
    #[serde(default)]
    pub our_kickback: bool,
    /// The corpus alternates the kickback regime per board
    /// (`dump-teacher --mix-kickback`), so one net covers both systems and
    /// tells them apart from the readings rather than from a knob.  Without
    /// this the mixed artifact is indistinguishable from the single-regime
    /// twin — same `conv`, same everything but row count.
    #[serde(default)]
    pub mix_kickback: bool,
}

/// A loaded teacher dataset, rows still in dump order (board-by-board).
pub struct Dataset {
    /// `rows * features_len` floats, row-major.
    pub features: Vec<f32>,
    /// `rows * SOFTMAX_LEN` floats, row-major (teacher softmax target).
    pub targets: Vec<f32>,
    /// `rows * dd_len` floats, row-major (per-row double-dummy regression
    /// target; empty when `dd_len == 0`).
    pub dd: Vec<f32>,
    /// One tag per row: `1` = contested phase, `0` = constructive.
    pub tags: Vec<u8>,
    pub rows: usize,
    /// Feature-vector length for this dump, read from the sidecar (160 for v1).
    pub features_len: usize,
    /// Double-dummy target width per row (20, or 0 when absent).
    pub dd_len: usize,
    pub meta: Meta,
}

impl Dataset {
    /// Load `<stem>.f32`, `<stem>.json`, and (optionally) `<stem>.tags`.
    pub fn load(stem: &str) -> Result<Self> {
        let json_path = format!("{stem}.json");
        let f32_path = format!("{stem}.f32");
        let tags_path = format!("{stem}.tags");

        let meta: Meta = serde_json::from_slice(
            &std::fs::read(&json_path).with_context(|| format!("reading sidecar {json_path}"))?,
        )
        .with_context(|| format!("parsing sidecar {json_path}"))?;

        // Accept any known feature version; size everything from the sidecar so a
        // v1 and a v2 dump both load. Only the softmax width is fixed (the call
        // set), and the row layout must be internally consistent.
        if !SUPPORTED_FEATURE_VERSIONS.contains(&meta.feature_version) {
            bail!(
                "feature_version {} unsupported; this trainer understands {SUPPORTED_FEATURE_VERSIONS:?} \
                 (bump together with pons::bidding::features)",
                meta.feature_version
            );
        }
        let features_len = meta.features_len;
        if meta.softmax_len != SOFTMAX_LEN {
            bail!(
                "softmax_len mismatch: dump {}, trainer expects {SOFTMAX_LEN}",
                meta.softmax_len
            );
        }
        let dd_len = meta.dd_len;
        let row_len = features_len + SOFTMAX_LEN + dd_len;
        if meta.row_len != row_len {
            bail!(
                "row_len mismatch: dump {} but features_len {features_len} + softmax_len {SOFTMAX_LEN} + dd_len {dd_len} = {row_len}",
                meta.row_len
            );
        }

        let bytes = std::fs::read(&f32_path).with_context(|| format!("reading {f32_path}"))?;
        let row_bytes = row_len * 4;
        if bytes.len() % row_bytes != 0 {
            bail!(
                "{f32_path} length {} is not a multiple of row size {row_bytes}",
                bytes.len()
            );
        }
        let rows = bytes.len() / row_bytes;
        if rows as u64 != meta.rows {
            bail!(
                "row count mismatch: {f32_path} has {rows}, sidecar says {}",
                meta.rows
            );
        }

        let mut features = Vec::with_capacity(rows * features_len);
        let mut targets = Vec::with_capacity(rows * SOFTMAX_LEN);
        let mut dd = Vec::with_capacity(rows * dd_len);
        for row in bytes.chunks_exact(row_bytes) {
            let mut floats = row
                .as_chunks::<4>()
                .0
                .iter()
                .copied()
                .map(f32::from_le_bytes);
            features.extend((&mut floats).take(features_len));
            targets.extend((&mut floats).take(SOFTMAX_LEN));
            dd.extend(floats);
        }

        let tags = load_tags(&tags_path, rows)?;

        Ok(Self {
            features,
            targets,
            dd,
            tags,
            rows,
            features_len,
            dd_len,
            meta,
        })
    }
}

/// Load one or more dumps as a single dataset, laid out `[train…][val…]`, and
/// report where the split falls.
///
/// A mixture corpus is two dumps with deliberately different distributions — a
/// uniform calibration bulk and an enriched slice
/// (`docs/ai-bidder/configured-net.md`) — so neither the split nor the batch
/// order may treat "dump 2" as "the end of the data":
///
/// - **Validation is each dump's own contiguous tail**, concatenated. Taking
///   one tail off the concatenation would make the held-out set *entirely*
///   enriched, which measures the wrong distribution; taking it at random would
///   put rows from one board on both sides of the split, which measures nothing
///   (the trainer's split is board-disjoint precisely because rows from a board
///   are near-duplicates).
/// - **Training heads are round-robined in `block`-row chunks**, proportionally,
///   because the epoch loop walks contiguous minibatches and never shuffles.
///   Concatenated, every epoch would run the bulk to exhaustion and then finish
///   on a long tail of nothing but enriched slam auctions.
///
/// One dump round-robins with itself, i.e. is returned unchanged, so this is
/// the single code path.
pub fn load_mixture(stems: &[String], val_frac: f64, block: usize) -> Result<(Dataset, usize)> {
    let [first, rest @ ..] = stems else {
        bail!("no --data given");
    };
    let dumps = std::iter::once(first)
        .chain(rest)
        .map(|stem| Dataset::load(stem))
        .collect::<Result<Vec<_>>>()?;

    // A mixture is only a mixture if the rows are commensurable (same layout and meaning).
    let head = &dumps[0];
    for (stem, d) in stems.iter().zip(&dumps).skip(1) {
        if (d.meta.feature_version, d.features_len, d.dd_len)
            != (head.meta.feature_version, head.features_len, head.dd_len)
        {
            bail!(
                "dump {stem} is feature v{} ({} features, dd {}) but {} is v{} ({} features, dd {})",
                d.meta.feature_version,
                d.features_len,
                d.dd_len,
                stems[0],
                head.meta.feature_version,
                head.features_len,
                head.dd_len
            );
        }
        if (
            &d.meta.teacher,
            &d.meta.card,
            &d.meta.conv,
            d.meta.our_kickback,
            d.meta.mix_kickback,
            &d.meta.git_sha,
        ) != (
            &head.meta.teacher,
            &head.meta.card,
            &head.meta.conv,
            head.meta.our_kickback,
            head.meta.mix_kickback,
            &head.meta.git_sha,
        ) {
            bail!(
                "dump {stem} metadata mismatch vs {} (teacher/card/conv/kickback/git_sha)",
                stems[0]
            );
        }
    }

    // Each dump's own board-disjoint tail is its validation share.
    let ntrain_each: Vec<usize> = dumps
        .iter()
        .map(|d| {
            let nval =
                ((d.rows as f64 * val_frac).round() as usize).clamp(1, d.rows.saturating_sub(1));
            d.rows - nval
        })
        .collect();

    let (features_len, dd_len) = (dumps[0].features_len, dumps[0].dd_len);
    let rows: usize = dumps.iter().map(|d| d.rows).sum();
    let ntrain: usize = ntrain_each.iter().sum();

    let mut out = Dataset {
        features: Vec::with_capacity(rows * features_len),
        targets: Vec::with_capacity(rows * SOFTMAX_LEN),
        dd: Vec::with_capacity(rows * dd_len),
        tags: Vec::with_capacity(rows),
        rows,
        features_len,
        dd_len,
        meta: Meta {
            rows: rows as u64,
            contested_rows: dumps.iter().map(|d| d.meta.contested_rows).sum(),
            ..dumps[0].meta.clone()
        },
    };

    // Round-robin the heads by *progress*, so a dump 4× the size contributes 4×
    // the blocks and the two run out together.
    let mut cursor = vec![0usize; dumps.len()];
    let block = block.max(1);
    while cursor.iter().zip(&ntrain_each).any(|(&at, &n)| at < n) {
        let pick = (0..dumps.len())
            .filter(|&i| cursor[i] < ntrain_each[i])
            .min_by(|&a, &b| {
                let progress = |i: usize| cursor[i] as f64 / ntrain_each[i].max(1) as f64;
                progress(a).total_cmp(&progress(b))
            })
            .expect("the while condition guarantees one dump still has rows");
        let take = block.min(ntrain_each[pick] - cursor[pick]);
        out.push_rows(&dumps[pick], cursor[pick], take);
        cursor[pick] += take;
    }
    // Then every dump's held-out tail, in order.
    for (d, &head) in dumps.iter().zip(&ntrain_each) {
        out.push_rows(d, head, d.rows - head);
    }

    Ok((out, ntrain))
}

impl Dataset {
    /// Append rows `[from, from + n)` of `src` — every parallel array at once, so
    /// features, targets, DD and tags cannot drift out of correspondence.
    fn push_rows(&mut self, src: &Dataset, from: usize, n: usize) {
        let w = self.features_len;
        self.features
            .extend_from_slice(&src.features[from * w..(from + n) * w]);
        self.targets
            .extend_from_slice(&src.targets[from * SOFTMAX_LEN..(from + n) * SOFTMAX_LEN]);
        if self.dd_len > 0 {
            let d = self.dd_len;
            self.dd.extend_from_slice(&src.dd[from * d..(from + n) * d]);
        }
        self.tags.extend_from_slice(&src.tags[from..from + n]);
    }
}

/// Read the per-row tag file, or fall back to all-zero (with a warning) if the
/// dump predates the `.tags` sibling.
fn load_tags(path: &str, rows: usize) -> Result<Vec<u8>> {
    if !Path::new(path).exists() {
        eprintln!(
            "warning: {path} missing; per-row constructive/contested split unavailable \
             (regenerate the dump to emit it). Reporting overall agreement only."
        );
        return Ok(vec![0u8; rows]);
    }
    let tags = std::fs::read(path).with_context(|| format!("reading {path}"))?;
    if tags.len() != rows {
        bail!("{path} has {} tags but {rows} rows", tags.len());
    }
    Ok(tags)
}
