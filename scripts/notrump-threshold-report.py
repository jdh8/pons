#!/usr/bin/env python3
"""3NT break-even tables from `examples/probe-notrump-threshold.rs` output.

    python3 scripts/notrump-threshold-report.py notrump.tsv

Prints Result 1 (break-even per population, X and F; detail near the gate)
and Result 2 (captured and regret of "points + adj(X, F) >= t") of
docs/notrump-game-threshold.md.
"""
import csv, collections, math, sys
rows = list(csv.DictReader(open(sys.argv[1]), delimiter='\t'))
COLS = ['2n', '2v', '1n', '1v']
# raw[var] = list of (X, F, M, P2, n, make, {col: (s, q)})
raw = collections.defaultdict(list)
for r in rows:
    raw[r['var']].append((int(r['X']), int(r['F']), int(r['M']), int(r['P2']), int(r['n']), int(r['make']),
                          {c: (int(r['s' + c]), int(r['q' + c])) for c in COLS}))

def table(var, pred):
    """P2 -> aggregated cell over rows matching pred(X, F, M)."""
    t = {}
    for X, F, M, P2, n, mk, s in raw[var]:
        if not pred(X, F, M): continue
        c = t.setdefault(P2, {'n': 0, 'make': 0, **{k: [0, 0] for k in COLS}})
        c['n'] += n; c['make'] += mk
        for k in COLS: c[k][0] += s[k][0]; c[k][1] += s[k][1]
    return t

def stat(c, k):
    n = c['n']; m = c[k][0] / n; q = c[k][1] / n
    return m, math.sqrt(max(q - m * m, 0) / n) if n > 1 else float('inf')

def cross(t, k):
    ps = sorted(p for p in t if t[p]['n'] >= 200)
    for a, b in zip(ps, ps[1:]):
        (ya, sa), (yb, sb) = stat(t[a], k), stat(t[b], k)
        if ya <= 0 < yb:
            x = a + (b - a) * ya / (ya - yb)
            se = (b - a) * math.sqrt(yb**2 * sa**2 + ya**2 * sb**2) / (ya - yb)**2
            return x / 2, se / 2, b / 2
    return None

def fmt(r):
    if r is None: return '—'
    x, se, g = r
    return f'{x:.2f}' + (f'±{1.96*se:.2f}' if 1.96 * se >= 0.01 else '')

SCALES = [('H', 'HCP'), ('F', 'Fifths'), ('P', 'point_count')]
def report(title, subsets):
    print(f'\n== {title}')
    for var, name in SCALES:
        print(f'-- {name}: subset | cases | P* 3v2 NV | P* 3v2 vul | gate NV/vul | P* 3v1 NV | P* 3v1 vul')
        for label, pred in subsets:
            t = table(var, pred); n = sum(c['n'] for c in t.values())
            r = [cross(t, k) for k in COLS]
            gates = '/'.join(f'{x[2]:g}' if x else '—' for x in r[:2])
            print(f'  {label:<28} {n:>11,} | ' + ' | '.join(fmt(x) for x in r[:2]) + f' | {gates} | ' + ' | '.join(fmt(x) for x in r[2:]))

report('Populations', [('all partnerships', lambda X, F, M: True),
                       ('no 8+ major fit', lambda X, F, M: M == 0),
                       ('8+ major fit', lambda X, F, M: M == 1)])
report('By length points X (no 8+ major fit)', [(f'X={x}' + ('+' if x == 5 else ''), lambda X, F, M, x=x: M == 0 and X == x) for x in range(6)])
report('By 4333 count F (no major fit, X=0)', [(f'F={f}', lambda X, F, M, f=f: M == 0 and X == 0 and F == f) for f in range(3)])
report('By 4333 count F (no major fit, any X)', [(f'F={f}', lambda X, F, M, f=f: M == 0 and F == f) for f in range(3)])

print('\n== Detail near the gate (no 8+ major fit), 3NT vs 2NT')
for var, name in SCALES:
    for label, pred in [('all', lambda X, F, M: M == 0)] + [(f'X={x}', lambda X, F, M, x=x: M == 0 and X == x) for x in range(4)]:
        t = table(var, pred); r = cross(t, '2n')
        if not r: continue
        ps = sorted(p for p in t if t[p]['n'] >= 200)
        g = int(r[2] * 2); i = ps.index(g)
        for p in ps[max(i - 1, 0):i + 1]:
            c = t[p]; (mn, sn), (mv, sv) = stat(c, '2n'), stat(c, '2v')
            print(f'  {name:<11} {label:<4} P={p/2:<5g} n={c["n"]:>10,} make={c["make"]/c["n"]:.0%} NV={mn:+.2f}±{1.96*sn:.2f} vul={mv:+.2f}±{1.96*sv:.2f}')

cells = collections.defaultdict(lambda: collections.defaultdict(lambda: [0, 0, 0]))
print('\n== Rules: P + adj(X, F) >= t (no 8+ major fit), mIMP/case')
for r in rows:
    if r['M'] == '1': continue
    e = cells[r['var']][(int(r['X']), int(r['F']), int(r['P2']))]
    e[0] += int(r['n']); e[1] += int(r['s2n']); e[2] += int(r['s2v'])
RULES = [('k=0', lambda X, F: 0)] + [(f'k={k}*X', lambda X, F, k=k: k * X) for k in (-1, -0.5, 0.25, 0.5, 1)] \
      + [(f'{k}*min(X,{cap})', lambda X, F, k=k, cap=cap: k * min(X, cap)) for k, cap in ((0.5, 1), (0.5, 2), (0.25, 2), (0.5, 3))] \
      + [(f'0.5*min(X,1) - 0.5*[F=2]', lambda X, F: 0.5 * min(X, 1) - 0.5 * (F == 2)),
         (f'0.5*min(X,2) - 0.5*[F=2]', lambda X, F: 0.5 * min(X, 2) - 0.5 * (F == 2)),
         (f'-0.5*[F=2]', lambda X, F: -0.5 * (F == 2))]
for var, name in [('H', 'HCP'), ('F', 'Fifths'), ('P', 'point_count')]:
    cs = cells[var]; W = sum(e[0] for e in cs.values())
    print(f'-- {name}: rule | NV best t, captured, regret | vul best t, captured, regret   (mIMP/case vs 2NT)')
    for label, adj in RULES:
        out = []
        for col in (1, 2):
            oracle = sum(max(e[col], 0) for e in cs.values())
            t, g = max(((t4 / 4, sum(e[col] for (X, F, P2), e in cs.items() if P2 / 2 + adj(X, F) >= t4 / 4))
                        for t4 in range(80, 112)), key=lambda z: z[1])
            out.append(f't={t:<5g} {g / W * 1000:7.2f} {(oracle - g) / W * 1000:6.2f}')
        print(f'  {label:<26} | ' + ' | '.join(out))
