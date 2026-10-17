#!/bin/sh
# ab-face-corroborate.sh — the second Phase-4 follow-up of
# docs/authored-reading-handoff.md § *Phase 4 → Owed*: the RKCB face-rung guard.
#
# *** REFUTED 2026-08-17, and the code change it prices was REVERTED. ***  Kept
# as the reproducibility record for the numbers in that doc's *Follow-up 2*.  To
# re-run it, first re-apply the one-line change it measures: in `answer_trump`
# (`src/bidding/instinct.rs`), `.or_else(|| face_trump(context.auction(), ask))`
# becomes `.or_else(|| face_trump(context.auction(), ask).filter(|&suit|
# corroborated(suit)))`.
#
# `answer_trump`'s two reading rungs already refuse the *answer's own suit*
# unless the pre-answer evidence justified it (the `corroborated` bar, authored
# after a `5♥` step answer was sat in a 4-1 "fit" for −20).  The face rung —
# the hand-free fallback — never carried that bar, and it reads the auction
# BELOW the ask, so it cannot see that the suit it keys is the suit the answer
# happens to name.  `(1♥) 2♥ - 2♠ - 3♣ - 3♦ - 4NT - 5♦ -` keyed diamonds off
# our own `3♦` and the asker passed the "minor answer" holding a singleton
# (−16).  This arm extends `corroborated` to that rung, one `.filter`.
#
# The known cost is the mirror failure: the answerer never vetoes (at its turn
# the answer does not exist yet, so `corroborated` is vacuous), so a veto here
# is the ASKER declining to decode an artificial answer already on the table —
# `instinct.rs`'s "hijacked window ... strands the auction in the 1430 answer".
# Whether the judgement fallback bids past it or sits it IS the measurement.
#
# Knobless (an instinct correction), so the arms differ by BINARY:
#   fix    the working tree (walk-shape follow-up 1 + this filter)
#   base   follow-up 1 alone — the `bba-gen-fix` binary of p4-walk-shape,
#          i.e. this change is priced ON TOP of the shipped follow-up 1
#
# Preparation: see scripts/ab-walk-shape.sh's header; the only difference is
# that `bba-gen-base` is copied from p4-walk-shape/bba-gen-fix rather than
# rebuilt from `main`.
#
#   setsid nohup scripts/idle-run.sh scripts/ab-face-corroborate.sh \
#       /mnt/hdd-data/jdh8/pons-ab-results/p4-face-corroborate \
#       >/mnt/hdd-data/jdh8/pons-ab-results/p4-face-corroborate.log 2>&1 </dev/null &
#
# PRE-REGISTERED (before the numbers): this is a floor change, NOT a soundness
# correction — it takes the standard bar, **a win on both scorers ships it**, a
# wash leaves the rung alone (the guard is not free: it costs decodes).  A loss
# gets its worst divergent boards traced for the strand signature above before
# any conclusion.  `smoke-default --count 20000 --seed 1` moves
# (`9c56a4b2…` -> `41f8c9c9…`), so unlike follow-up 1 this one is not
# self-play-inert.
#
# Resumable through ab-lib.sh: an arm resumes only on a matching arm.meta
# manifest (its copied binary's bytes, flags and deal set) and verified shards,
# a diff only on a matching .inputs stamp; SEED_BASE persists in $R/seed (a NEW
# dir -> a fresh seed).  Before ab-lib this runner skipped any existing arm
# dir unchecked, so a pre-ab-lib results dir refuses (no arm.meta) unless
# FORCE_REUSE=1.  SKIP_BUILD=1: the arms differ by binary, prebuilt into $R,
# so this script never invokes cargo (ab-lib still needs this checkout's
# target/release bba-gen and ab-dump-diff, which the preparation builds: the
# former verifies shards).  A copied binary from before `bba-gen --provenance`
# needs a reporting wrapper (ab-lib.sh, arm_fingerprint) or FORCE_REUSE=1.
R=${1:?usage: ab-face-corroborate.sh RESULTS_DIR}
SHOW=${SHOW:-40}
SKIP_BUILD=1
. "$(dirname "$0")/ab-lib.sh"

for bin in bba-gen-fix bba-gen-base ab-dump-diff; do
    [ -x "$R/$bin" ] || { echo "missing $R/$bin — see the preparation comment" >&2; exit 2; }
done
DIFF=$R/ab-dump-diff
bins=$(cd "$R" && pwd)
SEED_BASE=$(seed_for)

log "=== face-corroborate start, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in $VULS; do
    # Each arm runs its own copied binary; ab-lib fingerprints whatever BBA_GEN names.
    BBA_GEN=$bins/bba-gen-fix; export BBA_GEN
    arm fix "$vul"
    BBA_GEN=$bins/bba-gen-base; export BBA_GEN
    arm base "$vul"
    diffpair fix base "$vul"
done
log "=== face-corroborate done"
