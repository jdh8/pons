"""Count boards whose final contract moved between two bba-gen arms — no solver.

The cheap half of a bid-only pre-count (docs/measurement.md item 6): the arms
share SEED_BASE, so board i of `on-VUL` and board i of `off-VUL` are the same
deal, and a contract that did not move cannot score differently.
"""

import json
import pathlib
import sys

SEATS = ["North", "East", "South", "West"]


def contract(auction, dealer):
    """(level, strain, declarer, doubled) of a finished auction, or None if passed out."""
    calls = auction.split()
    start = SEATS.index(dealer)
    last = None
    doubled = 0
    for i, call in enumerate(calls):
        if call == "-":
            continue
        if call == "X":
            doubled = 1
        elif call == "XX":
            doubled = 2
        else:
            last, doubled = i, 0
    if last is None:
        return None
    strain = calls[last][1:]
    side = last % 2
    declarer = next(
        i for i in range(side, last + 1, 2) if calls[i] not in ("-", "X", "XX") and calls[i][1:] == strain
    )
    return calls[last][0], strain, (start + declarer) % 4, doubled


def boards(path):
    root = pathlib.Path(path)
    shards = sorted(root.glob("shard-*.json")) if root.is_dir() else [root]
    for shard in shards:
        yield from json.load(open(shard))["boards"]


def main(results):
    total = moved = auction_moved = 0
    for vul in ("none", "both"):
        for on, off in zip(boards(f"{results}/on-{vul}"), boards(f"{results}/off-{vul}")):
            assert on["deal"] == off["deal"], "arms are not seed-aligned"
            for table in ("table_a", "table_b"):
                total += 1
                if on[table] == off[table]:
                    continue
                auction_moved += 1
                if contract(on[table], on["dealer"]) != contract(off[table], off["dealer"]):
                    moved += 1
    pct = lambda n: 100.0 * n / total if total else 0.0
    print(f"tables compared   : {total}")
    print(f"auction differs   : {auction_moved} ({pct(auction_moved):.4f}%)")
    print(f"contract moved    : {moved} ({pct(moved):.4f}%)")


if __name__ == "__main__":
    main(sys.argv[1])
