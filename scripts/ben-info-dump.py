#!/usr/bin/env python3
"""Annotate probe-ben-info jsonl with BEN's Info-net predictions.

For each row (actor hand, dealer, vul, auction prefix), run one forward pass
of BEN's binfo net and append `ben`: predicted HCP and 4-suit shape of the
three hidden hands, [lho, partner, rho] of the actor, suits in ascending
C,D,H,S order (matching Inference.lengths; BEN's native order is S,H,D,C —
reversed here).

Run with BEN's venv, ideally idle-class:
  scripts/idle-run.sh ~/ben/.venv/bin/python scripts/ben-info-dump.py \
      probe.jsonl -o probe-ben.jsonl

Recipe per lorserker/ben v0.8.8.4 src/sample.py:501-517 (get_bidding_info);
model + n_cards read from BEN's own config so the pin follows the campaign's
version discipline. GPL boundary: this script runs inside BEN's checkout and
imports its code; nothing is linked into pons.
"""

import argparse
import configparser
import json
import os
import sys
from collections import defaultdict

# ponytail: 16 threads — transient courtesy cap while a calibration A/B
# shares the box; harmless when the box is free.
THREADS = 16
os.environ.setdefault("CUDA_VISIBLE_DEVICES", "-1")
os.environ.setdefault("TF_CPP_MIN_LOG_LEVEL", "3")
os.environ.setdefault("OMP_NUM_THREADS", str(THREADS))

NESW = "NESW"
TOKEN = {"P": "PASS"}  # probe emits ben-gen's ctx dialect; bids/X/XX match


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("jsonl", help="probe-ben-info output")
    ap.add_argument("-o", "--output", required=True)
    ap.add_argument("--ben-home", default=os.path.expanduser("~/ben"))
    ap.add_argument("--conf", default="src/config/BEN-21GF.conf")
    ap.add_argument("--batch", type=int, default=1024)
    args = ap.parse_args()

    sys.path.insert(0, os.path.join(args.ben_home, "src"))
    import tensorflow as tf

    tf.config.threading.set_intra_op_parallelism_threads(THREADS)
    tf.config.threading.set_inter_op_parallelism_threads(1)
    import numpy as np
    import binary
    from nn.bid_info_tf2 import BidInfo
    from types import SimpleNamespace

    conf = configparser.ConfigParser()
    conf.read(os.path.join(args.ben_home, args.conf))
    n_cards = conf.getint("models", "n_cards_bidding")
    info_path = os.path.join(args.ben_home, conf.get("bidding", "info"))
    models = SimpleNamespace(
        model_version=conf.getint("models", "model_version"),
        ns=conf.getint("models", "NS"),
        ew=conf.getint("models", "EW"),
        n_cards_bidding=n_cards,
        binfo_model=BidInfo(info_path),
    )
    parse_hand = binary.parse_hand_f(n_cards)
    print(f"binfo: {info_path} (n_cards {n_cards})", file=sys.stderr)

    rows = [json.loads(line) for line in open(args.jsonl)]

    # Encode every row, grouped by n_steps so each group batches into one
    # pred_fun call (the tf.function needs uniform T within a batch).
    groups = defaultdict(list)  # n_steps -> [(row index, encoded (1,T,F))]
    for i, row in enumerate(rows):
        dealer_i = NESW.index(row["dealer"])
        calls = [TOKEN.get(c, c) for c in row["auction"]]
        auction = ["PAD_START"] * dealer_i + calls
        assert len(auction) % 4 == NESW.index(row["seat"]), (i, row["seat"])
        hand = parse_hand(row["hand"])
        vuln = [row["vul_ns"], row["vul_ew"]]
        n_steps = binary.calculate_step_bidding_info(auction)
        A = binary.get_auction_binary_sampling(
            n_steps, auction, len(auction) % 4, hand, vuln, models, n_cards
        )
        groups[n_steps].append((i, A))

    done = 0
    for n_steps, members in sorted(groups.items()):
        for at in range(0, len(members), args.batch):
            chunk = members[at : at + args.batch]
            A = np.concatenate([a for _, a in chunk])
            p_hcp, p_shp = models.binfo_model.pred_fun(A)
            hcp = 4 * np.asarray(p_hcp).reshape((-1, n_steps, 3))[:, -1, :] + 10
            shp = 1.75 * np.asarray(p_shp).reshape((-1, n_steps, 12))[:, -1, :] + 3.25
            for (i, _), h3, s12 in zip(chunk, hcp, shp):
                s34 = s12.reshape((3, 4))  # [lho,pard,rho] x [S,H,D,C]
                rows[i]["ben"] = {
                    seat: {
                        "hcp": round(float(h3[k]), 2),
                        "shape": [round(float(x), 2) for x in s34[k][::-1]],
                    }
                    for k, seat in enumerate(("lho", "partner", "rho"))
                }
            done += len(chunk)
            print(f"{done}/{len(rows)} rows", file=sys.stderr)

    with open(args.output, "w") as out:
        for row in rows:
            out.write(json.dumps(row) + "\n")


if __name__ == "__main__":
    main()
