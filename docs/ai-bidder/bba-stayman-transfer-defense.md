# BBA's defense to the opponents' 1NT response (Stayman / Jacoby transfers)

How BBA/EPBot's compiled 2/1 card (system 0) competes in the **4th seat** after
the opponents open a strong 1NT and respond with Stayman or a Jacoby transfer —
the auctions `[1NT, P, 2♣]`, `[1NT, P, 2♦(→♥)]`, `[1NT, P, 2♥(→♠)]`. Every figure
is a *read of BBA's implementation*, distilled by driving the real EPBot engine on
40 000 random 4th-seat hands per auction via
[`examples/probe-bba-constraints`](../../examples/probe-bba-constraints/main.rs)
(`--mode stayman|xfer-h|xfer-s`). HCP/length bands are the 10th–90th percentile of
the sampled bucket; "med" is the median. Archetype spot-checks are in
[`examples/probe-bba-1nt`](../../examples/probe-bba-1nt/main.rs) (`responses`).

## The one rule

BBA plays a plain **lead-directing-double + natural** defense:

1. **X = the suit they actually BID** (the artificial relay suit), 5+ cards,
   ~8–18 HCP — *clubs* over Stayman, *diamonds* over the `2♦` transfer, *hearts*
   over the `2♥` transfer. It is a lead-direct/competitive show of that suit, **not
   takeout, not penalty** (only ~25% balanced; a 4-4-4-1 takeout shape passes).
2. **Cue of the suit they SHOWED = the other major + a minor** (Michaels, 5-5),
   only over a transfer (where the shown suit ≠ the bid suit): `2♥` over `2♦(→♥)`
   shows **5+ spades + a 5+ minor**; `2♠` over `2♥(→♠)` shows **5+ hearts + a 5+
   minor**.
3. **Natural suit overcalls** of a real 6+ suit (a cheap 2-level major needs only
   5), ~11–19 HCP, at whatever level the auction forces.
4. **No 2NT, no penalty double.** Across all 120 000 probes no `2NT` bucket clears
   1%; a strong balanced or both-minors hand simply passes (or bids a natural minor
   if long enough). Pass everything that fits none of the above — ~80–83% of hands.

## Direct (4th) seat over Stayman `[1NT, P, 2♣]`

| Call | Meaning | HCP (med) | Shape |
|------|---------|-----------|-------|
| **X** | **clubs** — lead-direct their `2♣` | 7–18 (11) | 5+ clubs; only ~24% balanced |
| 2♦ / 2♥ / 2♠ | natural | 12–19 (14) | a 6+ suit |
| 3♣ | natural clubs, to play (vs. the lead-direct X) | 11–19 (14) | 6+ clubs |
| Pass | everything else (incl. strong balanced, 5-5 majors) | — | — |

No cue exists here: their bid suit (clubs) *is* the X, and Stayman shows no anchor
suit to cue. 83% Pass.

## Direct seat over the `2♦` transfer (→ hearts) `[1NT, P, 2♦]`

| Call | Meaning | HCP (med) | Shape |
|------|---------|-----------|-------|
| **X** | **diamonds** — lead-direct their `2♦` | 7–18 (11) | 5+ diamonds |
| **2♥** | **cue (their shown suit) = spades + a minor** | 7–16 (11) | 5+ spades + a 5+ minor, ≤2 hearts |
| 2♠ | natural spades | 11–19 (14) | 5+ spades |
| 3♣ / 3♦ | natural clubs / diamonds | 11–20 (14–15) | 5–6+ in the minor |
| Pass | hearts (their shown suit) or insufficient | — | — |

## Direct seat over the `2♥` transfer (→ spades) `[1NT, P, 2♥]`

| Call | Meaning | HCP (med) | Shape |
|------|---------|-----------|-------|
| **X** | **hearts** — lead-direct their `2♥` | 8–18 (11) | 5+ hearts |
| **2♠** | **cue (their shown suit) = hearts + a minor** | 7–17 (11) | 5+ hearts + a 5+ minor, ≤2 spades |
| 3♣ / 3♦ / 3♥ | natural (forced to the 3-level) | 11–20 (14–15) | 5–6+ in that suit |
| Pass | spades (their shown suit) or insufficient | — | — |

## Notes for distilling this into our book

- The scheme is **symmetric and trivially keyed**: X always = the *bid* suit, the
  cue (transfers only) always = *the other major + a minor*. A single
  suit-variable template covers all three auctions — the same shape the BBA floor
  reverse-eng (`bba-floor.md`) uses everywhere.
- The X is a **DONT/lead-directing double**, the same family as our direct-seat
  DONT one-suiter `X` over (1NT) — reuse, don't reinvent.
- Whether this earns its keep is a DD/PD question, not a correctness one. Like all
  competitive-defense conventions it should be measured **vs. BBA**, not vs.
  always-pass (the passed-hand 1NT-defense lesson: a self-play-vs-always-pass win
  can flip to a loss vs. BBA), and the obstruction/lead-direction value of X is
  largely invisible to plain DD.

## Reproduce

```text
# statistical buckets (X / cue / natural, with HCP & length bands)
cargo run --release --example probe-bba-constraints -- --mode stayman --vul none,both --samples 40000
cargo run --release --example probe-bba-constraints -- --mode xfer-h  --vul none,both --samples 40000
cargo run --release --example probe-bba-constraints -- --mode xfer-s  --vul none,both --samples 40000
# archetype spot-check (one-suiters, two-suiters, strong balanced)
cargo run --release --example probe-bba-1nt responses
```
