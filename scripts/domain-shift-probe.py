#!/usr/bin/env python3
"""How far apart two readers' corpora are: the portable-judgement Phase 0 read.

    scripts/domain-shift-probe.py A_CHUNK_ROOT B_CHUNK_ROOT

Both roots are re-walks of the same windows (`<shard>/chunk-<c>`, v6 rows,
`dump-teacher --relabel --layouts 0`) under two readers, e.g. `--system
american` and `--system bba` (docs/portable-judgement.md §4 Phase 0).  Per
phase (the `.tags` byte: constructive, contested) it prints

1. column drift: the inference columns whose paired rows move most, in
   natural units (lengths in cards, points in HCP), with each reader's mean ± sd;
2. a domain discriminator: logistic regression on the inference block alone,
   label = which reader, AUROC on a 10% hold-out, once over paired rows (the
   same decision read two ways) and once over all rows; then the paired AUROC
   over column subsets, which says *which* gauges separate the readers;
3. label drift: the teacher's argmax agreement on paired rows.

Rows pair within a chunk on the reading-free key (hand block, auction-context
block, vulnerability, DD table); a key seen twice on either side is dropped.
v6 layout: hand 0:10, context 10:46, inferences 46:118 (seats Me, Lho,
Partner, Rho × 18: four suits' length {min, max}, points {min, max}, four
suits' support points {min, max}), vul 118:120, compact 120:176, teacher
softmax 176:214, DD 214:234.  Writes `domain-shift-probe.json` beside
A_CHUNK_ROOT.  Needs numpy (`python3.9` on Compute-Node02).
"""
import glob
import json
import os
import sys

import numpy as np

ROW = 234
INF = slice(46, 118)
KEY = np.r_[0:46, 118:120, 214:234]
SOFT = slice(176, 214)
PHASES = ("constructive", "contested")
SAMPLE = 400_000  # rows per side per phase the discriminators fit on, at most
KEEP = float(os.environ.get("PROBE_KEEP", "0.05"))  # share of rows sampled for the discriminators

SEATS = ("me", "lho", "pd", "rho")
FIELDS = [f"{s}{e}" for s in "CDHS" for e in ("min", "max")] + ["pts_min", "pts_max"] + [
    f"sup{s}_{e}" for s in "CDHS" for e in ("min", "max")
]
NAMES = [f"{seat}.{f}" for seat in SEATS for f in FIELDS]
_SUP = np.array(["sup" in n for n in NAMES])
_PTS = np.array(["pts" in n for n in NAMES])
SUBSETS = {
    "lengths": ~_SUP & ~_PTS,
    "points": _PTS,
    "support": _SUP,
    "lengths+points": ~_SUP,
    "lengths+support": ~_PTS,
    "points+support": _SUP | _PTS,
}
SCALE = np.array(([13.0] * 8 + [37.0] * 10) * 4)


def load(stem):
    rows = np.fromfile(stem + ".f32", dtype="<f4").reshape(-1, ROW)
    tags = np.fromfile(stem + ".tags", dtype=np.uint8)
    assert len(tags) == len(rows), stem
    return rows, tags


def unique_keys(rows):
    keys = np.ascontiguousarray(rows[:, KEY]).view(np.dtype((np.void, 4 * len(KEY)))).ravel()
    _, inv, counts = np.unique(keys, return_inverse=True, return_counts=True)
    ok = counts[inv] == 1
    return {k.tobytes(): i for i, k in zip(np.flatnonzero(ok), keys[ok])}


class Moments:
    def __init__(self):
        self.n = 0
        self.s = np.zeros(72)
        self.ss = np.zeros(72)

    def add(self, x):
        x = x.astype(np.float64) * SCALE
        self.n += len(x)
        self.s += x.sum(0)
        self.ss += (x * x).sum(0)

    def mean_sd(self):
        m = self.s / max(self.n, 1)
        return m, np.sqrt(np.maximum(self.ss / max(self.n, 1) - m * m, 0))


def auroc(score, y):
    """Mann-Whitney U with average ranks for ties."""
    _, inv, counts = np.unique(score, return_inverse=True, return_counts=True)
    upper = np.cumsum(counts)
    ranks = (upper - (counts - 1) / 2)[inv]
    n1 = y.sum()
    n0 = len(y) - n1
    return (ranks[y == 1].sum() - n1 * (n1 + 1) / 2) / (n1 * n0)


def logistic(x, y, l2=1e-3, iters=25):
    """Newton / IRLS on standardized inputs; returns a scorer."""
    mu = x.mean(0)
    sd = x.std(0)
    sd[sd == 0] = 1
    z = np.c_[np.ones(len(x)), (x - mu) / sd]
    w = np.zeros(z.shape[1])
    for _ in range(iters):
        p = 1 / (1 + np.exp(-np.clip(z @ w, -30, 30)))
        g = z.T @ (p - y) + l2 * np.r_[0, w[1:]]
        h = (z * (p * (1 - p))[:, None]).T @ z + l2 * np.eye(len(w))
        step = np.linalg.solve(h, g)
        w -= step
        if np.abs(step).max() < 1e-6:
            break
    return lambda q: np.c_[np.ones(len(q)), (q - mu) / sd] @ w, w[1:]


def discriminate(a, b, rng, paired):
    """a, b: inference blocks from each reader.  paired: rows i of a and b are one pair."""
    n = min(len(a), len(b))
    if n < 1000:
        return None
    if paired:
        idx = rng.permutation(n)
        held = np.zeros(n, bool)
        held[idx[: n // 10]] = True
        ha, hb, ta, tb = a[held], b[held], a[~held], b[~held]
    else:
        ia, ib = rng.permutation(len(a))[:n], rng.permutation(len(b))[:n]
        a, b = a[ia], b[ib]
        cut = n // 10
        ha, hb, ta, tb = a[:cut], b[:cut], a[cut:], b[cut:]
    x = np.r_[ta, tb].astype(np.float64)
    y = np.r_[np.zeros(len(ta)), np.ones(len(tb))]
    score, _ = logistic(x, y)
    hx = np.r_[ha, hb].astype(np.float64)
    hy = np.r_[np.zeros(len(ha)), np.ones(len(hb))].astype(int)
    return auroc(score(hx), hy), n


def main(dir_a, dir_b):
    rng = np.random.default_rng(7)
    stems = sorted(p[: -len(".json")] for p in glob.glob(os.path.join(dir_a, "*", "chunk-*.json")))
    mom = {(side, ph): Moments() for side in "ab" for ph in (0, 1)}
    absdiff = {ph: np.zeros(72) for ph in (0, 1)}
    moved = {ph: 0 for ph in (0, 1)}
    agree = {ph: 0 for ph in (0, 1)}
    npair = {ph: 0 for ph in (0, 1)}
    nrows = {(side, ph): 0 for side in "ab" for ph in (0, 1)}
    keep_pair = {ph: ([], []) for ph in (0, 1)}
    keep_all = {(side, ph): [] for side in "ab" for ph in (0, 1)}
    total_rows = {"a": 0, "b": 0}
    for stem_a in stems:
        rel = os.path.relpath(stem_a, dir_a)
        stem_b = os.path.join(dir_b, rel)
        if not os.path.exists(stem_b + ".json"):
            print(f"missing {rel} under {dir_b}", file=sys.stderr)
            continue
        ra, ta = load(stem_a)
        rb, tb = load(stem_b)
        total_rows["a"] += len(ra)
        total_rows["b"] += len(rb)
        for side, r, t in (("a", ra, ta), ("b", rb, tb)):
            for ph in (0, 1):
                sel = r[t == ph, INF]
                nrows[side, ph] += len(sel)
                if len(sel):
                    keep_all[side, ph].append(sel[rng.random(len(sel)) < KEEP])
        ka, kb = unique_keys(ra), unique_keys(rb)
        common = ka.keys() & kb.keys()
        ia = np.fromiter((ka[k] for k in common), dtype=np.int64, count=len(common))
        ib = np.fromiter((kb[k] for k in common), dtype=np.int64, count=len(common))
        for ph in (0, 1):
            m = ta[ia] == ph
            xa, xb = ra[ia[m]], rb[ib[m]]
            mom["a", ph].add(xa[:, INF])
            mom["b", ph].add(xb[:, INF])
            d = np.abs(xa[:, INF].astype(np.float64) - xb[:, INF]) * SCALE
            absdiff[ph] += d.sum(0)
            moved[ph] += int((d > 0).any(1).sum())
            agree[ph] += int((xa[:, SOFT].argmax(1) == xb[:, SOFT].argmax(1)).sum())
            npair[ph] += int(m.sum())
            take = rng.random(int(m.sum())) < KEEP
            keep_pair[ph][0].append(xa[take][:, INF])
            keep_pair[ph][1].append(xb[take][:, INF])

    out = {"chunks": len(stems), "rows": total_rows, "phases": {}}
    print(f"chunks {len(stems)}; rows american {total_rows['a']:,}, bba {total_rows['b']:,}")
    for ph in (0, 1):
        name = PHASES[ph]
        n = npair[ph]
        ma, sa = mom["a", ph].mean_sd()
        mb, sb = mom["b", ph].mean_sd()
        mad = absdiff[ph] / max(n, 1)
        top = np.argsort(-mad)[:15]
        pa, pb = np.concatenate(keep_pair[ph][0]), np.concatenate(keep_pair[ph][1])
        pr = discriminate(pa[:SAMPLE], pb[:SAMPLE], rng, paired=True)
        subsets = {
            k: discriminate(pa[:SAMPLE, cols], pb[:SAMPLE, cols], np.random.default_rng(3), paired=True)
            for k, cols in SUBSETS.items()
        }
        aa = np.concatenate(keep_all["a", ph])[: SAMPLE]
        ab = np.concatenate(keep_all["b", ph])[: SAMPLE]
        al = discriminate(aa, ab, rng, paired=False)
        rec = {
            "rows_american": nrows["a", ph],
            "rows_bba": nrows["b", ph],
            "paired": n,
            "paired_share_of_american": n / max(nrows["a", ph], 1),
            "moved_share": moved[ph] / max(n, 1),
            "label_agreement": agree[ph] / max(n, 1),
            "auroc_paired": pr[0] if pr else None,
            "auroc_paired_n": pr[1] if pr else 0,
            "auroc_all": al[0] if al else None,
            "auroc_all_n": al[1] if al else 0,
            "auroc_paired_by_subset": {k: (v[0] if v else None) for k, v in subsets.items()},
            "top_columns": [
                {
                    "col": NAMES[i],
                    "mean_abs_diff": float(mad[i]),
                    "american_mean": float(ma[i]),
                    "american_sd": float(sa[i]),
                    "bba_mean": float(mb[i]),
                    "bba_sd": float(sb[i]),
                }
                for i in top
            ],
        }
        out["phases"][name] = rec
        print(f"\n== {name}: rows american {rec['rows_american']:,} bba {rec['rows_bba']:,}; "
              f"paired {n:,} ({rec['paired_share_of_american']:.1%} of american)")
        print(f"   moved {rec['moved_share']:.1%}  label agreement {rec['label_agreement']:.2%}  "
              f"AUROC paired {rec['auroc_paired']:.3f}  all {rec['auroc_all']:.3f}")
        print("   paired AUROC by gauge: " + "  ".join(
            f"{k} {v:.3f}" for k, v in rec["auroc_paired_by_subset"].items() if v is not None))
        print(f"   {'column':16s} {'|Δ|':>6s}  {'am mean±sd':>14s}  {'bba mean±sd':>14s}")
        for c in rec["top_columns"]:
            print(f"   {c['col']:16s} {c['mean_abs_diff']:6.2f}  {c['american_mean']:6.2f}±{c['american_sd']:<6.2f}"
                  f"  {c['bba_mean']:6.2f}±{c['bba_sd']:<6.2f}")
    with open(os.path.join(os.path.dirname(dir_a.rstrip("/")), "domain-shift-probe.json"), "w") as f:
        json.dump(out, f, indent=1)


if __name__ == "__main__":
    main(*sys.argv[1:])
