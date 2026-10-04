# Major-suit game threshold vs. trump length — double-dummy census

Computed 2026-10-04 from the `pons` double-dummy deal bank. This is a plain
statistic over random deals, with no bidding simulation.

## Question

When should a partnership with a major-suit fit bid game (4M) instead of
stopping in a partscore? The study looks at two things:

1. the point threshold for each exact trump length (8 to 12+ cards), and
2. how many points each trump beyond eight is worth (1, 1.5 or 2?).

## Data and method

- **Deals:** 102,577,264 random deals, each with a complete double-dummy trick
  table. Every partnership (N–S and E–W) holding 8+ cards in hearts or spades
  is one case, about 102M cases in all.
- **Excluded:** 7-card fits, where the real alternative is usually 3NT, not
  3M. Also 8-card fits where one hand is (4333) and the other is balanced
  ((4333), (4432) or (5332)); such a pair plays 3NT. This removes 5.4M of the
  66.4M 8-card cases (8%).
- **Tricks:** double-dummy, taking the better of the two partners as declarer.
  This is slightly optimistic: it assumes the hand is always right-sided.
- **Decision value:** the IMPs gained by bidding 4M instead of stopping in 3M,
  with the same trick count, undoubled, and opponents silent. The 4M-vs-2M
  comparison was also computed. It moves every threshold about 0.3–0.5 points
  higher; the tables below use 4M vs 3M.
- **Break-even point (P\*):** the point total where the mean IMP gain crosses
  zero, by linear interpolation between neighbouring integers.
- **Gate:** the smallest whole point total where the mean IMP gain is positive.
- **Sample size:** about 7M cases per point value for 8-card fits, so the noise
  is negligible except for 12+ trumps. 95% intervals are shown where they
  exceed ±0.01.

### Point scales

Trump length is **never** included in these scales. It is the separate
variable being studied. Two counts are combined in different ways:

- *Support points of one hand.* Count trump suit HCP only. In each side suit,
  count max(HCP, shortness, HCP + shortness − 1), with void = 3,
  singleton = 2 and doubleton = 1. This avoids counting a short honour twice.
  Add 1 if the two longest suits total 10+ cards.
- *Ordinary points of one hand.* HCP plus a shape upgrade of 0–2: +1 for an
  unbalanced hand and +1 if the two longest suits total 10+ cards. Subtract 1
  for each wasted short honour (an honour in a suit of 2 or fewer cards,
  except Ax and Kx). The upgrade never goes below 0.

The three scales:

1. **Both hands on support points (the `pons` method; main scale).** Once a
   fit is established, both partners count support points, whichever hand
   holds more trumps.
2. **Dummy-only support points (the classic textbook method).** The hand with
   fewer trumps counts support points and the hand with more trumps counts
   ordinary points. When both hands hold the same number of trumps, the case
   is counted both ways at half weight.
3. **Raw HCP:** 4-3-2-1, summed over both hands.

## Result 1 — break-even point for each trump length

**Both hands on support points (`pons`):**

| trumps | cases | P\* not vul | P\* vul | gate not vul / vul | worth of this trump (not vul / vul) | P\* + trumps (not vul) |
|---|---|---|---|---|---|---|
| **8** | 61.0M | **24.68** | **24.23** | **25 / 25** | (baseline) | **32.7** |
| 9 | 30.2M | 23.32 | 22.83 | 24 / 23 | 1.37 / 1.40 | 32.3 |
| 10 | 8.9M | 22.55 | 22.05 | 23 / 23 | 0.77 / 0.78 | 32.5 |
| 11 | 1.6M | 22.12 ± 0.02 | 21.50 ± 0.02 | 23 / 22 | 0.43 / 0.55 | 33.1 |
| 12+ | 169k | 22.03 ± 0.07 | 21.40 ± 0.05 | 22–23 / 22 | 0.09 / 0.10 | ≈34.0 |

For 12+ trumps not vulnerable, 22 points is −0.04 ± 0.09 IMPs, which is not
significantly different from zero, so the gate is 22 or 23.

**Dummy-only support points (classic):**

| trumps | cases | P\* not vul | P\* vul | gate not vul / vul | worth of this trump (not vul / vul) | P\* + trumps (not vul) |
|---|---|---|---|---|---|---|
| **8** | 61.0M | **23.79** | **23.32** | **24 / 24** | (baseline) | **31.8** |
| 9 | 30.2M | 22.22 | 21.69 | 23 / 22 | 1.57 / 1.62 | 31.2 |
| 10 | 8.9M | 21.14 | 20.55 | 22 / 21 | 1.08 / 1.14 | 31.1 |
| 11 | 1.6M | 20.24 ± 0.02 | 19.53 ± 0.02 | 21 / 20 | 0.90 / 1.02 | 31.2 |
| 12+ | 169k | 19.62 ± 0.07 | 18.87 ± 0.12 | 20 / 19 | 0.62 / 0.66 | ≈31.7 |

**Raw HCP:**

| trumps | 8 | 9 | 10 | 11 | 12+ |
|---|---|---|---|---|---|
| P\* not vul | 22.33 | 20.37 | 18.68 | 17.14 | 15.83 ± 0.14 |
| P\* vul | 21.80 | 19.73 | 17.93 | 16.26 | 14.74 ± 0.20 |
| gate not vul / vul | 23 / 22 | 21 / 20 | 19 / 18 | 18 / 17 | 16 / 15 |
| worth of this trump, not vul | (baseline) | 1.96 | 1.69 | 1.54 | 1.31 |

**Detail near the gate, both hands on support points** (4M vs 3M):

| trumps | points | P(4M makes) | IMPs not vul | IMPs vul |
|---|---|---|---|---|
| 8 | 24 | 29% | −0.93 | −0.49 |
| 8 | 25 | 44% | +0.43 | +1.64 |
| 9 | 23 | 35% | −0.46 | +0.30 |
| 9 | 24 | 51% | +0.98 | +2.53 |
| 10 | 22 | 33% | −0.73 | −0.10 |
| 10 | 23 | 47% | +0.60 | +1.97 |
| 11 | 22 | 39% | −0.16 ± 0.03 | +0.80 ± 0.04 |
| 11 | 23 | 53% | +1.16 ± 0.03 | +2.84 ± 0.04 |
| 12+ | 22 | 41% | −0.04 ± 0.09 | +1.03 ± 0.13 |
| 12+ | 23 | 55% | +1.26 ± 0.09 | +3.01 ± 0.13 |

**Detail near the gate, dummy-only support points** (4M vs 3M):

| trumps | points | P(4M makes) | IMPs not vul | IMPs vul |
|---|---|---|---|---|
| 8 | 23 | 27% | −1.02 | −0.65 |
| 8 | 24 | 43% | +0.28 | +1.42 |
| 9 | 22 | 36% | −0.31 | +0.52 |
| 9 | 23 | 52% | +1.08 | +2.67 |
| 10 | 21 | 38% | −0.18 | +0.73 |
| 10 | 22 | 52% | +1.06 | +2.65 |
| 11 | 20 | 37% | −0.24 ± 0.05 | +0.61 ± 0.08 |
| 11 | 21 | 49% | +0.76 ± 0.05 | +2.18 ± 0.08 |
| 12+ | 19 | 34% | −0.55 ± 0.09 | +0.12 ± 0.13 |
| 12+ | 20 | 44% | +0.34 ± 0.09 | +1.55 ± 0.13 |

## Result 2 — how much is each trump beyond eight worth?

The rule tested is "bid game when points + k × (trumps − 8) ≥ t", with the best
t chosen for each k. The rule is scored by the IMPs it gives up per case
compared with bidding game in exactly the cells (trump length × point total)
where game gains on average. The loss is in thousandths of an IMP (mIMP) per
case, over fits of 8+ cards.

**Both hands on support points (`pons`):**

| k per extra trump | not vul: best t | not vul: loss | vul: best t | vul: loss |
|---|---|---|---|---|
| 0 (ignore length) | 24.5 | 48.9 | 23.5 | 50.1 |
| 0.5 | 24.5 | 4.4 | 24.5 | 22.9 |
| **1** | 24.5 | **0.3** | 24.5 | 7.3 |
| **1.5** | 25 | 7.0 | 24.5 | **3.9** |
| 2 | 25.5 | 27.4 | 24.5 | 17.2 |

**Dummy-only support points (classic):**

| k per extra trump | not vul: best t | not vul: loss | vul: best t | vul: loss |
|---|---|---|---|---|
| 0 (ignore length) | 22.5 | 60.5 | 22.5 | 79.3 |
| 0.5 | 23.5 | 9.2 | 22.5 | 36.9 |
| **1** | 23.5 | **0.0** | 23.5 | 18.8 |
| **1.5** | 24 | 1.7 | 23.5 | **0.8** |
| 2 | 24.5 | 15.0 | 23.5 | 8.4 |
| 2.5 | 25 | 24.5 | 24 | 22.1 |

**Raw HCP:**

| k per extra trump | not vul: best t | not vul: loss | vul: best t | vul: loss |
|---|---|---|---|---|
| 0 | 21.5 | 71.7 | 20.5 | 112.5 |
| 1 | 21.5 | 21.5 | 21.5 | 21.8 |
| 1.5 | 22.5 | 1.8 | 21.5 | 0.5 |
| **2** | 22.5 | **0.1** | 21.5 | **0.3** |
| 2.5 | 23 | 4.4 | 22 | 7.0 |

## Conclusions

1. **A trump is worth less with each extra trump, and how fast its value falls
   depends on the scale.** Measured from the 9th to the 12th trump, not
   vulnerable:
   - raw HCP: 2.0, 1.7, 1.5, 1.3;
   - dummy-only support points: 1.6, 1.1, 0.9, 0.6;
   - both hands on support points: 1.4, 0.8, 0.4, 0.1.

   Each scale that credits more shortness leaves less for the trump term.
   The long-trump hand's shortness is in effect its extra trumps, so a
   both-hands count already prices most of the 10th and later trumps.
2. **Under the `pons` count (both hands), one trump is worth one point in
   practice.** With k = 1 the gates come out at 25, 24, 23, 22 and 21 for 8,
   9, 10, 11 and 12+ trumps. The true gates are 25, 24, 23, 23 and 22–23, so
   the rule is about one point too light only with 11+ trumps. Those fits are
   rare, which keeps the loss at 0.3 mIMP not vul. Vulnerable, k = 1.5 is a
   little better (3.7 vs 7.0 mIMP), because the 9th trump moves the gate down
   two whole points (25 → 23).
3. **Rules of thumb for 8+ card fits:**
   - Both hands on support points: "25 with eight trumps, 1 less for each
     extra trump, but no lower than 23." That is 33 in "points + trumps"
     terms.
   - Dummy-only support points: "24 with eight trumps, 1½ less for each extra
     trump."
   - HCP: "22–23 HCP with eight trumps, 2 less for each extra trump" (22½ not
     vul, 21½ vul).
4. **Vulnerable thresholds are 0.45–0.75 points lower** at every trump length
   (up to about 1 point on raw HCP with 10+ trumps). This matches the lower
   break-even for vulnerable games at IMPs (about 37.5% vs about 45%).
5. **Measured on actual hands, the break-even for "points + trumps" under the
   `pons` count is about 32.3–32.7 for 8–10 trumps.** It rises to about 33–34
   for 11+, where the trump term overpays. On the dummy-only count it is about
   31.1–31.8.
6. **Thresholds on real hands vs. thresholds on shown ranges.** All numbers
   here use the actual combined holdings. A bidding gate that adds up the
   minimums partner has shown sees a lower total than the true one, so it
   should sit below these break-even points. The `pons` engine's gate is
   "own support points + partner's shown minimum support points + combined
   trumps ≥ 31". On actual hands its break-even is about 32.7 with an 8-card
   fit, so the range bottoms must absorb about 1.7 points. In engine A/Bs, 31
   measured better than 32.

## Reproduce

[`examples/probe-game-threshold.rs`](../examples/probe-game-threshold.rs)
writes the per-cell table;
[`scripts/game-threshold-report.py`](../scripts/game-threshold-report.py)
prints every table above from it.

```sh
cargo build --release --example probe-game-threshold
RAYON_NUM_THREADS=8 nice -n 19 target/release/examples/probe-game-threshold \
    /nfs2/jdh8/pons/22.pdd /nfs2/jdh8/pons/24.pdd /nfs2/jdh8/pons/shard-*.pdd \
    /mnt/hdd-data/jdh8/shards/shard-*.pdd > major.tsv
python3 scripts/game-threshold-report.py major.tsv
```

## Caveats

- Double-dummy play with the better declarer: no wrong-siding, and no
  single-dummy guessing by either side.
- Undoubled and uncontested. The study does not price competitive auctions or
  the opponents' sacrifices.
- The `pons` row treats both hands as counting support points exactly. In the
  engine, partner's contribution is the bottom of a shown support range, or
  of the plain point range if partner never showed support.
