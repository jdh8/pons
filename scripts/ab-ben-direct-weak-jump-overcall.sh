#!/usr/bin/env bash
#
# ab-ben-direct-weak-jump-overcall.sh — paired Tier-F validation of the direct
# weak jump overcall after its BBA guard passes. Exactly 16 already-running
# Tier-F servers are held unchanged across OFF/ON and both vulnerabilities.
# With the default 800 boards/shard this is 25,600 boards per arm in total.
# Plain DD + PD share one solve; plain SD + SD-PD use the standard 16 worlds.
#
#   SEED_BASE=<same seed as BBA> scripts/idle-run.sh \
#       scripts/ab-ben-direct-weak-jump-overcall.sh RESULTS_DIR
set -euo pipefail
cd "$(dirname "$(readlink -f "$0")")/.."

R=${1:?usage: ab-ben-direct-weak-jump-overcall.sh RESULTS_DIR}
: "${SEED_BASE:?set SEED_BASE to the paired BBA experiment seed}"
PER_SHARD=${PER_SHARD:-800}
SHOW=${SHOW:-50}
SERVERS=16
CONF=vendor/ben/BEN-21GF-F.conf
CONF_SHA=$(sha256sum "$CONF" | awk '{print $1}')
NOTE="tier-f-conf-sha256=$CONF_SHA"

ports=$(pgrep -u "$USER" -af 'gameapi\.py' | grep -o -- '--port [0-9]*' | awk '{print $2}' | sort -nu || true)
count=$(printf '%s\n' "$ports" | awk 'NF { n++ } END { print n + 0 }')
[ "$count" -eq "$SERVERS" ] || {
	echo "need exactly $SERVERS unchanged BEN servers, found $count: $ports" >&2
	exit 1
}
[ "$(printf '%s\n' "$ports" | head -n 1)" = 8085 ] &&
	[ "$(printf '%s\n' "$ports" | tail -n 1)" = 8100 ] || {
	echo "expected BEN ports 8085..8100, found: $ports" >&2
	exit 1
}

# ab-lib's arm/diffpair/sddiff: a manifest-checked, shard-verified resume and
# stamped reports (the old local arm skipped on a nonempty shard-0 and counted
# nonempty shards after).  SKIP_BUILD=1 keeps this runner build-free: it
# measures the binaries the BBA guard already ran, which must exist.  JOBS is
# the server count: ben-gen-parallel refuses any other fleet size.
GENERATOR=ben-gen
JOBS=$SERVERS
BUILD_EXTRA='--example ab-dump-sd'
SKIP_BUILD=1
. scripts/ab-lib.sh

log "=== BEN direct-weak-jump-overcall A/B start, sha=$(git rev-parse --short HEAD), SEED_BASE=$SEED_BASE, ${SERVERS}x${PER_SHARD} bd/arm/vul, $NOTE"
for vul in $VULS; do
	arm off "$vul" -t f --note "$NOTE" --no-ns-direct-weak-jump-overcall
	arm on "$vul" -t f --note "$NOTE"
	diffpair on off "$vul"
	sddiff on off "$vul" --on-ns-direct-weak-jump-overcall
done
log "=== BEN direct-weak-jump-overcall A/B done"
