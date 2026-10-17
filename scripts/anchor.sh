#!/bin/sh
# anchor.sh — the pons↔BBA gap anchor (docs/bba-gap-campaign.md, Protocol):
# generate both vulnerability arms at the persistent anchor seed, decompose
# into ranked IMP-loss buckets, and persist the report.
#
#   setsid nohup scripts/idle-run.sh scripts/anchor.sh \
#       >ab-results/anchor.log 2>&1 &
#
# The SERIES dir (ab-results/anchor) holds the persistent seed and the
# deal-keyed DD cache — the sanctioned exception to fresh-seed-per-experiment:
# successive anchors are arms of one longitudinal paired experiment, and the
# cache never invalidates because the seed series always deals the same
# boards.  Each run writes a SNAPSHOT subdir (date + sha) with the shard
# dumps, report.md, and boards.jsonl.  The first run pays the DD solve
# (~20-60 min); re-anchors after a batch of fixes take minutes (generation +
# cache-miss solves only).  Ship decisions stay per-fix fresh-seed A/Bs — the
# anchor is a tracking/attribution instrument (docs/measurement.md governs).
# Do NOT touch the codebase while it runs (bba-gen-parallel re-invokes cargo
# build; it must stay a no-op).
set -eu
cd "$(dirname "$0")/.."

R=${1:-ab-results/anchor}
mkdir -p "$R"
# A dirty tree is not the sha it claims to be, and a mislabelled snapshot is how
# a control goes stale without anyone noticing (docs/bba-gap-campaign.md,
# "Rules the series taught" #2, control at HEAD).
SHA=$(git rev-parse --short HEAD)$(git diff --quiet HEAD || echo -dirty)
SNAP="$R/$(date -u +%F)-$SHA"

# The series' deal set is SHARDS × PER_SHARD boards per arm per vul, shard i
# dealt from SEED_BASE + i, so both are series constants like the seed:
# recorded on first use, read back on every re-anchor.  JOBS is
# bba-gen-parallel's shard count, not just its parallelism; a different JOBS
# deals different boards, unpaired with the last snapshot and missing the DD
# cache.  A series whose seed predates these files must have them written by
# hand (the canonical one: 32 and 6400).
pin() { # pin NAME FILE DEFAULT [OVERRIDE] — echo the series value of NAME
    if [ -s "$2" ]; then
        if [ -n "${4:-}" ] && [ "$4" != "$(cat "$2")" ]; then
            echo "anchor.sh: $1=$4, but the series in $R was dealt with $1=$(cat "$2"); unset $1 or use a fresh series dir" >&2
            exit 1
        fi
    elif [ -s "$R/seed" ]; then
        echo "anchor.sh: $R has a seed but no $2; write the series' $1 there (the canonical series: shards 32, per-shard 6400)" >&2
        exit 1
    else
        echo "${4:-$3}" >"$2"
    fi
    cat "$2"
}
JOBS=$(pin JOBS "$R/shards" "$(nproc)" "${JOBS:-}")
PER_SHARD=$(pin PER_SHARD "$R/per-shard" 6400 "${PER_SHARD:-}")
export JOBS

cargo build --release --features serde \
    --example bba-gen --example bba-decompose --example ab-dump-diff

log() { echo "$(date -u +%FT%TZ) $*" | tee -a "$R/log" >&2; }
# verified DIR — every shard present, parsing, of the expected seed and size
# (bba-gen-parallel.sh --verify).  Not -d the dir, nor -s shard-0: an arm that
# died on startup leaves an empty dir, one that lost a worker a short arm, and
# either would be resumed past (the lesson ab-lib.sh's arm() records).
verified() {
    [ -d "$1" ] && scripts/bba-gen-parallel.sh --verify "$1" "$PER_SHARD" >>"$R/log" 2>&1
}

# A persistent SEED_BASE, fresh on first use, shared by every re-anchor.
if [ ! -s "$R/seed" ]; then echo $(date +%s%N) >"$R/seed"; fi
SEED_BASE=$(cat "$R/seed")
export SEED_BASE

log "=== anchor start, sha=$SHA, SEED_BASE=$SEED_BASE, ${JOBS}x$PER_SHARD bd/arm/vul -> $SNAP"
for vul in $VULS; do
    dir="$SNAP/$vul"
    verified "$dir" && { log "skip $dir (complete, verified)"; continue; }
    log "generate $dir"
    # --our-floor american-instinct: bba-decompose replays through the
    # deterministic books; american() now ships the non-decomposable net floor.
    # Disclosure is left at its `generated` default from 2026-07-28 on, so BBA
    # reads our alerts.  Anchors before that date faced a blind BBA and are a
    # different series — see CHANGELOG for the re-base.
    scripts/bba-gen-parallel.sh "$dir" "$PER_SHARD" -v "$vul" \
        --our-floor american-instinct >>"$R/log" 2>&1
done

log "decompose -> $SNAP/report.md"
target/release/examples/bba-decompose "$SNAP/none" "$SNAP/both" \
    --dd-cache "$R/dd-cache.json" \
    --report "$SNAP/report.md" \
    --jsonl "$SNAP/boards.jsonl" \
    2>&1 | tee -a "$R/log"

# The SHIPPING pair: what american() — the net floor — actually scores.
# Hand-rolled for the 0d8b755 anchor and never re-run since; in here because
# every trap it hit was a "generated somewhere else" trap.  Note the decompose
# below passes --our-floor american: replay is then 100% and the bucket rows
# are VALID.  The ~90% replay the campaign doc long called "by construction"
# was the flag being missed (7af286d added it), not a property of the net.
# And never diff a fresh `american` arm against an older snapshot's instinct
# arm; same $SNAP, same $SEED_BASE and same $SHA is what makes the paired delta
# below the floor's value rather than the floor's value plus a batch of book
# fixes (docs/bba-gap-campaign.md, rule #2).
for vul in $VULS; do
    dir="$SNAP/american-$vul"
    verified "$dir" && { log "skip $dir (complete, verified)"; continue; }
    log "generate $dir"
    scripts/bba-gen-parallel.sh "$dir" "$PER_SHARD" -v "$vul" \
        --our-floor american >>"$R/log" 2>&1
done

log "decompose shipping -> $SNAP/report-american.md"
target/release/examples/bba-decompose "$SNAP/american-none" "$SNAP/american-both" \
    --our-floor american \
    --dd-cache "$R/dd-cache.json" \
    --report "$SNAP/report-american.md" \
    --jsonl "$SNAP/boards-american.jsonl" \
    2>&1 | tee -a "$R/log"

# And the floor's own worth, paired: the tight instrument (rule #2), instead of
# subtracting two absolute vs-BBA gaps.  This is the direct A/B at table A;
# the headline difference of the two decomposes is the other metric, a
# reference-relative gain over both tables (docs/measurement.md, "Direct A/B
# vs reference-relative gain") — about twice it, and never summed with it.
for vul in $VULS; do
    log "diff american vs american-instinct ($vul, plain+pd)"
    target/release/examples/ab-dump-diff "$SNAP/american-$vul" "$SNAP/$vul" \
        --score both \
        --out-plain "$SNAP/diff.floor.$vul.plain.txt" \
        --out-pd "$SNAP/diff.floor.$vul.pd.txt" >>"$R/log" 2>&1
done

log "=== anchor done: $SNAP/report.md + $SNAP/report-american.md"
