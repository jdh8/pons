#!/bin/sh
# ab-landy-read.sh — N1g of the competitive-1NT campaign
# (docs/one-notrump-competitive.md §N1g): the Landy disclosure's READ-side
# wiring.  `their.two_clubs_landy` moved the book only; their 2♣ kept decoding
# through the natural walk as 5+♣ 8+, so the learned floor's LHO envelope was
# false on every board of the lane.  `--ns-their-landy-read` makes the declared
# 2♣ read 4-4+ majors (no strength claim) and natural-suppresses the advances.
#
#   JOBS=12 setsid nohup scripts/idle-run.sh scripts/ab-landy-read.sh \
#       ab-results/landy-read >ab-results/landy-read.log 2>&1 < /dev/null &
#
# BBA's 2/1 card overcalls 1NT with Multi-Landy, whose 2♣ *is* Landy, so the
# measured reading is TRUE of the reference opponent.  `--filter-1nt` is the
# enriched-probing gate (raw-hand test BEFORE any bidding) on every arm, so the
# arms stay seed-aligned for the paired diff.
#
# Arms (post-ship spelling — the wiring is the engine default since
# 2026-08-14): read-on = the default, read-off = `--ns-their-landy-read
# false`, the pre-ship arm.  landy-on = the pre-stack base counter, regenerated at HEAD on the
# same seeds for the owed `3♣`→`2♥` re-probe (the stack rerouted GF six-card
# minors through the cue; re-read that row against the shipped stack):
#     probe-divergence read-on landy-on --imps --jsonl ... per vul, filter the
#     records whose responder call flipped 3♣↔2♥ (call_on/call_off).
#
# Acceptance gate for the wiring pair: the reader is seat-gated by
# construction, so
#     probe-divergence read-on read-off --gate-opener ours
# must pass at ZERO foreign boards — one foreign board is a seat-correctness
# bug, not noise.
#
# NOT here: `--ns-lebensohl-completion-alert` (the sibling defect) — its fired
# population is the advance-sohl lanes (their weak two, our takeout X), which
# `--filter-1nt` filters OUT; it needs its own unfiltered run and is never
# folded into this one.
#
# Resumable: an existing arm dir or diff file is skipped, and SEED_BASE
# persists in $R/landy-read.seed.  Iron rule: do NOT rebuild binaries while
# this runs.
R=${1:?usage: ab-landy-read.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-read)
log "=== landy-read SEED_BASE=$SEED_BASE sha=$SHA"

for v in none both; do
    arm read-off "$v" --their-2c-landy true --ns-their-landy-read false --filter-1nt
    arm read-on "$v" --their-2c-landy true --filter-1nt
    arm landy-on "$v" --their-2c-landy true --defense-2c-landy-transfer false \
        --defense-2c-landy-cue-floor false --defense-2c-landy-fit-answers false \
        --defense-2c-landy-competition false --filter-1nt
    # The wiring is a reading knob under the neural floor — a bidding knob;
    # plain+PD decide, sd the tie-breaker (docs/measurement.md).
    diffpair read-on read-off "$v"
    sddiff read-on read-off "$v"
    # The re-probe pair (not a ship gate — a forensic input).
    diffpair read-on landy-on "$v"
done

log "landy-read done"
