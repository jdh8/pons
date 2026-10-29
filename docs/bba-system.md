# The `bba` system — BBA's book over our floor

**Status: built and read 2026-10-29.** Gate 0 did not fire the kill rule;
Gate 2 read **`bba` loses to `american`** (−0.197 / −0.252 plain, −0.175 /
−0.209 PD) and **loses to BBA by more than `american` does** (−0.634 /
−0.729 plain against −0.267 / −0.270): at this instrument's resolution the
gap is **judgement, not agreements** — bba-book.md §5.6's bet, not the
chat's. §9 is the build record — what exists, where, the Gate 0 table, the
Gate 2 table and what it does and does not separate. The idea came out of a Claude
Chat review of the BBA gap (commit `99509eea`, the raw notes deleted with
this plan; §5 keeps what they asked and what already answers it). The five
decisions of §7 were taken at their defaults to build; each is one line to
overturn.

**One line.** A second book, *compiled from BBA's own readings* rather than
authored, floored by the same net, accountants and rails `american()` ships.
It never ships as the default; it is the instrument that splits the gap into
**agreements** and **judgement**, and the oracle that prices BBA's agreements
node by node so the winners can be ported to `american()` as knobs.

## 0. The question

The anchor ([bba-gap-campaign.md](bba-gap-campaign.md)) buckets each board by
its *first* divergence and charges everything after it to that bucket. It
cannot say whether a bucket loses because BBA's **rules** admit a better call
there or because BBA's **bilans** judges the same admissible calls better.
Every bucket mixes both, and the two have opposite fixes: an agreement is a
knob, judgement is a retrain or search.

Holding the judgement fixed and swapping the rules separates them. With
`bba` = BBA's book over our floor:

| pair, same deals, same sha | isolates |
| --- | --- |
| `bba` − `american` (both on our floor, paired `ab-dump-diff`) | the **agreements** term, under our judgement |
| `bba` vs BBA (the `bba` arm's own headline) | the **judgement** term, at equal rules (plus BBA's 8% out-of-band calls, §1) |
| `american` vs BBA (today's headline, −0.266 plain / −0.049 PD at `40847142`) | both |

The chat's bet was the first term: "you lose on data, not smarts — BBA's
trie is bigger and denser, and that is the kind of gap automation closes".
[bba-book.md §5.6](ai-bidder/bba-book.md#56-the-bookfloor-partition) bets the
other way: "the axis that separates BBA from us is the **choice**, not the
meaning". One build reads both.

## 1. What "BBA's rules" are, and what they are not

**Available.** The hand-free interpreter walk
([bba-book.md](ai-bidder/bba-book.md)): at every node, for every legal call,
what BBA's reader says the call means — an HCP band (`h`), the suit-length
bands the call changed (`n`), stopper flags (`st`), and the feature deltas
(`f`: forcing, game-forcing, the alert triple, the total-points band
404/405). The shipped run is 55,792 nodes, 1,375,446 children, exhaustive to
depth 4 and reach-gated below (`--min-reach 2` on a 120k-board self-play
corpus), ceilings `4♠` constructive / `3NT` contested. Readings are
vulnerability-invariant (19,438/19,438) and a pure function of the auction,
so the dump *is* the book.

**Not available.** BBA's **precedence** among the calls that admit a hand.
The compiled engine exposes no rule weight (the `0–99` weights of `MB.TXT`
are a 2009 export the binary never loads —
[bba-floor.md §0](ai-bidder/bba-floor.md#0-does-mbtxt-even-drive-the-engine)),
and where no rule fires the bilans chooses, which is arithmetic over a
reconstructed deal, not a table. The bands are not a partition either:
at `1♠ -`, `1NT` (5–12, ≤3♠) and `2♠` (7–9, 3+♠) both admit a 3-card 8-count.
And BBA's own bidder falls outside its readings on **8.15%** of calls
(`--crosscheck`, 1,000 boards).

So the walk gives **admissibility, not choice**. "BBA's rules" in this plan
means *the set of calls BBA's book admits for a hand at a node*, and the
choice among them is exactly what "our own accountant" supplies: the shipped
floor — the K = 8 v6 ensemble, the constructive and competitive accountants,
the five rails. That is the literal reading of the one-liner and the only one
the data supports without a second source. §3's arm B adds the second source
(BBA's empirical precedence from its own dealt calls) *after* arm A reads.

**Which walk.** The shipped dump
(`/data/public/jdh8/pons/dl02/hdd-data/pons-ab-results/bba-book/2026-08-23-08c54312-dirty`)
is BBA playing **our** card on all four seats — the right column for reading
BBA's view of our agreements, the wrong one here. At the anchor BBA's own
pair plays EPBot's 2/1 defaults (`bba-gen` with no `--their-card`, system 0;
`examples/bba-gen/main.rs:3574`), which is the walk's `--card none` column,
and [bba-book.md §5.0](ai-bidder/bba-book.md#50-the-card-is-the-book) shows
the two differ (`1NT (2♣)` is Multi-Landy in one and natural in the other).
**Re-walk at HEAD with `--card none`.** Done 2026-10-29 (§9.1): 96
single-threaded shards took 23 s for the walk and 7 s for a fresh 120k-board
reach corpus under the same card. `scripts/bba-book.sh` now takes
`PROBE_BBA_BOOK=` (the binary to pin, the `BBA_GEN=` idiom) and
`PROBE_BBA_BOOK_LOADER=` (a command every shard runs under — the glibc-2.36
loader on this box, `CLAUDE.local.md`).

**Not this.** Transcribing `MB.TXT`'s 6,094 parametric rules. It is the 2009
export, its constraints are in a private feature language (`#0(seat,suit)`),
and the walk reads the *live* engine. It stays the reversible second source
if the walk's reach gate turns out to be the binding limit (§6 trap 10).

## 2. Design — a data-compiled book whose nodes are masks

### 2.1 The classifier

`BbaBook` compiles the dump into a `Trie` keyed by exact auction. Each walked
node carries one `Classifier` that, given the hand and context:

1. computes the **admissible set** `A` — every child whose cumulative bands
   contain the hand: HCP from `h`, four suit lengths from the `n` deltas
   accumulated along the path for that seat (the renderer already does this
   walk — reuse it), stoppers from `st` where the hand's holding is
   checkable;
2. runs the **floor** at that node and returns its logits **masked to `A`**
   (`−∞` outside) — the restriction rule of the proposal hook in
   [logit-calibration.md §4](ai-bidder/logit-calibration.md#4-the-design),
   with its epsilon: if the floor puts no finite mass on `A`, or `A` is empty
   (the bands are not a partition and BBA itself leaves them 8% of the
   time), the node **rejects** and the position falls through to the
   unmasked floor exactly as an authored table rejecting a hand does
   ([bidding-architecture.md § Resolution and shadowing](bidding-architecture.md#resolution-and-shadowing--the-invariants)).

Every call's constraint is its band, so **readings come free**: partner's
calls project into `Inferences` through the same `project` fold an authored
rule uses, which means partner reads BBA's meanings and the net's inference
inputs are honest. Alerts come from the dump's alert flag (`a`) with BBA's
label as the text, so `artificial_calls_are_alerted` holds by construction.
Off-walk nodes (deeper than the reach gate, above the ceilings) are the
plain floor on the natural walk — and note what that hands over: the walk
stops at `3NT` in contested auctions, so **the contested 4- and 5-level
decisions belong to our competitive accountant** in this system by
construction, which is the one place the campaign already knows the floor
beats a rule.

Relation to what exists, so nothing is reinvented: a tombstone is a
one-call mask from authoring; the proposal hook is the net restricted to the
book's admissible set; `american_floor()` is the floor with no mask at all.
`BbaBook` is "the tombstone's mask, from data, for every call", with the
hook's restriction and epsilon. No rebase, no guard, no `P*` fan: exact
nodes only, so a rejection always falls through (the simplest case of the
shadowing table).

**As built (2026-10-29).** The classifier is not a new type: it is an
ordinary [`Rules`](../src/bidding/rules.rs) table — one rule per legal call,
weight 0, the band as its constraint, BBA's label as its note and (where BBA
alerts) its alert — with the choice delegated through one new core seam,
`Rules::choose_by(floor)`: the admitted calls take the chooser's logits, every
other call stays at −∞, both on the plain and the compiled classify path. So
every reader (`as_rules`, the projection fold, `may_have_made`,
`explain_call`) sees an authored table and nothing in the reading, alerting
or fall-through machinery had to learn a second kind of node. A table with
no chooser is byte-identical to before (the fold is the identity), which is
the default system's non-inferiority proof for this commit.

### 2.2 The floor under it

`with_floor_v6`, the wiring `american()` uses — same net, same accountants,
same rails, same `Agreements` value for the regime input (defaults; the
compiled book reads no knob). One decision is open (**D1**, §7): the floor
partition floors constructive auctions with `instinct()`, whose natural
ladder may put no mass on `A` at a walked constructive node and so fall
through every time. Default **A**: at walked nodes mask the v6 forward pass
directly, the way the proposal hook calls it (`with_floors` bypassed); keep
the partition off-walk. Alternative **B**: mask `instinct()` and accept the
fall-through rate as a measured number. A is the honest "our judgement
chooses among BBA's calls"; B is the conservative one. Both are a flag.

### 2.3 Arm B — BBA's own precedence (later)

The pure replica: per walked node, weight each child by its empirical
log-probability among BBA's dealt calls that the band admits,
`w(c) = log P(call = c | hand ∈ band(c), node)`, estimated from BBA's pair at
the anchor dumps (409,600 BBA auctions with all four hands per snapshot; the
`dump-teacher` corpus is the bigger source), written as `Rules` weights in
centinats so the book chooses by strict first-max and our floor only acts
off-walk. `bba-B` − `bba-A` on the same deals is then the **choice** term at
equal admissibility — the chat's "judgement nodes written as rules" made
measurable. Build only if arm A reads something (**D4**).

### 2.4 Plumbing

- `--our-floor bba` in `examples/common::seat_floor`, the dump path from
  `BBA_BOOK=<run>` (load behind `serde`; 224 MB JSONL parses in seconds —
  compact to a `.bin` only if the load shows up). `bba-decompose
  --our-floor bba` replays it for the bucket ranking; the mask is a
  deterministic function of the floor, so replay should read 100.00%.
- **Disclosure (D5).** The `bba` pair plays EPBot's defaults, so the
  truthful card is none: `--disclose off`, under which BBA takes us for a
  BBA. `floor_card` currently hard-errors on a seat with no card generator
  (`bba-gen/main.rs:2750`); `bba` needs the `off` path to be legal for it.
- **Where it lives (D2).** `examples/common` first: it is a harness arm, and
  the house rule is that a system is a knob vector, never a second factory
  ([system-variants.md §2](system-variants.md#2-mechanism--a-variant-is-an-agreements-value-not-a-factory),
  the `dutch()` lesson). Promote `BbaBook` into `src/bidding/bba.rs` only if
  a ship needs it there. `Classifier` and whatever else the compile touches
  go `pub` (the expected move, not a concession). Built as
  [`examples/common/bba_book.rs`](../examples/common/bba_book.rs) (the dump
  schema moved there from the walker, the delta fold, `Band`, `BbaBook`,
  `system()`), with two additive core items: `Rules::choose_by` and
  `impl Constraint for Box<dyn Constraint>` (a band is a run-time-arity
  conjunction of `hcp` and `len` primitives, so its projection is the house
  reading of the same facts).
- **Size.** ≈ 1.4 M band constraints (24.7 children per node). At ~100
  bytes per boxed constraint that is ~150 MB and a few seconds to build:
  fine for a harness, never for `web/`.

## 3. Gates, cheapest first — each can stop the plan

**Gate 0 — the admissibility census (no DD, ~1 h, before any classifier).**
Built as `bba-decompose --book-census` (the replay harness already rebuilds
our side, verifies it and buckets the IMPs); read 2026-10-29 in §9.2 —
**the kill rule does not fire.**
Replay the latest anchor snapshot's `bba-gen` dumps (full deals, both
auctions) through `american()` to recover every decision of ours as
`(hand, prefix, call)`, and score each against the re-walked dump:

| column | what it says |
| --- | --- |
| **walked** | the prefix is a node of the dump — the mask's reach |
| **outside** | our call is *not* in BBA's admissible set — where the mask would change our call |
| **BBA-only** | a call BBA admits that no rule of ours admits — where BBA's book is denser |
| **empty** | BBA admits nothing for the hand — the epsilon fall-through rate |

Report per bucket (phase / family, the anchor's keys) with the bucket's IMPs
beside, and the dual on BBA's own calls (should reproduce the 8.15%).
**Kill rule (D3):** stop if the shipping arm's top-three buckets have their
first-divergence nodes walked less than half the time, or `outside` is under
2% of walked decisions — then the mask cannot move the boards that carry the
gap and no DD is owed. Either way the census is the first real
**diff of the two books** and feeds §4 on its own.

**Gate 1 — build, invariants.** The classifier and loader; the default
system byte-identical by construction (a new seat, nothing touched); the
alert invariant over the generated book; one pinned node
(`1♠ -` admits `2♠` on `KJ2.J654.Q87.T93` and reads it 7–9 with 3+♠, admits
`1NT` too, and the mask leaves the choice to the floor); the epsilon
fall-through counted by telemetry. **Done 2026-10-29**:
`examples/common/tests/bba_book.rs` on a three-node fixture cut from the
walk (the pinned node, the fold, the mask, partner's reading of the band and
of the alert, the opponents' pass read off BBA's book as ≤ 14),
`src/bidding/rules/tests.rs` for the seam, and the epsilon is the census's
`empty` column.

**Gate 2 — the A/B** ([measurement.md](measurement.md), `measure-ab`).
Arms `american` and `bba`, fresh `SEED_BASE`, `JOBS=96 BOARDS=204800
DDSS_THREADS=96`, `BBA_GEN` under the loader, arms sequential; both scorers;
both decomposes; the paired `ab-dump-diff`. Built as
[`scripts/ab-bba-system.sh`](../scripts/ab-bba-system.sh) with a third arm,
`bba-ladder` (D1's arm B), so the D1 question is measured rather than
decided; launched 2026-10-29 (§9.3). Reading:

| read | meaning | next |
| --- | --- | --- |
| `bba` non-inferior or better than `american` | BBA's agreements are worth IMPs under our judgement | §4 mining on the paired per-node table |
| `bba` loses to `american` | our agreements beat BBA's under the same judgement; the residual vs BBA is judgement | the search road ([ben-gap-campaign.md](ben-gap-campaign.md), M8); §4 still runs on the buckets `bba` wins |
| `bba` vs BBA, either way | the judgement term at equal rules; compare with −0.266 / −0.049 | if it is most of the gap, retrains and search outrank any book work |

**Gate 3 — arm B** (§2.3), only on a Gate 2 read that leaves the choice
term in doubt.

## 4. What the system is for — the mining loop

`bba` is not a ship candidate. It is an **oracle with a price list**: both
arms replay on the same deals under the same floor, so `bba-decompose` on
each, joined per node, gives `bba − american` in IMPs at every walked prefix
with **the agreement, not BBA's bilans, as the only difference**. The top of
that table is the authoring queue, each row an `author-convention` ticket
that arrives with its price tag and both admissible sets rendered side by
side (`render-book --prefix` for ours, `probe-bba-book --render` for BBA's).
Each winner is then a knob on `american()`, measured by the house method —
the chat's "A/B plus auto-filled nodes", with the pairing done right.

What this adds over today's first-divergence buckets: the anchor attributes
a board to a *node*; this attributes it to an *agreement*, because the
judgement is held constant across the pair. The two earlier census lessons
(the 2/1 suit choice over `1♠`, opener's `2♦` vs `2♣` on 5♦4♣ — "a
first-call census prices BBA's continuations with its call") do not recur
here: the continuations on both arms are ours.

## 5. What the review asked, and what already answers it

| the chat's axis | state |
| --- | --- |
| contested vs constructive share of the loss ("is it over 70% competitive?") | measured every anchor, by phase. Shipping arm at `40847142`: Constructive −56.4k > Competitive −26.6k > Defensive −26.0k. **No** — for the shipped system the constructive book is the largest phase |
| how often we fall off the trie and what it costs | the `floor` provenance: shipping `book` −85.5k vs `floor` −17.5k, so **83% of the loss is at authored nodes**. Coverage is the minority term; §3 Gate 0 measures BBA's *extra* coverage directly |
| overbid / underbid / wrong strain against DD par | **not in the report.** Cheap to add to `bba-decompose` (sign of our level minus par level, strain match); owed, not claimed |
| declaring side vs defending side | per table in the paired diffs; the rails trace at `7e0bc648` showed a bucket's move can sit at the mirror table. Partially answered; the per-table split is one flag away in `ab-dump-diff` |
| plain vs perfect-defense gap | every anchor row: −0.266 plain / −0.049 PD, so **four-fifths of the plain loss is scoring-conditional** — the chat's suspicion that part of the gap is contract types DD prices for BBA is right at anchor scale |
| BBA's 2/1 ≠ our 2/1, agreement vs quality | **this plan** |
| judgement nodes written as rules | arm B − arm A (§2.3) |
| the chart (board class × net IMP) | `report.md`'s bucket tables; `scripts/shrunk-pools.py` for the shrunk node pools |

## 6. Traps, each with its default

1. **Wrong dump column.** The shipped walk is on our card; the arm needs
   `--card none` (§1). Re-walk, don't reuse.
2. **Deltas, not states.** `n` is what the call *changed*; a child's band is
   the cumulative state along the path for that seat. The renderer
   accumulates; the compiler must use the same walk.
3. **HCP vs points.** `h` is HCP (slots 402/403); 404/405 is BBA's total
   points, whose scale is not our `points()`. Default: HCP + lengths +
   stoppers only; the points band behind a flag, admitted only if the
   crosscheck containment rises with it.
4. **The mask too tight.** The epsilon fall-through is the safety; if it
   fires on more than ~10% of walked decisions, widen bands by one HCP
   behind a knob before reading anything. Pass is maskable here (BBA's
   opening pass reads ≤ 11, and a 12-count must open) — unlike the tombstone
   invariant — so the guarantee is *≥ 1 admitted call or fall through*, not
   *Pass always admitted*.
5. **The reading regime under the net** (the 5542 lesson,
   [reading-drift-handoff.md](reading-drift-handoff.md)). The net's auction
   inputs were encoded under `american()`'s readings; under BBA's bands the
   inference block shifts. It is *inside* what Gate 2 measures and shared by
   both of its readings; a matched retrain on a `bba`-read corpus is the
   eventual fix and doubles as the second-system corpus M5.3 waits on
   ([system-variants.md §3](system-variants.md#3-the-floor-is-the-real-cost)
   made the same argument for `basic`). Not owed before Gate 2 reads.
6. **Constructive partition** — D1.
7. **Ceilings.** Above `4♠` constructive / `3NT` contested the walk recorded
   the child's reading but no subtree; the mask applies at the parent, the
   floor continues. Fine, and §2.1 says why it is the right owner.
8. **Opponents' readings in the walk** are BBA reading BBA. In the A/B the
   opponents *are* BBA, so every walked contested auction is in-distribution.
   Our reading of their calls is unchanged (the declared natural walk).
9. **Replay drift.** The arm's correctness is replay = 100.00% in
   `bba-decompose --our-floor bba` (campaign rule 1); any mismatch is the
   mask being non-deterministic (a `HashMap` iteration order in `A`, an
   uncounted epsilon) and is fixed before trusting a bucket.
10. **Reach as the binding limit.** If Gate 0's `walked` column is thin
    exactly where the IMPs are, lower `--min-reach` / raise `--reach-depth`
    first (the walk's cost is the only price), and `MB.TXT`'s generic
    templates are the second source after that.
11. **The bilans' glosses above the ceiling** (found at the pinned node).
    A ceiling child is read but never expanded, and where its label is
    `calculated bid` the reading is the floor's post-hoc explanation of a
    call BBA reaches only by arithmetic: `6♠` over `1♠ -` reads *9+ HCP,
    3+♠* (it admits a 10-count), and `4NT`…`7NT` over `1♣ (3NT)` all copy
    the gambling `3NT`'s band. 151,699 ceiling children are `calculated
    bid`; 72,504 of them floor no suit at five and start at or below 12 HCP.
    Default: **admitted** — a loose admission leaves the chooser exactly as
    free as it is off-book, while dropping them would also take the contested
    four-level, the accountant's own ground, out of every walked node whose
    `4M` BBA reads as its floor's. `BBA_BOOK_NO_CEILING_FLOOR=1` is the
    reversible other arm (`Mask::no_ceiling_floor`), and the census's
    `BBA-only` column is inflated by this class until it is read under it.
    `Not defined` (44 children: `5NT` as an opening and the like) is neither
    a rule nor a reading and is never admitted.

## 7. Decisions owed to jdh8

Taken at their defaults on 2026-10-29 to build; each is one line to
overturn.

| # | decision | default | as built |
| --- | --- | --- | --- |
| D1 | floor at walked constructive nodes: the v6 pass directly (A) or `instinct()` with fall-through (B) | A, both behind a flag | both are `--our-floor` names, `bba` (A) and `bba-ladder` (B), and both are arms of Gate 2 |
| D2 | where `BbaBook` lives | `examples/common`; promote on a ship | `examples/common/bba_book.rs`; the seam `Rules::choose_by` in core |
| D3 | Gate 0 kill threshold | top-3 shipping buckets < 50% walked, or `outside` < 2% of walked decisions | as stated, evaluated by `--book-census`; did not fire (§9.2) |
| D4 | build arm B (BBA's own precedence, §2.3) | only after Gate 2 reads | not built |
| D5 | the `bba` arm's disclosure | `--disclose off` (it plays EPBot's defaults) | `--disclose generated` **is** off for it: `floor_card("bba")` is EPBot's 2/1 with no row moved, so BBA is told the truth through the ordinary path and `bba-gen` needed no `off` special case. The European-minors declaration is skipped for the `bba` floors (their calls decode in BBA's own book) and the dump records it |

## 8. Cost

| step | wall |
| --- | --- |
| re-walk at HEAD, `--card none`, plus its reach corpus | ≈ 10 min with 96 single-threaded shards, after the loader fix |
| Gate 0 census | minutes of replay, no DD |
| Gate 1 build | 1–2 days (compiler, classifier, loader, seat name, decompose replay, tests, runner fix) |
| Gate 2 A/B | 2 arms × 2 vuls × 204,800 boards; the anchor's four arms take ≈ 30 min on a busy box, plus the first DD of the divergent union |

The box is not free today (load ≈ 845 from the M32 relabel in flight,
[CLAUDE.local.md](../CLAUDE.local.md)); nothing in this plan launches until
that run lands.

## 9. Build record (2026-10-29)

### 9.1 The walk

`ab-results/bba-book/2026-10-29-8fb078de-none` (on `/data/public`, the
`ab-results` symlink): `--card none --vuls none --corpus <96 × 1,250
self-play boards, card none> --min-reach 2`, sha `8fb078de`, 96 shards, 23 s;
56,854 nodes, 1,399,897 children, 234 MB. Same shape as the shipped on-card
walk (55,792 nodes): constructive no-rule share 11.9%, contested 13.8%. The
pinned node reads as the plan said: `1♠ -` has `1NT(5-12 ≤3♠)` alerted,
`2♠(7-9 3+♠)` as `calculated bid`, `2♣/2♦/2♥` as 12+ (Strength Lawrence).
`1NT (2♣)` is Multi-Landy in this column (§1's point).

Two facts of the dump the compiler leans on, both measured over every child:
no reading ever moves another seat's public block (`o` is zero throughout),
so a caller's cumulative bands are a fold of its own deltas; and `h` is
already cumulative (it is the caller's HCP band after the call). Stopper
flags take the values `1`, `2`, `−1`, `−2` (217k/178k/15k/50k of the 460k
set); their scale is not ours, so they stay behind `BBA_BOOK_STOPPERS=1`.

### 9.2 Gate 0 — read on the latest anchor snapshot

`bba-decompose … --our-floor american --book-census` over
`ab-results/anchor/2026-10-27-155b5cf8-dirty/american-{none,both}`
(409,600 boards, the shipping arm at the current anchor; replay 100.00%).
Our decisions, scored against the book:

| phase / family | decisions | walked | outside (of walked) | empty (of walked) | divergent boards | net plain IMPs |
| --- | --- | --- | --- | --- | --- | --- |
| Constructive / opening | 741,696 | 100.0% | 2.2% | 0.0% | 53,114 | −22,530 |
| Constructive / round-1 | 196,854 | 100.0% | 13.2% | 0.0% | 25,294 | −12,492 |
| Constructive / round-2 | 368,215 | 97.5% | 19.9% | 5.3% | 37,288 | −18,283 |
| Constructive / deep | 253,877 | 63.6% | 16.3% | 19.5% | 8,486 | −3,092 |
| Competitive / round-1 | 105,048 | 97.4% | 17.5% | 0.0% | 19,656 | −15,170 |
| Competitive / round-2 | 291,797 | 75.4% | 13.4% | 4.9% | 20,664 | −10,965 |
| Competitive / balancing | 129,312 | 42.1% | 7.1% | 5.7% | 3,739 | +932 |
| Competitive / deep | 168,026 | 18.3% | 7.5% | 10.8% | 2,906 | −1,443 |
| Defensive / round-1 | 572,900 | 99.1% | 10.8% | 0.1% | 59,296 | −14,524 |
| Defensive / round-2 | 678,337 | 81.6% | 8.1% | 3.5% | 22,715 | −11,012 |
| Defensive / balancing | 369,177 | 55.8% | 3.9% | 2.9% | 5,336 | +1,599 |
| Defensive / deep | 361,647 | 34.7% | 2.7% | 1.8% | 2,102 | −2,055 |
| **all** | 4,236,886 | **78.4%** | **9.4%** | **2.9%** | 260,596 | −109,035 |

The dual on BBA's own 4,211,841 decisions: walked 78.4%, outside **4.5%** of
walked, empty 1.6% — lower than the 8.15% hand-free crosscheck, which read
deltas on fresh boards at every depth; this reads cumulative bands at the
walked (common) nodes only.

The kill rule (D3) at the top three losing buckets' first-divergence nodes:

| bucket | boards | net plain IMPs | walked | outside (of walked) |
| --- | --- | --- | --- | --- |
| Constructive / book / opening | 53,114 | −22,530 | 100.0% | 16.9% |
| Constructive / book / round-2 | 32,389 | −17,290 | 100.0% | 49.2% |
| Defensive / book / round-1 | 48,149 | −12,671 | 100.0% | 39.5% |

Every divergence node of the three is walked, and the mask would have changed
our call at 17%, 49% and 40% of them; `outside` is 9.37% of walked decisions
overall. **Neither half of the kill rule fires**, so Gate 2 is owed. The
`BBA-only` column of the report (88% of walked-and-authored decisions, 5.3M
extra calls) is dominated by trap 11's ceiling glosses; re-read under
`BBA_BOOK_NO_CEILING_FLOOR=1` it is **53.0%** (2.48M extra calls) — at our
opening decisions 42.1% (BBA's `2NT`/`3NT`/Namyats/`4NT` bands sit beside a
table that admits one call), at defensive round 1 40.2%, at constructive and
competitive round 2 78–80%. That is the chat's "BBA's trie is denser" claim
measured: at four of five walked authored nodes BBA admits a call no rule of
ours does, and whether those calls are worth IMPs is exactly what Gate 2
prices. Under the knob `outside` is 9.8% and `empty` 3.1%; the kill rule
still does not fire.

### 9.3 Gate 2 — read

`scripts/ab-bba-system.sh ab-results/bba-system`, three arms (`american`,
`bba`, `bba-ladder`) × {none, both} × 205,056 boards, `JOBS=96`,
`SEED_BASE=1793256574471532953`, `BBA_BOOK=ab-results/bba-book/2026-10-29-8fb078de-none`,
mask at its defaults. Every arm replays at 100.00% in `bba-decompose`
(campaign rule 1). IMPs/board, 95% CI:

| pair | vul | plain DD | PD | verdict |
| --- | --- | --- | --- | --- |
| `bba` − `american` | none | −0.1965 ±0.0159 | −0.1750 ±0.0180 | loss |
| `bba` − `american` | both | −0.2521 ±0.0201 | −0.2091 ±0.0228 | loss |
| `bba-ladder` − `american` | none | −0.6255 ±0.0184 | −0.6564 ±0.0212 | loss |
| `bba-ladder` − `american` | both | −0.8265 ±0.0235 | −0.8769 ±0.0272 | loss |
| `bba` − `bba-ladder` | none | +0.4368 ±0.0152 | +0.4909 ±0.0177 | win |
| `bba` − `bba-ladder` | both | +0.5955 ±0.0196 | +0.6878 ±0.0228 | win |

Each arm's headline against BBA (`bba-decompose`, the same deals):

| arm | vul | plain DD | PD |
| --- | --- | --- | --- |
| `american` | none | −0.2667 | −0.0379 |
| `american` | both | −0.2695 | −0.0532 |
| `bba` | none | −0.6340 | −0.3635 |
| `bba` | both | −0.7293 | −0.4319 |
| `bba-ladder` | none | −1.4816 | −1.3237 |
| `bba-ladder` | both | −1.8698 | −1.7748 |

**Reading, against the §3 table.** Row 2: `bba` loses to `american`, so our
agreements beat BBA's under the same judgement, and the residual against BBA
is judgement. Row 3: the judgement term at equal rules (`bba` vs BBA) is
−0.63 / −0.73 plain, −0.36 / −0.43 PD — **larger than the whole gap**
`american` carries (−0.27 / −0.04). BBA's bands are not a system; what
chooses among them is the bilans, and swapping our chooser in under its
bands costs more than our book ever lost to it. That is
[bba-book.md §5.6](ai-bidder/bba-book.md#56-the-bookfloor-partition)'s bet
("the axis that separates BBA from us is the choice, not the meaning")
measured, and the chat's "you lose on data, not smarts" refuted at this
resolution. The campaign's next lever stays the floor (retrains, search),
not a bigger book. D1 is answered by the third pair: the net chooser beats
the ladder chooser by 0.44–0.60 plain, so a ladder inside loose bands is
exactly the "natural precedence with no mass in the band" failure §2.2
predicted (the `bba-ladder` arm bids 6NT/7NT at 0.40%/0.23% of its calls
against 0.08%/0.01% for `american`).

**Where the `bba` arm loses, and what that bounds.** Its decompose puts
−244,719 of −279,549 IMPs (88%) at **book** provenance — a walked, masked
node answered the first divergent call — and only −3,295 at the plain
floor; the biggest off-walk bucket is `Constructive / floor#141 / round-2`,
−21,092 IMPs on 4,145 boards (−5.09 per board), one instinct-ladder rule
answering off-walk constructive nodes under BBA's readings (a lead, not
read). So the loss sits where the mask acts, not where the floor continues
unmasked — but **trap 5 is inside the walked nodes too**: the net chooses
with inference inputs encoded from BBA's bands, which it never trained on,
and nothing here splits "the band admitted the wrong set" from "the net
read the band wrong". The split needs the matched retrain on a `bba`-read
corpus (§6 trap 5; the M5.3 second-system corpus), which is now the one
experiment that could still move the agreements term. Trap 11's ceiling
glosses are the other, cheaper arm (`BBA_BOOK_NO_CEILING_FLOOR=1`, owed as
a follow-up cell), and the mask's epsilon is not the cause: `empty` is 0.3%
of the `bba` arm's walked decisions, `outside` 0.4% (the §6 trap-4 threshold
is 10%). Top `bba` buckets vs BBA: `Constructive / book / round-2` −85,250
(−2.07 per divergent board), `Defensive / book / round-1` −38,187,
`Constructive / book / round-1` −33,550, `Constructive / book / opening`
−28,880 — opener's rebid under BBA's 12+ two-over-one and 5–12 forcing
`1NT` bands is where our judgement inside BBA's agreements fails worst.

**§4 mining.** Not run: the paired per-node join (`bba` − `american` at every
walked prefix) is the owed follow-up, and the buckets `bba` wins are the
only ones worth it. The paired diffs' per-board swings are one
`ab-dump-diff --jsonl` away; the join by first-divergence prefix is the
script to write.

### 9.4 Cost, measured

| step | wall |
| --- | --- |
| reach corpus, 96 × 1,250 boards, card none | 7 s |
| walk, 1,125 shards, 96 at a time | 23 s |
| `bba` bind (56,854 tables, the compiled sidecar) | ≈ 5 s per process |
| `bba-gen` with the `bba` floor | ≈ 30 boards/s per process after the bind |
| Gate 0 census, 409,600 boards, replay + census | ≈ 4 min at 96 threads |
| Gate 2: one arm-vul of 205,056 boards at `JOBS=96` | ≈ 2 min |
| Gate 2: one paired `ab-dump-diff` cell, plain + PD | ≈ 3–4 min |
| Gate 2: the three decomposes on a shared DD cache | ≈ 12 min together |
