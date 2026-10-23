#!/usr/bin/env python3
"""Seed-noise ruler for floor retrains (docs/shrinkage-plan.md § Plan B).

Reads two or more `ab-lib` arm directories (their `shard-*.json` dumps, bid
over the SAME deals and seed) and reports, per prefix key where our side was
on call, the total variation between the arms' next-call distributions,
weighted by reach.  Two training seeds of one recipe set the floor; the
shipped-vs-retrained number, localized by key, says whether a retrain moved
macro shape or only traded indifferent keys.

A key is the full auction prefix (both sides' calls, space-delimited as the
dumps write them) at a position where our side chose the next call.  Our
side is North/South at table A and East/West at table B (bba-gen's module
doc), so both tables contribute.  The dealer is not part of the key: given
that we are on call after the prefix, the sides alternate backwards from
there, so the four dealer rotations are one situation (the arms are bid at
one symmetric vulnerability, `none` or `both`).

    TV_k      = ½ Σ_c |p_k(c) − q_k(c)|          (next-call distributions at k)
    reach_k   = n_k / Σ n                        (n_k = arm-pooled visits of k)
    TV        = Σ_k reach_k · TV_k               (the headline, in [0, 1])

Usage:
    scripts/ruler-tv.py ab-results/ruler/seed2-none ab-results/ruler/seed3-none
    scripts/ruler-tv.py A B [C ...] [--min-n 50] [--show 25] [--depth 4]

With three or more arms every pair is reported; name the pair you read as the
floor first.  `--depth` caps keys at that many of OUR calls (the tail is
sparse and only adds noise to the ranking, not to the headline).
"""
import argparse
import glob
import json
import os
import sys
from collections import Counter, defaultdict

SEATS = ["North", "East", "South", "West"]


def keys_of(auction: str, dealer: str, ours_even: bool, depth: int):
    """Yield (prefix_key, call) at each position where our side was on call."""
    calls = auction.split()
    d = SEATS.index(dealer)
    depth_seen = 0
    for i, call in enumerate(calls):
        seat = (d + i) % 4
        if (seat % 2 == 0) == ours_even:
            if depth and depth_seen >= depth:
                return
            depth_seen += 1
            yield " ".join(calls[:i]), call


def shard_index(f: str) -> int:
    return int(os.path.basename(f)[len("shard-"):-len(".json")])


def load_arm(path: str, depth: int, parity=None):
    """Return {key: Counter(call)} and the board count for one arm dir.

    `parity` 0/1 keeps only even/odd shard indices: the two halves of one arm
    are bid over disjoint deals, and shard i holds the same deals in every arm
    of a results dir, so half-vs-half comparisons across arms are equal-sample.
    """
    dist = defaultdict(Counter)
    boards = 0
    files = sorted(glob.glob(os.path.join(path, "shard-*.json")))
    if parity is not None:
        files = [f for f in files if shard_index(f) % 2 == parity]
    if not files:
        sys.exit(f"{path}: no shard-*.json")
    for f in files:
        with open(f) as fh:
            dump = json.load(fh)
        for b in dump["boards"]:
            boards += 1
            for table, ours_even in (("table_a", True), ("table_b", False)):
                for k, c in keys_of(b[table], b["dealer"], ours_even, depth):
                    dist[k][c] += 1
    return dist, boards


def tv(p: Counter, q: Counter) -> float:
    np_, nq = sum(p.values()), sum(q.values())
    return 0.5 * sum(abs(p[c] / np_ - q[c] / nq) for c in set(p) | set(q))


def compare_quiet(a, b) -> float:
    keys = set(a) | set(b)
    total = sum(sum(a[k].values()) + sum(b[k].values()) for k in keys)
    return sum((sum(a[k].values()) + sum(b[k].values())) / total
               * (tv(a[k], b[k]) if a[k] and b[k] else 1.0) for k in keys)


def compare(name_a, a, name_b, b, min_n, show):
    keys = set(a) | set(b)
    total = sum(sum(a[k].values()) + sum(b[k].values()) for k in keys)
    rows = []
    head = 0.0
    for k in keys:
        n = sum(a[k].values()) + sum(b[k].values())
        reach = n / total
        if not a[k] or not b[k]:
            # One arm never reached the node: the whole mass moved upstream and
            # is already charged at the parent key.  Count as TV 1 at this reach.
            t = 1.0
        else:
            t = tv(a[k], b[k])
        head += reach * t
        if n >= min_n:
            rows.append((reach * t, t, n, k))
    rows.sort(reverse=True)
    print(f"\n== {name_a} vs {name_b}: reach-weighted TV = {head:.5f} "
          f"over {len(keys)} keys, {total} visits (both arms pooled)")
    print(f"   top {show} keys by reach·TV (n ≥ {min_n}):")
    print(f"   {'reach·TV':>9} {'TV':>6} {'n':>7}  prefix")
    for w, t, n, prefix in rows[:show]:
        print(f"   {w:9.5f} {t:6.3f} {n:7d}  {prefix or '(opening)'}")
    return head


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("arms", nargs="+", help="ab-lib arm directories (shard-*.json)")
    ap.add_argument("--min-n", type=int, default=50, help="pooled visits for a key to be listed")
    ap.add_argument("--show", type=int, default=25)
    ap.add_argument("--depth", type=int, default=0, help="cap at this many of our calls (0 = all)")
    ap.add_argument("--halves", action="store_true",
                    help="sampling baseline: TV(even half, odd half) within each arm, "
                         "against TV(even half of A, odd half of B) across arms, all at half sample")
    args = ap.parse_args()
    if len(args.arms) < 2:
        ap.error("need at least two arms")
    if args.halves:
        halves = []
        for p in args.arms:
            ev, _ = load_arm(p, args.depth, 0)
            od, _ = load_arm(p, args.depth, 1)
            halves.append((os.path.basename(os.path.normpath(p)), ev, od))
        print("== sampling baseline (half vs half, equal sample everywhere)")
        for n, ev, od in halves:
            print(f"   {n} even vs {n} odd (same net):   {compare_quiet(ev, od):.5f}")
        for i in range(len(halves)):
            for j in range(len(halves)):
                if i != j:
                    print(f"   {halves[i][0]} even vs {halves[j][0]} odd:   "
                          f"{compare_quiet(halves[i][1], halves[j][2]):.5f}")
        return
    arms = []
    for p in args.arms:
        dist, boards = load_arm(p, args.depth)
        print(f"{p}: {boards} boards, {len(dist)} keys")
        arms.append((os.path.basename(os.path.normpath(p)), dist))
    heads = []
    for i in range(len(arms)):
        for j in range(i + 1, len(arms)):
            heads.append((arms[i][0], arms[j][0],
                          compare(arms[i][0], arms[i][1], arms[j][0], arms[j][1],
                                  args.min_n, args.show)))
    print("\n== summary (reach-weighted TV)")
    for a, b, h in heads:
        print(f"   {a} vs {b}: {h:.5f}")


if __name__ == "__main__":
    main()
