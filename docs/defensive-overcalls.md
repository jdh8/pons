# Defensive round-1: the 1NT and suit overcalls — design

**Status: design agreed 2026-08-11, nothing shipped.** First work package of the
defensive round-1 redesign; its sibling is
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
| `P` → `2♥/2♠` | 483 | −764 | −137 | jumps outside the shipped 6-card 8–11 box |

Excluded here: `P` → `X` (n=1,359, plain −1,000 / **PD +1,182** — mixed sign;
see the takeout-double doc, which concludes the X's problem is boundaries, not
volume).

## Current state

Everything direct-seat lives in one table, `defense_to_suit` at `P* (1x)`
([overcall.rs:87-324](../src/bidding/american/defense/overcall.rs)). The
weight ladder (centinats):

| w | call | band |
| ---: | --- | --- |
| 200 | Michaels cue | 5-5, `points(8..) & hcp(8..)` |
| 190 | Unusual 2NT | same band |
| 150 | 1NT overcall | `hcp(15..=18) & balanced() & stopper_in_their_suits()` |
| 150 | weak jump 2M | `len(suit,6..=6) & points(8..) & !hcp(12..)`, majors only |
| 140 | 1-level overcall | `len(suit,5..)`, floor 8 points (discipline on) |
| 130 | takeout X | `hcp(12..)` + Strict shape (≤3 in theirs, 3+ in every unbid) |
| 120 | strong X | `hcp(18..)` any shape |
| 100 | 2-level overcall | `len(suit,5..)`, floor 11 (9 passed hand); flat, no quality atom |
| 0 | Pass | `hcp(..18)` complement gate |

The balancing seat `P* (1x) - -` is **unauthored** — floor territory, and it
stays that way (see Out of scope).

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

## Design items

Ordered; each is one experiment, arms sequential, fresh `SEED_BASE`.

### O1 — 1NT denies a 5-card major (`nt_overcall_no_major` re-measure)

The knob **already exists**, default off
([overcall.rs:92-99](../src/bidding/american/defense/overcall.rs)):
`len(H,..=4) & len(S,..=4)` on the 1NT rule — exactly BBA's box (5-card
minors stay allowed). The `1NT → suit` slice (−363/−815) is the direct
evidence; the house rule that rejected knobs are single-dummy re-measure
candidates applies. Fastest possible experiment: a one-flag A/B, no code.
The hands it moves fall through to the 1-level/2-level overcall rows, which
must be checked to catch them (a 15–18 5-card-major hand passes every
overcall gate; verify no gap at the 2-level for 5-card-minor+major shapes).

### O2 — the `X → 1NT` forensic (diagnosis gates any shape change)

The 158 hands our X wins where BEN overcalls 1NT all pass Strict shape (≤3
in theirs, 3+ in every unbid). Which 1NT atom fails them is **unknown**:
stopperless? semi-balanced (5422/6m322) failing `balanced()`? band edge
(BBA plays 16–18, we 15–18)? **Dump the 158 hands and classify by failing
atom before any constraint edit.** Only then design the widening (candidate:
semi-balanced shapes with stoppers, mirroring BBA's 2–5-in-every-suit box).
Trigger is rare (158/64,756 divergent) → enriched probing when it measures.

### O3 — minor weak jump extension

Extend the shipped box verbatim to diamonds: `2♦` over `(1♣)` =
`len(♦,6..=6) & points(8..) & !hcp(12..)`, same
`Alert("weak-jump-overcall")`, same disjointness (5-card and 12+ HCP hands
keep the simple `1♦` overcall). The `1♦ → 2♦` slice is clean on both scorers
(−295/−232) and this mirrors a treatment that just shipped with its
continuations already authored (the natural preempt structure). Row sketch —
it is one more assignment in the existing template, not a new package:

```text
P* (1♣)   →  2♦ : len(♦,6..=6) & points(8..) & !hcp(12..)   [w150]
```

Note `3♣` over `(1♦)` etc. are 3-level jumps and belong to O6, not here.

### O4 — the 2-level quality gate (the wall)

The single biggest PD number in the residue (`2♣ → P`, −2,222 PD), and the
sd-wall diagnosis already ruled the bucket REAL under single-dummy. The flat
`points(11..)` floor is the wrong shape of gate: BBA charges **suit quality**
so a good suit bids light and a poor suit stays home.

- **New Constraint atom** (the one DSL growth this campaign takes):
  `suit_quality(suit, q)` — honors-weighted suit strength, scale swept at
  implementation (start from `suit_hcp`, which `overcall_four_card` already
  uses). Projects `NONE` like the X's shape atoms — the reading stays a sound
  points band; no opaque `pred` hatch.
- **Primary arm**: replace the flat 2-level floor with a BBA-shaped gate
  (quality + points combined, vulnerability-scaled via the existing
  `points_by_vul` idiom).
- **Comparison arm, same seeds**: the existing
  `two_level_minor_overcall_tight` (11→15), which SD-PD already reversed
  into a promotion candidate.

Attribution wrinkle, recorded so nobody re-litigates it: vs BEN this slice
is plain-**positive** (+369) / PD −3,645, so it failed the *attribution
eligibility* gate — but the *ship* gate is plain-wash + PD-win by the
decision table, which a tightening can still pass. Score both, plus SD-PD.

### O5 — 1-level quality floor

Same atom, same formula at level 1 (`1♥ → P`, −162/−487): the 8-point floor
stands, but a quality-free minimum bid gets charged. Small expected effect;
runs after O4 proves the atom.

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
  mid-flight, control regenerated at HEAD.
- Rare triggers (O2, O3 fires only over `(1♣)`) → enriched probing, accept
  on raw hands before the bidder.
- New knobs sit outside the v5 floor's 13 trained slots, so they fold to
  zero and the A/Bs price the book alone
  ([ai-bidder/card-manifold.md](ai-bidder/card-manifold.md)). Defensive book
  is shared with `dutch()` — a default flip moves both; re-bless cards with
  `cargo run --example bba-card`.
- Disclosure bites this bucket specifically (def-r1 was the only head bucket
  to worsen at the `3c94802` disclosure flip): every widened box is a box we
  hand the opponents. Boxes stay exact — complements authored (the weak
  jump's `!hcp(12..)` pattern) so projections carry both edges.

## Out of scope (decided, not deferred by neglect)

- **Balancing/reopening seat** — stays with the floor lever (B2 item 0, the
  −37k floor#3 buckets). A book package at `P* (1x) - -` would shadow the
  floor there permanently; measured floors have beaten authored nodes
  repeatedly in this repo.
- **Relational constraints** (BBA's reopening band clamped to opener's shown
  minimum): a new dependency class for `Constraint` serving only the
  balancing seat — dead until balancing is book work, which it is not.
- **`P → X` volume** — see [takeout-double-layers.md](takeout-double-layers.md).
