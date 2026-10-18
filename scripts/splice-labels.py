#!/usr/bin/env python3
"""Transplant a cut corpus's labels onto a fresh re-walk of its chunks.

    scripts/splice-labels.py [--any-identity] NEW_CHUNK_DIR OLD_CHUNK_DIR OLD_CUT_STEM OUT_STEM
    scripts/splice-labels.py --validate [--any-identity] NEW_CHUNK_DIR OLD_CHUNK_DIR OLD_CUT_STEM

`NEW_CHUNK_DIR` holds one shard's chunks re-walked at HEAD (`dump-teacher
--relabel --layouts 0`, any feature width, raw teacher one-hot labels);
`OLD_CHUNK_DIR` the fleet's chunks of that shard (raw one-hot, the labels the
cut started from); `OLD_CUT_STEM` the shard as `dump-teacher --cut` wrote it
(the M32 labels).  The cut's label block is written over the new one, into
`OUT_STEM.{f32,tags,json}` — a trainer stem of the new width.

How rows are paired.  A chunk is paired *positionally* when its row count,
one-hot block, DD block and `.tags` equal the old chunk's byte for byte.
Otherwise rows are paired on the *reading-free key* (hand summary, auction
context, vulnerability, one-hot, DD).  Neither proves the row is the same
decision: the context block is a summary — strains bid, last bid, partner's
last bid, penalty, seat — not the auction, and the key leaves out the cell (the
compact block).  A key that occurs more than once among the old rows is dropped
(those new rows keep the teacher's one-hot); a key that occurs more than once
among the new rows sends the one old label to each of them.

So every pair must also agree on **identity**, reconstructed from each chunk
alone: the table ordinal (a table starts at an opening row, as the trainer's
board scan reads it), the cell (`cells[table % len(cells)]`, unknown for an
enriched walk without `--replay`) and the full auction prefix (each row's call
is its raw one-hot's argmax; a forced pass writes no row and is recovered from
the seat gap).  A pair whose identity differs is *refused*: the new row keeps
the teacher's one-hot.  On the M32 uniform shards that refuses every row of the
retired Dutch cells (features-v8.md §2).  `--any-identity` transplants them
anyway, reproducing the 2026-09-26 v8 splice.

`--validate` writes nothing and reports, per chunk and in total: rows paired
positionally or by key, new rows with no old key, new rows dropped as ambiguous
(and how many of those one identity would resolve), new rows sharing one old
row, pairs refused, labels transplanted and how many of them the cut had
changed, and pairs whose identity differs (`cell` and `prefix` count each
part).  It exits 1 when a transplanted row's identity or a paired row's tag
differs — under the default, a tag alone.

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
    The identity check in `pair` refuses those.
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


def pair(new_stem, old_stem, any_identity):
    """Pair one chunk's new rows with old rows, as the splice transplants them.

    Returns the two chunks, their tags, the `(i, j)` pairs whose labels move
    and a counter of everything else.
    """
    (nm, new), (om, old) = load(new_stem), load(old_stem)
    assert nm["skip"] == om["skip"] and nm["boards"] == om["boards"], f"{new_stem} vs {old_stem}: windows differ"
    nf, of = nm["features_len"], om["features_len"]
    new_tags = open(f"{new_stem}.tags", "rb").read()
    old_tags = open(f"{old_stem}.tags", "rb").read()
    new_id, old_id = identities(nm, new), identities(om, old)
    c = dict.fromkeys(FIELDS, 0)
    c["rows"] = new.shape[0]
    if new.shape[0] == old.shape[0] and np.array_equal(new[:, nf:], old[:, of:]) and new_tags == old_tags:
        # Aligned walk: row i is old row i's candidate.
        c["positional"] = new.shape[0]
        candidates = list(enumerate(range(new.shape[0])))
    else:
        # The walk drifted (a cell retired, a seat our book bids moved):
        # candidates by the reading-free key; an old key seen twice is dropped.
        c["keyed_chunks"] = 1
        old_rows = {}
        for j, k in enumerate(reading_free_key(old, of)):
            old_rows.setdefault(k, []).append(j)
        new_keys = reading_free_key(new, nf)
        uses = {}
        for k in new_keys:
            uses[k] = uses.get(k, 0) + 1
        candidates = []
        for i, k in enumerate(new_keys):
            js = old_rows.get(k, [])
            if not js:
                c["unmatched"] += 1
            elif len(js) > 1:
                c["ambiguous"] += 1
                c["resolvable"] += sum(old_id[j] == new_id[i] for j in js) == 1
            else:
                c["shared"] += uses[k] > 1
                candidates.append((i, js[0]))
    pairs = []
    for i, j in candidates:
        c["tag"] += new_tags[i] != old_tags[j]
        if new_id[i] != old_id[j]:
            c["identity"] += 1
            c["cell"] += new_id[i][1] != old_id[j][1]
            c["prefix"] += new_id[i][2] != old_id[j][2]
            if not any_identity:
                c["refused"] += 1
                continue
        pairs.append((i, j))
    c["transplanted"] = len(pairs)
    return (nm, new, new_tags), (om, old), pairs, c


FIELDS = ("rows", "positional", "keyed_chunks", "unmatched", "ambiguous", "resolvable", "shared", "refused", "transplanted", "relabelled", "identity", "cell", "prefix", "tag")


def main(any_identity, new_dir, old_dir, cut_stem, out_stem):
    new_stems, old_stems = chunks(new_dir), chunks(old_dir)
    assert len(new_stems) == len(old_stems), f"{len(new_stems)} new chunks vs {len(old_stems)} old"
    cm, cut = load(cut_stem)
    cf = cm["features_len"]
    rows, tags, base = [], b"", 0
    total = dict.fromkeys(FIELDS, 0)
    for new_stem, old_stem in zip(new_stems, old_stems):
        (nm, new, new_tags), (om, old), pairs, c = pair(new_stem, old_stem, any_identity)
        nf, of = nm["features_len"], om["features_len"]
        labels = cut[base : base + old.shape[0], cf : cf + SOFTMAX]
        assert c["tag"] == 0, f"{new_stem}: {c['tag']} paired rows' tags differ"
        for i, j in pairs:
            assert np.array_equal(new[i, nf : nf + SOFTMAX], old[j, of : of + SOFTMAX]), f"{new_stem}: row {i} teacher one-hot differs from old row {j}"
            c["relabelled"] += int((new[i, nf : nf + SOFTMAX] != labels[j]).any())
            new[i, nf : nf + SOFTMAX] = labels[j]
        for k in FIELDS:
            total[k] += c[k]
        base += old.shape[0]
        rows.append(new)
        tags += new_tags
    assert base == cut.shape[0], f"cut stem has {cut.shape[0]} rows, old chunks {base}"
    new = np.concatenate(rows)
    new.tofile(f"{out_stem}.f32")
    open(f"{out_stem}.tags", "wb").write(tags)
    meta = dict(cm)  # the cut's sidecar carries the shard-level pins the trainer compares
    meta.update({k: nm[k] for k in ("feature_version", "features_len", "row_len", "row_bytes", "git_sha") if k in nm})
    meta["rows"] = int(new.shape[0])
    meta["relabel"] = dict(
        cm.get("relabel", {}),
        spliced_from=cut_stem,
        rows_matched=total["transplanted"],
        rows_relabelled=total["relabelled"],
        rows_refused=total["refused"],
        any_identity=any_identity,
        chunks_keyed=total["keyed_chunks"],
        rewalk_git_sha=nm.get("git_sha"),
    )
    json.dump(meta, open(f"{out_stem}.json", "w"), indent=2)
    n = new.shape[0]
    print(f"{out_stem}: {n} rows, {total['transplanted']} matched ({100 * total['transplanted'] / n:.2f}%), {total['refused']} refused on identity, {total['keyed_chunks']} of {len(new_stems)} chunks key-matched, {total['relabelled']} labels transplanted")


def validate(any_identity, new_dir, old_dir, cut_stem):
    new_stems, old_stems = chunks(new_dir), chunks(old_dir)
    assert len(new_stems) == len(old_stems), f"{len(new_stems)} new chunks vs {len(old_stems)} old"
    cm, cut = load(cut_stem)
    cf = cm["features_len"]
    total = dict.fromkeys(FIELDS, 0)
    base = 0
    for new_stem, old_stem in zip(new_stems, old_stems):
        (nm, new, _), (om, old), pairs, c = pair(new_stem, old_stem, any_identity)
        nf = nm["features_len"]
        labels = cut[base : base + old.shape[0], cf : cf + SOFTMAX]
        for i, j in pairs:
            c["relabelled"] += int((new[i, nf : nf + SOFTMAX] != labels[j]).any())
        if c["unmatched"] or c["ambiguous"] or c["shared"] or c["identity"] or c["tag"]:
            print(f"{new_stem}: " + ", ".join(f"{k} {v}" for k, v in c.items()))
        for k in FIELDS:
            total[k] += c[k]
        base += old.shape[0]
    assert base == cut.shape[0], f"cut stem has {cut.shape[0]} rows, old chunks {base}"
    print(f"{new_dir}: {len(new_stems)} chunks; " + ", ".join(f"{k} {v}" for k, v in total.items()))
    print(
        "  positional = chunk rows paired by position; transplanted = rows taking an old label (relabelled:"
        " the cut had changed it); unmatched/ambiguous/refused keep the teacher's one-hot (resolvable ="
        " ambiguous rows one identity would match; refused = pairs whose identity differs); shared = rows"
        " sharing one old row; identity = pairs whose (table, cell, prefix) differs, of which cell and"
        " prefix count each part; tag = pairs whose phase tag differs"
    )
    if (any_identity and total["identity"]) or total["tag"]:
        sys.exit(1)


if __name__ == "__main__":
    args = sys.argv[1:]
    check = args[:1] == ["--validate"]
    args = args[check:]
    any_identity = args[:1] == ["--any-identity"]
    args = args[any_identity:]
    (validate if check else main)(any_identity, *args)
