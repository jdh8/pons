#!/usr/bin/env bash
#
# run.sh — spread pons double-dummy data-gen across machines (the fleet).
#
# The dumps (search-dump / teacher-dump) are deterministic given (git SHA, seed)
# and their .f32/.tags rows are independent and concatenable.  So distribution is
# just: partition the seed space, run a shard per seed on whatever host is free,
# pull the files back, concatenate (merge.sh).  No daemon, no queue — GNU
# `parallel` over ssh IS the scheduler.
#
# Each shard runs the prebuilt example wrapped in scripts/idle-run.sh, so every
# remote run stays SCHED_IDLE-polite on shared boxes.  -j1 = one all-core solver
# per host; faster hosts grab the next shard sooner, so heterogeneous speed
# self-balances (keep --per small and --shards large for this to bite).
#
# Usage:
#   scripts/fleet/run.sh --shards N [--per P] [--base-seed S] [--example E]
#                         [--hosts FILE] [--out DIR] [--provision] [-- EXTRA...]
#
#   --shards N      number of shards = number of distinct seeds (required)
#   --per P         boards per shard (default 200; tune so a shard is ~10-20 min)
#   --base-seed S   first seed; shard i uses seed S+i (default 1)
#   --example E     search-dump (default) or teacher-dump
#   --hosts FILE    sshloginfile (default scripts/fleet/hosts; see hosts.example)
#   --out DIR       local collect dir (default data/fleet-<runid>)
#   --provision     on each host: git fetch && checkout <SHA> && cargo build
#   -- EXTRA...     extra args forwarded to the example (e.g. --layouts 64)
#
# Env knobs: PONS_REMOTE_DIR (default ~/src/pons), REMOTE_DIR (default ~/pons-shards).
#
# Preconditions: the pinned SHA must be fetchable by the workers (git push it
# first); each worker needs a pons checkout + Rust toolchain; key-based ssh.
# Building on each host makes this arch-agnostic (native compile per box).
#
# Re-run the same command to resume: parallel --resume re-dispatches only the
# shards that didn't finish (a killed shard re-runs byte-identical — same seed).
#
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# --- defaults ---------------------------------------------------------------
HOSTS_FILE="$here/hosts"
EXAMPLE="search-dump"
PER=200
SHARDS=""
BASE_SEED=1
OUT=""
PROVISION=0
PASSTHRU=()
# Single-quoted so a leading ~ stays literal and expands on the *remote* shell
# (per-user $HOME), not locally.  An absolute override is used verbatim.
PONS_REMOTE_DIR="${PONS_REMOTE_DIR-}"; [ -n "$PONS_REMOTE_DIR" ] || PONS_REMOTE_DIR='~/src/pons'
REMOTE_DIR="${REMOTE_DIR-}";           [ -n "$REMOTE_DIR" ]      || REMOTE_DIR='~/pons-shards'

while [[ $# -gt 0 ]]; do
	case "$1" in
	--shards) SHARDS="$2"; shift 2 ;;
	--per) PER="$2"; shift 2 ;;
	--base-seed) BASE_SEED="$2"; shift 2 ;;
	--example) EXAMPLE="$2"; shift 2 ;;
	--hosts) HOSTS_FILE="$2"; shift 2 ;;
	--out) OUT="$2"; shift 2 ;;
	--provision) PROVISION=1; shift ;;
	--) shift; PASSTHRU=("$@"); break ;;
	-h | --help) sed -n '2,/^set -euo/p' "$0" | sed 's/^# \{0,1\}//;/^set -euo/d'; exit 0 ;;
	*) echo "run.sh: unknown arg: $1" >&2; exit 2 ;;
	esac
done

[ -n "$SHARDS" ] || { echo "run.sh: --shards N is required" >&2; exit 2; }
((SHARDS >= 1)) || { echo "run.sh: --shards must be >= 1" >&2; exit 2; }
[ -f "$HOSTS_FILE" ] || { echo "run.sh: hosts file not found: $HOSTS_FILE (copy hosts.example)" >&2; exit 2; }

FEATURES_FLAG=""
[[ "$EXAMPLE" == *search* ]] && FEATURES_FLAG="--features search"

SHA="$(git -C "$repo_root" rev-parse HEAD)"
RUNID="${SHA:0:8}-seed${BASE_SEED}"
REMOTE_RUNDIR="$REMOTE_DIR/$RUNID"
[ -n "$OUT" ] || OUT="$repo_root/data/fleet-$RUNID"
mkdir -p "$OUT"

# Real ssh targets (for preflight + collect): strip comments / blanks / "N/".
mapfile -t TARGETS < <(
	sed -E 's/#.*//; s/^[[:space:]]*//; s/[[:space:]]*$//' "$HOSTS_FILE" \
		| grep -v '^$' | sed -E 's#^[0-9]+/##'
)
((${#TARGETS[@]})) || { echo "run.sh: no hosts in $HOSTS_FILE" >&2; exit 2; }

echo "run.sh: pinned SHA $SHA  (run $RUNID, example $EXAMPLE)"

# --- preflight: every host must be at the pinned SHA (skew silently corrupts
#     the dataset).  --provision builds it there first. -----------------------
for t in "${TARGETS[@]}"; do
	if [ "$t" = ":" ]; then
		[ "$PROVISION" = 1 ] && { echo "  build (local)…"; (cd "$repo_root" && cargo build --release $FEATURES_FLAG --example "$EXAMPLE"); }
		[ "$(git -C "$repo_root" rev-parse HEAD)" = "$SHA" ] || { echo "run.sh: local HEAD moved mid-run" >&2; exit 1; }
		continue
	fi
	if [ "$PROVISION" = 1 ]; then
		echo "  provision $t…"
		ssh "$t" "cd $PONS_REMOTE_DIR && git fetch --quiet && git checkout --quiet $SHA && cargo build --release $FEATURES_FLAG --example $EXAMPLE" \
			|| { echo "run.sh: provision failed on $t (repo at $PONS_REMOTE_DIR? toolchain? SHA pushed & local tree clean?)" >&2; exit 1; }
	fi
	rsha="$(ssh "$t" "cd $PONS_REMOTE_DIR && git rev-parse HEAD" 2>/dev/null || true)"
	[ "$rsha" = "$SHA" ] || { echo "run.sh: $t is at '${rsha:-?}', not $SHA — rerun with --provision (and ensure the SHA is pushed)" >&2; exit 1; }
done

# --- dispatch: one shard per seed, -j1 per host, resumable via joblog ---------
echo "run.sh: $SHARDS shards × $PER boards, seeds $BASE_SEED..$((BASE_SEED + SHARDS - 1)) → idle-run on each host"
seq "$BASE_SEED" "$((BASE_SEED + SHARDS - 1))" \
	| parallel -j1 --sshloginfile "$HOSTS_FILE" --joblog "$OUT/joblog" --resume --tag \
		"cd $PONS_REMOTE_DIR && mkdir -p $REMOTE_RUNDIR && scripts/idle-run.sh ./target/release/examples/$EXAMPLE --boards $PER --seed {} --out $REMOTE_RUNDIR/shard-{} ${PASSTHRU[*]}"

# --- collect: pull each host's run dir back (shard names are seed-unique, so no
#     cross-host collisions). -------------------------------------------------
echo "run.sh: collecting → $OUT"
for t in "${TARGETS[@]}"; do
	if [ "$t" = ":" ]; then
		rsync -a "${REMOTE_RUNDIR/#\~/$HOME}/" "$OUT/" 2>/dev/null || true
	else
		rsync -a "$t:$REMOTE_RUNDIR/" "$OUT/" || echo "run.sh: rsync from $t failed (rerun to retry)" >&2
	fi
done

n="$(ls "$OUT"/shard-*.f32 2>/dev/null | wc -l)"
echo "run.sh: collected $n/$SHARDS shard .f32 files in $OUT"
echo "run.sh: merge with → scripts/fleet/merge.sh $OUT"
