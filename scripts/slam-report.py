#!/usr/bin/env python3
"""Tables for docs/suit-slam.md and docs/notrump-slam.md from
`examples/probe-slam.rs` output.

    python3 scripts/slam-report.py slam.tsv

Prints, for suit slams and then for notrump slams: the IMPs one threshold
wins at the small and the grand slam, alone and behind a control check; the
ranking power of each count; the chance of 12 tricks near each threshold; and
what each card, each short suit and each wasted honour is worth in the slam
zone against all hands.
"""
import bisect
import sys

import numpy as np

COLS = {'suit': ['value', 'shape', 'major', 'T', 'keys', 'open', 'tricks'],
        'nt': ['value', 'pop', 'keys', 'open', 'tricks']}
TERMS = {'smoment': 34, 'nmoment': 24}  # the regressors of probe-slam.rs; three targets follow
WIDTH = {k: n + 3 for k, n in TERMS.items()}
TRICKS, SMALL, GRAND = 0, 1, 2  # the targets: tricks, 12 or more, 13
SETS = {'smoment': 12, 'nmoment': 16}
HALVES = {'nltc': 2, 'fifths': 2, 'bumrap': 4, 'aq': 8, 'slam': 8, 'seen': 8, 'hl': 20, 'fl': 20, 'nts': 20}  # counts kept in fractions
raw = {}
moments = {k: np.zeros((SETS[k], w, w)) for k, w in WIDTH.items()}
with open(sys.argv[1]) as f:
    for line in f:
        t, *rest = line.split('\t')
        if t in moments:
            moments[t][int(rest[0]), int(rest[1]), int(rest[2])] = float(rest[3])
        else:
            raw.setdefault((t, rest[0]), []).append(rest[1:])
D = {}
for (t, e), rows in raw.items():
    a = np.array(rows, dtype=float).T
    D[t, e] = dict(zip(COLS[t], a), n=a[-1])
    D[t, e]['x'] = D[t, e]['value'] / HALVES.get(e, 1)


def ols(cols, y, w):
    """Weighted least squares over cells: beta, residuals."""
    X = np.column_stack([np.ones(len(y))] + [np.asarray(c, dtype=float) for c in cols])
    beta = np.linalg.solve(X.T @ (X * w[:, None]), X.T @ (w * y))
    return beta, y - X @ beta


def sd(res, w):
    return np.sqrt((w * res**2).sum() / w.sum())


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
    """An undoubled contract: `per` a trick, `first` extra for the first (notrump)."""
    if tricks < level + 6:
        return -(level + 6 - tricks) * (100 if vul else 50)
    game = per * level + first >= 100
    bonus = (500 if vul else 300) if game else 50
    bonus += {6: 750 if vul else 500, 7: 1500 if vul else 1000}.get(level, 0)
    return per * (tricks - 6) + first + bonus


def gains(hi, lo, vul, **kw):
    return np.array([imps(score(hi, t, vul, **kw) - score(lo, t, vul, **kw)) for t in range(14)])


def threshold(s, g, w, total):
    """Best rule `bid iff s >= t`: (mIMP per case of `total`, t)."""
    if not len(s):
        return 0.0, np.nan
    order = np.argsort(-s, kind='stable')
    run = np.cumsum((w * g)[order])
    last = np.flatnonzero(np.r_[s[order][1:] != s[order][:-1], True])
    i = last[np.argmax(run[last])]
    return 1000 * max(run[i], 0) / total, s[order][i]


def decide(d, s, pick, hi, lo, vul, check, **kw):
    """One threshold over the cases of `pick`, bidding only where `check` holds."""
    m = pick & check
    g = gains(hi, lo, vul, **kw)[d['tricks'][m].astype(int)]
    return threshold(s[m], g, d['n'][m], d['n'][pick].sum())


def perfect(d, pick, hi, lo, vul, **kw):
    g = gains(hi, lo, vul, **kw)[d['tricks'][pick].astype(int)]
    return 1000 * (d['n'][pick] * np.maximum(g, 0)).sum() / d['n'][pick].sum()


def small_check(d):
    return (d['keys'] <= 1) & (d['open'] == 0)


def grand_check(d):
    return (d['keys'] == 0) & (d['open'] == 0)


def no_check(d):
    return d['keys'] >= 0


def decision_table(table, scales, pick, levels, **kw):
    """Rows of scales; columns small and grand slam, NV and vul, alone and checked."""
    small, game = levels
    cols = [(small, game, False, no_check), (small, game, True, no_check),
            (small, game, False, small_check), (small, game, True, small_check),
            (7, small, False, no_check), (7, small, True, no_check),
            (7, small, False, grand_check), (7, small, True, grand_check)]
    d = D[table, scales[0][1]]
    print('perfect knowledge of the tricks'.ljust(40) + ' | ' + ' | '.join(
        f'{perfect(d, pick(d), hi, lo, vul, **kw):.0f}' for hi, lo, vul, _ in cols))
    for name, e, val in scales:
        d = D[table, e]
        out = []
        for hi, lo, vul, check in cols:
            g, t = decide(d, val(d), pick(d), hi, lo, vul, check(d), **kw)
            out.append(f'{g:.0f} ({t:g})')
        print(f'{name:40} | ' + ' | '.join(out))


def auc_table(table, scales, pick):
    tests = [('12+ tricks', 12, 0), ('12+ among 10+', 12, 10), ('12+ among 11+', 12, 11), ('13 among 12+', 13, 12)]
    for name, e, val in scales:
        d = D[table, e]
        out = []
        for _, need, floor in tests:
            m = pick(d) & (d['tricks'] >= floor)
            out.append(f"{auc(val(d)[m], d['tricks'][m] >= need, d['n'][m]):.3f}")
        print(f'{name:40} | ' + ' | '.join(out))
    print(' ' * 43 + ' | '.join(name for name, _, _ in tests))


def odds_table(table, scales, pick, lo_hi):
    """P(12+ tricks) and P(13) at each value near the slam thresholds."""
    for (name, e, val), (lo, hi, step) in zip(scales, lo_hi):
        d = D[table, e]
        s, m = val(d), pick(d)
        for v in np.arange(lo, hi + step / 2, step):
            c = m & (s == v)
            w = d['n'][c]
            if w.sum():
                print(f"{name:32} {v:5g} | {w.sum() / 1e3:9.0f}k | {w[d['tricks'][c] >= 12].sum() / w.sum():.1%} | "
                      f"{w[d['tricks'][c] >= 13].sum() / w.sum():.1%} | checked: "
                      f"{d['n'][c & small_check(d) & (d['tricks'] >= 12)].sum() / max(d['n'][c & small_check(d)].sum(), 1):.1%} "
                      f"of {d['n'][c & small_check(d)].sum() / w.sum():.0%}")


def moment_fit(kind, sets, rows, target=TRICKS):
    """Least squares with one constant per set: beta of `rows`, sd, standard errors, cases."""
    M_all, w, y = moments[kind], TERMS[kind], TERMS[kind] + target
    k = len(rows)
    xtx, xty = np.zeros((k + len(sets),) * 2), np.zeros(k + len(sets))
    n = yty = 0.0
    for j, s in enumerate(sets):
        M = M_all[s]
        const = np.zeros(w)
        const[0] = 1
        L = np.vstack(rows + [const])
        idx = list(range(k)) + [k + j]
        xtx[np.ix_(idx, idx)] += L @ M[:w, :w] @ L.T
        xty[idx] += L @ M[:w, y]
        n, yty = n + M[0, 0], yty + M[y, y]
    live = np.flatnonzero(np.diag(xtx))  # a term or a set that never occurs fits nothing
    beta, se = np.full(len(xty), np.nan), np.full(len(xty), np.nan)
    beta[live] = np.linalg.solve(xtx[np.ix_(live, live)], xty[live])
    sse = yty - beta[live] @ xty[live]
    se[live] = np.sqrt(np.diag(np.linalg.inv(xtx[np.ix_(live, live)])) * sse / (n - len(live)))
    return beta[:k], np.sqrt(sse / n), se[:k], n


def unit(kind, i):
    r = np.zeros(TERMS[kind])
    r[i] = 1
    return r


def term_table(kind, names, zones, target=TRICKS, skip=()):
    """Every regressor but `skip` fitted together, per zone: the target, and relative to an ace = 4."""
    keep = [i for i, name in enumerate(names) if not any(word in name for word in skip)]
    rows = [unit(kind, i + 1) for i in keep]
    cols = []
    for label, sets in zones:
        beta, s, se, n = moment_fit(kind, sets, rows, target)
        cols.append((label, beta, s, 1.96 * np.nanmax(se), n))
    print(' ' * 46 + ' | '.join(f'{label} ({n / 1e6:.1f}M, sd {s:.3f}, ±{ci:.3f})' for label, _, s, ci, n in cols))
    for j, i in enumerate(keep):
        print(f'{names[i]:44} | ' + ' | '.join(
            f'{b[j]:+.3f} [{4 * b[j] / b[0]:+.2f}]' if np.isfinite(b[j]) else '      -      ' for _, b, _, _, _ in cols))


# ---------------------------------------------------------------- suit slams
def suit(e):
    return D['suit', e]


SUIT_SCALES = [('HCP', 'hcp', lambda d: d['x']),
               ('point_count', 'pc', lambda d: d['x']),
               ('support points', 'sp', lambda d: d['x']),
               ('support points + trumps', 'sp', lambda d: d['x'] + d['T']),
               ('support points + controls', 'spc', lambda d: d['x']),
               ('support points + controls + trumps', 'spc', lambda d: d['x'] + d['T']),
               ('Zar', 'zar', lambda d: d['x']),
               ('NLTC', 'nltc', lambda d: -d['x'])]
def trumps(d):
    """A point for the 9th trump, half for the 10th, a quarter for the 11th."""
    return np.select([d['T'] == 9, d['T'] == 10, d['T'] >= 11], [1, 1.5, 1.75], 0)


SUIT_SCALES.insert(4, ('support points + 1, ½, ¼ for trumps 9-11', 'sp', lambda d: d['x'] + trumps(d)))
NEW = [('4-2-1-½', 'aq'), ('slam fit, hands apart', 'slam'), ('slam fit, shortness known', 'seen')]
for name, e in NEW:
    for k in (0, 0.5, 1, 1.5):
        SUIT_SCALES.append((f'{name} + {k:g} per trump', e, lambda d, k=k: np.round(2 * (d['x'] + k * d['T'])) / 2))
    SUIT_SCALES.append((f'{name} + 1, ½, ¼ for trumps 9-11', e, lambda d: np.round(4 * (d['x'] + trumps(d))) / 4))
MAJOR, MINOR = (lambda d: d['major'] == 1), (lambda d: d['major'] == 0)

d = suit('hcp')
print(f"fit cases: {d['n'].sum() / 1e6:.2f}M ({d['n'][MAJOR(d)].sum() / 1e6:.2f}M major)")
print(f"control check passes: small {d['n'][small_check(d)].sum() / d['n'].sum():.1%}, grand {d['n'][grand_check(d)].sum() / d['n'].sum():.1%}")
for need, check in ((12, small_check), (13, grand_check)):
    m = d['tricks'] >= need
    print(f"of pairs taking {need}+ ({d['n'][m].sum() / d['n'].sum():.2%} of cases): check passes {d['n'][m & check(d)].sum() / d['n'][m].sum():.1%}")

print('\n== suit slam, major fits: mIMP per case (threshold) | 6M/4M NV | vul | checked NV | vul | 7M/6M NV | vul | checked NV | vul')
decision_table('suit', SUIT_SCALES, MAJOR, (6, 4))
print('\n== suit slam, minor fits: 6m/5m and 7m/6m, same columns')
decision_table('suit', SUIT_SCALES, MINOR, (6, 5), per=20)
for name, pick in [('both balanced', lambda d: MAJOR(d) & (d['shape'] == 0)), ('no singleton', lambda d: MAJOR(d) & (d['shape'] == 1)),
                   ('a singleton', lambda d: MAJOR(d) & (d['shape'] == 2)), ('a void', lambda d: MAJOR(d) & (d['shape'] == 3)),
                   ('8 trumps', lambda d: MAJOR(d) & (d['T'] == 8)), ('9 trumps', lambda d: MAJOR(d) & (d['T'] == 9)),
                   ('10+ trumps', lambda d: MAJOR(d) & (d['T'] >= 10))]:
    print(f'\n== suit slam, major fits, {name}')
    decision_table('suit', SUIT_SCALES, pick, (6, 4))

def break_even(d, s, pick, hi, lo, vul):
    """The value where the mean IMP gain of bidding crosses zero, between neighbouring values."""
    g, n, s = gains(hi, lo, vul)[d['tricks'][pick].astype(int)] * d['n'][pick], d['n'][pick], s[pick]
    values = np.array([v for v in np.unique(s) if n[s == v].sum() >= 1000])  # thin cells cross by chance
    mean = np.array([g[s == v].sum() / n[s == v].sum() for v in values])
    i = np.flatnonzero((mean[:-1] < 0) & (mean[1:] >= 0))[-1]
    return values[i] + (values[i + 1] - values[i]) * -mean[i] / (mean[i + 1] - mean[i])


print('\n== suit slam, major fits: break-even value by trump length, 6M/4M NV / vul [checked NV] (worth of this trump, NV)')
for name, e in [('HCP', 'hcp'), ('point_count', 'pc'), ('support points', 'sp'), ('support points + controls', 'spc'),
                ('4-2-1-½, no trump term', 'aq'), ('slam fit, hands apart, no trump term', 'slam')]:
    d = suit(e)
    out, last = [], None
    for t in (8, 9, 10, 11):
        pick = MAJOR(d) & (d['T'] == t)
        x = np.round(2 * d['x']) / 2
        nv, vul = (break_even(d, x, pick, 6, 4, v) for v in (False, True))
        checked = break_even(d, x, pick & small_check(d), 6, 4, False)
        out.append(f'{nv:.2f} / {vul:.2f} [{checked:.2f}]' + ('' if last is None else f' ({last - nv:.2f})'))
        last = nv
    print(f'{name:38} | ' + ' | '.join(out))
print(' ' * 41 + '8 trumps | 9 trumps | 10 trumps | 11+ trumps')

print('\n== suit slam: ranking power (AUC), all fit cases')
auc_table('suit', SUIT_SCALES, lambda d: d['tricks'] >= 0)

print('\n== suit slam, major fits: value | cases | 12+ tricks | 13 tricks | 12+ with the control check passed (share passing)')
ODDS = {'HCP': (28, 34, 1), 'point_count': (29, 35, 1), 'support points + trumps': (38, 44, 1),
        'support points + controls + trumps': (46, 53, 1), 'Zar': (59, 66, 1),
        '4-2-1-½ + 0.5 per trump': (26.5, 31, 0.5), 'slam fit, hands apart + 0.5 per trump': (27, 31.5, 0.5)}
odds_table('suit', [x for x in SUIT_SCALES if x[0] in ODDS], MAJOR, [ODDS[x[0]] for x in SUIT_SCALES if x[0] in ODDS])

SUIT_TERMS = [f'side {c}' for c in ('ace', 'king', 'queen', 'jack', 'ten')]
SUIT_TERMS += [f'trump {c}' for c in ('ace', 'king', 'queen', 'jack', 'ten')]
SUIT_TERMS += [f'side {short}, hand with {trumps} trumps' for trumps in ('4+', '3', '2 or fewer')
               for short in ('void', 'singleton', 'doubleton')]
SUIT_TERMS += ['singleton side K', 'singleton side Q', 'singleton side J',
               'side doubleton Qx, Jx or QJ', 'side doubleton AQ, AJ, KQ or KJ']
SUIT_TERMS += [f"side {c} facing partner's {name}" for name in ('void', 'singleton') for c in 'AKQJ']
SUIT_TERMS += ['a second fit of 8+ cards']
SUIT_ZONES = [('all', range(12)), ('under 34', range(0, 3)), ('34-37', range(3, 6)), ('38-41', range(6, 9)),
              ('42+', range(9, 12)), ('38+, 8 trumps', [6, 9]), ('38+, 9 trumps', [7, 10]), ('38+, 10+ trumps', [8, 11])]
print('\n== suit: every term fitted together, by zone of support points + trumps: tricks [points where a side ace = 4]')
term_table('smoment', SUIT_TERMS, SUIT_ZONES)
print('\n== suit: the same for the chance of 12+ tricks')
term_table('smoment', SUIT_TERMS, SUIT_ZONES, SMALL)
print('\n== suit: the chance of 13 tricks')
term_table('smoment', SUIT_TERMS, SUIT_ZONES, GRAND)
print("\n== suit, hands apart (no term for partner's shortness): the chance of 12+ tricks")
term_table('smoment', SUIT_TERMS, SUIT_ZONES, SMALL, skip=('facing',))


# ------------------------------------------------------------- notrump slams
def nt(e):
    return D['nt', e]


NT_SCALES = [('HCP', 'hcp'), ('Fifths', 'fifths'), ('BUM-RAP', 'bumrap'), ('HCP + controls', 'hc'),
             ('point_count', 'pc'), ('Zar', 'zar')]
NT_SCALES += [('HCP + length', 'hl'), ('Fifths + length', 'fl'), ('slam fit', 'nts')]
NT_SCALES = [(name, e, (lambda d: np.round(4 * d['x']) / 4) if e in ('hl', 'fl', 'nts') else (lambda d: d['x']))
             for name, e in NT_SCALES]
NT_POPS = [('both balanced, no 8-card major fit', lambda d: d['pop'] == 0),
           ('both balanced', lambda d: d['pop'] <= 1),
           ('no 8-card major fit', lambda d: d['pop'] % 2 == 0),
           ('all pairs', lambda d: d['pop'] >= 0)]
d = nt('hcp')
print(f"\n\nnotrump cases: {d['n'].sum() / 1e6:.2f}M")
for name, pick in NT_POPS:
    m = pick(d)
    print(f"{name}: {d['n'][m].sum() / 1e6:.2f}M, 12+ tricks {d['n'][m & (d['tricks'] >= 12)].sum() / d['n'][m].sum():.2%}")
for name, pick in NT_POPS:
    print(f'\n== notrump slam, {name}: mIMP per case (threshold) | 6NT/3NT NV | vul | checked NV | vul | 7NT/6NT NV | vul | checked NV | vul')
    decision_table('nt', NT_SCALES, pick, (6, 3), first=10)
print('\n== notrump slam: ranking power (AUC), both balanced, no 8-card major fit')
auc_table('nt', NT_SCALES, NT_POPS[0][1])
print('\n== notrump slam: ranking power (AUC), no 8-card major fit')
auc_table('nt', NT_SCALES, NT_POPS[2][1])
print('\n== notrump slam, both balanced, no 8-card major fit: value | cases | 12+ tricks | 13 tricks | checked')
odds_table('nt', NT_SCALES[:2], NT_POPS[0][1], [(29, 37, 1), (29, 35, 0.5)])
print('\n== notrump slam, no 8-card major fit: the same')
odds_table('nt', NT_SCALES[:2], NT_POPS[2][1], [(29, 37, 1), (29, 35, 0.5)])

NT_TERMS = ['ace', 'king', 'queen', 'jack', 'ten', 'nine',
            '1st length point', '2nd length point', '3rd length point', '4th length point',
            'a 4-3-3-3 hand', 'longest combined suit 8+', 'longest combined suit 9+', 'longest combined suit 10+',
            'singleton K', 'singleton Q', 'singleton J', 'doubleton Qx, Jx or QJ', 'doubleton AQ, AJ, KQ or KJ',
            'doubleton AK', "K facing partner's void or singleton", "Q facing partner's void or singleton",
            "J facing partner's void or singleton"]


def nt_zones(pops):
    return [(label, [4 * z + p for z in zs for p in pops])
            for label, zs in [('all', range(4)), ('under 28', [0]), ('28-31', [1]), ('32-35', [2]), ('36+', [3]), ('28+', [1, 2, 3])]]


for name, pops in [('both balanced, no 8-card major fit', [0]), ('both balanced', [0, 1]), ('no 8-card major fit', [0, 2])]:
    print(f'\n== notrump, {name}: every term fitted together, by zone of HCP: tricks [points where an ace = 4]')
    term_table('nmoment', NT_TERMS, nt_zones(pops))
    print(f'-- the chance of 12+ tricks')
    term_table('nmoment', NT_TERMS, nt_zones(pops), SMALL)
