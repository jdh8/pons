#!/usr/bin/env bash
#
# ab-reading-knobs.sh — a reading-knob ship-gate A/B (docs/ben-gap-campaign.md,
# dual-reference rule): primary fresh-seed vs BEN Tier F, guard same-seed vs
# BBA, arms strictly sequential, scored plain + PD at the end.
#
#   setsid nohup scripts/idle-run.sh scripts/ab-reading-knobs.sh [KNOB] \
#       >>ab-results/reading-knobs/run.log 2>&1 &
#
# KNOB picks the knob under test: pass (default) | cue | length | table.  All
# four ship default-ON, so the *treatment* is the bare arm and the flagged arm
# is the `--no-ns-*` off-switch; a positive diff still means "the knob helps".
# (Before 2026-08-03 these were opt-in `--ns-*` flags and this script passed
# them on the `on` arm.  They flipped to default-on and the flags were renamed,
# which left every invocation dying on clap — hence the fresh EXP date: the
# 2026-07-17 results below it were taken under the old polarity and must not be
# pooled with new cells.)  The on arm is shared across knobs — one SEED_BASE
# (recorded in seed/note.txt) for the whole experiment series, cells resume by
# skip-if-done, so re-runs and later knobs only generate what's missing.  Probe fact 2026-07-17: cue,
# table, and pass are bid-inert in the default system (0, 0, and 1 divergent
# board per 211k — reading/instrument-side knobs); length is the one live
# arm (23/6400 boards).  BEN cells run FIRST — the servers live on deleted
# inodes (~/ben was removed) and cannot be restarted; BBA needs nothing
# external.  Binaries are prebuilt; the parallel scripts' cargo build must
# stay a no-op — do not touch the codebase while this runs.
set -euo pipefail
cd "$(dirname "$0")/.."

KNOB=${1:-pass}
case "$KNOB" in
pass) OFFFLAG=--no-ns-pass-reading ;;
cue) OFFFLAG=--no-ns-cue-reading ;;
length) OFFFLAG=--no-ns-length-soundness ;;
table) OFFFLAG=--no-ns-table-alert-reading ;;
*)
	echo "usage: $0 [pass|cue|length|table]" >&2
	exit 2
	;;
esac

EXP=ab-results/reading-knobs/2026-08-03
mkdir -p "$EXP/scores"
SHA=$(git rev-parse --short HEAD)
if [ ! -s "$EXP/seed" ]; then date +%s >"$EXP/seed"; fi
SEED_BASE=$(cat "$EXP/seed")
export SEED_BASE
PER=${PER:-6400}

log() { echo "$(date -u +%FT%TZ) $*" >&2; }
log "=== reading-knobs A/B start: knob=$KNOB sha=$SHA SEED_BASE=$SEED_BASE per-shard=$PER"
echo "knob=$KNOB sha=$SHA SEED_BASE=$SEED_BASE per-shard=$PER" >>"$EXP/note.txt"

# A shard that panicked leaves a missing/empty file; flag it loudly but keep
# the chain alive — score with matching shard subsets manually if this fires.
check() {
	local dir=$1 want=$2 ok=0
	for f in "$dir"/shard-*.json; do [ -s "$f" ] && ok=$((ok + 1)); done
	[ "$ok" -eq "$want" ] || log "!!! $dir has $ok/$want live shards — pair shards manually when scoring"
}

# Phase 1 — primary vs BEN Tier F (8 servers on 8085-8092, fragile: run first).
# A failed cell (servers down) logs loudly and the chain moves on — the guard
# phase must still run; re-running this script resumes the missing cells.
for arm in on "$KNOB-off"; do
	flags=()
	[ "$arm" = on ] || flags=("$OFFFLAG")
	for vul in none both; do
		dir="$EXP/ben-$arm/$vul"
		[ -s "$dir/shard-0.json" ] && { log "skip $dir (done)"; continue; }
		log "generate $dir"
		scripts/ben-gen-parallel.sh "$dir" "$PER" -v "$vul" -t f "${flags[@]+"${flags[@]}"}" \
			|| { log "!!! $dir failed — restore ~/ben + servers, re-run to resume"; continue; }
		check "$dir" 8
	done
done

# Phase 2 — guard vs BBA (same SEED_BASE; anchor-style cells)
for arm in on "$KNOB-off"; do
	flags=()
	[ "$arm" = on ] || flags=("$OFFFLAG")
	for vul in none both; do
		dir="$EXP/bba-$arm/$vul"
		[ -s "$dir/shard-0.json" ] && { log "skip $dir (done)"; continue; }
		log "generate $dir"
		scripts/bba-gen-parallel.sh "$dir" "$PER" -v "$vul" "${flags[@]+"${flags[@]}"}" \
			|| { log "!!! $dir failed"; continue; }
		check "$dir" "$(nproc)"
	done
done

# Phase 3 — scoring: per-arm pooled IMPs/board + paired on-vs-off diffs,
# both brackets (plain DD + PD), per vulnerability cell.
for ref in ben bba; do
	[ -s "$EXP/$ref-on/none/shard-0.json" ] || { log "no $ref data — skip scoring"; continue; }
	for arm in off "$KNOB"; do
		for vul in none both; do
			for score in plain pd; do
				out="$EXP/scores/$ref-$arm-$vul-$score.txt"
				[ -s "$out" ] && continue
				log "score $out"
				target/release/examples/bba-score "$EXP/$ref-$arm/$vul"/shard-*.json \
					--score "$score" >"$out" 2>&1 || log "!!! scoring failed: $out"
			done
		done
	done
	# One invocation per cell, both brackets: a scorer only re-prices solved
	# tables, so this pays for the shard parse and the DDS fan-out once.
	for vul in none both; do
		plain="$EXP/scores/diff-$ref-$KNOB-$vul-plain.txt"
		pd="$EXP/scores/diff-$ref-$KNOB-$vul-pd.txt"
		[ -s "$plain" ] && [ -s "$pd" ] && continue
		log "diff $plain + $pd"
		target/release/examples/ab-dump-diff "$EXP/$ref-on/$vul" "$EXP/$ref-$KNOB-off/$vul" \
			--score both --out-plain "$plain" --out-pd "$pd" >&2 \
			|| log "!!! diff failed: $plain + $pd"
	done
done

log "=== reading-knobs A/B done (knob=$KNOB): $EXP/scores/"
