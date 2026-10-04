#!/usr/bin/env python3
"""Tables for docs/zar.md from `examples/probe-zar.rs` output.

    python3 scripts/zar-report.py zar.tsv

Prints: Zar against the point counts with a fit (trick fit by shape and by
trump length, ranking power, the IMPs one threshold wins at game and slam);
the same at notrump; what each part of Zar, each card and each wasted honour
is worth; and one hand's count against what its side can make (the opening).
"""
import bisect
import sys

import numpy as np

COLS = {'suit': ['value', 'shape', 'long7', 'major', 'T', 'tricks'],
        'nt': ['value', 'shape', 'fit', 'majfit', 'tricks'],
        'hand': ['value', 'shape', 'own', 'game', 'off'],
        'open': ['class', 'shape', 'own', 'game', 'off', 'def'],
        'openhcp': ['class', 'hcp']}
W = 32  # regressors of `terms` in probe-zar.rs, then the tricks
raw = {}
moments = np.zeros((16, W, W))
with open(sys.argv[1]) as f:
    for line in f:
        t, *rest = line.split('\t')
        if t == 'moment':
            moments[int(rest[0]), int(rest[1]), int(rest[2])] = float(rest[3])
        elif t in ('open', 'openhcp'):
            raw.setdefault((t, ''), []).append(rest)
        else:
            raw.setdefault((t, rest[0]), []).append(rest[1:])
D = {}
for (t, e), rows in raw.items():
    a = np.array(rows, dtype=float).T
    D[t, e] = dict(zip(COLS[t], a), n=a[-1])
for (t, e), d in D.items():
    if 'value' in d:  # Fifths and the repaired Zar come in half points
        d['x'] = d['value'] / 2 if e in ('fifths', 'zarw', 'zarx') else d['value']


def ols(cols, y, w):
    """Weighted least squares over cells: beta, residuals."""
    X = np.column_stack([np.ones(len(y))] + [np.asarray(c, dtype=float) for c in cols])
    beta = np.linalg.solve(X.T @ (X * w[:, None]), X.T @ (w * y))
    return beta, y - X @ beta


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


def score(level, tricks, vul, per=30, first=0):
    """A major-suit (or, with `first` = 10, notrump) contract, undoubled."""
    if tricks < level + 6:
        return -(level + 6 - tricks) * (100 if vul else 50)
    game = per * level + first >= 100
    bonus = (500 if vul else 300) if game else 50
    bonus += {6: 750 if vul else 500, 7: 1500 if vul else 1000}.get(level, 0)
    return per * (tricks - 6) + first + bonus


def gains(hi, lo, vul, **kw):
    return np.array([imps(score(hi, t, vul, **kw) - score(lo, t, vul, **kw)) for t in range(14)])


def threshold(s, g, w):
    """Best rule `bid iff s >= t`: (mIMP per case, t)."""
    order = np.argsort(-s, kind='stable')
    total = np.cumsum((w * g)[order])
    last = np.flatnonzero(np.r_[s[order][1:] != s[order][:-1], True])
    i = last[np.argmax(total[last])]
    return 1000 * max(total[i], 0) / w.sum(), s[order][i]


def at(s, g, w, t):
    """mIMP per case of the rule `bid iff s >= t`."""
    return 1000 * (w * g)[s >= t].sum() / w.sum()


SHAPES = [('both balanced', lambda d: d['shape'] == 0),
          ('no singleton', lambda d: d['shape'] == 1),
          ('a singleton', lambda d: d['shape'] == 2),
          ('a void', lambda d: d['shape'] == 3)]
SUIT_ROWS = SHAPES + [('a 7+ card suit', lambda d: d['long7'] == 1)]
SUIT_ROWS += [(f'{t} trumps', lambda d, t=t: d['T'] == t) for t in (8, 9, 10)]
SUIT_ROWS += [('11+ trumps', lambda d: d['T'] == 11), ('major', lambda d: d['major'] == 1),
              ('minor', lambda d: d['major'] == 0)]


def suit(e):
    return D['suit', e]


def wiki_fit(d):
    """Extra trumps by the pair's shortness: 2 each with a void, 1 with a singleton."""
    return (d['T'] - 8) * np.select([d['shape'] == 3, d['shape'] == 2], [2, 1], 0)


# name, table eval, value (higher is better)
SCALES = [('HCP', 'hcp', lambda d: d['x']),
          ('point_count', 'pc', lambda d: d['x']),
          ('support points', 'sp', lambda d: d['x']),
          ('support points + trumps', 'sp', lambda d: d['x'] + d['T']),
          ('support points + controls', 'spc', lambda d: d['x']),
          ('support points + controls + trumps', 'spc', lambda d: d['x'] + d['T']),
          ('support points + controls + 1.5 trumps', 'spc', lambda d: d['x'] + 1.5 * d['T']),
          ('Zar, no discount', 'zar0', lambda d: d['x']),
          ('Zar', 'zar', lambda d: d['x']),
          ('Zar + 3 per trump over 8', 'zar', lambda d: d['x'] + 3 * (d['T'] - 8)),
          ('Zar + 2/1 per trump (void/singleton)', 'zar', lambda d: d['x'] + wiki_fit(d)),
          ('Zar, waste repaired', 'zarw', lambda d: d['x']),
          ('Zar, waste + trump terms', 'zarx', lambda d: d['x'])]

d = suit('zar')
print(f"fit cases: {d['n'].sum() / 1e6:.2f}M ({d['n'][d['major'] == 1].sum() / 1e6:.2f}M with a major as trumps)")

print('\n== with a fit: tricks = a + b * value, all cases\nscale | b | a | sd | R2')
FIT = {}
for name, e, val in SCALES:
    d = suit(e)
    beta, res = ols([val(d)], d['tricks'], d['n'])
    FIT[name] = res
    print(f"{name:38} | {beta[1]:+.3f} | {beta[0]:6.2f} | {sd(res, d['n']):.3f} | {r2(res, d['tricks'], d['n']):.3f}")

print('\n== with a fit, by row: sd of the fit within the row [mean miss of the all-cases fit]')
for row, pick in SUIT_ROWS:
    out = []
    for name, e, val in SCALES:
        d = suit(e)
        m = pick(d)
        y, w = d['tricks'][m], d['n'][m]
        _, res = ols([val(d)[m]], y, w)
        out.append(f"{sd(res, w):.3f} [{(w * FIT[name][m]).sum() / w.sum():+.2f}]")
    print(f"{row:14} | {w.sum() / 1e6:6.2f}M | " + ' | '.join(out))
print(' ' * 26 + ' | '.join(name for name, _, _ in SCALES))

print('\n== ranking power (AUC), fit cases')
TESTS = [('10+ tricks', 10, lambda d: d['tricks'] >= 0), ('12+ tricks', 12, lambda d: d['tricks'] >= 0),
         ('12+ given 10+', 12, lambda d: d['tricks'] >= 10), ('13 given 12+', 13, lambda d: d['tricks'] >= 12)]
TESTS += [(f'10+ tricks, {name}', 10, pick) for name, pick in SHAPES]
TESTS += [(f'12+ tricks, {name}', 12, pick) for name, pick in SHAPES]
for name, need, pick in TESTS:
    out = []
    for _, e, val in SCALES:
        d = suit(e)
        m = pick(d)
        out.append(f"{auc(val(d)[m], d['tricks'][m] >= need, d['n'][m]):.3f}")
    print(f'{name:30} | ' + ' | '.join(out))

DECISIONS = [('4M/3M NV', 4, 3, False), ('4M/3M vul', 4, 3, True), ('6M/4M NV', 6, 4, False),
             ('6M/4M vul', 6, 4, True), ('7M/6M NV', 7, 6, False), ('7M/6M vul', 7, 6, True)]
print('\n== one threshold, major fits: mIMP per case won over never bidding (threshold)')
d = suit('zar')
m = d['major'] == 1
t, w = d['tricks'][m].astype(int), d['n'][m]
print('perfect knowledge of the tricks | ' + ' | '.join(
    f"{name} {1000 * (w * np.maximum(gains(hi, lo, vul)[t], 0)).sum() / w.sum():.0f}"
    for name, hi, lo, vul in DECISIONS))
for name, e, val in SCALES:
    d = suit(e)
    m = d['major'] == 1
    out = []
    for dn, hi, lo, vul in DECISIONS:
        g, t = threshold(val(d)[m], gains(hi, lo, vul)[d['tricks'][m].astype(int)], d['n'][m])
        out.append(f'{g:.0f} ({t:g})')
    print(f'{name:38} | ' + ' | '.join(out))

print("\n== Zar's own thresholds, major fits: mIMP per case at 52 / 62 / 67 (NV, vul)")
for name, e, val in [x for x in SCALES if x[1] == 'zar']:
    d = suit(e)
    m = d['major'] == 1
    s, y, w = val(d)[m], d['tricks'][m].astype(int), d['n'][m]
    out = [f"{at(s, gains(hi, lo, vul)[y], w, t):.0f}" for t, (_, hi, lo, vul) in zip((52, 52, 62, 62, 67, 67), DECISIONS)]
    print(f'{name:38} | ' + ' | '.join(out))

print('\n== the decisions by shape, major fits, NV: mIMP per case of that shape (threshold)')
for row, pick in SHAPES:
    for dn, hi, lo, vul in DECISIONS[::2][:2]:
        out = []
        for name, e, val in SCALES:
            d = suit(e)
            m = (d['major'] == 1) & pick(d)
            g, t = threshold(val(d)[m], gains(hi, lo, vul)[d['tricks'][m].astype(int)], d['n'][m])
            out.append(f'{g:.0f} ({t:g})')
        print(f'{row:14} {dn:9} | ' + ' | '.join(out))

# Notrump
NT = [('HCP', 'hcp'), ('Fifths', 'fifths'), ('point_count', 'pc'), ('Zar, no discount', 'zar0'), ('Zar', 'zar')]
POPS = [('no 8-card fit', lambda d: d['fit'] == 0), ('no 8-card major fit', lambda d: d['majfit'] == 0),
        ('all pairs', lambda d: d['fit'] >= 0)]
POPS += [(f'no 8-card fit, {name}', lambda d, pick=pick: (d['fit'] == 0) & pick(d)) for name, pick in SHAPES[:3]]
print('\n== notrump tricks = a + b * value: b | sd | R2 | AUC 9+ | 3NT over 2NT mIMP NV (t) / vul (t)')
for pop, pick in POPS:
    d = D['nt', 'hcp']
    print(f"-- {pop}: {d['n'][pick(d)].sum() / 1e6:.2f}M")
    for name, e in NT:
        d = D['nt', e]
        m = pick(d)
        x, y, w = d['x'][m], d['tricks'][m], d['n'][m]
        beta, res = ols([x], y, w)
        dec = ['{:.0f} ({:g})'.format(*threshold(x, gains(3, 2, vul, first=10)[y.astype(int)], w)) for vul in (False, True)]
        print(f"{name:18} | {beta[1]:+.3f} | {sd(res, w):.3f} | {r2(res, y, w):.3f} | {auc(x, y >= 9, w):.3f} | " + ' / '.join(dec))

# What each part of Zar is worth: least squares from the moment sums.  The
# base regressors, in the order of `terms` in probe-zar.rs.
A, K, Q, J, T10, LA, LB, LD = range(1, 9)
OWN = [(9, 'singleton side K'), (10, 'singleton side Q'), (11, 'singleton side J'),
       (12, 'side doubleton Qx, Jx or QJ'), (13, 'side doubleton AQ, AJ, KQ or KJ')]
SHORT_TRUMP = [(14, 'trump honour Zar discounts (short K, Q, J)')]
FACING = [(15 + 4 * s + i, f"side {c} facing partner's {name}")
          for s, name in enumerate(('void', 'singleton')) for i, c in enumerate('AKQJ')]
TRUMP = [(23 + i, f'trump {c}') for i, c in enumerate(['ace', 'king', 'queen', 'jack', 'ten'])]
SHORTER = [(28 + i, f'shorter trump holding has {2 - i} cards') for i in range(3)]


def row(weights):
    r = np.zeros(W - 1)
    for i, x in weights.items():
        r[i] = x
    return r


HCP = row({A: 4, K: 3, Q: 2, J: 1})
CTRL = row({A: 2, K: 1})
ZAR0 = row({A: 6, K: 4, Q: 2, J: 1, LA: 2, LB: 1, LD: -1})
PARTS = [HCP, CTRL, row({LA: 1, LB: 1}), row({LA: 1, LD: -1})]
FREE = [row({i: 1}) for i in range(1, 9)]


def one(terms):
    return [row({i: 1}) for i, _ in terms]


def moment_fit(pops, rows, shapes=range(4)):
    """Least squares with one constant per population: beta of `rows`, sd, R2, the constants, standard errors."""
    k = len(rows)
    xtx, xty = np.zeros((k + len(pops),) * 2), np.zeros(k + len(pops))
    n = yty = ysum = 0.0
    for j, p in enumerate(pops):
        M = sum(moments[4 * p + s] for s in shapes)
        L = np.vstack(rows + [row({0: 1})])
        idx = list(range(k)) + [k + j]
        xtx[np.ix_(idx, idx)] += L @ M[:-1, :-1] @ L.T
        xty[idx] += L @ M[:-1, -1]
        n, yty, ysum = n + M[0, 0], yty + M[-1, -1], ysum + M[0, -1]
    live = np.flatnonzero(np.diag(xtx))  # a term that never occurs here fits nothing
    beta = np.full(len(xty), np.nan)
    beta[live] = np.linalg.solve(xtx[np.ix_(live, live)], xty[live])
    sse = yty - beta[live] @ xty[live]
    se = np.full(len(xty), np.nan)
    se[live] = np.sqrt(np.diag(np.linalg.inv(xtx[np.ix_(live, live)])) * sse / (n - len(live)))
    return beta[:k], np.sqrt(sse / n), 1 - sse / (yty - ysum**2 / n), beta[k:], se[:k]


SUITS, NOTRUMP = [1, 2, 3], [0]
print('\n== the parts of Zar: sd of the trick fit, one constant per trump length')
MODELS = [('constant only', lambda p: []),
          ('HCP', lambda p: [HCP]),
          ('Zar, no discount', lambda p: [ZAR0]),
          ('HCP, controls, a+b, a-d fitted apart', lambda p: PARTS),
          ('A K Q J T and a, b, d fitted apart', lambda p: FREE),
          ('+ own short side honours', lambda p: FREE + one(OWN)),
          ("+ honours facing partner's shortness", lambda p: FREE + one(OWN + FACING)),
          ('+ trump terms', lambda p: FREE + one(OWN + FACING + SHORT_TRUMP + SHORTER + TRUMP) if p else None),
          ('Zar, no discount, + waste terms', lambda p: [ZAR0] + one(OWN + FACING)),
          ('Zar, no discount, + waste and trump terms',
           lambda p: [ZAR0] + one(OWN + FACING + SHORT_TRUMP + SHORTER + TRUMP) if p else None)]
POPULATIONS = [('fit, all', SUITS), ('8 trumps', [1]), ('9 trumps', [2]), ('10+ trumps', [3]), ('notrump, no fit', NOTRUMP)]
for name, model in MODELS:
    out = []
    for _, pops in POPULATIONS:
        rows = model(pops != NOTRUMP)
        out.append('  -  ' if rows is None else f'{moment_fit(pops, rows)[1]:.3f}')
    print(f'{name:40} | ' + ' | '.join(out))
print(' ' * 43 + ' | '.join(name for name, _ in POPULATIONS))

print("\n== Zar's four parts fitted apart: tricks per point [in Zar points: / the slope of Zar]")
for name, pops in POPULATIONS:
    zar_b = moment_fit(pops, [ZAR0])[0][0]
    beta = moment_fit(pops, PARTS)[0]
    print(f"{name:16} | Zar {zar_b:+.3f} | " + ' | '.join(
        f'{part} {b:+.3f} [{b / zar_b:+.2f}]' for part, b in zip(('HCP', 'control', 'a+b', 'a-d'), beta)))

print('\n== every card and every waste term, fitted together: tricks [Zar points] (Zar prices A K Q J at 6 4 2 1, a b d at 2 1 -1)')
for name, pops in POPULATIONS:
    suit_play = pops != NOTRUMP
    extra = OWN + FACING + (SHORT_TRUMP + SHORTER + TRUMP if suit_play else [])
    zar_b = moment_fit(pops, [ZAR0])[0][0]
    beta, _, _, _, se = moment_fit(pops, FREE + one(extra))
    names = ['ace', 'king', 'queen', 'jack', 'ten', 'longest suit a', 'second suit b', 'shortest suit d']
    print(f'-- {name}: one Zar point = {zar_b:.3f} tricks; widest 95% interval ± {1.96 * se.max():.3f} tricks')
    for label, b in zip(names + [label for _, label in extra], beta):
        print(f'{label:44} | {b:+.3f} | {b / zar_b:+.2f}')

print('\n== what Zar as it stands owes: Zar (no discount) and the terms fitted together, in Zar points')
print(' ' * 44 + ' | ' + ' | '.join(name for name, _ in POPULATIONS) + ' | fit: ' + ' | '.join(name for name, _ in SHAPES))
for with_trump in (False, True):
    cols = []
    for pops, shapes in [(pops, range(4)) for _, pops in POPULATIONS] + [(SUITS, [s]) for s in range(4)]:
        suit_play = pops != NOTRUMP
        extra = OWN + FACING + (SHORT_TRUMP + SHORTER + TRUMP if suit_play and with_trump else [])
        beta, s, _, const, _ = moment_fit(pops, [ZAR0] + one(extra), shapes)
        cols.append(dict(zip([label for _, label in extra], beta[1:] / beta[0]), sd=s))
        if len(const) == 3:
            cols[-1].update({'9 trumps (against 8)': (const[1] - const[0]) / beta[0],
                             '10+ trumps (against 8)': (const[2] - const[0]) / beta[0]})
    for label in [label for _, label in OWN + FACING + (SHORT_TRUMP + SHORTER + TRUMP if with_trump else [])] + [
            '9 trumps (against 8)', '10+ trumps (against 8)', 'sd']:
        fmt = '{:.3f}' if label == 'sd' else '{:+.2f}'
        print(f'{label:44} | ' + ' | '.join(
            fmt.format(c[label]) if np.isfinite(c.get(label, np.nan)) else '  -  ' for c in cols))
    print()

# One hand: the opening
HAND = [('HCP', 'hcp'), ('point_count', 'pc'), ('HCP + two longest suits', 'r20'), ('Zar, no discount', 'zar0'), ('Zar', 'zar')]
HAND_SHAPES = [('all hands', lambda d: d['shape'] >= 0), ('balanced', lambda d: d['shape'] == 0),
               ('no singleton', lambda d: d['shape'] == 1), ('a singleton', lambda d: d['shape'] == 2),
               ('a void', lambda d: d['shape'] == 3)]
print("\n== one hand against its side's best strain: sd of the trick fit | R2 | AUC side owns the deal | AUC side makes game")
for row_name, pick in HAND_SHAPES:
    d = D['hand', 'hcp']
    print(f"-- {row_name}: {d['n'][pick(d)].sum() / 1e6:.2f}M hands")
    for name, e in HAND:
        d = D['hand', e]
        m = pick(d)
        x, y, w = d['x'][m], d['off'][m], d['n'][m]
        _, res = ols([x], y, w)
        print(f"{name:24} | {sd(res, w):.3f} | {r2(res, y, w):.3f} | "
              f"{auc(x, d['own'][m] == 1, w):.3f} | {auc(x, d['game'][m] == 1, w):.3f}")

print('\n== one hand at each value: hands | side owns the deal | side makes game | mean tricks, best strain')
for name, e, lo, hi in [('Zar', 'zar', 22, 30), ('HCP', 'hcp', 9, 15), ('point_count', 'pc', 9, 15), ('HCP + two longest suits', 'r20', 17, 23)]:
    d = D['hand', e]
    for v in range(lo, hi + 1):
        m = d['x'] == v
        w = d['n'][m]
        print(f"{name:24} {v:2} | {w.sum() / 1e6:6.2f}M | {(w * d['own'][m]).sum() / w.sum():.1%} | "
              f"{(w * d['game'][m]).sum() / w.sum():.1%} | {(w * d['off'][m]).sum() / w.sum():.2f}")

print("\n== who opens: hands | mean HCP | void / singleton / balanced | side owns the deal | makes game | tricks, best strain | the other side's")
o, oh = D['open', ''], D['openhcp', '']
total = o['n'].sum()
RULES = [('Zar >= 26', 1), ('HCP >= 12', 2), ('Rule of 20', 4), ('point_count >= 12', 8)]


def describe(name, pick):
    m, mh = pick(o['class'].astype(int)), pick(oh['class'].astype(int))
    w = o['n'][m]
    share = lambda v: (w * v).sum() / w.sum()
    print(f"{name:44} | {w.sum() / 1e6:6.2f}M ({w.sum() / total:.1%}) | {(oh['n'] * oh['hcp'])[mh].sum() / oh['n'][mh].sum():.1f} | "
          f"{share(o['shape'][m] == 3):.0%} / {share(o['shape'][m] == 2):.0%} / {share(o['shape'][m] == 0):.0%} | "
          f"{share(o['own'][m]):.1%} | {share(o['game'][m]):.1%} | {share(o['off'][m]):.2f} | {share(o['def'][m]):.2f}")


for name, bit in RULES:
    describe(name, lambda c, bit=bit: c & bit > 0)
for name, bit in RULES[1:]:
    describe(f'Zar opens, {name} does not', lambda c, bit=bit: (c & 1 > 0) & (c & bit == 0))
    describe(f'{name} opens, Zar does not', lambda c, bit=bit: (c & 1 == 0) & (c & bit > 0))
describe('no rule opens', lambda c: c == 0)
describe('every rule opens', lambda c: c == 15)
