#!/usr/bin/env python3
"""Bucket a probe-divergence --imps jsonl: legs, cells, auction classes, declarer-seat flips.

    python3 scripts/divergence-buckets.py $R/imps-none.jsonl

Born on the §N1-lia lia3 forensic (2026-09-02).  "Leg split V3" is the lia2 forensic's
definition (baseline call first, then the candidate's rung) and reproduces its table
exactly; V1/V2 are the alternatives it was checked against.  The leg rules are Landy-
lane specific; everything else is generic to any two-arm divergence set.
"""
import json, sys, collections, re
BOARDS = 4_608_000
CLUB_ON, DIA_ON = {'2♠', '3♣'}, {'2NT', '3♦'}

def load(p):
    return [json.loads(l) for l in open(p)]

def toks(a):
    return a.split()

def after_2c(r, k=3):
    t = toks(r['auction_on'])
    for i in range(len(t) - 1):
        if t[i] == '1NT' and t[i + 1] == '2♣':
            return ' '.join(t[i + 2:i + 2 + k]), i + 2
    return None, None

def resp_seat_diff(r):
    _, idx = after_2c(r)
    return idx is not None and r['first_diff'] == idx

def leg(r, variant):
    on, off = r['call_on'], r['call_off']
    if variant == 'V2' and not resp_seat_diff(r):
        return 'rest'
    if variant in ('V1', 'V2'):
        if on in CLUB_ON: return 'club'
        if on in DIA_ON: return 'diamond'
        if off == '2NT': return 'club'
        if off == '3♣': return 'diamond'
        return 'rest'
    if variant == 'V3':
        if off == '2NT': return 'club'
        if off == '3♣': return 'diamond'
        if on in CLUB_ON: return 'club'
        if on in DIA_ON: return 'diamond'
        return 'rest'

def agg(recs, key):
    d = collections.defaultdict(lambda: [0, 0, 0])
    for r in recs:
        k = key(r)
        if k is None: continue
        c = d[k]; c[0] += 1; c[1] += r['imps_plain']; c[2] += r['imps_pd']
    return d

def show(d, title, top=None, sortkey=lambda kv: kv[1][1]):
    print(f"\n--- {title}")
    rows = sorted(d.items(), key=sortkey)
    if top: rows = rows[:top]
    print(f"{'bucket':34} {'n':>8} {'plain':>9} {'/fired':>8} {'/board':>9} {'pd':>9} {'pd/fired':>9}")
    for k, (n, pl, pd) in rows:
        print(f"{str(k):34} {n:8d} {pl:9d} {pl/n:8.3f} {pl/BOARDS:9.4f} {pd:9d} {pd/n:9.3f}")

def contract_parts(c):
    if not c: return None, None
    m = re.match(r'(\S+) (\w+)', c)
    return (m.group(1), m.group(2)) if m else (c, None)

def main():
    recs = load(sys.argv[1])
    n = len(recs); pl = sum(r['imps_plain'] for r in recs); pd = sum(r['imps_pd'] for r in recs)
    print(f"{sys.argv[1]}: {n} divergent; plain {pl} ({pl/BOARDS:+.4f}/bd, {pl/n:+.3f}/fired); pd {pd} ({pd/BOARDS:+.4f}/bd)")
    for v in ('V1', 'V2', 'V3'):
        show(agg(recs, lambda r, v=v: leg(r, v)), f"leg split {v}")
    show(agg(recs, lambda r: f"{r['call_off']} -> {r['call_on']}"), "cells call_off -> call_on (worst 18)", top=18)
    show(agg(recs, lambda r: f"{r['call_off']} -> {r['call_on']}"), "cells call_off -> call_on (best 8)", top=8, sortkey=lambda kv: -kv[1][1])
    show(agg(recs, lambda r: after_2c(r, 3)[0]), "auction_on class: 3 calls after 2♣ (worst 20)", top=20)
    show(agg(recs, lambda r: after_2c(r, 4)[0]), "auction_on class: 4 calls after 2♣ (worst 20)", top=20)
    show(agg(recs, lambda r: 'responder-seat' if resp_seat_diff(r) else 'later'), "first divergence at responder's rung seat vs later")
    # same contract, declarer seat flip within our side
    def flip(r):
        (c1, s1), (c2, s2) = contract_parts(r['contract_on']), contract_parts(r['contract_off'])
        if c1 == c2 and s1 != s2 and r['declarer_on'] == r['declarer_off'] == 'NS': return 'seat flip'
        if c1 == c2 and r['declarer_on'] != r['declarer_off']: return 'side flip'
        return None
    show(agg(recs, flip), "same-contract declarer flips")
    show(agg(recs, lambda r: f"{r['call_off']} -> {r['call_on']}" if r['call_off'] == '3♦' and r['call_on'] == '-' else None), "the 3♦ -> - sell-out cell")
    show(agg(recs, lambda r: 'doubled on only' if r['doubled_on'] and not r['doubled_off'] else ('doubled off only' if r['doubled_off'] and not r['doubled_on'] else None)), "doubled in one arm")
    show(agg(recs, lambda r: f"level_on {r['level_on']}"), "by level_on")
if __name__ == '__main__':
    main()
