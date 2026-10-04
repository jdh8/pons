#!/usr/bin/env python3
"""Tables for docs/nltc.md from `examples/probe-nltc.rs` output.

    python3 scripts/nltc-report.py nltc.tsv

Prints, per evaluator (NLTC, support points, HCP, Zar): the trick fit, the
fit by shape, the ranking power (AUC) and the IMPs a single threshold wins
at the game and slam decisions; then the trump terms NLTC is missing, and
the hand rule built from them.
"""
import bisect
import sys

import numpy as np

EVALS = ['nltc', 'sp', 'hcp', 'zar']
COLS = ['value', 'shape', 'long7', 'major', 'T', 'm', 'q', 'k', 'j', 'tricks', 'n']
raw = {e: [] for e in EVALS}
with open(sys.argv[1]) as f:
    next(f)
    for line in f:
        e, *rest = line.split('\t')
        raw[e].append(rest)
D = {e: dict(zip(COLS, np.array(rows, dtype=float).T)) for e, rows in raw.items()}
for d in D.values():
    # strength: higher is better on every scale; NLTC comes in half losers
    d['x'] = d['value']
D['nltc']['x'] = D['nltc']['value'] / 2
SIGN = {'nltc': -1, 'sp': 1, 'hcp': 1, 'zar': 1}


def ols(cols, y, w):
    """Weighted least squares over cells: beta, standard errors, residuals."""
    X = np.column_stack([np.ones(len(y))] + [np.asarray(c, dtype=float) for c in cols])
    A = X.T @ (X * w[:, None])
    beta = np.linalg.solve(A, X.T @ (w * y))
    res = y - X @ beta
    var = (w * res**2).sum() / (w.sum() - X.shape[1])
    return beta, np.sqrt(np.diag(np.linalg.inv(A)) * var), res


def sd(res, w):
    return np.sqrt((w * res**2).sum() / w.sum())


def r2(res, y, w):
    mean = (w * y).sum() / w.sum()
    return 1 - (w * res**2).sum() / (w * (y - mean)**2).sum()


def auc(score, pos, w):
    """P(a random positive outranks a random negative), ties counting half."""
    order = np.argsort(score, kind='stable')
    s, p, n = score[order], (w * pos)[order], (w * ~pos)[order]
    edge = np.flatnonzero(np.r_[True, s[1:] != s[:-1]])
    p, n = np.add.reduceat(p, edge), np.add.reduceat(n, edge)
    below = np.cumsum(n) - n
    return (p * (below + n / 2)).sum() / (p.sum() * n.sum())


IMP = [20, 50, 90, 130, 170, 220, 270, 320, 370, 430, 500, 600, 750, 900,
       1100, 1300, 1500, 1750, 2000, 2250, 2500, 3000, 3500, 4000]


def imps(diff):
    return (1 if diff > 0 else -1) * bisect.bisect_right(IMP, abs(diff))


def score(level, tricks, vul):
    """A major-suit contract, undoubled."""
    if tricks < level + 6:
        return -(level + 6 - tricks) * (100 if vul else 50)
    bonus = 50 if level < 4 else 500 if vul else 300
    bonus += {6: 750 if vul else 500, 7: 1500 if vul else 1000}.get(level, 0)
    return 30 * (tricks - 6) + bonus


def gains(hi, lo, vul):
    return np.array([imps(score(hi, t, vul) - score(lo, t, vul)) for t in range(14)])


def threshold(s, g, w):
    """Best rule `bid iff s >= t`: (mIMP per case, t)."""
    order = np.argsort(-s, kind='stable')
    total = np.cumsum((w * g)[order])
    last = np.flatnonzero(np.r_[s[order][1:] != s[order][:-1], True])
    i = last[np.argmax(total[last])]
    return 1000 * max(total[i], 0) / w.sum(), s[order][i]


SHAPES = [('both balanced', lambda d: d['shape'] == 0),
          ('no singleton', lambda d: d['shape'] == 1),
          ('a singleton', lambda d: d['shape'] == 2),
          ('a void', lambda d: d['shape'] == 3),
          ('a 7+ card suit', lambda d: d['long7'] == 1)]

d = D['nltc']
print(f"cases: {d['n'].sum() / 1e6:.2f}M ({d['n'][d['major'] == 1].sum() / 1e6:.2f}M with a major as trumps)")

print('\n== tricks = a + b * value, all cases\neval | b | a | sd | R2')
fit = {}
for e in EVALS:
    d = D[e]
    beta, _, res = ols([d['x']], d['tricks'], d['n'])
    fit[e] = res
    print(f"{e:4} | {beta[1]:+.3f} | {beta[0]:.2f} | {sd(res, d['n']):.3f} | {r2(res, d['tricks'], d['n']):.3f}")

print('\n== by shape: sd of the fit within the row [mean miss of the all-cases fit]')
for name, pick in SHAPES:
    out = []
    for e in EVALS:
        d = D[e]
        m = pick(d)
        y, w = d['tricks'][m], d['n'][m]
        _, _, res = ols([d['x'][m]], y, w)
        out.append(f"{e} {sd(res, w):.3f} [{(w * fit[e][m]).sum() / w.sum():+.2f}]")
    print(f"{name:14} | {w.sum() / 1e6:6.2f}M | " + ' | '.join(out))

print('\n== ranking power (AUC)')
TESTS = [('10+ tricks', lambda d: d['tricks'] >= 10, lambda d: d['tricks'] >= 0),
         ('12+ tricks', lambda d: d['tricks'] >= 12, lambda d: d['tricks'] >= 0),
         ('12+ given 10+', lambda d: d['tricks'] >= 12, lambda d: d['tricks'] >= 10)]
TESTS += [(f'12+ tricks, {name}', lambda d: d['tricks'] >= 12, pick) for name, pick in SHAPES]
for name, pos, pick in TESTS:
    out = []
    for e in EVALS:
        d = D[e]
        m = pick(d)
        out.append(f"{e} {auc(SIGN[e] * d['x'][m], pos(d)[m], d['n'][m]):.3f}")
    print(f'{name:30} | ' + ' | '.join(out))

# NLTC with the trump suit counted as if three cards long: a short holding gets
# back the king and queen losers its length hid.
d = D['nltc']
restore = np.select([d['m'] == 2, d['m'] == 1, d['m'] == 0], [0.5, 1.5, 3.0], 0.0) - 0.5 * d['q'] - d['k']
RULES = [('NLTC', d['x']),
         ('+ trumps as if three long', d['x'] + restore),
         ('+ 9th trump -0.5', d['x'] + restore - 0.5 * (d['T'] >= 9)),
         ('+ trump J -0.25', d['x'] + restore - 0.5 * (d['T'] >= 9) - 0.25 * d['j'])]

print('\n== one threshold, major fits: mIMP per case won over never bidding (threshold)')
scales = [(name, 'nltc', -x) for name, x in RULES[::3]]
scales += [('support points', 'sp', D['sp']['x']),
           ('support points + trumps', 'sp', D['sp']['x'] + D['sp']['T']),
           ('HCP', 'hcp', D['hcp']['x']),
           ('HCP + 2 per trump', 'hcp', D['hcp']['x'] + 2 * D['hcp']['T']),
           ('Zar', 'zar', D['zar']['x'])]
DECISIONS = [('4M over 3M NV', 4, 3, False), ('4M over 3M vul', 4, 3, True),
             ('6M over 4M NV', 6, 4, False), ('6M over 4M vul', 6, 4, True)]
major = D['nltc']['major'] == 1
t, w = D['nltc']['tricks'][major].astype(int), D['nltc']['n'][major]
print('perfect knowledge of the tricks  | ' + ' | '.join(
    f"{name} {1000 * (w * np.maximum(gains(hi, lo, vul)[t], 0)).sum() / w.sum():.0f}"
    for name, hi, lo, vul in DECISIONS))
for name, e, s in scales:
    d = D[e]
    m = d['major'] == 1
    out = []
    for dn, hi, lo, vul in DECISIONS:
        g, at = threshold(s[m], gains(hi, lo, vul)[d['tricks'][m].astype(int)], d['n'][m])
        out.append(f'{dn} {g:.0f} ({abs(at):g})')
    print(f'{name:32} | ' + ' | '.join(out))


def trump_terms(d):
    cols = [d['x']] + [d['T'] == T for T in (9, 10, 11)] + [d['m'] == m for m in (3, 2, 1, 0)]
    return cols + [d['q'], d['k'], d['j']]


NAMES = ['value', '9 trumps (vs 8)', '10 trumps (vs 8)', '11+ trumps (vs 8)',
         'short hand has 3 trumps (vs 4+)', 'short hand has 2 trumps', 'short hand has 1 trump',
         'short hand has 0 trumps', 'trump Q in a holding of 2 or fewer', 'singleton trump K', 'trump J held']
print('\n== trump terms on top of NLTC: tricks ± 95% = losers')
d = D['nltc']
beta, se, res = ols(trump_terms(d), d['tricks'], d['n'])
for name, b, s in zip(NAMES, beta[1:], se[1:]):
    print(f'{name:36} | {b:+.3f} ± {1.96 * s:.3f} | {b / beta[1]:+.2f}')

print('\n== the same trump terms on every evaluator: sd before -> after | R2 after')
for e in EVALS:
    d = D[e]
    _, _, res = ols(trump_terms(d), d['tricks'], d['n'])
    print(f"{e:4} | {sd(fit[e], d['n']):.3f} -> {sd(res, d['n']):.3f} | {r2(res, d['tricks'], d['n']):.3f}")

print('\n== the hand rule, no fitted terms: b | sd | AUC 10+ | AUC 12+')
d = D['nltc']
for name, x in RULES:
    beta, _, res = ols([x], d['tricks'], d['n'])
    print(f"{name:28} | {beta[1]:+.3f} | {sd(res, d['n']):.3f} | "
          f"{auc(-x, d['tricks'] >= 10, d['n']):.3f} | {auc(-x, d['tricks'] >= 12, d['n']):.3f}")

print('\n== who a slam threshold selects, major fits: cases | share with a void or a 7+ card suit | 12+ tricks: those / the rest')
for name, e, pick in [('NLTC <= 12.5', 'nltc', lambda d: d['x'] <= 12.5), ('support points >= 31', 'sp', lambda d: d['x'] >= 31)]:
    d = D[e]
    m = pick(d) & (d['major'] == 1)
    wild = (d['shape'] == 3) | (d['long7'] == 1)
    make = lambda s: d['n'][s & (d['tricks'] >= 12)].sum() / d['n'][s].sum()
    print(f"{name:20} | {d['n'][m].sum() / 1e6:.2f}M | {d['n'][m & wild].sum() / d['n'][m].sum():.0%} | {make(m & wild):.0%} / {make(m & ~wild):.0%}")
