#!/bin/sh
# gib-scavenge: grow a GIB double-dummy database from idle CPU.
#
# Loops the one-shot `gib generate` and PAUSES while the target filesystem is
# low on space — so a forgotten scavenger can't fill a shared disk. Each pass
# GROWS an undersized shard rather than starting a new one, and only mints a
# fresh seed once every shard has reached GIB_CAP deals. So an interrupted run
# is resumed instead of stranded, and the directory holds a few big files
# instead of one per interruption. Merge with `gib convert` (each .pdd carries
# a header, so not `cat`).
#
# NAMING INVARIANT: `shard-<seed>.<ext>` is a *claim* that the file is the
# first k deals of that seed's stream — the resume path trusts it and appends
# deal k+1. A file with no single seed behind it (anything `gib convert`
# produced) must NOT be named `shard-*`, or it will be grown with a foreign
# deal stream and its name will become a lie.
#
# Supervised by scripts/gib-scavenge.service on Linux (SCHED_IDLE) or
# scripts/gib-scavenge.plist on macOS (Background QoS → E-cores only).
# SINGLE instance — enforced by flock below, because two scavengers would now
# pick the same shard and interleave appends into corruption.
#
# Knobs (env): GIB_OUT (dir), GIB_MIN_FREE_KIB (pause threshold), GIB_COUNT
#              (deals per pass, the pass being clamped so the shard lands ON
#              GIB_CAP, never past it), GIB_CAP (hard cap of deals per shard,
#              after which a new seed is minted), GIB_EXT (pdd|txt, default
#              pdd — binary is 2.6x smaller),
#              GIB_THREADS (DDS pool cap; Darwin defaults to the
#              E-core count to pair with the plist's Background QoS, empty =
#              all cores).
set -eu

OUT="${GIB_OUT:-$HOME/gib-shards}"
MIN_KIB="${GIB_MIN_FREE_KIB:-20971520}"          # pause below ~20 GiB free
COUNT="${GIB_COUNT:-1000000}"                    # deals appended per pass
# A sealed shard must stay loadable whole: `gib convert` and `gib verify` use
# pdd::load, ~48 B/deal decoded, so 10M deals is ~480 MB peak. Raising this
# raises that; the sampling consumers use load_slice and don't care.
CAP="${GIB_CAP:-10000000}"                       # exactly 340,000,008 B per .pdd shard
EXT="${GIB_EXT:-pdd}"                            # pdd (binary, 2.6x smaller) or txt
BIN="$(cd "$(dirname "$0")/.." && pwd)/target/release/examples/gib"

# One scavenger per output directory. flock dies with the process, so a SIGKILL
# leaves no stale lock to wedge the service's Restart=always.
# ponytail: macOS has no flock(1); launchd already enforces one instance per
# label there, so only a hand-started run can race.
if [ -z "${GIB_LOCKED:-}" ] && command -v flock > /dev/null 2>&1; then
    mkdir -p "$OUT"
    exec env GIB_LOCKED=1 flock -n "$OUT" "$0" "$@"
fi

# On-disk layout, so a shard's byte size converts to a deal count: the same
# arithmetic `gib generate --append` does when it resumes.  A ragged tail from a
# killed pass is under one record, so this floor divide ignores it exactly as
# the trim there does.
case "$EXT" in
    pdd) head_len=8; rec_len=34 ;;
    *)   head_len=0; rec_len=89 ;;
esac

# On Apple Silicon the plist runs us as ProcessType Background, which confines
# CPU-heavy work to the efficiency cluster — so cap the DDS pool at the E-core
# count instead of oversubscribing them with one worker per hardware core.
# Elsewhere (Linux SCHED_IDLE, Intel Macs) the sysctl is empty: no cap.
THREADS="${GIB_THREADS:-$(sysctl -n hw.perflevel1.logicalcpu 2>/dev/null || true)}"

mkdir -p "$OUT"
while true; do
    # df failure -> empty -> 0 -> treated as low (fail-safe: don't write).
    avail=$(df -Pk "$OUT" | awk 'NR == 2 { print $4 }')
    if [ "${avail:-0}" -lt "$MIN_KIB" ]; then
        echo "gib-scavenge: $(( ${avail:-0} / 1048576 )) GiB free below threshold, pausing 10m"
        sleep 600
        continue
    fi
    # Grow the first undersized shard; the glob is sorted, and taking the first
    # match keeps exactly one file hot, so every other shard is immutable and
    # safe to convert or copy.  CAP is a HARD cap: the pass is clamped to the
    # room left, because --count is deals *appended*, so an unclamped pass on a
    # nearly-full shard would overshoot by up to COUNT-1 deals.
    hot=""
    count="$COUNT"
    for f in "$OUT"/shard-*."$EXT"; do
        [ -f "$f" ] || continue                  # no match: glob stayed literal
        room=$(( CAP - ($(wc -c < "$f") - head_len) / rec_len ))
        if [ "$room" -ge 1 ]; then
            hot="$f"
            if [ "$room" -lt "$count" ]; then
                count="$room"
            fi
            break
        fi
    done
    if [ -n "$hot" ]; then
        seed=${hot##*/shard-}
        seed=${seed%.*}
    else
        seed=$(od -An -tu8 -N8 /dev/urandom | tr -d ' ')
        hot="$OUT/shard-$seed.$EXT"
    fi
    "$BIN" generate --append --count "$count" --seed "$seed" --out "$hot" \
        ${THREADS:+--threads "$THREADS"}
done
