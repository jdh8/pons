# The configured net — one net that reads the convention card

**Status: design, no code.** Supersedes the two-artifact twin scheme
(`american_bba.f32` + `american_bba_kickback.f32`) that
[`bba-kickback.md`](bba-kickback.md) §7.7 introduced and §7.12 showed the cost
of.

## Why

`set_kickback` drives rule presence *and* the floor's weights, so every cell it
measures prices the package. That is not a harness defect to be tightened —
it is structural, because a kickback-blind net bids a natural 4♥ into the
auction where the ladder has claimed 4♥ as the diamond ask. One knob has to
drive both, and so the convention and the net can never be separated by an
arm.

§7.12 put numbers on what that costs. **93.5% of divergent boards saw no
keycard ask from either side** — those cannot be the relocation, and they carry
89% of the PD gain. Worse, the ♠ lane, where the relocation *provably* cannot
move a call (♠ has nowhere to relocate to; 4NT already is its ask), still reads
−1.939 PD/board over 722 boards. The measured quantity is dominated by
something that is not the convention.

Two nets also differ by **vintage**, not only by regime. `american_bba.f32` was
trained 2026-07-25 and the twin 2026-08-02, with **68 commits to `src/bidding`
between them, 58 of them `feat`/`fix`** — the queen relay, the merged reply,
the answer gates, the reading repairs. The twin is fitted to a week-newer
system as well as to kickback.

The fix is not a better control arm. It is to put the configuration **in the
features**, so one net serves every regime and an arm differs by a config row
instead of by an artifact.

## Why the earlier mixed net failed, and why this is different

`dump-teacher --mix-kickback` already built a single net on both regimes (866k
rows, alternating by board). It was the better net by aggregate val CE — 0.4004
against the twin's 0.4431 — **and it still bid the phantom 4♥**.

The diagnosis is in `neural.rs`: the regime was not in the features *at the
moment the call is chosen*. Readings describe the auction so far, and
`1♦ P 1♠ P 2♦` is three natural bids in either system, so both regimes present
that decision with **byte-identical inputs and contradictory targets** — 2♥
from the kickback teacher, 4♥ from the plain one. The net could only average
them. The ranges do diverge, but one ply too late: only once a relocated ask
has been made, which is the decision we needed it to get right.

Adding the config bit is exactly what makes those two rows distinguishable. The
mixed-net result is therefore evidence *for* this design, not against it: the
corpus and the tooling already exist, and the one thing that broke it is the
thing being added.

(Val CEs across the three nets are **not** directly comparable — each was
measured on its own validation corpus. Cited for provenance, not as a ranking.)

## What the config is

**The convention card, both pairs.** `card.rs` already generates the card from
live knob state, so there is no new vocabulary to invent and no recurring
"which knobs count?" argument — and the features stay disclosable by
construction, because the feature vector *is* the disclosure.

| block | width | source |
| --- | ---: | --- |
| `features_v3` | 88 | unchanged |
| our card | 135 | `SCHEMA` (133) + `PONS_SCHEMA` (2) |
| their card | 135 | same encoding, opponents' knob state |
| **`FEATURES_LEN_V4`** | **358** | |

`PONS_SCHEMA` is the two conventions EPBot's schema has no name for (South
African Texas, Queen ask by available bid). "King ask by available bid" is a
real BBA row, not one of ours.

**Both pairs, because a mixed table is the normal case in an A/B.** The two
arms play each other, so at every table one side relocates its asks and the
other does not. Today the net cannot see that at all — it reads identical
features whether or not the opponents play kickback. A net that cannot
represent the mixed table is out of distribution on exactly the boards the
measurement is about.

**Full width, not the varying rows only.** Most rows are constant within any
one corpus and will train to ≈0. Pruning to the varying rows would make the
artifact's meaning depend on the corpus that produced it, and would re-open the
width question every campaign. The cost is real and worth stating: input width
**88 → 358**, roughly 4× on layer 1.

Reserving spare slots was considered and rejected: a net learns weight ≈0 for
an always-zero input, so populating a reserved slot later still needs a
retrain. Reserving saves plumbing churn, not the expensive part.

## Corpus and evaluation

- **Corpus deals: the banks at `/nfs2/jdh8/pons/`** (`24.pdd`, `22.pdd`).
  Double-dummy tables come pre-solved, so no solver runs. `dump-teacher
  --deals` already routes through `pons::pdd::load`, which accepts `.pdd`
  binary and GIB text alike — but it reads whole files, and `24.pdd` is 2 GB.
  Wire `pdd::load_slice` behind `--skip`, the pattern
  `probe-trick-variance` already uses. **Keep `24/22.pdd` byte-stable.**
- **Target: unchanged** — EPBot's chosen call. This is still distillation, not
  a value net.
- **Evaluation: freshly generated deals**, never bank rows. Both gates below
  run on a fresh `SEED_BASE`, so no held-out row was ever trained on.

## Acceptance — two gates

| gate | arms | question |
| --- | --- | --- |
| **1. Quality** | v4 net `kickback=off` vs shipped `WEIGHTS_BBA`, **both playing the plain system** | Is the configured net a better bidder at fixed rules? |
| **2. Convention** | ±kickback **within the v4 net** | What is the relocation worth, alone? |

Gate 1 holds the rules fixed and varies only the net, so it also finally prices
how much of the current +0.0705 PD/board was a two-week-newer net rather than a
convention. It subsumes the placebo arm (two same-system nets, different data
seeds) that would otherwise be needed to calibrate net-to-net variance.

Gate 2 is the fair comparison this whole campaign has lacked: **same weights**,
arms differing by one config row and the rules it gates. The confound is gone
by construction rather than corrected for.

Both scored on plain DD **and** perfect-defense, read off `docs/measurement.md`'s
decision table, arms sequential, fresh seed shared across arms.

## Sequence

| phase | work | note |
| --- | --- | --- |
| 0 | **Fix `epbot_set_conventions` per side** | **Prerequisite.** It takes a side (0/1), but seats 2/3 throw −2 and it is swallowed; `oracle.rs` works by a parity accident. Without this the teacher is silently symmetric and asymmetric rows are a lie |
| 1 | `features_v4` + config plumbing | the open design question below |
| 2 | `dump-teacher`: `--skip`/`load_slice`, per-board config sampling, per-side teacher config, both config vectors per row | |
| 3 | Train on GPU | off-crate, as always |
| 4 | Artifact + candle-parity fixture, wire `classify_bba` to v4, retire the twin selection | the twin artifact can go once gate 1 passes |
| 5 | Gate 1, then gate 2 | fresh deals |

## Open: how the opponents' config reaches the features

Our own card falls out of live knob state, exactly as `card()` reads it today.
The opponents' does not — nothing at a seat currently knows what the other
partnership has agreed.

Recommendation: **a thread-local holding the opponents' card**, set by the
harness beside the knobs it already arms per call. It matches the machinery
that exists (knobs are thread-locals; `card()` reads them live), needs no
signature change through `Classifier`/`Stance`, and the A/B loop is already
re-arming per call by side, so there is one obvious place to set it.

The alternative — carrying both cards on `Context` — is cleaner in principle
and touches every constructor in the bidding path. Not obviously worth it
before the design has proven itself.

`ponytail:` a thread-local is the cheap rung. If a second consumer ever needs
the opponents' agreements (a reader that adapts to their system, say), promote
it to `Context` then.
