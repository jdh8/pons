#!/usr/bin/env python3
"""Fold the frozen card coordinates of a trained net into its first-layer bias.

A net input that is constant at `c` across the whole training corpus is
algebraically a bias term: no gradient ever separates its column of `W1` from
`b1`, so the column sits at its initialisation draw, and flipping the input at
serving adds a *random* vector to every hidden pre-activation — the measured
≈ −0.015 IMP/board frozen-coordinate tax (`docs/ai-bidder/card-manifold.md`).
The fix is exact on the training distribution: `b1 += c·w`, then `w = 0`, for
every frozen column.  A folded row is inert instead of a landmine.

Which columns are frozen is **hardcoded geometry, not measured constancy**:
the v4 corpus cells vary exactly four card slots per side (base-system bits 0
and 2, `1D opening with 5 cards`, `Kickback 1430` — in-block offsets
{0, 2, 7, 77}).  The corpus itself is gone from disk, and the fixture cannot
substitute for a scan — six of the eight live slots happen to be constant
across its 8 rows and a constancy-derived fold set would wrongly zero them.
The fixture supplies only the constant *values* `c_i`, which every row agrees
on (asserted here) and which are exact 0.0/1.0 bits.

Idempotent: a second run folds `c·0` and re-zeroes zeros.  The
"nonzero folded columns" count printed below is the signal — 272 on the first
run, 0 on a rerun.

Usage:
    scripts/fold-constant-inputs.py src/bidding/weights/american_bba_v4
    scripts/fold-constant-inputs.py WEIGHTS_STEM --data CORPUS_STEM [--data ...]

Without `--data` this is the v4 **geometry mode** above.  With `--data` it is
**scan mode**, the standard last step of any train: a full streamed min/max
over every input column of the corpus the net was fitted on, folding every
column that never moves — no sampling, no geometry assumptions, any layout.
"""

import argparse
import json
import sys

import numpy as np

OFFSET_OUR_CARD, OFFSET_THEIR_CARD, CARD_LEN = 88, 228, 140
LIVE_SLOTS = sorted(
    base + slot for base in (OFFSET_OUR_CARD, OFFSET_THEIR_CARD) for slot in (0, 2, 7, 77)
)


def scan_constants(stems, features):
    """(fold_slots, c) from a full min/max scan of every corpus stem."""
    lo = np.full(features, np.inf)
    hi = np.full(features, -np.inf)
    for stem in stems:
        meta = json.load(open(f"{stem}.json"))
        if meta["features_len"] != features:
            sys.exit(f"{stem} has {meta['features_len']} features, net wants {features}")
        rows = np.memmap(f"{stem}.f32", dtype="<f4", mode="r").reshape(-1, meta["row_len"])
        for at in range(0, rows.shape[0], 1 << 18):
            chunk = rows[at : at + (1 << 18), :features]
            np.minimum(lo, chunk.min(axis=0), out=lo)
            np.maximum(hi, chunk.max(axis=0), out=hi)
    fold = [i for i in range(features) if lo[i] == hi[i]]
    assert np.isfinite(lo[fold]).all(), "scan saw no rows"
    return fold, lo.astype("<f4")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("stem", help="trained artifact stem, e.g. src/bidding/weights/american_bba_v4")
    ap.add_argument(
        "--data",
        action="append",
        default=[],
        metavar="STEM",
        help="corpus stem(s) for scan mode; omit for the v4 geometry mode",
    )
    args = ap.parse_args()

    meta = json.load(open(f"{args.stem}.json"))
    shapes, order = meta["param_shapes"], meta["param_order"]
    hidden, features = shapes["l1.weight"]
    total = sum(int(np.prod(shapes[name])) for name in order)

    flat = np.fromfile(f"{args.stem}.f32", dtype="<f4")
    if flat.size != total:
        sys.exit(f"{args.stem}.f32 has {flat.size} floats, sidecar says {total}")

    if args.data:
        fold, c = scan_constants(args.data, features)
    else:
        if shapes["l1.weight"] != [256, 368]:
            sys.exit(f"l1.weight is {shapes['l1.weight']}, not [256, 368]; geometry mode is v4-only")
        fold = [i for i in range(OFFSET_OUR_CARD, features) if i not in LIVE_SLOTS]
        assert len(fold) == 2 * CARD_LEN - len(LIVE_SLOTS)

        rows = np.array(json.load(open(f"{args.stem}.fixture.json"))["features"], dtype="<f4")
        if rows.shape[1] != features:
            sys.exit(f"fixture rows have {rows.shape[1]} features, net wants {features}")
        if (rows[:, fold].min(0) != rows[:, fold].max(0)).any():
            sys.exit("fixture rows disagree on a folded slot — it is live, not frozen")
        c = rows[0]
        if not np.isin(c[fold], (0.0, 1.0)).all():
            sys.exit("a folded slot's constant is not an exact 0/1 bit")

    w1 = flat[: hidden * features].reshape(hidden, features)
    b1 = flat[hidden * features : hidden * features + hidden]
    nonzero = int((w1[:, fold] != 0).any(axis=0).sum())
    print(f"nonzero folded columns before: {nonzero} of {len(fold)}")

    b1[:] = (b1.astype("f8") + w1[:, fold].astype("f8") @ c[fold].astype("f8")).astype("<f4")
    w1[:, fold] = 0.0

    flat.tofile(f"{args.stem}.f32")
    written = np.fromfile(f"{args.stem}.f32", dtype="<f4").size
    assert written == total, f"wrote {written} floats, expected {total}"

    meta["folded"] = {
        "c_source": (
            f"min/max scan of {args.data}"
            if args.data
            else "fixture.json features row 0 (all rows agree; exact 0/1 bits)"
        ),
        "live_slots": None if args.data else LIVE_SLOTS,
        "script": "scripts/fold-constant-inputs.py",
        "zeroed_columns": len(fold),
    }
    with open(f"{args.stem}.json", "w") as f:
        json.dump(meta, f, indent=2, sort_keys=True)
        f.write("\n")
    kept = [i for i in range(features) if i not in set(fold)]
    print(f"folded {len(fold)} columns into b1; {len(kept)} live columns kept")


if __name__ == "__main__":
    main()
