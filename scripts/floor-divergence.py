#!/usr/bin/env python3
"""Bucket an `ab-dump-diff --jsonl` by the first divergent call: who was right, by lane.

    scripts/floor-divergence.py ab-results/reseed-gap/forensic/k8new-{none,ns,ew,both}.jsonl
    scripts/floor-divergence.py A.jsonl ... [--show 20] [--depth 2]

Born on the reseed-gap step 2b forensic (docs/reseed-gap-handoff.md §3).  Each
record is one contract-divergent board of a direct A/B: `on` and `off` are the
two arms' table-A auctions (our pair North/South), `plain`/`pd` the IMP swing
for ON.  The board is charged to the first call where the auctions part, so
its swing includes every later call both arms then made differently — the
direct A/B's own attribution, not a common-continuation counterfactual.

Buckets:
  side   — which pair made the first divergent call (ours, else a bug: the
           reference bidder is deterministic, so it cannot part first)
  phase  — `opening` (our first non-pass call opens the auction),
           `constructive` (only we have bid), `competitive` (we opened, they
           have bid), `defensive` (they opened)
  lane   — the prefix's first `--depth` non-pass calls, dealer-free, with the
           seat that opened (`1st`..`4th`) and who (`we`/`they`)
  pair   — the ON call against the OFF call at the divergence
  move   — the pair's direction: ON passes where OFF acts, the reverse, ON
           bids higher or lower, or a double against a bid

Columns: n boards, ON wins / losses (plain), plain and PD IMPs, plain IMPs per
board of the whole cell set (the lane's share of the headline delta).
"""
import argparse
import collections
import json

SEATS = ["North", "East", "South", "West"]


def first_diff(on, off):
    for i, (a, b) in enumerate(zip(on, off)):
        if a != b:
            return i
    return min(len(on), len(off))


def classify(rec, depth):
    on, off = rec["on"].split(), rec["off"].split()
    i = first_diff(on, off)
    dealer = SEATS.index(rec["dealer"])
    ours = lambda k: (dealer + k) % 2 == 0  # North/South
    prefix = on[:i]
    bids = [(k, c) for k, c in enumerate(prefix) if c not in ("-", "X", "XX")]
    if not bids:
        phase, lane = "opening", f"{i + 1}{'stndrh'[2 * min(i, 3):2 * min(i, 3) + 2]} opening"
    else:
        opener = bids[0][0]
        we_opened = ours(opener)
        they_bid = any(not ours(k) for k, c in enumerate(prefix) if c != "-")
        if not we_opened:
            phase = "defensive"
        elif they_bid:
            phase = "competitive"
        else:
            phase = "constructive"
        seat = ["1st", "2nd", "3rd", "4th"][opener]
        acts = [c if ours(k) else f"({c})" for k, c in enumerate(prefix) if c != "-"]
        lane = f"{seat} {'we' if we_opened else 'they'}: " + " ".join(acts[:depth])
    side = "ours" if ours(i) else "theirs"
    call_on = on[i] if i < len(on) else "end"
    call_off = off[i] if i < len(off) else "end"
    return side, phase, move(call_on, call_off), lane, f"{call_on} vs {call_off}"


def rank(call):
    """Bid rank 1..35, None for a non-bid"""
    if len(call) < 2 or not call[0].isdigit():
        return None
    return 5 * (int(call[0]) - 1) + ["♣", "♦", "♥", "♠", "NT"].index(call[1:])


def move(on, off):
    if on == "-":
        return "ON passes, OFF acts"
    if off == "-":
        return "ON acts, OFF passes"
    a, b = rank(on), rank(off)
    if a is not None and b is not None:
        return "ON bids higher" if a > b else "ON bids lower"
    if on in ("X", "XX"):
        return "ON doubles, OFF bids"
    if off in ("X", "XX"):
        return "ON bids, OFF doubles"
    return "other"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("jsonl", nargs="+")
    ap.add_argument("--show", type=int, default=15)
    ap.add_argument("--depth", type=int, default=2)
    ap.add_argument("--boards", type=int, default=205056,
                    help="boards per cell, for IMPs/board (default: the reseed-gap A/B)")
    args = ap.parse_args()

    recs = [json.loads(l) for p in args.jsonl for l in open(p)]
    total = args.boards * len(args.jsonl)
    tables = {k: collections.defaultdict(lambda: [0, 0, 0, 0, 0]) for k in
              ("side", "phase", "move", "lane", "pair")}
    for r in recs:
        for name, key in zip(tables, classify(r, args.depth)):
            c = tables[name][key]
            c[0] += 1
            c[1] += r["plain"] > 0
            c[2] += r["plain"] < 0
            c[3] += r["plain"]
            c[4] += r.get("pd", 0)

    pl = sum(r["plain"] for r in recs)
    pd = sum(r.get("pd", 0) for r in recs)
    print(f"{len(recs)} divergent boards of {total}: plain {pl:+} IMPs "
          f"({pl / total:+.4f}/board), PD {pd:+} ({pd / total:+.4f}/board)")
    for name, d in tables.items():
        rows = sorted(d.items(), key=lambda kv: kv[1][3])
        if name in ("lane", "pair"):
            rows = rows[:args.show] + [("…", None)] + rows[-args.show:] \
                if len(rows) > 2 * args.show else rows
        print(f"\n--- by {name} (worst first)")
        print(f"{'n':>6} {'win':>5} {'loss':>5} {'plain':>7} {'pd':>7} {'/board':>8}  key")
        for key, c in rows:
            if c is None:
                print("   …")
                continue
            print(f"{c[0]:6} {c[1]:5} {c[2]:5} {c[3]:+7} {c[4]:+7} {c[3] / total:+8.4f}  {key}")


if __name__ == "__main__":
    main()
