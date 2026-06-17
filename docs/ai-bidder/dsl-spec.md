# The `Constraint` DSL spec — English → Rust authoring compiler

> **What this is.** A precise, self-contained spec for compiling the *meaning* of
> a bridge call (English) into a `Constraint` (Rust), the predicate the
> [`pons`](../../README.md) books bid on. It is written to be pasted as an LLM
> prompt: give the model this document plus an English gloss, and it returns one
> Rust `Constraint` expression. This is **Component A, Role 1** of the
> [AI-bidder plan](plan.md) (milestone **M4.1**); see
> [`03-description-lm.md`](03-description-lm.md) for the role in context.
>
> **It ships nothing.** The compiler is an offline authoring aid: an LLM proposes
> a `Constraint`, and deterministic Rust *verifies* it (this milestone's
> round-trip check; the behavioral verifier is M4.2). You review and commit the
> emitted Rust exactly as you would a hand-written rule. Nothing learned enters
> the crate.

---

## 1. The one idea that makes this verifiable

The DSL **renders its own meaning**. Every `Constraint` has a
[`describe()`](../../src/bidding/constraint.rs) method (milestone M4.0) that
returns canonical English:

```rust
(points(12..=21) & len(Suit::Spades, 5..)).describe().to_string()
// => "12–21 points, and 5+ ♠"
```

So the compiler is the **inverse of a function we already have**, and correctness
is a string compare:

> A compilation of gloss `G` is correct when `compiled.describe().to_string() == G`.

The English in the corpus *is* `describe()`'s output. Your target is therefore
not "some English a human might write" but the **exact canonical form** below.
(For looser human notes, normalize to the canonical primitives; see §7.)

---

## 2. Grammar

A `Constraint` is a tree. Leaves are *primitives* (§3); interior nodes are the
three combinators:

```text
constraint := primitive
            | constraint "&" constraint        // conjunction  (All)
            | constraint "|" constraint        // disjunction  (Any)
            | "!" constraint                   // negation     (Not)
            | "(" constraint ")"
            | described( "<label>", |hand, ctx| <bool expr> )   // escape hatch (§5)
```

**Rust operator precedence** (this governs what you must parenthesize):
`!` binds tightest, then `&`, then `|`. So `a & b | c` parses as `(a & b) | c`,
and to build an `Any` *inside* an `All` you must write `a & (b | c)`.

**How the tree renders** (`Display` on `Description`, pinned in
[`constraint.rs`](../../src/bidding/constraint.rs) ~lines 187–280):

| Tree | English |
|---|---|
| `A & B` | `A, and B` |
| `A & B & C` | `A, B, and C` (flattened — one comma list, not nested) |
| `A \| B` | `A, or B` |
| `!A` | `not (A)` |
| `!!A` | `A` (double negation cancels) |
| `(P \| Q) & Z` | `(P, or Q), and Z` (a nested `Any`/`All` member is parenthesized) |

Reading a gloss back to a tree:

- A top-level comma list ending in `…, and X` is a conjunction (`&`); ending in
  `…, or X` is a disjunction (`|`).
- `not (X)` is `!X`.
- A parenthesized comma list is a sub-tree (use Rust parentheses to match it).
- Every remaining comma-separated item is one **atom** — match it to a primitive
  in §3 by its phrase, or, failing that, to a `described(...)` escape hatch (§5).

---

## 3. Vocabulary (the complete primitive set)

Every constructor is `pub` in
[`pons::bidding::constraint`](../../src/bidding/constraint.rs). All take `Clone +
Send + Sync` range/value args and return a composable `Constraint`. The **Gloss**
column is the exact `describe()` output; `{R}` is the range rendered per §4.

### Strength

| Rust | Meaning | Gloss | Example → gloss |
|---|---|---|---|
| `hcp(range: u8)` | raw high-card points | `{R} HCP` | `hcp(15..=17)` → `15–17 HCP` |
| `points(range: u8)` | HCP + shape upgrade (suit-oriented strength) | `{R} points` | `points(12..=21)` → `12–21 points` |
| `fifths(range: f64)` | Andrews Fifths, 40-pt scale (notrump-defining strength) | `{Rf} fifths` | `fifths(15.0..18.0)` → `15.0–18.0 fifths` |
| `cccc_at_least(points: f64)` | Kaplan–Rubens CCCC floor | `CCCC ≥ {n}` | `cccc_at_least(14.9)` → `CCCC ≥ 14.9` |

### Shape

| Rust | Meaning | Gloss | Example → gloss |
|---|---|---|---|
| `len(suit: Suit, range: usize)` | length of a suit | `{R} {suit}` | `len(Suit::Spades, 5..)` → `5+ ♠` |
| `balanced()` | 4333, 4432, or 5332 | `balanced` | `balanced()` → `balanced` |

### Suit quality

| Rust | Meaning | Gloss | Example → gloss |
|---|---|---|---|
| `top_honors(suit: Suit, range: usize)` | count of A/K/Q in the suit | `{R} of the top honors in {suit}` | `top_honors(Suit::Clubs, 2..)` → `2+ of the top honors in ♣` |
| `stopper_in(suit: Suit)` | A, Kx, Qxx, or Jxxx | `stopper in {suit}` | `stopper_in(Suit::Hearts)` → `stopper in ♥` |
| `stopper_in_their_suits()` | a stopper in every suit they bid | `stopper in their suit(s)` | — |

### Partnership

| Rust | Meaning | Gloss | Example → gloss |
|---|---|---|---|
| `support(range: usize)` | our length in partner's last suit | `{R} card support for partner` | `support(3..)` → `3+ card support for partner` |
| `partner_suit_is(suit: Suit)` | which suit partner bid last | `partner's last suit is {suit}` | `partner_suit_is(Suit::Hearts)` → `partner's last suit is ♥` |
| `partner_shown_len(suit: Suit, range: u8)` | length partner has *promised* (from `Inferences`) | `{R} {suit} shown by partner` | `partner_shown_len(Suit::Diamonds, 3..)` → `3+ ♦ shown by partner` |
| `partner_shown_points(range: u8)` | points partner has *promised* | `{R} points shown by partner` | `partner_shown_points(12..)` → `12+ points shown by partner` |

### Auction state

| Rust | Meaning | Gloss | Example → gloss |
|---|---|---|---|
| `they_bid(strain: Strain)` | the opponents have bid the strain | `opponents bid {strain}` | `they_bid(Strain::Spades)` → `opponents bid ♠` |
| `short_in_their_suits()` | ≤3 cards in each suit they bid (takeout shape) | `at most three cards in each of their suits` | — |
| `min_level_is(level: u8, strain: Strain)` | the strain's cheapest legal bid is exactly this level | `{level}{strain} is the cheapest bid` | `min_level_is(2, Strain::Diamonds)` → `2♦ is the cheapest bid` |
| `passed_hand()` | the actor passed on their first turn | `a passed hand` | — |
| `undisturbed()` | the opponents have only passed | `the opponents have passed throughout` | — |
| `nth_seat(seat: u8)` | about to open in this seat (1–4) | `opening in seat {n}` | `nth_seat(3)` → `opening in seat 3` |

### Vulnerability

| Rust | Meaning | Gloss | Example → gloss |
|---|---|---|---|
| `vulnerable()` | our side is vulnerable | `vulnerable` | — |
| `they_vulnerable()` | the opponents are vulnerable | `opponents vulnerable` | — |

---

## 4. Range conventions

Integer primitives (`hcp`, `points`, `len`, `support`, `top_honors`,
`partner_shown_len`, `partner_shown_points`) take any Rust `RangeBounds` and
normalize to inclusive integers before rendering with the primitive's noun `N`:

| Rust range | Gloss | Note |
|---|---|---|
| `a..=b` | `a–b N` | inclusive band |
| `a..b` | `a–{b−1} N` | exclusive upper normalized down |
| `a..` | `a+ N` | open above (`0..` → `0+ N`, *not* "any") |
| `..=b` | `≤b N` | open below |
| `..b` | `≤{b−1} N` | exclusive upper normalized down |
| `a..=a` (or `a..{a+1}`) | `exactly a N` | a single value |
| `..` | `any N` | fully unbounded (rare) |

So a gloss has more than one valid spelling: `≤11 points` compiles equally to
`points(..=11)` or `points(..12)`; both round-trip. Pick whichever reads best;
the verifier accepts either.

`fifths` takes an `f64` range and renders endpoints **literally** to one decimal
(`{Rf}`): `15.0..18.0` → `15.0–18.0 fifths`, `22.0..` → `22.0+ fifths`. Bands are
conventionally written half-open (`15.0..18.0`) so adjacent bands tile.
`cccc_at_least` takes a single `f64` printed with default formatting (`14.9` →
`14.9`).

Suits render as `♠ ♥ ♦ ♣` (`Suit::Spades`, `Suit::Hearts`, `Suit::Diamonds`,
`Suit::Clubs`); strains add `NT` (`Strain::Notrump`).

---

## 5. The escape hatch: `described`

When a meaning has **no primitive** — better-minor selection, "5–5 in the two
lowest unbid suits", an RKCB keycard count, "longer hearts than spades" — emit a
labeled bespoke predicate:

```rust
described("prefers diamonds", |hand: Hand, _ctx: &Context<'_>| {
    hand[Suit::Diamonds].len() >= hand[Suit::Clubs].len()
})
```

- `described(label, closure)` renders to its **label, verbatim** — so reproduce
  the gloss phrase exactly as the label, and the round-trip closes.
- The **closure body** is your best effort at the predicate. It does *not* appear
  in the gloss, so the round-trip check cannot verify it — the behavioral verifier
  (M4.2) checks accept/reject behavior over random hands. When unsure, write the
  obvious implementation and flag it for review.
- **Never emit bare `pred(closure)`** (unlabeled). It renders `(opaque
  condition)` and throws away the meaning. Always `described`.

Real labels in the books, for reference: `"prefers diamonds"`, `"exactly 2
keycards"` / `"1+ keycards"` (RKCB), `"kings outside trumps"`, `"holds the ♠
queen"`, `"♥ at least as long as ♠"`, `"our side bid ♥"`.

---

## 6. Gold examples (gloss → Rust)

Harvested from the live 2/1 books (`cargo run --example render-book`). These span
the vocabulary; study the mapping, then compile new glosses the same way.

```rust
// ── strength + shape ────────────────────────────────────────────────────────
"22+ points"
    => points(22..)                                              // strong 2♣
"15.0–18.0 fifths, and balanced"
    => fifths(15.0..18.0) & balanced()                           // strong 1NT
"12–21 points, and 5+ ♠"
    => points(12..=21) & len(Suit::Spades, 5..)                  // 1♠ opening
"exactly 6 ♥, 5–10 points, and not (opening in seat 4)"
    => len(Suit::Hearts, 6..=6) & points(5..=10) & !nth_seat(4)  // weak 2♥
"≤11 points"
    => points(..=11)                                             // opener's pass

// ── nested mixed tree (note the Rust parentheses for the `|` group) ──────────
"9–11 points, 5+ ♠, and (opening in seat 3, or opening in seat 4)"
    => points(9..=11) & len(Suit::Spades, 5..) & (nth_seat(3) | nth_seat(4))

// ── responses / quality / partnership ───────────────────────────────────────
"4+ card support for partner"
    => support(4..)                                              // game raise
"5+ ♣, 2+ of the top honors in ♣, and 14+ points"
    => len(Suit::Clubs, 5..) & top_honors(Suit::Clubs, 2..) & points(14..)
"3+ ♦ shown by partner"
    => partner_shown_len(Suit::Diamonds, 3..)

// ── defense ──────────────────────────────────────────────────────────────────
"12+ HCP, and at most three cards in each of their suits"
    => hcp(12..) & short_in_their_suits()                        // takeout double
"15–18 HCP, balanced, and stopper in their suit(s)"
    => hcp(15..=18) & balanced() & stopper_in_their_suits()      // 1NT overcall
"2♦ is the cheapest bid, 5+ ♦, and 8+ points"
    => min_level_is(2, Strain::Diamonds) & len(Suit::Diamonds, 5..) & points(8..)

// ── escape hatch (label round-trips; closure is best-effort) ─────────────────
"12–21 points, prefers diamonds, ≤4 ♥, and ≤4 ♠"
    => points(12..=21)
        & described("prefers diamonds", |hand: Hand, _: &Context<'_>| {
            hand[Suit::Diamonds].len() >= hand[Suit::Clubs].len()
        })
        & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5)        // 1♦ opening
"exactly 2 keycards"
    => described("exactly 2 keycards", |hand: Hand, _: &Context<'_>| {
        count_keycards(hand /* , trump */) == 2  // best-effort; M4.2 verifies
    })
```

---

## 7. From human notes

Real system notes are looser than `describe()` prose. **Normalize to the
canonical primitives**: pick the noun the strength is expressed in (`points`,
`fifths`, or `HCP`), and write the constraint whose `describe()` is canonical. The
round-trip then checks your output against that canonical form.

```text
"15 to 17, balanced"                 => hcp(15..=17) & balanced()
                                        // renders "15–17 HCP, and balanced"
"game-forcing, at least five hearts" => points(13..) & len(Suit::Hearts, 5..)
                                        // renders "13+ points, and 5+ ♥"
"takeout of their suit, opening hand"=> hcp(12..) & short_in_their_suits()
```

(`fifths`/`points` vs `hcp` is a judgement call the gloss usually settles for you;
when authoring from notes, prefer `fifths` for notrump ranges, `points` for
suit-oriented strength, `hcp` where shape is irrelevant — matching how the 2/1
books choose.)

---

## 8. Compiler instructions (the task)

Given one English gloss, output **exactly one Rust `Constraint` expression** (the
`when` argument of a `rule(...)`) and nothing else:

1. Parse the gloss into a tree per §2: split the top-level comma list; map the
   final `, and …` / `, or …` to `&` / `|`; map `not (…)` to `!`; recurse into
   parenthesized sub-lists (and parenthesize them in Rust).
2. Map each atom to a primitive in §3 by its phrase; recover the range from the
   gloss per §4 (`5+`→`5..`, `≤4`→`..=4`, `12–21`→`12..=21`, `exactly 6`→`6..=6`,
   `8+`→`8..`).
3. If an atom matches **no** primitive phrase, emit `described("<the exact
   phrase>", |hand, ctx| { /* implement */ })` (§5).
4. Map suits/strains to `Suit::*` / `Strain::*`.
5. Self-check: mentally render your expression's `describe()` and confirm it
   equals the input gloss (modulo the range-spelling freedom of §4).

**Verification.** `tests/dsl_roundtrip.rs` runs this round-trip mechanically over
a held-out set of real rules and pins every primitive's gloss in §3 against
`describe()`. The behavioral check (does the compiled constraint accept/reject the
right hands?) is milestone **M4.2**.

---

## 9. Held-out validation (the M4.1 measure)

To prove this spec is sufficient, a set of real book rules **not used as gold
examples above** were compiled from their `describe()` gloss alone (this document,
no peeking at the original source) and checked for exact round-trip. The held-out
set and the per-primitive vocabulary coverage live in
[`tests/dsl_roundtrip.rs`](../../tests/dsl_roundtrip.rs) and run under `cargo test
--all-features`.

**Result:** all 12 held-out rules reproduced exactly (round-trip = identity), and
all 21 primitive glosses in §3 are pinned against `describe()`. The single
recurring ambiguity is range spelling (§4), where multiple Rust spellings render
the same gloss and the verifier accepts any.

*Honesty:* the same model authored this spec and acted as the compiler, so the
measure proves **sufficiency** (the spec contains enough to deterministically
recover round-tripping Rust) and **guards against `describe()` drift** — not
adversarial generalization, which the behavioral verifier (M4.2) tests.
