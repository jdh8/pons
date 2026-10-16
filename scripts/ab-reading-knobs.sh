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
# external.  Do not touch the codebase while this runs.
#
# Why this runner does not source ab-lib.sh: its cells mix two generators and
# live at $EXP/{ben,bba}-ARM/VUL, a layout its recorded results already use.
# It shares ab-lib's guards instead (2026-10-16): one build up front, a cell
# resumes only when its arm.meta manifest (generator bytes, flags, deal set)
# matches and `{bba,ben}-gen-parallel.sh --verify` passes, and every score or
# diff is reused only while its .inputs stamp (both cells' manifests, the
# scorer's bytes, its flags) matches.  It used to skip a cell on a nonempty
# shard-0 and merely count live shards.  BEN_SHARDS is the server count the
# BEN cells were (and must be) generated with; BBA cells use JOBS shards
# (default nproc).  A cell from before the guard (no arm.meta) refuses unless
# FORCE_REUSE=1; the shard check still runs.
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
if [ ! -s "$EXP/seed" ]; then echo $(date +%s%N) >"$EXP/seed"; fi
SEED_BASE=$(cat "$EXP/seed")
export SEED_BASE
PER=${PER:-6400}
BEN_SHARDS=${BEN_SHARDS:-8}
BBA_SHARDS=${JOBS:-$(nproc)}

log() { echo "$(date -u +%FT%TZ) $*" >&2; }
log "=== reading-knobs A/B start: knob=$KNOB sha=$SHA SEED_BASE=$SEED_BASE per-shard=$PER"
echo "knob=$KNOB sha=$SHA SEED_BASE=$SEED_BASE per-shard=$PER" >>"$EXP/note.txt"

# One build for the whole run, then SKIP_BUILD=1: the parallel scripts would
# otherwise rebuild per cell, and the manifests below cksum these very bytes.
cargo build --release --features serde --example ben-gen --example bba-gen \
	--example bba-score --example ab-dump-diff
export SKIP_BUILD=1

# manifest GEN SHARDS [flags...] — what a cell runs: the generator's bytes (and
# BBA_GEN's launcher for bba-gen), its flags and its deal set
manifest() {
	local gen=$1 shards=$2
	shift 2
	echo "generator $gen $(cksum <"target/release/examples/$gen")"
	if [ "$gen" = bba-gen ] && [ -n "${BBA_GEN:-}" ]; then
		echo "launcher $(cksum <"$BBA_GEN")"
	fi
	echo "flags: $*"
	echo "shards: $shards"
	echo "per-shard: $PER"
	echo "seed-base: $SEED_BASE"
}

# cell GEN SHARDS DIR [flags...] — generate DIR unless it already holds this
# manifest's verified shards; non-zero on a refusal or a failed run
cell() {
	local gen=$1 shards=$2 dir=$3 want
	shift 3
	want=$(manifest "$gen" "$shards" "$@")
	if ls "$dir"/shard-*.json >/dev/null 2>&1; then
		if [ "${FORCE_REUSE:-0}" != 1 ] &&
			[ "$(grep -v '^# ' "$dir/arm.meta" 2>/dev/null || :)" != "$want" ]; then
			log "!!! refuse $dir: its arm.meta does not match this generator, flags and deal set (missing = before the guard); FORCE_REUSE=1 after checking"
			return 1
		fi
		if SHARDS=$shards JOBS=$shards "scripts/$gen-parallel.sh" --verify "$dir" "$PER"; then
			log "skip $dir (verified)"
			return 0
		fi
		log "regenerate $dir: shard verification failed"
	fi
	mkdir -p "$dir"
	{
		echo "$want"
		echo "# source: $(git rev-parse HEAD)"
		echo "# written: $(date -u +%FT%TZ)"
	} >"$dir/arm.meta"
	log "generate $dir"
	SHARDS=$shards JOBS=$shards "scripts/$gen-parallel.sh" "$dir" "$PER" "$@"
}

# Phases 1 and 2 — primary vs BEN Tier F (BEN_SHARDS servers on 8085+, fragile:
# run first), then the guard vs BBA on the same SEED_BASE.  A failed cell
# (servers down) logs loudly and the chain moves on — the guard phase must
# still run; re-running this script resumes the missing cells.  A reference
# with any failed cell is not scored.
declare -A complete=([ben]=1 [bba]=1)
for ref in ben bba; do
	for arm in on "$KNOB-off"; do
		flags=()
		[ "$arm" = on ] || flags=("$OFFFLAG")
		for vul in none both; do
			dir="$EXP/$ref-$arm/$vul"
			if [ "$ref" = ben ]; then
				cell ben-gen "$BEN_SHARDS" "$dir" -v "$vul" -t f "${flags[@]+"${flags[@]}"}"
			else
				cell bba-gen "$BBA_SHARDS" "$dir" -v "$vul" "${flags[@]+"${flags[@]}"}"
			fi || {
				log "!!! $dir failed — for BEN restore ~/ben + servers; re-run to resume"
				complete[$ref]=0
			}
		done
	done
done

# inputs SCORER FLAGS DIR... — a report's stamp: its cells' manifests, the
# scorer's bytes and its flags
inputs() {
	local scorer=$1 flags=$2
	shift 2
	for d in "$@"; do
		echo "cell $d"
		cat "$d/arm.meta"
	done
	echo "scorer $(cksum <"$scorer")"
	echo "flags: $flags"
}

# Phase 3 — scoring: per-arm pooled IMPs/board + paired on-vs-off diffs,
# both brackets (plain DD + PD), per vulnerability cell.
score=target/release/examples/bba-score
diff=target/release/examples/ab-dump-diff
for ref in ben bba; do
	[ "${complete[$ref]}" = 1 ] || { log "!!! $ref cells incomplete — skip scoring"; continue; }
	for arm in on "$KNOB-off"; do
		for vul in none both; do
			for bracket in plain pd; do
				out="$EXP/scores/$ref-$arm-$vul-$bracket.txt"
				stamp=$(inputs "$score" "--score $bracket" "$EXP/$ref-$arm/$vul")
				[ -s "$out" ] && [ "$(cat "$out.inputs" 2>/dev/null)" = "$stamp" ] && continue
				log "score $out"
				if "$score" "$EXP/$ref-$arm/$vul"/shard-*.json --score "$bracket" >"$out" 2>&1; then
					printf '%s\n' "$stamp" >"$out.inputs"
				else
					log "!!! scoring failed: $out"
				fi
			done
		done
	done
	# One invocation per cell, both brackets: a scorer only re-prices solved
	# tables, so this pays for the shard parse and the DDS fan-out once.
	for vul in none both; do
		plain="$EXP/scores/diff-$ref-$KNOB-$vul-plain.txt"
		pd="$EXP/scores/diff-$ref-$KNOB-$vul-pd.txt"
		stamp=$(inputs "$diff" "--score both" "$EXP/$ref-on/$vul" "$EXP/$ref-$KNOB-off/$vul")
		[ -s "$plain" ] && [ -s "$pd" ] &&
			[ "$(cat "${plain%-plain.txt}.inputs" 2>/dev/null)" = "$stamp" ] && continue
		log "diff $plain + $pd"
		if "$diff" "$EXP/$ref-on/$vul" "$EXP/$ref-$KNOB-off/$vul" \
			--score both --out-plain "$plain" --out-pd "$pd" >&2; then
			printf '%s\n' "$stamp" >"${plain%-plain.txt}.inputs"
		else
			log "!!! diff failed: $plain + $pd"
		fi
	done
done

log "=== reading-knobs A/B done (knob=$KNOB): $EXP/scores/"
