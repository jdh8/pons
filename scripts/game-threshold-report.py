#!/usr/bin/env python3
"""Break-even tables from `examples/probe-game-threshold.rs` output.

    python3 scripts/game-threshold-report.py byfit.tsv [--minor]

Prints, per scale (A/B/C): the break-even point P* per trump length, the
detail rows near the gate, and the regret of the rule
"points + k * (trumps - 8) >= t".  With --minor, also game over 3NT (g2).
See docs/major-game-threshold.md and docs/minor-game-threshold.md.
"""
import collections
import csv
import math
import sys

FIT = {'8': 8, '9': 9, '10': 10, '11': 11, '12+': 12}
d = collections.defaultdict(dict)
for r in csv.DictReader(open(sys.argv[1]), delimiter='\t'):
    w = int(r['w']); n = w / 2
    cell = {'n': n, 'w': w, 'make': int(r['make']) / w}
    for c in ['1n', '1v', '2n', '2v']:
        s = int(r['g' + c]); m = s / w; q = int(r['q' + c]) / w
        cell[c] = (m, math.sqrt(max(q - m * m, 0) / n) if n > 1 else float('inf'), s)
    d[(r['var'], FIT[r['fit']])][int(r['P'])] = cell


def cross(tab, c):
    ps = sorted(tab)
    for a, b in zip(ps, ps[1:]):
        (ya, sa, _), (yb, sb, _) = tab[a][c], tab[b][c]
        if b == a + 1 and ya <= 0 < yb:
            se = math.sqrt(yb**2 * sa**2 + ya**2 * sb**2) / (ya - yb)**2
            return a + ya / (ya - yb), se, b


NAMES = [('A', 'both hands support points (pons)'), ('B', 'dummy-only support points'), ('C', 'raw HCP')]
for v, name in NAMES:
    print(f'\n== {name}\nfit | cases | P* NV | P* vul | gate NV/vul | worth NV/vul | P*+fit NV')
    prev = None
    for f in FIT.values():
        tab = d[(v, f)]
        (xn, en, gn), (xv, ev, gv) = cross(tab, '1n'), cross(tab, '1v')
        ci = lambda e: f' ± {1.96*e:.2f}' if 1.96 * e > 0.01 else ''
        worth = f'{prev[0]-xn:.2f} / {prev[1]-xv:.2f}' if prev else '(baseline)'
        print(f"{f:>3} | {sum(c['n'] for c in tab.values())/1e6:.2f}M | {xn:.2f}{ci(en)} | {xv:.2f}{ci(ev)} | {gn} / {gv} | {worth} | {xn+f:.1f}")
        prev = (xn, xv)
    if v != 'C':
        print('detail: fit P make NV vul')
        for f in FIT.values():
            tab = d[(v, f)]; g = cross(tab, '1n')[2]
            for p in (g - 1, g):
                c = tab[p]
                print(f"  {f} {p} {c['make']:.0%} {c['1n'][0]:+.2f}±{1.96*c['1n'][1]:.2f} {c['1v'][0]:+.2f}±{1.96*c['1v'][1]:.2f}")

print('\n== rule P + k*(trumps-8) >= t over 8+ fits, regret mIMP/case: k: NV (t) / vul (t)')
for v, name in NAMES:
    cs = [(f, p, c) for (vv, f), tab in d.items() if vv == v for p, c in tab.items()]
    W = sum(c['w'] for _, _, c in cs)
    print(name)
    for k in (0, 0.5, 1, 1.5, 2, 2.5):
        out = []
        for col in ('1n', '1v'):
            oracle = sum(max(c[col][2], 0) for *_, c in cs)
            best = max((sum(c[col][2] for f, p, c in cs if p + k * (f - 8) >= t2 / 2), t2 / 2) for t2 in range(30, 80))
            out.append(f'{(oracle - best[0]) / W * 1000:6.1f} ({best[1]:g})')
        print(f'  k={k:<3} ' + ' / '.join(out))

if '--minor' in sys.argv:
    print('\n== 5m vs 3NT: per fit, game-over-partscore / game-over-3NT NV at 26..32; mean over cells where game beats partscore')
    for v, name in NAMES:
        print(name)
        for f in FIT.values():
            tab = d[(v, f)]
            for col, label in (('n', 'NV'), ('v', 'vul')):
                g = [(c['w'], c['2' + col][0]) for c in tab.values() if c['1' + col][0] > 0]
                mean = sum(w * x for w, x in g) / sum(w for w, _ in g)
                cells = ' '.join(f"{p}:{tab[p]['1n'][0]:+.1f}/{tab[p]['2n'][0]:+.1f}" for p in (26, 28, 30, 32) if p in tab)
                print(f'  {f:>2} {label:3} mean {mean:+.2f}' + (f' | {cells}' if col == 'n' else ''))
