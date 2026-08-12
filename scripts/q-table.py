"""q(level, vul) = P(doubled | we declare) — the competitive accountant's doubling model.

    python3 scripts/q-table.py <off-dir> [<off-dir> ...] [--fired <on-dir> ...]

Each `<off-dir>` is an arm dump directory of `shard-*.json` (e.g.
`ab-results/two-level-minor-overcall-refresh/off-none`).  Unlike
`scripts/ab-classify.py` this is a **single-arm, all-boards** pass: no pairing,
no fired filter, and it reads **both** tables.  bba-gen seats our system N/S at
table A and E/W at table B (`bid_out(..., conv_is_ns, ...)`,
`examples/common/mod.rs:122`), so table B needs the seat-parity flip — that flip
is the only reason this is not a two-line change to ab-classify.py.

Counting every board rather than only the fired ones is deliberate: the gate
reads q at the 4- and 5-level, and the fired population of a single knob's
refutation has too few doubled 5+ contracts to clear the n >= 200 rule.  Pass
`--fired` with the matching ON arms (paired positionally) to get the fired-only
slice back as a sensitivity column; a table is "fired" when that same table's
auction differs between the arms.

Vulnerability is read from the shard's top-level `vulnerability` field ('' =
none, 'NS | EW' = both) and resolved to **our** side's vulnerability per table,
so the rows key on `vul_we` the way `break_even` does.

Wilson 95% intervals; counts at the 5-level are small even here.
"""
import collections
import glob
import json
import math
import os
import sys

SEATS = ["North", "East", "South", "West"]
MIN_N = 200  # no cell ships below this; thin cells inherit the level-pooled rate


def final(auction, dealer):
    """(level, strain, dbl, declarer_seat_index) or None for a passout.

    Copied from scripts/ab-classify.py, which owns the published population
    tables and stays untouched.
    """
    calls = auction.split()
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
        lvl, st = int(c[0]), c[1:]
        last, dbl = (lvl, st, seat), 0
        strains.setdefault((seat % 2, st), seat)  # first of that side to name it
    if last is None:
        return None
    lvl, st, seat = last
    return lvl, st, dbl, strains[(seat % 2, st)]


def gate_reached(auction, dealer, ours_even):
    """Did one of our seats ever face the accountant's trigger in this auction?

    The trigger, mirroring `their_live_bid_at_most` inverted (and
    `gate_node` in examples/eval-columns): the last live call is *their*
    undoubled bid at level >= 4, and our side has already named a strain.  This
    is the population the gate actually acts on — neither "every board" (which
    dilutes with uncontested slam tries that land in 5m) nor "the boards a
    retired knob happened to move".
    """
    calls = auction.split()
    d0 = SEATS.index(dealer)
    for length in range(len(calls)):
        if ((d0 + length) % 4 % 2 == 0) != ours_even:
            continue  # not our seat to act
        live = [i for i in range(length) if calls[i] != "-"]
        if not live:
            continue
        index = live[-1]
        if (length - index) % 2 != 1:
            continue  # the live call is ours, not theirs
        call = calls[index]
        if call in ("X", "XX") or int(call[0]) < 4:
            continue
        if any(
            calls[i] not in ("-", "X", "XX") and (length - i) % 2 == 0 for i in range(length)
        ):
            return True
    return False


def wilson(k, n, z=1.96):
    """Wilson score interval for k successes in n trials."""
    if n == 0:
        return 0.0, 0.0
    p = k / n
    d = 1 + z * z / n
    centre = (p + z * z / (2 * n)) / d
    half = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d
    return max(0.0, centre - half), min(1.0, centre + half)


def bucket(level):
    return "1-2" if level <= 2 else ("5+" if level >= 5 else str(level))


def shards(directory):
    for path in sorted(
        glob.glob(os.path.join(directory, "shard-*.json")),
        key=lambda p: int(p.split("-")[-1].split(".")[0]),
    ):
        yield json.load(open(path))


def tally(off_dir, on_dir, counts, gate_counts, fired_counts):
    """Fold one arm directory into the three slices: all, gate-reached, fired."""
    on_shards = shards(on_dir) if on_dir else None
    for dump in shards(off_dir):
        vul = dump["vulnerability"]
        on_boards = next(on_shards)["boards"] if on_shards else None
        for index, board in enumerate(dump["boards"]):
            dealer = board["dealer"]
            # Table A seats us N/S (declarer index 0/2), table B seats us E/W.
            for table, ours_even, side in (("table_a", True, "NS"), ("table_b", False, "EW")):
                auction = board[table]
                contract = final(auction, dealer)
                if contract is None:
                    continue
                level, _, dbl, declarer = contract
                if (declarer % 2 == 0) != ours_even:
                    continue  # they declare; q prices only contracts we buy
                key = (bucket(level), side in vul)
                doubled = dbl >= 1
                counts[key][0] += 1
                counts[key][1] += doubled
                if gate_reached(auction, dealer, ours_even):
                    gate_counts[key][0] += 1
                    gate_counts[key][1] += doubled
                if on_boards is not None and on_boards[index][table] != auction:
                    fired_counts[key][0] += 1
                    fired_counts[key][1] += doubled


def render(title, counts):
    print(f"\n## {title}\n")
    print("| level | vul_we | n declared | n doubled | q | 95% CI |")
    print("| ---: | --- | ---: | ---: | ---: | --- |")
    pooled = collections.defaultdict(lambda: [0, 0])
    for (level, vul), (n, k) in counts.items():
        pooled[level][0] += n
        pooled[level][1] += k
    for vul in (False, True):
        for level in ("1-2", "3", "4", "5+"):
            n, k = counts[(level, vul)]
            if n == 0:
                continue
            lo, hi = wilson(k, n)
            thin = ""
            if n < MIN_N:
                pn, pk = pooled[level]
                thin = f"  *(thin; pooled {pk / pn:.3f} over n={pn})*" if pn else "  *(thin)*"
            print(
                f"| {level} | {'both' if vul else 'none'} | {n} | {k} | "
                f"{k / n:.3f} | {lo:.3f}–{hi:.3f}{thin} |"
            )


def main():
    argv = sys.argv[1:]
    if "--fired" in argv:
        cut = argv.index("--fired")
        off_dirs, on_dirs = argv[:cut], argv[cut + 1 :]
    else:
        off_dirs, on_dirs = argv, []
    if not off_dirs:
        sys.exit(__doc__)
    if on_dirs and len(on_dirs) != len(off_dirs):
        sys.exit("--fired takes one ON dir per OFF dir, paired positionally")

    counts = collections.defaultdict(lambda: [0, 0])
    gate = collections.defaultdict(lambda: [0, 0])
    fired = collections.defaultdict(lambda: [0, 0])
    for index, off_dir in enumerate(off_dirs):
        tally(off_dir, on_dirs[index] if on_dirs else None, counts, gate, fired)

    total_n = sum(n for n, _ in counts.values())
    total_k = sum(k for _, k in counts.values())
    print(f"contracts we declare {total_n}  doubled {total_k} ({100 * total_k / total_n:.2f}%)")
    render("q — all boards, both tables", counts)
    render("q — auctions that passed through the gate's trigger (the shipping slice)", gate)
    if on_dirs:
        render("q — fired tables only (sensitivity)", fired)


main()
