# The multi-layered takeout double — design

**Status: design agreed 2026-08-11, nothing shipped.** Second work package of
the defensive round-1 redesign; runs after
[defensive-overcalls.md](defensive-overcalls.md) (which also holds the shared
evidence table and measurement discipline). Same bucket, same campaign docs:
[ben-gap-campaign.md](ben-gap-campaign.md),
[bba-gap-campaign.md](bba-gap-campaign.md).

## The organizing principle

**The takeout double is a tool to find the 4-4 unbid-major fit.** Everything
else it does — showing strength, keeping the auction alive — other calls can
do; the 4-4 major fit is the contract only the double finds, because a 4-card
suit is never overcalled naturally and never shown by 1NT. Two consequences:

1. **Precedence rungs by unbid-major length**, not just by strength. A hand
   with 4 cards in each unbid major is a *premium* double; a hand where some
   unbid major is only 3 cards is a *reluctant* double that should lose the
   weight race to any decent alternative call.
2. A hand with a **5-card unbid major bids it** — it fails the double's
   purpose, because its fit is findable by overcalling. The rung structure
   must never let the double swallow that hand.

Both references agree: BBA's base X over a minor *requires* 4+-4 majors
(MB.TXT:673, shape `[45].4.[34].[0-2]`), and the BEN residue's `2♣/2♦ → X`
slices (−180/−219 and −252/−337) are BEN doubling with both majors where we
overcall a 5-card minor.

## Evidence

From the same residue as the sibling doc (BEN decompose minus weak-jump
slice). The X's slices are **boundary** losses, not volume losses:

| slice (ours → ref) | n | plain | PD | reading |
| --- | ---: | ---: | ---: | --- |
| `P` → `X` | 1,359 | −1,000 | **+1,182** | mixed sign — widening the X is NOT supported |
| `X` → `1NT` | 158 | −254 | −369 | balanced stopper hands mis-doubled (forensic O2, sibling doc) |
| `2♣` → `X` | 140 | −180 | −219 | 5m + both majors: we overcall, BEN doubles |
| `2♦` → `X` | 69 | −252 | −337 | same |
| `X` → `3♣` | 63 | −140 | −70 | one-suiter hands mis-doubled; BEN jumps |
| `X` → `2♦` | 69 | −119 | −157 | same, simple overcall |
| `X` → `2NT` | 88 | −129 | +28 | mixed |

The BBA microscope agrees: takeout doubles are a named recoverable residue
(−16k PD-worse), and the shipped `takeout_support: Strict` +
`overcall_discipline` package came from this bucket.

## Current state vs BBA

Ours ([overcall.rs:106-149](../src/bidding/american/defense/overcall.rs)):
two tiers inside the direct-seat table — 12+ shapely (w130: `hcp(12..)`,
≤3 in each of their suits, 3+ in every unbid suit, `takeout_double_shape_ok`
trims weak 4333/5332) and 18+ any shape (w120), with weight-0 Pass
complements so the pass reading has a band. Doubler's rebids: **unauthored**,
floor. Balancing seat: unauthored, floor.

BBA's ladder (MB.TXT; 2009 export caveat as in the sibling doc): base band
level-scaled `12+2·(level−1)` requiring 4-4(+) majors over a minor (:673); a
9+-points shape-perfect tier, 3+ in *every* unbid suit (:1043-1046); off-shape
strong tiers 14+/16+/19+ (:3412, :1041, :1047); the second double = 18+
(:3728); reopening relational (band clamped to opener's shown min +3/4,
:701); balancing X 13+ (:1066-1069). The layering is disclosed by the
doubler's *rebids*, not by the double itself.

## Design

### The rung table

New ladder for the direct-seat X (weights relative to the existing table in
the sibling doc; no ties — `weight_tie_report` is asserted):

| w | rung | gate |
| ---: | --- | --- |
| 135 | **X₄** — premium | `hcp(12..)` + Strict shorts + **4 cards in each unbid major** |
| ~90 | **X₃** — reluctant | `hcp(12..)` + Strict shape as today (3-card major tolerated) |
| 120 | strong X | `hcp(18..)` any shape, unchanged |

Placement consequences, hand class by hand class:

- 4-4 unbid majors + 5-card minor: X₄ wins. It beats the 2-level overcall
  (100) outright; for the 1-level case the simple overcall's weight is
  **split by strain** — a 5-card unbid *major* keeps 140 and still outranks
  X₄ (preserving the principle), while the 1-level *minor* overcall drops
  below 135 so the premium double speaks first.
- Some unbid major only 3 cards: X₃ (~90) loses to every live overcall,
  1NT, and weak jump; it fires only when nothing else speaks. Today those
  hands double at 130.
- The weight split of the 1-level overcall (major 140 / minor <135) is part
  of this package, not the sibling's — it exists only to seat X₄.

**X₃ demotion is precedence-only in the default arm.** A second arm raises
X₃'s floor (13–14) behind a knob: the `P → X` slice being PD-positive warns
that thinning the X may *gain* PD, so the floor-raise arm is cheap insurance
that it, not the reshuffle, is the real winner. No 9–11 minimum tier —
that same slice is the evidence against importing BBA's light tier.

### The strong side: layers live in the rebids

The 17+ one-suiter (too strong for an overcall capped at `hcp(..18)`, too
shapely for 1NT) doubles first and shows the suit next — the deferred
"strong overcall-then-jump" sibling from the BBA campaign. The layer is not a
new double rule; it is **authored rebid rows that make the existing 18+ tier
disclosable** (and split it from the shapely 17+ one-suiter path where the
A/B says so).

### Doubler-rebid rows (complete the convention first)

Four families, minimal boxes (single band + finite catch-all each) — they
exist to complete the convention and carry readings, not to be clever; depth
stays with the floor. Sketches in the row grammar (advancer's call `1y`/`2y`
binds, cue = their suit `x`):

```text
P* (1x) X (-) 1y (-)   →  1NT : hcp(18..=19) & balanced() & stopper   [cheapest NT = 18–19]
                       →  2z  : len(z,5..) & hcp(17..) , z ≠ x,y      [new suit = strong one-suiter]
                       →  2x  : hcp(18..) & support(y,..)             [cue = GF, no clear direction]
                       →  2y  : support(y,4..) & hcp(15..=17)         [raise = extras, 4-card fit]
                       →  P   : catch-all                              [minimum, nothing to add]
```

(One template per advance level; `expand` cross-products the openings and
advances, domains pruned to ascending auctions — sketch, not final grammar.)

These rows are what make the tier structure *read*: the X keeps its single
`TAKEOUT_DOUBLE` alert, the shape atoms still project `NONE`, and the bands
ride the authored complements (the weak-jump `!hcp(12..)` pattern) so the
pass/fallback projections carry both edges of every tier.

### Boundaries owned elsewhere

- `X` vs `1NT` (the 158-board forensic): sibling doc, item O2 — diagnosis
  before any edit.
- `X` vs jump overcall (`X → 3♣`): falls out of O6's opt-in preempt family —
  a one-suiter that can preempt stops passing through the double.

## Out of scope

- **Balancing and reopening doubles** — floor lever B2 item 0, as in the
  sibling doc. The one obligation here: direct-tier bands stay exact and
  disjoint so the floor behind `P* (1x) - -` is not contradicted by what our
  card discloses about the direct seat.
- **Responsive and second doubles** (BBA: relational 16+−partner-shown;
  second X = 18+) — round-2 material, next design after this ships.
- **Relational constraints** — not needed by any rung above.

## Measurement

Everything from the sibling doc's plan applies (two-reference gate,
sequential arms, fresh `SEED_BASE`, knobs outside the trained slots, card
re-bless). Specific to this package:

- The X fires often — standard 204.8k BBA + same-seed 25.6k Tier-F, no
  enriched probing needed.
- **Rebids ship with the rungs, not after** — measuring the rung table with
  floor rebids is measuring an incomplete convention (iron rule).
- Arms: (1) rung table, precedence-only; (2) rung table + X₃ floor raise.
  Both against control-at-HEAD; decision per cell by the table, PD primary
  for the bid decision, plain for the contract.
