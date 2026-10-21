#!/usr/bin/env python3
"""Read the 1NT-defense tournament: scripts/nt-defense-tournament.py RESULTS_DIR

Collects scripts/ab-nt-defense-tournament.sh's cells (row − pass, IMPs per
kept board, 95% CI) into one table per (vul, scorer) on the realism pair
[plain DD, SD-PD] and applies the runner's pre-registered rules 1–3: the
maximin row, whether it is resolved over every rival, and the card.  A
missing cell prints as `—` and leaves that (vul, scorer) unresolved.
"""

import math
import re
import sys
from pathlib import Path

ROWS = ["pass", "natural", "dont", "woolsey", "meckwell"]
MATRIX_LABEL = {"natural": "natural", "dont": "DONT(6+)", "woolsey": "Woolsey", "meckwell": "Meckwell"}
COLS = ["BBA", "BEN", "default", "penalty-X", "soft", "sit"]
VULS = ["none", "both", "ns", "ew"]
SCORERS = {"plain DD": ("Delta (direct A/B", "plain DD"), "SD-PD": ("Delta perfect-defense", "sd-lead + perfect defense")}
CELL = re.compile(r"([+-]?\d+\.\d+) IMPs/board \[95% CI ±(\d+\.\d+)\]")
MATRIX_CELL = re.compile(r"([+-]\d+\.\d+)±(\d+\.\d+)")


def harness_cell(path, prefix):
    try:
        for line in path.read_text().splitlines():
            if line.startswith(prefix):
                m = CELL.search(line)
                if m:
                    return float(m[1]), float(m[2])
    except FileNotFoundError:
        pass
    return None


def matrix_cells(path, section):
    """{row: [cell per self-play column]} from one `--- <section> ---` table"""
    try:
        lines = path.read_text().splitlines()
    except FileNotFoundError:
        return {}
    out, inside = {}, False
    for line in lines:
        if line.startswith("--- "):
            inside = line.startswith(f"--- {section}")
            continue
        if inside:
            label = line.split("  ")[0].strip()
            cells = MATRIX_CELL.findall(line)
            if cells:
                out[label] = [(float(a), float(b)) for a, b in cells]
    return out


def table(root, vul, scorer):
    harness_prefix, section = SCORERS[scorer]
    m = {r: {} for r in ROWS}
    selfplay = matrix_cells(root / "self" / f"matrix.{vul}.txt", section)
    for r in ROWS:
        if r == "pass":
            m[r] = {c: (0.0, 0.0) for c in COLS}
            continue
        for col, sub in (("BBA", "bba"), ("BEN", "ben")):
            name = f"diff.{r}.vs.pass.{vul}.plain.txt" if scorer == "plain DD" else f"sd.{r}.vs.pass.{vul}.txt"
            m[r][col] = harness_cell(root / sub / name, harness_prefix)
        for col, cell in zip(COLS[2:], selfplay.get(MATRIX_LABEL[r], [None] * 4)):
            m[r][col] = cell
    return m


def verdict(m):
    complete = {r: all(m[r].get(c) for c in COLS) for r in ROWS}
    value = {r: min(m[r][c][0] for c in COLS) if complete[r] else None for r in ROWS}
    worst = {r: min(COLS, key=lambda c: m[r][c][0]) if complete[r] else None for r in ROWS}
    ranked = [r for r in ROWS if complete[r]]
    if len(ranked) < len(ROWS):
        return value, None, False
    w = max(ranked, key=lambda r: value[r])

    def beats(rival):
        v_r, ci_r = m[rival][worst[rival]]
        return all(m[w][c][0] - v_r - math.hypot(m[w][c][1], ci_r) > 0 for c in COLS)

    return value, w, all(beats(r) for r in ROWS if r != w)


def main():
    root = Path(sys.argv[1])
    winners = set()
    for vul in VULS:
        for scorer in SCORERS:
            m = table(root, vul, scorer)
            print(f"=== vul {vul}, {scorer} (row − pass, IMPs per kept board) ===")
            print(f"{'':10}" + "".join(f"{c:>16}" for c in COLS) + f"{'worst':>10}")
            value, w, resolved = verdict(m)
            for r in ROWS:
                cells = "".join(
                    f"{m[r][c][0]:+9.3f}±{m[r][c][1]:.3f}" if m[r].get(c) else f"{'—':>16}" for c in COLS
                )
                v = f"{value[r]:+10.3f}" if value[r] is not None else f"{'—':>10}"
                print(f"{r:10}{cells}{v}")
            status = "resolved" if resolved else "UNRESOLVED"
            print(f"maximin: {w or '— (incomplete)'} ({status})\n")
            winners.add((w, resolved))
    if len(winners) == 1 and next(iter(winners))[1]:
        print(f"CARD: {next(iter(winners))[0]} (rule 3: resolved maximin in all eight cells)")
    else:
        print("CARD: none (rule 3 not met); the per-cell winners above go to jdh8")


if __name__ == "__main__":
    main()
