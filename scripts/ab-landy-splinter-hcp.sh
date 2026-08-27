#!/bin/sh
# ab-landy-splinter-hcp.sh — §N1m, the Landy `(2♣)` both-minor splinter regrade:
# `3♥`/`3♠` take `hcp(10..)` instead of `points(10..)`.
#
#   JOBS=12 PER_SHARD=192000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-splinter-hcp.sh ab-results/landy-splinter-hcp \
#       >ab-results/landy-splinter-hcp.log 2>&1 < /dev/null &
#
# What is being tested — and it is NOT what §N1k finding 3 wrote down.  The
# splinters are the only branch negative on both scorers (23 bd, −18 plain /
# −32 PD), and 14 of the 17 `3♥` boards are exactly `3♥ - 3NT - - -` from a
# shortness-inflated 8–9 HCP hand: `points(10..)` counts the very shortness the
# call announces, so a 4=1=4=4 nine-count grades to ten.
#
# Finding 3 predicted the demoted hands would double instead.  **They do not.**
# The rung directly below is `landy_bba_responder`'s ungated `3NT`@168, itself
# `points(10..)`, so it catches exactly the same hand — verified on
# `AJ54.3.KJ54.9542`, which bids `3♥` shipped and `3NT` armed, never `X`.  The
# knob does not remove the failing game; it **right-sides** it:
#
#   shipped  `1NT (2♣) 3♥ - 3NT`  opener declares — responder's singleton in
#                                 dummy, the lead running up to it
#   armed    `1NT (2♣) 3NT`       responder declares — the lead comes into 15-17
#
# So the hypothesis under test is the right-siding one, and **a flat result
# refutes right-siding, not the strength claim.**  The strength claim needs a
# wider arm — regrading the `3NT`@168 blast rung on high cards too, which moves
# every shape, not just splinter shapes.  Flagged in §N1m, deliberately not
# taken here; do not silently fold it in on a null.
#
# Second falsifier: this is a right-siding knob, and docs/measurement.md is
# explicit that DD is blind to right-siding *by lead* only in part — the
# opening lead IS double-dummy here, so DD sees the swap but sees it from an
# omniscient leader.  If plain and PD both read flat while sd-lead reads
# positive, the blind-lead bracket is the honest column and the verdict is a
# real (small) win; that is the same 1NT-level lead bias §N1k hit.
#
# Own seed, own experiment: the demoted population overlaps §N1k's and §N1l's,
# so it is measured after both verdicts land, never beside them.
#
#   base   post-N1k/N1l `main`
#   spl    plus `competition.landy_splinter_hcp`
#
# Resumable; SEED_BASE persists in $R/landy-splinter-hcp.seed.  Gate must read
# 0 foreign before any headline.  Do NOT edit `src/` while this runs.
R=${1:?usage: ab-landy-splinter-hcp.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-splinter-hcp)
log "=== landy-splinter-hcp SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base "$v" --filter-landy
    arm spl  "$v" --filter-landy --ns-landy-splinter-hcp

    gatepair spl base "$v"
    diffpair spl base "$v"
    sddiff   spl base "$v"
done

log "landy-splinter-hcp done"
