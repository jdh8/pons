#!/usr/bin/env bash
# Fixed-corpus latency/reference run. This is deliberately not an ab-* driver:
# it scores no bridge outcomes and uses no bidding-treatment arms.
set -euo pipefail

repo_dir=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo_dir"

cargo build --release --features bench-internals --example bidding-performance
runner=target/release/examples/bidding-performance

if (($#)); then
    cpus=("$@")
else
    cpus=(4 14)
fi

runner_extra=()
if [[ ${ALLOW_UNSTABLE:-0} == 1 ]]; then
    echo "warning: ALLOW_UNSTABLE=1 disables the CV acceptance failure" >&2
    runner_extra+=(--allow-unstable)
fi

status=0

for cpu in "${cpus[@]}"; do
    case "$cpu" in
        4) sibling=20 ;;
        14) sibling=30 ;;
        *)
            echo "warning: CPU $cpu is not a documented benchmark core (expected 4 or 14)" >&2
            sibling=""
            ;;
    esac
    if [[ -n "$sibling" ]]; then
        busy=$(ps -eLo pid=,ppid=,psr=,stat=,comm= | awk -v core="$sibling" '$3 == core && $2 != 2 && $4 ~ /^[RD]/ && $5 != "ps" && $5 != "awk" { print; exit }')
        if [[ -n "$busy" ]]; then
            if [[ ${ALLOW_BUSY_SIBLING:-0} == 1 ]]; then
                echo "warning: ALLOW_BUSY_SIBLING=1; CPU $cpu is diagnostic only because sibling CPU $sibling appears busy: $busy" >&2
            else
                echo "error: skipping CPU $cpu because SMT sibling CPU $sibling appears busy: $busy" >&2
                echo "set ALLOW_BUSY_SIBLING=1 only for a non-acceptance diagnostic run" >&2
                status=1
                continue
            fi
        fi
    fi

    echo "cpu=$cpu warmups=2 repetitions=10"
    if ! taskset -c "$cpu" "$runner" --warmups 2 --repetitions 10 --skip-cache-acceptance "${runner_extra[@]}"; then
        status=1
    fi

    if command -v perf >/dev/null 2>&1 && perf stat -e cycles -- true >/dev/null 2>&1; then
        for engine in pons bba; do
            echo "perf cpu=$cpu engine=$engine (one process; 2 warmups + 10 measured loops)"
            if ! perf stat \
                -e cycles,instructions,cache-references,cache-misses,branches,branch-misses \
                -- taskset -c "$cpu" "$runner" --engine "$engine" --warmups 2 --repetitions 10 --skip-allocations --skip-cache-acceptance --allow-unstable \
                >/dev/null; then
                echo "warning: perf failed for cpu=$cpu engine=$engine; wall-time report retained" >&2
            fi
        done
    else
        echo "hardware counters unavailable (perf permission denied or perf missing); wall time and Rust allocation data retained" >&2
    fi
done

exit "$status"
