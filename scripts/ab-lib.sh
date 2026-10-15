# ab-lib.sh — shared plumbing for the scripts/*-ab.sh A/B runners.  Not
# executable on its own; a runner sets R (and optionally the knobs below) then
# sources it, and is left with only its experiment body — the arms and diff
# pairs — to spell out.  Sourcing turns on `set -eu`, cds to the repo root,
# builds the harnesses, and defines log / arm / gatepair / diffpair / sddiff /
# seed_for.
#
# Honored if set before sourcing:
#   BOARDS       total boards per arm per vul; sets PER_SHARD = BOARDS/JOBS
#   PER_SHARD    boards per shard per arm per vul     (default 6400)
#   SHOW         worst boards ab-dump-diff prints      (default 5)
#   BUILD_EXTRA  extra `cargo build` --example flags   (e.g. --example ab-dump-sd)
#
# Sourcing exports SKIP_BUILD=1, so the build it does here is the only one the
# whole experiment gets; see the comment at that line.
#
# A runner that scores single-dummy sets BUILD_EXTRA='--example ab-dump-sd' and
# calls sddiff; the two split-by-opening runners override diffpair/sddiff with
# dir-based variants after sourcing (their shard dirs are split by strain).
set -eu
cd "$(dirname "$0")/.."

: "${R:?source ab-lib.sh with R set to the results dir}"
mkdir -p "$R"
SHA=$(git rev-parse --short HEAD)$(git diff --quiet HEAD || echo -dirty)
DIFF=target/release/examples/ab-dump-diff
SD=target/release/examples/ab-dump-sd
PROBE=target/release/examples/probe-divergence
SHOW=${SHOW:-5}
SHARDS=${JOBS:-$(nproc)}   # shard count bba-gen-parallel.sh creates; runners log it
# BOARDS = total boards per arm per vul, the knob that actually sets statistical
# power; when given, PER_SHARD is derived so JOBS is pure parallelism (rounded up
# to a multiple of 4 per shard for dealer balance).  PER_SHARD alone still works,
# but note total = PER_SHARD × JOBS, so changing JOBS then rescales the sample.
if [ -n "${BOARDS:-}" ]; then
    PER_SHARD=$(( (BOARDS / SHARDS + 3) / 4 * 4 ))
fi
PER_SHARD=${PER_SHARD:-6400}

# Pairing guard: every arm of one results dir must be generated with the same
# shard count — shard i seeds SEED_BASE+i, so resuming a dir with a different
# JOBS would diff arms drawn from different deal sets.
# Both halves: a resume that keeps JOBS but drops BOARDS falls back to 6400/shard
# (ab-lib.sh:38), and if BOTH arms of a vulnerability are still missing it
# regenerates them aligned -- every assert passes and the headline is quoted on a
# silently smaller sample.  Pin the count too, so that fails loudly instead.
for k in shards:$SHARDS per-shard:$PER_SHARD; do
    f="$R/${k%%:*}"; want="${k#*:}"
    if [ -s "$f" ]; then
        [ "$(cat "$f")" = "$want" ] || {
            echo "ab-lib: $R was generated with ${k%%:*}=$(cat "$f"), not $want; rerun with JOBS=$(cat "$R/shards") and BOARDS=<shards x per-shard as recorded in $R>, or use a fresh results dir" >&2
            exit 1
        }
    else
        echo "$want" >"$f"
    fi
done

# SKIP_BUILD=1 from the caller skips even this build: the documented way to
# launch a second experiment while another is in flight, whose binaries a
# rebuild would swap under it.  The run then measures whatever is already in
# target/release, so every harness it needs must exist.
if [ "${SKIP_BUILD:-0}" = 1 ]; then
    # shellcheck disable=SC2086
    for ex in bba-gen ab-dump-diff $(echo ${BUILD_EXTRA:-} | sed 's/--example//g'); do
        [ -x "target/release/examples/$ex" ] || {
            echo "ab-lib: SKIP_BUILD=1 but target/release/examples/$ex is missing" >&2
            exit 1
        }
    done
    echo "ab-lib: SKIP_BUILD=1, measuring the binaries already in target/release" >&2
else
    # BUILD_EXTRA is a deliberately word-split flag list, not one argument.
    # shellcheck disable=SC2086
    cargo build --release --features serde --example bba-gen --example ab-dump-diff ${BUILD_EXTRA:-}
fi

# One build per experiment, here.  bba-gen-parallel.sh otherwise rebuilds at the
# head of *every arm*, so a `src/` edit mid-run makes late arms measure different
# code from early ones with nothing in the log to say so.  With this exported it
# hard-fails on a missing binary instead.  Cost: a run started on a dirty or
# stale tree measures this build for all its arms -- which is the point.
export SKIP_BUILD=1

log() { echo "$(date -u +%FT%TZ) $*" | tee -a "$R/log" >&2; }

# arm NAME VUL [bba-gen flags...] — generate one arm unless already present
#
# The resume check VERIFIES the arm (`bba-gen-parallel.sh --verify`): exactly
# shards 0..SHARDS-1, each parsing as PER_SHARD boards dealt from SEED_BASE+i.
# Testing the directory resumes past an arm that died on startup (bba-gen-
# parallel mkdirs first); testing shard-0, or merely counting shards, resumes
# past a short or truncated arm -- and if both arms lost the SAME index (one
# deterministic crash, same deal stream) every downstream assert still passes.
# An arm that fails the check is regenerated whole; bba-gen-parallel now exits
# non-zero on any failed worker or unverified shard, which stops the runner.
#
# An arm is named by role (`shipped`, `off`), not by what it measured, and a
# default flip between two sessions changes what `shipped` means.  So every arm
# records a MANIFEST in $dir/arm.meta, and a resume refuses an arm, complete or
# partial, whose manifest differs or was never recorded (an arm from before the
# guard).  The compared part is the runtime fingerprint below plus the deal
# set: `shards:`, `per-shard:` and `seed-base:`.  Lines starting `# ` are
# provenance, recorded and never compared: the source SHA, a cksum of the
# dirty diff, JOBS, BOARDS and the date.  The SHA is deliberately not
# identity -- the binary's bytes are (see below), so a doc-only commit during a
# long run does not wedge its resume.  An arm.meta from before the manifest
# (fingerprint lines only) still resumes when its fingerprint matches, with a
# warning: the shard check then stands in for the deal-set lines, since it
# pins every shard's seed and board count.  FORCE_REUSE=1 reuses a mismatched
# arm anyway, for a rescore you have checked by hand; it never skips the shard
# check, and an arm that fails it is regenerated under the current manifest.
#
# The fingerprint is what the arm RUNS, not the checkout: the source tree is no
# witness for a SKIP_BUILD=1 binary that predates it, nor for a BBA_GEN
# override.  It cksums the files `bba-gen --provenance` names -- the image its
# code was mapped from (seen through a BBA_GEN loader wrapper, so rebuilding
# the wrapped binary counts), the EPBot library, the PONS_FLOOR_WEIGHTS blobs
# -- plus the launcher, flags and every flag value naming a file (a `.bbsa`
# card). A legacy generator needs a wrapper that answers `--provenance` with
# its actual executable, library and weights; guessing what a script wraps
# cannot establish identity. Empty/incomplete reports and unreadable files
# fail closed, including on fresh arms. FORCE_REUSE=1 explicitly overrides a
# failed provenance check as well as a mismatched arm.meta. Doc-only commits
# and unrebuilt source edits leave the fingerprint unchanged; a relink that
# changes the bytes refuses.
#
# A subshell body, so its names cannot clobber a runner's (`for a; do arm …`).
arm_fingerprint() (
    # Resolved as bba-gen-parallel.sh resolves it; keep the two in step.
    bin=${BBA_GEN:-$PWD/target/release/examples/bba-gen}
    report=$("$bin" --provenance 2>/dev/null) || {
        echo "ab-lib: $bin does not report runtime provenance; use a wrapper that answers --provenance with its executable, library and weights" >&2
        return 1
    }
    fingerprint_file() {
        [ -f "$2" ] && [ -r "$2" ] && sum=$(cksum <"$2") || {
            echo "ab-lib: cannot fingerprint $1 file: $2" >&2
            return 1
        }
        echo "$1 $sum"
    }
    launcher=$(command -v "$bin") || return 1
    fingerprint_file launcher "$launcher" || return 1
    seen_exe=0; seen_lib=0
    while read -r role path; do
        case "$role" in
            exe) seen_exe=1 ;;
            lib) seen_lib=1 ;;
            weights) ;;
            *) echo "ab-lib: invalid --provenance role: $role" >&2; return 1 ;;
        esac
        fingerprint_file "$role" "$path" || return 1
    done <<EOF
$report
EOF
    [ "$seen_exe:$seen_lib" = 1:1 ] || {
        echo "ab-lib: --provenance must name both exe and lib files" >&2
        return 1
    }
    echo "flags: $*"
    for a in "$@"; do
        f=${a#*=}
        if [ -f "$f" ]; then fingerprint_file "file $f" "$f" || return 1; fi
    done
)
arm() {
    name=$1; vul=$2; shift 2
    dir="$R/$name-$vul"
    : "${SEED_BASE:?arm needs SEED_BASE (seed_for) so its arms share deals}"
    if fp=$(arm_fingerprint "$@"); then
        :
    elif [ "${FORCE_REUSE:-0}" = 1 ]; then
        log "WARNING $dir: FORCE_REUSE=1 overrides failed runtime provenance; identity is unverified"
        fp='unverified runtime provenance (FORCE_REUSE=1)'
    else
        log "refuse $dir: cannot establish runtime provenance; use a reporting generator/wrapper, or FORCE_REUSE=1 after checking"
        exit 1
    fi
    want="$fp
shards: $SHARDS
per-shard: $PER_SHARD
seed-base: $SEED_BASE"
    if ls "$dir"/shard-*.json >/dev/null 2>&1 && [ "${FORCE_REUSE:-0}" != 1 ]; then
        have=$(grep -v '^# ' "$dir/arm.meta" 2>/dev/null || :)
        if [ "$have" = "$want" ]; then
            :
        elif [ -n "$have" ] && [ "$have" = "$fp" ]; then
            log "WARNING $dir: arm.meta predates the manifest (fingerprint only); its deal set is checked by the shard verification alone"
        else
            log "refuse $dir: its arm.meta manifest does not match these runtime files, flags and deal set (missing = generated before the guard); use a fresh results dir, or FORCE_REUSE=1 after checking"
            exit 1
        fi
    fi
    if ls "$dir"/shard-*.json >/dev/null 2>&1; then
        if JOBS=$SHARDS SEED_BASE=$SEED_BASE scripts/bba-gen-parallel.sh --verify "$dir" "$PER_SHARD" \
            >>"$R/log" 2>&1; then
            log "skip $dir (complete, verified)"
            return 0
        fi
        log "regenerate $dir: shard verification failed (details above in $R/log)"
    fi
    mkdir -p "$dir"
    {
        echo "$want"
        echo "# source: $(git rev-parse HEAD 2>/dev/null || echo unknown)"
        echo "# dirty-diff: $(git diff HEAD 2>/dev/null | cksum)"
        echo "# jobs: ${JOBS:-} boards: ${BOARDS:-}"
        echo "# written: $(date -u +%FT%TZ)"
    } >"$dir/arm.meta"
    log "generate $dir (SEED_BASE=$SEED_BASE, flags: $*)"
    SEED_BASE=$SEED_BASE JOBS=$SHARDS scripts/bba-gen-parallel.sh "$dir" "$PER_SHARD" -v "$vul" "$@" \
        >>"$R/log" 2>&1 || {
        log "FAILED $dir: bba-gen-parallel exited non-zero (see $R/log); rerun to regenerate the arm"
        exit 1
    }
}

# report_inputs BIN ON OFF VUL [flags...] — what a report was computed from:
# both arms' manifests, the scorer's bytes and its flags.  A report is reused
# only while its .inputs stamp still matches, so a regenerated arm, a rebuilt
# scorer or a changed flag rescores it.  A report with no stamp (an older
# results dir) is rescored once, then stamped.
report_inputs() (
    bin=$1; shift
    for a in "$1" "$2"; do
        echo "arm $a-$3"
        cat "$R/$a-$3/arm.meta" 2>/dev/null || echo "(no arm.meta)"
    done
    shift 3
    echo "scorer $(cksum <"$bin")"
    echo "flags: $*"
)
# fresh_report STAMP INPUTS — the report's stamp records exactly INPUTS
fresh_report() { [ "$(cat "$1" 2>/dev/null)" = "$2" ]; }

# gatepair ON OFF VUL [ours|theirs] — the isolation gate before any headline
# (docs/measurement.md): it must read **0 foreign**, i.e. no divergent board was
# opened by the other side.  Needs BUILD_EXTRA='--example probe-divergence'.
#
# Do NOT shadow this in a runner.  Eleven did (all with the pre-fix `-s` guard
# below), which made the 2026-09-01 fix inert for them until they were deleted;
# ab-nt-natural-overcall.sh needed `theirs`, which is now the 4th argument.
gatepair() {
    on=$1; off=$2; vul=$3; side=${4:-ours}
    out="$R/gate.$on.vs.$off.$vul.txt"
    # Guard on the PASSED line, not on the file.  probe-divergence writes its
    # FAILED summary to the same stdout it is judged by (examples/probe-
    # divergence/main.rs:419 bails *after* the shell redirect has filled $out),
    # so an `-s` guard makes a failed gate sticky: the resume skips it and goes
    # on to print the very headline the runner pre-registers the gate against.
    inputs=$(report_inputs "$PROBE" "$on" "$off" "$vul" --gate-opener "$side")
    grep -q 'isolation gate PASSED' "$out" 2>/dev/null && fresh_report "$out.inputs" "$inputs" \
        && { log "skip $out (passed)"; return 0; }
    log "isolation gate $on vs $off ($vul)"
    "$PROBE" "$R/$on-$vul" "$R/$off-$vul" --gate-opener "$side" >"$out"
    printf '%s\n' "$inputs" >"$out.inputs"
}

# diffpair ON OFF VUL — paired diff over the whole arm, plain + pd.  ab-dump-diff
# folds every shard of the arm dir into a single DDS fan-out — one solver owns
# all cores instead of one process per shard oversubscribing the box.
#
# One invocation, both brackets: a scorer only re-prices solved tables, so
# `--score both` pays for the shard parse and the DDS fan-out once.  Running it
# twice, once per scorer, paid for each twice.
#
# The skip guard wants BOTH files and a matching .inputs stamp (report_inputs),
# written only after both: a run killed between the two writes redoes the cell
# instead of resuming past a half-written pair, and a pair scored from other
# arms or by another scorer build is rescored, not reused.
diffpair() {
    on=$1; off=$2; vul=$3
    plain="$R/diff.$on.vs.$off.$vul.plain.txt"
    pd="$R/diff.$on.vs.$off.$vul.pd.txt"
    stamp="$R/diff.$on.vs.$off.$vul.inputs"
    inputs=$(report_inputs "$DIFF" "$on" "$off" "$vul" --score both --show "$SHOW")
    [ -s "$plain" ] && [ -s "$pd" ] && fresh_report "$stamp" "$inputs" \
        && { log "skip $plain + $pd (exist, inputs unchanged)"; return 0; }
    log "diff $on vs $off ($vul, plain+pd)"
    "$DIFF" "$R/$on-$vul" "$R/$off-$vul" --score both \
        --out-plain "$plain" --out-pd "$pd" --show "$SHOW" >>"$R/log" 2>&1
    printf '%s\n' "$inputs" >"$stamp"
}

# sddiff ON OFF VUL [ab-dump-sd flags...] — sd-lead paired delta over the whole
# arm, 16 worlds, one solver; extra flags (e.g. --on-ns-negative-double-shape)
# disclose the ON arm's knobs to the blind leader.
sddiff() {
    on=$1; off=$2; vul=$3; shift 3
    out="$R/sd.$on.vs.$off.$vul.txt"
    # Guard on the result line, not on the file -- same reason as gatepair, and
    # worse here: `2>&1` below folds a panic (ab-dump-sd asserts the arms are
    # aligned) into the very file the guard reads, so an `-s` guard skips the
    # cell permanently and the sd column silently goes missing.
    inputs=$(report_inputs "$SD" "$on" "$off" "$vul" -v "$vul" --sd-worlds 16 --show 0 "$@")
    grep -q '^Delta' "$out" 2>/dev/null && fresh_report "$out.inputs" "$inputs" \
        && { log "skip $out (scored)"; return 0; }
    log "sd-diff $on vs $off ($vul, 16 worlds$*)"
    "$SD" "$R/$on-$vul" "$R/$off-$vul" -v "$vul" --sd-worlds 16 --show 0 "$@" >"$out" 2>&1
    printf '%s\n' "$inputs" >"$out.inputs"
}

# seed_for [NAME] — a persistent SEED_BASE in $R/[NAME.]seed, fresh on first use.
# No NAME: one $R/seed for the whole run; NAME: one $R/NAME.seed per experiment.
seed_for() {
    f="$R/${1:+$1.}seed"
    if [ ! -s "$f" ]; then date +%s%N >"$f"; fi
    cat "$f"
}
