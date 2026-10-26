#!/usr/bin/env python3
"""The cut gate's own noise: re-cut a shard's `.ret` files with the pools moved.

    scripts/cut-noise.py [-m M] [--margin X] CHUNK_DIR...

`dump-teacher --cut M` selects a winner on layouts `[0, M)` and validates it on
`[M, 2M)`.  The same returns cut with the pools **swapped** (`[M, 2M)` selects,
`[0, M)` validates) are a second draw of the same decisions under the same
book, so the share of labels that differ is a floor under any label diff
read across two cuts — a relabel at another commit moves at least this many.
A swap shares its layouts between the two cuts; two **independent** draws
differ more, and `[0, M/2)/[M/2, M)` against `[M, 3M/2)/[3M/2, 2M)` measures
that at half depth, so `independent / swap` at `M/2` scales the swap at `M`.

Reseed-gap pilot, axis-0004 at `d705cc94` (docs/reseed-gap-handoff.md §5):
M = 32 swap moves 4.7% of decisions, M = 16 independent 11.3%, ratio 2.3.
"""

import argparse
import glob
import struct

import numpy as np


def read_ret(path):
    """[(row, candidates, layouts, swings[alt][layout][scorer])] of one chunk."""
    b = open(path, "rb").read()
    assert b[:4] == b"PRET", f"{path}: not a .ret file"
    (version, n) = struct.unpack_from("<II", b, 4)
    assert version == 1, f"{path}: .ret version {version}"
    at, out = 12, []
    for _ in range(n):
        (row,) = struct.unpack_from("<I", b, at)
        at += 4
        nc = b[at]
        at += 1
        cands = list(b[at : at + nc])
        at += nc
        (lay,) = struct.unpack_from("<H", b, at)
        at += 2
        count = (nc - 1) * lay
        swings = np.frombuffer(b, dtype=np.int8, count=2 * count, offset=at).reshape(nc - 1, lay, 2)
        at += 2 * count
        out.append((row, cands, lay, swings))
    assert at == len(b), f"{path}: trailing bytes"
    return out


def gate(cands, lay, swings, m, margin, select, validate):
    """`relabel::relabel` with the select pool at `[select, select + m)` and the
    validate pool at `[validate, validate + m)`; the own call when it does not fire."""
    own = cands[0]
    if lay < 2 * m:
        return own
    first = swings[:, select : select + m, :].astype(np.int64).sum(1) / m
    second = swings[:, validate : validate + m, :].astype(np.int64).sum(1) / m
    winner, best, held = [own, own], [0.0, 0.0], [0.0, 0.0]
    for c in range(len(cands) - 1):
        for scorer in range(2):
            if first[c, scorer] > best[scorer]:
                best[scorer], held[scorer], winner[scorer] = first[c, scorer], second[c, scorer], cands[c + 1]
    fires = winner[0] == winner[1] and winner[0] != own and held[0] > margin and held[1] > margin
    return winner[0] if fires else own


def cuts(roots, m, margin, pools):
    """Per pool `(select, validate)`, the label of every decision; plus the own calls."""
    own, labels = [], {p: [] for p in pools}
    for root in roots:
        for path in sorted(glob.glob(f"{root}/chunk-*.ret")):
            for _, cands, lay, swings in read_ret(path):
                own.append(cands[0])
                for p in pools:
                    labels[p].append(gate(cands, lay, swings, m, margin, *p))
    return np.array(own), {p: np.array(v) for p, v in labels.items()}


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("-m", type=int, default=32)
    ap.add_argument("--margin", type=float, default=0.25)
    ap.add_argument("roots", nargs="+")
    a = ap.parse_args()
    m, h = a.m, a.m // 2
    own, L = cuts(a.roots, m, a.margin, [(0, m), (m, 0)])
    n = len(own)
    std, swp = L[(0, m)], L[(m, 0)]
    fired = (std != own) | (swp != own)
    moved = std != swp
    print(f"{n} decisions; M={m} fired {(std != own).sum()} standard, {(swp != own).sum()} swapped, both {((std != own) & (swp != own)).sum()}")
    print(f"  swap moved {moved.sum()} = {100 * moved.mean():.3f}% of decisions, {100 * moved.sum() / max(1, fired.sum()):.1f}% of labels fired by either")
    _, H = cuts(a.roots, h, a.margin, [(0, h), (h, 0), (m, m + h), (m + h, m)])
    swap_h = ((H[(0, h)] != H[(h, 0)]).sum() + (H[(m, m + h)] != H[(m + h, m)]).sum()) / 2
    indep_h = ((H[(0, h)] != H[(m, m + h)]).sum() + (H[(h, 0)] != H[(m + h, m)]).sum()) / 2
    ratio = indep_h / max(1, swap_h)
    print(f"  M={h}: swap moved {100 * swap_h / n:.3f}%, independent draws {100 * indep_h / n:.3f}%, ratio {ratio:.2f}")
    print(f"  estimated independent-draw noise at M={m}: {100 * moved.mean() * ratio:.1f}% of decisions (bounded by the M={h} independent figure)")


if __name__ == "__main__":
    main()
