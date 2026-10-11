"""The O4-vul meta check on the accountant pool (docs/ai-bidder/competitive-accountant.md,
"Session D arm 2"): where the pool's PD-minus-plain gap comes from, at what level it
sits, how often each reference doubles our failing contracts at the table, and what
every table double was worth against the same contract undoubled.

    python3 -I scripts/accountant-pool-meta.py <anchor-dir> [rows.jsonl] [vul-dir-prefix]

    # BEN Tier-S at c72dd03e
    python3 -I scripts/accountant-pool-meta.py ab-results/ben-anchor/2026-10-11-c72dd03e
    # BBA at 7838d8c2, the shipped `american` arms
    python3 -I scripts/accountant-pool-meta.py ab-results/anchor/2026-10-10-7838d8c2 \
        ab-results/anchor/2026-10-10-7838d8c2/boards-american.jsonl american-

Reads <anchor-dir>/<prefix>{none,both}/shard-*.json, the DD cache (the anchor's, or
its series dir's) and the decompose rows.  Pool = Defensive / round-1 / book rows whose
our_call is a two-level suit overcall and their_call is P.  Pure JSON, no DD solve.
Scoring is re-derived here and asserted against every row's swing_plain / swing_pd,
so the scorer is proven on every board it reports.  ~5 s on the 409.6k-board anchor.
"""
import collections, json, os, sys

SEATS = ["North", "East", "South", "West"]
STRAIN = {"♣": 0, "♦": 1, "♥": 2, "♠": 3, "NT": 4}
IMP_BOUNDS = [20, 50, 90, 130, 170, 220, 270, 320, 370, 430, 500, 600, 750, 900, 1100,
              1300, 1500, 1750, 2000, 2250, 2500, 3000, 3500, 4000]


def imps(d):
    n = sum(1 for b in IMP_BOUNDS if abs(d) >= b)
    return n if d >= 0 else -n


def score(level, strain, dbl, tricks, vul):
    """Duplicate score for declarer (positive = made)."""
    need = 6 + level
    if tricks < need:
        down = need - tricks
        if dbl == 0:
            return -down * (100 if vul else 50)
        if vul:
            pts = 200 + 300 * (down - 1)
        else:
            pts = 100 + 200 * min(down - 1, 2) + 300 * max(down - 3, 0)
        return -pts * dbl  # dbl: 1 = X, 2 = XX
    per = 20 if strain in (0, 1) else 30
    trick = per * level + (10 if strain == 4 else 0)
    trick *= (1, 2, 4)[dbl]
    bonus = (500 if vul else 300) if trick >= 100 else 50
    if level == 6:
        bonus += 750 if vul else 500
    if level == 7:
        bonus += 1500 if vul else 1000
    over = tricks - need
    if dbl == 0:
        over_pts = over * per
    else:
        over_pts = over * (200 if vul else 100) * dbl
    return trick + bonus + over_pts + (50 * dbl if dbl else 0)


def final(calls, dealer):
    """(level, strain, dbl, declarer_seat) or None for a passout."""
    d0 = SEATS.index(dealer)
    last, dbl, strains = None, 0, {}
    for i, c in enumerate(calls):
        seat = (d0 + i) % 4
        if c == "-":
            continue
        if c == "X":
            dbl = 1
            continue
        if c == "XX":
            dbl = 2
            continue
        lvl, st = int(c[0]), STRAIN[c[1:]]
        last, dbl = (lvl, st, seat), 0
        strains.setdefault((seat % 2, st), seat)
    if last is None:
        return None
    lvl, st, seat = last
    return lvl, st, dbl, strains[(seat % 2, st)]


def tricks(table, strain, seat):
    return (table[strain] >> (4 * seat)) & 0xF


def ns_scores(contract, table, vul):
    """(plain, pd) signed for NS."""
    if contract is None:
        return 0, 0
    lvl, st, dbl, dec = contract
    t = tricks(table, st, dec)
    vul_dec = vul == "both"  # anchors run none/both only
    fails = t < 6 + lvl
    plain = score(lvl, st, dbl, t, vul_dec)
    pd = score(lvl, st, 1 if fails else 0, t, vul_dec)
    sign = 1 if dec % 2 == 0 else -1
    return sign * plain, sign * pd


def load(anchor, prefix=""):
    shards = {}
    for vul in ("none", "both"):
        d = os.path.join(anchor, prefix + vul)
        for f in sorted(os.listdir(d)):
            if f.startswith("shard-") and f.endswith(".json"):
                j = json.load(open(os.path.join(d, f)))
                shards[(vul, j["seed"])] = j["boards"]
    cp = os.path.join(anchor, "dd-cache.json")
    if not os.path.exists(cp):
        cp = os.path.join(os.path.dirname(anchor.rstrip("/")), "dd-cache.json")
    cache = json.load(open(cp))
    dd = {k.strip('"'): v for k, v in cache.items()}
    return shards, dd


def lvl_bucket(l):
    return "2-3" if l <= 3 else ("4" if l == 4 else "5+")


def main():
    anchor = sys.argv[1]
    rows_path = sys.argv[2] if len(sys.argv) > 2 else os.path.join(anchor, "boards.jsonl")
    prefix = sys.argv[3] if len(sys.argv) > 3 else ""
    shards, dd = load(anchor, prefix)
    rows = [json.loads(l) for l in open(rows_path)]
    pool = [r for r in rows if r["phase"] == "Defensive" and r["family"] == "round-1"
            and r["provenance"] == "book" and r["their_call"] == "P"
            and len(r["our_call"]) == 2 and r["our_call"][0] == "2" and r["our_call"][1] in "♣♦♥♠"]
    print(f"rows {len(rows)}  pool {len(pool)}")

    # --- global baseline: how often is a DD-failing contract doubled at the table, by level and declaring side
    # Over every divergent row (both tables), "us" = our seat at that table.
    base = collections.Counter()
    for r in rows:
        b = shards[(r["vul"], r["seed"])][r["board"]]
        table = dd[b["deal"]]
        for tbl, ours_ns in (("table_a", True), ("table_b", False)):
            c = final(b[tbl].split(), b["dealer"])
            if c is None:
                continue
            lvl, st, dbl, dec = c
            we_declare = (dec % 2 == 0) == ours_ns
            fails = tricks(table, st, dec) < 6 + lvl
            if not fails:
                continue
            key = ("we" if we_declare else "they", lvl_bucket(lvl))
            base[key + ("n",)] += 1
            base[key + ("X",)] += 1 if dbl else 0
    print("\n== baseline over all divergent rows, both tables: P(doubled at table | contract fails DD) ==")
    print("declarer  level   n     doubled   rate")
    for side in ("we", "they"):
        for lb in ("2-3", "4", "5+"):
            n, x = base[(side, lb, "n")], base[(side, lb, "X")]
            print(f"{side:8s}  {lb:5s}  {n:5d}  {x:7d}   {100*x/max(n,1):5.1f}%")

    # --- pool
    tot = collections.Counter()
    gap_by = collections.Counter()
    gap_n = collections.Counter()
    pool_lvl = collections.Counter()
    worst = []
    ov_fail = collections.Counter()
    plain_by = collections.Counter()
    for r in pool:
        b = shards[(r["vul"], r["seed"])][r["board"]]
        table = dd[b["deal"]]
        ca = final(b["table_a"].split(), b["dealer"])
        cb = final(b["table_b"].split(), b["dealer"])
        pa, qa = ns_scores(ca, table, r["vul"])
        pb, qb = ns_scores(cb, table, r["vul"])
        plain, pd = imps(pa - pb), imps(qa - qb)
        assert (plain, pd) == (r["swing_plain"], r["swing_pd"]), (r, plain, pd)
        tot[(r["vul"], "n")] += 1
        tot[(r["vul"], "plain")] += plain
        tot[(r["vul"], "pd")] += pd

        seat = (SEATS.index(b["dealer"]) + r["div_index"]) % 4
        ours_at_a = seat % 2 == 0
        ov, ref = ("table_a", "table_b") if ours_at_a else ("table_b", "table_a")
        # per-table rescoring classification, from our side's point of view at that table
        def classify(tbl, ours_ns):
            c = final(b[tbl].split(), b["dealer"])
            if c is None:
                return None
            lvl, st, dbl, dec = c
            we_declare = (dec % 2 == 0) == ours_ns
            t = tricks(table, st, dec)
            fails = t < 6 + lvl
            if fails and dbl == 0:
                kind = "add-X"
            elif not fails and dbl:
                kind = "strip-X"
            else:
                return ("none", we_declare, lvl, 6 + lvl - t, dbl)
            return (kind, we_declare, lvl, 6 + lvl - t, dbl)

        ka = classify(ov, ours_at_a)
        kb = classify(ref, not ours_at_a)
        gap = pd - plain
        resc = []
        if ka and ka[0] != "none":
            resc.append(("ov",) + ka[:3])
        if kb and kb[0] != "none":
            resc.append(("ref",) + kb[:3])
        if len(resc) == 1:
            k = resc[0]
            key = (k[0], k[1], "we" if k[2] else "they", lvl_bucket(k[3]))
            gap_by[key] += gap
            gap_n[key] += 1
        elif len(resc) == 2:
            gap_by[("both",)] += gap
            gap_n[("both",)] += 1
        else:
            gap_by[("neither",)] += gap
            gap_n[("neither",)] += 1

        # pool loss by the overcall table's final level
        if ka:
            lb = lvl_bucket(ka[2])
            pool_lvl[(lb, "n")] += 1
            pool_lvl[(lb, "plain")] += plain
            pool_lvl[(lb, "pd")] += pd
            # our failing contracts at the overcall table: doubled at the table or not, by level and undertricks
            if ka[1] and ka[3] > 0:
                ov_fail[(lb, min(ka[3], 3), "n")] += 1
                ov_fail[(lb, min(ka[3], 3), "X")] += 1 if ka[4] else 0
                ov_fail[(lb, min(ka[3], 3), "gap")] += gap
        if ka:
            key = ("we" if ka[1] else "they", lvl_bucket(ka[2]), "down" if ka[3] > 0 else "made", "X" if ka[4] else "-")
        else:
            key = ("passout",)
        plain_by[key + ("n",)] += 1
        plain_by[key + ("plain",)] += plain
        plain_by[key + ("pd",)] += pd
        worst.append((pd, plain, r["vul"], b["dealer"], b[ov], b[ref], ov, ka, kb))

    print("\n== pool totals ==")
    for vul in ("none", "both"):
        print(f"{vul:5s} n {tot[(vul,'n')]:4d}  plain {tot[(vul,'plain')]:5d}  pd {tot[(vul,'pd')]:5d}")

    print("\n== pool loss by final level at the overcall table (ov) ==")
    for lb in ("2-3", "4", "5+"):
        print(f"level {lb:4s} n {pool_lvl[(lb,'n')]:4d}  plain {pool_lvl[(lb,'plain')]:5d}  pd {pool_lvl[(lb,'pd')]:5d}")

    print("\n== pool swings by the overcall-table contract: declarer, level, result, doubled at table ==")
    keys = sorted({k[:-1] for k in plain_by})
    print("declarer level result X    n   plain    pd")
    for k in sorted(keys, key=lambda k: plain_by[k + ("plain",)]):
        lab = " ".join(f"{x:5s}" for x in k)
        print(f"{lab:26s} {plain_by[k+('n',)]:4d} {plain_by[k+('plain',)]:6d} {plain_by[k+('pd',)]:6d}")
    print("\n== PD-minus-plain gap (IMPs) attributed to the one rescored contract ==")
    print("table  kind     declarer level   n    gap")
    for key in sorted(gap_by, key=lambda k: gap_by[k]):
        if len(key) == 1:
            print(f"{key[0]:34s} {gap_n[key]:4d} {gap_by[key]:6d}")
        else:
            print(f"{key[0]:5s}  {key[1]:8s} {key[2]:8s} {key[3]:5s} {gap_n[key]:4d} {gap_by[key]:6d}")

    print("\n== ov table, we declare and fail DD: doubled at the table?  (gap = PD-plain IMPs on those boards) ==")
    print("level  down  n   doubled  rate   gap")
    for lb in ("2-3", "4", "5+"):
        for dn in (1, 2, 3):
            n, x, g = ov_fail[(lb, dn, "n")], ov_fail[(lb, dn, "X")], ov_fail[(lb, dn, "gap")]
            if n:
                print(f"{lb:5s}  {dn}{'+' if dn == 3 else ' '}   {n:4d} {x:5d}   {100*x/n:5.1f}%  {g:5d}")

    worst.sort()
    print("\n== 15 worst pool boards by PD swing ==")
    for pd, plain, vul, dealer, ov_auc, ref_auc, ovt, ka, kb in worst[:15]:
        print(f"pd {pd:4d} plain {plain:4d} {vul:4s} dealer {dealer:5s} ov={ovt}\n   ov : {ov_auc}\n       {ka}\n   ref: {ref_auc}\n       {kb}")


main()


def extra():
    """(1) per-vul CI of the pool on each scorer; (2) marginal table value of every
    double on divergent boards, by who doubled, level and result — the counterfactual
    is the same contract undoubled (the gate's own leaf approximation)."""
    import math
    anchor = sys.argv[1]
    rows_path = sys.argv[2] if len(sys.argv) > 2 else os.path.join(anchor, "boards.jsonl")
    prefix = sys.argv[3] if len(sys.argv) > 3 else ""
    shards, dd = load(anchor, prefix)
    rows = [json.loads(l) for l in open(rows_path)]
    per_vul = {v: sum(len(b) for (vv, s), b in shards.items() if vv == v) for v in ("none", "both")}
    pool = [r for r in rows if r["phase"] == "Defensive" and r["family"] == "round-1"
            and r["provenance"] == "book" and r["their_call"] == "P"
            and len(r["our_call"]) == 2 and r["our_call"][0] == "2" and r["our_call"][1] in "♣♦♥♠"]
    print("\n== pool per-board CI (zeros on every other board of the arm) ==")
    for vul in ("none", "both"):
        N = per_vul[vul]
        for sc in ("swing_plain", "swing_pd"):
            xs = [r[sc] for r in pool if r["vul"] == vul]
            s, ss = sum(xs), sum(x * x for x in xs)
            mean = s / N
            var = ss / N - mean * mean
            print(f"{vul:5s} {sc:11s} n {len(xs):5d}  sum {s:6d}  {mean:+.4f} ±{1.96*math.sqrt(var/N):.4f} IMPs/board")

    marg = collections.Counter()
    for r in rows:
        b = shards[(r["vul"], r["seed"])][r["board"]]
        table = dd[b["deal"]]
        ca = final(b["table_a"].split(), b["dealer"])
        cb = final(b["table_b"].split(), b["dealer"])
        pa, _ = ns_scores(ca, table, r["vul"])
        pb, _ = ns_scores(cb, table, r["vul"])
        diff = pa - pb
        for tbl, c, ours_ns, sign in (("a", ca, True, 1), ("b", cb, False, -1)):
            if c is None or c[2] == 0:
                continue
            lvl, st, dbl, dec = c
            undoubled, _ = ns_scores((lvl, st, 0, dec), table, r["vul"])
            p_with = pa if tbl == "a" else pb
            diff_without = diff - sign * (p_with - undoubled)
            delta = imps(diff) - imps(diff_without)  # IMPs the double earned our team
            we_declare = (dec % 2 == 0) == ours_ns
            fails = tricks(table, st, dec) < 6 + lvl
            key = ("they-X-us" if we_declare else "we-X-them", lvl_bucket(lvl), "down" if fails else "made")
            marg[key + ("n",)] += 1
            marg[key + ("imps",)] += delta
    print("\n== marginal IMPs earned by each double at the table (vs the same contract undoubled), divergent boards ==")
    print("who          level result    n    IMPs   /board")
    for who in ("we-X-them", "they-X-us"):
        for lb in ("2-3", "4", "5+"):
            tot_n = tot_i = 0
            for res in ("made", "down"):
                n, i = marg[(who, lb, res, "n")], marg[(who, lb, res, "imps")]
                tot_n += n; tot_i += i
                print(f"{who:12s} {lb:5s} {res:6s} {n:5d} {i:7d}")
            print(f"{who:12s} {lb:5s} {'all':6s} {tot_n:5d} {tot_i:7d}   {tot_i/sum(per_vul.values()):+.4f}")


extra()
