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
>
> Update (2026-07-26): the negative control below prices this design's prize at
> **0.65–1.27 IMPs/board** — about the whole remaining gap to BBA. Build it.
> The census that followed (`examples/probe-reading-census.rs`, its own section
> below) says *where*: the blind head is passes through the `Fallback::classify`
> layer, which needs machinery rather than probing, and the largest bid-side ⊤
> mass is **rule competition** on the 1♥/1♠ openings, which needs probing and
> nothing else can reach.
>
> Update (2026-07-30): **both census work items landed, defensive-first, both
> default-off pending A/B.** The census's "missing machinery" mechanism story
> was a misdiagnosis — see the correction in the census section — and the two
> fixes that actually move the blind head are `set_pass_exclusion_reading`
> (symbolic: a pass excludes the strictly-heavier sibling gates it declined)
> and **Stage B itself as `Stance::probe`** (behavioral: self-play boxes keyed
> by *traffic*, not authorship — which is what reaches the floor's passes, the
> real residue). Viability gate cleared by `examples/probe-pass-meaning`:
> one 100k-board self-play sweep gives ≥100 samples to 57.9% of decision
> traffic, no per-node rejection sampling needed. The 1♥/1♠ rule-competition
> chop is deliberately deferred behind the defensive head (user's ordering,
> 2026-07-29).

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

> **Status (2026-07-26): Stages 0, A and C have all landed.** Stage 0 is
> `authored_calls_read_what_they_gate` (095ac85); Stage A is the `Flip`
> complements (1e11c49); Stage C is the whole DNF migration, shipped
> **default-on** on 2026-07-23 after the F2b′ A/B won 4/4 — ledger in
> [`../dnf-migration.md`](../dnf-migration.md). Stage D (cap selection) is
> parked behind a `debug_assert!(< 64)` that has never fired. What remains is
> the census below and **Stage B**. The stage text is kept as the record.

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

**Stage C — the DNF.** `Constraint::project` returns an `EnvelopeUnion` (a capped
DNF) instead of one `Envelope`; combinators implement their cases; `Flip` gets NNF. Consumers wanting a
single box take the hull, **which is exactly today's behaviour**, so the migration is
incremental and each consumer moves at its own pace. Feature-version bump when the
net starts reading more than the hull.

**Stage D — cap selection.** Cluster-count histogram across the book. Evidence, not
argument.

## What readings are worth — the negative control (2026-07-26)

Every attempt to improve a reading tightens some box and measures the
**derivative** of what readings are worth. Two of those in a row landed in the
noise: the agreement overlay (`set_announced_reading`, follow-up F in
[`evaluator-net.md`](evaluator-net.md)) was a wash in all four cells, and the
`Or`-wall chops in [`../dnf-migration.md`](../dnf-migration.md) have been small.
A derivative near zero admits two readings that call for opposite work — *the
prize is large and we are near its optimum*, or *the prize is small* — and
nothing in a tightening experiment can tell them apart.

So measure the **level**. `features::set_blind_inference` (`#[doc(hidden)]`,
default off, `--ns-blind-inference`) blanks all four inference blocks in both
feature vectors: every seat reads `Envelope::unknown`, and the nets reason from
the auction alone. Only the nets go blind — `within_ranges`, `admits` and the
opening-lead sampling read the `Inferences` directly and are untouched, so the
deals are identical and only the decisions taken on them move.
`scripts/ab-blind-inference.sh`, `SEED_BASE=1785072635`, 204,800 bd/arm/vul:

| vul | scorer | IMPs/board | IMPs/fired | fired |
| --- | --- | --- | --- | --- |
| none | plain DD | **−0.6501** [±0.0177] | −1.844 | 72199 (35.25%) |
| none | perfect defense | **−1.1052** [±0.0221] | −3.135 | " |
| both | plain DD | **−0.8850** [±0.0215] | −2.737 | 66223 (32.34%) |
| both | perfect defense | **−1.2685** [±0.0257] | −3.923 | " |

For scale, the 2026-07-26 anchor puts the deterministic pons **−1.152 plain /
−1.355 PD** behind BBA. Readings are worth roughly the entire remaining gap.

The loss is **broad, not a tail**. A third of boards diverge and the mean
divergent board loses 1.8–3.9 IMPs; a tail-driven number would show a small
mean beside a few −24s. The doubled/redoubled wrecks that dominate the *worst*
list are a symptom rather than the driver — doubled finals go 6.44% → 8.09% and
redoubles 0.11% → 0.35%, and that 0.24pp of extra redoubles cannot fund a
0.65 IMP/board swing. What it does explain is the PD-vs-plain spread
(0.38–0.46, PD always worse): a blind seat walks into more doubles, and perfect
defense collects on them.

### What this says about where to spend

The two results together are the whole argument for this design:

| experiment | what it moved | result |
| --- | --- | --- |
| `--ns-announced-reading` | 0.9% of boards, sharpening calls that already read | wash |
| `--ns-blind-inference` | 33% of boards, deleting every reading | −0.65 … −1.27 |

The payoff is in **coverage** — how many calls read as *anything* — not in
sharpening the ones that already do. That is exactly the axis a sampled
projection moves and a per-rule reader does not: one derivation covers every
call, including the ones no one will ever author a reader for and the ones a
learned criterion decides.

The census that sizes the remaining headroom inside that prize is *what
fraction of calls project ⊤ today*, weighted by how often they fire — it turns
"0.65–1.27 is the ceiling" into "and here is the part still on the table." It
is below.

## The census — where the ⊤s actually are (2026-07-26)

`examples/probe-reading-census.rs` counts, at every decision node of real
self-play auctions, how many of the five axes `features::push_inference` hands
the nets (four suit lengths and `points`) are still at their
`Envelope::unknown` value. Same surface the negative control moved, so the two
numbers are commensurable. Frequency weighting is free — every node counts
once — and there is no double-dummy: 20,000 boards cost **1.4 s**.

20,000 boards, seed 1785200001, `american().against(NATURAL)`, 498,687
hidden-seat readings, shipped defaults:

| read seat | readings | ⊤ axes / seat (of 5) | ⊤ on all five |
| --- | --- | --- | --- |
| has bid | 306,017 | 2.646 | 6.29% |
| passes only | 192,670 | 4.257 | 26.21% |

The headroom is real, it is **not** where the 2/1 fit-split bug pointed, and
the ranked worklist splits into two mechanisms that want opposite work.

### The blind head is passes, and it is missing machinery

Every fully-blind key is a pass. Ranked by 5/5-⊤ readings:

| key | readings | ⊤/seat | blind |
| --- | --- | --- | --- |
| `1NT P` | 4,518 | 4.907 | **90.7%** |
| `2♦ P` / `2♥ P` / `2♠ P` | 650 / 631 / 563 | 5.000 | **100%** |
| `2NT P` | 821 | 4.866 | 86.6% |
| `2♣ P` | 867 | 4.803 | 80.3% |
| `1NT P 2♣` | 1,089 | 4.747 | 74.7% |
| `1♦ 1♠ P`, `1♣ 1♠ P`, `1♣ 1♦ P` | 511 / 492 / 421 | 4.77–4.82 | 73–82% |
| `1x P 1y P` (fourth seat) | 1,620–2,481 | 4.36–4.48 | 35–48% |

Against that, `1♣ P` / `1♦ P` / `1♥ P` / `1♠ P` are **0.00% blind** — a pass
over a natural one-of-a-suit opening reads its points band and nothing else,
which is the pass reading working as designed.

The mechanism is the **fallback blind spot** the DNF ledger predicted, now
measured. `project_authored` projects a call only when its classifier answers
`as_rules()`, and `Some` comes from `Rules` alone
([rules.rs](../../src/bidding/rules.rs)) — the blanket
`impl<F> Classifier for F` inherits the `None` default
([trie.rs](../../src/bidding/trie.rs)). Every position wired as
`Fallback::classify` therefore has **no projection attempted at all**, which is
why a pass over a weak two reads 5/5 ⊤ on 100% of readings. This is not a
missing box on an authored rule; it is a layer the projection pass never
reaches. Cheap to fix relative to Stage B, and it owns the plurality of the
blindness.

**Correction (2026-07-30, code/git audit): the mechanism above is wrong,
though the numbers stand.** The fallback-resolution machinery had already
shipped a month before the census (`Trie::authoring_classifier` +
`FALLBACK_PROJECTION`, default **on**, 2026-06-28), and essentially every
authored fallback site passes a `Rules` *value* whose `as_rules` survives the
`Arc<dyn Classifier>` erasure — the `fallback_rules_read_what_they_gate`
meter pins exactly 6 opaque installations, none owning census mass. The
blindness decomposes instead as:

- **≈7,000 readings, vacuous authored gates** — projection *runs* and
  honestly reports the `hcp(0..)` catch-all Pass gate (weak-two and 1NT
  defenses, `over_their_overcall`). Fixed symbolically by
  `set_pass_exclusion_reading` (below).
- **≈4,700 readings, the neural floor's passes** — `2NT P`, `2♣ P`,
  fourth-seat `1x P 1y P`: chosen by the net, nothing to expose. **Only the
  probe reads these.**
- **≈800 readings, truthful unions** — `1NT P 2♣` with garbage + crawling
  Stayman shipped genuinely promises ~nothing on the census axes. Correct
  blindness; leave it.
- `1NT P` (≈4,100) is **hull-irreducible**: the only strong tier over their
  1NT is shaped (`hcp(15..) & balanced` double), so no sound points ceiling
  exists for the passer at hull level — probe territory too.

### The biggest *bid* ⊤ mass is rule competition — only a probe can reach it

| opening | readings | ⊤/seat | axes read |
| --- | --- | --- | --- |
| `1♥` / `1♠` | 8,802 / 9,369 | 3.03–3.04 | own suit + points |
| `1♣` / `1♦` | 11,850 / 12,675 | 1.08–1.09 | own suit + points + **both majors** |

The asymmetry is in the rule text, not the machinery: the minor openings carry
`len(♥, ..5) & len(♠, ..5)` explicitly, the major openings carry no cap on the
other major ([american/openings.rs](../../src/bidding/american/openings.rs)).
`1♥` does imply at most four spades — but only because `1♠` outranks it at 1.6
against 1.5 and takes the 5-5 hands. That is **rule competition**, exactly the
axis no per-rule algebra can see and a probe can. The 1♥/1♠ pair is 55,177 ⊤
axes over 18,171 readings, the largest bid bucket in the census.

Caution before treating it as free: filling it tightens a length *ceiling*
(♠ `0..13` → `0..4`, so `max/13` moves 1.00 → 0.31), which is the C1-shaped
move refuted 4/4 in [`../dnf-migration.md`](../dnf-migration.md). It differs
from C1 in that it moves real **mass** — five-card spade suits are excluded,
not merely made unreachable — so a retrain can be earned on it where C1's could
not. Measure it as a chop; do not assume it.

### What it decides

Two separate pieces of work, in cost order:

1. **Project the fallback layer's passes.** Missing machinery, no probe needed,
   and it owns the blind head.
2. **Stage B for rule competition.** The 1♥/1♠ ceilings are unreachable
   symbolically by construction — the first measured case where the probe is
   the *only* instrument rather than the more convenient one.

## What landed (2026-07-30) — and what the A/B still owes

**Pass-exclusion** (`set_pass_exclusion_reading`, default off) is the sound
symbolic completion of the pass reading: under argmax a pass proves the hand
outside every sibling gate whose weight strictly beats every Pass rule's, so
the pass band is intersected with those gates' complements — single-box
complements only (a shape-free tier like the weak-two defense's
`points(17..)` double; a shaped or bounded gate complements to a union or ⊤
and is skipped, costing precision never soundness). Census (20k boards, seed
1785200001): `2♦/2♥/2♠ P` **100% blind → 0.00%**, ⊤/seat 5.000 → 4.000,
every control key byte-unmoved. Guarded by the
`passes_read_within_their_table` sweep: wherever a table's argmax is (or
ties with) Pass, the knob-on projection must admit the hand. Expectation
management: the band is equivalent to the refuted `weak_two_pass_gate`
(C1-encoding loss pre-retrain), so it ships off, queued for the next retrain.

**Stage B is `Stance::probe`** (+ `set_probed_reading`, default off), with
one design amendment over the staging above: coverage is keyed by
**traffic**, not authorship. One self-play sweep records the actor's hand at
every decision; every prefix key with ≥200 observations stores a widened
bounding box (points ±2, lengths ±1 — a sample edge is not a rule edge), two
iterations with fixed-point drift reported. This dissolves the per-node
acceptance-rate question the caveat below poses: there is no per-node
rejection sampling at all, and `examples/probe-pass-meaning` (the viability
gate) measures 57.9% of decision traffic at ≥100 samples from 100k boards.
The class-C keys it reaches are real content: `1NT P` passer mean 7.8 points
p99 17 (vs ⊤, and hull-irreducible symbolically), `2NT P` mean 7.1, `2♣ P`
mean 6.2, all vs ⊤ today.

**The probed census** (20k boards, seed 1785200001, `--probe 100000`: 520
keys stored, 241 drifted between the two probe iterations) moves the whole
surface, not just the pass head: has-bid ⊤/seat **2.642 → 0.541** (blind
6.22% → 3.05%), passes-only **4.257 → 1.442** (blind 26.21% → **4.67%**).
Every key of the old blind head leaves the worklist — including the **1♥/1♠
rule-competition ceilings** (3.03 → ~0.09 ⊤/seat), the chop deferred above:
the probe fills an axis without asking why it was ⊤. The residual head is
exactly what traffic-keying predicts, the sub-`MIN_SAMPLES` tail (`1♣ 1NT P`
at 280 readings, `2♠ X XX`, deep competitive keys) — coverage there is a
boards-count dial, not a design question. The 241/520 drift is the honest
fixed-point number: probed readings materially move the bidder's own
auctions, so a consumer that retrains on probed features must retrain on the
*post-probe* auction distribution.

The same example doubles as a **published-vs-actual divergence meter**, and
its first run reported two candidate defects. One was the meter's own:
`1♣ P 1♥` appeared to announce 6..=11 against responders running past 20.
Both readings were correct and the *row* was wrong — `auction_key` strips
leading passes, so the key pooled `1♣ P 1♥` (unpassed responder, a new suit
is unlimited: `at_least(6, POINTS_CAP)`) with `P P 1♣ P 1♥` (passed
responder, where `set_pass_reading`'s 11-point opening-pass cap correctly
intersects it to 6..=11), and printed whichever prefix the aggregate happened
to store. **Two populations, one row.** The meter now keys on the full prefix
(2026-07-30); split, the same seed gives 6..=37 against observed 6-20 for the
unpassed key and 6..=11 against observed 6-11 for the passed ones — sound on
both sides. *Lesson: a divergence meter must key on everything that changes
the reading, and passer status changes it.*

The second was real, and is now **closed (2026-07-30)** — but the recorded
attribution ("fuzzy-gate slack") was wrong. The re-run on corrected keys
(100k boards, seed 1785200001) confirmed the breach survives the split on
every passer variant — `1♠ P 2♠` observed 4-10 (n=722, p1 5) against a
published 6..=10, `P P 1♠ P 2♠` observed 4-9 (n=332), `P 1♠ P 2♠` 5-10,
`P P P 1♠ P 2♠` 5-9; the ceiling held everywhere — so it was not pooling,
and the strength dial defaults to 0, so it was not fuzz either. The
mechanism is a **unit transplant**: the raise gate is
`support(3..) & support_points(major, 6..=9)` — a *support-scale* band,
where side-suit shortness has value — and the hand-written reader published
that band verbatim on the legacy `point_count` axis
(`narrow_points(6..=10)`) beside the correct dedicated slot. A 4-HCP hand
with a singleton holds 6+ support points, raises inside its gate, and sat
outside the published legacy box. The same transplant lived at the jump
raise (10..=12), the limit-plus cue-raise (10..), and the Rubens cue-raise
(10..). `SupportPoints::project` had refused this from birth — its comment
warns the legacy gauge has "no lower bound on the shortness scale" — only
the hand-written reader transplanted.

The fix is three-sided, because three consumer classes want three different
things from the same number:

- **The envelope** (sampler `admits`, the meter, disclosure) gets soundness:
  the reader keeps the exact band on the support slot and writes only its
  sound image on the legacy axis — `support_band_to_points`, floor −5 /
  ceiling +1, the statically derived maximum inter-scale skew with 3+
  trumps, pinned by `support_band_points_image_is_sound`. `admits` gauges
  the legacy axis unconditionally, so before the fix constrained sampling
  *rejected* partner's true shapely raises — the census's phantom-precision
  failure mode, on our own partner.
- **The floor's arithmetic gates** (`combined_points`, `combined_hcp`,
  `fit_sum_game`, `partner_slam_strength`) get the same figure they always
  calibrated on, from its correct home: `Strength::shown_floor()`, the
  legacy floor lifted by every populated support promise. At every
  hand-written raise node this reproduces the old number exactly.
- **The nets** get their training distribution: `features::net_points`
  folds the support slots back into the served points hull, byte-identical
  at raise nodes to the transplanted hull the corpora contained. Serving
  the honest widening instead would be the pass-exclusion OOD lesson again
  (a reading change the net consumes is a retrain, not a free edit); the
  next feature version retires the fold by serving the slots as columns.

**Verification (2026-07-30).** Post-fix meter, same seed: every `1♠ P 2♠`
passer variant reads `1..=11` with the observed 4-10 inside — sound on both
bounds — and the per-key sample counts are byte-identical, so the self-play
traffic did not move at the fixed keys. The behavioral residue was priced
with a `bba-gen` dump diff (seed 1785400000, 6400 boards): 27 of 12,800
tables diverge (0.21%), 10 contract changes, scored **plain +0.0034
[±0.0088] / PD +0.0033 [±0.0088]** IMPs/board — a wash with positive lean
on both scorers. The divergence does *not* sit on simple raises (their
gates see the identical `shown_floor`); it concentrates on Jacoby-2NT and
GF-machine auctions whose **book-projected** `support_points(major,
16../18..)` slots existed but were consumed by nobody — `shown_floor` and
the fold now read those sound gate claims, so a few slam auctions move in
both directions. A full A/B can pin the residue down if it ever needs a
sharper bound; at 0.16% contract divergence measuring wash-positive, the
soundness fix ships without one. `probe-reading-sound` (2000 boards, seed
1785400000, old vs new binary) prices the fix's share of the partner
soundness defect: partner box-excludes-truth **3.515% → 3.351%** (297 →
283 readings) — real movement, but the raise transplant was a minor slice;
partner's residual ~3.35% stays an open defect with its mass elsewhere.

### The A/B verdict (2026-07-30, seed 1785344858, 204,800 bd/arm/vul)

| arm | plain none | plain both | PD none | PD both | fired |
| --- | --- | --- | --- | --- | --- |
| exclusion | −0.0060 | −0.0062 | −0.0029 | −0.0032 | 0.68% |
| probed | −0.314 | −0.428 | −0.570 | −0.690 | 25–28% |

**Exclusion**: the pre-registered C1 signature — a small real plain loss on a
net-visible surface encoding a band the features already refuted. Stays off,
re-measure after the feature retrain; the pre-retrain number is a floor, not
the verdict.

**Probed v1 is refuted as a bidding input**, and not marginally: two orders of
magnitude past convention scale, negative on both scorers (so not a PD
doubling artifact), and the mechanism is legible in the worst boards — **104
of the 160 worst end in a contract we redoubled that the base arm never
doubled**. The boxes are too tight, so the floor reads the opponents as
limited, doubles, and gets redoubled into a making contract. This is the
soundness asymmetry of §"probing fails toward false precision" arriving on
schedule: the census metric (⊤ mass removed) rewards *tightness*, and
tightness is exactly the failure direction. **The census is not a proxy for
the A/B — the two metrics point opposite ways.**

What survives: the boxes are good *description* (disclosure, sd-lead pricing,
sampler priors) and the class-C content is real. What is refuted is v1
widening (points ±2, length ±1) feeding a bidder that trusts its readings.
Next candidate, if this is picked up: widen by *sample quantiles with a
coverage guarantee* — store the box that admits ≥99% of observed hands with a
count-scaled margin, so a thin key widens toward ⊤ rather than toward a lie —
and re-price penalty doubles separately, since they are the sole loss channel
identified.

### v2 (2026-07-31): the vacuous-scoped serving

Picked up from the other end — before sharpening the *boxes* (the quantile
candidate above), scope the *serving* to the slice v1's mechanism cannot
reach.  `set_probed_vacuous_reading` (`--ns-probe N --ns-probe-vacuous`,
default off) serves the same probed map:

- **own-side calls only** (the self-referential caveat: right for partner,
  wrong for opponents — and v1's redouble trap was the opponents' boxes);
- **onto fully-open axes only** — fill-⊤, never tighten.  The mask is judged
  against the *complete* symbolic reading, so the fold lives at the end of
  `Inferences::read`, after the natural walk (inside `project_authored` it
  tightened half-open axes — the walk stamps after that returns);
- **contested prefixes only** — from the first index where both sides have
  acted.  Measured necessary, not hypothesized: unscoped fill-⊤ smoke-tested
  at 23% fired, **−0.67 IMPs/board**, all constructive grand blasts
  (`1NT–2♦–2♥–3NT–7NT X`) — filling constructive ⊤ axes (opener's minors,
  responder's side suits) shrinks sampling σ on slam auctions, the exclusion
  retrain's worst-board signature at 30× the fired rate.  Contested-scoped,
  the same 200-board smoke reads 8% fired, +0.015 [±0.212].

`Stance::probe` runs its fixed-point iteration under whichever fold is armed
(set the vacuous knob before probing), so the boxes are consistent with
their serving policy.  The target population is the reading-drift ledger's
coverage hole — contested free bids the walk stamps nothing for
(`1♦ (2♣) 2♠` partner ♠ `0..13`) — which is also where the keycard rail's
`recognizable` gate reads.

**Verdict (A/B `ab-results/probed-vacuous/`, SEED_BASE 1785493701, 204,800
bd/arm/vul): LOSS in all four cells** — plain −0.0467/−0.0658, PD
−0.1118/−0.1337 (none/both), ~10% fired — full table and worst-board trace
in docs/reading-drift-handoff.md.  One mechanism, the pre-registered one:
the contested floor net, fed partner boxes tighter than its training
distribution, keeps acting where the base arm settles (reopens, blasts,
doubles into redoubles) — the exclusion retrain's σ-shrink signature on the
slice where the floor net decides most.  The knob stays opt-in; the queued
path is the probe-first retrain gate, then the F2b twin served under the
knob, with the quantile widening above as the box-side lever if the retrain
washes.  v2's serving scopes stand as measured constraints for any v3: the
constructive slice is untouchable pre-retrain at −0.67 IMPs/board, and
own-side/fill-⊤ alone is not enough — the *consumer* (a net that trusts its
readings without having trained on them) is the binding wall, not the
serving policy.

### The exclusion retrain (2026-07-30)

The queued retrain was bought **probe-first**, per the C-P lesson that C1's
retrain was unearnable and a ~2s probe can say so before a GPU run and an A/B
are spent. `probe-closure-features --pass-exclusion` (new arm; the run also
repaired the example's label table, stale since the round-one shape block —
45 labels vs `LEN_SEAT_SHAPE` 75, a panic on every arm) pre-registered a
stark-only kill criterion: kill only on the C1 picture, endpoints moving with
moments unmoved and zero rejections. The measurement (2000 boards, 20,347
nodes, seed 1785351807) is the anti-C1 picture on both halves: **10,034 of
93,220 cross-arm layouts rejected (10.8%**, vs C1's 0/409,708), and the
moment columns moving for real — E ≈1σ, sd to 2.49σ, histogram cells to
3.22σ at p90, mass 1.67σ — at 1.07% of nodes, beside endpoint movement at
3.60% (`pts max` at 575 nodes, 1.89σ; and *length* ceilings from single-box
shape complements, ♦ max 7.33σ at 35 nodes, e.g. ♦ `0..13 → 0..2`). Both
predicted channels are live, so the perturbation carries information and the
retrain can be earned.

The twin is the F2b recipe on the shipped v3: `dump-evaluator --encoding
eval3 --envelope-union --pass-exclusion` (new flag, sidecar records the regime) over
500k deals of `22.pdd` seed 1 → 10,161,643 rows (knob-on auctions move, as
they should); trainer `--hidden 256 --epochs 150 --batch 4096 --lr 0.001
--seed 1`. **Held-out gate passed: val NLL −1.55010 / MAE 1.391 tricks** on
the knob-on deal-disjoint tail vs the dnf twin's −1.54872 on its own regime —
the OOD penalty gone at better-than-equal fit. Serving keys on
`pass_exclusion_reading()` inside the v3 calls-tail path
(`trick_estimates_with_auction`), so the ON arm of any A/B picks the twin up
through the thread-local knob with no harness change; knob-off is
structurally byte-identical (`exclusion_matches_candle_fixture`,
`exclusion_knob_swaps_v3_weights`).

**Re-measure DONE 2026-07-30**: `scripts/ab-exclusion-retrain.sh`
(`ab-results/exclusion-retrain/`), base vs `--ns-pass-exclusion`, 204,800
bd/arm/vul, SEED_BASE 1785354456, sha ad0983f plus this uncommitted tree.
Pre-registered disposition: plain wash + PD win → the reading flips
default-on; a plain loss again → thread closed, knob stays opt-in
permanently.

**Verdict: wash in all four cells — no PD win, so the ship condition fails;
the knob stays opt-in and the thread is closed.** Plain −0.0024
[±0.0040]/−0.0028 [±0.0051], PD −0.0016 [±0.0045]/−0.0001 [±0.0057]
(none/both), fired 1.45%/1.50% — up from 0.68% pre-retrain because the twin
moves *evaluations*, not just readings. The retrain did recover the channel
it could: plain −0.0060/−0.0062 → −0.0024/−0.0028 and PD-both to −0.0001,
landing the residual exactly where the interpretation note below predicted —
in the authored-gate channel, which a further retrain cannot reach. Worst-
board trace (iron rule): the dominant ON-side pattern is grand-slam blasts —
13–15 of the 40 worst per vul bid 7NT/7M where OFF stops in six (grands in
the OFF lines: ≤1). Reading opponents' passes caps their strength, which
concentrates the missing honors in partner's inferred box — μ up, σ down —
and the twin blasts on confidence the deal repays only half the time; the
aggregate wash says this is variance, not edge.

Note for interpretation: the pre-retrain loss was never isolated between the
net-OOD and authored-gate channels (C1 split ~60/40); if the post-retrain
number lands between wash and the old −0.006, the residual is the authored
gates reading the tightened bounds, and a further retrain cannot recover it.

## Per-node training — the probe as the sync step (recorded 2026-07-29)

The larger ambition this build serves (jdh8): *discover* meanings from
self-play, per node — not only read them off the authored system. Three
anchors keep that honest:

- **The probe is bidder-agnostic.** "The acceptance test is the system" holds
  whether the decision-maker is authored rules, the distilled floor, or a
  future per-node-trained policy. For a learned policy the probe is the
  *only* reader — its conventions exist purely behaviorally — and the only
  route to disclosure (`artificial_calls_are_alerted` demands a reading).
- **Per-node training is coordinate ascent on the partnership's joint
  policy**, and the probe is its synchronization step: train a node →
  re-probe its calls → downstream nodes see fresh readings → train them.
  Leaves are coordination-free (no downstream language to break) and
  constructive leaves already proved out (bilans, M6.4); competitive leaves
  are where the DD reward is most biased (the obstruction wall that killed
  M7).
- **The walls are measured, not hypothetical**: DD reward bias unlearns
  preemption; self-play drifts to private codes (fine vs itself, wrong vs
  the field and human partners — BEN anchors to human data for this);
  convention *discovery* needs orders of magnitude more games than
  distillation. Defensive/contested territory first, which also respects the
  book/floor partition invariant (learned layers wrap contested books only).

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
