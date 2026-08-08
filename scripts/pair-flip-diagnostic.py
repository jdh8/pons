#!/usr/bin/env python3
"""Did the configured net learn to *read* the kickback bit?

Gate 2 measures IMPs, which the relocated keycard ask barely moves, so a null
there cannot distinguish "the convention is worth nothing" from "the net never
learned the card is an input".  This settles the second question directly and
cheaply (`docs/ai-bidder/configured-net.md`, "The diagnostic that actually
settles it").

A **matched pair** is one deal replayed under two cells (`dump-teacher
--replay --cell a-on/a-on --cell a-off/a-off`): two rows identical in all 366
non-card features, differing only where `Kickback 1430` sits in each side's
card block.  A **moving** pair is one whose *teacher* picks a different call
across the flip — the only rows that can teach what the bit means.

The test: feed the trained net both halves of each held-out moving pair and
require its argmax to move too.  A net that answers identically has not learned
to read the card, and no A/B on top of it means anything.

Usage:
    scripts/pair-flip-diagnostic.py --data target/corpus-v4/enriched \\
        --weights src/bidding/weights/american_v4 [--val-frac 0.10]
"""

import argparse
import json
import sys

import numpy as np

# Block geometry per feature version: (our offset, their offset, default slot).
# v4: the 140-wide card blocks, default slot 77 = `Kickback 1430` (SCHEMA[72]).
# v5: the 28-wide compact blocks (`features.rs` §"The compact-config
# extractor"); no default — pass the axis dim under test via --slot.
GEOMETRY = {4: (88, 228, 77), 5: (88, 116, None)}


def load_dump(stem):
    meta = json.load(open(f"{stem}.json"))
    if meta["feature_version"] not in GEOMETRY:
        sys.exit(f"{stem} is feature v{meta['feature_version']}; this needs v4 or v5")
    rows = np.fromfile(f"{stem}.f32", dtype="<f4").reshape(-1, meta["row_len"])
    n = meta["features_len"]
    return meta, rows[:, :n], rows[:, n : n + meta["softmax_len"]]


def forward(weights, x):
    """The shipped arch: x -> Linear -> relu -> Linear -> relu -> Linear."""
    shapes, order = weights["param_shapes"], weights["param_order"]
    flat, at = np.fromfile(f"{weights['_stem']}.f32", dtype="<f4"), 0
    params = {}
    for name in order:
        size = int(np.prod(shapes[name]))
        params[name] = flat[at : at + size].reshape(shapes[name])
        at += size
    h = np.maximum(x @ params["l1.weight"].T + params["l1.bias"], 0)
    h = np.maximum(h @ params["l2.weight"].T + params["l2.bias"], 0)
    return h @ params["l3.weight"].T + params["l3.bias"]


def matched_pairs(feats, targets, lo, indices):
    """Pairs among rows `>= lo`, keyed on everything but the probed slots."""
    keyed = np.delete(feats[lo:], indices, axis=1)
    seen, pairs = {}, []
    for i, key in enumerate(map(lambda r: r.tobytes(), keyed)):
        if (j := seen.pop(key, None)) is not None:
            a, b = lo + j, lo + i
            # Order the pair (bit off, bit on) so the flip has a sign.
            if feats[a][indices[0]] > feats[b][indices[0]]:
                a, b = b, a
            if feats[a][indices[0]] != feats[b][indices[0]]:
                pairs.append((a, b))
        else:
            seen[key] = i
    return [(a, b) for a, b in pairs if targets[a].argmax() != targets[b].argmax()]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", required=True, help="enriched dump stem (--replay)")
    ap.add_argument("--weights", required=True, help="trained artifact stem")
    ap.add_argument(
        "--val-frac",
        type=float,
        default=0.10,
        help="held-out tail the trainer kept back; 1 to scan the whole dump",
    )
    ap.add_argument(
        "--slot",
        default=None,
        help="in-block slot(s) under test, comma-separated for one-hot axes "
        "whose flip moves two dims — list the flip-target dim FIRST (it orders "
        "the pair). v4 default: 77 = Kickback 1430; v5: the compact axis dims, "
        "see features.rs",
    )
    args = ap.parse_args()

    meta, feats, targets = load_dump(args.data)
    ours, theirs, default_slot = GEOMETRY[meta["feature_version"]]
    slots = (
        [int(s) for s in args.slot.split(",")] if args.slot is not None else [default_slot]
    )
    if slots == [None]:
        sys.exit("this dump's feature version has no default slot; pass --slot")
    indices = [ours + s for s in slots] + [theirs + s for s in slots]
    weights = json.load(open(f"{args.weights}.json"))
    weights["_stem"] = args.weights
    if weights["features_len"] != feats.shape[1]:
        sys.exit(f"net wants {weights['features_len']} features, dump has {feats.shape[1]}")

    # The trainer's split is the contiguous tail of each dump; mirror it exactly.
    lo = feats.shape[0] - round(feats.shape[0] * args.val_frac)
    moving = matched_pairs(feats, targets, lo, indices)
    print(f"{meta['rows']} rows, held-out tail from {lo}: {len(moving)} moving pairs")
    if not moving:
        sys.exit("no moving pairs held out — enrich harder or widen --val-frac")

    off = forward(weights, feats[[a for a, _ in moving]])
    on = forward(weights, feats[[b for _, b in moving]])
    net_moved = (off.argmax(1) != on.argmax(1)).sum()
    agreed = sum(
        int(o.argmax() == targets[a].argmax() and n.argmax() == targets[b].argmax())
        for o, n, (a, b) in zip(off, on, moving)
    )
    logit_shift = float(np.abs(off - on).max(1).mean())

    print(f"net argmax moves on {net_moved}/{len(moving)} ({100 * net_moved / len(moving):.1f}%)")
    print(f"net matches the teacher on both halves: {agreed}/{len(moving)}")
    print(f"mean max |logit| shift across the flip: {logit_shift:.4f}")
    if net_moved == 0:
        sys.exit("FAIL: the net is blind to the card — gate 2 would be meaningless")


if __name__ == "__main__":
    main()
