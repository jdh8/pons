# Component A: the description / tag language model

The half that connects *words* ("15–17 balanced", "natural game-forcing") to the
machine. Decision taken: **both roles, sequenced** — an authoring *compiler*
first, a runtime *encoder* later. They share the corpus from
[§1d of foundations](01-foundations.md#1d-the-description-corpus-component-as-prerequisite)
but are otherwise independent and can be tackled in either order once the corpus
exists.

---

## Role 1 — The authoring compiler (near-term leverage)

**What:** a function `English meaning → Constraint`. You write the *meaning* of a
call in words (the way a convention card or system notes do), and it produces the
`Constraint` (or the `Rules` entry) that the books currently require you to write
by hand in Rust.

**Why this first:** it attacks the actual bottleneck. Porting the
[Strawberry Polish Club](https://polish.club/) is on the roadmap; today each node
is hand-coded Rust. If authoring becomes "write the meaning in English", the port
goes at the speed of describing the system, and the descriptions *are* the corpus
Component A's runtime half needs. Two birds.

**Why it does not need a trained-from-scratch model:** the target language — the
`Constraint` DSL — is small and closed (`hcp`, `points`, `fifths`, `len`,
`support`, `balanced`, `stopper_in_their_suits`, `&`/`|`/`!`, …). This is a
*semantic parsing* problem (text → a small formal language), and a capable
general LLM (e.g. Claude) prompted with the DSL grammar and a handful of worked
examples does it well today. So Role 1 is, pragmatically:

1. Write a precise spec of the `Constraint` DSL as a prompt (grammar + the
   vocabulary table from `constraint.rs` + 10–20 gold `(English, Rust)` pairs
   harvested from existing rules). **Done (M4.1):** [`dsl-spec.md`](dsl-spec.md),
   verified by `tests/dsl_roundtrip.rs` (12/12 held-out rules round-trip exactly).
2. Compile each described node through the LLM into a candidate `Constraint`.
3. **Verify mechanically** — this is the part that makes it trustworthy and is
   squarely your domain:
   - the candidate compiles;
   - on a large random sample of hands, the candidate's accept/reject set matches
     the human's *intent* (compare against hand-labeled examples, or against the
     existing rule when porting);
   - soundness checks against `Inferences` (a "5+ hearts" description must not
     compile to a constraint that accepts 4-card holdings).

The model proposes; deterministic Rust verification disposes. An LLM
mis-compilation becomes a failing test, not a silent bidding bug.

**Deliverable shape:** an offline tool (a `dev-dependency`, an `xtask`, or just a
documented prompt + a verification example) — not a crate dependency. It emits
Rust `Constraint` source you review and commit, exactly as you would hand-written
rules. Nothing learned ships in `pons`.

> Note the symmetry with what already exists: `Rules::explain()` goes
> `Constraint → English-ish` (it names the winning rule). The compiler is the
> inverse, `English → Constraint`. Building both makes the corpus
> round-trippable, which is the strongest possible consistency check.

---

## Role 2 — The runtime meaning-encoder (the portability dream)

**What:** at classification time, the policy net reads not just the raw call
tokens but the *meaning* of each prior call, as a learned vector. "Read tags and
descriptions from each call", literally.

**Why it is the dream:** today the policy net (Component B, Phase 1) is trained
on *one* system — its weights *are* 2/1. To bid Polish Club it must be retrained.
If instead the system enters the model only as **text descriptions of each
call**, then one trained net bids *any* system: feed it the Polish Club node
descriptions and it plays Polish Club, feed it Precision's and it plays Precision
— no retraining. The net learns "given what each bid *means* and my hand, what
should I do", a system-agnostic skill, instead of memorizing one system's table.

**How embeddings work (in your terms):** an embedding is a lookup table from a
discrete token to a trainable vector in `ℝᵈ` — structurally the same as
`encode_call` mapping a `Call` to an array index, except the cell holds a learned
`d`-vector rather than a count. A description like "natural game-forcing, 5+
hearts" is tokenized into words/subwords, each word indexes the table, and the
word-vectors are combined (a small transformer / attention layer — a learned,
differentiable weighted average) into one *meaning vector* for that call. The
auction becomes a sequence of meaning vectors, which the sequence-model body of
Component B (Phase 2) consumes alongside the hand features.

**Where it plugs in:** it is an *input* to Component B's Phase-2 sequence model,
not a separate runtime. Component A (encoder) and Component B (policy) are one
network with two input streams: hand features, and per-call meaning vectors.

**Honesty about difficulty:** this is the most ambitious piece and rightly comes
last. It needs (a) the corpus at real scale and quality, (b) Component B already
on a sequence architecture, and (c) training data spanning *more than one system*
to actually teach portability — a net trained only on 2/1 descriptions will lean
on 2/1 regularities even if you feed it Polish Club text. The Polish Club port
(via Role 1) is therefore a *prerequisite*: it produces the second system's
corpus that makes "portability" measurable rather than aspirational.

**Do we even need a bespoke "domain-specific language model"?** Probably not a
large one. The vocabulary of bridge meanings is tiny and regular. Options, cheap
to expensive:
- **Structured tags only** (no free text): embed the discrete `tags` field
  (`transfer`, `takeout`, `game-forcing`, `5+♥`, …) as categorical features. This
  captures most of the signal with none of the NLP and is the right first cut —
  it is "read the *tags* from each call", which may be all that is needed.
- **Small learned text encoder** trained jointly with the policy on the corpus.
- **Frozen embeddings from a general model**, used as fixed features.

Start with structured tags; reach for text only if free-form descriptions carry
signal the tags miss.

---

## Sequencing within Component A

1. **Corpus** (shared prerequisite; a foundations milestone): per-node
   `{auction, call, tags, description, constraint}`, bootstrapped from
   `explain()` + doc comments.
2. **Compiler** (Role 1): English → `Constraint`, LLM-proposed + Rust-verified,
   used to accelerate the Polish Club port. Offline, ships nothing.
3. **Tag features** (Role 2, first cut): discrete tags as categorical inputs to
   Component B.
4. **Meaning encoder** (Role 2, full): text descriptions → meaning vectors, joint
   with a sequence-model policy, trained across ≥ 2 systems to earn portability.

Steps 1–2 are tractable now and high-leverage. Steps 3–4 depend on Component B
reaching its sequence-model phase and on a second system's corpus existing.

---

Next: [`04-integration-and-eval.md`](04-integration-and-eval.md).
</content>
