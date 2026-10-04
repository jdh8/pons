# Zar points vs. point counts, and the wasted values Zar misses — double-dummy census

Computed 2026-10-04 from the `pons` double-dummy deal bank. This is a plain
statistic over random deals, with no bidding simulation. It is the companion
of [nltc.md](nltc.md), which has Zar as one column.

## Question

Zar points (Zar Petkov) replace the point count with one number per hand:
open on 26, bid game on 52, slam on 62. The study asks:

1. When does Zar judge a hand better than a point count? With a fit or
   without one? With which shapes? At game or at slam? For the opening bid?
2. Which part of Zar does the work, its honour scale or its distribution
   count?
3. Does Zar need to adjust for wasted values, and only in suit contracts?
   Two kinds are tested: an honour in one's own short suit, and an honour
   facing partner's shortness.
4. Do Zar's own numbers hold: 26, 52, 62 and 67, three points for an extra
   trump, a point for an honour in partner's suit?

## Data and method

- **Deals:** 102,577,264 random deals, each with a complete double-dummy trick
  table.
- **Fit cases:** every partnership (N–S and E–W) whose longest combined suit
  has 8+ cards, with that suit as trumps. Between suits of equal length, the
  one that takes more tricks. 172.86M cases, 90.86M of them with a major as
  trumps.
- **Notrump cases:** every partnership, on its notrump tricks. 205.15M cases,
  32.29M of them with no 8-card fit.
- **Hands:** every single hand, 410.31M, for the opening bid (Result 6).
- **Tricks:** double-dummy, taking the better of the two partners as declarer.
  This is slightly optimistic: it assumes the hand is always right-sided.
- **Holdings:** the actual 26 cards of the partnership, both hands summed on
  every scale, except in Result 6.
- **Sample size:** the 95% interval of every fitted term is ±0.004 tricks or
  less over all fit cases and at notrump, ±0.01 with nine trumps and ±0.03
  (0.16 Zar points) with ten or more, so none is shown.

### Scales

- **Zar.** HCP, plus controls (ace 2, king 1), plus the two longest suits
  (a + b), plus the longest suit less the shortest (a − d). The honours come
  to ace 6, king 4, queen 2, jack 1. A flat 4-3-3-3 has 8 distribution points
  and a 5-4-3-1 has 13.
- **Zar's discount.** One point off for a singleton king, queen or jack, and
  for a doubleton that holds a queen or a jack. This is the rule in
  `eval::zar`. "Zar, no discount" leaves it out.
- **Zar's fit points.** Petkov's paper adds 3 points for each trump over the
  promised length. Wikipedia's version adds 2 for each trump over eight with a
  void and 1 with a singleton. Both are tested on the trumps over eight; the
  second takes the shortest suit of either hand.
- **Raw HCP:** 4-3-2-1.
- **`point_count`:** HCP plus the `pons` shape upgrade of 0–2 (+1 unbalanced,
  +1 if the two longest suits total 10+ cards, −1 per wasted short honour).
- **Support points (the `pons` count).** Trump-suit HCP, and in each side suit
  max(HCP, shortness, HCP + shortness − 1) with void = 3, singleton = 2 and
  doubleton = 1. Add 1 if the two longest suits total 10+ cards. "Support
  points + trumps" is the `pons` game gate.
- **Fifths** (notrump only): A 4.0, K 2.8, Q 1.8, J 1.0, T 0.4.
- **HCP + two longest suits** (one hand only): the Rule of 20 opens on 20.

Sources for Zar: Zar Petkov, *Zar Points – Aggressive Bidding Hand
Evaluation*, and [Wikipedia](https://en.wikipedia.org/wiki/Zar_Points).

### Measures

- **Fit:** the line tricks = a + b × value, and the standard deviation (sd) of
  its misses in tricks.
- **Mean miss:** actual tricks minus the tricks that line predicts. A negative
  miss means the scale overvalues those hands.
- **Ranking power (AUC):** the chance that a random pair which takes N+ tricks
  rates higher than a random pair which does not. 0.5 is a coin flip, 1 is
  perfect.
- **Decision value:** the IMPs won by the best rule of the form "bid it when
  the value reaches t", against never bidding it. 4M is compared with 3M, 6M
  with 4M, 7M with 6M and 3NT with 2NT, undoubled, with the same trick count
  and the opponents silent. The unit is mIMP per case (1000 mIMP = 1 IMP).
- **A term in Zar points:** its trick value divided by the trick value of one
  Zar point in the same fit.

## Result 1 — with a fit

All fit cases:

| scale | tricks per point (b) | a | sd | R² |
| --- | --- | --- | --- | --- |
| raw HCP | +0.347 | 1.67 | 1.190 | 0.657 |
| `point_count` | +0.363 | 1.01 | 1.070 | 0.723 |
| support points | +0.363 | 0.45 | 1.004 | 0.756 |
| **support points + trumps** | +0.360 | −2.57 | **0.945** | 0.784 |
| Zar, no discount | +0.238 | −3.23 | 1.020 | 0.748 |
| Zar | +0.240 | −3.20 | 1.005 | 0.755 |
| Zar + 3 per trump over eight | +0.223 | −2.76 | 0.983 | 0.766 |
| Zar + 2 or 1 per trump over eight (void, singleton) | +0.235 | −3.05 | 0.975 | 0.770 |

By the shape of the pair and by trump length. Each cell is the sd of a line
fitted inside that row, with the mean miss of the all-cases line in brackets:

| row | cases | raw HCP | `point_count` | support points | support points + trumps | Zar | Zar + 2 or 1 per trump |
| --- | --- | --- | --- | --- | --- | --- | --- |
| both hands balanced | 33.40M | 0.905 [−0.91] | 0.905 [−0.56] | 0.879 [−0.28] | 0.861 [−0.15] | **0.851** [−0.02] | **0.851** [+0.08] |
| unbalanced, no singleton | 33.65M | 0.915 [−0.33] | 0.901 [−0.17] | 0.892 [+0.11] | **0.857** [+0.11] | 0.885 [+0.02] | 0.885 [+0.14] |
| a singleton, no void | 86.99M | 1.082 [+0.25] | 1.032 [+0.15] | 1.010 [+0.04] | **0.950** [+0.01] | 1.026 [−0.01] | 0.982 [−0.04] |
| a void | 18.82M | 1.231 [+1.06] | 1.228 [+0.63] | 1.211 [+0.12] | **1.120** [+0.03] | 1.259 [+0.03] | 1.157 [−0.22] |
| a 7+ card suit | 15.61M | 1.122 [+0.78] | 1.111 [+0.45] | 1.063 [+0.13] | **0.985** [−0.09] | 1.122 [−0.33] | 1.033 [−0.52] |
| 8 trumps | 93.84M | 1.021 [−0.47] | **0.938** [−0.41] | 0.943 [−0.32] | 0.943 [−0.10] | 0.999 [−0.28] | 0.999 [−0.16] |
| 9 trumps | 57.65M | 1.073 [+0.39] | 0.966 [+0.35] | 0.920 [+0.29] | 0.920 [+0.15] | **0.908** [+0.27] | 0.910 [+0.21] |
| 10 trumps | 17.79M | 1.080 [+0.93] | 0.965 [+0.79] | 0.882 [+0.59] | 0.882 [+0.09] | 0.836 [+0.49] | **0.818** [+0.18] |
| 11+ trumps | 3.58M | 1.072 [+1.31] | 0.967 [+1.07] | 0.859 [+0.69] | 0.859 [−0.16] | 0.810 [+0.50] | **0.769** [−0.13] |

The first four rows split all cases by the shortest suit in either hand.
"Balanced" is (4333), (4432) or (5332). The 7+ row overlaps them. Inside a
trump-length row the trump term is a constant, so support points and support
points + trumps have the same sd. Major and minor fits read the same (Zar
1.005 and 1.006).

Ranking power:

| question | raw HCP | `point_count` | support points | support points + trumps | Zar | Zar + 2 or 1 per trump |
| --- | --- | --- | --- | --- | --- | --- |
| 10+ tricks | 0.896 | 0.920 | 0.933 | **0.941** | 0.934 | 0.939 |
| 12+ tricks | 0.922 | 0.942 | 0.953 | 0.957 | 0.958 | **0.961** |
| 12+ tricks, among pairs that take 10+ | 0.804 | 0.841 | 0.864 | 0.871 | 0.878 | **0.884** |
| 13 tricks, among pairs that take 12+ | 0.694 | 0.723 | 0.741 | 0.743 | 0.774 | **0.776** |
| 10+ tricks, both hands balanced | 0.956 | 0.956 | 0.959 | 0.959 | **0.963** | **0.963** |
| 10+ tricks, unbalanced, no singleton | 0.949 | 0.951 | 0.951 | 0.952 | **0.954** | **0.954** |
| 10+ tricks, a singleton | 0.909 | 0.919 | 0.924 | **0.933** | 0.922 | 0.929 |
| 10+ tricks, a void | 0.876 | 0.876 | 0.882 | **0.903** | 0.869 | 0.894 |
| 12+ tricks, both hands balanced | 0.981 | 0.981 | 0.981 | 0.981 | **0.986** | **0.986** |
| 12+ tricks, unbalanced, no singleton | 0.973 | 0.974 | 0.973 | 0.972 | **0.979** | **0.979** |
| 12+ tricks, a singleton | 0.943 | 0.949 | 0.950 | 0.952 | 0.957 | **0.958** |
| 12+ tricks, a void | 0.885 | 0.888 | 0.894 | **0.908** | 0.889 | 0.904 |

Decision value in mIMP per case, major fits, with the best threshold in
brackets:

| scale | 4M over 3M, not vul | 4M over 3M, vul | 6M over 4M, not vul | 6M over 4M, vul | 7M over 6M, not vul | 7M over 6M, vul |
| --- | --- | --- | --- | --- | --- | --- |
| perfect knowledge of the tricks | 2100 | 3501 | 803 | 949 | 174 | 205 |
| raw HCP | 1219 (22) | 2228 (21) | 169 (29) | 199 (29) | 8 (33) | 9 (33) |
| `point_count` | 1323 (23) | 2377 (22) | 207 (30) | 245 (30) | 11 (34) | 13 (34) |
| support points | 1382 (24) | 2503 (24) | 219 (31) | 258 (31) | 8 (36) | 9 (36) |
| **support points + trumps** | **1431** (33) | **2551** (33) | 226 (40) | 266 (40) | 8 (45) | 9 (45) |
| Zar, no discount | 1379 (52) | 2487 (52) | 216 (62) | 254 (62) | 9 (69) | 10 (70) |
| Zar | 1396 (52) | 2511 (51) | 229 (62) | 270 (62) | 10 (68) | 11 (69) |
| Zar + 3 per trump over eight | 1384 (54) | 2504 (53) | 190 (65) | 224 (65) | 6 (74) | 7 (74) |
| Zar + 2 or 1 per trump over eight | 1422 (52) | 2549 (52) | **232** (63) | **273** (63) | 10 (70) | 11 (70) |

The same on Zar's own thresholds, 52 for game, 62 for slam and 67 for a grand
slam:

| scale | 4M at 52, not vul | 4M at 52, vul | 6M at 62, not vul | 6M at 62, vul | 7M at 67, not vul | 7M at 67, vul |
| --- | --- | --- | --- | --- | --- | --- |
| Zar | 1396 | 2481 | 229 | 270 | 7 | 6 |
| Zar + 3 per trump over eight | 1341 | 2487 | 80 | 91 | −81 | −105 |
| Zar + 2 or 1 per trump over eight | 1422 | 2549 | 231 | 271 | −10 | −16 |

The game and slam decisions by shape, major fits, not vulnerable, in mIMP per
case of that shape (4M over 3M / 6M over 4M):

| shape | raw HCP | `point_count` | support points | support points + trumps | Zar | Zar + 2 or 1 per trump |
| --- | --- | --- | --- | --- | --- | --- |
| both hands balanced | 695 / 77 | 695 / 77 | 715 / 78 | 709 / 71 | **732 / 97** | **732 / 97** |
| unbalanced, no singleton | 1021 / 140 | 1037 / 151 | 1021 / 136 | 1035 / 134 | **1047 / 169** | **1047 / 169** |
| a singleton, no void | 1490 / 226 | 1536 / 245 | 1566 / 238 | **1615** / 242 | 1563 / 261 | 1589 / **270** |
| a void | 2410 / 518 | 2421 / 520 | 2463 / 525 | **2538 / 607** | 2443 / 481 | 2516 / 574 |

## Result 2 — at notrump

Notrump tricks, both hands summed. The decision is 3NT over 2NT, not
vulnerable / vulnerable, with the best threshold in brackets:

| population | scale | tricks per point | sd | R² | AUC 9+ tricks | 3NT over 2NT, mIMP per case |
| --- | --- | --- | --- | --- | --- | --- |
| no 8-card fit (32.29M) | raw HCP | +0.483 | 0.949 | 0.861 | 0.968 | 814 (24) / 1469 (24) |
| | **Fifths** | +0.506 | **0.920** | 0.870 | 0.971 | **834** (24) / **1484** (23½) |
| | `point_count` | +0.474 | 1.021 | 0.839 | 0.962 | 793 (25) / 1410 (25) |
| | Zar, no discount | +0.269 | 1.469 | 0.668 | 0.914 | 594 (54) / 1127 (53) |
| | Zar | +0.273 | 1.451 | 0.676 | 0.916 | 603 (53) / 1140 (52) |
| no 8-card fit, both hands balanced (16.00M) | raw HCP | +0.482 | 0.929 | 0.866 | 0.968 | 802 (24) / 1453 (24) |
| | Fifths | +0.504 | **0.902** | 0.874 | 0.971 | **824** (24) / **1467** (23½) |
| | Zar | +0.322 | 1.112 | 0.808 | 0.953 | 729 (51) / 1329 (50) |
| no 8-card fit, unbalanced, no singleton (4.10M) | raw HCP | +0.494 | 0.977 | 0.859 | 0.967 | 845 (24) / 1513 (24) |
| | Zar | +0.343 | 1.085 | 0.826 | 0.958 | 785 (54) / 1424 (53) |
| no 8-card fit, a singleton (10.81M) | raw HCP | +0.483 | 0.966 | 0.857 | 0.968 | 821 (24) / 1479 (24) |
| | Zar | +0.305 | 1.291 | 0.744 | 0.937 | 687 (56) / 1262 (55) |
| no 8-card major fit (104.87M) | raw HCP | +0.506 | 1.239 | 0.793 | 0.952 | 724 (24) / 1316 (24) |
| | Fifths | +0.530 | **1.208** | 0.804 | 0.955 | **736** (24) / **1331** (23½) |
| | Zar | +0.291 | 1.640 | 0.638 | 0.912 | 570 (54) / 1076 (53) |

The HCP and Fifths rows agree with
[notrump-game-threshold.md](notrump-game-threshold.md).

## Result 3 — the parts of Zar

One regression of tricks per model. In the fit columns each trump length (8,
9, 10+) has its own constant, so the sds are lower than in Result 1.

| model | fit, all | 8 trumps | 9 trumps | 10+ trumps | notrump, no 8-card fit |
| --- | --- | --- | --- | --- | --- |
| a constant only | 1.959 | 2.081 | 1.858 | 1.655 | 2.548 |
| raw HCP | 1.061 | 1.021 | 1.073 | 1.090 | 0.949 |
| Zar, no discount | 0.972 | 1.015 | 0.926 | 0.856 | 1.469 |
| Zar's four parts, each with its own weight | 0.920 | 0.912 | 0.896 | 0.828 | 0.948 |
| every honour (A K Q J 10) and suit length (a, b, d) with its own weight | 0.892 | 0.888 | 0.859 | 0.793 | 0.910 |
| + own short side honours (Result 4) | 0.880 | 0.877 | 0.848 | 0.782 | 0.904 |
| + honours facing partner's shortness (Result 4) | 0.849 | 0.853 | 0.812 | 0.742 | 0.903 |
| + trump terms (Result 4) | 0.842 | 0.840 | 0.808 | 0.735 | — |

Zar's four parts with free weights. Zar gives each of them 1:

| population | one Zar point, tricks | an HCP | a control | a card of a + b | a card of a − d |
| --- | --- | --- | --- | --- | --- |
| fit, all | 0.232 | 0.97 | 1.34 | 1.02 | 0.34 |
| 8 trumps | 0.242 | 1.13 | 1.07 | 0.75 | 0.31 |
| 9 trumps | 0.223 | 0.82 | 1.61 | 1.28 | 0.37 |
| 10+ trumps | 0.206 | 0.55 | 2.01 | 1.56 | 0.80 |
| notrump, no 8-card fit | 0.269 | 1.72 | 0.19 | 0.14 | −0.10 |

Every card with a free weight, in Zar points, fitted together with the waste
and trump terms of Result 4:

| card | Zar's price | fit, all | 8 trumps | 9 trumps | 10+ trumps | notrump, no 8-card fit |
| --- | --- | --- | --- | --- | --- | --- |
| ace | 6 | 6.80 | 6.81 | 6.82 | 6.74 | 7.43 |
| king | 4 | 4.45 | 4.55 | 4.34 | 4.15 | 5.47 |
| queen | 2 | 2.23 | 2.39 | 2.04 | 1.70 | 3.58 |
| jack | 1 | 1.07 | 1.19 | 0.94 | 0.66 | 2.03 |
| ten | 0 | 0.36 | 0.43 | 0.31 | 0.22 | 0.94 |
| a card in the longest suit (a) | 2 | 1.08 | 0.83 | 1.25 | 1.82 | −0.31 |
| a card in the second suit (b) | 1 | 1.11 | 0.86 | 1.31 | 1.51 | +0.21 |
| a card in the shortest suit (d) | −1 | −1.83 | −1.67 | −2.07 | −2.28 | −0.41 |

In tricks, with a fit: ace 1.58, king 1.03, queen 0.52, jack 0.25, ten 0.09.
At notrump: 2.00, 1.47, 0.96, 0.55, 0.25.

## Result 4 — wasted values

**Side suits.** "Zar owes" is the whole deduction from Zar with no discount,
fitted with Zar as one variable, so it is the number to compare with Zar's
discount of 1. The last two columns are in tricks, from the regression of
Result 3 where every card has its own weight.

| holding | Zar's discount | Zar owes: fit, all | 8 trumps | 9 trumps | 10+ trumps | tricks, with a fit | tricks, at notrump |
| --- | --- | --- | --- | --- | --- | --- | --- |
| singleton king | 1 | **2.49** | 2.44 | 2.49 | 2.55 | −0.64 | −0.43 |
| singleton queen | 1 | 1.10 | 1.18 | 1.02 | 1.01 | −0.27 | −0.32 |
| singleton jack | 1 | 0.39 | 0.56 | 0.26 | 0.20 | −0.08 | −0.19 |
| doubleton Qx, Jx or QJ | 1 | 1.07 | 0.96 | 1.16 | 1.42 | −0.20 | −0.08 |
| doubleton AQ, AJ, KQ or KJ | 1 | 1.01 | 0.94 | 1.04 | 1.18 | −0.22 | −0.19 |
| ace facing partner's void | 0 | **2.66** | 2.68 | 2.48 | 2.27 | −0.73 | −0.13 |
| king facing partner's void | 0 | **1.95** | 2.27 | 1.72 | 1.10 | −0.51 | −0.12 |
| queen facing partner's void | 0 | 0.51 | 1.11 | 0.15 | −0.74 | −0.13 | −0.02 |
| jack facing partner's void | 0 | −0.16 | 0.58 | −0.58 | −1.59 | +0.06 | +0.03 |
| ace facing partner's singleton | 0 | 0.09 | 0.29 | −0.16 | −0.56 | −0.05 | +0.02 |
| king facing partner's singleton | 0 | **1.99** | 1.94 | 2.00 | 2.00 | −0.49 | −0.04 |
| queen facing partner's singleton | 0 | 0.76 | 0.95 | 0.61 | 0.30 | −0.17 | 0.00 |
| jack facing partner's singleton | 0 | 0.25 | 0.58 | 0.01 | −0.44 | −0.03 | +0.02 |

With these terms the sd of the Zar fit falls from 0.972 to 0.914 (one
constant per trump length). At notrump Zar has no meaningful "owes" column:
its shape points are wrong there (Result 3), and a fit with Zar as one
variable charges that error to the shortness terms.

**The trump suit.** The same fit with the trump terms added (sd 0.871):

| term | Zar owes: fit, all | 8 trumps | 9 trumps | 10+ trumps |
| --- | --- | --- | --- | --- |
| a short trump honour that Zar discounts | −0.04 | 0.10 | −0.10 | −0.16 |
| the shorter trump holding is a doubleton | 2.51 | 2.41 | 2.76 | 2.18 |
| the shorter trump holding is a singleton | 4.58 | 4.51 | 4.76 | 4.00 |
| the shorter trump holding is a void | 6.25 | 6.16 | 6.37 | 5.48 |
| trump ace | −0.31 | −0.47 | −0.15 | 0.35 |
| trump king | −0.29 | −0.67 | 0.08 | 1.16 |
| trump queen | −0.54 | −1.00 | −0.03 | 0.88 |
| trump jack | −0.41 | −0.75 | −0.03 | 0.59 |
| trump ten | −0.68 | −0.86 | −0.46 | −0.18 |
| 9 trumps (against 8) | −1.81 | | | |
| 10+ trumps (against 8) | −2.36 | | | |

A negative number is a bonus. Without the trump terms, the 9th trump is worth
2.34 Zar points and 10+ trumps 3.32.

## Result 5 — Zar repaired, and Zar's honours on the `pons` count

Three counts built from Results 3 and 4, in round numbers with nothing
fitted:

- **Zar, waste repaired.** Zar with its discount, and in the side suits: a
  singleton king loses 2½ instead of 1; facing partner's void an ace loses
  2½, a king 2 and a queen ½; facing partner's singleton a king loses 2 and
  a queen 1.
- **Zar, waste and trump terms.** The same, and: +2 for 9 trumps, +2½ for
  10+; −2½, −4½ or −6 when the shorter trump holding is a doubleton,
  singleton or void; no discount for a short trump honour; with exactly eight
  trumps, +1 for each of the trump queen, jack and ten.
- **Support points + controls.** The `pons` count plus Zar's controls (ace 2,
  king 1), with nothing else from Zar. It needs no knowledge of partner's
  shortness.

| count | tricks per point | sd | mean miss, a void | mean miss, a 7+ card suit | AUC 10+ | AUC 12+ | AUC 12+ among 10+ | AUC 13 among 12+ |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Zar | +0.240 | 1.005 | +0.03 | −0.33 | 0.934 | 0.958 | 0.878 | 0.774 |
| Zar, waste repaired | +0.257 | 0.968 | +0.28 | −0.23 | 0.938 | 0.964 | 0.891 | 0.803 |
| Zar, waste and trump terms | +0.263 | 0.905 | +0.31 | +0.09 | 0.947 | **0.969** | **0.903** | **0.812** |
| support points + trumps | +0.360 | 0.945 | +0.03 | −0.09 | 0.941 | 0.957 | 0.871 | 0.743 |
| support points + controls | +0.265 | 0.983 | +0.37 | +0.30 | 0.937 | 0.958 | 0.877 | 0.770 |
| support points + controls + trumps | +0.266 | 0.920 | +0.30 | +0.13 | 0.945 | 0.963 | 0.888 | 0.777 |
| support points + controls + 1½ per trump | +0.264 | **0.904** | +0.27 | +0.06 | **0.948** | 0.964 | 0.890 | 0.777 |

Decision value in mIMP per case, major fits, with the best threshold in
brackets:

| count | 4M over 3M, not vul | 4M over 3M, vul | 6M over 4M, not vul | 6M over 4M, vul | 7M over 6M, not vul | 7M over 6M, vul |
| --- | --- | --- | --- | --- | --- | --- |
| Zar | 1396 (52) | 2511 (51) | 229 (62) | 270 (62) | 10 (68) | 11 (69) |
| Zar, waste repaired | 1413 (50½) | 2525 (49½) | 276 (59½) | 325 (59½) | 23 (64½) | 26 (65½) |
| Zar, waste and trump terms | 1453 (52½) | 2584 (51½) | **308** (61½) | **364** (61½) | **30** (66½) | **35** (66½) |
| support points + trumps | 1431 (33) | 2551 (33) | 226 (40) | 266 (40) | 8 (45) | 9 (45) |
| support points + controls | 1401 (31) | 2501 (31) | 264 (40) | 312 (40) | 18 (46) | 21 (46) |
| support points + controls + trumps | 1446 (40) | 2576 (39) | 279 (49) | 330 (49) | 19 (54) | 22 (54) |
| support points + controls + 1½ per trump | **1456** (43½) | **2593** (43½) | 278 (52½) | 328 (53) | 18 (59) | 21 (59) |

By shape, major fits, not vulnerable (4M over 3M / 6M over 4M):

| shape | Zar | Zar, waste repaired | Zar, waste and trump terms | support points + trumps | support points + controls + trumps |
| --- | --- | --- | --- | --- | --- |
| both hands balanced | 732 / 97 | 732 / 97 | 733 / 96 | 709 / 71 | 732 / 90 |
| unbalanced, no singleton | 1047 / 169 | 1047 / 169 | 1072 / 171 | 1035 / 134 | **1090 / 172** |
| a singleton, no void | 1563 / 261 | 1604 / 313 | **1657 / 357** | 1615 / 242 | 1649 / 317 |
| a void | 2443 / 481 | 2540 / 684 | **2605 / 827** | 2538 / 607 | 2548 / 654 |

## Result 6 — one hand: the opening

One hand's count against what its side can do. **Tricks** are the most the
side takes in any strain. The side **makes game** if it takes 9 tricks in
notrump, 10 in a major or 11 in a minor. The side **owns the deal** if it
holds the highest makeable contract of the two sides.

| hands | scale | sd of the trick fit | R² | AUC, owns the deal | AUC, makes game |
| --- | --- | --- | --- | --- | --- |
| all (410.31M) | raw HCP | 1.865 | 0.220 | 0.738 | 0.730 |
| | `point_count` | 1.834 | 0.246 | **0.743** | 0.740 |
| | HCP + two longest suits | 1.823 | 0.255 | 0.741 | 0.741 |
| | Zar, no discount | 1.802 | 0.272 | 0.740 | 0.744 |
| | **Zar** | **1.800** | **0.274** | 0.741 | **0.745** |
| balanced (195.33M) | raw HCP | 1.852 | 0.242 | 0.748 | 0.746 |
| | Zar | 1.843 | 0.249 | 0.748 | 0.745 |
| unbalanced, no singleton (68.65M) | raw HCP | 1.769 | 0.227 | 0.732 | 0.733 |
| | `point_count` | 1.765 | 0.231 | 0.733 | 0.734 |
| | Zar | 1.758 | 0.237 | 0.734 | 0.736 |
| a singleton (125.38M) | raw HCP | 1.779 | 0.215 | 0.730 | 0.721 |
| | `point_count` | 1.765 | 0.228 | 0.732 | 0.726 |
| | Zar | 1.755 | 0.236 | 0.730 | 0.729 |
| a void (20.94M) | raw HCP | 1.732 | 0.223 | 0.743 | 0.722 |
| | `point_count` | 1.729 | 0.225 | 0.741 | 0.723 |
| | Zar | 1.707 | 0.245 | 0.740 | 0.731 |

What a hand of each value is worth:

| value | hands | side owns the deal | side makes game | tricks |
| --- | --- | --- | --- | --- |
| Zar 24 | 26.02M | 48.2% | 26.2% | 8.39 |
| Zar 25 | 25.06M | 51.9% | 29.4% | 8.57 |
| **Zar 26** | 24.43M | **55.8%** | 32.7% | 8.74 |
| Zar 27 | 22.67M | 59.5% | 36.1% | 8.92 |
| Zar 28 | 21.11M | 63.4% | 39.8% | 9.10 |
| HCP 10 | 38.59M | 49.9% | 28.1% | 8.47 |
| HCP 11 | 36.70M | 55.7% | 32.7% | 8.71 |
| **HCP 12** | 32.94M | **61.2%** | 37.6% | 8.95 |
| HCP 13 | 28.38M | 66.7% | 42.7% | 9.18 |
| `point_count` 11 | 37.93M | 53.0% | 30.4% | 8.61 |
| **`point_count` 12** | 35.14M | **58.9%** | 35.5% | 8.87 |
| HCP + two longest suits 19 | 37.13M | 52.3% | 29.8% | 8.58 |
| **HCP + two longest suits 20** | 34.71M | **58.0%** | 34.6% | 8.83 |

Who opens. The last column is the most tricks the other side takes in any
strain:

| hands | share of all hands | mean HCP | void / singleton / balanced | side owns the deal | side makes game | tricks | the other side's tricks |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Zar ≥ 26 | 42.1% | 13.6 | 9% / 39% / 35% | 70.6% | 48.8% | 9.48 | 7.58 |
| HCP ≥ 12 | 34.8% | 14.5 | 5% / 30% / 48% | 73.3% | 50.7% | 9.51 | 7.36 |
| Rule of 20 | 40.3% | 13.9 | 7% / 36% / 39% | 71.3% | 49.3% | 9.48 | 7.51 |
| `point_count` ≥ 12 | 39.0% | 14.1 | 7% / 34% / 43% | 72.1% | 49.8% | 9.49 | 7.46 |
| Zar opens, HCP ≥ 12 does not | 10.1% | 10.1 | 18% / 59% / 7% | 57.0% | 37.2% | 9.09 | 8.41 |
| HCP ≥ 12 opens, Zar does not | 2.9% | 12.3 | 0% / 2% / 94% | 56.4% | 31.3% | 8.44 | 7.91 |
| Zar opens, Rule of 20 does not | 4.6% | 9.5 | 18% / 54% / 16% | 54.0% | 33.5% | 8.90 | 8.47 |
| Rule of 20 opens, Zar does not | 2.8% | 12.0 | 0% / 10% / 68% | 54.7% | 30.5% | 8.49 | 8.07 |
| Zar opens, `point_count` ≥ 12 does not | 6.1% | 9.7 | 17% / 57% / 12% | 53.9% | 34.2% | 8.94 | 8.51 |
| `point_count` ≥ 12 opens, Zar does not | 3.0% | 12.3 | 0% / 5% / 90% | 56.5% | 31.3% | 8.45 | 7.91 |
| no rule opens | 54.2% | 7.1 | 3% / 25% / 56% | 33.7% | 16.7% | 7.63 | 9.14 |
| every rule opens | 31.9% | 14.7 | 6% / 33% / 44% | 74.9% | 52.5% | 9.61 | 7.31 |

## Conclusions

### When Zar is better

1. **With a fit, Zar beats every count that ignores shortness and ties plain
   support points.** Its sd is 1.005 tricks against 1.190 for raw HCP, 1.070
   for `point_count` and 1.004 for support points. The `pons` gate, support
   points + trumps, is ahead of it at 0.945.
2. **Zar decides best when neither hand has a singleton.** With both hands
   balanced its sd is 0.851 against 0.861 for the gate, and it wins 732
   against 709 mIMP per case at game and 97 against 71 at slam. Unbalanced
   with no singleton it wins 1047 against 1035 and 169 against 134, although
   its sd is behind there (0.885 against 0.857). Two balanced hands differ
   little in shape, so the gain is the honour scale.
3. **Zar is the best slam scale of the standard ones.** Among pairs that take
   10 tricks it ranks the 12-trick ones at AUC 0.878 against 0.871 for the
   gate, and the 13-trick ones among 12 at 0.774 against 0.743. As a
   threshold it wins 229 mIMP per case against 226. The reason is the control
   count: with a fit a control is worth 1.34 Zar points, and 2.0 with 10+
   trumps.
4. **Zar beats support points with nine or more trumps and loses with
   eight.** Inside a trump length its sd is 0.908, 0.836 and 0.810 with 9, 10
   and 11+ trumps, against 0.920, 0.882 and 0.859. With eight it is 0.999
   against 0.943, behind even `point_count` (0.938). The likely reason: the
   eight-card fits include the 6-2 and 7-1 ones, and Zar pays a short trump
   holding as if it could ruff (Result 4).
5. **One hand alone: Zar is the best predictor of the side's tricks and of
   its game.** The sd is 1.800 against 1.865 for raw HCP and 1.823 for the
   Rule of 20 count, and the AUC for game 0.745 against 0.730. The gain is
   in the unbalanced hands. On balanced hands every scale reads the same.

### When it is not

6. **At notrump Zar is the wrong scale, even with two balanced hands.** With
   no 8-card fit its sd is 1.451 against 0.949 for raw HCP and 0.920 for
   Fifths, and it wins 603 mIMP per case at 3NT against 814 and 834. With both
   hands balanced it is still 1.112 against 0.929. A control is worth 0.19
   of a point at notrump and a card of length nothing, and the honours stand
   at 7.4, 5.5, 3.6, 2.0 and 0.9 (ace to ten), which is Fifths and not
   6-4-2-1.
7. **With a void or a 7+ card suit Zar is no better than raw HCP.** Its sd
   is 1.259 against 1.231 with a void and 1.122 for both with a 7+ card suit,
   which it overvalues by 0.33 tricks. At the game decision with a void it wins 2443 mIMP per case against
   2538 for the gate, and at slam 481 against 607.
8. **At the game decision Zar trails the `pons` gate** (1396 against 1431
   mIMP per case not vulnerable) and beats plain support points (1382).
9. **Zar does not find who owns the deal better than a point count.** The AUC
   is 0.741 against 0.743 for `point_count` and 0.738 for raw HCP. A shapely
   hand takes tricks for its side, and gives the other side a fit as well.

### What does the work

10. **The honour scale is right, the distribution count is not.** With a fit
    the cards are worth 6.8, 4.5, 2.2 and 1.1 Zar points against Zar's 6, 4, 2
    and 1, and a ten 0.4. The suit lengths are worth 1.1, 1.1 and −1.8 against
    Zar's 2, 1 and −1: Zar pays the longest suit twice its value and the
    short suit half. The a − d term as a whole earns a third of its points
    (0.34). That is why Zar overvalues long suits and falls behind support
    points when shortness decides.
11. **Zar's honours on the `pons` count beat both.** Support points +
    controls + trumps has sd 0.920 against 0.945 for the gate and 1.005 for
    Zar. It wins 1446 mIMP per case at game (1431 and 1396), 279 at slam
    (226 and 229) and 19 at the grand slam (8 and 10). With 1½ per trump the
    sd is 0.904 and game 1456. It is a blind count: each hand adds its own
    number.

### Wasted values

12. **Zar's own discount is about right in a suit contract, except for the
    singleton king.** A doubleton queen or jack owes 1.07 points, with an ace
    or king beside it 1.01, a singleton queen 1.10. A singleton king owes
    2.49, not 1. A singleton jack owes 0.39. The discount is worth 0.015
    tricks of sd (1.020 to 1.005) and 13 mIMP per case at slam (216 to 229).
13. **The larger waste is an honour facing partner's shortness, and Zar has
    no term for it.** Facing a void an ace owes 2.66 points and a king 1.95.
    Facing a singleton a king owes 1.99 and a queen 0.76. An ace facing a
    singleton, and a jack facing anything, owe nothing. In tricks: 0.73 for
    the ace, 0.5 for either king, 0.17 for the queen.
14. **It is a suit-contract adjustment.** At notrump an honour facing
    partner's shortness loses almost nothing (0.12 to 0.13 tricks for an ace
    or king facing a void, 0.04 or less for the rest). One's own short honours do lose at
    notrump: 0.43, 0.32 and 0.19 tricks for a singleton king, queen and jack.
15. **A short trump honour needs no discount.** Once the short trump holding
    itself is priced, the honour in it is worth its full points (0.04). Zar,
    which does not know the trump suit, takes a point off.
16. **A short trump holding is the largest single correction.** Zar pays its
    shortness as distribution. A doubleton owes 2.5 points, a singleton 4.6
    and a void 6.3. [nltc.md](nltc.md) found the same for NLTC.
17. **Repairing the waste is worth the most at slam.** With the round
    numbers of Result 5 the sd falls from 1.005 to 0.968. Game moves from
    1396 to 1413 mIMP per case, slam from 229 to 276 and the grand slam from
    10 to 23. The gain is all in the shapely hands: slam with a void goes
    from 481 to 684, with a singleton from 261 to 313, and with no singleton
    nothing moves. With the trump terms as well the sd is 0.905 and the
    count wins 1453, 308 and 30. That is the best slam count here; at game
    the blind count of conclusion 11 matches it (1456 with 1½ per trump).
    Unlike that count, this one needs partner's shortness to be known.

### Zar's own numbers

18. **52 for game and 62 for slam are the best thresholds.** The fitted ones
    are 52 (51 vulnerable) and 62. A grand slam wants 68 or 69, not 67, and
    no blind scale decides it: 10 of the 174 mIMP per case available.
19. **26 Zar is a lighter opening than 12 HCP.** A hand of exactly 26 Zar
    owns the deal 55.8% of the time, the same as 11 HCP (55.7%). The matching
    numbers are 61.2% for 12 HCP, 58.9% for `point_count` 12 and 58.0% for
    the Rule of 20. Zar opens 42.1% of all hands against 34.8%, 39.0% and
    40.3%.
20. **The hands only Zar opens are sound on offence and contested.** They
    are 10.1% of all hands, average 10.1 HCP, and 77% have a singleton or
    void. Their side owns the deal 57.0% of the time and makes game 37.2%.
    The other side takes 8.41 tricks in its best strain. The hands only
    12 HCP opens (2.9%, 94% balanced) own the deal as often (56.4%) and make
    game less often (31.3%), and the other side takes 7.91.
21. **Three points for every extra trump is too many.** The 9th trump is
    worth 2.3 Zar points and the 10th one more (1.8 and ½ once a short trump
    holding is priced). Petkov's rule overvalues 10 trumps by 0.41 tricks and
    11+ by 1.05, and loses at slam (190 mIMP per case against 229 for plain
    Zar). At Zar's own threshold of 62 it wins 80. The Wikipedia rule, 2 with
    a void and 1 with a singleton, is the better one: sd 0.975, game 1422 and
    slam 232.
22. **A point for an honour in partner's suit holds for an eight-card fit
    only.** With eight trumps the trump queen, jack and ten are worth 1.00,
    0.75 and 0.86 extra points and the ace and king 0.47 and 0.67. With nine
    they are worth nothing but the ten (0.46). With ten or more the top
    honours are worth less in trumps than outside.

### For `pons`

23. **Nothing changes in `pons` yet.** Three candidates come out of this
    study, each needing its own A/B:
    - the control count on top of the game and slam gates (conclusion 11),
      which is blind and gains the most at slam;
    - the deductions of conclusion 13 after partner shows shortness (a
      splinter, a short-suit trial), for both partners' kings and queens in
      that suit;
    - notrump stays on HCP and Fifths. Zar has nothing to offer there.

## Reproduce

[`examples/probe-zar.rs`](../examples/probe-zar.rs) writes the cell table;
[`scripts/zar-report.py`](../scripts/zar-report.py) prints every table above
from it.

```sh
cargo build --release --example probe-zar
RAYON_NUM_THREADS=8 nice -n 19 target/release/examples/probe-zar \
    /nfs2/jdh8/pons/22.pdd /nfs2/jdh8/pons/24.pdd /nfs2/jdh8/pons/shard-*.pdd \
    /mnt/hdd-data/jdh8/shards/shard-*.pdd > zar.tsv
python3 scripts/zar-report.py zar.tsv
```

## Caveats

- Double-dummy play with the better declarer: no wrong-siding, and no
  single-dummy guessing by either side.
- Undoubled and uncontested, on the actual combined holdings. A bidder sees
  one hand and partner's shown range, not the total.
- Trumps are the longest combined suit. A pair with a 4-4 major fit and a
  longer minor is counted in the minor.
- The slam rows count 12 tricks as a slam and 13 as a grand slam. No control
  check is modelled.
- Zar's fit points are tested on the trumps over eight. Petkov counts the
  trumps over the length the auction promised, which a census cannot know.
  His other adjustments (concentrated honours, the spade suit at 25, honours
  in the opponents' suits) are not tested.
- The terms of Results 3 and 4 are fitted together and are correlated with
  shape. "Zar owes" depends on what else is in the fit: the columns with and
  without the trump terms differ by up to 0.2 points.
- "Facing partner's shortness" uses partner's actual holding. At the table
  it is known only when partner has shown the short suit.
- The round numbers of Result 5 were read off the same deals they are scored
  on. That is about 20 numbers against 173M cases.
- "Owns the deal" compares the highest contract each side can make. It
  ignores sacrifices, vulnerability and who gets to bid first. More than half
  of the hands at every opening threshold own the deal, so it measures a
  hand, not the right threshold.
