#!/bin/sh
# ab-probe-kappa.sh — docs/shrinkage-plan.md § A3, the first pair: is the probed
# serving's measured loss a wall or a slope in the prior strength κ?
#
#   base       shipped defaults (the probed map empty)
#   kappa0     --ns-probe 100000 --ns-probe-vacuous --ns-probe-kappa 0
#              the 2026-07-31 arm, the 200-sample cliff — the negative control:
#              its mechanism must reproduce (plain −0.0467 / −0.0658 none / both,
#              PD −0.1118 / −0.1337 at ~10% fired, knobs.rs `probed_vacuous`)
#   kappa200   --ns-probe 100000 --ns-probe-vacuous
#              the κ-slope (crate default 200): every key from 20 samples serves
#              its slack-padded box shrunk toward ⊤ by n / (n + κ); the A0 ruler
#              read the cliff share of the probe's seed floor 88% → 17% here
#
# Pre-registered readings (the plan, § A3):
#   kappa200 no better than kappa0 on both scorers  → the loss is the net's
#       out-of-distribution wall; nothing short of the retrain lap (A4) helps;
#   kappa200 clearly above kappa0, and non-inferior to base (lower bound above
#       −δ on plain, worst cell) → a slope; the serving ships on its own full A/B
#       (δ = 0.002 IMPs/board, docs/measurement.md decision table).
#
#   JOBS=96 BOARDS=204800 DDSS_THREADS=96 \
#   BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen \
#   setsid nohup scripts/idle-run.sh scripts/ab-probe-kappa.sh \
#       ab-results/probe-kappa >ab-results/probe-kappa.log 2>&1 &
#
# Each shard re-probes 100k boards (three harvests since the A0 ruler), so a
# probed arm carries ~10 min of sequential probe per shard before its deals.
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: ab-probe-kappa.sh RESULTS_DIR}
SHOW=${SHOW:-40}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== probe-kappa start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in $VULS; do
    arm base "$vul"
    arm kappa0 "$vul" --ns-probe 100000 --ns-probe-vacuous --ns-probe-kappa 0
    arm kappa200 "$vul" --ns-probe 100000 --ns-probe-vacuous
    diffpair kappa0 base "$vul"
    diffpair kappa200 base "$vul"
    diffpair kappa200 kappa0 "$vul"
done
log "=== probe-kappa done"
