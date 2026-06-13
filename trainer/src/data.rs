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

/// Feature-vector length (`pons::bidding::features::FEATURES_LEN`).
pub const FEATURES_LEN: usize = 160;
/// Softmax width = number of distinct calls (`bidding::array::CALL_VARIANTS`).
pub const SOFTMAX_LEN: usize = 38;
/// Floats per training row.
pub const ROW_LEN: usize = FEATURES_LEN + SOFTMAX_LEN;
/// Feature-spec version this trainer is written against
/// (`pons::bidding::features::FEATURES_VERSION`). Bump together.
pub const EXPECTED_FEATURE_VERSION: u32 = 1;

/// Fields of the teacher-dump JSON sidecar that we care about (serde ignores
/// the rest).
#[derive(Debug, Deserialize)]
pub struct Meta {
    pub feature_version: u32,
    pub features_len: usize,
    pub softmax_len: usize,
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
    /// `rows * FEATURES_LEN` floats, row-major.
    pub features: Vec<f32>,
    /// `rows * SOFTMAX_LEN` floats, row-major (teacher softmax target).
    pub targets: Vec<f32>,
    /// One tag per row: `1` = contested phase, `0` = constructive.
    pub tags: Vec<u8>,
    pub rows: usize,
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

        // Fail loudly on any layout/version drift between dump and trainer.
        if meta.feature_version != EXPECTED_FEATURE_VERSION {
            bail!(
                "feature_version mismatch: dump is {}, trainer expects {EXPECTED_FEATURE_VERSION} \
                 (bump the trainer constants together with pons::bidding::features)",
                meta.feature_version
            );
        }
        if meta.features_len != FEATURES_LEN || meta.softmax_len != SOFTMAX_LEN {
            bail!(
                "layout mismatch: dump features_len/softmax_len = {}/{}, trainer expects {FEATURES_LEN}/{SOFTMAX_LEN}",
                meta.features_len,
                meta.softmax_len
            );
        }
        if meta.row_len != ROW_LEN {
            bail!("row_len mismatch: dump {}, trainer {ROW_LEN}", meta.row_len);
        }

        let bytes = std::fs::read(&f32_path).with_context(|| format!("reading {f32_path}"))?;
        let row_bytes = ROW_LEN * 4;
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

        let mut features = Vec::with_capacity(rows * FEATURES_LEN);
        let mut targets = Vec::with_capacity(rows * SOFTMAX_LEN);
        for row in bytes.chunks_exact(row_bytes) {
            let mut floats = row
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]));
            features.extend((&mut floats).take(FEATURES_LEN));
            targets.extend(floats);
        }

        let tags = load_tags(&tags_path, rows)?;

        Ok(Self {
            features,
            targets,
            tags,
            rows,
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
