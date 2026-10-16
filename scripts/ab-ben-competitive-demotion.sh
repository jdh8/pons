#!/bin/bash
# ab-ben-competitive-demotion.sh — the BEN Tier-F guard for
# scripts/ab-competitive-demotion.sh, same SEED_BASE, same three arms.
#
#   BASE=ab-results/competitive-demotion
#   SEED_BASE="$(<"$BASE/seed")" PER_SHARD=1600 scripts/idle-run.sh \
#       scripts/ab-ben-competitive-demotion.sh "$BASE/ben"
#
# Needs 16 unchanged BEN servers on ports 8085..8100 (see ab-ben-defensive-
# overcalls.sh).  Ship = BBA pass + BEN non-refute, plain DD the arbiter.
set -euo pipefail
cd "$(dirname "$(readlink -f "$0")")/.."

R=${1:?usage: ab-ben-competitive-demotion.sh RESULTS_DIR}
: "${SEED_BASE:?set SEED_BASE to the matching BBA experiment seed}"
PER_SHARD=${PER_SHARD:-1600}
SHOW=${SHOW:-8}
SERVERS=16
CONF=vendor/ben/BEN-21GF-F.conf
NOTE="tier-f-conf-sha256=$(sha256sum "$CONF" | awk '{print $1}')"

ports=$(pgrep -af 'gameapi.py' | grep -o -- '--port [0-9]*' | awk '{print $2}' | sort -nu)
count=$(printf '%s\n' "$ports" | grep -c . || true)
[ "$count" = "$SERVERS" ] && [ "$(printf '%s\n' "$ports" | head -n 1)" = 8085 ] \
    && [ "$(printf '%s\n' "$ports" | tail -n 1)" = 8100 ] || {
    echo "need unchanged BEN ports 8085..8100; found $count: $ports" >&2
    exit 1
}

# ab-lib's arm/diffpair/sddiff: a manifest-checked, shard-verified resume and
# stamped reports (the old local arm skipped on a nonempty shard-0 alone).
# JOBS is the server count: ben-gen-parallel refuses any other fleet size.
GENERATOR=ben-gen
JOBS=$SERVERS
BUILD_EXTRA='--example ab-dump-sd'
. scripts/ab-lib.sh

# ben NAME VUL [flags...] — one Tier-F arm
ben() { arm "$1" "$2" -t f --note "$NOTE" "${@:3}"; }

log "=== BEN competitive demotion start, sha=$(git rev-parse --short HEAD), SEED_BASE=$SEED_BASE, ${SERVERS}x${PER_SHARD} bd/arm/vul, $NOTE"
for vul in none both; do
    ben control "$vul" --no-ns-competitive-push-natural-only
    ben off "$vul" --no-ns-competitive-pass-demotion
    ben natural "$vul"
    for on in off natural; do
        diffpair "$on" control "$vul"
        sddiff "$on" control "$vul"
    done
done
log "=== BEN competitive demotion done"
