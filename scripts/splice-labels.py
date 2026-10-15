#!/usr/bin/env python3
"""Transplant a cut corpus's labels onto a fresh re-walk of its chunks.

    scripts/splice-labels.py NEW_CHUNK_DIR OLD_CHUNK_DIR OLD_CUT_STEM OUT_STEM
    scripts/splice-labels.py --validate NEW_CHUNK_DIR OLD_CHUNK_DIR OLD_CUT_STEM

`NEW_CHUNK_DIR` holds one shard's chunks re-walked at HEAD (`dump-teacher
--relabel --layouts 0`, any feature width, raw teacher one-hot labels);
`OLD_CHUNK_DIR` the fleet's chunks of that shard (raw one-hot, the labels the
cut started from); `OLD_CUT_STEM` the shard as `dump-teacher --cut` wrote it
(the M32 labels).  The cut's label block is written over the new one, into
`OUT_STEM.{f32,tags,json}` — a trainer stem of the new width.

What the splice checks, and what it does not.  A chunk is spliced
*positionally* when its row count, one-hot block, DD block and `.tags` equal
the old chunk's byte for byte; nothing else about the row is compared, so the
check proves the labels, DD tables and phases line up, not that each row is the
same decision.  Otherwise rows are matched on the *reading-free key* (hand
summary, auction context, vulnerability, one-hot, DD).  The context block is a
summary — strains bid, last bid, partner's last bid, penalty, seat — not the
auction, and the key leaves out the cell (the compact block), so equal keys do
not prove an equal auction prefix or an equal table configuration.  A key that
occurs more than once among the old rows is dropped (those new rows keep the
teacher's one-hot); a key that occurs more than once among the new rows sends
the one old label to each of them.  Neither is reported by the splice itself.

`--validate` writes nothing and reports, per chunk and in total: rows matched
positionally or by key, new rows with no old key, new rows dropped as
ambiguous (and how many of those one identity would resolve), new rows sharing
one old row, labels transplanted, and — the check the splice lacks — rows whose
**identity** differs from the old row whose label they take.  Identity is
reconstructed from each chunk alone: the table ordinal (a table starts at an
opening row, as the trainer's board scan reads it), the cell
(`cells[table % len(cells)]`, unknown for an enriched walk without `--replay`)
and the full auction prefix (each row's call is its raw one-hot's argmax; a
forced pass writes no row and is recovered from the seat gap).  It exits 1 when
any transplanted row's identity differs or a matched row's tag differs.

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
    row).  Leaving the compact block out means the key does not see the cell:
    a walk whose per-board cell rotation changed (the uniform shards' six cells
    → three) can match a row to the same deal and seat under another cell.
    `--validate` counts those.
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
            # teacher's one-hot.  An old key seen twice maps to -1 and is
            # dropped; `--validate` reports how many rows that costs.
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


# The context offsets an auction-opening row is read from (no bid yet, seat 0);
# pinned by `features::tests::trainer_board_scan_offsets_hold`.
BID_PRESENT, SEATS = 20, slice(41, 45)


def identities(meta, rows):
    """(table ordinal, cell, auction prefix) per row, rebuilt from the chunk alone.

    `dump-teacher` writes each table's decisions in auction order and every
    call is either a row (its raw one-hot's argmax) or a forced pass (no row,
    visible as a seat gap; a gap of four cannot occur inside a live auction).
    A table starts at row 0, or at a row with no bid yet whose predecessor had
    one or sat at a later seat.  The one table start this misses — a passed-out
    table whose last decisions were forced, followed by a later-seat start —
    merges two tables and shifts every later ordinal, so it shows up loudly.
    """
    f = meta["features_len"]
    bid = rows[:, BID_PRESENT] != 0
    seat = rows[:, SEATS].argmax(axis=1)
    call = rows[:, f : f + SOFTMAX].argmax(axis=1)
    cells = meta.get("cells") or []
    # Rejected deals write no rows, so without `--replay` the board index
    # behind `cells[board % len]` is lost under `--enrich`.
    cell_known = bool(cells) and (meta.get("replay") or not meta.get("enrich"))
    out, table, prefix = [], -1, ()
    for i in range(rows.shape[0]):
        if i == 0 or (not bid[i] and (bid[i - 1] or seat[i] <= seat[i - 1])):
            table, prefix = table + 1, (0,) * int(seat[i])
        else:
            prefix += (0,) * int((seat[i] - seat[i - 1] - 1) % 4)
        out.append((table, cells[table % len(cells)] if cell_known else None, prefix))
        prefix += (int(call[i]),)
    return out


def validate():
    new_dir, old_dir, cut_stem = sys.argv[2:5]
    new_stems, old_stems = chunks(new_dir), chunks(old_dir)
    assert len(new_stems) == len(old_stems), f"{len(new_stems)} new chunks vs {len(old_stems)} old"
    cm, cut = load(cut_stem)
    cf = cm["features_len"]
    fields = ("rows", "positional", "keyed_chunks", "unmatched", "ambiguous", "resolvable", "shared", "transplanted", "relabelled", "identity", "cell", "prefix", "tag")
    total = dict.fromkeys(fields, 0)
    base = 0
    for new_stem, old_stem in zip(new_stems, old_stems):
        (nm, new), (om, old) = load(new_stem), load(old_stem)
        assert nm["skip"] == om["skip"] and nm["boards"] == om["boards"], f"{new_stem} vs {old_stem}: windows differ"
        nf, of = nm["features_len"], om["features_len"]
        new_tags = open(f"{new_stem}.tags", "rb").read()
        old_tags = open(f"{old_stem}.tags", "rb").read()
        labels = cut[base : base + old.shape[0], cf : cf + SOFTMAX]
        new_id, old_id = identities(nm, new), identities(om, old)
        c = dict.fromkeys(fields, 0)
        c["rows"] = new.shape[0]
        if new.shape[0] == old.shape[0] and np.array_equal(new[:, nf:], old[:, of:]) and new_tags == old_tags:
            # The splice's positional path: row i takes old row i's label.
            c["positional"] = new.shape[0]
            pairs = list(enumerate(range(new.shape[0])))
        else:
            # The splice's keyed path, with every row it drops accounted for.
            c["keyed_chunks"] = 1
            old_rows = {}
            for j, k in enumerate(reading_free_key(old, of)):
                old_rows.setdefault(k, []).append(j)
            new_keys = reading_free_key(new, nf)
            uses = {}
            for k in new_keys:
                uses[k] = uses.get(k, 0) + 1
            pairs = []
            for i, k in enumerate(new_keys):
                js = old_rows.get(k, [])
                if not js:
                    c["unmatched"] += 1
                elif len(js) > 1:
                    c["ambiguous"] += 1
                    c["resolvable"] += sum(old_id[j] == new_id[i] for j in js) == 1
                else:
                    c["shared"] += uses[k] > 1
                    pairs.append((i, js[0]))
        for i, j in pairs:
            c["transplanted"] += 1
            c["relabelled"] += int((new[i, nf : nf + SOFTMAX] != labels[j]).any())
            c["identity"] += new_id[i] != old_id[j]
            c["cell"] += new_id[i][1] != old_id[j][1]
            c["prefix"] += new_id[i][2] != old_id[j][2]
            c["tag"] += new_tags[i] != old_tags[j]
        if c["unmatched"] or c["ambiguous"] or c["shared"] or c["identity"] or c["tag"]:
            print(f"{new_stem}: " + ", ".join(f"{k} {v}" for k, v in c.items()))
        for k in fields:
            total[k] += c[k]
        base += old.shape[0]
    assert base == cut.shape[0], f"cut stem has {cut.shape[0]} rows, old chunks {base}"
    print(f"{new_dir}: {len(new_stems)} chunks; " + ", ".join(f"{k} {v}" for k, v in total.items()))
    print(
        "  positional/transplanted = rows taking an old label; unmatched/ambiguous keep the teacher's"
        " one-hot (resolvable = ambiguous rows one identity would match); shared = rows sharing one"
        " old row; identity = transplanted rows whose (table, cell, prefix) differs, of which cell"
        " and prefix count each part; tag = transplanted rows whose phase tag differs"
    )
    if total["identity"] or total["tag"]:
        sys.exit(1)


if __name__ == "__main__":
    validate() if sys.argv[1:2] == ["--validate"] else main()
