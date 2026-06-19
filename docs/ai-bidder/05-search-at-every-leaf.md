# Search at every leaf

> *Rules propose, DD disposes — at every leaf, not just off-book.*

This is the design for **Milestone 7**. It generalizes the one trick that the
Leaping Michaels work proved (`21gf-ledger.md`): an authored bid should encode
*meaning*, and a double-dummy search should make the *judgement*. Today that only
happens where the book is silent. M7 makes it happen on authored leaves too.

Read [`01-foundations.md`](01-foundations.md) §0 first — the safety invariants
here are inherited verbatim, not re-derived.

---

## 0. The gap

A bid in `pons` is produced by walking the [`Trie`](../../src/bidding/trie.rs) to
the most specific authored node for the auction and asking its `Classifier` for
[`Logits`](../../src/bidding/array.rs). For authored conventions that node is a
[`Rules`](../../src/bidding/rules.rs) ladder: each rule is a `(Call, weight,
Constraint)`, and the logit of a call is `max(weight + constraint.eval(hand))`
over the rules. That is an **inflexible rule** — the weights are authored once and
fixed; the same shape always picks the same call, regardless of where the
specific 13 cards sit.

The double-dummy search bidder
([`SearchFloor`](../../src/bidding/search_floor.rs)) already exists and is *not*
inflexible — it prices candidate calls by cardplay EV over sampled layouts. But
it is wired in exactly one place:

```text
Trie::classify_floored(hand, ctx, auction):     # trie.rs:286
    node = resolve(ctx, auction)                # the most specific authored node
    if node.classify(hand).has_mass():          # any finite logit?
        return node logits          ──────────► TERMINAL. search never runs here.
    return fallback-chain logits    ──────────► the floor. SearchFloor lives here,
                                                 contested books only (american.rs:330).
```

So DD search runs **only where the book has nothing to say**. An authored leaf
that admits the hand is the final word. Leaping Michaels got around this *by
hand*: it capped the authored advance at game, deliberately leaving the slam zone
unauthored, so the auction fell through to the contested floor where `SearchFloor`
priced `4M/5m/slam` — and it added `leaping_michaels_reading` to
[`inference.rs`](../../src/bidding/inference.rs) so the sampler knew the two-suiter
and the EVs were sound. A directional A/B measured **+2.8 IMPs/board** for search
on top of the rule floor, reaching slams the game-capped rules cannot.

M7 turns that hand-done trick into the default shape of the system.

---

## 1. Principle

An authored constraint says *what a call shows* — `len(Suit::Spades, 5..) &
hcp(11..=21)` is the meaning of a `1♠` overcall. That is the right job for a rule:
it is the bidding *agreement*, the thing partner relies on, and it is true
regardless of the deal.

An authored *weight* says *which call to pick* among the legal ones. That is a
**judgement**, and judgement is exactly where specific cards change the answer:
which game, slam or not, which strain, sacrifice or defend. A fixed weight cannot
see the cards. A double-dummy search can.

> **Meaning stays authored. Judgement moves to DD.**

This is the same "net proposes, search disposes" stance the search floor already
takes ([`search_floor.rs:9`](../../src/bidding/search_floor.rs)), widened: the
proposer is no longer only the net — it is the **authored leaf**, optionally
unioned with the net. The leaf's logits become *a prior*, not a verdict.

---

## 2. Mechanism — swap the prior source, reuse the seam

`SearchFloor`'s body is already prior-agnostic. Strip the prior and it is three
reusable steps over any `Logits`:

```text
shortlist(prior, k)          # search_floor.rs:161 — top-k legal calls by prior
  → ev_all(...)              # ev.rs:87 — price each over sample_layouts (sampler.rs:58)
  → blend(prior, evs, temp)  # search_floor.rs:180 — re-seat the best above the prior tail
```

Today the prior is `neural::classify(&feats)`
([`search_floor.rs:127`](../../src/bidding/search_floor.rs)). For a book leaf the
prior is **the resolved book logits** — the very thing `classify_floored` is about
to return. Nothing in `shortlist`/`ev_all`/`blend` needs to change.

**Candidate set: book finite calls ∪ neural top-k.** The book leaf proposes the
conventional calls (with their authored ordering); the net contributes natural
alternatives the rule never listed. DD prices the union and picks. This matters:
the whole point is to let DD *override an inflexible rule*, which it cannot do if
the rule's calls are the only candidates. (A pure-book candidate set degrades to
"DD re-ranks what the rule already allowed" — still useful, but it cannot escape a
rule that authored only one call.)

**Where it attaches.** A search-aware classification path: when `classify_floored`
resolves a book leaf with mass and the situation is not forced, feed those logits
(∪ net) through the three steps instead of returning them raw. Cleanest as a
`Pair`/`Table`-level wrapper or a `Trie` method variant, exposed as a new gated
constructor (e.g. `american_search_book()`) alongside `american_search()`.
`instinct()` and `american()` are untouched; the new bidder is opt-in behind the
`search` feature.

---

## 3. The soundness gate (the linchpin)

DD EV is only as good as the layouts it is averaged over, and the layouts are only
as good as [`Inferences::read`](../../src/bidding/inference.rs) — the decoder that
turns the auction into per-player range constraints the sampler
([`sample_layouts`](../../src/bidding/sampler.rs)) conditions on.

If a convention is **undecoded**, `read` leaves the bidder's partner range wide.
The sampler then deals partners who need *not* hold the shape the convention
promised, so the rollout prices a call against a partner who, in real play, would
never have made the auction. The EV is biased.

Crucially this **never crashes and never produces an illegal or unsound result**:
inferences are sound-by-construction (supersets — "soundness over tightness", see
`inference.rs`), so a missing decode only *widens* ranges. The failure mode is
*quietly weak EV*, not breakage. That makes the gate easy to state and easy to get
wrong:

> A leaf is only as DD-empowered as its `Inferences::read` decode. An undecoded
> conventional leaf gets garbage-flavored EV that still *looks* like a number.

Therefore **decoding every authored convention (M7.1) gates the *quality* of
wrapping every leaf (M7.0/M7.2)**, even though it does not gate correctness.
Explicit fallback: a leaf with no usable decode keeps its authored logits — i.e.
detect "this auction carries an undecoded convention" and skip the search there,
rather than ship a confidently-wrong EV. (Detecting that is itself work; the
conservative default is to wrap leaves only as their decode lands, convention by
convention, each gated by its own A/B.)

---

## 4. Continuation fidelity (flagged, not solved here)

The rollout finishes each sampled auction with `POLICY` — the bare distilled net
bound for self-play ([`search_floor.rs:76`](../../src/bidding/search_floor.rs)) —
*not* the book+floor system being measured. Pricing a **book** leaf assuming the
**net** continues is a mismatch: Leaping Michaels needed both the reading *and* a
net that understood the convention before the EVs tracked reality.

Two options, deferred to measurement:

- **(a) Accept it.** The net is distilled from book+floor, so it roughly tracks
  the real continuation; the bias may be small relative to the judgement gain.
- **(b) Make the continuation policy the full system.** Use book+floor for the
  rollout continuation instead of the bare net. More faithful, more expensive, and
  it reintroduces the very recursion the net was distilled to avoid — so only if
  (a) measurably leaves EV on the table.

This is M7.3, optional, taken only if M7.0/M7.2 show residual bias.

---

## 5. Safety invariants (inherited verbatim)

From `01-foundations.md` §0 — M7 changes the *prior source*, nothing else:

- **Forced rails first.** `forced(context)` short-circuits to deterministic
  `instinct()` *before* any search ([`search_floor.rs:120`](../../src/bidding/search_floor.rs)).
  The net is never trusted on the rails; neither is the search; neither is a book
  leaf's judgement. Wrapping leaves does not touch this.
- **Legality.** The mask is unchanged; illegal calls stay `-∞` and the shortlist
  never lifts one.
- **`Pass` stays finite** and every legal call keeps a sane fallback logit
  (`blend`'s EV band sits *above* the prior tail, it does not erase it).
- **Determinism.** Same decision → same layouts via `seed_from_features`
  ([`search_floor.rs:229`](../../src/bidding/search_floor.rs)); `classify` stays a
  pure function (§0.5).
- **`instinct()` stays the baseline and default.** Every learned/searched bidder
  is an added, gated option, never a removal (the standing ai-bidder decision).

---

## 6. Measurement

The same yardstick as every milestone: IMPs/board on the A/B duplicate match,
**perfect-defense** default (failing contracts priced doubled). Reuse the
[`search-floor`](../../examples/search-floor/) harness for contested and the
[`constructive-abc`](../../examples/constructive-abc/) A/B/C for constructive. Add
a per-convention A/B as each leaf is decoded and wrapped, so the gain is
attributed to the convention, not lost in the aggregate.

The two boundaries this milestone is *re-testing*, not assuming:

- **Constructive.** `project_floors_contested_only` measured that putting the raw
  *net* on the constructive book lost 0.8 IMPs/board. DD-pricing the *authored
  book candidates* is a different experiment (the candidate set is the convention's
  own calls, not the net's free judgement), so the boundary is re-opened — and
  must clear its own A/B before constructive leaves are wrapped by default.
- **Obstruction blindness.** `project_preemption-dd-negative`: the DD /
  perfect-defense harness is blind to obstruction and concealment. M7's gains
  should show up in *constructive reach* (better games/slams), not in light
  competitive judgement, where DD cannot see the value. Expect the wins where
  cardplay, not opponents' confusion, decides the board.

---

## 7. Milestones

See [`plan.md`](plan.md) M7 for the chunked, dependency-ordered version. In short:
**M7.0** wires the search-aware path (parity-or-better vs `american_search` on
contested, rails green); **M7.1** sweeps `Inferences::read` to decode every
authored convention (gates M7.0 quality); **M7.2** extends to constructive leaves
(re-tests the boundary above); **M7.3** (optional) upgrades the rollout
continuation policy if EV bias remains.
