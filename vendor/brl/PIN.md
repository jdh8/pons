# brl — pin

[harukaki/brl](https://github.com/harukaki/brl) — Kita et al., *A Simple,
Solid, and Reproducible Baseline for Bridge Bidding AI* (IEEE CoG 2024).
Apache-2.0 (compatible; weights may be vendored/embedded if ever needed —
no GPL wall, unlike BEN).

| What | Value |
| --- | --- |
| Upstream commit | `fdd958ff8adcb44844d70ea2e233bed18d5a7b70` (2024-06-03) |
| Model | `bridge_models/model-pretrained-rl-with-fsp.pkl` (the README's +1.24 IMPs/bd vs WBridge5 entry) |
| Weights sha256 | `63bff43eeb685d8c6b402ad3116199a531cfb939a249d83f6107da675cea16a2` |
| Architecture | "DeepMind": 4 × Linear(1024) + ReLU → policy head (38) + value head (dm-haiku) |
| Observation | pgx `bridge_bidding` 480-dim: vul(4) ‖ history incl. pre-passes(424) ‖ own hand(52); **no DD info at inference** |
| Action space | Pass=0, X=1, XX=2, bids 3..37 (1C..7NT, strain order C<D<H<S<NT) — identical to pons `array.rs` |
| Eval protocol | greedy: `distrax.Categorical(logits + min·~legal_mask).mode()` (`src/evaluation.py`) — the dumper reproduces this |
| Runtime pins | `pgx==1.4.0`, `jax==0.4.23`, `jaxlib==0.4.23`, `dm-haiku==0.0.11`, python 3.10 |

## Local setup (out of repo, like `~/ben`)

```sh
git clone https://github.com/harukaki/brl ~/brl && cd ~/brl
git checkout fdd958ff8adcb44844d70ea2e233bed18d5a7b70
uv venv --python 3.10 .venv
uv pip install --python .venv/bin/python 'jax==0.4.23' 'jaxlib==0.4.23' \
  'dm-haiku==0.0.11' 'distrax==0.1.5' 'pgx==1.4.0' 'numpy==1.26.3'
```

## Corpus dumper

`dump_selfplay.py` (this directory) — batched greedy self-play of brl×4,
one JSONL line per board, for the book-extraction probe
(`examples/probe-brl-book`, report in `docs/brl-book-extraction.md`).

Design notes:

- **No DDS dataset needed.** pgx's `env.init` draws deals *from* its DD
  lookup table (capped, with replacement) and randomizes vul/dealer — wrong
  for a paired-vulnerability corpus. The dumper constructs `State` directly:
  own uniform deals (numpy seeded permutations), dealer fixed North (the
  observation is seat-relative), all four vul combos per deal. pgx's own
  `_observe`/`_step` do the bidding mechanics; the DD table argument gets a
  dummy (it is only consulted for terminal *rewards*, which we never read).
- **Startup self-checks**: weights sha256, and the pure-function round-trip
  `_key_to_hand(_state_to_key(state)) == state._hand` guarding the hand
  layout. Downstream, the Rust ingest asserts legality-replay and argmax
  consistency.
- **Replay validation (2026-07-17)**: 40 pilot boards (~500 decisions)
  rebuilt from the dumped PBN strings via pgx's own `_pbn_to_key` logic
  (inlined with numpy — upstream mutates an immutable jnp array) reproduce
  every dumped call exactly. This is the canonical obs-scramble guard. It
  matters because brl's system violates human priors — e.g. its dealer
  opening rate is **anti-monotone in HCP** (99% of 0–3 counts open; ~47% of
  12–15 counts pass), so "looks wrong" is not evidence of a wiring bug here.

```sh
cd ~/brl && nice -n 19 .venv/bin/python \
  /path/to/pons/vendor/brl/dump_selfplay.py \
  --deals 200000 --seed <fresh> --out corpus/brl-selfplay.jsonl
```
