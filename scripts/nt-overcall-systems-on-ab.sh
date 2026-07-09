#!/bin/sh
# nt-overcall-systems-on-ab.sh — A/B for the systems-on advances of our 1NT
# overcall (`set_nt_overcall_systems_on`, docs/bba-gap-campaign.md def-r1
# 1NT-overcall lever: the [1t,1NT,P] advance was unauthored). ON = shipped
# default (the full opening-1NT structure — Stayman/transfers/Smolen — grafted
# below [1t,1NT], finding and right-siding major fits); OFF =
# --no-ns-nt-overcall-systems-on (floored advance). Scored SEPARATELY over minor
# openings (both majors findable) and major openings (one major is theirs).
# Both vulnerabilities, three scorers (plain+pd via ab-dump-diff, sd via
# ab-dump-sd), arms sequential, one shared SEED_BASE. Do NOT touch the codebase
# while it runs (bba-gen-parallel re-invokes cargo build; must stay a no-op).
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/nt-overcall-systems-on-ab.sh ab-results/nt-overcall-systems-on \
#       >ab-results/nt-overcall-systems-on.log 2>&1 &
R=${1:?usage: nt-overcall-systems-on-ab.sh RESULTS_DIR}
SPLIT=${SPLIT:?set SPLIT to the split_by_opening.py path}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

# Shards are split by opening strain, so the diff runs over dir/tag pairs (not
# the name-based $R/$name-$vul convention): override ab-lib's diffpair/sddiff.
# diffpair ON_DIR OFF_DIR TAG VUL — plain + pd, 8 solvers wide
diffpair() {
    on=$1; off=$2; tag=$3; vul=$4
    for score in plain pd; do
        out="$R/diff.$tag.$score.txt"
        [ -s "$out" ] && { log "skip $out"; continue; }
        log "diff $tag ($score)"
        i=0
        while [ "$i" -lt "$SHARDS" ]; do
            [ -f "$on/shard-$i.json" ] && "$DIFF" "$on/shard-$i.json" "$off/shard-$i.json" \
                --score "$score" --show 0 >"$out.shard-$i" 2>&1 &
            [ $(((i + 1) % 8)) -eq 0 ] && wait
            i=$((i + 1))
        done
        wait
        cat "$out".shard-* >"$out" 2>/dev/null; rm -f "$out".shard-*
    done
}

# sddiff ON_DIR OFF_DIR TAG VUL — sd-lead, 16 worlds
sddiff() {
    on=$1; off=$2; tag=$3; vul=$4
    out="$R/sd.$tag.txt"
    [ -s "$out" ] && { log "skip $out"; return 0; }
    log "sd-diff $tag"
    i=0
    while [ "$i" -lt "$SHARDS" ]; do
        [ -f "$on/shard-$i.json" ] && "$SD" "$on/shard-$i.json" "$off/shard-$i.json" \
            -v "$vul" --sd-worlds 16 --show 0 >"$out.shard-$i" 2>&1 &
        [ $(((i + 1) % 8)) -eq 0 ] && wait
        i=$((i + 1))
    done
    wait
    cat "$out".shard-* >"$out" 2>/dev/null; rm -f "$out".shard-*
}

log "=== nt-overcall-systems-on A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-nt-overcall-systems-on
    arm on  "$vul"
    python3 "$SPLIT" "$R/on-$vul"
    python3 "$SPLIT" "$R/off-$vul"
    for kind in minor major; do
        diffpair "$R/on-$vul-$kind" "$R/off-$vul-$kind" "$vul.$kind" "$vul"
        sddiff   "$R/on-$vul-$kind" "$R/off-$vul-$kind" "$vul.$kind" "$vul"
    done
done
log "=== nt-overcall-systems-on A/B done"
