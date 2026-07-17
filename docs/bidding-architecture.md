# The bidding architecture

Orientation for `src/bidding`: the layers, the resolution order, and the
invariants that keep the system sound. Symbols are named so you can `grep` them;
line numbers drift, names don't. The end-to-end procedure for adding a
convention is the `author-convention` skill; measurement is
[measurement.md](measurement.md).

## The layer cake

```text
auction + hand
  → Stance          (book.rs — one seat's view; routes by Phase to a book)
  → book Trie       (trie.rs — authored nodes keyed by auction suffix)
  →   Rules         (rules.rs — weighted, constraint-gated rule tables)
  → floor chain     (fallback.rs — when no node claims the hand)
  →   instinct()    (instinct.rs — keyless natural-action ladder)
  →   learned floors (neural_floor.rs / search_floor.rs, feature-gated)
```

- **Books** (`book.rs`): `Constructive`, `Competitive`, `Defensive` tries per
  side, bundled into `Pair`; `Stance` is one seat's runtime view
  (`classify_with_provenance`, `infer`). `Phase::of` is the single routing
  point deciding which book an auction belongs to. `Pair::against` selects a
  defensive book by the opponents' `Family` (`NATURAL`, `STRONG_CLUB`,
  `WEAK_NOTRUMP`).
- **System factories** (`american.rs`): `american()` is the shipped 2/1
  system; `american_classic`, `american_search`, `american_neural*` are
  variants. The private `with_floor` is where floors attach.

## Resolution and shadowing — the invariants

- `Trie::classify_floored` consults the exact node first and falls through to
  the fallback chain (ultimately the floor) only when the node yields **no
  mass** for the hand (all rules score −∞). A node that gives the hand any
  finite logit — including a `Pass, 0.0, hcp(0..)` catch-all — **shadows the
  floor completely** for every deeper auction resolving to it.
- Therefore: to let the floor own a position, **delete the node** (leaving it
  rule-less is not enough if any catch-all matches). Adding a smart floor rule
  under a live book node is dead code. Verify a floor rule fires through the
  full `Stance`, not bare `instinct()` — the `ab-instinct-floor` telemetry
  shows activations.
- **Every rule table ends in a finite catch-all** — a table that can reject
  every hand once produced a degenerate best-call (the 7NT bug). The flip
  side: rejecting is *how* a node hands a position to the floor, so an
  intentional rejection must have the floor behind it.
- **The floor partition**: learned floors (neural, live search) wrap the
  competitive and defensive books **only**; the constructive book is floored
  by deterministic `instinct()`. Measured, not just triage: the net on
  constructive play loses 0.8 IMPs/board to `instinct()`. Keep the partition.
- `instinct()` is keyless except for reading **our own strong notrump**: it
  completes Jacoby/Texas transfers and refuses to pass out a forced game.
  Deep conventional continuations that run off-book should be caught by a
  *smarter floor*, not by authoring a node per artificial bid (whack-a-mole;
  one attempt authored 42 nodes and still missed a family).

## The Constraint DSL (`constraint.rs`)

A `Constraint` folds three ways, and every authored rule should support all
three:

| Fold | Returns | Consumer |
| --- | --- | --- |
| `eval` | logit for a hand | classification (bidding) |
| `describe` | `Description` tree | disclosure, corpus, `render-book` |
| `project` | forward `Inference` envelope (floors) | decoding a call back into shape/strength |
| `project_band` | two-sided envelope (ceilings return) | the pass reading — a *declined* call reads by what its gate would have allowed (`set_pass_reading`) |

Builders: `hcp`, `points`, `fifths`, `len`, `balanced`, `support`,
`stopper_in_their_suits`, `they_bid`, combined with `&`/`|`/`!`. The suit-set
combinators state multi-suit shapes so they read like the spec *and* project:
`and(suits, range)` = every suit in range; `or(suits, range)` = some suit in
range (projects the union — sound but loose).

- Two-suiter minimums are convention-specific — DONT 4-4
  (`and([♥,♠],4..)`), Landy 5-4 (`and(4..) & or(5..)`), Michaels 5-5
  (`and(5..)`); Multi's unknown 6-card major is `or([♠,♥],6..)`. Don't merge
  their shape functions.
- Avoid the opaque escape hatches `pred`/`described` in new rules: they don't
  project, so the call can't be decoded (`verify::compare` guards a rewrite
  from opaque to DSL).
- Overlapping rules resolve by **weight**; structures that depend on disjoint
  meanings (Woolsey) must keep their shapes disjoint or equal-weight rules tie
  unpredictably. A cheaper overlapping rule (a transfer) can swallow the hands
  a new rule was written for — check who wins the weight race.

## Disclosure: `Alert` and readings

- `Alert("kebab-slug")` (`rules.rs`) marks a call artificial. The system-wide
  identity: **a rule that floors a suit its bid didn't name is artificial, and
  every artificial bid must be alerted** — enforced by the unit invariant test
  `artificial_calls_are_alerted` (`src/bidding/inference.rs`). Passes and
  doubles are natural-by-default (they defend the contract on the table);
  artificiality is bid-only. A pass still *reads*: its general meaning is
  negative inference — excluding every other call its table offered — decoded
  from the table's own Pass gate via `project_band` (`set_pass_reading`,
  default off pending A/B), each pass resolved in the trie of its own turn.
- Alerted calls are decoded by **rule projection** (`project_authored` in
  `inference.rs`, master switch `set_alert_reading`, default on): the reader
  replays the authoring rule's `project` fold. Unalerted = natural =
  floor-safe.
- Projection reaches only calls the reader's own book authored — the
  opponents' calls read through the natural walk. **Table-wide alert
  reading** (`set_table_alert_reading`, default off pending A/B) extends
  disclosure to the whole table, as at a real one: each opponent call is
  resolved in *their* phase-routed book (`Stance::trie_for` on the auction
  cut at their turn — `Phase::of` is slice-relative, so their side's phase
  falls out) under their at-the-time context, and decoded when alerted. The
  stance models the opponents as playing our own books: exact in self-play,
  an approximation against other natural-family engines.
- `Inferences::read` (`inference.rs`) accumulates per-player `Inference`
  (per-suit length ranges + points) from the auction — design law **soundness
  over tightness** (never claim more than the calls promise). Convention
  readings suppress the literal natural reading at the artificial bid's index
  and post-walk narrow the real shape; the per-suit ranges can't express
  disjunctions, so pin the *other* suits and let the sampler deal the residual
  into the long suit.
- Footgun: `Rules::gated` blocks keyed on an alert slug **silently drop** the
  rule when the slug isn't in the active set; alert-const names can collide
  with toggle thread-locals — pick distinct names.

## Samplers (`sampler.rs`)

"The inverse of `Inferences`": deal layouts consistent with an auction.

- `sample_layouts` — rejection-sample within the `Inferences` ranges.
- `sample_layouts_replay` — additionally re-runs the policy at every authored
  node, accepting hands whose made call ranks within `MARGIN` nats of best
  (knob `set_rule_accept`, default off). Passes replay like any call — the
  sample-level negative inference (rejects a candidate that would have
  opened/preempted), the disjunctive half the interval envelope can't hold.
- Budget philosophy: a deal costs ~0.3 µs, a DD solve dwarfs it — when the
  sampler starves, **draw more deals** (cap `REPLAY_DRAW_CAP` ≈ 50M), never
  loosen the reading. A consecutive-reject dry-limit distinguishes
  budget-starved (keep drawing) from infeasible (bail to ranges).

## Search, EV, single-dummy

- `ev.rs` — a call's worth by rollout, scored `ns_score_bid` (perfect
  defense; evaluating a *call*, not a result).
- `search_floor.rs` — `SearchFloor` / `american_search()`: the gated live
  "thinking" bidder; samples layouts, prices shortlisted calls by DD. Authored
  rules are the **fast-floor prior**; search disposes and finds the slams
  game-capped rules can't — which is why every convention needs its inference
  reading (an unreadable convention strands the search).
- `single_dummy.rs` — MC-DD trick estimation; `single_dummy_leads` prices the
  blind opening lead (the known DD bias at 1NT); `Stance::infer` attaches the
  trie so alerted conventions decode in the leader's sampling.

## Knobs

~100 `set_*` free functions on thread-locals, clustered in `instinct.rs`,
`constraint.rs`, `inference.rs`, and `american/{openings,rebids,notrump,competition,defense}.rs`.
Conventions:

- The default encodes the measured verdict ([measurement.md](measurement.md)
  ship rules); the non-default state of a shipped knob keeps an off-switch in
  `bba-gen` (`--no-ns-*` for default-on knobs).
- Most knobs are read at **book construction** — baked into the books, safe
  under rayon. The `inference.rs` knobs are read at **classify time** — set
  them inside worker closures in parallel harnesses.
- A knob's off-state must leave the default system byte-identical while the
  treatment is unshipped.

## Reference systems

- **BBA/EPBot** (vendored `vendor/bba` submodule, driven natively via
  `libloading`) is the reference opponent — `examples/bba-gen` (bid + dump) and
  `bba-score` (score), sharded by `scripts/bba-gen-parallel.sh`, diffed by
  `ab-dump-diff`. Per-seat conventions toggle via `--our-conv`/`--their-conv`.
- `tests/demo_system.rs` is a thin living-documentation system; `render-book`
  prints any book as prose.
