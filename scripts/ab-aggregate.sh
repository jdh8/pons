#!/bin/sh
# ab-aggregate.sh — sum per-shard `ab-dump-diff` summaries into one line.
#
#   scripts/ab-aggregate.sh ab-results/competitive-book/diff.*.txt
#
# Prints one aggregate per file: boards, fired count/rate, total IMPs,
# IMPs/board, IMPs/fired. (CI: treat per-shard means as 32 samples — see the
# repo measurement doc; this script only sums.)
set -eu
for f in "$@"; do
    awk -v name="$(basename "$f")" '
        / fired \(/ {
            for (i = 1; i <= NF; i++) {
                if ($i == "boards,") { gsub(/\(/, "", $(i-1)); boards += $(i-1) }
                if ($i == "fired")   { fired += $(i-1) }
            }
        }
        /^Delta/ {
            # "Delta (run − sit): -9 IMPs, …" — the IMP total is the field
            # before the "IMPs," token (the spaced minus shifts positions).
            for (i = 2; i <= NF; i++) {
                if ($i == "IMPs,") {
                    v = $(i - 1)
                    gsub(/[+,]/, "", v)
                    imps += v
                    n += 1
                    break
                }
            }
        }
        END {
            if (boards == 0) { printf "%s: no summaries\n", name; exit }
            printf "%s\n  %d boards, %d fired (%.3f%%), %+d IMPs, %+.5f IMPs/board, %+.3f IMPs/fired (%d shards)\n",
                name, boards, fired, 100 * fired / boards, imps, imps / boards,
                fired ? imps / fired : 0, n
        }
    ' "$f"
done
