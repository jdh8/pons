#!/bin/sh
# ab-bws.sh — price the Bridge World Standard 2017 preset (docs/bws-2017.md).
#
# Three arms per cell, identical deals, all vs BBA on its 2/1 card, each arm's
# own card generated from its agreements:
#   advanced  the default as measured: since 2026-11-27 it passes
#             `--no-ns-semiforcing-notrump`
#   bws       `--ns-preset bws`: every knob at the state BWS 2017 prescribes
#             where that state is authored, same floor
#   sf        the one knob authored for the preset, alone
#             (`rebid.semiforcing_notrump`: opener passes `1M - 1NT` on a
#             5-3-3-2 or 4=5=2=2 with at most 12 HCP); the default since
#             2026-11-27, so the arm takes no flag (it was
#             `--ns-semiforcing-notrump`)
#
# `bws vs advanced` is the price of playing the expert-consensus card instead
# of the measured one.  Expected: a loss, since most flips move a knob off
# its measured default; the headline is how much, and `--by node` says which
# flips carry it.  `sf vs advanced` is the knob's own ship verdict.  Four
# cells: the preset moves competitive and defensive agreements (Woolsey,
# lebensohl, the negative double of their overcall of our 1NT), which read
# the colours.
#
#   BOARDS=819200 scripts/queue-ab.sh bws scripts/ab-bws.sh ab-results/bws
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; delta = 0.002 IMPs/board.
#
# MEASURED 2026-11-27 (sha 2b5035a2; seed 1795754828414854101; 819,200
# bd/cell).  IMPs/board none / ns / ew / both:
#
#   bws vs advanced  plain -0.0216 / -0.0238 / -0.0119 / -0.0188  (loss 4/4)
#                    PD    -0.0971 / -0.1030 / -0.0940 / -0.0989  (loss 4/4)
#   sf  vs advanced  plain +202 / +326 / +204 / +286 IMPs        (win 4/4)
#                    PD    +418 / +658 / +431 / +600 IMPs        (win 4/4)
#                    fired 1099 / 1197 / 1094 / 1199 (0.13-0.15%)
#
# sf's isolation gate: 2 / 1 / 0 / 0 foreign boards, the declared EPBot book
# copying the knob (their `1♠ - 1NT - Pass`); pinned since.  sf shipped
# default-on.  The bws price is docs/bws-2017.md §5; its attribution was
# queued as pueue 44 (ab-results/bws/bucket.*.txt).
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-bws.sh RESULTS_DIR}
SHOW=${SHOW:-40}
BUILD_EXTRA='--example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== bws start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm advanced "$vul" --no-ns-semiforcing-notrump
    arm bws "$vul" --ns-preset bws
    diffpair bws advanced "$vul"
    arm sf "$vul"
    gatepair sf advanced "$vul" || log "isolation gate sf vs advanced ($vul) FAILED, scoring anyway"
    diffpair sf advanced "$vul"
done
log "=== bws done"
