#!/usr/bin/env python3
"""Cut an anchor bucket by auction: prefix, our call, BBA's call.

    python3 -I scripts/cut-auction.py SNAPSHOT [--arm american] \
        [--bucket "Constructive/book/round-2"] [--top 25] [--min-rows 20]

`shrunk-pools.py` keys a node by rule text and call, which pools every
auction sharing a rule.  This joins each row of `boards[-ARM].jsonl` to its
shard auction (`seed` + `board`) and keys it by the shared prefix up to
`div_index`, leading passes folded.  A deal sits in both vulnerability
cells, so rows are about twice the deals.  A first-call pair prices BBA's
continuation with its call: trace the worst boards before building.
"""
import argparse
import collections
import json
import pathlib

p = argparse.ArgumentParser()
p.add_argument("snapshot", type=pathlib.Path)
p.add_argument("--arm", default="american", help="'american' or 'instinct'")
p.add_argument("--bucket", help="phase/provenance/family; default: every bucket's top")
p.add_argument("--top", type=int, default=25)
p.add_argument("--min-rows", type=int, default=20)
a = p.parse_args()

pre = "american-" if a.arm == "american" else ""
rows = [json.loads(l) for l in open(a.snapshot / f"boards{'-american' if pre else ''}.jsonl")]
base = min(r["seed"] for r in rows)
shards = {}


def auction(r):
    key = (r["vul"], r["seed"])
    if key not in shards:
        f = a.snapshot / f"{pre}{r['vul']}" / f"shard-{r['seed'] - base}.json"
        shards[key] = json.load(open(f))["boards"]
    b = shards[key][r["board"]]
    ta, tb = b["table_a"].split(), b["table_b"].split()
    i = r["div_index"]
    assert ta[:i] == tb[:i], (r, b)
    calls = ta[:i]
    while calls and calls[0] == "-":
        calls.pop(0)
    return " ".join(calls) or "(opening)"


cut = collections.defaultdict(lambda: [0, 0, 0, set()])
for r in rows:
    bucket = f"{r['phase']}/{r['provenance']}/{r['family']}"
    if a.bucket and bucket != a.bucket:
        continue
    c = cut[(bucket, auction(r), r["our_call"], r["bba_call"])]
    c[0] += 1
    c[1] += r["swing_plain"]
    c[2] += r["swing_pd"]
    c[3].add((r["seed"], r["board"]))

keys = sorted((k for k in cut if cut[k][0] >= a.min_rows), key=lambda k: cut[k][1])
print("| bucket | prefix | ours | BBA's | rows (deals) | plain | PD |")
print("| --- | --- | --- | --- | --- | --- | --- |")
for k in keys[: a.top]:
    n, plain, pd, deals = cut[k]
    print(f"| {k[0]} | `{k[1]}` | `{k[2]}` | `{k[3]}` | {n} ({len(deals)}) | {plain:+} | {pd:+} |")
