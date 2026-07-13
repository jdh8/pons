#!/usr/bin/env python3
"""Partition a bba-gen shard dir's boards by the opponents' opening strain.

Usage: split_by_opening.py DIR

Writes sibling dirs DIR-major and DIR-minor, each holding shard-*.json with the
same Dump schema but boards filtered to a major (♥/♠) or non-major first
non-pass call in `table_a`.  Gladiator (our 1NT overcall of their 1M) only fires
over a MAJOR opening, so the minor split is the ~0 sanity check and the major
split is the real test.  The opening is opponent-bid — the same per deal in both
arms — so the ON and OFF splits stay seed-aligned board-for-board for the paired
diff.  1NT / notrump / passed-out openings carry no ♥/♠ symbol and land in
`-minor` (irrelevant to Gladiator), which is correct.
"""
import json
import os
import sys


def opening_is_major(table_a):
    """True iff the first non-pass call in the auction is a heart/spade bid."""
    for tok in table_a.split():
        if tok == "-":  # Pass
            continue
        return "♥" in tok or "♠" in tok
    return False  # passed out — no opener


def main():
    src = sys.argv[1]
    for kind in ("major", "minor"):
        os.makedirs(f"{src}-{kind}", exist_ok=True)
    for name in os.listdir(src):
        if not name.endswith(".json"):
            continue
        with open(os.path.join(src, name), encoding="utf-8") as f:
            dump = json.load(f)
        for kind, want in (("major", True), ("minor", False)):
            out = dict(dump)
            out["boards"] = [
                b for b in dump["boards"] if opening_is_major(b["table_a"]) == want
            ]
            path = os.path.join(f"{src}-{kind}", name)
            with open(path, "w", encoding="utf-8") as f:
                json.dump(out, f, ensure_ascii=False)


if __name__ == "__main__":
    main()
