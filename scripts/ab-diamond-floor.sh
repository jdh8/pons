#!/bin/sh
# ab-diamond-floor.sh — B2.4 of docs/bba-gap-campaign.md.
#
# Under ReadingScope::All the authored natural 1♦ rule substitutes for the
# walk, but its opaque prefers_diamonds() predicate projected no diamond axis.
# The fix states the implied 3+ ♦ floor explicitly, restoring the reading that
# partner_shown_len(♦, 3..) gates throughout the system.
#
# Knobless — a projection correction, not a treatment — so the arms differ by
# BINARY, not by flag:
#   fix    the working tree
#   base   main HEAD (04bd6432)
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
#   setsid nohup scripts/idle-run.sh scripts/ab-diamond-floor.sh \
#       /mnt/hdd-data/jdh8/pons-ab-results/b2-4-diamond-floor \
#       >/mnt/hdd-data/jdh8/pons-ab-results/b2-4-diamond-floor.log 2>&1 </dev/null &
#
# PRE-REGISTERED (before the numbers): B2.4 is a soundness correction on the
# same ledger as Phases 1–4, so it takes the reading-drift bar — non-loss ships
# it (plain wash-or-win AND PD non-loss, both vuls). A plain win with a PD loss
# is a doubling artifact and does not ship. Any loss gets
# `ab-dump-bucket --by node` and its worst divergent boards traced before any
# conclusion. Likely suspects are slam decisions: partner_shown_len(♦, 3..)
# feeds known_eight_card_fit and fit_sum_game, and the floor also reaches the
# v6 nets through features::push_inference.
#
# RESULT (2026-08-18): REJECTED. Three 204,800-board seeds per arm/vul
# (1787039750 / 1787040222 / 1787040684) were negative in 11/12 cells. The
# 614,400-board pools lost under PD non-vul (-0.0022 +/- 0.0020 IMP/bd) and
# plain DD vul (-0.0024 +/- 0.0020), so the atom was not retained. Every moved
# call followed our pair's 1♦ opening; BBA-opened 1♦ subsets moved zero calls.
# See the B2.4 row in docs/bba-gap-campaign.md for the soundness and node trace.
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
R=${1:?usage: ab-diamond-floor.sh RESULTS_DIR}
SHOW=${SHOW:-40}
SKIP_BUILD=1
. "$(dirname "$0")/ab-lib.sh"

for bin in bba-gen-fix bba-gen-base ab-dump-diff; do
    [ -x "$R/$bin" ] || { echo "missing $R/$bin — see the preparation comment" >&2; exit 2; }
done
DIFF=$R/ab-dump-diff
bins=$(cd "$R" && pwd)
SEED_BASE=$(seed_for)

log "=== diamond-floor start, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    # Each arm runs its own copied binary; ab-lib fingerprints whatever BBA_GEN names.
    BBA_GEN=$bins/bba-gen-fix; export BBA_GEN
    arm fix "$vul"
    BBA_GEN=$bins/bba-gen-base; export BBA_GEN
    arm base "$vul"
    diffpair fix base "$vul"
done
log "=== diamond-floor done"
