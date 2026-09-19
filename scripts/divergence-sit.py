#!/usr/bin/env python3
"""The *sit* counterfactual over a `probe-divergence --imps --jsonl` file.

    python3 scripts/divergence-sit.py $R/imps-none.jsonl none --sit 3NT@0-9 --sit 4♣@10-12

For every divergent board where OUR side's last non-pass call is one of `--sit`,
re-score the board as if that seat had passed (the contract reverts to what stood
before it, doubles included) and report, per responder-seat cell and overall, the
as-played vs sat plain/PD IMPs.  Needs the `dd`/`score_on` fields probe-divergence
writes since 2026-09-19.  Born on the §N1q run-2 forensic, where three families of
floor phantom bids at unauthored nodes (a 3NT over their raise, opener's 4♣ over a
making 3NT) carried the whole PD deficit; the sit says what authored Pass rails at
those nodes would have been worth.  A sit is an upper bound only in the sense that
it assumes the opponents' calls before the phantom stay put.

`CALL@LO-HI` restricts that sit to boards where the opener's partner holds LO-HI
HCP; a phantom at one strength band is a sound bid at another.
"""
import argparse, collections, json

SEATS = 'NESW'
HCP = {'A': 4, 'K': 3, 'Q': 2, 'J': 1}
BOARDS = 4_608_000
IMP_SCALE = [20, 50, 90, 130, 170, 220, 270, 320, 370, 430, 500, 600, 750, 900, 1100,
             1300, 1500, 1750, 2000, 2250, 2500, 3000, 3500, 4000]


def imps(diff):
    a = abs(diff); i = 0
    while i < len(IMP_SCALE) and a >= IMP_SCALE[i]:
        i += 1
    return i if diff >= 0 else -i


def score(level, strain, tricks, vul, dbl):
    """Duplicate score for declarer; strain in 'cdhsn', dbl 0/1/2."""
    need = 6 + level
    if tricks < need:
        down = need - tricks
        if dbl == 0:
            return -(100 if vul else 50) * down
        pen = 0
        for i in range(down):
            pen += (200 if vul else 100) if i == 0 else ((300 if vul else 200) if i < 3 else 300)
        return -pen * dbl
    per = 20 if strain in 'cd' else 30
    base = per * level + (10 if strain == 'n' else 0)
    if dbl:
        base *= 2 * dbl
    over = tricks - need
    total = base + (over * per if dbl == 0 else over * (200 if vul else 100) * dbl)
    total += 500 if (base >= 100 and vul) else (300 if base >= 100 else 50)
    total += 50 * dbl
    if level == 6:
        total += 750 if vul else 500
    if level == 7:
        total += 1500 if vul else 1000
    return total


def ns_score(contract, dd, vul_ns, vul_ew, pd=False):
    """NS score of 'BID[x|xx] Seat' (None = pass-out); `pd` doubles a failing undoubled contract."""
    if contract is None:
        return 0
    bid, seat = contract.split()
    level = int(bid[0])
    dbl = 2 if bid.endswith('xx') else (1 if bid.endswith('x') else 0)
    strain = {'♣': 'c', '♦': 'd', '♥': 'h', '♠': 's', 'N': 'n'}[bid[1]]
    tricks = dd[strain][SEATS.index(seat[0])]
    ns = seat[0] in 'NS'
    vul = vul_ns if ns else vul_ew
    if pd and tricks < 6 + level and dbl == 0:
        dbl = 1
    s = score(level, strain, tricks, vul, dbl)
    return s if ns else -s


def final_contract(tokens, dealer, upto=None):
    """Contract of tokens[:upto] followed by passes: 'BID[x] S' or None."""
    t = tokens if upto is None else tokens[:upto]
    last = None; dbl = ''; first_in = {}
    for i, c in enumerate(t):
        seat = SEATS[(dealer + i) % 4]
        if c == '-':
            continue
        if c in ('X', 'XX'):
            dbl = c.lower(); continue
        side = 'NS' if seat in 'NS' else 'EW'
        first_in.setdefault((side, c[1:]), seat)
        last = (c, side); dbl = ''
    if last is None:
        return None
    c, side = last
    return f"{c}{dbl} {first_in[(side, c[1:])]}"


def load(path, vul):
    vul_ns = vul_ew = (vul == 'both')
    recs = []
    for line in open(path):
        r = json.loads(line)
        t = r['auction_on'].split(); r['t'] = t
        dealer = SEATS.index(r['dealer'][0]); r['dealer_i'] = dealer
        i = t.index(r['opening_call'])
        r['opener'] = SEATS[(dealer + i) % 4]; r['resp'] = SEATS[(dealer + i + 2) % 4]
        r['resp_seat'] = r['first_diff'] == i + 2
        first, rest = r['deal'].split(':'); hs = rest.split(); off = SEATS.index(first[0])
        hands = {SEATS[(off + k) % 4]: hs[k] for k in range(4)}
        r['resp_hcp'] = sum(HCP.get(ch, 0) for ch in hands[r['resp']])
        r['dd'] = {tag: [int(x) for x in row.split(',')]
                   for tag, row in (part.split(':') for part in r['dd'].split(';'))}
        r['vul'] = (vul_ns, vul_ew)
        assert ns_score(r['contract_on'], r['dd'], vul_ns, vul_ew) == r['score_on'], r['index']
        recs.append(r)
    return recs


def sit(r, calls):
    """(contract, plain IMPs vs OFF, PD IMPs vs OFF) with our last call passed, if it is in
    `calls` (a dict call -> (lo, hi) responder-HCP band); else None."""
    t = r['t']; ours = {r['opener'], r['resp']}
    for i in range(len(t) - 1, -1, -1):
        c = t[i]
        if c == '-':
            continue
        band = calls.get(c)
        if SEATS[(r['dealer_i'] + i) % 4] in ours and band and band[0] <= r['resp_hcp'] <= band[1]:
            contract = final_contract(t, r['dealer_i'], i)
            v = r['vul']
            off_pl = ns_score(r['contract_off'], r['dd'], *v)
            off_pd = ns_score(r['contract_off'], r['dd'], *v, pd=True)
            return (contract, imps(ns_score(contract, r['dd'], *v) - off_pl),
                    imps(ns_score(contract, r['dd'], *v, pd=True) - off_pd))
        return None  # our last call is not a sit candidate
    return None


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('jsonl'); ap.add_argument('vul', choices=['none', 'both'])
    ap.add_argument('--sit', action='append', required=True, help='CALL or CALL@LO-HI: a call of ours to replace by Pass (repeatable)')
    a = ap.parse_args()
    calls = {}
    for spec in a.sit:
        call, _, band = spec.partition('@')
        lo, hi = (int(x) for x in band.split('-')) if band else (0, 40)
        calls[call] = (lo, hi)
    recs = load(a.jsonl, a.vul)
    tot = [0, 0]; sat = [0, 0]; n_sat = 0
    per_cell = collections.defaultdict(lambda: [0, 0, 0, 0, 0])  # n, plain, pd, sat plain, sat pd
    lands = collections.Counter()
    for r in recs:
        tot[0] += r['imps_plain']; tot[1] += r['imps_pd']
        s = sit(r, calls)
        if s is None:
            sat[0] += r['imps_plain']; sat[1] += r['imps_pd']; continue
        contract, pl, pd = s; n_sat += 1
        sat[0] += pl; sat[1] += pd
        cell = f"{r['call_off']} -> {r['call_on']}" if r['resp_seat'] else 'other seat'
        row = per_cell[cell]; row[0] += 1; row[1] += r['imps_plain']; row[2] += r['imps_pd']; row[3] += pl; row[4] += pd
        lands[contract.split()[0] if contract else 'pass-out'] += 1
    print(f"{a.jsonl}: {len(recs)} divergent; sit {a.sit}: {n_sat} boards sat")
    print(f"  as played   plain {tot[0]:+7} ({tot[0]/BOARDS:+.4f}/bd)   pd {tot[1]:+7} ({tot[1]/BOARDS:+.4f}/bd)")
    print(f"  with sits   plain {sat[0]:+7} ({sat[0]/BOARDS:+.4f}/bd)   pd {sat[1]:+7} ({sat[1]/BOARDS:+.4f}/bd)")
    print("  sat boards land in: " + ', '.join(f"{k} {n}" for k, n in lands.most_common(8)))
    print(f"  {'cell':22} {'n':>6} {'plain':>8} {'pd':>8} | {'sat plain':>9} {'sat pd':>8}")
    for cell, (n, pl, pd, spl, spd) in sorted(per_cell.items(), key=lambda kv: kv[1][2]):
        print(f"  {cell:22} {n:6} {pl:+8} {pd:+8} | {spl:+9} {spd:+8}")


if __name__ == '__main__':
    main()
