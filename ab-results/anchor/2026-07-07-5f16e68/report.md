=== arm 0: our american floor (us) vs BBA 2/1 (them), vulnerability none, 204800 boards ===
replay verification: 100.00% of 2158694 our-side calls (0 mismatched)
auction-divergent: 188300 (92%), contract-divergent: 157645 (77%)
plain DD: -1.6724 IMPs/board (95% CI [-1.6950, -1.6497]), -342503 IMPs total
perfect defense: -1.9843 IMPs/board (95% CI [-2.0114, -1.9571])
gen_args: --count 6400 --seed 1783375064 --output ab-results/anchor/2026-07-07-5f16e68/none/shard-0.json -v none

=== arm 1: our american floor (us) vs BBA 2/1 (them), vulnerability both, 204800 boards ===
replay verification: 100.00% of 2136554 our-side calls (0 mismatched)
auction-divergent: 187580 (92%), contract-divergent: 156508 (76%)
plain DD: -2.3161 IMPs/board (95% CI [-2.3450, -2.2871]), -474328 IMPs total
perfect defense: -2.7444 IMPs/board (95% CI [-2.7783, -2.7105])
gen_args: --count 6400 --seed 1783375064 --output ab-results/anchor/2026-07-07-5f16e68/both/shard-0.json -v both


## IMP histogram (plain, per contract-divergent board)

right-siding-only divergences (same contract, different auction): 61727

  -24 IMPs: 1
  -22 IMPs: 4
  -21 IMPs: 30
  -20 IMPs: 50
  -19 IMPs: 211
  -18 IMPs: 430
  -17 IMPs: 1268
  -16 IMPs: 1448
  -15 IMPs: 3253
  -14 IMPs: 4648
  -13 IMPs: 9737
  -12 IMPs: 9016
  -11 IMPs: 14588
  -10 IMPs: 20417
   -9 IMPs: 8592
   -8 IMPs: 7389
   -7 IMPs: 13669
   -6 IMPs: 20259
   -5 IMPs: 15998
   -4 IMPs: 8425
   -3 IMPs: 11844
   -2 IMPs: 12990
   -1 IMPs: 11255
   +0 IMPs: 49368
   +1 IMPs: 10474
   +2 IMPs: 9886
   +3 IMPs: 8006
   +4 IMPs: 5768
   +5 IMPs: 15364
   +6 IMPs: 13069
   +7 IMPs: 6212
   +8 IMPs: 2451
   +9 IMPs: 2490
  +10 IMPs: 5089
  +11 IMPs: 4217
  +12 IMPs: 2687
  +13 IMPs: 2863
  +14 IMPs: 481
  +15 IMPs: 104
  +16 IMPs: 50
  +17 IMPs: 48
  +18 IMPs: 2
  +19 IMPs: 2

## Ranked buckets (phase / provenance / family), losses first

| bucket | boards | net plain IMPs | IMPs/divergent ±CI | net PD IMPs | flag |
| --- | --- | --- | --- | --- | --- |
| Defensive / book / round-1 | 59437 | -142733 | -2.40 ±0.06 | -188939 | |
| Constructive / book / opening | 47692 | -103480 | -2.17 ±0.06 | -106037 | |
| Constructive / book / round-2 | 43212 | -98201 | -2.27 ±0.06 | -98215 | |
| Constructive / book / round-1 | 29727 | -76291 | -2.57 ±0.08 | -86039 | |
| Competitive / fallback@1 / round-1 | 13846 | -44169 | -3.19 ±0.11 | -47594 | |
| Competitive / fallback@2 / round-1 | 12606 | -42221 | -3.35 ±0.11 | -48671 | |
| Defensive / floor#3 / round-2 | 9900 | -31665 | -3.20 ±0.13 | -34371 | |
| Defensive / floor#3 / round-1 | 8597 | -29193 | -3.40 ±0.15 | -26309 | |
| Competitive / fallback@3 / round-2 | 7354 | -21563 | -2.93 ±0.15 | -26841 | |
| Defensive / floor#242 / round-1 | 3162 | -11623 | -3.68 ±0.26 | -14522 | |
| Competitive / floor#242 / round-2 | 3094 | -11431 | -3.69 ±0.24 | -18818 | |
| Defensive / book / round-2 | 4304 | -11190 | -2.60 ±0.21 | -13874 | |
| Competitive / fallback@4 / round-2 | 3299 | -10909 | -3.31 ±0.22 | -14365 | |
| Competitive / fallback@3 / round-1 | 2231 | -7904 | -3.54 ±0.29 | -10497 | |
| Competitive / floor#3 / round-2 | 2468 | -7794 | -3.16 ±0.27 | -8363 | |
| Defensive / floor#242 / round-2 | 1444 | -6182 | -4.28 ±0.34 | -10639 | |
| Constructive / floor#3 / round-2 | 2675 | -6162 | -2.30 ±0.23 | -5420 | |
| Competitive / floor#242 / balancing | 2212 | -5514 | -2.49 ±0.27 | -11573 | |
| Defensive / floor#242 / balancing | 2303 | -5226 | -2.27 ±0.27 | -10942 | |
| Competitive / floor#242+rb / round-2 | 751 | -5123 | -6.82 ±0.47 | -6531 | |
| Competitive / floor#3 / round-1 | 1054 | -5069 | -4.81 ±0.43 | -2717 | |
| Constructive / floor#140 / round-2 | 1646 | -4896 | -2.97 ±0.38 | -5062 | |
| Defensive / floor#3 / balancing | 1685 | -4857 | -2.88 ±0.27 | -3746 | |
| Constructive / floor#3 / round-1 | 1267 | -4651 | -3.67 ±0.39 | -2433 | |
| Constructive / book / deep | 4745 | -4534 | -0.96 ±0.18 | -4965 | |
| Defensive / floor#20 / round-1 | 1483 | -4438 | -2.99 ±0.33 | -5725 | |
| Defensive / floor#50 / round-1 | 1855 | -4416 | -2.38 ±0.29 | -5901 | |
| Defensive / floor#60 / round-2 | 1332 | -3964 | -2.98 ±0.32 | -6567 | |
| Defensive / floor#35 / round-1 | 1507 | -3771 | -2.50 ±0.32 | -5079 | |
| Competitive / floor#46 / round-2 | 822 | -2902 | -3.53 ±0.42 | -4100 | |
| Defensive / floor#64 / round-1 | 1174 | -2862 | -2.44 ±0.40 | -3090 | |
| Constructive / floor#61 / deep | 1227 | -2537 | -2.07 ±0.17 | -4028 | |
| Defensive / floor#35 / round-2 | 1102 | -2249 | -2.04 ±0.36 | -3314 | |
| Defensive / floor#20 / round-2 | 1204 | -2175 | -1.81 ±0.33 | -3156 | |
| Defensive / floor#202 / round-2 | 429 | -2141 | -4.99 ±0.58 | -3294 | |
| Constructive / floor#3 / deep | 1254 | -2089 | -1.67 ±0.38 | -1749 | |
| Defensive / floor#202 / round-1 | 502 | -1978 | -3.94 ±0.58 | -2608 | |
| Defensive / floor#45 / round-2 | 535 | -1968 | -3.68 ±0.53 | -3173 | |
| Competitive / floor#30 / round-2 | 543 | -1948 | -3.59 ±0.47 | -2998 | |
| Competitive / floor#46 / round-1 | 303 | -1785 | -5.89 ±0.82 | -1771 | |
| Competitive / floor#31 / round-1 | 364 | -1779 | -4.89 ±0.72 | -1492 | |
| Competitive / floor#61 / round-1 | 253 | -1753 | -6.93 ±0.80 | -1705 | |
| Defensive / floor#132 / round-1 | 455 | -1662 | -3.65 ±0.52 | -3226 | |
| Competitive / floor#3 / balancing | 728 | -1656 | -2.27 ±0.40 | -218 | |
| Defensive / floor#20 / balancing | 757 | -1481 | -1.96 ±0.41 | -2770 | |
| Defensive / floor#65 / round-1 | 940 | -1471 | -1.56 ±0.40 | -2464 | |
| Defensive / floor#200 / round-1 | 300 | -1385 | -4.62 ±0.79 | -1771 | |
| Defensive / floor#61 / round-2 | 405 | -1347 | -3.33 ±0.60 | -1773 | |
| Defensive / floor#60 / round-1 | 321 | -1311 | -4.08 ±0.63 | -2032 | |
| Defensive / floor#131 / balancing | 409 | -1304 | -3.19 ±0.58 | -2084 | |
| Constructive / fallback@4 / deep | 423 | -1246 | -2.95 ±0.66 | -1394 | |
| Defensive / floor#243 / round-1 | 262 | -1221 | -4.66 ±0.91 | -1689 | |
| Constructive / fallback@5 / deep | 376 | -1172 | -3.12 ±0.75 | -1269 | |
| Competitive / book+rb / round-2 | 546 | -1144 | -2.10 ±0.45 | -1704 | |
| Constructive / floor#61 / round-2 | 419 | -1111 | -2.65 ±0.62 | -1268 | |
| Defensive / floor#46 / round-2 | 332 | -1076 | -3.24 ±0.61 | -1648 | |
| Competitive / floor#242 / round-1 | 460 | -1042 | -2.27 ±0.55 | -2370 | |
| Competitive / fallback@2 / round-2 | 331 | -1006 | -3.04 ±0.82 | -914 | |
| Defensive / floor#200 / round-2 | 275 | -994 | -3.61 ±0.82 | -1564 | |
| Constructive / floor#46 / round-2 | 522 | -969 | -1.86 ±0.56 | -1075 | |
| Competitive / floor#16 / round-1 | 136 | -967 | -7.11 ±1.11 | -1006 | |
| Constructive / floor#46 / deep | 609 | -941 | -1.55 ±0.38 | -1753 | |
| Defensive / floor#30 / round-2 | 319 | -897 | -2.81 ±0.65 | -1543 | |
| Competitive / floor#5 / round-2 | 235 | -861 | -3.66 ±0.97 | -1248 | |
| Defensive / floor#197 / round-1 | 210 | -859 | -4.09 ±0.87 | -1012 | |
| Defensive / floor#35 / balancing | 512 | -857 | -1.67 ±0.46 | -1777 | |
| Defensive / floor#45 / round-1 | 244 | -853 | -3.50 ±0.86 | -1216 | |
| Competitive / floor#243 / balancing | 198 | -819 | -4.14 ±0.76 | -1435 | |
| Constructive / floor#32 / round-1 | 181 | -754 | -4.17 ±0.91 | -695 | |
| Defensive / floor#132 / balancing | 295 | -754 | -2.56 ±0.53 | -1822 | |
| Competitive / floor#243 / round-2 | 175 | -734 | -4.19 ±0.85 | -1266 | |
| Defensive / floor#16 / round-2 | 190 | -713 | -3.75 ±0.86 | -934 | |
| Defensive / floor#64 / round-2 | 328 | -689 | -2.10 ±0.69 | -1072 | |
| Defensive / floor#199 / round-1 | 270 | -688 | -2.55 ±0.78 | -896 | |
| Defensive / floor#243 / balancing | 230 | -686 | -2.98 ±0.86 | -1184 | |
| Competitive / fallback@1 / round-2 | 293 | -655 | -2.24 ±0.88 | -529 | |
| Defensive / floor#243 / round-2 | 168 | -654 | -3.89 ±0.95 | -1213 | |
| Competitive / floor#240 / balancing | 185 | -634 | -3.43 ±1.01 | -708 | |
| Defensive / floor#51 / round-1 | 244 | -599 | -2.45 ±0.89 | -581 | |
| Defensive / floor#3 / deep | 208 | -598 | -2.88 ±0.86 | -596 | |
| Competitive / floor#6 / round-2 | 175 | -596 | -3.41 ±1.02 | -1131 | |
| Constructive / floor#140 / deep | 297 | -593 | -2.00 ±0.83 | -604 | |
| Defensive / floor#50 / round-2 | 661 | -587 | -0.89 ±0.42 | -1148 | |
| Constructive / floor#145 / round-2 | 86 | -571 | -6.64 ±1.82 | -571 | |
| Defensive / floor#21 / round-1 | 184 | -558 | -3.03 ±0.99 | -961 | |
| Defensive / floor#66 / round-1 | 179 | -533 | -2.98 ±1.04 | -662 | |
| Competitive / floor#240 / round-2 | 183 | -529 | -2.89 ±1.02 | -556 | |
| Constructive / floor#17 / round-1 | 133 | -509 | -3.83 ±1.13 | -403 | |
| Competitive / floor#61 / round-2 | 104 | -506 | -4.87 ±1.16 | -646 | |
| Defensive / floor#50 / balancing | 403 | -481 | -1.19 ±0.48 | -1431 | |
| Defensive / floor#198 / round-2 | 118 | -477 | -4.04 ±1.16 | -726 | |
| Defensive / floor#131 / round-1 | 142 | -475 | -3.35 ±1.08 | -811 | |
| Defensive / floor#198 / round-1 | 158 | -471 | -2.98 ±1.12 | -734 | |
| Competitive / floor#239 / round-2 | 112 | -460 | -4.11 ±1.13 | -642 | |
| Defensive / floor#49 / round-1 | 192 | -440 | -2.29 ±0.92 | -336 | |
| Competitive / floor#237 / round-2 | 144 | -406 | -2.82 ±0.90 | -708 | |
| Defensive / floor#30 / round-1 | 121 | -403 | -3.33 ±1.08 | -482 | |
| Competitive / floor#241 / round-2 | 90 | -398 | -4.42 ±1.25 | -511 | |
| Competitive / floor#242 / deep | 60 | -395 | -6.58 ±1.58 | -610 | |
| Competitive / floor#16 / round-2 | 92 | -392 | -4.26 ±1.48 | -436 | |
| Competitive / floor#30 / balancing | 126 | -384 | -3.05 ±0.88 | -727 | |
| Defensive / floor#36 / round-1 | 144 | -383 | -2.66 ±1.01 | -743 | |
| Defensive / floor#65 / balancing | 241 | -362 | -1.50 ±0.60 | -1101 | |
| Constructive / floor#16 / round-2 | 72 | -359 | -4.99 ±1.45 | -335 | |
| Competitive / floor#3 / deep | 187 | -356 | -1.90 ±0.92 | -442 | |
| Defensive / floor#61 / round-1 | 92 | -337 | -3.66 ±1.61 | -356 | |
| Constructive / floor#31 / round-2 | 80 | -336 | -4.20 ±1.49 | -289 | |
| Constructive / floor#151 / round-2 | 51 | -334 | -6.55 ±2.35 | -329 | |
| Defensive / floor#31 / round-2 | 145 | -334 | -2.30 ±1.10 | -431 | |
| Defensive / floor#49 / balancing | 184 | -332 | -1.80 ±0.93 | -512 | |
| Competitive / floor#15 / balancing | 79 | -322 | -4.08 ±1.13 | -519 | |
| Competitive / floor#238 / balancing | 156 | -320 | -2.05 ±1.10 | -381 | |
| Competitive / floor#1 / round-2 | 383 | -313 | -0.82 ±0.77 | -519 | |
| Competitive / floor#236 / balancing | 153 | -301 | -1.97 ±0.92 | -417 | |
| Competitive / floor#46 / deep | 102 | -291 | -2.85 ±1.08 | -541 | |
| Defensive / floor#133 / round-1 | 315 | -290 | -0.92 ±0.92 | -718 | ~noise |
| Defensive / floor#17 / round-1 | 69 | -288 | -4.17 ±1.58 | -241 | |
| Competitive / floor#31 / round-2 | 73 | -286 | -3.92 ±1.46 | -317 | |
| Competitive / floor#10 / round-2 | 72 | -282 | -3.92 ±1.36 | -361 | |
| Competitive / floor#15 / round-2 | 58 | -279 | -4.81 ±1.33 | -439 | |
| Defensive / floor#32 / round-1 | 45 | -277 | -6.16 ±2.16 | -226 | |
| Defensive / floor#205 / round-1 | 97 | -268 | -2.76 ±1.32 | -263 | |
| Defensive / floor#129 / round-2 | 197 | -266 | -1.35 ±1.10 | -550 | |
| Competitive / floor#3+rb / round-2 | 153 | -265 | -1.73 ±1.14 | -334 | |
| Competitive / floor#235 / round-2 | 46 | -251 | -5.46 ±1.33 | -355 | |
| Competitive / floor#238 / round-2 | 72 | -246 | -3.42 ±1.75 | -284 | |
| Competitive / fallback@5 / round-2 | 103 | -244 | -2.37 ±1.11 | -197 | |
| Competitive / floor#241 / balancing | 92 | -239 | -2.60 ±1.26 | -389 | |
| Competitive / floor#234 / balancing | 91 | -236 | -2.59 ±1.30 | -301 | |
| Constructive / floor#140 / round-1 | 18 | -229 | -12.72 ±0.82 | -229 | |
| Competitive / floor#31 / balancing | 30 | -222 | -7.40 ±1.81 | -247 | |
| Defensive / floor#203 / round-1 | 65 | -206 | -3.17 ±1.39 | -251 | |
| Competitive / floor#240+rb / round-2 | 59 | -199 | -3.37 ±1.84 | -210 | |
| Defensive / floor#27 / round-2 | 42 | -196 | -4.67 ±1.54 | -233 | |
| Competitive / floor#25 / round-2 | 100 | -193 | -1.93 ±1.13 | -210 | |
| Competitive / floor#61 / deep | 86 | -189 | -2.20 ±1.21 | -456 | |
| Defensive / floor#204 / round-1 | 80 | -186 | -2.33 ±1.59 | -159 | |
| Defensive / floor#64 / balancing | 225 | -178 | -0.79 ±0.80 | -429 | ~noise |
| Competitive / floor#234+rb / round-2 | 19 | -177 | -9.32 ±2.14 | -212 | |
| Defensive / floor#237 / round-2 | 52 | -173 | -3.33 ±1.67 | -265 | |
| Defensive / floor#17 / round-2 | 61 | -169 | -2.77 ±1.48 | -241 | |
| Defensive / floor#197 / round-2 | 88 | -168 | -1.91 ±1.56 | -275 | |
| Competitive / floor#238+rb / round-2 | 53 | -167 | -3.15 ±1.65 | -216 | |
| Defensive / floor#242 / deep | 33 | -166 | -5.03 ±1.91 | -309 | |
| Defensive / floor#1 / round-2 | 235 | -165 | -0.70 ±1.07 | -323 | ~noise |
| Competitive / floor#16 / deep | 38 | -162 | -4.26 ±1.39 | -207 | |
| Defensive / floor#63 / round-2 | 35 | -160 | -4.57 ±2.24 | -144 | |
| Competitive / floor#236 / round-2 | 33 | -158 | -4.79 ±2.16 | -160 | |
| Defensive / floor#34 / balancing | 147 | -158 | -1.07 ±1.03 | -240 | |
| Competitive / floor#32 / round-1 | 24 | -155 | -6.46 ±2.64 | -150 | |
| Competitive / floor#140 / round-1 | 42 | -152 | -3.62 ±2.92 | -149 | |
| Defensive / floor#5 / round-2 | 65 | -150 | -2.31 ±2.06 | -200 | |
| Defensive / floor#6 / round-2 | 40 | -150 | -3.75 ±2.25 | -207 | |
| Competitive / floor#236+rb / round-2 | 37 | -149 | -4.03 ±2.54 | -163 | |
| Competitive / floor#9 / round-2 | 48 | -149 | -3.10 ±1.33 | -224 | |
| Competitive / floor#55 / round-2 | 31 | -148 | -4.77 ±2.20 | -155 | |
| Defensive / floor#65 / round-2 | 282 | -148 | -0.52 ±0.65 | -425 | ~noise |
| Competitive / floor#16 / balancing | 48 | -147 | -3.06 ±1.73 | -263 | |
| Defensive / floor#12 / round-2 | 46 | -144 | -3.13 ±1.66 | -184 | |
| Competitive / floor#46+rb / deep | 44 | -143 | -3.25 ±0.89 | -283 | |
| Constructive / floor#147 / round-2 | 37 | -140 | -3.78 ±3.22 | -140 | |
| Competitive / floor#2 / round-2 | 128 | -135 | -1.05 ±1.30 | -399 | ~noise |
| Competitive / floor#30 / deep | 43 | -134 | -3.12 ±1.57 | -234 | |
| Defensive / floor#239 / round-2 | 46 | -132 | -2.87 ±2.28 | -243 | |
| Defensive / floor#1 / deep | 43 | -125 | -2.91 ±2.13 | -247 | |
| Defensive / floor#48 / round-2 | 26 | -125 | -4.81 ±2.22 | -170 | |
| Competitive / floor#12 / round-2 | 33 | -124 | -3.76 ±1.97 | -196 | |
| Defensive / floor#235 / round-2 | 42 | -121 | -2.88 ±1.58 | -188 | |
| Competitive / floor#17 / deep | 26 | -120 | -4.62 ±2.25 | -153 | |
| Constructive / floor#47 / round-2 | 12 | -120 | -10.00 ±2.70 | -120 | |
| Defensive / floor#51 / balancing | 50 | -116 | -2.32 ±1.36 | -148 | |
| Competitive / floor#39 / round-2 | 20 | -115 | -5.75 ±1.63 | -163 | |
| Defensive / floor#11 / round-2 | 24 | -114 | -4.75 ±1.30 | -192 | |
| Competitive / floor#243+rb / round-2 | 21 | -106 | -5.05 ±2.52 | -205 | |
| Defensive / floor#204 / round-2 | 38 | -106 | -2.79 ±1.88 | -138 | |
| Competitive / floor#234 / round-2 | 50 | -105 | -2.10 ±1.96 | -173 | |
| Competitive / floor#42 / round-2 | 22 | -105 | -4.77 ±3.80 | -144 | |
| Competitive / floor#63 / round-2 | 21 | -104 | -4.95 ±2.32 | -177 | |
| Defensive / floor#153 / round-1 | 18 | -104 | -5.78 ±4.67 | -75 | |
| Competitive / floor#140 / round-2 | 34 | -103 | -3.03 ±2.56 | -107 | |
| Competitive / floor#47 / round-2 | 89 | -101 | -1.13 ±1.62 | -377 | ~noise |
| Constructive / floor#30 / round-1 | 46 | -101 | -2.20 ±1.85 | -144 | |
| Defensive / floor#26 / round-2 | 14 | -101 | -7.21 ±2.85 | -142 | |
| Defensive / floor#21 / balancing | 59 | -100 | -1.69 ±1.34 | -198 | |
| Defensive / floor#42 / round-2 | 43 | -98 | -2.28 ±2.97 | -80 | ~noise |
| Competitive / floor#11 / round-2 | 10 | -96 | -9.60 ±2.52 | -143 | |
| Competitive / floor#57 / round-2 | 13 | -96 | -7.38 ±3.90 | -97 | |
| Competitive / floor#24 / round-2 | 16 | -95 | -5.94 ±3.01 | -124 | |
| Defensive / floor#147 / round-1 | 12 | -94 | -7.83 ±4.79 | -86 | |
| Defensive / floor#20 / deep | 23 | -93 | -4.04 ±1.39 | -161 | |
| Constructive / floor#62 / round-1 | 36 | -91 | -2.53 ±2.50 | -99 | |
| Defensive / floor#133 / balancing | 83 | -90 | -1.08 ±1.67 | -250 | ~noise |
| Defensive / floor#33 / round-2 | 22 | -90 | -4.09 ±2.75 | -54 | |
| Competitive / floor#129 / deep | 10 | -87 | -8.70 ±3.34 | -100 | |
| Defensive / floor#32 / round-2 | 52 | -83 | -1.60 ±1.61 | -144 | ~noise |
| Constructive / floor#31 / deep | 20 | -82 | -4.10 ±2.40 | -81 | |
| Defensive / floor#66 / balancing | 21 | -82 | -3.90 ±2.28 | -100 | |
| Competitive / floor#242+rb / balancing | 17 | -81 | -4.76 ±3.73 | -124 | |
| Competitive / floor#235 / deep | 12 | -80 | -6.67 ±2.88 | -114 | |
| Competitive / floor#237 / balancing | 46 | -79 | -1.72 ±1.41 | -218 | |
| Competitive / floor#241 / deep | 21 | -79 | -3.76 ±2.92 | -113 | |
| Defensive / floor#21 / round-2 | 53 | -79 | -1.49 ±1.74 | -242 | ~noise |
| Defensive / floor#238 / round-2 | 11 | -78 | -7.09 ±4.06 | -112 | |
| Competitive / floor#33 / round-1 | 12 | -76 | -6.33 ±3.23 | -108 | |
| Competitive / floor#33 / round-2 | 24 | -76 | -3.17 ±1.99 | -126 | |
| Competitive / floor#17 / round-1 | 13 | -72 | -5.54 ±4.07 | -45 | |
| Competitive / floor#239 / balancing | 54 | -72 | -1.33 ±1.32 | -180 | |
| Constructive / floor#62 / round-2 | 8 | -72 | -9.00 ±3.90 | -72 | |
| Defensive / floor#129 / round-1 | 71 | -71 | -1.00 ±1.64 | -180 | ~noise |
| Defensive / floor#36 / balancing | 38 | -71 | -1.87 ±1.97 | -141 | ~noise |
| Defensive / floor#49 / round-2 | 74 | -71 | -0.96 ±1.39 | -114 | ~noise |
| Defensive / floor#240 / round-2 | 22 | -70 | -3.18 ±2.11 | -81 | |
| Competitive / fallback@6 / round-2 | 18 | -69 | -3.83 ±3.54 | -44 | |
| Constructive / floor#17 / deep | 74 | -69 | -0.93 ±0.55 | -76 | |
| Defensive / floor#33 / round-1 | 12 | -67 | -5.58 ±4.15 | -73 | |
| Constructive / floor#32 / deep | 59 | -65 | -1.10 ±0.60 | -76 | |
| Competitive / book+rb / deep | 72 | -64 | -0.89 ±0.82 | -101 | |
| Defensive / floor#239 / balancing | 26 | -64 | -2.46 ±2.48 | -78 | ~noise |
| Defensive / floor#36 / round-2 | 25 | -62 | -2.48 ±2.20 | -96 | |
| Competitive / floor#242+rb / deep | 16 | -61 | -3.81 ±4.17 | -126 | ~noise |
| Competitive / floor#40 / round-2 | 31 | -60 | -1.94 ±2.56 | -102 | ~noise |
| Defensive / floor#46 / round-1 | 24 | -60 | -2.50 ±2.97 | -64 | ~noise |
| Competitive / floor#31 / deep | 41 | -56 | -1.37 ±1.42 | -115 | ~noise |
| Constructive / floor#153 / round-2 | 19 | -56 | -2.95 ±4.44 | -46 | ~noise |
| Defensive / floor#41 / round-2 | 48 | -53 | -1.10 ±2.44 | -83 | ~noise |
| Competitive / floor#237 / deep | 15 | -50 | -3.33 ±2.20 | -114 | |
| Defensive / floor#54 / deep | 11 | -50 | -4.55 ±3.18 | -79 | |
| Defensive / floor#55 / round-2 | 17 | -50 | -2.94 ±2.97 | -49 | ~noise |
| Competitive / floor#47 / balancing | 44 | -49 | -1.11 ±2.05 | -85 | ~noise |
| Competitive / floor#54 / round-2 | 16 | -49 | -3.06 ±3.83 | -42 | ~noise |
| Competitive / floor#27 / round-2 | 11 | -48 | -4.36 ±4.91 | -64 | ~noise |
| Defensive / floor#203 / round-2 | 42 | -47 | -1.12 ±1.95 | -70 | ~noise |
| Defensive / floor#16 / round-1 | 9 | -46 | -5.11 ±4.33 | -48 | |
| Competitive / floor#153 / round-1 | 3 | -45 | -15.00 ±2.99 | -45 | |
| Constructive / floor#157 / round-1 | 4 | -45 | -11.25 ±1.67 | -45 | |
| Constructive / floor#47 / round-1 | 34 | -45 | -1.32 ±2.89 | -43 | ~noise |
| Defensive / floor#10 / round-2 | 4 | -45 | -11.25 ±5.21 | -57 | |
| Competitive / floor#48 / round-2 | 29 | -44 | -1.52 ±2.78 | -115 | ~noise |
| Defensive / floor#151 / round-1 | 10 | -44 | -4.40 ±6.57 | -47 | ~noise |
| Defensive / floor#29 / round-1 | 9 | -44 | -4.89 ±4.96 | -37 | ~noise |
| Defensive / floor#40 / round-2 | 11 | -43 | -3.91 ±4.08 | -68 | ~noise |
| Competitive / floor#129 / round-2 | 26 | -42 | -1.62 ±3.23 | -109 | ~noise |
| Competitive / floor#46+rb / round-2 | 10 | -42 | -4.20 ±4.44 | -43 | ~noise |
| Defensive / floor#18 / round-2 | 19 | -41 | -2.16 ±3.63 | -23 | ~noise |
| Defensive / floor#47 / round-1 | 7 | -41 | -5.86 ±6.92 | -37 | ~noise |
| Competitive / floor#60 / balancing | 28 | -39 | -1.39 ±2.77 | -73 | ~noise |
| Competitive / floor#60 / round-2 | 6 | -39 | -6.50 ±2.98 | -51 | |
| Competitive / floor#60+rb / round-2 | 8 | -39 | -4.88 ±3.46 | -53 | |
| Competitive / floor#153 / round-2 | 5 | -38 | -7.60 ±8.67 | -37 | ~noise |
| Competitive / floor#18 / round-2 | 18 | -38 | -2.11 ±2.80 | -15 | ~noise |
| Competitive / floor#147 / round-1 | 7 | -36 | -5.14 ±5.19 | -31 | ~noise |
| Competitive / floor#5 / deep | 13 | -36 | -2.77 ±3.00 | -65 | ~noise |
| Defensive / floor#5 / round-1 | 34 | -36 | -1.06 ±2.61 | -15 | ~noise |
| Competitive / floor#45 / balancing | 11 | -35 | -3.18 ±6.04 | -15 | ~noise |
| Competitive / floor#61+rb / deep | 13 | -35 | -2.69 ±1.89 | -49 | |
| Defensive / floor#236 / round-2 | 4 | -35 | -8.75 ±2.02 | -35 | |
| Defensive / floor#29 / round-2 | 7 | -35 | -5.00 ±5.46 | -33 | ~noise |
| Defensive / floor#48 / round-1 | 13 | -34 | -2.62 ±5.09 | -50 | ~noise |
| Competitive / floor#47+rb / balancing | 4 | -33 | -8.25 ±4.48 | -33 | |
| Constructive / floor#147 / deep | 24 | -33 | -1.38 ±4.11 | -20 | ~noise |
| Defensive / floor#31 / round-1 | 13 | -33 | -2.54 ±3.50 | -47 | ~noise |
| Competitive / floor#3+rb / deep | 19 | -32 | -1.68 ±2.67 | -76 | ~noise |
| Competitive / floor#6 / deep | 2 | -32 | -16.00 ±1.96 | -32 | |
| Constructive / floor#63 / round-2 | 10 | -32 | -3.20 ±3.74 | -60 | ~noise |
| Defensive / floor#241 / deep | 8 | -31 | -3.88 ±5.22 | -38 | ~noise |
| Defensive / floor#241 / round-2 | 6 | -31 | -5.17 ±5.02 | -25 | |
| Competitive / floor#241+rb / deep | 5 | -30 | -6.00 ±6.59 | -31 | ~noise |
| Competitive / floor#243 / round-1 | 9 | -30 | -3.33 ±2.69 | -40 | |
| Competitive / floor#46 / balancing | 11 | -30 | -2.73 ±1.96 | -79 | |
| Constructive / floor#16 / deep | 4 | -30 | -7.50 ±2.47 | -38 | |
| Defensive / floor#47 / round-2 | 128 | -30 | -0.23 ±1.29 | -272 | ~noise |
| Defensive / floor#54 / round-2 | 7 | -30 | -4.29 ±1.53 | -71 | |
| Competitive / floor#39 / deep | 13 | -29 | -2.23 ±0.83 | -68 | |
| Competitive / floor#48 / round-1 | 6 | -29 | -4.83 ±6.14 | -18 | ~noise |
| Defensive / floor#218 / round-2 | 7 | -29 | -4.14 ±1.31 | -45 | |
| Defensive / floor#56 / round-1 | 15 | -29 | -1.93 ±3.97 | -36 | ~noise |
| Competitive / floor#45+rb / round-2 | 4 | -27 | -6.75 ±4.83 | -52 | |
| Defensive / floor#205 / round-2 | 69 | -27 | -0.39 ±1.63 | -80 | ~noise |
| Defensive / floor#208 / round-2 | 6 | -27 | -4.50 ±4.58 | -20 | ~noise |
| Defensive / floor#61 / balancing | 22 | -27 | -1.23 ±2.24 | -68 | ~noise |
| Competitive / floor#57 / deep | 2 | -26 | -13.00 ±1.96 | -29 | |
| Competitive / floor#62 / round-1 | 3 | -26 | -8.67 ±9.49 | -42 | ~noise |
| Defensive / floor#140 / balancing | 4 | -26 | -6.50 ±5.09 | -26 | |
| Competitive / floor#140+rb / round-2 | 2 | -25 | -12.50 ±2.94 | -25 | |
| Competitive / floor#239+rb / deep | 5 | -25 | -5.00 ±9.02 | -30 | ~noise |
| Competitive / floor#62 / deep | 12 | -25 | -2.08 ±3.29 | -41 | ~noise |
| Defensive / floor#227 / round-2 | 3 | -25 | -8.33 ±2.61 | -25 | |
| Defensive / floor#237 / balancing | 25 | -25 | -1.00 ±2.12 | -74 | ~noise |
| Defensive / floor#31 / deep | 7 | -25 | -3.57 ±3.89 | -30 | ~noise |
| Competitive / floor#145 / balancing | 4 | -24 | -6.00 ±6.84 | -24 | ~noise |
| Competitive / floor#241+rb / balancing | 10 | -24 | -2.40 ±3.05 | -44 | ~noise |
| Competitive / floor#47 / round-1 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Competitive / floor#63 / round-1 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Constructive / floor#145 / round-1 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Constructive / floor#148 / round-2 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Defensive / floor#14 / round-2 | 6 | -24 | -4.00 ±6.66 | -17 | ~noise |
| Defensive / floor#211 / round-2 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Defensive / floor#229 / round-2 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Defensive / floor#32 / deep | 9 | -24 | -2.67 ±1.13 | -48 | |
| Defensive / floor#35 / deep | 13 | -24 | -1.85 ±3.39 | -43 | ~noise |
| Competitive / floor#235 / balancing | 10 | -23 | -2.30 ±3.42 | -51 | ~noise |
| Competitive / floor#56 / round-2 | 2 | -23 | -11.50 ±0.98 | -24 | |
| Competitive / floor#143 / round-2 | 2 | -22 | -11.00 ±1.96 | -24 | |
| Competitive / floor#17 / round-2 | 2 | -22 | -11.00 ±1.96 | -24 | |
| Defensive / floor#228 / round-2 | 7 | -22 | -3.14 ±4.69 | -14 | ~noise |
| Defensive / floor#27 / round-1 | 5 | -22 | -4.40 ±5.42 | -17 | ~noise |
| Competitive / floor#235+rb / balancing | 2 | -21 | -10.50 ±2.94 | -21 | |
| Competitive / fallback@2 / balancing | 2 | -20 | -10.00 ±1.96 | -20 | |
| Competitive / fallback@3+rb / round-2 | 6 | -20 | -3.33 ±3.94 | -10 | ~noise |
| Competitive / floor#129+rb / deep | 2 | -20 | -10.00 ±3.92 | -21 | |
| Competitive / floor#18 / round-1 | 15 | -20 | -1.33 ±4.04 | +2 | ~noise plain/PD-flip |
| Competitive / floor#45 / round-2 | 2 | -20 | -10.00 ±1.96 | -18 | |
| Competitive / floor#239+rb / balancing | 4 | -19 | -4.75 ±6.47 | -40 | ~noise |
| Defensive / floor#56 / deep | 3 | -19 | -6.33 ±2.61 | -38 | |
| Competitive / floor#145 / round-2 | 3 | -18 | -6.00 ±6.88 | -25 | ~noise |
| Competitive / floor#241+rb / round-2 | 3 | -18 | -6.00 ±11.81 | -16 | ~noise |
| Competitive / floor#30+rb / round-2 | 7 | -18 | -2.57 ±1.86 | -66 | |
| Competitive / floor#63+rb / round-2 | 2 | -18 | -9.00 ±5.88 | -10 | |
| Constructive / floor#157 / round-2 | 18 | -18 | -1.00 ±4.18 | -18 | ~noise |
| Defensive / floor#61 / deep | 22 | -18 | -0.82 ±2.11 | -99 | ~noise |
| Competitive / floor#24 / deep | 4 | -17 | -4.25 ±3.70 | -24 | |
| Competitive / floor#62+rb / round-2 | 4 | -17 | -4.25 ±11.61 | -4 | ~noise |
| Constructive / floor#17 / round-2 | 2 | -17 | -8.50 ±2.94 | -17 | |
| Competitive / fallback@4+rb / round-2 | 4 | -16 | -4.00 ±2.89 | -19 | |
| Defensive / floor#129 / deep | 21 | -15 | -0.71 ±3.32 | -10 | ~noise |
| Defensive / floor#17 / deep | 4 | -15 | -3.75 ±7.52 | -12 | ~noise |
| Defensive / floor#50 / deep | 12 | -15 | -1.25 ±2.23 | -15 | ~noise |
| Competitive / floor#135 / round-2 | 2 | -14 | -7.00 ±3.92 | -26 | |
| Competitive / floor#140 / deep | 2 | -14 | -7.00 ±1.96 | -22 | |
| Competitive / floor#61+rb / balancing | 2 | -14 | -7.00 ±3.92 | -14 | |
| Constructive / floor#32 / round-2 | 4 | -14 | -3.50 ±11.94 | -6 | ~noise |
| Defensive / floor#11 / deep | 2 | -14 | -7.00 ±13.72 | -14 | ~noise |
| Competitive / floor#145 / round-1 | 1 | -13 | -13.00 ±0.00 | -13 | ~noise |
| Competitive / floor#63+rb / deep | 2 | -13 | -6.50 ±8.82 | -14 | ~noise |
| Defensive / book / deep | 4 | -13 | -3.25 ±4.55 | -19 | ~noise |
| Defensive / floor#18 / deep | 3 | -13 | -4.33 ±0.65 | -24 | |
| Defensive / floor#26 / deep | 1 | -13 | -13.00 ±0.00 | -14 | ~noise |
| Defensive / floor#39 / round-2 | 3 | -13 | -4.33 ±3.46 | -36 | |
| Competitive / floor#26 / deep | 1 | -12 | -12.00 ±0.00 | -14 | ~noise |
| Defensive / floor#28 / round-1 | 2 | -12 | -6.00 ±1.96 | -14 | |
| Defensive / floor#55 / deep | 2 | -12 | -6.00 ±11.76 | -12 | ~noise |
| Competitive / floor#45+rb / deep | 2 | -11 | -5.50 ±2.94 | -21 | |
| Constructive / floor#147 / round-1 | 1 | -11 | -11.00 ±0.00 | -11 | ~noise |
| Defensive / floor#230 / round-2 | 2 | -11 | -5.50 ±0.98 | -14 | |
| Defensive / floor#26 / round-1 | 1 | -11 | -11.00 ±0.00 | -14 | ~noise |
| Defensive / floor#41 / deep | 3 | -11 | -3.67 ±5.23 | -9 | ~noise |
| Defensive / floor#63 / deep | 2 | -11 | -5.50 ±2.94 | -14 | |
| Competitive / floor#151 / round-2 | 10 | -10 | -1.00 ±5.31 | -11 | ~noise |
| Defensive / floor#212 / round-2 | 1 | -10 | -10.00 ±0.00 | -10 | ~noise |
| Defensive / floor#27 / deep | 2 | -10 | -5.00 ±1.96 | -18 | |
| Defensive / floor#28 / round-2 | 1 | -10 | -10.00 ±0.00 | -10 | ~noise |
| Competitive / fallback@3 / balancing | 1 | -9 | -9.00 ±0.00 | -9 | ~noise |
| Competitive / fallback@5+rb / round-2 | 2 | -9 | -4.50 ±0.98 | -12 | |
| Competitive / floor#32 / deep | 5 | -9 | -1.80 ±2.18 | -12 | ~noise |
| Competitive / floor#41 / deep | 2 | -9 | -4.50 ±8.82 | -5 | ~noise |
| Competitive / floor#47+rb / deep | 1 | -9 | -9.00 ±0.00 | -12 | ~noise |
| Defensive / floor#60 / deep | 6 | -9 | -1.50 ±1.58 | -26 | ~noise |
| Competitive / floor#237+rb / balancing | 4 | -8 | -2.00 ±10.03 | -5 | ~noise |
| Constructive / floor#157 / deep | 20 | -8 | -0.40 ±3.23 | -1 | ~noise |
| Defensive / floor#1 / round-1 | 19 | -8 | -0.42 ±4.10 | -8 | ~noise |
| Defensive / floor#231 / round-2 | 6 | -8 | -1.33 ±6.41 | -30 | ~noise |
| Defensive / floor#237 / deep | 2 | -8 | -4.00 ±3.92 | -22 | |
| Competitive / floor#235+rb / deep | 5 | -7 | -1.40 ±2.02 | -40 | ~noise |
| Competitive / floor#54 / deep | 3 | -7 | -2.33 ±4.57 | -1 | ~noise |
| Defensive / floor#62 / deep | 5 | -7 | -1.40 ±6.58 | -17 | ~noise |
| Competitive / floor#144 / round-1 | 1 | -6 | -6.00 ±0.00 | -6 | ~noise |
| Competitive / floor#144 / round-2 | 7 | -6 | -0.86 ±6.73 | -6 | ~noise |
| Defensive / floor#238 / balancing | 1 | -6 | -6.00 ±0.00 | -6 | ~noise |
| Defensive / floor#219 / round-2 | 1 | -5 | -5.00 ±0.00 | -6 | ~noise |
| Defensive / floor#25 / round-2 | 2 | -5 | -2.50 ±4.90 | -6 | ~noise |
| Defensive / floor#46 / deep | 2 | -5 | -2.50 ±0.98 | -12 | |
| Defensive / floor#48 / deep | 2 | -5 | -2.50 ±0.98 | -12 | |
| Competitive / floor#56 / deep | 4 | -4 | -1.00 ±3.84 | -1 | ~noise |
| Competitive / floor#60+rb / deep | 3 | -4 | -1.33 ±3.64 | -15 | ~noise |
| Defensive / floor#199 / round-2 | 62 | -3 | -0.05 ±1.42 | -96 | ~noise |
| Defensive / floor#235 / deep | 5 | -3 | -0.60 ±1.18 | -17 | ~noise |
| Defensive / floor#240 / deep | 1 | -3 | -3.00 ±0.00 | -9 | ~noise |
| Defensive / floor#9 / round-2 | 1 | -3 | -3.00 ±0.00 | -9 | ~noise |
| Competitive / fallback@4 / balancing | 1 | -2 | -2.00 ±0.00 | -2 | ~noise |
| Competitive / floor#237+rb / deep | 1 | -2 | -2.00 ±0.00 | -2 | ~noise |
| Defensive / floor#128 / deep | 2 | -2 | -1.00 ±3.92 | -7 | ~noise |
| Defensive / floor#235 / balancing | 15 | -2 | -0.13 ±2.39 | -32 | ~noise |
| Defensive / floor#18 / round-1 | 9 | -1 | -0.11 ±6.23 | +5 | ~noise plain/PD-flip |
| Defensive / floor#238 / deep | 2 | -1 | -0.50 ±2.94 | -12 | ~noise |
| Competitive / floor#12 / deep | 6 | +0 | +0.00 ±4.08 | +0 | ~noise |
| Competitive / floor#143 / balancing | 2 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Defensive / floor#226 / round-2 | 1 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Defensive / floor#24 / round-2 | 2 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Defensive / floor#57 / round-1 | 1 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Competitive / floor#14 / round-2 | 1 | +1 | +1.00 ±0.00 | +1 | ~noise |
| Competitive / floor#33 / balancing | 1 | +1 | +1.00 ±0.00 | -2 | ~noise plain/PD-flip |
| Constructive / floor#154 / deep | 13 | +1 | +0.08 ±3.45 | +10 | ~noise |
| Constructive / floor#151 / deep | 8 | +2 | +0.25 ±0.32 | +2 | ~noise |
| Competitive / floor#0 / deep | 9 | +3 | +0.33 ±5.03 | -15 | ~noise plain/PD-flip |
| Competitive / floor#26 / round-2 | 2 | +3 | +1.50 ±6.86 | +4 | ~noise |
| Defensive / floor#127 / round-2 | 8 | +3 | +0.38 ±3.72 | +0 | ~noise |
| Defensive / floor#39 / deep | 6 | +3 | +0.50 ±5.33 | -19 | ~noise plain/PD-flip |
| Competitive / floor#147 / deep | 2 | +4 | +2.00 ±0.00 | +4 | |
| Competitive / floor#18 / deep | 3 | +4 | +1.33 ±3.64 | +9 | ~noise |
| Defensive / floor#13 / round-2 | 3 | +4 | +1.33 ±11.39 | +3 | ~noise |
| Defensive / floor#2 / round-1 | 8 | +4 | +0.50 ±6.79 | +5 | ~noise |
| Defensive / floor#41 / round-1 | 6 | +4 | +0.67 ±5.13 | +35 | ~noise |
| Defensive / floor#42 / deep | 3 | +4 | +1.33 ±11.33 | +16 | ~noise |
| Competitive / floor#15 / deep | 6 | +6 | +1.00 ±3.47 | +2 | ~noise |
| Competitive / floor#40 / deep | 1 | +6 | +6.00 ±0.00 | +8 | ~noise |
| Competitive / floor#62 / balancing | 5 | +6 | +1.20 ±5.59 | +43 | ~noise |
| Constructive / floor#145 / deep | 17 | +6 | +0.35 ±4.56 | +6 | ~noise |
| Competitive / floor#32 / balancing | 1 | +7 | +7.00 ±0.00 | +7 | ~noise |
| Constructive / floor#48 / deep | 8 | +7 | +0.88 ±4.63 | -7 | ~noise plain/PD-flip |
| Defensive / floor#241 / balancing | 22 | +7 | +0.32 ±2.19 | -29 | ~noise plain/PD-flip |
| Defensive / floor#14 / round-1 | 6 | +9 | +1.50 ±4.52 | +19 | ~noise |
| Defensive / floor#145 / round-1 | 10 | +9 | +0.90 ±6.21 | +38 | ~noise |
| Defensive / floor#34 / round-1 | 1 | +9 | +9.00 ±0.00 | +9 | ~noise |
| Defensive / floor#38 / round-2 | 2 | +9 | +4.50 ±0.98 | +12 | |
| Defensive / floor#63 / round-1 | 10 | +9 | +0.90 ±5.44 | +0 | ~noise |
| Competitive / floor#60 / deep | 3 | +10 | +3.33 ±6.53 | +15 | ~noise |
| Constructive / floor#47 / deep | 15 | +10 | +0.67 ±3.27 | -12 | ~noise plain/PD-flip |
| Constructive / floor#48 / round-2 | 1 | +10 | +10.00 ±0.00 | +10 | ~noise |
| Defensive / floor#47 / balancing | 2 | +10 | +5.00 ±0.00 | +10 | |
| Defensive / floor#57 / deep | 1 | +10 | +10.00 ±0.00 | +4 | ~noise |
| Defensive / floor#40 / deep | 4 | +11 | +2.75 ±10.87 | +0 | ~noise |
| Defensive / floor#47 / deep | 5 | +12 | +2.40 ±5.87 | +17 | ~noise |
| Competitive / floor#151 / balancing | 4 | +16 | +4.00 ±4.80 | +16 | ~noise |
| Competitive / floor#239 / deep | 17 | +16 | +0.94 ±2.85 | -63 | ~noise plain/PD-flip |
| Competitive / floor#63 / deep | 2 | +16 | +8.00 ±3.92 | +16 | |
| Defensive / floor#0 / round-1 | 3 | +16 | +5.33 ±2.85 | +17 | |
| Defensive / floor#145 / balancing | 2 | +16 | +8.00 ±3.92 | +16 | |
| Defensive / floor#16 / deep | 4 | +16 | +4.00 ±6.55 | +3 | ~noise |
| Defensive / floor#57 / round-2 | 11 | +16 | +1.45 ±6.18 | +25 | ~noise |
| Competitive / floor#1 / deep | 136 | +17 | +0.12 ±1.05 | -87 | ~noise plain/PD-flip |
| Competitive / floor#61 / balancing | 4 | +17 | +4.25 ±3.03 | +29 | |
| Competitive / floor#147 / round-2 | 17 | +21 | +1.24 ±4.37 | +36 | ~noise |
| Competitive / floor#41 / round-2 | 11 | +21 | +1.91 ±6.23 | -35 | ~noise plain/PD-flip |
| Competitive / floor#18+rb / deep | 2 | +24 | +12.00 ±1.96 | +26 | |
| Defensive / floor#62 / round-1 | 7 | +24 | +3.43 ±6.77 | +21 | ~noise |
| Competitive / floor#151 / round-1 | 4 | +26 | +6.50 ±6.28 | +26 | |
| Constructive / floor#148 / deep | 4 | +26 | +6.50 ±6.28 | +29 | |
| Constructive / floor#62 / deep | 25 | +27 | +1.08 ±2.61 | -5 | ~noise plain/PD-flip |
| Defensive / floor#62 / round-2 | 92 | +27 | +0.29 ±1.44 | -89 | ~noise plain/PD-flip |
| Competitive / floor#32 / round-2 | 36 | +29 | +0.81 ±1.97 | +0 | ~noise |
| Competitive / floor#47+rb / round-2 | 2 | +30 | +15.00 ±1.96 | +30 | |
| Constructive / floor#153 / deep | 4 | +32 | +8.00 ±2.26 | +32 | |
| Defensive / floor#210 / round-2 | 4 | +35 | +8.75 ±3.78 | +38 | |
| Defensive / floor#42 / round-1 | 4 | +38 | +9.50 ±2.59 | +51 | |
| Defensive / floor#207 / round-2 | 8 | +45 | +5.62 ±2.92 | +65 | |
| Defensive / floor#56 / round-2 | 21 | +45 | +2.14 ±3.67 | +59 | ~noise |
| Competitive / floor#62 / round-2 | 49 | +58 | +1.18 ±1.93 | -237 | ~noise plain/PD-flip |
| Defensive / floor#140 / round-1 | 28 | +62 | +2.21 ±3.85 | +79 | ~noise |
| Defensive / floor#6 / round-1 | 26 | +72 | +2.77 ±2.91 | +74 | ~noise |
| Competitive / floor#47 / deep | 24 | +80 | +3.33 ±2.26 | +43 | |
| Defensive / floor#0 / round-2 | 31 | +125 | +4.03 ±2.58 | +102 | |
| Competitive / floor#0 / round-2 | 77 | +326 | +4.23 ±1.55 | +293 | |

## By phase

  -312941 IMPs  137243 boards  Constructive
  -308155 IMPs  116969 boards  Defensive
  -195735 IMPs   59941 boards  Competitive

## By provenance

  -436442 IMPs  189121 boards  book
   -94090 IMPs   30023 boards  floor#3
   -44824 IMPs   14139 boards  fallback@1
   -43247 IMPs   12939 boards  fallback@2
   -41579 IMPs   12768 boards  floor#242
   -29476 IMPs    9586 boards  fallback@3
   -12157 IMPs    3723 boards  fallback@4
    -8187 IMPs    3467 boards  floor#20
    -8059 IMPs    2727 boards  floor#46
    -7808 IMPs    2634 boards  floor#61
    -6901 IMPs    3134 boards  floor#35
    -5951 IMPs    2071 boards  floor#140
    -5499 IMPs    2931 boards  floor#50
    -5352 IMPs    1696 boards  floor#60
    -5265 IMPs     784 boards  floor#242+rb
    -4144 IMPs    1042 boards  floor#243
    -4119 IMPs     931 boards  floor#202
    -3867 IMPs    1198 boards  floor#30
    -3729 IMPs    1727 boards  floor#64
    -3153 IMPs     773 boards  floor#31
    -2876 IMPs     792 boards  floor#45
    -2800 IMPs     593 boards  floor#16
    -2416 IMPs     750 boards  floor#132
    -2379 IMPs     575 boards  floor#200
    -1981 IMPs    1463 boards  floor#65
    -1779 IMPs     551 boards  floor#131
    -1416 IMPs     479 boards  fallback@5
    -1345 IMPs     416 boards  floor#32
    -1281 IMPs     384 boards  floor#17
    -1236 IMPs     391 boards  floor#240
    -1208 IMPs     618 boards  book+rb
    -1083 IMPs     347 boards  floor#5
    -1027 IMPs     298 boards  floor#197
     -948 IMPs     276 boards  floor#198
     -843 IMPs     450 boards  floor#49
     -771 IMPs     239 boards  floor#241
     -741 IMPs     284 boards  floor#237
     -737 IMPs     296 boards  floor#21
     -715 IMPs     294 boards  floor#51
     -712 IMPs     255 boards  floor#239
     -706 IMPs     243 boards  floor#6
     -691 IMPs     332 boards  floor#199
     -651 IMPs     242 boards  floor#238
     -619 IMPs     125 boards  floor#145
     -615 IMPs     200 boards  floor#66
     -595 IMPs     143 boards  floor#15
     -594 IMPs     816 boards  floor#1
     -516 IMPs     207 boards  floor#36
     -494 IMPs     190 boards  floor#236
     -481 IMPs     325 boards  floor#129
     -480 IMPs     130 boards  floor#235
     -380 IMPs     398 boards  floor#133
     -344 IMPs      87 boards  floor#151
     -341 IMPs     141 boards  floor#234
     -327 IMPs      76 boards  floor#10
     -308 IMPs      71 boards  floor#33
     -306 IMPs      82 boards  floor#63
     -298 IMPs     362 boards  floor#47
     -297 IMPs     172 boards  floor#3+rb
     -295 IMPs     166 boards  floor#205
     -292 IMPs     118 boards  floor#204
     -289 IMPs     100 boards  floor#147
     -276 IMPs      60 boards  floor#27
     -268 IMPs      85 boards  floor#12
     -253 IMPs     107 boards  floor#203
     -224 IMPs      36 boards  floor#11
     -220 IMPs      85 boards  floor#48
     -211 IMPs      49 boards  floor#153
     -210 IMPs      50 boards  floor#55
     -199 IMPs      59 boards  floor#240+rb
     -198 IMPs     102 boards  floor#25
     -185 IMPs      54 boards  floor#46+rb
     -177 IMPs      19 boards  floor#234+rb
     -167 IMPs      53 boards  floor#238+rb
     -161 IMPs      72 boards  floor#42
     -154 IMPs      42 boards  floor#39
     -152 IMPs      49 boards  floor#9
     -149 IMPs      37 boards  floor#236+rb
     -149 IMPs     148 boards  floor#34
     -136 IMPs      37 boards  floor#54
     -134 IMPs      19 boards  floor#26
     -131 IMPs     136 boards  floor#2
     -112 IMPs      22 boards  floor#24
     -109 IMPs      67 boards  floor#18
     -106 IMPs      21 boards  floor#243+rb
      -96 IMPs      28 boards  floor#57
      -86 IMPs      47 boards  floor#40
      -79 IMPs      16 boards  floor#29
      -79 IMPs     242 boards  floor#62
      -72 IMPs      18 boards  floor#241+rb
      -71 IMPs      42 boards  floor#157
      -69 IMPs      18 boards  fallback@6
      -49 IMPs      15 boards  floor#61+rb
      -48 IMPs      70 boards  floor#41
      -44 IMPs       9 boards  floor#239+rb
      -43 IMPs      11 boards  floor#60+rb
      -38 IMPs       6 boards  floor#45+rb
      -31 IMPs       4 boards  floor#63+rb
      -30 IMPs      45 boards  floor#56
      -29 IMPs       7 boards  floor#218
      -28 IMPs       7 boards  floor#235+rb
      -27 IMPs       6 boards  floor#208
      -25 IMPs       2 boards  floor#140+rb
      -25 IMPs       3 boards  floor#227
      -24 IMPs       2 boards  floor#211
      -24 IMPs       2 boards  floor#229
      -22 IMPs       4 boards  floor#143
      -22 IMPs       7 boards  floor#228
      -22 IMPs       3 boards  floor#28
      -20 IMPs       6 boards  fallback@3+rb
      -20 IMPs       2 boards  floor#129+rb
      -18 IMPs       7 boards  floor#30+rb
      -17 IMPs       4 boards  floor#62+rb
      -16 IMPs       4 boards  fallback@4+rb
      -14 IMPs       2 boards  floor#135
      -14 IMPs      13 boards  floor#14
      -12 IMPs       8 boards  floor#144
      -12 IMPs       7 boards  floor#47+rb
      -11 IMPs       2 boards  floor#230
      -10 IMPs       1 boards  floor#212
      -10 IMPs       5 boards  floor#237+rb
       -9 IMPs       2 boards  fallback@5+rb
       -8 IMPs       6 boards  floor#231
       -5 IMPs       1 boards  floor#219
       -2 IMPs       2 boards  floor#128
       +0 IMPs       1 boards  floor#226
       +1 IMPs      13 boards  floor#154
       +2 IMPs       6 boards  floor#148
       +3 IMPs       8 boards  floor#127
       +4 IMPs       3 boards  floor#13
       +9 IMPs       2 boards  floor#38
      +24 IMPs       2 boards  floor#18+rb
      +35 IMPs       4 boards  floor#210
      +45 IMPs       8 boards  floor#207
     +470 IMPs     120 boards  floor#0

## By family

  -406801 IMPs  146085 boards  round-1
  -260890 IMPs   97428 boards  round-2
  -103480 IMPs   47692 boards  opening
   -28581 IMPs   12131 boards  balancing
   -17079 IMPs   10817 boards  deep

## By direction

  -527721 IMPs   71589 boards  other
  -248310 IMPs   36374 boards  overbid
  -195831 IMPs   22680 boards  missed-game
  -152576 IMPs   21013 boards  sold-out
   -84382 IMPs   15031 boards  wrong-strain
   -80978 IMPs    6590 boards  missed-slam
   -11431 IMPs     780 boards  missed-grand
   -10398 IMPs    1465 boards  doubling
       +0 IMPs   49368 boards  flat
  +494796 IMPs   89263 boards  gain

## Worst boards per losing bucket

### Defensive / book / round-1 (59437 boards, -142733 IMPs)

[vul both, seed Some(1783375083), board 313] swing -3250 pts / -22 IMPs (PD -22), diverged at call 1 (3♣ ours vs P BBA), other
  rule: 5+ ♣, and 10–16 points
  W:A763.AKQ96.JT76. Q95..KQ982.AT873 KJT2.JT7543.43.5 84.82.A5.KQJ9642
  ours NS @ A: 2♥ 3♣ 4♣ - 4♥ - - -  -> 4♥ by East
  ours EW @ B: 2♥ - 2NT - 3♣ X - - -  -> 3♣x by East

[vul both, seed Some(1783375074), board 2840] swing -2500 pts / -21 IMPs (PD -21), diverged at call 2 (3♣ ours vs 5♣ BBA), missed-slam
  rule: 5+ ♣, and 10–16 points
  W:963.AKQJ832.6.95 QT52.54.KJT74.K8 AKJ874.T76.932.J .9.AQ85.AQT76432
  ours NS @ A: - 2♠ 3♣ 3♠ - - X - - -  -> 3♠x by East
  ours EW @ B: - 2♠ 5♣ - 6♣ - - -  -> 6♣ by South

[vul both, seed Some(1783375077), board 2125] swing -2560 pts / -21 IMPs (PD -21), diverged at call 2 (2♥ ours vs X BBA), other
  rule: 5+ ♠, (5+ ♣, or 5+ ♦), and 8+ points
  W:AKJT7652...AKJ86 4.T62.A652.QT542 Q98.QJ543.QT73.7 3.AK987.KJ984.93
  ours NS @ A: - 1♥ X 2♥ - - 6♠ - - -  -> 6♠ by West
  ours EW @ B: - 1♥ 2♥ X - - -  -> 2♥x by West

### Constructive / book / opening (47692 boards, -103480 IMPs)

[vul both, seed Some(1783375067), board 372] swing -3400 pts / -22 IMPs (PD -22), diverged at call 0 (2♦ ours vs P BBA), overbid
  rule: exactly 6 ♦, 5–10 points, and not (opening in seat 4)
  W:A5.KQ973.J.97543 KT6.4.Q86432.QT6 832.AT852.T7.AJ8 QJ974.J6.AK95.K2
  ours NS @ A: 2♦ - 2NT - 3♥ X - - -  -> 3♥x by North
  ours EW @ B: - - 1♠ 2♠ X - - -  -> 2♠x by West

[vul both, seed Some(1783375064), board 13] swing -2800 pts / -21 IMPs (PD -21), diverged at call 0 (P ours vs 1♠ BBA), overbid
  rule: ≤11 points
  W:J97.A95.Q4.AJT97 Q43.3.AKJ9853.Q2 AKT85.7642..K865 62.KQJT8.T762.43
  ours NS @ A: 1♠ - 1NT 2♦ - - 3♠ X - 3NT X - - -  -> 3NTx by South
  ours EW @ B: - - 1♣ 1♦ X 2♦ X - 4♥ X - - -  -> 4♥x by East

[vul both, seed Some(1783375076), board 3305] swing -2540 pts / -21 IMPs (PD -22), diverged at call 0 (1♦ ours vs P BBA), other
  rule: 10–11 HCP, Rule of 20, prefers diamonds, ≤4 ♥, and ≤4 ♠
  W:.AK2.AQ63.AJ9754 987654.QJ864.8.8 KQ32.T5.KJ954.QT AJT.973.T72.K632
  ours NS @ A: - - 1♣ - 1♦ - 5♠ - 6♣ - 7♦ - - -  -> 7♦ by East
  ours EW @ B: 1♦ - 2♣ - 2♠ - 3♣ - 4♠ - - -  -> 4♠ by East

### Constructive / book / round-2 (43212 boards, -98201 IMPs)

[vul both, seed Some(1783375080), board 5575] swing -2560 pts / -21 IMPs (PD -21), diverged at call 4 (3♣ ours vs 4♦ BBA), other
  rule: 4+ card support for partner
  W:AQ987.AKJ7..K753 JT3.T94.JT84.J94 K42.Q5.A65.AQT82 65.8632.KQ9732.6
  ours NS @ A: 1♠ - 2♣ - 4♦ - 4♠ - 5♥ - 6♠ - - -  -> 6♠ by West
  ours EW @ B: 1♠ - 2♣ - 3♣ - 3♠ - 4NT - 5♦ X - - -  -> 5♦x by East

[vul both, seed Some(1783375068), board 274] swing -2830 pts / -21 IMPs (PD -21), diverged at call 4 (2♦ ours vs 3♦ BBA), missed-slam
  rule: 4+ ♦
  W:T765.J654.KT3.87 AJ4.AQ9.94.QJ642 KQ9832.7.Q86.T95 .KT832.AJ752.AK3
  ours NS @ A: 1♥ - 2♣ - 2♦ - 3♥ - 4NT - 5♠ X - - -  -> 5♠x by North
  ours EW @ B: 1♥ - 2♣ - 3♦ - 3♥ - 5♠ - 6♣ - 6♥ - - -  -> 6♥ by South

[vul none, seed Some(1783375068), board 274] swing -2080 pts / -19 IMPs (PD -19), diverged at call 4 (2♦ ours vs 3♦ BBA), missed-slam
  rule: 4+ ♦
  W:T765.J654.KT3.87 AJ4.AQ9.94.QJ642 KQ9832.7.Q86.T95 .KT832.AJ752.AK3
  ours NS @ A: 1♥ - 2♣ - 2♦ - 3♥ - 4NT - 5♠ X - - -  -> 5♠x by North
  ours EW @ B: 1♥ - 2♣ - 3♦ - 3♥ - 5♠ - 6♣ - 6♥ - - -  -> 6♥ by South

### Constructive / book / round-1 (29727 boards, -76291 IMPs)

[vul both, seed Some(1783375092), board 5926] swing -4510 pts / -24 IMPs (PD -24), diverged at call 2 (4♣ ours vs 3♦ BBA), missed-grand
  rule: 4+ card support for partner, 10–13 points, and ≤1 ♣
  W:J62.3.9752.KJ875 K943.KQJT8764..2 7.95.KQJT8643.Q6 AQT85.A2.A.AT943
  ours NS @ A: 1♠ - 4♣ - 4NT - 5♣ - 5NT - 6♦ X - - -  -> 6♦x by North
  ours EW @ B: 1♠ - 3♦ - 3♥ - 5♦ - 6♣ - 6♦ - 6NT - 7♠ - - -  -> 7♠ by South

[vul both, seed Some(1783375086), board 3748] swing -3460 pts / -22 IMPs (PD -22), diverged at call 2 (4♣ ours vs 2NT BBA), wrong-strain
  rule: 4+ card support for partner, 10–13 points, and ≤1 ♣
  W:.Q4.JT96.KQJ7654 KJ542.K7.AK54.AT T97.JT32.Q82.832 AQ863.A9865.73.9
  ours NS @ A: 1♠ - 4♣ X 4NT - 5♠ - 5NT - 6♣ X - - -  -> 6♣x by South
  ours EW @ B: 1♠ - 2NT - 3♠ - 4♣ - 4♦ - 4♠ - 4NT - 5♠ - 6♠ - - -  -> 6♠ by North

[vul none, seed Some(1783375086), board 3748] swing -2710 pts / -21 IMPs (PD -21), diverged at call 2 (4♣ ours vs 2NT BBA), wrong-strain
  rule: 4+ card support for partner, 10–13 points, and ≤1 ♣
  W:.Q4.JT96.KQJ7654 KJ542.K7.AK54.AT T97.JT32.Q82.832 AQ863.A9865.73.9
  ours NS @ A: 1♠ - 4♣ X 4NT - 5♠ - 5NT - 6♣ X - - -  -> 6♣x by South
  ours EW @ B: 1♠ - 2NT - 3♠ - 4♣ - 4♦ - 4♠ - 4NT - 5♠ - 6♠ - - -  -> 6♠ by North

### Competitive / fallback@1 / round-1 (13846 boards, -44169 IMPs)

[vul both, seed Some(1783375079), board 667] swing -3220 pts / -22 IMPs (PD -22), diverged at call 2 (4♣ ours vs 4♥ BBA), other
  rule: 5+ ♣, (5+ ♥, or 5+ ♠), and 10+ points
  W:AQ95.J93.Q8.KQJ6 KJT864.A5.AT932. 3.KQT872.J.A9754 72.64.K7654.T832
  ours NS @ A: 1NT 2♦ 4♥ - - -  -> 4♥ by East
  ours EW @ B: 1NT 2♦ 4♣ - 4♦ X - - -  -> 4♦x by West

[vul both, seed Some(1783375087), board 5247] swing -2700 pts / -21 IMPs (PD -21), diverged at call 2 (P ours vs 2♥ BBA), overbid
  rule: 0+ HCP
  W:542.AKQ75.T.A875 KQJ763.T96.AQ.64 98.J42.J63.KT932 AT.83.K987542.QJ
  ours NS @ A: 1♥ 1♠ 2♥ - - 2♠ - - 3♥ 3♠ - - -  -> 3♠ by North
  ours EW @ B: 1♥ 1♠ - 2♦ X 3♠ - - X - 3NT - - X - - -  -> 3NTx by East

[vul none, seed Some(1783375087), board 5247] swing -2350 pts / -20 IMPs (PD -20), diverged at call 2 (P ours vs 2♥ BBA), overbid
  rule: 0+ HCP
  W:542.AKQ75.T.A875 KQJ763.T96.AQ.64 98.J42.J63.KT932 AT.83.K987542.QJ
  ours NS @ A: 1♥ 1♠ 2♥ - - 2♠ - - 3♥ 3♠ - - -  -> 3♠ by North
  ours EW @ B: 1♥ 1♠ - 2♦ X 3♠ - - X - 3NT - - X - - -  -> 3NTx by East

### Competitive / fallback@2 / round-1 (12606 boards, -42221 IMPs)

[vul both, seed Some(1783375081), board 2158] swing -2520 pts / -21 IMPs (PD -19), diverged at call 2 (P ours vs 1♠ BBA), sold-out
  rule: 0+ HCP
  W:A74.AKJ9.Q.AQJ54 QJT952.Q8.75.982 .T76542.JT986.T6 K863.3.AK432.K73
  ours NS @ A: 1♦ X - 1♥ X XX - - -  -> 1♥xx by East
  ours EW @ B: 1♦ X 1♠ - 2♠ X - 3♥ - 4♥ 4♠ - - -  -> 4♠ by North

[vul both, seed Some(1783375092), board 5373] swing -2590 pts / -21 IMPs (PD -21), diverged at call 3 (P ours vs 2♦ BBA), sold-out
  rule: 0+ HCP
  W:AQJ93.QT6..KQJT8 K2.752.AQJ842.42 T765.K94.T965.65 84.AJ83.K73.A973
  ours NS @ A: - 1♣ 1♠ - - X XX - - -  -> 1♠xx by West
  ours EW @ B: - 1♣ 1♠ 2♦ - 2♥ X XX - 3♦ X - - -  -> 3♦x by North

[vul both, seed Some(1783375093), board 3413] swing -2950 pts / -21 IMPs (PD -21), diverged at call 2 (3♥ ours vs X BBA), missed-game
  rule: 3+ ♥, and 6–9 points
  W:873.AQ843.J82.65 QJ954.5.9743.KJ3 .KT972.AQT65.Q84 AKT62.J6.K.AT972
  ours NS @ A: 1♥ 2♥ X - - -  -> 2♥x by South
  ours EW @ B: 1♥ 2♥ 3♥ 3♠ 4♥ - - 4♠ - - -  -> 4♠ by North

### Defensive / floor#3 / round-2 (9900 boards, -31665 IMPs)

[vul both, seed Some(1783375094), board 1033] swing -2940 pts / -21 IMPs (PD -21), diverged at call 5 (P ours vs 3♠ BBA), missed-game
  rule: not ((opaque condition)), or (opaque condition)
  W:6.KJT7632.2.Q862 KQ72.98.KQT.KJ54 A83.A54.J985.A97 JT954.Q.A7643.T3
  ours NS @ A: 1♦ - 1♥ X XX - - -  -> 1♥xx by West
  ours EW @ B: 1♦ - 1♥ X XX 3♠ - - 4♥ - - 4♠ - - -  -> 4♠ by South

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

### Defensive / floor#3 / round-1 (8597 boards, -29193 IMPs)

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

[vul both, seed Some(1783375074), board 1753] swing -2060 pts / -19 IMPs (PD -19), diverged at call 3 (P ours vs 1♥ BBA), other
  rule: not ((opaque condition)), or (opaque condition)
  W:A85.JT43.QJ7.KQ5 JT92.9876.962.84 Q3.A.AKT854.J732 K764.KQ52.3.AT96
  ours NS @ A: 1♦ X XX - - -  -> 1♦xx by East
  ours EW @ B: 1♦ X XX 1♥ 2♦ - 3♦ - 4♦ - - -  -> 4♦ by East

### Competitive / fallback@3 / round-2 (7354 boards, -21563 IMPs)

[vul both, seed Some(1783375089), board 6277] swing -2320 pts / -20 IMPs (PD -20), diverged at call 4 (P ours vs X BBA), missed-game
  rule: 0+ HCP
  W:J.K6.KJ9832.AJ96 AQ875.AJ852..752 62.QT743.AT65.K8 KT943.9.Q74.QT43
  ours NS @ A: - - 1♦ 2♦ X - - -  -> 2♦x by North
  ours EW @ B: - - 1♦ 2♦ - 4♠ - - -  -> 4♠ by South

[vul both, seed Some(1783375064), board 4909] swing -2000 pts / -19 IMPs (PD -19), diverged at call 4 (X ours vs 2♦ BBA), other
  rule: 4+ ♠, and 8+ HCP
  W:6.AT762.AJT3.QT5 KQJT32.K93.4.AJ6 A754.5.KQ9876.83 98.QJ84.52.K9742
  ours NS @ A: - - 1♥ 1♠ 2♦ - - 2♠ - - 3♦ 3♠ 5♦ - - -  -> 5♦ by East
  ours EW @ B: - - 1♥ 1♠ X - 4♥ - - X - - -  -> 4♥x by West

[vul both, seed Some(1783375086), board 93] swing -2020 pts / -19 IMPs (PD -21), diverged at call 4 (P ours vs X BBA), overbid
  rule: 0+ HCP
  W:J543.A5.AJ.AT743 AK.KQJ63.3.KJ965 Q872.742.KQT97.8 T96.T98.86542.Q2
  ours NS @ A: - - 1♣ 1♥ X - 3♠ X - 3NT - - -  -> 3NT by South
  ours EW @ B: - - 1♣ 1♥ - - X XX - - -  -> 1♥xx by North

### Defensive / floor#242 / round-1 (3162 boards, -11623 IMPs)

[vul both, seed Some(1783375080), board 2303] swing -2540 pts / -21 IMPs (PD -21), diverged at call 3 (X ours vs 2NT BBA), missed-game
  rule: (opaque condition), at most three cards in each of their suits, 12+ HCP, and (opaque condition)
  W:.KQ8.Q964.AKT742 T852.JT3.T2.J953 K43.A7542.73.Q86 AQJ976.96.AKJ85.
  ours NS @ A: 1♣ - 1♥ X XX - - -  -> 1♥xx by East
  ours EW @ B: 1♣ - 1♥ 2NT 3♥ - - 3♠ 4♥ 4♠ - - -  -> 4♠ by South

[vul both, seed Some(1783375085), board 569] swing -2520 pts / -21 IMPs (PD -21), diverged at call 3 (X ours vs P BBA), other
  rule: (opaque condition), at most three cards in each of their suits, 12+ HCP, and (opaque condition)
  W:5.QT98752.5.QJ42 AKT9.J3.A92.T975 73.AK6.KQ84.AK83 QJ8642.4.JT763.6
  ours NS @ A: 1♦ - 1♥ X XX - - -  -> 1♥xx by West
  ours EW @ B: 1♦ - 1♥ - 2NT - - -  -> 2NT by East

[vul both, seed Some(1783375088), board 325] swing -2250 pts / -20 IMPs (PD -20), diverged at call 1 (X ours vs P BBA), other
  rule: (opaque condition), at most three cards in each of their suits, 12+ HCP, and (opaque condition)
  W:AQ2.AQ84.JT7.AQ4 J94.J965.98.9763 T75.72.AQ65432.J K863.KT3.K.KT852
  ours NS @ A: 3♦ X XX - - -  -> 3♦xx by East
  ours EW @ B: 3♦ - 4♦ - - -  -> 4♦ by East

