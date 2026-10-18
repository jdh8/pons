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
#   VULS         vulnerability cells a runner loops    (default "none both";
#                "none ns ew both" for a competitive treatment, see
#                docs/measurement.md checklist 8 — we sit N/S, so `ns` is
#                unfavourable and `ew` favourable)
#   DENSITY      accepted ÷ scanned of a --filter-* run; when set, diffpair and
#                sddiff pass it as --density so the verdict is not withheld
#                (density_of computes it from the arm's shards in $R/log)
#   GENERATOR    bba-gen (default) or ben-gen: the opponent every `arm` bids
#                against, through scripts/$GENERATOR-parallel.sh.  A ben-gen
#                runner sets JOBS to its BEN server count (the shard count,
#                which ben-gen-parallel then insists on) and passes `-t f
#                --note "$(ben_note)"` in each arm's flags; BBA_GEN applies to
#                bba-gen only.
#   BEN_DIR      the BEN checkout ben-servers.sh launched (default ~/ben); its
#                run/servers.meta is part of every ben-gen arm's fingerprint
#
# Sourcing exports SKIP_BUILD=1, so the build it does here is the only one the
# whole experiment gets; see the comment at that line.
#
# A runner that scores single-dummy sets BUILD_EXTRA='--example ab-dump-sd' and
# calls sddiff; the two split-by-opening runners score strain-split copies of
# each arm through report_inputs/fresh_report (their shard dirs are split by
# strain), and the binary-arm runners export BBA_GEN per arm.  Do not shadow
# arm/diffpair/sddiff: a shadow keeps its own resume check and silently loses
# the manifest, the shard verification and the report stamps below.
set -eu
cd "$(dirname "$0")/.."

: "${R:?source ab-lib.sh with R set to the results dir}"
mkdir -p "$R"
SHA=$(git rev-parse --short HEAD)$(git diff --quiet HEAD || echo -dirty)
DIFF=target/release/examples/ab-dump-diff
SD=target/release/examples/ab-dump-sd
PROBE=target/release/examples/probe-divergence
SHOW=${SHOW:-5}
VULS=${VULS:-none both}
GENERATOR=${GENERATOR:-bba-gen}
BEN_META=${BEN_DIR:-$HOME/ben}/run/servers.meta
case "$GENERATOR" in
    bba-gen | ben-gen) ;;
    *) echo "ab-lib: GENERATOR must be bba-gen or ben-gen, not $GENERATOR" >&2; exit 1 ;;
esac
PARALLEL=scripts/$GENERATOR-parallel.sh
SHARDS=${JOBS:-$(nproc)}   # shard count $PARALLEL creates; runners log it
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
    for ex in "$GENERATOR" ab-dump-diff $(echo ${BUILD_EXTRA:-} | sed 's/--example//g'); do
        [ -x "target/release/examples/$ex" ] || {
            echo "ab-lib: SKIP_BUILD=1 but target/release/examples/$ex is missing" >&2
            exit 1
        }
    done
    echo "ab-lib: SKIP_BUILD=1, measuring the binaries already in target/release" >&2
else
    # BUILD_EXTRA is a deliberately word-split flag list, not one argument.
    # shellcheck disable=SC2086
    cargo build --release --features serde --example "$GENERATOR" --example ab-dump-diff ${BUILD_EXTRA:-}
fi

# One build per experiment, here.  $PARALLEL otherwise rebuilds at the
# head of *every arm*, so a `src/` edit mid-run makes late arms measure different
# code from early ones with nothing in the log to say so.  With this exported it
# hard-fails on a missing binary instead.  Cost: a run started on a dirty or
# stale tree measures this build for all its arms -- which is the point.
export SKIP_BUILD=1

log() { echo "$(date -u +%FT%TZ) $*" | tee -a "$R/log" >&2; }

# arm NAME VUL [generator flags...] — generate one arm unless already present
#
# The resume check VERIFIES the arm (`$PARALLEL --verify`): exactly shards
# 0..SHARDS-1, each parsing as PER_SHARD boards dealt from SEED_BASE+i.
# Testing the directory resumes past an arm that died on startup (the parallel
# scripts mkdir first); testing shard-0, or merely counting shards, resumes
# past a short or truncated arm -- and if both arms lost the SAME index (one
# deterministic crash, same deal stream) every downstream assert still passes.
# An arm that fails the check is regenerated whole; both parallel scripts exit
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
# override.  It cksums the files `$GENERATOR --provenance` names -- the image its
# code was mapped from (seen through a BBA_GEN loader wrapper, so rebuilding
# the wrapped binary counts), the EPBot library, the PONS_FLOOR_WEIGHTS blobs
# -- plus the launcher, flags and every flag value naming a file (a `.bbsa`
# card). A legacy generator needs a wrapper that answers `--provenance` with
# its actual executable, library and weights; guessing what a script wraps
# cannot establish identity. Empty/incomplete reports and unreadable files
# fail closed, including on fresh arms. FORCE_REUSE=1 explicitly overrides a
# failed provenance check as well as a mismatched arm.meta. Doc-only commits
# and unrebuilt source edits leave the fingerprint unchanged; a relink that
# changes the bytes refuses.  A ben-gen arm's fingerprint also cksums the BEN
# fleet's manifest, $BEN_META, which scripts/ben-servers.sh writes at start
# and removes at stop (BEN's commit and patches, the conf, the interpreter and
# its packages, the server flags, tier and fleet size): no manifest, no arm,
# and a fleet restarted from another runtime refuses a resume.  Each arm keeps
# a copy as $dir/servers.meta, since the live one is rewritten by the next
# start.
#
# A subshell body, so its names cannot clobber a runner's (`for a; do arm …`).
arm_fingerprint() (
    # Resolved as $PARALLEL resolves it; keep them in step.  ben-gen has no
    # override: ben-gen-parallel.sh always runs this checkout's build.
    if [ "$GENERATOR" = bba-gen ]; then
        bin=${BBA_GEN:-$PWD/target/release/examples/bba-gen}
    else
        bin=$PWD/target/release/examples/ben-gen
    fi
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
    if [ "$GENERATOR" = ben-gen ]; then
        fingerprint_file servers "$BEN_META" || return 1
    fi
    echo "flags: $*"
    for a in "$@"; do
        f=${a#*=}
        if [ -f "$f" ]; then fingerprint_file "file $f" "$f" || return 1; fi
    done
)
# ben_note — a ben-gen arm's `--note`: the fleet manifest's sha256, so every
# shard carries the identity of the servers that bid it (load_dump compares
# notes across a shard dir).  Fails when no fleet manifest exists.
ben_note() {
    sum=$(sha256sum <"$BEN_META") || {
        echo "ab-lib: no BEN fleet manifest at $BEN_META; start the servers with scripts/ben-servers.sh" >&2
        return 1
    }
    echo "ben-servers-sha256=${sum%% *}"
}
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
        # JOBS is bba-gen-parallel's shard count, SHARDS ben-gen-parallel's.
        if JOBS=$SHARDS SHARDS=$SHARDS SEED_BASE=$SEED_BASE "$PARALLEL" --verify "$dir" "$PER_SHARD" \
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
    if [ "$GENERATOR" = ben-gen ]; then cp "$BEN_META" "$dir/servers.meta"; fi
    log "generate $dir (SEED_BASE=$SEED_BASE, flags: $*)"
    SEED_BASE=$SEED_BASE JOBS=$SHARDS SHARDS=$SHARDS "$PARALLEL" "$dir" "$PER_SHARD" -v "$vul" "$@" \
        >>"$R/log" 2>&1 || {
        log "FAILED $dir: $PARALLEL exited non-zero (see $R/log); rerun to regenerate the arm"
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
    # shellcheck disable=SC2086
    inputs=$(report_inputs "$DIFF" "$on" "$off" "$vul" --score both --show "$SHOW" \
        ${DENSITY:+--density $DENSITY})
    [ -s "$plain" ] && [ -s "$pd" ] && fresh_report "$stamp" "$inputs" \
        && { log "skip $plain + $pd (exist, inputs unchanged)"; return 0; }
    log "diff $on vs $off ($vul, plain+pd${DENSITY:+, density $DENSITY})"
    # shellcheck disable=SC2086
    "$DIFF" "$R/$on-$vul" "$R/$off-$vul" --score both \
        --out-plain "$plain" --out-pd "$pd" --show "$SHOW" \
        ${DENSITY:+--density $DENSITY} >>"$R/log" 2>&1
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
    # shellcheck disable=SC2086
    inputs=$(report_inputs "$SD" "$on" "$off" "$vul" -v "$vul" --sd-worlds 16 --show 0 \
        ${DENSITY:+--density $DENSITY} "$@")
    grep -q '^Delta' "$out" 2>/dev/null && fresh_report "$out.inputs" "$inputs" \
        && { log "skip $out (scored)"; return 0; }
    log "sd-diff $on vs $off ($vul, 16 worlds$*)"
    # shellcheck disable=SC2086
    "$SD" "$R/$on-$vul" "$R/$off-$vul" -v "$vul" --sd-worlds 16 --show 0 \
        ${DENSITY:+--density $DENSITY} "$@" >"$out" 2>&1
    printf '%s\n' "$inputs" >"$out.inputs"
}

# density_of ARM_DIR — accepted ÷ scanned of an enriched (--filter-*) arm, from
# its shards' `wrote N boards (M scanned) to ARM_DIR/shard-i.json` lines in
# $R/log; the last line per shard wins, so a regenerated arm counts once.  A
# resumed arm keeps the lines its generation wrote.  Fails on an arm with no
# such line (generated elsewhere), rather than printing a density of nothing.
density_of() {
    awk -v pre="$1/shard-" '
        /wrote [0-9]+ boards \([0-9]+ scanned\) to / {
            for (i = 1; i < NF; i++)
                if ($i == "to" && index($(i + 1), pre) == 1) {
                    for (j = 1; j < i; j++)
                        if ($j == "wrote") { acc[$(i + 1)] = $(j + 1); sc[$(i + 1)] = substr($(j + 3), 2) }
                }
        }
        END {
            for (k in acc) { a += acc[k]; n += sc[k] }
            if (n == 0) exit 1
            printf "%.6f\n", a / n
        }' "$R/log"
}

# seed_for [NAME] — a persistent SEED_BASE in $R/[NAME.]seed, fresh on first use.
# No NAME: one $R/seed for the whole run; NAME: one $R/NAME.seed per experiment.
seed_for() {
    f="$R/${1:+$1.}seed"
    if [ ! -s "$f" ]; then date +%s%N >"$f"; fi
    cat "$f"
}
