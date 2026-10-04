# NLTC vs. point counts, its trump adjustments, and cover cards — double-dummy census

Computed 2026-10-04 from the `pons` double-dummy deal bank. This is a plain
statistic over random deals, with no bidding simulation.

## Question

The New Losing Trick Count (NLTC) is the usual alternative to points once a
fit is found. The study asks:

1. When does NLTC judge a hand better than a point count? With unbalanced
   hands? At slam level?
2. NLTC does not know which suit is trumps. Does it need an adjustment for a
   short trump holding, where shortness has no ruffing value, and for extra
   trumps?
3. NLTC is not additive: a loser goes away only when partner holds the card
   that covers it. Is it better to count losers in the hand with longer trumps
   and cover cards in the other?

## Data and method

- **Deals:** 102,577,264 random deals, each with a complete double-dummy trick
  table. Every partnership (N–S and E–W) whose longest combined suit has 8+
  cards is one case, 172.86M cases in all. 90.86M of them have a major as
  trumps.
- **Trumps:** the longest combined suit. Between suits of equal length, the
  one that takes more tricks.
- **Tricks:** double-dummy, taking the better of the two partners as declarer.
  This is slightly optimistic: it assumes the hand is always right-sided.
- **Holdings:** the actual 26 cards of the partnership, both hands summed on
  every scale.
- **Sample size:** the 95% interval of every fitted term is ±0.005 tricks or
  less, so none is shown.

### Scales

- **NLTC.** In each suit a missing ace is 1.5 losers, a missing king 1 (in a
  suit of two or more cards) and a missing queen 0.5 (three or more cards).
  A void has no losers, and no suit has more than 3.
- **Support points (the `pons` count).** Trump-suit HCP, and in each side suit
  max(HCP, shortness, HCP + shortness − 1) with void = 3, singleton = 2 and
  doubleton = 1. Add 1 if the two longest suits total 10+ cards. Both hands
  count this way. Shortness in trumps counts nothing, and trump length is not
  included.
- **Raw HCP:** 4-3-2-1.
- **Zar points:** HCP, plus controls (ace = 2, king = 1), plus the two longest
  suits, plus the longest suit minus the shortest.

### Measures

- **Fit:** the line tricks = a + b × value, and the standard deviation (sd) of
  its misses in tricks.
- **Mean miss:** actual tricks minus the tricks that line predicts. A negative
  miss means the scale overvalues those hands.
- **Ranking power (AUC):** the chance that a random pair which takes N+ tricks
  rates higher than a random pair which does not. 0.5 is a coin flip, 1 is
  perfect.
- **Decision value:** the IMPs won by the best rule of the form "bid it when
  the value reaches t", against never bidding it. 4M is compared with 3M and
  6M with 4M, undoubled, with the same trick count and the opponents silent.
  The unit is mIMP per case (1000 mIMP = 1 IMP), averaged over every case
  with a major as trumps.

## Result 1 — fit, overall and by shape

All cases:

| scale | tricks per unit (b) | a | sd | R² |
| --- | --- | --- | --- | --- |
| NLTC | −0.842 | 22.11 | 1.094 | 0.710 |
| support points | +0.363 | 0.45 | 1.004 | 0.756 |
| raw HCP | +0.347 | 1.67 | 1.190 | 0.657 |
| Zar | +0.240 | −3.20 | 1.005 | 0.755 |

By the shape of the pair. Each cell is the sd of a line fitted inside that
row, with the mean miss of the all-cases line in brackets:

| shape | cases | NLTC | support points | raw HCP | Zar |
| --- | --- | --- | --- | --- | --- |
| both hands balanced | 33.40M | 0.903 [+0.18] | 0.879 [−0.28] | 0.905 [−0.91] | 0.851 [−0.02] |
| unbalanced, no singleton | 33.65M | 0.912 [+0.28] | 0.892 [+0.11] | 0.915 [−0.33] | 0.885 [+0.02] |
| a singleton, no void | 86.99M | 1.057 [−0.01] | 1.010 [+0.04] | 1.082 [+0.25] | 1.026 [−0.01] |
| a void | 18.82M | 1.301 [−0.77] | 1.211 [+0.12] | 1.231 [+1.06] | 1.259 [+0.03] |
| a 7+ card suit | 15.61M | 1.238 [−0.38] | 1.063 [+0.13] | 1.122 [+0.78] | 1.122 [−0.33] |

The first four rows split all cases by the shortest suit in either hand.
"Balanced" is (4333), (4432) or (5332). The last row overlaps the others.

## Result 2 — the game and slam decisions

Ranking power:

| question | NLTC | support points | raw HCP | Zar |
| --- | --- | --- | --- | --- |
| 10+ tricks | 0.923 | 0.933 | 0.896 | 0.934 |
| 12+ tricks | 0.950 | 0.953 | 0.922 | 0.958 |
| 12+ tricks, among pairs that take 10+ | 0.861 | 0.864 | 0.804 | 0.878 |
| 12+ tricks, both hands balanced | 0.981 | 0.981 | 0.981 | 0.986 |
| 12+ tricks, unbalanced, no singleton | 0.975 | 0.973 | 0.973 | 0.979 |
| 12+ tricks, a singleton | 0.950 | 0.950 | 0.943 | 0.957 |
| 12+ tricks, a void | 0.883 | 0.894 | 0.885 | 0.889 |
| 12+ tricks, a 7+ card suit | 0.906 | 0.925 | 0.902 | 0.924 |

Decision value in mIMP per case, major fits, with the best threshold in
brackets:

| scale | 4M over 3M, not vul | 4M over 3M, vul | 6M over 4M, not vul | 6M over 4M, vul |
| --- | --- | --- | --- | --- |
| perfect knowledge of the tricks | 2100 | 3501 | 803 | 949 |
| NLTC | 1368 (≤ 15.5) | 2490 (≤ 15.5) | 147 (≤ 12.5) | 170 (≤ 12.5) |
| NLTC, adjusted (Result 4) | 1401 (≤ 15) | 2536 (≤ 15.25) | 175 (≤ 12.25) | 205 (≤ 12.25) |
| support points | 1382 (24) | 2503 (24) | 219 (31) | 258 (31) |
| **support points + trumps** | **1431** (33) | **2551** (33) | 226 (40) | 266 (40) |
| raw HCP | 1219 (22) | 2228 (21) | 169 (29) | 199 (29) |
| raw HCP + 2 per trump | 1311 (39) | 2372 (39) | 170 (46) | 201 (46) |
| Zar | 1396 (52) | 2511 (51) | **229** (62) | **270** (62) |

"Support points + trumps" is the `pons` game gate. Its threshold of 33 is the
same number [major-game-threshold.md](major-game-threshold.md) found.

Who the best slam threshold selects, major fits:

| rule | cases | share with a void or a 7+ card suit | 12+ tricks, those hands | 12+ tricks, the rest |
| --- | --- | --- | --- | --- |
| NLTC ≤ 12.5 | 5.23M | 63% | 51% | 76% |
| support points ≥ 31 | 4.58M | 38% | 68% | 70% |

## Result 3 — what the trump suit owes NLTC

One regression of tricks on the NLTC total plus the terms below, all fitted
together. A term in losers is its trick value divided by the trick value of
one loser (0.851 in this regression). A positive number means NLTC counted too
few losers.

| term | tricks | losers to add |
| --- | --- | --- |
| shorter trump holding has 3 cards (against 4+) | −0.048 | +0.06 |
| shorter trump holding has 2 cards | −0.534 | +0.63 |
| shorter trump holding has 1 card | −1.435 | +1.69 |
| shorter trump holding is void | −2.819 | +3.31 |
| trump queen in a holding of 2 or fewer | +0.513 | −0.60 |
| singleton trump king | +0.859 | −1.01 |
| 9 trumps (against 8) | +0.424 | −0.50 |
| 10 trumps (against 8) | +0.561 | −0.66 |
| 11+ trumps (against 8) | +0.506 | −0.60 |
| trump jack held | +0.281 | −0.33 |

The same terms added to each scale:

| scale | sd before | sd after | R² after |
| --- | --- | --- | --- |
| NLTC | 1.094 | 1.016 | 0.750 |
| support points | 1.004 | 0.936 | 0.788 |
| raw HCP | 1.190 | 1.053 | 0.732 |
| Zar | 1.005 | 0.915 | 0.797 |

## Result 4 — the adjusted count as a hand rule

The terms of Result 3 rounded to simple numbers and applied one after another,
with nothing fitted:

| rule | tricks per loser | sd | AUC 10+ | AUC 12+ |
| --- | --- | --- | --- | --- |
| NLTC | −0.842 | 1.094 | 0.923 | 0.950 |
| + count the trump suit as if it had three cards | −0.866 | 1.057 | 0.929 | 0.953 |
| + subtract 0.5 for a 9th trump | −0.853 | 1.027 | 0.932 | 0.953 |
| + subtract 0.25 for the trump jack | −0.855 | 1.019 | 0.934 | 0.954 |

"As if it had three cards" means a missing trump king or queen is a loser
whatever the length of the holding. A small doubleton counts 3 losers instead
of 2.5, a small singleton 3 instead of 1.5, a void 3 instead of 0. Qx stays
at 2.5, and a singleton king counts 2 instead of 1.5.

## Result 5 — losers in one hand, cover cards in the other

The hand with longer trumps is **declarer** and counts NLTC. With equal
trumps, declarer is the hand with fewer losers. The other hand is **dummy**
and counts cover cards.

Summing NLTC over both hands is already a cover count. A suit of three or
more cards has 3 losers less 1.5, 1 and 0.5 for its ace, king and queen, and
a doubleton, singleton and void have 0.5, 1.5 and 3 fewer. So

    NLTC(declarer) + NLTC(dummy) = NLTC(declarer) + 12 − covers(dummy)

with the cover values of the middle column below. The right column is what
the deals say: one regression of tricks on declarer's NLTC and the listed
features of dummy's hand, each term divided by the trick value of one loser
of declarer (0.880).

| dummy holds | summed NLTC credits | losers covered, fitted |
| --- | --- | --- |
| side ace | 1.5 | 1.69 |
| side king, 2+ cards | 1 | 0.99 |
| side queen, 3+ cards | 0.5 | 0.44 |
| singleton side king | 0 | 0.50 |
| side queen, 2 or fewer cards | 0 | 0.33 |
| trump ace | 1.5 | 1.66 |
| trump king | 1 (singleton 0) | 1.13 |
| trump queen | 0.5 (2 or fewer cards 0) | 0.62 |
| trump jack, either hand | 0 | 0.34 |
| side void, 4+ trumps | 3 | 1.63 |
| side singleton, 4+ trumps | 1.5 | 0.92 |
| side doubleton, 4+ trumps | 0.5 | 0.30 |
| side void, 3 trumps | 3 | 0.96 |
| side singleton, 3 trumps | 1.5 | 0.55 |
| side doubleton, 3 trumps | 0.5 | 0.15 |
| side void, 2 or fewer trumps | 3 | 0.04 |
| side singleton, 2 or fewer trumps | 1.5 | −0.10 |
| side doubleton, 2 or fewer trumps | 0.5 | −0.12 |
| 9th trump | 0 | 0.49 |
| 10th trump | 0 | 0.17 |

The fit is tricks = 11.94 − 0.880 × (losers − covers), sd 0.954, R² 0.780.
With dummy's honours alone (no shortness, no trump length) the sd is 1.065.

Two counts built on this, with nothing fitted:

- **Hands apart.** Dummy counts cover cards without seeing declarer's hand:
  ace 1.5, king 1 (a singleton side king 0.5), queen 0.5. A side void,
  singleton and doubleton are 1.5, 1 and 0.5 with four or more trumps, 1, 0.5
  and 0 with three, and nothing with fewer. The count is declarer's NLTC less
  these.
- **Hands seen.** Declarer's losers that dummy's actual cards leave uncovered.
  NLTC charges a missing ace, king and queen to the first, second and third
  round of a suit. A loser is covered if dummy holds that honour, or if dummy
  is out of the side suit by that round and has a trump left to ruff with.
  An honour of dummy opposite a void, or a king opposite a singleton, covers
  nothing.

| count | tricks per loser | a | sd | mean miss, a void | mean miss, a 7+ card suit | AUC 10+ | AUC 12+ |
| --- | --- | --- | --- | --- | --- | --- | --- |
| NLTC, both hands summed | −0.842 | 22.11 | 1.094 | −0.77 | −0.38 | 0.923 | 0.950 |
| NLTC, adjusted (Result 4) | −0.855 | 22.05 | 1.019 | −0.80 | −0.18 | 0.934 | 0.954 |
| losers − cover cards, hands apart | −0.919 | 12.62 | 1.011 | −0.56 | +0.05 | 0.934 | 0.954 |
| + 9th trump −0.5, trump jack −0.25 | −0.903 | 12.19 | 0.977 | −0.60 | −0.08 | 0.938 | 0.955 |
| losers left uncovered, hands seen | −1.014 | 13.48 | 0.961 | −0.35 | +0.21 | 0.942 | 0.969 |
| + 9th trump −0.5, trump jack −0.25 | −1.006 | 13.04 | **0.900** | −0.42 | +0.05 | **0.949** | **0.971** |
| support points + trumps | +0.360 | −2.57 | 0.945 | +0.03 | −0.09 | 0.941 | 0.957 |

Decision value in mIMP per case, major fits, with the best threshold in
brackets (losers less covers, at most):

| count | 4M over 3M, not vul | 4M over 3M, vul | 6M over 4M, not vul | 6M over 4M, vul |
| --- | --- | --- | --- | --- |
| NLTC, both hands summed | 1368 (≤ 15.5) | 2490 (≤ 15.5) | 147 (≤ 12.5) | 170 (≤ 12.5) |
| NLTC, adjusted (Result 4) | 1401 (≤ 15) | 2536 (≤ 15.25) | 175 (≤ 12.25) | 205 (≤ 12.25) |
| losers − cover cards, hands apart | 1402 (≤ 3.5) | 2502 (≤ 4) | 202 (≤ 1) | 238 (≤ 1) |
| + 9th trump, trump jack | 1413 (≤ 3.25) | 2536 (≤ 3.5) | 201 (≤ 0.5) | 237 (≤ 0.5) |
| losers left uncovered, hands seen | 1448 (≤ 4) | 2571 (≤ 4) | 309 (≤ 2) | 363 (≤ 2) |
| + 9th trump, trump jack | **1475** (≤ 3.75) | **2627** (≤ 3.75) | **337** (≤ 1.5) | **398** (≤ 1.5) |
| support points + trumps | 1431 (33) | 2551 (33) | 226 (40) | 266 (40) |

## Conclusions

1. **With a fit, NLTC beats raw HCP and nothing else.** Its sd is 1.09 tricks
   against 1.19 for HCP, but 1.00 for support points and for Zar. Any point
   count that prices shortness is ahead of it, overall and in every shape row.
2. **NLTC is at its worst with very unbalanced hands.** With a void or a 7+
   card suit it is the noisiest of the four scales, behind even raw HCP, and
   it overvalues those hands by 0.77 and 0.38 tricks. The likely reasons: a
   void is zero losers whatever partner holds opposite it, and shortness in
   trumps is counted as if it could ruff (Result 3). NLTC is unbiased only in
   the middle, with a singleton.
3. **NLTC is not a slam tool.** It ranks slam hands almost as well as support
   points (AUC 0.950 against 0.953). But as a threshold it wins 147 mIMP per
   case not vulnerable, against 219 for support points, 229 for Zar and 169
   for raw HCP. A low loser count selects the hands NLTC overvalues: 63% of
   the pairs with 12½ losers or fewer hold a void or a 7+ card suit, and those
   take 12 tricks only 51% of the time, against 76% for the rest. The matching
   support-point threshold selects 38% such hands, and they make as often as
   the rest (68% against 70%).
4. **At the game decision NLTC is close to plain support points** (1368
   against 1382 mIMP per case not vulnerable) and behind the `pons` gate,
   support points + trumps (1431). The best thresholds match the textbook
   "25 − losers": game with 15½ losers or fewer, slam with 12½ or fewer.
5. **A short trump holding does need an adjustment, and it is the larger of
   the two.** The rule "count the trump suit as if it had three cards" adds
   0.5, 1.5 and 3 losers for a small doubleton, singleton and void. The data
   asks for 0.63, 1.69 and 3.31. A short queen gives back 0.60, so Qx needs
   no correction. A singleton king gives back 1.01 and still owes 0.68, close
   to the rule's 0.5 for the missing queen. Three trumps against four make no
   difference (0.06).
6. **The 9th trump is worth half a loser (0.50). Later trumps are worth almost
   nothing:** the 10th adds 0.16 and the 11th none. NLTC already counts the
   shortness that comes with the extra length, as the both-hands point count
   does in [major-game-threshold.md](major-game-threshold.md).
7. **The trump jack is worth a third of a loser (0.33).** NLTC ignores jacks.
8. **The adjusted count still trails.** The three adjustments take the sd
   from 1.094 to 1.019 and the game decision from 1368 to 1401 mIMP per case.
   That passes plain support points (1382) but not support points + trumps
   (1431), and at slam it stays behind every point count that prices
   shortness. The same trump terms improve every scale, so the gap does not
   close: 1.016 for NLTC against 0.936 for support points and 0.915 for Zar.
9. **Summed NLTC is a cover count with the wrong prices for shortness.**
   It credits dummy's void, singleton and doubleton with 3, 1.5 and 0.5
   losers in every suit. They cover 1.63, 0.92 and 0.30 with four trumps,
   0.96, 0.55 and 0.15 with three, and nothing with two or fewer. Dummy's
   honours are priced almost right (ace 1.69, king 0.99, queen 0.44 against
   1.5, 1, 0.5). The trump adjustments of Result 3 are the trump-suit part of
   the same repair: the shorter trump holding is dummy's.
10. **Counted apart, losers and cover cards beat summed NLTC but not the
    `pons` gate.** The sd falls from 1.094 to 1.011, and to 0.977 with the 9th
    trump and the trump jack, against 0.945 for support points + trumps. The
    gain is largest at slam (202 mIMP per case not vulnerable, against 147
    summed and 175 adjusted), most likely because dummy's shortness is no
    longer counted double; support points + trumps still wins 226. A hand with a 7+ card
    suit is no longer overvalued. A hand with a void still is, by 0.56
    tricks: declarer's own void counts no losers whatever dummy holds there.
11. **The non-additivity is real, and it is worth the most at slam.** Once
    dummy's cards are matched against declarer's actual losers, one uncovered
    loser is one trick (1.01; tricks = 13.5 − losers left), the sd is 0.961
    (0.900 with the 9th trump and the jack), and the count passes every blind
    scale here: 1448 against 1431 mIMP per case at game and 309 against 226
    at slam, 337 with the trump terms. No sum of two separate hand values can
    do this, since each hand must know which of its cards work. That is the
    information a splinter, a control bid and a keycard answer carry.
12. **Nothing changes in `pons` yet.** `support_point_count_in` already drops
    the ruffing value of a short trump holding, and the game gate already adds
    the trumps, so the blind cover count offers the gate nothing. NLTC stays
    an evaluator (`eval::NLTC`), not a gate. This agrees with the two engine
    A/Bs that tried it: the weak-two band and the slam entry (see
    [CHANGELOG.md](../CHANGELOG.md)). The matched count is a candidate for the
    slam decision after a splinter or a keycard answer, where one hand does
    know the other's shortness or honours. It needs its own A/B.

## Reproduce

[`examples/probe-nltc.rs`](../examples/probe-nltc.rs) writes the cell table;
[`scripts/nltc-report.py`](../scripts/nltc-report.py) prints every table above
from it.

```sh
cargo build --release --example probe-nltc
RAYON_NUM_THREADS=8 nice -n 19 target/release/examples/probe-nltc \
    /nfs2/jdh8/pons/22.pdd /nfs2/jdh8/pons/24.pdd /nfs2/jdh8/pons/shard-*.pdd \
    /mnt/hdd-data/jdh8/shards/shard-*.pdd > nltc.tsv
python3 scripts/nltc-report.py nltc.tsv
```

## Caveats

- Double-dummy play with the better declarer: no wrong-siding, and no
  single-dummy guessing by either side.
- Undoubled and uncontested, on the actual combined holdings. A bidder sees
  one hand and partner's shown range, not the total.
- Trumps are the longest combined suit. A pair with a 4-4 major fit and a
  longer minor is counted in the minor.
- The slam rows count 12 tricks as a slam. No control check is modelled, so
  they say how well a scale finds the tricks, not how well it avoids two
  quick losers.
- The terms of Result 3 are fitted together and are correlated with shape: a
  short trump holding opposite an 8+ card fit implies a long suit opposite.
- In Result 5 the roles are fixed by trump length alone. The "hands seen"
  count hands out dummy's ruffs suit by suit, charges nothing for drawing
  trumps, and gives no credit for discards on dummy's long suit. It is a
  simple rule with both hands in view, not an upper bound. Support points
  have no such version in this study, so its rows are compared with blind
  scales only.
- `examples/eval-calibrate` drops cases that take fewer than 6 tricks. With
  that filter the NLTC line is 20.9 − 0.76 × losers and the 9th trump is
  worth 0.43 losers. The other terms move by 0.1 or less.
