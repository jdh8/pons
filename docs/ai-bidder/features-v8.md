# features_v8 — the artificial-call block, and the label transplant

**Status (2026-09-26).** Built and trained; the A/B against the shipped M32 v6
floor (`scripts/ab-v8-floor.sh`, `ab-results/v8-floor`, seed 1790371926,
204,800 bd/arm/vul) is **in flight**. Opt-in as `--our-floor american-v8` /
`american_v8()`; `american()` still ships v6. Verdict goes in §5 and in
[../floor-rail-campaign.md](../floor-rail-campaign.md) (stop criterion 5).

## 1. What v8 is

`features_v6` (176 floats) sets the we-bid-this-strain bit for artificial
calls too, so a transfer `2♠` and a natural `2♠` reach the net as the same
float. Every rail of the floor-rail series patched that blindness on the
output side; this is the input-side fix
([new-suit-veto.md](new-suit-veto.md) §2 has the diagnosis).

v8 = the v6 vector **verbatim** (the compact regime input, the fold and every
v6 probe stay comparable) + 12 values (`LEN_ARTIFICIAL`), read off the
`Inferences` per-call masks the walk already builds (`call_artificial`):

| slot | meaning |
| --- | --- |
| 0–4 | our side named this strain **only** through alerted calls (never naturally) |
| 5–9 | the same for their side |
| 10 | the last bid (v6's contract-to-beat) was artificial |
| 11 | partner's last bid was artificial |

"Only artificially" rather than "ever artificially": a natural call in the
strain, before or after, clears the bit, which is the v6 raw bit minus the
phantom suits — the signal the rails needed. Code: `features_v8` /
`push_artificial` in `src/bidding/features.rs`; serving through
`ConfiguredFloorV6::new_v8` (the shell now pairs extractor and net in one
`fn(Hand, &Context) -> Logits`, so a blob can never meet the wrong width);
`neural::classify_bba_v8`; `american_v8()`; `--feature-version 8` in
`dump-teacher`; `8` in the trainer's supported versions.

## 2. Labels without a relabel — the transplant

The decision (jdh8, 2026-09-26): no M32 rollout re-price ("anything but the
megachonk"); reuse the M32 labels. They live per row in the cut stems
(`target/corpus-relabel-m32/<shard>`), so the job is a **row-aligned splice**
onto a fresh v8 dump of the same recipe. Three things had to be true, and each
was checked rather than assumed:

1. **The walk must be the fleet's walk.** A plain `scripts/dump-v6.sh` dump
   does *not* align: the relabel walk seeds each board's dealer/vul from a
   different stream, and only ~6% of rows coincide (chance agreement of
   dealer × vul). The aligned re-walk is `scripts/relabel-worker.sh` with
   `LAYOUTS=0` and `DUMP_COMMON="… --feature-version 8"` — the fleet's
   188 chunk windows, zero layouts drawn, so no double dummy at all. Proved on
   `axis-0004/chunk-0`: `dump-teacher --diff` reads 105,640 rows, labels
   moved 0, features moved 0.12% (reading drift), decisions entered/left 0.
   20 workers (`STRIDE=20 OFFSETS=k`) under `idle-run` walked all 188 chunks
   in **10 minutes**.
2. **Positional where the chunk is byte-aligned, keyed where it drifted.**
   `scripts/splice-labels.py NEW_CHUNKS OLD_CHUNKS CUT_STEM OUT_STEM`: a
   chunk whose one-hot, DD and `.tags` equal the fleet chunk's is spliced
   positionally; otherwise rows are matched on the **reading-free key**
   (hand block, auction context, vulnerability, one-hot, DD) and unmatched
   rows keep BBA's one-hot. The inference block is excluded (readings drift
   with the book) and so is the compact block — slot 0 (`wide_one_club`)
   flipped on every row since the fleet ran. All 12 axis/enriched shards
   aligned 100% positionally; the 8 uniform shards key-matched **84–85%**:
   their per-board cell sampling drew from six cells at `39318c7f` including
   the since-retired `dutch-*` ones, and today's walk has three, so the rows
   on retired cells cannot be reproduced and stay on the teacher's label.
3. **The stale-label caveat, stated.** A transplanted label is an M32 rollout
   verdict under the book of 2026-09-13; the corpus features are the reading
   of today's book. The row's *auction* is identical (that is what the key
   asserts), only its reading moved on 0.12% of rows. Labels transplanted:
   ≈424k of M32's 460,341 (the uniform shortfall).

## 3. Training and export

Manifest command (M32's, `--data` on the spliced stems):
`pons-trainer --arch mlp --cuda --hidden 256 --epochs 300 --lr 0.001 --wd 0
--batch 4096 --val-frac 0.10 --dd-weight 0 --init-seed 1`, 9 min 42 s end to
end including the splice. Then `scripts/fold-constant-inputs.py` over the
20 stems.

| | M32 v6 (shipped) | v8 |
| --- | ---: | ---: |
| rows / train / val | 6,766,821 / 6,090,135 / 676,686 | 6,763,868 / 6,087,477 / 676,391 |
| held-out CE | 0.4163 | **0.3959** |
| top-1 overall / constructive / contested | 85.3 / 89.1 / 83.0% | **86.0 / 89.9 / 83.7%** |
| fitted `T` (diagnostic; serving is raw) | 1.0915 | 1.0795 |
| folded columns | 30 | 32 — all compact-config slots; **every one of the 12 new columns is live** |

Held-out here is the trainer's contiguous 10% tail on the same deals as
training (the shipped corpus's own gate); the A/B is on fresh deals, as
always. A better fit to the same labels is necessary, not sufficient.

## 4. Pipeline, for the next retrain

```sh
# 1. re-walk the fleet chunks with the new extractor (10 min on 20 cores)
for k in $(seq 0 19); do OUT=$CHUNKS STRIDE=20 OFFSETS=$k LAYOUTS=0 \
  DUMP_COMMON="--deals /nfs2/jdh8/pons/22.pdd --teacher bba --configured --feature-version 8" \
  setsid nohup scripts/idle-run.sh scripts/relabel-worker.sh >$CHUNKS/worker-$k.log 2>&1 </dev/null & done
# 2. splice, per shard
python3 scripts/splice-labels.py $CHUNKS/$s $FLEET/$s target/corpus-relabel-m32/$s $OUT/$s
# 3. train (manifest command), 4. fold, 5. copy to src/bidding/weights/, 6. A/B
```

Any future feature bump rides the same three steps; a future *relabel* is the
fleet-week this document exists to avoid.

## 5. Verdict — SUSPECT, not shipped (2026-09-26)

`ab-results/v8-floor`, seed **1790371926**, 204,800 bd/arm/vul,
`v8` vs `plain` (the shipped M32 v6), fired 15.6% (none):

| vul | plain DD | PD | sd plain | sd PD |
| --- | ---: | ---: | ---: | ---: |
| none | **+0.0171 ±0.0105** | **−0.0127 ±0.0121** | **+0.0353 ±0.0109** | +0.0091 ±0.0122 |
| both | **+0.0152 ±0.0126** | −0.0076 ±0.0144 | **+0.0331 ±0.0130** | +0.0134 ±0.0145 |

Decision table row: *plain win, PD erases it* — "reaching contracts a
competent doubler would slaughter; suspect, don't ship on this evidence."
The single-dummy bracket is positive on both scorers (CI-clear on plain), so
this is not a refutation either: the arm **stays opt-in**, `american()`
unchanged, the rails stand, and the net's parameters stay fixed for R4/R5.

**Worst-board trace (none, PD, 40 boards, −18 to −24 each).** Not one
mechanism but a family the rail series already knows — junk *action* by the
floor in contested auctions past game: a 5♥/5♦ bid over *their* 4NT
(Blackwood) or 5♣ where v6 passed, a late double of their making game or slam
(`4♠ - - X`, `3♠ - - X`), and redoubles at the five level (`5♦ X XX`,
`4♥ X XX`; the arm makes 8,065 redoubles to plain's 7,506, and 645 auctions
end on one vs 412). The plain-DD column does not price these (the doubled
contract makes double-dummy or the sacrifice is cheap undoubled); PD does.
The artificial block plausibly *causes* the slam-auction slice: their 4NT
now reads as artificial (slot 5–10), an input the M32 labels almost never
exercise, so the net is off-distribution there.

**Next levers, in order.** (1) A rail for the new slice, since a rail is
cheap and the vein is 3/3 on narrow gates: *our silent side acts over their
keycard ask* (`4NT` artificial by their reading, we have not bid); price it on
the v8 arm's own decompose before authoring. (2) The lost 15% of uniform rows
(retired Dutch cells) — re-sample those boards on today's cells and label them
with BBA only. (3) A relabel at HEAD — the fleet-week — which is the honest
fix for labels that never saw the new inputs.
