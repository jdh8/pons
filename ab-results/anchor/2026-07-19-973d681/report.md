=== arm 0: our american-instinct floor (us) vs BBA 2/1 Game Force (them), vulnerability none, 204800 boards ===
replay verification: 100.00% of 2127573 our-side calls (0 mismatched)
auction-divergent: 185759 (91%), contract-divergent: 148947 (73%)
plain DD: -1.3001 IMPs/board (95% CI [-1.3219, -1.2784]), -266269 IMPs total
perfect defense: -1.3902 IMPs/board (95% CI [-1.4160, -1.3645])
gen_args: --count 6400 --seed 1783375064 --output ab-results/anchor/2026-07-19-973d681/none/shard-0.json -v none --our-floor american-instinct

=== arm 1: our american-instinct floor (us) vs BBA 2/1 Game Force (them), vulnerability both, 204800 boards ===
replay verification: 100.00% of 2113723 our-side calls (0 mismatched)
auction-divergent: 184956 (90%), contract-divergent: 147363 (72%)
plain DD: -1.6998 IMPs/board (95% CI [-1.7276, -1.6721]), -348124 IMPs total
perfect defense: -1.9755 IMPs/board (95% CI [-2.0079, -1.9431])
gen_args: --count 6400 --seed 1783375064 --output ab-results/anchor/2026-07-19-973d681/both/shard-0.json -v both --our-floor american-instinct


## IMP histogram (plain, per contract-divergent board)

right-siding-only divergences (same contract, different auction): 74405

  -22 IMPs: 5
  -21 IMPs: 41
  -20 IMPs: 50
  -19 IMPs: 151
  -18 IMPs: 288
  -17 IMPs: 989
  -16 IMPs: 1050
  -15 IMPs: 2108
  -14 IMPs: 3529
  -13 IMPs: 8831
  -12 IMPs: 7674
  -11 IMPs: 12710
  -10 IMPs: 15543
   -9 IMPs: 7725
   -8 IMPs: 6319
   -7 IMPs: 11667
   -6 IMPs: 17666
   -5 IMPs: 16088
   -4 IMPs: 7595
   -3 IMPs: 11839
   -2 IMPs: 12995
   -1 IMPs: 11459
   +0 IMPs: 51573
   +1 IMPs: 10255
   +2 IMPs: 9730
   +3 IMPs: 8054
   +4 IMPs: 5835
   +5 IMPs: 13736
   +6 IMPs: 12275
   +7 IMPs: 5812
   +8 IMPs: 2301
   +9 IMPs: 2324
  +10 IMPs: 6509
  +11 IMPs: 4810
  +12 IMPs: 2809
  +13 IMPs: 3256
  +14 IMPs: 491
  +15 IMPs: 98
  +16 IMPs: 55
  +17 IMPs: 58
  +18 IMPs: 5
  +19 IMPs: 2

## Ranked buckets (phase / provenance / family), losses first

| bucket | boards | net plain IMPs | IMPs/divergent ±CI | net PD IMPs | flag |
| --- | --- | --- | --- | --- | --- |
| Defensive / book / round-1 | 59098 | -111127 | -1.88 ±0.05 | -135156 | |
| Constructive / book / opening | 50993 | -86728 | -1.70 ±0.06 | -88698 | |
| Constructive / book / round-2 | 37470 | -64692 | -1.73 ±0.07 | -72218 | |
| Constructive / book / round-1 | 24459 | -49912 | -2.04 ±0.09 | -56109 | |
| Defensive / floor#3 / round-2 | 8861 | -28219 | -3.18 ±0.13 | -24816 | |
| Competitive / fallback@1 / round-1 | 9177 | -24201 | -2.64 ±0.13 | -25269 | |
| Defensive / floor#3 / round-1 | 7419 | -23153 | -3.12 ±0.16 | -15528 | |
| Competitive / fallback@2 / round-1 | 8054 | -21439 | -2.66 ±0.14 | -23756 | |
| Competitive / fallback@3 / round-2 | 5933 | -13015 | -2.19 ±0.16 | -14270 | |
| Competitive / floor#3 / round-2 | 3724 | -10516 | -2.82 ±0.21 | -8535 | |
| Competitive / fallback@4 / round-2 | 2498 | -7340 | -2.94 ±0.25 | -8749 | |
| Defensive / book / round-2 | 4871 | -6183 | -1.27 ±0.18 | -8355 | |
| Competitive / fallback@3 / round-1 | 1756 | -5751 | -3.28 ±0.32 | -6590 | |
| Defensive / floor#246 / round-1 | 1163 | -5560 | -4.78 ±0.45 | -5701 | |
| Constructive / floor#3 / round-2 | 2538 | -5299 | -2.09 ±0.23 | -4415 | |
| Competitive / floor#245 / round-2 | 1740 | -5273 | -3.03 ±0.30 | -7990 | |
| Defensive / floor#3 / balancing | 2089 | -4882 | -2.34 ±0.22 | -2037 | |
| Constructive / floor#140 / round-2 | 1820 | -4873 | -2.68 ±0.39 | -5105 | |
| Defensive / floor#245 / round-1 | 1613 | -4826 | -2.99 ±0.36 | -5703 | |
| Defensive / floor#60 / round-2 | 1705 | -4688 | -2.75 ±0.28 | -6789 | |
| Competitive / floor#3 / round-1 | 983 | -4086 | -4.16 ±0.44 | -1399 | |
| Defensive / floor#20 / round-1 | 1562 | -3861 | -2.47 ±0.32 | -5018 | |
| Defensive / floor#45 / round-2 | 1353 | -3801 | -2.81 ±0.31 | -5052 | |
| Constructive / floor#3 / round-1 | 1125 | -3760 | -3.34 ±0.41 | -1684 | |
| Constructive / book / deep | 4594 | -3752 | -0.82 ±0.18 | -4669 | |
| Defensive / floor#245 / round-2 | 902 | -3520 | -3.90 ±0.45 | -5507 | |
| Defensive / floor#35 / round-1 | 1628 | -3259 | -2.00 ±0.31 | -4668 | |
| Competitive / fallback@1 / round-2 | 1167 | -2986 | -2.56 ±0.39 | -2971 | |
| Defensive / floor#50 / round-1 | 2025 | -2847 | -1.41 ±0.29 | -4192 | |
| Competitive / floor#3 / balancing | 1471 | -2770 | -1.88 ±0.28 | +393 | plain/PD-flip |
| Competitive / fallback@2 / round-2 | 1052 | -2635 | -2.50 ±0.42 | -2428 | |
| Competitive / floor#245+rb / round-2 | 336 | -2588 | -7.70 ±0.71 | -2920 | |
| Defensive / floor#20 / round-2 | 1344 | -2578 | -1.92 ±0.32 | -3890 | |
| Defensive / floor#45 / round-1 | 853 | -2305 | -2.70 ±0.43 | -2636 | |
| Defensive / floor#64 / round-1 | 1139 | -2239 | -1.97 ±0.40 | -2364 | |
| Defensive / floor#60 / round-1 | 632 | -2207 | -3.49 ±0.47 | -2787 | |
| Constructive / floor#61 / deep | 921 | -2049 | -2.22 ±0.20 | -3361 | |
| Defensive / floor#35 / round-2 | 1208 | -1927 | -1.60 ±0.35 | -3335 | |
| Competitive / floor#46 / round-2 | 687 | -1912 | -2.78 ±0.44 | -2601 | |
| Competitive / floor#246 / round-2 | 550 | -1824 | -3.32 ±0.54 | -2751 | |
| Defensive / floor#246 / balancing | 596 | -1805 | -3.03 ±0.58 | -2482 | |
| Constructive / floor#3 / deep | 1274 | -1709 | -1.34 ±0.37 | -1331 | |
| Competitive / floor#31 / round-1 | 338 | -1702 | -5.04 ±0.73 | -1277 | |
| Defensive / floor#61 / round-2 | 528 | -1660 | -3.14 ±0.49 | -1940 | |
| Defensive / floor#132 / round-1 | 446 | -1639 | -3.67 ±0.52 | -2939 | |
| Defensive / floor#245 / balancing | 1356 | -1521 | -1.12 ±0.35 | -3481 | |
| Defensive / floor#202 / round-2 | 371 | -1362 | -3.67 ±0.66 | -2064 | |
| Defensive / floor#202 / round-1 | 359 | -1342 | -3.74 ±0.69 | -1661 | |
| Competitive / floor#30 / round-2 | 493 | -1336 | -2.71 ±0.48 | -1874 | |
| Competitive / floor#61 / round-1 | 182 | -1328 | -7.30 ±0.88 | -1329 | |
| Defensive / floor#246 / round-2 | 394 | -1322 | -3.36 ±0.72 | -2378 | |
| Constructive / fallback@5 / deep | 377 | -1273 | -3.38 ±0.75 | -1379 | |
| Competitive / floor#245 / balancing | 674 | -1264 | -1.88 ±0.49 | -2560 | |
| Competitive / floor#46 / round-1 | 211 | -1211 | -5.74 ±0.94 | -1156 | |
| Defensive / floor#50 / round-2 | 840 | -1197 | -1.43 ±0.39 | -2092 | |
| Competitive / floor#6 / round-2 | 357 | -1186 | -3.32 ±0.60 | -2222 | |
| Constructive / fallback@4 / deep | 439 | -1176 | -2.68 ±0.64 | -1310 | |
| Defensive / floor#65 / round-1 | 997 | -1122 | -1.13 ±0.39 | -1912 | |
| Defensive / floor#30 / round-2 | 377 | -981 | -2.60 ±0.56 | -1594 | |
| Defensive / floor#46 / round-2 | 434 | -962 | -2.22 ±0.55 | -1177 | |
| Competitive / floor#16 / round-1 | 117 | -901 | -7.70 ±1.16 | -926 | |
| Competitive / floor#246 / balancing | 370 | -868 | -2.35 ±0.68 | -1778 | |
| Constructive / floor#145 / round-2 | 178 | -857 | -4.81 ±1.25 | -880 | |
| Defensive / floor#30 / round-1 | 240 | -842 | -3.51 ±0.75 | -846 | |
| Defensive / floor#20 / balancing | 709 | -836 | -1.18 ±0.38 | -1772 | |
| Competitive / floor#5 / round-2 | 348 | -807 | -2.32 ±0.73 | -1551 | |
| Defensive / floor#131 / balancing | 382 | -800 | -2.09 ±0.55 | -1357 | |
| Constructive / floor#46 / deep | 486 | -785 | -1.62 ±0.38 | -1537 | |
| Defensive / floor#132 / balancing | 276 | -726 | -2.63 ±0.53 | -1742 | |
| Defensive / floor#3 / deep | 224 | -714 | -3.19 ±0.82 | -552 | |
| Constructive / floor#32 / round-1 | 158 | -685 | -4.34 ±0.97 | -638 | |
| Constructive / floor#61 / round-2 | 316 | -656 | -2.08 ±0.68 | -891 | |
| Competitive / floor#237 / round-2 | 190 | -644 | -3.39 ±0.84 | -941 | |
| Competitive / floor#2 / round-2 | 305 | -613 | -2.01 ±0.67 | -1413 | |
| Constructive / floor#140 / deep | 521 | -591 | -1.13 ±0.58 | -650 | |
| Competitive / floor#240 / balancing | 213 | -590 | -2.77 ±0.91 | -594 | |
| Defensive / floor#64 / round-2 | 353 | -579 | -1.64 ±0.68 | -901 | |
| Defensive / floor#31 / round-2 | 215 | -574 | -2.67 ±0.82 | -551 | |
| Defensive / floor#21 / round-1 | 225 | -569 | -2.53 ±0.92 | -973 | |
| Defensive / floor#35 / balancing | 488 | -567 | -1.16 ±0.45 | -1310 | |
| Defensive / floor#66 / round-1 | 190 | -551 | -2.90 ±1.00 | -733 | |
| Constructive / floor#17 / round-1 | 130 | -550 | -4.23 ±1.16 | -436 | |
| Defensive / floor#200 / round-1 | 184 | -538 | -2.92 ±1.04 | -707 | |
| Defensive / floor#197 / round-1 | 200 | -533 | -2.67 ±0.92 | -621 | |
| Defensive / floor#131 / round-1 | 155 | -526 | -3.39 ±0.99 | -835 | |
| Competitive / book+rb / round-2 | 343 | -518 | -1.51 ±0.55 | -843 | |
| Defensive / floor#51 / round-1 | 264 | -495 | -1.88 ±0.82 | -488 | |
| Defensive / floor#200 / round-2 | 202 | -467 | -2.31 ±0.97 | -730 | |
| Constructive / floor#153 / round-2 | 128 | -466 | -3.64 ±1.72 | -444 | |
| Defensive / floor#63 / round-2 | 84 | -466 | -5.55 ±1.44 | -342 | |
| Defensive / floor#48 / round-2 | 77 | -462 | -6.00 ±1.44 | -513 | |
| Competitive / floor#3+rb / round-2 | 163 | -446 | -2.74 ±0.89 | -381 | |
| Defensive / floor#133 / round-1 | 321 | -441 | -1.37 ±0.92 | -903 | |
| Defensive / floor#16 / round-2 | 142 | -438 | -3.08 ±1.03 | -444 | |
| Competitive / floor#240 / round-2 | 183 | -436 | -2.38 ±1.00 | -404 | |
| Constructive / floor#46 / round-2 | 427 | -426 | -1.00 ±0.61 | -525 | |
| Competitive / floor#245 / round-1 | 195 | -412 | -2.11 ±0.84 | -846 | |
| Competitive / floor#239 / round-2 | 96 | -392 | -4.08 ±1.16 | -503 | |
| Constructive / floor#147 / round-2 | 83 | -357 | -4.30 ±1.95 | -361 | |
| Defensive / floor#199 / round-1 | 212 | -356 | -1.68 ±0.78 | -479 | |
| Competitive / floor#25 / round-2 | 196 | -353 | -1.80 ±0.75 | -403 | |
| Defensive / floor#198 / round-1 | 152 | -353 | -2.32 ±1.19 | -504 | |
| Defensive / floor#49 / round-1 | 185 | -350 | -1.89 ±0.95 | -209 | |
| Constructive / floor#31 / round-2 | 69 | -344 | -4.99 ±1.45 | -311 | |
| Competitive / floor#61 / round-2 | 126 | -339 | -2.69 ±1.15 | -520 | |
| Defensive / floor#198 / round-2 | 103 | -331 | -3.21 ±1.22 | -464 | |
| Competitive / floor#30 / balancing | 122 | -330 | -2.70 ±0.89 | -635 | |
| Competitive / floor#3 / deep | 304 | -327 | -1.08 ±0.67 | -201 | |
| Competitive / floor#10 / round-2 | 130 | -322 | -2.48 ±0.91 | -290 | |
| Defensive / floor#63 / round-1 | 65 | -306 | -4.71 ±1.54 | -237 | |
| Constructive / floor#16 / round-2 | 51 | -304 | -5.96 ±1.59 | -293 | |
| Competitive / floor#60 / round-2 | 141 | -299 | -2.12 ±1.09 | -223 | |
| Defensive / floor#153 / round-1 | 116 | -296 | -2.55 ±1.63 | -184 | |
| Defensive / floor#36 / round-1 | 165 | -296 | -1.79 ±0.98 | -659 | |
| Competitive / floor#241 / balancing | 89 | -289 | -3.25 ±1.24 | -439 | |
| Defensive / floor#204 / round-1 | 78 | -283 | -3.63 ±1.48 | -266 | |
| Defensive / floor#65 / balancing | 229 | -277 | -1.21 ±0.60 | -1033 | |
| Competitive / floor#235 / round-2 | 95 | -272 | -2.86 ±1.22 | -423 | |
| Competitive / fallback@5 / round-2 | 97 | -268 | -2.76 ±1.19 | -174 | |
| Competitive / floor#16 / round-2 | 72 | -268 | -3.72 ±1.61 | -265 | |
| Competitive / floor#15 / round-2 | 57 | -248 | -4.35 ±1.30 | -333 | |
| Competitive / floor#31 / round-2 | 63 | -246 | -3.90 ±1.50 | -283 | |
| Competitive / floor#15 / balancing | 75 | -241 | -3.21 ±1.15 | -409 | |
| Defensive / floor#205 / round-1 | 104 | -237 | -2.28 ±1.26 | -195 | |
| Defensive / floor#197 / round-2 | 83 | -235 | -2.83 ±1.42 | -332 | |
| Defensive / floor#32 / round-1 | 50 | -231 | -4.62 ±2.05 | -170 | |
| Defensive / floor#237 / round-2 | 80 | -227 | -2.84 ±1.39 | -317 | |
| Competitive / floor#241 / round-2 | 68 | -226 | -3.32 ±1.56 | -263 | |
| Competitive / floor#236 / balancing | 110 | -225 | -2.05 ±1.15 | -283 | |
| Competitive / floor#246+rb / round-2 | 61 | -220 | -3.61 ±1.77 | -376 | |
| Defensive / floor#65 / round-2 | 380 | -218 | -0.57 ±0.62 | -692 | ~noise |
| Competitive / floor#31 / balancing | 47 | -217 | -4.62 ±1.54 | -278 | |
| Competitive / floor#9 / round-2 | 145 | -216 | -1.49 ±0.79 | -280 | |
| Defensive / floor#147 / round-1 | 92 | -206 | -2.24 ±1.71 | -83 | |
| Competitive / floor#238 / balancing | 131 | -202 | -1.54 ±1.20 | -154 | |
| Defensive / floor#17 / round-1 | 62 | -191 | -3.08 ±1.59 | -152 | |
| Competitive / floor#16 / balancing | 66 | -190 | -2.88 ±1.43 | -330 | |
| Defensive / floor#235 / round-2 | 55 | -190 | -3.45 ±1.69 | -243 | |
| Competitive / floor#234 / round-2 | 45 | -183 | -4.07 ±1.74 | -234 | |
| Competitive / floor#147 / round-1 | 59 | -181 | -3.07 ±2.34 | -191 | |
| Defensive / floor#27 / round-2 | 36 | -168 | -4.67 ±1.69 | -199 | |
| Competitive / floor#234+rb / round-2 | 19 | -166 | -8.74 ±2.84 | -197 | |
| Competitive / floor#147 / round-2 | 147 | -165 | -1.12 ±1.20 | -356 | ~noise |
| Defensive / floor#50 / balancing | 412 | -164 | -0.40 ±0.45 | -968 | ~noise |
| Defensive / floor#49 / balancing | 187 | -160 | -0.86 ±0.99 | -406 | ~noise |
| Defensive / floor#32 / round-2 | 56 | -151 | -2.70 ±1.49 | -156 | |
| Competitive / floor#32 / round-1 | 23 | -145 | -6.30 ±2.59 | -129 | |
| Defensive / floor#61 / round-1 | 43 | -145 | -3.37 ±1.87 | -160 | |
| Defensive / floor#17 / round-2 | 74 | -136 | -1.84 ±1.05 | -194 | |
| Defensive / floor#42 / round-2 | 34 | -136 | -4.00 ±3.32 | -111 | |
| Defensive / floor#129 / round-2 | 148 | -134 | -0.91 ±1.25 | -357 | ~noise |
| Competitive / floor#153 / round-1 | 37 | -132 | -3.57 ±2.96 | -132 | |
| Defensive / floor#18 / round-2 | 28 | -130 | -4.64 ±2.63 | -93 | |
| Competitive / floor#238 / round-2 | 75 | -129 | -1.72 ±1.68 | -115 | |
| Competitive / floor#236+rb / round-2 | 34 | -127 | -3.74 ±2.55 | -132 | |
| Constructive / floor#140 / round-1 | 26 | -127 | -4.88 ±4.45 | -127 | |
| Competitive / floor#140 / round-2 | 65 | -126 | -1.94 ±1.99 | -170 | ~noise |
| Competitive / floor#61 / deep | 45 | -125 | -2.78 ±1.56 | -218 | |
| Defensive / floor#203 / round-1 | 83 | -122 | -1.47 ±1.40 | -127 | |
| Defensive / floor#204 / round-2 | 51 | -121 | -2.37 ±1.34 | -104 | |
| Defensive / floor#48 / round-1 | 50 | -121 | -2.42 ±1.95 | -89 | |
| Defensive / floor#51 / balancing | 50 | -121 | -2.42 ±1.43 | -159 | |
| Defensive / floor#133 / balancing | 84 | -116 | -1.38 ±1.66 | -277 | ~noise |
| Defensive / floor#66 / balancing | 23 | -116 | -5.04 ±2.29 | -141 | |
| Competitive / floor#246 / round-1 | 47 | -115 | -2.45 ±1.64 | -219 | |
| Defensive / floor#238 / round-2 | 22 | -115 | -5.23 ±2.37 | -144 | |
| Defensive / floor#239 / round-2 | 55 | -114 | -2.07 ±1.91 | -155 | |
| Competitive / floor#46+rb / deep | 38 | -111 | -2.92 ±0.96 | -229 | |
| Competitive / floor#47 / round-2 | 86 | -108 | -1.26 ±1.49 | -323 | ~noise |
| Competitive / floor#237 / balancing | 56 | -107 | -1.91 ±1.24 | -260 | |
| Constructive / floor#153 / deep | 123 | -105 | -0.85 ±1.20 | -328 | ~noise |
| Competitive / floor#140 / round-1 | 38 | -102 | -2.68 ±2.92 | -100 | ~noise |
| Defensive / floor#33 / round-2 | 36 | -98 | -2.72 ±2.22 | -102 | |
| Constructive / floor#157 / round-2 | 78 | -96 | -1.23 ±1.66 | -104 | ~noise |
| Constructive / floor#47 / round-2 | 8 | -96 | -12.00 ±0.74 | -96 | |
| Competitive / floor#57 / round-2 | 16 | -93 | -5.81 ±3.99 | -90 | |
| Constructive / floor#47 / round-1 | 19 | -92 | -4.84 ±3.60 | -91 | |
| Defensive / floor#239 / balancing | 27 | -91 | -3.37 ±2.32 | -125 | |
| Competitive / floor#33 / round-2 | 25 | -90 | -3.60 ±2.17 | -139 | |
| Competitive / floor#236 / round-2 | 33 | -89 | -2.70 ±2.39 | -111 | |
| Competitive / floor#239 / balancing | 62 | -88 | -1.42 ±1.13 | -211 | |
| Competitive / floor#245 / deep | 19 | -88 | -4.63 ±3.32 | -152 | |
| Defensive / floor#245 / deep | 19 | -88 | -4.63 ±2.74 | -163 | |
| Competitive / floor#63 / round-2 | 30 | -87 | -2.90 ±2.06 | -116 | |
| Competitive / floor#153 / round-2 | 107 | -86 | -0.80 ±1.34 | -261 | ~noise |
| Competitive / floor#17 / round-2 | 38 | -86 | -2.26 ±1.68 | -98 | |
| Defensive / floor#12 / round-2 | 40 | -85 | -2.12 ±1.90 | -74 | |
| Defensive / floor#36 / balancing | 38 | -81 | -2.13 ±2.15 | -176 | ~noise |
| Competitive / floor#60 / balancing | 45 | -80 | -1.78 ±1.75 | -71 | |
| Constructive / floor#30 / round-1 | 24 | -79 | -3.29 ±2.50 | -111 | |
| Competitive / floor#32 / round-2 | 53 | -78 | -1.47 ±1.53 | -127 | ~noise |
| Constructive / floor#63 / round-2 | 14 | -78 | -5.57 ±0.61 | -136 | |
| Competitive / floor#17 / round-1 | 13 | -77 | -5.92 ±4.19 | -46 | |
| Defensive / book / deep | 44 | -76 | -1.73 ±1.38 | -102 | |
| Defensive / floor#11 / round-2 | 20 | -76 | -3.80 ±1.69 | -114 | |
| Defensive / floor#31 / round-1 | 14 | -75 | -5.36 ±3.21 | -87 | |
| Competitive / floor#47 / balancing | 44 | -74 | -1.68 ±1.96 | -113 | ~noise |
| Defensive / floor#129 / round-1 | 55 | -74 | -1.35 ±1.69 | -160 | ~noise |
| Competitive / floor#30 / deep | 33 | -72 | -2.18 ±1.54 | -107 | |
| Constructive / floor#32 / deep | 59 | -72 | -1.22 ±0.56 | -91 | |
| Competitive / floor#1 / round-2 | 515 | -71 | -0.14 ±0.59 | +312 | ~noise plain/PD-flip |
| Competitive / floor#24 / round-2 | 46 | -71 | -1.54 ±1.30 | -76 | |
| Constructive / floor#17 / deep | 66 | -71 | -1.08 ±0.57 | -84 | |
| Defensive / floor#34 / balancing | 156 | -71 | -0.46 ±0.95 | -192 | ~noise |
| Competitive / floor#235+rb / round-2 | 10 | -70 | -7.00 ±3.41 | -54 | |
| Competitive / floor#42 / round-2 | 12 | -69 | -5.75 ±5.26 | -83 | |
| Constructive / floor#157 / round-1 | 6 | -69 | -11.50 ±1.21 | -69 | |
| Defensive / floor#26 / round-2 | 7 | -68 | -9.71 ±2.99 | -84 | |
| Constructive / floor#31 / deep | 16 | -66 | -4.12 ±2.82 | -65 | |
| Competitive / floor#239+rb / round-2 | 10 | -65 | -6.50 ±2.25 | -74 | |
| Defensive / floor#241 / deep | 10 | -64 | -6.40 ±4.28 | -71 | |
| Competitive / floor#12 / round-2 | 24 | -63 | -2.62 ±1.77 | -93 | |
| Competitive / floor#129 / round-2 | 22 | -61 | -2.77 ±3.36 | -110 | ~noise |
| Competitive / floor#46 / deep | 50 | -61 | -1.22 ±1.79 | -136 | ~noise |
| Competitive / floor#151 / round-2 | 22 | -59 | -2.68 ±3.71 | -70 | ~noise |
| Constructive / floor#147 / deep | 130 | -58 | -0.45 ±1.28 | -181 | ~noise |
| Competitive / floor#3+rb / balancing | 15 | -57 | -3.80 ±2.78 | -59 | |
| Defensive / floor#16 / round-1 | 8 | -57 | -7.12 ±3.75 | -64 | |
| Defensive / floor#41 / round-2 | 43 | -57 | -1.33 ±2.51 | -81 | ~noise |
| Constructive / floor#147 / round-1 | 31 | -56 | -1.81 ±3.47 | -52 | ~noise |
| Competitive / book+rb / deep | 65 | -54 | -0.83 ±0.90 | -91 | ~noise |
| Defensive / floor#21 / balancing | 43 | -54 | -1.26 ±1.44 | -157 | ~noise |
| Competitive / floor#129 / deep | 13 | -53 | -4.08 ±5.19 | -68 | ~noise |
| Competitive / floor#234 / balancing | 37 | -51 | -1.38 ±1.83 | -84 | ~noise |
| Competitive / floor#5 / deep | 14 | -51 | -3.64 ±3.24 | -80 | |
| Defensive / floor#140 / round-2 | 6 | -51 | -8.50 ±2.13 | -64 | |
| Competitive / floor#30+rb / round-2 | 6 | -49 | -8.17 ±3.05 | -52 | |
| Defensive / floor#241 / round-2 | 27 | -49 | -1.81 ±2.41 | -75 | ~noise |
| Competitive / floor#235 / balancing | 18 | -48 | -2.67 ±2.47 | -79 | |
| Competitive / floor#33 / round-1 | 10 | -48 | -4.80 ±3.03 | -82 | |
| Constructive / floor#153 / round-1 | 20 | -46 | -2.30 ±4.10 | -44 | ~noise |
| Defensive / floor#36 / round-2 | 31 | -45 | -1.45 ±1.94 | -114 | ~noise |
| Defensive / floor#147 / round-2 | 14 | -43 | -3.07 ±5.16 | -46 | ~noise |
| Defensive / floor#246 / deep | 11 | -43 | -3.91 ±3.65 | -91 | |
| Defensive / floor#203 / round-2 | 63 | -42 | -0.67 ±1.57 | -27 | ~noise |
| Defensive / floor#55 / round-2 | 14 | -42 | -3.00 ±3.38 | -36 | ~noise |
| Competitive / floor#237+rb / round-2 | 19 | -41 | -2.16 ±3.13 | -47 | ~noise |
| Competitive / floor#240+rb / round-2 | 45 | -41 | -0.91 ±1.82 | +12 | ~noise plain/PD-flip |
| Defensive / floor#33 / round-1 | 17 | -41 | -2.41 ±4.27 | -34 | ~noise |
| Competitive / floor#147 / balancing | 6 | -39 | -6.50 ±6.21 | -40 | |
| Competitive / floor#238+rb / round-2 | 35 | -38 | -1.09 ±1.93 | +17 | ~noise plain/PD-flip |
| Defensive / floor#29 / round-1 | 7 | -38 | -5.43 ±5.69 | -36 | ~noise |
| Competitive / fallback@6 / round-2 | 11 | -37 | -3.36 ±4.96 | -19 | ~noise |
| Defensive / floor#18 / round-1 | 9 | -37 | -4.11 ±5.21 | -31 | ~noise |
| Competitive / floor#11 / round-2 | 6 | -36 | -6.00 ±4.61 | -52 | |
| Competitive / floor#140 / deep | 4 | -36 | -9.00 ±2.53 | -46 | |
| Competitive / floor#241 / deep | 18 | -36 | -2.00 ±2.96 | -37 | ~noise |
| Competitive / floor#244 / round-2 | 22 | -36 | -1.64 ±2.21 | -66 | ~noise |
| Competitive / floor#60+rb / round-2 | 6 | -36 | -6.00 ±3.92 | -44 | |
| Defensive / floor#143 / round-1 | 5 | -36 | -7.20 ±4.22 | -39 | |
| Defensive / floor#61 / deep | 22 | -36 | -1.64 ±2.16 | -118 | ~noise |
| Competitive / floor#61+rb / deep | 9 | -35 | -3.89 ±2.34 | -49 | |
| Constructive / floor#151 / round-2 | 147 | -35 | -0.24 ±1.51 | -51 | ~noise |
| Competitive / book+rb / round-1 | 4 | -34 | -8.50 ±5.09 | -59 | |
| Competitive / floor#45 / round-2 | 73 | -33 | -0.45 ±1.40 | -53 | ~noise |
| Defensive / floor#40 / round-2 | 4 | -32 | -8.00 ±2.26 | -32 | |
| Defensive / floor#54 / deep | 5 | -32 | -6.40 ±4.05 | -40 | |
| Competitive / floor#143 / round-2 | 5 | -31 | -6.20 ±4.74 | -33 | |
| Competitive / floor#16 / deep | 9 | -31 | -3.44 ±2.40 | -36 | |
| Defensive / floor#143 / round-2 | 4 | -31 | -7.75 ±3.03 | -35 | |
| Competitive / floor#32 / deep | 14 | -30 | -2.14 ±1.69 | -29 | |
| Constructive / floor#16 / deep | 4 | -30 | -7.50 ±2.47 | -38 | |
| Defensive / floor#14 / round-2 | 3 | -30 | -10.00 ±2.26 | -30 | |
| Competitive / floor#3+rb / deep | 19 | -29 | -1.53 ±2.85 | -54 | ~noise |
| Constructive / floor#62 / round-1 | 25 | -29 | -1.16 ±2.92 | -39 | ~noise |
| Competitive / floor#45 / balancing | 14 | -28 | -2.00 ±4.34 | +1 | ~noise plain/PD-flip |
| Competitive / floor#18 / round-2 | 13 | -27 | -2.08 ±3.23 | -17 | ~noise |
| Competitive / floor#45+rb / round-2 | 4 | -27 | -6.75 ±4.83 | -52 | |
| Competitive / floor#62 / round-1 | 2 | -27 | -13.50 ±0.98 | -28 | |
| Competitive / floor#17 / deep | 12 | -26 | -2.17 ±2.95 | -39 | ~noise |
| Competitive / floor#48 / round-2 | 31 | -26 | -0.84 ±2.78 | -123 | ~noise |
| Competitive / floor#57 / deep | 2 | -26 | -13.00 ±1.96 | -29 | |
| Defensive / floor#17 / deep | 3 | -26 | -8.67 ±2.36 | -27 | |
| Competitive / fallback@4+rb / round-2 | 6 | -25 | -4.17 ±1.55 | -23 | |
| Competitive / floor#39 / round-2 | 15 | -25 | -1.67 ±2.51 | -26 | ~noise |
| Competitive / floor#62 / deep | 20 | -25 | -1.25 ±2.83 | -58 | ~noise |
| Competitive / floor#62+rb / round-2 | 6 | -25 | -4.17 ±7.36 | -13 | ~noise |
| Defensive / floor#144 / round-1 | 2 | -25 | -12.50 ±0.98 | -25 | |
| Defensive / floor#49 / round-2 | 77 | -25 | -0.32 ±1.30 | -63 | ~noise |
| Defensive / floor#61 / balancing | 27 | -25 | -0.93 ±1.87 | -70 | ~noise |
| Competitive / floor#60 / deep | 5 | -24 | -4.80 ±5.93 | -30 | ~noise |
| Competitive / floor#63 / round-1 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Constructive / floor#145 / round-1 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Constructive / floor#148 / round-2 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Constructive / floor#157 / deep | 34 | -24 | -0.71 ±2.09 | -17 | ~noise |
| Constructive / floor#159 / deep | 2 | -24 | -12.00 ±1.96 | -24 | |
| Constructive / floor#160 / deep | 2 | -24 | -12.00 ±1.96 | -24 | |
| Constructive / floor#63 / deep | 4 | -22 | -5.50 ±0.57 | -28 | |
| Defensive / floor#13 / round-2 | 3 | -22 | -7.33 ±7.95 | -22 | ~noise |
| Defensive / floor#29 / round-2 | 7 | -22 | -3.14 ±5.58 | -15 | ~noise |
| Defensive / floor#64 / balancing | 217 | -22 | -0.10 ±0.79 | -314 | ~noise |
| Defensive / floor#240 / deep | 2 | -21 | -10.50 ±2.94 | -21 | |
| Competitive / fallback@2 / balancing | 2 | -20 | -10.00 ±1.96 | -20 | |
| Competitive / floor#237 / deep | 2 | -20 | -10.00 ±3.92 | -20 | |
| Competitive / floor#241+rb / balancing | 7 | -20 | -2.86 ±4.41 | -20 | ~noise |
| Competitive / floor#46+rb / round-2 | 6 | -20 | -3.33 ±3.27 | -44 | |
| Competitive / floor#61 / balancing | 13 | -20 | -1.54 ±3.78 | -35 | ~noise |
| Constructive / floor#154 / deep | 18 | -20 | -1.11 ±3.08 | -11 | ~noise |
| Defensive / floor#235 / deep | 4 | -20 | -5.00 ±4.16 | -32 | |
| Competitive / floor#48 / round-1 | 5 | -19 | -3.80 ±7.10 | -8 | ~noise |
| Constructive / floor#155 / round-2 | 4 | -19 | -4.75 ±8.25 | -12 | ~noise |
| Defensive / floor#140 / balancing | 10 | -19 | -1.90 ±4.20 | -22 | ~noise |
| Competitive / floor#241+rb / round-2 | 3 | -18 | -6.00 ±11.81 | -16 | ~noise |
| Defensive / floor#27 / round-1 | 2 | -18 | -9.00 ±3.92 | -18 | |
| Defensive / floor#46 / round-1 | 15 | -18 | -1.20 ±3.47 | +12 | ~noise plain/PD-flip |
| Defensive / floor#56 / deep | 3 | -18 | -6.00 ±2.99 | -34 | |
| Constructive / floor#17 / round-2 | 2 | -17 | -8.50 ±2.94 | -17 | |
| Defensive / floor#70 / round-1 | 2 | -17 | -8.50 ±2.94 | -26 | |
| Constructive / floor#145 / deep | 24 | -16 | -0.67 ±3.52 | -23 | ~noise |
| Defensive / floor#237 / deep | 3 | -16 | -5.33 ±3.97 | -30 | |
| Competitive / floor#45 / deep | 3 | -15 | -5.00 ±4.08 | -31 | |
| Defensive / floor#228 / round-2 | 11 | -15 | -1.36 ±3.66 | +0 | ~noise plain/PD-flip |
| Defensive / floor#240 / round-2 | 25 | -15 | -0.60 ±2.56 | -14 | ~noise |
| Defensive / floor#6 / round-2 | 39 | -15 | -0.38 ±2.29 | -52 | ~noise |
| Defensive / floor#67 / round-2 | 2 | -15 | -7.50 ±2.94 | -26 | |
| Defensive / floor#71 / round-2 | 2 | -15 | -7.50 ±2.94 | -26 | |
| Competitive / floor#135 / round-2 | 2 | -14 | -7.00 ±3.92 | -26 | |
| Competitive / floor#235 / deep | 5 | -14 | -2.80 ±3.69 | -33 | ~noise |
| Competitive / floor#24 / deep | 5 | -14 | -2.80 ±4.04 | -17 | ~noise |
| Competitive / floor#6 / deep | 2 | -14 | -7.00 ±3.92 | -26 | |
| Constructive / floor#32 / round-2 | 4 | -14 | -3.50 ±11.94 | -6 | ~noise |
| Defensive / floor#34 / round-1 | 2 | -14 | -7.00 ±0.00 | -14 | |
| Defensive / floor#5 / round-1 | 28 | -14 | -0.50 ±2.85 | -1 | ~noise |
| Defensive / floor#5 / round-2 | 47 | -14 | -0.30 ±2.02 | -39 | ~noise |
| Competitive / floor#145 / balancing | 8 | -13 | -1.62 ±5.04 | -19 | ~noise |
| Competitive / floor#145 / round-1 | 1 | -13 | -13.00 ±0.00 | -13 | ~noise |
| Competitive / floor#63+rb / deep | 2 | -13 | -6.50 ±8.82 | -14 | ~noise |
| Competitive / floor#0 / deep | 4 | -12 | -3.00 ±2.12 | -24 | |
| Competitive / floor#153 / balancing | 2 | -12 | -6.00 ±1.96 | -12 | |
| Competitive / floor#26 / deep | 1 | -12 | -12.00 ±0.00 | -14 | ~noise |
| Competitive / floor#47+rb / balancing | 2 | -12 | -6.00 ±1.96 | -12 | |
| Defensive / floor#117 / deep | 1 | -12 | -12.00 ±0.00 | -14 | ~noise |
| Defensive / floor#151 / round-1 | 12 | -12 | -1.00 ±5.28 | +7 | ~noise plain/PD-flip |
| Defensive / floor#27 / deep | 2 | -12 | -6.00 ±3.92 | -12 | |
| Defensive / floor#28 / round-1 | 2 | -12 | -6.00 ±1.96 | -14 | |
| Competitive / floor#12 / deep | 2 | -11 | -5.50 ±2.94 | -14 | |
| Competitive / floor#45+rb / deep | 2 | -11 | -5.50 ±2.94 | -21 | |
| Competitive / floor#55 / round-2 | 29 | -11 | -0.38 ±2.25 | +26 | ~noise plain/PD-flip |
| Defensive / floor#235 / balancing | 21 | -11 | -0.52 ±2.60 | -32 | ~noise |
| Defensive / floor#26 / round-1 | 1 | -11 | -11.00 ±0.00 | -14 | ~noise |
| Defensive / floor#63 / deep | 2 | -11 | -5.50 ±2.94 | -14 | |
| Competitive / floor#241+rb / deep | 3 | -10 | -3.33 ±10.27 | -10 | ~noise |
| Competitive / floor#245+rb / balancing | 2 | -10 | -5.00 ±3.92 | -26 | |
| Competitive / floor#54 / round-2 | 8 | -10 | -1.25 ±2.79 | -3 | ~noise |
| Defensive / floor#212 / round-2 | 1 | -10 | -10.00 ±0.00 | -10 | ~noise |
| Defensive / floor#28 / round-2 | 1 | -10 | -10.00 ±0.00 | -10 | ~noise |
| Defensive / floor#32 / deep | 2 | -10 | -5.00 ±5.88 | -14 | ~noise |
| Defensive / floor#40 / deep | 1 | -10 | -10.00 ±0.00 | -10 | ~noise |
| Defensive / floor#56 / round-1 | 11 | -10 | -0.91 ±5.21 | -4 | ~noise |
| Competitive / fallback@3 / balancing | 2 | -9 | -4.50 ±8.82 | -9 | ~noise |
| Competitive / floor#140 / balancing | 1 | -9 | -9.00 ±0.00 | -12 | ~noise |
| Competitive / floor#145 / round-2 | 16 | -9 | -0.56 ±4.49 | -12 | ~noise |
| Competitive / floor#27 / round-2 | 6 | -9 | -1.50 ±7.57 | -23 | ~noise |
| Competitive / floor#62 / balancing | 7 | -9 | -1.29 ±5.20 | +22 | ~noise plain/PD-flip |
| Constructive / floor#151 / deep | 15 | -9 | -0.60 ±3.58 | -8 | ~noise |
| Defensive / floor#18 / deep | 4 | -9 | -2.25 ±2.58 | -12 | ~noise |
| Defensive / floor#41 / round-1 | 4 | -9 | -2.25 ±6.99 | -4 | ~noise |
| Competitive / floor#18 / round-1 | 14 | -8 | -0.57 ±4.80 | +35 | ~noise plain/PD-flip |
| Defensive / floor#229 / round-2 | 4 | -8 | -2.00 ±11.46 | -8 | ~noise |
| Defensive / floor#231 / round-2 | 9 | -8 | -0.89 ±4.16 | -30 | ~noise |
| Defensive / floor#237 / balancing | 23 | -8 | -0.35 ±2.09 | -68 | ~noise |
| Defensive / floor#54 / round-2 | 2 | -8 | -4.00 ±3.92 | -26 | |
| Competitive / floor#1+rb / round-2 | 1 | -7 | -7.00 ±0.00 | -7 | ~noise |
| Competitive / floor#235+rb / deep | 5 | -7 | -1.40 ±2.02 | -40 | ~noise |
| Competitive / floor#242 / balancing | 52 | -7 | -0.13 ±1.56 | -114 | ~noise |
| Competitive / floor#47+rb / deep | 1 | -7 | -7.00 ±0.00 | -9 | ~noise |
| Competitive / floor#46 / balancing | 10 | -6 | -0.60 ±3.08 | -44 | ~noise |
| Competitive / floor#54+rb / round-2 | 2 | -5 | -2.50 ±0.98 | -12 | |
| Defensive / floor#145 / round-2 | 1 | -5 | -5.00 ±0.00 | -6 | ~noise |
| Defensive / floor#68 / round-2 | 2 | -5 | -2.50 ±0.98 | -12 | |
| Competitive / floor#56 / deep | 4 | -4 | -1.00 ±3.84 | -1 | ~noise |
| Competitive / floor#60+rb / deep | 3 | -4 | -1.33 ±3.64 | -15 | ~noise |
| Defensive / floor#10 / round-2 | 4 | -4 | -1.00 ±6.55 | +9 | ~noise plain/PD-flip |
| Competitive / floor#31 / deep | 17 | -3 | -0.18 ±1.46 | +1 | ~noise plain/PD-flip |
| Defensive / floor#230 / round-2 | 6 | -3 | -0.50 ±6.53 | -15 | ~noise |
| Defensive / floor#39 / round-2 | 2 | -3 | -1.50 ±2.94 | -14 | ~noise |
| Defensive / floor#46 / deep | 11 | -3 | -0.27 ±2.47 | -29 | ~noise |
| Defensive / floor#51 / round-2 | 2 | -3 | -1.50 ±2.94 | -14 | ~noise |
| Defensive / floor#69 / round-2 | 1 | -3 | -3.00 ±0.00 | -3 | ~noise |
| Competitive / fallback@4 / balancing | 1 | -2 | -2.00 ±0.00 | -2 | ~noise |
| Competitive / floor#143 / balancing | 3 | -2 | -0.67 ±1.31 | -5 | ~noise |
| Competitive / floor#18+rb / balancing | 2 | -2 | -1.00 ±1.96 | -8 | ~noise |
| Defensive / floor#31 / deep | 11 | -2 | -0.18 ±2.78 | -25 | ~noise |
| Defensive / floor#62 / round-1 | 1 | -2 | -2.00 ±0.00 | -5 | ~noise |
| Defensive / floor#69 / round-1 | 4 | -2 | -0.50 ±3.25 | +14 | ~noise plain/PD-flip |
| Competitive / floor#48 / deep | 2 | -1 | -0.50 ±2.94 | -14 | ~noise |
| Defensive / floor#238 / deep | 2 | -1 | -0.50 ±2.94 | -12 | ~noise |
| Competitive / floor#11 / deep | 2 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Competitive / floor#140+rb / round-2 | 3 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Competitive / floor#144 / round-1 | 3 | +0 | +0.00 ±6.30 | +15 | ~noise |
| Competitive / floor#239+rb / balancing | 2 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Competitive / floor#41 / deep | 1 | +0 | +0.00 ±0.00 | +7 | ~noise |
| Constructive / floor#148 / deep | 12 | +0 | +0.00 ±4.12 | +3 | ~noise |
| Defensive / floor#24 / round-2 | 2 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Defensive / floor#47 / round-1 | 1 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Competitive / floor#14 / round-2 | 1 | +1 | +1.00 ±0.00 | +1 | ~noise |
| Competitive / floor#144 / round-2 | 12 | +1 | +0.08 ±3.20 | -2 | ~noise plain/PD-flip |
| Competitive / floor#239+rb / deep | 3 | +1 | +0.33 ±11.56 | -4 | ~noise plain/PD-flip |
| Defensive / floor#47 / balancing | 1 | +1 | +1.00 ±0.00 | +1 | ~noise |
| Defensive / floor#47 / deep | 3 | +1 | +0.33 ±7.53 | +1 | ~noise |
| Defensive / floor#128 / deep | 2 | +2 | +1.00 ±0.00 | -6 | plain/PD-flip |
| Competitive / floor#26 / round-2 | 2 | +3 | +1.50 ±6.86 | +4 | ~noise |
| Defensive / floor#127 / round-2 | 12 | +3 | +0.25 ±2.43 | +0 | ~noise |
| Defensive / floor#9 / round-2 | 1 | +3 | +3.00 ±0.00 | +5 | ~noise |
| Defensive / floor#42 / deep | 3 | +4 | +1.33 ±11.33 | +16 | ~noise |
| Competitive / floor#18 / deep | 2 | +5 | +2.50 ±4.90 | +7 | ~noise |
| Competitive / floor#245+rb / deep | 2 | +5 | +2.50 ±0.98 | +0 | |
| Competitive / floor#246 / deep | 2 | +5 | +2.50 ±0.98 | +0 | |
| Defensive / floor#207 / round-2 | 2 | +5 | +2.50 ±0.98 | +12 | |
| Defensive / floor#31 / balancing | 2 | +5 | +2.50 ±0.98 | +12 | |
| Defensive / floor#41 / deep | 1 | +5 | +5.00 ±0.00 | +6 | ~noise |
| Competitive / floor#40 / deep | 1 | +6 | +6.00 ±0.00 | +8 | ~noise |
| Defensive / floor#14 / round-1 | 3 | +6 | +2.00 ±9.87 | +12 | ~noise |
| Defensive / floor#208 / round-2 | 7 | +6 | +0.86 ±4.69 | +18 | ~noise |
| Competitive / floor#32 / balancing | 1 | +7 | +7.00 ±0.00 | +7 | ~noise |
| Defensive / floor#129 / deep | 16 | +7 | +0.44 ±3.63 | -14 | ~noise plain/PD-flip |
| Defensive / floor#0 / deep | 1 | +8 | +8.00 ±0.00 | +8 | ~noise |
| Defensive / floor#239 / deep | 5 | +8 | +1.60 ±1.59 | +1 | |
| Competitive / floor#237+rb / balancing | 2 | +9 | +4.50 ±0.98 | +12 | |
| Defensive / floor#16 / deep | 5 | +9 | +1.80 ±6.66 | -9 | ~noise plain/PD-flip |
| Defensive / floor#21 / round-2 | 63 | +9 | +0.14 ±1.51 | -179 | ~noise plain/PD-flip |
| Defensive / floor#38 / round-2 | 2 | +9 | +4.50 ±0.98 | +12 | |
| Competitive / floor#134 / round-2 | 2 | +10 | +5.00 ±5.88 | +2 | ~noise |
| Competitive / floor#40 / round-2 | 29 | +10 | +0.34 ±2.31 | +29 | ~noise |
| Constructive / floor#47 / deep | 15 | +10 | +0.67 ±3.27 | -12 | ~noise plain/PD-flip |
| Defensive / floor#57 / deep | 1 | +10 | +10.00 ±0.00 | +4 | ~noise |
| Defensive / floor#60 / deep | 3 | +10 | +3.33 ±3.27 | +14 | |
| Competitive / floor#15 / deep | 6 | +11 | +1.83 ±2.93 | +14 | ~noise |
| Competitive / floor#39 / deep | 15 | +11 | +0.73 ±2.60 | -29 | ~noise plain/PD-flip |
| Competitive / floor#41 / round-2 | 2 | +11 | +5.50 ±0.98 | -24 | plain/PD-flip |
| Defensive / floor#210 / round-2 | 2 | +11 | +5.50 ±0.98 | +14 | |
| Defensive / floor#48 / deep | 4 | +11 | +2.75 ±6.17 | +4 | ~noise |
| Competitive / floor#55 / deep | 4 | +13 | +3.25 ±2.93 | +26 | |
| Constructive / floor#149 / round-2 | 1 | +13 | +13.00 ±0.00 | +13 | ~noise |
| Defensive / floor#2 / round-1 | 6 | +13 | +2.17 ±7.94 | +14 | ~noise |
| Defensive / floor#226 / round-2 | 6 | +13 | +2.17 ±5.51 | +23 | ~noise |
| Competitive / floor#63 / deep | 3 | +14 | +4.67 ±6.91 | +11 | ~noise |
| Defensive / floor#57 / round-1 | 4 | +14 | +3.50 ±2.59 | +21 | |
| Defensive / floor#153 / round-2 | 40 | +15 | +0.38 ±2.44 | +60 | ~noise |
| Competitive / floor#144 / balancing | 2 | +16 | +8.00 ±3.92 | +16 | |
| Defensive / floor#0 / round-1 | 3 | +16 | +5.33 ±2.85 | +17 | |
| Defensive / floor#107 / balancing | 5 | +16 | +3.20 ±9.03 | +5 | ~noise |
| Defensive / floor#145 / balancing | 2 | +16 | +8.00 ±3.92 | +16 | |
| Competitive / floor#244 / deep | 2 | +17 | +8.50 ±2.94 | +17 | |
| Competitive / floor#243 / round-2 | 2 | +18 | +9.00 ±3.92 | +18 | |
| Competitive / floor#54 / deep | 3 | +18 | +6.00 ±4.53 | +24 | |
| Constructive / floor#48 / round-2 | 5 | +18 | +3.60 ±6.28 | +5 | ~noise |
| Competitive / floor#153 / deep | 4 | +20 | +5.00 ±3.84 | -12 | plain/PD-flip |
| Defensive / floor#1 / deep | 13 | +22 | +1.69 ±3.09 | -59 | ~noise plain/PD-flip |
| Defensive / floor#236 / round-2 | 2 | +22 | +11.00 ±1.96 | +24 | |
| Defensive / floor#57 / round-2 | 13 | +22 | +1.69 ±4.30 | +38 | ~noise |
| Defensive / floor#62 / deep | 10 | +22 | +2.20 ±4.81 | +7 | ~noise |
| Constructive / floor#48 / deep | 10 | +23 | +2.30 ±4.14 | +9 | ~noise |
| Defensive / floor#199 / round-2 | 67 | +23 | +0.34 ±1.12 | -112 | ~noise plain/PD-flip |
| Defensive / floor#241 / balancing | 18 | +23 | +1.28 ±2.14 | -5 | ~noise plain/PD-flip |
| Competitive / floor#147 / deep | 9 | +24 | +2.67 ±4.17 | -1 | ~noise plain/PD-flip |
| Competitive / floor#18+rb / deep | 2 | +24 | +12.00 ±1.96 | +26 | |
| Constructive / floor#158 / deep | 4 | +24 | +6.00 ±6.84 | +24 | ~noise |
| Competitive / floor#143 / round-1 | 2 | +26 | +13.00 ±1.96 | +32 | |
| Competitive / floor#151 / balancing | 8 | +27 | +3.38 ±4.90 | +26 | ~noise |
| Competitive / floor#47+rb / round-2 | 2 | +27 | +13.50 ±0.98 | +27 | |
| Defensive / floor#1 / round-1 | 15 | +28 | +1.87 ±4.13 | +31 | ~noise |
| Defensive / floor#42 / round-1 | 4 | +28 | +7.00 ±2.89 | +43 | |
| Defensive / floor#227 / round-2 | 7 | +29 | +4.14 ±1.45 | +38 | |
| Competitive / floor#239 / deep | 10 | +31 | +3.10 ±3.14 | +29 | ~noise |
| Defensive / floor#6 / round-1 | 19 | +31 | +1.63 ±3.39 | +29 | ~noise |
| Defensive / floor#67 / round-1 | 6 | +31 | +5.17 ±3.87 | +44 | |
| Constructive / floor#62 / deep | 29 | +32 | +1.10 ±2.44 | -3 | ~noise plain/PD-flip |
| Defensive / floor#140 / round-1 | 46 | +34 | +0.74 ±2.77 | +51 | ~noise |
| Defensive / floor#1 / round-2 | 214 | +46 | +0.21 ±1.06 | +80 | ~noise |
| Defensive / floor#205 / round-2 | 92 | +49 | +0.53 ±1.29 | +78 | ~noise |
| Defensive / floor#56 / round-2 | 12 | +50 | +4.17 ±4.22 | +60 | ~noise |
| Competitive / floor#151 / round-1 | 7 | +60 | +8.57 ±3.89 | +63 | |
| Defensive / floor#145 / round-1 | 12 | +60 | +5.00 ±3.91 | +93 | |
| Constructive / floor#159 / round-2 | 16 | +70 | +4.38 ±6.70 | +70 | ~noise |
| Competitive / floor#62 / round-2 | 64 | +74 | +1.16 ±1.66 | -218 | ~noise plain/PD-flip |
| Competitive / floor#47 / deep | 21 | +91 | +4.33 ±2.58 | +54 | |
| Defensive / floor#62 / round-2 | 48 | +115 | +2.40 ±1.96 | +89 | |
| Competitive / floor#1 / deep | 76 | +143 | +1.88 ±1.37 | +135 | |
| Defensive / floor#0 / round-2 | 31 | +153 | +4.94 ±2.21 | +153 | |
| Defensive / floor#47 / round-2 | 132 | +164 | +1.24 ±1.25 | +8 | ~noise |
| Competitive / floor#0 / round-2 | 214 | +440 | +2.06 ±0.85 | +608 | |

## By phase

  -251275 IMPs  117195 boards  Defensive
  -232496 IMPs  129558 boards  Constructive
  -130622 IMPs   49557 boards  Competitive

## By provenance

  -322470 IMPs  181529 boards  book
   -85435 IMPs   30012 boards  floor#3
   -27187 IMPs   10344 boards  fallback@1
   -24094 IMPs    9108 boards  fallback@2
   -18775 IMPs    7691 boards  fallback@3
   -16992 IMPs    6518 boards  floor#245
   -11532 IMPs    3133 boards  floor#246
    -8518 IMPs    2938 boards  fallback@4
    -7288 IMPs    2531 boards  floor#60
    -7275 IMPs    3615 boards  floor#20
    -6383 IMPs    2223 boards  floor#61
    -6182 IMPs    2296 boards  floor#45
    -5900 IMPs    2537 boards  floor#140
    -5753 IMPs    3324 boards  floor#35
    -5384 IMPs    2331 boards  floor#46
    -4208 IMPs    3277 boards  floor#50
    -3640 IMPs    1289 boards  floor#30
    -3224 IMPs     792 boards  floor#31
    -2840 IMPs    1709 boards  floor#64
    -2704 IMPs     730 boards  floor#202
    -2593 IMPs     340 boards  floor#245+rb
    -2365 IMPs     722 boards  floor#132
    -2210 IMPs     474 boards  floor#16
    -1617 IMPs    1606 boards  floor#65
    -1541 IMPs     474 boards  fallback@5
    -1409 IMPs     420 boards  floor#32
    -1326 IMPs     537 boards  floor#131
    -1184 IMPs     417 boards  floor#6
    -1180 IMPs     400 boards  floor#17
    -1108 IMPs     577 boards  floor#153
    -1081 IMPs     571 boards  floor#147
    -1062 IMPs     423 boards  floor#240
    -1022 IMPs     354 boards  floor#237
    -1005 IMPs     386 boards  floor#200
     -980 IMPs     204 boards  floor#63
     -886 IMPs     437 boards  floor#5
     -861 IMPs     244 boards  floor#145
     -768 IMPs     283 boards  floor#197
     -684 IMPs     255 boards  floor#198
     -667 IMPs     213 boards  floor#66
     -646 IMPs     255 boards  floor#239
     -641 IMPs     230 boards  floor#241
     -619 IMPs     316 boards  floor#51
     -614 IMPs     331 boards  floor#21
     -606 IMPs     412 boards  book+rb
     -600 IMPs     311 boards  floor#2
     -577 IMPs     184 boards  floor#48
     -557 IMPs     405 boards  floor#133
     -555 IMPs     198 boards  floor#235
     -535 IMPs     449 boards  floor#49
     -532 IMPs     197 boards  floor#3+rb
     -478 IMPs     138 boards  floor#15
     -447 IMPs     230 boards  floor#238
     -422 IMPs     234 boards  floor#36
     -404 IMPs     129 boards  floor#204
     -353 IMPs     196 boards  floor#25
     -333 IMPs     279 boards  floor#199
     -326 IMPs     134 boards  floor#10
     -315 IMPs     254 boards  floor#129
     -292 IMPs     145 boards  floor#236
     -277 IMPs      88 boards  floor#33
     -234 IMPs      82 boards  floor#234
     -220 IMPs      61 boards  floor#246+rb
     -213 IMPs     146 boards  floor#9
     -207 IMPs      46 boards  floor#27
     -206 IMPs      70 boards  floor#18
     -189 IMPs     118 boards  floor#157
     -188 IMPs     196 boards  floor#205
     -173 IMPs      53 boards  floor#42
     -166 IMPs      19 boards  floor#234+rb
     -164 IMPs     146 boards  floor#203
     -159 IMPs      66 boards  floor#12
     -131 IMPs      44 boards  floor#46+rb
     -127 IMPs      34 boards  floor#236+rb
     -112 IMPs      28 boards  floor#11
     -103 IMPs     330 boards  floor#47
      -88 IMPs      11 boards  floor#26
      -85 IMPs      53 boards  floor#24
      -85 IMPs     158 boards  floor#34
      -77 IMPs      15 boards  floor#235+rb
      -74 IMPs      19 boards  floor#143
      -73 IMPs      36 boards  floor#57
      -64 IMPs      15 boards  floor#239+rb
      -60 IMPs      14 boards  floor#29
      -50 IMPs      51 boards  floor#41
      -49 IMPs       6 boards  floor#30+rb
      -48 IMPs      13 boards  floor#241+rb
      -41 IMPs      45 boards  floor#240+rb
      -40 IMPs      47 boards  floor#55
      -40 IMPs       9 boards  floor#60+rb
      -38 IMPs      35 boards  floor#238+rb
      -38 IMPs       6 boards  floor#45+rb
      -37 IMPs      11 boards  fallback@6
      -35 IMPs       9 boards  floor#61+rb
      -32 IMPs      21 boards  floor#237+rb
      -32 IMPs      18 boards  floor#54
      -28 IMPs     211 boards  floor#151
      -26 IMPs      35 boards  floor#40
      -25 IMPs       6 boards  fallback@4+rb
      -25 IMPs       6 boards  floor#62+rb
      -24 IMPs      14 boards  floor#148
      -24 IMPs       2 boards  floor#160
      -23 IMPs       7 boards  floor#14
      -22 IMPs       3 boards  floor#13
      -22 IMPs       3 boards  floor#28
      -20 IMPs      18 boards  floor#154
      -19 IMPs       4 boards  floor#155
      -19 IMPs      24 boards  floor#244
      -17 IMPs      32 boards  floor#39
      -17 IMPs       2 boards  floor#70
      -15 IMPs      11 boards  floor#228
      -15 IMPs       2 boards  floor#71
      -14 IMPs       2 boards  floor#135
      -13 IMPs       2 boards  floor#63+rb
      -12 IMPs       1 boards  floor#117
      -10 IMPs       1 boards  floor#212
       -8 IMPs      19 boards  floor#144
       -8 IMPs       4 boards  floor#229
       -8 IMPs       9 boards  floor#231
       -7 IMPs       1 boards  floor#1+rb
       -7 IMPs      52 boards  floor#242
       -5 IMPs       2 boards  floor#54+rb
       -5 IMPs       2 boards  floor#68
       -5 IMPs       5 boards  floor#69
       -3 IMPs       6 boards  floor#230
       +0 IMPs       3 boards  floor#140+rb
       +2 IMPs       2 boards  floor#128
       +3 IMPs      12 boards  floor#127
       +5 IMPs       2 boards  floor#207
       +6 IMPs       7 boards  floor#208
       +8 IMPs       5 boards  floor#47+rb
       +9 IMPs       2 boards  floor#38
      +10 IMPs       2 boards  floor#134
      +11 IMPs       2 boards  floor#210
      +13 IMPs       1 boards  floor#149
      +13 IMPs       6 boards  floor#226
      +16 IMPs       5 boards  floor#107
      +16 IMPs       8 boards  floor#67
      +18 IMPs       2 boards  floor#243
      +18 IMPs      30 boards  floor#56
      +22 IMPs       4 boards  floor#18+rb
      +24 IMPs       4 boards  floor#158
      +29 IMPs       7 boards  floor#227
      +46 IMPs      18 boards  floor#159
     +151 IMPs     206 boards  floor#62
     +168 IMPs     833 boards  floor#1
     +605 IMPs     253 boards  floor#0

## By family

  -291035 IMPs  129795 boards  round-1
  -202501 IMPs   93701 boards  round-2
   -86728 IMPs   50993 boards  opening
   -20264 IMPs   11265 boards  balancing
   -13865 IMPs   10556 boards  deep

## By direction

  -442483 IMPs   62913 boards  other
  -203218 IMPs   30734 boards  overbid
  -158236 IMPs   23690 boards  sold-out
  -144454 IMPs   16679 boards  missed-game
   -75876 IMPs   13953 boards  wrong-strain
   -74318 IMPs    6120 boards  missed-slam
   -10914 IMPs     768 boards  missed-grand
    -8823 IMPs    1465 boards  doubling
       +0 IMPs   51573 boards  flat
  +503929 IMPs   88415 boards  gain

## Worst boards per losing bucket

### Defensive / book / round-1 (59098 boards, -111127 IMPs)

[vul both, seed Some(1783375079), board 667] swing -3220 pts / -22 IMPs (PD -22), diverged at call 1 (P ours vs 2♦ BBA), other
  rule: 0+ HCP
  W:AQ95.J93.Q8.KQJ6 KJT864.A5.AT932. 3.KQT872.J.A9754 72.64.K7654.T832
  ours NS @ A: 1NT - 4♦ - 4♥ - - -  -> 4♥ by West
  ours EW @ B: 1NT 2♦ 4♣ - 4♦ X - - -  -> 4♦x by West

[vul both, seed Some(1783375069), board 5899] swing -3130 pts / -22 IMPs (PD -22), diverged at call 1 (2♦ ours vs P BBA), other
  rule: 5+ ♦, 9+ points, ≤17 HCP, and (11+ points, or a passed hand)
  W:AKT94.AQT62.K4.T 6.K984.AQ9873.J4 QJ853.7.J62.A762 72.J53.T5.KQ9853
  ours NS @ A: 1♠ 2♦ 3♦ - 3♥ - 4♣ - 4NT - 5♦ - 5♥ - 5NT - 6♠ - - -  -> 6♠ by West
  ours EW @ B: 1♠ - 4♥ - 4NT - 5♣ X - - -  -> 5♣x by East

[vul both, seed Some(1783375074), board 2840] swing -2500 pts / -21 IMPs (PD -21), diverged at call 2 (3♣ ours vs 5♣ BBA), missed-slam
  rule: 5+ ♣, and 10–16 points
  W:963.AKQJ832.6.95 QT52.54.KJT74.K8 AKJ874.T76.932.J .9.AQ85.AQT76432
  ours NS @ A: - 2♠ 3♣ 3♠ - - X - - -  -> 3♠x by East
  ours EW @ B: - 2♠ 5♣ - 6♣ - - -  -> 6♣ by South

### Constructive / book / opening (50993 boards, -86728 IMPs)

[vul both, seed Some(1783375067), board 372] swing -3400 pts / -22 IMPs (PD -22), diverged at call 0 (2♦ ours vs P BBA), overbid
  rule: exactly 6 ♦, 5–10 points, and not (opening in seat 4)
  W:A5.KQ973.J.97543 KT6.4.Q86432.QT6 832.AT852.T7.AJ8 QJ974.J6.AK95.K2
  ours NS @ A: 2♦ - 2NT - 3♥ X - - -  -> 3♥x by North
  ours EW @ B: - - 1♠ 2♠ X - - -  -> 2♠x by West

[vul both, seed Some(1783375076), board 3305] swing -2540 pts / -21 IMPs (PD -22), diverged at call 0 (1♦ ours vs P BBA), other
  rule: 12–21 points, prefers diamonds, ≤4 ♥, and ≤4 ♠
  W:.AK2.AQ63.AJ9754 987654.QJ864.8.8 KQ32.T5.KJ954.QT AJT.973.T72.K632
  ours NS @ A: - - 1♣ - 1♦ - 5♠ - 6♣ - 7♦ - - -  -> 7♦ by East
  ours EW @ B: 1♦ - 2♣ - 2♠ - 3♣ - 4♠ - - -  -> 4♠ by East

[vul both, seed Some(1783375078), board 5315] swing -2860 pts / -21 IMPs (PD -21), diverged at call 0 (1♠ ours vs 2♠ BBA), other
  rule: 12–21 points, and 5+ ♠
  W:AT9832.J9.AT853. Q5.KQ532.QJ942.7 KJ64.A86..AKT865 7.T74.K76.QJ9432
  ours NS @ A: 2♠ 3♦ 4♦ - 4♥ - 5♦ - 5♠ - 6♠ - - -  -> 6♠ by West
  ours EW @ B: 1♠ - 2NT - 3♣ - 4NT - 5♥ X - - -  -> 5♥x by West

### Constructive / book / round-2 (37470 boards, -64692 IMPs)

[vul both, seed Some(1783375080), board 5575] swing -2560 pts / -21 IMPs (PD -21), diverged at call 4 (3♣ ours vs 4♦ BBA), other
  rule: 4+ card support for partner
  W:AQ987.AKJ7..K753 JT3.T94.JT84.J94 K42.Q5.A65.AQT82 65.8632.KQ9732.6
  ours NS @ A: 1♠ - 2♣ - 4♦ - 4♠ - 5♥ - 6♠ - - -  -> 6♠ by West
  ours EW @ B: 1♠ - 2♣ - 3♣ - 3♠ - 4NT - 5♦ X - - -  -> 5♦x by East

[vul both, seed Some(1783375082), board 0] swing -2650 pts / -21 IMPs (PD -21), diverged at call 7 (3♠ ours vs 4♥ BBA), other
  rule: 3+ ♠
  W:Q42.KQ63.AT432.Q 963.8.Q965.AKJ72 AJT87.AJ754.K.95 K5.T92.J87.T8643
  ours NS @ A: - 1♠ - 2♦ - 2♥ - 4♥ - - -  -> 4♥ by East
  ours EW @ B: - 1♠ - 2♦ - 2♥ - 3♠ - 4NT - 5♣ X - - -  -> 5♣x by West

[vul both, seed Some(1783375086), board 746] swing -2950 pts / -21 IMPs (PD -21), diverged at call 4 (4NT ours vs 4♣ BBA), wrong-strain
  rule: 22+ support points
  W:5.J763.AT865.854 KQ83.T52.932.QT9 J64.9.J7.AKJ7632 AT972.AKQ84.KQ4.
  ours NS @ A: 1♠ - 2♠ - 4NT - 5♣ X - - -  -> 5♣x by North
  ours EW @ B: 1♠ - 2♠ - 4♣ - 4♠ - - -  -> 4♠ by South

### Constructive / book / round-1 (24459 boards, -49912 IMPs)

[vul both, seed Some(1783375092), board 5201] swing -3130 pts / -22 IMPs (PD -22), diverged at call 2 (3♦ ours vs 4♦ BBA), other
  rule: 5+ ♥, and hearts not outnumbered (longer-major discipline)
  W:AT85.QJ8543.65.9 J43.T976.J943.K7 KQ9.AK2.A82.A652 762..KQT7.QJT843
  ours NS @ A: 2NT - 4♦ - 4♥ - 4NT - 5♦ - 6♥ - - -  -> 6♥ by East
  ours EW @ B: 2NT - 3♦ - 3♥ - 4NT - 5♣ X - - -  -> 5♣x by East

[vul both, seed Some(1783375073), board 3153] swing -3220 pts / -22 IMPs (PD -22), diverged at call 2 (4♥ ours vs 3♦ BBA), other
  rule: 4+ card support for partner, 10–13 support points, and ≤1 ♥
  W:K973.Q.KJ9853.63 65.T97.AQ.AQJT85 AQJ842.A863.42.K T.KJ542.T76.9742
  ours NS @ A: 1♠ - 3♦ X 3♠ - 4♠ - - -  -> 4♠ by East
  ours EW @ B: 1♠ - 4♥ - 4NT - 5♣ X - - -  -> 5♣x by West

[vul none, seed Some(1783375073), board 3153] swing -2720 pts / -21 IMPs (PD -21), diverged at call 2 (4♥ ours vs 3♦ BBA), other
  rule: 4+ card support for partner, 10–13 support points, and ≤1 ♥
  W:K973.Q.KJ9853.63 65.T97.AQ.AQJT85 AQJ842.A863.42.K T.KJ542.T76.9742
  ours NS @ A: 1♠ - 3♦ X 3♠ - 4♠ - - -  -> 4♠ by East
  ours EW @ B: 1♠ - 4♥ - 4NT - 5♣ X - - -  -> 5♣x by West

### Defensive / floor#3 / round-2 (8861 boards, -28219 IMPs)

[vul both, seed Some(1783375065), board 3233] swing -2490 pts / -20 IMPs (PD -20), diverged at call 4 (P ours vs 2♥ BBA), other
  rule: not ((opaque condition)), or (opaque condition)
  W:8.AK86.KJ42.QT76 AT.QT4.AT93.KJ94 6542.J97532.65.2 KQJ973..Q87.A853
  ours NS @ A: - 1♠ X XX 2♥ 2♠ - - -  -> 2♠ by South
  ours EW @ B: - 1♠ X XX - - -  -> 1♠xx by South

[vul both, seed Some(1783375075), board 3397] swing -2150 pts / -19 IMPs (PD -19), diverged at call 5 (P ours vs 2♦ BBA), missed-slam
  rule: not ((opaque condition)), or (opaque condition)
  W:KJ8764.4.42.6543 9.AQJ98532.Q.AKJ AQT.6.KJT73.QT92 532.KT7.A9865.87
  ours NS @ A: 1♦ - 1♠ X XX - - -  -> 1♠xx by West
  ours EW @ B: 1♦ - 1♠ X XX 2♦ 2♠ 4♥ 4♠ 4NT - 5♠ - 6♥ - - -  -> 6♥ by North

[vul both, seed Some(1783375085), board 1306] swing -2040 pts / -19 IMPs (PD -19), diverged at call 4 (P ours vs 2♣ BBA), other
  rule: not ((opaque condition)), or (opaque condition)
  W:KJ9632.A54.KJ2.8 84.Q98.AQ63.KJ73 AT7.KT762.75.AQ6 Q5.J3.T984.T9542
  ours NS @ A: - 1♠ X XX - - -  -> 1♠xx by West
  ours EW @ B: - 1♠ X XX 2♣ 2♠ - 4♠ - - -  -> 4♠ by West

### Competitive / fallback@1 / round-1 (9177 boards, -24201 IMPs)

[vul both, seed Some(1783375074), board 650] swing -2010 pts / -19 IMPs (PD -19), diverged at call 2 (2NT ours vs 2♥ BBA), missed-slam
  rule: 2NT is the cheapest bid, 11–12 HCP, and stopper in their suit(s)
  W:AKQ964.98754..72 .AQ2.QT8532.KT53 J8732.T.K96.Q986 T5.KJ63.AJ74.AJ4
  ours NS @ A: 1♦ 2♦ 2NT 4♠ - - -  -> 4♠ by East
  ours EW @ B: 1♦ 2♦ 2♥ - 3♥ - 4♠ - 5♦ - 6♦ - - -  -> 6♦ by South

[vul both, seed Some(1783375079), board 1001] swing -2040 pts / -19 IMPs (PD -18), diverged at call 2 (2NT ours vs 3♣ BBA), sold-out
  rule: 2NT is the cheapest bid, 11–12 HCP, and stopper in their suit(s)
  W:A9..QJ65.KQT9752 76532.T953.T7.84 8.QJ2.AK843.AJ63 KQJT4.AK8764.92.
  ours NS @ A: 1♦ 2♦ 3♣ - 3♥ - 5♥ - 6♣ - 7♣ - - -  -> 7♣ by West
  ours EW @ B: 1♦ 2♦ 2NT 4♠ - - -  -> 4♠ by North

[vul both, seed Some(1783375091), board 91] swing -2200 pts / -19 IMPs (PD -21), diverged at call 2 (3♥ ours vs P BBA), overbid
  rule: 2♥ is the cheapest bid, 6+ ♥, 2–5 points, and not (opponents bid ♥)
  W:.J6.AQ9762.KQ532 AKQ52.3.J4.AT764 986.KT87542.T85. JT743.AQ9.K3.J98
  ours NS @ A: 1♦ 1♠ - 2♥ - 4♠ - - -  -> 4♠ by North
  ours EW @ B: 1♦ 1♠ 3♥ 3♠ X - 3NT - - X - - -  -> 3NTx by East

### Defensive / floor#3 / round-1 (7419 boards, -23153 IMPs)

[vul both, seed Some(1783375074), board 3588] swing -2540 pts / -21 IMPs (PD -22), diverged at call 3 (P ours vs X BBA), other
  rule: not ((opaque condition)), or (opaque condition)
  W:A843.A.KT8654.76 KQT.QJ9654.Q9.J9 .KT8.A732.AQ8543 J97652.732.J.KT2
  ours NS @ A: 1♥ 2♣ 2♥ X 3♥ - - 4♦ - 4NT - 5♣ - 5♦ - 6♦ - 7♦ - - -  -> 7♦ by West
  ours EW @ B: 1♥ 2♣ 2♥ - - 3♣ - - 3♥ X - 4♠ - - -  -> 4♠ by West

[vul both, seed Some(1783375076), board 2824] swing -2830 pts / -21 IMPs (PD -21), diverged at call 3 (P ours vs 4♠ BBA), missed-grand
  rule: not ((opaque condition)), or (opaque condition)
  W:QJ9843.985.J.J72 KT.KJ6432.32.A84 A7652..K965.QT63 .AQT7.AQT874.K95
  ours NS @ A: 1♥ 1♠ 2♠ 4♠ - - -  -> 4♠ by East
  ours EW @ B: 1♥ 1♠ 2♠ - 4♥ - 4♠ - 5♦ - 7♥ - - -  -> 7♥ by North

[vul both, seed Some(1783375086), board 1704] swing -2270 pts / -20 IMPs (PD -20), diverged at call 3 (P ours vs 1♥ BBA), wrong-strain
  rule: not ((opaque condition)), or (opaque condition)
  W:972.JT82.T93.QT8 T4.K9.AQ7642.A92 J83.A743.K.KJ654 AKQ65.Q65.J85.73
  ours NS @ A: 1♦ X XX 1♥ 2♦ - 3♦ - 4♦ - - -  -> 4♦ by North
  ours EW @ B: 1♦ X XX - - 2♣ 2♠ - - X XX - - -  -> 2♠xx by South

### Competitive / fallback@2 / round-1 (8054 boards, -21439 IMPs)

[vul both, seed Some(1783375093), board 3413] swing -2950 pts / -21 IMPs (PD -21), diverged at call 2 (3♥ ours vs X BBA), missed-game
  rule: 3+ ♥, and 6–9 points
  W:873.AQ843.J82.65 QJ954.5.9743.KJ3 .KT972.AQT65.Q84 AKT62.J6.K.AT972
  ours NS @ A: 1♥ 2♥ X - - -  -> 2♥x by South
  ours EW @ B: 1♥ 2♥ 3♥ 3♠ 4♥ - - 4♠ - - -  -> 4♠ by North

[vul both, seed Some(1783375079), board 5800] swing -2320 pts / -20 IMPs (PD -20), diverged at call 2 (3♥ ours vs X BBA), other
  rule: 3+ ♥, and 6–9 points
  W:T86532.T.643.A94 .AQJ9873.AK72.76 AKQ94.5.J9.QJT83 J7.K642.QT85.K52
  ours NS @ A: 1♥ 2♥ 3♥ 4♠ - - -  -> 4♠ by West
  ours EW @ B: 1♥ 2♥ X - - -  -> 2♥x by East

[vul both, seed Some(1783375080), board 4786] swing -2320 pts / -20 IMPs (PD -20), diverged at call 3 (P ours vs X BBA), missed-game
  rule: 0+ HCP
  W:87.A53.KJT9.KJ98 AJT32.QJ8762..A4 9.KT.A876543.Q76 KQ654.94.Q2.T532
  ours NS @ A: - 1♦ 2♦ X - - -  -> 2♦x by North
  ours EW @ B: - 1♦ 2♦ - 4♠ - - -  -> 4♠ by South

### Competitive / fallback@3 / round-2 (5933 boards, -13015 IMPs)

[vul both, seed Some(1783375094), board 6351] swing -2620 pts / -21 IMPs (PD -21), diverged at call 4 (P ours vs X BBA), wrong-strain
  rule: 0+ HCP
  W:T76.K832.76.AT42 K94.T64.542.9763 32.AJ7.QT98.KQJ5 AQJ85.Q95.AKJ3.8
  ours NS @ A: - - 1♦ 1♠ X - 2♥ X - 3♣ X - - -  -> 3♣x by North
  ours EW @ B: - - 1♦ 1♠ - - X XX - - -  -> 1♠xx by South

[vul both, seed Some(1783375089), board 6277] swing -2320 pts / -20 IMPs (PD -20), diverged at call 4 (2♥ ours vs X BBA), missed-game
  rule: 2♥ is the cheapest bid, 5+ ♥, 10+ points, and not (opponents bid ♥)
  W:J.K6.KJ9832.AJ96 AQ875.AJ852..752 62.QT743.AT65.K8 KT943.9.Q74.QT43
  ours NS @ A: - - 1♦ 2♦ X - - -  -> 2♦x by North
  ours EW @ B: - - 1♦ 2♦ 2♥ 2♠ - - 3♦ 3♠ 4♦ - - 4♠ - - -  -> 4♠ by South

[vul both, seed Some(1783375090), board 6062] swing -2240 pts / -19 IMPs (PD -19), diverged at call 4 (P ours vs 2♦ BBA), wrong-strain
  rule: 0+ HCP
  W:87542.53.973.Q93 KJ63.J8.J5.T8642 AQ9.T76.QT4.AK75 T.AKQ942.AK862.J
  ours NS @ A: 1♥ - 1♠ X - - -  -> 1♠x by North
  ours EW @ B: 1♥ - 1♠ X 2♦ - 2♥ X XX - - -  -> 2♥xx by South

### Competitive / floor#3 / round-2 (3724 boards, -10516 IMPs)

[vul both, seed Some(1783375068), board 5739] swing -2690 pts / -21 IMPs (PD -21), diverged at call 4 (P ours vs 1♠ BBA), wrong-strain
  rule: not ((opaque condition)), or (opaque condition)
  W:T764.K74.Q.AKQJ4 .AQT3.T9764.9873 Q32.98.J832.T652 AKJ985.J652.AK5.
  ours NS @ A: 1♣ - - X 1♠ - - X - 2♦ - 3♦ - - -  -> 3♦ by North
  ours EW @ B: 1♣ - - X - 1♥ - 2♥ X XX - - -  -> 2♥xx by North

[vul both, seed Some(1783375087), board 3046] swing -1830 pts / -18 IMPs (PD -18), diverged at call 6 (P ours vs 3♦ BBA), overbid
  rule: not ((opaque condition)), or (opaque condition)
  W:AT84.AJ8762.4.A4 KJ63.K43.K832.J9 Q975.T9.T95.8762 2.Q5.AQJ76.KQT53
  ours NS @ A: 1♦ 1♥ X - 2♣ 2♥ - - X - 4♠ - - X - - -  -> 4♠x by North
  ours EW @ B: 1♦ 1♥ X - 2♣ 2♥ 3♦ - 4♦ - - -  -> 4♦ by South

[vul none, seed Some(1783375087), board 3046] swing -1530 pts / -17 IMPs (PD -17), diverged at call 6 (P ours vs 3♦ BBA), overbid
  rule: not ((opaque condition)), or (opaque condition)
  W:AT84.AJ8762.4.A4 KJ63.K43.K832.J9 Q975.T9.T95.8762 2.Q5.AQJ76.KQT53
  ours NS @ A: 1♦ 1♥ X - 2♣ 2♥ - - X - 4♠ - - X - - -  -> 4♠x by North
  ours EW @ B: 1♦ 1♥ X - 2♣ 2♥ 3♦ - 4♦ - - -  -> 4♦ by South

