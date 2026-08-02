# The `.pdd` deal banks — standing rules and the slice ledger

The banks at `/nfs2/jdh8/pons/` are pre-solved deals: each row is a deal plus
its double-dummy table, so a consumer reads them with **no solver running**.
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

**Keep `24.pdd` and `22.pdd` byte-stable.** Experiments cite them by row
offset; appending or rewriting invalidates every recorded slice in this repo.

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
| `22.pdd` | 0..1,000,000 | evaluator corpora — `evaluator_v2`, `v3`, `v4` and their `_dnf`/`_exclusion` variants (drawn `--count` from the front at `--seed 1`: 100k, 400k, 500k and 1M deals across campaigns) |
| `22.pdd` | 2,000,000..2,220,000 | **no fit** — configured-net corpus instrumentation (`dump-teacher --replay`, the 400/20k/20k slices behind the pair-rate numbers in `docs/ai-bidder/configured-net.md`). Recorded so the same rows are not later mistaken for a training draw |

`24.pdd` has no training draws recorded; its consumption is A/B slices only.

**Reserved next:** the configured-net corpus will draw ~750k deals from `22.pdd`
past row 2.5M — ~250k uniform bulk plus ~500k drawn for the `--enrich 28:9`
slice, of which ~24k are kept. Log the exact window here when it is dumped; it
is a training draw, so it advances no cursor but must never score that net.

## Slice ledger — `24.pdd`

**Cursor: 42,000,000.** Rows 0..42M are consumed by A/B slices.

| rows | what |
| --- | --- |
| 0..12,300,000 | point-count threshold campaign, stage 1 (archived ledger) |
| 12.3M..38.7M | remnant fixes, two-over-one gate rescale (23M..35M), sd legs 38.5M–38.7M |
| 38.7M..40.8M | weak-two A/B (`scripts/weak-two-ab.sh`, `OFF` default 38,700,000) |
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
