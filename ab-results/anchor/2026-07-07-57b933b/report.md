=== arm 0: our american floor (us) vs BBA 2/1 (them), vulnerability none, 102400 boards ===
replay verification: 100.00% of 1076327 our-side calls (0 mismatched)
auction-divergent: 94136 (92%), contract-divergent: 78895 (77%)
plain DD: -1.6646 IMPs/board (95% CI [-1.6967, -1.6326]), -170458 IMPs total
perfect defense: -1.9762 IMPs/board (95% CI [-2.0146, -1.9378])
gen_args: --count 6400 --seed 1783375064 --output ab-results/anchor/2026-07-07-57b933b/none/shard-0.json -v none

=== arm 1: our american floor (us) vs BBA 2/1 (them), vulnerability both, 102400 boards ===
replay verification: 100.00% of 1065392 our-side calls (0 mismatched)
auction-divergent: 93769 (92%), contract-divergent: 78311 (76%)
plain DD: -2.2909 IMPs/board (95% CI [-2.3317, -2.2501]), -234587 IMPs total
perfect defense: -2.7177 IMPs/board (95% CI [-2.7656, -2.6698])
gen_args: --count 6400 --seed 1783375064 --output ab-results/anchor/2026-07-07-57b933b/both/shard-0.json -v both


## IMP histogram (plain, per contract-divergent board)

right-siding-only divergences (same contract, different auction): 30699

  -23 IMPs: 1
  -22 IMPs: 6
  -21 IMPs: 27
  -20 IMPs: 45
  -19 IMPs: 126
  -18 IMPs: 249
  -17 IMPs: 678
  -16 IMPs: 749
  -15 IMPs: 1559
  -14 IMPs: 2382
  -13 IMPs: 4825
  -12 IMPs: 4583
  -11 IMPs: 7115
  -10 IMPs: 9769
   -9 IMPs: 4382
   -8 IMPs: 3655
   -7 IMPs: 6657
   -6 IMPs: 9847
   -5 IMPs: 7995
   -4 IMPs: 4212
   -3 IMPs: 6111
   -2 IMPs: 6620
   -1 IMPs: 5792
   +0 IMPs: 25423
   +1 IMPs: 5243
   +2 IMPs: 4969
   +3 IMPs: 4008
   +4 IMPs: 2913
   +5 IMPs: 7608
   +6 IMPs: 6298
   +7 IMPs: 3077
   +8 IMPs: 1210
   +9 IMPs: 1255
  +10 IMPs: 2587
  +11 IMPs: 2119
  +12 IMPs: 1367
  +13 IMPs: 1430
  +14 IMPs: 227
  +15 IMPs: 47
  +16 IMPs: 21
  +17 IMPs: 16
  +18 IMPs: 1
  +19 IMPs: 2

## Ranked buckets (phase / provenance / family), losses first

| bucket | boards | net plain IMPs | IMPs/divergent ±CI | net PD IMPs | flag |
| --- | --- | --- | --- | --- | --- |
| Defensive / book / round-1 | 28670 | -67707 | -2.36 ±0.08 | -91569 | |
| Constructive / book / opening | 29142 | -67689 | -2.32 ±0.08 | -64361 | |
| Constructive / book / round-2 | 21188 | -45181 | -2.13 ±0.09 | -45933 | |
| Constructive / book / round-1 | 14207 | -38654 | -2.72 ±0.12 | -43747 | |
| Competitive / fallback@2 / round-1 | 5833 | -19578 | -3.36 ±0.17 | -23568 | |
| Competitive / fallback@1 / round-1 | 6306 | -19570 | -3.10 ±0.16 | -22004 | |
| Defensive / floor#3 / round-2 | 4657 | -14716 | -3.16 ±0.19 | -16569 | |
| Defensive / floor#3 / round-1 | 3940 | -13468 | -3.42 ±0.21 | -12333 | |
| Competitive / fallback@3 / round-2 | 3675 | -10946 | -2.98 ±0.21 | -13436 | |
| Competitive / floor#242 / round-2 | 1588 | -5843 | -3.68 ±0.33 | -9644 | |
| Defensive / floor#242 / round-1 | 1561 | -5705 | -3.65 ±0.36 | -7036 | |
| Defensive / book / round-2 | 2143 | -5606 | -2.62 ±0.30 | -7102 | |
| Competitive / fallback@4 / round-2 | 1614 | -5457 | -3.38 ±0.31 | -7324 | |
| Competitive / fallback@3 / round-1 | 997 | -3504 | -3.51 ±0.44 | -4682 | |
| Competitive / floor#3 / round-2 | 1097 | -3232 | -2.95 ±0.42 | -3384 | |
| Constructive / floor#140 / round-2 | 1095 | -2931 | -2.68 ±0.44 | -3155 | |
| Defensive / floor#242 / balancing | 1106 | -2830 | -2.56 ±0.39 | -5809 | |
| Competitive / floor#242 / balancing | 1115 | -2770 | -2.48 ±0.38 | -5947 | |
| Competitive / floor#3 / round-1 | 553 | -2680 | -4.85 ±0.58 | -1389 | |
| Defensive / floor#3 / balancing | 825 | -2653 | -3.22 ±0.38 | -2042 | |
| Competitive / floor#242+rb / round-2 | 393 | -2642 | -6.72 ±0.66 | -3400 | |
| Defensive / floor#242 / round-2 | 679 | -2641 | -3.89 ±0.53 | -4599 | |
| Constructive / floor#3 / round-2 | 1113 | -2308 | -2.07 ±0.34 | -2110 | |
| Constructive / floor#3 / round-1 | 659 | -2270 | -3.44 ±0.55 | -1123 | |
| Defensive / floor#60 / round-2 | 640 | -2013 | -3.15 ±0.46 | -3358 | |
| Defensive / floor#20 / round-1 | 695 | -1954 | -2.81 ±0.48 | -2556 | |
| Defensive / floor#50 / round-1 | 836 | -1771 | -2.12 ±0.42 | -2421 | |
| Defensive / floor#35 / round-1 | 701 | -1713 | -2.44 ±0.46 | -2430 | |
| Defensive / floor#64 / round-1 | 609 | -1595 | -2.62 ±0.55 | -1672 | |
| Competitive / floor#46 / round-2 | 384 | -1415 | -3.68 ±0.58 | -2096 | |
| Constructive / book / deep | 2208 | -1398 | -0.63 ±0.26 | -1510 | |
| Defensive / floor#202 / round-2 | 224 | -1199 | -5.35 ±0.80 | -1817 | |
| Constructive / floor#61 / deep | 603 | -1194 | -1.98 ±0.24 | -1856 | |
| Defensive / floor#35 / round-2 | 578 | -1066 | -1.84 ±0.48 | -1419 | |
| Defensive / floor#202 / round-1 | 246 | -1025 | -4.17 ±0.85 | -1326 | |
| Defensive / floor#20 / round-2 | 604 | -948 | -1.57 ±0.46 | -1657 | |
| Defensive / floor#65 / round-1 | 488 | -942 | -1.93 ±0.56 | -1423 | |
| Defensive / floor#45 / round-2 | 268 | -928 | -3.46 ±0.79 | -1470 | |
| Competitive / floor#31 / round-1 | 162 | -911 | -5.62 ±1.00 | -771 | |
| Competitive / floor#46 / round-1 | 144 | -909 | -6.31 ±1.07 | -933 | |
| Competitive / floor#61 / round-1 | 132 | -907 | -6.87 ±1.05 | -906 | |
| Constructive / floor#3 / deep | 598 | -891 | -1.49 ±0.51 | -738 | |
| Competitive / floor#30 / round-2 | 260 | -855 | -3.29 ±0.65 | -1406 | |
| Defensive / floor#60 / round-1 | 173 | -813 | -4.70 ±0.84 | -1211 | |
| Defensive / floor#20 / balancing | 359 | -757 | -2.11 ±0.60 | -1454 | |
| Competitive / floor#3 / balancing | 309 | -756 | -2.45 ±0.60 | -12 | |
| Defensive / floor#132 / round-1 | 197 | -729 | -3.70 ±0.74 | -1505 | |
| Defensive / floor#61 / round-2 | 201 | -724 | -3.60 ±0.82 | -943 | |
| Defensive / floor#131 / balancing | 204 | -689 | -3.38 ±0.80 | -1209 | |
| Defensive / floor#200 / round-2 | 139 | -662 | -4.76 ±1.09 | -936 | |
| Defensive / floor#243 / round-1 | 119 | -654 | -5.50 ±1.25 | -813 | |
| Defensive / floor#45 / round-1 | 130 | -558 | -4.29 ±1.10 | -745 | |
| Competitive / floor#242 / round-1 | 231 | -546 | -2.36 ±0.79 | -1249 | |
| Constructive / fallback@5 / deep | 187 | -543 | -2.90 ±1.07 | -573 | |
| Competitive / book+rb / round-2 | 299 | -541 | -1.81 ±0.59 | -836 | |
| Defensive / floor#200 / round-1 | 150 | -529 | -3.53 ±1.19 | -762 | |
| Defensive / floor#197 / round-1 | 121 | -518 | -4.28 ±1.20 | -587 | |
| Constructive / fallback@4 / deep | 201 | -510 | -2.54 ±1.01 | -551 | |
| Defensive / floor#30 / round-2 | 165 | -481 | -2.92 ±0.92 | -803 | |
| Defensive / floor#46 / round-2 | 161 | -450 | -2.80 ±0.82 | -711 | |
| Defensive / floor#243 / balancing | 121 | -433 | -3.58 ±1.22 | -691 | |
| Competitive / floor#243 / balancing | 92 | -422 | -4.59 ±1.19 | -693 | |
| Competitive / fallback@2 / round-2 | 128 | -421 | -3.29 ±1.27 | -395 | |
| Defensive / floor#35 / balancing | 244 | -414 | -1.70 ±0.65 | -903 | |
| Defensive / floor#51 / round-1 | 132 | -407 | -3.08 ±1.25 | -410 | |
| Defensive / floor#50 / round-2 | 351 | -382 | -1.09 ±0.58 | -767 | |
| Competitive / floor#243 / round-2 | 87 | -377 | -4.33 ±1.18 | -575 | |
| Defensive / floor#243 / round-2 | 79 | -377 | -4.77 ±1.28 | -659 | |
| Defensive / floor#132 / balancing | 155 | -369 | -2.38 ±0.72 | -955 | |
| Constructive / floor#46 / deep | 246 | -335 | -1.36 ±0.55 | -673 | |
| Constructive / floor#32 / round-1 | 93 | -329 | -3.54 ±1.33 | -288 | |
| Defensive / floor#21 / round-1 | 83 | -327 | -3.94 ±1.30 | -558 | |
| Competitive / floor#1 / round-2 | 181 | -325 | -1.80 ±1.09 | -444 | |
| Constructive / floor#17 / round-1 | 79 | -325 | -4.11 ±1.30 | -258 | |
| Competitive / floor#242 / deep | 42 | -315 | -7.50 ±1.80 | -482 | |
| Defensive / floor#3 / deep | 87 | -306 | -3.52 ±1.31 | -386 | |
| Defensive / floor#64 / round-2 | 158 | -306 | -1.94 ±1.04 | -499 | |
| Competitive / floor#6 / round-2 | 91 | -301 | -3.31 ±1.43 | -600 | |
| Competitive / floor#16 / round-1 | 45 | -297 | -6.60 ±2.33 | -285 | |
| Defensive / floor#131 / round-1 | 83 | -288 | -3.47 ±1.41 | -444 | |
| Constructive / floor#61 / round-2 | 128 | -276 | -2.16 ±1.03 | -400 | |
| Defensive / floor#129 / round-2 | 82 | -274 | -3.34 ±1.45 | -398 | |
| Defensive / floor#199 / round-1 | 125 | -274 | -2.19 ±1.17 | -328 | |
| Defensive / floor#66 / round-1 | 112 | -274 | -2.45 ±1.32 | -363 | |
| Competitive / floor#240 / round-2 | 82 | -270 | -3.29 ±1.41 | -342 | |
| Competitive / floor#237 / round-2 | 81 | -260 | -3.21 ±1.09 | -436 | |
| Competitive / floor#16 / round-2 | 40 | -256 | -6.40 ±2.15 | -299 | |
| Competitive / floor#5 / round-2 | 97 | -237 | -2.44 ±1.53 | -371 | |
| Defensive / floor#198 / round-2 | 61 | -237 | -3.89 ±1.69 | -352 | |
| Competitive / floor#61 / round-2 | 37 | -233 | -6.30 ±2.02 | -300 | |
| Defensive / floor#30 / round-1 | 56 | -229 | -4.09 ±1.76 | -225 | |
| Defensive / floor#50 / balancing | 184 | -223 | -1.21 ±0.67 | -743 | |
| Competitive / floor#240 / balancing | 76 | -222 | -2.92 ±1.51 | -265 | |
| Defensive / floor#36 / round-1 | 64 | -217 | -3.39 ±1.48 | -398 | |
| Competitive / floor#239 / round-2 | 47 | -206 | -4.38 ±1.80 | -289 | |
| Defensive / floor#31 / round-2 | 63 | -196 | -3.11 ±1.67 | -240 | |
| Defensive / floor#65 / balancing | 133 | -185 | -1.39 ±0.89 | -546 | |
| Competitive / floor#46 / deep | 42 | -176 | -4.19 ±1.69 | -252 | |
| Competitive / floor#3 / deep | 86 | -175 | -2.03 ±1.25 | -194 | |
| Competitive / floor#241 / round-2 | 44 | -174 | -3.95 ±1.61 | -220 | |
| Defensive / floor#32 / round-1 | 23 | -170 | -7.39 ±3.06 | -139 | |
| Defensive / floor#17 / round-1 | 43 | -168 | -3.91 ±2.17 | -120 | |
| Competitive / fallback@1 / round-2 | 122 | -165 | -1.35 ±1.48 | -108 | ~noise |
| Defensive / floor#16 / round-2 | 72 | -165 | -2.29 ±1.51 | -171 | |
| Competitive / floor#3+rb / round-2 | 76 | -164 | -2.16 ±1.44 | -197 | |
| Defensive / floor#133 / round-1 | 174 | -156 | -0.90 ±1.24 | -429 | ~noise |
| Competitive / floor#234 / balancing | 55 | -155 | -2.82 ±1.76 | -216 | |
| Defensive / floor#198 / round-1 | 71 | -155 | -2.18 ±1.69 | -291 | |
| Defensive / floor#49 / round-1 | 93 | -143 | -1.54 ±1.20 | -130 | |
| Constructive / floor#140 / deep | 130 | -127 | -0.98 ±1.13 | -110 | ~noise |
| Competitive / floor#31 / balancing | 14 | -126 | -9.00 ±1.16 | -143 | |
| Competitive / floor#25 / round-2 | 54 | -123 | -2.28 ±1.46 | -136 | |
| Constructive / floor#147 / round-2 | 23 | -122 | -5.30 ±3.30 | -122 | |
| Competitive / floor#235 / round-2 | 25 | -121 | -4.84 ±2.31 | -173 | |
| Constructive / floor#151 / round-2 | 34 | -121 | -3.56 ±3.02 | -120 | |
| Competitive / floor#238 / balancing | 67 | -119 | -1.78 ±1.76 | -187 | |
| Defensive / floor#17 / round-2 | 33 | -119 | -3.61 ±2.00 | -157 | |
| Competitive / floor#236 / balancing | 64 | -117 | -1.83 ±1.44 | -133 | |
| Defensive / floor#5 / round-2 | 35 | -113 | -3.23 ±2.81 | -157 | |
| Competitive / floor#234+rb / round-2 | 11 | -111 | -10.09 ±2.54 | -129 | |
| Competitive / floor#241 / balancing | 53 | -111 | -2.09 ±1.53 | -177 | |
| Competitive / floor#30 / balancing | 48 | -111 | -2.31 ±1.29 | -239 | |
| Competitive / floor#238 / round-2 | 36 | -109 | -3.03 ±2.22 | -114 | |
| Competitive / floor#240+rb / round-2 | 31 | -106 | -3.42 ±2.88 | -119 | |
| Defensive / floor#49 / balancing | 88 | -106 | -1.20 ±1.35 | -233 | ~noise |
| Competitive / floor#55 / round-2 | 17 | -104 | -6.12 ±2.84 | -96 | |
| Constructive / floor#145 / round-2 | 47 | -103 | -2.19 ±2.96 | -107 | ~noise |
| Competitive / floor#61 / deep | 49 | -98 | -2.00 ±1.63 | -261 | |
| Defensive / floor#12 / round-2 | 27 | -95 | -3.52 ±1.97 | -121 | |
| Defensive / floor#129 / round-1 | 41 | -94 | -2.29 ±2.22 | -169 | |
| Competitive / fallback@5 / round-2 | 52 | -91 | -1.75 ±1.57 | -22 | |
| Competitive / floor#15 / balancing | 29 | -89 | -3.07 ±1.79 | -173 | |
| Competitive / floor#46+rb / deep | 20 | -89 | -4.45 ±1.00 | -173 | |
| Competitive / floor#236 / round-2 | 14 | -88 | -6.29 ±2.63 | -105 | |
| Defensive / floor#204 / round-1 | 40 | -88 | -2.20 ±2.13 | -71 | |
| Competitive / floor#15 / round-2 | 24 | -86 | -3.58 ±2.43 | -173 | |
| Constructive / floor#46 / round-2 | 140 | -85 | -0.61 ±1.05 | -147 | ~noise |
| Defensive / floor#239 / round-2 | 21 | -85 | -4.05 ±3.36 | -141 | |
| Competitive / floor#47 / round-2 | 41 | -84 | -2.05 ±2.24 | -181 | ~noise |
| Defensive / floor#6 / round-2 | 28 | -84 | -3.00 ±2.91 | -124 | |
| Defensive / floor#235 / round-2 | 25 | -81 | -3.24 ±1.87 | -141 | |
| Defensive / floor#205 / round-1 | 52 | -80 | -1.54 ±1.99 | -67 | ~noise |
| Constructive / floor#31 / deep | 14 | -78 | -5.57 ±3.12 | -77 | |
| Competitive / floor#31 / round-2 | 28 | -75 | -2.68 ±2.16 | -80 | |
| Defensive / floor#237 / round-2 | 23 | -75 | -3.26 ±2.60 | -142 | |
| Defensive / floor#47 / round-2 | 54 | -75 | -1.39 ±1.84 | -177 | ~noise |
| Competitive / floor#17 / deep | 13 | -74 | -5.69 ±3.16 | -84 | |
| Competitive / floor#10 / round-2 | 31 | -73 | -2.35 ±2.28 | -74 | |
| Competitive / floor#32 / round-1 | 12 | -72 | -6.00 ±3.70 | -59 | |
| Competitive / floor#48 / round-2 | 16 | -72 | -4.50 ±2.83 | -133 | |
| Competitive / floor#63 / round-2 | 13 | -72 | -5.54 ±2.71 | -115 | |
| Defensive / floor#1 / deep | 19 | -71 | -3.74 ±2.94 | -89 | |
| Constructive / floor#62 / round-1 | 24 | -70 | -2.92 ±2.41 | -76 | |
| Defensive / floor#204 / round-2 | 24 | -69 | -2.88 ±2.16 | -101 | |
| Defensive / floor#5 / round-1 | 15 | -69 | -4.60 ±4.17 | -62 | |
| Competitive / floor#243+rb / round-2 | 13 | -67 | -5.15 ±3.74 | -132 | |
| Competitive / floor#2 / round-2 | 75 | -66 | -0.88 ±1.86 | -211 | ~noise |
| Defensive / floor#203 / round-1 | 33 | -66 | -2.00 ±2.20 | -86 | ~noise |
| Competitive / floor#30 / deep | 20 | -65 | -3.25 ±2.06 | -98 | |
| Defensive / floor#33 / round-1 | 8 | -64 | -8.00 ±3.05 | -69 | |
| Defensive / floor#55 / round-2 | 10 | -64 | -6.40 ±2.40 | -64 | |
| Competitive / floor#237 / balancing | 17 | -63 | -3.71 ±2.19 | -148 | |
| Competitive / floor#140 / round-1 | 36 | -62 | -1.72 ±2.96 | -55 | ~noise |
| Competitive / floor#16 / deep | 16 | -62 | -3.88 ±1.34 | -99 | |
| Defensive / floor#26 / round-2 | 9 | -62 | -6.89 ±4.10 | -77 | |
| Competitive / fallback@6 / round-2 | 11 | -61 | -5.55 ±3.98 | -46 | |
| Defensive / floor#11 / round-2 | 12 | -60 | -5.00 ±1.91 | -102 | |
| Defensive / floor#242 / deep | 14 | -60 | -4.29 ±3.19 | -114 | |
| Competitive / floor#239 / balancing | 30 | -56 | -1.87 ±1.64 | -143 | |
| Constructive / floor#140 / round-1 | 4 | -55 | -13.75 ±2.45 | -55 | |
| Competitive / floor#235 / deep | 8 | -53 | -6.62 ±3.68 | -80 | |
| Defensive / floor#48 / round-2 | 11 | -52 | -4.73 ±4.49 | -78 | |
| Defensive / floor#27 / round-2 | 10 | -49 | -4.90 ±4.05 | -48 | |
| Competitive / floor#143 / round-2 | 5 | -48 | -9.60 ±3.85 | -50 | |
| Competitive / floor#241 / deep | 12 | -48 | -4.00 ±4.15 | -71 | ~noise |
| Defensive / floor#51 / balancing | 24 | -48 | -2.00 ±1.95 | -60 | |
| Competitive / floor#16 / balancing | 27 | -47 | -1.74 ±1.93 | -140 | ~noise |
| Competitive / floor#27 / round-2 | 8 | -46 | -5.75 ±5.14 | -61 | |
| Constructive / floor#16 / round-2 | 14 | -46 | -3.29 ±3.02 | -43 | |
| Competitive / floor#153 / round-1 | 3 | -45 | -15.00 ±2.99 | -45 | |
| Constructive / floor#153 / round-2 | 18 | -45 | -2.50 ±4.47 | -36 | ~noise |
| Constructive / floor#157 / round-1 | 4 | -45 | -11.25 ±1.67 | -45 | |
| Defensive / floor#21 / round-2 | 25 | -45 | -1.80 ±1.91 | -118 | ~noise |
| Competitive / floor#242+rb / balancing | 8 | -44 | -5.50 ±5.00 | -69 | |
| Defensive / floor#1 / round-2 | 128 | -44 | -0.34 ±1.43 | -193 | ~noise |
| Defensive / floor#21 / balancing | 34 | -43 | -1.26 ±1.90 | -82 | ~noise |
| Defensive / floor#46 / round-1 | 13 | -43 | -3.31 ±4.05 | -45 | ~noise |
| Competitive / floor#238+rb / round-2 | 23 | -42 | -1.83 ±3.05 | -18 | ~noise |
| Competitive / floor#33 / round-2 | 11 | -42 | -3.82 ±2.16 | -66 | |
| Competitive / floor#24 / round-2 | 8 | -41 | -5.12 ±7.64 | -47 | ~noise |
| Competitive / floor#234 / round-2 | 19 | -40 | -2.11 ±3.39 | -64 | ~noise |
| Competitive / floor#129 / round-2 | 13 | -39 | -3.00 ±4.96 | -107 | ~noise |
| Defensive / floor#36 / balancing | 21 | -39 | -1.86 ±1.91 | -92 | ~noise |
| Competitive / floor#9 / round-2 | 24 | -38 | -1.58 ±1.63 | -66 | ~noise |
| Defensive / floor#203 / round-2 | 20 | -38 | -1.90 ±3.00 | -56 | ~noise |
| Defensive / floor#240 / round-2 | 9 | -38 | -4.22 ±2.33 | -53 | |
| Defensive / floor#33 / round-2 | 10 | -38 | -3.80 ±2.93 | -21 | |
| Defensive / floor#63 / round-2 | 19 | -38 | -2.00 ±3.47 | -31 | ~noise |
| Competitive / floor#129 / deep | 5 | -37 | -7.40 ±5.67 | -42 | |
| Defensive / floor#153 / round-1 | 8 | -36 | -4.50 ±7.32 | -24 | ~noise |
| Defensive / floor#36 / round-2 | 9 | -35 | -3.89 ±2.05 | -42 | |
| Defensive / floor#54 / deep | 6 | -34 | -5.67 ±4.37 | -49 | |
| Constructive / floor#31 / round-2 | 12 | -33 | -2.75 ±3.73 | -34 | ~noise |
| Defensive / floor#61 / deep | 7 | -33 | -4.71 ±3.19 | -60 | |
| Defensive / floor#61 / round-1 | 35 | -33 | -0.94 ±2.81 | -40 | ~noise |
| Defensive / floor#64 / balancing | 109 | -33 | -0.30 ±1.14 | -132 | ~noise |
| Competitive / floor#39 / round-2 | 6 | -32 | -5.33 ±2.24 | -42 | |
| Defensive / floor#1 / round-1 | 8 | -32 | -4.00 ±7.43 | -33 | ~noise |
| Defensive / floor#239 / balancing | 14 | -32 | -2.29 ±3.56 | -42 | ~noise |
| Competitive / floor#147 / balancing | 2 | -31 | -15.50 ±0.98 | -31 | |
| Competitive / floor#31 / deep | 15 | -30 | -2.00 ±3.08 | -34 | ~noise |
| Defensive / floor#32 / round-2 | 35 | -29 | -0.83 ±1.97 | -84 | ~noise |
| Defensive / floor#35 / deep | 8 | -29 | -3.62 ±2.59 | -39 | |
| Constructive / floor#30 / round-1 | 18 | -28 | -1.56 ±2.84 | -43 | ~noise |
| Defensive / floor#29 / round-2 | 3 | -28 | -9.33 ±2.36 | -29 | |
| Defensive / floor#34 / balancing | 60 | -28 | -0.47 ±1.62 | -49 | ~noise |
| Competitive / floor#45+rb / round-2 | 4 | -27 | -6.75 ±4.83 | -52 | |
| Competitive / floor#62 / round-1 | 2 | -27 | -13.50 ±0.98 | -28 | |
| Defensive / floor#14 / round-2 | 3 | -27 | -9.00 ±2.99 | -23 | |
| Defensive / floor#228 / round-2 | 5 | -27 | -5.40 ±5.39 | -26 | |
| Competitive / floor#242+rb / deep | 8 | -26 | -3.25 ±6.58 | -59 | ~noise |
| Constructive / floor#63 / round-2 | 9 | -26 | -2.89 ±4.12 | -52 | ~noise |
| Defensive / floor#241 / deep | 6 | -26 | -4.33 ±6.97 | -26 | ~noise |
| Defensive / floor#231 / round-2 | 4 | -25 | -6.25 ±2.93 | -47 | |
| Competitive / book+rb / deep | 16 | -24 | -1.50 ±2.12 | -38 | ~noise |
| Competitive / floor#153 / round-2 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Competitive / floor#241+rb / round-2 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Competitive / floor#47 / round-1 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Competitive / floor#63 / round-1 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Constructive / floor#145 / round-1 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Constructive / floor#148 / round-2 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Defensive / floor#49 / round-2 | 37 | -24 | -0.65 ±2.18 | -63 | ~noise |
| Competitive / floor#11 / round-2 | 4 | -22 | -5.50 ±2.47 | -49 | |
| Competitive / floor#241+rb / balancing | 4 | -22 | -5.50 ±1.70 | -35 | |
| Constructive / floor#32 / deep | 30 | -22 | -0.73 ±0.73 | -24 | |
| Defensive / floor#47 / round-1 | 4 | -22 | -5.50 ±6.22 | -22 | ~noise |
| Competitive / floor#235+rb / balancing | 2 | -21 | -10.50 ±2.94 | -21 | |
| Competitive / floor#60+rb / round-2 | 2 | -21 | -10.50 ±2.94 | -17 | |
| Constructive / floor#154 / deep | 7 | -21 | -3.00 ±4.59 | -21 | ~noise |
| Defensive / floor#20 / deep | 10 | -21 | -2.10 ±1.22 | -36 | |
| Competitive / fallback@2 / balancing | 2 | -20 | -10.00 ±1.96 | -20 | |
| Competitive / floor#45 / balancing | 5 | -20 | -4.00 ±9.99 | -10 | ~noise |
| Defensive / floor#147 / round-1 | 6 | -20 | -3.33 ±8.31 | -20 | ~noise |
| Defensive / floor#17 / deep | 2 | -20 | -10.00 ±3.92 | -23 | |
| Defensive / floor#197 / round-2 | 40 | -20 | -0.50 ±2.42 | -59 | ~noise |
| Defensive / floor#65 / round-2 | 147 | -20 | -0.14 ±0.85 | -158 | ~noise |
| Competitive / floor#147 / round-1 | 2 | -19 | -9.50 ±2.94 | -14 | |
| Competitive / floor#235 / balancing | 6 | -19 | -3.17 ±4.92 | -23 | ~noise |
| Competitive / floor#39 / deep | 9 | -19 | -2.11 ±1.20 | -44 | |
| Defensive / floor#227 / round-2 | 2 | -18 | -9.00 ±3.92 | -18 | |
| Defensive / floor#32 / deep | 7 | -18 | -2.57 ±1.41 | -38 | |
| Competitive / floor#32 / round-2 | 19 | -17 | -0.89 ±3.15 | -40 | ~noise |
| Defensive / floor#10 / round-2 | 1 | -17 | -17.00 ±0.00 | -17 | ~noise |
| Defensive / floor#236 / round-2 | 2 | -17 | -8.50 ±2.94 | -17 | |
| Defensive / floor#40 / round-2 | 4 | -17 | -4.25 ±4.69 | -28 | ~noise |
| Defensive / floor#54 / round-2 | 4 | -17 | -4.25 ±2.17 | -40 | |
| Constructive / floor#47 / deep | 4 | -16 | -4.00 ±1.79 | -26 | |
| Defensive / floor#18 / round-2 | 9 | -16 | -1.78 ±5.84 | -11 | ~noise |
| Defensive / floor#241 / round-2 | 2 | -16 | -8.00 ±3.92 | -16 | |
| Defensive / floor#29 / round-1 | 6 | -16 | -2.67 ±6.82 | -9 | ~noise |
| Competitive / floor#18 / round-2 | 14 | -15 | -1.07 ±3.12 | +7 | ~noise plain/PD-flip |
| Competitive / floor#46 / balancing | 4 | -15 | -3.75 ±1.86 | -39 | |
| Defensive / floor#66 / balancing | 3 | -15 | -5.00 ±5.19 | -17 | ~noise |
| Competitive / fallback@3+rb / round-2 | 2 | -14 | -7.00 ±0.00 | -14 | |
| Competitive / floor#135 / round-2 | 2 | -14 | -7.00 ±3.92 | -26 | |
| Competitive / floor#140 / deep | 2 | -14 | -7.00 ±1.96 | -22 | |
| Competitive / floor#33 / round-1 | 2 | -14 | -7.00 ±3.92 | -26 | |
| Competitive / floor#60 / round-2 | 2 | -14 | -7.00 ±3.92 | -26 | |
| Constructive / floor#17 / deep | 17 | -14 | -0.82 ±0.88 | -19 | ~noise |
| Defensive / floor#11 / deep | 2 | -14 | -7.00 ±13.72 | -14 | ~noise |
| Defensive / floor#56 / deep | 2 | -14 | -7.00 ±3.92 | -29 | |
| Defensive / floor#61 / balancing | 14 | -14 | -1.00 ±3.02 | -29 | ~noise |
| Competitive / floor#140+rb / round-2 | 2 | -13 | -6.50 ±8.82 | -13 | ~noise |
| Competitive / floor#145 / round-1 | 1 | -13 | -13.00 ±0.00 | -13 | ~noise |
| Competitive / floor#17 / round-1 | 6 | -13 | -2.17 ±6.85 | +5 | ~noise plain/PD-flip |
| Competitive / floor#48 / round-1 | 1 | -13 | -13.00 ±0.00 | -13 | ~noise |
| Competitive / floor#57 / round-2 | 1 | -13 | -13.00 ±0.00 | -14 | ~noise |
| Defensive / floor#26 / deep | 1 | -13 | -13.00 ±0.00 | -14 | ~noise |
| Defensive / floor#39 / round-2 | 3 | -13 | -4.33 ±3.46 | -36 | |
| Defensive / floor#56 / round-1 | 10 | -13 | -1.30 ±3.05 | -18 | ~noise |
| Competitive / floor#47+rb / balancing | 2 | -12 | -6.00 ±1.96 | -12 | |
| Constructive / floor#145 / deep | 7 | -12 | -1.71 ±8.02 | -12 | ~noise |
| Competitive / floor#45+rb / deep | 2 | -11 | -5.50 ±2.94 | -21 | |
| Defensive / floor#230 / round-2 | 2 | -11 | -5.50 ±0.98 | -14 | |
| Defensive / floor#62 / round-2 | 49 | -11 | -0.22 ±1.99 | -92 | ~noise |
| Defensive / floor#63 / deep | 2 | -11 | -5.50 ±2.94 | -14 | |
| Competitive / floor#237 / deep | 7 | -10 | -1.43 ±2.30 | -46 | ~noise |
| Competitive / floor#40 / round-2 | 20 | -10 | -0.50 ±3.31 | -22 | ~noise |
| Defensive / floor#238 / round-2 | 1 | -10 | -10.00 ±0.00 | -10 | ~noise |
| Defensive / floor#241 / balancing | 15 | -10 | -0.67 ±2.72 | -34 | ~noise |
| Defensive / floor#28 / round-2 | 1 | -10 | -10.00 ±0.00 | -10 | ~noise |
| Defensive / floor#50 / deep | 7 | -10 | -1.43 ±1.59 | -13 | ~noise |
| Defensive / floor#63 / round-1 | 5 | -10 | -2.00 ±9.38 | +0 | ~noise plain/PD-flip |
| Competitive / fallback@5+rb / round-2 | 2 | -9 | -4.50 ±0.98 | -12 | |
| Competitive / floor#54 / deep | 2 | -9 | -4.50 ±2.94 | -9 | |
| Competitive / floor#62 / deep | 7 | -9 | -1.29 ±4.83 | -19 | ~noise |
| Defensive / floor#205 / round-2 | 38 | -9 | -0.24 ±2.24 | -66 | ~noise |
| Defensive / floor#60 / deep | 6 | -9 | -1.50 ±1.58 | -26 | ~noise |
| Competitive / floor#236+rb / round-2 | 12 | -8 | -0.67 ±4.46 | -12 | ~noise |
| Competitive / floor#30+rb / round-2 | 4 | -8 | -2.00 ±2.26 | -40 | ~noise |
| Competitive / floor#56 / deep | 2 | -8 | -4.00 ±1.96 | -21 | |
| Competitive / floor#62 / balancing | 3 | -8 | -2.67 ±5.23 | +17 | ~noise plain/PD-flip |
| Defensive / floor#237 / deep | 2 | -8 | -4.00 ±3.92 | -22 | |
| Competitive / floor#151 / round-2 | 10 | -7 | -0.70 ±6.36 | -16 | ~noise |
| Competitive / floor#235+rb / deep | 5 | -7 | -1.40 ±2.02 | -40 | ~noise |
| Defensive / floor#18 / deep | 4 | -7 | -1.75 ±5.08 | -16 | ~noise |
| Defensive / floor#218 / round-2 | 3 | -7 | -2.33 ±0.65 | -17 | |
| Competitive / floor#151 / round-1 | 8 | -6 | -0.75 ±4.09 | +4 | ~noise plain/PD-flip |
| Competitive / floor#239+rb / balancing | 2 | -5 | -2.50 ±4.90 | -26 | ~noise |
| Competitive / floor#61+rb / deep | 8 | -5 | -0.62 ±0.82 | -12 | ~noise |
| Defensive / floor#219 / round-2 | 1 | -5 | -5.00 ±0.00 | -6 | ~noise |
| Defensive / floor#46 / deep | 2 | -5 | -2.50 ±0.98 | -12 | |
| Competitive / floor#140 / round-2 | 39 | -4 | -0.10 ±2.11 | -34 | ~noise |
| Competitive / floor#18 / round-1 | 11 | -4 | -0.36 ±5.22 | +18 | ~noise plain/PD-flip |
| Competitive / floor#243 / round-1 | 5 | -4 | -0.80 ±2.93 | -9 | ~noise |
| Competitive / floor#47 / balancing | 19 | -4 | -0.21 ±3.48 | -17 | ~noise |
| Competitive / floor#60 / balancing | 10 | -4 | -0.40 ±2.63 | -41 | ~noise |
| Defensive / floor#140 / balancing | 2 | -4 | -2.00 ±0.00 | -4 | |
| Competitive / floor#24 / deep | 2 | -3 | -1.50 ±2.94 | -3 | ~noise |
| Competitive / floor#3+rb / deep | 12 | -3 | -0.25 ±3.28 | -60 | ~noise |
| Defensive / floor#16 / deep | 2 | -3 | -1.50 ±4.90 | -7 | ~noise |
| Defensive / floor#235 / deep | 3 | -3 | -1.00 ±1.96 | -17 | ~noise |
| Defensive / floor#240 / deep | 1 | -3 | -3.00 ±0.00 | -9 | ~noise |
| Competitive / floor#54 / round-2 | 6 | -2 | -0.33 ±6.59 | +4 | ~noise plain/PD-flip |
| Defensive / floor#237 / balancing | 3 | -2 | -0.67 ±2.36 | -16 | ~noise |
| Defensive / floor#42 / round-2 | 17 | -2 | -0.12 ±5.33 | +14 | ~noise plain/PD-flip |
| Competitive / floor#145 / round-2 | 6 | -1 | -0.17 ±6.51 | -8 | ~noise |
| Competitive / floor#46+rb / round-2 | 4 | -1 | -0.25 ±6.85 | +21 | ~noise plain/PD-flip |
| Defensive / floor#16 / round-1 | 2 | -1 | -0.50 ±4.90 | -2 | ~noise |
| Defensive / floor#238 / deep | 2 | -1 | -0.50 ±2.94 | -12 | ~noise |
| Defensive / floor#41 / deep | 1 | -1 | -1.00 ±0.00 | -1 | ~noise |
| Defensive / floor#62 / deep | 3 | -1 | -0.33 ±10.14 | -3 | ~noise |
| Competitive / floor#12 / deep | 4 | +0 | +0.00 ±6.45 | +0 | ~noise |
| Competitive / floor#143 / balancing | 2 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Competitive / floor#151 / balancing | 2 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Competitive / floor#32 / deep | 3 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Competitive / floor#41 / deep | 1 | +0 | +0.00 ±0.00 | +7 | ~noise |
| Competitive / floor#61 / balancing | 1 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Constructive / floor#151 / deep | 4 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Constructive / floor#47 / round-1 | 10 | +0 | +0.00 ±4.98 | +0 | ~noise |
| Defensive / floor#226 / round-2 | 1 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Competitive / floor#14 / round-2 | 1 | +1 | +1.00 ±0.00 | +1 | ~noise |
| Competitive / floor#33 / balancing | 1 | +1 | +1.00 ±0.00 | -2 | ~noise plain/PD-flip |
| Competitive / floor#60+rb / deep | 2 | +1 | +0.50 ±0.98 | -3 | ~noise plain/PD-flip |
| Defensive / floor#14 / round-1 | 5 | +1 | +0.20 ±4.57 | +7 | ~noise |
| Defensive / floor#151 / round-1 | 4 | +1 | +0.25 ±6.52 | -2 | ~noise plain/PD-flip |
| Competitive / floor#42 / round-2 | 8 | +2 | +0.25 ±6.85 | -25 | ~noise plain/PD-flip |
| Constructive / floor#148 / deep | 2 | +2 | +1.00 ±0.00 | +2 | |
| Defensive / floor#18 / round-1 | 4 | +2 | +0.50 ±8.64 | +11 | ~noise |
| Defensive / floor#31 / deep | 2 | +2 | +1.00 ±1.96 | -3 | ~noise plain/PD-flip |
| Competitive / floor#12 / round-2 | 14 | +3 | +0.21 ±2.73 | -8 | ~noise plain/PD-flip |
| Defensive / floor#31 / round-1 | 5 | +3 | +0.60 ±5.80 | +0 | ~noise |
| Competitive / floor#147 / deep | 2 | +4 | +2.00 ±0.00 | +4 | |
| Competitive / floor#18 / deep | 3 | +4 | +1.33 ±3.64 | +9 | ~noise |
| Defensive / floor#145 / round-1 | 5 | +4 | +0.80 ±10.67 | +12 | ~noise |
| Competitive / floor#5 / deep | 6 | +5 | +0.83 ±4.97 | +2 | ~noise |
| Competitive / floor#62+rb / round-2 | 2 | +5 | +2.50 ±20.58 | +12 | ~noise |
| Competitive / floor#40 / deep | 1 | +6 | +6.00 ±0.00 | +8 | ~noise |
| Competitive / floor#32 / balancing | 1 | +7 | +7.00 ±0.00 | +7 | ~noise |
| Constructive / floor#157 / deep | 8 | +7 | +0.88 ±0.78 | +14 | |
| Defensive / floor#42 / round-1 | 1 | +7 | +7.00 ±0.00 | +10 | ~noise |
| Defensive / floor#0 / round-1 | 2 | +8 | +4.00 ±1.96 | +8 | |
| Competitive / floor#237+rb / balancing | 2 | +9 | +4.50 ±0.98 | +12 | |
| Defensive / floor#34 / round-1 | 1 | +9 | +9.00 ±0.00 | +9 | ~noise |
| Defensive / floor#38 / round-2 | 2 | +9 | +4.50 ±0.98 | +12 | |
| Defensive / floor#39 / deep | 2 | +9 | +4.50 ±0.98 | -12 | plain/PD-flip |
| Defensive / floor#57 / round-2 | 1 | +9 | +9.00 ±0.00 | +9 | ~noise |
| Competitive / floor#60 / deep | 1 | +10 | +10.00 ±0.00 | +10 | ~noise |
| Constructive / floor#147 / deep | 8 | +10 | +1.25 ±6.38 | +24 | ~noise |
| Constructive / floor#48 / round-2 | 1 | +10 | +10.00 ±0.00 | +10 | ~noise |
| Defensive / floor#56 / round-2 | 14 | +10 | +0.71 ±4.29 | +18 | ~noise |
| Defensive / floor#57 / deep | 1 | +10 | +10.00 ±0.00 | +4 | ~noise |
| Constructive / floor#48 / deep | 4 | +11 | +2.75 ±6.17 | +4 | ~noise |
| Competitive / floor#239+rb / deep | 1 | +12 | +12.00 ±0.00 | +10 | ~noise |
| Defensive / floor#13 / round-2 | 1 | +12 | +12.00 ±0.00 | +11 | ~noise |
| Defensive / floor#235 / balancing | 5 | +12 | +2.40 ±6.31 | +1 | ~noise |
| Competitive / floor#15 / deep | 2 | +13 | +6.50 ±0.98 | +16 | |
| Defensive / floor#48 / round-1 | 5 | +13 | +2.60 ±8.82 | +13 | ~noise |
| Defensive / floor#62 / round-1 | 5 | +13 | +2.60 ±8.51 | +13 | ~noise |
| Competitive / floor#47 / deep | 12 | +15 | +1.25 ±3.53 | -20 | ~noise plain/PD-flip |
| Competitive / floor#140 / balancing | 2 | +16 | +8.00 ±3.92 | +16 | |
| Constructive / floor#153 / deep | 2 | +16 | +8.00 ±3.92 | +16 | |
| Defensive / floor#145 / balancing | 2 | +16 | +8.00 ±3.92 | +16 | |
| Defensive / floor#41 / round-1 | 5 | +16 | +3.20 ±1.57 | +47 | |
| Competitive / floor#144 / round-2 | 4 | +18 | +4.50 ±7.40 | +20 | ~noise |
| Defensive / floor#47 / deep | 3 | +18 | +6.00 ±4.53 | +21 | |
| Competitive / floor#0 / deep | 2 | +20 | +10.00 ±3.92 | +20 | |
| Constructive / floor#62 / deep | 10 | +20 | +2.00 ±4.34 | +8 | ~noise |
| Competitive / floor#62 / round-2 | 29 | +23 | +0.79 ±2.47 | -154 | ~noise plain/PD-flip |
| Defensive / floor#2 / round-1 | 3 | +23 | +7.67 ±5.23 | +23 | |
| Competitive / floor#239 / deep | 10 | +24 | +2.40 ±3.69 | -17 | ~noise plain/PD-flip |
| Competitive / floor#41 / round-2 | 2 | +24 | +12.00 ±1.96 | +24 | |
| Constructive / floor#157 / round-2 | 12 | +24 | +2.00 ±4.89 | +24 | ~noise |
| Defensive / floor#40 / deep | 2 | +24 | +12.00 ±1.96 | +24 | |
| Defensive / floor#199 / round-2 | 36 | +27 | +0.75 ±1.89 | -38 | ~noise plain/PD-flip |
| Defensive / floor#129 / deep | 7 | +28 | +4.00 ±6.49 | +25 | ~noise |
| Defensive / floor#207 / round-2 | 4 | +29 | +7.25 ±5.45 | +39 | |
| Competitive / floor#47+rb / round-2 | 2 | +30 | +15.00 ±1.96 | +30 | |
| Defensive / floor#210 / round-2 | 4 | +35 | +8.75 ±3.78 | +38 | |
| Competitive / floor#145 / balancing | 4 | +40 | +10.00 ±2.89 | +40 | |
| Defensive / floor#6 / round-1 | 13 | +42 | +3.23 ±4.05 | +49 | ~noise |
| Defensive / floor#140 / round-1 | 12 | +43 | +3.58 ±6.80 | +48 | ~noise |
| Competitive / floor#147 / round-2 | 9 | +51 | +5.67 ±6.69 | +53 | ~noise |
| Defensive / floor#133 / balancing | 45 | +51 | +1.13 ±2.27 | +3 | ~noise |
| Competitive / floor#1 / deep | 73 | +55 | +0.75 ±1.52 | -35 | ~noise plain/PD-flip |
| Defensive / floor#41 / round-2 | 27 | +55 | +2.04 ±2.76 | +48 | ~noise |
| Defensive / floor#0 / round-2 | 18 | +139 | +7.72 ±1.49 | +119 | |
| Competitive / floor#0 / round-2 | 35 | +143 | +4.09 ±1.87 | +125 | |

## By phase

  -165851 IMPs   72368 boards  Constructive
  -147270 IMPs   56432 boards  Defensive
   -91924 IMPs   28406 boards  Competitive

## By provenance

  -226235 IMPs   97558 boards  book
   -43455 IMPs   13924 boards  floor#3
   -20710 IMPs    6336 boards  floor#242
   -20019 IMPs    5963 boards  fallback@2
   -19735 IMPs    6428 boards  fallback@1
   -14450 IMPs    4672 boards  fallback@3
    -5967 IMPs    1815 boards  fallback@4
    -3680 IMPs    1668 boards  floor#20
    -3512 IMPs    1207 boards  floor#61
    -3433 IMPs    1136 boards  floor#46
    -3222 IMPs    1531 boards  floor#35
    -3138 IMPs    1322 boards  floor#140
    -2843 IMPs     832 boards  floor#60
    -2712 IMPs     409 boards  floor#242+rb
    -2386 IMPs    1378 boards  floor#50
    -2267 IMPs     503 boards  floor#243
    -2224 IMPs     470 boards  floor#202
    -1934 IMPs     876 boards  floor#64
    -1769 IMPs     567 boards  floor#30
    -1506 IMPs     403 boards  floor#45
    -1444 IMPs     315 boards  floor#31
    -1191 IMPs     289 boards  floor#200
    -1147 IMPs     768 boards  floor#65
    -1098 IMPs     352 boards  floor#132
     -977 IMPs     287 boards  floor#131
     -877 IMPs     218 boards  floor#16
     -733 IMPs     193 boards  floor#17
     -650 IMPs     223 boards  floor#32
     -634 IMPs     239 boards  fallback@5
     -565 IMPs     315 boards  book+rb
     -538 IMPs     161 boards  floor#197
     -533 IMPs     168 boards  floor#240
     -455 IMPs     156 boards  floor#51
     -418 IMPs     133 boards  floor#237
     -417 IMPs     409 boards  floor#1
     -416 IMPs     148 boards  floor#129
     -415 IMPs     142 boards  floor#21
     -414 IMPs     153 boards  floor#5
     -392 IMPs     132 boards  floor#198
     -385 IMPs     132 boards  floor#241
     -355 IMPs     122 boards  floor#239
     -343 IMPs     132 boards  floor#6
     -291 IMPs      94 boards  floor#36
     -289 IMPs     115 boards  floor#66
     -273 IMPs     218 boards  floor#49
     -265 IMPs      72 boards  floor#235
     -247 IMPs     161 boards  floor#199
     -239 IMPs     106 boards  floor#238
     -222 IMPs      80 boards  floor#236
     -195 IMPs      74 boards  floor#234
     -192 IMPs     149 boards  floor#47
     -181 IMPs      50 boards  floor#63
     -168 IMPs      27 boards  floor#55
     -167 IMPs      88 boards  floor#3+rb
     -162 IMPs      55 boards  floor#15
     -157 IMPs      64 boards  floor#204
     -157 IMPs      32 boards  floor#33
     -134 IMPs      33 boards  floor#153
     -133 IMPs      62 boards  floor#151
     -127 IMPs      52 boards  floor#147
     -123 IMPs      54 boards  floor#25
     -111 IMPs      11 boards  floor#234+rb
     -106 IMPs      31 boards  floor#240+rb
     -105 IMPs     219 boards  floor#133
     -104 IMPs      53 boards  floor#203
     -103 IMPs      38 boards  floor#48
      -96 IMPs      18 boards  floor#11
      -95 IMPs      18 boards  floor#27
      -93 IMPs      74 boards  floor#145
      -92 IMPs      45 boards  floor#12
      -90 IMPs      32 boards  floor#10
      -90 IMPs      24 boards  floor#46+rb
      -89 IMPs      90 boards  floor#205
      -75 IMPs      10 boards  floor#26
      -70 IMPs     132 boards  floor#62
      -67 IMPs      13 boards  floor#243+rb
      -62 IMPs      18 boards  floor#54
      -61 IMPs      11 boards  fallback@6
      -55 IMPs      20 boards  floor#39
      -48 IMPs       7 boards  floor#143
      -46 IMPs       6 boards  floor#241+rb
      -44 IMPs      10 boards  floor#24
      -44 IMPs       9 boards  floor#29
      -43 IMPs      78 boards  floor#2
      -42 IMPs      23 boards  floor#238+rb
      -38 IMPs       6 boards  floor#45+rb
      -38 IMPs      24 boards  floor#9
      -36 IMPs      45 boards  floor#18
      -28 IMPs       7 boards  floor#235+rb
      -27 IMPs       5 boards  floor#228
      -25 IMPs       9 boards  floor#14
      -25 IMPs       4 boards  floor#231
      -25 IMPs      28 boards  floor#56
      -22 IMPs       4 boards  floor#148
      -21 IMPs       7 boards  floor#154
      -20 IMPs       4 boards  floor#60+rb
      -19 IMPs      61 boards  floor#34
      -18 IMPs       2 boards  floor#227
      -14 IMPs       2 boards  fallback@3+rb
      -14 IMPs       2 boards  floor#135
      -14 IMPs      24 boards  floor#157
      -13 IMPs       2 boards  floor#140+rb
      -11 IMPs       2 boards  floor#230
      -10 IMPs       1 boards  floor#28
       -9 IMPs       2 boards  fallback@5+rb
       -8 IMPs      12 boards  floor#236+rb
       -8 IMPs       4 boards  floor#30+rb
       -7 IMPs       3 boards  floor#218
       -5 IMPs       1 boards  floor#219
       -5 IMPs       8 boards  floor#61+rb
       +0 IMPs       1 boards  floor#226
       +3 IMPs      27 boards  floor#40
       +5 IMPs       2 boards  floor#62+rb
       +6 IMPs       3 boards  floor#57
       +7 IMPs       3 boards  floor#239+rb
       +7 IMPs      26 boards  floor#42
       +9 IMPs       2 boards  floor#237+rb
       +9 IMPs       2 boards  floor#38
      +12 IMPs       1 boards  floor#13
      +18 IMPs       4 boards  floor#144
      +18 IMPs       4 boards  floor#47+rb
      +29 IMPs       4 boards  floor#207
      +35 IMPs       4 boards  floor#210
      +94 IMPs      36 boards  floor#41
     +310 IMPs      57 boards  floor#0

## By family

  -194033 IMPs   69647 boards  round-1
  -122233 IMPs   47521 boards  round-2
   -67689 IMPs   29142 boards  opening
   -14164 IMPs    5850 boards  balancing
    -6926 IMPs    5046 boards  deep

## By direction

  -263140 IMPs   35722 boards  other
  -124448 IMPs   18231 boards  overbid
   -90430 IMPs   10445 boards  missed-game
   -76246 IMPs   10634 boards  sold-out
   -44438 IMPs    7885 boards  wrong-strain
   -41106 IMPs    3340 boards  missed-slam
    -5894 IMPs     397 boards  missed-grand
    -5262 IMPs     731 boards  doubling
       +0 IMPs   25423 boards  flat
  +245919 IMPs   44398 boards  gain

## Worst boards per losing bucket

### Defensive / book / round-1 (28670 boards, -67707 IMPs)

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

[vul both, seed Some(1783375066), board 5738] swing -2920 pts / -21 IMPs (PD -21), diverged at call 2 (2♦ ours vs P BBA), other
  rule: 5+ ♦, and 11–17 points
  W:KT5.AQJT5.32.Q92 97.97.AQJ94.KJT5 AQ82.K86432.87.4 J643..KT65.A8763
  ours NS @ A: - 1♥ 2♦ 3♦ - 4♥ - - -  -> 4♥ by West
  ours EW @ B: - 1♥ - 4♣ X - - -  -> 4♣x by East

### Constructive / book / opening (29142 boards, -67689 IMPs)

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

### Constructive / book / round-2 (21188 boards, -45181 IMPs)

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

### Constructive / book / round-1 (14207 boards, -38654 IMPs)

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

### Competitive / fallback@2 / round-1 (5833 boards, -19578 IMPs)

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

### Competitive / fallback@1 / round-1 (6306 boards, -19570 IMPs)

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

### Defensive / floor#3 / round-2 (4657 boards, -14716 IMPs)

[vul both, seed Some(1783375064), board 295] swing -2350 pts / -20 IMPs (PD -20), diverged at call 4 (P ours vs X BBA), missed-game
  rule: not ((opaque condition)), or (opaque condition)
  W:643..KJ872.KJ843 KQ87.AK962.QT6.9 T952.T3.3.AQ7652 AJ.QJ8754.A954.T
  ours NS @ A: - 1♥ - 4♣ X - - -  -> 4♣x by South
  ours EW @ B: - 1♥ - 4♣ - 4♦ - 4NT - 5♥ - - -  -> 5♥ by North

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

### Defensive / floor#3 / round-1 (3940 boards, -13468 IMPs)

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

### Competitive / fallback@3 / round-2 (3675 boards, -10946 IMPs)

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

### Competitive / floor#242 / round-2 (1588 boards, -5843 IMPs)

[vul both, seed Some(1783375064), board 1440] swing -1840 pts / -18 IMPs (PD -18), diverged at call 4 (X ours vs P BBA), other
  rule: (opaque condition), at most three cards in each of their suits, 12+ HCP, and (opaque condition)
  W:763.A74.T76.AJ43 AKJ2.JT.KQ842.T9 9.KQ853.AJ5.KQ62 QT854.962.93.875
  ours NS @ A: 1♦ 1♥ - 2♥ X XX - - -  -> 2♥xx by East
  ours EW @ B: 1♦ 1♥ - 2♥ - 3♥ - - -  -> 3♥ by East

[vul both, seed Some(1783375064), board 1826] swing -1800 pts / -18 IMPs (PD -18), diverged at call 4 (X ours vs P BBA), overbid
  rule: (opaque condition), at most three cards in each of their suits, 12+ HCP, and (opaque condition)
  W:QT94.K62.43.AQT6 87.QT983.J65.532 AK652.A.T87.J987 J3.J754.AKQ92.K4
  ours NS @ A: 1♦ - - 1♠ X 1NT - 3♠ X - 3NT - - X - - -  -> 3NTx by North
  ours EW @ B: 1♦ - - 1♠ - 2♥ - 2♠ - 3♠ - - -  -> 3♠ by East

[vul both, seed Some(1783375074), board 104] swing -1810 pts / -18 IMPs (PD -18), diverged at call 4 (X ours vs P BBA), other
  rule: (opaque condition), at most three cards in each of their suits, 12+ HCP, and (opaque condition)
  W:AJT6.84.A.JT9865 43.AKQ7.K652.Q32 Q987.J9.QT73.AK7 K52.T6532.J984.4
  ours NS @ A: 1♦ - - 2♣ X XX - - -  -> 2♣xx by West
  ours EW @ B: 1♦ - - 2♣ - 2♦ - 3♣ - - -  -> 3♣ by West

