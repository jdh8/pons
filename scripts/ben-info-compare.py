#!/usr/bin/env python3
"""Rank where our reading layer disagrees with reality and with BEN's Info net.

Input: probe-ben-info jsonl, optionally annotated by ben-info-dump.py with a
`ben` field per row ({lho,partner,rho}: {hcp: float, shape: [C,D,H,S]}).
Suit order everywhere is ascending C,D,H,S (Inference.lengths order).

Three reports, worst first:
  1. truth violations — the actual hidden hand falls OUTSIDE our shown band.
     Self-play is honest, so every violation is a reading bug (a call decoded
     wrong), not a psyche.
  2. BEN-vs-us (needs `ben`) — BEN's mean falls outside our band by a margin
     beyond noise+scale slack. Our `points` is the upgraded scale (raw HCP
     only when balanced): HCP slack 2, length slack 1.25.
  3. vagueness (needs `ben`) — our band is uninformative (full range) while
     BEN commits away from the deal prior: signal we're not reading at all
     (e.g. passes narrow nothing).

Usage: scripts/ben-info-compare.py probe.jsonl [--top 25]
"""

import argparse
import json
import sys
from collections import defaultdict

SUITS = "CDHS"
SEATS = ("lho", "partner", "rho")
HCP_SLACK = 2.0
LEN_SLACK = 1.25
FULL_POINTS = (0, 37)
FULL_LENGTH = (0, 13)


def band(rng):
    return rng["min"], rng["max"]


def outside(value, lo, hi, slack=0.0):
    """How far value falls outside [lo-slack, hi+slack] (0 = inside)."""
    return max(0.0, (lo - slack) - value, value - (hi + slack))


def key(row, seat):
    return (" ".join(row["auction"]), seat)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("jsonl")
    ap.add_argument("--top", type=int, default=25)
    args = ap.parse_args()

    rows = [json.loads(line) for line in open(args.jsonl)]
    have_ben = rows and "ben" in rows[0]

    # (auction, seat) -> [count, sum margin, example row index, what]
    truth_viol = defaultdict(lambda: [0, 0.0, None, ""])
    ben_viol = defaultdict(lambda: [0, 0.0, None, ""])
    vague = defaultdict(lambda: [0, 0.0, None, ""])
    seen = defaultdict(int)  # (auction, seat) -> rows, for rates

    for i, row in enumerate(rows):
        for seat in SEATS:
            ours, truth = row["ours"][seat], row["truth"][seat]
            k = key(row, seat)
            seen[k] += 1
            plo, phi = band(ours["points"])

            # 1. truth violations (points compare on the upgraded point_count
            # scale the bands are denominated in; fit-known support_points can
            # legitimately exceed it on raises — small residual noise there.
            # Both bounds are checked: floors catch an over-promising read,
            # ceilings an over-tight pass band (set_pass_reading).  Lengths
            # compare exactly).
            m = (
                0.0
                if (plo, phi) == FULL_POINTS
                else max(0.0, plo - truth["points"], truth["points"] - phi)
            )
            what = (
                f"points {truth['points']} (hcp {truth['hcp']}) outside shown {plo}-{phi}"
                if m
                else ""
            )
            for s in range(4):
                llo, lhi = band(ours["lengths"][s])
                lm = outside(truth["lengths"][s], llo, lhi)
                if lm > m:
                    m, what = lm, f"{SUITS[s]} len {truth['lengths'][s]} vs shown {llo}-{lhi}"
            if m > 0:
                e = truth_viol[k]
                e[0] += 1
                e[1] += m
                if e[2] is None:
                    e[2], e[3] = i, what

            if not have_ben:
                continue
            ben = row["ben"][seat]

            # 2. BEN's mean outside our band
            m = outside(ben["hcp"], plo, phi, HCP_SLACK)
            what = f"BEN hcp {ben['hcp']:.1f} vs shown {plo}-{phi}"
            for s in range(4):
                llo, lhi = band(ours["lengths"][s])
                lm = outside(ben["shape"][s], llo, lhi, LEN_SLACK)
                if lm > m:
                    m, what = lm, (
                        f"BEN {SUITS[s]} len {ben['shape'][s]:.1f} vs shown {llo}-{lhi}"
                    )
            if m > 0:
                e = ben_viol[k]
                e[0] += 1
                e[1] += m
                if e[2] is None:
                    e[2], e[3] = i, what

            # 3. we read nothing, BEN commits away from the prior (10 hcp)
            if (plo, phi) == FULL_POINTS and all(
                band(ours["lengths"][s]) == FULL_LENGTH for s in range(4)
            ):
                dev = abs(ben["hcp"] - 10.0)
                if dev >= 3.0:
                    e = vague[k]
                    e[0] += 1
                    e[1] += dev
                    if e[2] is None:
                        e[2], e[3] = i, f"BEN hcp {ben['hcp']:.1f}, we show nothing"

    def report(title, table):
        print(f"\n## {title}\n")
        ranked = sorted(table.items(), key=lambda kv: -kv[1][1])[: args.top]
        if not ranked:
            print("(none)")
        for (auction, seat), (n, tot, i, what) in ranked:
            r = rows[i]
            print(
                f"{tot:8.1f}  {n:4}/{seen[(auction, seat)]:<4} "
                f"[{auction or '(open)'}] {seat} of {r['seat']}  "
                f"e.g. board {r['board']}: {what}"
            )

    print(f"{len(rows)} rows, ben annotations: {have_ben}")
    report("Truth violations (definite reading bugs), by total margin", truth_viol)
    if have_ben:
        report("BEN outside our shown band, by total margin", ben_viol)
        report("We show nothing, BEN commits (vagueness), by total deviation", vague)


if __name__ == "__main__":
    sys.exit(main())
