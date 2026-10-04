# Suit slams: the best count, the control check, and wasted honours — double-dummy census

Computed 2026-10-04 from the `pons` double-dummy deal bank. This is a plain
statistic over random deals, with no bidding simulation. It continues
[zar.md](zar.md) and [nltc.md](nltc.md), which found that every count is
weakest at slam. The notrump companion is [notrump-slam.md](notrump-slam.md).

## Question

1. Which count decides a suit slam best: the small slam over game, and the
   grand slam over the small one?
2. What are the cards and the short suits worth in the slam zone, against
   what they are worth on all hands?
3. Does the count still matter once a control check (keycards) is made?
4. Do wasted honours need an adjustment at slam level?

## Data and method

- **Deals:** 102,577,264 random deals, each with a complete double-dummy trick
  table. Every partnership (N–S and E–W) whose longest combined suit has 8+
  cards is one case, with that suit as trumps: 172.86M cases, 90.86M with a
  major as trumps and 82.00M with a minor. 7.25% of them take 12+ tricks and
  1.56% take 13.
- **Tricks:** double-dummy, taking the better of the two partners as declarer.
- **Holdings:** the actual 26 cards of the partnership, both hands summed.
- **Decision value:** the IMPs won by the best rule "bid it when the value
  reaches t", against never bidding it. 6M is compared with 4M, 6m with 5m and
  7 with 6, undoubled, with the same trick count and the opponents silent. The
  unit is mIMP per case (1000 mIMP = 1 IMP) over every major fit, or every
  minor fit.
- **Control check.** A small slam is bid only if at most one of the five
  keycards (four aces and the trump king) is missing and no suit has two top
  losers (neither ace nor king, with two or more cards in each hand, or in
  trumps). A grand slam is bid only with all five keycards and no such suit.
  16.8% of all cases pass the small-slam check and 85.6% of the pairs that
  take 12 tricks; 3.4% pass the grand-slam check and 67.2% of the pairs that
  take 13. The check does not know that a void replaces an ace.
- **Zone:** support points + trumps, the `pons` gate, in bands: under 34, 34
  to 37, 38 to 41 (the band where the small slam is decided), 42 and over.
- **Sample size:** the 95% interval of every fitted term is ±0.007 tricks or
  ±0.5 percentage points of a chance, or less, so none is shown.

### Counts

The standard ones are as in [zar.md](zar.md): raw HCP, `point_count`, support
points (with trumps added: the `pons` gate), support points + controls
(ace 2, king 1), Zar, and NLTC. Three more are built here, on a scale where
an ace is 4:

- **4-2-1-½.** Ace 4, king 2, queen 1, jack ½ in every suit. A side void,
  singleton and doubleton are 3, 1½ and ½ in either hand.
- **Slam fit, hands apart.** The weights of Result 3, fitted to the chance of
  12 tricks in the 38–41 band, with no term that needs partner's hand. Rounded
  to eighths (`SUIT_COUNTS` in the probe).
- **Slam fit, shortness known.** The same with the terms for an honour facing
  partner's void or singleton.

None of the three counts the trumps. Each is shown with no trump term, with
½ or 1 point per trump, and with the stepped term of Result 5: 1 point for
the 9th trump, ½ for the 10th and ¼ for the 11th.

## Result 1 — the decision

Major fits, in mIMP per case with the best threshold in brackets. "Checked"
bids only when the control check passes.

| count | 6M over 4M, not vul | vul | checked, not vul | checked, vul | 7M over 6M, not vul | vul | checked, not vul | checked, vul |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| perfect knowledge of the tricks | 803 | 949 | 803 | 949 | 174 | 205 | 174 | 205 |
| raw HCP | 169 (29) | 199 (29) | 225 (28) | 266 (28) | 8 (33) | 9 (33) | 24 (31) | 28 (31) |
| `point_count` | 207 (30) | 245 (30) | 273 (29) | 322 (29) | 11 (34) | 13 (34) | 32 (32) | 37 (32) |
| support points | 219 (31) | 258 (31) | 308 (30) | 364 (30) | 8 (36) | 9 (36) | 37 (33) | 42 (33) |
| support points + trumps (the gate) | 226 (40) | 266 (40) | 320 (39) | 378 (39) | 8 (45) | 9 (45) | 38 (42) | 44 (42) |
| support points + controls | 264 (40) | 312 (40) | 304 (39) | 359 (39) | 18 (46) | 21 (46) | 36 (44) | 42 (44) |
| support points + controls + trumps | 279 (49) | 330 (49) | 325 (48) | 385 (48) | 19 (54) | 22 (54) | 39 (52) | 45 (52) |
| Zar | 229 (62) | 270 (62) | 308 (60) | 363 (60) | 10 (68) | 11 (69) | 40 (65) | 46 (65) |
| NLTC | 147 (≤ 12½) | 170 (≤ 12½) | 274 (≤ 13) | 322 (≤ 13) | 2 (≤ 9½) | 1 (≤ 9) | 30 (≤ 12) | 34 (≤ 12) |
| support points + 1, ½, ¼ for trumps 9–11 | 227 (31½) | 268 (31½) | 320 (30½) | 378 (30½) | 9 (36½) | 10 (36½) | 38 (33½) | 45 (33½) |
| 4-2-1-½, no trump term | 279 (24½) | 329 (24½) | 301 (24) | 355 (24) | 27 (27½) | 30 (27½) | 38 (27) | 43 (27) |
| 4-2-1-½ + ½ per trump | 299 (28½) | 353 (28½) | 317 (28½) | 374 (28½) | 28 (32) | 32 (32) | 39 (31) | 45 (31) |
| 4-2-1-½ + 1 per trump | 296 (33) | 350 (33) | 320 (32½) | 378 (32½) | 24 (36½) | 28 (36½) | 37 (35½) | 43 (35½) |
| **4-2-1-½ + 1, ½, ¼ for trumps 9–11** | 301 (24¾) | 355 (24¾) | 323 (24¼) | 382 (24¼) | 27 (28) | 31 (28½) | 39 (27½) | 45 (27½) |
| slam fit, hands apart, no trump term | 292 (24½) | 345 (24½) | 303 (24½) | 358 (24½) | 31 (27½) | 35 (28) | 37 (27½) | 43 (27½) |
| slam fit, hands apart, + ½ per trump | 309 (29) | 365 (29) | 322 (28½) | 381 (28½) | **32** (32) | **36** (32) | 39 (31½) | 45 (31½) |
| slam fit, hands apart, + 1 per trump | 301 (33½) | 356 (33½) | 324 (33) | 382 (33) | 28 (36½) | 32 (37) | 37 (36) | 42 (36) |
| **slam fit, hands apart, + 1, ½, ¼ for trumps 9–11** | **312** (25) | **368** (25) | **327** (24¾) | **386** (24¾) | 31 (28¼) | 36 (28½) | 39 (27¾) | 44 (27¾) |
| slam fit, shortness known, no trump term | 340 (24) | 402 (24) | 330 (24) | 390 (24) | 48 (27) | 55 (27) | 40 (27) | 47 (27) |
| slam fit, shortness known, + ½ per trump | 364 (28½) | 429 (28½) | 348 (28½) | 411 (28½) | 49 (31½) | 57 (31½) | 41 (31½) | 48 (31½) |
| slam fit, shortness known, + 1, ½, ¼ for trumps 9–11 | 363 (24¾) | 429 (24¾) | 347 (24¾) | 410 (24¾) | 48 (27¾) | 56 (27¾) | 41 (27½) | 47 (27½) |

Minor fits, not vulnerable:

| count | 6m over 5m | checked | 7m over 6m | checked |
| --- | --- | --- | --- | --- |
| perfect knowledge of the tricks | 791 | 791 | 169 | 169 |
| `point_count` | 240 (29) | 305 (28) | 11 (34) | 31 (32) |
| support points | 250 (30) | 335 (29) | 8 (36) | 36 (33) |
| support points + trumps | 261 (39) | 352 (38) | 8 (45) | 38 (42) |
| support points + controls | 289 (39) | 331 (39) | 17 (46) | 35 (44) |
| support points + controls + trumps | 308 (48) | 350 (47) | 18 (54) | 38 (52) |
| Zar | 278 (61) | 343 (59) | 10 (69) | 39 (65) |
| NLTC | 226 (≤ 13) | 328 (≤ 13½) | 1 (≤ 9½) | 29 (≤ 12) |
| 4-2-1-½, no trump term | 313 (24) | 335 (23½) | 26 (27½) | 37 (27) |
| 4-2-1-½ + ½ per trump | 326 (28½) | 349 (28) | 27 (32) | 38 (31) |
| 4-2-1-½ + 1, ½, ¼ for trumps 9–11 | 331 (24½) | 351 (24) | 26 (28½) | 38 (27½) |
| slam fit, hands apart, no trump term | 325 (24) | 340 (24) | 30 (27½) | 36 (27½) |
| slam fit, hands apart, + ½ per trump | **339** (28½) | 349 (28) | **31** (32) | 38 (31½) |
| slam fit, hands apart, + 1, ½, ¼ for trumps 9–11 | **339** (24¾) | **356** (24½) | 30 (28¼) | 37 (27¾) |
| slam fit, shortness known, no trump term | 377 (24) | 362 (24) | 46 (27) | 39 (27) |
| slam fit, shortness known, + ½ per trump | 386 (28½) | 369 (28) | 47 (31½) | 40 (31½) |

By shape of the pair and by trump length, major fits, not vulnerable, in mIMP
per case of that row (small slam alone / checked):

| row | perfect | support points + trumps | support points + controls + trumps | Zar | 4-2-1-½ + ½ per trump | slam fit, hands apart | slam fit, shortness known |
| --- | --- | --- | --- | --- | --- | --- | --- |
| both hands balanced | 287 | 71 / 88 | 90 / 94 | **97 / 102** | 89 / 89 | 84 / 84 | 90 / 90 |
| unbalanced, no singleton | 477 | 134 / 186 | 172 / **200** | 169 / 193 | **182** / 192 | 178 / 184 | 185 / 193 |
| a singleton, no void | 869 | 242 / 376 | 317 / **387** | 261 / 355 | 347 / 377 | **362** / 385 | 394 / 402 |
| a void | 1965 | 607 / **725** | 654 / 713 | 481 / 651 | 654 / 696 | **699** / 721 | 1024 / 828 |
| 8 trumps | 557 | 137 / 194 | 181 / **202** | 126 / 177 | 182 / 192 | **192** / 199 | 213 / 213 |
| 9 trumps | 998 | 294 / 425 | 362 / 427 | 327 / **430** | 392 / 426 | **401** / 423 | 473 / 452 |
| 10+ trumps | 1439 | 462 / 663 | 547 / 656 | 559 / **684** | 619 / 669 | **628** / 663 | 780 / 709 |

The bold cells are the best of the blind counts, alone and checked. The last
column needs partner's shortness and is not compared. The three new counts
carry ½ per trump here.

The same rows with no trump term in any count:

| row | support points | support points + controls | 4-2-1-½ | slam fit, hands apart | slam fit, shortness known |
| --- | --- | --- | --- | --- | --- |
| both hands balanced | 78 / 89 | **93 / 95** | 89 / 89 | 83 / 83 | 91 / 91 |
| unbalanced, no singleton | 136 / 186 | 176 / **198** | **182** / 190 | 180 / 184 | 188 / 190 |
| a singleton, no void | 238 / 363 | 304 / **368** | 327 / 356 | **343** / 363 | 379 / 388 |
| a void | 525 / **678** | 576 / 668 | 588 / 657 | **638** / 677 | 954 / 792 |
| 8 trumps | 137 / 194 | 181 / **202** | 182 / 192 | **192** / 199 | 213 / 213 |
| 9 trumps | 294 / 425 | 362 / **427** | 392 / 426 | **398** / 425 | 475 / 455 |
| 10+ trumps | 476 / 663 | 546 / 659 | 617 / **668** | **630** / 661 | 784 / 710 |

Inside a trump-length row a trump term is nearly a constant, so those rows
match the table above.

Ranking power (AUC), all fit cases:

| count | 12+ tricks | 12+ among pairs that take 10+ | 12+ among 11+ | 13 among 12+ |
| --- | --- | --- | --- | --- |
| raw HCP | 0.922 | 0.804 | 0.725 | 0.694 |
| `point_count` | 0.942 | 0.841 | 0.760 | 0.723 |
| support points | 0.953 | 0.864 | 0.782 | 0.741 |
| support points + trumps | 0.957 | 0.871 | 0.788 | 0.743 |
| support points + controls | 0.958 | 0.877 | 0.799 | 0.770 |
| support points + controls + trumps | 0.963 | 0.888 | 0.810 | 0.777 |
| Zar | 0.958 | 0.878 | 0.801 | 0.774 |
| NLTC | 0.950 | 0.861 | 0.784 | 0.754 |
| 4-2-1-½, no trump term | 0.965 | 0.894 | 0.822 | 0.819 |
| 4-2-1-½ + ½ per trump | 0.969 | 0.903 | 0.830 | 0.824 |
| 4-2-1-½ + 1, ½, ¼ for trumps 9–11 | 0.969 | 0.904 | 0.832 | 0.823 |
| slam fit, hands apart, no trump term | 0.968 | 0.901 | 0.830 | 0.833 |
| slam fit, hands apart, + ½ per trump | 0.971 | 0.909 | 0.839 | **0.838** |
| slam fit, hands apart, + 1, ½, ¼ for trumps 9–11 | **0.972** | **0.911** | **0.841** | **0.838** |
| slam fit, shortness known, no trump term | 0.977 | 0.926 | 0.864 | 0.879 |
| slam fit, shortness known, + ½ per trump | 0.978 | 0.930 | 0.869 | 0.878 |

## Result 2 — the odds at each value

Major fits. The last two columns are the share of pairs that pass the control
check and the chance of 12 tricks among them.

| count | value | cases | 12+ tricks | 13 tricks | pass the check | 12+ tricks, checked |
| --- | --- | --- | --- | --- | --- | --- |
| support points + trumps | 38 | 2.85M | 31.5% | 3.8% | 55% | 48.7% |
| | 39 | 2.12M | 45.0% | 7.2% | 63% | 62.6% |
| | **40** | 1.52M | **58.4%** | 12.4% | 71% | 74.4% |
| | 41 | 1.04M | 70.2% | 19.6% | 78% | 83.4% |
| | 42 | 0.68M | 79.5% | 28.4% | 83% | 89.4% |
| | 43 | 0.42M | 86.2% | 38.3% | 88% | 93.1% |
| | 44 | 0.25M | 90.9% | 48.5% | 92% | 95.5% |
| `point_count` | 29 | 1.92M | 48.1% | 8.6% | 73% | 61.0% |
| | **30** | 1.31M | **63.2%** | 14.6% | 82% | 73.5% |
| | 31 | 0.85M | 76.5% | 23.3% | 89% | 83.4% |
| | 32 | 0.52M | 86.2% | 34.5% | 94% | 90.0% |
| | 33 | 0.30M | 92.4% | 47.4% | 97% | 94.0% |
| | 34 | 0.16M | 95.9% | 60.2% | 99% | 96.4% |
| | 35 | 0.08M | 97.7% | 72.1% | 100% | 97.8% |
| raw HCP | 28 | 1.92M | 42.8% | 8.3% | 72% | 55.9% |
| | **29** | 1.32M | **56.6%** | 13.1% | 81% | 67.3% |
| | 30 | 0.86M | 70.2% | 20.6% | 89% | 77.7% |
| | 31 | 0.53M | 81.5% | 30.3% | 94% | 86.0% |
| | 32 | 0.31M | 89.6% | 41.5% | 98% | 91.5% |
| | 33 | 0.17M | 94.8% | 55.2% | 100% | 95.2% |
| 4-2-1-½ + ½ per trump | 28 | 1.38M | 43.1% | 3.9% | 78% | 48.5% |
| | **28½** | 1.15M | **52.9%** | 6.3% | 84% | 57.9% |
| | 29 | 0.95M | 62.9% | 10.0% | 88% | 67.1% |
| | 30 | 0.61M | 79.9% | 22.4% | 94% | 82.3% |
| | 31 | 0.36M | 90.4% | 40.8% | 97% | 91.5% |

A small slam breaks even at 50%: it wins and loses 11 IMPs not vulnerable and
13 vulnerable.

## Result 3 — what the cards are worth in the slam zone

One regression per column, every term of this table and of Result 4 fitted
together, with one constant per band and trump length. Each number is in
points on a scale where a side ace is 4. The bottom row gives the ace itself.

| term | support points | tricks, all hands | tricks, 38–41 | chance of 12, 38–41 | chance of 12, 38–41, hands apart | chance of 13, 42+ |
| --- | --- | --- | --- | --- | --- | --- |
| side king | 3 | 2.65 | 2.07 | 2.15 | 1.85 | 1.02 |
| side queen | 2 | 1.33 | 0.83 | 0.88 | 0.88 | 0.33 |
| side jack | 1 | 0.65 | 0.34 | 0.36 | 0.39 | 0.14 |
| side ten | 0 | 0.21 | 0.13 | 0.13 | 0.14 | 0.05 |
| trump ace | 4 | 4.02 | 3.79 | 3.78 | 4.21 | 3.93 |
| trump king | 3 | 2.74 | 2.24 | 2.27 | 2.53 | 2.14 |
| trump queen | 2 | 1.63 | 1.12 | 1.10 | 1.22 | 0.91 |
| trump jack | 1 | 0.91 | 0.57 | 0.54 | 0.60 | 0.39 |
| trump ten | 0 | 0.43 | 0.29 | 0.26 | 0.27 | 0.15 |
| side void, hand with 4+ trumps | 3 | 5.65 | 5.56 | 5.46 | 2.92 | 4.60 |
| side singleton, hand with 4+ trumps | 2 | 3.04 | 2.16 | 2.28 | 1.42 | 0.77 |
| side doubleton, hand with 4+ trumps | 1 | 0.78 | 0.42 | 0.44 | 0.40 | 0.26 |
| side void, hand with 3 trumps | 3 | 4.67 | 5.10 | 5.10 | 2.76 | 4.54 |
| side singleton, hand with 3 trumps | 2 | 2.53 | 2.13 | 2.26 | 1.34 | 0.79 |
| side doubleton, hand with 3 trumps | 1 | 0.68 | 0.50 | 0.52 | 0.42 | 0.33 |
| side void, hand with 2 or fewer trumps | 3 | 3.42 | 3.67 | 4.30 | 2.08 | 4.16 |
| side singleton, hand with 2 or fewer trumps | 2 | 1.72 | 1.36 | 1.91 | 0.99 | 0.77 |
| side doubleton, hand with 2 or fewer trumps | 1 | 0.59 | 0.41 | 0.49 | 0.28 | 0.33 |
| a second fit of 8+ cards | — | 0.31 | 0.15 | 0.13 | 0.50 | 0.09 |
| **a side ace (= 4)** | 4 | 1.62 tricks | 1.38 tricks | +67.5% | +64.0% | +73.2% |

"Hands apart" leaves out the terms of Result 4 that need partner's hand, so
its shortness is the average over what partner holds opposite it.

## Result 4 — wasted honours

The same regressions. Each number is what the holding loses, in points where
a side ace is 4, on top of the card's full value in Result 3.

| holding | tricks, all hands | tricks, 38–41 | chance of 12, 38–41 | share of the card's value kept | chance of 12, 38–41, hands apart | chance of 13, 42+ |
| --- | --- | --- | --- | --- | --- | --- |
| singleton side king | −1.79 | −1.31 | −1.39 | 35% | −0.76 | −0.56 |
| singleton side queen | −0.87 | −0.45 | −0.49 | 44% | −0.43 | −0.10 |
| singleton side jack | −0.40 | −0.12 | −0.15 | 58% | −0.21 | −0.07 |
| side doubleton Qx, Jx or QJ | −0.31 | −0.15 | −0.20 | | −0.25 | −0.08 |
| side doubleton AQ, AJ, KQ or KJ | −0.36 | −0.14 | −0.14 | | −0.12 | −0.11 |
| side ace facing partner's void | −2.34 | −3.11 | **−2.88** | **28%** | | −3.37 |
| side king facing partner's void | −1.84 | −1.73 | **−1.57** | **27%** | | −0.61 |
| side queen facing partner's void | −0.93 | −0.72 | −0.57 | 35% | | −0.12 |
| side jack facing partner's void | −0.50 | −0.32 | −0.23 | 36% | | −0.04 |
| side ace facing partner's singleton | −0.26 | −0.20 | −0.21 | 95% | | +0.06 |
| side king facing partner's singleton | −1.40 | −1.14 | **−1.19** | **45%** | | −0.40 |
| side queen facing partner's singleton | −0.60 | −0.26 | −0.25 | 72% | | +0.11 |
| side jack facing partner's singleton | −0.26 | −0.01 | 0.00 | 100% | | +0.11 |

## Result 5 — what a trump is worth

The break-even value of each count with no trump term, by trump length: the
value where bidding 6M instead of 4M starts to gain. Major fits; the
vulnerable values are the same to 0.01, since both break even at 50%.

| count | 8 trumps | 9 trumps | 10 trumps | 11+ trumps | worth of the 9th, 10th, 11th trump |
| --- | --- | --- | --- | --- | --- |
| raw HCP | 29.10 | 27.75 | 26.85 | 26.12 | 1.34, 0.90, 0.73 |
| `point_count` | 29.70 | 28.49 | 27.76 | 27.27 | 1.21, 0.73, 0.49 |
| **support points** | 31.16 | 30.16 | 29.68 | 29.52 | **1.00, 0.48, 0.16** |
| support points + controls | 40.34 | 38.81 | 38.01 | 37.59 | 1.53, 0.80, 0.42 |
| **4-2-1-½** | 24.59 | 23.61 | 23.12 | 22.89 | **0.97, 0.49, 0.23** |
| slam fit, hands apart | 24.83 | 23.90 | 23.40 | 23.19 | 0.93, 0.50, 0.22 |
| support points, behind the control check | 30.20 | 28.82 | 28.17 | 28.02 | 1.38, 0.65, 0.15 |
| 4-2-1-½, behind the control check | 24.40 | 23.26 | 22.70 | 22.47 | 1.14, 0.56, 0.23 |

At the game decision the same trumps are worth 1.37, 0.77 and 0.43 support
points ([major-game-threshold.md](major-game-threshold.md)).

## Conclusions

### The count

1. **At slam each honour is worth about half the one above it.** On all
   hands the trick values are 4, 2.65, 1.33 and 0.65 for ace to jack, which is
   Zar's 6-4-2-1. In the band where the small slam is decided they are 4,
   2.1, 0.85 and 0.35. A king falls from two thirds of an ace to a half, a
   queen from a third to a fifth. At the grand slam a side king is a quarter
   of an ace and a queen a twelfth.
2. **4-2-1-½ is the simple slam count.** Ace 4, king 2, queen 1, jack ½, a
   side void 3, singleton 1½, doubleton ½, plus 1, ½ and ¼ for the 9th, 10th
   and 11th trump. As a threshold of 24¾ it wins 301 mIMP per case not
   vulnerable, against 226 for the `pons` gate, 229 for Zar and 279 for
   support points + controls + trumps. Fitted to the eighth of a point, the
   count wins 312. It ranks the 12-trick pairs among the 10-trick ones at
   AUC 0.911 (0.904 for 4-2-1-½) against 0.871 for the gate.
3. **A void keeps its price; a singleton and a doubleton fall with the kings
   and queens.** Hands apart and against an ace of 4, a void is worth 2.9
   points with three or more trumps, a singleton 1.4 and a doubleton 0.4.
   Support points pay 3, 2 and 1.
4. **Trump honours are worth more than side honours.** Hands apart, the
   trump king, queen and jack are worth 2.5, 1.2 and 0.6 against 1.85, 0.9
   and 0.4 outside. Summed over a suit that is about one point for the trump
   suit's king-queen-jack.
5. **The trumps count, but less than at game, and only when there is
   something to ruff.** The 9th trump is worth 1 point, the 10th ½ and the
   11th ¼, on support points and on 4-2-1-½ alike (Result 5); at game they
   are worth 1.4, 0.8 and 0.4. With no trump term the 4-2-1-½ count wins 279
   mIMP per case, with a flat ½ per trump 299, with 1 per trump 296, and
   with the stepped term 301. On support points the term adds only 7 (219
   to 226), against 49 at the game decision. The gain is all in the shapely
   hands: with a void the trump term lifts support points from 525 to 607,
   and with two balanced hands it costs (78 without, 71 with).
6. **Two balanced hands are the exception.** There Zar is the best count (97
   mIMP per case against 71 for the gate and 89 for 4-2-1-½). The fitted slam
   count is tuned on shapely hands and is behind (84).

### The control check

7. **A control check is worth more than any change of count.** It lifts the
   gate from 226 to 320 mIMP per case and Zar from 229 to 308. The best
   threshold falls by one point (40 to 39 on the gate).
8. **Once the check is made, the blind counts are equal.** The gate wins 320,
   support points + controls + trumps 325, 4-2-1-½ 317 to 323 and the fitted
   count 322 to 327, depending on the trump term. Weighting the aces does
   blindly what the check does by asking. The count matters where no check
   is available, and there it is worth 75 to 86 mIMP per case.
9. **NLTC stays last** even behind the check (274).

### Shortness and wasted honours

10. **Yes, wasted honours need an adjustment at slam, and it is the largest
   gain left.** Facing partner's void, an ace keeps 28% of its value and a
   king 27%. Facing a singleton, a king keeps 45% and a queen 72%; an ace
   keeps everything. With these terms the count wins 364 mIMP per case
   against 309 without them, and 1024 against 699 when a hand has a void.
11. **A void that faces no wasted honour is worth more than an ace**: 5.1
    to 5.5 points with three or more trumps and 4.3 with two or fewer,
    against 2.8 to 2.9 and 2.1 when partner's holding is unknown. The difference is the waste
    of conclusion 10, averaged.
12. **One's own short honours lose less than that.** A singleton king keeps
    35% of its value, a singleton queen 44%. A doubleton with a queen or
    jack loses 0.2 points. Hands apart these are −¾, −⅜ and −¼ of a point.
13. **With shortness known, the keycard check costs.** The count wins 364
    alone and 348 behind the check, and with a void 1024 against 828. The
    check stops the slams where a void stands in for the missing ace. That
    is the case for Exclusion keycard and for showing a void before asking.

### The grand slam

14. **No blind count decides a grand slam.** The best wins 32 of the 174
    mIMP per case available, the gate 8. Behind the check the gate, Zar and
    the new counts all win 37 to 40, so the check is nearly all of it. In the grand-slam zone the chance
    of 13 tricks depends on the aces (4), a void (4.6), the trump king (2.1)
    and the trump queen (0.9); a side king is 1.0 and a side queen 0.3. That
    is what a keycard ask with the queen ask finds.

### Thresholds

15. **On actual holdings a small slam wants 40 on the gate**, 39 behind the
    check: 58% of the 40-point pairs take 12 tricks, and 63% of the 39-point
    pairs that pass the check. The matching numbers are 30 and 29 on
    `point_count`, 29 and 28 on raw HCP, 62 and 60 on Zar, and 24¾ on 4-2-1-½
    with its stepped trump term (28½ with ½ per trump). By trump length the
    gate's support points are 31 with eight trumps, 30 with nine and 29½
    with ten or more (Result 5).
    A grand slam behind the check wants 42 on the gate and 32 on
    `point_count`.
16. **These are totals of actual hands.** A bidding gate adds the minimum
    partner has shown and so sees less than the total; the floor's milestones
    are 33 and 37 on that sum. See
    [major-game-threshold.md](major-game-threshold.md), conclusion 6.

### For `pons`

17. **Nothing changes in `pons` yet.** Candidates, each needing its own A/B:
    - where a slam is bid without a keycard ask, gauge it on a 4-2-1-½ style
      count instead of the point count (conclusion 8 says the gain is there
      and only there);
    - after partner shows a void or a singleton, drop the honours facing it
      as in Result 4 (the same lever as [zar.md](zar.md), conclusion 23, and
      larger at slam);
    - let a shown void count as a keycard in the suit (conclusion 13).

## Reproduce

[`examples/probe-slam.rs`](../examples/probe-slam.rs) writes the cell table;
[`scripts/slam-report.py`](../scripts/slam-report.py) prints every table
above, and those of [notrump-slam.md](notrump-slam.md), from it.

```sh
cargo build --release --example probe-slam
RAYON_NUM_THREADS=8 nice -n 19 target/release/examples/probe-slam \
    /nfs2/jdh8/pons/22.pdd /nfs2/jdh8/pons/24.pdd /nfs2/jdh8/pons/shard-*.pdd \
    /mnt/hdd-data/jdh8/shards/shard-*.pdd > slam.tsv
python3 scripts/slam-report.py slam.tsv
```

## Caveats

- Double-dummy play with the better declarer: no wrong-siding, and no
  single-dummy guessing. At slam level this flatters declarer, who never
  misguesses a queen, so real thresholds sit a little higher.
- Undoubled and uncontested, on the actual combined holdings.
- Trumps are the longest combined suit. A slam in a 4-4 fit beside a longer
  suit, and 6NT with a fit, are not priced here.
- The control check is an idealised one: it sees every ace, the trump king
  and every suit with two top losers, and nothing else. It does not see the
  trump queen, a void, or which opponent holds a missing ace.
- The bands are cut on support points + trumps. Inside a band the fit is a
  local one, and its weights are for hands the gate already rates near a
  slam.
- The fitted counts were read off the same deals they are scored on: about
  30 weights against 14M cases in the band.
- "Shortness known" uses partner's actual holding in every side suit. At the
  table it is known in one suit, after partner shows it.
