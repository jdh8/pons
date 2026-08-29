# BBA self-play tree after `1NT (2♣)` Landy

Run date: **2026-08-30**.

This is the coherent four-BBA-player companion to the fixed-seat probes in
[`bba-1nt-counter-defense.md`](bba-1nt-counter-defense.md) and the source
survey in
[`landy-2c-counter-defense-research.md`](landy-2c-counter-defense-research.md).
All four seats use the same BBA engine and convention state.  The root is the
exact auction `1NT (2♣)`, with `(2♣)` disclosed by BBA as Multi-Landy,
both majors.

The exhaustive machine-readable tree and its rendered form stay outside git:

```text
ab-results/bba-book/2026-08-30-4fdb9633-landy2c-bba-selfplay/tree.jsonl
ab-results/bba-book/2026-08-30-4fdb9633-landy2c-bba-selfplay/tree.txt
```

This document records the durable results, especially what opener's and
responder's later doubles mean.  It does not turn BBA's black-box behavior into
a recommendation for pons.

## Result in brief

1. BBA still never used responder's direct `X`: **0/4,074** root auctions.
   The interpreter calls that unchosen action `bidable suit`, not penalty or
   values, and projects 5-11 total points with four-plus clubs.
2. After `1NT (2♣) - (2M)`, opener's immediate `X` is formally **takeout**,
   exact 17 with 4-4 minors.  BBA chose it **0/798** over `(2♥)` and
   **0/834** over `(2♠)`.
3. After opener also passes, responder's delayed `X` is a genuine
   **reopening double**.  It occurred 71/757 after hearts and 90/817 after
   spades.  It is short in the chosen major and passable: after advancer passed,
   opener sat only 33/154 times and otherwise pulled chiefly to a minor.
4. Apart from those early special cases, every observed opener/responder `X`
   was either `penalty` or a natural/lead-directing `bidable suit` double of
   diamonds. An observed penalty double behaved like one: responder sat
   119/125 opener doubles after the next opponent passed; opener sat all 62/62
   responder doubles.
5. Observed `XX` was strength-showing, not a request to play.  Opener's
   `surplus` redouble preserved the diamond transfer; responder's `surplus`
   redouble was followed by opener bidding a minor.

The coherent corpus changes the direct-call shares from the older fixed-seat
random-hand probe because both the 1NT opening and Landy overcall actually
occurred on the same deal.  Among those deals, pass was the majority response,
not `3NT`.

## Run and completeness

The run used commit `4fdb9633dee0f154f6caca24fd43f6a30a2ebd97` and a
frozen dirty binary.  The only diff added `probe-bba-book --no-ceiling`; the
diff and binary are preserved beside the result, and the binary's SHA-256 is
`239e081a701c457e1398b3d21ba66833849d5a937ffbd8eadcfcfd54aecb9ee9`.

| Item | Value |
| --- | ---: |
| seed base | 1,788,028,093 |
| corpus | 24 shards × 192,000 = **4,608,000** boards |
| workers | 12 initially; raised to **24** after 11 shards completed |
| scheduler | `scripts/idle-run.sh`; first shard started at load1 0.21 (<10) |
| cards | `none` on all four seats |
| overrides | `Multi-Landy=1`, `Cappelletti=0`, `Landy=0` |
| dealer/vulnerability | dealer and all four vulnerabilities rotate evenly |
| root reach | 4,074 auctions (0.0884% of boards) |
| reach gate | non-calculated children are exhaustive through six calls; calculated children always require corpus reach |
| deeper expansion | every expanded child had corpus reach of at least one |
| focused observed depth | 22 calls; every observed continuation reached an auction end |
| emitted walk nodes | 40,333 |
| observed expanded nodes | 1,929 |
| observed child edges | 2,383, of which 455 end the auction |
| dangling render nodes | 0 |

The top-level log contains one `cut` warning from formatting the load line in
the temporary runner. The actual gate used `awk`, passed at load1 0.21, and the
warning did not touch a shard or artifact.

No corpus-observed child stopped at `calc`, `reach`, `ceil`, or `depth`; the
only observed stop verdict was `end`.  The walk used a safety depth of 60, well
past the deepest reached node.  The dump contains 344,152 book readings and
41,912 calculated-floor readings, so **10.9%** of the candidate surface is
floor-owned.

“Full tree” therefore means every route that 4.608 million coherent self-play
deals actually took, all the way to auction end, plus the hand-free book
surface around it.  It does not mean expanding every unreached
calculated-floor child: those children are corpus-gated even inside the
six-call reach horizon, and their hypothetical pass/bid tree is effectively
unbounded.

The reach corpus aggregates the four vulnerabilities.  Meanings and constraint
projections in this report come from the walk's `--vuls none` reading.  The run
does not prove that those readings or action shares are vulnerability-invariant.

## Direct responder tree

### Every direct candidate

Reach is conditional on the 4,074 corpus auctions that reached `1NT (2♣)`.
Zero-reach rows are real hand-free BBA rules, but counterfactual in this corpus.

| Responder | Reach | Share | BBA reading in the nonvulnerable walk |
| --- | ---: | ---: | --- |
| `-` | 2,361 | 58.0% | 0-9 total points |
| `2♦` | 0 | 0 | `bidable suit`; 4-9, 5+♦ |
| `2♥` | 0 | 0 | generic `artificial`; no useful constraint |
| `2♠` | 239 | 5.9% | transfer to clubs; 5-16, 6+♣, at most four cards in either major |
| `2NT` | 445 | 10.9% | 8-9, both major stoppers |
| `3♣` | 275 | 6.8% | transfer to diamonds; 5-16, 6+♦, at most four cards in either major |
| `3♦` | 65 | 1.6% | `bidable suit`; exactly 4, 6+♦ |
| `3♥` / `3♠` | 0 | 0 | generic `artificial`; no useful constraint |
| `3NT` | 604 | 14.8% | 9-15, both major stoppers |
| `4♣` | 0 | 0 | `bidable suit`; 7-12, 6+♣ |
| `4♦` | 0 | 0 | Texas to hearts |
| `4♥` | 0 | 0 | Texas to spades |
| `4♠` | 77 | 1.9% | `Minors`; 5+♣, 5+♦, at most three cards in either major |
| `4NT` | 0 | 0 | 8-16, both major stoppers |
| `5♣` / `5♦` | 4 / 4 | 0.1% each | natural six-card minor, 6-16 |
| `5♥` / `5♠` | 0 | 0 | generic `artificial`; no useful constraint |
| `5NT` | 0 | 0 | 11-16, both major stoppers |
| `6♣` / `6♦` | 0 | 0 | natural six-card minor, 7-16 |
| `6♥` / `6♠` | 0 | 0 | generic `artificial`; no useful constraint |
| `6NT` | 0 | 0 | 14-16, both major stoppers |
| `7♣` / `7♦` | 0 | 0 | natural six-card minor, 10-16 |
| `7♥` / `7♠` | 0 | 0 | generic `artificial`; no useful constraint |
| `7NT` | 0 | 0 | exactly 16, both major stoppers |
| `X` | 0 | 0 | `bidable suit`; 5-11, 4+♣ |

The formal stopper gates on direct `2NT` and `3NT` are new evidence.  They are
stronger than the older note that the fixed-seat probe did not establish a
stopper condition.

### All observed first continuations

This is the material topology through opener's next call.  Counts are corpus
reaches; omitted legal calls had zero reach.  Opponents' calls are
parenthesized.

| Responder | Advancer | Opener's observed actions |
| --- | --- | --- |
| `-` | `-` 199 | `-` 198, `2♦` 1 |
| `-` | `(2♦)` 324 | `-` 274, `X` 50 |
| `-` | `(2♥)` 798 | `-` 795, `3♦` 2, `3♣` 1 |
| `-` | `(2♠)` 834 | `-` 828, `3♦` 6 |
| `-` | `(2NT)` 20; `(3♣)` 3; `(3NT)` 1; `(3♥)` 114; `(4♥)` 34; `(4♠)` 32; `(4NT)` 2 | opener passed every time |
| `2♠` | `-` 194 | `3♣` 194: transfer completed every time |
| `2♠` | `(3♦)` 5 | `-` 3, `4♣` 2 |
| `2♠` | `(3♥)` 23 | `4♣` 15, `-` 7, `5♣` 1 |
| `2♠` | `(3♠)` 17 | `4♣` 12, `-` 5 |
| `2NT` | `-` 423 | `-` 327, `3NT` 94, `3♣` 1, `3♦` 1 |
| `2NT` | `(3♥)` 8 | `X` 3, `3NT` 2, `-` 3 |
| `2NT` | `(3♠)` 12 | `X` 7, `3NT` 2, `-` 3 |
| `2NT` | `(4♠)` 2 | `X` 2 |
| `3♣` | `-` 194 | `3♦` 194: transfer completed every time |
| `3♣` | `(X)` 25 | `3♦` 17, `XX` 4, `-` 4 |
| `3♣` | `(3♥)` 25 | `4♦` 15, `-` 7, `X` 3 |
| `3♣` | `(3♠)` 25 | `4♦` 18, `-` 6, `X` 1 |
| `3♣` | `(4♥)` 2 | `-` 2 |
| `3♣` | `(4♠)` 4 | `X` 2, `-` 2 |
| `3♦` | `-` 27 | `-` 27 |
| `3♦` | `(3♥)` 14 | `-` 8, `4♦` 6 |
| `3♦` | `(3♠)` 19 | `-` 11, `4♦` 7, `X` 1 |
| `3♦` | `(4♥)` 3 | `5♦` 1, `X` 1, `-` 1 |
| `3♦` | `(4♠)` 1 | `5♦` 1 |
| `3♦` | `(X)` 1 | `-` 1 |
| `3NT` | `-` 591 | `-` 591 |
| `3NT` | `(4♥)` 6; `(4♠)` 7 | `X` on all 13 |
| `4♠` | `-` 76 | `5♣` 39, `5♦` 35, `4NT` 2 |
| `4♠` | `(X)` 1 | `5♣` 1 |
| `5♣` / `5♦` | `-` 4 / 4 | opener passed all eight |

After the direct pass and a chosen major, responder's material second-turn
tree was:

| Auction before responder's second turn | Reach | Responder's actions |
| --- | ---: | --- |
| `1NT (2♣) - (2♥) - -` | 757 | `-` 491, `3♣` 100, `3♦` 80, `X` 71, `2NT` 8, `3NT` 4, `5♣` 3 |
| `1NT (2♣) - (2♠) - -` | 817 | `-` 522, `3♣` 117, `3♦` 84, `X` 90, `2NT` 3, `3NT` 1 |

## Double and redouble ledger

The raw ledger is exhaustive for opener and responder:

```text
ab-results/bba-book/2026-08-30-4fdb9633-landy2c-bba-selfplay/opener-responder-doubles.tsv
ab-results/bba-book/2026-08-30-4fdb9633-landy2c-bba-selfplay/observed-opener-responder-doubles.tsv
```

The first file contains the exact auction, actor, call, node reach, call reach,
conditional share, stop verdict, label, and full reading for **31,263** `X`/`XX`
candidates. The second is the compact 133-row observed inventory, enriched with
constraints, feature deltas, provenance, and partner's next action. The
following partition accounts for every row in the exhaustive file:

| Status | Rows | Meaning |
| --- | ---: | --- |
| observed call | 133 | call was chosen at least once; 418 total `X`/`XX` actions |
| observed node, unchosen call | 413 | live position, counterfactual `X`/`XX` |
| unreached node | 30,717 | wholly counterfactual rule-tree position |

The 413 reachable but unchosen calls split as follows: opener `X` has 113
`penalty`, 48 `takeout double`, four `bidable suit`, and one keycard answer;
opener `XX` has 37 `strong` and two keycard/cue readings; responder `X` has
158 `penalty`, five `takeout double`, and seven `bidable suit`; responder `XX`
has 36 `strong` and two `surplus`.  The 30,717 unreached rows are preserved in
the TSV but are not evidence that BBA would ever create those auctions.

### The early doubles that answer the design question

| Exact auction | Actor | Reach and use | BBA meaning | Partner semantics |
| --- | --- | --- | --- | --- |
| `1NT (2♣) X` | responder | 0/4,074 | `bidable suit`; 5-11, 4+♣, at most six diamonds | no live continuation; every later branch is counterfactual |
| `1NT (2♣) - (2♦) X` | opener | 50/324 | `bidable suit`; 15-17, exactly five diamonds, 2-4 clubs, 2-3 in each major | the next opponent always ran: `(2♥)` 16, `(2♠)` 19, `(2NT)` 15; responder never got a conversion decision |
| `1NT (2♣) - (2♥) X` | opener | 0/798 | `takeout double`; exact 17, 4-4 minors | no observed partner action |
| `1NT (2♣) - (2♠) X` | opener | 0/834 | `takeout double`; exact 17, 4-4 minors | no observed partner action |
| `1NT (2♣) - (2♥) - - X` | responder | 71/757 (9.4%) | `reopening double`; 5-9, 0-2 hearts, 1-4 spades, 2-5 in each minor | after advancer passed: opener passed 20/70; pulled to `3♣` 26, `3♦` 17, or `2NT` 7 |
| `1NT (2♣) - (2♠) - - X` | responder | 90/817 (11.0%) | `reopening double`; 5-9, 0-2 spades, 1-4 hearts, 2-5 in each minor | after advancer passed: opener passed 13/84; pulled to `3♣` 41, `3♦` 24, `2NT` 5, or `3NT` 1 |
| `1NT (2♣) X (2♥) X` | opener | root `X` never reached | `takeout double`; exact 17, four diamonds, 2-3 clubs | counterfactual; no partner evidence |
| `1NT (2♣) X (2♠) X` | opener | root `X` never reached | same `takeout double` | counterfactual; no partner evidence |

The direct-pass lane therefore has two distinct meanings.  Opener's immediate
double of a chosen major is a very narrow takeout action that never occurred;
responder's later double is a regularly used reopening action that opener may
convert but usually pulls.  Neither is BBA evidence for a routine penalty
double by opener.

The `(2♦)` double is natural or lead-directing over advancer's artificial ask:
BBA calls it `bidable suit` and projects exactly five diamonds. The overcaller
always ran, so this corpus does not expose responder's conversion decision.

### Every observed semantic class and what partner did

This table partitions all 133 observed `X`/`XX` nodes.  “After pass” means the
next opponent passed; later competition is excluded from the partner column.

| Actor and call | Label | Nodes | Calls | Partner continuation after the next opponent passed |
| --- | --- | ---: | ---: | --- |
| opener `X` | `bidable suit` | 2 | 53 | only one opponent pass; responder sat once |
| opener `X` | `penalty` | 78 | 126 | 125 opponent passes: responder sat 119, pulled to `4♣` twice, `5♣` once, `5♦` three times |
| opener `XX` | `surplus` | 1 | 4 | responder bid `3♦` all four times, completing the transfer after `1NT (2♣) 3♣ (X) XX -` |
| responder `X` | `reopening double` | 2 | 161 | 154 opponent passes: opener sat 33, bid `3♣` 67, `3♦` 41, `2NT` 12, `3NT` 1 |
| responder `X` | `penalty` | 48 | 69 | 62 opponent passes: opener sat all 62 |
| responder `XX` | `surplus` | 2 | 5 | opener bid `4♣` twice and `4♦` three times |

The material early penalty examples are straightforward:

- `1NT (2♣) 2NT (3♥) X` was penalty and occurred 3/8; the spade twin
  occurred 7/12, and `1NT (2♣) 2NT (4♠) X` occurred 2/2.
- `1NT (2♣) 3NT (4♥) X` and its spade twin occurred on all 13 reaches.
- After either minor transfer, opener's doubles of opponents' natural suits
  and responder's later doubles were overwhelmingly labelled penalty.  The
  exact low-frequency tails remain in the TSV rather than being paraphrased
  into a fictitious compact convention.

## Discrepancies and limits

- The older fixed-seat research says BBA supplied no delayed-double path.  The
  coherent tree supplies one: responder's delayed `X` after a chosen major is
  explicitly `reopening double` and occurred 161 times.  That old statement is
  stale.
- The older actor-only root frequencies are not live estimates: `3NT` falls
  from 49.7% to 14.8% and Pass rises from 26.8% to 58.0%. Its `6NT` and `7NT`
  buckets disappear because a real 15–17 opener plus a 9+ Landy overcaller
  leave at most 16 HCP across responder and advancer together.
- The earlier `3NT` note said its probe did not establish a formal stopper
  condition.  The hand-free reader here requires both major stoppers for
  direct `2NT` and `3NT`.  This is a reader/probe discrepancy, not permission
  to silently choose either statement for pons.
- BBA's direct `X` is neither a conventional penalty proposal nor the values
  double currently authored by pons.  It is an unobserved club-suit action in
  this engine state.
- Generic unobserved `artificial` calls project 0-37 points and no shape.  They
  are interpreter fallbacks, not usable agreements.  Likewise, the thousands
  of unreached high-level double/keycard readings are retained for audit but
  do not describe live Landy counter-defense.
- Reach counts combine all vulnerabilities, while readings were rendered only
  nonvulnerable. A vulnerability-split behavior report needs a
  corpus format that retains vulnerability; this run cannot recover it after
  aggregation.
- This is BBA playing all four hands, not pons against BBA.  It answers what
  BBA's opener and responder do with BBA partners; it must not be used to label
  pons's calls.

## Reproduction

Use the frozen binary in the run directory to reproduce the exact dirty-tree
result.  One shard is:

```sh
run=ab-results/bba-book/2026-08-30-4fdb9633-landy2c-bba-selfplay
i=0
seed=$((1788028093 + 192000 * i))
"$run/probe-bba-book" \
  --card none \
  --conv 'Multi-Landy=1' \
  --conv 'Cappelletti=0' \
  --conv 'Landy=0' \
  --selfplay 192000 \
  --seed "$seed" \
  --output "$run/corpus/reach-$(printf '%02d' "$i")-$seed.jsonl"
```

Repeat for `i=0..23`; process concurrency does not change the deterministic
deal streams.  Then walk and render the focused tree:

```sh
run=ab-results/bba-book/2026-08-30-4fdb9633-landy2c-bba-selfplay
"$run/probe-bba-book" \
  --card none \
  --conv 'Multi-Landy=1' \
  --conv 'Cappelletti=0' \
  --conv 'Landy=0' \
  --prefix '1NT (2♣)' \
  --vuls none \
  --corpus "$run/corpus" \
  --min-reach 1 \
  --reach-depth 6 \
  --max-depth 60 \
  --extended \
  --no-ceiling \
  --output "$run/tree.jsonl"

"$run/probe-bba-book" \
  --render "$run" \
  --prefix '1NT (2♣)' \
  --verbose >"$run/tree.txt"

"$run/probe-bba-book" --stats "$run" >"$run/stats.txt"
```

Before trusting a different engine/card state, re-run the effective-convention
audit and verify that Multi-Landy plays on while Cappelletti and Landy play
off.  The preserved `effective.txt`, `probe-bba-book.diff`,
`probe-bba-book.sha256`, `RUN`, `SEED_BASE`, and `stats.txt` are the provenance
record for this run.
