#!/usr/bin/env python3
"""Join a paired diff's per-board swings by first-divergence node (docs/bba-system.md §4).

    python3 scripts/bba-node-join.py --book ab-results/bba-book/<run> \\
        $R/diff.bba.vs.american.none.jsonl $R/diff.bba.vs.american.both.jsonl

Input: `ab-dump-diff --jsonl` records (ON = the `bba` arm, OFF = `american`,
same deals, table A, N/S ours).  Each contract-divergent board is charged to
the prefix both arms shared before their first different call.  The same
judgement chooses on both arms, so a node's sum is the price of the agreement
there: positive means BBA's bands beat ours at that node.  `walked` says
whether the prefix is a node of the book dump, i.e. whether the mask acted.

z is the node's sum over its standard error (sd × √n); a node is an authoring
lead only with |z| well above the ~3 a table of a few thousand rows throws up
by chance.
"""
import argparse, collections, glob, json, math, re

SEATS = ['North', 'East', 'South', 'West']


def book_keys(run):
    key = re.compile(r'^\{"k":"((?:[^"\\]|\\.)*)"')
    keys = set()
    for path in glob.glob(f'{run}/*.jsonl'):
        if path.rsplit('/', 1)[-1].startswith('reach-'):
            continue
        for line in open(path, encoding='utf-8'):
            m = key.match(line)
            if m:
                keys.add(json.loads(f'"{m.group(1)}"'))
    return keys


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--book', required=True, help='probe-bba-book run dir the arm played')
    ap.add_argument('--top', type=int, default=30, help='rows per table')
    ap.add_argument('--min-boards', type=int, default=30, help='smallest node in the tables')
    ap.add_argument('jsonl', nargs='+')
    args = ap.parse_args()

    walked = book_keys(args.book)
    nodes = collections.defaultdict(lambda: {'n': 0, 'p': 0.0, 'pp': 0.0, 'd': 0.0,
                                             'side': collections.Counter(),
                                             'swap': collections.Counter()})
    total = {'n': 0, 'p': 0.0, 'd': 0.0}
    by = collections.defaultdict(lambda: [0, 0.0, 0.0])
    for path in args.jsonl:
        for line in open(path, encoding='utf-8'):
            r = json.loads(line)
            on, off = r['on'].split(), r['off'].split()
            i = next((k for k, (a, b) in enumerate(zip(on, off)) if a != b), min(len(on), len(off)))
            prefix = ' '.join(on[:i])
            seat = SEATS[(SEATS.index(r['dealer']) + i) % 4]
            side = 'ours' if seat in ('North', 'South') else 'theirs'
            swap = f"{off[i] if i < len(off) else '·'} → {on[i] if i < len(on) else '·'}"
            p, d = r['plain'], r['pd']
            s = nodes[prefix]
            s['n'] += 1; s['p'] += p; s['pp'] += p * p; s['d'] += d
            s['side'][side] += 1; s['swap'][swap] += 1
            total['n'] += 1; total['p'] += p; total['d'] += d
            b = by[(prefix in walked, side)]
            b[0] += 1; b[1] += p; b[2] += d

    def z(s):
        n = s['n']
        var = (s['pp'] - s['p'] ** 2 / n) / (n - 1) if n > 1 else 0.0
        return s['p'] / math.sqrt(var * n) if var > 0 else math.copysign(math.inf, s['p'])

    print(f"# Per-node join: {', '.join(args.jsonl)}\n")
    print(f"Book `{args.book}`, {len(walked):,} walked nodes.  {total['n']:,} contract-divergent "
          f"boards, Σ plain {total['p']:+,.0f}, Σ PD {total['d']:+,.0f} IMPs (ON − OFF), "
          f"{len(nodes):,} distinct first-divergence nodes.\n")
    print('| walked | first divergent call | boards | Σ plain | Σ PD |')
    print('| --- | --- | ---: | ---: | ---: |')
    for (w, side), (n, p, d) in sorted(by.items(), key=lambda kv: kv[1][1]):
        print(f"| {'yes' if w else 'no'} | {side} | {n:,} | {p:+,.0f} | {d:+,.0f} |")

    rows = [(k, s) for k, s in nodes.items() if s['n'] >= args.min_boards]
    for title, sign in [('ON wins (the authoring queue)', 1), ('ON loses', -1)]:
        print(f"\n## {title} — top {args.top} by Σ plain, nodes with ≥ {args.min_boards} boards\n")
        print('| node | walked | boards | Σ plain | /board | z | Σ PD | top swap (OFF → ON) |')
        print('| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |')
        side = [kv for kv in rows if sign * kv[1]['p'] > 0]
        for k, s in sorted(side, key=lambda kv: -sign * kv[1]['p'])[:args.top]:
            (swap, c), = s['swap'].most_common(1)
            ours = s['side']['ours'] / s['n']
            node = f"`{k}`" if k else '(opening seat, dealer)'
            note = '' if ours == 1 else f" ({ours:.0%} ours)"
            print(f"| {node}{note} | {'yes' if k in walked else 'no'} | {s['n']:,} | {s['p']:+,.0f} | "
                  f"{s['p'] / s['n']:+.2f} | {z(s):+.1f} | {s['d']:+,.0f} | {swap} ({c / s['n']:.0%}) |")


if __name__ == '__main__':
    main()
