#!/bin/sh
# Re-sweep the strength floor and ceiling of our natural defense to their 1NT
# under the shipped point scale.  Every arm is paired against BBA on the same
# defended-1NT deals and advertised as natural.  Plain + PD pre-screen every
# range; SD_RANGES selects the sd-lead finalists (the competitive-range arbiter).
#
# Example (about 200k boards/arm/vulnerability with 12 polite shards):
#   JOBS=12 PER_SHARD=17000 SD_RANGES='7:14 8:37' \
#     scripts/idle-run.sh scripts/one-nt-defense-range-ab.sh ab-results/nt-range
#
# Override NATURAL_RANGES to narrow or extend the grid; the 8:14 baseline is
# always generated even when omitted from the override.
R=${1:?usage: one-nt-defense-range-ab.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"

NATURAL_RANGES=${NATURAL_RANGES:-'6:14 7:14 7:37 8:14 9:14 10:14 8:15 8:16 8:18 8:37'}
SD_RANGES=${SD_RANGES:-''}
COMMON='--isolate-defense --filter-1nt --advertise-natural'

range_name() { echo "$1" | tr ':/' '--'; }

contains_range() {
    needle=$1
    for item in $2; do
        [ "$item" = "$needle" ] && return 0
    done
    return 1
}

SEED_BASE=$(seed_for natural)
log "=== natural 1NT-defense range sweep SEED_BASE=$SEED_BASE sha=$SHA"
for v in none both; do
    base=natural-8-14
    arm "$base" "$v" --ns-overcall 8:14 $COMMON
    for range in $NATURAL_RANGES; do
        [ "$range" = '8:14' ] && continue
        name="natural-$(range_name "$range")"
        arm "$name" "$v" --ns-overcall "$range" $COMMON
    done
    for range in $NATURAL_RANGES; do
        [ "$range" = '8:14' ] && continue
        name="natural-$(range_name "$range")"
        diffpair "$name" "$base" "$v"
        if contains_range "$range" "$SD_RANGES"; then
            sddiff "$name" "$base" "$v" \
                --on-ns-overcall "$range" --off-ns-overcall 8:14
        fi
    done
done

log "1NT-defense range sweep done"
