#!/bin/sh
# ab-landy-tails.sh — §N1m, the three unfinished Landy `(2♣)` tails, one knob,
# one A/B (`competition.landy_tail_completion`).
#
#   JOBS=12 PER_SHARD=192000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-tails.sh ab-results/landy-tails \
#       >ab-results/landy-tails.log 2>&1 < /dev/null &
#
# Three repairs, bundled because each pool alone is far too thin to carry a
# verdict (6 bd, 5 seats, and one catch-all row):
#
#   1. **Their overcall of our minor transfer.**  `2NT (3♥)` / `3♣ (3♠)` and
#      siblings were never registered.  The bucket cut prices `3♣ (3♠)` at
#      6 bd, +1 plain / **−38 PD** — the floor blasting `5♦` over a transfer
#      that promised no values.  `landy_transfer_overcalled` is
#      `landy_cue_overcalled`'s doctrine one seat over: Pass is the default and
#      it is safe, and above it sit only the three calls Pass cannot make.
#   2. **Five floored four-level seats** (the N1j share of the 2026-08-25
#      minor-transfer survey).  Each is a `4m` at a node with `fallback:
#      Some(0)` where the floor cannot keycard, because `instinct`'s `4NT` ask
#      is gated on `Context::undisturbed` and this lane fails that by
#      construction.  Measured on `1NT (2♣) 3♥ - 4♣ -` with `AK54.3.KQ54.KJ54`:
#      floor bids a phantom `4♠`; armed, responder asks keycard.
#   3. **The manufactured `4♣`.**  `landy_bba_ask_answer`'s catch-all names
#      clubs on `hcp(0..)`, so on the no-minor branch opener bids it on a
#      singleton (`9432.AKQ32.K43.A`).  Armed, the last resort is the longer
#      *three*-card minor (clubs first when equal) and that hand bids `4♦`.
#
# Bundling is the known weakness, and measurement.md's split rule is the thing
# to watch: if any slice gives the three different signs they are three arms,
# not one knob.  **Before reading a null as "the tails do not pay", split the
# fired boards by prefix** (`3♣ (3♠)`-family vs `4m`-family vs the `4♣` row) —
# repairs 1 and 3 both *narrow* what we bid while repair 2 *adds* slam
# machinery, and a wash can be repair 2's gain cancelling repair 1's loss.
#
# Repair 2 is also the slam-boundary case: docs/measurement.md's addendum says
# sd-lead is an UPPER bound at slam level (it removes lead pessimism and keeps
# DD's play optimism), so never read sd as slam insurance here.
#
#   base   post-N1k/N1l `main`
#   tail   plus `competition.landy_tail_completion`
#
# Own seed, after §N1k/§N1l.  Gate must read 0 foreign before any headline.
# Resumable; SEED_BASE persists in $R/landy-tails.seed.  Do NOT edit `src/`
# while this runs.
R=${1:?usage: ab-landy-tails.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-tails)
log "=== landy-tails SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base "$v" --filter-landy
    arm tail "$v" --filter-landy --ns-landy-tail-completion

    gatepair tail base "$v"
    diffpair tail base "$v"
    sddiff   tail base "$v"
done

log "landy-tails done"
