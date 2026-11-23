#!/usr/bin/env bash
#
# queue-ab.sh — queue a heavy run (usually an A/B) behind the others on this box.
#
# Every heavy run saturates the machine, so they go one at a time through
# pueue (`cargo install --locked pueue`; `pueued` runs as a systemd user unit;
# its default group is one task wide).  This script freezes the checkout as it
# is right now, uncommitted and untracked files included, into a detached
# worktree, and queues the command there under scripts/idle-run.sh.  You keep
# editing; the queued run builds and measures the frozen copy, and its
# `cargo build` waits its turn too, so nothing is rebuilt under a running A/B.
#
# Usage:
#   [ENV=...] scripts/queue-ab.sh SLUG COMMAND [ARGS...]
#
# Example:
#   BOARDS=1638400 scripts/queue-ab.sh drury-keycard \
#       scripts/ab-drury-keycard.sh ab-results/drury-keycard
#
#   worktree  ${AB_WORKTREES:-~/ab}/<repo>-SLUG   (its own target/, a few GB)
#   results   the worktree's ab-results is a symlink to the main checkout's,
#             so `ab-results/...` arguments work unchanged and outlive it
#   log       ab-results/SLUG.log (appended; `idle-run: ... exited N` ends it)
#   env       captured now, at queue time (BOARDS, VULS, JOBS, ...)
#   SHA       a clean snapshot commit whose parent is HEAD, not `HEAD-dirty`
#
# Ignored files (data/, weights outside the tree) are not in the snapshot:
# pass those as absolute paths.  vendor/bba is an empty submodule there, so
# BBA_LIB defaults to the main checkout's library.
#
#   pueue status              the queue (add --json for scripts)
#   pueue restart -i ID       resume a failed run in place (runners are resumable)
#   pueue kill ID / remove ID stop a running task / drop a queued one
#   git worktree remove --force ~/ab/<repo>-SLUG     once the verdict is read
#
# Another project needs none of this script: `pueue add -w DIR -- 'command'`
# joins the same queue.
#
set -euo pipefail

if [[ $# -lt 2 || $1 == -h || $1 == --help ]]; then
	sed -n '2,/^set -euo/p' "$0" | sed 's/^# \{0,1\}//;/^set -euo/d'
	exit 0
fi
slug=$1
shift

cd "$(git rev-parse --show-toplevel)"
main=$(git worktree list --porcelain | sed -n '1s/^worktree //p')
wt=${AB_WORKTREES:-$HOME/ab}/${main##*/}-$slug

# Snapshot the working tree through a scratch index: the real index, the
# branch and the files are untouched.
rev=$(git rev-parse HEAD)
if [[ -n $(git status --porcelain) ]]; then
	idx=$(mktemp)
	trap 'rm -f "$idx"' EXIT
	cp "$(git rev-parse --git-path index)" "$idx"
	GIT_INDEX_FILE=$idx git add -A
	tree=$(GIT_INDEX_FILE=$idx git write-tree)
	rev=$(git commit-tree --no-gpg-sign "$tree" -p "$rev" -m "queue-ab: $slug")
fi

# Refuses an existing worktree: remove it, or `pueue restart` its task.
git worktree add --detach "$wt" "$rev" >&2
mkdir -p "$main/ab-results"
ln -s "$main/ab-results" "$wt/ab-results"
export BBA_LIB=${BBA_LIB:-$main/vendor/bba/Native-libraries/linux/x64/libEPBot.so}

printf -v cmd '%q ' scripts/idle-run.sh "$@"
printf -v log '%q' "$main/ab-results/$slug.log"
pueue add --label "$slug" --working-directory "$wt" -- "$cmd>>$log 2>&1"
