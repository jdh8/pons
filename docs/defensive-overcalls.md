# Defensive round-1: the 1NT and suit overcalls — implementation and measurement

**Status: generic rows implemented; the direct-1NT reader trial failed its
non-loss gate and was reverted, blocking O1/O2; O3 passed both reference gates
and ships on by default; O4 stopped at its live-BBA model gate. O3 is the only
default changed.** First work package of the defensive round-1 redesign; its
sibling is
[takeout-double-layers.md](takeout-double-layers.md) (the multi-layered X,
second package). Both mine the same bucket: `Defensive/book/round-1`, the #1
BBA anchor bucket for six anchors running (−74,202 plain / −90,593 PD at
`782f09e`) and the #2 BEN bucket. Campaign context:
[ben-gap-campaign.md](ben-gap-campaign.md) (north star),
[bba-gap-campaign.md](bba-gap-campaign.md) (guard + microscope).

The redesign premise: the variable-row grammar
([rows.rs](../src/bidding/rows.rs) module doc) now expresses BBA-style
pattern rules — one template over all openings, bands computed per binding —
so the defensive initial actions can be restructured the way BBA's MB.TXT
structures them, instead of one flat table of hand-tuned rungs.

## Evidence — the BEN decompose residue

Source: `ab-results/ben-decompose/2026-08-10-42454d2/boards.jsonl`, bucket
`Defensive/book/round-1` (12,508 rows, −5,479 plain / −8,748 PD), minus the
shipped weak-jump slice (our `1M` vs reference `2M`): **11,160 rows, −3,994
plain / −7,432 PD**. Slices relevant to this document (net IMPs over the
102.4k-board Tier-F corpus; ref = BEN's call in our seat):

| slice (ours → ref) | n | plain | PD | reading |
| --- | ---: | ---: | ---: | --- |
| `1NT` → suit | 228 | −363 | −815 | our 1NT overcall fires holding a 5-card major; BEN bids the suit |
| `X` → `1NT` | 158 | −254 | −369 | our 12+ takeout tier wins hands BEN overcalls 1NT |
| `1♦` → `2♦` | 226 | −295 | −232 | weak jump in a **minor** — the shipped treatment covers majors only |
| `2♣` → `P` | 1,296 | −147 | **−2,222** | the 2-level wall: our light 2♣ overcall slaughtered under PD |
| `1♥` → `P` | 294 | −162 | −487 | light 1-level overcalls BEN declines |
| `P` → `3♣/3♦/3♥/3♠` | 489 | −875 | −315 | BEN's 3-level jump overcalls we never make |
| `P` → `2♥/2♠` | 483 | −764 | −137 | jumps outside the shipped exact-six, 8+ points, ≤11-HCP box |

Excluded here: `P` → `X` (n=1,359, plain −1,000 / **PD +1,182** — mixed sign;
see the takeout-double doc, which concludes the X's problem is boundaries, not
volume).

## Implemented state

Everything direct-seat lives in `defense_to_suit` at `P* (1x)`
([overcall.rs](../src/bidding/american/defense/overcall.rs)). The default
weight ladder (centinats) remains:

| w | call | band |
| ---: | --- | --- |
| 200 | Michaels cue | 5-5, `points(8..) & hcp(8..)` |
| 190 | Unusual 2NT | same band |
| 150 | 1NT overcall | `hcp(15..=18) & balanced() & stopper_in_their_suits()` |
| 150 | weak jump 2M; `(1♣) 2♦` | `len(suit,6..=6) & points(8..) & !hcp(12..)`, shipped |
| 140 | 1-level overcall | `len(suit,5..)`, floor 8 points (discipline on) |
| 130 | takeout X | `hcp(12..)` + Strict shape (≤3 in theirs, 3+ in every unbid) |
| 120 | strong X | `hcp(18..)` any shape |
| 100 | 2-level overcall | `len(suit,5..)`, floor 11 (9 passed hand); flat, no quality atom |
| 0 | Pass | `hcp(..18)` complement gate |

The old per-opening insertion has been replaced by one generic three-row
package using the regex declarative expander:

```text
P* (1x)          → defense_to_suit(1x)
P* (1x) 2x -     → Michaels advances
P* (1x) 2NT -    → Unusual-2NT advances
```

The rows bind all four one-suit openings and every zero-to-three leading-pass
fan. `suit_defense_rows_bind_every_opening_and_pass_fan` pins the direct and
both continuation rows. This is a behavior-preserving structural refactor, so
it needs regression coverage rather than an A/B.

At the structural checkpoint before O3's measured default flip, an isolated
clean-HEAD versus row-only comparison was byte-identical:
`render-book` SHA-256 `4540cd9e76209cf7dc5349610356cb99b150c4933dc6fd01e598676586b1cbce`
and `smoke-default --count 20000 --seed 1` SHA-256
`a0816ca83e741eeb1cbe3e0bcc8e45bc33d57921f07132fb7898d62eb773344f`.

The balancing seat `P* (1x) - -` is **unauthored** — floor territory, and it
stays that way (see Out of scope).

### Direct-1NT inference trial (measured and reverted)

The natural direct `1NT` overcall is unalerted, while the generic natural walk
cannot reconstruct its authored box. A route-local trial decoded exactly that
authored `(1x) 1NT` rule under `ReadingScope::Alerted`, preserved its initial
projection through systems-on stripping, and used the same projection when
Gladiator prevented stripping. `ReadingScope::None`, missing rules, actor
orientation, and caller-relative vulnerability were regression-pinned.

The trial was sound but failed the preregistered non-loss gate. In the trial
working tree atop base SHA `1922df3`, seed `1786449525`, and 204,800 BBA
boards per arm/vulnerability, fixed minus historical scored:

| scorer | none | both |
| --- | ---: | ---: |
| plain DD | −0.0008 ±0.0010 | −0.0003 ±0.0013 |
| PD | **−0.0016 ±0.0012** | −0.0012 ±0.0015 |
| plain SD | −0.0005 ±0.0010 | +0.0002 ±0.0013 |
| SD-PD | −0.0011 ±0.0012 | −0.0005 ±0.0015 |

The same seed's 25,600-board-per-arm/vulnerability Tier-F guard was entirely
wash and cannot rehabilitate the failed BBA gate:

| scorer | none | both |
| --- | ---: | ---: |
| plain DD | +0.0003 ±0.0025 | −0.0020 ±0.0031 |
| PD | −0.0010 ±0.0028 | −0.0041 ±0.0047 |
| plain SD | +0.0014 ±0.0025 | +0.0000 ±0.0029 |
| SD-PD | +0.0004 ±0.0030 | −0.0015 ±0.0045 |

Every changed auction contained the exact direct face and the first different
call was always ours; no scope leak appeared. The behavioral mechanism is the
forward authored projection: `hcp(15..=18)` publishes `15..37` (a sound floor,
not its band ceiling), replacing the historical synthetic 15–17 opening feature
seen by the already-frozen floor. That changed later competitive and game
judgment, and the non-vulnerable PD loss is CI-clear. The SD comparison also
used the trial reader for both stored arms, so it priced auction/contract
changes rather than historical-vs-trial disclosure.

Per the decision table, the trial was reverted. The historical reader remains
in production, O1/O2 are blocked, and no global natural-rule reading or defense
settings in `ReadingProfile` were added.

## Reference behavior

BBA (MB.TXT verbatim; 2009 export, orphaned from the live engine but
confirmed behaviorally at shallow depth — see
[ai-bidder/bba-floor.md](ai-bidder/bba-floor.md)):

- **Overcalls are quality-gated, not points-gated**: the templated family
  (MB.TXT:977) prices `quality(suit) + 2·(points−10) > 8 + 2·level` — a good
  suit buys a light bid, a poor suit needs 12+. The 2-level box is ~10–11 to
  18, 5+ cards (MB.TXT:3459).
- **1NT overcall = 16–18, stopper, no 5-card major** (MB.TXT:3466: shape
  `(4.|5[CDb])....[23]` — a 5-card suit only if a minor or their suit).
- **Aggressive weak jumps** behind toggle `#1(57)`: single jump **5+** cards,
  3 HCP to ~9+vul points; 3-level 6+ cards; double jump 7 (MB.TXT:3430-3436).
  Toggle off: intermediate 13–16 good suit.

BEN agrees on direction everywhere the residue bleeds: prefers the 5-card
major to 1NT, jumps in minors, declines quality-free 2-level overcalls.

## Candidates and gates

Ordered; each measured item is one experiment, arms sequential, fresh
`SEED_BASE`. O1/O2 and the inert O4 reservation remain default-off; measured
O3 now ships on.

### O1 — which unbid 5-card major should displace `1NT`?

Two independent comparison arms are implemented:

- `nt_overcall_no_major` is the **strict** arm. It caps every **unbid** major
  at four cards. Length in opener's major remains allowed, while an unbid
  five-card major is shown naturally even when that requires a two-level bid.
- `nt_overcall_prefer_one_level_major` is the **cheap** arm. It caps an unbid
  major only when that suit ranks above opener's suit and is therefore
  available at the one level. Over `(1♠)`, for example, five hearts may remain
  inside `1NT`; five spades may remain there under either arm because spades
  are opener's suit.

Strict wins if both fields are set. The old A5 measurement capped both majors
regardless of opener's suit, so it is historical context, not a verdict on
either implemented arm. The prerequisite direct-1NT reader failed before this
sweep ran. **O1 verdict: blocked; both fields remain default-off and the runner
rejects `o1`.**

**The trigger, when O1 unblocks (found 2026-08-13 while rebuilding
[takeout-double-layers.md](takeout-double-layers.md)).** Across all 448 in-node
boards where we overcall `1NT` (−446 plain / −1,002 PD), one cell — an **unbid
five-card major at 15–17 HCP** — is 46% of the rows and **80% of both losses**,
and BEN bids that exact major on 207 of the 256 such hands. That is the strict
arm's cell, at a *narrower* HCP band than either implemented arm applies. 18
HCP is 42% of the rows and the **least** lossy cell per board (−0.32 plain), so
an arm that also displaces the 18-count is diluting itself; a band clause
belongs on whichever arm runs. Note also that BBA's own `1NT` overcall is
**16–18** (`MB.TXT:3466`, subject to that file's provenance caveat), narrower
than our `hcp(15..=18)` at both edges.

**The band arm, measured 2026-10-12** (`nt_overcall_major_band` on the strict
arm, runner mode `o1band`, historical reader; seeds 1791735897 and
1791736497, numbers in the CHANGELOG and the ledger below).  Seed 1 read
`plain wash | PD win` at both vulnerabilities; seed 2 did not replicate it,
and its non-vulnerable plain SD lost outside the CI.  The worst boards share
one mechanism: the advancer passes a one-level overcall with game values
(`1♣ 1♠ - - -` opposite 9 HCP and five hearts; `1♦ 1♥ - 1♠ - 1NT - 2♥`
short of `3NT`), because the overcall's reading still stops below 15.  The
narrow band fixed the dilution, but the reader gate is the binding
constraint after all: displacing the 15–17 major pays only once partner
reads the overcall as possibly 15–17, and under a frozen floor that is a
reading knob that moves calls ([reading-drift-handoff.md](reading-drift-handoff.md)).
The knob stays opt-in.

### O2 — the `X → 1NT` forensic and independent widenings

The 158 hands our X wins where BEN overcalls `1NT` all pass Strict takeout
shape. Classification against the current natural-`1NT` atoms is now exact:
**150 fail only the stopper**, **7 fail only balance**, and **1 fails both**.
Stopperlessness is therefore the dominant hypothesis; balance widening is a
small, separately priced hypothesis rather than a bundled change.

The stopperless arm remains implemented default-off:

- `nt_overcall_without_stopper`: balanced 15–18 HCP may overcall `1NT`
  without a stopper; the configured O1 major preference still applies.

The semi-balanced arm was removed after the reader gate failed. The historical
systems-on rewrite treats the overcall as a constructive opening `1NT` and can
exclude hands the proposed arm admits (notably major 6322, every 7222, and some
18-HCP upgrades), so retaining that knob would make an unsound opt-in. The one
forensic hand failing both atoms is therefore not admitted by either surviving
candidate. **O2 verdict: blocked; stopperless remains default-off, semi-balanced
is not implemented, and the runner rejects `o2`.**

### O3 — exact minor weak-jump extension

`direct_minor_weak_jump_overcall` is implemented as exactly `2♦` over `(1♣)`:
`len(♦,6..=6) & points(8..) & !hcp(12..)`, with the same
`Alert("weak-jump-overcall")` as the shipped major family. Five-card and 12+
HCP diamond hands keep the simple `1♦` overcall; no other minor pair changes.
The `1♦ → 2♦` slice is clean on both scorers (−295/−232), and the existing
natural-preempt continuation handles the jump. The declarative row is:

```text
P* (1♣)   →  2♦ : len(♦,6..=6) & points(8..) & !hcp(12..)   [w150]
```

Note `3♣` over `(1♦)` etc. are 3-level jumps and belong to O6, not here.
In the treatment working tree atop base SHA `1922df3`, fresh seed
`1786465192`, and 204,800 BBA boards per arm/vulnerability, there were 516/515
contract divergences (0.25%) and 725/724 auction divergences (0.35%) for
none/both:

| scorer | none | both |
| --- | ---: | ---: |
| plain DD | +0.0000 ±0.0012 | +0.0002 ±0.0015 |
| PD | +0.0007 ±0.0014 | +0.0006 ±0.0017 |
| plain SD | −0.0003 ±0.0012 | +0.0003 ±0.0016 |
| SD-PD | +0.0003 ±0.0014 | +0.0005 ±0.0018 |

Per final-contract divergence, plain DD scored +0.012/+0.074 IMP and PD
+0.266/+0.221; per auction divergence, plain SD scored −0.072/+0.072 and
SD-PD +0.079/+0.144 for none/both.

Every cell is a wash and both honest brackets are non-negative. As an
established natural weak-jump treatment, O3 therefore passed the BBA gate by
the repository's naturalness tiebreak. The same seed's 25,600-board-per-arm/
vulnerability Tier-F guard also washed:

| scorer | none | both |
| --- | ---: | ---: |
| plain DD | +0.0004 ±0.0032 | +0.0004 ±0.0044 |
| PD | −0.0001 ±0.0040 | −0.0004 ±0.0054 |
| plain SD | +0.0007 ±0.0035 | +0.0016 ±0.0045 |
| SD-PD | +0.0005 ±0.0042 | +0.0010 ±0.0054 |

There were 69/73 final-contract divergences and 94/91 auction divergences for
none/both. No confidence interval excludes zero, so BEN did not refute the
BBA decision. Per final-contract divergence, plain DD scored +0.159/+0.151 IMP
and PD −0.043/−0.137; per auction divergence, plain SD scored +0.202/+0.451
and SD-PD +0.128/+0.286 for none/both. The result reports retain the eight
worst auction traces for every DD/PD bracket. **O3 ships on by default; setting
the field to `false` restores the simple `1♦` overcall.**

The alert's authored projection is admission-tested, but exact external range
disclosure is unavailable in every measurement harness: the BBA card has no
weak-jump-overcall range row, BEN receives only the auction context, and
`ab-dump-sd`'s opponent-side disclosure flags are currently inert (the known
limitation in `measurement.md`). These rows therefore price the references'
native interpretation and the changed auctions/contracts, not explicit
exact-six, 8+ points, ≤11-HCP disclosure.

### O4 — the 2-level quality gate (the wall)

The single biggest PD number in the residue (`2♣ → P`, −2,222 PD), and the
sd-wall diagnosis already ruled the bucket REAL under single-dummy. The flat
`points(11..)` floor is the wrong shape of gate: BBA charges **suit quality**
so a good suit bids light and a poor suit stays home.

**Model-selection gate, 2026-08-11: STOPPED — no model qualified.** The live
BBA probe covered every legal natural two-level pair (`(1♦) 2♣`, `(1♥)
2♣/2♦`, `(1♠) 2♣/2♦/2♥`), 5- and 6-card candidate suits, all 32
AKQJT masks, generated whole-hand HCP 6–18, and all four relative
vulnerabilities. The fitted staircase used our live `point_count(hand)` as
total strength; HCP was only the controlled-hand generation axis. Reproduce:

```sh
cargo run --quiet --release --example probe-bba-constraints -- \
  --mode o4 --vul none,we,they,both \
  --o4-replicates 2 --o4-holdout 1000 \
  --seed 1786431801 --out /tmp/o4-full-points.md
```

The controlled fit made **37,824** live-BBA calls: **31,264 labeled**
candidate-vs-Pass (**82.7% coverage**; 14,294 bid / 16,970 Pass), with 4,196
X and 2,364 1NT routes excluded. The disjoint random holdout made **24,000**
calls: **21,396 labeled** (**89.1% coverage**; 7,010 bid / 14,386 Pass), with
2,152 X and 452 1NT routes excluded. This conditional label is deliberate:
those higher-precedence calls hide whether BBA's natural-overcall gate itself
accepted the hand, and the coverage is reported so the accuracy cannot be
mistaken for whole-node accuracy.

| model | fit balanced accuracy | held-out balanced accuracy | worst pair/vul | monotonicity violations |
| --- | ---: | ---: | ---: | ---: |
| raw suit HCP | 90.58% | 90.59% | 89.23% | 0 |
| suit HCP + length | 94.31% | 93.88% | 92.36% | 0 |
| AKQJT mask + length | **95.18%** | **94.29%** | **92.53%** | 0 |

The preregistered rule was held-out balanced accuracy ≥95%, zero
monotonicity violations, then choose the simplest model within 0.5 percentage
points of best. The richest model missed the accuracy floor. **O4 stops here:
do not add `suit_quality` and do not activate a quality gate or A/B arm.**
`DefenseKnobs::two_level_overcall_quality` exists only as a false-by-default,
deliberately inert reserved field: the rule builder never reads it, neither
generator exposes a CLI flag, and both defensive-overcall A/B runners reject
`o4`. Keep that tombstone inert unless a new model-selection gate succeeds.
The existing `two_level_minor_overcall_tight` promotion candidate remains a
separate measured treatment, not evidence for inventing the failed atom.

**That fallback is now refuted too, 2026-08-12** — and it fails in the direction
this section predicted. A fresh-seed A/B at `abdafcc` (seed `1786488117`,
409.6k bd/arm/vul) put the tightening at plain DD **−0.0102 ±0.0021 NV** /
−0.0011 ±0.0027 vul with SD-PD **−0.0008 ±0.0026 NV** / +0.0090 ±0.0033 vul; the
NV plain-DD veto stopped the promotion and no Tier-F guard was run
(`docs/archive/bba-gap-anchor-history.md`, `ab-results/two-level-minor-overcall-refresh/`).
Forensics on the 20 worst NV boards read **trigger-too-broad**: 9–12 of them show
the loose arm's 2m overcall buying a profitable doubled sacrifice against a
making game that the 15-point arm never reaches. So both O4 levers have now
failed, from opposite ends — the quality atom missed its accuracy gate, and the
blunt points floor measures negative because *points are the wrong axis*, exactly
as the opening paragraph argues. The `2♣ → P` −2,222 PD leak is unclaimed again,
and nothing in the current toolbox addresses it; a future attempt needs the
suit-quality axis this section could not fit, not another strength band.

**Forensics superseded 2026-08-12, same day, by the population pass** over all
10,250 fired NV boards: the mechanism is a **declare-vs-defend switch**, not a
broad trigger — the loose default declares 35.1% vs the tight arm's 10.5%, is
doubled 3× as often, and still wins 0.58 IMPs per score-diverged board under
plain DD. §O4's thesis stands and sharpens: points are the wrong axis, and so is
the entry gate itself — the improvable decision is the contested 5-level node
the loose overcall later walks into. That node's pricing is now designed in
[ai-bidder/competitive-accountant.md](ai-bidder/competitive-accountant.md), with
the population tables and P(double) calibration in
[ai-bidder/doubling-calibration.md](ai-bidder/doubling-calibration.md).

Attribution wrinkle, recorded so nobody re-litigates it: vs BEN this slice
is plain-**positive** (+369) / PD −3,645, so it failed the *attribution
eligibility* gate — but the *ship* gate is plain-wash + PD-win by the
decision table, which a tightening can still pass. Score both, plus SD-PD.

**O4-vul — the vulnerability-gated floor (built 2026-09-25, A/B in flight).**
The BEN Phase 2 trace ([ben-gap-campaign.md](ben-gap-campaign.md), Tier-S
anchor `daa8bf4a`) re-priced this wall from the north-star side: in the
`Defensive / book / round-1` bucket, two-level suit overcalls where BEN
passes total −305 plain / **−1,144 PD** (n=572), concentrated in the
**vulnerable 5-card** slice (n=212 at `both`: −147 plain / −533 PD), with the
worst boards going for numbers (−800, −850, −1290) — and **BBA sides with
BEN's pass on all 572**.  That reframes the 2026-08-12 refutation: the
all-vulnerability arm was vetoed by its **non-vul plain-DD cell alone**
(−0.0102 ±0.0021), the cell where the declare-vs-defend switch bought
profitable sacrifices off BBA's structural inability to penalty-double;
its **vulnerable** cells measured plain −0.0011 ±0.0027 (wash) / SD-PD
+0.0090 ±0.0033 (win) — already the shippable shape.  BEN *does* double,
so the loose vulnerable overcall has no sacrifice equilibrium to hide in.
`defense.two_level_minor_overcall_vul_tight` (default **off**) applies the
same 15+ floor **only when vulnerable**, spelled `points_by_vul` so the
reading carries both bands; non-vul is byte-identical by construction
(seeded `smoke-default --count 20000 --seed 1` byte-identity
`e43c4554…` vs `main`).  This is a strength-band move, not the tombstoned
quality atom — §O4's gate on `suit_quality` stands untouched.  Runners:
`scripts/ab-two-level-minor-overcall-vul.sh` (BBA guard) then
`scripts/ab-ben-two-level-minor-overcall-vul.sh` (paired Tier-F, same
`SEED_BASE`).

**Measured 2026-09-25**, `d03ee106`, `SEED_BASE=1790348603`, non-vul cells
**0 fired** on every harness (the byte-identity, empirically).  Vul-both
IMPs/board deltas (ON − OFF):

| harness | fired | plain DD | DD-PD | plain SD | SD-PD |
| --- | ---: | ---: | ---: | ---: | ---: |
| BBA guard (204.8k bd/arm) | 1.71% | −0.0046 ±0.0038 | **+0.0099 ±0.0046** | −0.0101 ±0.0039 | +0.0022 ±0.0046 |
| BEN Tier-F (12.8k bd/arm) | 1.52% | +0.0080 ±0.0130 | **+0.0277 ±0.0169** | −0.0052 ±0.0138 | +0.0117 ±0.0172 |

Vs BEN — the campaign primary — the decision table reads shippable
(plain wash with a positive lean, PD win at +1.825 IMPs/fired, SD-PD wash
leaning positive).  Vs BBA the PD pair agrees (DD-PD win, SD-PD wash) but
both plain scorers read a loss — the tight arm stops collecting the
doubled sacrifices that exist because BBA does not penalty-double the
overcall directly.

**Verdict: NOT shipped — default stays off (jdh8, 2026-09-25).**  The
field-model call went the other way: in the current meta a direct double
of a two-level overcall is **takeout/negative by default**, so BBA's
no-direct-penalty-double behavior is *field-representative*, not a hole —
punishment of a vulnerable two-level overcall arrives only via the slower
reopening-X-converted-by-pass route (see [pdi.md](pdi.md)), which both
plain scorers already price.  It is BEN's direct penalty doubles, and the
PD scorer's assume-the-double pessimism, that overprice the risk on this
lane.  The BBA plain-DD −0.0046 is therefore a real cost and holds the
veto.  The knob stays as the measured opt-in treatment (house convention:
finished code, measurement done); re-measure only if the reference or the
meta changes.  Campaign corollary, worth its weight: **a vs-BEN PD win on
a doubling-sensitive lane needs this meta check before it counts** — the
BBA guard exists precisely to catch BEN-specific exploits, and this is the
first one it caught from the other side.

### O5 — 1-level quality floor

Same atom, same formula at level 1 (`1♥ → P`, −162/−487): the 8-point floor
would stand while a quality-free minimum bid gets charged. **Blocked by O4's
failed atom gate; no implementation or measurement is authorized.** Reopen it
only with a separately validated expressible model.

### O6 — 3-level jumps (opt-in, not fought for)

`P → 3x` is −875 plain but PD-mixed, and preempts measure DD-negative in
this harness (obstruction is invisible to DD). Author the BBA-shaped
family — 3-level jump = 6+ cards preemptive, double jump = 7 — as an
**opt-in knob**, measured once under SD-PD, shipped only if it clears.
Likewise the BBA aggressive 5-card single jump (`#1(57)` band): authored as
opt-in alongside, not fought for. The `P → 2M` residue (−764 plain, PD flat)
gets a band-edge sweep (floor 8→6) as a follow-up, not a lead item.

## Measurement plan

Per [measurement.md](measurement.md), no exceptions:

- Two-reference gate on every ship: 204.8k/arm/vul BBA plain+PD (+SD spot),
  then same-seed 25.6k Tier-F BEN. Ship = BBA pass + BEN non-refute. Watch
  the BBA−BEN spread (exploit guard).
- Arms sequential on this box, fresh `SEED_BASE` per experiment, no rebuilds
  mid-flight. The completed reader trial compared a saved pre-trial binary with
  the trial binary. O1/O2 stop at that failed prerequisite; O3 regenerates its
  control at HEAD.
- O3 fires only over `(1♣)`, so its enriched probe accepts raw hands before the
  bidder.
- New knobs sit outside the v5 floor's 13 trained slots, so they fold to
  zero and the A/Bs price the book alone
  ([ai-bidder/card-manifold.md](ai-bidder/card-manifold.md)). The defensive
  book is shared with `dutch()`, so O3's default applies to both. The checked-in
  convention-card goldens remain unchanged because their schema has no
  weak-jump-overcall range field; no unsupported `.bbsa` field was added.
- Disclosure bites this bucket specifically (def-r1 was the only head bucket
  to worsen at the `3c94802` disclosure flip). O3's complement is authored
  (`!hcp(12..)`), so its alerted projection carries both strength edges.

### Enriched O3 probe

The probe rejects raw South hands before bidding, retains only auctions where
the feature exposes its exact direct face and control differs, then reports DD
and PD. Run both vulnerability brackets with the fixed seed:

```sh
for vul in none both; do
  cargo run --release --example probe-defensive-overcalls -- \
    --count 20000 --vulnerability "$vul" \
    --seed 1786431804
done
```

Both brackets drew 1,042,771 raw deals, accepted 20,000 exact weak-diamond
hands, and reached 2,631 exact `(1♣) 2♦` faces: **0.25231% board trigger
density**. None had 1,712 contract divergences and scored DD
**−0.08894 ±0.16151 IMP/face** / PD **−0.22653 ±0.20606**, or board-equivalent
−0.000224 ±0.000408 / −0.000572 ±0.000520. Both had 1,709 divergences and
scored DD **−0.14177 ±0.21433** / PD **−0.25960 ±0.26410 IMP/face**, or
board-equivalent −0.000358 ±0.000541 / −0.000655 ±0.000666. All conditional
and board-equivalent cells are washes with a negative point-estimate lean.

### Full BBA and BEN runners

O3 owns a persistent fresh seed below its results directory and scores DD, PD,
SD, and SD-PD:

```sh
BASE=ab-results/defensive-overcalls/2026-08-11-1922df3
JOBS=32 PER_SHARD=6400 scripts/idle-run.sh \
  scripts/ab-defensive-overcalls.sh o3 "$BASE"
```

If BBA passes, run the same-seed Tier-F guard against 16 unchanged BEN servers
on ports 8085–8100:

```sh
BASE=ab-results/defensive-overcalls/2026-08-11-1922df3
SEED_BASE="$(<"$BASE/o3/seed")" \
  PER_SHARD=1600 scripts/idle-run.sh \
  scripts/ab-ben-defensive-overcalls.sh o3 \
  "$BASE/ben-o3"
```

| item | BBA DD/PD | SD/SD-PD | Tier-F BEN | ship verdict |
| --- | --- | --- | --- | --- |
| direct-1NT reader trial | plain wash; PD none −0.0016 ±0.0012 (loss), both −0.0012 ±0.0015 | plain SD wash; SD-PD wash-negative; contract effect only | all four cells wash both vuls | **reverted; failed BBA gate** |
| O1 strict / cheap | not run | not run | not run | blocked by reader gate; both OFF |
| O1 strict + 15–17 band (`o1band`) | seed 1 plain wash, PD win both vuls; seed 2 plain and PD wash | seed 1 SD-PD wash-positive; seed 2 plain SD none −0.0012 ±0.0007, SD-PD wash-negative | not run (BBA did not replicate) | **non-replicating wash; stays OFF** |
| O2 stopperless / semi-balanced | not run | not run | not run | stopperless blocked/OFF; semi-balanced removed |
| O3 exact `(1♣) 2♦` | all cells wash, non-negative DD/PD | all cells wash, non-negative SD-PD | all cells wash; no refutation | **ship default-on** |
| O4 quality gate | not run; stopped before A/B | not run | not run | no model, no atom, no active gate |

The same two runners also carry the takeout-double package's arms — `bar`
(`suppress_long_minor_takeout`) and `seam` (`defensive_seam_split`). Both were
measured 2026-08-13 on the full two-reference gate and **washed in every cell on
both references**, so both stay opt-in, default-off. Their design, evidence, and
verdict ledger live in [takeout-double-layers.md](takeout-double-layers.md), not
here; the transferable lesson is there too — `bar` fired on 0.07–0.10% of boards
(18 of 25,600 on the BEN guard), far below what its 69-board residue implied,
because the residue was counted over a corpus scoped differently from the node's
trigger. Price the trigger density before building.

## Defense to their **1NT** — the `(1NT) 2♦` mirror panel (forensic, 2026-08-23)

**Filter scope (2026-10-16):** the strict `--filter-1nt` scan covers this
lane's trigger to within 2%. BBA opens 1NT on 98.0% strict-balanced hands
and 2.0% 5m422, never 6m322 (4,000 `--isolate-defense` boards, seed
1792160000, none vulnerable), so strict-scan verdicts here stand unscoped
([the rule](archive/research-method-review-closed.md#filter-scope-the-rule-and-the-runner-classification)).

A different node from everything above: `chain_natural_overcalls`
([nt_defense.rs:95](../src/bidding/american/defense/nt_defense.rs)), the four
natural two-level overcalls of *their* 1NT, gated `len(suit,5..) &
points(8..=14)` (`natural_overcall_points`, weight 100, **no vulnerability
gate, no alert, no authored advance**).  Filed here because it is "they open,
we act"; the competitive-1NT campaign found it while pricing N4f and handed it
over ([one-notrump-competitive.md](one-notrump-competitive.md) §N4-mirror).

**Source.** Anchor arms
`ab-results/anchor-confirm/2026-08-22-053c4fb8/american-{none,both}`, read with
`probe-1nt-interference --table b --bucket "2♦"` (the `--table` / `--show-score`
flags and the solve-every-shown-board pass were added for this forensic).  The
two arms are the **same 204,800 deals at two vulnerabilities**, not 409,600
independent boards.  Numbers are board swings on the `B-only` cut (our own 1NT
uncontested or absent at table A) — they **rank, they do not isolate**.

### The campaign row quoted one sub-bucket of one arm

The whole lane, both arms pooled — every board where BBA opened 1NT and we
overcalled `2♦`:

| arm | bd | plain | **PD** | plain/bd | **PD/bd** |
| --- | ---: | ---: | ---: | ---: | ---: |
| non-vul | 1060 | +464 | +24 | +0.438 | +0.023 |
| vulnerable | 1079 | +232 | −421 | +0.215 | −0.390 |
| **pooled** | **2139** | **+696** | **−397** | **+0.325** | **−0.186** |

The headline `−558 PD` was the **`BBA passes` row of the non-vul arm alone**.
Split by BBA's response, pooled:

| BBA's response to our `2♦` | bd | plain | PD | plain/bd | PD/bd |
| --- | ---: | ---: | ---: | ---: | ---: |
| `X` (takeout — see below) | 1135 | +450 | +729 | +0.396 | +0.642 |
| **Pass** | **339** | **−209** | **−1529** | **−0.617** | **−4.510** |
| `3NT` | 102 | +319 | +328 | +3.127 | +3.216 |
| `2♥` | 134 | +122 | +111 | +0.910 | +0.828 |
| `2♠` | 120 | +121 | +56 | +1.008 | +0.467 |
| `3♣` | 98 | −57 | −54 | −0.582 | −0.551 |
| `2NT` | 73 | −51 | −48 | −0.699 | −0.658 |
| `4♠` / `4♥` / rare | 138 | +1 | +10 | — | — |

The overcall is not a hole; it is a lane that wins on plain DD and pays it back
under perfect defense, with one pathological sub-bucket.  Deleting the call
would give up the `X` and `3NT` rows too, and `Pass` is not selectable at bid
time.

### Two structural facts the sub-bucket rests on

**1. BBA cannot penalty-double our two-level overcall.** Of the 1135 boards
where BBA's responder doubled our `2♦`, **zero** ended in `2♦x`: the ladder is
`X P 2♥` (229), `X P 2♠` (156), `X P 3♦` (153), and the final contracts are
`3NT` ×200, `3♠` ×160, `3♥` ×155, `2♠` ×129, `4♥` ×120, `4♠` ×108.  Their `X`
is takeout.  So "BBA, a competent doubler, declined to double" is **not**
evidence that PD over-prices this lane — BBA structurally never gets the
chance, and a real field playing penalty doubles of a natural 1NT overcall
would.  The PD column here is the honest pessimistic end of the bracket, not a
PD-scorer artifact to wave away.

**2. Half of the `Pass` sub-bucket is at the other table and unreachable.**
Mean *our* raw score per board: at table B (we hold the `2♦` hand)
−151 plain / −125 PD across the lane, at table A (we are the 1NT **opener**,
BBA holds the same cards and passes) +167 plain / +118 PD.  On the `Pass`
sub-bucket those become −12 / −116 and −12 / −117 — i.e. the `Pass` boards are
~180 points/board worse than the lane at table A too, and a 1NT-*defense* knob
cannot move table A on these boards at all.  In raw points the `Pass` bucket's
deficit splits −4,170 plain / −39,520 PD at table A against −4,000 / −39,400 at
table B: **roughly half the row is not ours to fix.**

### Where the loss is, on axes we can gate

Every board in the lane sits inside `points(8..=14)` — the rule owns the node,
nothing leaks from the floor.  But `point_count` is HCP **plus distribution**,
so the 8-point floor admits 6- and 7-HCP hands (a 5-5 six-count reaches 8).
Split by the overcaller's raw HCP:

| HCP at the overcall | bd | plain | PD | plain/bd | PD/bd |
| --- | ---: | ---: | ---: | ---: | ---: |
| **≤ 7** | **264** | **−120** | **−158** | **−0.455** | **−0.598** |
| 8–9 | 833 | +210 | −259 | +0.252 | −0.311 |
| 10–11 | 679 | +325 | −7 | +0.479 | −0.010 |
| 12–13 | 295 | +274 | +59 | +0.929 | +0.200 |
| 14+ | 68 | +7 | −32 | +0.103 | −0.471 |

Monotone from ≤7 to 12–13 on **both** scorers, and the ≤7 rung is the only
slice in the whole forensic that is negative on both scorers at both
vulnerabilities (non-vul −43 plain / −66 PD, vul −77 / −92).  It is 12.3% of
the lane and sits entirely at rule-points 8 (207 bd) and 9 (57 bd) — a strict
refinement inside the two bottom rungs, not a blunt band move.

Priced as "boards a tightening would remove" (removal does **not** recover
these IMPs — those hands would pass into an unmeasured elsewhere; this ranks
candidates, an A/B decides):

| candidate cut | bd | plain | PD | nv plain/PD | vul plain/PD |
| --- | ---: | ---: | ---: | ---: | ---: |
| **HCP floor 8** (drop hcp ≤7) | 264 | **−120** | **−158** | −43 / −66 | −77 / −92 |
| HCP floor 9 (drop hcp ≤8) | 712 | −73 | −438 | +1 / −156 | −74 / −282 |
| points floor 8→9 | 439 | −24 | −198 | −7 / −66 | −17 / −132 |
| points floor 8→11 | 1273 | **+140** | −489 | +128 / −128 | +12 / −361 |
| points ceiling 14→13 | 122 | +55 | −39 | +45 / +2 | +10 / −41 |

Only the HCP floors cut a slice that is losing on plain DD as well.  The blunt
points-floor row is the `plain win / PD loss` shape §O4 already chased and lost
with — its 2026-08-12 refutation of `two_level_minor_overcall_tight` measured
**plain −0.0102 ±0.0021 NV**, mechanism "declare-vs-defend switch", conclusion
*points are the wrong axis*.  That precedent argues against the points band and
for the HCP refinement; it is also the reason to expect a tightening here to
give something back.

For orientation, our natural two-level overcall of a **suit** opening has a
floor of **11 points** ([Implemented state](#implemented-state)); over their
*strong notrump*, where our side is capped near 25 HCP and has no sacrifice to
buy, the same call is authored **three points looser**.

### The unauthored advance is the worst per-board cell

`(1NT) 2♦ -` has no authored node, so our advance is the instinct floor:

| our advance | bd | plain | PD | plain/bd | PD/bd |
| --- | ---: | ---: | ---: | ---: | ---: |
| Pass | 148 | +20 | −629 | +0.135 | −4.250 |
| `2♥` | 57 | −55 | −244 | −0.965 | −4.281 |
| `2♠` | 53 | −11 | −182 | −0.208 | −3.434 |
| **`2NT`** | **33** | **−135** | **−285** | **−4.091** | **−8.636** |
| `3♦` | 19 | −14 | −73 | −0.737 | −3.842 |
| rest | 29 | −14 | −116 | — | — |

The `2NT` advance fails on **26 of 33** boards (down 1 ×13, down 2 ×7,
down 3 ×4, down 4 ×2).  Worst board, non-vul: overcaller `96.A9532.Q7543.3`
(6 HCP, 8 points) bids `2♦` over `1NT`; advancer `J32.KJ4.T96.AQ95` — flat,
11 HCP, three diamonds — bids `2NT` **into** a 15–17 opener, and plays it four
down (−200 plain, −800 PD).  That is the iron rule's unauthored continuation,
firing on 17 combined HCP.  Absolute size is small (−135 plain in the `2♦`
lane; ×4 suits ≈ −0.0013 IMPs/bd of an arm), so it is a correctness fix that
rides a package, **not** something an A/B can resolve alone.

### Candidates

- **M1 — HCP floor on the natural two-level overcall of their 1NT.**  Add
  `& hcp(k..)` to `chain_natural_overcalls`, as a new
  `DefenseKnobs::natural_overcall_hcp_floor: u8` with `0` inert so the default
  stays byte-identical.  Do **not** move `natural_overcall_points`: it is a
  *reading* knob shared with the DONT and Meckwell one-suiter floors
  ([nt_dont.rs](../src/bidding/american/defense/nt_dont.rs),
  [nt_meckwell.rs](../src/bidding/american/defense/nt_meckwell.rs)) and with the
  Dutch profile's `(9, 13)`.  Arms `k ∈ {8, 9}` against the shipped default.
- **M2 — author the `(1NT) 2x -` advance.**  Pass as the finite catch-all,
  raise on support, new suit natural and non-forcing.  Completes the convention
  so M1 is measured against a live continuation, per the iron rule.

**SHIPPED DEFAULT-ON 2026-08-23** (the floor reverted to **0** on 2026-10-16, [the sweep](#every-live-floor-against-floor-0-the-sweep-2026-10-16--floor-0-ships)) — `natural_overcall_hcp_floor: u8` = **8** and
`natural_overcall_advance_enabled: bool` = **true** in `DefenseKnobs`.  `0` /
`false` restore the old behaviour.  As built:

| rung | constraint | weight |
| --- | --- | ---: |
| `4M` (partner's suit, majors only) | `len(t, 4..) & points(22 − lo ..)` | 140 |
| `3t` raise | `len(t, 4..) & points(20 − lo ..)` | 110 |
| new suit at the **two** level (ranks above `t`) | `len(s, 5..) & len(t, ..=2)` | 100 |
| `Pass` | `hcp(0..)` — the finite catch-all | 0 |

**Deviation from the candidate as written above, flagged not silently resolved:
there is no notrump rung at all.**  The original M2 line said "`2NT`/`3NT` only
on a real stopper plus values"; over a `1NT` opening there is no suit to stop,
and the values do not exist either — their opener holds 15–17 and partner is
capped at `natural_overcall_points.1`, so our side is capped near 25 HCP with
the strength sitting over the advancer.  Since a book node with finite mass
shadows the floor, dropping the rung *is* the fix to the lane's worst cell
(`2NT`, 33 bd, −4.09/bd plain, failing 26 of 33).  The reversible default is the
knob itself: it is off, and restoring an invitational `2NT` is a rung to add,
not a design to unpick.  Two consequences, both accepted: with a minor fit there
is no route to `3NT` or `5m` above the `3t` raise, and the advance is authored
only at the `-` tail — `(1NT) 2x (X)` stays with the floor, which is the lane's
**best** bucket (1135 bd, +450 plain / +729 PD) and needs no help.

Scoped to [`NotrumpDefense::Natural`], the shipped system and the measured lane;
the other bundles repurpose these slots and own their own advance packages.

### The A/B (`scripts/ab-nt-natural-overcall.sh pack`, 2026-08-23)

M1 `k = 8` **and** M2 as one package against the shipped default, per the
pre-registration.  `sha=42248850`, seeds `1787429313` / `1787430539`, 16 shards
× 12,800 = **204,800 boards per arm per vulnerability**, `--filter-1nt` on both
arms so they deal the same board set.  About **2.6%** of accepted deals fired.
The isolation gate (`probe-divergence --gate-opener theirs`) read **0 foreign of
~5,300 divergent boards in all four cells** — the knobs reach only auctions they
own.

| seed · vul | plain DD | PD | plain SD | **SD-PD** |
| --- | ---: | ---: | ---: | ---: |
| 1 · none | +0.0011 ±0.0034 | +0.0180 ±0.0045 | −0.0049 ±0.0035 | **+0.0086 ±0.0044** |
| 1 · both | +0.0068 ±0.0044 | +0.0287 ±0.0058 | −0.0020 ±0.0045 | **+0.0164 ±0.0057** |
| 2 · none | +0.0041 ±0.0033 | +0.0194 ±0.0045 | −0.0022 ±0.0035 | **+0.0100 ±0.0044** |
| 2 · both | +0.0096 ±0.0043 | +0.0305 ±0.0057 | +0.0024 ±0.0045 | **+0.0202 ±0.0056** |

IMPs/board; per fired, plain DD runs +0.04 to +0.38 and SD-PD +0.26 to +0.63.

**Verdict: the pre-registered rule is met in every cell** — a plain-DD non-loss
(all four positive, the two vulnerable cells CI-clear) *and* an SD-PD win (all
four CI-clear).  Shipped default-on.

**The 2×2 separates exactly as [measurement.md](measurement.md) says it must**,
which is what makes the verdict readable rather than lucky:

| axis | held fixed | measured |
| --- | --- | ---: |
| doubling (PD − plain DD) | double-dummy lead | +0.0169 / +0.0219 / +0.0153 / +0.0209 |
| doubling (SD-PD − plain SD) | blind lead | +0.0135 / +0.0184 / +0.0122 / +0.0178 |
| lead (plain SD − plain DD) | fallible doubler | −0.0060 / −0.0088 / −0.0063 / −0.0072 |
| lead (SD-PD − PD) | perfect doubler | −0.0094 / −0.0123 / −0.0094 / −0.0103 |

So **§O4's predicted give-back is real and visible** — the lead axis costs
−0.006 to −0.012, the declare-vs-defend switch charging for the overcalls we no
longer make — and the doubling axis pays 2–3× more.  That is precisely the
signature the forensic predicted from its first structural fact: the anchor
*cannot* penalty-double our two-level overcall, so plain DD never charges the
loose baseline for it and the two doubling-aware brackets do.  Read against
measurement.md's three-outcome table, SD-PD **higher** than plain SD is the
"treatment is the sounder bidder, so restoring the doubling punishes the
baseline more" row.

Two limits stated rather than papered over:

- **The package cannot be attributed.**  M2's own size (~−0.0013 IMPs/board of
  an arm) is below this A/B's resolution by design, so it ships as a
  correctness fix riding a measured win, never as a separately confirmed one.
  `bisect` mode exists to split them and was **not** run: the pre-registration
  reserves it for a loss or a split verdict, and running it now would be
  choosing an arm after seeing the result.
- **`k = 9` is untested.**  `k = 8` was the pre-registered ship candidate and it
  won; `nine` mode remains available as its own future question, not as a
  post-hoc upgrade of this one.

`smoke-default --count 20000 --seed 1` moves `39ca60a2…` → `eccf17bc…` (a real
bidding change).  Both `cards/*.bbsa` are **unchanged**: BBA's schema has rows
for named conventions, none for the strength band of a natural overcall or for
a natural advance, so there is nothing to disclose.

### Re-measure under the disclosed SD leader (2026-10-16)

The SD-PD column above came from `ab-dump-sd` before `7c60e309`, whose
East–West leader read only our alerted calls, so the unalerted `(1NT) 2x`
read as `points 8..37` in both arms and the floor's disclosure was never
priced when we declared ([research-method-review.md §4](research-method-review.md#4-disclosure-in-sd-dump-rescoring)).
Re-run on fresh deals with the current code:
[`scripts/ab-nt-natural-overcall-disclosed.sh`](../scripts/ab-nt-natural-overcall-disclosed.sh),
shipped (bare) against the pre-ship arm (`--ns-nt-overcall-hcp-floor 0
--no-ns-nt-overcall-advance`), SEED_BASE `1792107984640740881`, code
`178f301b`, `--filter-1nt --wide-1nt`, **393,216 accepted boards per arm per
cell**, all four vulnerability cells, density **0.253354** (accepted ÷
scanned), isolation gate 0 foreign in every cell, leader reading
`disclosed`. Pre-registered in the runner header before any number was read.

| cell | plain DD | PD | plain SD | **SD-PD** |
| --- | ---: | ---: | ---: | ---: |
| none | −0.0017 ±0.0018 NI | +0.0094 ±0.0024 win | −0.0045 ±0.0019 loss | **+0.0047 ±0.0023 win** |
| ns (we vul) | +0.0008 ±0.0021 NI | +0.0134 ±0.0028 win | −0.0020 ±0.0022 NI | **+0.0090 ±0.0028 win** |
| ew (we NV) | **−0.0035 ±0.0021 loss** | +0.0081 ±0.0027 win | −0.0066 ±0.0023 loss | **+0.0032 ±0.0027 win** |
| both | +0.0003 ±0.0024 NI | +0.0133 ±0.0031 win | −0.0017 ±0.0025 NI | **+0.0094 ±0.0030 win** |

IMPs per accepted board, `shipped − base`; about 1.5% of accepted boards
fired. **Gate result: the decision row is `loss | win`**, worst cell `ew`,
the row the decision table calls an artifact of PD's synthetic X. The `ew`
loss is −0.0009 IMPs per random board, inside δ, but its upper bound is
below zero. SD-PD is a win in every cell under the disclosed reader, so that
half of the original rationale stands; the plain-DD non-loss half fails in
the new `ew` cell (the 08-23 run measured none/both only).

**Trace: the win is the advance, not the floor.** `ab-dump-bucket --by node`
splits the divergent boards by the first differing call (plain / PD IMPs over
393,216 boards):

| slice | none | ns | ew | both |
| --- | ---: | ---: | ---: | ---: |
| M1 `1NT ⟨- vs 2x⟩` (the floor removes the overcall) | −2591 / −601 | −1661 / +748 | −3067 / −1053 | −2118 / +164 |
| M2 `(1NT) 2x - ⟨…⟩` (the authored advance) | +1748 / +4769 | +2685 / +6255 | +1569 / +4837 | +2575 / +6244 |
| later calls in the overcall auction | +169 / −491 | −715 / −1717 | +122 / −611 | −336 / −1180 |

The floor's slice loses on plain DD in all four cells and on PD when we are
not vulnerable. This reverses the 08-23 attribution, which was to the floor.
The slice compares Pass with an overcall that the instinct floor advances, so
it bounds M1 alone only loosely. That is the split verdict the original
pre-registration held `bisect` in reserve for.

> **Flagged for jdh8; nothing changed.** Proposed reversible default: keep
> both knobs until a pre-registered bisect on a fresh seed measures the floor
> on its own (advance on, `natural_overcall_hcp_floor` 0 against 8). If that
> reads a plain loss, set `natural_overcall_hcp_floor = 0` and keep the
> advance on. The knob value is the reversal.

### The floor on its own: bisect (2026-10-16)

[`scripts/ab-nt-overcall-floor-bisect.sh`](../scripts/ab-nt-overcall-floor-bisect.sh),
pre-registered (`e23c935a`) before any number was read: `floor8` (shipped)
against `floor0` (`--ns-nt-overcall-hcp-floor 0`), the advance on in both,
SEED_BASE `1792119766568383243`, `--filter-1nt --wide-1nt`, 393,216 accepted
boards per arm per cell, density 0.252942, gate 0 foreign in every cell,
leader reading `disclosed`.

| cell | fired | plain DD | PD | plain SD | SD-PD |
| --- | ---: | ---: | ---: | ---: | ---: |
| none | 3257 | −0.0074 ±0.0015 loss | −0.0051 ±0.0018 loss | −0.0075 ±0.0017 loss | −0.0058 ±0.0019 loss |
| ns (we vul) | 3234 | −0.0071 ±0.0017 loss | −0.0044 ±0.0020 loss | −0.0073 ±0.0018 loss | −0.0051 ±0.0020 loss |
| ew (we NV) | 3251 | −0.0076 ±0.0019 loss | −0.0056 ±0.0022 loss | −0.0082 ±0.0020 loss | −0.0066 ±0.0023 loss |
| both | 3207 | −0.0076 ±0.0020 loss | −0.0047 ±0.0024 loss | −0.0082 ±0.0021 loss | −0.0060 ±0.0024 loss |

IMPs per accepted board, `floor8 − floor0`; about −0.0019 per random board
plain, −0.6 to −0.9 IMPs per fired board. **The floor loses on every scorer
in every cell**, so `floor0` over `floor8` is win | win. The worst boards are
the light overcall's obstruction made visible to DD: over `1NT - 2♣` or a
transfer, the 2-level overcall pushes them out of their structure into a
worse contract (`1NT 2♦ 3NT` against `1NT - 2♣ - 2♦ - 3♠ X - 3NT X`), and
DD prices it because their contract actually moves.

> **The pre-registered reversal condition is met; flagged for jdh8, nothing
> changed.** Proposed default: `defense.natural_overcall_hcp_floor = 0` with
> the advance kept on. One seed; the earlier package run (SEED_BASE
> `1792107984640740881`) agrees in direction (its M1 slice lost plain in
> all four cells). Before flipping, decide whether a 0-HCP floor is the
> natural-overcall style wanted at the table, or whether an intermediate
> floor should be swept on a fresh seed.
>
> **Correction (2026-10-16): there is no floor between 1 and 6.** `points`
> is raw HCP plus an upgrade capped at 2, so `points(8..=14)` already holds
> every overcall to 6+ HCP and floors 1–6 bid exactly as floor 0 (checked:
> floor 6 against floor 0 on 32,000 seeded boards, identical auctions; floor
> 7 differs). The live floors are 7, 8 and 9, swept in
> [`scripts/ab-nt-overcall-floor-sweep.sh`](../scripts/ab-nt-overcall-floor-sweep.sh).

### Every live floor against floor 0: the sweep (2026-10-16) — floor 0 ships

[`scripts/ab-nt-overcall-floor-sweep.sh`](../scripts/ab-nt-overcall-floor-sweep.sh),
pre-registered (`edebcc80`) before any number was read: floors 7, 8 and 9
against floor 0, the advance on in every arm, a fresh SEED_BASE
`1792143998575635198`, `--filter-1nt --wide-1nt`, 393,216 accepted boards
per arm per cell, density 0.253419, gate 0 foreign in every pair, leader
reading `disclosed`. Floors 1–6 are not arms because they bid as floor 0
(the correction above).

| floor | cell | fired | plain DD | PD | plain SD | SD-PD |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 7 | none | 823 | −0.0013 ±0.0008 loss | −0.0020 ±0.0010 loss | −0.0003 ±0.0009 NI | −0.0011 ±0.0010 loss |
| 7 | ns | 845 | −0.0014 ±0.0009 loss | −0.0023 ±0.0011 loss | −0.0007 ±0.0009 NI | −0.0017 ±0.0010 loss |
| 7 | ew | 818 | −0.0013 ±0.0010 loss | −0.0021 ±0.0011 loss | −0.0004 ±0.0010 NI | −0.0013 ±0.0012 loss |
| 7 | both | 776 | −0.0010 ±0.0010 NI | −0.0022 ±0.0012 loss | +0.0001 ±0.0011 NI | −0.0011 ±0.0012 NI |
| 8 | none | 3227 | −0.0082 ±0.0016 loss | −0.0052 ±0.0019 loss | −0.0089 ±0.0017 loss | −0.0066 ±0.0019 loss |
| 8 | ns | 3261 | −0.0074 ±0.0017 loss | −0.0041 ±0.0021 loss | −0.0076 ±0.0018 loss | −0.0050 ±0.0021 loss |
| 8 | ew | 3249 | −0.0085 ±0.0019 loss | −0.0062 ±0.0023 loss | −0.0085 ±0.0021 loss | −0.0067 ±0.0023 loss |
| 8 | both | 3189 | −0.0073 ±0.0020 loss | −0.0044 ±0.0024 loss | −0.0086 ±0.0021 loss | −0.0068 ±0.0024 loss |
| 9 | none | 7712 | −0.0205 ±0.0023 loss | −0.0087 ±0.0028 loss | −0.0250 ±0.0025 loss | −0.0153 ±0.0028 loss |
| 9 | ns | 7745 | −0.0155 ±0.0025 loss | −0.0007 ±0.0031 NI | −0.0212 ±0.0026 loss | −0.0081 ±0.0031 loss |
| 9 | ew | 7745 | −0.0219 ±0.0028 loss | −0.0101 ±0.0035 loss | −0.0269 ±0.0031 loss | −0.0176 ±0.0035 loss |
| 9 | both | 7631 | −0.0167 ±0.0029 loss | −0.0020 ±0.0037 NI | −0.0238 ±0.0031 loss | −0.0113 ±0.0037 loss |

IMPs per accepted board, `floorK − floor0`. The loss grows with the floor,
and floor 7 already loses on PD in every cell, though it only removes the
6-HCP overcalls. `floor7 − floor8` (descriptive) reads plain +0.0060 to
+0.0072 and PD +0.0017 to +0.0041. **No floor is a candidate; floor 8
replicates the bisect** (plain loss in all four cells, the same size on a
second seed).

> **Decided 2026-10-16 (jdh8): `defense.natural_overcall_hcp_floor = 0`,
> the authored advance kept on.** The 08-23 package's gain was the advance.
> `bba-gen --ns-nt-overcall-hcp-floor` and `ab-dump-sd
> --{on,off}-ns-nt-overcall-hcp-floor` default to `0` with it; the historical
> runners pin `8` on their floor-8 arms.

#### Against BEN: floor 0 is a system, not a BBA exploit (2026-10-20)

Floor 0 shipped on BBA evidence only, and a 0–6 HCP overcall that gains with
failures doubled could gain because BBA mis-bids after it. Step 1 of the
1NT-defense plan asked BEN: `scripts/ab-ben-nt-overcall-floor.sh`, floor 8
against floor 0 over BEN's 1NT, Tier F, `--filter-1nt --wide-1nt`, 480
servers × 208 = 99,840 accepted boards per arm per cell, density 0.2529,
SEED_BASE 1792441814692277547, `ab-results/ben-nt-overcall-floor-2`. The
reading was pre-registered in the runner header. The first launch
(`ab-results/ben-nt-overcall-floor`) died at the isolation gate on a ben-gen
bug: BEN's model copied our arm's defense knobs (fixed in `f4e044e9`). Every
gate of the rerun passed with 0 foreign boards.

`floor8 − floor0`, IMPs per accepted board (95% CI), about 1,065 divergent
boards per cell:

| vul | plain DD | PD | SD plain | SD-PD |
| --- | ---: | ---: | ---: | ---: |
| none | −0.0038 ±0.0035 | −0.0034 ±0.0043 | −0.0047 ±0.0037 | −0.0042 ±0.0043 |
| both | −0.0041 ±0.0046 | −0.0012 ±0.0055 | −0.0071 ±0.0048 | −0.0046 ±0.0055 |
| ns | −0.0040 ±0.0036 | −0.0025 ±0.0046 | −0.0062 ±0.0039 | −0.0048 ±0.0047 |
| ew | −0.0034 ±0.0045 | −0.0017 ±0.0054 | −0.0060 ±0.0048 | −0.0050 ±0.0054 |

**Verdict: SYSTEM — floor 0 stands.** Floor 0 is plain non-inferior in every
cell (the largest upper bound of `floor8 − floor0` is +0.0011, below +δ),
and floor 8 wins no cell on PD or SD-PD; every one of the 16 point estimates
favours floor 0. The sign matches the BBA sweep in all four cells, so the
gain is not specific to BBA. Nothing changes; no second seed is owed.

### Follow-ups this A/B opened

- **`(1NT) 2x (X)` is still the floor; the census below closes widening it as
  below resolution.**  M2 authors only the `-` tail.  The contested tail shows
  up in the worst-board dumps (`1NT 2♠ X 2NT`, the floor bidding the very
  notrump rung M2 removed over a pass), but a `--show` tail is a selected worst
  tail, not a population claim.  The `(X)` bucket is the lane's **best** on the
  forensic census (1135 bd, +450 plain / +729 PD).
- **The `k = 9` arm**, above — measured in the sweep: a loss.
- **The 8-vs-11 floor gap** against a suit opening, still open and still
  proposed as "leave it".

#### `(1NT) 2x (X)` census — closed below resolution (2026-08-23)

The gate was registered before reading the output: build only if a non-Pass
advance is negative on both scorers at both vulnerabilities and costs at least
M2's ≈0.0013 plain IMPs per board of an arm.  The source is the pre-ship
`053c4fb8` anchor population (loose overcall, unauthored advance), read with
`probe-1nt-interference --table b --responses 1` across all four suits.  M1 has
since cut about 12% of the overcall population; the `(X)` tail's floor behaviour
is unchanged.  As above, **these rank, they do not isolate**.

Each vulnerability cell is `boards / plain total / PD total / plain total ÷
204800`; the pooled cell is `boards / plain total / PD total / plain per board /
PD per board`.

| our advance | non-vul | vulnerable | pooled |
| --- | ---: | ---: | ---: |
| Pass | 1159 / +213 / +562 / +0.001040 | 1222 / +330 / +661 / +0.001611 | 2381 / +543 / +1223 / +0.228 / +0.514 |
| `2NT` | 1 / 0 / 0 / 0 | 2 / −2 / −2 / −0.000010 | 3 / −2 / −2 / −0.667 / −0.667 |
| `2♥` | 34 / −3 / −11 / −0.000015 | 36 / −20 / −47 / −0.000098 | 70 / −23 / −58 / −0.329 / −0.829 |
| `2♠` | 48 / +43 / −22 / +0.000210 | 49 / +41 / −16 / +0.000200 | 97 / +84 / −38 / +0.866 / −0.392 |
| `3♣` | 5 / −10 / −19 / −0.000049 | 5 / −17 / −31 / −0.000083 | 10 / −27 / −50 / −2.700 / −5.000 |
| `3♦` | 5 / +11 / −7 / +0.000054 | 4 / +1 / −20 / +0.000005 | 9 / +12 / −27 / +1.333 / −3.000 |
| `3♥` | 35 / +53 / +5 / +0.000259 | 27 / −1 / −58 / −0.000005 | 62 / +52 / −53 / +0.839 / −0.855 |
| `3♠` | 27 / +84 / +46 / +0.000410 | 34 / +65 / +5 / +0.000317 | 61 / +149 / +51 / +2.443 / +0.836 |
| `4♥` | 1 / +12 / +12 / +0.000059 | 2 / −3 / −3 / −0.000015 | 3 / +9 / +9 / +3.000 / +3.000 |
| `4♠` | 4 / −29 / −32 / −0.000142 | 5 / −54 / −56 / −0.000264 | 9 / −83 / −88 / −9.222 / −9.778 |
| `XX` | 0 / 0 / 0 / 0 | 2 / +4 / +11 / +0.000020 | 2 / +4 / +11 / +2.000 / +5.500 |
| **non-Pass total** | **160 / +161 / −28 / +0.000786** | **166 / +14 / −217 / +0.000068** | **326 / +175 / −245 / +0.537 / −0.752** |

Sanity gate: the `2♦` rows contribute 562/573 `X` boards non-vul/vulnerable,
exactly the forensic's pooled 1135.  The decision gate does **not** open: the
whole non-Pass mass is plain-positive at both vulnerabilities.  The only calls
negative on both scorers in both arms are `2♥`, `3♣`, and `4♠`; the largest
plain arm cost among them is `4♠` at −0.000142/−0.000264, still about 5×
below the resolution floor in its worse arm.  Close the contested-tail
follow-up as below resolution, the same limit M2 itself carries; no build or A/B.

**Pre-registered reading** (before running, per
[measurement.md](measurement.md) and its domain addendum): M1 is a knob that
**bids less**, against an anchor that **cannot double**.  Plain DD is therefore
the *optimistic* end for the loose default (it never charges the overcall a
double BBA never makes) and the *pessimistic* end for the tightening; PD is the
reverse.  The arbiter is the realism pair **[plain DD, SD-PD]**
(measurement.md:188-212), both vulnerabilities, two seeds.  Ship default-on
only on a non-loss on plain DD **and** a win on SD-PD; a plain-DD loss with a
PD win is the §O4 shape and stays an opt-in knob.

### Step-2 forensics: where the 1NT defense still leaks (2026-10-19)

Step 2 of the 1NT-defense plan (step 1 is the BEN check of floor 0,
`scripts/ab-ben-nt-overcall-floor.sh`). Four evidence sweeps, then a skeptic
per proposed fix that re-priced it on clean data. Scratch, scripts and
per-board JSONL: `/data/public/jdh8/pons/forensics-2026-10-19/`. The data:

- **Natural census**: the floor sweep's `floor0-{none,both}` arms, all 96
  shards, 393,216 accepted boards per cell. Table A was replayed at HEAD
  `f8286a5a` with zero mismatches. Its table B is *our* E/W against BBA's
  defense of our own 1NT, so its table-A − table-B node costs are
  contaminated (`convention-tuning.md` § Per-call forensics). They rank lanes;
  they do not price nodes.
- **Natural isolate**: `bba-gen --isolate-defense`, `f8286a5a`, none,
  SEED_BASE 1792300000, 32,000 kept (0.2287 kept per accepted board).
- **Woolsey isolate**: `--isolate-defense --ns-notrump-defense woolsey`,
  `f8286a5a`, none, SEED_BASE 1792415758, 204,800 kept (density 0.0586).

What it found:

1. **The direct overcalls do not leak on plain DD.** Census lane totals,
   IMPs ×10⁻³ per accepted board, plain/PD, none: `2♥` +10.7/+5.0, `2♦`
   +10.5/+2.6, `2♣` +7.3/−9.6, `2♠` +6.9/−3.5, `X` +0.5/−2.7. At both, `2♣`
   is −2.5/−26.2. The PD losses sit where BBA passes and the overcall plays
   undoubled: step 1's question, not a continuation bug.
2. **The leak is our silent side acting later, all of it floored.** When
   they open 1NT, we have only passed, and their last bid is an invitation,
   the v6 floor competes or doubles; BBA passes at every such prefix. Clean
   (natural isolate) cost −0.0144 ±0.0020 plain / −0.0117 ±0.0019 PD per
   accepted board, ≈ −3.7/−3.0 per 1,000 random deals. Per fired board,
   plain/PD: `(1NT) - (2NT)` suit bid (160 bd) −2.00/−2.92; `(1NT) - (2NT) -
   -` X (78) −6.85/−1.37; `(1NT) - (2♣) - (2♥) - (2NT) - -` 3♣ (76)
   −8.57/−9.97; `(1NT) - (2♣) - (2♥) - (3♥)` X (52) −10.4/−6.5. Mechanism:
   `bba-gen` models BBA as playing our card, so their natural 1NT-2NT reads
   as our ♦ transfer (`ben-gen` already declares their European minors), and
   the floor then acts against 23–25 combined. Pass was the best surviving
   call on 56 of 56 probes with the competing calls masked. The 3♣ node is
   the R4 node `floor-rail-campaign.md` left to the retrain; here it is one
   member of a parametric family. Under Woolsey the same nodes cost −0.080
   plain per kept board: the leak is defense-independent.
3. **The M2 misfit escape is refuted as a leak.** The census priced the
   new-suit escape at −0.0049/−0.0143 per accepted board (both), but 98% of
   those boards split at or before the overcall (BBA passed on 70%). On the
   same deals, passing partner's overcall instead of escaping scores worse:
   pass − escape −1.77 plain / −3.47 PD per fired board at both, −1.00/−2.66
   at none, and dropping only the 5-2 escapes is plain-inert. Keep the rung.
4. **The natural penalty X's continuations are a small leak.** A
   counterfactual (substitute the sit/run advance, re-bid with pons vs EPBot,
   DD) gains +0.74 to +1.78 ×10⁻³ plain per accepted board, every lower bound
   above 0, ≈ +0.2 to +0.45 per 1,000 random deals. About half of it is the
   redoubled node: the advancer sits 1NTxx on 0–4 HCP, and the doubler sits
   in 1NTxx on 56/65 boards per cell (none/both) at −7.0/−8.3 IMPs each,
   17/22 of them holding a 5+ suit. The June instinct runouts
   (`advancer_xx_runout`, `doubler_xx_runout`) cover exactly those hands but
   are inert under the net floor: neither node is a forced rail. BBA's
   runout after our penalty X is transfers (opener completes every time), so
   an authored `(1NT) X (2x)` table would need a transfer reading.
5. **Woolsey at HEAD trails BBA's Woolsey by −0.170 ±0.012 plain / −0.232
   ±0.014 PD per kept board** (NV), not the July −0.23 plain
   (`ai-bidder/gto-1nt-defense.md` § Rung 1). Split: −0.050 is harness
   pollution (BBA's N/S opens first at table B on 5.9% of kept boards;
   `--isolate-defense` checks table A only), −0.095 the defense-independent
   floor family of item 2, −0.020 the continuations after a Woolsey action,
   and −0.005 plain / −0.078 PD card divergence. The largest authored loss is
   the doubled-Muiderberg 2NT escape (−0.0109/−0.0178), 94% of it escaping
   with a doubleton in the major; BBA escapes only with a void or singleton.

Packages built from it, all default off, A/B owed after step 1 (jdh8,
2026-10-19):

| package | knobs | runner | clean estimate |
| --- | --- | --- | --- |
| silent rail over their 1NT invitations, plus a reading arm | `InstinctProfile::their_nt_invite_{double,bid}_veto`, `bba-gen --declare-their-european-minors` | `scripts/ab-nt-invite-rail.sh` | +2.3 to +3.5 plain, +1.8 to +2.8 PD per 1,000 random deals (none) |
| XX runouts as forced rails | `InstinctProfile::xx_runout_rails` | `scripts/ab-xx-runout-rails.sh` | ≈ +0.1 to +0.3 per 1,000 (rare) |
| Woolsey completion | `defense.woolsey_muiderberg_sit`, `defense.woolsey_tails` | `scripts/ab-woolsey-complete.sh` | +0.65 to +1.0 plain per 1,000 NV, inside the opt-in card |
| Woolsey card coverage | four `defense.woolsey_*` knobs | `scripts/ab-woolsey-coverage.sh` | +0.3 to +0.5 plain per 1,000 NV |

**Verdicts, 2026-10-20** (vs BBA; numbers per accepted or kept board, full
cells in CHANGELOG):

- **Silent rail — shipped default-on, both halves.** Both vs neither plain
  +0.0151 to +0.0182, PD +0.0138 to +0.0166 (`win | win` every cell,
  ≈ +0.004 per random deal, about 1.5× the clean estimate); the bid veto
  wins on top of the double veto in every cell. The reading arm wins too
  (ns plain only non-inferior) and, with it on, the rail almost never
  fires: the rail's gain is mostly our Puppet misread of their `2NT`.
  The declaration became `bba-gen`'s default the same day (jdh8: honest
  bridge); the rail stays, since it still wins slightly under it.
  BEN guard (`scripts/ab-ben-nt-invite-rail.sh`, pre-count, none, 8,256
  boards, SEED_BASE 1792490245610720393): BEN reaches `(1NT) - (2NT)` with
  our pass on 77 boards (0.9%), and the arms diverge on **0**: under the
  honest reading the floor already passes there. Rule 0: infeasible at BEN's
  throughput; the rail stays on its BBA evidence, a recorded gap.
- **Woolsey completion — shipped default-on inside the Woolsey card.** Sit
  plain +0.0355 to +0.0557, PD +0.0589 to +0.0796 per kept board; tails
  plain +0.0054 to +0.0069, PD +0.0063 to +0.0083; additive. ≈ +2 to +3
  per 1,000 random deals, 3× the clean estimate.
  BEN guard **confirmed** 2026-10-20 (`scripts/ab-ben-woolsey-complete.sh`,
  `ab-results/ben-woolsey-complete`, SEED_BASE 1792493644940166504, Tier F
  on 48 servers, 8,256 kept boards per arm per cell, density 0.062–0.063,
  every gate 0 foreign, 104–108 contract-divergent boards per cell): sit and
  tails together, shipped − base per kept board, none / ns / ew / both,
  plain +0.0057 ±0.0140 / **+0.0164 ±0.0160** / +0.0144 ±0.0162 /
  **+0.0248 ±0.0172** (lowest lower bound −0.0083 against −δ ≈ −0.032), PD
  +0.0394 / +0.0489 / +0.0522 / +0.0585 (win), SD plain +0.0004 to +0.0200,
  SD-PD +0.0305 to +0.0518 (win). Rule 1: the knobs stay as shipped. The
  gain is smaller than against BBA (plain about a seventh to two-fifths of
  sit + tails, PD about two-thirds). Worst boards, none a loss to trace: the
  tails' six-card `2♠` advance (`- (1NT) 2♦ (2♥) 2♠ … (4♥)`, −14 to −16)
  and the sit passing `(1NT) 2♠ (X)` where base escapes to `2NT` (−12 to
  −14).
- **XX runouts — neither ships; both knobs stay off.** The first run
  stopped at the `dont` arm's isolation gate (the undeclared mirror read
  their runouts after our own `1NT (X) XX` as DONT; fixed in
  `common::mirror_agreements`). Re-run `ab-results/xx-runout-rails-2`
  (fresh seed, 393,216 accepted boards per cell, every gate 0 foreign,
  108–176 contract-divergent boards per cell so rule 0 passes), per
  accepted board, none / ns / ew / both:
  - `rails − off`: plain −0.0005 / −0.0000 / **−0.0009 ±0.0006 (loss)** /
    −0.0004, PD +0.0007 to +0.0009 (win). Rule 4's predicted culprit: the
    advancer arm sits weak 4-4 / 4-3-3-3 hands in `1NTxx` where the net
    runs, and `1NTxx` makes (47–55 boards per cell, plain −336 to −687
    IMPs); the doubler arm gains (plain +87 to +151). PD's win is the
    rule-3 bias in the rail's favour (it erases the redouble of the making
    `1NTxx`).
  - `dont − off`: plain **+0.0015 / +0.0015 / +0.0018 / +0.0019 (win)**, PD
    −0.0004 / −0.0010 / −0.0005 / −0.0012 (loss), SD-PD −0.0002 to −0.0009.
    Rule 2: no ship. Rule 3 split: the PD loss lives in the biased boards
    (off sits a making `1NTxx`: plain +613 to +857, PD −111 to −311; our
    runout fails undoubled and PD doubles it: plain +67 to +97, PD −53 to
    −113). On the boards PD scores without bias DONT is plain −92 to −169,
    PD −36 to +18 per cell, and that loss sits in the advancer's
    two-suiters (`2♣`/`2♦` where the net bids a natural `2♥`/`2♠`: a
    five-card major with a four-card minor goes through the minor).
  - Recorded trade-off for jdh8 (rule 3): DONT's plain gain is real (it
    escapes making `1NTxx`) and PD cannot see it. A redesign that bids a
    five-card major naturally ahead of a 5-4 two-suiter, plus the rails'
    doubler arm alone, is the follow-up.
  - Follow-up built 2026-10-20: split by the advancer's shape, DONT's
    two-suiters with a longer major were 94 boards, plain −203, PD −301
    (5M-5m: 8 boards, plain −82; flat 4-4s plain +1184, PD −601). Knob
    `defense.xx_dont_major_first` (`Off` / `Longer` / `AtLeastAsLong`,
    default `Off`); jdh8 asked for both modes to be measured. Runner
    `scripts/ab-xx-dont-major-first.sh` (arms `off` / `dont` / `longer` /
    `tied` / `doubler`, pre-registered there, rules 6–7 for choosing among
    alternatives).
  - **Verdict 2026-10-20** (`ab-results/xx-dont-major-first`, SEED_BASE
    1792489934007903414, 393,216 accepted boards per cell, every gate 0
    foreign), per accepted board, none / ns / ew / both:
    - `longer − off`: plain +0.0014 / +0.0012 / +0.0017 / +0.0017 (win),
      PD −0.0002 / **−0.0009** / −0.0002 / **−0.0006** (loss in ns, both).
      `tied − off` within 0.0001 of it; `dont − off` 0.0001 below it.
      Rule 2: no ship.
    - Rule-3 split (`tied − off`): the plain gain is the `sits` bucket (off
      sits a making `1NTxx`): 46–57 boards, plain +488 to +647, PD −99 to
      −249 per cell. On the unbiased boards `tied` is plain +10 / −89 / +11 /
      +1 and PD +76 / −30 / +104 / +92; `dont` was plain −26 to −133. The
      major-first shape closes most of the unbiased hole; what PD charges is
      now the bias.
    - `longer − dont` / `tied − dont`: positive on both scorers in all four
      cells but 12–15 divergent boards per cell, pooled plain +0.6 / +1.1
      ×10⁻⁴ with lower bounds −0.43 / −0.17 ×10⁻⁴. Rule 7 not met:
      inconclusive, `Off` stays the package's shape.
    - `doubler − off`: 13–16 divergent boards per cell, every cell under
      rule 0's 50, plain +1 to +16 IMPs, PD −51 to −89. Rule 0 asks for a
      2× re-run, which still stays under 50; the doubler node is reached
      mostly when the advancer sits, so the rails' doubler gain in
      `xx-runout-rails-2` was the advancer rail's own sits being rescued.
      Not re-run (owed to jdh8 if wanted, at ≥ 4×).
    - Owed to jdh8: DONT (with `tied` or `longer`) is a plain win in every
      cell and a PD loss that the split puts in PD's bias against running —
      the same recorded trade-off as before, now without the unbiased
      plain loss.

Dropped: the M2 escape (item 3); the their-4NT pull veto (≈ 8 boards on
v6, below the rail series' bar); the `(1NT) - (2♠) - (3♣)` 3♠ lane (inside
the fourth-seat lane closed 2026-09-27); the 12+ HCP 5-5 minors route for
Woolsey (route undecided, owed to jdh8).

## Out of scope (decided, not deferred by neglect)

- **Balancing/reopening seat** — stays with the floor lever (B2 item 0, the
  −37k floor#3 buckets). A book package at `P* (1x) - -` would shadow the
  floor there permanently; measured floors have beaten authored nodes
  repeatedly in this repo.
- **Relational constraints** (BBA's reopening band clamped to opener's shown
  minimum): a new dependency class for `Constraint` serving only the
  balancing seat — dead until balancing is book work, which it is not.
- **`P → X` volume** — see [takeout-double-layers.md](takeout-double-layers.md).

## The DONT-style runout after `(1NT) X (XX)` (built and measured 2026-10-20, default off)

Step 2 of the 1NT-defense plan, package B, book alternative to the ladder
rails (`InstinctProfile::xx_runout_rails`). Knob `defense.xx_dont_runout`,
**default off**, ours alone (the undeclared mirror pins it off, so their
runouts after our `1NT (X) XX` keep the house reading — the first A/B's
isolation gate caught that leak, 2026-10-20); code
`src/bidding/american/defense/nt_xx_runout.rs`; arms
`off` / `rails` / `dont` of `scripts/ab-xx-runout-rails.sh` (pre-registered
there). Owner decisions jdh8, 2026-10-20. Step-2 ledger row:

| package | knobs | runner | clean estimate |
| --- | --- | --- | --- |
| DONT-style XX runout (book) | `defense.xx_dont_runout` | `scripts/ab-xx-runout-rails.sh` (`dont` arm) | the four-card share of ≈ +0.1 to +0.3 per 1,000 (rare) |

**The fact it rests on.** Their `1NT` is 15+, our natural penalty `X` is 15+,
so the advancer holds at most 40 − 15 − 15 = 10 HCP and, after a business
redouble (our own floor reads 7+), 0–3 in practice. Karen Walker's
values-pass branch ("pass with values, play `1NTxx`") is empty against a strong
`1NT`, so the advancer's pass can be **forcing** at no cost, and running is
right at every vulnerability: a made `1NTxx` is 560 / 760 plus 200 / 400 an
overtrick, against 300 / 500 for a doubled two-level runout two down. The
doubler is balanced (short in at most one suit), so any two four-card suits
find a seven-card fit through pass-or-correct and any five-card suit is a
seven-card fit; only a 4-3-3-3 naming its four-card suit opposite the
doubler's doubleton lands in 4-2.

**The scheme** (direct seat `P* (1NT) X (XX)` only, two leading passes at
most; natural defense only). The balancing `(1NT) - - X (XX)` is excluded
because that advancer passed `1NT` in the direct seat and *can* hold values.

| Node | Call | Shows | Alert |
| --- | --- | --- | --- |
| advancer, `(1NT) X (XX)` | `2♣` | ♣ and a higher four-card suit, 4-4+; three-suiters bid it | `1ntd:xx-runout-2c` |
| | `2♦` | ♦ and a four-card major, no four clubs | `1ntd:dont-2d` |
| | `2♥` | both majors 4-4+, no four-card minor | `1ntd:dont-2h` |
| | `2♠` | five-plus spades, no other four-card suit | — |
| | `-` | **forcing**: a 5+ one-suiter in ♣/♦/♥ or any 4-3-3-3, at most 10 HCP | — (a pass) |
| doubler, `… (XX) - -` | `2♣` | relay, any hand; never passes | `1ntd:dont-pc` |
| advancer, `… - - 2♣ -` / `2♣ (X)` | `-` / `2♦` / `2♥` / `2♠` | clubs / that suit: five-plus, or the 4-3-3-3's four (a four-spade 4333 passed first, so a direct `2♠` stays five) | — |
| doubler, `… (XX) 2♣ -` | `-` / `2♦` | three-plus clubs / "name your higher suit" | `1ntd:dont-pc` |
| doubler, `… (XX) 2♦ -` | `-` / `2♥` | three-plus diamonds / pass-or-correct | `1ntd:dont-pc` |
| doubler, `… (XX) 2♥ -` and `2♥ (X)` | `-` / `2♠` | hearts at least as long / longer spades | `1ntd:dont-pc` |
| doubler, `… (XX) 2♣ (X)` / `2♦ (X)` | `-` / `XX` | three-plus / **SOS**: "bid your other suit" | `1ntd:xx-runout-sos` |
| advancer after the SOS | `2♦` / `2♥` / `2♠` | the cheapest four-plus suit above the one shown | — |
| any later double of theirs | `-` | sit: their redouble was business, so every later double is penalty | — |

The advancer's table is an exact partition of every hand up to 10 HCP, so each
call's reading is its own shape. HCP is a condition nowhere except the forcing
pass's `hcp(..=10)`, which is what makes it *read* as weak; a one-suiter or
4-3-3-3 above it cannot occur opposite 15 + 15 and is left to the floor
rather than to a catch-all, whose ⊤ reading would erase the pass's. The pass
cannot be alerted (`docs/authored-reading-handoff.md`: artificiality is
bid-only); its forcing nature is the doubler's table having no `Pass`, and its
meaning reaches the reader through the pass reading (`project_pass`: the
union of the node's Pass rules). The relays reuse `nt_dont.rs`'s
`passed_dont_*` tables and its "one table whether the relay is passed or
doubled" idiom; `sit()` is `nt_landy.rs`'s.

**Owed (jdh8, 2026-10-20): the doubler's pass reads as anything.** In the
reused `passed_dont_*` tables the doubler's catch-all `Pass` over a two-suiter
reads as any hand, not "3+ in the suit". Tightening it sharpens the reading
but moves DONT's own lane too, so it is its own change with its own A/B, after
this package's verdict.

**PDI and the latch.** None of these calls is a PDI trigger: business and SOS
redoubles are excluded by definition ([pdi.md](pdi.md)), the structural
conversion `X - -` cannot match because every double here is theirs, and the
penalty-latch double reader (`penalty_latch_double_reading`) counts only our
`Call::Double`, so the SOS `XX` is never read as a penalty double to sit for.

**Against the rails.** A book node with finite mass shadows the floor, so with
both knobs on this package answers `(1NT) X (XX)`, `(1NT) X (XX) - -` and every
continuation it keys, and the rails never fire there. The rails cover the
five-card runouts only (a 4-4 or 4-3-3-3 bust still sits `1NTxx` on the
ladder); this package covers both. The runner measures each against `off`,
with `dont − rails` descriptive, and pre-registers which ships if both qualify.

**Open choices** (the write-up's §5, decided as above; revisit only with
data): the 4-3-3-3 names its own four-card suit (pure DONT) rather than
passing and letting the doubler bid his cheapest suit (Bridgebum / Walker's
scramble — 4-2 risk about 2.3% vs about 4% of advancer hands, a priori a wash);
a doubled `2♥` is answered as undoubled (pass or `2♠`), since its other suit is
known. Not changed: DONT's own `(1NT) X (XX)` keeps its `2♣` relay where
Bridgebum's DONT turns it off.

**Major first** (`defense.xx_dont_major_first`, default `Off`; built after the
first A/B priced the advancer's 5M-4m two-suiters at −203 plain over 94
boards). `Longer`: a five-plus major strictly longer than its four-plus minor,
at most three of the other major, bids `2♠` (spades) or passes and names `2♥`
over the relay (hearts). `AtLeastAsLong` adds the 5-5s. Both majors still bid
`2♥`. Verdicts in § step-2 verdicts above (`ab-results/xx-dont-major-first`).

### Loose ends (2026-10-20)

Nothing here blocks the knob staying off; each item names what would close it.

1. **The ship decision (jdh8).** Every DONT arm is a plain win in every cell
   (+0.0011 to +0.0018 per accepted board) and a PD loss in ns and both. The
   rule-3 split puts the PD loss in PD's bias; with `tied` the unbiased boards
   are plain ≈ 0. Shipping default-on would be a recorded trade-off, not a
   decision-table ship. The candidate shape is `tied` or `longer`, not `Off`.
2. **PD cannot price this lane.** `ns_score_bid` scores a making contract
   undoubled and a failing one doubled, whatever the table bid
   (`src/scoring.rs` module doc). So the off arm's sit in a making `1NTxx` is
   scored as undoubled `1NT`, and our runout is scored doubled even when they
   never doubled it. Escaping a business redouble is exactly what PD erases. A
   PD variant that keeps a business `XX` of a making contract would be a
   scorer change across every A/B, so it is a separate design question.
   Until then this lane's PD is descriptive, as rule 3 says.
3. **`Off` vs major-first is inconclusive.** `tied − dont` is positive on both
   scorers in every cell, but on only 12–15 boards per cell. The pooled plain
   gain is +1.07 ±1.24 ×10⁻⁴ per board, with a conservative CI because the
   cells share deals. About 1.5× the boards would put its lower bound above 0
   if the effect holds. A 2× re-run of `scripts/ab-xx-dont-major-first.sh`
   (BOARDS=786432, same SEED_BASE, fresh dir) takes about 2 h at JOBS=64.
   `tied − longer` moves 2 boards per cell and will not separate.
4. **The ns cell's unbiased loss is untraced.** `tied − off` unbiased is plain
   −89 IMPs over 66 boards in ns against ≈ 0 in the other cells, and `dont`
   is −133 there too. The worst boards (`div.tied.vs.off.ns.jsonl`, the
   `unbiased` bucket) have not been traced. Rule 4's candidates are their bid
   over our runout (floored; item 5) and a 4-3-3-3 landing in 4-2.
5. **Their bid over our runout is floored.** The package authors their pass
   and their double at every node, but not a *bid*, e.g. `(1NT) X (XX) 2♣
   (2♥)` or `… - - 2♣ (2♠)`. Those seats go to the net, and the iron rule
   asks for interfered tails before a convention counts as complete. A
   census of how often they bid there, which is cheap from the run's dumps,
   decides whether it is worth authoring.
6. **The doubler's pass reads as anything** (owed above): the reused
   `passed_dont_*` catch-all. That is its own change with its own A/B, since
   it moves DONT's lane too.
7. **Looser two-suiter readings under major-first** (accepted by jdh8,
   2026-10-20). `2♣`/`2♦` keep the `Off` reading: their `!major_first`
   exclusion overruns `EXCLUSION_BOX_BUDGET` (16) and is dropped. That costs
   precision, never soundness. `2♠` and the forcing pass do read the moved
   hands.
8. **No BEN guard, and none is feasible.** `ben-gen` has the flags
   (`--ns-xx-dont-runout --ns-xx-dont-major-first …`). The lane moves about
   160 boards per 393,216 against BBA, so BEN's ≈ 8k boards per cell would
   see about 3. Unless BEN redoubles far more often than EPBot, this stays
   BBA-only. A one-cell pre-count settles it, as for the rail.
9. **The rails' doubler arm alone is dead.** It moves 13–16 boards per cell
   at plain ≈ 0. The doubler node is reached mostly when the advancer sits,
   so the rails' doubler gain in `xx-runout-rails-2` was rescuing the
   advancer rail's own sits. Re-run at ≥ 4× only on request.
10. **The balancing seat is out of scope by design.** `(1NT) - - X (XX)` and
    a passed doubler are excluded, because that advancer can hold values.
    Not owed unless the balancing seat becomes book work (§ Out of scope).

Sources (✓ = read at the source during the 2026-10-20 research; ~ = secondary
or snippet only):

- Karen Walker, "Contest their notrump?" Part 13, ACBL Bridge Bulletin, Jan
  2020 — <https://kwbridge.com/bb/nt-5.htm> ✓ (business `XX` = a pass;
  values pass vs weak pull; the scramble).
- Bridgebum, DONT, "Partner Doubles, and RHO Redoubles" —
  <https://www.bridgebum.com/dont.php> ✓ (relay off; pass = "let partner run";
  SOS `XX` over a doubled two-suiter).
- Adventures in Bridge #547 —
  <https://www.advinbridge.com/this-week-in-bridge/547> ✓ (DONT runouts,
  two-suiters "at least 4-4").
- "Rescue Systems Over 1NT Doubled", bridge-tips.co.il, 2015 ✓ (DONT rescue:
  cheapest of two or three 4+ suits; its "which higher suit?" weakness).
- Larry Cohen, DONT — <https://www.larryco.com/bridge-articles/dont> ✓
  (`(1NT) 2♣ (X)`: pass = willing, `XX` = bid your other suit); "Redoubles" —
  <https://www.larryco.com/bridge-articles/redoubles> ✓ (responder's
  redouble of a doubled opening is business; later doubles penalty).
- Karen Walker, DONT summary — <http://kwbridge.com/dont.htm> ✓.
- Bridgebum, Touching Escapes — <https://www.bridgebum.com/touching_escapes.php>
  ✓ (the only source ruling on 4-3-3-3: treat the four-card suit as a five);
  Helvic, SWINE, Owen Wriggle pages on bridgebum ✓ (forcing-pass schemes).
- David Stevenson, "Responses to 1NT when it has been Doubled" —
  <https://www.blakjak.org/dbl_1nt1.htm> ✓ (every listed system's `XX` forces
  `2♣`).
- BBO forum, "Escape structure after 1NT-X" ~ (awm: forcing weak pass;
  Hinden's "1NTX must remain available" objection, void here);
  Celestial "Chapter 9" ~ (pass for a one-suiter, then `XX` to learn it).

## Ledger (memory compaction, 2026-08-16)

- **Strong-double gauge, 2026-07-29:** `StrongDoubleEdge::Points(18)` stays
  opt-in; `Hcp(18)` remains shipped. On 1M boards/vul (24.pdd rows
  44.1M–46.1M; 1.11%/0.96% divergent), Points lost plain −0.0034 ± 0.0014 /
  −0.0026 ± 0.0016 and PD −0.0057 ± 0.0017 / −0.0043 ± 0.0019 NV/vul (all
  CIs below zero). The
  mechanism was the changed direct-Pass projection (`points(..n)` vs
  `hcp(..n)`), not evidence that pulling 16–17 HCP hands into the strong X was
  itself wrong: balancing `1♠ → P` lost −4.68/fired ×19, sandwich `P → 1♠`
  −2.92 ×25, and even constructive `4♠ → 6♠` −9.00 ×13. Price the gate
  separately from the Pass reading before trying another gauge.
- **Stopperless residual X, 2026-07-29:** the proposed 16+ direct-seat double
  with no stopper, no five-card suit and no takeout shape was rejected on
  theory and reverted. Its residue is a 4333 with four in opener's suit or a
  4432 with the doubleton in an unbid suit — both are trap passes with strength
  over opener; doubling only gives opener an escape suit. Every no-five-card
  residue is balanced, so a stopper already routes it to the 1NT overcall; a
  16–17 cap is behaviorally inert because 18+ already reaches the strong-X
  tier. The observed fire rate was ~0.05% (0/6400), bounding aggregate impact
  near ±0.0005 IMPs/bd, so absence from the table was not evidence of a
  coverage hole.
- **Weights and distillation:** swapping the disjoint strong-X and natural
  two-level-overcall weights is behaviorally inert, but not corpus-inert:
  `examples/dump-teacher` softmaxes the logits, so such a weight edit changes
  teacher targets even when argmax never changes.
