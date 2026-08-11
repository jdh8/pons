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
DIFF=target/release/examples/ab-dump-diff
SD=target/release/examples/ab-dump-sd
mkdir -p "$R"

log() { echo "$(date -u +%FT%TZ) $*" | tee -a "$R/log" >&2; }

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

for bin in target/release/examples/ben-gen "$DIFF" "$SD"; do
	[ -x "$bin" ] || { echo "release binary missing before BEN run: $bin" >&2; exit 1; }
done

arm() {
	local name=$1 vul=$2
	shift 2
	local dir="$R/$name-$vul"
	[ -s "$dir/shard-0.json" ] && { log "skip $dir (exists)"; return; }
	log "generate $dir (SEED_BASE=$SEED_BASE, flags: $*)"
	SEED_BASE=$SEED_BASE SKIP_BUILD=1 scripts/ben-gen-parallel.sh "$dir" "$PER_SHARD" \
		-v "$vul" -t f --note "$NOTE" "$@" >>"$R/log" 2>&1
	local got
	got=$(find "$dir" -maxdepth 1 -name 'shard-*.json' -type f -size +0c | wc -l)
	[ "$got" -eq "$SERVERS" ] || {
		echo "$dir has $got/$SERVERS complete shards" >&2
		exit 1
	}
}

diffpair() {
	local vul=$1
	local plain="$R/diff.on.vs.off.$vul.plain.txt"
	local pd="$R/diff.on.vs.off.$vul.pd.txt"
	log "diff on vs off ($vul, plain+pd)"
	"$DIFF" "$R/on-$vul" "$R/off-$vul" --score both \
		--out-plain "$plain" --out-pd "$pd" --show "$SHOW" >>"$R/log" 2>&1
	log "sd-diff on vs off ($vul, 16 worlds)"
	"$SD" "$R/on-$vul" "$R/off-$vul" -v "$vul" --sd-worlds 16 --show 0 \
		--on-ns-direct-weak-jump-overcall >"$R/sd.on.vs.off.$vul.txt" 2>&1
}

log "=== BEN direct-weak-jump-overcall A/B start, sha=$(git rev-parse --short HEAD), SEED_BASE=$SEED_BASE, ${SERVERS}x${PER_SHARD} bd/arm/vul, $NOTE"
for vul in none both; do
	arm off "$vul"
	arm on "$vul" --ns-direct-weak-jump-overcall
	diffpair "$vul"
done
log "=== BEN direct-weak-jump-overcall A/B done"
