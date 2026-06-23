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
/// block, v3 is the restrictive disclosable-only vector (88 floats). The actual
/// `features_len` is read from the dump sidecar and the model input is sized
/// from it, so every supported version trains unchanged.
pub const SUPPORTED_FEATURE_VERSIONS: [u32; 3] = [1, 2, 3];

/// Fields of the teacher-dump JSON sidecar that we care about (serde ignores
/// the rest).
#[derive(Debug, Deserialize)]
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
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]));
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
