# Notrump slams: the best count, the threshold, and wasted honours — double-dummy census

Computed 2026-10-04 from the `pons` double-dummy deal bank. This is a plain
statistic over random deals, with no bidding simulation. It is the notrump
companion of [suit-slam.md](suit-slam.md) and the slam-level sequel of
[notrump-game-threshold.md](notrump-game-threshold.md).

## Question

1. Which count decides 6NT and 7NT best?
2. How many points does 6NT need, and 7NT?
3. What are the cards, a long suit and a flat hand worth in the slam zone?
4. Do wasted honours need an adjustment?

## Data and method

- **Deals:** 102,577,264 random deals, each with a complete double-dummy trick
  table. Every partnership (N–S and E–W) is one case, on its notrump tricks.

  | population | cases | take 12+ tricks |
  | --- | --- | --- |
  | both hands balanced, no 8-card major fit (the main one) | 31.77M | 1.81% |
  | both hands balanced | 49.39M | 1.88% |
  | no 8-card major fit | 104.87M | 2.55% |
  | all pairs | 205.15M | 2.76% |

  "Balanced" is (4333), (4432) or (5332).
- **Tricks:** double-dummy notrump tricks, taking the better of the two
  partners as declarer.
- **Decision value:** the IMPs won by the best rule "bid it when the value
  reaches t", against never bidding it. 6NT is compared with 3NT and 7NT with
  6NT, undoubled, with the same trick count and the opponents silent. The unit
  is mIMP per case (1000 mIMP = 1 IMP) of the population.
- **Control check.** 6NT is bid only if at most one ace is missing and no
  suit lacks both the ace and the king. 7NT is bid only with all four aces
  and no such suit.
- **Zone:** combined HCP in bands: under 28, 28 to 31 (where 6NT is decided),
  32 to 35, 36 and over.
- **Sample size:** slams are rare, so the intervals are wider than in the
  game studies. For the main population the 95% interval of a fitted term is
  ±0.7 percentage points of a chance in the 28–31 band and ±1.9 in the 32–35
  band. For "no 8-card major fit" it is ±0.2 and ±0.3.

### Counts

Each is summed over both hands.

- **Raw HCP:** 4-3-2-1.
- **Fifths:** A 4.0, K 2.8, Q 1.8, J 1.0, T 0.4.
- **BUM-RAP:** A 4.5, K 3.0, Q 1.5, J 0.75, T 0.25.
- **HCP + controls:** A 6, K 4, Q 2, J 1 (Zar's honours).
- **`point_count`** and **Zar**, as in [zar.md](zar.md).
- **HCP + length** and **Fifths + length:** 0.65 of a point when the pair has
  a suit of five or more cards, and 0.3 more for a second length point. A
  length point is a card beyond four in any suit of either hand.
- **Slam fit:** the weights of Result 3 in round numbers (`NOTRUMP_COUNTS`
  in the probe): A 4, K 2.7, Q 1.6, J 0.85, T 0.35, nine 0.15; length points
  0.65, 0.3, 0.4, 0.4; −0.2 for each 4-3-3-3 hand; −0.15 for a doubleton
  with a queen or jack, −0.25 for doubleton ace-king, −0.4 for a singleton
  king, queen or jack.

## Result 1 — the decision

mIMP per case with the best threshold in brackets. "Checked" bids only when
the control check passes.

**Both hands balanced, no 8-card major fit:**

| count | 6NT over 3NT, not vul | vul | checked, not vul | checked, vul | 7NT over 6NT, not vul | vul | checked, not vul | checked, vul |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| perfect knowledge of the tricks | 200 | 236 | 200 | 236 | 33 | 39 | 33 | 39 |
| raw HCP | 60 (31) | 71 (31) | 65 (31) | 77 (31) | 3 (35) | 4 (35) | 4 (35) | 5 (35) |
| Fifths | 65 (30½) | 77 (30½) | 68 (30½) | 80 (30½) | 4 (34) | 4 (34½) | 4 (34) | 5 (34) |
| BUM-RAP | 65 (31¼) | 76 (31¼) | 65 (31¼) | 77 (31¼) | 5 (35) | 5 (35) | 5 (34¾) | 5 (35) |
| HCP + controls | 62 (42) | 73 (42) | 62 (42) | 73 (42) | 4 (46) | 5 (46) | 5 (46) | 5 (46) |
| `point_count` | 60 (31) | 71 (31) | 65 (31) | 77 (31) | 3 (35) | 4 (35) | 4 (35) | 5 (35) |
| Zar | 59 (61) | 69 (61) | 60 (61) | 70 (61) | 4 (66) | 5 (66) | 4 (66) | 5 (66) |
| HCP + length | 66 (31¾) | 78 (31¾) | 69 (31¾) | 81 (31¾) | 3 (35¾) | 4 (35¾) | 5 (34¾) | 6 (34¾) |
| **Fifths + length** | 69 (30¾) | 81 (30¾) | 72 (30¾) | 85 (30¾) | 4 (34¼) | 5 (34¼) | 5 (33¾) | 6 (33¾) |
| slam fit | **71** (29¾) | **84** (29¾) | **73** (29½) | **86** (29½) | 5 (32¾) | 6 (32¾) | 5 (32¾) | 6 (32¾) |

**No 8-card major fit** (balanced or not):

| count | 6NT over 3NT, not vul | vul | checked, not vul | checked, vul | 7NT over 6NT, not vul | vul | checked, not vul | checked, vul |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| perfect knowledge of the tricks | 281 | 332 | 281 | 332 | 66 | 78 | 66 | 78 |
| raw HCP | 82 (31) | 96 (31) | 86 (31) | 102 (31) | 5 (34) | 6 (34) | 8 (34) | 9 (34) |
| Fifths | 87 (30) | 103 (30) | 93 (30) | 109 (30) | 6 (33½) | 7 (33½) | 9 (32½) | 10 (33) |
| BUM-RAP | 90 (30½) | 106 (30¾) | 92 (30½) | 109 (30½) | 8 (34) | 9 (34) | 9 (33¾) | 10 (33¾) |
| HCP + controls | 88 (41) | 104 (41) | 89 (41) | 105 (41) | 8 (45) | 9 (45) | 8 (45) | 10 (45) |
| `point_count` | 77 (32) | 91 (32) | 91 (31) | 107 (31) | 5 (35) | 6 (35) | 10 (34) | 11 (34) |
| Zar | 34 (66) | 38 (66) | 46 (65) | 52 (65) | 2 (72) | 2 (72) | 7 (68) | 7 (68) |
| HCP + length | 87 (31) | 102 (31) | 97 (31) | 114 (31) | 6 (35) | 7 (35) | 10 (34) | 12 (34) |
| Fifths + length | 92 (30¾) | 108 (30¾) | 99 (30¼) | 117 (30¼) | 7 (34) | 8 (34) | 10 (33¼) | 12 (33¼) |
| slam fit | **99** (29½) | **117** (29½) | **104** (29¼) | **122** (29¼) | **9** (32½) | **10** (32½) | **11** (32) | **13** (32) |

**The other two populations**, 6NT over 3NT not vulnerable, alone / checked:

| count | both hands balanced | all pairs |
| --- | --- | --- |
| perfect knowledge of the tricks | 206 | 304 |
| raw HCP | 61 (31) / 66 (31) | 83 (31) / 95 (30) |
| Fifths | 66 (30½) / 68 (30½) | 91 (30) / 99 (29½) |
| BUM-RAP | 66 (31¼) / 66 (31¼) | 97 (30½) / 99 (30½) |
| HCP + controls | 62 (42) / 62 (42) | 95 (40) / 98 (40) |
| Zar | 62 (61) / 63 (61) | 42 (66) / 58 (64) |
| Fifths + length | 70 (30¾) / 73 (30¾) | 96 (30¼) / 106 (30¼) |
| slam fit | **72** (29½) / **74** (29½) | **106** (29¼) / **112** (29¼) |

Ranking power (AUC):

| count | balanced, no major fit: 12+ among pairs that take 10+ | 12+ among 11+ | 13 among 12+ | no major fit: 12+ among 10+ | 12+ among 11+ | 13 among 12+ |
| --- | --- | --- | --- | --- | --- | --- |
| raw HCP | 0.883 | 0.797 | 0.765 | 0.846 | 0.762 | 0.693 |
| Fifths | 0.893 | 0.810 | 0.785 | 0.857 | 0.773 | 0.713 |
| BUM-RAP | 0.896 | 0.817 | 0.803 | 0.870 | 0.791 | 0.738 |
| HCP + controls | 0.892 | 0.812 | 0.798 | 0.867 | 0.788 | 0.736 |
| Zar | 0.888 | 0.812 | 0.808 | 0.831 | 0.761 | 0.749 |
| HCP + length | 0.891 | 0.807 | 0.777 | 0.859 | 0.775 | 0.707 |
| Fifths + length | 0.902 | 0.821 | 0.798 | 0.868 | 0.786 | 0.724 |
| slam fit | **0.906** | **0.829** | **0.813** | **0.883** | **0.805** | **0.755** |

## Result 2 — the odds at each value

| population | count | value | cases | 12+ tricks | 13 tricks | pass the check | 12+ tricks, checked |
| --- | --- | --- | --- | --- | --- | --- | --- |
| both balanced, no major fit | HCP | 29 | 487k | 16.0% | 1.0% | 82% | 19.4% |
| | | 30 | 323k | 31.2% | 2.7% | 89% | 34.9% |
| | | **31** | 203k | **51.7%** | 6.3% | 94% | 54.8% |
| | | 32 | 120k | 73.0% | 13.3% | 98% | 74.8% |
| | | 33 | 66k | 89.1% | 25.4% | 100% | 89.5% |
| | | 34 | 34k | 96.6% | 42.2% | 100% | 96.6% |
| | | **35** | 16k | 99.4% | **61.5%** | 100% | 99.4% |
| | | 36 | 7k | 99.9% | 81.4% | 100% | 99.9% |
| | | 37 | 3k | 100.0% | 94.5% | 100% | 100.0% |
| | Fifths | 30 | 162k | 40.5% | 3.9% | 94% | 43.1% |
| | | **30½** | 87k | **52.1%** | 5.3% | 96% | 54.5% |
| | | 31 | 105k | 64.1% | 9.9% | 98% | 65.7% |
| | | 32 | 60k | 83.8% | 20.7% | 99% | 84.4% |
| | | 33 | 29k | 95.1% | 36.3% | 100% | 95.2% |
| | | 34 | 12k | 98.9% | 56.0% | 100% | 98.9% |
| | | 35 | 5k | 99.9% | 78.6% | 100% | 99.9% |
| no major fit | HCP | 29 | 1564k | 26.5% | 4.0% | 82% | 32.5% |
| | | 30 | 1027k | 44.1% | 8.0% | 89% | 49.7% |
| | | **31** | 637k | **64.0%** | 14.8% | 94% | 68.1% |
| | | 32 | 373k | 81.9% | 25.0% | 98% | 83.9% |
| | | 33 | 205k | 93.4% | 39.7% | 99% | 93.8% |
| | | **34** | 103k | 98.3% | **56.7%** | 100% | 98.3% |
| | | 35 | 48k | 99.7% | 73.8% | 100% | 99.7% |
| | | 36 | 21k | 99.9% | 89.3% | 100% | 99.9% |

6NT over 3NT breaks even at 50%: it wins and loses 11 IMPs not vulnerable and
13 vulnerable. 7NT over 6NT breaks even at 56 to 57%: it wins 11 and loses
14 not vulnerable, and wins 13 and loses 17 vulnerable.

## Result 3 — what the cards are worth in the slam zone

One regression per column, every term of this table and of Result 4 fitted
together. Each number is in points on a scale where an ace is 4. The bottom
row gives the ace itself.

**Both hands balanced, no 8-card major fit:**

| term | Fifths | tricks, all hands | tricks, 28–31 | chance of 12, 28–31 | tricks, 32–35 | chance of 12, 32–35 |
| --- | --- | --- | --- | --- | --- | --- |
| king | 2.8 | 2.82 | 2.73 | 2.72 | 2.39 | 2.81 |
| queen | 1.8 | 1.77 | 1.65 | 1.62 | 1.33 | 1.59 |
| jack | 1.0 | 0.97 | 0.86 | 0.88 | 0.64 | 0.66 |
| ten | 0.4 | 0.42 | 0.39 | 0.35 | 0.31 | 0.43 |
| nine | 0 | 0.16 | 0.15 | 0.13 | 0.12 | 0.17 |
| 1st length point | — | 0.14 | 0.60 | **0.66** | 0.63 | 0.70 |
| 2nd length point | — | −0.08 | 0.28 | 0.28 | 0.11 | −0.01 |
| each 4-3-3-3 hand | — | −0.09 | −0.12 | −0.19 | −0.39 | −0.57 |
| **an ace (= 4)** | 4 | 2.09 tricks | 1.96 tricks | +60.9% | 1.32 tricks | +47.5% |

**No 8-card major fit:**

| term | Fifths | tricks, all hands | tricks, 28–31 | chance of 12, 28–31 | tricks, 32–35 | chance of 12, 32–35 |
| --- | --- | --- | --- | --- | --- | --- |
| king | 2.8 | 2.65 | 2.53 | 2.60 | 2.00 | 2.68 |
| queen | 1.8 | 1.59 | 1.45 | 1.46 | 1.05 | 1.40 |
| jack | 1.0 | 0.87 | 0.74 | 0.75 | 0.49 | 0.55 |
| ten | 0.4 | 0.38 | 0.31 | 0.27 | 0.23 | 0.38 |
| nine | 0 | 0.15 | 0.11 | 0.10 | 0.08 | 0.15 |
| 1st length point | — | 0.13 | 0.54 | 0.48 | 0.56 | 0.94 |
| 2nd length point | — | 0.01 | 0.33 | 0.40 | 0.26 | 0.16 |
| 3rd length point | — | −0.08 | 0.24 | 0.42 | 0.26 | 0.17 |
| 4th length point | — | −0.27 | 0.08 | 0.39 | 0.19 | 0.08 |
| each 4-3-3-3 hand | — | 0.00 | 0.09 | 0.04 | −0.14 | −0.36 |
| longest combined suit has 8+ cards | — | −0.33 | 0.01 | 0.29 | 0.14 | 0.14 |
| longest combined suit has 9+ cards | — | −0.12 | 0.11 | 0.36 | 0.23 | 0.34 |
| **an ace (= 4)** | 4 | 2.20 tricks | 2.14 tricks | +72.7% | 1.38 tricks | +34.9% |

## Result 4 — wasted honours

The same regressions. Each number is what the holding loses, in points where
an ace is 4.

| holding | population | tricks, all hands | tricks, 28–31 | chance of 12, 28–31 | chance of 12, 32–35 |
| --- | --- | --- | --- | --- | --- |
| doubleton Qx, Jx or QJ | balanced, no major fit | −0.26 | −0.20 | −0.16 | −0.15 |
| doubleton AQ, AJ, KQ or KJ | balanced, no major fit | −0.28 | −0.14 | −0.13 | −0.14 |
| doubleton AK | balanced, no major fit | −0.35 | −0.29 | −0.24 | −0.33 |
| doubleton Qx, Jx or QJ | no major fit | −0.24 | −0.17 | −0.10 | −0.02 |
| doubleton AQ, AJ, KQ or KJ | no major fit | −0.18 | −0.07 | −0.10 | −0.08 |
| doubleton AK | no major fit | −0.34 | −0.29 | −0.28 | −0.39 |
| singleton king | no major fit | −0.90 | −0.66 | −0.37 | −0.53 |
| singleton queen | no major fit | −0.65 | −0.66 | −0.47 | −0.46 |
| singleton jack | no major fit | −0.41 | −0.46 | −0.38 | −0.13 |
| king facing partner's void or singleton | no major fit | +0.21 | +0.12 | −0.01 | −0.07 |
| queen facing partner's void or singleton | no major fit | +0.23 | +0.02 | −0.14 | −0.15 |
| jack facing partner's void or singleton | no major fit | +0.15 | −0.10 | −0.30 | −0.05 |

## Conclusions

### The threshold

1. **On actual holdings 6NT wants 31 HCP, not 33.** With two balanced hands
   and no major fit, 31 HCP take 12 tricks 51.7% of the time, 32 take them
   73.0% and 33 take them 89.1%. The best threshold is 31 on HCP, 30½ on
   Fifths and 31¼ on BUM-RAP. Without the balanced condition 31 HCP make
   64.0% and 30 make 44.1%.
2. **7NT wants 35 HCP with two balanced hands** (61.5% for 13 tricks, against
   the 56 to 57% needed). Without that condition 34 is the break-even
   (56.7%). At 37 it is 94.5%.
3. **These are totals of actual hands, double-dummy.** A bidding gate adds
   the minimum partner has shown and sees less than the total; the floor's
   milestones are 33 and 37 on that sum. A declarer who has to guess needs a
   little more than these numbers.

### The count

4. **Fifths is the best standard count for 6NT, as it is for 3NT.** With two
   balanced hands it wins 65 mIMP per case against 60 for raw HCP, out of
   200 available. BUM-RAP wins the same 65 and ranks a little better (AUC
   0.896 against 0.893 for the 12-trick pairs among the 10-trick ones).
   Without the balanced condition BUM-RAP is ahead: 90 against 87 for Fifths
   and 82 for HCP.
5. **The honours do not steepen at notrump the way they do in a suit.** In
   the 28–31 band the cards are worth 4, 2.7, 1.6, 0.9 and 0.35 (ace to ten),
   which is Fifths. In a suit slam the king and queen fall to 2.1 and 0.85
   ([suit-slam.md](suit-slam.md)). At notrump no ruff replaces a queen, so
   the lower honours keep their value. Only in the 32–35 band do the tricks
   lean towards the top cards (2.4, 1.3, 0.6).
6. **Controls add little.** HCP + controls wins 62 against 60 for HCP with
   two balanced hands, less than Fifths. Zar is the worst count once a hand
   is unbalanced (34 against 82 for HCP).
7. **A long suit is worth twice as much at slam as at game.** The first
   length point is worth 0.66 of a point in the 28–31 band and the second
   0.28, against 0.38 and 0.16 HCP at the 3NT decision
   ([notrump-game-threshold.md](notrump-game-threshold.md)). "Add a point for
   a five-card suit" still overpays by half. Adding 0.65 and 0.3 lifts HCP
   from 60 to 66 and Fifths from 65 to 69.
8. **A 4-3-3-3 hand costs 0.2 of a point in the 28–31 band and 0.6 in the
   32–35 band.** With no long suit the twelfth trick has to come from high
   cards.
9. **The fitted count wins 71, against 69 for Fifths + length.** The nine,
   the flat hand and the waste terms together add 2 mIMP per case. Fifths +
   length is the count to remember; the best threshold is 30¾.
10. **No count decides 7NT**: 3 to 5 of the 33 mIMP per case available with
    two balanced hands, and 9 of 66 without that condition.

### The control check

11. **At notrump the check adds little.** It lifts HCP from 60 to 65 and
    BUM-RAP not at all. 94% of the 31-point pairs pass it, and with 33 HCP
    two aces cannot be missing. In a suit slam the same check was worth 94
    mIMP per case, because shortness lets a pair reach the slam zone on far
    fewer high cards.

### Wasted honours

12. **No adjustment is needed at notrump.** With two balanced hands the only
    waste is a doubleton honour: 0.16 of a point for a queen or jack, 0.13
    beside an ace or king, and 0.24 for doubleton ace-king, which blocks.
    Together with the nine and the flat hand they are worth the 2 mIMP per
    case of conclusion 9.
13. **A singleton honour loses about 0.4 of a point** (unbalanced hands
    only): 0.37 for the king, 0.47 for the queen, 0.38 for the jack.
14. **An honour facing partner's shortness loses nothing**: 0.01 for a king
    and 0.14 for a queen. A jack loses 0.30. This is the opposite of a suit
    slam, where a king facing a void keeps a quarter of its value. At notrump
    it is a stopper.

### For `pons`

15. **Nothing changes in `pons` yet.** Two candidates, each needing its own
    A/B: gauge the 6NT milestone and the quantitative raise on Fifths +
    length instead of HCP, and test a lower threshold. The census agrees with
    the engine note in [next-steps.md](next-steps.md) that double-dummy 6NT
    pays from about 31 combined HCP.

## Reproduce

[`examples/probe-slam.rs`](../examples/probe-slam.rs) writes the cell table;
[`scripts/slam-report.py`](../scripts/slam-report.py) prints every table
above, after those of [suit-slam.md](suit-slam.md).

```sh
cargo build --release --example probe-slam
RAYON_NUM_THREADS=8 nice -n 19 target/release/examples/probe-slam \
    /nfs2/jdh8/pons/22.pdd /nfs2/jdh8/pons/24.pdd /nfs2/jdh8/pons/shard-*.pdd \
    /mnt/hdd-data/jdh8/shards/shard-*.pdd > slam.tsv
python3 scripts/slam-report.py slam.tsv
```

## Caveats

- Double-dummy play with the better declarer: no wrong-siding, a perfect
  opening lead, and no single-dummy guessing. At slam level the guessing
  matters more than the lead, so real thresholds sit a little higher.
- Undoubled and uncontested, on the actual combined holdings.
- 6NT is compared only with 3NT. A suit slam, or 6NT against a making suit
  slam, is not priced; with a fit the suit slam is usually the alternative.
- The bands are cut on raw HCP. Inside a band the fit is a local one.
- Slams are rare: the 32–35 band of the main population holds 0.2M cases,
  and its terms carry ±1.9 percentage points.
- The length terms do not say where the long suit is, or how good it is.
- The fitted count was read off the same deals it is scored on.
