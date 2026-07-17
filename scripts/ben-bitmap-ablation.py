#!/usr/bin/env python3
"""Ablate BEN's bidder policy net: does honor *location* matter beyond HCP+shape?

BEN's bidder input (model_version 3, n_cards_bidding 24) feeds total HCP and all
four suit lengths as explicit named scalars — and they are exact linear read-outs
of the 24-cell hand bitmap (get_hcp/get_shape). The only residual the bitmap
carries beyond (HCP, shape) is *which suit each honor sits in*. This probe holds
the auction fixed and swaps the true hand for canonical hands of **identical HCP
and identical four suit lengths**, honors reassigned, then measures how much the
policy softmax moves. Forward passes only; no /bid search (that is Tier-S DD
override, a different question). Mirrors botbidder.next_bid_np's v3 path exactly.

  scripts/idle-run.sh ~/ben/.venv/bin/python scripts/ben-bitmap-ablation.py \
      ab-results/ben-info-probe/2026-07-17/probe-nv.jsonl \
      -o ab-results/ben-info-probe/2026-07-17/bitmap-ablation.json --seed-base $(date +%s)

Self-check (no TF, no model): scripts/ben-bitmap-ablation.py --selftest
GPL boundary: runs inside BEN's checkout, imports its code; nothing linked into pons.
"""

import argparse
import configparser
import json
import math
import os
import random
import sys
from collections import defaultdict

THREADS = 16  # ponytail: courtesy cap while a calibration A/B shares the box
NESW = "NESW"
TOKEN = {"P": "PASS"}       # probe emits ben-gen's ctx dialect; bids/X/XX match
HONORS = "AKQJT"           # bitmap honor cells, high->low
HCP_VAL = {"A": 4, "K": 3, "Q": 2, "J": 1}
SPOTS = "98765432"         # 8 distinct non-honor ranks
RANKS = "AKQJT98765432"
EPS = 1e-9


def bid_number(auction):
    """BEN's get_bid_number_for_player_to_bid: n_steps for the player to bid."""
    i = len(auction) % 4           # start at the actor's seat, not 0
    while i < len(auction) and auction[i] == "PAD_START":
        i += 4
    return 1 + (len(auction) - i) // 4


def hand_stats(hand_str):
    suits = hand_str.split(".")
    lengths = [len(s) for s in suits]
    honors = [c for s in suits for c in s if c in HONORS]
    return lengths, honors


def pack(lengths, honors, longest_first):
    """Reassign the honor multiset across suits with lengths fixed, clustering
    honors toward long (longest_first) or short suits. Fill with spots. Returns a
    hand string, or None if greedy can't place legally (rare; caller logs+skips)."""
    order = sorted(range(4), key=lambda i: (-lengths[i] if longest_first else lengths[i], i))
    rank_of = {i: order.index(i) for i in range(4)}
    min_h = [max(0, L - len(SPOTS)) for L in lengths]  # long suits must hold honors
    suit_h = [[] for _ in range(4)]
    cnt = [0] * 4
    for r in sorted(honors, key=HONORS.index):          # high honors first
        cands = [i for i in range(4) if cnt[i] < lengths[i] and r not in suit_h[i]]
        if not cands:
            return None
        cands.sort(key=lambda i: (cnt[i] >= min_h[i], rank_of[i]))  # floors, then priority
        i = cands[0]
        suit_h[i].append(r)
        cnt[i] += 1
    parts = []
    for i in range(4):
        need = lengths[i] - len(suit_h[i])
        if need > len(SPOTS):
            return None
        hs = sorted(suit_h[i], key=HONORS.index)
        parts.append("".join(hs) + SPOTS[:need])
    return ".".join(parts)


def random_hand(seed):
    """A random legal 13-card hand (generically different HCP/shape): sensitivity
    normalizer. Deterministic in seed (seed hygiene)."""
    rng = random.Random(seed)
    cards = rng.sample(range(52), 13)
    suits = [[], [], [], []]  # S,H,D,C to match "S.H.D.C"
    for c in cards:
        suits[c // 13].append(RANKS[c % 13])
    return ".".join("".join(sorted(s, key=RANKS.index)) for s in suits)


def check_invariant(true_str, cand_str):
    """Canonical hand must match true HCP and per-suit lengths (the guard)."""
    tl, th = hand_stats(true_str)
    cl, ch = hand_stats(cand_str)
    thcp = sum(HCP_VAL.get(c, 0) for c in th)
    chcp = sum(HCP_VAL.get(c, 0) for c in ch)
    return tl == cl and thcp == chcp


def kl(p, q):
    """KL(p||q) in nats, both softmaxes; smoothed."""
    import numpy as np
    p = np.asarray(p, dtype=np.float64) + EPS
    q = np.asarray(q, dtype=np.float64) + EPS
    p /= p.sum()
    q /= q.sum()
    return float(np.sum(p * np.log(p / q)))


def tv(p, q):
    import numpy as np
    p = np.asarray(p, dtype=np.float64)
    q = np.asarray(q, dtype=np.float64)
    return 0.5 * float(np.sum(np.abs(p / p.sum() - q / q.sum())))


def last_strain(calls):
    """NT/major/minor/other of the last contract bid, for bucketing."""
    for c in reversed(calls):
        if len(c) == 2 and c[0] in "1234567":
            s = c[1]
            return {"N": "NT", "S": "major", "H": "major", "D": "minor", "C": "minor"}.get(s, "other")
    return "none"


def selftest():
    import numpy as np
    # Packer preserves HCP + shape, and long/short bracket concentration.
    hands = ["3.Q74.8742.AJ532", "KJ6.AK32.K93.Q64", "QT98754..T65.987",
             "AKQJT98765432...", "A.K.Q.JT98765432"]  # last: 10-card minor
    for h in hands:
        L, H = hand_stats(h)
        assert sum(L) == 13, (h, L)
        for lf in (True, False):
            c = pack(L, H, lf)
            assert c is not None, (h, lf)
            assert check_invariant(h, c), (h, lf, c)
            assert sum(len(s) for s in c.split(".")) == 13, (h, c)
    # KL is a proper divergence: zero iff identical, positive otherwise.
    p = np.array([0.7, 0.2, 0.1])
    q = np.array([0.2, 0.3, 0.5])
    assert abs(kl(p, p)) < 1e-9
    assert kl(p, q) > 0
    assert 0.0 <= tv(p, q) <= 1.0
    # A concentrated hand really does pack differently long vs short.
    L, H = hand_stats("AK2.QJ3.T98.7654")
    assert pack(L, H, True) != pack(L, H, False)
    print("selftest OK")


def pct(xs, p):
    import numpy as np
    return float(np.percentile(xs, p)) if xs else float("nan")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("jsonl", nargs="?", help="probe corpus (reused, read-only)")
    ap.add_argument("-o", "--output")
    ap.add_argument("--ben-home", default=os.path.expanduser("~/ben"))
    ap.add_argument("--conf", default="src/config/BEN-21GF.conf")
    ap.add_argument("--batch", type=int, default=1024)
    ap.add_argument("--seed-base", type=int, default=0)
    ap.add_argument("--limit", type=int, default=0, help="smoke: first N rows only")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()

    if args.selftest:
        selftest()
        return
    if not (args.jsonl and args.output):
        ap.error("jsonl and -o are required unless --selftest")

    os.environ.setdefault("CUDA_VISIBLE_DEVICES", "-1")
    os.environ.setdefault("TF_CPP_MIN_LOG_LEVEL", "3")
    os.environ.setdefault("OMP_NUM_THREADS", str(THREADS))
    sys.path.insert(0, os.path.join(args.ben_home, "src"))
    import tensorflow as tf
    tf.config.threading.set_intra_op_parallelism_threads(THREADS)
    tf.config.threading.set_inter_op_parallelism_threads(1)
    import numpy as np
    import binary
    from bidding import bidding
    from nn.bidder_tf2 import Bidder
    from types import SimpleNamespace

    conf = configparser.ConfigParser()
    conf.read(os.path.join(args.ben_home, args.conf))
    n_cards = conf.getint("models", "n_cards_bidding")
    bidder_path = os.path.join(args.ben_home, conf.get("bidding", "bidder"))
    models = SimpleNamespace(
        model_version=conf.getint("models", "model_version"),
        ns=conf.getint("models", "NS"),
        ew=conf.getint("models", "EW"),
        n_cards_bidding=n_cards,
        adjust_hcp=conf.getboolean("bidding", "adjust_hcp"),
        bidder_model=Bidder("bidder", bidder_path, False),
    )
    parse_hand = binary.parse_hand_f(n_cards)
    print(f"bidder: {bidder_path} (v{models.model_version}, n_cards {n_cards})", file=sys.stderr)

    rows = [json.loads(line) for line in open(args.jsonl)]
    if args.limit:
        rows = rows[: args.limit]

    # Build all (row, variant) hands, encode each into (1, n_steps, F), group by
    # n_steps so a group batches into one pred_fun_seq call (uniform T per batch).
    groups = defaultdict(list)  # n_steps -> [((row_i, kind), X)]
    skips = 0
    for i, row in enumerate(rows):
        dealer_i = NESW.index(row["dealer"])
        calls = [TOKEN.get(c, c) for c in row["auction"]]
        auction = ["PAD_START"] * dealer_i + calls
        assert len(auction) % 4 == NESW.index(row["seat"]), (i, row["seat"])
        hand_ix = len(auction) % 4
        vuln = [row["vul_ns"], row["vul_ew"]]
        n_steps = bid_number(auction)
        L, H = hand_stats(row["hand"])
        variants = {
            "true": row["hand"],
            "long": pack(L, H, True),
            "short": pack(L, H, False),
            "rand": random_hand(args.seed_base + i),
        }
        for kind, hs in variants.items():
            if hs is None or (kind in ("long", "short") and not check_invariant(row["hand"], hs)):
                skips += 1
                print(f"skip row {i} {kind}: pack fail", file=sys.stderr)
                continue
            hand = parse_hand(hs)
            X = binary.get_auction_binary(n_steps, auction, hand_ix, hand, vuln, models)
            groups[n_steps].append(((i, kind), X))

    dist = {}  # (row_i, kind) -> 40-vec
    done = 0
    for n_steps, members in sorted(groups.items()):
        for at in range(0, len(members), args.batch):
            chunk = members[at : at + args.batch]
            X = np.concatenate([x for _, x in chunk])
            bids, _ = models.bidder_model.pred_fun_seq(X)
            last = np.asarray(bids)[:, -1, :]  # (batch, 40) softmax
            for (key, _), d in zip(chunk, last):
                dist[key] = d.astype(np.float64)
            done += len(chunk)
            print(f"{done} forward passes", file=sys.stderr)

    # Per-row metrics.
    out_rows = []
    for i, row in enumerate(rows):
        dt = dist.get((i, "true"))
        if dt is None:
            continue
        rec = {
            "row": i,
            "hcp": sum(HCP_VAL.get(c, 0) for c in hand_stats(row["hand"])[1]),
            "auction_len": len(row["auction"]),
            "strain": last_strain([TOKEN.get(c, c) for c in row["auction"]]),
            "true_bid": bidding.ID2BID[int(np.argmax(dt))],
        }
        for kind in ("long", "short", "rand"):
            dk = dist.get((i, kind))
            if dk is None:
                continue
            rec[kind] = {
                "bid": bidding.ID2BID[int(np.argmax(dk))],
                "flip": int(np.argmax(dk) != np.argmax(dt)),
                "kl": round(kl(dt, dk), 5),
                "tv": round(tv(dt, dk), 5),
            }
        out_rows.append(rec)

    # Aggregate. KL_honor = worst-case relocation (max over long/short); ratio vs
    # rand = share of hand-sensitivity attributable to honor location.
    def has(r, k):
        return k in r and isinstance(r[k], dict)

    MATERIAL = 0.1  # nats: a flip with KL_honor above this is a real policy shift,
                    # below it is tie-breaking between near-equal top calls.
    kl_honor, ratio, kl_rand_all = [], [], []
    flips = 0
    flip_denom = 0            # rows where the hand demonstrably matters (KL_rand large)
    flips_all = 0
    material_flips = 0        # flip AND KL_honor > MATERIAL: honor decisively changed the call
    buckets = defaultdict(lambda: [0, 0])  # key -> [material flips, n]
    for r in out_rows:
        if not (has(r, "long") and has(r, "short") and has(r, "rand")):
            continue
        kh = max(r["long"]["kl"], r["short"]["kl"])
        kr = r["rand"]["kl"]
        kl_honor.append(kh)
        kl_rand_all.append(kr)
        fl = int(r["long"]["flip"] or r["short"]["flip"])
        mfl = int(fl and kh > MATERIAL)
        flips_all += fl
        material_flips += mfl
        for bk in (f"len{r['auction_len']}", f"hcp{min(r['hcp'] // 4 * 4, 20)}", r["strain"]):
            buckets[bk][0] += mfl
            buckets[bk][1] += 1
        if kr > 1e-3:         # hand matters at this prefix
            flip_denom += 1
            flips += fl
            ratio.append(kh / kr)

    n = len(kl_honor)
    summary = {
        "meta": {
            "seed_base": args.seed_base,
            "n_rows": len(rows),
            "n_scored": n,
            "n_skips": skips,
            "bidder": os.path.relpath(bidder_path, args.ben_home),
            "note": "raw policy net (pred_fun_seq), not /bid search; 24-card collapse means spot cards below T are invisible to the bitmap and not probed.",
        },
        "flip_rate_all": round(flips_all / n, 4) if n else None,
        "flip_rate_hand_matters": round(flips / flip_denom, 4) if flip_denom else None,
        "material_flip_rate": round(material_flips / n, 4) if n else None,
        "material_flip_threshold_nats": MATERIAL,
        "kl_honor": {"mean": round(sum(kl_honor) / n, 5) if n else None,
                     "p50": round(pct(kl_honor, 50), 5), "p90": round(pct(kl_honor, 90), 5),
                     "p95": round(pct(kl_honor, 95), 5)},
        "kl_rand": {"mean": round(sum(kl_rand_all) / n, 5) if n else None,
                    "p50": round(pct(kl_rand_all, 50), 5), "p95": round(pct(kl_rand_all, 95), 5)},
        "ratio_honor_over_rand": {"n": len(ratio), "median": round(pct(ratio, 50), 4),
                                  "p90": round(pct(ratio, 90), 4)},
        "flip_buckets": {k: {"flips": v[0], "n": v[1], "rate": round(v[0] / v[1], 4)}
                         for k, v in sorted(buckets.items())},
    }

    # Verdict (advisory). Separate frequency (argmax flips) from magnitude
    # (material flips + median ratio): most flips are near-tie tie-breaks.
    mfr = summary["material_flip_rate"] or 0
    med = summary["ratio_honor_over_rand"]["median"] or 0
    p90r = summary["ratio_honor_over_rand"]["p90"] or 0
    if mfr < 0.02 and p90r < 0.10:
        verdict = ("(HCP, shape) ~ sufficient statistic for BEN's bidder: honor "
                   "location decisively changes the call in <2% of hands and is a "
                   "sub-10% slice of hand-sensitivity even at p90. The gap is "
                   "search/auction-state, not a missing hand feature.")
    elif med < 0.05:
        verdict = ("Honor location is mostly irrelevant to BEN's bidder (median "
                   f"{med:.1%} of hand-sensitivity) but decisive in a thin tail "
                   f"(material-flip rate {mfr:.1%}, p90 ratio {p90r:.1%}) — see "
                   "buckets/top rows for where. A suit-quality term would help those "
                   "auctions specifically, not the floor globally.")
    else:
        verdict = ("Honor location is load-bearing for BEN's bidder: relocating "
                   "honors at fixed HCP+shape shifts the call materially and often. A "
                   "floor knowing only HCP+shape is structurally blind; add an "
                   "honor-concentration / suit-quality term to disclosable features.")
    summary["verdict"] = verdict

    top = sorted((r for r in out_rows if has(r, "long") and has(r, "short")),
                 key=lambda r: -max(r["long"]["kl"], r["short"]["kl"]))[:20]

    with open(args.output, "w") as f:
        json.dump({"summary": summary, "top_kl_honor": top, "rows": out_rows}, f, indent=1)

    print(json.dumps(summary, indent=1), file=sys.stderr)
    print(f"\nVERDICT: {verdict}", file=sys.stderr)


if __name__ == "__main__":
    main()
