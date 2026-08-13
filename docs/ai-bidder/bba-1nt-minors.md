# EPBot's 1NT minor scheme — the probe behind `european.rs`

**What this is for.** `src/bidding/american/notrump/european.rs` is not a system
we play. Puppet is our default and beats European head-to-head
([bidding-options.md](../bidding-options.md), `set_notrump_minors`). European
exists so we can *recognize* a continental opponent — BEN's declared card and
every vendored BBA card carry exactly its row pattern
(`vendor/ben/BEN-21GF.bbsa:9-13`: `1N-2S transfer to clubs = 1`,
`1N-3C transfer to diamonds = 1`, `1N-3C Puppet Stayman = 0`).

So its acceptance test is **fidelity to what EPBot actually plays, not IMPs**.
An opponent model that bids *better* than the opponent is a worse model. That
inverts the usual rule: no A/B gates a change here, and a soundness argument
does not outrank the probe.

**Why the vendored cards are not the evidence.** The FFI ignores `.bbsa` files
entirely — `examples/probe-bba-1nt/main.rs:5-8` records the strace showing the
`.so` opens no data file. Cards drive `BBA.exe`; the engine we link is
configured through `set_conventions`. They can and do disagree: `1N-3M splinter`
is `1` in `21GF.bbsa` and `0` in the engine. Only the live probe counts.

## Repro

`--conv` **replaces** the default convention set
(`probe-bba-constraints/main.rs`), so the whole family must be pinned or the
scheme silently reverts:

```bash
CONV=(--conv "1N-3C transfer to diamonds"=1 --conv "1N-3C Puppet Stayman"=0
      --conv "1N-2S transfer to clubs"=1  --conv "1N-2N transfer to diamonds"=0
      --conv "Multi-Landy"=1              --conv "Cappelletti"=0)

# 1a — the response classes
cargo run --release --example probe-bba-constraints -- \
  --mode nt-resp --samples 40000 --seed 7 --trim 0.0 --min-share 0.002 "${CONV[@]}"

# 1b — opener's answer to the diamond transfer
cargo run --release --example probe-bba-constraints -- \
  --mode nt-3c --samples 60000 --seed 7 --trim 0.0 --min-share 0.002 "${CONV[@]}"

# 1c — responder's rebid over the completion
cargo run --release --example probe-bba-constraints -- \
  --mode nt-3c-3d --samples 400000 --seed 7 --trim 0.0 --min-share 0.002 "${CONV[@]}"
```

`--trim 0.0` reports **hard min/max**, not percentiles — that is the whole point
for a class question, and it is what the `1N-3M splinter` study used. Raw output
in `ab-results/european-minors/`. The `nt-3c` and `nt-3c-3d` modes were added by
this study; `nt-3c-3d` filters on hands EPBot actually bids `3♣` with (enriched
probing — accept on the raw hand *before* the studied node), which is why 400k
samples yield 10,260 at the node.

## 1a — the response classes (`nt-resp`, n=40000, vul none)

| call | share | HCP (hard) | the tell |
| --- | --- | --- | --- |
| `2♣` | 32.1% | 6–26 | Stayman |
| Pass | 13.1% | 0–9 | — |
| `3NT` | 12.8% | 6–15 | longest minor 4–6 |
| `2♥` | 12.4% | 0–24 | ♠ **5–6** |
| `2♦` | 12.3% | 0–25 | ♥ **5–6** |
| `4♥` | 3.5% | 1–24 | ♠ **6–7** (Texas) |
| `4♦` | 3.5% | 0–24 | ♥ **6–7** (Texas) |
| **`3♣`** | **2.6%** | **0–23** | **♦ 6–7, ♣ 1–4** |
| `2NT` | 2.3% | 6–10 | balanced 80% |
| **`2♠`** | **2.3%** | **0–25** | **♣ 6–7** |
| `4♣` | 1.0% | 15–27 | balanced 82% (Gerber) |
| `3♦` | 0.9% | 9–26 | ♦ 5–6 |
| `5NT` | 0.5% | 18–21 | balanced 85% |
| `4♠` | 0.5% | 0–20 | ♣ **5–5** ♦ — 5-5 minors |

**The finding.** Both minor transfers are **strictly six-card one-suiters**.
`3♣`'s diamond length has a hard minimum of 6 over 1042 hands; `2♠`'s club
length likewise. EPBot routes the shapes that are *not* six-card one-suiters
elsewhere: 5♦ with values → `3♦` (5–6♦, 9+ HCP), 5♦ without → `3NT`, and 5-5
minors → `4♠`.

This refutes the class `european.rs` shipped from commit `2bcbd3e` until
2026-08-13: `len(♦,6..) | (len(♦,5..) & len(♣,4..))`. That disjunction was
deleted from Puppet's `2NT` rule and pasted byte-identically into *both*
`puppet_minors()` and `european_minors()`, then rationalized in a doc comment
("no room below 3♦ to show the clubs"). It was never probed and no test pinned
the two-suiter arm. Puppet keeps it — that class is measured, and Puppet is a
system we play. European loses it.

Note the fold was *worse* for European than for Puppet on its own terms:
Puppet's `3♣` denial gives a 5♦4♣ hand a pass-or-correct escape, while
European's unconditional completion gives it none.

## 1b — opener's answer (`nt-3c`, n=2968 after the 1NT filter)

| call | share | HCP | balanced |
| --- | --- | --- | --- |
| `3♦` | **100.0%** | 15–17 | 98% |

One bucket, no second call at any share. The completion is unconditional — there
is no super-accept — which is what `european_three_club_answer`'s `hcp(0..)`
already said. Confirmed rather than assumed.

## 1c — responder's rebid (`nt-3c-3d`, n=10260 at the node)

Every bucket has ♦6+ — the filter's own guarantee, restated by the data.

| call | share | HCP (hard) | shape tell |
| --- | --- | --- | --- |
| Pass | 46.4% | 0–7 | ♦6–7 |
| `3NT` | 19.7% | 8–14 | ♦ exactly 6 |
| `4♣` | 11.1% | 10–18 | ♣ 1–4, **median 3** → control cue |
| `5♦` | 8.5% | 1–15 | ♦6–7, signoff |
| `4NT` | 6.5% | 7–28 | keycard (trump AKQ mean 2.09) |
| `4♥` | 3.6% | 10–18 | ♥ **1–3** → control cue |
| **`4♠`** | 2.3% | 7–23 | **♠ 0–0** — void |
| **`5♥`** | 1.6% | 9–21 | **♥ 0–0** — void |
| **`5♣`** | 0.4% | 4–19 | **♣ 0–0** — void, ♦7–9 |

**The finding.** EPBot *does* have shortness-showing calls here, but they are
nothing like the Puppet lane's:

- **Void-only.** `4♠`/`5♥`/`5♣` have the short suit pinned to **0–0** on hard
  min *and* max. A singleton never triggers them. Our Puppet twin splinters on a
  void *or* a low singleton (`splinter_short`).
- **Different rungs.** There is **no `3♥`/`3♠` bucket at any share** — the two
  calls the Puppet lane uses. EPBot's `4♥` and `4♣` occupy the mid rungs as
  ordinary control cues (1–3 and 1–4 cards in the bid suit, medians 2 and 3).
- Two of the three void shows sit *above* `5♦`, so this is not a
  "let opener pick the game" mechanism at all — it is slam machinery running
  beside a plain `4NT` keycard ask.

So the rain check that commit `3d0f376` wrote against this node — *"revisit only
if the diamond splinter measures a win under Puppet"* — resolves to **no**, and
for a reason unrelated to the one recorded there. That commit's stated premise
(European's `3♦` is "a blind transfer completion, not a fit promise, so the
premise is absent") was false on its own terms: responder holds 6+♦ and opener
is balanced, which is an eight-card fit by arithmetic. The correct reason is
simply that EPBot does not bid `3♥`/`3♠` here, and a model that did would be
less faithful.

## What we model, and what we do not

Modelled after 2026-08-13:

- `2♠` = 6+♣, `3♣` = 6+♦ — both exact against the probe.
- Opener's unconditional `3♦` completion — exact.
- Responder's Pass (`hcp(..8)`) / `3NT` (`hcp(8..)`) split — covers the node's
  two biggest buckets, 66.1% combined, and the 8 boundary matches EPBot's
  Pass 0–7 / `3NT` 8–14.

**Not** modelled, deliberately:

- The `5♦` signoff (8.5%), `4NT` keycard (6.5%), `4♣`/`4♥` control cues (14.7%).
- The three void shows `4♠`/`5♥`/`5♣` (4.3%).

Together that is ~34% of the node. These are recognition gaps, not bidding
losses — we never take these calls ourselves. The pin
`no_three_level_splinter_over_the_diamond_completion`
(`tests/american_european_minors.rs`) keeps the Puppet rungs out; adding the
void shows is the natural next fidelity increment, and needs opener's answers
authored with them.

## Ledger

Closes row 12 of [21gf-ledger.md](21gf-ledger.md) (`1N-3♣ transfer to ♦`), open
with an empty A/B column since it was written.
