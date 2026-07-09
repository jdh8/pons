=== arm 0: our american floor (us) vs BBA 2/1 (them), vulnerability none, 204800 boards ===
replay verification: 100.00% of 2128300 our-side calls (0 mismatched)
auction-divergent: 186395 (91%), contract-divergent: 154464 (75%)
plain DD: -1.5223 IMPs/board (95% CI [-1.5442, -1.5004]), -311768 IMPs total
perfect defense: -1.5590 IMPs/board (95% CI [-1.5851, -1.5329])
gen_args: --count 6400 --seed 1783375064 --output ab-results/anchor/2026-07-09-5f9d6c2/none/shard-0.json -v none

=== arm 1: our american floor (us) vs BBA 2/1 (them), vulnerability both, 204800 boards ===
replay verification: 100.00% of 2111345 our-side calls (0 mismatched)
auction-divergent: 185490 (91%), contract-divergent: 152713 (75%)
plain DD: -1.9932 IMPs/board (95% CI [-2.0213, -1.9652]), -408213 IMPs total
perfect defense: -2.1689 IMPs/board (95% CI [-2.2019, -2.1360])
gen_args: --count 6400 --seed 1783375064 --output ab-results/anchor/2026-07-09-5f9d6c2/both/shard-0.json -v both


## IMP histogram (plain, per contract-divergent board)

right-siding-only divergences (same contract, different auction): 64708

  -24 IMPs: 1
  -22 IMPs: 4
  -21 IMPs: 26
  -20 IMPs: 43
  -19 IMPs: 162
  -18 IMPs: 306
  -17 IMPs: 1020
  -16 IMPs: 1104
  -15 IMPs: 2507
  -14 IMPs: 3585
  -13 IMPs: 9105
  -12 IMPs: 7926
  -11 IMPs: 13776
  -10 IMPs: 18993
   -9 IMPs: 8091
   -8 IMPs: 6864
   -7 IMPs: 12720
   -6 IMPs: 19850
   -5 IMPs: 15755
   -4 IMPs: 7867
   -3 IMPs: 11754
   -2 IMPs: 13505
   -1 IMPs: 12178
   +0 IMPs: 51519
   +1 IMPs: 10433
   +2 IMPs: 9877
   +3 IMPs: 8323
   +4 IMPs: 5943
   +5 IMPs: 15057
   +6 IMPs: 12610
   +7 IMPs: 6059
   +8 IMPs: 2230
   +9 IMPs: 2354
  +10 IMPs: 5336
  +11 IMPs: 4226
  +12 IMPs: 2563
  +13 IMPs: 2881
  +14 IMPs: 442
  +15 IMPs: 91
  +16 IMPs: 44
  +17 IMPs: 43
  +18 IMPs: 2
  +19 IMPs: 2

## Ranked buckets (phase / provenance / family), losses first

| bucket | boards | net plain IMPs | IMPs/divergent ±CI | net PD IMPs | flag |
| --- | --- | --- | --- | --- | --- |
| Defensive / book / round-1 | 58956 | -127014 | -2.15 ±0.05 | -146649 | |
| Constructive / book / opening | 47185 | -94604 | -2.00 ±0.06 | -87735 | |
| Constructive / book / round-2 | 41307 | -81380 | -1.97 ±0.06 | -83029 | |
| Constructive / book / round-1 | 29658 | -70522 | -2.38 ±0.08 | -75653 | |
| Competitive / fallback@1 / round-1 | 13968 | -41021 | -2.94 ±0.10 | -35146 | |
| Competitive / fallback@2 / round-1 | 12130 | -37151 | -3.06 ±0.11 | -34548 | |
| Defensive / floor#3 / round-2 | 9000 | -28828 | -3.20 ±0.13 | -26500 | |
| Defensive / floor#3 / round-1 | 7997 | -26074 | -3.26 ±0.15 | -18277 | |
| Competitive / fallback@3 / round-2 | 7349 | -18636 | -2.54 ±0.14 | -19107 | |
| Competitive / fallback@4 / round-2 | 3046 | -9609 | -3.15 ±0.23 | -11389 | |
| Competitive / floor#3 / round-2 | 3111 | -8718 | -2.80 ±0.23 | -7061 | |
| Defensive / book / round-2 | 4506 | -8574 | -1.90 ±0.20 | -10062 | |
| Competitive / fallback@3 / round-1 | 1926 | -6522 | -3.39 ±0.31 | -8160 | |
| Constructive / floor#3 / round-2 | 2797 | -6362 | -2.27 ±0.22 | -5557 | |
| Competitive / floor#242 / round-2 | 1532 | -5760 | -3.76 ±0.32 | -9105 | |
| Constructive / floor#140 / round-2 | 2084 | -5566 | -2.67 ±0.34 | -5826 | |
| Defensive / floor#3 / balancing | 2240 | -5388 | -2.41 ±0.22 | -2250 | |
| Defensive / floor#242 / round-1 | 1679 | -5011 | -2.98 ±0.35 | -6452 | |
| Competitive / floor#3 / round-1 | 1183 | -4950 | -4.18 ±0.40 | -1848 | |
| Constructive / floor#3 / round-1 | 1343 | -4856 | -3.62 ±0.38 | -2384 | |
| Constructive / book / deep | 4805 | -4315 | -0.90 ±0.18 | -4764 | |
| Defensive / floor#20 / round-1 | 1450 | -4145 | -2.86 ±0.33 | -5125 | |
| Defensive / floor#50 / round-1 | 1792 | -3650 | -2.04 ±0.29 | -4603 | |
| Defensive / floor#60 / round-2 | 1323 | -3601 | -2.72 ±0.32 | -5487 | |
| Defensive / floor#242 / round-2 | 814 | -3552 | -4.36 ±0.45 | -6010 | |
| Defensive / floor#243 / round-1 | 718 | -3425 | -4.77 ±0.57 | -3659 | |
| Competitive / floor#3 / balancing | 1610 | -3411 | -2.12 ±0.26 | +21 | plain/PD-flip |
| Defensive / floor#35 / round-1 | 1457 | -3286 | -2.26 ±0.33 | -4388 | |
| Defensive / floor#64 / round-1 | 1164 | -2631 | -2.26 ±0.40 | -2557 | |
| Competitive / floor#46 / round-2 | 831 | -2620 | -3.15 ±0.40 | -3524 | |
| Defensive / floor#242 / balancing | 1469 | -2605 | -1.77 ±0.34 | -5947 | |
| Constructive / floor#61 / deep | 1247 | -2470 | -1.98 ±0.17 | -4014 | |
| Defensive / floor#20 / round-2 | 1192 | -2172 | -1.82 ±0.32 | -3172 | |
| Defensive / floor#35 / round-2 | 1095 | -2129 | -1.94 ±0.36 | -3244 | |
| Constructive / floor#3 / deep | 1334 | -2033 | -1.52 ±0.36 | -1633 | |
| Competitive / floor#242 / balancing | 1069 | -1940 | -1.81 ±0.38 | -4331 | |
| Defensive / floor#45 / round-2 | 562 | -1886 | -3.36 ±0.50 | -2949 | |
| Competitive / floor#46 / round-1 | 315 | -1809 | -5.74 ±0.78 | -1708 | |
| Competitive / floor#242+rb / round-2 | 256 | -1782 | -6.96 ±0.87 | -2057 | |
| Defensive / floor#202 / round-1 | 491 | -1772 | -3.61 ±0.56 | -2166 | |
| Competitive / floor#61 / round-1 | 239 | -1767 | -7.39 ±0.75 | -1685 | |
| Defensive / floor#202 / round-2 | 425 | -1764 | -4.15 ±0.57 | -2599 | |
| Competitive / floor#31 / round-1 | 379 | -1754 | -4.63 ±0.70 | -1319 | |
| Defensive / floor#132 / round-1 | 455 | -1651 | -3.63 ±0.50 | -3021 | |
| Competitive / floor#30 / round-2 | 522 | -1573 | -3.01 ±0.47 | -2351 | |
| Defensive / floor#61 / round-2 | 451 | -1475 | -3.27 ±0.53 | -1879 | |
| Competitive / floor#243 / round-2 | 490 | -1317 | -2.69 ±0.55 | -2033 | |
| Constructive / fallback@4 / deep | 425 | -1246 | -2.93 ±0.65 | -1394 | |
| Defensive / floor#243 / balancing | 463 | -1242 | -2.68 ±0.65 | -1792 | |
| Constructive / fallback@5 / deep | 376 | -1172 | -3.12 ±0.75 | -1269 | |
| Defensive / floor#200 / round-1 | 294 | -1158 | -3.94 ±0.77 | -1443 | |
| Constructive / floor#61 / round-2 | 425 | -1152 | -2.71 ±0.61 | -1312 | |
| Defensive / floor#60 / round-1 | 314 | -1124 | -3.58 ±0.62 | -1705 | |
| Defensive / floor#65 / round-1 | 911 | -1112 | -1.22 ±0.39 | -1814 | |
| Defensive / floor#46 / round-2 | 354 | -1093 | -3.09 ±0.59 | -1577 | |
| Competitive / floor#243 / balancing | 445 | -1066 | -2.40 ±0.61 | -2085 | |
| Defensive / floor#131 / balancing | 410 | -1046 | -2.55 ±0.53 | -1614 | |
| Defensive / floor#20 / balancing | 756 | -1016 | -1.34 ±0.38 | -2001 | |
| Constructive / floor#46 / round-2 | 528 | -994 | -1.88 ±0.56 | -1103 | |
| Constructive / floor#46 / deep | 625 | -983 | -1.57 ±0.37 | -1817 | |
| Competitive / floor#16 / round-1 | 136 | -978 | -7.19 ±1.10 | -998 | |
| Competitive / fallback@2 / round-2 | 334 | -926 | -2.77 ±0.82 | -776 | |
| Defensive / floor#30 / round-2 | 323 | -887 | -2.75 ±0.63 | -1474 | |
| Constructive / floor#32 / round-1 | 187 | -804 | -4.30 ±0.88 | -754 | |
| Constructive / floor#145 / round-2 | 162 | -803 | -4.96 ±1.28 | -814 | |
| Defensive / floor#200 / round-2 | 278 | -769 | -2.77 ±0.76 | -1218 | |
| Defensive / floor#45 / round-1 | 244 | -761 | -3.12 ±0.85 | -1080 | |
| Defensive / floor#3 / deep | 252 | -752 | -2.98 ±0.78 | -574 | |
| Defensive / floor#243 / round-2 | 244 | -747 | -3.06 ±0.84 | -1379 | |
| Defensive / floor#132 / balancing | 297 | -744 | -2.51 ±0.51 | -1808 | |
| Competitive / fallback@1 / round-2 | 307 | -711 | -2.32 ±0.84 | -594 | |
| Defensive / floor#197 / round-1 | 214 | -696 | -3.25 ±0.85 | -753 | |
| Defensive / floor#35 / balancing | 514 | -687 | -1.34 ±0.44 | -1438 | |
| Competitive / floor#61 / round-2 | 161 | -668 | -4.15 ±0.94 | -1036 | |
| Competitive / book+rb / round-2 | 395 | -658 | -1.67 ±0.52 | -1064 | |
| Defensive / floor#50 / round-2 | 660 | -651 | -0.99 ±0.42 | -1273 | |
| Constructive / floor#140 / deep | 486 | -633 | -1.30 ±0.62 | -701 | |
| Competitive / floor#240 / balancing | 199 | -609 | -3.06 ±0.92 | -512 | |
| Defensive / floor#16 / round-2 | 185 | -601 | -3.25 ±0.86 | -748 | |
| Defensive / floor#199 / round-1 | 263 | -557 | -2.12 ±0.72 | -567 | |
| Competitive / floor#240 / round-2 | 199 | -536 | -2.69 ±0.97 | -537 | |
| Defensive / floor#66 / round-1 | 179 | -528 | -2.95 ±1.04 | -648 | |
| Competitive / floor#5 / round-2 | 227 | -522 | -2.30 ±0.93 | -890 | |
| Defensive / floor#51 / round-1 | 242 | -520 | -2.15 ±0.90 | -370 | |
| Defensive / floor#21 / round-1 | 182 | -518 | -2.85 ±0.97 | -884 | |
| Constructive / floor#17 / round-1 | 137 | -509 | -3.72 ±1.10 | -403 | |
| Defensive / floor#64 / round-2 | 303 | -503 | -1.66 ±0.70 | -839 | |
| Competitive / floor#6 / round-2 | 171 | -481 | -2.81 ±1.00 | -922 | |
| Competitive / floor#239 / round-2 | 114 | -474 | -4.16 ±1.12 | -580 | |
| Defensive / floor#131 / round-1 | 138 | -467 | -3.38 ±1.09 | -730 | |
| Competitive / floor#237 / round-2 | 153 | -423 | -2.76 ±0.86 | -767 | |
| Defensive / floor#198 / round-2 | 115 | -417 | -3.63 ±1.14 | -620 | |
| Competitive / floor#242 / round-1 | 244 | -412 | -1.69 ±0.78 | -1085 | |
| Defensive / floor#30 / round-1 | 117 | -371 | -3.17 ±1.07 | -411 | |
| Constructive / floor#16 / round-2 | 68 | -368 | -5.41 ±1.46 | -347 | |
| Defensive / floor#49 / round-1 | 188 | -363 | -1.93 ±0.90 | -213 | |
| Competitive / floor#3+rb / round-2 | 220 | -361 | -1.64 ±0.77 | -269 | |
| Defensive / floor#50 / balancing | 403 | -359 | -0.89 ±0.44 | -1098 | |
| Competitive / floor#16 / round-2 | 89 | -357 | -4.01 ±1.50 | -379 | |
| Defensive / floor#49 / balancing | 193 | -354 | -1.83 ±0.90 | -544 | |
| Defensive / floor#198 / round-1 | 158 | -342 | -2.16 ±1.13 | -506 | |
| Competitive / floor#241 / round-2 | 85 | -340 | -4.00 ±1.25 | -439 | |
| Defensive / floor#61 / round-1 | 99 | -339 | -3.42 ±1.58 | -329 | |
| Competitive / floor#30 / balancing | 127 | -337 | -2.65 ±0.87 | -668 | |
| Defensive / floor#65 / balancing | 242 | -327 | -1.35 ±0.58 | -1026 | |
| Constructive / floor#31 / round-2 | 78 | -318 | -4.08 ±1.52 | -271 | |
| Competitive / floor#241 / balancing | 109 | -310 | -2.84 ±1.13 | -481 | |
| Competitive / floor#243+rb / round-2 | 62 | -307 | -4.95 ±1.45 | -611 | |
| Competitive / floor#31 / round-2 | 80 | -306 | -3.83 ±1.38 | -339 | |
| Competitive / floor#238 / balancing | 155 | -302 | -1.95 ±1.06 | -265 | |
| Competitive / floor#235 / round-2 | 63 | -300 | -4.76 ±1.27 | -440 | |
| Defensive / floor#36 / round-1 | 145 | -291 | -2.01 ±0.99 | -584 | |
| Defensive / floor#133 / round-1 | 315 | -290 | -0.92 ±0.92 | -718 | ~noise |
| Defensive / floor#31 / round-2 | 174 | -287 | -1.65 ±0.91 | -336 | |
| Competitive / floor#61 / deep | 125 | -286 | -2.29 ±0.80 | -584 | |
| Competitive / floor#10 / round-2 | 72 | -283 | -3.93 ±1.18 | -310 | |
| Defensive / floor#17 / round-1 | 74 | -274 | -3.70 ±1.55 | -212 | |
| Defensive / floor#32 / round-1 | 47 | -273 | -5.81 ±2.13 | -218 | |
| Competitive / floor#15 / round-2 | 58 | -267 | -4.60 ±1.36 | -399 | |
| Competitive / floor#15 / balancing | 79 | -265 | -3.35 ±1.11 | -436 | |
| Defensive / floor#129 / round-2 | 173 | -260 | -1.50 ±1.19 | -546 | |
| Competitive / floor#46 / deep | 95 | -243 | -2.56 ±1.07 | -419 | |
| Constructive / floor#151 / round-2 | 88 | -242 | -2.75 ±2.16 | -236 | |
| Competitive / floor#236 / balancing | 150 | -240 | -1.60 ±0.91 | -286 | |
| Competitive / fallback@5 / round-2 | 100 | -235 | -2.35 ±1.14 | -171 | |
| Defensive / floor#204 / round-1 | 72 | -232 | -3.22 ±1.57 | -205 | |
| Constructive / floor#140 / round-1 | 18 | -229 | -12.72 ±0.82 | -229 | |
| Competitive / floor#31 / balancing | 34 | -226 | -6.65 ±1.75 | -273 | |
| Defensive / floor#205 / round-1 | 95 | -221 | -2.33 ±1.34 | -179 | |
| Defensive / floor#27 / round-2 | 41 | -192 | -4.68 ±1.57 | -228 | |
| Competitive / floor#240+rb / round-2 | 57 | -189 | -3.32 ±1.90 | -194 | |
| Competitive / floor#3 / deep | 194 | -184 | -0.95 ±0.86 | -93 | |
| Competitive / floor#234+rb / round-2 | 19 | -177 | -9.32 ±2.14 | -212 | |
| Defensive / floor#237 / round-2 | 52 | -173 | -3.33 ±1.67 | -265 | |
| Competitive / floor#16 / balancing | 52 | -172 | -3.31 ±1.61 | -289 | |
| Competitive / floor#234 / balancing | 89 | -172 | -1.93 ±1.26 | -193 | |
| Defensive / floor#17 / round-2 | 80 | -167 | -2.09 ±1.09 | -242 | |
| Competitive / floor#234 / round-2 | 51 | -166 | -3.25 ±1.71 | -198 | |
| Defensive / floor#64 / balancing | 231 | -165 | -0.71 ±0.77 | -402 | ~noise |
| Competitive / floor#238 / round-2 | 78 | -161 | -2.06 ±1.65 | -113 | |
| Defensive / floor#203 / round-1 | 78 | -157 | -2.01 ±1.34 | -154 | |
| Competitive / floor#32 / round-1 | 24 | -155 | -6.46 ±2.64 | -150 | |
| Competitive / floor#242 / deep | 22 | -154 | -7.00 ±2.71 | -212 | |
| Defensive / floor#63 / round-2 | 39 | -153 | -3.92 ±2.12 | -130 | |
| Competitive / floor#140 / round-1 | 42 | -152 | -3.62 ±2.92 | -152 | |
| Defensive / floor#153 / round-1 | 22 | -152 | -6.91 ±3.94 | -123 | |
| Defensive / floor#242 / deep | 28 | -150 | -5.36 ±2.16 | -266 | |
| Competitive / floor#236+rb / round-2 | 37 | -149 | -4.03 ±2.54 | -163 | |
| Competitive / floor#236 / round-2 | 47 | -143 | -3.04 ±2.02 | -154 | |
| Competitive / floor#237 / balancing | 68 | -140 | -2.06 ±1.09 | -316 | |
| Constructive / floor#147 / round-2 | 37 | -140 | -3.78 ±3.22 | -140 | |
| Competitive / floor#30 / deep | 37 | -138 | -3.73 ±1.67 | -211 | |
| Competitive / floor#46+rb / deep | 40 | -129 | -3.23 ±0.97 | -259 | |
| Defensive / floor#197 / round-2 | 87 | -127 | -1.46 ±1.43 | -212 | |
| Competitive / floor#47 / round-2 | 92 | -126 | -1.37 ±1.46 | -322 | ~noise |
| Competitive / floor#9 / round-2 | 48 | -123 | -2.56 ±1.24 | -184 | |
| Competitive / floor#55 / round-2 | 27 | -122 | -4.52 ±2.44 | -105 | |
| Constructive / floor#47 / round-2 | 12 | -120 | -10.00 ±2.70 | -120 | |
| Defensive / floor#48 / round-2 | 31 | -116 | -3.74 ±2.40 | -165 | |
| Competitive / floor#33 / round-2 | 31 | -115 | -3.71 ±1.91 | -163 | |
| Defensive / floor#12 / round-2 | 43 | -112 | -2.60 ±1.67 | -120 | |
| Defensive / floor#51 / balancing | 51 | -112 | -2.20 ±1.35 | -135 | |
| Defensive / floor#33 / round-2 | 22 | -111 | -5.05 ±2.73 | -70 | |
| Defensive / floor#42 / round-2 | 37 | -110 | -2.97 ±3.30 | -65 | ~noise |
| Defensive / floor#235 / round-2 | 40 | -108 | -2.70 ±1.63 | -159 | |
| Defensive / floor#65 / round-2 | 281 | -107 | -0.38 ±0.66 | -392 | ~noise |
| Competitive / floor#16 / deep | 32 | -106 | -3.31 ±1.14 | -132 | |
| Competitive / floor#63 / round-2 | 23 | -104 | -4.52 ±2.09 | -167 | |
| Competitive / floor#238+rb / round-2 | 41 | -103 | -2.51 ±1.97 | -124 | |
| Constructive / floor#30 / round-1 | 48 | -103 | -2.15 ±1.78 | -146 | |
| Defensive / floor#11 / round-2 | 22 | -101 | -4.59 ±1.40 | -167 | |
| Competitive / floor#243 / round-1 | 33 | -99 | -3.00 ±2.03 | -166 | |
| Constructive / floor#157 / round-2 | 45 | -97 | -2.16 ±2.25 | -90 | ~noise |
| Constructive / floor#62 / round-1 | 37 | -97 | -2.62 ±2.44 | -107 | |
| Competitive / floor#39 / round-2 | 19 | -96 | -5.05 ±1.34 | -143 | |
| Competitive / floor#57 / round-2 | 13 | -96 | -7.38 ±3.90 | -93 | |
| Competitive / floor#31 / deep | 46 | -94 | -2.04 ±1.22 | -162 | |
| Defensive / floor#147 / round-1 | 12 | -94 | -7.83 ±4.79 | -86 | |
| Defensive / floor#20 / deep | 21 | -91 | -4.33 ±1.46 | -159 | |
| Defensive / floor#129 / round-1 | 58 | -90 | -1.55 ±1.78 | -187 | ~noise |
| Defensive / floor#133 / balancing | 85 | -89 | -1.05 ±1.65 | -244 | ~noise |
| Competitive / floor#140 / round-2 | 36 | -88 | -2.44 ±2.62 | -91 | ~noise |
| Defensive / floor#204 / round-2 | 45 | -88 | -1.96 ±1.43 | -41 | |
| Defensive / floor#21 / balancing | 62 | -86 | -1.39 ±1.24 | -205 | |
| Competitive / floor#17 / round-1 | 11 | -83 | -7.55 ±3.66 | -59 | |
| Constructive / floor#31 / deep | 20 | -82 | -4.10 ±2.40 | -81 | |
| Defensive / floor#66 / balancing | 21 | -82 | -3.90 ±2.28 | -100 | |
| Competitive / floor#239 / balancing | 68 | -80 | -1.18 ±1.11 | -182 | |
| Defensive / floor#46 / round-1 | 34 | -79 | -2.32 ±2.80 | -62 | ~noise |
| Competitive / floor#60 / balancing | 53 | -77 | -1.45 ±1.63 | -60 | ~noise |
| Competitive / floor#2 / round-2 | 128 | -76 | -0.59 ±1.25 | -331 | ~noise |
| Competitive / floor#33 / round-1 | 12 | -76 | -6.33 ±3.23 | -108 | |
| Defensive / floor#47 / round-1 | 12 | -76 | -6.33 ±4.49 | -72 | |
| Competitive / floor#42 / round-2 | 14 | -75 | -5.36 ±4.46 | -101 | |
| Defensive / floor#32 / round-2 | 57 | -74 | -1.30 ±1.33 | -100 | ~noise |
| Competitive / floor#129 / deep | 9 | -73 | -8.11 ±3.51 | -86 | |
| Constructive / floor#62 / round-2 | 8 | -72 | -9.00 ±3.90 | -72 | |
| Defensive / floor#238 / round-2 | 11 | -72 | -6.55 ±2.73 | -101 | |
| Defensive / floor#26 / round-2 | 11 | -70 | -6.36 ±3.40 | -97 | |
| Competitive / fallback@6 / round-2 | 18 | -69 | -3.83 ±3.54 | -44 | |
| Defensive / floor#239 / round-2 | 47 | -67 | -1.43 ±2.17 | -101 | ~noise |
| Defensive / floor#33 / round-1 | 12 | -67 | -5.58 ±4.15 | -73 | |
| Constructive / floor#32 / deep | 59 | -65 | -1.10 ±0.60 | -76 | |
| Defensive / floor#240 / round-2 | 21 | -65 | -3.10 ±2.22 | -67 | |
| Competitive / floor#3+rb / deep | 26 | -64 | -2.46 ±2.23 | -109 | |
| Competitive / floor#45 / balancing | 19 | -63 | -3.32 ±3.81 | -29 | ~noise |
| Defensive / floor#36 / balancing | 36 | -63 | -1.75 ±2.05 | -133 | ~noise |
| Constructive / floor#17 / deep | 74 | -62 | -0.84 ±0.49 | -72 | |
| Competitive / floor#241 / deep | 20 | -61 | -3.05 ±2.89 | -74 | |
| Defensive / floor#49 / round-2 | 72 | -61 | -0.85 ±1.37 | -99 | ~noise |
| Competitive / floor#12 / round-2 | 24 | -59 | -2.46 ±1.66 | -71 | |
| Competitive / book+rb / deep | 59 | -56 | -0.95 ±0.86 | -87 | |
| Constructive / floor#153 / round-2 | 19 | -56 | -2.95 ±4.44 | -46 | ~noise |
| Defensive / floor#40 / round-2 | 6 | -56 | -9.33 ±2.24 | -56 | |
| Competitive / floor#1 / round-2 | 304 | -55 | -0.18 ±0.82 | +78 | ~noise plain/PD-flip |
| Defensive / floor#239 / balancing | 26 | -54 | -2.08 ±2.25 | -47 | ~noise |
| Defensive / floor#34 / balancing | 151 | -54 | -0.36 ±0.96 | -122 | ~noise |
| Defensive / floor#55 / round-2 | 15 | -52 | -3.47 ±3.27 | -46 | |
| Competitive / floor#17 / deep | 16 | -51 | -3.19 ±2.54 | -62 | |
| Competitive / floor#24 / round-2 | 14 | -51 | -3.64 ±2.79 | -68 | |
| Defensive / floor#203 / round-2 | 51 | -51 | -1.00 ±1.84 | -44 | ~noise |
| Competitive / floor#17 / round-2 | 6 | -50 | -8.33 ±2.00 | -62 | |
| Competitive / floor#25 / round-2 | 100 | -50 | -0.50 ±0.99 | +44 | ~noise plain/PD-flip |
| Competitive / floor#32 / round-2 | 37 | -50 | -1.35 ±1.60 | -122 | ~noise |
| Defensive / floor#41 / round-2 | 45 | -50 | -1.11 ±2.50 | -81 | ~noise |
| Competitive / floor#48 / round-2 | 29 | -47 | -1.62 ±2.75 | -110 | ~noise |
| Defensive / floor#31 / round-1 | 13 | -47 | -3.62 ±2.90 | -59 | |
| Defensive / floor#36 / round-2 | 23 | -47 | -2.04 ±2.29 | -78 | ~noise |
| Competitive / floor#153 / round-2 | 9 | -46 | -5.11 ±6.82 | -45 | ~noise |
| Defensive / floor#16 / round-1 | 9 | -46 | -5.11 ±4.33 | -48 | |
| Competitive / floor#153 / round-1 | 3 | -45 | -15.00 ±2.99 | -45 | |
| Constructive / floor#157 / round-1 | 4 | -45 | -11.25 ±1.67 | -45 | |
| Constructive / floor#47 / round-1 | 34 | -45 | -1.32 ±2.89 | -43 | ~noise |
| Defensive / floor#151 / round-1 | 10 | -44 | -4.40 ±6.57 | -47 | ~noise |
| Defensive / floor#21 / round-2 | 49 | -43 | -0.88 ±1.76 | -206 | ~noise |
| Competitive / floor#18 / round-1 | 15 | -41 | -2.73 ±4.33 | -17 | ~noise |
| Competitive / floor#237 / deep | 7 | -41 | -5.86 ±3.46 | -62 | |
| Defensive / floor#18 / round-2 | 21 | -41 | -1.95 ±3.29 | -23 | ~noise |
| Competitive / floor#47 / balancing | 45 | -39 | -0.87 ±2.06 | -78 | ~noise |
| Competitive / floor#60+rb / round-2 | 8 | -39 | -4.88 ±3.46 | -53 | |
| Defensive / book / deep | 45 | -39 | -0.87 ±1.40 | -49 | ~noise |
| Defensive / floor#29 / round-1 | 7 | -38 | -5.43 ±5.69 | -36 | ~noise |
| Defensive / floor#54 / deep | 6 | -38 | -6.33 ±3.31 | -54 | |
| Defensive / floor#241 / round-2 | 7 | -36 | -5.14 ±4.24 | -34 | |
| Competitive / floor#61+rb / deep | 9 | -35 | -3.89 ±2.34 | -49 | |
| Defensive / floor#236 / round-2 | 4 | -35 | -8.75 ±2.02 | -35 | |
| Defensive / floor#48 / round-1 | 13 | -34 | -2.62 ±5.09 | -50 | ~noise |
| Competitive / floor#47+rb / balancing | 4 | -33 | -8.25 ±4.48 | -33 | |
| Constructive / floor#147 / deep | 24 | -33 | -1.38 ±4.11 | -20 | ~noise |
| Competitive / floor#6 / deep | 2 | -32 | -16.00 ±1.96 | -32 | |
| Constructive / floor#63 / round-2 | 10 | -32 | -3.20 ±3.74 | -60 | ~noise |
| Defensive / floor#205 / round-2 | 89 | -32 | -0.36 ±1.34 | -32 | ~noise |
| Defensive / floor#229 / round-2 | 6 | -32 | -5.33 ±8.36 | -32 | ~noise |
| Competitive / floor#11 / round-2 | 4 | -31 | -7.75 ±6.01 | -41 | |
| Competitive / floor#54 / round-2 | 11 | -31 | -2.82 ±4.38 | -33 | ~noise |
| Defensive / floor#241 / deep | 8 | -31 | -3.88 ±5.22 | -38 | ~noise |
| Competitive / floor#241+rb / deep | 5 | -30 | -6.00 ±6.59 | -31 | ~noise |
| Competitive / floor#60 / round-2 | 5 | -30 | -6.00 ±3.45 | -42 | |
| Constructive / floor#16 / deep | 4 | -30 | -7.50 ±2.47 | -38 | |
| Defensive / floor#17 / deep | 6 | -30 | -5.00 ±5.06 | -24 | ~noise |
| Competitive / floor#235 / balancing | 11 | -29 | -2.64 ±3.16 | -57 | ~noise |
| Competitive / floor#235 / deep | 6 | -29 | -4.83 ±3.48 | -42 | |
| Competitive / floor#39 / deep | 11 | -29 | -2.64 ±0.76 | -60 | |
| Competitive / floor#48 / round-1 | 6 | -29 | -4.83 ±6.14 | -18 | ~noise |
| Defensive / floor#218 / round-2 | 7 | -29 | -4.14 ±1.31 | -45 | |
| Defensive / floor#56 / round-1 | 12 | -29 | -2.42 ±4.97 | -36 | ~noise |
| Competitive / floor#45+rb / round-2 | 4 | -27 | -6.75 ±4.83 | -52 | |
| Defensive / floor#61 / balancing | 22 | -27 | -1.23 ±2.24 | -68 | ~noise |
| Competitive / floor#5 / deep | 12 | -26 | -2.17 ±3.00 | -54 | ~noise |
| Competitive / floor#57 / deep | 2 | -26 | -13.00 ±1.96 | -29 | |
| Competitive / floor#62 / round-1 | 3 | -26 | -8.67 ±9.49 | -42 | ~noise |
| Defensive / floor#140 / balancing | 4 | -26 | -6.50 ±5.09 | -26 | |
| Competitive / floor#140+rb / round-2 | 5 | -25 | -5.00 ±6.07 | -25 | ~noise |
| Competitive / floor#18 / round-2 | 12 | -25 | -2.08 ±3.52 | -15 | ~noise |
| Competitive / floor#239+rb / deep | 5 | -25 | -5.00 ±9.02 | -30 | ~noise |
| Competitive / floor#62 / deep | 12 | -25 | -2.08 ±3.29 | -41 | ~noise |
| Defensive / floor#237 / balancing | 25 | -25 | -1.00 ±2.12 | -74 | ~noise |
| Defensive / floor#6 / round-2 | 37 | -25 | -0.68 ±2.35 | -62 | ~noise |
| Competitive / floor#129 / round-2 | 21 | -24 | -1.14 ±3.13 | -77 | ~noise |
| Competitive / floor#145 / balancing | 4 | -24 | -6.00 ±6.84 | -24 | ~noise |
| Competitive / floor#241+rb / balancing | 10 | -24 | -2.40 ±3.05 | -44 | ~noise |
| Competitive / floor#242+rb / balancing | 5 | -24 | -4.80 ±3.74 | -51 | |
| Competitive / floor#40 / round-2 | 27 | -24 | -0.89 ±2.95 | -10 | ~noise |
| Competitive / floor#63 / round-1 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Constructive / floor#145 / round-1 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Constructive / floor#148 / round-2 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Defensive / floor#211 / round-2 | 2 | -24 | -12.00 ±1.96 | -24 | |
| Defensive / floor#35 / deep | 13 | -24 | -1.85 ±3.39 | -43 | ~noise |
| Competitive / floor#147 / round-1 | 6 | -23 | -3.83 ±5.34 | -18 | ~noise |
| Competitive / floor#143 / round-2 | 2 | -22 | -11.00 ±1.96 | -24 | |
| Defensive / floor#13 / round-2 | 3 | -22 | -7.33 ±7.95 | -22 | ~noise |
| Defensive / floor#199 / round-2 | 62 | -22 | -0.35 ±1.30 | -74 | ~noise |
| Defensive / floor#27 / round-1 | 5 | -22 | -4.40 ±5.42 | -17 | ~noise |
| Defensive / floor#29 / round-2 | 7 | -22 | -3.14 ±5.58 | -15 | ~noise |
| Competitive / floor#235+rb / balancing | 2 | -21 | -10.50 ±2.94 | -21 | |
| Competitive / fallback@2 / balancing | 2 | -20 | -10.00 ±1.96 | -20 | |
| Competitive / floor#45 / round-2 | 2 | -20 | -10.00 ±1.96 | -18 | |
| Defensive / floor#61 / deep | 25 | -20 | -0.80 ±1.87 | -110 | ~noise |
| Competitive / floor#46 / balancing | 12 | -19 | -1.58 ±2.87 | -70 | ~noise |
| Defensive / floor#32 / deep | 7 | -19 | -2.71 ±1.46 | -36 | |
| Defensive / floor#56 / deep | 3 | -19 | -6.33 ±2.61 | -38 | |
| Competitive / floor#145 / round-2 | 3 | -18 | -6.00 ±6.88 | -25 | ~noise |
| Competitive / floor#147 / round-2 | 25 | -18 | -0.72 ±3.76 | -21 | ~noise |
| Competitive / floor#241+rb / round-2 | 3 | -18 | -6.00 ±11.81 | -16 | ~noise |
| Competitive / floor#63+rb / round-2 | 2 | -18 | -9.00 ±5.88 | -10 | |
| Defensive / floor#140 / round-2 | 2 | -18 | -9.00 ±1.96 | -26 | |
| Competitive / floor#62+rb / round-2 | 4 | -17 | -4.25 ±11.61 | -4 | ~noise |
| Constructive / floor#17 / round-2 | 2 | -17 | -8.50 ±2.94 | -17 | |
| Defensive / floor#14 / round-2 | 5 | -17 | -3.40 ±8.02 | -14 | ~noise |
| Competitive / fallback@4+rb / round-2 | 4 | -16 | -4.00 ±2.89 | -19 | |
| Defensive / floor#228 / round-2 | 11 | -15 | -1.36 ±3.66 | +0 | ~noise plain/PD-flip |
| Defensive / floor#50 / deep | 12 | -15 | -1.25 ±2.23 | -15 | ~noise |
| Competitive / floor#135 / round-2 | 2 | -14 | -7.00 ±3.92 | -26 | |
| Competitive / floor#140 / deep | 2 | -14 | -7.00 ±1.96 | -22 | |
| Competitive / floor#239+rb / balancing | 4 | -14 | -3.50 ±6.86 | -14 | ~noise |
| Competitive / floor#24 / deep | 5 | -14 | -2.80 ±4.04 | -17 | ~noise |
| Competitive / floor#61+rb / balancing | 2 | -14 | -7.00 ±3.92 | -14 | |
| Constructive / floor#32 / round-2 | 4 | -14 | -3.50 ±11.94 | -6 | ~noise |
| Defensive / floor#46 / deep | 4 | -14 | -3.50 ±1.27 | -24 | |
| Competitive / floor#145 / round-1 | 1 | -13 | -13.00 ±0.00 | -13 | ~noise |
| Competitive / floor#47 / round-1 | 1 | -13 | -13.00 ±0.00 | -13 | ~noise |
| Competitive / floor#63+rb / deep | 2 | -13 | -6.50 ±8.82 | -14 | ~noise |
| Defensive / floor#26 / deep | 1 | -13 | -13.00 ±0.00 | -14 | ~noise |
| Defensive / floor#40 / deep | 2 | -13 | -6.50 ±6.86 | -24 | ~noise |
| Defensive / floor#28 / round-1 | 2 | -12 | -6.00 ±1.96 | -14 | |
| Competitive / floor#12 / deep | 2 | -11 | -5.50 ±2.94 | -14 | |
| Competitive / floor#45+rb / deep | 2 | -11 | -5.50 ±2.94 | -21 | |
| Competitive / floor#56 / round-2 | 1 | -11 | -11.00 ±0.00 | -12 | ~noise |
| Constructive / floor#147 / round-1 | 1 | -11 | -11.00 ±0.00 | -11 | ~noise |
| Defensive / floor#230 / round-2 | 2 | -11 | -5.50 ±0.98 | -14 | |
| Defensive / floor#26 / round-1 | 1 | -11 | -11.00 ±0.00 | -14 | ~noise |
| Defensive / floor#63 / deep | 2 | -11 | -5.50 ±2.94 | -14 | |
| Competitive / floor#151 / round-2 | 10 | -10 | -1.00 ±5.31 | -11 | ~noise |
| Competitive / floor#30+rb / round-2 | 3 | -10 | -3.33 ±3.46 | -26 | ~noise |
| Defensive / floor#208 / round-2 | 9 | -10 | -1.11 ±4.45 | +2 | ~noise plain/PD-flip |
| Defensive / floor#212 / round-2 | 1 | -10 | -10.00 ±0.00 | -10 | ~noise |
| Defensive / floor#28 / round-2 | 1 | -10 | -10.00 ±0.00 | -10 | ~noise |
| Competitive / fallback@3 / balancing | 1 | -9 | -9.00 ±0.00 | -9 | ~noise |
| Competitive / floor#32 / deep | 7 | -9 | -1.29 ±1.64 | -12 | ~noise |
| Competitive / floor#47+rb / deep | 1 | -9 | -9.00 ±0.00 | -12 | ~noise |
| Constructive / floor#33 / deep | 2 | -9 | -4.50 ±0.98 | -12 | |
| Defensive / floor#18 / deep | 5 | -9 | -1.80 ±2.18 | -12 | ~noise |
| Competitive / floor#237+rb / balancing | 4 | -8 | -2.00 ±10.03 | -5 | ~noise |
| Constructive / floor#157 / deep | 20 | -8 | -0.40 ±3.23 | -1 | ~noise |
| Defensive / floor#231 / round-2 | 9 | -8 | -0.89 ±4.16 | -30 | ~noise |
| Defensive / floor#237 / deep | 2 | -8 | -4.00 ±3.92 | -22 | |
| Competitive / floor#235+rb / deep | 5 | -7 | -1.40 ±2.02 | -40 | ~noise |
| Competitive / floor#26 / deep | 1 | -7 | -7.00 ±0.00 | -12 | ~noise |
| Competitive / floor#3+rb / balancing | 8 | -7 | -0.88 ±2.17 | -16 | ~noise |
| Competitive / floor#54 / deep | 3 | -7 | -2.33 ±4.57 | -1 | ~noise |
| Defensive / floor#62 / deep | 5 | -7 | -1.40 ±6.58 | -17 | ~noise |
| Competitive / floor#144 / round-1 | 1 | -6 | -6.00 ±0.00 | -6 | ~noise |
| Competitive / floor#144 / round-2 | 7 | -6 | -0.86 ±6.73 | -6 | ~noise |
| Defensive / floor#10 / round-2 | 4 | -6 | -1.50 ±6.33 | +1 | ~noise plain/PD-flip |
| Defensive / floor#129 / deep | 20 | -6 | -0.30 ±3.38 | +2 | ~noise plain/PD-flip |
| Defensive / floor#238 / balancing | 1 | -6 | -6.00 ±0.00 | -6 | ~noise |
| Defensive / floor#39 / round-2 | 2 | -6 | -3.00 ±3.92 | -21 | ~noise |
| Defensive / floor#219 / round-2 | 1 | -5 | -5.00 ±0.00 | -6 | ~noise |
| Defensive / floor#25 / round-2 | 2 | -5 | -2.50 ±4.90 | -6 | ~noise |
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
| Defensive / floor#41 / round-1 | 4 | -2 | -0.50 ±7.61 | +14 | ~noise plain/PD-flip |
| Defensive / floor#5 / round-1 | 28 | -2 | -0.07 ±2.88 | +12 | ~noise plain/PD-flip |
| Competitive / floor#46+rb / round-2 | 6 | -1 | -0.17 ±4.28 | -14 | ~noise |
| Defensive / floor#18 / round-1 | 9 | -1 | -0.11 ±6.23 | +5 | ~noise plain/PD-flip |
| Defensive / floor#238 / deep | 2 | -1 | -0.50 ±2.94 | -12 | ~noise |
| Competitive / floor#143 / balancing | 2 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Competitive / floor#41 / deep | 1 | +0 | +0.00 ±0.00 | +7 | ~noise |
| Defensive / floor#24 / round-2 | 2 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Defensive / floor#57 / round-1 | 1 | +0 | +0.00 ±0.00 | +0 | ~noise |
| Competitive / floor#14 / round-2 | 1 | +1 | +1.00 ±0.00 | +1 | ~noise |
| Competitive / floor#33 / balancing | 1 | +1 | +1.00 ±0.00 | -2 | ~noise plain/PD-flip |
| Constructive / floor#154 / deep | 13 | +1 | +0.08 ±3.45 | +10 | ~noise |
| Constructive / floor#151 / deep | 8 | +2 | +0.25 ±0.32 | +2 | ~noise |
| Defensive / floor#1 / round-1 | 17 | +2 | +0.12 ±4.32 | +5 | ~noise |
| Competitive / floor#26 / round-2 | 2 | +3 | +1.50 ±6.86 | +4 | ~noise |
| Defensive / floor#127 / round-2 | 8 | +3 | +0.38 ±3.72 | +0 | ~noise |
| Competitive / floor#147 / deep | 2 | +4 | +2.00 ±0.00 | +4 | |
| Competitive / floor#18 / deep | 3 | +4 | +1.33 ±3.64 | +9 | ~noise |
| Defensive / floor#227 / round-2 | 10 | +4 | +0.40 ±3.92 | +13 | ~noise |
| Defensive / floor#42 / deep | 3 | +4 | +1.33 ±11.33 | +16 | ~noise |
| Competitive / floor#242+rb / deep | 2 | +5 | +2.50 ±0.98 | +0 | |
| Defensive / floor#5 / round-2 | 55 | +5 | +0.09 ±1.85 | -5 | ~noise plain/PD-flip |
| Competitive / floor#15 / deep | 6 | +6 | +1.00 ±3.47 | +2 | ~noise |
| Competitive / floor#40 / deep | 1 | +6 | +6.00 ±0.00 | +8 | ~noise |
| Competitive / floor#62 / balancing | 5 | +6 | +1.20 ±5.59 | +43 | ~noise |
| Defensive / floor#14 / round-1 | 3 | +6 | +2.00 ±9.87 | +12 | ~noise |
| Defensive / floor#60 / deep | 6 | +6 | +1.00 ±2.58 | +3 | ~noise |
| Competitive / floor#32 / balancing | 1 | +7 | +7.00 ±0.00 | +7 | ~noise |
| Constructive / floor#48 / deep | 8 | +7 | +0.88 ±4.63 | -7 | ~noise plain/PD-flip |
| Defensive / floor#226 / round-2 | 6 | +7 | +1.17 ±5.32 | +15 | ~noise |
| Defensive / floor#241 / balancing | 22 | +7 | +0.32 ±2.19 | -29 | ~noise plain/PD-flip |
| Defensive / floor#31 / deep | 4 | +7 | +1.75 ±1.23 | +9 | |
| Competitive / floor#27 / round-2 | 4 | +9 | +2.25 ±9.45 | -5 | ~noise plain/PD-flip |
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
| Competitive / floor#41 / round-2 | 2 | +11 | +5.50 ±0.98 | -24 | plain/PD-flip |
| Defensive / floor#48 / deep | 4 | +11 | +2.75 ±6.17 | +4 | ~noise |
| Defensive / floor#47 / deep | 5 | +12 | +2.40 ±5.87 | +17 | ~noise |
| Competitive / floor#151 / balancing | 4 | +16 | +4.00 ±4.80 | +16 | ~noise |
| Competitive / floor#63 / deep | 2 | +16 | +8.00 ±3.92 | +16 | |
| Defensive / floor#0 / round-1 | 3 | +16 | +5.33 ±2.85 | +17 | |
| Defensive / floor#145 / balancing | 2 | +16 | +8.00 ±3.92 | +16 | |
| Defensive / floor#16 / deep | 4 | +16 | +4.00 ±6.55 | +3 | ~noise |
| Defensive / floor#2 / round-1 | 7 | +16 | +2.29 ±6.72 | +17 | ~noise |
| Competitive / floor#61 / balancing | 4 | +17 | +4.25 ±3.03 | +29 | |
| Competitive / floor#0 / deep | 7 | +21 | +3.00 ±4.76 | +3 | ~noise |
| Competitive / floor#18+rb / deep | 2 | +24 | +12.00 ±1.96 | +26 | |
| Defensive / floor#62 / round-1 | 7 | +24 | +3.43 ±6.77 | +21 | ~noise |
| Competitive / floor#151 / round-1 | 4 | +26 | +6.50 ±6.28 | +26 | |
| Constructive / floor#148 / deep | 4 | +26 | +6.50 ±6.28 | +29 | |
| Constructive / floor#62 / deep | 25 | +27 | +1.08 ±2.61 | -5 | ~noise plain/PD-flip |
| Defensive / floor#1 / deep | 17 | +28 | +1.65 ±2.42 | -47 | ~noise plain/PD-flip |
| Competitive / floor#47+rb / round-2 | 2 | +30 | +15.00 ±1.96 | +30 | |
| Constructive / floor#153 / deep | 4 | +32 | +8.00 ±2.26 | +32 | |
| Defensive / floor#6 / round-1 | 19 | +32 | +1.68 ±3.40 | +37 | ~noise |
| Competitive / floor#239 / deep | 15 | +35 | +2.33 ±2.64 | -2 | ~noise plain/PD-flip |
| Competitive / floor#62 / round-2 | 53 | +35 | +0.66 ±1.87 | -233 | ~noise plain/PD-flip |
| Defensive / floor#210 / round-2 | 4 | +35 | +8.75 ±3.78 | +38 | |
| Defensive / floor#42 / round-1 | 4 | +38 | +9.50 ±2.59 | +51 | |
| Defensive / floor#57 / round-2 | 9 | +44 | +4.89 ±5.21 | +55 | ~noise |
| Defensive / floor#207 / round-2 | 8 | +45 | +5.62 ±2.92 | +65 | |
| Defensive / floor#140 / round-1 | 28 | +62 | +2.21 ±3.85 | +79 | ~noise |
| Defensive / floor#1 / round-2 | 217 | +74 | +0.34 ±1.06 | +127 | ~noise |
| Defensive / floor#62 / round-2 | 84 | +74 | +0.88 ±1.43 | +6 | ~noise |
| Defensive / floor#47 / round-2 | 139 | +75 | +0.54 ±1.21 | -103 | ~noise plain/PD-flip |
| Defensive / floor#56 / round-2 | 17 | +78 | +4.59 ±3.49 | +80 | |
| Competitive / floor#47 / deep | 24 | +80 | +3.33 ±2.26 | +43 | |
| Competitive / floor#1 / deep | 115 | +90 | +0.78 ±1.07 | +92 | ~noise |
| Defensive / floor#0 / round-2 | 27 | +147 | +5.44 ±2.14 | +147 | |
| Competitive / floor#0 / round-2 | 75 | +325 | +4.33 ±1.48 | +350 | |

## By phase

  -282622 IMPs  135928 boards  Constructive
  -268039 IMPs  113453 boards  Defensive
  -169320 IMPs   57796 boards  Competitive

## By provenance

  -386448 IMPs  186462 boards  book
   -91556 IMPs   31061 boards  floor#3
   -41732 IMPs   14275 boards  fallback@1
   -38097 IMPs   12466 boards  fallback@2
   -25167 IMPs    9276 boards  fallback@3
   -19584 IMPs    6857 boards  floor#242
   -10857 IMPs    3472 boards  fallback@4
    -8187 IMPs    2798 boards  floor#61
    -7896 IMPs    2393 boards  floor#243
    -7854 IMPs    2798 boards  floor#46
    -7424 IMPs    3419 boards  floor#20
    -6664 IMPs    2702 boards  floor#140
    -6126 IMPs    3079 boards  floor#35
    -4816 IMPs    1704 boards  floor#60
    -4675 IMPs    2867 boards  floor#50
    -3536 IMPs     916 boards  floor#202
    -3409 IMPs    1174 boards  floor#30
    -3299 IMPs    1698 boards  floor#64
    -3107 IMPs     828 boards  floor#31
    -2730 IMPs     827 boards  floor#45
    -2642 IMPs     579 boards  floor#16
    -2395 IMPs     752 boards  floor#132
    -1927 IMPs     572 boards  floor#200
    -1801 IMPs     263 boards  floor#242+rb
    -1546 IMPs    1434 boards  floor#65
    -1513 IMPs     548 boards  floor#131
    -1456 IMPs     430 boards  floor#32
    -1407 IMPs     476 boards  fallback@5
    -1243 IMPs     406 boards  floor#17
    -1213 IMPs     420 boards  floor#240
     -847 IMPs     203 boards  floor#145
     -823 IMPs     301 boards  floor#197
     -810 IMPs     307 boards  floor#237
     -778 IMPs     453 boards  floor#49
     -771 IMPs     251 boards  floor#241
     -759 IMPs     273 boards  floor#198
     -714 IMPs     454 boards  book+rb
     -647 IMPs     293 boards  floor#21
     -640 IMPs     270 boards  floor#239
     -632 IMPs     293 boards  floor#51
     -610 IMPs     200 boards  floor#66
     -579 IMPs     325 boards  floor#199
     -545 IMPs     322 boards  floor#5
     -542 IMPs     247 boards  floor#238
     -526 IMPs     143 boards  floor#15
     -506 IMPs     229 boards  floor#6
     -471 IMPs     140 boards  floor#235
     -453 IMPs     281 boards  floor#129
     -432 IMPs     254 boards  floor#3+rb
     -418 IMPs     201 boards  floor#236
     -401 IMPs     204 boards  floor#36
     -379 IMPs     400 boards  floor#133
     -377 IMPs      80 boards  floor#33
     -338 IMPs     140 boards  floor#234
     -320 IMPs     117 boards  floor#204
     -315 IMPs     107 boards  floor#147
     -307 IMPs      62 boards  floor#243+rb
     -299 IMPs      88 boards  floor#63
     -289 IMPs      76 boards  floor#10
     -267 IMPs      57 boards  floor#153
     -253 IMPs     184 boards  floor#205
     -252 IMPs     124 boards  floor#151
     -232 IMPs     381 boards  floor#47
     -208 IMPs     129 boards  floor#203
     -205 IMPs      50 boards  floor#27
     -198 IMPs      92 boards  floor#48
     -189 IMPs      57 boards  floor#240+rb
     -182 IMPs      69 boards  floor#12
     -177 IMPs      19 boards  floor#234+rb
     -174 IMPs      42 boards  floor#55
     -150 IMPs      69 boards  floor#157
     -149 IMPs      37 boards  floor#236+rb
     -143 IMPs      58 boards  floor#42
     -132 IMPs      26 boards  floor#11
     -131 IMPs      32 boards  floor#39
     -130 IMPs      46 boards  floor#46+rb
     -126 IMPs      49 boards  floor#9
     -113 IMPs      65 boards  floor#18
     -103 IMPs      41 boards  floor#238+rb
      -98 IMPs      16 boards  floor#26
      -87 IMPs      36 boards  floor#40
      -76 IMPs      20 boards  floor#54
      -72 IMPs      18 boards  floor#241+rb
      -69 IMPs      18 boards  fallback@6
      -68 IMPs      26 boards  floor#57
      -65 IMPs      21 boards  floor#24
      -61 IMPs     239 boards  floor#62
      -60 IMPs     135 boards  floor#2
      -60 IMPs      14 boards  floor#29
      -55 IMPs     102 boards  floor#25
      -49 IMPs      11 boards  floor#61+rb
      -45 IMPs     152 boards  floor#34
      -43 IMPs      11 boards  floor#60+rb
      -41 IMPs      52 boards  floor#41
      -39 IMPs       9 boards  floor#239+rb
      -38 IMPs       6 boards  floor#45+rb
      -32 IMPs       6 boards  floor#229
      -31 IMPs       4 boards  floor#63+rb
      -29 IMPs       7 boards  floor#218
      -28 IMPs       7 boards  floor#235+rb
      -25 IMPs       5 boards  floor#140+rb
      -24 IMPs       2 boards  floor#211
      -22 IMPs       3 boards  floor#13
      -22 IMPs       4 boards  floor#143
      -22 IMPs       3 boards  floor#28
      -17 IMPs       4 boards  floor#62+rb
      -16 IMPs       4 boards  fallback@4+rb
      -15 IMPs      11 boards  floor#228
      -14 IMPs       2 boards  floor#135
      -12 IMPs       8 boards  floor#144
      -12 IMPs       7 boards  floor#47+rb
      -11 IMPs       2 boards  floor#230
      -10 IMPs       9 boards  floor#14
      -10 IMPs       9 boards  floor#208
      -10 IMPs       1 boards  floor#212
      -10 IMPs       5 boards  floor#237+rb
      -10 IMPs       3 boards  floor#30+rb
       -8 IMPs       9 boards  floor#231
       -5 IMPs       1 boards  floor#219
       -2 IMPs       2 boards  floor#128
       +1 IMPs      13 boards  floor#154
       +2 IMPs       6 boards  floor#148
       +3 IMPs       8 boards  floor#127
       +4 IMPs      10 boards  floor#227
       +7 IMPs       6 boards  floor#226
       +9 IMPs       2 boards  floor#38
      +15 IMPs      37 boards  floor#56
      +22 IMPs       4 boards  floor#18+rb
      +35 IMPs       4 boards  floor#210
      +45 IMPs       8 boards  floor#207
     +139 IMPs     670 boards  floor#1
     +509 IMPs     112 boards  floor#0

## By family

  -364244 IMPs  143054 boards  round-1
  -220957 IMPs   93604 boards  round-2
   -94604 IMPs   47185 boards  opening
   -24178 IMPs   12203 boards  balancing
   -15998 IMPs   11131 boards  deep

## By direction

  -473954 IMPs   66650 boards  other
  -192188 IMPs   28986 boards  overbid
  -189936 IMPs   21833 boards  missed-game
  -175137 IMPs   26546 boards  sold-out
   -80176 IMPs    6556 boards  missed-slam
   -76980 IMPs   14405 boards  wrong-strain
   -11288 IMPs     778 boards  missed-grand
    -8751 IMPs    1388 boards  doubling
       +0 IMPs   51519 boards  flat
  +488429 IMPs   88516 boards  gain

## Worst boards per losing bucket

### Defensive / book / round-1 (58956 boards, -127014 IMPs)

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

### Constructive / book / opening (47185 boards, -94604 IMPs)

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

### Constructive / book / round-2 (41307 boards, -81380 IMPs)

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

### Constructive / book / round-1 (29658 boards, -70522 IMPs)

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

### Competitive / fallback@1 / round-1 (13968 boards, -41021 IMPs)

[vul both, seed Some(1783375079), board 667] swing -3220 pts / -22 IMPs (PD -22), diverged at call 2 (4♣ ours vs 4♥ BBA), other
  rule: 5+ ♣, (5+ ♥, or 5+ ♠), and 10+ points
  W:AQ95.J93.Q8.KQJ6 KJT864.A5.AT932. 3.KQT872.J.A9754 72.64.K7654.T832
  ours NS @ A: 1NT 2♦ 4♥ - - -  -> 4♥ by East
  ours EW @ B: 1NT 2♦ 4♣ - 4♦ X - - -  -> 4♦x by West

[vul both, seed Some(1783375074), board 650] swing -2010 pts / -19 IMPs (PD -19), diverged at call 2 (P ours vs 2♥ BBA), missed-slam
  rule: 0+ HCP
  W:AKQ964.98754..72 .AQ2.QT8532.KT53 J8732.T.K96.Q986 T5.KJ63.AJ74.AJ4
  ours NS @ A: 1♦ 2♦ - 4♠ - - -  -> 4♠ by East
  ours EW @ B: 1♦ 2♦ 2♥ - 3♥ - 4♠ - 5♦ - 6♦ - - -  -> 6♦ by South

[vul both, seed Some(1783375079), board 1001] swing -2040 pts / -19 IMPs (PD -18), diverged at call 2 (P ours vs 3♣ BBA), sold-out
  rule: 0+ HCP
  W:A9..QJ65.KQT9752 76532.T953.T7.84 8.QJ2.AK843.AJ63 KQJT4.AK8764.92.
  ours NS @ A: 1♦ 2♦ 3♣ - 3♥ - 5♥ - 6♣ - 7♣ - - -  -> 7♣ by West
  ours EW @ B: 1♦ 2♦ - 4♠ - - -  -> 4♠ by North

### Competitive / fallback@2 / round-1 (12130 boards, -37151 IMPs)

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

### Defensive / floor#3 / round-2 (9000 boards, -28828 IMPs)

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

### Defensive / floor#3 / round-1 (7997 boards, -26074 IMPs)

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

### Competitive / fallback@3 / round-2 (7349 boards, -18636 IMPs)

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

### Competitive / fallback@4 / round-2 (3046 boards, -9609 IMPs)

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

