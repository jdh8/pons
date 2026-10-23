#!/usr/bin/env python3
"""Rank the per-node IMP pools inside each loss bucket of a BBA anchor snapshot
with empirical-Bayes shrinkage, so a thin node with a huge per-board loss stops
outranking a fat node with a modest one (Plan C of docs/shrinkage-plan.md).

    python3 -I scripts/shrunk-pools.py SNAP/boards-american.jsonl
    python3 -I scripts/shrunk-pools.py SNAP/boards.jsonl --buckets 8 --nodes 10
    python3 -I scripts/shrunk-pools.py SNAP/boards.jsonl \\
        --bucket 'Defensive / book / round-1' --scorer pd
    python3 -I scripts/shrunk-pools.py --self-test
    python3 -I scripts/shrunk-pools.py SNAP/boards.jsonl --holdout

Input is a snapshot's `boards*.jsonl` (one row per contract-divergent board).
A **bucket** is `(phase, provenance, family)`, as the anchor report keys it; a
**node** inside a bucket is `(rule, our_call)` — `rule` is the authored rule's
constraint text for book rows and empty for floor rows.  `swing_plain` and
`swing_pd` are the board's IMPs for our side (negative = our loss).

Per bucket and per scorer, for nodes j with swings x_ij, counts n_j and sample
means μ̂_j:

    σ²  = Σ_j Σ_i (x_ij − μ̂_j)² / Σ_j (n_j − 1)       pooled within-node, n_j ≥ 2
                                                       (fallback: bucket variance)
    μ̄   = Σ x / Σ n_j                                  grand mean
    τ²  = DerSimonian–Laird: (Q − (k−1)) / (W − Σw²/W), w_j = n_j/σ²,
          Q = Σ w_j (μ̂_j − μ̄_w)², floored at 0        over nodes with n_j ≥ 2
    κ   = σ²/τ²                                        +∞ when τ² = 0
    μ_j^post = (n_j μ̂_j + κ μ̄)/(n_j + κ)
    pool_j   = n_j μ_j^post                            raw pool = n_j μ̂_j

`--tau2 mom` is the unweighted method of moments the plan first specified
(`Var_j(μ̂_j) − mean_j(σ²/n_j)`), `--tau2 pm` Paule–Mandel.  `--holdout` is how
the choice was made (2026-10-23): split every node's boards in two by deal, fit
on one half, predict the other half's node means, score the n-weighted squared
error against raw means and full pooling; DL predicted best in all four
arm × scorer cells (0.68–0.70 of raw; MoM 0.78, PM 0.75, pooling 0.75–0.84)
and by the most in the thin-node floor bucket MoM collapsed.

A node is *resolved* when its raw pool exceeds 1.96·σ·√n_j in magnitude.
Buckets are ordered by raw total plain pool (losses first); nodes by shrunk
pool on the ranking scorer.  Below each table the nodes the shrinkage demoted
out of / promoted into the raw top-M are listed.
"""

import argparse
import json
import math
import random
import sys
import zlib
from collections import defaultdict

SCORERS = {"plain": "swing_plain", "pd": "swing_pd"}
MINUS = "−"


def load(path, half=None):
    """bucket → node → list of (plain, pd); `half` 0/1 keeps one deal-parity half."""
    buckets = defaultdict(lambda: defaultdict(list))
    with open(path, encoding="utf-8") as f:
        for line in f:
            r = json.loads(line)
            if half is not None and zlib.crc32(f"{r['seed']}:{r['board']}".encode()) & 1 != half:
                continue
            bucket = f"{r['phase']} / {r['provenance']} / {r['family']}"
            buckets[bucket][(r["rule"], r["our_call"])].append(
                (float(r["swing_plain"]), float(r["swing_pd"])))
    return buckets


def tau2_of(multi, sigma2, method):
    """Between-node variance from [(n_j, μ̂_j)] with n_j ≥ 2 and the pooled σ²."""
    k = len(multi)
    if k < 2:
        return 0.0
    if method == "mom":
        mbar = sum(m for _, m in multi) / k
        var = sum((m - mbar) ** 2 for _, m in multi) / (k - 1)
        return max(0.0, var - sum(sigma2 / n for n, _ in multi) / k)
    if method == "dl":
        w = [n / sigma2 for n, _ in multi]
        total = sum(w)
        mw = sum(wi * m for wi, (_, m) in zip(w, multi)) / total
        q = sum(wi * (m - mw) ** 2 for wi, (_, m) in zip(w, multi))
        c = total - sum(wi * wi for wi in w) / total
        return max(0.0, (q - (k - 1)) / c) if c > 0 else 0.0
    if method == "pm":
        def q(t):
            w = [1 / (sigma2 / n + t) for n, _ in multi]
            mw = sum(wi * m for wi, (_, m) in zip(w, multi)) / sum(w)
            return sum(wi * (m - mw) ** 2 for wi, (_, m) in zip(w, multi))
        if q(0.0) <= k - 1:
            return 0.0
        lo, hi = 0.0, 1.0
        while q(hi) > k - 1 and hi < 1e9:
            hi *= 2
        for _ in range(60):
            mid = (lo + hi) / 2
            lo, hi = (mid, hi) if q(mid) > k - 1 else (lo, mid)
        return (lo + hi) / 2
    raise ValueError(method)


def shrink(nodes, col, method="dl"):
    """nodes: node → list of swing tuples; col: 0 plain, 1 pd.
    Returns (stats, per_node) with per_node[node] = dict(n, mean, raw, post, half)."""
    per = {}
    ss_sum = dof = 0.0
    total = count = 0.0
    for node, rows in nodes.items():
        xs = [r[col] for r in rows]
        n = len(xs)
        m = sum(xs) / n
        ss = sum((x - m) ** 2 for x in xs)
        per[node] = {"n": n, "mean": m, "ss": ss}
        total += n * m
        count += n
        if n >= 2:
            ss_sum += ss
            dof += n - 1
    grand = total / count
    if dof > 0:
        sigma2 = ss_sum / dof
    else:
        all_xs = [r[col] for rows in nodes.values() for r in rows]
        sigma2 = sum((x - grand) ** 2 for x in all_xs) / max(1, len(all_xs) - 1)
    multi = [(p["n"], p["mean"]) for p in per.values() if p["n"] >= 2]
    if method == "raw":
        kappa = 0.0
    elif method == "pooled":
        kappa = math.inf
    else:
        tau2 = tau2_of(multi, sigma2, method)
        kappa = sigma2 / tau2 if tau2 > 0 else math.inf
    tau2 = sigma2 / kappa if kappa > 0 else math.inf
    sigma = math.sqrt(sigma2)
    for p in per.values():
        n, m = p["n"], p["mean"]
        post = grand if math.isinf(kappa) else (n * m + kappa * grand) / (n + kappa)
        p["raw"] = n * m
        p["post"] = n * post
        p["half"] = 1.96 * sigma * math.sqrt(n)
    stats = {"sigma": sigma, "tau": math.sqrt(tau2) if math.isfinite(tau2) else math.inf,
             "kappa": kappa,
             "grand": grand, "n": int(count), "total": total}
    return stats, per


def num(x, digits=0):
    if math.isinf(x):
        return "∞"
    s = f"{abs(x):,.{digits}f}"
    return (MINUS if x < 0 and float(s.replace(",", "")) != 0 else "") + s


def label(node, width=50):
    rule, call = node
    rule = rule.replace("|", "/") if rule else "(floor)"
    if len(rule) > width:
        rule = rule[: width - 1] + "…"
    return f"`{call}` {rule}"


def holdout(path):
    """Split-half predictive check: fit each estimator on one deal-parity half,
    predict the other half's node means, sum n_B·(μ̂_B − μ_post)² over buckets
    and both directions; print the ratio to raw means per scorer."""
    halves = [load(path, 0), load(path, 1)]
    methods = ["raw", "mom", "dl", "pm", "pooled"]
    print("| scorer | " + " | ".join(methods) + " |")
    print("| --- |" + " --- |" * len(methods))
    for col, scorer in enumerate(SCORERS):
        err = dict.fromkeys(methods, 0.0)
        for a, b in ((0, 1), (1, 0)):
            for bucket, fit_nodes in halves[a].items():
                test_nodes = halves[b].get(bucket)
                if len(fit_nodes) < 2 or not test_nodes:
                    continue
                for m in methods:
                    stats, per = shrink(fit_nodes, col, m)
                    for node, rows in test_nodes.items():
                        xs = [r[col] for r in rows]
                        mean_b = sum(xs) / len(xs)
                        post = per[node]["post"] / per[node]["n"] if node in per else stats["grand"]
                        err[m] += len(xs) * (mean_b - post) ** 2
        print(f"| {scorer} | " + " | ".join(f"{err[m] / err['raw']:.3f}" for m in methods) + " |")


def report(name, nodes, scorer, top, method):
    plain_s, plain = shrink(nodes, 0, method)
    pd_s, pd = shrink(nodes, 1, method)
    rank_s, rank = (plain_s, plain) if scorer == "plain" else (pd_s, pd)
    print(f"## {name} — {plain_s['n']:,} boards, raw plain {num(plain_s['total'])}, "
          f"PD {num(pd_s['total'])}; {len(nodes)} nodes")
    print(f"plain: σ {plain_s['sigma']:.2f}, τ {plain_s['tau']:.3f}, κ {num(plain_s['kappa'], 1)} | "
          f"PD: σ {pd_s['sigma']:.2f}, τ {pd_s['tau']:.3f}, κ {num(pd_s['kappa'], 1)} | "
          f"ranked on {scorer}")
    print()
    by_shrunk = sorted(nodes, key=lambda k: rank[k]["post"])[:top]
    by_raw = sorted(nodes, key=lambda k: rank[k]["raw"])[:top]
    print("| # | node | n | mean plain | raw plain | shrunk plain | raw PD | shrunk PD | n+κ | res |")
    print("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |")
    for i, k in enumerate(by_shrunk, 1):
        p, q, r = plain[k], pd[k], rank[k]
        resolved = "✓" if abs(r["raw"]) > r["half"] else "·"
        print(f"| {i} | {label(k)} | {p['n']:,} | {num(p['mean'], 2)} | {num(p['raw'])} | "
              f"{num(p['post'])} | {num(q['raw'])} | {num(q['post'])} | "
              f"{num(p['n'] + rank_s['kappa'])} | {resolved} |")
    moved = [("demoted", [k for k in by_raw if k not in by_shrunk]),
             ("promoted", [k for k in by_shrunk if k not in by_raw])]
    for what, keys in moved:
        items = ", ".join(f"{label(k, 30)} (n {rank[k]['n']:,}, raw {num(rank[k]['raw'])} → "
                          f"{num(rank[k]['post'])})" for k in keys) or "none"
        print(f"\n{what}: {items}")
    print()


def self_test():
    rng = random.Random(20261023)
    nodes = {("thin", "A"): [(rng.gauss(-33, 6),) * 2 for _ in range(12)],
             ("fat", "B"): [(rng.gauss(-0.9, 6),) * 2 for _ in range(1500)]}
    for j in range(20):
        nodes[(f"noise {j}", "C")] = [(rng.gauss(0, 6),) * 2 for _ in range(200)]
    stats, per = shrink(nodes, 0)
    thin, fat = per[("thin", "A")], per[("fat", "B")]
    first = min(nodes, key=lambda k: per[k]["post"])
    print(f"σ {stats['sigma']:.2f}, τ {stats['tau']:.2f}, κ {num(stats['kappa'], 2)}")
    print(f"thin: n 12, raw {num(thin['raw'], 1)}, shrunk {num(thin['post'], 1)}")
    print(f"fat: n 1,500, raw {num(fat['raw'], 1)}, shrunk {num(fat['post'], 1)}")
    ok = first == ("fat", "B") and thin["post"] > thin["raw"] + 10 \
        and abs(fat["post"] - fat["raw"]) < 0.01 * abs(fat["raw"])
    print("self-test", "ok" if ok else "FAILED")
    return 0 if ok else 1


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("boards", nargs="?", help="a snapshot's boards*.jsonl")
    ap.add_argument("--buckets", type=int, default=8, help="buckets to print (default 8)")
    ap.add_argument("--nodes", type=int, default=10, help="nodes per bucket (default 10)")
    ap.add_argument("--bucket", help="print only this bucket, e.g. 'Defensive / book / round-1'")
    ap.add_argument("--scorer", choices=SCORERS, default="plain", help="ranking scorer")
    ap.add_argument("--tau2", choices=["dl", "mom", "pm"], default="dl",
                    help="between-node variance estimator (default dl)")
    ap.add_argument("--holdout", action="store_true",
                    help="split-half predictive check of every estimator instead of the ranking")
    ap.add_argument("--self-test", action="store_true", help="synthetic check, no file")
    args = ap.parse_args()
    if args.self_test:
        return self_test()
    if not args.boards:
        ap.error("boards*.jsonl required (or --self-test)")
    if args.holdout:
        holdout(args.boards)
        return 0
    buckets = load(args.boards)
    if args.bucket:
        if args.bucket not in buckets:
            sys.exit(f"no bucket {args.bucket!r}; have: " + ", ".join(sorted(buckets)))
        names = [args.bucket]
    else:
        total = {b: sum(r[0] for rows in n.values() for r in rows) for b, n in buckets.items()}
        names = sorted(buckets, key=total.get)[: args.buckets]
    for name in names:
        report(name, buckets[name], args.scorer, args.nodes, args.tau2)
    return 0


if __name__ == "__main__":
    sys.exit(main())
