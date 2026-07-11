=== arm 0: our american floor (us) vs BBA 2/1 (them), vulnerability none, 204800 boards ===
replay verification: 100.00% of 2117065 our-side calls (0 mismatched)
auction-divergent: 185738 (91%), contract-divergent: 152945 (75%)
plain DD: -1.4491 IMPs/board (95% CI [-1.4708, -1.4274]), -296771 IMPs total
perfect defense: -1.4439 IMPs/board (95% CI [-1.4696, -1.4182])
gen_args: --count 6400 --seed 1783375064 --output ab-results/anchor/2026-07-10-5b5115d/none/shard-0.json -v none

=== arm 1: our american floor (us) vs BBA 2/1 (them), vulnerability both, 204800 boards ===
replay verification: 100.00% of 2100208 our-side calls (0 mismatched)
auction-divergent: 184766 (90%), contract-divergent: 151319 (74%)
plain DD: -1.9187 IMPs/board (95% CI [-1.9465, -1.8908]), -392943 IMPs total
perfect defense: -2.0867 IMPs/board (95% CI [-2.1192, -2.0542])
gen_args: --count 6400 --seed 1783375064 --output ab-results/anchor/2026-07-10-5b5115d/both/shard-0.json -v both


## IMP histogram (plain, per contract-divergent board)

right-siding-only divergences (same contract, different auction): 66240

  -24 IMPs: 1
  -22 IMPs: 4
  -21 IMPs: 26
  -20 IMPs: 44
  -19 IMPs: 159
  -18 IMPs: 300
  -17 IMPs: 975
  -16 IMPs: 1082
  -15 IMPs: 2379
  -14 IMPs: 3384
  -13 IMPs: 8975
  -12 IMPs: 7521
  -11 IMPs: 13164
  -10 IMPs: 19442
   -9 IMPs: 7632
   -8 IMPs: 6519
   -7 IMPs: 12092
   -6 IMPs: 20179
   -5 IMPs: 14931
   -4 IMPs: 7424
   -3 IMPs: 11529
   -2 IMPs: 13357
   -1 IMPs: 12167
   +0 IMPs: 52315
   +1 IMPs: 10300
   +2 IMPs: 9952
   +3 IMPs: 8247
   +4 IMPs: 6160
   +5 IMPs: 15252
   +6 IMPs: 12891
   +7 IMPs: 6002
   +8 IMPs: 2183
   +9 IMPs: 2195
  +10 IMPs: 5331
  +11 IMPs: 4180
  +12 IMPs: 2517
  +13 IMPs: 2849
  +14 IMPs: 434
  +15 IMPs: 82
  +16 IMPs: 42
  +17 IMPs: 42
  +18 IMPs: 2
  +19 IMPs: 2

## Ranked buckets (phase / provenance / family), losses first

| bucket | boards | net plain IMPs | IMPs/divergent ±CI | net PD IMPs | flag |
| --- | --- | --- | --- | --- | --- |
| Defensive / book / round-1 | 58928 | -123392 | -2.09 ±0.05 | -141682 | |
| Constructive / book / opening | 47131 | -93067 | -1.97 ±0.06 | -86265 | |
| Constructive / book / round-2 | 41295 | -81168 | -1.97 ±0.06 | -82583 | |
| Constructive / book / round-1 | 29591 | -69526 | -2.35 ±0.08 | -73047 | |
| Defensive / floor#3 / round-2 | 9276 | -29137 | -3.14 ±0.13 | -24563 | |
| Competitive / fallback@1 / round-1 | 9512 | -26434 | -2.78 ±0.12 | -26263 | |
| Defensive / floor#3 / round-1 | 8169 | -26386 | -3.23 ±0.15 | -17505 | |
| Competitive / fallback@2 / round-1 | 8428 | -23647 | -2.81 ±0.14 | -24204 | |
| Competitive / fallback@3 / round-2 | 6017 | -13414 | -2.23 ±0.15 | -13954 | |
| Competitive / floor#3 / round-2 | 4149 | -11540 | -2.78 ±0.20 | -8902 | |
| Defensive / book / round-2 | 4504 | -8243 | -1.83 ±0.19 | -9038 | |
| Competitive / fallback@4 / round-2 | 2560 | -7827 | -3.06 ±0.25 | -8964 | |
| Constructive / floor#3 / round-2 | 2797 | -6185 | -2.21 ±0.22 | -5253 | |
| Competitive / fallback@3 / round-1 | 1838 | -6176 | -3.36 ±0.31 | -6764 | |
| Constructive / floor#140 / round-2 | 2084 | -5566 | -2.67 ±0.34 | -5826 | |
| Competitive / floor#245 / round-2 | 1804 | -5507 | -3.05 ±0.29 | -8268 | |
| Defensive / floor#3 / balancing | 2417 | -5268 | -2.18 ±0.20 | -1835 | |
| Competitive / floor#3 / round-1 | 1188 | -4932 | -4.15 ±0.40 | -1742 | |
| Constructive / floor#3 / round-1 | 1346 | -4858 | -3.61 ±0.38 | -2385 | |
| Defensive / floor#60 / round-2 | 1752 | -4666 | -2.66 ±0.27 | -6533 | |
| Defensive / floor#245 / round-1 | 1669 | -4329 | -2.59 ±0.35 | -4885 | |
| Constructive / book / deep | 4805 | -4315 | -0.90 ±0.18 | -4764 | |
| Defensive / floor#20 / round-1 | 1442 | -4063 | -2.82 ±0.33 | -4908 | |
| Defensive / floor#45 / round-2 | 1453 | -3968 | -2.73 ±0.30 | -5103 | |
| Competitive / fallback@1 / round-2 | 1239 | -3676 | -2.97 ±0.37 | -3465 | |
| Defensive / floor#50 / round-1 | 1792 | -3657 | -2.04 ±0.29 | -4613 | |
| Defensive / floor#245 / round-2 | 886 | -3424 | -3.86 ±0.44 | -5321 | |
| Defensive / floor#246 / round-1 | 718 | -3390 | -4.72 ±0.57 | -3570 | |
| Competitive / fallback@2 / round-2 | 1134 | -3296 | -2.91 ±0.40 | -2971 | |
| Defensive / floor#35 / round-1 | 1455 | -3257 | -2.24 ±0.33 | -4303 | |
| Competitive / floor#3 / balancing | 1548 | -3035 | -1.96 ±0.27 | +587 | plain/PD-flip |
| Defensive / floor#64 / round-1 | 1164 | -2651 | -2.28 ±0.40 | -2555 | |
| Constructive / floor#61 / deep | 1245 | -2475 | -1.99 ±0.17 | -4026 | |
| Defensive / floor#45 / round-1 | 909 | -2376 | -2.61 ±0.41 | -2660 | |
| Competitive / floor#46 / round-2 | 779 | -2264 | -2.91 ±0.42 | -2829 | |
| Defensive / floor#60 / round-1 | 658 | -2197 | -3.34 ±0.46 | -2709 | |
| Defensive / floor#20 / round-2 | 1192 | -2160 | -1.81 ±0.32 | -3152 | |
| Defensive / floor#35 / round-2 | 1096 | -2151 | -1.96 ±0.36 | -3256 | |
| Constructive / floor#3 / deep | 1334 | -2033 | -1.52 ±0.36 | -1633 | |
| Competitive / floor#46 / round-1 | 317 | -1813 | -5.72 ±0.77 | -1682 | |
| Competitive / floor#61 / round-1 | 236 | -1769 | -7.50 ±0.74 | -1680 | |
| Competitive / floor#31 / round-1 | 376 | -1746 | -4.64 ±0.70 | -1270 | |
| Defensive / floor#245 / balancing | 1458 | -1732 | -1.19 ±0.33 | -4005 | |
| Defensive / floor#132 / round-1 | 454 | -1636 | -3.60 ±0.50 | -2976 | |
| Competitive / floor#245+rb / round-2 | 250 | -1576 | -6.30 ±0.89 | -1684 | |
| Defensive / floor#61 / round-2 | 515 | -1539 | -2.99 ±0.50 | -1777 | |
| Defensive / floor#202 / round-1 | 429 | -1528 | -3.56 ±0.61 | -1721 | |
| Defensive / floor#202 / round-2 | 357 | -1504 | -4.21 ±0.63 | -2126 | |
| Competitive / floor#30 / round-2 | 508 | -1371 | -2.70 ±0.47 | -1862 | |
| Competitive / floor#245 / balancing | 692 | -1296 | -1.87 ±0.48 | -2699 | |
| Competitive / floor#246 / round-2 | 482 | -1284 | -2.66 ±0.55 | -1984 | |
| Defensive / floor#246 / balancing | 467 | -1273 | -2.73 ±0.65 | -1804 | |
| Constructive / fallback@4 / deep | 425 | -1246 | -2.93 ±0.65 | -1394 | |
| Constructive / fallback@5 / deep | 376 | -1172 | -3.12 ±0.75 | -1269 | |
| Constructive / floor#61 / round-2 | 425 | -1152 | -2.71 ±0.61 | -1312 | |
| Defensive / floor#65 / round-1 | 911 | -1112 | -1.22 ±0.39 | -1814 | |
| Competitive / floor#6 / round-2 | 350 | -1077 | -3.08 ±0.61 | -2088 | |
| Defensive / floor#20 / balancing | 755 | -1032 | -1.37 ±0.37 | -1954 | |
| Constructive / floor#46 / round-2 | 528 | -994 | -1.88 ±0.56 | -1103 | |
| Defensive / floor#30 / round-2 | 399 | -989 | -2.48 ±0.53 | -1592 | |
| Defensive / floor#46 / round-2 | 426 | -985 | -2.31 ±0.56 | -1156 | |
| Constructive / floor#46 / deep | 625 | -983 | -1.57 ±0.37 | -1817 | |
| Competitive / floor#16 / round-1 | 136 | -978 | -7.19 ±1.10 | -998 | |
| Defensive / floor#131 / balancing | 411 | -887 | -2.16 ±0.52 | -1422 | |
| Defensive / floor#246 / round-2 | 274 | -871 | -3.18 ±0.79 | -1576 | |
| Defensive / floor#200 / round-1 | 240 | -817 | -3.40 ±0.87 | -945 | |
| Constructive / floor#32 / round-1 | 182 | -805 | -4.42 ±0.89 | -763 | |
| Constructive / floor#145 / round-2 | 162 | -803 | -4.96 ±1.28 | -814 | |
| Defensive / floor#3 / deep | 245 | -770 | -3.14 ±0.79 | -590 | |
| Competitive / floor#61 / round-2 | 229 | -758 | -3.31 ±0.81 | -962 | |
| Defensive / floor#132 / balancing | 297 | -744 | -2.51 ±0.51 | -1808 | |
| Defensive / floor#30 / round-1 | 240 | -739 | -3.08 ±0.74 | -823 | |
| Competitive / floor#237 / round-2 | 212 | -720 | -3.40 ±0.78 | -1089 | |
| Competitive / floor#246 / balancing | 326 | -710 | -2.18 ±0.72 | -1491 | |
| Competitive / floor#5 / round-2 | 350 | -689 | -1.97 ±0.70 | -1424 | |
| Defensive / floor#50 / round-2 | 660 | -687 | -1.04 ±0.43 | -1313 | |
| Defensive / floor#35 / balancing | 516 | -670 | -1.30 ±0.44 | -1389 | |
| Competitive / book+rb / round-2 | 393 | -634 | -1.61 ±0.52 | -1003 | |
| Constructive / floor#140 / deep | 486 | -633 | -1.30 ±0.62 | -701 | |
| Defensive / floor#16 / round-2 | 156 | -618 | -3.96 ±0.94 | -684 | |
| Competitive / floor#240 / balancing | 199 | -606 | -3.05 ±0.93 | -511 | |
| Defensive / floor#197 / round-1 | 179 | -605 | -3.38 ±0.93 | -652 | |
| Competitive / floor#240 / round-2 | 205 | -557 | -2.72 ±0.95 | -516 | |
| Defensive / floor#31 / round-2 | 226 | -550 | -2.43 ±0.81 | -494 | |
| Competitive / floor#239 / round-2 | 132 | -534 | -4.05 ±1.03 | -664 | |
| Defensive / floor#66 / round-1 | 179 | -528 | -2.95 ±1.04 | -648 | |
| Defensive / floor#64 / round-2 | 305 | -524 | -1.72 ±0.69 | -794 | |
| Defensive / floor#51 / round-1 | 242 | -520 | -2.15 ±0.90 | -370 | |
| Constructive / floor#17 / round-1 | 137 | -509 | -3.72 ±1.10 | -403 | |
| Defensive / floor#63 / round-2 | 89 | -501 | -5.63 ±1.38 | -431 | |
| Defensive / floor#199 / round-1 | 215 | -496 | -2.31 ±0.77 | -518 | |
| Defensive / floor#21 / round-1 | 182 | -488 | -2.68 ±0.98 | -813 | |
| Defensive / floor#48 / round-2 | 85 | -466 | -5.48 ±1.37 | -525 | |
| Defensive / floor#131 / round-1 | 138 | -465 | -3.37 ±1.09 | -721 | |
| Competitive / floor#2 / round-2 | 323 | -462 | -1.43 ±0.68 | -1245 | |
| Competitive / floor#3+rb / round-2 | 222 | -401 | -1.81 ±0.75 | -204 | |
| Defensive / floor#200 / round-2 | 210 | -391 | -1.86 ±0.91 | -583 | |
| Competitive / floor#245 / round-1 | 241 | -386 | -1.60 ±0.77 | -890 | |
| Constructive / floor#16 / round-2 | 68 | -368 | -5.41 ±1.46 | -347 | |
| Defensive / floor#50 / balancing | 403 | -359 | -0.89 ±0.44 | -1090 | |
| Defensive / floor#49 / round-1 | 188 | -354 | -1.88 ±0.90 | -200 | |
| Competitive / floor#10 / round-2 | 132 | -346 | -2.62 ±0.91 | -308 | |
| Competitive / floor#25 / round-2 | 207 | -342 | -1.65 ±0.74 | -359 | |
| Competitive / floor#60 / round-2 | 142 | -341 | -2.40 ±1.06 | -298 | |
| Defensive / floor#49 / balancing | 193 | -341 | -1.77 ±0.91 | -495 | |
| Competitive / floor#241 / round-2 | 85 | -340 | -4.00 ±1.25 | -439 | |
| Defensive / floor#61 / round-1 | 98 | -339 | -3.46 ±1.59 | -321 | |
| Competitive / floor#30 / balancing | 127 | -337 | -2.65 ±0.87 | -668 | |
| Defensive / floor#198 / round-2 | 102 | -334 | -3.27 ±1.30 | -471 | |
| Defensive / floor#65 / balancing | 242 | -327 | -1.35 ±0.58 | -1026 | |
| Competitive / floor#31 / round-2 | 79 | -322 | -4.08 ±1.34 | -338 | |
| Defensive / floor#198 / round-1 | 152 | -322 | -2.12 ±1.13 | -430 | |
| Constructive / floor#31 / round-2 | 78 | -318 | -4.08 ±1.52 | -271 | |
| Defensive / floor#63 / round-1 | 65 | -318 | -4.89 ±1.54 | -309 | |
| Competitive / floor#241 / balancing | 109 | -310 | -2.84 ±1.13 | -481 | |
| Competitive / floor#16 / round-2 | 85 | -303 | -3.56 ±1.50 | -282 | |
| Competitive / floor#3 / deep | 313 | -293 | -0.94 ±0.65 | -121 | |
| Defensive / floor#36 / round-1 | 145 | -291 | -2.01 ±0.99 | -584 | |
| Defensive / floor#133 / round-1 | 315 | -290 | -0.92 ±0.92 | -718 | ~noise |
| Competitive / floor#235 / round-2 | 103 | -281 | -2.73 ±1.18 | -421 | |
| Defensive / floor#17 / round-1 | 74 | -274 | -3.70 ±1.55 | -212 | |
| Defensive / floor#32 / round-1 | 47 | -273 | -5.81 ±2.13 | -218 | |
| Competitive / floor#15 / balancing | 79 | -265 | -3.35 ±1.11 | -436 | |
| Defensive / floor#129 / round-2 | 173 | -260 | -1.50 ±1.19 | -546 | |
| Competitive / floor#238 / balancing | 134 | -251 | -1.87 ±1.18 | -195 | |
| Constructive / floor#151 / round-2 | 88 | -242 | -2.75 ±2.16 | -236 | |
| Competitive / floor#15 / round-2 | 58 | -237 | -4.09 ±1.33 | -319 | |
| Competitive / floor#246+rb / round-2 | 62 | -236 | -3.81 ±1.59 | -379 | |
| Competitive / fallback@5 / round-2 | 100 | -235 | -2.35 ±1.14 | -171 | |
| Defensive / floor#204 / round-1 | 72 | -232 | -3.22 ±1.57 | -205 | |
| Competitive / floor#31 / balancing | 48 | -230 | -4.79 ±1.67 | -296 | |
| Constructive / floor#140 / round-1 | 18 | -229 | -12.72 ±0.82 | -229 | |
| Defensive / floor#237 / round-2 | 70 | -223 | -3.19 ±1.45 | -291 | |
| Defensive / floor#205 / round-1 | 95 | -221 | -2.33 ±1.34 | -179 | |
| Competitive / floor#16 / balancing | 68 | -216 | -3.18 ±1.33 | -361 | |
| Competitive / floor#9 / round-2 | 145 | -216 | -1.49 ±0.79 | -280 | |
| Competitive / floor#236 / balancing | 112 | -213 | -1.90 ±1.13 | -261 | |
| Defensive / floor#27 / round-2 | 38 | -189 | -4.97 ±1.66 | -222 | |
| Competitive / floor#240+rb / round-2 | 57 | -180 | -3.16 ±1.79 | -127 | |
| Defensive / floor#33 / round-2 | 40 | -178 | -4.45 ±2.07 | -159 | |
| Defensive / floor#235 / round-2 | 51 | -172 | -3.37 ±1.58 | -201 | |
| Competitive / floor#234+rb / round-2 | 19 | -169 | -8.89 ±2.29 | -203 | |
| Competitive / floor#234 / round-2 | 51 | -166 | -3.25 ±1.71 | -198 | |
| Competitive / floor#238 / round-2 | 80 | -163 | -2.04 ±1.61 | -115 | |
| Defensive / floor#64 / balancing | 231 | -163 | -0.71 ±0.76 | -398 | ~noise |
| Defensive / floor#203 / round-1 | 78 | -162 | -2.08 ±1.34 | -148 | |
| Competitive / floor#55 / round-2 | 34 | -159 | -4.68 ±1.96 | -121 | |
| Competitive / floor#140 / round-1 | 42 | -152 | -3.62 ±2.92 | -152 | |
| Defensive / floor#153 / round-1 | 22 | -152 | -6.91 ±3.94 | -123 | |
| Competitive / floor#236+rb / round-2 | 33 | -151 | -4.58 ±2.71 | -149 | |
| Defensive / floor#48 / round-1 | 48 | -151 | -3.15 ±1.89 | -94 | |
| Defensive / floor#239 / round-2 | 69 | -146 | -2.12 ±1.70 | -136 | |
| Competitive / floor#237 / balancing | 79 | -143 | -1.81 ±1.01 | -362 | |
| Constructive / floor#147 / round-2 | 37 | -140 | -3.78 ±3.22 | -140 | |
| Defensive / floor#17 / round-2 | 73 | -136 | -1.86 ±1.10 | -193 | |
| Competitive / floor#32 / round-1 | 21 | -135 | -6.43 ±2.97 | -133 | |
| Defensive / floor#197 / round-2 | 63 | -132 | -2.10 ±1.50 | -172 | |
| Competitive / floor#236 / round-2 | 47 | -125 | -2.66 ±2.02 | -134 | |
| Competitive / floor#46+rb / deep | 40 | -120 | -3.00 ±0.92 | -241 | |
| Competitive / floor#47 / round-2 | 107 | -120 | -1.12 ±1.40 | -326 | ~noise |
| Constructive / floor#47 / round-2 | 12 | -120 | -10.00 ±2.70 | -120 | |
| Competitive / floor#32 / round-2 | 60 | -119 | -1.98 ±1.32 | -184 | |
| Defensive / floor#42 / round-2 | 38 | -119 | -3.13 ±3.29 | -85 | ~noise |
| Competitive / floor#46 / deep | 56 | -117 | -2.09 ±1.43 | -214 | |
| Defensive / floor#238 / round-2 | 23 | -116 | -5.04 ±2.36 | -137 | |
| Competitive / floor#33 / round-2 | 28 | -115 | -4.11 ±2.03 | -155 | |
| Defensive / floor#51 / balancing | 51 | -112 | -2.20 ±1.35 | -135 | |
| Constructive / floor#30 / round-1 | 48 | -111 | -2.31 ±1.79 | -160 | |
| Defensive / floor#65 / round-2 | 281 | -107 | -0.38 ±0.66 | -392 | ~noise |
| Competitive / floor#245 / deep | 22 | -101 | -4.59 ±3.07 | -156 | |
| Competitive / floor#239 / balancing | 70 | -99 | -1.41 ±1.09 | -213 | |
| Competitive / floor#246 / round-1 | 33 | -99 | -3.00 ±2.03 | -166 | |
| Competitive / floor#57 / round-2 | 13 | -99 | -7.62 ±3.62 | -98 | |
| Defensive / floor#245 / deep | 18 | -99 | -5.50 ±2.61 | -144 | |
| Constructive / floor#157 / round-2 | 45 | -97 | -2.16 ±2.25 | -90 | ~noise |
| Constructive / floor#62 / round-1 | 37 | -97 | -2.62 ±2.44 | -107 | |
| Defensive / floor#147 / round-1 | 12 | -94 | -7.83 ±4.79 | -86 | |
| Defensive / floor#12 / round-2 | 43 | -93 | -2.16 ±1.77 | -95 | |
| Defensive / floor#20 / deep | 21 | -91 | -4.33 ±1.46 | -159 | |
| Competitive / floor#17 / round-2 | 34 | -90 | -2.65 ±1.73 | -105 | |
| Defensive / floor#129 / round-1 | 58 | -90 | -1.55 ±1.78 | -187 | ~noise |
| Defensive / floor#133 / balancing | 85 | -89 | -1.05 ±1.65 | -244 | ~noise |
| Competitive / floor#63 / round-2 | 27 | -87 | -3.22 ±2.23 | -139 | |
| Defensive / floor#204 / round-2 | 45 | -86 | -1.91 ±1.44 | -36 | |
| Defensive / floor#21 / balancing | 62 | -85 | -1.37 ±1.24 | -201 | |
| Defensive / floor#241 / round-2 | 28 | -85 | -3.04 ±2.12 | -109 | |
| Competitive / floor#17 / round-1 | 10 | -84 | -8.40 ±3.59 | -54 | |
| Defensive / floor#18 / round-2 | 26 | -84 | -3.23 ±2.85 | -47 | |
| Defensive / floor#11 / round-2 | 22 | -83 | -3.77 ±1.60 | -124 | |
| Constructive / floor#31 / deep | 20 | -82 | -4.10 ±2.40 | -81 | |
| Defensive / floor#66 / balancing | 21 | -82 | -3.90 ±2.28 | -100 | |
| Competitive / floor#60 / balancing | 53 | -77 | -1.45 ±1.63 | -60 | ~noise |
| Competitive / floor#33 / round-1 | 12 | -76 | -6.33 ±3.23 | -108 | |
| Defensive / floor#47 / round-1 | 12 | -76 | -6.33 ±4.49 | -72 | |
| Competitive / floor#42 / round-2 | 14 | -75 | -5.36 ±4.46 | -101 | |
| Defensive / floor#32 / round-2 | 44 | -74 | -1.68 ±1.56 | -70 | |
| Defensive / floor#33 / round-1 | 18 | -74 | -4.11 ±3.88 | -53 | |
| Defensive / floor#49 / round-2 | 72 | -74 | -1.03 ±1.30 | -105 | ~noise |
| Competitive / floor#129 / deep | 9 | -73 | -8.11 ±3.51 | -86 | |
| Constructive / floor#62 / round-2 | 8 | -72 | -9.00 ±3.90 | -72 | |
| Competitive / floor#24 / round-2 | 46 | -71 | -1.54 ±1.30 | -76 | |
| Defensive / floor#26 / round-2 | 11 | -70 | -6.36 ±3.40 | -97 | |
| Defensive / floor#46 / round-1 | 34 | -70 | -2.06 ±2.76 | -48 | ~noise |
| Competitive / fallback@6 / round-2 | 18 | -69 | -3.83 ±3.54 | -44 | |
| Competitive / floor#30 / deep | 33 | -66 | -2.00 ±1.54 | -92 | |
| Constructive / floor#32 / deep | 59 | -65 | -1.10 ±0.60 | -76 | |
| Defensive / floor#241 / deep | 14 | -65 | -4.64 ±3.49 | -81 | |
| Competitive / floor#12 / round-2 | 23 | -63 | -2.74 ±1.84 | -66 | |
| Competitive / floor#241 / deep | 21 | -63 | -3.00 ±2.76 | -67 | |
| Competitive / floor#45 / balancing | 19 | -63 | -3.32 ±3.81 | -29 | ~noise |
| Defensive / floor#36 / balancing | 36 | -63 | -1.75 ±2.05 | -133 | ~noise |
| Competitive / floor#61 / deep | 43 | -62 | -1.44 ±1.87 | -142 | ~noise |
| Constructive / floor#17 / deep | 74 | -62 | -0.84 ±0.49 | -72 | |
| Defensive / floor#239 / balancing | 32 | -60 | -1.88 ±1.87 | -81 | |
| Competitive / floor#153 / round-2 | 17 | -59 | -3.47 ±4.83 | -54 | ~noise |
| Competitive / book+rb / deep | 59 | -56 | -0.95 ±0.86 | -87 | |
| Constructive / floor#153 / round-2 | 19 | -56 | -2.95 ±4.44 | -46 | ~noise |
| Defensive / floor#40 / round-2 | 6 | -56 | -9.33 ±2.24 | -56 | |
| Defensive / book / deep | 47 | -55 | -1.17 ±1.41 | -65 | ~noise |
| Defensive / floor#246 / deep | 9 | -55 | -6.11 ±2.56 | -101 | |
| Competitive / floor#151 / round-2 | 14 | -54 | -3.86 ±4.48 | -55 | ~noise |
| Competitive / floor#48 / round-2 | 29 | -54 | -1.86 ±2.64 | -131 | ~noise |
| Defensive / floor#41 / round-2 | 45 | -53 | -1.18 ±2.40 | -77 | ~noise |
| Competitive / floor#234 / balancing | 45 | -52 | -1.16 ±1.73 | -62 | ~noise |
| Defensive / floor#55 / round-2 | 15 | -52 | -3.47 ±3.27 | -40 | |
| Competitive / floor#5 / deep | 14 | -51 | -3.64 ±3.24 | -80 | |
| Competitive / floor#3+rb / deep | 25 | -48 | -1.92 ±2.32 | -40 | ~noise |
| Defensive / floor#31 / round-1 | 13 | -47 | -3.62 ±2.90 | -59 | |
| Defensive / floor#36 / round-2 | 23 | -47 | -2.04 ±2.29 | -78 | ~noise |
| Competitive / floor#16 / deep | 15 | -46 | -3.07 ±1.98 | -40 | |
| Defensive / floor#16 / round-1 | 9 | -46 | -5.11 ±4.33 | -48 | |
| Defensive / floor#203 / round-2 | 51 | -46 | -0.90 ±1.83 | -32 | ~noise |
| Competitive / floor#153 / round-1 | 3 | -45 | -15.00 ±2.99 | -45 | |
| Constructive / floor#157 / round-1 | 4 | -45 | -11.25 ±1.67 | -45 | |
| Constructive / floor#47 / round-1 | 34 | -45 | -1.32 ±2.89 | -43 | ~noise |
| Competitive / floor#238+rb / round-2 | 39 | -44 | -1.13 ±1.94 | +20 | ~noise plain/PD-flip |
| Defensive / floor#151 / round-1 | 10 | -44 | -4.40 ±6.57 | -47 | ~noise |
| Defensive / floor#21 / round-2 | 49 | -43 | -0.88 ±1.76 | -206 | ~noise |
| Competitive / floor#18 / round-1 | 15 | -41 | -2.73 ±4.33 | -17 | ~noise |
| Competitive / floor#237 / deep | 7 | -41 | -5.86 ±3.46 | -62 | |
| Competitive / floor#45 / round-2 | 70 | -41 | -0.59 ±1.45 | -60 | ~noise |
| Competitive / floor#47 / balancing | 45 | -39 | -0.87 ±2.06 | -78 | ~noise |
| Competitive / floor#60+rb / round-2 | 8 | -39 | -4.88 ±3.46 | -53 | |
| Competitive / floor#39 / round-2 | 15 | -38 | -2.53 ±2.57 | -37 | ~noise |
| Defensive / floor#29 / round-1 | 7 | -38 | -5.43 ±5.69 | -36 | ~noise |
| Defensive / floor#34 / balancing | 151 | -38 | -0.25 ±0.96 | -83 | ~noise |
| Defensive / floor#61 / balancing | 27 | -38 | -1.41 ±2.00 | -72 | ~noise |
| Competitive / floor#244 / round-2 | 22 | -36 | -1.64 ±2.21 | -66 | ~noise |
| Competitive / floor#61+rb / deep | 9 | -35 | -3.89 ±2.34 | -49 | |
| Constructive / floor#147 / deep | 24 | -33 | -1.38 ±4.11 | -20 | ~noise |
| Constructive / floor#63 / round-2 | 10 | -32 | -3.20 ±3.74 | -60 | ~noise |
| Defensive / floor#205 / round-2 | 89 | -32 | -0.36 ±1.34 | -32 | ~noise |
| Defensive / floor#229 / round-2 | 6 | -32 | -5.33 ±8.36 | -32 | ~noise |
| Defensive / floor#54 / deep | 5 | -32 | -6.40 ±4.05 | -40 | |
| Competitive / floor#241+rb / deep | 5 | -30 | -6.00 ±6.59 | -31 | ~noise |
| Constructive / floor#16 / deep | 4 | -30 | -7.50 ±2.47 | -38 | |
| Competitive / floor#48 / round-1 | 6 | -29 | -4.83 ±6.14 | -18 | ~noise |
| Competitive / floor#143 / round-2 | 3 | -28 | -9.33 ±3.46 | -32 | |
| Defensive / floor#237 / balancing | 25 | -28 | -1.12 ±2.19 | -85 | ~noise |
| Competitive / floor#45+rb / round-2 | 4 | -27 | -6.75 ±4.83 | -52 | |
| Defensive / floor#218 / round-2 | 6 | -27 | -4.50 ±1.31 | -40 | |
| Defensive / floor#61 / deep | 25 | -27 | -1.08 ±1.92 | -119 | ~noise |
| Competitive / floor#147 / round-2 | 31 | -26 | -0.84 ±3.25 | -28 | ~noise |
| Competitive / floor#57 / deep | 2 | -26 | -13.00 ±1.96 | -29 | |
| Competitive / floor#62 / round-1 | 3 | -26 | -8.67 ±9.49 | -42 | ~noise |
| Defensive / floor#140 / balancing | 4 | -26 | -6.50 ±5.09 | -26 | |
| Competitive / floor#1 / round-2 | 545 | -25 | -0.05 ±0.57 | +382 | ~noise plain/PD-flip |
| Competitive / floor#11 / round-2 | 4 | -25 | -6.25 ±7.17 | -26 | ~noise |
| Competitive / floor#140+rb / round-2 | 5 | -25 | -5.00 ±6.07 | -25 | ~noise |
| Competitive / floor#18 / round-2 | 12 | -25 | -2.08 ±3.52 | -15 | ~noise |
| Competitive / floor#239+rb / deep | 5 | -25 | -5.00 ±9.02 | -30 | ~noise |
| Defensive / floor#240 / round-2 | 28 | -25 | -0.89 ±2.25 | -16 | ~noise |
| Defensive / floor#6 / round-2 | 37 | -25 | -0.68 ±2.35 | -62 | ~noise |
| Competitive / floor#129 / round-2 | 21 | -24 | -1.14 ±3.13 | -77 | ~noise |
| Competitive / floor#145 / balancing | 6 | -24 | -4.00 ±4.98 | -24 | ~noise |
| Competitive / floor#235 / deep | 7 | -24 | -3.43 ±3.42 | -43 | |
| Competitive / floor#241+rb / balancing | 10 | -24 | -2.40 ±3.05 | -44 | ~noise |
| Competitive / floor#63 / round-1 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Constructive / floor#145 / round-1 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Constructive / floor#148 / round-2 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Defensive / floor#211 / round-2 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Defensive / floor#35 / deep | 13 | -24 | -1.85 ±3.39 | -43 | ~noise |
| Competitive / floor#147 / round-1 | 6 | -23 | -3.83 ±5.34 | -18 | ~noise |
| Competitive / floor#235 / balancing | 17 | -22 | -1.29 ±2.48 | -50 | ~noise |
| Defensive / floor#13 / round-2 | 3 | -22 | -7.33 ±7.95 | -22 | ~noise |
| Defensive / floor#27 / round-1 | 5 | -22 | -4.40 ±5.42 | -17 | ~noise |
| Defensive / floor#29 / round-2 | 7 | -22 | -3.14 ±5.58 | -15 | ~noise |
| Competitive / fallback@4+rb / round-2 | 4 | -21 | -5.25 ±0.94 | -24 | |
| Competitive / floor#235+rb / balancing | 2 | -21 | -10.50 ±2.94 | -21 | |
| Competitive / fallback@2 / balancing | 2 | -20 | -10.00 ±1.96 | -20 | |
| Competitive / floor#61 / balancing | 13 | -20 | -1.54 ±3.78 | -35 | ~noise |
| Defensive / floor#17 / deep | 3 | -20 | -6.67 ±6.91 | -18 | ~noise |
| Competitive / floor#140 / round-2 | 40 | -19 | -0.47 ±2.74 | -22 | ~noise |
| Competitive / floor#46 / balancing | 12 | -19 | -1.58 ±2.87 | -70 | ~noise |
| Competitive / floor#241+rb / round-2 | 3 | -18 | -6.00 ±11.81 | -16 | ~noise |
| Competitive / floor#63+rb / round-2 | 2 | -18 | -9.00 ±5.88 | -10 | |
| Defensive / floor#140 / round-2 | 2 | -18 | -9.00 ±1.96 | -26 | |
| Defensive / floor#236 / round-2 | 2 | -18 | -9.00 ±3.92 | -18 | |
| Defensive / floor#56 / deep | 3 | -18 | -6.00 ±2.99 | -34 | |
| Competitive / floor#245+rb / balancing | 5 | -17 | -3.40 ±5.13 | -35 | ~noise |
| Competitive / floor#62+rb / round-2 | 4 | -17 | -4.25 ±11.61 | -4 | ~noise |
| Constructive / floor#17 / round-2 | 2 | -17 | -8.50 ±2.94 | -17 | |
| Defensive / floor#14 / round-2 | 5 | -17 | -3.40 ±8.02 | -14 | ~noise |
| Defensive / floor#199 / round-2 | 47 | -17 | -0.36 ±1.25 | -87 | ~noise |
| Competitive / floor#30+rb / round-2 | 4 | -16 | -4.00 ±2.77 | -32 | |
| Defensive / floor#228 / round-2 | 11 | -15 | -1.36 ±3.66 | +0 | ~noise plain/PD-flip |
| Defensive / floor#50 / deep | 12 | -15 | -1.25 ±2.23 | -15 | ~noise |
| Competitive / floor#135 / round-2 | 2 | -14 | -7.00 ±3.92 | -26 | |
| Competitive / floor#140 / deep | 2 | -14 | -7.00 ±1.96 | -22 | |
| Competitive / floor#239+rb / balancing | 4 | -14 | -3.50 ±6.86 | -14 | ~noise |
| Competitive / floor#24 / deep | 5 | -14 | -2.80 ±4.04 | -17 | ~noise |
| Competitive / floor#31 / deep | 19 | -14 | -0.74 ±1.71 | +1 | ~noise plain/PD-flip |
| Constructive / floor#32 / round-2 | 4 | -14 | -3.50 ±11.94 | -6 | ~noise |
| Defensive / floor#34 / round-1 | 2 | -14 | -7.00 ±0.00 | -14 | |
| Competitive / floor#145 / round-1 | 1 | -13 | -13.00 ±0.00 | -13 | ~noise |
| Competitive / floor#47 / round-1 | 1 | -13 | -13.00 ±0.00 | -13 | ~noise |
| Competitive / floor#63+rb / deep | 2 | -13 | -6.50 ±8.82 | -14 | ~noise |
| Competitive / floor#47+rb / balancing | 2 | -12 | -6.00 ±1.96 | -12 | |
| Defensive / floor#28 / round-1 | 2 | -12 | -6.00 ±1.96 | -14 | |
| Competitive / floor#12 / deep | 2 | -11 | -5.50 ±2.94 | -14 | |
| Competitive / floor#17 / deep | 12 | -11 | -0.92 ±3.82 | -4 | ~noise |
| Competitive / floor#45+rb / deep | 2 | -11 | -5.50 ±2.94 | -21 | |
| Competitive / floor#56 / round-2 | 1 | -11 | -11.00 ±0.00 | -12 | ~noise |
| Constructive / floor#147 / round-1 | 1 | -11 | -11.00 ±0.00 | -11 | ~noise |
| Defensive / floor#230 / round-2 | 2 | -11 | -5.50 ±0.98 | -14 | |
| Defensive / floor#26 / round-1 | 1 | -11 | -11.00 ±0.00 | -14 | ~noise |
| Defensive / floor#63 / deep | 2 | -11 | -5.50 ±2.94 | -14 | |
| Competitive / floor#60 / deep | 5 | -10 | -2.00 ±5.85 | -3 | ~noise |
| Defensive / floor#208 / round-2 | 9 | -10 | -1.11 ±4.45 | +2 | ~noise plain/PD-flip |
| Defensive / floor#212 / round-2 | 1 | -10 | -10.00 ±0.00 | -10 | ~noise |
| Defensive / floor#28 / round-2 | 1 | -10 | -10.00 ±0.00 | -10 | ~noise |
| Defensive / floor#32 / deep | 2 | -10 | -5.00 ±5.88 | -14 | ~noise |
| Defensive / floor#40 / deep | 1 | -10 | -10.00 ±0.00 | -10 | ~noise |
| Defensive / floor#56 / round-1 | 12 | -10 | -0.83 ±4.76 | -4 | ~noise |
| Competitive / fallback@3 / balancing | 1 | -9 | -9.00 ±0.00 | -9 | ~noise |
| Competitive / floor#145 / round-2 | 11 | -9 | -0.82 ±5.73 | -16 | ~noise |
| Competitive / floor#29 / deep | 1 | -9 | -9.00 ±0.00 | -9 | ~noise |
| Competitive / floor#32 / deep | 7 | -9 | -1.29 ±1.64 | -12 | ~noise |
| Competitive / floor#62 / deep | 22 | -9 | -0.41 ±2.78 | -31 | ~noise |
| Constructive / floor#33 / deep | 2 | -9 | -4.50 ±0.98 | -12 | |
| Defensive / floor#18 / deep | 5 | -9 | -1.80 ±2.18 | -12 | ~noise |
| Defensive / floor#41 / round-1 | 4 | -9 | -2.25 ±6.99 | -4 | ~noise |
| Competitive / floor#1+rb / round-2 | 3 | -8 | -2.67 ±4.57 | +4 | ~noise plain/PD-flip |
| Competitive / floor#144 / round-2 | 9 | -8 | -0.89 ±5.14 | -8 | ~noise |
| Competitive / floor#237+rb / balancing | 3 | -8 | -2.67 ±14.06 | -5 | ~noise |
| Competitive / floor#40 / round-2 | 37 | -8 | -0.22 ±2.34 | +44 | ~noise plain/PD-flip |
| Constructive / floor#157 / deep | 20 | -8 | -0.40 ±3.23 | -1 | ~noise |
| Defensive / floor#231 / round-2 | 9 | -8 | -0.89 ±4.16 | -30 | ~noise |
| Defensive / floor#54 / round-2 | 2 | -8 | -4.00 ±3.92 | -26 | |
| Competitive / floor#235+rb / deep | 5 | -7 | -1.40 ±2.02 | -40 | ~noise |
| Competitive / floor#242 / balancing | 54 | -7 | -0.13 ±1.50 | -114 | ~noise |
| Competitive / floor#26 / deep | 1 | -7 | -7.00 ±0.00 | -12 | ~noise |
| Competitive / floor#47+rb / deep | 1 | -7 | -7.00 ±0.00 | -9 | ~noise |
| Defensive / floor#237 / deep | 2 | -7 | -3.50 ±2.94 | -21 | |
| Defensive / floor#62 / deep | 5 | -7 | -1.40 ±6.58 | -17 | ~noise |
| Competitive / floor#0 / deep | 3 | -6 | -2.00 ±1.13 | -22 | |
| Competitive / floor#144 / round-1 | 1 | -6 | -6.00 ±0.00 | -6 | ~noise |
| Defensive / floor#129 / deep | 20 | -6 | -0.30 ±3.38 | +2 | ~noise plain/PD-flip |
| Defensive / floor#238 / balancing | 1 | -6 | -6.00 ±0.00 | -6 | ~noise |
| Defensive / floor#31 / deep | 13 | -6 | -0.46 ±2.49 | -17 | ~noise |
| Competitive / floor#62 / balancing | 6 | -5 | -0.83 ±6.06 | +31 | ~noise plain/PD-flip |
| Defensive / floor#219 / round-2 | 1 | -5 | -5.00 ±0.00 | -6 | ~noise |
| Defensive / floor#25 / round-2 | 2 | -5 | -2.50 ±4.90 | -6 | ~noise |
| Defensive / floor#47 / deep | 2 | -5 | -2.50 ±8.82 | -7 | ~noise |
| Competitive / floor#56 / deep | 4 | -4 | -1.00 ±3.84 | -1 | ~noise |
| Competitive / floor#60+rb / deep | 3 | -4 | -1.33 ±3.64 | -15 | ~noise |
| Defensive / floor#10 / round-2 | 4 | -4 | -1.00 ±6.55 | +9 | ~noise plain/PD-flip |
| Defensive / floor#235 / deep | 4 | -3 | -0.75 ±1.47 | -14 | ~noise |
| Defensive / floor#240 / deep | 1 | -3 | -3.00 ±0.00 | -9 | ~noise |
| Defensive / floor#39 / round-2 | 2 | -3 | -1.50 ±2.94 | -14 | ~noise |
| Defensive / floor#46 / deep | 9 | -3 | -0.33 ±3.05 | -29 | ~noise |
| Competitive / fallback@4 / balancing | 1 | -2 | -2.00 ±0.00 | -2 | ~noise |
| Competitive / floor#18+rb / balancing | 2 | -2 | -1.00 ±1.96 | -8 | ~noise |
| Competitive / floor#237+rb / deep | 1 | -2 | -2.00 ±0.00 | -2 | ~noise |
| Defensive / floor#128 / deep | 2 | -2 | -1.00 ±3.92 | -7 | ~noise |
| Defensive / floor#5 / round-1 | 28 | -2 | -0.07 ±2.88 | +12 | ~noise plain/PD-flip |
| Defensive / floor#57 / round-1 | 6 | -2 | -0.33 ±5.13 | +5 | ~noise plain/PD-flip |
| Defensive / floor#18 / round-1 | 9 | -1 | -0.11 ±6.23 | +5 | ~noise plain/PD-flip |
| Defensive / floor#238 / deep | 2 | -1 | -0.50 ±2.94 | -12 | ~noise |
| Competitive / floor#11 / deep | 2 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Competitive / floor#143 / balancing | 2 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Competitive / floor#41 / deep | 1 | +0 | +0.00 ±0.00 | +7 | ~noise |
| Defensive / floor#24 / round-2 | 2 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Competitive / floor#14 / round-2 | 1 | +1 | +1.00 ±0.00 | +1 | ~noise |
| Competitive / floor#33 / balancing | 1 | +1 | +1.00 ±0.00 | -2 | ~noise plain/PD-flip |
| Constructive / floor#154 / deep | 13 | +1 | +0.08 ±3.45 | +10 | ~noise |
| Competitive / floor#39 / deep | 9 | +2 | +0.22 ±0.29 | +2 | ~noise |
| Competitive / floor#54 / round-2 | 10 | +2 | +0.20 ±2.92 | -3 | ~noise plain/PD-flip |
| Competitive / floor#54+rb / round-2 | 3 | +2 | +0.67 ±6.23 | -10 | ~noise plain/PD-flip |
| Competitive / floor#55 / deep | 1 | +2 | +2.00 ±0.00 | +5 | ~noise |
| Constructive / floor#151 / deep | 8 | +2 | +0.25 ±0.32 | +2 | ~noise |
| Defensive / floor#1 / round-1 | 17 | +2 | +0.12 ±4.32 | +5 | ~noise |
| Competitive / floor#26 / round-2 | 2 | +3 | +1.50 ±6.86 | +4 | ~noise |
| Defensive / floor#127 / round-2 | 8 | +3 | +0.38 ±3.72 | +0 | ~noise |
| Defensive / floor#9 / round-2 | 1 | +3 | +3.00 ±0.00 | +5 | ~noise |
| Competitive / floor#147 / deep | 2 | +4 | +2.00 ±0.00 | +4 | |
| Competitive / floor#18 / deep | 3 | +4 | +1.33 ±3.64 | +9 | ~noise |
| Defensive / floor#227 / round-2 | 10 | +4 | +0.40 ±3.92 | +13 | ~noise |
| Defensive / floor#235 / balancing | 16 | +4 | +0.25 ±2.36 | -26 | ~noise plain/PD-flip |
| Defensive / floor#239 / deep | 3 | +4 | +1.33 ±2.61 | +2 | ~noise |
| Defensive / floor#42 / deep | 3 | +4 | +1.33 ±11.33 | +16 | ~noise |
| Competitive / floor#245+rb / deep | 2 | +5 | +2.50 ±0.98 | +0 | |
| Defensive / floor#5 / round-2 | 55 | +5 | +0.09 ±1.85 | -5 | ~noise plain/PD-flip |
| Competitive / floor#3+rb / balancing | 12 | +6 | +0.50 ±1.81 | +27 | ~noise |
| Competitive / floor#40 / deep | 1 | +6 | +6.00 ±0.00 | +8 | ~noise |
| Defensive / floor#14 / round-1 | 3 | +6 | +2.00 ±9.87 | +12 | ~noise |
| Defensive / floor#60 / deep | 6 | +6 | +1.00 ±2.58 | +3 | ~noise |
| Competitive / floor#32 / balancing | 1 | +7 | +7.00 ±0.00 | +7 | ~noise |
| Competitive / floor#46+rb / round-2 | 6 | +7 | +1.17 ±3.18 | -7 | ~noise plain/PD-flip |
| Constructive / floor#48 / deep | 8 | +7 | +0.88 ±4.63 | -7 | ~noise plain/PD-flip |
| Defensive / floor#226 / round-2 | 6 | +7 | +1.17 ±5.32 | +15 | ~noise |
| Competitive / floor#27 / round-2 | 4 | +9 | +2.25 ±9.45 | -5 | ~noise plain/PD-flip |
| Defensive / floor#145 / round-1 | 10 | +9 | +0.90 ±6.21 | +38 | ~noise |
| Defensive / floor#38 / round-2 | 2 | +9 | +4.50 ±0.98 | +12 | |
| Constructive / floor#145 / deep | 19 | +10 | +0.53 ±4.07 | +10 | ~noise |
| Constructive / floor#47 / deep | 15 | +10 | +0.67 ±3.27 | -12 | ~noise plain/PD-flip |
| Constructive / floor#48 / round-2 | 1 | +10 | +10.00 ±0.00 | +10 | ~noise |
| Defensive / floor#31 / balancing | 3 | +10 | +3.33 ±1.73 | +19 | |
| Defensive / floor#57 / deep | 1 | +10 | +10.00 ±0.00 | +4 | ~noise |
| Competitive / floor#15 / deep | 6 | +11 | +1.83 ±2.93 | +14 | ~noise |
| Competitive / floor#41 / round-2 | 2 | +11 | +5.50 ±0.98 | -24 | plain/PD-flip |
| Defensive / floor#47 / balancing | 3 | +11 | +3.67 ±2.61 | +11 | |
| Defensive / floor#48 / deep | 4 | +11 | +2.75 ±6.17 | +4 | ~noise |
| Competitive / floor#63 / deep | 3 | +14 | +4.67 ±6.91 | +11 | ~noise |
| Competitive / floor#151 / balancing | 4 | +16 | +4.00 ±4.80 | +16 | ~noise |
| Defensive / floor#0 / round-1 | 3 | +16 | +5.33 ±2.85 | +17 | |
| Defensive / floor#145 / balancing | 2 | +16 | +8.00 ±3.92 | +16 | |
| Defensive / floor#16 / deep | 4 | +16 | +4.00 ±6.55 | +3 | ~noise |
| Defensive / floor#2 / round-1 | 7 | +16 | +2.29 ±6.72 | +17 | ~noise |
| Competitive / floor#244 / deep | 2 | +17 | +8.50 ±2.94 | +17 | |
| Defensive / floor#241 / balancing | 23 | +17 | +0.74 ±2.08 | -10 | ~noise plain/PD-flip |
| Competitive / floor#243 / round-2 | 2 | +18 | +9.00 ±3.92 | +18 | |
| Competitive / floor#54 / deep | 3 | +18 | +6.00 ±4.53 | +24 | |
| Defensive / floor#1 / deep | 15 | +20 | +1.33 ±2.70 | -61 | ~noise plain/PD-flip |
| Competitive / floor#18+rb / deep | 2 | +24 | +12.00 ±1.96 | +26 | |
| Defensive / floor#62 / round-1 | 7 | +24 | +3.43 ±6.77 | +21 | ~noise |
| Competitive / floor#151 / round-1 | 4 | +26 | +6.50 ±6.28 | +26 | |
| Constructive / floor#148 / deep | 4 | +26 | +6.50 ±6.28 | +29 | |
| Competitive / floor#47+rb / round-2 | 2 | +27 | +13.50 ±0.98 | +27 | |
| Constructive / floor#62 / deep | 25 | +27 | +1.08 ±2.61 | -5 | ~noise plain/PD-flip |
| Defensive / floor#207 / round-2 | 4 | +29 | +7.25 ±5.45 | +39 | |
| Competitive / floor#239 / deep | 14 | +31 | +2.21 ±2.50 | +6 | ~noise |
| Constructive / floor#153 / deep | 4 | +32 | +8.00 ±2.26 | +32 | |
| Defensive / floor#6 / round-1 | 19 | +32 | +1.68 ±3.40 | +37 | ~noise |
| Defensive / floor#210 / round-2 | 4 | +35 | +8.75 ±3.78 | +38 | |
| Defensive / floor#57 / round-2 | 19 | +37 | +1.95 ±3.63 | +60 | ~noise |
| Defensive / floor#42 / round-1 | 5 | +41 | +8.20 ±3.24 | +58 | |
| Defensive / floor#140 / round-1 | 28 | +62 | +2.21 ±3.85 | +79 | ~noise |
| Defensive / floor#56 / round-2 | 17 | +68 | +4.00 ±3.14 | +78 | |
| Competitive / floor#47 / deep | 24 | +80 | +3.33 ±2.26 | +43 | |
| Defensive / floor#1 / round-2 | 216 | +80 | +0.37 ±1.06 | +141 | ~noise |
| Competitive / floor#1 / deep | 76 | +105 | +1.38 ±1.37 | +102 | |
| Competitive / floor#62 / round-2 | 66 | +125 | +1.89 ±1.71 | -158 | plain/PD-flip |
| Defensive / floor#47 / round-2 | 123 | +137 | +1.11 ±1.33 | +0 | ~noise |
| Defensive / floor#0 / round-2 | 27 | +147 | +5.44 ±2.14 | +147 | |
| Defensive / floor#62 / round-2 | 54 | +153 | +2.83 ±1.89 | +127 | |
| Competitive / floor#0 / round-2 | 217 | +412 | +1.90 ±0.86 | +577 | |

## By phase

  -279716 IMPs  135791 boards  Constructive
  -269196 IMPs  116694 boards  Defensive
  -140802 IMPs   51779 boards  Competitive

## By provenance

  -379766 IMPs  186301 boards  book
   -94437 IMPs   32782 boards  floor#3
   -30110 IMPs   10751 boards  fallback@1
   -26963 IMPs    9564 boards  fallback@2
   -19599 IMPs    7856 boards  fallback@3
   -16874 IMPs    6790 boards  floor#245
    -9075 IMPs    2986 boards  fallback@4
    -8179 IMPs    2856 boards  floor#61
    -7682 IMPs    2309 boards  floor#246
    -7346 IMPs    3410 boards  floor#20
    -7285 IMPs    2616 boards  floor#60
    -7248 IMPs    2786 boards  floor#46
    -6595 IMPs    2706 boards  floor#140
    -6448 IMPs    2451 boards  floor#45
    -6102 IMPs    3080 boards  floor#35
    -4718 IMPs    2867 boards  floor#50
    -3613 IMPs    1355 boards  floor#30
    -3338 IMPs    1700 boards  floor#64
    -3305 IMPs     875 boards  floor#31
    -3032 IMPs     786 boards  floor#202
    -2589 IMPs     545 boards  floor#16
    -2380 IMPs     751 boards  floor#132
    -1588 IMPs     257 boards  floor#245+rb
    -1546 IMPs    1434 boards  floor#65
    -1497 IMPs     427 boards  floor#32
    -1407 IMPs     476 boards  fallback@5
    -1352 IMPs     549 boards  floor#131
    -1208 IMPs     450 boards  floor#200
    -1203 IMPs     419 boards  floor#17
    -1191 IMPs     433 boards  floor#240
    -1162 IMPs     395 boards  floor#237
    -1070 IMPs     406 boards  floor#6
     -959 IMPs     198 boards  floor#63
     -846 IMPs     280 boards  floor#241
     -838 IMPs     213 boards  floor#145
     -804 IMPs     320 boards  floor#239
     -769 IMPs     453 boards  floor#49
     -737 IMPs     242 boards  floor#197
     -737 IMPs     447 boards  floor#5
     -690 IMPs     452 boards  book+rb
     -672 IMPs     181 boards  floor#48
     -656 IMPs     254 boards  floor#198
     -632 IMPs     293 boards  floor#51
     -616 IMPs     293 boards  floor#21
     -610 IMPs     200 boards  floor#66
     -537 IMPs     240 boards  floor#238
     -513 IMPs     262 boards  floor#199
     -498 IMPs     198 boards  floor#235
     -491 IMPs     143 boards  floor#15
     -453 IMPs     281 boards  floor#129
     -451 IMPs     101 boards  floor#33
     -446 IMPs     330 boards  floor#2
     -443 IMPs     259 boards  floor#3+rb
     -401 IMPs     204 boards  floor#36
     -379 IMPs     400 boards  floor#133
     -356 IMPs     161 boards  floor#236
     -350 IMPs     136 boards  floor#10
     -347 IMPs     209 boards  floor#25
     -323 IMPs     113 boards  floor#147
     -318 IMPs     117 boards  floor#204
     -296 IMPs     128 boards  floor#151
     -280 IMPs      65 boards  floor#153
     -253 IMPs     184 boards  floor#205
     -236 IMPs      62 boards  floor#246+rb
     -218 IMPs      96 boards  floor#234
     -213 IMPs     146 boards  floor#9
     -209 IMPs      50 boards  floor#55
     -208 IMPs     129 boards  floor#203
     -202 IMPs      47 boards  floor#27
     -180 IMPs      57 boards  floor#240+rb
     -180 IMPs     378 boards  floor#47
     -169 IMPs      19 boards  floor#234+rb
     -167 IMPs      68 boards  floor#12
     -156 IMPs      70 boards  floor#18
     -151 IMPs      33 boards  floor#236+rb
     -150 IMPs      69 boards  floor#157
     -149 IMPs      60 boards  floor#42
     -113 IMPs      46 boards  floor#46+rb
     -108 IMPs      28 boards  floor#11
      -85 IMPs      53 boards  floor#24
      -85 IMPs      15 boards  floor#26
      -80 IMPs      41 boards  floor#57
      -72 IMPs      18 boards  floor#241+rb
      -69 IMPs      18 boards  fallback@6
      -69 IMPs      15 boards  floor#29
      -68 IMPs      45 boards  floor#40
      -52 IMPs     153 boards  floor#34
      -51 IMPs      52 boards  floor#41
      -44 IMPs      39 boards  floor#238+rb
      -43 IMPs      11 boards  floor#60+rb
      -39 IMPs       9 boards  floor#239+rb
      -39 IMPs      26 boards  floor#39
      -38 IMPs       6 boards  floor#45+rb
      -35 IMPs       9 boards  floor#61+rb
      -32 IMPs       6 boards  floor#229
      -31 IMPs       4 boards  floor#63+rb
      -28 IMPs       5 boards  floor#143
      -28 IMPs       7 boards  floor#235+rb
      -27 IMPs       6 boards  floor#218
      -25 IMPs       5 boards  floor#140+rb
      -24 IMPs       2 boards  floor#211
      -22 IMPs       3 boards  floor#13
      -22 IMPs       3 boards  floor#28
      -21 IMPs       4 boards  fallback@4+rb
      -20 IMPs      20 boards  floor#54
      -19 IMPs      24 boards  floor#244
      -17 IMPs       4 boards  floor#62+rb
      -16 IMPs       4 boards  floor#30+rb
      -15 IMPs      11 boards  floor#228
      -14 IMPs       2 boards  floor#135
      -14 IMPs      10 boards  floor#144
      -11 IMPs       2 boards  floor#230
      -10 IMPs       9 boards  floor#14
      -10 IMPs       9 boards  floor#208
      -10 IMPs       1 boards  floor#212
      -10 IMPs       4 boards  floor#237+rb
       -8 IMPs       3 boards  floor#1+rb
       -8 IMPs       9 boards  floor#231
       -7 IMPs      54 boards  floor#242
       -5 IMPs       1 boards  floor#219
       -2 IMPs       2 boards  floor#128
       +1 IMPs      13 boards  floor#154
       +2 IMPs       6 boards  floor#148
       +2 IMPs       3 boards  floor#54+rb
       +3 IMPs       8 boards  floor#127
       +4 IMPs      10 boards  floor#227
       +7 IMPs       6 boards  floor#226
       +8 IMPs       5 boards  floor#47+rb
       +9 IMPs       2 boards  floor#38
      +18 IMPs       2 boards  floor#243
      +22 IMPs       4 boards  floor#18+rb
      +25 IMPs      37 boards  floor#56
      +29 IMPs       4 boards  floor#207
      +35 IMPs       4 boards  floor#210
     +113 IMPs     233 boards  floor#62
     +182 IMPs     869 boards  floor#1
     +569 IMPs     250 boards  floor#0

## By family

  -333346 IMPs  135885 boards  round-1
  -226368 IMPs   98354 boards  round-2
   -93067 IMPs   47131 boards  opening
   -21503 IMPs   11845 boards  balancing
   -15430 IMPs   11049 boards  deep

## By direction

  -469225 IMPs   65957 boards  other
  -189275 IMPs   21868 boards  missed-game
  -176849 IMPs   26983 boards  overbid
  -166719 IMPs   25359 boards  sold-out
   -80145 IMPs    6554 boards  missed-slam
   -75329 IMPs   14400 boards  wrong-strain
   -11286 IMPs     778 boards  missed-grand
    -8607 IMPs    1387 boards  doubling
       +0 IMPs   52315 boards  flat
  +487721 IMPs   88663 boards  gain

## Worst boards per losing bucket

### Defensive / book / round-1 (58928 boards, -123392 IMPs)

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

### Constructive / book / opening (47131 boards, -93067 IMPs)

[vul both, seed Some(1783375067), board 372] swing -3400 pts / -22 IMPs (PD -22), diverged at call 0 (2♦ ours vs P BBA), overbid
  rule: exactly 6 ♦, 5–10 points, and not (opening in seat 4)
  W:A5.KQ973.J.97543 KT6.4.Q86432.QT6 832.AT852.T7.AJ8 QJ974.J6.AK95.K2
  ours NS @ A: 2♦ - 2NT - 3♥ X - - -  -> 3♥x by North
  ours EW @ B: - - 1♠ 2♠ X - - -  -> 2♠x by West

[vul both, seed Some(1783375076), board 3305] swing -2540 pts / -21 IMPs (PD -22), diverged at call 0 (1♦ ours vs P BBA), other
  rule: 10–11 HCP, Rule of 20, prefers diamonds, ≤4 ♥, and ≤4 ♠
  W:.AK2.AQ63.AJ9754 987654.QJ864.8.8 KQ32.T5.KJ954.QT AJT.973.T72.K632
  ours NS @ A: - - 1♣ - 1♦ - 5♠ - 6♣ - 7♦ - - -  -> 7♦ by East
  ours EW @ B: 1♦ - 2♣ - 2♠ - 3♣ - 4♠ - - -  -> 4♠ by East

[vul both, seed Some(1783375086), board 2865] swing -2530 pts / -21 IMPs (PD -21), diverged at call 1 (P ours vs 1♥ BBA), other
  rule: ≤11 points
  W:AKQ763.A.AQT98.4 98.KJ5.76.KQT862 JT52.8742.J54.93 4.QT963.K32.AJ75
  ours NS @ A: - - 1♠ - - 2♥ 3♦ 3♥ 4♠ - 4NT - 5♣ - 6♠ - - -  -> 6♠ by West
  ours EW @ B: - 1♥ 2♥ X - - -  -> 2♥x by West

### Constructive / book / round-2 (41295 boards, -81168 IMPs)

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

### Constructive / book / round-1 (29591 boards, -69526 IMPs)

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

### Defensive / floor#3 / round-2 (9276 boards, -29137 IMPs)

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

### Competitive / fallback@1 / round-1 (9512 boards, -26434 IMPs)

[vul both, seed Some(1783375079), board 667] swing -3220 pts / -22 IMPs (PD -22), diverged at call 2 (4♣ ours vs 4♥ BBA), other
  rule: 5+ ♣, (5+ ♥, or 5+ ♠), and 10+ points
  W:AQ95.J93.Q8.KQJ6 KJT864.A5.AT932. 3.KQT872.J.A9754 72.64.K7654.T832
  ours NS @ A: 1NT 2♦ 4♥ - - -  -> 4♥ by East
  ours EW @ B: 1NT 2♦ 4♣ - 4♦ X - - -  -> 4♦x by West

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

### Defensive / floor#3 / round-1 (8169 boards, -26386 IMPs)

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

### Competitive / fallback@2 / round-1 (8428 boards, -23647 IMPs)

[vul both, seed Some(1783375081), board 2158] swing -2520 pts / -21 IMPs (PD -19), diverged at call 2 (P ours vs 1♠ BBA), sold-out
  rule: 0+ HCP
  W:A74.AKJ9.Q.AQJ54 QJT952.Q8.75.982 .T76542.JT986.T6 K863.3.AK432.K73
  ours NS @ A: 1♦ X - 1♥ X XX - - -  -> 1♥xx by East
  ours EW @ B: 1♦ X 1♠ - 2♠ X - 3♥ - 4♥ 4♠ - - -  -> 4♠ by North

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

### Competitive / fallback@3 / round-2 (6017 boards, -13414 IMPs)

[vul both, seed Some(1783375066), board 5126] swing -2300 pts / -20 IMPs (PD -20), diverged at call 4 (3♦ ours vs P BBA), other
  rule: 2♦ is the cheapest bid, 6+ ♦, 2–5 points, and not (opponents bid ♦)
  W:K32.JT5.K9.K9853 T8754.76.AQJ.AT7 AJ96.AK3.T7.QJ42 Q.Q9842.865432.6
  ours NS @ A: - - 1♠ 1NT 3♦ 3NT - - -  -> 3NT by East
  ours EW @ B: - - 1♠ 1NT - 3♣ - 3♦ X - - -  -> 3♦x by East

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

### Competitive / floor#3 / round-2 (4149 boards, -11540 IMPs)

[vul both, seed Some(1783375068), board 5739] swing -2690 pts / -21 IMPs (PD -21), diverged at call 4 (P ours vs 1♠ BBA), wrong-strain
  rule: not ((opaque condition)), or (opaque condition)
  W:T764.K74.Q.AKQJ4 .AQT3.T9764.9873 Q32.98.J832.T652 AKJ985.J652.AK5.
  ours NS @ A: 1♣ - - X 1♠ - - X - 2♦ - 3♦ - - -  -> 3♦ by North
  ours EW @ B: 1♣ - - X - 1♥ - 2♥ X XX - - -  -> 2♥xx by North

[vul both, seed Some(1783375065), board 5877] swing -1760 pts / -18 IMPs (PD -18), diverged at call 6 (P ours vs XX BBA), other
  rule: not ((opaque condition)), or (opaque condition)
  W:AT76.KQJ742.4.73 942.65.T76.JT864 53.AT8.AKQ832.95 KQJ8.93.J95.AKQ2
  ours NS @ A: 1♦ - 1♥ - 2♦ X XX - - -  -> 2♦xx by East
  ours EW @ B: 1♦ - 1♥ - 2♦ X - 3♣ 3♥ - - -  -> 3♥ by West

[vul both, seed Some(1783375086), board 4039] swing -1930 pts / -18 IMPs (PD -18), diverged at call 4 (P ours vs 2♥ BBA), other
  rule: not ((opaque condition)), or (opaque condition)
  W:AJ864.AQ63..AT76 973.T98.JT85.J42 KT2.KJ54.74.K853 Q5.72.AKQ9632.Q9
  ours NS @ A: 1♠ - 1NT 2♦ 2♥ - 3♠ X XX - - -  -> 3♠xx by West
  ours EW @ B: 1♠ - 1NT 2♦ - - 2♠ - 3♠ - - -  -> 3♠ by West

