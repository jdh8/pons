# 21GF ledger — pons 2/1 vs BBA's `21GF.bbsa`

Target spec for "author pons's 2/1 about as deep as BBA". One row per relevant
`vendor/bba/21GF.bbsa` toggle. The plan that governs this work:
`~/.claude/plans/author-the-2-1-bidding-replicated-mochi.md`.

**Status legend:** `shipped` (authored + tested) · `partial` (some of it) ·
`floor` (handled by `instinct()` only, not authored) · `gap` (absent) ·
`conflict` (pons plays it differently) · `override` (user chose ≠ 21GF) ·
`out` (out of scope).

**Campaign metric:** `examples/bba-match` IMPs/board vs BBA 2/1. Original baseline
**−2.59** (2000 boards); fresh re-measure **−1.997** (4000 boards, CI
[−2.16, −1.83], 81% divergent) — gap already narrowed by recent M6.1 work. Trend
it up as batches land. Worst-board themes: competitive doubles/advances +
balancing/reopening, and slam accuracy (missed grands).

## Locked decisions (from planning dialogue)

1. Mirror `21GF.bbsa` ON toggles; author by IMP-priority; user vetoes per item.
2. Shipped conflicts: keep pons's, A/B head-to-head; switch only if BBA wins.
3. Keycard: **keep 1430** (not 0314).
4. Jump shifts: **keep weak**; fit-showing only as a competition gadget later.
5. Beyond-21GF additions in scope: **plain-4NT minor keycard**, **Garbage Stayman**.
6. Out of scope: Multi 2♦, Benjamin/French 2D, Gambling, Gazzilli, Ghestem,
   Raptor 1NT, plain Landy, Polish/Wilkosz, Kickback/Crosswood.
7. Overrides: **Woolsey** over their 1NT (≠ Cappelletti); **Rubensohl /
   transfer-Lebensohl SEF-2018+** over interference on our 1NT (≠ Lebensohl).
8. **Kickback** postponed to its own session (spec in the plan).

## Constructive — 1NT/2NT responses

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 9 | 1N-2♠ transfer to clubs | shipped (two-way 2♠) | keep | — | — |
| 12 | 1N-3♣ transfer to ♦ | conflict (Puppet) | keep Puppet, A/B | — | — |
| 14 | 1N-3♦ majors | shipped | keep | — | — |
| 18 | 1N-3M splinter | verify | — | — | — |
| 88 | Minor transfers after 2NT | shipped | keep | — | — |
| 98 | Quantitative 4NT | shipped | keep | — | — |
| 109 | Smolen | shipped | keep | — | — |
| 119 | Texas | conflict (SAT 4♣/4♦) | keep SAT, A/B | — | — |
| 55 | Extended acceptance after NT | partial/verify | — | — | — |
| 115 | Super acceptance after NT | partial/verify | — | — | — |
| — | **Garbage Stayman** | gap (in scope) | add (Batch 3) | — | — |

## Constructive — suit raises & responses

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 7 | 1M-3M inviting (limit raise) | shipped | keep | — | — |
| 70 | Jacoby 2NT | shipped | keep | — | — |
| 113 | Splinter | shipped | keep | — | — |
| 68 | Inverted minors | shipped | keep | — | — |
| 37 | Bergen | conflict (limit+J2NT) | A/B (Batch 3) | — | — |
| 89 | Mixed raise | gap/conflict | A/B w/ Bergen | — | — |
| 56 | Fit showing jumps | conflict (weak JS) | keep weak; comp later | — | — |
| 116 | Support 1NT | verify | — | — | — |

## Constructive — opener rebids / checkback

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 57 | Forcing 1NT | shipped | keep | — | — |
| 125 | Two-Way NMF / XYZ | gap (floored) | add, 2NT→3♣ variant (Batch 3) | — | — |
| 58 | Fourth suit forcing | gap (floored) | add (Batch 3) | — | — |
| 124 | Two-way game tries | gap | add (Batch 3) | — | — |
| 52 | Drury | gap | add (Batch 3) | — | — |
| — | Meckstroth adjunct (pons-only) | shipped | keep (complementary to XYZ) | — | — |

## Slam

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 39/40 | Blackwood 0314 / 1430 | shipped 1430 | **keep 1430** | — | — |
| 75 | King ask by 5NT | shipped (majors) | keep | — | — |
| 35 | 5NT pick a slam | verify | — | — | — |
| 64 | Gerber | gap | add (Batch 2) | — | — |
| 53 | Exclusion | gap | add (Batch 2) | — | — |
| 51 | DOPI | gap | add (Batch 2) | — | — |
| 103 | ROPI | gap | add (Batch 2) | — | — |
| 42 | BROMAD | gap (confirm meaning) | Batch 2 | — | — |
| — | **Plain-4NT minor keycard** | gap (in scope) | add (Batch 2) | — | — |

## Competitive — our opening contested

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 80 | Lebensohl after 1NT | **shipped** | plain Lebensohl (Rubensohl tried first, lost) | Ruben −1.68/div; **Leben +0.26/div** (200k, 0.9% divergent) | next commit |
| 105 | Rubensohl after 1m | floor (Rubens advances) | upgrade (Batch 1) | — | — |
| 106 | Rubensohl after double | floor | upgrade (Batch 1) | — | — |
| 100 | Responsive double | floor | author (Batch 1) | — | — |
| 83 | Maximal doubles | gap | add (Batch 1) | — | — |
| 71 | Jordan/Truscott 2NT | gap | add (Batch 1) | — | — |
| 117 | Support double/redouble | shipped | keep | — | — |
| 28/30 | 1X-(Y)-2Z forcing/weak | partial | verify | — | — |
| 122 | Transfers if RHO bids clubs | gap | add (Batch 1) | — | — |

## Defensive — their opening

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 43 | Cappelletti | override → **Woolsey** | add (Batch 1) | — | — |
| 84 | Michaels cuebid | shipped | keep | — | — |
| 127 | Unusual 2NT | shipped | keep | — | — |
| 126 | Unusual 1NT | gap | add (Batch 1) | — | — |
| 79 | Leaping Michaels | gap | add (Batch 1) | — | — |
| 123 | Two-suit takeout double | gap | add (Batch 1) | — | — |
| 129 | Unusual 4NT | verify | — | — | — |
| 48 | Cue bid | partial | verify | — | — |

## Openings

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 20/24/26 | 1NT NT-style / 15-17 / 5422 | shipped (default) | keep | — | — |
| 5 | 1m allows 5M | verify | — | — | — |
| 132 | Weak natural 2♦ | shipped | keep | — | — |
| 133 | Weak natural 2M | shipped | keep | — | — |
| 92 | Namyats | conflict (natural 4-lvl) | A/B (Batch 3) | — | — |
| 33 | 4NT opening | gap (rare) | low priority | — | — |

## Jargon to confirm against authoritative text (at authoring time)

BROMAD (42) · Strength Lawrence structure (114) · Mixed raise (89) ·
Maximal doubles (83) · Super/Extended acceptance after NT (55,115) ·
Unusual 4NT (129) · Support 1NT (116).

## Out of scope (21GF turns OFF; user confirmed skip)

Multi 2♦ (90) · Benjamin 2D (36) · French 2D (60) · Gambling (61) ·
Gazzilli (63) · Ghestem (66) · Raptor 1NT (99) · plain Landy (78) ·
Multi-Landy as a whole (91 — but Woolsey's Multi-Landy *structure* is in via
override) · Polish two-suiters (97) · Wilkosz (134) · Kickback (72-74) /
Crosswood (45-47) · New Minor Forcing (94) / plain Checkback (44) ·
Soloway jump shifts (111-112) · Snapdragon (110) · weak jump shifts off in
21GF (130-131 — pons keeps weak JS regardless).
