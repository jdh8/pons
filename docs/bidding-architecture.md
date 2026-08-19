# The bidding architecture

Orientation for `src/bidding`: the layers, the resolution order, and the
invariants that keep the system sound. Symbols are named so you can `grep` them;
line numbers drift, names don't. The end-to-end procedure for adding a
convention is the `author-convention` skill; measurement is
[measurement.md](measurement.md).

## The layer cake

```text
auction + hand
  → Partnership          (book.rs — one seat's view; routes by Phase to a book)
  → book Trie       (trie.rs — authored nodes keyed by auction suffix)
  →   Rules         (rules.rs — weighted, constraint-gated rule tables)
  → floor chain     (fallback.rs — when no node claims the hand)
  →   instinct()    (instinct.rs — keyless natural-action ladder)
  →   learned floor  (neural_floor.rs — the configured BBA-distilled net,
                       which reads both partnerships' convention cards)
```

- **Books** (`book.rs`): `Constructive`, `Competitive`, `Defensive` tries per
  side, bundled into `System`; `Partnership` is one seat's runtime view
  (`classify_with_provenance`, `infer`). `Phase::of` is the single routing
  point deciding which book an auction belongs to. `System::bind` binds the
  books into a `Partnership`. There is no whole-system identity label (`Family`
  was deleted in 0.11): a system announces itself through its calls' own
  alerts and readings, and defense dispatch against a future non-natural
  book belongs in reading-gated rules, not a label argument.
- **System factories** (`american.rs`): `american()` is the shipped 2/1
  system. The other three vary exactly one axis: `american_book()` is the
  authored books with no floor, `american_floor()` the floor with no book, and
  `american_instinct()` the books over the deterministic ladder instead of the
  net. The private `with_floor` is where floors attach.
- **What the authored book is worth**: `american` − `american_floor`, 204800
  boards/vul vs the BBA reference (2026-08-06, `scripts/ab-book-value.sh`, seed
  base 1785947357, both arms on the configured v4 floor), is **+0.3049 ±0.0174
  plain / +0.5470 ±0.0212 PD** NV and **+0.4509 ±0.0224 plain / +0.7994 ±0.0271
  PD** vul. Auctions diverge on 49% of boards, so the book earns ≈+0.6 plain /
  ≈+1.4 PD IMPs per board it actually touches. Read it as the book's *total*
  contribution: an empty book also stops projecting into `Inferences`, so the
  net's `features_v4` inference block collapses to unknown, and the gap is the
  book as authored calls **and** as disclosure.

  **The gap grew where a stronger floor was predicted to shrink it**, and
  entirely in PD: against the v3-floor run (2026-07-20, `7b0b51d`) plain moved
  +0.29 → +0.30 NV and +0.37 → +0.45 vul, while PD moved +0.23 → **+0.55** and
  +0.27 → **+0.80**. So the floor swap's PD gain — gate 1's +0.53/+0.54 — is
  roughly the *same size* as the growth in this gap, which is what you would see
  if the configured net's PD gain accrues **only where an authored book sits
  above it**. Two co-explanations are not separated by this run: the fortnight of
  conventions shipped in between were themselves mostly plain-wash/PD-win, and
  `american_floor` now hands the net a card describing agreements it has no book
  to play, which can only widen the gap. Separating them needs an
  instinct-floored no-book arm, which is not currently a factory — the shipped
  reading is "the book is worth more under the configured floor, mechanism
  unattributed."

## Resolution and shadowing — the invariants

- `Trie::classify_floored` consults the exact node first and falls through to
  the fallback chain (ultimately the floor) only when the node yields **no
  mass** for the hand (all rules score −∞). A node that gives the hand any
  finite logit — including a `Pass, 0.0, hcp(0..)` catch-all — **shadows the
  floor completely** for every deeper auction resolving to it.
- Therefore: to let the floor own a position, **delete the node** (leaving it
  rule-less is not enough if any catch-all matches). Adding a smart floor rule
  under a live book node is dead code. Verify a floor rule fires through the
  full `Partnership`, not bare `instinct()` — the `ab-instinct-floor` telemetry
  shows activations. Corollary: a node whose catch-all is finite for every
  hand (`Pass, 0.0, hcp(0..)`, as `defense_to_suit` at `[1X]`) shadows the
  floor for *every* deeper auction resolving back to it, so stripping its
  other rules does not make it floor-transparent — **delete the node**. Rubens
  advances paid for this seam in commit `792710c`, which removed the
  finite-catch-all `advances()` nodes from both defenses so the floor could
  own those calls; re-adding a book `advances()` node would shadow them again.
- **Every rule table ends in a finite catch-all** — a table that can reject
  every hand once produced a degenerate best-call (the 7NT bug). The flip
  side: rejecting is *how* a node hands a position to the floor, so an
  intentional rejection must have the floor behind it.
- **The floor partition**: learned floors (neural, live search) wrap the
  competitive and defensive books **only**; the constructive book is floored
  by deterministic `instinct()`. Measured, not just triage: the net on
  constructive play loses 0.8 IMPs/board to `instinct()`. The 2026-06
  `constructive-abc` run used 2 000 none-vulnerable boards: live `SearchFloor`
  tied at +0.002 but was about 1 000× slower; only ~28% of the old corpus was
  constructive, and its teacher used `instinct()` there. Keep the partition.
- `instinct()` is keyless except for reading **our own strong notrump**: it
  completes Jacoby/Texas transfers and refuses to pass out a forced game.
  Deep conventional continuations that run off-book should be caught by a
  *smarter floor*, not by authoring a node per artificial bid (whack-a-mole;
  one attempt authored 42 nodes and still missed a family).
- **An artificial forcing call is not finished until its interfered tails are
  authored.** Registration is suffix-exact: a table registered on `{call} -`
  covers only the clean continuation, so the moment an opponent doubles or
  overcalls, the auction drops to a floor whose `Inferences` carry no forcing
  channel — it passes out the doubled ask, or bids a phantom suit off the bare
  envelope. This is distinct from the smarter-floor rule above: deep *clean*
  continuations belong to the floor, but the `(X)` and cheap-overcall tails of
  a forcing gadget are part of the convention itself. A
  `systems_on_over_double` rebase often covers the whole `(X)` tail for free;
  registration blocks whose every suffix ends in `-` are the tell. Priced
  twice in one campaign (N1b's doubled ask passed out in `3♥x`; N1c's
  interference hole, −4.68 PD/fired vul —
  [closed N1 history](archive/one-notrump-competitive-closed.md#n1--the-landy-2-counter-shipped-default-on-2026-08-14)).

### Settle floor and the rejected TTL

The shipped settle floor (`set_settle_floor`, default on, commit `9badc15`)
treats Pass as playing the current contract with its real X/XX, permits a
penalty pass of partner's takeout double, and requires values for a voluntary
four-level free bid. Its 200 000-board Stage-1 verdict and plain-DD recheck are
recorded in `CHANGELOG.md` under “Pass = play the top bid.” Because this is a
deterministic-floor change, changing its scorer alone does not imply retraining;
only feeding that scorer into the search teacher would.

⚠ Two PD figures exist for that shipped run: a later plain-DD recheck quotes
the same **+0.178/+0.294** plain result against **+0.270/+0.379 PD**, while
`CHANGELOG.md` records **+0.264/+0.372 PD** (200 000 boards). No second seed
was recorded, so both stand until their provenance is identified.

The proposed Stage 2 “the floor buys the contract once” TTL was built and
reverted. It used off-book bid count, excluding authored bids through
`Context::prefixes()` book depth. Bare TTL measured **+0.851 PD but −0.401
plain DD IMPs/board**; a level-≥4 gate still measured **−0.018 plain**, and
level ≥5 was inert (**0.00**). The PD win came from `ns_score_pd` synthetically
doubling the competitive contracts TTL declined to bid. A hop limit keyed on
climb distance damps makeable and failing jumps alike, so it is orthogonal to
double-dummy make-ness; its obstruction/judgement thesis needs single-dummy
evidence. The design was therefore judged unsound, not deferred.

## The Constraint DSL (`constraint.rs`)

A `Constraint` folds three ways, and every authored rule should support all
three:

| Fold | Returns | Consumer |
| --- | --- | --- |
| `eval` | logit for a hand | classification (bidding) |
| `describe` | `Description` tree | disclosure, corpus, `render-book` |
| `project` | forward `EnvelopeUnion` (floors) | decoding a call back into shape/strength |
| `project_band` | two-sided `EnvelopeUnion` (ceilings return) | the pass reading — a *declined* call reads by what its gate would have allowed (`set_pass_reading`) |

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
- Weights are **centinats** — integer hundredths of a nat, so `155` is the old
  `1.55` and a near-deterministic gap is about `300`. Integral on purpose: two
  rules either share a rung exactly or they do not. Under `f32` they could not,
  because several tables build a declining ladder by repeated subtraction and
  the rounding drifted a rung off the literal it was meant to match at some
  sites but not others.
- **Two rules for one call at one weight is a claim, and the build checks it**
  (`rows::weight_tie_report`, asserted by `assert_package_invariants`). Such a
  pair is redundant: the logit is the max and constraints are crisp, so it
  evaluates to `w + crisp(C₁ ∨ C₂)`, and the reader already disjoins every
  matching rule's projection. It is also lossy — `Rules::explain` breaks the tie
  with a strict `>` and names only the first rule, so which alert and which
  label describe the call falls to authoring order. Prefer one rule with an
  authored `EnvelopeUnion`. Differing weights are the opposite and are fine:
  an authored precedence, the lower rule speaking only for hands the higher
  one rejects.

### Trie × envelope unions (assessed 2026-07-28: the union stays a fold, not the storage)

The trie stores *rules*; `EnvelopeUnion` is the compiled union of `Envelope`
boxes — mathematically a DNF — and is the reading produced by the
`project*`/`announce` folds of the constraint, not its
replacement. Storing the book as a trie of envelope unions was considered and declined:

- Boxes carry only membership. The other folds do the bidding: `eval`'s
  weighted logits are how overlapping rules resolve, and gates like
  `points_or_net` or the RKCB answers accept hands no box contains — ⊤ is
  their only *sound* projection. `describe`/`announce` carry disclosure.
- **The wrong-seat trap** bars static box storage outright for
  context-relative legs (`support`, `stopper_in_their_suits`): they
  re-project under the *reader's* context, so no box list fixed at authoring
  time reproduces them ([dnf-migration.md](dnf-migration.md), the F2b′
  Jacoby family). Where a node's boxes *are* static, `envelope_union_upgrade(legacy,
  boxes)` pins them on the node — `EnvelopeUnion` incorporated into the trie exactly
  where it is sound, and only there.
- The measured record prices representation churn negative even when
  information-preserving: chop C1 (−0.037 plain), the F flip (lost until the
  evaluator was retrained), MARG/MASS (refused at the NLL gate).

A rule whose meaning genuinely is a box union can be authored as one — 
`Envelope`/`EnvelopeUnion` implement `Constraint` with identity projection
(chop C).
The live sequel is retiring the hand-written disjunction readers in favor of
the authored projections: [reader-retirement.md](reader-retirement.md).

The corollary (assessed 2026-07-28): **no opaque-predicate field on
`Envelope`/`EnvelopeUnion`** — `boxes & pred(...)` already merges the two with the box
half projecting exactly, `sample_layouts_replay` already gives every pred
membership teeth under the correct authoring-seat context, and a stored
closure would make `subset_of` undecidable (breaking `tidy`'s dedup) while
replaying under the *reader's* context — the wrong-seat trap. The long-run
convergence is scoped instead: **own-hand** preds may become axes/boxes
on-demand ([dnf-migration.md](dnf-migration.md) §Irreducible tail is the
worklist and its triggers); pair-level gates (`combined_points`,
`fit_sum_game`, …) and net gates stay fold-side permanently — an `Envelope`
describes one hand, and a net's accept region is no box union — read via a
seat-carrying `project` or the sampled projection.

## Disclosure: `Alert` and readings

- `Alert("kebab-slug")` (`rules.rs`) marks a call artificial. The system-wide
  identity: **a rule that floors a suit its bid didn't name is artificial, and
  every artificial bid must be alerted** — enforced by the unit invariant test
  `artificial_calls_are_alerted` (`src/bidding/inference/tests.rs`). Passes and
  doubles are natural-by-default (they defend the contract on the table);
  artificiality is bid-only. A pass still *reads*: its general meaning is
  negative inference — excluding every other call its table offered — decoded
  from the table's own Pass gate via `project_band` (`set_pass_reading`,
  default on), each pass resolved in the trie of its own turn.
- Alerted calls are decoded by **rule projection** (`project_authored` in
  `inference/projection.rs`, master switch `set_alert_reading`, default on): the reader
  replays the authoring rule's `project` fold. Unalerted = natural =
  floor-safe.
- Projection reaches only calls the reader's own book authored — the
  opponents' calls read through the natural walk. **Table-wide alert
  reading** (`set_table_alert_reading`, default on) extends
  disclosure to the whole table, as at a real one: each opponent call is
  resolved in *their* phase-routed book (`Partnership::trie_for` on the auction
  cut at their turn — `Phase::of` is slice-relative, so their side's phase
  falls out) under their at-the-time context, and decoded when alerted. The
  partnership models the opponents as playing our own books: exact in self-play,
  an approximation against other natural-family engines.
- `Inferences::read` (`inference/read.rs`) accumulates per-player `Envelope`
  (per-suit length ranges + points) from the auction — design law **soundness
  over tightness** (never claim more than the calls promise). Convention
  readings suppress the literal natural reading at the artificial bid's index
  and post-walk narrow the real shape; the per-suit ranges can't express
  disjunctions, so pin the *other* suits and let the sampler deal the residual
  into the long suit. That set is **shrinking**: with envelope unions the projection
  carries the disjunction itself, so the hand-written readers are being retired
  one measured chop at a time — see
  [reader-retirement.md](reader-retirement.md).
- `Rules::gated` takes an **explicit slug set** (2026-08-14; `gated_out` is
  its complement for "everything except the dormant scheme").  The old
  closure form silently dropped a rule whose slug wasn't active; now a stale
  slug in the set panics at build, and two variants surviving onto one call
  panic instead of shipping into one trie.
- The completeness limit of the artificial-call invariant — a vacuous
  (`hcp(0..)`) forced completion derives as natural, which is why derivation
  left the decode gate — is covered behaviourally:
  `completion_readings_admit_the_bidder` replays the bidder through the
  completion lanes under `reading.completion_alerts` (the family knob that
  alerts every completion/answer uniformly; shipped default-on 2026-08-14) and
  requires every reading to admit its own bidder.  The verdict history and
  the whole alert-vs-derivation question live in
  [reader-retirement.md](reader-retirement.md) §The alert question.

## Samplers (`sampler.rs`)

"The inverse of `Inferences`": deal layouts consistent with an auction.

- `sample_layouts` — rejection-sample within the `Inferences` ranges.
- `sample_layouts_replay` — additionally re-runs the policy at every authored
  node, accepting hands whose made call ranks within `MARGIN` nats of best
  (knob `set_rule_accept`, default on since M8.1b `74d783d`). Passes replay like
  any call — the
  sample-level negative inference (rejects a candidate that would have
  opened/preempted), the disjunctive half the interval envelope can't hold.
- Budget philosophy: a deal costs ~0.3 µs, a DD solve dwarfs it — when the
  sampler starves, **draw more deals** (cap `REPLAY_DRAW_CAP` ≈ 50M), never
  loosen the reading. A consecutive-reject dry-limit distinguishes
  budget-starved (keep drawing) from infeasible (bail to ranges).

## Search, EV, single-dummy

- `ev.rs` — a call's worth by rollout, scored `ns_score_bid` (perfect
  defense; evaluating a *call*, not a result).
- Authored rules are the **fast-floor prior**, and every convention still
  needs its inference reading — an unreadable convention strands anything that
  reasons over the auction. (The gated live-search bidder that made this acute
  was retired with the M1–M3 neural line; see the CHANGELOG.)
- `single_dummy.rs` — MC-DD trick estimation; `single_dummy_leads` prices the
  blind opening lead (the known DD bias at 1NT); `Partnership::infer` attaches the
  trie so alerted conventions decode in the leader's sampling.

## Serving one decision: `Bidder`, `Context`, `DecisionCache`

`Bidder` (`bidding.rs`) is the adapter interface — one method, "here is a hand
and an auction, give me logits". `Partnership` implements it, and so do the
foreign engines we measure against: BBA/EPBot, BEN, and the bench `Legacy`
implementor. `Table<B, B>` is generic over it, which is what lets a pons seat
play a BBA seat with no special case.

`Context` (`context.rs`) is what that call may consult besides the hand, and it
is **three strata in one struct**:

| Stratum | What it is | Bare context |
| --- | --- | --- |
| Mechanical facts | vulnerability, the auction, and eleven facts derived from it | always present |
| Attachments | borrows of what the caller already built: the serving `Partnership` and the one it models for them, both convention cards, the authored projection, the trie prefixes, and the `DecisionProfile` | all absent; the profile falls back to `DecisionProfile::default()` |
| Per-decision memo | `DecisionCache`, plus a `revision` counter that invalidates it | absent — a bare context recomputes, by design |

Nothing in stratum 2 is *discovered*; it is handed over. Since 0.11 that is
literally true — no bidding knob lives on a thread, so `Context::reading_profile`
is one field read rather than the four-arm cascade (cache, attached system,
explicit pin, thread-local) it was through 0.10.

The facts are derived **twice, by two different algorithms**: `ContextCursor`
maintains them incrementally as the reading walk advances one turn at a time,
while `Context::new` rescans. Nothing makes them agree by construction, so
`incremental_and_rescanned_facts_agree_on_the_frozen_corpus` holds them to the
same answer at every prefix of all 512 frozen positions. A drift there is a
silent wrong-bidding bug.

`DecisionCache` memoizes one classification's readings, features and pure gate
results, keyed by hand, thread and profile so it cannot answer a question it
was not built for. It and the compiled rule path are ~690 lines that landed on
2026-08-05 in two commits with empty bodies (`42a35cc`, `6a109be`) and no
CHANGELOG entry; their design record is
[bidding-performance-handoff.md](bidding-performance-handoff.md). **Read that
before changing either.**

## Knobs

Every knob is a public field of `Agreements` or one of its area/profile
structs. Conventions:

- The default encodes the measured verdict ([measurement.md](measurement.md)
  ship rules); the non-default state of a shipped knob keeps an off-switch in
  `bba-gen` (`--no-ns-*` for default-on knobs).
- **The `Agreements` value is the input, not the thread.** A system factory
  takes `&Agreements`, bakes the books from it, and the `System` keeps it;
  `System::bind` pins its classify-time half (`agreements.decision`) into the
  `Partnership` and bakes the compiled-rule sidecar under the same value. One value,
  one build — the rules, the sidecar, the readings and the floor cannot come
  from different reads. `Partnership::agreements()` asks a built partnership what it
  plays.
- A harness starts with `Agreements::default()`, writes every field that defines
  the arm, and builds from that same value. A built partnership is a pure value: hand
  it to workers directly. An A/B arm is one agreements value and one partnership.
- Which half a setting belongs to is decided by *where it is read*, and a
  setting read at both times lives in the classify half alone, with the book
  reading it from there (`no_knob_lives_in_two_homes` enforces this across all
  eight build-time areas).
- Set-after-build is inert until the next build. To move a setting on a partnership
  already built, edit its own pin with `Partnership::profile_mut`, which does not
  rebuild the book — the hook for an eval-time-only arm, and for the two
  settings (`probed`, `probed_vacuous`) that are only knowable around a
  `probe` run.
- A context with no partnership attached (a bare `Context::new` — tests,
  diagnostics) falls back to `DecisionProfile::default()`; a context *derived*
  from a reader's (the per-turn walk in `project_authored`) inherits the
  reader's pin via `Context::with_profile`.
- A knob's off-state must leave the default system byte-identical while the
  treatment is unshipped.

## Reference systems

- **BBA/EPBot** (vendored `vendor/bba` submodule, driven natively via
  `libloading`) is the reference opponent — `examples/bba-gen` (bid + dump) and
  `bba-score` (score), sharded by `scripts/bba-gen-parallel.sh`, diffed by
  `ab-dump-diff`. Per-seat conventions toggle via `--our-conv`/`--their-conv`.
- `tests/demo_system.rs` is a thin living-documentation system; `render-book`
  prints any book as prose.
