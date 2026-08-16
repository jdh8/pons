# The multi-layered takeout double — design

**Status: rebuilt 2026-08-13 on what the evidence actually says; both knobs
built and measured 2026-08-13 — both wash on both references, so both stay
opt-in, default off** ([verdicts](#verdicts)). Second work package of the defensive
round-1 redesign; runs after [defensive-overcalls.md](defensive-overcalls.md)
(which also holds the shared evidence table and measurement discipline). Same
bucket, same campaign docs: [ben-gap-campaign.md](ben-gap-campaign.md),
[bba-gap-campaign.md](bba-gap-campaign.md).

## The organizing principle

**The takeout double is what we bid when nothing natural is biddable.** It is
not primarily a 4-4-major-fit finder — see the refutation below — and it is not
primarily a strength bid either. It is the residual call, and the two ways to
improve it are therefore:

1. **Shrink it from below.** Every hand with a suit long enough to overcall
   naturally should overcall. Five for a major is shipped
   (`suppress_5card_major_takeout`); six for a minor is the new
   `suppress_long_minor_takeout`.
2. **Disclose it from above.** Hands too strong for a natural overcall have to
   double whatever their shape — *that is a strength problem, not a fitting
   one* — so the double's top end is unavoidably wide. What can be fixed is
   that the strong tier is currently indistinguishable from a bare 12-count
   until the doubler bids again, and the doubler's rebid was unauthored.

### What was refuted (2026-08-13)

The earlier version of this doc was organized around "the double is a tool to
find the 4-4 unbid-major fit", with a rung table (X₄ premium at w135, X₃
reluctant at ~90) and a strain-split of the 1-level overcall's weight to seat
X₄. Pulling the actual boards behind the evidence table below refutes the
principle and finds two errors in the table:

| the claim | what the boards say |
| --- | --- |
| `2♣`→`X` (n=140) is "5m + both majors" | **13.6%** are 4-4 in both unbid majors, netting −12/−21. 129 boards are ordinary 10–11 HCP two-level overcalls carrying −164 of the −180 plain — that is O4's wall, already refuted twice |
| `2♦`→`X` (n=69) is "same" | **2.9%** are 4-4 majors. Same wall |
| `X`→`3♣` (n=63) is in scope | **100% weak-two openings** (`P* (2x)`, `weak_two_defense.rs`) — zero rows in the node this design touches |
| X₄ should promote 4-4-major hands over `1NT` | On the 24 boards where we overcall `1NT` holding 4-4 in the unbid majors and BEN doubles, **we win +46 plain / +62 PD**. In the reverse slice our 4-4-major doubles lose −20/−6. Both directions say `1NT` beats the double on that shape |

Neither slice *directly* measures an X₄ rung — BEN's double is followed by
BEN's continuations, not ours — so the data refutes the premise rather than
pricing the rung. That is enough: the rung existed only to serve the premise.

BBA's base X over a minor does require 4+-4 majors (`MB.TXT:673`, shape
`[45].4.[34].[0-2]`), but see the provenance caveat below — that is a reading
of a 2009 export, not a live census.

## Evidence

From the same residue as the sibling doc (BEN decompose minus the weak-jump
slice), `ab-results/ben-decompose/2026-08-10-42454d2/boards.jsonl`. The X's
slices are **boundary** losses, not volume losses:

| slice (ours → ref) | n | plain | PD | reading |
| --- | ---: | ---: | ---: | --- |
| `P` → `X` | 1,359 | −1,000 | **+1,182** | mixed sign — widening the X is NOT supported |
| `X` → `1NT` | 158 | −254 | −369 | balanced stopper hands mis-doubled (forensic O2, sibling doc) |
| `2♣` → `X` | 140 | −180 | −219 | **not** the 4-4-major slice: 129 boards are the 10–11 HCP two-level-overcall wall |
| `2♦` → `X` | 69 | −252 | −337 | same wall |
| `X` → `2♦` | 69 | −119 | −157 | **100% in-node.** Zero hands hold a 5+ unbid major (the shipped bar already removed that class); **45 of 69** hold a 6+ card unbid minor |

`X` → `3♣` (n=63, −140/−70) is dropped from this table: every row is a weak-two
opening, so it belongs to `weak_two_defense.rs`, not here.

**Stakes, stated plainly.** Every board in `Defensive/book/round-1` where we
doubled over a one-of-a-suit opening totals **363 boards, −384 plain / −736
PD**, against a node total of −5,831 / −6,553 over 7,643 boards. This is a
small package. It is worth running because Package A is one clause aimed at 45
identified losing boards, and Package C is the only route to disclosing the
strong tier at all — disclosure has been priced across the book at roughly
−1 IMP/board ([reading program](ai-bidder/sampled-projection.md)).

## Current state vs BBA

Ours ([overcall.rs:129-172](../src/bidding/american/defense/overcall.rs)): two
tiers inside the direct-seat table — 12+ shapely (w130: `hcp(12..)`, ≤3 in each
of their suits, 3+ in every unbid suit, `takeout_double_shape_ok` trimming weak
4333/5332 and the biddable suits) and 18+ any shape (w120), with weight-0 Pass
complements so the pass reading has a band. Note the shape gate is ANDed into
the **12+ tier only**: the 5-card-major principle is scoped to the overcallable
band, and a hand above `strong_double_hcp` reaches the strong tier whatever its
shape. That is what "a strength problem, not a fitting problem" means in code,
and it was already true before this package.

BBA's ladder (`MB.TXT`; see the provenance caveat): base band level-scaled
`12+2·(level−1)` requiring 4-4(+) majors over a minor (:673); a 9+-points
shape-perfect tier, 3+ in *every* unbid suit (:1043-1046); off-shape strong
tiers 14+/16+/19+ (:3412, :1041, :1047); the second double = 18+ (:3728);
reopening relational (band clamped to opener's shown min +3/4, :701); balancing
X 13+ (:1066-1069). The layering is disclosed by the doubler's *rebids*, not by
the double itself — which is the one structural idea from the old design that
survives, and is Package C.

### Provenance caveat (applies to both defensive docs)

**No live BBA census over a one-of-a-suit opening exists in this repo.** The
`def1-c`/`def1-d`/`def1-h`/`def1-s` probe modes of
`examples/probe-bba-constraints` have never been run and recorded, so every
claim above about BBA's direct-seat double is a reading of the 2009 `MB.TXT`
export, which is orphaned from the live engine. `bba-multi-2d.md`'s 41%
takeout-double row is a different node and a trap for the next grep: it is the
X by the **1NT opener's partner** over a Multi `2♦` overcall (`--mode counter`,
actor seat 2), not a direct-seat double of a suit opening. (This warning
previously called it the *advancer's* X, which is a third node again — the
probe's seat index settles it; the full counter-defense set is in
[ai-bidder/bba-1nt-counter-defense.md](ai-bidder/bba-1nt-counter-defense.md).)

## Design

### Package A — the biddable-suit bar (`suppress_long_minor_takeout`)

One clause in `takeout_double_shape_ok`
([constraint.rs](../src/bidding/constraint.rs)): reject an **unbid minor of six
or more cards**, unconditional on HCP, exactly as its five-card-major sibling
does. `TakeoutSupport::Strict` demands three cards in every unbid suit, so a
6-3-3-1 sails through the shape gate today and buries a suit that is perfectly
biddable.

- Fires over all four openings. The residue is 1♥/1♠ only, so the
  minor-opening subset is **unmeasured and pulls the other way** (over a minor
  opening our long minor is more likely to be their suit, and the double is
  more likely to be the takeout of a short suit). Split it out in forensics.
- No HCP clause: the gate is ANDed into the 12+ tier only, so the strength
  escape is already the separate 17+/18+ rule.
- **Scope leak, deliberate:** `takeout_double_shape_ok` is shared with
  `weak_two_defense.rs:60/68/76`, so the clause changes the weak-two node too.
  The four existing suppressions behave the same way, so this is consistent
  rather than novel — but the divergence report must split 1-level from
  weak-two openings so the attribution is visible. If the weak-two half
  contaminates, gate the clause on the opening's level.

### Package B+C — the seam split and the rebids that disclose it

Bundled behind one knob (`defensive_seam_split`) because B alone routes 17-HCP
one-suiters into a node with no authored continuation, which is the
incomplete-convention failure the iron rule exists to prevent.

**B — a level-dependent overcall ceiling.** "Too strong for a natural overcall"
is cheaper at the one level than at the two: a 17-count with a five-card suit
happily overcalls `1♠`, but overcalling `2♣` on the same values commits the
partnership a level higher with no extra information. So the knob drops the
strong double's floor to 17 (from `strong_double_hcp`'s 18) and the **2-level**
overcall's ceiling with it, while the **1-level** ceiling stays at 18. At the
seam both the 1-level overcall (w140) and the strong X (w120) are live, and
argmax keeps the overcall — so nothing changes at the one level, and only the
awkward two-level hand moves into the double. Composes with the HCP-gauged
partition only; the legacy `points` path ignores it.

**C — doubler-rebid rows** (`defense/doubler_rebid.rs`, package
`doubler-rebid`). Twelve nodes: four openings × three unbid suits, at
advancer's **minimum** advance only (one level up when advancer's suit ranks
above theirs, two when it does not). A jump advance is invitational and a
notrump advance is [`advance_2nt`]'s; both keep the floor.

```text
P* (1x) X - 1y -
  1NT  w150  hcp(19..) & !hcp(22..) & balanced() & stopper_in_their_suits()
  2z   w140  len(z,5..) & hcp(15..)            for each z ∉ {x,y}
  2y   w130  support(4..) & hcp(15..)
  2x   w120  hcp(17..)                          [artificial — DOUBLER_CUE]
  P    w0    !hcp(15..)
```

`P* (1x) X - 2y -` carries the same five rungs with every floor raised one step
of two HCP (Pass `!hcp(17..)`, raise/new suit `hcp(17..)`, cue `hcp(19..)`, NT
`hcp(21..) & !hcp(24..)`) — a two-level advance burns a level of room before
the doubler speaks. The builder derives every level from the advance, so one
function serves both templates.

Four design points the old sketch got wrong:

- **1NT is 19–21, not 18–19.** A balanced 18 with a stopper bids the direct
  `1NT` overcall at w150, which outranks both doubles, so it never reaches this
  node. Ceilings are spelled as complements (`!hcp(22..)`) because forward
  projection carries point floors but drops a plain range top — the weak-jump
  idiom.
- **The new suit is `hcp(15..)`, not `hcp(17..)`.** A 14–16 HCP 5-3-3-2 with a
  five-card minor still doubles legitimately (the bar removes 5+ majors and 6+
  minors only) and would otherwise be stranded with nothing to bid.
- **Rungs overlap in strength and separate by shape**, ordered by weight.
  Exact-and-disjoint is a constraint on the *direct* tiers, whose complements
  the pass reading rides — not on rebids.
- **No `hcp(0..)` catch-all.** These are exact `Pattern::node` rows, and an
  exact node that rejects a hand falls through to the floor
  (`trie.rs:494-539`); only *guarded* tables must stay total. Hands above the
  Pass band fitting no rung stay with the floor deliberately — including 22+
  balanced. Pass is **banded**, not a catch-all: only `hcp(0..)` is a true
  catch-all, and the reading has to say *how* minimum.

### Boundaries owned elsewhere

- `X` vs `1NT` (the 158-board forensic): sibling doc, item O1/O2 — and the
  sharper trigger found here is recorded there.
- `X` vs jump overcall (`X → 3♣`): weak-two node, not this one.

## Out of scope

- **Balancing and reopening doubles** — floor lever B2 item 0, as in the
  sibling doc. Note the balancing seat is **0 rows in this bucket by
  construction**: `family()` tags a double-pass prefix `balancing` before the
  round split, so nothing balancing can appear in `Defensive/book/round-1`.
  The one obligation here: direct-tier bands stay exact and disjoint so the
  floor behind `P* (1x) - -` is not contradicted by what our card discloses
  about the direct seat.
- **Responsive and second doubles** (BBA: relational 16+−partner-shown; second
  X = 18+) — round-2 material, next design after this ships.
- **Relational constraints** — not needed by any rung above.

## Measurement

Everything from the sibling doc's plan applies (two-reference gate, sequential
arms, fresh `SEED_BASE`, knobs outside the trained slots, card re-bless). Both
knobs default off and the off state is proved byte-identical to HEAD
(`render-book` `6fa476f9…`, `smoke-default --count 20000 --seed 1`
`b9cd64a7…`), so each arm prices exactly its own knob.

Two experiments, **sequential**, fresh `SEED_BASE=$(date +%s)` each:

1. `scripts/ab-defensive-overcalls.sh bar` — 204.8k boards/arm/vul BBA, scored
   plain DD + PD + SD; then `scripts/ab-ben-defensive-overcalls.sh bar` at the
   same seed, 25.6k/arm/vul Tier-F. Ship = BBA pass + BEN non-refute; watch the
   BBA−BEN spread.
2. `… seam` — same protocol, control regenerated at HEAD.

Read the verdict from the decision table in [measurement.md](measurement.md); a
PD-only win is a doubling artifact, a plain-DD wash plus a PD win ships
default-on. **Split every divergence report by opening class** (1-level vs weak
two, minor vs major opening) — the bar's minor-opening and weak-two subsets are
both unmeasured, and "trigger-too-broad" is what killed the last defensive
tightening in this bucket.

## Verdicts

Both ran 2026-08-13 at `aadd547`, sequential, fresh seeds (`bar` 1786562541,
`seam` 1786563002), BBA 204.8k bd/arm/vul then the same-seed 25.6k Tier-F BEN
guard. Results in `ab-results/takeout-double-layers/2026-08-13/`.

Both knobs are **doubling** knobs — `bar` doubles less, `seam` doubles more — so
each is judged on plain DD and the PD column is the bracket that cannot price
them ([`feedback_pd-cannot-price-double-more`](measurement.md)). The signs are
exactly the mirror image that reading predicts.

| knob | ref | vul | fired | plain DD | PD | SD plain |
| --- | --- | --- | --- | --- | --- | --- |
| `suppress_long_minor_takeout` | BBA | none | 0.10% | −0.0002 ±0.0008 | +0.0004 ±0.0009 | −0.0000 ±0.0008 |
| | BBA | both | 0.09% | −0.0004 ±0.0009 | +0.0003 ±0.0011 | −0.0003 ±0.0010 |
| | BEN | none | 0.07% | −0.0001 ±0.0018 | −0.0007 ±0.0024 | −0.0003 ±0.0019 |
| | BEN | both | 0.07% | +0.0005 ±0.0023 | +0.0003 ±0.0026 | +0.0007 ±0.0026 |
| `defensive_seam_split` | BBA | none | 0.29% | +0.0011 ±0.0014 | −0.0001 ±0.0016 | +0.0009 ±0.0014 |
| | BBA | both | 0.29% | +0.0011 ±0.0018 | −0.0005 ±0.0021 | +0.0014 ±0.0018 |
| | BEN | none | 0.28% | +0.0003 ±0.0036 | −0.0025 ±0.0043 | +0.0012 ±0.0036 |
| | BEN | both | 0.28% | +0.0003 ±0.0047 | −0.0005 ±0.0059 | +0.0016 ±0.0048 |

**Every cell is a wash.** No refutation on either reference, no gain on either;
both stay opt-in with the default system byte-identical, the house outcome for
rejected-but-interesting treatments.

`bar`'s failure is the instructive one, and it is a **stakes error made at design
time, not a measured loss**. Its case was 45 of the 69 boards in the `X` → `2♦`
residue; the knob fires on 0.07–0.10% of boards, which on the BEN guard is **18
boards in 25,600**. The residue was counted over a corpus scoped differently from
this node's trigger, so the 69-board figure never was 69 boards *of trigger*. At
this density the clause cannot move the anchor whichever way it leans, and the
split-by-opening-class report the plan owed it is moot: there is not enough mass
in either half to attribute. Read the count before the sign — a trigger this thin
is a reason not to build, and it was visible in the design.

`seam` is the one that leans the right way and leans there consistently: plain DD
positive in all four plain cells across both references. It is still inside its
CI, so it does not ship — but it is the better of the two re-measure candidates,
and the cheapest way to sharpen it is more boards on the BBA side rather than a
redesign. One caveat against over-reading that consistency: `none` and `both`
share a `SEED_BASE`, so they are the same deals priced at two vulnerabilities,
and the SD cells re-score those same boards again. That is one ~200k-deal sample
viewed four ways, not four independent confirmations.

The weak-two scope leak the design flagged is **live and visible**: several of
`bar`'s worst BBA divergences are over a weak two (`2♥ 3♣` vs `2♥ X`, `2♦ 3♣` vs
`2♦ X`), because `takeout_double_shape_ok` is shared with
[`weak_two_defense`](../src/bidding/american/defense/weak_two_defense.rs). Any
future reopening of this clause must gate it on the opening's level first.

## Ledger (memory compaction, 2026-08-16)

- **Advancer penalty-pass strength cap, 2026-07-29:** narrowing the rich
  book's conversion with `--ns-advance-penalty-pass-cap {13,15,17}` was
  refuted (sha 808b7af, seed 1785313075, 32×6400 boards/arm, NV). Cap 13 lost
  plain −0.0006 ± 0.0005 (−1.95/fired) and PD −0.0008 ± 0.0006
  (−2.64/fired); cap 15 was similar. Both strong and very weak trump-stack
  sits earn, so the later major-yield and suit-quality sweeps correctly kept
  the wide strength band while testing other axes. BBA converted only 8.6% of
  these advancer hands (median 12 HCP) against our 22.5% (median 11), and
  shrinking our rate toward BBA's lost the head-to-head A/B. Tooling caveat:
  `ab-dump-diff`/`ab-dump-sd` print a stale `Delta (run − sit)` label; the
  reported delta is always ON−OFF.
- **4432 suppression is measured, not pending.** Minor-opening plain was a
  wash; the apparent vulnerable major-opening gain (+0.0269 plain) came from
  an unrelated competitive floor X and its unauthored response, not an
  unsound opening takeout double. Both `set_suppress_4432_vs_major` and
  `set_suppress_4432_vs_minor` therefore stay opt-in/default-off. ⚠ The global
  `docs/bidding-options.md` rows still label both arms “unmeasured”; see the
  compaction overflow for the index correction.
- Advance package provenance: `ee63c4b` (5332 discipline), `48badf5` (rich
  advance), `af66ba7` (Rubens layer), and `8657b22` (longest-first DNF rewrite).

[`advance_2nt`]: ../src/bidding/american/defense/advance_2nt.rs
