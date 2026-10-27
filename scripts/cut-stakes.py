#!/usr/bin/env python3
"""Two independent cuts of one shard at 128 layouts: how far apart, and what is at stake.

    scripts/cut-stakes.py [--margin X] CHUNK_DIR

Needs chunks stored at 128 layouts.  Cuts them six ways with `cut-noise.py`'s
gate — M = 32 on `[0, 64)` and on `[64, 128)` (two independent draws), each
also with its pools swapped, and M = 64 standard and swapped — and prints, per
pair, the labels moved as a share of all decisions and of the labels either
cut fired.  Then, for the labels two cuts disagree on, the gap between the
two labels' mean swings over all 128 layouts (is the gate breaking ties?),
and for each M = 32 cut's firings, their mean swing on the other half's 64
layouts, which that cut never saw (held-out precision), split by whether the
other half's cut repeats the label.

Reseed-gap pilot, axis-0004 ⊤ arm at `f8999e18` (docs/reseed-gap-handoff.md
§5 "Read 2026-10-27, 128 layouts"): independent M = 32 cuts move 9.8% of
decisions; the labels they disagree on differ by a median 0.86 IMP plain;
an M = 32 firing beats the own call on the unseen half 83% of the time.
"""

import argparse
import glob
import os
from importlib.machinery import SourceFileLoader

import numpy as np

cn = SourceFileLoader("cut_noise", os.path.join(os.path.dirname(__file__), "cut-noise.py")).load_module()

# name -> (M, select pool start, validate pool start)
POOLS = {
    "A32": (32, 0, 32),
    "A32s": (32, 32, 0),
    "B32": (32, 64, 96),
    "B32s": (32, 96, 64),
    "Q64": (64, 0, 64),
    "Q64s": (64, 64, 0),
}
PAIRS = [
    ("A32", "A32s"),
    ("B32", "B32s"),
    ("A32", "B32"),
    ("A32s", "B32s"),
    ("A32", "B32s"),
    ("A32s", "B32"),
    ("Q64", "Q64s"),
    ("Q64", "A32"),
    ("Q64", "B32"),
    ("Q64s", "A32"),
    ("Q64s", "B32"),
]


def mean_swing(rec, call, lo, hi):
    """Mean (plain, PD) swing of `call` over the own call on layouts `[lo, hi)`; the own call is 0."""
    _, cands, _, swings = rec
    if call == cands[0]:
        return np.zeros(2)
    k = cands.index(call) - 1
    return swings[k, lo:hi, :].astype(np.float64).mean(0)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--margin", type=float, default=0.25)
    ap.add_argument("root")
    a = ap.parse_args()
    recs = []
    for path in sorted(glob.glob(f"{a.root}/chunk-*.ret")):
        recs.extend(cn.read_ret(path))
    own = np.array([c[0] for _, c, _, _ in recs])
    lay = np.array([l for _, _, l, _ in recs])
    L = {k: np.array([cn.gate(c, l, w, m, a.margin, s, v) for _, c, l, w in recs]) for k, (m, s, v) in POOLS.items()}
    n = len(own)
    full = lay >= 128
    print(f"{n} decisions; {(lay >= 64).sum()} with >= 64 layouts, {full.sum()} with >= 128")
    print("fired:", {k: int((L[k] != own).sum()) for k in POOLS})
    for x, y in PAIRS:
        moved = (L[x] != L[y]).sum()
        fired = ((L[x] != own) | (L[y] != own)).sum()
        print(f"{x:>5} vs {y:<5}: moved {moved:6d} = {100 * moved / n:6.3f}% of decisions, {100 * moved / fired:5.1f}% of {fired} fired by either")
    for x in ("A32", "B32"):
        f = L[x] != own
        for q in ("Q64", "Q64s"):
            keep = f & (L[q] == L[x])
            print(f"of {f.sum()} {x} firings, {q} keeps {keep.sum()} ({100 * keep.sum() / f.sum():.1f}%)")
    for x, y, name in [("A32", "B32", "independent M = 32 halves"), ("Q64", "Q64s", "M = 64 swap")]:
        idx = np.nonzero((L[x] != L[y]) & full)[0]
        d = np.array([np.abs(mean_swing(recs[i], L[x][i], 0, 128) - mean_swing(recs[i], L[y][i], 0, 128)) for i in idx])
        for s, nm in ((0, "plain"), (1, "PD")):
            q = np.percentile(d[:, s], [25, 50, 75, 90])
            print(f"{name}: {len(idx)} moved; |EV(x) - EV(y)| over 128 layouts, {nm}: q25 {q[0]:.2f} med {q[1]:.2f} q75 {q[2]:.2f} q90 {q[3]:.2f} IMP; < 0.25: {100 * (d[:, s] < 0.25).mean():.0f}%, < 0.5: {100 * (d[:, s] < 0.5).mean():.0f}%, >= 1: {100 * (d[:, s] >= 1).mean():.0f}%")
    for x, other, lo, hi in (("A32", "B32", 64, 128), ("B32", "A32", 0, 64)):
        idx = np.nonzero((L[x] != own) & full)[0]
        h = np.array([mean_swing(recs[i], L[x][i], lo, hi) for i in idx])
        beats = (h[:, 0] > 0) & (h[:, 1] > 0)
        clears = (h[:, 0] > a.margin) & (h[:, 1] > a.margin)
        loses = (h[:, 0] < 0) & (h[:, 1] < 0)
        print(f"{x} on [{lo},{hi}): {len(idx)} fired; held-out mean swing plain {h[:, 0].mean():.3f} PD {h[:, 1].mean():.3f} IMP; beats own on both scorers {100 * beats.mean():.1f}%, clears margin on both {100 * clears.mean():.1f}%, loses on both {100 * loses.mean():.1f}%")
        agree = L[other][idx] == L[x][idx]
        for flag, nm in ((agree, "other half repeats the label"), (~agree, "other half does not")):
            hh = h[flag]
            print(f"    {nm}: {flag.sum()}; held-out mean plain {hh[:, 0].mean():.3f} PD {hh[:, 1].mean():.3f}; beats own on both {100 * ((hh[:, 0] > 0) & (hh[:, 1] > 0)).mean():.1f}%")


if __name__ == "__main__":
    main()
