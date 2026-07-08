#!/bin/sh
# weak-two-balancing-ab.sh — tournament-vs-BBA A/B for the two obstruction knobs
# that DD hid (the reason: their value is opponents mis-bidding under pressure,
# which a same-bidder DD A/B cannot see — only a fallible opponent, BBA, can be
# obstructed). One three-arm experiment on shared deals:
#
#   base       — neither knob (shipped default)
#   weak       — base + --ns-weak-two-comp   (set_weak_two_competition: our
#                contested weak twos — business XX + systems-on Ogust over their
#                double, Ogust/values-X/preemptive raises over their overcall)
#   balancing  — base + --ns-balancing       (set_notrump_balancing: extend our
#                1NT defense to the balancing seat, (1NT) P P ?)
#
# Each feature arm sets ONLY its own flag, so the paired per-shard diff vs base
# isolates exactly one knob against BBA. Both vulnerabilities, both scorers
# (plain DD + perfect-defense), arms strictly sequential, one shared SEED_BASE
# (paired diffs need identical deals). Modeled on scripts/rule-of-20-ab.sh; do
# NOT touch the codebase while it runs (bba-gen-parallel re-invokes cargo build;
# it must stay a no-op).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/weak-two-balancing-ab.sh ab-results/weak-two-balancing \
#       >ab-results/weak-two-balancing.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
set -eu
cd "$(dirname "$0")/.."

R=${1:?usage: weak-two-balancing-ab.sh RESULTS_DIR}
mkdir -p "$R"
SHA=$(git rev-parse --short HEAD)
DIFF=target/release/examples/ab-dump-diff
PER_SHARD=${PER_SHARD:-6400}
SHARDS=$(nproc)

cargo build --release --features serde --example bba-gen --example ab-dump-diff

log() { echo "$(date -u +%FT%TZ) $*" | tee -a "$R/log" >&2; }

# a persistent SEED_BASE, fresh on first use
if [ ! -s "$R/seed" ]; then date +%s >"$R/seed"; fi
SEED_BASE=$(cat "$R/seed")

# arm NAME VUL [flags...] — generate one arm unless already present
arm() {
    name=$1; vul=$2; shift 2
    dir="$R/$name-$vul"
    [ -d "$dir" ] && { log "skip $dir (exists)"; return 0; }
    log "generate $dir (SEED_BASE=$SEED_BASE, flags: $*)"
    SEED_BASE=$SEED_BASE scripts/bba-gen-parallel.sh "$dir" "$PER_SHARD" -v "$vul" "$@" \
        >>"$R/log" 2>&1
}

# diffpair ON OFF VUL — per-shard paired diff, both scorers, 8 solvers wide
diffpair() {
    on=$1; off=$2; vul=$3
    for score in plain pd; do
        out="$R/diff.$on.vs.$off.$vul.$score.txt"
        [ -s "$out" ] && { log "skip $out (exists)"; continue; }
        log "diff $on vs $off ($vul, $score)"
        i=0
        while [ "$i" -lt "$SHARDS" ]; do
            "$DIFF" "$R/$on-$vul/shard-$i.json" "$R/$off-$vul/shard-$i.json" \
                --score "$score" --show 5 >"$out.shard-$i" 2>&1 &
            [ $(((i + 1) % 8)) -eq 0 ] && wait
            i=$((i + 1))
        done
        wait
        cat "$out".shard-* >"$out"; rm -f "$out".shard-*
    done
}

log "weak-two/balancing A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, shards=$SHARDS x $PER_SHARD boards/arm/vul"

for vul in none both; do
    arm base      "$vul"                       # shipped default (neither knob)
    arm weak      "$vul" --ns-weak-two-comp    # + contested weak twos
    arm balancing "$vul" --ns-balancing        # + 1NT balancing-seat defense
    diffpair weak      base "$vul"
    diffpair balancing base "$vul"
done

log "weak-two/balancing A/B done"
