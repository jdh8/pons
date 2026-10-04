# 3NT game threshold vs. HCP — double-dummy census

Computed 2026-10-04 from the `pons` double-dummy deal bank. This is a plain
statistic over random deals, with no bidding simulation. It is the notrump
companion to the [major-suit](major-game-threshold.md) and
[minor-suit](minor-game-threshold.md) studies.

## Question

When should a partnership bid 3NT instead of stopping in a notrump partscore?
The study looks at three things:

1. the combined-HCP threshold for 3NT,
2. how much a long suit is worth on top of HCP ("add a point for a five-card
   suit"?), and
3. how much flat 4-3-3-3 hands cost.

## Data and method

- **Deals:** 102,577,264 random deals, each with a complete double-dummy trick
  table. Every partnership (N–S and E–W) is one case, 205M cases in all. The
  main population is the 105M cases **without an 8+ card major fit**, where
  3NT is the usual game. The 100M cases with a major fit are reported
  separately; for them 4M is usually the real alternative and is not priced
  here.
- **Tricks:** double-dummy notrump tricks, taking the better of the two
  partners as declarer. This is slightly optimistic: it assumes the hand is
  always right-sided.
- **Decision value:** the IMPs gained by bidding 3NT instead of stopping in
  2NT, with the same trick count, undoubled, and opponents silent. The
  3NT-vs-1NT comparison was also computed. It moves every threshold about
  0.25–0.6 points higher; the tables below use 3NT vs 2NT unless noted.
- **Break-even point (P\*):** the point total where the mean IMP gain crosses
  zero, by linear interpolation between neighbouring values.
- **Gate:** the smallest point total where the mean IMP gain is positive.
- **Sample size:** about 6–7M cases per HCP value near the gate, so the noise
  is negligible. 95% intervals are shown where they exceed ±0.01.

### Point scales and side variables

Three strength scales, each summed over both hands:

1. **Raw HCP** (4-3-2-1). The main scale.
2. **Fifths** (Thomas Andrews' notrump scale: A 4.0, K 2.8, Q 1.8, J 1.0,
   T 0.4; 40 in the deck). It is binned to the nearest half point.
3. **`pons` `point_count`:** HCP plus the 0–2 shape upgrade (+1 unbalanced,
   +1 if the two longest suits total 10+ cards, −1 per wasted short honour,
   never below 0). On balanced hands it equals HCP.

Two side variables, never included in any scale:

- **Length points X:** the number of cards beyond four in every suit of both
  hands, Σ max(0, length − 4). A 5-3-3-2 opposite a 4-3-3-3 is X = 1; a
  6-card suit, or a 5-card suit in each hand, is X = 2. Capped at 5+.
- **Flat count F:** how many of the two hands are 4-3-3-3 (0, 1 or 2).

## Result 1 — break-even point for 3NT

**By population** (3NT vs 2NT; last two columns 3NT vs 1NT):

| scale | population | cases | P\* not vul | P\* vul | gate not vul / vul | P\* vs 1NT not vul / vul |
|---|---|---|---|---|---|---|
| **HCP** | no 8+ major fit | 104.9M | **23.75** | **23.35** | **24 / 24** | 24.13 / 23.78 |
| HCP | 8+ major fit | 100.3M | 23.86 | 23.43 | 24 / 24 | 24.35 / 24.00 |
| Fifths | no 8+ major fit | 104.9M | 23.62 | 23.26 | 24 / 23½ | 23.96 / 23.63 |
| `point_count` | no 8+ major fit | 104.9M | 24.47 | 24.09 | 25 / 25 | 24.90 / 24.55 |

**By length points X** (no 8+ major fit, 3NT vs 2NT, not vul):

| X | cases | HCP P\* | HCP gate nv / vul | worth of this length point (HCP) | Fifths P\* | `point_count` P\* |
|---|---|---|---|---|---|---|
| 0 | 18.1M | 24.17 | 25 / 24 | (baseline) | 24.00 | 24.22 |
| 1 | 30.8M | 23.79 | 24 / 24 | 0.38 | 23.66 | 24.16 |
| 2 | 28.6M | 23.63 | 24 / 24 | 0.16 | 23.51 | 24.47 |
| 3 | 17.0M | 23.51 | 24 / 24 | 0.12 | 23.39 | 25.01 |
| 4 | 7.3M | 23.48 | 24 / 24 | 0.03 | 23.37 ± 0.01 | 25.52 |
| 5+ | 3.1M | 23.49 ± 0.01 | 24 / 24 | −0.01 | 23.36 ± 0.02 | 26.10 ± 0.02 |

Vulnerable P\* runs 0.35–0.45 below each not-vulnerable value; the
increments are the same.

**By flat 4-3-3-3 count F** (no 8+ major fit, X = 0 so that length is held
fixed, not vul):

| F | cases | HCP P\* | cost vs F = 0 | Fifths P\* | cost vs F = 0 |
|---|---|---|---|---|---|
| 0 | 6.9M | 24.08 | — | 23.91 | — |
| 1 | 8.9M | 24.17 | 0.09 | 24.00 | 0.09 |
| 2 | 2.3M | 24.38 | 0.30 | 24.21 ± 0.01 | 0.30 |

Without holding X fixed, F = 2 looks 0.66 HCP worse than F = 0. More than
half of that is the missing long suit, not the flatness itself.

**Detail near the gate, HCP** (no 8+ major fit, 3NT vs 2NT):

| X | HCP | P(3NT makes) | IMPs not vul | IMPs vul |
|---|---|---|---|---|
| all | 23 | 25% | −1.02 | −0.77 |
| all | 24 | 42% | +0.34 | +1.43 |
| 0 | 24 | 37% | −0.31 | +0.52 |
| 0 | 25 | 57% | +1.59 | +3.44 |
| 1 | 23 | 24% | −1.14 | −0.93 |
| 1 | 24 | 42% | +0.31 | +1.41 |
| 2 | 23 | 26% | −0.87 | −0.55 |
| 2 | 24 | 43% | +0.52 | +1.69 |
| 3 | 23 | 28% | −0.67 | −0.27 |
| 3 | 24 | 44% | +0.65 | +1.84 |

**Detail near the gate, Fifths** (no 8+ major fit):

| X | Fifths | P(3NT makes) | IMPs not vul | IMPs vul |
|---|---|---|---|---|
| all | 23½ | 36% | −0.21 | +0.58 |
| all | 24 | 45% | +0.62 | +1.87 |
| 0 | 23½ | 30% | −0.94 | −0.46 |
| 0 | 24 | 40% | +0.01 | +1.03 |
| 1 | 23½ | 36% | −0.28 | +0.49 |
| 1 | 24 | 45% | +0.61 | +1.88 |

## Result 2 — how much is length worth?

The rule tested is "bid 3NT when points + adj(X, F) ≥ t", with the best t
(on a quarter-point grid) chosen for each adjustment. Each rule is scored two
ways, both in thousandths of an IMP (mIMP) per case over the no-major-fit
population:

- **captured:** the IMPs the rule gains over always stopping in 2NT. This
  number compares across scales.
- **loss:** the IMPs given up compared with bidding 3NT in exactly the cells
  (X × F × points) where 3NT gains on average. This number compares rules
  within one scale only, because Fifths has twice as many cells.

| scale | adjustment | not vul: t / captured / loss | vul: t / captured / loss |
|---|---|---|---|
| HCP | none | 23¼ / 723.5 / 4.0 | 23¼ / 1315.5 / 3.3 |
| HCP | +¼ per length point | 24¼ / 725.8 / 1.7 | 24 / 1314.7 / 4.2 |
| HCP | +½ per length point | 24¾ / 717.7 / 9.8 | 24¼ / 1303.6 / 15.2 |
| HCP | +1 per length point | 25¼ / 683.3 / 44.2 | 24¼ / 1239.8 / 79.0 |
| **HCP** | **+½ once, for any 5+ card suit** | **24¼ / 726.7 / 0.8** | 23¾ / 1315.5 / 3.3 |
| HCP | −½ if both hands 4-3-3-3 | 23¾ / 724.5 / 3.0 | 23¾ / 1315.7 / 3.2 |
| Fifths | none | 23¾ / 736.0 / 3.1 | 23¼ / 1331.1 / 4.9 |
| **Fifths** | **+¼ per length point, max 2** | 24 / **737.0** / 2.1 | 23¾ / **1333.1** / 2.9 |
| Fifths | +1 per length point | 25¼ / 687.5 / 51.6 | 24¾ / 1257.2 / 78.8 |
| `point_count` | none | 24¼ / 711.4 / 6.3 | 24¼ / 1283.7 / 18.5 |
| `point_count` | −½ per length point | 23¾ / 711.9 / 5.8 | 23¼ / 1298.7 / 3.4 |
| `point_count` | +½ per length point | 25¼ / 672.5 / 45.2 | 24¾ / 1239.6 / 62.5 |

On the integer HCP scale, "+½ once with t = 24¼" means **25 HCP with no
five-card suit, 24 with one** not vulnerable. Vulnerable, 24 is the gate either
way.

## Conclusions

1. **The 3NT gate is 24 combined HCP, both vulnerable and not, with no
   8-card major fit.** At 24 HCP, 3NT makes 42% of the time and gains 0.34
   IMPs not vulnerable and 1.43 vulnerable. The break-even is 23.75 not vul
   and 23.35 vul. Against stopping in 1NT instead of 2NT it is about 0.4
   higher (24.13 / 23.78).
2. **A long suit is worth much less than a point.** The first length point
   (a 5-card suit anywhere) is worth 0.38 HCP. The second adds 0.16, the
   third 0.12, and after that nothing. "Add 1 for a five-card suit" overpays
   2.5×. "+1 per length point" gives up 44 mIMP not vul and 79 vul, about ten
   times the loss of ignoring length altogether. Fifths shows the same curve
   (0.34, 0.15, 0.12, 0.02), so its card weights do not absorb length.
3. **The only length rule worth stating: not vulnerable, 25 HCP with no
   five-card suit, 24 with one.** Vulnerable, bid 3NT on 24 regardless. The
   rule recovers about 3 mIMP per case over plain HCP not vulnerable and
   nothing vulnerable, so it is a rounding refinement and not a valuation
   principle.
4. **The curse of 4-3-3-3 is real but small for game.** With length held
   fixed, one flat hand costs 0.09 HCP and two flat hands cost 0.30. On the
   integer scale this almost never moves the gate. A −½ adjustment for
   4-3-3-3 opposite 4-3-3-3 gains about 1 mIMP per case.
5. **Fifths is the better notrump scale.** It captures about 12 mIMP per case
   more than HCP not vulnerable (736.0 vs 723.5) and about 16 more vulnerable
   (1331.1 vs 1315.5). That is three to five times what any length rule adds
   to HCP. Its gate is 24 not vul and 23½ vul.
6. **`point_count` is the wrong scale for 3NT on unbalanced hands.** Its shape
   upgrade pushes the break-even up instead of down: 24.2 at X = 0–1 and 26.1
   at X = 5+. It captures 12 mIMP per case less than HCP not vulnerable and
   32 less vulnerable. On balanced hands it equals HCP, so this matters only
   where the engine gauges an unbalanced hand's 3NT decision with `points()`.
7. **Compared with the major-suit study:** with an 8-card major fit the 4M
   gate on raw HCP is 22.33 not vul (gate 23). 3NT needs about 1.4 HCP more
   (23.75, gate 24). In "points + trumps" terms, a 4-4 or 5-3 major fit is
   worth about 1.4 HCP over notrump at the game boundary. (The major study
   leaves out 8-card fits with a 4-3-3-3 opposite a balanced hand; with them
   in, its 8-trump break-even was 22.49 and the gap 1.3 HCP.)
8. **Thresholds on real hands vs. thresholds on shown ranges.** As in the
   major study, these numbers use actual combined holdings. A bidding gate
   that adds up the minimums partner has shown should sit below them.

## Reproduce

[`examples/probe-notrump-threshold.rs`](../examples/probe-notrump-threshold.rs)
writes the per-cell table;
[`scripts/notrump-threshold-report.py`](../scripts/notrump-threshold-report.py)
prints Results 1 and 2 from it.

```sh
cargo build --release --example probe-notrump-threshold
RAYON_NUM_THREADS=8 nice -n 19 target/release/examples/probe-notrump-threshold \
    /nfs2/jdh8/pons/22.pdd /nfs2/jdh8/pons/24.pdd /nfs2/jdh8/pons/shard-*.pdd \
    /mnt/hdd-data/jdh8/shards/shard-*.pdd > notrump.tsv
python3 scripts/notrump-threshold-report.py notrump.tsv
```

## Caveats

- Double-dummy play with the better declarer: no wrong-siding, a perfect
  opening lead, and no single-dummy guessing. In notrump the perfect lead
  helps the defence, and the perfect play helps declarer. Both studies share
  this bias, so the comparison between them holds.
- Undoubled and uncontested. The study does not price competitive auctions,
  the opponents' leads into an unstopped suit after a revealing auction, or
  penalty doubles of 3NT.
- 3NT is compared only with a notrump partscore. When length or a fit makes
  a suit contract (4M, 5m, or a minor partscore) the real alternative, the
  value of length is understated here. That is most relevant for X ≥ 3.
- Length points X do not distinguish one 6-card suit from two 5-card suits,
  or a suit in one hand from a suit split across both.
