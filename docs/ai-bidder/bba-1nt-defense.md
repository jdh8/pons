# BBA's defense to a 1NT opening — Woolsey "Multi-Landy"

BBA/EPBot's compiled 2/1 card (system 0) defends a strong 1NT with **Woolsey's
"Multi-Landy"**. Every figure below is a *read of BBA's implementation*, distilled
by driving the real EPBot engine on thousands of random hands via
[`examples/probe-bba-constraints`](../../examples/probe-bba-constraints/main.rs)
(`Multi-Landy=1, Cappelletti=0` forced on all seats). HCP/length bands are the
10th–90th percentile of the sampled bucket; "med" is the median. A `sketch` is a
candidate constraint, not a proof of internal logic.

Reproduce any line below (see commands at the end); archetype spot-checks are in
[`examples/probe-bba-1nt`](../../examples/probe-bba-1nt/main.rs).

## Direct seat over (1NT)

| Call | Meaning | HCP (nv / vul) | Shape |
|------|---------|----------------|-------|
| **X**  | 4-card major **+ longer minor** — *Woolsey, not penalty* | 12–19 / 12–19 (med 14) | exactly 4 in one major, 5–6 in a minor; **never balanced** |
| **2♣** | both majors | 9–19 / 10–19 (med 12) | **≥ 5-4** majors (one 5+, the other 4+) |
| **2♦** | Multi — one **6+ card major** | 9–18 / 10–18 (med 12) | a single 6+ major, nothing else long |
| **2♥** | Muiderberg | 9–19 / 10–19 (med 13) | **exactly 5 hearts + a 4+ minor** |
| **2♠** | Muiderberg | 9–18 / 10–19 (med 13) | **exactly 5 spades + a 4+ minor** |
| Pass | everything else, *including strong balanced* | — | — |

What the buckets and archetypes establish:

- **X is Woolsey, never penalty.** Of 573 sampled X hands, 0% are balanced; all hold
  exactly a 4-card major and a 5–6 card minor (median: 4 spades-or-hearts + 5 diamonds).
  Strong balanced hands sit in *Pass* — a flat **22 HCP** passes (archetype). There is
  no penalty double in this structure.
- **2♣ requires at least 5-4 in the majors** — a 4-4-major hand passes.
- **2♥/2♠ require the 4+ minor** (Muiderberg). A bare 5332 major passes; a 6th card in
  the major makes it the **2♦ Multi** instead.
- **No natural minor overcall.** A 6-card minor one-suiter passes. (BBA's both-minors
  hand bids 2NT — Unusual NT — which is outside this defense.)
- The four suit overcalls are **wide-range (9–18 HCP), not preempts**; vulnerability
  lifts the floor ≈ 1 HCP. The structure relies on the relays below to sort out level.

## Continuations over the 2♦ Multi — advancer at `(1NT)-2♦-P-?`

The advancer almost never passes 2♦ (0.1%); it bids a major as pass-or-correct, in two
strengths:

- **2♥** (33%, 2–14 HCP, med 6) — **weak** pass-or-correct. Overcaller **passes with
  hearts**, **bids 2♠ with spades**, and **jumps 3♥/3♠ with a 7+ suit / extras**. Final
  contract 2♥ or 2♠.
- **2♠** (67%, 7–18 HCP, med 11) — **constructive / invitational** pass-or-correct.
  Overcaller **passes with spades**, **bids 2NT (a heart relay) with hearts** so the
  stronger advancer places the contract (3♥+). Lands a level higher: 2♠ or 3♥.

`rebid-d` / `rebid-d2s` confirm the Multi is a genuine 6+ single major, **symmetric**
between hearts and spades (≈42% pass / ≈42% correct to 2♠; the rest are 7-card jumps).

## Continuations over the 2♥/2♠ Muiderberg — advancer at `(1NT)-2M-P-?`

Three calls do essentially all the work:

- **Pass** (≈49% nv / ≈43% vul, ≤ 12 HCP) — weak, plays 2M.
- **2NT** (≈46% nv / ≈53% vul, 6–18 HCP, med 12) — **artificial minor-ask**. The
  overcaller replies **3♣** (clubs) or **3♦** (diamonds), ≈50/50, showing its 4+ minor.
- **3NT** (≈3%, 14–20 HCP, with a stopper / running minor) — to play.

> **The advancer's direct 3♣/3♦ (and a raise) are vestigial — each < 0.3%.** BBA routes
> all constructive action through the 2NT ask, so in this structure **"3♣/3♦" are the
> *overcaller's answers to 2NT*, not advances.**

## Reproduce

```text
# direct seat: X / 2♣ / 2♦ / 2♥ / 2♠
cargo run --release --example probe-bba-constraints -- --mode multi     --vul none,both --samples 20000
# advances
cargo run --release --example probe-bba-constraints -- --mode advance   --vul none,both
cargo run --release --example probe-bba-constraints -- --mode muider-h  --vul none,both
cargo run --release --example probe-bba-constraints -- --mode muider-s  --vul none,both
# overcaller's rebid (confirms meaning; --min-share 0 to see the full distribution)
cargo run --release --example probe-bba-constraints -- --mode rebid-d   --vul none,both --samples 60000
cargo run --release --example probe-bba-constraints -- --mode rebid-d2s --vul none,both --samples 60000
cargo run --release --example probe-bba-constraints -- --mode rebid-h   --vul none,both --samples 60000
cargo run --release --example probe-bba-constraints -- --mode rebid-s   --vul none,both --samples 60000
# archetype spot-check
cargo run --release --example probe-bba-1nt
```
