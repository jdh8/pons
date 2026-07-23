# Sampled projection — probe the bidder at authoring time, store the answer

> Status (2026-07-22): **design only, no code.** Motivated by a measured bug (the
> 2/1 reading is erased) and a measured feasibility probe, both below. Successor
> to [`archive/rule-projection.md`](archive/rule-projection.md), which shipped
> `Constraint::project` and recorded `Or`-disjunction as its wall. This is the
> proposal for that wall.
>
> Note: `constraint.rs:861` points at `docs/ai-bidder/rule-projection.md`; the file
> is in `archive/`. Stale path — worth fixing when that comment is next touched,
> since it is the comment justifying the vacuous `SupportPoints::project` this
> design makes moot.

## The idea, in one line

**Probe the bidder while authoring; store the answer as the projection.** Runtime
cost is a lookup.

Rule projection asks the authored rule *what a call promises*, and must answer
soundly — the reading has to contain every hand that could have made the call. The
sound answer for a disjunction is the union, and the only single box containing a
union of boxes is the bounding box, which is frequently everything.

Probing asks a different question: *which hands does the bidder actually make this
call with?* Deal hands, replay the one decision, keep the matches. The acceptance
test **is** the system, so disjunctions, cross-suit correlation, rule competition
and knob interactions all fall out with no soundness slack and no per-convention
reader.

Two halves, and they are independent:

- **Representation** — what a reading *is*: a capped DNF over slabs (below).
- **Derivation** — where the numbers come from: probing at authoring time.

## Primer: how a reading is computed today

Skip if you already know `Constraint`. Everything below leans on it.

A reading is a **summary** of the hands a call is consistent with — a range per
suit length and a range of points. Interval arithmetic, but on hands. The one
rule is that the summary must **contain every hand partner could actually hold**.
Too wide is safe (we merely know less than we could); too narrow is a disaster
(the sampler rejects a hand partner genuinely has, then deals them something
impossible). *Sound* means "never too narrow", and that asymmetry drives every
decision in this doc.

A [`Constraint`](../../src/bidding/constraint.rs) answers three questions about
one authored rule:

```rust
fn eval(&self, hand, ctx) -> f32;      // "does THIS hand qualify?"  0.0 yes, -inf no
fn describe(&self) -> Description;     // "what do you mean?"        (English)
fn project(&self, ctx) -> Envelope;   // "summarise ALL hands that qualify"
```

`eval` runs **backwards** (we have a hand, does it fit?) — that is our own
bidding. `project` runs **forwards** (no hand; what does the call imply?) — that
is reading partner. The trait states the contract exactly: *a finite `eval(hand,
context)` implies `hand` lies within `project(context)`*.

Writing `hcp(13..) & len(♣, 4..)` computes nothing; it builds the value
`And(Hcp(13..), Len(♣, 4..))`. Each combinator says how to answer all three:

| | `eval` | `project` |
| --- | --- | --- |
| `And<A,B>` | both pass (sum the scores) | **intersect** the summaries |
| `Or<A,B>` | either passes (max the scores) | **union** the summaries |
| `Flip<A>` | inner must *fail* | *(not implemented — see below)* |

**The default is what keeps it safe:**

```rust
fn project(&self, _ctx) -> Envelope { Envelope::unknown() }   // "any hand"
```

A constraint that does not override this says "I know nothing." Loose, never
wrong — which is why `Balanced` and `StopperIn` are sound without anyone
reasoning about them.

### Why `!` is the subtle one

`Flip` implements `eval` and `describe` but **not** `project`, so it inherits the
shrug: `!len(♠, 4..)` reads as "any hand" when it plainly means at most three
spades. Sound, just lazy.

The trap is the obvious fix:

```rust
// WRONG
fn project(&self, ctx) -> Envelope { complement(self.0.project(ctx)) }
```

For `!len(♠, 4..)` that works, because the inner summary is exact. For
`!balanced` it is a catastrophe: the inner summary is the shrug ("any hand"), and
the complement of "any hand" is "**no hand**" — too narrow, the dangerous side.
6-5-1-1 is not balanced, yet we would have told the sampler nothing qualifies.

The root of it: `project` returns an *over-approximation*, and **an
over-approximation cannot be complemented** — flipping a superset yields a subset
of the truth, not a superset of it.

**The fix is to ask, not to flip** — a fourth question with the same safe default:

```rust
/// Summarise all hands that FAIL this constraint.
fn project_complement(&self, _ctx) -> Envelope { Envelope::unknown() }
```

`Flip::project` then calls `self.0.project_complement(ctx)`, asking the inner
constraint about *its own* negation rather than flipping its answer to a
different question. `Len` overrides it exactly (`0..=3`); `Balanced` does not and
stays safe. **Precision is opt-in; safety is the default**, so forgetting to
think about a constraint costs precision and never correctness — no macro, no
type-level enforcement, no compile-time pass required.

This is the same shape as `project_band`, which already sits directly below
`project` for the same reason: `project` claims floors only, so a declined call
needed its own two-sided question rather than one derived from `project`. The
lesson was already learned once in this file.

## The bug that motivated this

Measured 2026-07-22. **Every 2/1 game force reads as `points 0..=37`** — no
strength information at all — while every natural response reads correctly.

| response | partner's points |
| --- | --- |
| `1♠–2♣` / `2♦` / `2♥`, `1♥–2♣` / `2♦` (every 2/1) | **0..=37** |
| `1♠–1NT` (forcing) | 6..=12 |
| `1♠–2♠` (raise) | 6..=10 |
| `1♠–3♠` (limit) | 10..=12 |

The cause is the shipped fit-split in
[`american/responses.rs`](../../src/bidding/american/responses.rs):

```rust
len(suit, 4..) & !support(4..)
    & (hcp(13..) | (support(3..) & support_points(13..)))
```

`project` on an `Or` is the union of its branches. `hcp(13..)` floors points at 13;
`SupportPoints::project` deliberately returns `unknown()` (an exact floor *is*
unsound — `new_point_count` is not a lower bound on legacy `point_count`). Union of
`13..=37` with `0..=37` is `0..=37`. Confirmed by flipping the knob: fit-split on →
`0..=37`, off → `13..=37`.

**A second, independent gap in the same rule.** `!support(4..)` means at most three
spades — conjunctive, and a perfectly representable interval — yet the responder's
spades read `0..=13`. `Flip<T>` implements only `eval` and `describe`, so it
inherits the default `project` returning `unknown()`. Negation projects nothing.

Consequences reach past the reading. A `0..=37` envelope is maximally wide, which
maximises the evaluator's `σ`; with `μ` below target a fatter `σ` *raises*
`P(≥ tricks)`, so the bilans slam gate over-fires — a 12-count opposite a 2/1 asked
keycards instead of signing off in 4♠. Every A/B baseline touching a game force was
measured against this blind envelope.

## Feasibility, measured

Release build, `1♠ – (P) – 2♣`, dealing responder's hand and replaying the bidder:

| source | partner's points |
| --- | --- |
| projection, fit-split on (shipped) | 0..=37 |
| projection, fit-split off | 13..=37 |
| **behavioural probe** | **11..=26, mean 15.1** |

The real finding is the **floor moving 13 → 11** — the fit-split branch genuinely
admits shapely 11-counts, so the sound reading is not merely loose, it is wrong on
that side — and the **density**, concentrated near 15.1, which no projection
conveys at all.

### The 26 is a support bound, not a ceiling — and this is the danger

Do **not** read `26` as "the rule stops here." A 27-count opposite an opening bid
is extraordinarily rare (partner already holds ~12+), so `26` is almost certainly
just the largest hand the sample happened to contain: 20,000 deals × 8.6%
acceptance ≈ 1,718 hands, and the tail beyond 26 may never have been drawn.

This inverts the failure modes, and it is the single most important safety point
in this document:

| approach | fails by | consequence |
| --- | --- | --- |
| projection | vagueness (too wide) | we know less than we could — **safe** |
| probing | false precision (too narrow) | **excludes legal hands — catastrophic** |

Projection cannot violate soundness; probing can, trivially, by mistaking the edge
of a sample for the edge of the rule. Two mitigations, both required:

- **Never store an observed upper bound as a hard bound.** Widen upper bounds to
  the theoretical maximum unless the sample shows mass dying off well *before* the
  boundary, and record the sample count so consumers know how much to trust the
  edge.
- **Judge a probe by separation, not extremity.** `26` versus `37` is worth nothing
  if no hand lives up there anyway. The valuable signal is where the mass is, and
  the floor — both well inside the support.

Cost: **~1.2 µs per candidate**, acceptance **8.59%** for one call and **4.00%** for
two (`2♣` then `3♠`). The second call costs only a ~47% factor rather than another
8.6% — a deterministic bidder's successive calls are strongly correlated, so there
is no exponential blow-up along our own side's calls. ≈3 ms per 100 accepted hands,
against the ~1.4 s sample-and-solve loop the evaluator net exists to amortize (that
cost is double-dummy solving, not sampling).

## Representation: atoms are slabs, combinators do the rest

`Envelope` has exactly five axes — four suit lengths and points. Every primitive
constrains **one** axis to a contiguous interval and leaves the rest at ⊤:

| constraint | axis | atom? |
| --- | --- | --- |
| `len(suit, r)`, `support(r)` | that suit's length | ✓ contiguous |
| `points(r)`, `hcp(r)` | points | ✓ contiguous (modulo scale slack) |
| `suit_hcp`, `flat_4333`, `support_points` | *none* — off-axis | ⊤, **both polarities** |

So atoms are **slabs**, not boxes, and the algebra is small:

```
literal      = slab             (atom, or negated atom)
conjunction  = box              (intersect per axis; ⊤ on untouched axes)
reading      = union of boxes   (a DNF)
```

- `∧` distributes — terms multiply
- `∨` concatenates
- `¬` is pushed to the leaves by **NNF** and never seen again

**Negation is cheap under NNF.** Complementing a *box* would cost up to `2d` boxes,
but we never complement a box — only atoms. A half-open range (`4..`) complements to
`..4`, still one slab; a bounded range (`4..=6`) gives two slabs, absorbed as an
`Or`. So `!support(4..)` is a single slab, `spades 0..=3`, needing no new
representation at all.

**NNF is also a soundness requirement, not a convenience.** Off-axis atoms are
approximated as ⊤, and `¬⊤ = ⊥` — tighter than the truth, therefore unsound: the
sampler would reject legal hands. Converting to NNF first means `¬` only reaches
atoms, and each *literal* is approximated independently with non-representable ones
mapping to ⊤ under **both** polarities. Getting this backwards is the worst
available failure mode: silent and shape-dependent.

**Cap and widening.** `∧` of two `Or`s is the only real growth source. Cap the term
count; on overflow merge the nearest terms into their hull — always a sound
*over*-approximation. The cap subsumes every special case previously considered:

| cap | equals |
| --- | --- |
| 1 | today's behaviour (bounding box) |
| 2 | a "two-slot" reading — the fit-split, weak-or-strong two-ways |
| 3 | full Multi (weak ♥ / weak ♠ / strong) |
| 4 | DONT's double — a one-suiter in any of four suits |
| ∞ | exact |

`or([♥,♠], 6..)` is then just a two-term DNF of slabs, so Multi needs no special
case. This also retires hand-maintained workarounds: [`or()`][cons] documents itself
as *"sound but loose, since a one-of-N suit cannot floor any single suit"*, and
`inference.rs:1229` caps the **other** three suits so the residual forces length into
the long suit — *"the same loose handling Landy uses for its 5-4."* Both say in code
what a two-term DNF says directly.

**Do not guess the cap — measure it.** Once probing exists, the number of hypotheses
a node needs is empirical: cluster each node's accepted hands and histogram the
cluster count across the book. Same move that settled the width ladder in
[`evaluator-net.md`](evaluator-net.md).

[cons]: ../../src/bidding/constraint.rs

## Derivation: probe at authoring time

Do **not** run the acceptance test at gate time. Gate → sampler → bidder → same gate
is a recursion trap. Probe once when the system is constructed, store the DNF on the
node, and runtime cost returns to a lookup.

Pipeline, per authored call in the trie:

1. Deal hands for the seat that made the call, conditioned on the auction prefix.
2. Replay **that one decision** through the bidder; keep the hands whose call matches.
3. Fit ≤ `cap` boxes to the accepted set (cluster).
4. Store as the node's projection.
5. Iterate to a fixed point.

Three constraints:

- **It is a fixed point.** Probing invokes the bidder, which consults projections,
  which is what is being computed. Iterate — v0 = today's symbolic projections,
  probe → v1, probe → v2 — and *assert* stability rather than assuming one pass
  converges.
- **Hook it at `Stance` construction, not build time.** `const` cannot run a bidder
  and `build.rs` cannot see the knobs — and readings are knob-dependent (the whole
  bug is that `set_two_over_one_fit` changes what `2♣` means), so a baked table would
  be wrong for every A/B arm. `american()` already builds a `Stance`. At ~1 s per
  thousand-node book this amortizes to nothing across a 200k-board A/B.
- **Reading-affecting knobs must be fixed at construction.** `ab-bilans-floor` flips
  `set_bilans_floor` *per seat per call*; that is safe only because it gates the
  floor, not the book's rule tables. Assert the invariant — do not assume it.

### Why probing beats deriving symbolically

The slab algebra is exact with respect to *one rule*. Probing beats it on two axes
that no per-rule algebra can reach:

- **Rule competition.** A rule's own text says nothing about the hands that pass it
  but bid something *else* because a higher-weighted rule outranks it. Only a probe
  sees the call the bidder actually makes. (The measured `26` is **not** evidence of
  this — see the support-bound warning above. The effect is real; that number is not
  its demonstration.)
- **Off-axis atoms.** `suit_hcp(♠, 5..)` is ⊤ symbolically forever, but a probe sees
  its *shadow* on the axes we do record — points shifted, lengths skewed.

### Where the symbolic path survives

Auctions the book never authored — floor positions, deep competitive sequences —
have no node to have probed. Those fall back to the symbolic DNF, which is cheap and
needs no fixed point. It is a genuine second path, not a vestige.

## Implementation stages

Each stage changes the net's inputs, so each costs regen + retrain (~30 min on the
GPU trainer) + A/B both vulnerabilities, plain and PD. **The validation dominates the
code.**

**Stage 0 — the invariant test (arguably the highest value here).** The fit-split
bug was not an authoring error: `hcp(13..) | (support(3..) & support_points(13..))`
is a *correct* bidding rule that measured as a win. The machinery silently degraded
it, and `0..=37` is a perfectly well-formed `Envelope` — nothing errored, nothing
was empty, no test went red. It simply stopped knowing anything and kept a straight
face.

The repo already has the idiom for catching this class: `artificial_calls_are_alerted`.
The sibling is

> **`authored_calls_read_what_they_gate`** — for each authored call, if the rule
> mentions an axis (`hcp`, `points`, `len`, `support`), the projection must not be
> ⊤ on that axis.

The fit-split would have failed it the day it shipped: the rule mentions `hcp`, the
projection says nothing about points. It also catches the `Flip` gap, since
`!support(4..)` mentions spades and projects ⊤ there. Stage A fixes one leak; this
catches the next one. **Do this first.**

The principle it encodes: the machinery may be *imprecise*, but it must never be
imprecise *invisibly* — ⊤ is a fine answer as long as ⊤ is distinguishable from an
answer.

**Stage A — `Flip::project` (hours, no feature-version bump).** Add
`project_complement` to `Constraint`, defaulting to `unknown()`; `Flip::project`
delegates to it; implement on `Len`, `Support`, `Points`, `Hcp` for **half-open
ranges only** (a bounded range's complement is two slabs and must wait for the DNF).
Test: the 2/1 responder's spades go `0..=13` → `0..=3`. Independent of everything
else and nearly free.

**Stage B — the headline bug.** Either B1, a slacked `SupportPoints::project`
(~`8..=37`, an hour, throwaway); or B2, probing (`11..=26`, a day, and it is the
instrument Stage D needs). **Go straight to B2**; B1 only if B2 slips. This is the
stage that unparks the evaluator — the three tests pinning the 12-count at 4♠ are
downstream of the blind envelope.

**Stage C — the DNF.** `Constraint::project` returns a capped DNF instead of one
`Envelope`; combinators implement their cases; `Flip` gets NNF. Consumers wanting a
single box take the hull, **which is exactly today's behaviour**, so the migration is
incremental and each consumer moves at its own pace. Feature-version bump when the
net starts reading more than the hull.

**Stage D — cap selection.** Cluster-count histogram across the book. Evidence, not
argument.

## Caveats

1. **The feasibility probe is optimistic.** It dealt a fresh deal and tested one seat
   in isolation. The real thing holds our hand fixed, deals the other 39 cards, and
   must replay the opponents' calls too. Measure that acceptance rate first — it is
   the number that decides viability.
2. **Self-referential by construction.** "Hands partner would bid `2♣` with" means
   *our* system's partner. Correct for partner modelling, wrong for opponents playing
   something else (`against(Family::NATURAL)`, the BBA exploit guard).
3. **Every ledger A/B baseline was measured blind** on game-force auctions. After
   Stage B, prior results involving a 2/1 are not strictly comparable to new ones.
4. **`set_rule_accept` is the adjacent knob**, not this. It replays *authoring rules*
   in the sampler (default off, runs ~0.09 tricks tight). This is the same slot with a
   better acceptance test.

## Related

- [`archive/rule-projection.md`](archive/rule-projection.md) — the shipped projection
  design and its `Or` wall
- [`evaluator-net.md`](evaluator-net.md) — the consumer whose `σ` inflates on a wide
  envelope
- [`ben-architecture.md`](../ben-architecture.md) — the learned-reading alternative; a
  net is a better consumer of a disjunction than a box is
- [`bidding-architecture.md`](../bidding-architecture.md) — the book/floor/inference
  layer cake
