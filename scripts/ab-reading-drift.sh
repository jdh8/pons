#!/bin/sh
# ab-reading-drift.sh — the batched A/B for the reading-drift repairs
# (docs/reading-drift-handoff.md, campaign ledger).  The repairs are knobless
# code corrections (walk arms, the at-the-time projection context, the strip
# re-key, the transfer-GF 3NT gates), so the arms differ by BINARY, not by
# flag: prebuild both bidders and the diff harness, copy them into $R, and
# this script never invokes cargo — the no-rebuild-in-flight rule holds by
# construction.
#
# Preparation (from the repo root, fixes in the working tree):
#   cargo build --release --features serde --example bba-gen --example ab-dump-diff
#   mkdir -p ab-results/reading-drift
#   cp target/release/examples/bba-gen      ab-results/reading-drift/bba-gen-fix
#   cp target/release/examples/ab-dump-diff ab-results/reading-drift/ab-dump-diff
#   git stash && cargo build --release --features serde --example bba-gen
#   cp target/release/examples/bba-gen      ab-results/reading-drift/bba-gen-base
#   git stash pop && cargo build --release --features serde --example bba-gen
#
#   setsid nohup scripts/idle-run.sh scripts/ab-reading-drift.sh \
#       ab-results/reading-drift >ab-results/reading-drift.log 2>&1 </dev/null &
#
# Decision table (docs/measurement.md): the repairs are soundness corrections,
# so a non-loss (wash or win, plain and PD read together) ships them as they
# stand; a loss gets its worst divergent boards traced before any conclusion.
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
R=${1:?usage: ab-reading-drift.sh RESULTS_DIR}
SHOW=${SHOW:-40}
SKIP_BUILD=1
. "$(dirname "$0")/ab-lib.sh"
# Optional extra bba-gen flags per arm (word-split intentionally), for A/Bs
# where the arms differ by knob rather than binary — the same binary is then
# copied to both bba-gen-fix and bba-gen-base.
FIX_ARGS=${FIX_ARGS:-}
BASE_ARGS=${BASE_ARGS:-}

for bin in bba-gen-fix bba-gen-base ab-dump-diff; do
    [ -x "$R/$bin" ] || { echo "missing $R/$bin — see the preparation comment" >&2; exit 2; }
done
DIFF=$R/ab-dump-diff
bins=$(cd "$R" && pwd)
SEED_BASE=$(seed_for)

log "=== reading-drift start, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in $VULS; do
    # Each arm runs its own copied binary; ab-lib fingerprints whatever BBA_GEN names.
    BBA_GEN=$bins/bba-gen-fix; export BBA_GEN
    # shellcheck disable=SC2086 — $FIX_ARGS/$BASE_ARGS word-split by design
    arm fix "$vul" $FIX_ARGS
    BBA_GEN=$bins/bba-gen-base; export BBA_GEN
    # shellcheck disable=SC2086
    arm base "$vul" $BASE_ARGS
    diffpair fix base "$vul"
done
log "=== reading-drift done"
