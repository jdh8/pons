=== arm 0: our american floor (us) vs BBA 2/1 (them), vulnerability none, 204800 boards ===
replay verification: 100.00% of 2143691 our-side calls (0 mismatched)
auction-divergent: 186801 (91%), contract-divergent: 155131 (76%)
plain DD: -1.5536 IMPs/board (95% CI [-1.5759, -1.5314]), -318181 IMPs total
perfect defense: -1.7186 IMPs/board (95% CI [-1.7452, -1.6921])
gen_args: --count 6400 --seed 1783375064 --output ab-results/anchor/2026-07-09-dd5524b/none/shard-0.json -v none

=== arm 1: our american floor (us) vs BBA 2/1 (them), vulnerability both, 204800 boards ===
replay verification: 100.00% of 2124164 our-side calls (0 mismatched)
auction-divergent: 185971 (91%), contract-divergent: 153612 (75%)
plain DD: -2.0814 IMPs/board (95% CI [-2.1098, -2.0530]), -426276 IMPs total
perfect defense: -2.3743 IMPs/board (95% CI [-2.4076, -2.3409])
gen_args: --count 6400 --seed 1783375064 --output ab-results/anchor/2026-07-09-dd5524b/both/shard-0.json -v both


## IMP histogram (plain, per contract-divergent board)

right-siding-only divergences (same contract, different auction): 64029

  -24 IMPs: 1
  -22 IMPs: 4
  -21 IMPs: 27
  -20 IMPs: 49
  -19 IMPs: 177
  -18 IMPs: 350
  -17 IMPs: 1129
  -16 IMPs: 1227
  -15 IMPs: 2731
  -14 IMPs: 4088
  -13 IMPs: 9353
  -12 IMPs: 8510
  -11 IMPs: 13955
  -10 IMPs: 19230
   -9 IMPs: 8245
   -8 IMPs: 6962
   -7 IMPs: 12814
   -6 IMPs: 19527
   -5 IMPs: 15788
   -4 IMPs: 8096
   -3 IMPs: 11554
   -2 IMPs: 13170
   -1 IMPs: 11784
   +0 IMPs: 50898
   +1 IMPs: 10498
   +2 IMPs: 9963
   +3 IMPs: 8202
   +4 IMPs: 5851
   +5 IMPs: 15002
   +6 IMPs: 12672
   +7 IMPs: 6131
   +8 IMPs: 2379
   +9 IMPs: 2456
  +10 IMPs: 5317
  +11 IMPs: 4299
  +12 IMPs: 2709
  +13 IMPs: 2913
  +14 IMPs: 475
  +15 IMPs: 109
  +16 IMPs: 48
  +17 IMPs: 47
  +18 IMPs: 2
  +19 IMPs: 1

## Ranked buckets (phase / provenance / family), losses first

| bucket | boards | net plain IMPs | IMPs/divergent ±CI | net PD IMPs | flag |
| --- | --- | --- | --- | --- | --- |
| Defensive / book / round-1 | 59857 | -134485 | -2.25 ±0.05 | -164830 | |
| Constructive / book / opening | 47309 | -96377 | -2.04 ±0.06 | -94166 | |
| Constructive / book / round-2 | 41292 | -81297 | -1.97 ±0.06 | -83198 | |
| Constructive / book / round-1 | 29719 | -71195 | -2.40 ±0.08 | -79022 | |
| Competitive / fallback@1 / round-1 | 13956 | -43168 | -3.09 ±0.11 | -40599 | |
| Competitive / fallback@2 / round-1 | 12137 | -39005 | -3.21 ±0.12 | -39609 | |
| Defensive / floor#3 / round-2 | 8706 | -28211 | -3.24 ±0.14 | -28525 | |
| Defensive / floor#3 / round-1 | 7680 | -24524 | -3.19 ±0.15 | -19237 | |
| Competitive / fallback@3 / round-2 | 7357 | -20148 | -2.74 ±0.14 | -22693 | |
| Competitive / fallback@4 / round-2 | 3072 | -9724 | -3.17 ±0.23 | -11817 | |
| Competitive / floor#242 / round-2 | 2705 | -9499 | -3.51 ±0.25 | -15272 | |
| Defensive / book / round-2 | 4500 | -8979 | -2.00 ±0.20 | -10766 | |
| Defensive / floor#242 / round-1 | 2389 | -8247 | -3.45 ±0.30 | -9609 | |
| Competitive / floor#3 / round-2 | 2650 | -8041 | -3.03 ±0.26 | -7967 | |
| Competitive / fallback@3 / round-1 | 1949 | -6692 | -3.43 ±0.31 | -8571 | |
| Constructive / floor#3 / round-2 | 2795 | -6373 | -2.28 ±0.22 | -5571 | |
| Constructive / floor#140 / round-2 | 2083 | -5554 | -2.67 ±0.34 | -5813 | |
| Defensive / floor#3 / balancing | 2015 | -5103 | -2.53 ±0.24 | -3145 | |
| Constructive / floor#3 / round-1 | 1315 | -4815 | -3.66 ±0.39 | -2504 | |
| Competitive / floor#3 / round-1 | 1033 | -4624 | -4.48 ±0.43 | -2147 | |
| Defensive / floor#242 / round-2 | 1136 | -4608 | -4.06 ±0.39 | -7933 | |
| Constructive / book / deep | 4805 | -4315 | -0.90 ±0.18 | -4764 | |
| Defensive / floor#242 / balancing | 1972 | -4278 | -2.17 ±0.30 | -8743 | |
| Defensive / floor#20 / round-1 | 1452 | -4172 | -2.87 ±0.33 | -5240 | |
| Defensive / floor#50 / round-1 | 1818 | -3934 | -2.16 ±0.29 | -5271 | |
| Defensive / floor#60 / round-2 | 1321 | -3785 | -2.87 ±0.32 | -5895 | |
| Competitive / floor#242 / balancing | 1857 | -3754 | -2.02 ±0.30 | -8099 | |
| Defensive / floor#35 / round-1 | 1457 | -3333 | -2.29 ±0.33 | -4514 | |
| Competitive / floor#242+rb / round-2 | 462 | -3035 | -6.57 ±0.63 | -3740 | |
| Competitive / floor#46 / round-2 | 837 | -2862 | -3.42 ±0.41 | -3969 | |
| Defensive / floor#64 / round-1 | 1165 | -2667 | -2.29 ±0.40 | -2700 | |
| Constructive / floor#61 / deep | 1247 | -2470 | -1.98 ±0.17 | -4014 | |
| Defensive / floor#20 / round-2 | 1192 | -2168 | -1.82 ±0.33 | -3198 | |
| Defensive / floor#35 / round-2 | 1095 | -2129 | -1.94 ±0.36 | -3244 | |
| Competitive / floor#3 / balancing | 997 | -2110 | -2.12 ±0.34 | -223 | |
| Constructive / floor#3 / deep | 1334 | -2033 | -1.52 ±0.36 | -1633 | |
| Defensive / floor#202 / round-2 | 432 | -1892 | -4.38 ±0.57 | -2917 | |
| Defensive / floor#45 / round-2 | 562 | -1886 | -3.36 ±0.50 | -2949 | |
| Defensive / floor#202 / round-1 | 496 | -1870 | -3.77 ±0.58 | -2360 | |
| Competitive / floor#31 / round-1 | 352 | -1750 | -4.97 ±0.72 | -1406 | |
| Competitive / floor#61 / round-1 | 230 | -1749 | -7.60 ±0.75 | -1699 | |
| Competitive / floor#46 / round-1 | 294 | -1743 | -5.93 ±0.81 | -1702 | |
| Defensive / floor#132 / round-1 | 455 | -1674 | -3.68 ±0.51 | -3197 | |
| Competitive / floor#30 / round-2 | 523 | -1665 | -3.18 ±0.48 | -2486 | |
| Defensive / floor#61 / round-2 | 436 | -1442 | -3.31 ±0.56 | -1857 | |
| Defensive / floor#65 / round-1 | 929 | -1396 | -1.50 ±0.40 | -2291 | |
| Defensive / floor#20 / balancing | 756 | -1288 | -1.70 ±0.40 | -2374 | |
| Defensive / floor#60 / round-1 | 321 | -1250 | -3.89 ±0.62 | -1906 | |
| Constructive / fallback@4 / deep | 425 | -1246 | -2.93 ±0.65 | -1394 | |
| Defensive / floor#243 / round-1 | 262 | -1217 | -4.65 ±0.91 | -1686 | |
| Constructive / fallback@5 / deep | 376 | -1172 | -3.12 ±0.75 | -1269 | |
| Defensive / floor#131 / balancing | 409 | -1167 | -2.85 ±0.57 | -1883 | |
| Defensive / floor#200 / round-1 | 294 | -1158 | -3.94 ±0.77 | -1443 | |
| Constructive / floor#61 / round-2 | 425 | -1152 | -2.71 ±0.61 | -1312 | |
| Defensive / floor#46 / round-2 | 349 | -1102 | -3.16 ±0.58 | -1606 | |
| Constructive / floor#46 / round-2 | 528 | -994 | -1.88 ±0.56 | -1103 | |
| Constructive / floor#46 / deep | 625 | -983 | -1.57 ±0.37 | -1817 | |
| Competitive / floor#16 / round-1 | 136 | -978 | -7.19 ±1.10 | -998 | |
| Competitive / fallback@2 / round-2 | 330 | -953 | -2.89 ±0.82 | -816 | |
| Defensive / floor#30 / round-2 | 323 | -887 | -2.75 ±0.63 | -1474 | |
| Competitive / floor#242 / round-1 | 425 | -875 | -2.06 ±0.60 | -2078 | |
| Competitive / floor#243 / round-2 | 202 | -828 | -4.10 ±0.86 | -1411 | |
| Constructive / floor#145 / round-2 | 162 | -803 | -4.96 ±1.28 | -814 | |
| Competitive / floor#243 / balancing | 200 | -794 | -3.97 ±0.75 | -1426 | |
| Constructive / floor#32 / round-1 | 181 | -766 | -4.23 ±0.91 | -710 | |
| Defensive / floor#35 / balancing | 517 | -763 | -1.48 ±0.45 | -1605 | |
| Defensive / floor#45 / round-1 | 244 | -761 | -3.12 ±0.85 | -1080 | |
| Defensive / floor#200 / round-2 | 277 | -753 | -2.72 ±0.77 | -1223 | |
| Competitive / floor#61 / round-2 | 160 | -744 | -4.65 ±0.90 | -1119 | |
| Defensive / floor#132 / balancing | 297 | -744 | -2.51 ±0.51 | -1808 | |
| Competitive / fallback@1 / round-2 | 299 | -701 | -2.34 ±0.86 | -587 | |
| Competitive / floor#5 / round-2 | 229 | -701 | -3.06 ±0.97 | -1074 | |
| Defensive / floor#243 / balancing | 225 | -675 | -3.00 ±0.86 | -1144 | |
| Competitive / book+rb / round-2 | 396 | -662 | -1.67 ±0.52 | -1076 | |
| Defensive / floor#197 / round-1 | 212 | -657 | -3.10 ±0.86 | -765 | |
| Defensive / floor#243 / round-2 | 166 | -655 | -3.95 ±0.96 | -1201 | |
| Defensive / floor#50 / round-2 | 660 | -651 | -0.99 ±0.42 | -1273 | |
| Defensive / floor#16 / round-2 | 187 | -649 | -3.47 ±0.86 | -843 | |
| Constructive / floor#140 / deep | 486 | -633 | -1.30 ±0.62 | -701 | |
| Competitive / floor#240 / balancing | 190 | -602 | -3.17 ±0.98 | -611 | |
| Defensive / floor#3 / deep | 237 | -596 | -2.51 ±0.80 | -495 | |
| Competitive / floor#6 / round-2 | 173 | -595 | -3.44 ±1.00 | -1104 | |
| Defensive / floor#51 / round-1 | 244 | -585 | -2.40 ±0.88 | -532 | |
| Competitive / floor#240 / round-2 | 202 | -546 | -2.70 ±0.98 | -599 | |
| Defensive / floor#199 / round-1 | 268 | -545 | -2.03 ±0.73 | -656 | |
| Defensive / floor#66 / round-1 | 179 | -528 | -2.95 ±1.04 | -648 | |
| Defensive / floor#21 / round-1 | 182 | -513 | -2.82 ±0.98 | -884 | |
| Constructive / floor#17 / round-1 | 134 | -509 | -3.80 ±1.12 | -403 | |
| Defensive / floor#64 / round-2 | 303 | -503 | -1.66 ±0.70 | -839 | |
| Competitive / floor#239 / round-2 | 117 | -500 | -4.27 ±1.10 | -695 | |
| Defensive / floor#131 / round-1 | 142 | -466 | -3.28 ±1.07 | -772 | |
| Defensive / floor#50 / balancing | 403 | -428 | -1.06 ±0.46 | -1344 | |
| Competitive / floor#237 / round-2 | 153 | -423 | -2.76 ±0.86 | -767 | |
| Defensive / floor#198 / round-2 | 115 | -417 | -3.63 ±1.14 | -620 | |
| Defensive / floor#49 / round-1 | 188 | -407 | -2.16 ±0.94 | -274 | |
| Competitive / floor#241 / round-2 | 90 | -398 | -4.42 ±1.25 | -511 | |
| Defensive / floor#30 / round-1 | 117 | -371 | -3.17 ±1.07 | -411 | |
| Competitive / floor#16 / round-2 | 91 | -370 | -4.07 ±1.48 | -407 | |
| Constructive / floor#16 / round-2 | 68 | -368 | -5.41 ±1.46 | -347 | |
| Defensive / floor#65 / balancing | 242 | -366 | -1.51 ±0.59 | -1108 | |
| Competitive / floor#242 / deep | 48 | -362 | -7.54 ±1.79 | -525 | |
| Competitive / floor#30 / balancing | 127 | -356 | -2.80 ±0.88 | -687 | |
| Defensive / floor#198 / round-1 | 158 | -342 | -2.16 ±1.13 | -506 | |
| Defensive / floor#61 / round-1 | 92 | -337 | -3.66 ±1.61 | -356 | |
| Competitive / floor#3 / deep | 187 | -323 | -1.73 ±0.93 | -372 | |
| Competitive / floor#15 / balancing | 81 | -319 | -3.94 ±1.14 | -493 | |
| Constructive / floor#31 / round-2 | 78 | -318 | -4.08 ±1.52 | -271 | |
| Competitive / floor#238 / balancing | 158 | -314 | -1.99 ±1.10 | -350 | |
| Defensive / floor#49 / balancing | 192 | -313 | -1.63 ±0.89 | -515 | |
| Competitive / floor#46 / deep | 109 | -309 | -2.83 ±1.01 | -554 | |
| Competitive / floor#235 / round-2 | 63 | -300 | -4.76 ±1.27 | -440 | |
| Defensive / floor#36 / round-1 | 144 | -298 | -2.07 ±1.00 | -598 | |
| Defensive / floor#129 / round-2 | 181 | -297 | -1.64 ±1.16 | -599 | |
| Competitive / floor#1 / round-2 | 372 | -292 | -0.78 ±0.76 | -334 | |
| Defensive / floor#133 / round-1 | 315 | -290 | -0.92 ±0.92 | -718 | ~noise |
| Defensive / floor#31 / round-2 | 167 | -288 | -1.72 ±0.97 | -349 | |
| Competitive / floor#31 / round-2 | 73 | -286 | -3.92 ±1.46 | -317 | |
| Competitive / floor#10 / round-2 | 72 | -284 | -3.94 ±1.28 | -350 | |
| Defensive / floor#17 / round-1 | 74 | -274 | -3.70 ±1.55 | -212 | |
| Defensive / floor#32 / round-1 | 47 | -273 | -5.81 ±2.13 | -218 | |
| Competitive / floor#236 / balancing | 153 | -268 | -1.75 ±0.92 | -344 | |
| Competitive / floor#15 / round-2 | 58 | -267 | -4.60 ±1.36 | -399 | |
| Defensive / floor#205 / round-1 | 97 | -253 | -2.61 ±1.35 | -240 | |
| Competitive / floor#241 / balancing | 96 | -247 | -2.57 ±1.21 | -407 | |
| Constructive / floor#151 / round-2 | 88 | -242 | -2.75 ±2.16 | -236 | |
| Competitive / fallback@5 / round-2 | 100 | -235 | -2.35 ±1.14 | -171 | |
| Constructive / floor#140 / round-1 | 18 | -229 | -12.72 ±0.82 | -229 | |
| Competitive / floor#234 / balancing | 91 | -227 | -2.49 ±1.25 | -295 | |
| Competitive / floor#31 / balancing | 34 | -226 | -6.65 ±1.75 | -273 | |
| Defensive / floor#204 / round-1 | 84 | -221 | -2.63 ±1.55 | -194 | |
| Competitive / floor#61 / deep | 95 | -218 | -2.29 ±1.07 | -500 | |
| Competitive / floor#238 / round-2 | 80 | -194 | -2.42 ±1.67 | -214 | |
| Defensive / floor#27 / round-2 | 41 | -192 | -4.68 ±1.57 | -228 | |
| Competitive / floor#240+rb / round-2 | 57 | -189 | -3.32 ±1.90 | -194 | |
| Competitive / floor#234+rb / round-2 | 19 | -177 | -9.32 ±2.14 | -212 | |
| Competitive / floor#16 / balancing | 52 | -173 | -3.33 ±1.60 | -297 | |
| Defensive / floor#237 / round-2 | 52 | -173 | -3.33 ±1.67 | -265 | |
| Defensive / floor#242 / deep | 33 | -166 | -5.03 ±1.91 | -309 | |
| Competitive / floor#3+rb / round-2 | 146 | -165 | -1.13 ±1.05 | -163 | |
| Defensive / floor#63 / round-2 | 35 | -160 | -4.57 ±2.24 | -144 | |
| Defensive / floor#203 / round-1 | 78 | -157 | -2.01 ±1.34 | -154 | |
| Competitive / floor#32 / round-1 | 24 | -155 | -6.46 ±2.64 | -150 | |
| Defensive / floor#17 / round-2 | 71 | -155 | -2.18 ±1.23 | -231 | |
| Competitive / floor#236+rb / round-2 | 37 | -149 | -4.03 ±2.54 | -163 | |
| Competitive / floor#9 / round-2 | 48 | -149 | -3.10 ±1.33 | -224 | |
| Competitive / floor#55 / round-2 | 31 | -148 | -4.77 ±2.20 | -155 | |
| Defensive / floor#64 / balancing | 231 | -148 | -0.64 ±0.77 | -412 | ~noise |
| Competitive / floor#16 / deep | 36 | -140 | -3.89 ±1.48 | -163 | |
| Competitive / floor#25 / round-2 | 100 | -140 | -1.40 ±1.09 | -107 | |
| Constructive / floor#147 / round-2 | 37 | -140 | -3.78 ±3.22 | -140 | |
| Defensive / floor#12 / round-2 | 43 | -139 | -3.23 ±1.77 | -172 | |
| Competitive / floor#30 / deep | 37 | -138 | -3.73 ±1.67 | -211 | |
| Competitive / floor#236 / round-2 | 47 | -135 | -2.87 ±2.00 | -152 | |
| Defensive / floor#239 / round-2 | 46 | -132 | -2.87 ±2.28 | -243 | |
| Competitive / floor#46+rb / deep | 40 | -129 | -3.23 ±0.97 | -259 | |
| Defensive / floor#153 / round-1 | 20 | -128 | -6.40 ±4.27 | -99 | |
| Competitive / floor#140 / round-1 | 40 | -126 | -3.15 ±2.99 | -123 | |
| Competitive / floor#234 / round-2 | 52 | -125 | -2.40 ±1.93 | -177 | |
| Defensive / floor#48 / round-2 | 26 | -125 | -4.81 ±2.22 | -170 | |
| Defensive / floor#197 / round-2 | 87 | -122 | -1.40 ±1.43 | -210 | ~noise |
| Defensive / floor#235 / round-2 | 42 | -121 | -2.88 ±1.58 | -188 | |
| Constructive / floor#47 / round-2 | 12 | -120 | -10.00 ±2.70 | -120 | |
| Competitive / floor#17 / deep | 25 | -115 | -4.60 ±2.34 | -141 | |
| Competitive / floor#39 / round-2 | 20 | -115 | -5.75 ±1.63 | -163 | |
| Defensive / floor#51 / balancing | 51 | -115 | -2.25 ±1.34 | -140 | |
| Defensive / floor#42 / round-2 | 37 | -110 | -2.97 ±3.30 | -65 | ~noise |
| Competitive / floor#63 / round-2 | 23 | -109 | -4.74 ±2.12 | -179 | |
| Competitive / floor#57 / round-2 | 14 | -108 | -7.71 ±3.67 | -109 | |
| Defensive / floor#11 / round-2 | 23 | -108 | -4.70 ±1.36 | -179 | |
| Competitive / floor#12 / round-2 | 33 | -107 | -3.24 ±1.90 | -173 | |
| Defensive / floor#65 / round-2 | 281 | -107 | -0.38 ±0.66 | -392 | ~noise |
| Competitive / floor#47 / round-2 | 90 | -106 | -1.18 ±1.61 | -383 | ~noise |
| Competitive / floor#42 / round-2 | 22 | -105 | -4.77 ±3.80 | -144 | |
| Competitive / floor#238+rb / round-2 | 41 | -103 | -2.51 ±1.97 | -131 | |
| Constructive / floor#30 / round-1 | 48 | -103 | -2.15 ±1.78 | -146 | |
| Competitive / floor#33 / round-2 | 26 | -101 | -3.88 ±2.08 | -151 | |
| Defensive / floor#204 / round-2 | 41 | -98 | -2.39 ±1.88 | -82 | |
| Competitive / floor#242+rb / balancing | 13 | -97 | -7.46 ±3.76 | -138 | |
| Constructive / floor#157 / round-2 | 45 | -97 | -2.16 ±2.25 | -90 | ~noise |
| Defensive / floor#1 / round-2 | 221 | -97 | -0.44 ±1.08 | -112 | ~noise |
| Competitive / floor#2 / round-2 | 128 | -96 | -0.75 ±1.27 | -362 | ~noise |
| Defensive / floor#147 / round-1 | 12 | -94 | -7.83 ±4.79 | -86 | |
| Competitive / floor#239 / balancing | 58 | -91 | -1.57 ±1.29 | -205 | |
| Constructive / floor#62 / round-1 | 36 | -91 | -2.53 ±2.50 | -99 | |
| Defensive / floor#20 / deep | 21 | -91 | -4.33 ±1.46 | -159 | |
| Defensive / floor#129 / round-1 | 59 | -90 | -1.53 ±1.75 | -187 | ~noise |
| Defensive / floor#133 / balancing | 83 | -90 | -1.08 ±1.67 | -250 | ~noise |
| Defensive / floor#33 / round-2 | 22 | -90 | -4.09 ±2.75 | -54 | |
| Competitive / floor#31 / deep | 48 | -89 | -1.85 ±1.20 | -156 | |
| Competitive / floor#129 / deep | 10 | -87 | -8.70 ±3.34 | -100 | |
| Competitive / floor#237 / balancing | 48 | -87 | -1.81 ±1.36 | -232 | |
| Defensive / floor#21 / balancing | 62 | -86 | -1.39 ±1.24 | -205 | |
| Competitive / floor#17 / round-1 | 11 | -83 | -7.55 ±3.66 | -59 | |
| Competitive / floor#60 / balancing | 39 | -83 | -2.13 ±2.30 | -119 | ~noise |
| Constructive / floor#31 / deep | 20 | -82 | -4.10 ±2.40 | -81 | |
| Defensive / floor#66 / balancing | 21 | -82 | -3.90 ±2.28 | -100 | |
| Competitive / floor#243+rb / round-2 | 17 | -81 | -4.76 ±3.08 | -158 | |
| Competitive / floor#140 / round-2 | 33 | -79 | -2.39 ±2.61 | -84 | ~noise |
| Defensive / floor#238 / round-2 | 11 | -78 | -7.09 ±4.06 | -112 | |
| Competitive / floor#33 / round-1 | 12 | -76 | -6.33 ±3.23 | -108 | |
| Competitive / floor#11 / round-2 | 6 | -74 | -12.33 ±1.49 | -94 | |
| Constructive / floor#62 / round-2 | 8 | -72 | -9.00 ±3.90 | -72 | |
| Competitive / floor#241 / deep | 20 | -70 | -3.50 ±3.02 | -99 | |
| Defensive / floor#240 / round-2 | 22 | -70 | -3.18 ±2.11 | -81 | |
| Defensive / floor#26 / round-2 | 11 | -70 | -6.36 ±3.40 | -97 | |
| Competitive / fallback@6 / round-2 | 18 | -69 | -3.83 ±3.54 | -44 | |
| Competitive / floor#235 / deep | 9 | -69 | -7.67 ±3.61 | -84 | |
| Defensive / floor#33 / round-1 | 12 | -67 | -5.58 ±4.15 | -73 | |
| Constructive / floor#32 / deep | 59 | -65 | -1.10 ±0.60 | -76 | |
| Defensive / floor#46 / round-1 | 29 | -65 | -2.24 ±2.97 | -69 | ~noise |
| Defensive / floor#239 / balancing | 26 | -64 | -2.46 ±2.48 | -78 | ~noise |
| Constructive / floor#17 / deep | 74 | -62 | -0.84 ±0.49 | -72 | |
| Competitive / floor#242+rb / deep | 16 | -61 | -3.81 ±4.17 | -126 | ~noise |
| Defensive / floor#49 / round-2 | 72 | -61 | -0.85 ±1.37 | -99 | ~noise |
| Competitive / floor#27 / round-2 | 13 | -60 | -4.62 ±4.14 | -85 | |
| Competitive / floor#3+rb / deep | 23 | -60 | -2.61 ±2.38 | -92 | |
| Defensive / floor#1 / deep | 35 | -59 | -1.69 ±2.35 | -183 | ~noise |
| Competitive / book+rb / deep | 59 | -56 | -0.95 ±0.86 | -87 | |
| Constructive / floor#153 / round-2 | 19 | -56 | -2.95 ±4.44 | -46 | ~noise |
| Defensive / floor#32 / round-2 | 55 | -54 | -0.98 ±1.53 | -111 | ~noise |
| Defensive / floor#34 / balancing | 151 | -54 | -0.36 ±0.96 | -122 | ~noise |
| Competitive / floor#129 / round-2 | 25 | -53 | -2.12 ±3.20 | -115 | ~noise |
| Defensive / floor#36 / balancing | 36 | -52 | -1.44 ±1.99 | -122 | ~noise |
| Competitive / floor#24 / round-2 | 14 | -51 | -3.64 ±2.79 | -68 | |
| Competitive / floor#45 / balancing | 13 | -51 | -3.92 ±5.18 | -31 | ~noise |
| Defensive / floor#203 / round-2 | 51 | -51 | -1.00 ±1.84 | -44 | ~noise |
| Competitive / floor#17 / round-2 | 6 | -50 | -8.33 ±2.00 | -62 | |
| Competitive / floor#237 / deep | 15 | -50 | -3.33 ±2.20 | -114 | |
| Defensive / floor#41 / round-2 | 45 | -50 | -1.11 ±2.50 | -81 | ~noise |
| Defensive / floor#55 / round-2 | 17 | -50 | -2.94 ±2.97 | -49 | ~noise |
| Competitive / floor#54 / round-2 | 16 | -49 | -3.06 ±3.83 | -42 | ~noise |
| Competitive / floor#153 / round-2 | 9 | -46 | -5.11 ±6.82 | -45 | ~noise |
| Defensive / floor#16 / round-1 | 9 | -46 | -5.11 ±4.33 | -48 | |
| Defensive / floor#36 / round-2 | 23 | -46 | -2.00 ±2.28 | -80 | ~noise |
| Competitive / floor#153 / round-1 | 3 | -45 | -15.00 ±2.99 | -45 | |
| Constructive / floor#157 / round-1 | 4 | -45 | -11.25 ±1.67 | -45 | |
| Constructive / floor#47 / round-1 | 34 | -45 | -1.32 ±2.89 | -43 | ~noise |
| Defensive / floor#10 / round-2 | 4 | -45 | -11.25 ±5.21 | -57 | |
| Competitive / floor#48 / round-2 | 29 | -44 | -1.52 ±2.78 | -115 | ~noise |
| Defensive / floor#151 / round-1 | 10 | -44 | -4.40 ±6.57 | -47 | ~noise |
| Defensive / floor#21 / round-2 | 49 | -43 | -0.88 ±1.76 | -206 | ~noise |
| Defensive / floor#40 / round-2 | 9 | -42 | -4.67 ±4.87 | -56 | ~noise |
| Defensive / floor#18 / round-2 | 19 | -41 | -2.16 ±3.63 | -23 | ~noise |
| Defensive / floor#47 / round-1 | 7 | -41 | -5.86 ±6.92 | -37 | ~noise |
| Competitive / floor#47 / balancing | 45 | -39 | -0.87 ±2.06 | -78 | ~noise |
| Competitive / floor#6 / deep | 3 | -39 | -13.00 ±5.99 | -41 | |
| Competitive / floor#60+rb / round-2 | 8 | -39 | -4.88 ±3.46 | -53 | |
| Defensive / book / deep | 45 | -39 | -0.87 ±1.40 | -49 | ~noise |
| Competitive / floor#18 / round-2 | 18 | -38 | -2.11 ±2.80 | -15 | ~noise |
| Competitive / floor#40 / round-2 | 31 | -38 | -1.23 ±2.61 | -59 | ~noise |
| Defensive / floor#29 / round-1 | 7 | -38 | -5.43 ±5.69 | -36 | ~noise |
| Competitive / floor#5 / deep | 13 | -36 | -2.77 ±3.00 | -65 | ~noise |
| Defensive / floor#241 / round-2 | 7 | -36 | -5.14 ±4.24 | -34 | |
| Competitive / floor#61+rb / deep | 9 | -35 | -3.89 ±2.34 | -49 | |
| Defensive / floor#236 / round-2 | 4 | -35 | -8.75 ±2.02 | -35 | |
| Defensive / floor#48 / round-1 | 13 | -34 | -2.62 ±5.09 | -50 | ~noise |
| Competitive / floor#47+rb / balancing | 4 | -33 | -8.25 ±4.48 | -33 | |
| Constructive / floor#147 / deep | 24 | -33 | -1.38 ±4.11 | -20 | ~noise |
| Defensive / floor#31 / round-1 | 13 | -33 | -2.54 ±3.50 | -45 | ~noise |
| Defensive / floor#54 / deep | 9 | -33 | -3.67 ±3.62 | -62 | |
| Constructive / floor#63 / round-2 | 10 | -32 | -3.20 ±3.74 | -60 | ~noise |
| Defensive / floor#29 / round-2 | 8 | -32 | -4.00 ±5.12 | -26 | ~noise |
| Defensive / floor#241 / deep | 8 | -31 | -3.88 ±5.22 | -38 | ~noise |
| Competitive / floor#241+rb / deep | 5 | -30 | -6.00 ±6.59 | -31 | ~noise |
| Competitive / floor#243 / round-1 | 9 | -30 | -3.33 ±2.69 | -40 | |
| Competitive / floor#60 / round-2 | 5 | -30 | -6.00 ±3.45 | -42 | |
| Constructive / floor#16 / deep | 4 | -30 | -7.50 ±2.47 | -38 | |
| Defensive / floor#5 / round-1 | 32 | -30 | -0.94 ±2.68 | -16 | ~noise |
| Competitive / floor#235 / balancing | 11 | -29 | -2.64 ±3.16 | -57 | ~noise |
| Competitive / floor#39 / deep | 13 | -29 | -2.23 ±0.83 | -68 | |
| Competitive / floor#48 / round-1 | 6 | -29 | -4.83 ±6.14 | -18 | ~noise |
| Defensive / floor#218 / round-2 | 7 | -29 | -4.14 ±1.31 | -45 | |
| Defensive / floor#56 / round-1 | 12 | -29 | -2.42 ±4.97 | -36 | ~noise |
| Defensive / floor#6 / round-2 | 37 | -28 | -0.76 ±2.38 | -68 | ~noise |
| Competitive / floor#45+rb / round-2 | 4 | -27 | -6.75 ±4.83 | -52 | |
| Defensive / floor#61 / balancing | 22 | -27 | -1.23 ±2.24 | -68 | ~noise |
| Competitive / floor#57 / deep | 2 | -26 | -13.00 ±1.96 | -29 | |
| Competitive / floor#62 / round-1 | 3 | -26 | -8.67 ±9.49 | -42 | ~noise |
| Defensive / floor#140 / balancing | 4 | -26 | -6.50 ±5.09 | -26 | |
| Defensive / floor#205 / round-2 | 76 | -26 | -0.34 ±1.49 | -78 | ~noise |
| Competitive / floor#140+rb / round-2 | 5 | -25 | -5.00 ±6.07 | -25 | ~noise |
| Competitive / floor#239+rb / deep | 5 | -25 | -5.00 ±9.02 | -30 | ~noise |
| Competitive / floor#62 / deep | 12 | -25 | -2.08 ±3.29 | -41 | ~noise |
| Defensive / floor#237 / balancing | 25 | -25 | -1.00 ±2.12 | -74 | ~noise |
| Competitive / floor#145 / balancing | 4 | -24 | -6.00 ±6.84 | -24 | ~noise |
| Competitive / floor#241+rb / balancing | 10 | -24 | -2.40 ±3.05 | -44 | ~noise |
| Competitive / floor#63 / round-1 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Constructive / floor#145 / round-1 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Constructive / floor#148 / round-2 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Defensive / floor#211 / round-2 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Defensive / floor#229 / round-2 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Defensive / floor#35 / deep | 13 | -24 | -1.85 ±3.39 | -43 | ~noise |
| Competitive / floor#147 / round-1 | 6 | -23 | -3.83 ±5.34 | -18 | ~noise |
| Competitive / floor#143 / round-2 | 2 | -22 | -11.00 ±1.96 | -24 | |
| Defensive / floor#228 / round-2 | 7 | -22 | -3.14 ±4.69 | -14 | ~noise |
| Defensive / floor#27 / round-1 | 5 | -22 | -4.40 ±5.42 | -17 | ~noise |
| Competitive / floor#235+rb / balancing | 2 | -21 | -10.50 ±2.94 | -21 | |
| Defensive / floor#208 / round-2 | 7 | -21 | -3.00 ±4.86 | -12 | ~noise |
| Defensive / floor#54 / round-2 | 5 | -21 | -4.20 ±1.57 | -45 | |
| Competitive / fallback@2 / balancing | 2 | -20 | -10.00 ±1.96 | -20 | |
| Competitive / floor#18 / round-1 | 15 | -20 | -1.33 ±4.04 | +2 | ~noise plain/PD-flip |
| Competitive / floor#45 / round-2 | 2 | -20 | -10.00 ±1.96 | -18 | |
| Defensive / floor#227 / round-2 | 4 | -20 | -5.00 ±6.79 | -19 | ~noise |
| Competitive / floor#239+rb / balancing | 4 | -19 | -4.75 ±6.47 | -40 | ~noise |
| Competitive / floor#46 / balancing | 12 | -19 | -1.58 ±2.87 | -70 | ~noise |
| Defensive / floor#32 / deep | 7 | -19 | -2.71 ±1.46 | -36 | |
| Defensive / floor#56 / deep | 3 | -19 | -6.33 ±2.61 | -38 | |
| Competitive / floor#145 / round-2 | 3 | -18 | -6.00 ±6.88 | -25 | ~noise |
| Competitive / floor#147 / round-2 | 25 | -18 | -0.72 ±3.76 | -21 | ~noise |
| Competitive / floor#241+rb / round-2 | 3 | -18 | -6.00 ±11.81 | -16 | ~noise |
| Competitive / floor#63+rb / round-2 | 2 | -18 | -9.00 ±5.88 | -10 | |
| Defensive / floor#5 / round-2 | 55 | -18 | -0.33 ±1.93 | -33 | ~noise |
| Competitive / floor#62+rb / round-2 | 4 | -17 | -4.25 ±11.61 | -4 | ~noise |
| Constructive / floor#17 / round-2 | 2 | -17 | -8.50 ±2.94 | -17 | |
| Defensive / floor#14 / round-2 | 5 | -17 | -3.40 ±8.02 | -14 | ~noise |
| Defensive / floor#61 / deep | 23 | -17 | -0.74 ±2.02 | -103 | ~noise |
| Competitive / fallback@4+rb / round-2 | 4 | -16 | -4.00 ±2.89 | -19 | |
| Defensive / floor#129 / deep | 21 | -15 | -0.71 ±3.32 | -10 | ~noise |
| Defensive / floor#17 / deep | 4 | -15 | -3.75 ±7.52 | -12 | ~noise |
| Defensive / floor#50 / deep | 12 | -15 | -1.25 ±2.23 | -15 | ~noise |
| Competitive / floor#135 / round-2 | 2 | -14 | -7.00 ±3.92 | -26 | |
| Competitive / floor#140 / deep | 2 | -14 | -7.00 ±1.96 | -22 | |
| Competitive / floor#24 / deep | 5 | -14 | -2.80 ±4.04 | -17 | ~noise |
| Competitive / floor#61+rb / balancing | 2 | -14 | -7.00 ±3.92 | -14 | |
| Constructive / floor#32 / round-2 | 4 | -14 | -3.50 ±11.94 | -6 | ~noise |
| Defensive / floor#11 / deep | 2 | -14 | -7.00 ±13.72 | -14 | ~noise |
| Defensive / floor#46 / deep | 4 | -14 | -3.50 ±1.27 | -24 | |
| Competitive / floor#145 / round-1 | 1 | -13 | -13.00 ±0.00 | -13 | ~noise |
| Competitive / floor#47 / round-1 | 1 | -13 | -13.00 ±0.00 | -13 | ~noise |
| Competitive / floor#63+rb / deep | 2 | -13 | -6.50 ±8.82 | -14 | ~noise |
| Defensive / floor#18 / deep | 5 | -13 | -2.60 ±2.11 | -24 | |
| Defensive / floor#26 / deep | 1 | -13 | -13.00 ±0.00 | -14 | ~noise |
| Competitive / floor#26 / deep | 1 | -12 | -12.00 ±0.00 | -14 | ~noise |
| Defensive / floor#28 / round-1 | 2 | -12 | -6.00 ±1.96 | -14 | |
| Defensive / floor#55 / deep | 2 | -12 | -6.00 ±11.76 | -12 | ~noise |
| Competitive / floor#12 / deep | 4 | -11 | -2.75 ±3.34 | -14 | ~noise |
| Competitive / floor#45+rb / deep | 2 | -11 | -5.50 ±2.94 | -21 | |
| Competitive / floor#56 / round-2 | 1 | -11 | -11.00 ±0.00 | -12 | ~noise |
| Constructive / floor#147 / round-1 | 1 | -11 | -11.00 ±0.00 | -11 | ~noise |
| Defensive / floor#1 / round-1 | 18 | -11 | -0.61 ±4.32 | -8 | ~noise |
| Defensive / floor#230 / round-2 | 2 | -11 | -5.50 ±0.98 | -14 | |
| Defensive / floor#26 / round-1 | 1 | -11 | -11.00 ±0.00 | -14 | ~noise |
| Defensive / floor#41 / deep | 3 | -11 | -3.67 ±5.23 | -9 | ~noise |
| Defensive / floor#63 / deep | 2 | -11 | -5.50 ±2.94 | -14 | |
| Competitive / floor#151 / round-2 | 10 | -10 | -1.00 ±5.31 | -11 | ~noise |
| Competitive / floor#30+rb / round-2 | 3 | -10 | -3.33 ±3.46 | -26 | ~noise |
| Defensive / floor#13 / round-2 | 4 | -10 | -2.50 ±11.02 | -11 | ~noise |
| Defensive / floor#212 / round-2 | 1 | -10 | -10.00 ±0.00 | -10 | ~noise |
| Defensive / floor#27 / deep | 2 | -10 | -5.00 ±1.96 | -18 | |
| Defensive / floor#28 / round-2 | 1 | -10 | -10.00 ±0.00 | -10 | ~noise |
| Competitive / fallback@3 / balancing | 1 | -9 | -9.00 ±0.00 | -9 | ~noise |
| Competitive / floor#3+rb / balancing | 6 | -9 | -1.50 ±1.88 | -12 | ~noise |
| Competitive / floor#32 / deep | 5 | -9 | -1.80 ±2.18 | -12 | ~noise |
| Competitive / floor#47+rb / deep | 1 | -9 | -9.00 ±0.00 | -12 | ~noise |
| Constructive / floor#33 / deep | 2 | -9 | -4.50 ±0.98 | -12 | |
| Competitive / floor#237+rb / balancing | 4 | -8 | -2.00 ±10.03 | -5 | ~noise |
| Constructive / floor#157 / deep | 20 | -8 | -0.40 ±3.23 | -1 | ~noise |
| Defensive / floor#231 / round-2 | 6 | -8 | -1.33 ±6.41 | -30 | ~noise |
| Defensive / floor#237 / deep | 2 | -8 | -4.00 ±3.92 | -22 | |
| Competitive / floor#235+rb / deep | 5 | -7 | -1.40 ±2.02 | -40 | ~noise |
| Competitive / floor#54 / deep | 3 | -7 | -2.33 ±4.57 | -1 | ~noise |
| Defensive / floor#62 / deep | 5 | -7 | -1.40 ±6.58 | -17 | ~noise |
| Competitive / floor#144 / round-1 | 1 | -6 | -6.00 ±0.00 | -6 | ~noise |
| Competitive / floor#144 / round-2 | 7 | -6 | -0.86 ±6.73 | -6 | ~noise |
| Defensive / floor#199 / round-2 | 62 | -6 | -0.10 ±1.38 | -74 | ~noise |
| Defensive / floor#238 / balancing | 1 | -6 | -6.00 ±0.00 | -6 | ~noise |
| Defensive / floor#39 / round-2 | 2 | -6 | -3.00 ±3.92 | -21 | ~noise |
| Defensive / floor#219 / round-2 | 1 | -5 | -5.00 ±0.00 | -6 | ~noise |
| Defensive / floor#25 / round-2 | 2 | -5 | -2.50 ±4.90 | -6 | ~noise |
| Defensive / floor#48 / deep | 2 | -5 | -2.50 ±0.98 | -12 | |
| Competitive / floor#56 / deep | 4 | -4 | -1.00 ±3.84 | -1 | ~noise |
| Competitive / floor#60+rb / deep | 3 | -4 | -1.33 ±3.64 | -15 | ~noise |
| Defensive / floor#235 / deep | 5 | -3 | -0.60 ±1.18 | -17 | ~noise |
| Defensive / floor#240 / deep | 1 | -3 | -3.00 ±0.00 | -9 | ~noise |
| Defensive / floor#9 / round-2 | 1 | -3 | -3.00 ±0.00 | -9 | ~noise |
| Competitive / fallback@4 / balancing | 1 | -2 | -2.00 ±0.00 | -2 | ~noise |
| Competitive / floor#18+rb / balancing | 2 | -2 | -1.00 ±1.96 | -8 | ~noise |
| Competitive / floor#237+rb / deep | 1 | -2 | -2.00 ±0.00 | -2 | ~noise |
| Defensive / floor#128 / deep | 2 | -2 | -1.00 ±3.92 | -7 | ~noise |
| Defensive / floor#235 / balancing | 15 | -2 | -0.13 ±2.39 | -32 | ~noise |
| Defensive / floor#31 / deep | 5 | -2 | -0.40 ±4.32 | +0 | ~noise plain/PD-flip |
| Defensive / floor#41 / round-1 | 4 | -2 | -0.50 ±7.61 | +14 | ~noise plain/PD-flip |
| Competitive / floor#46+rb / round-2 | 6 | -1 | -0.17 ±4.28 | -14 | ~noise |
| Defensive / floor#18 / round-1 | 9 | -1 | -0.11 ±6.23 | +5 | ~noise plain/PD-flip |
| Defensive / floor#238 / deep | 2 | -1 | -0.50 ±2.94 | -12 | ~noise |
| Competitive / floor#143 / balancing | 2 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Competitive / floor#41 / deep | 1 | +0 | +0.00 ±0.00 | +7 | ~noise |
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
| Defensive / floor#42 / deep | 3 | +4 | +1.33 ±11.33 | +16 | ~noise |
| Competitive / floor#15 / deep | 6 | +6 | +1.00 ±3.47 | +2 | ~noise |
| Competitive / floor#40 / deep | 1 | +6 | +6.00 ±0.00 | +8 | ~noise |
| Competitive / floor#62 / balancing | 5 | +6 | +1.20 ±5.59 | +43 | ~noise |
| Defensive / floor#14 / round-1 | 3 | +6 | +2.00 ±9.87 | +12 | ~noise |
| Defensive / floor#60 / deep | 6 | +6 | +1.00 ±2.58 | +3 | ~noise |
| Competitive / floor#32 / balancing | 1 | +7 | +7.00 ±0.00 | +7 | ~noise |
| Constructive / floor#48 / deep | 8 | +7 | +0.88 ±4.63 | -7 | ~noise plain/PD-flip |
| Defensive / floor#241 / balancing | 22 | +7 | +0.32 ±2.19 | -29 | ~noise plain/PD-flip |
| Defensive / floor#145 / round-1 | 10 | +9 | +0.90 ±6.21 | +38 | ~noise |
| Defensive / floor#34 / round-1 | 1 | +9 | +9.00 ±0.00 | +9 | ~noise |
| Defensive / floor#38 / round-2 | 2 | +9 | +4.50 ±0.98 | +12 | |
| Defensive / floor#63 / round-1 | 10 | +9 | +0.90 ±5.44 | +0 | ~noise |
| Competitive / floor#60 / deep | 3 | +10 | +3.33 ±6.53 | +15 | ~noise |
| Constructive / floor#145 / deep | 19 | +10 | +0.53 ±4.07 | +10 | ~noise |
| Constructive / floor#47 / deep | 15 | +10 | +0.67 ±3.27 | -12 | ~noise plain/PD-flip |
| Constructive / floor#48 / round-2 | 1 | +10 | +10.00 ±0.00 | +10 | ~noise |
| Defensive / floor#47 / balancing | 2 | +10 | +5.00 ±0.00 | +10 | |
| Defensive / floor#57 / deep | 1 | +10 | +10.00 ±0.00 | +4 | ~noise |
| Defensive / floor#40 / deep | 4 | +11 | +2.75 ±10.87 | +0 | ~noise |
| Defensive / floor#47 / deep | 5 | +12 | +2.40 ±5.87 | +17 | ~noise |
| Competitive / floor#32 / round-2 | 36 | +15 | +0.42 ±2.00 | -24 | ~noise plain/PD-flip |
| Competitive / floor#151 / balancing | 4 | +16 | +4.00 ±4.80 | +16 | ~noise |
| Competitive / floor#239 / deep | 17 | +16 | +0.94 ±2.85 | -63 | ~noise plain/PD-flip |
| Competitive / floor#63 / deep | 2 | +16 | +8.00 ±3.92 | +16 | |
| Defensive / floor#0 / round-1 | 3 | +16 | +5.33 ±2.85 | +17 | |
| Defensive / floor#145 / balancing | 2 | +16 | +8.00 ±3.92 | +16 | |
| Defensive / floor#16 / deep | 4 | +16 | +4.00 ±6.55 | +3 | ~noise |
| Defensive / floor#2 / round-1 | 7 | +16 | +2.29 ±6.72 | +17 | ~noise |
| Defensive / floor#57 / round-2 | 11 | +16 | +1.45 ±6.18 | +25 | ~noise |
| Competitive / floor#61 / balancing | 4 | +17 | +4.25 ±3.03 | +29 | |
| Competitive / floor#41 / round-2 | 11 | +21 | +1.91 ±6.23 | -35 | ~noise plain/PD-flip |
| Competitive / floor#18+rb / deep | 2 | +24 | +12.00 ±1.96 | +26 | |
| Defensive / floor#62 / round-1 | 7 | +24 | +3.43 ±6.77 | +21 | ~noise |
| Competitive / floor#151 / round-1 | 4 | +26 | +6.50 ±6.28 | +26 | |
| Constructive / floor#148 / deep | 4 | +26 | +6.50 ±6.28 | +29 | |
| Constructive / floor#62 / deep | 25 | +27 | +1.08 ±2.61 | -5 | ~noise plain/PD-flip |
| Competitive / floor#47+rb / round-2 | 2 | +30 | +15.00 ±1.96 | +30 | |
| Constructive / floor#153 / deep | 4 | +32 | +8.00 ±2.26 | +32 | |
| Defensive / floor#6 / round-1 | 19 | +32 | +1.68 ±3.40 | +37 | ~noise |
| Defensive / floor#210 / round-2 | 4 | +35 | +8.75 ±3.78 | +38 | |
| Defensive / floor#42 / round-1 | 4 | +38 | +9.50 ±2.59 | +51 | |
| Defensive / floor#207 / round-2 | 8 | +45 | +5.62 ±2.92 | +65 | |
| Competitive / floor#62 / round-2 | 49 | +48 | +0.98 ±1.86 | -247 | ~noise plain/PD-flip |
| Defensive / floor#62 / round-2 | 84 | +57 | +0.68 ±1.48 | -26 | ~noise plain/PD-flip |
| Defensive / floor#140 / round-1 | 28 | +62 | +2.21 ±3.85 | +79 | ~noise |
| Defensive / floor#56 / round-2 | 18 | +77 | +4.28 ±3.35 | +84 | |
| Competitive / floor#47 / deep | 24 | +80 | +3.33 ±2.26 | +43 | |
| Defensive / floor#47 / round-2 | 138 | +80 | +0.58 ±1.22 | -97 | ~noise plain/PD-flip |
| Competitive / floor#1 / deep | 134 | +113 | +0.84 ±1.03 | +75 | ~noise |
| Defensive / floor#0 / round-2 | 29 | +116 | +4.00 ±2.53 | +97 | |
| Competitive / floor#0 / round-2 | 78 | +341 | +4.37 ±1.54 | +331 | |

## By phase

  -284899 IMPs  136057 boards  Constructive
  -278778 IMPs  114311 boards  Defensive
  -180780 IMPs   58375 boards  Competitive

## By provenance

  -396687 IMPs  187527 boards  book
   -86753 IMPs   28949 boards  floor#3
   -43869 IMPs   14255 boards  fallback@1
   -39978 IMPs   12469 boards  fallback@2
   -31789 IMPs   10565 boards  floor#242
   -26849 IMPs    9307 boards  fallback@3
   -10972 IMPs    3498 boards  fallback@4
    -8139 IMPs    2734 boards  floor#61
    -8091 IMPs    2787 boards  floor#46
    -7719 IMPs    3421 boards  floor#20
    -6599 IMPs    2694 boards  floor#140
    -6249 IMPs    3082 boards  floor#35
    -5132 IMPs    1695 boards  floor#60
    -5028 IMPs    2893 boards  floor#50
    -4199 IMPs    1064 boards  floor#243
    -3762 IMPs     928 boards  floor#202
    -3520 IMPs    1175 boards  floor#30
    -3318 IMPs    1699 boards  floor#64
    -3193 IMPs     491 boards  floor#242+rb
    -3074 IMPs     790 boards  floor#31
    -2738 IMPs     587 boards  floor#16
    -2718 IMPs     821 boards  floor#45
    -2418 IMPs     752 boards  floor#132
    -1911 IMPs     571 boards  floor#200
    -1869 IMPs    1452 boards  floor#65
    -1633 IMPs     551 boards  floor#131
    -1407 IMPs     476 boards  fallback@5
    -1333 IMPs     419 boards  floor#32
    -1280 IMPs     401 boards  floor#17
    -1221 IMPs     415 boards  floor#240
     -847 IMPs     203 boards  floor#145
     -785 IMPs     329 boards  floor#5
     -781 IMPs     452 boards  floor#49
     -779 IMPs     299 boards  floor#197
     -775 IMPs     243 boards  floor#241
     -771 IMPs     264 boards  floor#239
     -766 IMPs     295 boards  floor#237
     -759 IMPs     273 boards  floor#198
     -718 IMPs     455 boards  book+rb
     -700 IMPs     295 boards  floor#51
     -642 IMPs     293 boards  floor#21
     -630 IMPs     232 boards  floor#6
     -610 IMPs     200 boards  floor#66
     -593 IMPs     252 boards  floor#238
     -580 IMPs     145 boards  floor#15
     -551 IMPs     330 boards  floor#199
     -542 IMPs     296 boards  floor#129
     -524 IMPs     145 boards  floor#235
     -438 IMPs     204 boards  floor#236
     -396 IMPs     203 boards  floor#36
     -380 IMPs     398 boards  floor#133
     -352 IMPs     143 boards  floor#234
     -346 IMPs     780 boards  floor#1
     -342 IMPs      75 boards  floor#33
     -329 IMPs      76 boards  floor#10
     -319 IMPs     125 boards  floor#204
     -315 IMPs     107 boards  floor#147
     -311 IMPs      84 boards  floor#63
     -284 IMPs      61 boards  floor#27
     -279 IMPs     173 boards  floor#205
     -257 IMPs      80 boards  floor#12
     -252 IMPs     124 boards  floor#151
     -243 IMPs      55 boards  floor#153
     -234 IMPs     175 boards  floor#3+rb
     -220 IMPs      85 boards  floor#48
     -210 IMPs      50 boards  floor#55
     -208 IMPs     129 boards  floor#203
     -196 IMPs      31 boards  floor#11
     -189 IMPs      57 boards  floor#240+rb
     -177 IMPs      19 boards  floor#234+rb
     -173 IMPs      66 boards  floor#42
     -172 IMPs     373 boards  floor#47
     -152 IMPs      49 boards  floor#9
     -150 IMPs      69 boards  floor#157
     -149 IMPs      37 boards  floor#236+rb
     -147 IMPs      41 boards  floor#39
     -145 IMPs     102 boards  floor#25
     -130 IMPs      46 boards  floor#46+rb
     -110 IMPs      33 boards  floor#54
     -109 IMPs      69 boards  floor#18
     -108 IMPs      29 boards  floor#57
     -103 IMPs      41 boards  floor#238+rb
     -103 IMPs      16 boards  floor#26
      -81 IMPs      17 boards  floor#243+rb
      -80 IMPs     135 boards  floor#2
      -72 IMPs      18 boards  floor#241+rb
      -70 IMPs      15 boards  floor#29
      -69 IMPs      18 boards  fallback@6
      -65 IMPs      21 boards  floor#24
      -63 IMPs      45 boards  floor#40
      -59 IMPs     234 boards  floor#62
      -49 IMPs      11 boards  floor#61+rb
      -45 IMPs     152 boards  floor#34
      -44 IMPs       9 boards  floor#239+rb
      -43 IMPs      11 boards  floor#60+rb
      -42 IMPs      64 boards  floor#41
      -38 IMPs       6 boards  floor#45+rb
      -31 IMPs       4 boards  floor#63+rb
      -29 IMPs       7 boards  floor#218
      -28 IMPs       7 boards  floor#235+rb
      -25 IMPs       5 boards  floor#140+rb
      -24 IMPs       2 boards  floor#211
      -24 IMPs       2 boards  floor#229
      -22 IMPs       4 boards  floor#143
      -22 IMPs       7 boards  floor#228
      -22 IMPs       3 boards  floor#28
      -21 IMPs       7 boards  floor#208
      -20 IMPs       4 boards  floor#227
      -17 IMPs       4 boards  floor#62+rb
      -16 IMPs       4 boards  fallback@4+rb
      -14 IMPs       2 boards  floor#135
      -12 IMPs       8 boards  floor#144
      -12 IMPs       7 boards  floor#47+rb
      -11 IMPs       2 boards  floor#230
      -10 IMPs       4 boards  floor#13
      -10 IMPs       9 boards  floor#14
      -10 IMPs       1 boards  floor#212
      -10 IMPs       5 boards  floor#237+rb
      -10 IMPs       3 boards  floor#30+rb
       -8 IMPs       6 boards  floor#231
       -5 IMPs       1 boards  floor#219
       -2 IMPs       2 boards  floor#128
       +0 IMPs       1 boards  floor#226
       +1 IMPs      13 boards  floor#154
       +2 IMPs       6 boards  floor#148
       +3 IMPs       8 boards  floor#127
       +9 IMPs       2 boards  floor#38
      +14 IMPs      38 boards  floor#56
      +22 IMPs       4 boards  floor#18+rb
      +35 IMPs       4 boards  floor#210
      +45 IMPs       8 boards  floor#207
     +476 IMPs     119 boards  floor#0

## By family

  -376842 IMPs  143949 boards  round-1
  -228757 IMPs   94146 boards  round-2
   -96377 IMPs   47309 boards  opening
   -25893 IMPs   12116 boards  balancing
   -16588 IMPs   11223 boards  deep

## By direction

  -492298 IMPs   68247 boards  other
  -215542 IMPs   31829 boards  overbid
  -184351 IMPs   21279 boards  missed-game
  -163288 IMPs   23703 boards  sold-out
   -82291 IMPs   14939 boards  wrong-strain
   -80075 IMPs    6552 boards  missed-slam
   -11315 IMPs     780 boards  missed-grand
    -9569 IMPs    1442 boards  doubling
       +0 IMPs   50898 boards  flat
  +494272 IMPs   89074 boards  gain

## Worst boards per losing bucket

### Defensive / book / round-1 (59857 boards, -134485 IMPs)

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

### Constructive / book / opening (47309 boards, -96377 IMPs)

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

### Constructive / book / round-2 (41292 boards, -81297 IMPs)

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

### Constructive / book / round-1 (29719 boards, -71195 IMPs)

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

### Competitive / fallback@1 / round-1 (13956 boards, -43168 IMPs)

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

### Competitive / fallback@2 / round-1 (12137 boards, -39005 IMPs)

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

### Defensive / floor#3 / round-2 (8706 boards, -28211 IMPs)

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

### Defensive / floor#3 / round-1 (7680 boards, -24524 IMPs)

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

### Competitive / fallback@3 / round-2 (7357 boards, -20148 IMPs)

[vul both, seed Some(1783375066), board 5126] swing -2300 pts / -20 IMPs (PD -20), diverged at call 4 (3♦ ours vs P BBA), other
  rule: 2♦ is the cheapest bid, 6+ ♦, 2–5 points, and not (opponents bid ♦)
  W:K32.JT5.K9.K9853 T8754.76.AQJ.AT7 AJ96.AK3.T7.QJ42 Q.Q9842.865432.6
  ours NS @ A: - - 1♠ 1NT 3♦ 3NT - - -  -> 3NT by East
  ours EW @ B: - - 1♠ 1NT - 3♣ - 3♦ X - - -  -> 3♦x by East

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

### Competitive / fallback@4 / round-2 (3072 boards, -9724 IMPs)

[vul both, seed Some(1783375082), board 6074] swing -2320 pts / -20 IMPs (PD -20), diverged at call 4 (3♥ ours vs X BBA), other
  rule: 3+ ♥, and 6–9 points
  W:JT865.AT2.62.K83 9.KQJ94.J3.AQT92 AKQ73.6.QT9875.7 42.8753.AK4.J654
  ours NS @ A: - - 1♥ 2♥ 3♥ 4♠ - - -  -> 4♠ by West
  ours EW @ B: - - 1♥ 2♥ X - - -  -> 2♥x by East

[vul both, seed Some(1783375076), board 2469] swing -2120 pts / -19 IMPs (PD -19), diverged at call 4 (1NT ours vs XX BBA), other
  rule: 6–9 HCP
  W:AQT8732.JT8.K2.Q 95.KQ63.QT5.AJT5 J4.A972.A83.8732 K6.54.J9764.K964
  ours NS @ A: - - 1♠ X XX - - -  -> 1♠xx by West
  ours EW @ B: - - 1♠ X 1NT 2♦ 2♠ - - -  -> 2♠ by West

[vul both, seed Some(1783375077), board 2437] swing -2080 pts / -19 IMPs (PD -19), diverged at call 4 (3♥ ours vs X BBA), missed-game
  rule: 3+ ♥, and 6–9 points
  W:A76.AQT9832.6.J3 KJT43..KJ75432.A 95.KJ764.T8.K976 Q82.5.AQ9.QT8542
  ours NS @ A: - - 1♥ 2♥ X - - -  -> 2♥x by North
  ours EW @ B: - - 1♥ 2♥ 3♥ 4♠ - - -  -> 4♠ by South

