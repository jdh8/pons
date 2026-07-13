#!/bin/sh
# nt-overcall-gladiator-ab.sh — A/B for Gladiator advances of our 1NT overcall of
# their MAJOR (`set_nt_overcall_gladiator`, docs/bba-gap-campaign.md def-r1 lever):
# a `2♣` weak relay + a cue-of-their-major Stayman for the one unbid major +
# natural INV + splinter / Leaping-Michaels, replacing the opening-1NT graft over
# major openings only.  OFF = shipped default (systems-on opening-1NT graft over
# majors); ON = --ns-nt-overcall-gladiator.  Both arms keep systems-on for the
# MINOR openings (untouched), so the minor split is a ~0 sanity check and the
# major split is the real test.  Both vulnerabilities, three scorers (plain+pd via
# ab-dump-diff, sd via ab-dump-sd), arms sequential, one shared SEED_BASE.  Do NOT
# touch the codebase while it runs (bba-gen-parallel re-invokes cargo build; must
# stay a no-op).
#
#   PER_SHARD=6400 SPLIT=<split_by_opening.py> setsid nohup scripts/idle-run.sh \
#       scripts/nt-overcall-gladiator-ab.sh ab-results/nt-overcall-gladiator \
#       >ab-results/nt-overcall-gladiator.log 2>&1 &
R=${1:?usage: nt-overcall-gladiator-ab.sh RESULTS_DIR}
SPLIT=${SPLIT:?set SPLIT to the split_by_opening.py path}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

# Shards are split by opening strain, so the diff runs over dir/tag pairs (not
# the name-based $R/$name-$vul convention): override ab-lib's diffpair/sddiff.
# ab-dump-diff/-sd fold a whole strain dir's shards into one solve.
# diffpair ON_DIR OFF_DIR TAG VUL — plain + pd
diffpair() {
    on=$1; off=$2; tag=$3; vul=$4
    for score in plain pd; do
        out="$R/diff.$tag.$score.txt"
        [ -s "$out" ] && { log "skip $out"; continue; }
        log "diff $tag ($score)"
        "$DIFF" "$on" "$off" --score "$score" --show 0 >"$out" 2>&1
    done
}

# sddiff ON_DIR OFF_DIR TAG VUL — sd-lead, 16 worlds
sddiff() {
    on=$1; off=$2; tag=$3; vul=$4
    out="$R/sd.$tag.txt"
    [ -s "$out" ] && { log "skip $out"; return 0; }
    log "sd-diff $tag"
    "$SD" "$on" "$off" -v "$vul" --sd-worlds 16 --show 0 >"$out" 2>&1
}

log "=== nt-overcall-gladiator A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm on  "$vul" --ns-nt-overcall-gladiator
    python3 "$SPLIT" "$R/on-$vul"
    python3 "$SPLIT" "$R/off-$vul"
    for kind in minor major; do
        diffpair "$R/on-$vul-$kind" "$R/off-$vul-$kind" "$vul.$kind" "$vul"
        sddiff   "$R/on-$vul-$kind" "$R/off-$vul-$kind" "$vul.$kind" "$vul"
    done
done
log "=== nt-overcall-gladiator A/B done"
