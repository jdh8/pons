#!/usr/bin/env bash
#
# idle-run.sh — run a long job as a polite "scavenger" on a shared machine.
#
# Wraps any command in SCHED_IDLE (CPU only when a core would otherwise sit idle)
# plus the idle I/O class. The job uses 100% of *spare* capacity on a quiet box
# and is preempted off any core the instant a normal-priority task wakes there —
# so it soaks idle time without a quota and gets out of everyone's way under load.
#
# This is strictly politer than `nice -19`: SCHED_IDLE is a separate scheduling
# class below the minimum nice weight, with lower preemption latency, it needs no
# privilege, and child processes inherit it (so a multithreaded solver is covered
# by wrapping the parent). See docs/shared-machine-data-gen.md for the rationale
# and the caveats it does NOT cover (turbo droop, shared cache / memory
# bandwidth, and cross-user cgroup weighting).
#
# Usage:
#   scripts/idle-run.sh <command> [args...]
#
# Example (regenerate the AI-bidder search-data set, ~16 h on an idle box):
#   scripts/idle-run.sh cargo run --release --features search \
#     --example search-dump -- --boards 10000 --seed 1 --progress
#
# To survive an SSH disconnect, run it inside tmux/screen, or:
#   setsid nohup scripts/idle-run.sh <command> >run.log 2>&1 < /dev/null &
#
# To also cap RAM (guard against a runaway OOM-ing colleagues) on a systemd box:
#   systemd-run --user --scope -p MemoryMax=12G scripts/idle-run.sh <command>
#
set -euo pipefail

if [[ $# -eq 0 || "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
	sed -n '2,/^set -euo/p' "$0" | sed 's/^# \{0,1\}//;/^set -euo/d'
	exit 0
fi

# Build the privilege prefix from whatever this machine has, degrading
# gracefully: SCHED_IDLE for CPU, idle class for I/O.
prefix=()
if command -v chrt >/dev/null 2>&1; then
	# nice is cosmetic here: SCHED_IDLE ignores it, but htop still paints it blue.
	prefix+=(nice -n10 chrt --idle 0) # SCHED_IDLE, static priority 0 (the only legal value)
else
	echo "idle-run: chrt not found; falling back to 'nice -n19'" >&2
	prefix+=(nice -n19)
fi
command -v ionice >/dev/null 2>&1 && prefix+=(ionice -c3) # idle I/O class

exec "${prefix[@]}" "$@"
