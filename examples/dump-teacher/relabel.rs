//! The `M`-series relabel (`docs/ai-bidder/logit-calibration.md` §4d, §6):
//! price every **net-served** decision of the corpus walk by rollout, store
//! the raw per-layout returns beside the rows, and cut labels from them later.
//!
//! Two phases, one binary:
//!
//! * **Write** (`dump-teacher --relabel`): during the walk, each row whose node
//!   the floor shell's net answers (unauthored, contested, not a `forced`
//!   rail) and whose proposal offers a live alternative to BBA's call is
//!   harvested as a [`Decision`].  After the walk, every decision draws
//!   `--layouts` worlds from the replay sampler, one double-dummy solve per
//!   world is shared across its candidates, and each candidate's swing over
//!   the own call is stored — `[candidate][layout] -> (plain DD, PD)` in IMPs —
//!   in a `.ret` sibling.  **No label is cut here**, which is what lets `M` be
//!   chosen after the double dummy is spent.
//! * **Cut** (`dump-teacher --cut M`): read every chunk, take pools `[0, M)`
//!   and `[M, 2M)`, select on the first, validate on the second, and overwrite
//!   the row's one-hot when the same call clears the margin on both scorers
//!   (§4c's production gate).  Emits the trainer's `.f32`/`.tags`/`.json`.
//!
//! **Partition invariance.** A chunk is a bank window; its rows and returns
//! depend only on the seed and the bank index, never on how the window was
//! split, so chunks written on different boxes concatenate into the corpus one
//! box would have written.  **Extension.** The sampler's accepted sequence is
//! a prefix-stable function of its stream, so a chunk stored at `L` layouts is
//! extended to `L' > L` by re-drawing, solving only `[L, L')`, and appending —
//! no solve is repeated, and a cut at any `2M ≤ L` is byte-identical before
//! and after.

use anyhow::{Context as _, bail, ensure};
use contract_bridge::auction::Call;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat};
use ddss::{NonEmptyStrainFlags, Solver, TrickCountTable};
use pons::bidding::Partnership;
use pons::bidding::array::Logits;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use crate::SOFTMAX_LEN;
use crate::common::rollout::{sample_for, swings};

/// One net-served decision harvested during the walk, priced after it
pub struct Decision {
    /// Row index within this dump
    pub row: u32,
    /// Bank deal index — the partition-invariant seed of this decision's draw
    pub deal_index: u64,
    /// Ordinal of this row within its deal, across every table it was bid at
    pub ordinal: u32,
    pub hand: Hand,
    pub seat: Seat,
    pub dealer: Seat,
    pub vul: AbsoluteVulnerability,
    pub prefix: Vec<Call>,
    /// Own call (the teacher's label) first, then the alternatives
    pub candidates: Vec<Call>,
    /// `per_side` labels of the acting side and its opponents
    pub ours: String,
    pub theirs: String,
}

/// The stored returns of one decision, as they sit in the `.ret` file
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Priced {
    pub row: u32,
    /// Call indices, own first
    pub candidates: Vec<u8>,
    /// Layouts drawn (the sampler may starve below the request)
    pub layouts: u16,
    /// `[candidate - 1][layout]` swings over the own call, `[plain DD, PD]`
    /// in IMPs — the own call's row is identically zero and not stored
    pub swings: Vec<[i8; 2]>,
}

/// The write mode's knobs, from the command line
#[derive(Clone, Copy)]
pub struct Knobs {
    pub layouts: usize,
    pub top_k: usize,
    pub epsilon: f32,
    pub temperature: f32,
}

const RET_MAGIC: &[u8; 4] = b"PRET";
const RET_VERSION: u32 = 1;

/// The `Logits` order, so a call round-trips through one byte
fn calls() -> Vec<Call> {
    Logits::new().iter().map(|(call, _)| call).collect()
}

fn call_index(call: Call) -> u8 {
    let index = calls()
        .iter()
        .position(|&c| c == call)
        .expect("every call has a Logits slot");
    u8::try_from(index).expect("38 slots")
}

/// Per-decision layout stream: a function of the seed and the decision's bank
/// address only.  The multiplier spreads shard seeds far apart, so two shards'
/// `(seed, deal, ordinal)` triples cannot alias by a small XOR.
pub fn layout_seed(seed: u64, deal_index: u64, ordinal: u32) -> u64 {
    seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((deal_index << 16) | u64::from(ordinal))
}

/// Per-board stream (dealer, vulnerability, a random board when there is no
/// bank): the ordinal slot no decision reaches
pub fn board_seed(seed: u64, deal_index: u64) -> u64 {
    layout_seed(seed, deal_index, 0xFFFF)
}

/// Price every decision: draw, solve once per new layout, bid out every
/// candidate, and return the stored form.  `existing` (an earlier `.ret` of
/// this same chunk) is extended rather than recomputed.
pub fn price<'p>(
    decisions: &[Decision],
    side: impl Fn(&str) -> &'p Partnership + Sync,
    knobs: Knobs,
    seed: u64,
    existing: Option<Vec<Priced>>,
) -> anyhow::Result<(Vec<Priced>, usize)> {
    let mut priced: Vec<Priced> = match existing {
        Some(existing) => {
            ensure!(
                existing.len() == decisions.len()
                    && existing.iter().zip(decisions).all(|(p, d)| {
                        p.row == d.row
                            && p.candidates.len() == d.candidates.len()
                            && p.candidates
                                .iter()
                                .zip(&d.candidates)
                                .all(|(&i, &c)| i == call_index(c))
                    }),
                "the existing .ret was written by a different walk (other seed, window, \
                 or commit); delete it to redo the chunk"
            );
            existing
        }
        None => decisions
            .iter()
            .map(|d| Priced {
                row: d.row,
                candidates: d.candidates.iter().map(|&c| call_index(c)).collect(),
                layouts: 0,
                swings: Vec::new(),
            })
            .collect(),
    };

    // Draw. Pure bidding, rayon's; the stored prefix is re-drawn and dropped.
    let fresh: Vec<Vec<FullDeal>> = decisions
        .par_iter()
        .zip(&priced)
        .map(|(d, p)| {
            let drawn = sample_for(
                d.hand,
                d.seat,
                side(&d.ours),
                d.vul,
                &d.prefix,
                knobs.layouts,
                layout_seed(seed, d.deal_index, d.ordinal),
            );
            drawn
                .get(usize::from(p.layouts)..)
                .map_or(Vec::new(), <[_]>::to_vec)
        })
        .collect();

    // One solve per new layout, on this thread: the solver owns the core pool.
    let all: Vec<FullDeal> = fresh.iter().flatten().copied().collect();
    let tables = Solver::lock(None).solve_deals(&all, NonEmptyStrainFlags::ALL);
    let mut offsets = Vec::with_capacity(fresh.len());
    let mut at = 0usize;
    for layouts in &fresh {
        offsets.push(at);
        at += layouts.len();
    }

    // Bid out and price, rayon again.
    let new_swings: Vec<Vec<Vec<[i64; 2]>>> = decisions
        .par_iter()
        .enumerate()
        .map(|(i, d)| {
            let layouts = &fresh[i];
            if layouts.is_empty() {
                return Vec::new();
            }
            let tables: &[TrickCountTable] = &tables[offsets[i]..offsets[i] + layouts.len()];
            swings(
                &d.candidates,
                &d.prefix,
                d.dealer,
                d.seat,
                layouts,
                tables,
                side(&d.ours),
                side(&d.theirs),
                d.vul,
            )
        })
        .collect();

    let mut extended = 0usize;
    for (p, new) in priced.iter_mut().zip(new_swings) {
        if new.is_empty() {
            continue;
        }
        extended += 1;
        let old = usize::from(p.layouts);
        let added = new[0].len();
        let alternatives = p.candidates.len() - 1;
        let mut merged = Vec::with_capacity(alternatives * (old + added));
        for (c, candidate) in new.iter().enumerate().skip(1) {
            merged.extend_from_slice(&p.swings[(c - 1) * old..c * old]);
            merged.extend(candidate.iter().map(|s| {
                [
                    i8::try_from(s[0]).expect("IMPs fit i8"),
                    i8::try_from(s[1]).expect("IMPs fit i8"),
                ]
            }));
        }
        p.swings = merged;
        p.layouts = u16::try_from(old + added).expect("layouts fit u16");
    }
    Ok((priced, extended))
}

pub fn write_ret(path: &Path, priced: &[Priced]) -> anyhow::Result<()> {
    let mut w = BufWriter::new(std::fs::File::create(path)?);
    w.write_all(RET_MAGIC)?;
    w.write_all(&RET_VERSION.to_le_bytes())?;
    w.write_all(&u32::try_from(priced.len())?.to_le_bytes())?;
    for p in priced {
        w.write_all(&p.row.to_le_bytes())?;
        w.write_all(&[u8::try_from(p.candidates.len())?])?;
        w.write_all(&p.candidates)?;
        w.write_all(&p.layouts.to_le_bytes())?;
        for s in &p.swings {
            w.write_all(&[s[0].to_le_bytes()[0], s[1].to_le_bytes()[0]])?;
        }
    }
    w.flush()?;
    Ok(())
}

pub fn read_ret(path: &Path) -> anyhow::Result<Vec<Priced>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?.read_to_end(&mut bytes)?;
    let mut at = 0usize;
    let mut take = |n: usize| -> anyhow::Result<&[u8]> {
        let slice = bytes
            .get(at..at + n)
            .with_context(|| format!("{}: truncated", path.display()))?;
        at += n;
        Ok(slice)
    };
    ensure!(take(4)? == RET_MAGIC, "{}: not a .ret file", path.display());
    let version = u32::from_le_bytes(take(4)?.try_into()?);
    ensure!(
        version == RET_VERSION,
        "{}: .ret version {version}",
        path.display()
    );
    let n = u32::from_le_bytes(take(4)?.try_into()?) as usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let row = u32::from_le_bytes(take(4)?.try_into()?);
        let ncand = usize::from(take(1)?[0]);
        let candidates = take(ncand)?.to_vec();
        let layouts = u16::from_le_bytes(take(2)?.try_into()?);
        let count = ncand.saturating_sub(1) * usize::from(layouts);
        let swings = take(count * 2)?
            .chunks_exact(2)
            .map(|s| [i8::from_le_bytes([s[0]]), i8::from_le_bytes([s[1]])])
            .collect();
        out.push(Priced {
            row,
            candidates,
            layouts,
            swings,
        });
    }
    ensure!(at == bytes.len(), "{}: trailing bytes", path.display());
    Ok(out)
}

/// The label gate of §4c on one stored decision: the same call wins on both
/// scorers on pool `[0, M)`, differs from the own call, and clears `margin` on
/// the independent pool `[M, 2M)`.  `None` when it does not fire.
pub fn relabel(p: &Priced, m: usize, margin: f64) -> Option<u8> {
    let stored = usize::from(p.layouts);
    if stored < 2 * m {
        return None;
    }
    let own = p.candidates[0];
    let mut winner = [own; 2];
    let mut in_sample = [0.0f64; 2];
    let mut held_out = [0.0f64; 2];
    #[allow(clippy::cast_precision_loss)] // IMP sums are small integers
    let mean = |sum: i64| sum as f64 / m as f64;
    for (c, &call) in p.candidates.iter().enumerate().skip(1) {
        let rows = &p.swings[(c - 1) * stored..c * stored];
        for bracket in 0..2 {
            let first = mean(rows[..m].iter().map(|s| i64::from(s[bracket])).sum());
            let second = mean(rows[m..2 * m].iter().map(|s| i64::from(s[bracket])).sum());
            if first > in_sample[bracket] {
                in_sample[bracket] = first;
                held_out[bracket] = second;
                winner[bracket] = call;
            }
        }
    }
    (winner[0] == winner[1] && winner[0] != own && held_out[0] > margin && held_out[1] > margin)
        .then_some(winner[0])
}

/// Sidecar fields that legitimately differ between chunks of one shard
const PER_CHUNK: [&str; 6] = [
    "skip",
    "boards",
    "rows",
    "contested_rows",
    "forced_pass_decisions",
    "enrich_rejected",
];
/// Sidecar fields that must agree across **every** chunk of the corpus
const GLOBAL: [&str; 7] = [
    "git_sha",
    "feature_version",
    "features_len",
    "softmax_len",
    "dd_len",
    "row_len",
    "teacher",
];

/// The cut: `roots/<shard>/chunk-<c>.{f32,tags,seq,json,ret}` →
/// `out/<shard>.{f32,tags,seq,json}`, labels overwritten where the gate fires.
pub fn cut(roots: &[PathBuf], out: &Path, m: usize, margin: f64) -> anyhow::Result<()> {
    ensure!(m > 0, "--cut M must be positive");
    // shard -> chunk id -> stem
    let mut shards: BTreeMap<String, BTreeMap<u64, PathBuf>> = BTreeMap::new();
    for root in roots {
        for shard in std::fs::read_dir(root)
            .with_context(|| format!("--chunks {}", root.display()))?
            .filter_map(Result::ok)
            .filter(|e| e.path().is_dir())
        {
            let name = shard.file_name().to_string_lossy().into_owned();
            for entry in std::fs::read_dir(shard.path())?.filter_map(Result::ok) {
                let path = entry.path();
                let Some(stem) = path
                    .file_name()
                    .and_then(|f| f.to_str())
                    .and_then(|f| f.strip_suffix(".json"))
                    .and_then(|f| f.strip_prefix("chunk-"))
                else {
                    continue;
                };
                let id: u64 = stem
                    .parse()
                    .with_context(|| format!("{}", path.display()))?;
                let stem = path.with_extension("");
                if let Some(other) = shards.entry(name.clone()).or_default().insert(id, stem) {
                    bail!(
                        "chunk {id} of {name} exists twice: {} and {}",
                        other.display(),
                        path.display()
                    );
                }
            }
        }
    }
    ensure!(
        !shards.is_empty(),
        "no <shard>/chunk-<c>.json under the --chunks roots"
    );
    std::fs::create_dir_all(out)?;

    let mut global: Option<serde_json::Value> = None;
    let mut summary = Vec::new();
    for (shard, chunks) in &shards {
        let mut metas: Vec<(u64, serde_json::Value, PathBuf)> = Vec::new();
        for (&id, stem) in chunks {
            let meta: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(stem.with_extension("json"))?)?;
            metas.push((id, meta, stem.clone()));
        }
        // Chunks are bank windows: sort by `skip`, require contiguity.
        metas.sort_by_key(|(_, meta, _)| meta["skip"].as_u64());
        let pins = |meta: &serde_json::Value| {
            let mut meta = meta.clone();
            for key in PER_CHUNK {
                meta.as_object_mut().map(|o| o.remove(key));
            }
            if let Some(r) = meta.get_mut("relabel").and_then(|r| r.as_object_mut()) {
                for key in ["layouts", "decisions", "priced", "starved", "extended"] {
                    r.remove(key);
                }
            }
            meta
        };
        let head = pins(&metas[0].1);
        let global_pins = |meta: &serde_json::Value| -> Vec<serde_json::Value> {
            GLOBAL.iter().map(|k| meta[k].clone()).collect()
        };
        let g = serde_json::Value::Array(global_pins(&metas[0].1));
        match &global {
            None => global = Some(g),
            Some(prev) => ensure!(
                *prev == g,
                "{shard} disagrees with an earlier shard on {GLOBAL:?}: {prev} vs {g}"
            ),
        }
        for pair in metas.windows(2) {
            let (a, b) = (&pair[0].1, &pair[1].1);
            ensure!(
                a["skip"].as_u64().unwrap_or(0) + a["boards"].as_u64().unwrap_or(0)
                    == b["skip"].as_u64().unwrap_or(u64::MAX),
                "{shard}: chunks {} and {} are not contiguous (skip {} + boards {} != skip {})",
                pair[0].0,
                pair[1].0,
                a["skip"],
                a["boards"],
                b["skip"]
            );
        }
        for (id, meta, _) in &metas {
            ensure!(
                pins(meta) == head,
                "{shard}: chunk {id}'s sidecar disagrees with chunk {}'s (same walk, same commit?)",
                metas[0].0
            );
            let layouts = meta["relabel"]["layouts"].as_u64().unwrap_or(0);
            ensure!(
                layouts as usize >= 2 * m,
                "{shard}: chunk {id} stores {layouts} layouts, --cut {m} needs {}",
                2 * m
            );
        }

        let features_len = head["features_len"].as_u64().context("features_len")? as usize;
        let row_bytes = head["row_bytes"].as_u64().context("row_bytes")? as usize;
        let seq_bytes = head["seq"]["row_bytes"].as_u64().map(|b| b as usize);
        let stem = out.join(shard);
        let mut f32_w = BufWriter::new(std::fs::File::create(stem.with_extension("f32.tmp"))?);
        let mut tags_w = BufWriter::new(std::fs::File::create(stem.with_extension("tags.tmp"))?);
        let mut seq_w = seq_bytes
            .map(|_| std::fs::File::create(stem.with_extension("seq.tmp")).map(BufWriter::new))
            .transpose()?;
        let (mut rows, mut contested, mut forced, mut rejected, mut boards) =
            (0u64, 0u64, 0u64, 0u64, 0u64);
        let (mut decisions, mut eligible, mut fired) = (0usize, 0usize, 0usize);
        let mut min_layouts = u64::MAX;
        for (id, meta, chunk) in &metas {
            let n = meta["rows"].as_u64().context("rows")? as usize;
            let mut f32_bytes = std::fs::read(chunk.with_extension("f32"))?;
            ensure!(
                f32_bytes.len() == n * row_bytes,
                "{shard}: chunk {id}'s .f32 is {} bytes, sidecar says {n} rows × {row_bytes}",
                f32_bytes.len()
            );
            let tags = std::fs::read(chunk.with_extension("tags"))?;
            ensure!(
                tags.len() == n,
                "{shard}: chunk {id}'s .tags is not {n} bytes"
            );
            let priced = read_ret(&chunk.with_extension("ret"))?;
            for p in &priced {
                decisions += 1;
                if usize::from(p.layouts) >= 2 * m {
                    eligible += 1;
                }
                if let Some(winner) = relabel(p, m, margin) {
                    fired += 1;
                    let at = p.row as usize * row_bytes + features_len * 4;
                    let slot = &mut f32_bytes[at..at + SOFTMAX_LEN * 4];
                    slot.fill(0);
                    let w = usize::from(winner) * 4;
                    slot[w..w + 4].copy_from_slice(&1.0f32.to_le_bytes());
                }
            }
            f32_w.write_all(&f32_bytes)?;
            tags_w.write_all(&tags)?;
            if let (Some(w), Some(b)) = (&mut seq_w, seq_bytes) {
                let seq = std::fs::read(chunk.with_extension("seq"))?;
                ensure!(
                    seq.len() == n * b,
                    "{shard}: chunk {id}'s .seq is not {n} × {b} bytes"
                );
                w.write_all(&seq)?;
            }
            rows += n as u64;
            contested += meta["contested_rows"].as_u64().unwrap_or(0);
            forced += meta["forced_pass_decisions"].as_u64().unwrap_or(0);
            rejected += meta["enrich_rejected"].as_u64().unwrap_or(0);
            boards += meta["boards"].as_u64().unwrap_or(0);
            min_layouts = min_layouts.min(meta["relabel"]["layouts"].as_u64().unwrap_or(0));
        }
        f32_w.flush()?;
        tags_w.flush()?;
        if let Some(w) = &mut seq_w {
            w.flush()?;
        }
        drop((f32_w, tags_w, seq_w));

        let mut meta = metas[0].1.clone();
        let o = meta.as_object_mut().context("sidecar is an object")?;
        o.insert("rows".into(), rows.into());
        o.insert("contested_rows".into(), contested.into());
        o.insert("forced_pass_decisions".into(), forced.into());
        o.insert("enrich_rejected".into(), rejected.into());
        o.insert("boards".into(), boards.into());
        o.insert("chunks".into(), metas.len().into());
        o.insert(
            "relabel".into(),
            serde_json::json!({
                "m": m,
                "margin": margin,
                "layouts": min_layouts,
                "top_k": head["relabel"]["top_k"],
                "epsilon": head["relabel"]["epsilon"],
                "temperature": head["relabel"]["temperature"],
                "decisions": decisions,
                "eligible": eligible,
                "fired": fired,
                "note": "labels cut by dump-teacher --cut: pools [0,M) select, [M,2M) validate, same winner on both scorers clearing margin held out",
            }),
        );
        std::fs::write(stem.with_extension("json.tmp"), format!("{meta:#}\n"))?;
        for ext in ["f32", "tags", "json"] {
            std::fs::rename(
                stem.with_extension(format!("{ext}.tmp")),
                stem.with_extension(ext),
            )?;
        }
        if seq_bytes.is_some() {
            std::fs::rename(stem.with_extension("seq.tmp"), stem.with_extension("seq"))?;
        }
        summary.push(format!(
            "{shard}: {} chunks, {rows} rows, {decisions} decisions ({eligible} at ≥ 2M layouts), {fired} relabelled",
            metas.len()
        ));
    }
    for line in summary {
        eprintln!("cut M={m} margin={margin}: {line}");
    }
    Ok(())
}
