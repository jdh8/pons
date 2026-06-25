# Rule projection — reading an authored call straight off its rule

> Status (2026-06-25): **Stages 1–3 shipped.** Stage 1 (`Constraint::project`),
> Stage 2 (M6.2b: `Rule::project` + the generic projection pass + the equivalence
> test), and **Stage 3 (M6.2c): the pass is wired into production and the three
> declarative readers are deleted** — the authored rule is the single source of
> truth, no longer mirrored by hand. **Stage 4 (M6.2d)** — re-authoring the opaque
> `described()` conventions so they project too — remains. This doc is the full
> design so the effort is resumable.
>
> **Milestone map** (the records split, 2026-06-25):
> - **M6.2b — validate** (done): `Rule::project`, the offline generic pass, the
>   equivalence harness. No production wiring, no deletions, no behavior change.
> - **M6.2c — wire + retire declarative** (done): the keyless leak was a *single*
>   site — `SearchBook::classify` (`search_floor.rs:241`); the floors already get
>   the book's prefixed context and the other keyless `Inferences::read` callers are
>   `#[cfg(test)]`. So `Stance::prefixed_context` is made real, `SearchBook` prefixes
>   itself, and `Inferences::read` folds in `project_authored` — the artificial
>   detector drives both the suppression and the recording. `transfer_major_reading`,
>   `leaping_michaels_reading`, `landy_reading` (and `LandyReading`) are **deleted**;
>   only the Landy advancer-relay survives as the `landy_advance_suppress` stub. Two
>   sound reading shifts fall out: a completed transfer pins its five-card floor (the
>   old six-card jump upgrade drops — a natural-suit raise is outside the projection's
>   artificial-only scope), and Woolsey's `2♣` reads its true 4-5 majors. `ab-landy`
>   reproduces its DD-negative value (the reading is byte-identical per the M6.2b
>   test); `ab-search-floor` shows no gross regression.
> - **M6.2d — Stage 4**: re-author the opaque `described()` conventions
>   (DONT/Woolsey/Multi) as `len` conjuncts (`verify::compare`-guarded), retire the
>   rest; keep relay-suppression as `ponytail:` stubs.

## The idea

"What does an artificial call mean" is authored exactly once — as the
`Constraint` on the `Rule` for the call that gets made
([`rules.rs`](../../src/bidding/rules.rs), `struct Rule { call, weight, when,
label }`). Two consumers re-derive that meaning instead of reading it:

- the **layout sampler** — but it already reads the rule *directly*:
  `rules_accept`/`made_plausibly` re-run `policy.classify` (the rule's `eval`) on
  each candidate hand ([`sampler.rs`](../../src/bidding/sampler.rs)). Not a gap.
- the **forward reader** `Inferences::read` — the meaning the *bidder* uses to
  choose its own call — hand-writes it per-convention in seven `*_reading`
  functions in [`inference.rs`](../../src/bidding/inference.rs):
  `rubens_reading`, `leaping_michaels_reading`, `landy_reading`, `multi_reading`,
  `woolsey_x_reading`, `dont_reading`, `transfer_major_reading` (the M6.1
  inference). **This is the gap.**

The bidder cannot use `eval` (it does not hold partner's hand) — it needs the
*envelope* of partner's possible hands, the forward dual of `eval`. So the whole
idea is one new fold on the constraint DSL that turns a rule into that envelope,
read where the readers are hand-written today. The rule is already inspectable
data in the trie; the sampler's replay already proves "direct rule access"
works; this gives the forward reader the same.

`Constraint` already folds two ways: `eval(hand) -> logit` and
`describe() -> Description`. We add a third: `project(context) -> Inference`.

## Stage 1 — `Constraint::project` (SHIPPED)

A third fold beside `eval` and `describe`
([`constraint.rs`](../../src/bidding/constraint.rs)):

```rust
fn project(&self, _context: &Context<'_>) -> Inference {
    Inference::unknown()   // no-info default: sound, NON-BREAKING
}
```

Returns the existing `Inference { lengths: [Range;4], points: Range }`. The
homomorphism, folded pointwise over the four suit ranges and points:

| Constraint            | `project`                                                         |
| --------------------- | ----------------------------------------------------------------- |
| `len(suit, r)`        | `lengths[suit] = r`, both bounds (length is exact)                |
| `points(r)`           | `points = floor(r)..cap` — **floor only**                         |
| `hcp(lo..=hi)`        | `points = lo..cap` — floor only (ceiling unsound: `pts ≥ hcp`)    |
| `support`, `balanced`, `top_honors`, context preds, … | `unknown()` (default, no override)        |
| `&` / `And`           | pointwise `Range::intersect` (disjoint → widen, the existing rule)|
| `\|` / `Or`           | pointwise `Range::union` (loosest span — soundness over tightness)|
| `!` / `Flip`, opaque `pred`/`described` | `unknown()` (sound, loose)                      |

**Why points/HCP are floor-only.** The `Inference` point axis is the *upgraded*
`point_count` scale. A `points(8..=16)` band accepts a hand whose upgraded count
is 8–16, but `point_count = raw_hcp + upgrade ≥ raw_hcp`, so projecting an upper
bound risks unsoundness when the fuzzy upgrade is off, and `hcp` ceilings are
unsound even when it is on. Floor-only is sound in **both** fuzzy modes and
exactly mirrors every hand-written reader (`Range::at_least(floor, cap)`).

**Why `Or` is union.** A hand satisfying `a | b` need only satisfy one arm, so
the sound envelope is the span of both. Landy's
`(len(♥,5..)&len(♠,4..)) | (len(♥,4..)&len(♠,5..))` projects to `{♥:4+, ♠:4+}` —
exactly the sound 4-4 floor `landy_reading` records.

**Soundness invariant** (executable property test in
[`verify.rs`](../../src/bidding/verify.rs)): for every hand `h` and context `c`,
`eval(h,c)` finite ⟹ `h ∈ project(c)`. Proven by structural induction;
primitives are exact, `&`→intersect and `|`→union preserve containment, opaque
and `!` project everything. The test samples ~32k hands across primitives,
conjunction, the disjoint-suit disjunctions, a negative-inference shape, and the
opaque escape hatch.

Supporting additions: `Inference::intersect`/`union` (pointwise) and
`Range::union` in [`inference.rs`](../../src/bidding/inference.rs).

## Stages 2–4 — retire the readers (DEFERRED, design intact)

### The generic mechanism

A single pass replaces all seven decoders. The authoring rule for any prior call
is recoverable from the trie the context already carries: `CommonPrefixes`
(in `context.prefixes()`) yields `(query[..i], classifier_i)` for each authored
exact node along the auction path; `classifier.as_rules()` downcasts to `&Rules`
([`trie.rs`](../../src/bidding/trie.rs), the same hook the corpus exporter uses).
So, walking the prefixes:

```text
for (prefix, classifier) in context.prefixes().clone():
    i = prefix.len();  made = auction[i]
    rules = classifier.as_rules()?            // floor/closure → skip
    projection = union of rule.project(context) for rules whose call == made
    if artificial(projection, made):          // see detector below
        narrow players[relative_of(len,i)] by projection   // suppress + record
```

**The artificial-call detector falls out of the projection itself:** a call is
artificial iff its projection floors a suit *other than* the one it names — a
Jacoby 2♦ projects hearts (≠ diamonds), a Landy 2♣ projects the majors (≠
clubs), a both-minors 2NT projects the minors (no named suit). A *natural*
opening floors its own strain (1♠ → spades) or no suit (1NT → only points), so
it is left untouched and `apply_opening` still reads it. No per-convention list,
no suppression allowlist: "at an authored artificial node, trust the projection
and skip the natural read."

`Rule::project(context)` mirrors `Rule::describe`. The context-relative
primitives (`support`, `partner_shown_*`) project no-info, so passing the
current context rather than the prefix's is correct for the length/points
primitives that carry the meaning — no per-prefix context or relative-vul juggle.

### Per-reader verdict

| Reader                    | Authored as                              | Verdict |
| ------------------------- | ---------------------------------------- | ------- |
| `transfer_major_reading`  | declarative `len & …`                    | **RETIRE** (cleanest; uncontested/constructive — the validation anchor) |
| `leaping_michaels_reading`| declarative `len & len & points`         | **RETIRE** |
| `landy_reading`           | `(len&len)\|(len&len)`, both-minors `len&len` | **RETIRE core**; keep advancer-relay suppression |
| `rubens_reading`          | overcall declarative; cue/transfer relays| **PARTIAL** — stop double-reading the overcall; keep cue strength + relay suppression |
| `multi_reading`           | Multi/Muiderberg = opaque `described()`  | **RE-AUTHOR** as `len` conjuncts (both shapes incl. the ≤4-minor negative are expressible) → retire |
| `woolsey_x_reading`       | opaque `described()` double-disjunction  | **KEEP thin** — projects to points-floor only either way; retire the suit half (asserts nothing), keep the `2♣`-relay suppression |
| `dont_reading`            | opaque `described()`                     | **MIXED** — re-author both-majors / minor-major / the X's `len(♠,..=3)` negative inference as explicit conjuncts → retire core; keep relay suppression |

The relay-suppression logic ("the advancer names a suit it does not hold") is
genuinely *not* a projection of any single rule and stays as small
`ponytail:`-marked stubs regardless.

### Stage 4 — re-author the opaque conventions

DONT/Woolsey/Multi shapes are authored with the opaque `described(label,
closure)` escape hatch, so they project no info and the detector cannot see them.
Re-author each as `len` conjunctions that expose the sound fact (Muiderberg
`len(major,5..=5) & len(other,..=3)`; the Multi 2♦ `… & len(♣,..=4) & len(♦,..=4)`;
the DONT X `… & len(♠,..=3)`), each guarded by `verify::compare` against the
original closure's accept-set so `eval` behaviour is unchanged.

## The keyless-trie blocker — RESOLVED in M6.2c

Projection needs the trie, and the readers were **keyless by design**. The fear
was that *two* real consumers read without a trie — the search sampler and
`features` — so retirement would be a cross-cutting "give every keyless path trie
access" change. **Wiring it for real showed the leak is a single site.** The floors
([`SearchFloor`](../../src/bidding/search_floor.rs), `NeuralFloor`) are
`Classifier`s that *receive* a context: through the normal book path they already
get the prefixed one ([`book.rs:56`](../../src/bidding/book.rs#L56) attaches
prefixes before `classify_floored` threads it to the floor). The only production
keyless construction that feeds `Inferences::read`/`features` is
[`SearchBook::classify`](../../src/bidding/search_floor.rs#L241), which re-derived a
fresh `Context::new`; every other keyless `Inferences::read` caller is
`#[cfg(test)]`. So the wire is one line — `SearchBook` builds
`self.stance.prefixed_context(...)` instead — no `System`-trait accessor needed.

What shipped:

1. `Stance::prefixed_context` made real (was `#[cfg(test)]`);
2. `SearchBook::classify` prefixes its search context;
3. `Inferences::read` folds in `project_authored` — one prefix walk yields both the
   per-seat recording overlay and a bitset of artificial indices to suppress;
4. `transfer_major_reading`, `leaping_michaels_reading`, `landy_reading` deleted;
   only `landy_advance_suppress` (the advancer relay, which projects nothing) stays.

Payoff is **architectural** (one mechanism replaces the declarative readers; single
source of truth; lets rule-replay stand alone) — **not IMPs**. The two sound reading
shifts (transfer five-card floor, Woolsey `2♣` 4-5) are documented in the unit tests
and the changelog; `ab-landy` reproduces its value, `ab-search-floor` no gross
regression.

## Verification (when resumed)

- **Soundness** — the `project` property test (shipped).
- **Range-equivalence** per retired reader — run old `read` vs the generic pass
  on the convention auctions, assert per-player `Inference` equal (or new ⊆ old).
  The keyless convention unit tests in `inference.rs` already assert the exact
  ranges and are the oracle — but note they are keyless, so the equivalence
  harness must build *prefixed* contexts (via `Stance` classification).
- **Re-author guard** — `verify::compare` the `described()` closure against its
  `len`-conjunct replacement; must agree on every sampled hand.
- **IMPs/board A/B** — `examples/ab-landy` for the deterministic forward-reader
  path; `examples/ab-search-floor --features search` for the sampler path
  (`probe-replay-yield` to confirm no starvation), per
  [`../../HANDOFF-rule-replay-ab.md`](../../HANDOFF-rule-replay-ab.md).

## Files

- [`constraint.rs`](../../src/bidding/constraint.rs) — `project` on the trait +
  `Len`/`Points`/`Hcp` + the `And`/`Or`/`Flip`/`Cons` folds (shipped)
- [`inference.rs`](../../src/bidding/inference.rs) — `Inference::intersect`/`union`,
  `Range::union` (shipped); the generic `authored_reading` pass (deferred)
- [`rules.rs`](../../src/bidding/rules.rs) — `Rule::project` (deferred)
- [`american/defense.rs`](../../src/bidding/american/defense.rs) — re-author the
  `described()` shapes (deferred, Stage 4)
- [`verify.rs`](../../src/bidding/verify.rs) — soundness test (shipped) +
  re-author guards (deferred)
