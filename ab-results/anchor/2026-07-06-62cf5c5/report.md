=== arm 0: our american floor (us) vs BBA 2/1 (them), vulnerability none, 102400 boards ===
replay verification: 100.00% of 1074619 our-side calls (0 mismatched)
auction-divergent: 94916 (93%), contract-divergent: 79476 (78%)
plain DD: -1.6748 IMPs/board (95% CI [-1.7070, -1.6426]), -171501 IMPs total
perfect defense: -2.0216 IMPs/board (95% CI [-2.0601, -1.9830])
gen_args: --count 6400 --seed 1783375064 --output ab-results/anchor/2026-07-06-62cf5c5/none/shard-0.json -v none

=== arm 1: our american floor (us) vs BBA 2/1 (them), vulnerability both, 102400 boards ===
replay verification: 100.00% of 1063479 our-side calls (0 mismatched)
auction-divergent: 94601 (92%), contract-divergent: 78856 (77%)
plain DD: -2.3102 IMPs/board (95% CI [-2.3512, -2.2692]), -236561 IMPs total
perfect defense: -2.7707 IMPs/board (95% CI [-2.8187, -2.7226])
gen_args: --count 6400 --seed 1783375064 --output ab-results/anchor/2026-07-06-62cf5c5/both/shard-0.json -v both


## IMP histogram (plain, per contract-divergent board)

right-siding-only divergences (same contract, different auction): 31185

  -23 IMPs: 1
  -22 IMPs: 6
  -21 IMPs: 27
  -20 IMPs: 47
  -19 IMPs: 135
  -18 IMPs: 249
  -17 IMPs: 672
  -16 IMPs: 772
  -15 IMPs: 1548
  -14 IMPs: 2454
  -13 IMPs: 4879
  -12 IMPs: 4716
  -11 IMPs: 7187
  -10 IMPs: 9644
   -9 IMPs: 4481
   -8 IMPs: 3680
   -7 IMPs: 6665
   -6 IMPs: 9763
   -5 IMPs: 8156
   -4 IMPs: 4284
   -3 IMPs: 6140
   -2 IMPs: 6647
   -1 IMPs: 5765
   +0 IMPs: 25664
   +1 IMPs: 5336
   +2 IMPs: 5042
   +3 IMPs: 4079
   +4 IMPs: 2949
   +5 IMPs: 7482
   +6 IMPs: 6279
   +7 IMPs: 3146
   +8 IMPs: 1220
   +9 IMPs: 1277
  +10 IMPs: 2656
  +11 IMPs: 2138
  +12 IMPs: 1377
  +13 IMPs: 1433
  +14 IMPs: 241
  +15 IMPs: 48
  +16 IMPs: 29
  +17 IMPs: 15
  +18 IMPs: 1
  +19 IMPs: 2

## Ranked buckets (phase / provenance / family), losses first

| bucket | boards | net plain IMPs | IMPs/divergent ±CI | net PD IMPs | flag |
| --- | --- | --- | --- | --- | --- |
| Defensive / book / round-1 | 40416 | -98478 | -2.44 ±0.07 | -136494 | |
| Constructive / book / opening | 29232 | -68344 | -2.34 ±0.08 | -67152 | |
| Constructive / book / round-2 | 18363 | -39557 | -2.15 ±0.10 | -40090 | |
| Constructive / book / round-1 | 12600 | -33912 | -2.69 ±0.12 | -38198 | |
| Competitive / fallback@2 / round-1 | 5448 | -18141 | -3.33 ±0.17 | -21416 | |
| Competitive / fallback@1 / round-1 | 5707 | -17673 | -3.10 ±0.17 | -19352 | |
| Defensive / floor#3 / round-2 | 4135 | -13247 | -3.20 ±0.19 | -14691 | |
| Defensive / floor#3 / round-1 | 3419 | -11581 | -3.39 ±0.23 | -10317 | |
| Competitive / fallback@3 / round-2 | 3284 | -9394 | -2.86 ±0.22 | -11334 | |
| Defensive / book / round-2 | 2650 | -6710 | -2.53 ±0.26 | -8530 | |
| Competitive / fallback@4 / round-2 | 1633 | -5549 | -3.40 ±0.31 | -7431 | |
| Defensive / floor#242 / round-1 | 1483 | -5459 | -3.68 ±0.38 | -6565 | |
| Competitive / floor#242 / round-2 | 1409 | -5127 | -3.64 ±0.35 | -8753 | |
| Competitive / fallback@3 / round-1 | 1013 | -3525 | -3.48 ±0.44 | -4742 | |
| Constructive / floor#140 / round-2 | 1053 | -2828 | -2.69 ±0.45 | -3003 | |
| Competitive / floor#242+rb / round-2 | 402 | -2650 | -6.59 ±0.65 | -3427 | |
| Competitive / floor#3 / round-2 | 940 | -2641 | -2.81 ±0.46 | -2684 | |
| Competitive / floor#3 / round-1 | 482 | -2438 | -5.06 ±0.62 | -1229 | |
| Constructive / floor#3 / round-1 | 657 | -2251 | -3.43 ±0.55 | -1091 | |
| Defensive / floor#242 / balancing | 874 | -2138 | -2.45 ±0.45 | -4183 | |
| Constructive / floor#3 / round-2 | 1012 | -2097 | -2.07 ±0.36 | -1846 | |
| Competitive / floor#242 / balancing | 792 | -2076 | -2.62 ±0.45 | -4390 | |
| Defensive / floor#60 / round-2 | 615 | -1916 | -3.12 ±0.46 | -3235 | |
| Defensive / floor#20 / round-1 | 621 | -1842 | -2.97 ±0.51 | -2368 | |
| Defensive / floor#3 / balancing | 647 | -1838 | -2.84 ±0.43 | -1261 | |
| Defensive / floor#242 / round-2 | 444 | -1790 | -4.03 ±0.69 | -3057 | |
| Defensive / floor#50 / round-1 | 771 | -1701 | -2.21 ±0.44 | -2252 | |
| Defensive / floor#35 / round-1 | 636 | -1534 | -2.41 ±0.48 | -2164 | |
| Defensive / floor#64 / round-1 | 541 | -1481 | -2.74 ±0.58 | -1523 | |
| Defensive / floor#202 / round-2 | 224 | -1199 | -5.35 ±0.80 | -1817 | |
| Constructive / book / deep | 1960 | -1193 | -0.61 ±0.28 | -1300 | |
| Competitive / floor#46 / round-2 | 289 | -1118 | -3.87 ±0.66 | -1555 | |
| Constructive / floor#61 / deep | 492 | -1025 | -2.08 ±0.27 | -1622 | |
| Defensive / floor#202 / round-1 | 244 | -1020 | -4.18 ±0.86 | -1326 | |
| Constructive / floor#3 / deep | 564 | -872 | -1.55 ±0.54 | -729 | |
| Defensive / floor#65 / round-1 | 450 | -860 | -1.91 ±0.58 | -1319 | |
| Defensive / floor#45 / round-2 | 246 | -845 | -3.43 ±0.82 | -1384 | |
| Defensive / floor#35 / round-2 | 467 | -832 | -1.78 ±0.53 | -1103 | |
| Competitive / floor#30 / round-2 | 242 | -787 | -3.25 ±0.68 | -1263 | |
| Competitive / floor#31 / round-1 | 126 | -781 | -6.20 ±1.13 | -727 | |
| Competitive / floor#61 / round-1 | 112 | -770 | -6.88 ±1.19 | -805 | |
| Defensive / floor#60 / round-1 | 163 | -769 | -4.72 ±0.88 | -1129 | |
| Defensive / floor#20 / round-2 | 440 | -723 | -1.64 ±0.55 | -1241 | |
| Defensive / floor#132 / round-1 | 195 | -718 | -3.68 ±0.74 | -1484 | |
| Defensive / floor#61 / round-2 | 190 | -690 | -3.63 ±0.85 | -894 | |
| Defensive / floor#243 / round-1 | 117 | -659 | -5.63 ±1.26 | -819 | |
| Defensive / floor#200 / round-2 | 135 | -657 | -4.87 ±1.05 | -931 | |
| Defensive / floor#131 / balancing | 175 | -633 | -3.62 ±0.86 | -1088 | |
| Competitive / floor#3 / balancing | 272 | -632 | -2.32 ±0.63 | +51 | plain/PD-flip |
| Competitive / floor#46 / round-1 | 110 | -622 | -5.65 ±1.28 | -718 | |
| Defensive / floor#20 / balancing | 302 | -617 | -2.04 ±0.62 | -1215 | |
| Competitive / book+rb / round-2 | 312 | -610 | -1.96 ±0.58 | -933 | |
| Defensive / floor#200 / round-1 | 150 | -529 | -3.53 ±1.19 | -762 | |
| Defensive / floor#45 / round-1 | 126 | -524 | -4.16 ±1.12 | -703 | |
| Defensive / floor#197 / round-1 | 119 | -518 | -4.35 ±1.22 | -587 | |
| Defensive / floor#30 / round-2 | 156 | -485 | -3.11 ±0.94 | -805 | |
| Constructive / fallback@4 / deep | 168 | -467 | -2.78 ±1.07 | -498 | |
| Constructive / fallback@5 / deep | 163 | -452 | -2.77 ±1.14 | -473 | |
| Defensive / floor#243 / round-2 | 86 | -422 | -4.91 ±1.24 | -707 | |
| Competitive / floor#243 / balancing | 83 | -406 | -4.89 ±1.21 | -630 | |
| Defensive / floor#35 / balancing | 208 | -385 | -1.85 ±0.71 | -832 | |
| Defensive / floor#51 / round-1 | 121 | -385 | -3.18 ±1.30 | -357 | |
| Competitive / floor#242 / round-1 | 114 | -375 | -3.29 ±1.21 | -634 | |
| Defensive / floor#132 / balancing | 155 | -369 | -2.38 ±0.72 | -955 | |
| Defensive / floor#46 / round-2 | 151 | -369 | -2.44 ±0.84 | -640 | |
| Competitive / floor#243 / round-2 | 80 | -340 | -4.25 ±1.26 | -507 | |
| Constructive / floor#32 / round-1 | 93 | -329 | -3.54 ±1.33 | -288 | |
| Defensive / floor#21 / round-1 | 80 | -329 | -4.11 ±1.32 | -552 | |
| Defensive / floor#243 / balancing | 109 | -327 | -3.00 ±1.28 | -548 | |
| Constructive / floor#17 / round-1 | 79 | -325 | -4.11 ±1.30 | -258 | |
| Defensive / floor#50 / round-2 | 265 | -311 | -1.17 ±0.67 | -665 | |
| Competitive / floor#6 / round-2 | 91 | -301 | -3.31 ±1.43 | -600 | |
| Defensive / floor#64 / round-2 | 140 | -299 | -2.14 ±1.12 | -513 | |
| Constructive / floor#61 / round-2 | 116 | -298 | -2.57 ±1.06 | -421 | |
| Competitive / floor#16 / round-1 | 45 | -297 | -6.60 ±2.33 | -285 | |
| Defensive / floor#131 / round-1 | 83 | -288 | -3.47 ±1.41 | -444 | |
| Defensive / floor#66 / round-1 | 108 | -282 | -2.61 ±1.35 | -364 | |
| Defensive / floor#3 / deep | 74 | -281 | -3.80 ±1.46 | -286 | |
| Defensive / floor#129 / round-2 | 82 | -274 | -3.34 ±1.45 | -398 | |
| Competitive / floor#237 / round-2 | 79 | -259 | -3.28 ±1.11 | -424 | |
| Defensive / floor#199 / round-1 | 123 | -256 | -2.08 ±1.18 | -310 | |
| Defensive / floor#36 / round-1 | 60 | -254 | -4.23 ±1.30 | -438 | |
| Competitive / floor#242 / deep | 35 | -249 | -7.11 ±2.06 | -388 | |
| Defensive / floor#30 / round-1 | 54 | -241 | -4.46 ±1.74 | -246 | |
| Competitive / floor#16 / round-2 | 38 | -239 | -6.29 ±2.26 | -293 | |
| Competitive / floor#5 / round-2 | 97 | -237 | -2.44 ±1.53 | -371 | |
| Defensive / floor#198 / round-2 | 61 | -237 | -3.89 ±1.69 | -352 | |
| Competitive / floor#61 / round-2 | 37 | -233 | -6.30 ±2.02 | -300 | |
| Competitive / fallback@2 / round-2 | 97 | -230 | -2.37 ±1.37 | -198 | |
| Constructive / floor#46 / deep | 198 | -226 | -1.14 ±0.65 | -473 | |
| Defensive / floor#50 / balancing | 167 | -208 | -1.25 ±0.67 | -696 | |
| Defensive / floor#65 / balancing | 132 | -181 | -1.37 ±0.89 | -534 | |
| Competitive / floor#3+rb / round-2 | 78 | -180 | -2.31 ±1.42 | -213 | |
| Competitive / floor#239 / round-2 | 42 | -176 | -4.19 ±1.96 | -251 | |
| Competitive / floor#241 / round-2 | 44 | -174 | -3.95 ±1.61 | -220 | |
| Competitive / floor#240 / round-2 | 63 | -171 | -2.71 ±1.62 | -214 | |
| Defensive / floor#31 / round-2 | 59 | -171 | -2.90 ±1.72 | -214 | |
| Defensive / floor#133 / round-1 | 174 | -156 | -0.90 ±1.24 | -429 | ~noise |
| Competitive / floor#234 / balancing | 55 | -155 | -2.82 ±1.76 | -216 | |
| Defensive / floor#198 / round-1 | 71 | -155 | -2.18 ±1.69 | -291 | |
| Competitive / fallback@1 / round-2 | 84 | -147 | -1.75 ±1.86 | -111 | ~noise |
| Constructive / floor#46 / round-2 | 127 | -143 | -1.13 ±1.07 | -195 | |
| Defensive / floor#49 / round-1 | 92 | -143 | -1.55 ±1.22 | -130 | |
| Competitive / floor#46 / deep | 34 | -138 | -4.06 ±1.83 | -205 | |
| Defensive / floor#16 / round-2 | 67 | -129 | -1.93 ±1.56 | -138 | |
| Constructive / floor#140 / deep | 130 | -127 | -0.98 ±1.13 | -110 | ~noise |
| Competitive / floor#25 / round-2 | 54 | -123 | -2.28 ±1.46 | -136 | |
| Constructive / floor#147 / round-2 | 23 | -122 | -5.30 ±3.30 | -122 | |
| Competitive / floor#1 / round-2 | 130 | -121 | -0.93 ±1.30 | -107 | ~noise |
| Competitive / floor#235 / round-2 | 25 | -121 | -4.84 ±2.31 | -173 | |
| Constructive / floor#151 / round-2 | 34 | -121 | -3.56 ±3.02 | -120 | |
| Competitive / floor#30 / balancing | 44 | -116 | -2.64 ±1.33 | -235 | |
| Competitive / floor#234+rb / round-2 | 11 | -111 | -10.09 ±2.54 | -129 | |
| Competitive / floor#55 / round-2 | 16 | -110 | -6.88 ±2.58 | -104 | |
| Competitive / floor#240+rb / round-2 | 31 | -106 | -3.42 ±2.88 | -119 | |
| Competitive / floor#31 / balancing | 11 | -104 | -9.45 ±1.36 | -111 | |
| Constructive / floor#145 / round-2 | 47 | -103 | -2.19 ±2.96 | -107 | ~noise |
| Defensive / floor#5 / round-2 | 28 | -102 | -3.64 ±3.02 | -146 | |
| Defensive / floor#17 / round-2 | 31 | -101 | -3.26 ±2.07 | -139 | |
| Defensive / floor#65 / round-2 | 118 | -98 | -0.83 ±0.96 | -223 | ~noise |
| Competitive / floor#3 / deep | 63 | -97 | -1.54 ±1.34 | -102 | |
| Defensive / floor#129 / round-1 | 41 | -94 | -2.29 ±2.22 | -169 | |
| Defensive / floor#47 / round-2 | 51 | -94 | -1.84 ±1.86 | -162 | ~noise |
| Competitive / fallback@5 / round-2 | 50 | -91 | -1.82 ±1.63 | -22 | |
| Competitive / floor#15 / balancing | 29 | -89 | -3.07 ±1.79 | -173 | |
| Competitive / floor#46+rb / deep | 20 | -89 | -4.45 ±1.00 | -173 | |
| Competitive / floor#15 / round-2 | 24 | -86 | -3.58 ±2.43 | -173 | |
| Defensive / floor#237 / round-2 | 18 | -81 | -4.50 ±2.36 | -113 | |
| Defensive / floor#6 / round-2 | 26 | -77 | -2.96 ±3.13 | -117 | ~noise |
| Defensive / floor#12 / round-2 | 25 | -76 | -3.04 ±1.99 | -89 | |
| Competitive / floor#31 / round-2 | 28 | -75 | -2.68 ±2.16 | -80 | |
| Competitive / floor#32 / round-1 | 12 | -72 | -6.00 ±3.70 | -59 | |
| Competitive / floor#61 / deep | 34 | -71 | -2.09 ±1.89 | -162 | |
| Constructive / floor#31 / deep | 8 | -71 | -8.88 ±1.55 | -73 | |
| Defensive / floor#1 / deep | 19 | -71 | -3.74 ±2.94 | -89 | |
| Constructive / floor#62 / round-1 | 24 | -70 | -2.92 ±2.41 | -76 | |
| Defensive / floor#5 / round-1 | 15 | -69 | -4.60 ±4.17 | -62 | |
| Competitive / floor#243+rb / round-2 | 15 | -67 | -4.47 ±3.35 | -132 | |
| Competitive / floor#2 / round-2 | 75 | -66 | -0.88 ±1.86 | -211 | ~noise |
| Defensive / floor#33 / round-1 | 8 | -64 | -8.00 ±3.05 | -69 | |
| Defensive / floor#55 / round-2 | 10 | -64 | -6.40 ±2.40 | -64 | |
| Competitive / floor#236 / round-2 | 8 | -62 | -7.75 ±3.35 | -70 | |
| Defensive / floor#26 / round-2 | 9 | -62 | -6.89 ±4.10 | -77 | |
| Competitive / floor#30 / deep | 15 | -61 | -4.07 ±2.59 | -82 | |
| Defensive / floor#11 / round-2 | 12 | -60 | -5.00 ±1.91 | -102 | |
| Defensive / floor#153 / round-1 | 6 | -60 | -10.00 ±2.63 | -48 | |
| Defensive / floor#235 / round-2 | 17 | -60 | -3.53 ±2.21 | -91 | |
| Defensive / floor#63 / round-2 | 17 | -59 | -3.47 ±3.19 | -52 | |
| Competitive / floor#16 / deep | 14 | -57 | -4.07 ±1.50 | -87 | |
| Competitive / floor#236 / balancing | 52 | -56 | -1.08 ±1.60 | -33 | ~noise |
| Competitive / floor#27 / round-2 | 7 | -55 | -7.86 ±3.54 | -66 | |
| Constructive / floor#140 / round-1 | 4 | -55 | -13.75 ±2.45 | -55 | |
| Competitive / floor#10 / round-2 | 29 | -54 | -1.86 ±2.33 | -55 | ~noise |
| Competitive / floor#140 / round-1 | 33 | -53 | -1.61 ±3.20 | -46 | ~noise |
| Competitive / floor#235 / deep | 8 | -53 | -6.62 ±3.68 | -80 | |
| Defensive / floor#239 / round-2 | 12 | -53 | -4.42 ±5.02 | -92 | ~noise |
| Defensive / floor#49 / balancing | 70 | -53 | -0.76 ±1.53 | -171 | ~noise |
| Competitive / floor#240 / balancing | 25 | -52 | -2.08 ±2.23 | -63 | ~noise |
| Competitive / fallback@6 / round-2 | 10 | -51 | -5.10 ±4.29 | -36 | |
| Competitive / floor#238 / round-2 | 24 | -49 | -2.04 ±3.08 | -43 | ~noise |
| Competitive / floor#48 / round-2 | 9 | -49 | -5.44 ±3.70 | -103 | |
| Defensive / floor#27 / round-2 | 10 | -49 | -4.90 ±4.05 | -48 | |
| Competitive / floor#143 / round-2 | 5 | -48 | -9.60 ±3.85 | -50 | |
| Competitive / floor#17 / deep | 11 | -48 | -4.36 ±3.12 | -58 | |
| Competitive / floor#241 / deep | 12 | -48 | -4.00 ±4.15 | -71 | ~noise |
| Defensive / floor#204 / round-1 | 15 | -48 | -3.20 ±3.23 | -35 | ~noise |
| Defensive / floor#21 / balancing | 33 | -48 | -1.45 ±1.92 | -88 | ~noise |
| Defensive / floor#51 / balancing | 24 | -48 | -2.00 ±1.95 | -60 | |
| Competitive / floor#16 / balancing | 27 | -47 | -1.74 ±1.93 | -140 | ~noise |
| Competitive / floor#47 / round-2 | 30 | -47 | -1.57 ±2.72 | -124 | ~noise |
| Constructive / floor#16 / round-2 | 14 | -46 | -3.29 ±3.02 | -43 | |
| Competitive / floor#153 / round-1 | 3 | -45 | -15.00 ±2.99 | -45 | |
| Constructive / floor#153 / round-2 | 18 | -45 | -2.50 ±4.47 | -36 | ~noise |
| Constructive / floor#157 / round-1 | 4 | -45 | -11.25 ±1.67 | -45 | |
| Defensive / floor#21 / round-2 | 25 | -45 | -1.80 ±1.91 | -118 | ~noise |
| Competitive / floor#242+rb / balancing | 8 | -44 | -5.50 ±5.00 | -69 | |
| Defensive / floor#1 / round-2 | 128 | -44 | -0.34 ±1.43 | -193 | ~noise |
| Defensive / floor#205 / round-1 | 24 | -44 | -1.83 ±2.76 | -38 | ~noise |
| Competitive / floor#238+rb / round-2 | 23 | -42 | -1.83 ±3.05 | -18 | ~noise |
| Constructive / floor#63 / round-2 | 7 | -42 | -6.00 ±0.74 | -68 | |
| Defensive / floor#48 / round-2 | 9 | -41 | -4.56 ±5.53 | -57 | ~noise |
| Competitive / floor#234 / round-2 | 19 | -40 | -2.11 ±3.39 | -64 | ~noise |
| Defensive / floor#242 / deep | 6 | -40 | -6.67 ±2.51 | -82 | |
| Competitive / floor#129 / round-2 | 13 | -39 | -3.00 ±4.96 | -107 | ~noise |
| Constructive / floor#30 / round-1 | 16 | -39 | -2.44 ±2.91 | -57 | ~noise |
| Defensive / floor#36 / balancing | 21 | -39 | -1.86 ±1.91 | -92 | ~noise |
| Competitive / floor#9 / round-2 | 24 | -38 | -1.58 ±1.63 | -66 | ~noise |
| Defensive / floor#33 / round-2 | 7 | -38 | -5.43 ±3.05 | -29 | |
| Competitive / floor#235+rb / balancing | 4 | -37 | -9.25 ±2.45 | -37 | |
| Competitive / floor#237 / balancing | 6 | -36 | -6.00 ±2.77 | -73 | |
| Competitive / floor#39 / round-2 | 7 | -36 | -5.14 ±1.93 | -47 | |
| Defensive / floor#36 / round-2 | 9 | -35 | -3.89 ±2.05 | -42 | |
| Defensive / floor#240 / round-2 | 8 | -34 | -4.25 ±2.64 | -49 | |
| Defensive / floor#54 / deep | 6 | -34 | -5.67 ±4.37 | -49 | |
| Competitive / floor#33 / round-2 | 9 | -32 | -3.56 ±2.55 | -56 | |
| Competitive / floor#63 / round-2 | 8 | -32 | -4.00 ±3.88 | -42 | |
| Defensive / floor#1 / round-1 | 8 | -32 | -4.00 ±7.43 | -33 | ~noise |
| Defensive / floor#241 / deep | 5 | -32 | -6.40 ±6.95 | -34 | ~noise |
| Competitive / floor#147 / balancing | 2 | -31 | -15.50 ±0.98 | -31 | |
| Defensive / floor#147 / round-1 | 3 | -31 | -10.33 ±10.27 | -31 | |
| Defensive / floor#17 / round-1 | 11 | -31 | -2.82 ±4.33 | -26 | ~noise |
| Defensive / floor#32 / round-2 | 35 | -29 | -0.83 ±1.97 | -84 | ~noise |
| Competitive / floor#238 / balancing | 37 | -28 | -0.76 ±2.01 | -55 | ~noise |
| Defensive / floor#203 / round-2 | 10 | -28 | -2.80 ±3.44 | -51 | ~noise |
| Competitive / floor#239 / balancing | 5 | -27 | -5.40 ±4.37 | -58 | |
| Competitive / floor#24 / round-2 | 6 | -27 | -4.50 ±10.33 | -26 | ~noise |
| Competitive / floor#45+rb / round-2 | 4 | -27 | -6.75 ±4.83 | -52 | |
| Competitive / floor#62 / round-1 | 2 | -27 | -13.50 ±0.98 | -28 | |
| Defensive / floor#14 / round-2 | 3 | -27 | -9.00 ±2.99 | -23 | |
| Competitive / floor#242+rb / deep | 8 | -26 | -3.25 ±6.58 | -59 | ~noise |
| Defensive / floor#61 / deep | 4 | -26 | -6.50 ±4.56 | -42 | |
| Defensive / floor#231 / round-2 | 4 | -25 | -6.25 ±2.93 | -47 | |
| Defensive / floor#34 / balancing | 59 | -25 | -0.42 ±1.65 | -42 | ~noise |
| Competitive / book+rb / deep | 16 | -24 | -1.50 ±2.12 | -38 | ~noise |
| Competitive / floor#153 / round-2 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Competitive / floor#241+rb / round-2 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Competitive / floor#47 / round-1 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Competitive / floor#63 / round-1 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Constructive / floor#145 / round-1 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Constructive / floor#148 / round-2 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Defensive / floor#49 / round-2 | 37 | -24 | -0.65 ±2.18 | -63 | ~noise |
| Competitive / floor#129 / deep | 4 | -23 | -5.75 ±6.01 | -28 | ~noise |
| Competitive / floor#241+rb / balancing | 4 | -22 | -5.50 ±1.70 | -35 | |
| Constructive / floor#31 / round-2 | 4 | -22 | -5.50 ±3.62 | -29 | |
| Constructive / floor#32 / deep | 30 | -22 | -0.73 ±0.73 | -24 | |
| Defensive / floor#63 / round-1 | 4 | -22 | -5.50 ±8.26 | -3 | ~noise |
| Competitive / floor#60+rb / round-2 | 2 | -21 | -10.50 ±2.94 | -17 | |
| Constructive / floor#154 / deep | 7 | -21 | -3.00 ±4.59 | -21 | ~noise |
| Competitive / fallback@2 / balancing | 2 | -20 | -10.00 ±1.96 | -20 | |
| Defensive / floor#17 / deep | 2 | -20 | -10.00 ±3.92 | -23 | |
| Defensive / floor#197 / round-2 | 40 | -20 | -0.50 ±2.42 | -59 | ~noise |
| Defensive / floor#32 / round-1 | 5 | -20 | -4.00 ±7.99 | -10 | ~noise |
| Competitive / floor#147 / round-1 | 2 | -19 | -9.50 ±2.94 | -14 | |
| Competitive / floor#235 / balancing | 6 | -19 | -3.17 ±4.92 | -23 | ~noise |
| Competitive / floor#39 / deep | 7 | -19 | -2.71 ±1.19 | -36 | |
| Competitive / floor#31 / deep | 12 | -18 | -1.50 ±3.73 | -18 | ~noise |
| Defensive / floor#18 / round-2 | 5 | -18 | -3.60 ±6.75 | -14 | ~noise |
| Defensive / floor#29 / round-2 | 2 | -18 | -9.00 ±3.92 | -18 | |
| Defensive / floor#32 / deep | 7 | -18 | -2.57 ±1.41 | -38 | |
| Competitive / floor#32 / round-2 | 19 | -17 | -0.89 ±3.15 | -40 | ~noise |
| Competitive / floor#45 / balancing | 4 | -17 | -4.25 ±12.89 | -5 | ~noise |
| Competitive / floor#45+rb / deep | 3 | -17 | -5.67 ±1.73 | -31 | |
| Defensive / floor#235 / deep | 3 | -17 | -5.67 ±5.58 | -21 | |
| Defensive / floor#236 / round-2 | 2 | -17 | -8.50 ±2.94 | -17 | |
| Defensive / floor#40 / round-2 | 4 | -17 | -4.25 ±4.69 | -28 | ~noise |
| Defensive / floor#46 / round-1 | 8 | -17 | -2.12 ±4.15 | -22 | ~noise |
| Defensive / floor#54 / round-2 | 4 | -17 | -4.25 ±2.17 | -40 | |
| Constructive / floor#47 / deep | 4 | -16 | -4.00 ±1.79 | -26 | |
| Defensive / floor#239 / balancing | 10 | -16 | -1.60 ±2.76 | -16 | ~noise |
| Defensive / floor#241 / round-2 | 2 | -16 | -8.00 ±3.92 | -16 | |
| Defensive / floor#29 / round-1 | 6 | -16 | -2.67 ±6.82 | -9 | ~noise |
| Competitive / floor#18 / round-2 | 14 | -15 | -1.07 ±3.12 | +7 | ~noise plain/PD-flip |
| Competitive / floor#46 / balancing | 4 | -15 | -3.75 ±1.86 | -39 | |
| Defensive / floor#66 / balancing | 3 | -15 | -5.00 ±5.19 | -17 | ~noise |
| Competitive / fallback@3+rb / round-2 | 2 | -14 | -7.00 ±0.00 | -14 | |
| Competitive / floor#135 / round-2 | 2 | -14 | -7.00 ±3.92 | -26 | |
| Competitive / floor#140 / deep | 2 | -14 | -7.00 ±1.96 | -22 | |
| Competitive / floor#33 / round-1 | 2 | -14 | -7.00 ±3.92 | -26 | |
| Competitive / floor#6 / deep | 2 | -14 | -7.00 ±3.92 | -26 | |
| Constructive / floor#17 / deep | 17 | -14 | -0.82 ±0.88 | -19 | ~noise |
| Defensive / floor#11 / deep | 2 | -14 | -7.00 ±13.72 | -14 | ~noise |
| Defensive / floor#61 / balancing | 14 | -14 | -1.00 ±3.02 | -29 | ~noise |
| Competitive / floor#140+rb / round-2 | 2 | -13 | -6.50 ±8.82 | -13 | ~noise |
| Competitive / floor#145 / round-1 | 1 | -13 | -13.00 ±0.00 | -13 | ~noise |
| Competitive / floor#17 / round-1 | 6 | -13 | -2.17 ±6.85 | +5 | ~noise plain/PD-flip |
| Competitive / floor#48 / round-1 | 1 | -13 | -13.00 ±0.00 | -13 | ~noise |
| Competitive / floor#57 / round-2 | 1 | -13 | -13.00 ±0.00 | -14 | ~noise |
| Defensive / floor#205 / round-2 | 6 | -13 | -2.17 ±7.02 | -43 | ~noise |
| Defensive / floor#39 / round-2 | 3 | -13 | -4.33 ±3.46 | -36 | |
| Defensive / floor#56 / round-1 | 10 | -13 | -1.30 ±3.05 | -18 | ~noise |
| Competitive / floor#47+rb / balancing | 2 | -12 | -6.00 ±1.96 | -12 | |
| Constructive / floor#145 / deep | 7 | -12 | -1.71 ±8.02 | -12 | ~noise |
| Defensive / floor#151 / round-1 | 2 | -11 | -5.50 ±0.98 | -14 | |
| Defensive / floor#228 / round-2 | 3 | -11 | -3.67 ±8.79 | -10 | ~noise |
| Defensive / floor#63 / deep | 2 | -11 | -5.50 ±2.94 | -14 | |
| Competitive / floor#237 / deep | 7 | -10 | -1.43 ±2.30 | -46 | ~noise |
| Defensive / floor#238 / round-2 | 1 | -10 | -10.00 ±0.00 | -10 | ~noise |
| Defensive / floor#28 / round-2 | 1 | -10 | -10.00 ±0.00 | -10 | ~noise |
| Defensive / floor#62 / round-2 | 46 | -10 | -0.22 ±2.05 | -88 | ~noise |
| Competitive / fallback@5+rb / round-2 | 2 | -9 | -4.50 ±0.98 | -12 | |
| Competitive / floor#62 / deep | 7 | -9 | -1.29 ±4.83 | -19 | ~noise |
| Defensive / floor#60 / deep | 6 | -9 | -1.50 ±1.58 | -26 | ~noise |
| Competitive / floor#11 / round-2 | 2 | -8 | -4.00 ±1.96 | -23 | |
| Competitive / floor#236+rb / round-2 | 12 | -8 | -0.67 ±4.46 | -12 | ~noise |
| Competitive / floor#30+rb / round-2 | 4 | -8 | -2.00 ±2.26 | -40 | ~noise |
| Competitive / floor#56 / deep | 2 | -8 | -4.00 ±1.96 | -21 | |
| Competitive / floor#62 / balancing | 3 | -8 | -2.67 ±5.23 | +17 | ~noise plain/PD-flip |
| Defensive / floor#204 / round-2 | 11 | -8 | -0.73 ±2.27 | -41 | ~noise |
| Defensive / floor#237 / deep | 2 | -8 | -4.00 ±3.92 | -22 | |
| Defensive / floor#61 / round-1 | 22 | -8 | -0.36 ±3.42 | -12 | ~noise |
| Competitive / floor#151 / round-2 | 10 | -7 | -0.70 ±6.36 | -16 | ~noise |
| Competitive / floor#235+rb / deep | 5 | -7 | -1.40 ±2.02 | -40 | ~noise |
| Defensive / floor#18 / deep | 4 | -7 | -1.75 ±5.08 | -16 | ~noise |
| Defensive / floor#218 / round-2 | 3 | -7 | -2.33 ±0.65 | -17 | |
| Competitive / floor#151 / round-1 | 8 | -6 | -0.75 ±4.09 | +4 | ~noise plain/PD-flip |
| Competitive / floor#54 / round-2 | 4 | -6 | -1.50 ±9.88 | -7 | ~noise |
| Competitive / floor#239+rb / balancing | 2 | -5 | -2.50 ±4.90 | -26 | ~noise |
| Competitive / floor#60 / balancing | 6 | -5 | -0.83 ±4.09 | -33 | ~noise |
| Competitive / floor#61+rb / deep | 8 | -5 | -0.62 ±0.82 | -12 | ~noise |
| Defensive / floor#219 / round-2 | 1 | -5 | -5.00 ±0.00 | -6 | ~noise |
| Defensive / floor#241 / balancing | 11 | -5 | -0.45 ±2.83 | -11 | ~noise |
| Defensive / floor#46 / deep | 2 | -5 | -2.50 ±0.98 | -12 | |
| Competitive / floor#140 / round-2 | 39 | -4 | -0.10 ±2.11 | -34 | ~noise |
| Competitive / floor#18 / round-1 | 11 | -4 | -0.36 ±5.22 | +18 | ~noise plain/PD-flip |
| Competitive / floor#47 / balancing | 19 | -4 | -0.21 ±3.48 | -17 | ~noise |
| Defensive / floor#140 / balancing | 2 | -4 | -2.00 ±0.00 | -4 | |
| Competitive / floor#3+rb / deep | 12 | -3 | -0.25 ±3.28 | -60 | ~noise |
| Defensive / floor#240 / deep | 1 | -3 | -3.00 ±0.00 | -9 | ~noise |
| Competitive / floor#241 / balancing | 11 | -2 | -0.18 ±1.99 | +5 | ~noise plain/PD-flip |
| Competitive / floor#243 / round-1 | 3 | -2 | -0.67 ±5.23 | -4 | ~noise |
| Defensive / floor#42 / round-2 | 17 | -2 | -0.12 ±5.33 | +14 | ~noise plain/PD-flip |
| Competitive / floor#145 / round-2 | 6 | -1 | -0.17 ±6.51 | -8 | ~noise |
| Competitive / floor#42 / round-2 | 2 | -1 | -0.50 ±4.90 | -6 | ~noise |
| Competitive / floor#46+rb / round-2 | 4 | -1 | -0.25 ±6.85 | +21 | ~noise plain/PD-flip |
| Defensive / floor#16 / round-1 | 2 | -1 | -0.50 ±4.90 | -2 | ~noise |
| Defensive / floor#238 / deep | 2 | -1 | -0.50 ±2.94 | -12 | ~noise |
| Defensive / floor#41 / deep | 1 | -1 | -1.00 ±0.00 | -1 | ~noise |
| Defensive / floor#62 / deep | 3 | -1 | -0.33 ±10.14 | -3 | ~noise |
| Defensive / floor#64 / balancing | 84 | -1 | -0.01 ±1.20 | -71 | ~noise |
| Competitive / floor#143 / balancing | 2 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Competitive / floor#151 / balancing | 2 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Competitive / floor#32 / deep | 3 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Competitive / floor#41 / deep | 1 | +0 | +0.00 ±0.00 | +7 | ~noise |
| Constructive / floor#151 / deep | 4 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Constructive / floor#47 / round-1 | 10 | +0 | +0.00 ±4.98 | +0 | ~noise |
| Competitive / floor#14 / round-2 | 1 | +1 | +1.00 ±0.00 | +1 | ~noise |
| Competitive / floor#33 / balancing | 1 | +1 | +1.00 ±0.00 | -2 | ~noise plain/PD-flip |
| Competitive / floor#60+rb / deep | 2 | +1 | +0.50 ±0.98 | -3 | ~noise plain/PD-flip |
| Defensive / floor#14 / round-1 | 5 | +1 | +0.20 ±4.57 | +7 | ~noise |
| Defensive / floor#16 / deep | 1 | +1 | +1.00 ±0.00 | -2 | ~noise plain/PD-flip |
| Defensive / floor#237 / balancing | 1 | +1 | +1.00 ±0.00 | -2 | ~noise plain/PD-flip |
| Constructive / floor#148 / deep | 2 | +2 | +1.00 ±0.00 | +2 | |
| Defensive / floor#18 / round-1 | 4 | +2 | +0.50 ±8.64 | +11 | ~noise |
| Defensive / floor#203 / round-1 | 8 | +2 | +0.25 ±4.99 | +15 | ~noise |
| Defensive / floor#31 / deep | 2 | +2 | +1.00 ±1.96 | -3 | ~noise plain/PD-flip |
| Competitive / floor#239 / deep | 6 | +3 | +0.50 ±4.01 | -40 | ~noise plain/PD-flip |
| Defensive / floor#31 / round-1 | 5 | +3 | +0.60 ±5.80 | +0 | ~noise |
| Competitive / floor#147 / deep | 2 | +4 | +2.00 ±0.00 | +4 | |
| Competitive / floor#18 / deep | 2 | +4 | +2.00 ±5.88 | +9 | ~noise |
| Constructive / floor#62 / deep | 8 | +4 | +0.50 ±4.86 | -8 | ~noise plain/PD-flip |
| Defensive / floor#145 / round-1 | 5 | +4 | +0.80 ±10.67 | +12 | ~noise |
| Competitive / floor#5 / deep | 6 | +5 | +0.83 ±4.97 | +2 | ~noise |
| Competitive / floor#62+rb / round-2 | 2 | +5 | +2.50 ±20.58 | +12 | ~noise |
| Competitive / floor#40 / deep | 1 | +6 | +6.00 ±0.00 | +8 | ~noise |
| Competitive / floor#40 / round-2 | 18 | +6 | +0.33 ±3.46 | -6 | ~noise plain/PD-flip |
| Competitive / floor#47 / deep | 8 | +6 | +0.75 ±4.20 | -2 | ~noise plain/PD-flip |
| Competitive / floor#32 / balancing | 1 | +7 | +7.00 ±0.00 | +7 | ~noise |
| Constructive / floor#157 / deep | 8 | +7 | +0.88 ±0.78 | +14 | |
| Defensive / floor#42 / round-1 | 1 | +7 | +7.00 ±0.00 | +10 | ~noise |
| Defensive / floor#0 / round-1 | 2 | +8 | +4.00 ±1.96 | +8 | |
| Competitive / floor#237+rb / balancing | 2 | +9 | +4.50 ±0.98 | +12 | |
| Defensive / floor#34 / round-1 | 1 | +9 | +9.00 ±0.00 | +9 | ~noise |
| Defensive / floor#38 / round-2 | 2 | +9 | +4.50 ±0.98 | +12 | |
| Defensive / floor#39 / deep | 2 | +9 | +4.50 ±0.98 | -12 | plain/PD-flip |
| Defensive / floor#57 / round-2 | 1 | +9 | +9.00 ±0.00 | +9 | ~noise |
| Competitive / floor#12 / round-2 | 8 | +10 | +1.25 ±3.62 | +3 | ~noise |
| Competitive / floor#60 / deep | 1 | +10 | +10.00 ±0.00 | +10 | ~noise |
| Constructive / floor#147 / deep | 8 | +10 | +1.25 ±6.38 | +24 | ~noise |
| Constructive / floor#48 / round-2 | 1 | +10 | +10.00 ±0.00 | +10 | ~noise |
| Defensive / floor#56 / round-2 | 14 | +10 | +0.71 ±4.29 | +18 | ~noise |
| Defensive / floor#57 / deep | 1 | +10 | +10.00 ±0.00 | +4 | ~noise |
| Competitive / floor#239+rb / deep | 1 | +12 | +12.00 ±0.00 | +10 | ~noise |
| Defensive / floor#13 / round-2 | 1 | +12 | +12.00 ±0.00 | +11 | ~noise |
| Defensive / floor#235 / balancing | 5 | +12 | +2.40 ±6.31 | +1 | ~noise |
| Competitive / floor#15 / deep | 2 | +13 | +6.50 ±0.98 | +16 | |
| Defensive / floor#62 / round-1 | 5 | +13 | +2.60 ±8.51 | +13 | ~noise |
| Competitive / floor#62 / round-2 | 15 | +14 | +0.93 ±3.51 | -56 | ~noise plain/PD-flip |
| Defensive / floor#140 / round-1 | 5 | +14 | +2.80 ±10.29 | +16 | ~noise |
| Competitive / floor#140 / balancing | 2 | +16 | +8.00 ±3.92 | +16 | |
| Constructive / floor#153 / deep | 2 | +16 | +8.00 ±3.92 | +16 | |
| Constructive / floor#48 / deep | 2 | +16 | +8.00 ±3.92 | +16 | |
| Defensive / floor#145 / balancing | 2 | +16 | +8.00 ±3.92 | +16 | |
| Defensive / floor#41 / round-1 | 5 | +16 | +3.20 ±1.57 | +47 | |
| Defensive / floor#47 / deep | 2 | +16 | +8.00 ±3.92 | +16 | |
| Competitive / floor#144 / round-2 | 4 | +18 | +4.50 ±7.40 | +20 | ~noise |
| Competitive / floor#0 / deep | 2 | +20 | +10.00 ±3.92 | +20 | |
| Defensive / floor#2 / round-1 | 3 | +23 | +7.67 ±5.23 | +23 | |
| Competitive / floor#41 / round-2 | 2 | +24 | +12.00 ±1.96 | +24 | |
| Constructive / floor#157 / round-2 | 12 | +24 | +2.00 ±4.89 | +24 | ~noise |
| Defensive / floor#40 / deep | 2 | +24 | +12.00 ±1.96 | +24 | |
| Defensive / floor#48 / round-1 | 4 | +24 | +6.00 ±7.46 | +25 | ~noise |
| Defensive / floor#199 / round-2 | 36 | +27 | +0.75 ±1.89 | -38 | ~noise plain/PD-flip |
| Defensive / floor#129 / deep | 7 | +28 | +4.00 ±6.49 | +25 | ~noise |
| Defensive / floor#207 / round-2 | 4 | +29 | +7.25 ±5.45 | +39 | |
| Competitive / floor#47+rb / round-2 | 2 | +30 | +15.00 ±1.96 | +30 | |
| Competitive / floor#1 / deep | 60 | +31 | +0.52 ±1.74 | +21 | ~noise |
| Defensive / floor#210 / round-2 | 4 | +35 | +8.75 ±3.78 | +38 | |
| Competitive / floor#145 / balancing | 4 | +40 | +10.00 ±2.89 | +40 | |
| Defensive / floor#6 / round-1 | 13 | +42 | +3.23 ±4.05 | +49 | ~noise |
| Competitive / floor#147 / round-2 | 9 | +51 | +5.67 ±6.69 | +53 | ~noise |
| Defensive / floor#133 / balancing | 45 | +51 | +1.13 ±2.27 | +3 | ~noise |
| Defensive / floor#41 / round-2 | 27 | +55 | +2.04 ±2.76 | +48 | ~noise |
| Competitive / floor#0 / round-2 | 33 | +129 | +3.91 ±1.96 | +111 | |
| Defensive / floor#0 / round-2 | 18 | +139 | +7.72 ±1.49 | +119 | |

## By phase

  -170567 IMPs   65524 boards  Defensive
  -155271 IMPs   67336 boards  Constructive
   -82224 IMPs   25472 boards  Competitive

## By provenance

  -248194 IMPs  105221 boards  book
   -37975 IMPs   12265 boards  floor#3
   -18391 IMPs    5547 boards  fallback@2
   -17820 IMPs    5791 boards  fallback@1
   -17254 IMPs    5157 boards  floor#242
   -12919 IMPs    4297 boards  fallback@3
    -6016 IMPs    1801 boards  fallback@4
    -3182 IMPs    1363 boards  floor#20
    -3135 IMPs    1021 boards  floor#61
    -3055 IMPs    1270 boards  floor#140
    -2751 IMPs    1311 boards  floor#35
    -2720 IMPs     418 boards  floor#242+rb
    -2689 IMPs     791 boards  floor#60
    -2653 IMPs     923 boards  floor#46
    -2220 IMPs    1203 boards  floor#50
    -2219 IMPs     468 boards  floor#202
    -2156 IMPs     478 boards  floor#243
    -1781 IMPs     765 boards  floor#64
    -1729 IMPs     527 boards  floor#30
    -1386 IMPs     376 boards  floor#45
    -1237 IMPs     255 boards  floor#31
    -1186 IMPs     285 boards  floor#200
    -1139 IMPs     700 boards  floor#65
    -1087 IMPs     350 boards  floor#132
     -921 IMPs     258 boards  floor#131
     -815 IMPs     208 boards  floor#16
     -634 IMPs     328 boards  book+rb
     -552 IMPs     157 boards  floor#17
     -543 IMPs     213 boards  fallback@5
     -538 IMPs     159 boards  floor#197
     -500 IMPs     205 boards  floor#32
     -433 IMPs     145 boards  floor#51
     -422 IMPs     138 boards  floor#21
     -403 IMPs     146 boards  floor#5
     -402 IMPs     147 boards  floor#129
     -393 IMPs     113 boards  floor#237
     -392 IMPs     132 boards  floor#198
     -350 IMPs     132 boards  floor#6
     -328 IMPs      90 boards  floor#36
     -297 IMPs     111 boards  floor#66
     -277 IMPs      85 boards  floor#241
     -269 IMPs      75 boards  floor#239
     -260 IMPs      97 boards  floor#240
     -258 IMPs      64 boards  floor#235
     -237 IMPs     345 boards  floor#1
     -229 IMPs     159 boards  floor#199
     -220 IMPs     199 boards  floor#49
     -195 IMPs      74 boards  floor#234
     -190 IMPs      40 boards  floor#63
     -183 IMPs      90 boards  floor#3+rb
     -174 IMPs      26 boards  floor#55
     -163 IMPs     126 boards  floor#47
     -162 IMPs      55 boards  floor#15
     -158 IMPs      31 boards  floor#153
     -147 IMPs      27 boards  floor#33
     -145 IMPs      60 boards  floor#151
     -138 IMPs      49 boards  floor#147
     -135 IMPs      62 boards  floor#236
     -123 IMPs      54 boards  floor#25
     -111 IMPs      11 boards  floor#234+rb
     -106 IMPs      31 boards  floor#240+rb
     -105 IMPs     219 boards  floor#133
     -104 IMPs      17 boards  floor#27
      -94 IMPs     113 boards  floor#62
      -93 IMPs      74 boards  floor#145
      -90 IMPs      24 boards  floor#46+rb
      -88 IMPs      64 boards  floor#238
      -82 IMPs      16 boards  floor#11
      -67 IMPs      15 boards  floor#243+rb
      -66 IMPs      33 boards  floor#12
      -62 IMPs       9 boards  floor#26
      -59 IMPs      19 boards  floor#39
      -57 IMPs      30 boards  floor#205
      -57 IMPs      14 boards  floor#54
      -56 IMPs      26 boards  floor#204
      -54 IMPs      29 boards  floor#10
      -53 IMPs      26 boards  floor#48
      -51 IMPs      10 boards  fallback@6
      -48 IMPs       7 boards  floor#143
      -46 IMPs       6 boards  floor#241+rb
      -44 IMPs       9 boards  floor#235+rb
      -44 IMPs       7 boards  floor#45+rb
      -43 IMPs      78 boards  floor#2
      -42 IMPs      23 boards  floor#238+rb
      -38 IMPs      40 boards  floor#18
      -38 IMPs      24 boards  floor#9
      -34 IMPs       8 boards  floor#29
      -27 IMPs       6 boards  floor#24
      -26 IMPs      18 boards  floor#203
      -25 IMPs       9 boards  floor#14
      -25 IMPs       4 boards  floor#231
      -22 IMPs       4 boards  floor#148
      -21 IMPs       7 boards  floor#154
      -20 IMPs       4 boards  floor#60+rb
      -16 IMPs      60 boards  floor#34
      -14 IMPs       2 boards  fallback@3+rb
      -14 IMPs       2 boards  floor#135
      -14 IMPs      24 boards  floor#157
      -13 IMPs       2 boards  floor#140+rb
      -11 IMPs       3 boards  floor#228
      -11 IMPs      26 boards  floor#56
      -10 IMPs       1 boards  floor#28
       -9 IMPs       2 boards  fallback@5+rb
       -8 IMPs      12 boards  floor#236+rb
       -8 IMPs       4 boards  floor#30+rb
       -7 IMPs       3 boards  floor#218
       -5 IMPs       1 boards  floor#219
       -5 IMPs       8 boards  floor#61+rb
       +4 IMPs      20 boards  floor#42
       +5 IMPs       2 boards  floor#62+rb
       +6 IMPs       3 boards  floor#57
       +7 IMPs       3 boards  floor#239+rb
       +9 IMPs       2 boards  floor#237+rb
       +9 IMPs       2 boards  floor#38
      +12 IMPs       1 boards  floor#13
      +18 IMPs       4 boards  floor#144
      +18 IMPs       4 boards  floor#47+rb
      +19 IMPs      25 boards  floor#40
      +29 IMPs       4 boards  floor#207
      +35 IMPs       4 boards  floor#210
      +94 IMPs      36 boards  floor#41
     +296 IMPs      55 boards  floor#0

## By family

  -212576 IMPs   77407 boards  round-1
  -110301 IMPs   42623 boards  round-2
   -68344 IMPs   29232 boards  opening
   -10876 IMPs    4682 boards  balancing
    -5965 IMPs    4388 boards  deep

## By direction

  -265244 IMPs   35919 boards  other
  -128768 IMPs   18803 boards  overbid
   -88599 IMPs   10211 boards  missed-game
   -76549 IMPs   10577 boards  sold-out
   -45071 IMPs    7970 boards  wrong-strain
   -40259 IMPs    3281 boards  missed-slam
    -5916 IMPs     399 boards  missed-grand
    -5568 IMPs     758 boards  doubling
       +0 IMPs   25664 boards  flat
  +247912 IMPs   44750 boards  gain

## Worst boards per losing bucket

### Defensive / book / round-1 (40416 boards, -98478 IMPs)

[vul both, seed Some(1783375065), board 2073] swing -2840 pts / -21 IMPs (PD -21), diverged at call 2 (X ours vs P BBA), wrong-strain
  rule: 12+ HCP, and at most three cards in each of their suits
  W:QT.A5432.AJ74.JT K94.J.KQ5.AKQ985 86.QT986.T983.42 AJ7532.K7.62.763
  ours NS @ A: - 2♠ - 2NT - 3♥ X - - -  -> 3♥x by South
  ours EW @ B: - 2♠ X XX - - -  -> 2♠xx by South

[vul both, seed Some(1783375075), board 4291] swing -2650 pts / -21 IMPs (PD -21), diverged at call 1 (2♣ ours vs P BBA), other
  rule: 5+ ♣, and 8–16 points
  W:KQT973.JT.AQ42.J 6.KQ64.KJ3.98432 AJ854.A752.9875. 2.983.T6.AKQT765
  ours NS @ A: 1♠ 2♣ 3♣ - 3♦ - 3♥ - 3NT - 4♠ - - -  -> 4♠ by West
  ours EW @ B: 1♠ - 4♣ X - - -  -> 4♣x by East

[vul both, seed Some(1783375077), board 2125] swing -2560 pts / -21 IMPs (PD -21), diverged at call 2 (2♥ ours vs X BBA), other
  rule: 5+ ♠, (5+ ♣, or 5+ ♦), and 8+ points
  W:AKJT7652...AKJ86 4.T62.A652.QT542 Q98.QJ543.QT73.7 3.AK987.KJ984.93
  ours NS @ A: - 1♥ X 2♥ - - 6♠ - - -  -> 6♠ by West
  ours EW @ B: - 1♥ 2♥ X - - -  -> 2♥x by West

### Constructive / book / opening (29232 boards, -68344 IMPs)

[vul both, seed Some(1783375065), board 4051] swing -3130 pts / -22 IMPs (PD -22), diverged at call 0 (1♠ ours vs 1♦ BBA), other
  rule: 12–21 points, and 5+ ♠
  W:KJT72.73.AKQT86. 3.Q98.J7.QT97642 A98654.K64.953.A Q.AJT52.42.KJ853
  ours NS @ A: 1♦ - 1♠ 2♥ 4♠ - 4NT - 5♥ - 6♠ - - -  -> 6♠ by East
  ours EW @ B: 1♠ - 4♣ X - - -  -> 4♣x by East

[vul both, seed Some(1783375067), board 372] swing -3400 pts / -22 IMPs (PD -22), diverged at call 0 (2♦ ours vs P BBA), overbid
  rule: exactly 6 ♦, 5–10 points, and not (opening in seat 4)
  W:A5.KQ973.J.97543 KT6.4.Q86432.QT6 832.AT852.T7.AJ8 QJ974.J6.AK95.K2
  ours NS @ A: 2♦ - 2NT - 3♥ X - - -  -> 3♥x by North
  ours EW @ B: - - 1♠ 2♠ X - - -  -> 2♠x by West

[vul both, seed Some(1783375070), board 466] swing -3250 pts / -22 IMPs (PD -22), diverged at call 0 (1♠ ours vs 4♦ BBA), wrong-strain
  rule: 12–21 points, and 5+ ♠
  W:5.KQ763.Q754.JT2 9832.9.83.AKQ986 6.AJ852.AKJT92.5 AKQJT74.T4.6.743
  ours NS @ A: 1♠ - 4♥ X - - -  -> 4♥x by North
  ours EW @ B: 4♦ - 4♠ - - -  -> 4♠ by North

### Constructive / book / round-2 (18363 boards, -39557 IMPs)

[vul both, seed Some(1783375068), board 274] swing -2830 pts / -21 IMPs (PD -21), diverged at call 4 (2♦ ours vs 3♦ BBA), missed-slam
  rule: 4+ ♦
  W:T765.J654.KT3.87 AJ4.AQ9.94.QJ642 KQ9832.7.Q86.T95 .KT832.AJ752.AK3
  ours NS @ A: 1♥ - 2♣ - 2♦ - 3♥ - 4NT - 5♠ X - - -  -> 5♠x by North
  ours EW @ B: 1♥ - 2♣ - 3♦ - 3♥ - 5♠ - 6♣ - 6♥ - - -  -> 6♥ by South

[vul both, seed Some(1783375071), board 2223] swing -2340 pts / -20 IMPs (PD -21), diverged at call 5 (2♦ ours vs 3♦ BBA), missed-grand
  rule: 4+ ♦
  W:98763.JT8753.7.7 JT4.AKQ92.KJT62. KQ52.64.543.KJ95 A..AQ98.AQT86432
  ours NS @ A: - 1♥ - 2♣ - 2♦ - 3♦ - 4♥ - - -  -> 4♥ by North
  ours EW @ B: - 1♥ - 2♣ - 3♦ - 4♣ - 4♦ - 4NT - 5♥ - 7♦ - - -  -> 7♦ by North

[vul none, seed Some(1783375068), board 274] swing -2080 pts / -19 IMPs (PD -19), diverged at call 4 (2♦ ours vs 3♦ BBA), missed-slam
  rule: 4+ ♦
  W:T765.J654.KT3.87 AJ4.AQ9.94.QJ642 KQ9832.7.Q86.T95 .KT832.AJ752.AK3
  ours NS @ A: 1♥ - 2♣ - 2♦ - 3♥ - 4NT - 5♠ X - - -  -> 5♠x by North
  ours EW @ B: 1♥ - 2♣ - 3♦ - 3♥ - 5♠ - 6♣ - 6♥ - - -  -> 6♥ by South

### Constructive / book / round-1 (12600 boards, -33912 IMPs)

[vul both, seed Some(1783375079), board 1671] swing -3610 pts / -23 IMPs (PD -23), diverged at call 3 (4♣ ours vs 3♦ BBA), missed-grand
  rule: 4+ card support for partner, 10–13 points, and ≤1 ♣
  W:T.Q983.74.KQJ953 AK9876.A7.A53.A4 42.KJT.T98.T8762 QJ53.6542.KQJ62.
  ours NS @ A: - 1♠ - 4♣ X - - -  -> 4♣x by South
  ours EW @ B: - 1♠ - 3♦ - 3♥ - 5♣ - 6♣ - 7♠ - - -  -> 7♠ by North

[vul both, seed Some(1783375065), board 2448] swing -3310 pts / -22 IMPs (PD -22), diverged at call 3 (4♣ ours vs 2NT BBA), other
  rule: 4+ card support for partner, 10–13 points, and ≤1 ♣
  W:KJ82.J2.KQJ875.7 Q.K983.943.KQT52 A9543.AT75.A2.A4 T76.Q64.T6.J9863
  ours NS @ A: - 1♠ - 2NT - 3♠ - 4♣ - 4♦ - 4NT - 5♦ - 5♥ - 5♠ - 6♣ - 6♦ - 7♠ - - -  -> 7♠ by East
  ours EW @ B: - 1♠ - 4♣ X - - -  -> 4♣x by West

[vul both, seed Some(1783375079), board 920] swing -3250 pts / -22 IMPs (PD -22), diverged at call 2 (4♦ ours vs 2NT BBA), missed-game
  rule: 4+ card support for partner, 10–13 points, and ≤1 ♦
  W:AQ95.43.AQ986.J4 K.AT9752.J4.AT85 T87632.6.K7532.Q J4.KQJ8.T.K97632
  ours NS @ A: 1♥ - 4♦ X - - -  -> 4♦x by South
  ours EW @ B: 1♥ - 2NT 3♦ 4♥ - - -  -> 4♥ by North

### Competitive / fallback@2 / round-1 (5448 boards, -18141 IMPs)

[vul both, seed Some(1783375079), board 5800] swing -2320 pts / -20 IMPs (PD -20), diverged at call 2 (3♥ ours vs X BBA), other
  rule: 3+ ♥, and 6–9 points
  W:T86532.T.643.A94 .AQJ9873.AK72.76 AKQ94.5.J9.QJT83 J7.K642.QT85.K52
  ours NS @ A: 1♥ 2♥ 3♥ 4♠ - - -  -> 4♠ by West
  ours EW @ B: 1♥ 2♥ X - - -  -> 2♥x by East

[vul both, seed Some(1783375067), board 5147] swing -2370 pts / -20 IMPs (PD -20), diverged at call 2 (1NT ours vs P BBA), overbid
  rule: 6–9 HCP
  W:AKJT632.T84.2.K2 Q7.KQJ7.AK84.QT5 8.A962.9653.AJ94 954.53.QJT7.8763
  ours NS @ A: 1♠ X - 2♣ 2♠ 3♣ X - - 4♣ X - - -  -> 4♣x by South
  ours EW @ B: 1♠ X 1NT - 2♠ - - -  -> 2♠ by West

[vul both, seed Some(1783375068), board 163] swing -2320 pts / -20 IMPs (PD -20), diverged at call 3 (3♦ ours vs X BBA), other
  rule: 3♦ is the cheapest bid, 5+ card support for partner, and 6–9 points
  W:T9532.A7632..JT5 AQJ.T.KT7632.KQ4 K8764.KQ985.A5.2 .J4.QJ984.A98763
  ours NS @ A: - 1♦ 2♦ 3♦ 4♠ - - -  -> 4♠ by West
  ours EW @ B: - 1♦ 2♦ X - - -  -> 2♦x by East

### Competitive / fallback@1 / round-1 (5707 boards, -17673 IMPs)

[vul both, seed Some(1783375079), board 667] swing -3220 pts / -22 IMPs (PD -22), diverged at call 2 (4♣ ours vs 4♥ BBA), other
  rule: 5+ ♣, (5+ ♥, or 5+ ♠), and 10+ points
  W:AQ95.J93.Q8.KQJ6 KJT864.A5.AT932. 3.KQT872.J.A9754 72.64.K7654.T832
  ours NS @ A: 1NT 2♦ 4♥ - - -  -> 4♥ by East
  ours EW @ B: 1NT 2♦ 4♣ - 4♦ X - - -  -> 4♦x by West

[vul none, seed Some(1783375068), board 4960] swing -2080 pts / -19 IMPs (PD -19), diverged at call 2 (P ours vs X BBA), other
  rule: 0+ HCP
  W:T75.98.65432.KT5 AJ4.2.T987.AQ863 Q92.AKQJ753.J.74 K863.T64.AKQ.J92
  ours NS @ A: 1♣ 1♥ - - -  -> 1♥ by East
  ours EW @ B: 1♣ 1♥ X - 1♠ 2♥ 2♠ - - 3♥ 3♠ - - X - 3NT - - X - - -  -> 3NTx by West

[vul both, seed Some(1783375074), board 650] swing -2010 pts / -19 IMPs (PD -19), diverged at call 2 (P ours vs 2♥ BBA), missed-slam
  rule: 0+ HCP
  W:AKQ964.98754..72 .AQ2.QT8532.KT53 J8732.T.K96.Q986 T5.KJ63.AJ74.AJ4
  ours NS @ A: 1♦ 2♦ - 4♠ - - -  -> 4♠ by East
  ours EW @ B: 1♦ 2♦ 2♥ - 3♥ - 4♠ - 5♦ - 6♦ - - -  -> 6♦ by South

### Defensive / floor#3 / round-2 (4135 boards, -13247 IMPs)

[vul both, seed Some(1783375078), board 2246] swing -2380 pts / -20 IMPs (PD -20), diverged at call 4 (P ours vs X BBA), other
  rule: not ((opaque condition)), or (opaque condition)
  W:A.AKT32.Q92.8632 T8753.Q4.A754.T7 J4.J9765.3.AKQJ5 KQ962.8.KJT86.94
  ours NS @ A: - 1♥ - 4♦ - 4♥ - - -  -> 4♥ by West
  ours EW @ B: - 1♥ - 4♦ X - - -  -> 4♦x by East

[vul both, seed Some(1783375065), board 3233] swing -2040 pts / -19 IMPs (PD -19), diverged at call 4 (P ours vs 2♥ BBA), other
  rule: not ((opaque condition)), or (opaque condition)
  W:8.AK86.KJ42.QT76 AT.QT4.AT93.KJ94 6542.J97532.65.2 KQJ973..Q87.A853
  ours NS @ A: - 1♠ X XX 2♥ 2♠ - 4♠ - - -  -> 4♠ by South
  ours EW @ B: - 1♠ X XX - - -  -> 1♠xx by South

[vul both, seed Some(1783375075), board 3397] swing -2150 pts / -19 IMPs (PD -19), diverged at call 5 (P ours vs 2♦ BBA), missed-slam
  rule: not ((opaque condition)), or (opaque condition)
  W:KJ8764.4.42.6543 9.AQJ98532.Q.AKJ AQT.6.KJT73.QT92 532.KT7.A9865.87
  ours NS @ A: 1♦ - 1♠ X XX - - -  -> 1♠xx by West
  ours EW @ B: 1♦ - 1♠ X XX 2♦ 2♠ 4♥ 4♠ 4NT - 5♠ - 6♥ - - -  -> 6♥ by North

### Defensive / floor#3 / round-1 (3419 boards, -11581 IMPs)

[vul both, seed Some(1783375076), board 2824] swing -2830 pts / -21 IMPs (PD -21), diverged at call 3 (P ours vs 4♠ BBA), missed-grand
  rule: not ((opaque condition)), or (opaque condition)
  W:QJ9843.985.J.J72 KT.KJ6432.32.A84 A7652..K965.QT63 .AQT7.AQT874.K95
  ours NS @ A: 1♥ 1♠ 2♠ 4♠ - - -  -> 4♠ by East
  ours EW @ B: 1♥ 1♠ 2♠ - 4♥ - 4♠ - 5♦ - 7♥ - - -  -> 7♥ by North

[vul both, seed Some(1783375066), board 655] swing -2060 pts / -19 IMPs (PD -19), diverged at call 3 (P ours vs 2♣ BBA), other
  rule: not ((opaque condition)), or (opaque condition)
  W:65.A3.AQJT94.Q85 KQ87.QJ96.K3.K62 AJ3.K8752.8752.A T942.T4.6.JT9743
  ours NS @ A: 1♦ X XX - - -  -> 1♦xx by West
  ours EW @ B: 1♦ X XX 2♣ 2♦ - 3♦ - 4♦ - - -  -> 4♦ by West

[vul none, seed Some(1783375076), board 2824] swing -1930 pts / -18 IMPs (PD -18), diverged at call 3 (P ours vs 4♠ BBA), missed-grand
  rule: not ((opaque condition)), or (opaque condition)
  W:QJ9843.985.J.J72 KT.KJ6432.32.A84 A7652..K965.QT63 .AQT7.AQT874.K95
  ours NS @ A: 1♥ 1♠ 2♠ 4♠ - - -  -> 4♠ by East
  ours EW @ B: 1♥ 1♠ 2♠ - 4♥ - 4♠ - 5♦ - 7♥ - - -  -> 7♥ by North

### Competitive / fallback@3 / round-2 (3284 boards, -9394 IMPs)

[vul both, seed Some(1783375064), board 4909] swing -2000 pts / -19 IMPs (PD -19), diverged at call 4 (X ours vs 2♦ BBA), other
  rule: 4+ ♠, and 8+ HCP
  W:6.AT762.AJT3.QT5 KQJT32.K93.4.AJ6 A754.5.KQ9876.83 98.QJ84.52.K9742
  ours NS @ A: - - 1♥ 1♠ 2♦ - - 2♠ - - 3♦ 3♠ 5♦ - - -  -> 5♦ by East
  ours EW @ B: - - 1♥ 1♠ X - 4♥ - - X - - -  -> 4♥x by West

[vul both, seed Some(1783375069), board 5374] swing -2110 pts / -19 IMPs (PD -19), diverged at call 4 (P ours vs 2♦ BBA), missed-grand
  rule: 0+ HCP
  W:KJT32.J.53.98543 AQ9765.QT6.K7.72 4.972.982.AKQJT6 8.AK8543.AQJT64.
  ours NS @ A: 1♥ - 1♠ 2♣ - 4♣ - - -  -> 4♣ by East
  ours EW @ B: 1♥ - 1♠ 2♣ 2♦ 4♣ 4♥ - 5♣ - 5♥ - 5♠ - 6♦ - 7♥ - - -  -> 7♥ by South

[vul both, seed Some(1783375070), board 5280] swing -2020 pts / -19 IMPs (PD -19), diverged at call 4 (P ours vs X BBA), other
  rule: 0+ HCP
  W:QT8765.KQJ83.8.K 42.75.J963.AQJ93 J.AT4.AT52.86542 AK93.962.KQ74.T7
  ours NS @ A: - - 1♦ 2♦ - 2♥ - 2♠ - 4♥ - - -  -> 4♥ by East
  ours EW @ B: - - 1♦ 2♦ X - - -  -> 2♦x by West

### Defensive / book / round-2 (2650 boards, -6710 IMPs)

[vul both, seed Some(1783375064), board 3650] swing -2080 pts / -19 IMPs (PD -19), diverged at call 4 (2♥ ours vs P BBA), other
  rule: 5+ ♥, and 8–16 points
  W:A862.J632.A986.5 3.A98.T73.KQT873 QJT975.5.KQJ.A96 K4.KQT74.542.J42
  ours NS @ A: - - - 1♠ 2♥ 2NT - 4♥ - 4♠ - - -  -> 4♠ by East
  ours EW @ B: - - - 1♠ - 4♣ X - - -  -> 4♣x by West

[vul both, seed Some(1783375074), board 4640] swing -2000 pts / -19 IMPs (PD -22), diverged at call 4 (2♦ ours vs 1♠ BBA), overbid
  rule: 5+ ♥, 5+ ♠, and 8+ points
  W:KJ.Q.K8763.AKT62 AQ964.97652.Q.95 85.K843.AJT.QJ83 T732.AJT.9542.74
  ours NS @ A: - - - 1♦ 2♦ X - - -  -> 2♦x by North
  ours EW @ B: - - - 1♦ 1♠ - - X - 4♥ - - -  -> 4♥ by East

[vul both, seed Some(1783375067), board 4920] swing -2120 pts / -19 IMPs (PD -20), diverged at call 4 (1♥ ours vs 1♠ BBA), wrong-strain
  rule: 4+ ♥
  W:J.T42.T9753.Q652 T9842.8763.Q62.J A63.AK95.K.T9874 KQ75.QJ.AJ84.AK3
  ours NS @ A: - 1♣ X - 1♥ - - -  -> 1♥ by North
  ours EW @ B: - 1♣ X - 1♠ X XX - - -  -> 1♠xx by North

