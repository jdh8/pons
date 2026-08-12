"""Classify an A/B's fired boards by final-contract shape — no DD solve.

    python3 scripts/ab-classify.py <arm-a-dir> <arm-b-dir>

Both arguments are arm dump directories of `shard-*.json` files (the retained
output of an A/B runner, e.g. `ab-results/<experiment>/on-none`).  Boards are
paired by position; a board "fires" when the two arms' auctions differ, which is
a wider net than `ab-dump-diff`'s score-diverged count.  For each fired board
the final contract of each arm is tagged ON/OFF (first argument = ON) and
counted by declaring side, doubling, and level.  NS is the side carrying the
knob; declarer seat 0/2 == NS.  Pure JSON, ~3 s over 409,600 boards.

Wrote the population tables in docs/ai-bidder/doubling-calibration.md.
"""
import json, sys, glob, os, collections

SEATS = ["North", "East", "South", "West"]
STRAIN = {"♣": 0, "♦": 1, "♥": 2, "♠": 3, "NT": 4}


def final(auction, dealer):
    """(level, strain, dbl, declarer_seat_index) or None for passout."""
    calls = auction.split()
    d0 = SEATS.index(dealer)
    last, dbl, strains = None, 0, {}
    for i, c in enumerate(calls):
        seat = (d0 + i) % 4
        if c == "-":
            continue
        if c == "X":
            dbl = 1
            continue
        if c == "XX":
            dbl = 2
            continue
        lvl, st = int(c[0]), c[1:]
        last, dbl = (lvl, st, seat), 0
        strains.setdefault((seat % 2, st), seat)  # first of that side to name it
    if last is None:
        return None
    lvl, st, seat = last
    return lvl, st, dbl, strains[(seat % 2, st)]


def rows(d):
    out = []
    for f in sorted(glob.glob(os.path.join(d, "shard-*.json")),
                    key=lambda p: int(p.split("-")[-1].split(".")[0])):
        for b in json.load(open(f))["boards"]:
            out.append((b["deal"], b["dealer"], b["table_a"]))
    return out


on, off = rows(sys.argv[1]), rows(sys.argv[2])
assert len(on) == len(off), (len(on), len(off))

# NS is the side carrying the knob; declarer seat 0/2 == NS.
n = fired = 0
stat = collections.Counter()
for (d1, dl1, a1), (d2, dl2, a2) in zip(on, off):
    assert d1 == d2 and dl1 == dl2
    n += 1
    if a1 == a2:
        continue
    fired += 1
    fa, fb = final(a1, dl1), final(a2, dl2)
    for tag, f in (("ON", fa), ("OFF", fb)):
        if f is None:
            stat[f"{tag}/passout"] += 1
            continue
        lvl, st, dbl, dec = f
        ours = dec % 2 == 0
        stat[f"{tag}/{'we' if ours else 'they'}play"] += 1
        if dbl:
            stat[f"{tag}/{'we' if ours else 'they'}play/doubled"] += 1
        if lvl >= 5:
            stat[f"{tag}/level5+"] += 1
        if lvl >= 5 and dbl:
            stat[f"{tag}/level5+/doubled"] += 1
        stat[f"{tag}/lvl{lvl}"] += 1

print(f"boards {n}  fired {fired} ({100 * fired / n:.2f}%)")
for k in sorted(stat):
    print(f"  {k:28s} {stat[k]:6d}  {100 * stat[k] / fired:6.2f}%")
