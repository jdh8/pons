# Rule projection — reading an authored call straight off its rule

> Status (2026-06-25): **Stage 1 shipped** (`Constraint::project`). **Stage 2
> validated** (M6.2b): `Rule::project` + the generic `authored_reading` pass exist,
> `#[cfg(test)]`-only, and an equivalence test proves the pass reproduces the three
> declarative readers (`transfer_major`, `leaping_michaels`, `landy` core) exactly —
> signature suit lengths *and* points — on prefixed contexts. **Stages 3–4** (the
> keyless-trie-access wiring, then retiring the readers + re-authoring the opaque
> conventions) **deferred** — an IMP-neutral architectural refactor. This doc is the
> full design so the effort is resumable.
>
> **Milestone map** (the records split, 2026-06-25):
> - **M6.2b — validate** (done): `Rule::project`, the offline generic pass, the
>   equivalence harness. No production wiring, no deletions, no behavior change.
> - **M6.2c — wire + retire declarative**: a `Stance` `CommonPrefixes` accessor
>   (the `#[cfg(test)] Stance::prefixed_context` seam, made real), prefix the
>   keyless sampler/features call sites, switch `Inferences::read` onto the pass,
>   delete the three clean declarative readers. A/B-gate `ab-search-floor` +
>   `ab-landy` neutral-or-better.
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

## Why it is deferred — the keyless-trie blocker

Projection needs the trie. The readers are **keyless by design** because two of
the three real consumers read *without* one:

- the **search floor's sampler** builds its context as `Context::new(vul,
  auction)` — **no prefixes** ([`search_floor.rs:241`](../../src/bidding/search_floor.rs#L241)) —
  then `ev_all → Inferences::read` runs with no trie to project from;
- **features** (neural) likewise build keyless contexts;
- only the book's own classification prefixes the context
  ([`book.rs:56`](../../src/bidding/book.rs#L56)), so only the floor's re-entrant
  constraint-eval reads (`partner_shown_len`, `has_fit`) can project.

So retiring the readers regresses the sampler and features (and the keyless
convention unit tests go red with nothing to replace them). **Full retirement is
therefore "give the keyless sampler + features paths trie access," not "add a
fold."** That requires:

1. a `System` accessor exposing the phase-routed trie's `CommonPrefixes` for an
   auction (default `None`; `Stance` overrides via `trie_for`);
2. prefixing the contexts at `search_floor.rs:241`/`:411` and the `features`
   call sites from the active book;
3. then the generic pass + the re-authoring above.

Payoff is **architectural** (one mechanism replaces seven readers; single source
of truth; lets rule-replay stand alone) — **not IMPs** (it is a refactor; gate
is neutral-or-better). Cross-cutting, with real risk to the search floor's
sampling. Banked Stage 1; deferred the rest pending a decision that the
architectural cleanup is worth a multi-day, A/B-gated change.

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
