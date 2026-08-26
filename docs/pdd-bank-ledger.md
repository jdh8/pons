# The `.pdd` deal banks — standing rules and the slice ledger

The banks moved to `/nfs2/jdh8/pons/` on 2026-07-23. They are pre-solved
deals: each row is a deal plus its double-dummy table, so a consumer reads them
with **no solver running**.
That is the whole point — the solver is the expensive part of every experiment,
and the bank is a few gigabytes of it already paid for.

This file is the **canonical ledger**. It was previously kept inside a campaign
doc that has since been archived
(`docs/archive/point-count-threshold-campaign.md`), which left `scripts/*.sh`
pointing at a path that no longer exists.

## The standing rule (every training run, no exceptions)

> **Draw the corpus from `/nfs2/jdh8/pons/`. Test on freshly generated deals.**

Both halves matter, for different reasons:

- **Corpus from the bank** — the DD tables come free, so a corpus regen costs
  dumping time and no solving. `pons::pdd::load` accepts both the binary
  `.pdd` format and GIB text, so `--deals /nfs2/jdh8/pons/24.pdd` works
  anywhere `--deals` does. For a multi-gigabyte bank use `pdd::load_slice`
  behind `--skip`/`--offset` rather than reading the file whole (`24.pdd` is
  2 GB).
- **Test on fresh deals** — never score a model on rows it was trained on. A/B
  harnesses that generate their own deals (`seeded_deals`, fresh `SEED_BASE`)
  satisfy this by construction, at the cost of running the solver live. That
  cost is the price of an honest number.

**Keep `24.pdd` and `22.pdd` byte-stable.** Their generating seeds were not
recorded, and experiments cite them by absolute row offset; deleting,
regenerating, appending, or rewriting either bank invalidates every recorded
slice and prevents exact net reproduction.

### Training draws do not advance the never-replay cursor

Two different disciplines, often confused:

| use | may rows repeat? | advances cursor? |
| --- | --- | --- |
| **training corpus** | yes — overlap between training runs is harmless | **no** |
| **bank-backed A/B slice** (`--deals`/`--offset` scoring) | **no** — replaying deals across experiments correlates their results | **yes** |

Only the second consumes the cursor below. A net trained on rows 0..5M and
another on rows 0..8M is fine; two A/Bs sharing a slice is not.

**But training draws are not free either — they are just constrained
differently.** A slice used to *train* a net must never later be used to
*score that net*: that is train-on-test, and it inflates the result silently.
So training draws do not advance the cursor, but they do have to be recorded,
which is what the register below is for. Evaluating on freshly generated deals
sidesteps this entirely, which is the other reason the standing rule says to.

## ⚠ A known defect in every vs-BBA teacher corpus

Recorded 2026-08-27, **not fixed**, because the fix is a reading knob and a
reading knob is a bidding knob.

[`examples/dump-teacher`](../examples/dump-teacher/main.rs) (`feature_agreements`,
~line 450) forces `decision.reading.their_multi_double_reading = true` for
**every** `vs_bba` corpus, alongside `their_multi_advance_reading`. That knob
is off in the shipped system, and it was written when `1NT (2♦) X` over a
declared Multi was `multi_2d_responder`'s `hcp(6..)` call: it lowers the
reader's flat `DoubleStyle` floor from 8 to 6 so the reading stops asserting
two points responder never promised.

`competition.multi_kokish_kraft` shipped default-on **2026-08-25** and replaced
that whole subtree. The K–K double is authored `hcp(8..)`, so against the
shipped system the flat 8 is now the *correct* hull and the knob publishes
`points 6..` two points **below** the live rule. The defect it was written to
cure has inverted, and the teacher corpora carry the inverted version.

Scope, so this is not over-read: it is confined to `1NT (2♦) X` rows with the
opponents' `2♦` **declared** Multi — a thin slice of any corpus, and absent
entirely from undeclared-opponent draws. Nets trained before 2026-08-25 saw
the knob doing what it was designed to do; nets trained after see it
under-describing the double.

The repair is one line (floor 6 → 8, or read the K–K rule directly, in
`inference::readers::responder_overcall_double_reading`) and is **flagged
rather than applied**: the knob remains unmeasured, its lane measures
`−1.02 plain / +0.67 PD`, and changing what a corpus says about a call changes
what a net trained on it bids. Decide it with an A/B or a deliberate ruling,
not as a drive-by.

## Capacity

| bank | rows (deals) | size |
| --- | ---: | ---: |
| `24.pdd` | **61,698,256** | 2.0 G |
| `22.pdd` | **31,404,048** | 1019 M |
| `shard-*.pdd` × 7 | 7,000,000 | 33 M each |
| **total** | **100,102,304** | |

Row layout is 8-byte `MAGIC` + fixed 34-byte rows (`src/pdd.rs`), so
`rows = (bytes − 8) / 34` — recompute rather than trusting this table if a file
date changes.

## Trained-on register — do not score these nets on these rows

Not a cursor: these rows stay available for A/Bs of *other* nets, and for
further training draws. They are recorded only so a bank-backed A/B never
scores a net on deals it was fitted to.

| bank | rows | fitted models |
| --- | --- | --- |
| `22.pdd` | 0..1,000,000 | evaluator corpora — `evaluator_v2`, `v3`, `v4`, Phase-5 `evaluator_v5_honest` (train 0..450k, held-out 450k..500k), and their `_dnf`/`_exclusion` variants (drawn `--count` from the front: 100k, 400k, 500k and 1M deals across campaigns) |
| `22.pdd` | 2,000,000..2,220,000 | **no fit** — configured-net corpus instrumentation (`dump-teacher --replay`, the 400/20k/20k slices behind the pair-rate numbers in `docs/ai-bidder/configured-net.md`). Recorded so the same rows are not later mistaken for a training draw |
| `22.pdd` | 2,500,000..3,250,000 | `american_bba_v4` — the configured net's mixture corpus (250k uniform + 500k drawn enriched, 3,362,892 rows). Its two gates score on freshly generated deals, never here |
| `22.pdd` | 5,000,000..5,200,000 | **no fit** — `examples/eval-columns`, the per-declarer-column scoring of the *shipped* `evaluator_v3_dnf` (gates 0 and 1 of [`docs/ai-bidder/competitive-accountant.md`](ai-bidder/competitive-accountant.md), measured 2026-08-12). Deliberately past every registered draw so the net is scored on deals it never saw; that is the whole point of the range, so keep the probe's `--skip 5000000` default clear of future training draws |
| `22.pdd` | 3,250,000..4,200,000 | retired `american_bba_v5`, Phase-5 `american_bba_v6` (registered 2026-08-18), and the `v6-their-reading` twin (registered 2026-08-23; deliberate same-deal retrain with BBA's Multi-Landy disclosure and the two parked Multi readers) — the same compact-config mixture regenerated through each extractor: v4-shaped bulk (250k uniform rows 3.25M..3.5M + 500k enriched-draw rows 3.5M..4.0M) plus 8 axis shards (one per top-8 knob axis, 20k deals each, rows 4.0M..4.16M, 2-cell `--replay`; the v5 dump script is retired, `scripts/dump-v6.sh` remains). Gates score on fresh deals, never here |

`24.pdd` has no training draws recorded; its consumption is A/B slices only.

**Drawn 2026-08-03** (was "reserved next"): the configured-net corpus took
**rows 2,500,000..3,250,000** of `22.pdd`, 750k deals — 250k uniform bulk
(8 shards of 31,250, rows 2.5M..2.75M) and 500k drawn for the `--enrich 28:9`
slice (4 shards of 125,000, rows 2.75M..3.25M), of which 24,864 were kept.
3,362,892 rows total. Recorded in the register above as a training draw for
`american_bba_v4`: it advances no cursor, but that net must never be scored on
these rows — both of its gates ran on freshly generated deals.

> **Scope widened 2026-08-05.** `american_bba_v4` is now the floor of
> `american()` *and* `dutch()`, so this constraint no longer binds one opt-in
> factory — **it binds every measurement of the shipped default.** Any A/B or
> probe that seats `american()`/`dutch()` and draws these rows is scoring a net
> on its own training deals. Most harnesses draw `24.pdd` and are unaffected;
> the one to watch is `examples/probe-keycard-reach/main.rs`, which documents
> `22.pdd --count 200000` and builds `american().bind()`. Prefer fresh deals,
> or draw `22.pdd` outside 2.5M..3.25M.

## Slice ledger — `24.pdd`

**Cursor: 42,000,000.** Rows 0..42M are consumed by A/B slices.

| rows | what |
| --- | --- |
| 0..12,300,000 | point-count threshold campaign, stage 1 (archived ledger) |
| 12.3M..38.7M | remnant fixes, two-over-one gate rescale (23M..35M), sd legs 38.5M–38.7M |
| 38.7M..40.8M | weak-two A/B (`scripts/ab-weak-two.sh`, `OFF` default 38,700,000) |
| ~40M..42M | `set_two_over_one_heart_light` (REFUTED 2026-07-25) |

Individually recorded slices inside that range, for cross-reference:
18.3M–20.3M (rebids), 22.5M..23.5M, 2M..3M, 6.1M..7.1M, 0..1M, 24.5M–38.5M.

### ⚠ Remaining: ~19.7M rows, 32% of the bank

At the 1–2M rows an A/B arm typically takes, that is **roughly ten more
bank-backed A/B runs**. Plan for it now rather than discovering it mid-campaign:

- Prefer **fresh generated deals** for evaluation (the standing rule already
  points that way) — it costs solver time but consumes no bank.
- `22.pdd` is nearly untouched: evaluator corpora have drawn ≤1M deals from the
  front, and those were *training* draws, which do not advance a cursor. It is
  **31.4M rows of essentially unused A/B capacity** and is the natural next
  bank when `24.pdd` runs dry. New training draws should start **past row
  1,000,000**, not because the front is spent, but to keep the option of a
  bank-backed A/B for the evaluator nets that were fitted there.
- Generating more bank is possible (`scripts/gib-scavenge.sh`, and
  `docs/shared-machine-data-gen.md` for the fleet) but costs real solver hours.

## After a run

Update the cursor row above in the same commit as the result. A cursor that
lags reality is worse than no cursor — the next experiment silently replays
deals and the two results correlate without anyone knowing.
