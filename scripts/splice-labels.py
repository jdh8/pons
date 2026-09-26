#!/usr/bin/env python3
"""Transplant a cut corpus's labels onto a fresh re-walk of its chunks.

    scripts/splice-labels.py NEW_CHUNK_DIR OLD_CHUNK_DIR OLD_CUT_STEM OUT_STEM

`NEW_CHUNK_DIR` holds one shard's chunks re-walked at HEAD (`dump-teacher
--relabel --layouts 0`, any feature width, raw teacher one-hot labels);
`OLD_CHUNK_DIR` the fleet's chunks of that shard (raw one-hot, the labels the
cut started from); `OLD_CUT_STEM` the shard as `dump-teacher --cut` wrote it
(the M32 labels).  Row alignment is *proven*, not assumed: chunk by chunk,
the new walk's one-hot block, DD block and `.tags` must equal the old chunk's
byte for byte.  Only then is the cut's label block written over the new one,
into `OUT_STEM.{f32,tags,json}` — a trainer stem of the new width.

ponytail: whole-shard numpy in RAM (~1.5 GB for the largest shard); stream it
if a shard ever outgrows the box.
"""

import json
import os
import sys

import numpy as np

SOFTMAX = 38


def load(stem):
    meta = json.load(open(f"{stem}.json"))
    width = meta["features_len"] + SOFTMAX + meta["dd_len"]
    rows = np.fromfile(f"{stem}.f32", dtype="<f4").reshape(-1, width)
    assert rows.shape[0] == meta["rows"], f"{stem}: {rows.shape[0]} rows, sidecar says {meta['rows']}"
    return meta, rows


# v6 layout, which v8 extends at the tail: hand 10 | context 36 | inferences 72 | vul 2 | compact 56.
HAND_CONTEXT, VUL = slice(0, 46), slice(118, 120)


def reading_free_key(rows, features_len):
    """Hand, auction context, vulnerability, one-hot and DD per row, as bytes.

    Not the inference block (readings drift with the book) and not the compact
    config (a knob default that moved since the fleet ran flips a slot on every
    row; the cell is implied by the walk).
    """
    key = np.concatenate([rows[:, HAND_CONTEXT], rows[:, VUL], rows[:, features_len:]], axis=1)
    return [r.tobytes() for r in np.ascontiguousarray(key)]


def chunks(directory):
    """Chunk stems of one shard, in bank-window order."""
    stems = [
        os.path.join(directory, f[: -len(".json")])
        for f in os.listdir(directory)
        if f.startswith("chunk-") and f.endswith(".json")
    ]
    return sorted(stems, key=lambda s: json.load(open(f"{s}.json"))["skip"])


def main():
    new_dir, old_dir, cut_stem, out_stem = sys.argv[1:5]
    new_stems, old_stems = chunks(new_dir), chunks(old_dir)
    assert len(new_stems) == len(old_stems), f"{len(new_stems)} new chunks vs {len(old_stems)} old"
    cm, cut = load(cut_stem)
    cf = cm["features_len"]
    rows, tags, base, matched, keyed, relabelled = [], b"", 0, 0, 0, 0
    for new_stem, old_stem in zip(new_stems, old_stems):
        (nm, new), (om, old) = load(new_stem), load(old_stem)
        assert nm["skip"] == om["skip"] and nm["boards"] == om["boards"], f"{new_stem} vs {old_stem}: windows differ"
        nf, of = nm["features_len"], om["features_len"]
        new_tags = open(f"{new_stem}.tags", "rb").read()
        old_tags = open(f"{old_stem}.tags", "rb").read()
        labels = cut[base : base + old.shape[0], cf : cf + SOFTMAX]
        if new.shape[0] == old.shape[0] and np.array_equal(new[:, nf:], old[:, of:]) and new_tags == old_tags:
            # Aligned walk: positional.
            relabelled += int((new[:, nf : nf + SOFTMAX] != labels).any(axis=1).sum())
            new[:, nf : nf + SOFTMAX] = labels
            matched += new.shape[0]
        else:
            # The walk drifted (a cell retired, a seat our book bids moved):
            # match rows by the reading-free key and leave the rest on the
            # teacher's one-hot.
            keyed += 1
            index = {}
            for j, k in enumerate(reading_free_key(old, of)):
                index[k] = -1 if k in index else j
            for i, k in enumerate(reading_free_key(new, nf)):
                j = index.get(k, -1)
                if j < 0:
                    continue
                assert np.array_equal(new[i, nf : nf + SOFTMAX], old[j, of : of + SOFTMAX]), f"{new_stem}: row {i} teacher one-hot differs at an identical key"
                assert new_tags[i] == old_tags[j], f"{new_stem}: row {i} tag differs at an identical key"
                relabelled += int((new[i, nf : nf + SOFTMAX] != labels[j]).any())
                new[i, nf : nf + SOFTMAX] = labels[j]
                matched += 1
        base += old.shape[0]
        rows.append(new)
        tags += new_tags
    assert base == cut.shape[0], f"cut stem has {cut.shape[0]} rows, old chunks {base}"
    new = np.concatenate(rows)
    nf = nm["features_len"]
    new.tofile(f"{out_stem}.f32")
    open(f"{out_stem}.tags", "wb").write(tags)
    meta = dict(cm)  # the cut's sidecar carries the shard-level pins the trainer compares
    meta.update({k: nm[k] for k in ("feature_version", "features_len", "row_len", "row_bytes", "git_sha") if k in nm})
    meta["rows"] = int(new.shape[0])
    meta["relabel"] = dict(cm.get("relabel", {}), spliced_from=cut_stem, rows_matched=matched, rows_relabelled=relabelled, chunks_keyed=keyed, rewalk_git_sha=nm.get("git_sha"))
    json.dump(meta, open(f"{out_stem}.json", "w"), indent=2)
    print(f"{out_stem}: {new.shape[0]} rows, {matched} matched ({100 * matched / new.shape[0]:.2f}%), {keyed} of {len(new_stems)} chunks key-matched, {relabelled} labels transplanted")


if __name__ == "__main__":
    main()
