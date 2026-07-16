#!/usr/bin/env python3
"""Greedy self-play corpus dumper for brl (see PIN.md in this directory).

Reproduces brl's own evaluation protocol (eval.py / src/evaluation.py):
argmax over legal-masked logits — deterministic given (hand, auction, vul).

Bypasses pgx's `env.init` (which draws deals from its DD lookup table with
replacement and randomizes vul/dealer): constructs `State` directly from
seeded-uniform numpy deals, dealer fixed North, all four vul combos per deal.
pgx's own `_observe`/`_step` provide the bidding mechanics; the DD table
argument is a dummy — it is only consulted for terminal rewards, never read.

Output: one JSON line per board
  {"deal": int, "vul": "none|ns|ew|both", "pbn": "N:S.H.D.C E S W",
   "calls": [token..], "top3": [[[token, prob]x3]..], "ent": [nats..]}
plus a `<out>.sidecar.json` pinning weights hash, versions, seed, protocol.

Usage (from the brl checkout, so `src.models` resolves):
  cd ~/brl && nice -n 19 .venv/bin/python dump_selfplay.py \
      --deals 200000 --seed 1752700000 --out corpus/brl-selfplay.jsonl
"""

import argparse
import hashlib
import json
import sys
import time
from pathlib import Path

import numpy as np

WEIGHTS_SHA256 = "63bff43eeb685d8c6b402ad3116199a531cfb939a249d83f6107da675cea16a2"
TOKENS = ["P", "X", "XX"] + [f"{lv}{st}" for lv in range(1, 8) for st in ("C", "D", "H", "S", "NT")]
VULS = [("none", 0, 0), ("ns", 1, 0), ("ew", 0, 1), ("both", 1, 1)]
RANKS = "A23456789TJQK"  # card = suit*13 + rank; suit order S,H,D,C (pgx PBN parse)
TMAX = 96  # greedy auctions end far below this; the loop asserts termination


def pbn(deal):
    """deal: (4,13) card ints, seats NESW -> 'N:S.H.D.C E S W' (ranks descending)."""
    power = lambda r: 13 if r == 0 else r  # A > K(12) > ... > 2(1)
    hands = []
    for seat in range(4):
        suits = [[], [], [], []]
        for c in sorted(deal[seat], key=lambda c: -power(c % 13)):
            suits[c // 13].append(RANKS[c % 13])
        hands.append(".".join("".join(s) for s in suits))
    return "N:" + " ".join(hands)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--brl-root", default=str(Path.home() / "brl"))
    ap.add_argument("--model", default="bridge_models/model-pretrained-rl-with-fsp.pkl")
    ap.add_argument("--deals", type=int, required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--batch-deals", type=int, default=2048, help="deals per jit batch (boards = 4x)")
    args = ap.parse_args()

    sys.path.insert(0, args.brl_root)
    import jax
    import jax.numpy as jnp
    import pickle
    from src.models import make_forward_pass
    from pgx.bridge_bidding import State, _key_to_hand, _observe, _state_to_key, _step

    model_path = Path(args.brl_root) / args.model
    digest = hashlib.sha256(model_path.read_bytes()).hexdigest()
    assert digest == WEIGHTS_SHA256, f"weights hash mismatch: {digest}"
    params = pickle.load(open(model_path, "rb"))
    fwd = make_forward_pass(activation="relu", model_type="DeepMind")

    def make_one(hand52, vul_ns, vul_ew):
        legal = jnp.ones(38, dtype=jnp.bool_).at[1].set(False).at[2].set(False)
        return State(
            _shuffled_players=jnp.arange(4, dtype=jnp.int8),
            current_player=jnp.int8(0),
            _hand=hand52,
            _dealer=jnp.int8(0),
            _vul_NS=vul_ns,
            _vul_EW=vul_ew,
            legal_action_mask=legal,
        )

    make_batch = jax.vmap(make_one)
    dkeys = jnp.zeros((1, 4), dtype=jnp.int32)
    dvals = jnp.zeros((1, 4), dtype=jnp.int32)
    neg = jnp.finfo(jnp.float32).min

    @jax.jit
    def run_batch(state):
        B = state.current_player.shape[0]

        def cond(carry):
            state, t = carry[0], carry[1]
            return (~state.terminated.all()) & (t < TMAX)

        def body(carry):
            state, t, acts, t3i, t3p, ents = carry
            obs = jax.vmap(_observe)(state, state.current_player).astype(jnp.float32)
            logits, _ = fwd.apply(params, obs)
            masked = jnp.where(state.legal_action_mask, logits, neg)
            act = jnp.argmax(masked, axis=1).astype(jnp.int32)
            p = jax.nn.softmax(masked, axis=1)
            ent = -jnp.sum(jnp.where(p > 0, p * jnp.log(p), 0.0), axis=1)
            tp, ti = jax.lax.top_k(p, 3)
            live = ~state.terminated
            acts = acts.at[t].set(jnp.where(live, act, -1))
            t3i = t3i.at[t].set(jnp.where(live[:, None], ti, -1))
            t3p = t3p.at[t].set(jnp.where(live[:, None], tp, 0.0))
            ents = ents.at[t].set(jnp.where(live, ent, 0.0))
            stepped = jax.vmap(_step, in_axes=(0, 0, None, None))(state, act, dkeys, dvals)
            state = jax.tree_util.tree_map(
                lambda n, o: jnp.where(live.reshape((-1,) + (1,) * (n.ndim - 1)), n, o),
                stepped,
                state,
            )
            return state, t + 1, acts, t3i, t3p, ents

        carry = (
            state,
            jnp.int32(0),
            jnp.full((TMAX, B), -1, jnp.int32),
            jnp.full((TMAX, B, 3), -1, jnp.int32),
            jnp.zeros((TMAX, B, 3), jnp.float32),
            jnp.zeros((TMAX, B), jnp.float32),
        )
        state, _, acts, t3i, t3p, ents = jax.lax.while_loop(cond, body, carry)
        return state.terminated.all(), acts, t3i, t3p, ents

    rng = np.random.default_rng(args.seed)
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    written = 0
    t0 = time.time()
    with open(out, "w") as f:
        for base in range(0, args.deals, args.batch_deals):
            n = min(args.batch_deals, args.deals - base)
            deals = np.stack([rng.permutation(52).reshape(4, 13) for _ in range(n)])
            deals.sort(axis=2)
            hands = jnp.asarray(np.repeat(deals.reshape(n, 52), 4, axis=0), dtype=jnp.int32)
            vul_ns = jnp.asarray(np.tile([v[1] for v in VULS], n), dtype=jnp.bool_)
            vul_ew = jnp.asarray(np.tile([v[2] for v in VULS], n), dtype=jnp.bool_)
            state = make_batch(hands, vul_ns, vul_ew)
            if base == 0:  # hand-layout round trip guards the state surgery
                rt = jax.vmap(lambda s: _key_to_hand(_state_to_key(s)))(state)
                assert bool((rt == state._hand).all()), "hand layout round-trip failed"
            done, acts, t3i, t3p, ents = jax.device_get(run_batch(state))
            assert bool(done), "batch hit TMAX without terminating"
            acts, t3i, t3p, ents = acts.T, t3i.swapaxes(0, 1), t3p.swapaxes(0, 1), ents.T
            for b in range(4 * n):
                deal_id, (vul, _, _) = base + b // 4, VULS[b % 4]
                length = int((acts[b] >= 0).sum())
                row = {
                    "deal": deal_id,
                    "vul": vul,
                    "pbn": pbn(deals[b // 4]),
                    "calls": [TOKENS[a] for a in acts[b, :length]],
                    "top3": [
                        [[TOKENS[i], round(float(p), 4)] for i, p in zip(t3i[b, t], t3p[b, t])]
                        for t in range(length)
                    ],
                    "ent": [round(float(e), 3) for e in ents[b, :length]],
                }
                f.write(json.dumps(row, separators=(",", ":")) + "\n")
                written += 1
            rate = written / (time.time() - t0)
            print(f"{written} boards ({rate:.0f}/s)", file=sys.stderr, flush=True)

    sidecar = {
        "weights": str(model_path),
        "weights_sha256": digest,
        "brl_commit": "fdd958ff8adcb44844d70ea2e233bed18d5a7b70",
        "pgx": "1.4.0",
        "jax": jax.__version__,
        "seed": args.seed,
        "deals": args.deals,
        "boards": written,
        "vuls": [v[0] for v in VULS],
        "dealer": "N (fixed; observation is seat-relative)",
        "protocol": "greedy argmax over legal-masked logits (brl eval.py); softmax masked-to-legal",
        "deal_source": "numpy default_rng(seed) permutations (uniform, unlimited)",
        "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "created": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
    }
    Path(str(out) + ".sidecar.json").write_text(json.dumps(sidecar, indent=2) + "\n")
    print(f"done: {written} boards -> {out}", file=sys.stderr)


if __name__ == "__main__":
    main()
