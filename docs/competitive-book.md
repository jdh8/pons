# The competitive-book campaign

Filling the competitive book — auctions where **we open and the opponents come
in**. Read [bidding-architecture.md](bidding-architecture.md) first; every
package here ships (or doesn't) by [measurement.md](measurement.md).

## Why this campaign exists

The rendered book (text and web `#book`) showed an **empty competitive
section** while constructive and defensive were rich. Two causes:

1. **A disclosure artifact.** The competitive book was never empty — ~3,000
   lines of authored competition (cue-raises, negative doubles, support
   doubles, Lebensohl/Transfer-Lebensohl, UvU, the contested-Stayman/transfer
   packages) attach as **guarded fallbacks** (`fallback_all_seats`), and both
   renderers walked only exact trie nodes. Fixed by Workstream 0 below.
2. **Genuine coverage gaps**, tracked in the ledger: their two-suiters over
   our 1M, our contested weak twos and 2♣, overcall responses above 2♠ plus
   free bids, and their takeout double of our 1x (today a bare systems-on
   rebase).

## Wiring idiom (applies to every package)

- **Deeper-key guarded fallbacks** — the Section-5 stolen-Stayman idiom: key
  responder tables at `[open, their_concrete_call]` with `SuffixIs(vec![])`,
  opener continuations at the same key with exact `SuffixIs` suffixes. A
  deeper node's fallback beats every shallower entry (`resolve_at` walks
  deepest-up), so there are no declaration-order races with
  `OvercallAtMost(2♠)` or a `FirstIs(X)` rebase — and the rebase survives as
  the catch-all for every suffix the new guards don't claim.
- **Guarded tables cannot reject-to-floor.** `classify_floored`'s single
  fall-through skips only the *exact-node* classifier; a guarded table with no
  mass returns degenerate logits. Every guarded table is **total** (finite
  catch-all); "everything else systems-on" is guard scope, never rejection.
- **Prefer `len(suit, n..)` over `support(n..)`** in new competitive rules:
  `support` projects nothing, `len` projects tight, and an alerted call
  decodes by its own rule's projection (fallback projection is default-on).
- **Renderability is an invariant.** Every guard and rebase must
  `describe()` (test `competitive_fallbacks_are_renderable`): use `SuffixIs`
  for exact suffixes, `described_guard`/`described_rewrite` around closures.

## Workstream 0 — render the competitive book ✅

Behavior-preserving disclosure fix (render output for existing nodes is
byte-identical; the competitive section went from 0 to ~100 sections):

- `Guard::describe` / `Rewrite::describe` (default `None`); `SuffixIs` guard;
  `described_guard` / `described_rewrite` label wrappers (`fallback.rs`).
- `Trie::fallbacks()` — depth-first enumeration, declaration order within a
  node, Pass child last so seat-fanned entries dedupe to the pass-less key.
- All competition.rs guards converted to `SuffixIs` / described wrappers.
- `render-book` and the web `book()` print guarded sections: heading = node
  auction + guard description, body = rules table or `→ systems on …` note.

Known follow-up (own commit): extend `artificial_calls_are_alerted` over
`Trie::fallbacks()` — fallback-attached rules currently escape that invariant,
and running it may surface genuine missing alerts.

## The packages

Each is a `set_*` knob, default **off** until its A/B ships it, with a
`--ns-*`/`--no-ns-*` switch in `bba-gen`. One knob = one measured change.

### P1 — their two-suiters over our 1M (`set_uvu_over_majors`)

Responder structure over `[1M, (2NT unusual)]` and `[1M, (2M Michaels)]`,
UvU-style: cheaper cue (their lower suit / the other-major cue) = limit+ raise
(alert `comp:uvu-major-raise`), second cue = GF other-major, X = values /
penalty interest, direct raises stay natural-competitive, jump raise
preemptive. Opener reuses `answer_cue_raise`/`answer_cue_minor_raise` for the
limit+ cue. One hand-written reading: suppress the natural walk's "their cue
of our opened suit = natural 5+ suit" misread (unsound vs Michaels), record
the two-suiter shape when the knob is on. Fixes a live misbid: today the
negative-double rule fires over `1♥-(2♥ Michaels)`.
Deferral: their-Michaels-over-our-minors (`1m-(2m)`), same misread.

### P2 — contested weak twos (`set_weak_two_competition`) + strong 2♣ (`set_strong_two_competition`)

Nothing is keyed on `[2x]` today — pure floor. Weak twos: over (X), uncontested
responses ride + business XX 13+ (`comp:weak-two-xx`), Ogust survives via a
systems-on rebase; over an overcall ≤3♠, Ogust-when-legal / penalty-leaning X /
preemptive raises. McCabe = named deferral. Strong 2♣: over (X) systems-on
rebase; over an overcall, natural-GF new suits, X = cards (shadows the floor's
takeout-X — a live bug: a 22+ opener behind a "takeout" double), Pass =
waiting backed by opener's **forced reopening** (finite catch-all X).

### P3 — extended overcall responses (four knobs)

- `set_major_support_double` — support X/XX after `1♥-(P)-1♠-(overcall below
  2♠, or X)`, reusing `support_rules(Spades)`.
- `set_free_bids` — natural free bids inside `over_their_overcall`: 1-level
  new suit 5+ & 6+, 2-level non-jump 5+ & 10+, 1NT 6–10 / 2NT 11–12 with
  stopper.
- `set_negative_double_shape` — enum, `BothMajors` (current, byte-identical
  default) / `Modern` / `Cachalot`. See the theory note below.
- `set_high_overcall_responses` — a second guarded entry for overcalls in
  `2NT < b ≤ 3♠`: neg X through 3♠ (10+), 3NT with stopper, forcing 3-level
  new suits, raises; opener's forced answer to the 3-level neg double.
  4-level cue dropped (X-then-raise or blast) — deferral.

### P4 — over their takeout double (`set_jordan_truscott`)

Responder table at the deeper `[1x, X]` key (total): XX = 10+ no fit
(`comp:value-redouble`), Jordan/Truscott 2NT = 4+ support limit+
(`comp:jordan`), jump raise flips preemptive, 1-level suits forcing, 2-level
suits weak NF (2/1 off over X), 1NT 6–9. The `[1x]` `FirstIs(X)` rebase stays
for every deeper suffix; opener continuations that the rebase would misread
get exact-suffix nodes (`[2NT, P]` → cue-raise answers — else Jordan lands on
Jacoby 2NT; `[3o, P]`; weak `[2x, P]`).

## Negative doubles at the 1-level (theory verdict, 2026-07-06)

- **Sputnik (Roth–Stone 1957/58):** X = 7+ HCP, denies a biddable (5-card)
  major at the 1-level. Over (1♦) 4-4 majors exactly; over (1♥) exactly 4♠;
  over (1♠) 4+ ♥.
- **Modern (BWS 2017 / Cohen; what BBA plays, untoggleably):** floor 6,
  unbounded. (1♦) → 4+/4+ majors; (1♥) → **exactly** 4♠ (1♠ = 5+); (1♠) → 4+
  ♥; `1♥-(1♠)-X` ≈ 4-4 minors. Through 3♠ or higher. Weak 5-card major →
  double-then-bid, NF. Trap pass requires opener's reopening-X duty.
- **Cachalot / Sardine / Spoutnik rotatif (French; Claire Martel memo):**
  transfer Walsh in competition, over 1♣/1♦ and (1♦)/(1♥) only: X = 4+
  adjacent major, 1♥ = 4+ ♠, 1♠ = takeout hand denying 4♠. Opener's 1-level
  completion = **exactly 3-card support, forcing**; 1NT doesn't deny 3; raise
  = 4. Reverts to natural if they bid again. Most projection-friendly (pure
  per-suit bounds); its headline benefit is right-siding → **DD-blind, the PD
  bracket decides**; no BBA analogue to distill.

Don't author the SEF "4 w/8+ OR 5 w/7–10" disjunction (the OR-projection
wall). Measure as three arms: `BothMajors`+free-bids / `Modern`+free-bids /
`Cachalot`.

## Measurement discipline per package

- P1 cues, P3 (all), P4 Jordan/XX: constructive contract-finding — DD-visible,
  normal win/wash ship rules.
- P2a preemptive raises and P4's 3o flip: obstruction-wall — plain-DD ≈ 0 is
  *expected*; score both brackets, bucket by call before judging, and carve a
  dragging preemptive bucket behind a sub-toggle rather than sinking the
  package.
- P2b (2♣ contested) is small-N: judge on IMPs/divergent + worst-board
  forensics.
- Cachalot: PD bracket decisive (right-siding).

## Ledger

| Package | Knob | Status | Verdict (plain / PD, IMPs) |
| --- | --- | --- | --- |
| WS0 renderer | — | **shipped** | render-only, node output byte-identical |
| P1 two-suiters over 1M | `set_uvu_over_majors` | **authored** (off, `--ns-uvu-over-majors`) | pending A/B |
| P2a weak twos contested | `set_weak_two_competition` | **authored** (off, `--ns-weak-two-comp`) | pending A/B |
| P2b strong 2♣ contested | `set_strong_two_competition` | **authored** (off, `--ns-strong-two-comp`) | pending A/B |
| P3c major support double | `set_major_support_double` | **authored** (off, `--ns-major-support-double`) | pending A/B |
| P3b free bids | `set_free_bids` | **authored** (off, `--ns-free-bids`) | pending A/B |
| P3d neg-X shape | `set_negative_double_shape` | **authored** (BothMajors, `--ns-negative-double-shape`) | pending A/B |
| P3d′ Cachalot arm | `NegativeDoubleShape::Cachalot` | **authored** (incl. opener completions) | pending A/B |
| P3a 3-level overcalls | `set_high_overcall_responses` | **authored** (off, `--ns-high-overcall`) | pending A/B |
| P4 Jordan/Truscott over (X) | `set_jordan_truscott` | **authored** (off, `--ns-jordan-truscott`) | pending A/B |
| alert invariant over fallbacks | — | follow-up | — |
