# BBA's defense to a 1NT opening — Woolsey "Multi-Landy"

BBA/EPBot's compiled 2/1 card (system 0) defends a strong 1NT with **Woolsey's
"Multi-Landy"**. Every figure below is a *read of BBA's implementation*. The hand
bands come from driving the real EPBot engine on thousands of random hands via
[`examples/probe-bba-constraints`](../../examples/probe-bba-constraints/main.rs);
the `2♣` continuation frequencies come from coherent live deals and name their
corpus below. HCP/length bands are the 10th–90th percentile of the sampled bucket;
"med" is the median. A `sketch` is a candidate constraint, not a proof of internal
logic.

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

## Mixed pons–BBA continuations over the 2♣ Landy — `1NT (2♣) [Pass or X]`

This is the requested pons-opener perspective: pons opens `1NT`, BBA overcalls
`(2♣)`, and our responder passes (`-`) or doubles. The table is BBA advancer's
next call. It comes from all 24 shards of
`ab-results/landy-doubler-flip/base-{none,both}` (seed `1787942099`), keeping a
table only when ownership proves that pons opened and BBA made both opposing
calls. A raw search for `1NT 2♣` is invalid because each dump also contains the
opposite table.

This is not BBA's own counter-defense: pons occupies both seats on the opener's
side. The coherent four-BBA continuation tree, including opener's and
responder's later doubles, is in
[`bba-1nt-landy-tree.md`](bba-1nt-landy-tree.md).

| our call | vul | live n | BBA `2♥` | `2♠` | `2♦` | Pass | `3♥` | other |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `-` | none | 781,796 | 35.2% | 35.7% | 13.8% | 9.0% | 4.3% | 2.0% |
| `-` | both | 694,257 | 34.4% | 35.1% | 13.0% | 9.8% | 5.0% | 2.7% |
| `X` | none | 239,098 | 46.1% | 36.3% | 8.1% | 9.1% | 0.3% | <0.1% |
| `X` | both | 187,177 | 48.0% | 35.9% | 5.5% | 10.1% | 0.5% | <0.1% |

The material continuation rates are conditional on our player between BBA's
two turns passing, except where the balancing tail is stated explicitly:

- **`2♥`/`2♠` are weak preferences.** The Landy overcaller passes a direct
  `2♥` 94.7–95.4% and a direct `2♠` 98.2–98.4% after our Pass. After our `X`,
  both preferences are passed more than 99.5%.
- **`2♦` is an artificial ask in both branches, not natural.** After our Pass,
  the overcaller answers `2♥`/`2♠`/`2NT` about 41/40/19%. After our `X`, those
  shares are 36/37/27% non-vulnerable and 33/33/34% vulnerable. On a one-shard
  continuation, advancer passed a named major 90.5–96.1% after our Pass and
  100% after our `X`; over `2NT`, Pass was still most common (57–59% / 72–79%),
  followed by corrections to `3♠` and `3♥`.
- **Pass is live.** After our Pass it plays `2♣` when opener also passes; in a
  one-shard check opener instead balanced with `2♦` 10.2% non-vulnerable and
  6.7% vulnerable. Over that balance BBA passed 44/55%, bid `2♥` 28/27%, bid
  `2♠` 20/15%, or bid `3♣` 8/2%. After our `X` and two passes, the Landy
  overcaller sits for `2♣X` 80.8% non-vulnerable / 83.2% vulnerable, otherwise
  running chiefly to `2♥` (10.6/9.2%) or `2♠` (8.5/7.5%).
- **Direct `3♥` after Pass is an invitational heart preference.** Every one of
  2,874 one-shard hands held four-plus hearts (median 9 HCP); overcaller raised
  to `4♥` 50.3/46.1%, passed 44.3/47.8%, and bid `3♠` about 4–5%. After our `X`
  the branch was tiny and weaker; overcaller passed about 81%, so the live
  sample cannot prove that BBA assigns it a distinct encoded meaning.
- **XX is not BBA's equal-major relay here:** it occurred zero times in the
  426,275 coherent live `X` auctions. An actor-only control produced one XX in
  4,000 random hands, on seven clubs and no hearts; that makes it a negligible
  escape tail, not impossible. Direct `4M` and `2NT` make up most of the
  remaining Pass branch.

An explicit control run set `Multi-Landy=1, Cappelletti=0, Landy=0`. BBA labeled
`2♣` “Multi-Landy, both majors”, our `X` “bidable suit”, `2♦` “artificial”, and
the direct `2M` advances “weak”. The control conditions only the acting hand,
so the live corpus—not its arbitrary-hand frequencies—is the evidence for the
continuation tree.

This is a treatment difference, not by itself a pons bug. Our own
[`nt_landy.rs`](../../src/bidding/american/defense/nt_landy.rs) deliberately
documents XX as the equal-major relay and `2♦` as natural after a double, and
its undoubled table has no catch-all Pass. Leave that default unchanged unless
the complete BBA tree is trialled behind a reversible knob and measured.

## Continuations over the 2♦ Multi — advancer at `(1NT) 2♦ - ?`

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

## Continuations over the 2♥/2♠ Muiderberg — advancer at `(1NT) 2M - ?`

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
# explicit setting/meaning guards for the 2♣ Pass and Double branches
cargo run --release --example probe-bba-constraints -- --mode custom --seat 3 --calls '1NT 2♣ -' --vul none,both --samples 2000 --meanings 50 --conv 'Multi-Landy=1' --conv 'Cappelletti=0' --conv 'Landy=0'
cargo run --release --example probe-bba-constraints -- --mode custom --seat 3 --calls '1NT 2♣ X' --vul none,both --samples 2000 --meanings 50 --conv 'Multi-Landy=1' --conv 'Cappelletti=0' --conv 'Landy=0'
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
