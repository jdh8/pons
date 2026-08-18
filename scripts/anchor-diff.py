#!/usr/bin/env python3
"""Join two anchor snapshots' `boards.jsonl` and split each bucket's move into
churn vs same-board regression (docs/bba-gap-campaign.md, rule 5; the worked
example is docs/archive/bba-gap-floor-forensic.md).

    python3 scripts/anchor-diff.py A_SNAP B_SNAP
    python3 scripts/anchor-diff.py A_SNAP B_SNAP \
        --bucket 'Competitive / floor#46 / round-2' --lane here --show 10

A re-anchor's "this bucket got worse" is not yet a regression: bucket membership
moves whenever an auction changes, so a worse row may simply be different boards.
Keyed on `(vul, seed, board)` — the anchor seed series deals the same boards
forever — every bucket's delta decomposes as

    Δ(X) = Σ_stayed (swing_B − swing_A) + Σ_entered swing_B − Σ_left swing_A

per scorer.  Δ carried by *entered/left* is churn: an **earlier** call changed
and the boards moved buckets, so the attribution belongs where they came from.

`stayed` splits again, and getting this right needs the two-table geometry.  A
board is bid twice: at `table_a` our pair sits N/S, at `table_b` it sits E/W
(`bid_out(ours, opponent, conv_is_ns)`), and `div_index` is the first index where
the two auctions differ.  The row's `our_call` is therefore **whichever table our
pair held that seat at** — `table_a` when the diverging seat is N/S, `table_b`
when it is E/W — so a `floor#N` row's rule fired at only one of the two tables.
`stayed-here` (that table's auction moved) is the only slice the bucket's rule
can be blamed for; `stayed-other` (its own table byte-identical, the other one
moved) is our bidding elsewhere on the board landing in a bucket it does not own.
That third confound sits beside the two the campaign doc names (churn, small n).

`--bucket` then joins the shard dumps to print the worst boards with both
auctions, the first index where *our* call differs, and a paste-ready
`probe-decision` line (`PROBE_FLOOR=instinct`, since `floor#N` names an instinct
rule).  `--lane here` keeps only the boards the bucket owns.

To name the mechanism, replay that line against a build of the older commit and
bisect the reading defaults the window flipped on the newer one:
`PROBE_SCOPE=alerted PROBE_CEILINGS=0 PROBE_BID_EXCLUSION=0
PROBE_FORCING_CEILING=0 PROBE_UPGRADE_CLOSURE=0`.  A knob subset that restores
the old call is reading drift; a read that comes back while the call does not is
something else in the floor (the window that first used this had also swapped the
hand evaluator, which no knob reaches).
"""

import argparse
import collections
import json
import os
import sys

# ponytail: buckets key on the report's own (phase, provenance, family) rather
# than joining on rule text — floor rule *descriptions* are not unique (111 texts
# carry >1 label; every opaque instinct rule renders alike), so a text join
# silently merges distinct rules.  Renumbering is caught instead by
# `label_drift()` below, which is the property the text join was reaching for.
def bucket(row):
    return (row["phase"], row["provenance"], row["family"])


# Dealer-relative seat sides: our pair is N/S at table A and E/W at table B, and
# the two sides alternate down the auction, so the side acting at `div_index`
# says which table the row's `our_call` was made at.
SIDE = {"North": 0, "South": 0, "East": 1, "West": 1}
SEATS = ("North", "East", "South", "West")


def hand_at(board, index):
    """The hand of the seat acting at `index`.  A continuation probe is often
    *partner's* seat, not the one the bucket row records."""
    # "N:<north> <east> <south> <west>" — always dealt from North.
    hands = board["deal"].split(":", 1)[1].split()
    seat = (SEATS.index(board["dealer"]) + index) % 4
    return hands[seat]


def our_table(board, div_index):
    """'table_a' or 'table_b' — where this row's rule actually fired."""
    ns = (SIDE[board["dealer"]] + div_index) % 2 == 0
    return "table_a" if ns else "table_b"


def name(key):
    return " / ".join(key)


def load(snap):
    rows = {}
    with open(os.path.join(snap, "boards.jsonl"), encoding="utf-8") as f:
        for line in f:
            r = json.loads(line)
            rows[(r["vul"], r["seed"], r["board"])] = r
    return rows


def label_drift(a, b):
    """Labels whose rule text moved between snapshots — a `floor#N` join is only
    valid while the numbering is stable, and this is the whole check."""
    def texts(rows):
        m = {}
        for r in rows.values():
            m.setdefault(r["provenance"], set()).add(r["rule"])
        return m

    ta, tb = texts(a), texts(b)
    return {
        lab: (sorted(ta[lab]), sorted(tb[lab]))
        for lab in sorted(set(ta) & set(tb))
        # `book`/`fallback@d` legitimately carry one text per node; only the
        # instinct labels are 1:1 with a rule and can therefore drift.
        if lab.startswith("floor#") and ta[lab] != tb[lab]
    }


def deltas(a, b, ia=None, ib=None):
    """Per bucket: counts, per-scorer totals, and the stayed/entered/left split.

    With both shard indices, `stayed` splits into `here` (our `table_a` moved)
    and `other` (only `table_b` moved — our defense at the other table).
    """
    stat = collections.defaultdict(
        lambda: {
            "n_a": 0, "n_b": 0,
            "a_plain": 0, "a_pd": 0, "b_plain": 0, "b_pd": 0,
            "stayed": [0, 0], "here": [0, 0], "other": [0, 0],
            "entered": [0, 0], "left": [0, 0],
            "n_stayed": 0, "n_here": 0, "n_other": 0, "n_entered": 0, "n_left": 0,
            "from": collections.Counter(), "to": collections.Counter(),
        }
    )
    for ra in a.values():
        s = stat[bucket(ra)]
        s["n_a"] += 1
        s["a_plain"] += ra["swing_plain"]
        s["a_pd"] += ra["swing_pd"]
    for rb in b.values():
        s = stat[bucket(rb)]
        s["n_b"] += 1
        s["b_plain"] += rb["swing_plain"]
        s["b_pd"] += rb["swing_pd"]
    for k, rb in b.items():
        kb, ra = bucket(rb), a.get(k)
        s = stat[kb]
        if ra is not None and bucket(ra) == kb:
            dpl = rb["swing_plain"] - ra["swing_plain"]
            dpd = rb["swing_pd"] - ra["swing_pd"]
            s["n_stayed"] += 1
            s["stayed"][0] += dpl
            s["stayed"][1] += dpd
            # Only boards that actually moved get a lane: a stayed board whose
            # swing is unchanged says nothing about either table.
            if ia is not None and ib is not None and (dpl or dpd):
                vul, seed, board = k
                xa, xb = ia[(vul, seed)][board], ib[(vul, seed)][board]
                t = our_table(xb, rb["div_index"])
                lane = "other" if xa[t] == xb[t] else "here"
                s[f"n_{lane}"] += 1
                s[lane][0] += dpl
                s[lane][1] += dpd
        else:
            s["n_entered"] += 1
            s["entered"][0] += rb["swing_plain"]
            s["entered"][1] += rb["swing_pd"]
            s["from"][name(bucket(ra)) if ra else "(not divergent in A)"] += 1
    for k, ra in a.items():
        ka, rb = bucket(ra), b.get(k)
        if rb is not None and bucket(rb) == ka:
            continue
        s = stat[ka]
        s["n_left"] += 1
        s["left"][0] -= ra["swing_plain"]
        s["left"][1] -= ra["swing_pd"]
        s["to"][name(bucket(rb)) if rb else "(not divergent in B)"] += 1
    return stat


def per_div(s, scorer):
    """(A/div, B/div, Δ/div) — the report ranks on /div and so does the doc."""
    i = "plain" if scorer == 0 else "pd"
    a = s[f"a_{i}"] / s["n_a"] if s["n_a"] else 0.0
    b = s[f"b_{i}"] / s["n_b"] if s["n_b"] else 0.0
    return a, b, b - a


def summary(stat, args):
    print(f"{len(stat)} buckets; showing those worse on >=1 scorer per divergent "
          f"board with n>={args.min_n} in both snapshots\n")
    rows = []
    for k, s in stat.items():
        if s["n_a"] < args.min_n or s["n_b"] < args.min_n:
            continue
        dpl, dpd = per_div(s, 0)[2], per_div(s, 1)[2]
        if dpl < 0 or dpd < 0:
            rows.append((min(dpl, dpd), k, s, dpl, dpd))
    rows.sort()
    print(f"{'bucket':42s} {'n_A':>6s} {'n_B':>6s} | "
          f"{'plain/div':>18s} {'Δ':>6s} | {'pd/div':>18s} {'Δ':>6s} | "
          "Δ plain (here/other/ent/left)      Δ pd (here/other/ent/left)")
    for _, k, s, dpl, dpd in rows:
        pa, pb, _ = per_div(s, 0)
        da, db, _ = per_div(s, 1)
        print(
            f"{name(k):42s} {s['n_a']:6d} {s['n_b']:6d} | "
            f"{pa:+8.2f} -> {pb:+6.2f} {dpl:+6.2f} | "
            f"{da:+8.2f} -> {db:+6.2f} {dpd:+6.2f} | "
            f"{s['b_plain'] - s['a_plain']:+7d} "
            f"({s['here'][0]:+d}/{s['other'][0]:+d}/{s['entered'][0]:+d}/{s['left'][0]:+d})   "
            f"{s['b_pd'] - s['a_pd']:+7d} "
            f"({s['here'][1]:+d}/{s['other'][1]:+d}/{s['entered'][1]:+d}/{s['left'][1]:+d})"
        )
        print(f"{'':42s} {s['n_stayed']:6d} stayed, of which {s['n_here']} moved "
              f"here and {s['n_other']} only at the other table; "
              f"{s['n_entered']} entered, {s['n_left']} left")
        for tag, ctr in (("  entered from", s["from"]), ("  left to     ", s["to"])):
            if ctr:
                top = ", ".join(f"{n}x {b}" for b, n in ctr.most_common(3))
                print(f"{tag:42s} {top}")
    print(f"\n{len(rows)} regressed buckets.")
    return rows


def shard_index(snap):
    """(vul, seed) -> parsed shard.  ponytail: loads every shard of the arms it
    is asked for; ~1 s and a few hundred MB, against a positional-index scheme
    that would silently mis-join if a shard were ever regenerated."""
    idx = {}
    for vul in ("none", "both"):
        d = os.path.join(snap, vul)
        if not os.path.isdir(d):
            continue
        for fn in os.listdir(d):
            if not fn.endswith(".json"):
                continue
            with open(os.path.join(d, fn), encoding="utf-8") as f:
                dump = json.load(f)
            idx[(vul, dump["seed"])] = dump["boards"]
    return idx


def first_our_diff(ta, tb, div_index):
    """First index where the two auctions differ on a call *by our seat*.

    Seats alternate, so `div_index` fixes our parity: the bucket's own call sits
    at `div_index` and every index of the same parity is ours.  Returns
    (index, ours) or (None, _) when the auctions agree on our calls.
    """
    ca, cb = ta.split(), tb.split()
    for i in range(max(len(ca), len(cb))):
        x = ca[i] if i < len(ca) else None
        y = cb[i] if i < len(cb) else None
        if x != y:
            return i, (i - div_index) % 2 == 0
    return None, False


def detail(a, b, stat, args, ia, ib):
    for want in args.bucket:
        key = tuple(x.strip() for x in want.split("/"))
        if key not in stat:
            sys.exit(f"no such bucket: {want}")
        s = stat[key]
        print(f"\n=== {name(key)} ===")
        pa, pb, dpl = per_div(s, 0)
        da, db, dpd = per_div(s, 1)
        print(f"n {s['n_a']} -> {s['n_b']}  plain/div {pa:+.2f} -> {pb:+.2f} ({dpl:+.2f})"
              f"  pd/div {da:+.2f} -> {db:+.2f} ({dpd:+.2f})")
        for i, tag in ((0, "plain"), (1, "pd   ")):
            tot = s["b_plain" if i == 0 else "b_pd"] - s["a_plain" if i == 0 else "a_pd"]
            print(f"Δ {tag} {tot:+5d} = stayed {s['stayed'][i]:+d} "
                  f"(here {s['here'][i]:+d}, other-table {s['other'][i]:+d})"
                  f" + entered {s['entered'][i]:+d} + left {s['left'][i]:+d}")
        print(f"   boards: {s['n_stayed']} stayed, of which {s['n_here']} moved here "
              f"and {s['n_other']} only at the other table; "
              f"{s['n_entered']} entered, {s['n_left']} left")

        # Score every board this bucket touches by its contribution to Δ.
        cand = []
        for k, rb in b.items():
            if bucket(rb) != key:
                continue
            ra = a.get(k)
            if ra is not None and bucket(ra) == key:
                dpl = rb["swing_plain"] - ra["swing_plain"]
                dpd = rb["swing_pd"] - ra["swing_pd"]
                if args.lane != "any":
                    vul, seed, board = k
                    xa, xb = ia[(vul, seed)][board], ib[(vul, seed)][board]
                    t = our_table(xb, rb["div_index"])
                    if not (dpl or dpd) or (xa[t] == xb[t]) != (args.lane == "other"):
                        continue
                cand.append((dpl, dpd, "stayed", k, ra, rb))
            elif args.lane == "any":
                cand.append((rb["swing_plain"], rb["swing_pd"], "entered", k, ra, rb))
        for k, ra in a.items():
            if bucket(ra) != key or args.lane != "any":
                continue
            rb = b.get(k)
            if rb is not None and bucket(rb) == key:
                continue
            cand.append((-ra["swing_plain"], -ra["swing_pd"], "left", k, ra, rb))
        cand.sort(key=lambda c: c[0] + c[1])

        for dpl_b, dpd_b, kind, k, ra, rb in cand[: args.show]:
            vul, seed, board = k
            ba = ia.get((vul, seed), [{}])[board] if (vul, seed) in ia else {}
            bb = ib.get((vul, seed), [{}])[board] if (vul, seed) in ib else {}
            row = rb or ra
            seats = bb or ba
            t = our_table(seats, row["div_index"])
            other = "table_b" if t == "table_a" else "table_a"
            ta, tb = ba.get(t), bb.get(t)
            print(f"\n-- {kind}  Δplain {dpl_b:+d} Δpd {dpd_b:+d}  vul={vul} "
                  f"seed={seed} board={board}")
            print(f"   hand {row['hand']}  dealer {seats.get('dealer')}"
                  f"  div_index {row['div_index']}  our {row['our_call']} "
                  f"vs their {row['their_call']}  (we bid this in {t})")
            for tag, r, mine, theirs in (("A", ra, ta, ba.get(other)),
                                         ("B", rb, tb, bb.get(other))):
                where = name(bucket(r)) if r else "(not divergent)"
                swing = f"  swing {r['swing_plain']}/{r['swing_pd']}" if r else ""
                print(f"   {tag} {where}{swing}\n     ours  {mine}\n     other {theirs}")
            if not (ta and tb):
                continue
            probes = []
            if ta == tb and ba.get(other) != bb.get(other):
                # The table this rule bid at is byte-identical; the swing moved
                # at the other one, so this bucket's rule is innocent.
                print(f"   our {t} IDENTICAL — the swing moved at {other}, on a "
                      "board this bucket does not own")
                continue
            if ta == tb:
                # Same auction, same call, different rule: the decision did not
                # move, its *attribution* did — a reading change let a different
                # ladder rung answer first.  Nothing downstream to trace.
                print("   auctions IDENTICAL — provenance moved, not the call")
                probes.append(row["div_index"])
            else:
                i, ours = first_our_diff(ta, tb, row["div_index"])
                if i is None:
                    print("   our calls agree; only their calls moved")
                else:
                    rel = ("at the bucket call" if i == row["div_index"]
                           else f"{'before' if i < row['div_index'] else 'after'} the "
                                f"bucket call (div_index {row['div_index']})")
                    print(f"   first differing call: index {i} "
                          f"({'ours' if ours else 'theirs'}), {rel}: "
                          f"A={ta.split()[i] if i < len(ta.split()) else '(end)'} "
                          f"B={tb.split()[i] if i < len(tb.split()) else '(end)'}")
                    if ours:
                        probes.append(i)
                probes.append(row["div_index"])
            for j in dict.fromkeys(probes):
                prefix = " ".join(tb.split()[:j])
                tail = " # the bucket call" if j == row["div_index"] else ""
                print(f'   PROBE_FLOOR=instinct cargo run --release --example '
                      f'probe-decision -- "{hand_at(seats, j)}" "{prefix}" '
                      f'{vul}{tail}')


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("a_snap", help="older anchor snapshot dir")
    ap.add_argument("b_snap", help="newer anchor snapshot dir")
    ap.add_argument("--bucket", action="append", default=[],
                    help="'Phase / provenance / family' to detail (repeatable)")
    ap.add_argument("--lane", choices=("any", "here", "other"), default="any",
                    help="restrict --bucket boards to the stayed lane that moved "
                         "at this table ('here' — the only slice this rule owns) "
                         "or only at the other one ('other')")
    ap.add_argument("--show", type=int, default=10,
                    help="worst boards to print per --bucket (default 10)")
    ap.add_argument("--min-n", type=int, default=300,
                    help="minimum divergent boards in both snapshots (default 300; "
                         "the threshold the 53a3c254 re-anchor's '35 of 487' used)")
    args = ap.parse_args()

    a, b = load(args.a_snap), load(args.b_snap)
    drift = label_drift(a, b)
    if drift:
        print("WARNING: floor labels renumbered between snapshots — buckets are "
              "NOT comparable for these:")
        for lab, (xa, xb) in drift.items():
            print(f"  {lab}: A {xa} != B {xb}")
        print()
    ia, ib = shard_index(args.a_snap), shard_index(args.b_snap)
    stat = deltas(a, b, ia, ib)
    summary(stat, args)
    if args.bucket:
        detail(a, b, stat, args, ia, ib)


if __name__ == "__main__":
    main()
