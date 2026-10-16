#!/bin/sh
# ab-walk-shape.sh — the first Phase-4 follow-up of
# docs/authored-reading-handoff.md § *Phase 4 → Owed*: walk length floors under
# the exclusion fold.
#
# A `hcp(0..)` catch-all projects ⊤ and hands its call to the natural walk.
# Under `bid_exclusion` the fold makes `⊤ ∩ ¬(heavier siblings)` informative,
# `substitutes_natural` fires, and the walk is suppressed — taking its *length
# floors* with it.  `(1♥) 2♥ - 2♠` read the Michaels advance ♠ 0..13 instead of
# ♠ 3..13, `answer_trump`'s provable-eight rung then failed, and the keycard
# ladder keyed a suit the asker was void in (the two −16/−17 RKCB boards Phase
# 4's own A/B census filed).
#
# The fix splits the two halves of a reading: such a call keeps reading its
# SHAPE off the walk (`CallMasks::walk_shape`) and its STRENGTH off the fold —
# which is the half the fold alone knows, and the half Phase 4 shipped for
# (`bid_exclusion_admits_the_jacoby_sign_off` pins it, and stays green).
#
# Knobless — a projection correction, not a treatment — so the arms differ by
# BINARY, not by flag:
#   fix    the working tree
#   base   `main` HEAD
#
# Preparation (from the repo root, the fix in the working tree):
#   cargo +nightly build --release --features serde --example bba-gen --example ab-dump-diff
#   mkdir -p $R
#   cp target/release/examples/bba-gen      $R/bba-gen-fix
#   cp target/release/examples/ab-dump-diff $R/ab-dump-diff
#   git stash && cargo +nightly build --release --features serde --example bba-gen
#   cp target/release/examples/bba-gen      $R/bba-gen-base
#   git stash pop
#
#   setsid nohup scripts/idle-run.sh scripts/ab-walk-shape.sh \
#       /mnt/hdd-data/jdh8/pons-ab-results/p4-walk-shape \
#       >/mnt/hdd-data/jdh8/pons-ab-results/p4-walk-shape.log 2>&1 </dev/null &
#
# PRE-REGISTERED (before the numbers): this is a precision correction on the
# same soundness ledger as Phase 4 itself, so it takes the reading-drift bar —
# **non-loss ships it** (plain wash-or-win AND PD non-loss, both vuls).  A
# plain win with a PD loss is a doubling artifact and does not ship.  Any loss
# gets `ab-dump-bucket --by node` and its worst divergent boards traced first.
# `smoke-default --count 20000 --seed 1` is already byte-identical at
# `9c56a4b2…`, so self-play is untouched; the whole footprint is competitive.
#
# That bar predates the non-inferiority margin (2026-10-14, #7): a plain wash
# is no proof of "non-loss".  A re-run reads the worst cell's `Verdict` lines
# (non-inferior = lower bound above −δ, docs/measurement.md § The decision
# table); the recorded ship's status is in
# docs/archive/research-method-review-closed.md § Issue #7 non-inferiority
# inventory.
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
R=${1:?usage: ab-walk-shape.sh RESULTS_DIR}
SHOW=${SHOW:-40}
SKIP_BUILD=1
. "$(dirname "$0")/ab-lib.sh"

for bin in bba-gen-fix bba-gen-base ab-dump-diff; do
    [ -x "$R/$bin" ] || { echo "missing $R/$bin — see the preparation comment" >&2; exit 2; }
done
DIFF=$R/ab-dump-diff
bins=$(cd "$R" && pwd)
SEED_BASE=$(seed_for)

log "=== walk-shape start, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    # Each arm runs its own copied binary; ab-lib fingerprints whatever BBA_GEN names.
    BBA_GEN=$bins/bba-gen-fix; export BBA_GEN
    arm fix "$vul"
    BBA_GEN=$bins/bba-gen-base; export BBA_GEN
    arm base "$vul"
    diffpair fix base "$vul"
done
log "=== walk-shape done"
