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
(`docs/bba-gap-campaign.md`, `ab-results/two-level-minor-overcall-refresh/`).
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

## Out of scope (decided, not deferred by neglect)

- **Balancing/reopening seat** — stays with the floor lever (B2 item 0, the
  −37k floor#3 buckets). A book package at `P* (1x) - -` would shadow the
  floor there permanently; measured floors have beaten authored nodes
  repeatedly in this repo.
- **Relational constraints** (BBA's reopening band clamped to opener's shown
  minimum): a new dependency class for `Constraint` serving only the
  balancing seat — dead until balancing is book work, which it is not.
- **`P → X` volume** — see [takeout-double-layers.md](takeout-double-layers.md).
