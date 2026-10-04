# Minor-suit game threshold vs. trump length — double-dummy census

Computed 2026-10-04 from the `pons` double-dummy deal bank. This is a plain
statistic over random deals, with no bidding simulation. It is the companion of
[major-game-threshold.md](major-game-threshold.md) and uses the same method;
only the differences are spelled out here.

## Question

When should a partnership with a minor-suit fit bid game (5m) instead of
stopping in a partscore? The study looks at three things:

1. the point threshold for each exact trump length (8 to 12+ cards),
2. how many points each trump beyond eight is worth, and
3. how 5m compares with 3NT, the usual alternative game.

## Data and method

Same as the major study except:

- **Cases:** every partnership holding 8+ cards in clubs or diamonds, about
  102M cases. The same exclusion applies: 8-card fits where one hand is (4333)
  and the other is balanced. The case counts match the major study to three
  digits, as symmetry predicts.
- **Decision value:** the IMPs gained by bidding 5m instead of stopping in 4m,
  with the same trick count, undoubled, and opponents silent.
- **5m vs 3NT:** the IMPs of 5m over 3NT, each with the better declarer of the
  pair in its own strain.
- **Point scales:** the same three, with support points counted for the minor
  as trump suit.

## Result 1 — break-even point for each trump length

**Both hands on support points (`pons`):**

| trumps | cases | P\* not vul | P\* vul | gate not vul / vul | worth of this trump (not vul / vul) | P\* + trumps (not vul) |
|---|---|---|---|---|---|---|
| **8** | 61.0M | **27.43** | **26.96** | **28 / 27** | (baseline) | **35.4** |
| 9 | 30.2M | 26.25 | 25.72 | 27 / 26 | 1.19 / 1.24 | 35.2 |
| 10 | 8.9M | 25.67 | 25.15 | 26 / 26 | 0.58 / 0.58 | 35.7 |
| 11 | 1.6M | 25.44 ± 0.01 | 24.88 ± 0.02 | 26 / 25 | 0.23 / 0.27 | 36.4 |
| 12+ | 169k | 25.45 ± 0.04 | 24.93 ± 0.07 | 26 / 25 | −0.01 / −0.05 | ≈37.5 |

With eight trumps vulnerable, 27 points is only +0.07 IMPs, so the gate there
is marginal.

**Dummy-only support points (classic):**

| trumps | cases | P\* not vul | P\* vul | gate not vul / vul | worth of this trump (not vul / vul) | P\* + trumps (not vul) |
|---|---|---|---|---|---|---|
| **8** | 61.0M | **26.50** | **26.04** | **27 / 27** | (baseline) | **34.5** |
| 9 | 30.2M | 25.15 | 24.60 | 26 / 25 | 1.35 / 1.44 | 34.2 |
| 10 | 8.9M | 24.33 | 23.74 | 25 / 24 | 0.83 / 0.86 | 34.3 |
| 11 | 1.6M | 23.77 ± 0.02 | 23.11 ± 0.02 | 24 / 24 | 0.55 / 0.63 | 34.8 |
| 12+ | 169k | 23.39 ± 0.05 | 22.72 ± 0.07 | 24 / 23 | 0.38 / 0.40 | ≈35.4 |

**Raw HCP:**

| trumps | 8 | 9 | 10 | 11 | 12+ |
|---|---|---|---|---|---|
| P\* not vul | 25.15 | 23.49 | 22.25 | 21.21 ± 0.03 | 20.29 ± 0.07 |
| P\* vul | 24.58 | 22.88 | 21.50 | 20.38 ± 0.02 | 19.45 ± 0.07 |
| gate not vul / vul | 26 / 25 | 24 / 23 | 23 / 22 | 22 / 21 | 21 / 20 |
| worth of this trump, not vul | (baseline) | 1.66 | 1.25 | 1.04 | 0.92 |

**Detail near the gate, both hands on support points** (5m vs 4m):

| trumps | points | P(5m makes) | IMPs not vul | IMPs vul |
|---|---|---|---|---|
| 8 | 27 | 33% | −0.58 | +0.07 |
| 8 | 28 | 48% | +0.76 | +2.15 |
| 9 | 26 | 37% | −0.35 | +0.48 |
| 9 | 27 | 52% | +1.06 | +2.65 |
| 10 | 25 | 31% | −0.86 | −0.30 ± 0.02 |
| 10 | 26 | 46% | +0.43 | +1.71 ± 0.02 |
| 11 | 25 | 35% | −0.57 ± 0.03 | +0.18 ± 0.04 |
| 11 | 26 | 49% | +0.72 ± 0.03 | +2.17 ± 0.04 |
| 12+ | 25 | 35% | −0.63 ± 0.08 | +0.10 ± 0.11 |
| 12+ | 26 | 50% | +0.76 ± 0.08 | +2.25 ± 0.12 |

**Detail near the gate, dummy-only support points** (5m vs 4m):

| trumps | points | P(5m makes) | IMPs not vul | IMPs vul |
|---|---|---|---|---|
| 8 | 26 | 32% | −0.69 | −0.09 |
| 8 | 27 | 47% | +0.69 | +2.06 |
| 9 | 25 | 38% | −0.21 | +0.69 |
| 9 | 26 | 53% | +1.19 | +2.86 |
| 10 | 24 | 36% | −0.40 | +0.39 ± 0.02 |
| 10 | 25 | 50% | +0.84 | +2.32 ± 0.02 |
| 11 | 23 | 31% | −0.75 ± 0.02 | −0.18 ± 0.04 |
| 11 | 24 | 43% | +0.22 ± 0.03 | +1.37 ± 0.04 |
| 12+ | 23 | 36% | −0.43 ± 0.08 | +0.35 ± 0.11 |
| 12+ | 24 | 48% | +0.68 ± 0.08 | +2.09 ± 0.12 |

## Result 2 — how much is each trump beyond eight worth?

Same rule and loss measure as the major study: "bid game when points +
k × (trumps − 8) ≥ t", best t for each k, loss in mIMP per case over fits of
8+ cards.

**Both hands on support points (`pons`):**

| k per extra trump | not vul: best t | not vul: loss | vul: best t | vul: loss |
|---|---|---|---|---|
| 0 (ignore length) | 27 | 22.4 | 27 | 23.9 |
| **0.5** | 27.5 | 2.7 | 26.5 | **0.0** |
| **1** | 28 | **1.1** | 27 | 4.5 |
| 1.5 | 28 | 9.8 | 27.5 | 10.3 |
| 2 | 29 | 32.8 | 28 | 27.6 |

**Dummy-only support points (classic):**

| k per extra trump | not vul: best t | not vul: loss | vul: best t | vul: loss |
|---|---|---|---|---|
| 0 (ignore length) | 26 | 30.0 | 26 | 40.1 |
| 0.5 | 26.5 | 5.6 | 25.5 | 5.6 |
| **1** | 27 | **0.1** | 26 | 3.3 |
| **1.5** | 27 | 4.3 | 26.5 | **2.5** |
| 2 | 27 | 23.3 | 27 | 14.3 |
| 2.5 | 28 | 37.4 | 27 | 34.2 |

**Raw HCP:**

| k per extra trump | not vul: best t | not vul: loss | vul: best t | vul: loss |
|---|---|---|---|---|
| 0 | 25 | 34.2 | 24 | 61.1 |
| 1 | 25 | 6.0 | 25 | 8.0 |
| **1.5** | 25.5 | **0.3** | 24.5 | **0.6** |
| 2 | 26 | 3.3 | 25 | 6.9 |
| 2.5 | 26 | 12.1 | 25 | 20.7 |

## Result 3 — 5m vs 3NT

Mean IMPs of 5m over 3NT, not vulnerable, both hands on support points. The
first number in each cell is 5m vs 4m, for orientation.

| trumps | 26 pts | 28 pts | 30 pts | 32 pts | over all cells where 5m beats 4m |
|---|---|---|---|---|---|
| 8 | −1.6 / **−0.7** | +0.8 / **−0.6** | +3.3 / **−0.5** | +4.8 / **−0.7** | **−0.6** |
| 9 | −0.3 / +1.5 | +2.5 / +1.8 | +4.7 / +1.5 | +5.6 / +0.6 | +1.4 |
| 10 | +0.4 / +3.2 | +3.3 / +3.5 | +5.1 / +2.8 | +5.8 / +1.5 | +2.9 |
| 11 | +0.7 / +4.1 | +3.6 / +4.5 | +5.3 / +3.8 | +5.8 / +2.4 | +3.8 |
| 12+ | +0.8 / +5.3 | +3.6 / +5.6 | +5.3 / +5.1 | +5.9 / +3.5 | +4.9 |

Vulnerable, the 5m-over-3NT numbers are 0–0.4 IMPs better for 8 trumps and
0.3–3 better for 9+. On raw HCP the picture is the same in sign: 3NT wins
with 8 trumps (−1.7 over the game-worthy cells), 9 trumps is about even
(+0.1), and 5m wins from 10 trumps up.

## Conclusions

1. **A minor game needs about three points more than a major game.** With
   eight trumps the break-even is 27.4 support points (both hands) against
   24.7 for a major; 25.2 HCP against 22.3. The gap is the same 2.7–2.8 points
   on all three scales and at both vulnerabilities. The textbook "26 for a
   major, 29 for a minor" has the right gap, but all of it sits too high on
   double-dummy.
2. **Extra trumps are worth less in a minor.** From the 9th to the 12th trump,
   not vulnerable:
   - raw HCP: 1.7, 1.3, 1.0, 0.9 (majors: 2.0, 1.7, 1.5, 1.3);
   - dummy-only support points: 1.4, 0.8, 0.6, 0.4 (majors: 1.6, 1.1, 0.9, 0.6);
   - both hands on support points: 1.2, 0.6, 0.2, 0.0 (majors: 1.4, 0.8, 0.4, 0.1).

   Eleven tricks need more than length: a minor game is short of high cards
   for the eleventh trick, not short of trumps.
3. **Rules of thumb for 8+ card fits:**
   - Both hands on support points: "28 with eight trumps, 1 less for each
     extra trump, but no lower than 26" not vul, which reproduces the true
     gates 28, 27, 26, 26, 26. Vulnerable, "26½ with eight trumps, ½ less for
     each extra trump" is exact (gates 27, 26, 26, 25, 25).
   - Dummy-only support points: "27 with eight trumps, 1 less for each extra
     trump" not vul; vul "26½, 1½ less per extra trump" is slightly better.
   - HCP: "25–26 HCP with eight trumps, 1½ less for each extra trump" (25½
     not vul, 24½ vul).
4. **Vulnerable thresholds are 0.45–0.65 points lower** at every trump length
   (up to 0.85 on raw HCP with 10+ trumps), as for the majors.
5. **"Points + trumps" break-even under the `pons` count is about 35.2–35.7
   for 8–10 trumps**, about 3 above the majors' 32.3–32.7, and rises to 36–37.5
   for 11+.
6. **With eight trumps, 3NT beats 5m on average; with nine or more, 5m beats
   3NT on average.** Over the cells where 5m is worth bidding, 5m loses 0.6
   IMPs to 3NT with 8 trumps and gains 1.4, 2.9, 3.8 and 4.9 with 9, 10, 11
   and 12+. This average is over all hands with the fit, including those where
   3NT has an unstopped suit, which a real auction would steer away from. It
   does not show that a stopped 9-card-fit hand should prefer 5m.

## Observation on the engine (not acted on)

The floor's 5m rule (`instinct.rs`, the minor loop before the major-fit rule)
bids 5m on `combined_points(25)` with a known eight-card fit, and only when a
suit the opponents bid is unstopped. Uncontested, 3NT is always the choice.
Two points of contact with this census, both leads rather than conclusions:

- The rule has no trump-length term, and its 25 is on the engine's plain point
  scale summed over shown minimums, which no table above measures directly.
- Result 3 says a 9+ card minor fit beats 3NT on average, and the floor never
  bids 5m uncontested. Whether that survives conditioning on stoppers is a
  separate census (stopper status per side suit) and then an A/B.

## Reproduce

[`examples/probe-game-threshold.rs`](../examples/probe-game-threshold.rs)
writes the per-cell table;
[`scripts/game-threshold-report.py`](../scripts/game-threshold-report.py)
prints every table above from it.

```sh
cargo build --release --example probe-game-threshold
RAYON_NUM_THREADS=8 nice -n 19 target/release/examples/probe-game-threshold --minor \
    /nfs2/jdh8/pons/22.pdd /nfs2/jdh8/pons/24.pdd /nfs2/jdh8/pons/shard-*.pdd \
    /mnt/hdd-data/jdh8/shards/shard-*.pdd > minor.tsv
python3 scripts/game-threshold-report.py minor.tsv --minor
```

## Caveats

The major study's caveats apply unchanged: double-dummy play with the better
declarer, undoubled and uncontested, and the `pons` row uses exact support
points for both hands. In addition, the 3NT comparison lets each strain pick
its own better declarer, so it can pair a 5m played from one side with a 3NT
played from the other.
