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
# Sources ~/.config/pons/idle-run.local.sh when present.
# Set PONS_IDLE_LOCAL= (empty) to skip the machine-local hook.
#
# Example (a full two-arm A/B against the BBA reference, hours on an idle box):
#   PER_SHARD=6400 scripts/idle-run.sh \
#     scripts/ab-book-value.sh ab-results/book-value
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

# Optional machine-local politeness hook: a box that runs other heavy services
# (this one has a poker solver) sources a file here to pause them for the run and
# restore them on exit. Untracked, since the services differ per machine.
# Set PONS_IDLE_LOCAL= (empty) to skip it — e.g. for long-lived idle servers.
# ponytail: no restore logic here; the hook owns its own trap.
local_hook="${PONS_IDLE_LOCAL-$HOME/.config/pons/idle-run.local.sh}"
[[ -n $local_hook && -r $local_hook ]] && . "$local_hook"

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

# Always leave a terminal line in the log, whatever happens. A watcher that
# greps for the *job's own* "A/B done" line hangs forever when the job dies
# (a mid-run rebuild, a `set -e` bail), so the failure goes unnoticed instead
# of being reported. This line appears on success and on failure alike, with
# the exit status — see docs/measurement.md § "Watching a run".
#
# The braces make bash parse the whole tail before running it, so editing this
# file mid-run cannot corrupt an in-flight instance (`exec` used to give that
# for free).
{
	status=0
	"${prefix[@]}" "$@" || status=$?
	printf 'idle-run: %s exited %d\n' "${1##*/}" "$status"
	exit "$status"
}
