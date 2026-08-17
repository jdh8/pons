#!/bin/sh
# ab-bid-exclusion.sh — Phase 4 of docs/authored-reading-handoff.md.
#
# `reading.bid_exclusion` generalises the pass-side negative inference to every
# made bid.  Selection is argmax over legal calls with finite logits and a book
# logit is `weight/100` or −∞, so "the table made C through rule i" says exactly
# that every sibling rule strictly heavier than i was FALSE — and C's reading may
# be intersected with those siblings' complements.  Per-rule threshold; a sibling
# counts only when it is face-live and was legal at that turn.
#
# The payoff is the book's 148 non-Pass `hcp(0..)` catch-alls, which read ⊤ today
# and hand the call back to the natural walk: `1♥ - 2NT - 4♥` reads opener
# `points 16..21` off the walk, wrong for every hand that bid it.
#
# Two arms per vul, identical deals:
#   off   --ns-bid-exclusion=false (the pre-2026-08-17 default; the knob
#         SHIPPED default-on on the run below, so the off arm now spells it)
#   bex   --ns-bid-exclusion
#
# MEASURED 2026-08-17 (sha 0cf06c67, seeds 1786968900/1786969295/1786969688,
# 204,800 bd/arm/vul): wash on both scorers at both vuls in every cell — 93
# diverging boards in 1,228,800, +36 plain / +51 PD pooled — the pre-registered
# non-loss; shipped default-on.  docs/authored-reading-handoff.md § Phase 4.
#
# The frozen nets are shielded by `legacy_view` (default on; `context.rs` folds
# the knob off in the legacy clone), so this pair prices the SAMPLER +
# AUTHORED-GATE + INSTINCT channel, not the net encoding.
#
# PRE-REGISTERED (jdh8, before the numbers): this is a soundness/information
# correction, so it takes the reading-drift bar — **non-loss ships it**
# default-on (plain wash-or-win AND PD non-loss, both vuls).  A plain win with a
# PD loss is a doubling artifact and does not ship.  Any loss gets
# `ab-dump-bucket --by lane` and its worst divergent boards traced before a
# conclusion.
#
#   setsid nohup scripts/idle-run.sh scripts/ab-bid-exclusion.sh \
#       ab-results/p4-bid-exclusion >ab-results/p4-bid-exclusion.log 2>&1 </dev/null &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: ab-bid-exclusion.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== Phase 4 bid-exclusion start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --ns-bid-exclusion=false
    arm bex "$vul" --ns-bid-exclusion
    diffpair bex off "$vul"
done
log "=== Phase 4 bid-exclusion done"
