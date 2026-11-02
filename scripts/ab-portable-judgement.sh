#!/bin/sh
# ab-portable-judgement.sh — Phase 1 of docs/portable-judgement.md: the matched
# retrain.  Four arms on paired deals, every one against BBA told the arm's
# own card:
#
#   american     the shipped 2/1 books over the shipped K = 8 floor
#   american-d   american-file: the same books, the net a logit mean over
#                AMERICAN_D (the distillation-only K = 8 on the american-read
#                corpus; the control)
#   bba          BBA's walked bands, the shipped net choosing (bba-system.md)
#   bba-d        bba-file: the same bands, the net a logit mean over BBA_D
#                (the same recipe on the bba-read corpus)
#
# With G(arm) the arm's plain loss to BBA (decompose.<arm>.md), the read is
#   T = [G(bba-d) − G(american-d)] − [G(bba) − G(american)];
# the paired diffs below give each bracket's terms on the same deals.
#
# Needs BBA_BOOK=<the --card none walk>, AMERICAN_D and BBA_D as
# comma-separated .f32 blob lists; on Compute-Node02 also
# BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen (CLAUDE.local.md).
#
#   BBA_BOOK=ab-results/bba-book/<run> AMERICAN_D=a1.f32,...,a8.f32 \
#       BBA_D=b1.f32,...,b8.f32 BOARDS=204800 \
#       setsid nohup scripts/idle-run.sh scripts/ab-portable-judgement.sh \
#       ab-results/portable-judgement >ab-results/portable-judgement.log 2>&1 &
#
# Optional AMERICAN_S adds a fifth arm, american-s: american-file over a
# distillation-only K = 8 on the stock-cell corpus (the shipped cut's walk,
# teacher labels), diffed against american and american-d.  It splits the
# control's gap to shipped into labels vs corpus (Phase 1 read, D1).
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir → a new seed).
R=${1:?usage: ab-portable-judgement.sh RESULTS_DIR}
: "${BBA_BOOK:?set BBA_BOOK to the walked book the bba arms play}"
: "${AMERICAN_D:?set AMERICAN_D to the american-d blobs, a.f32,b.f32,...}"
: "${BBA_D:?set BBA_D to the bba-d blobs, a.f32,b.f32,...}"
export BBA_BOOK
BUILD_EXTRA='--example bba-decompose'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)
echo "$BBA_BOOK" >"$R/bba-book"

# Pin each file arm's blobs as ab-floor-file.sh does: a resume with other
# weights would diff an arm drawn from two different nets.
ARMS="american american-d bba bba-d${AMERICAN_S:+ american-s}"
for a in american-d="$AMERICAN_D" bba-d="$BBA_D" ${AMERICAN_S:+american-s=$AMERICAN_S}; do
    n=${a%%=*}
    f=$R/weights.$n
    W=$(echo "${a#*=}" | tr , '\n' | xargs sha256sum)
    [ -s "$f" ] || echo "$W" >"$f"
    [ "$(cat "$f")" = "$W" ] || { echo "ab-portable-judgement: $n in $R was generated with other blobs (see $f)" >&2; exit 1; }
    log "arm $n: ${a#*=}"
done

# weights_of ARM — the blob list a file arm serves, empty for a shipped arm
weights_of() {
    case $1 in
        american-d) echo "$AMERICAN_D" ;;
        bba-d) echo "$BBA_D" ;;
        american-s) echo "$AMERICAN_S" ;;
    esac
}
# floor_of ARM — the --our-floor name an arm generates and replays under
floor_of() {
    case $1 in
        american-d | american-s) echo american-file ;;
        bba-d) echo bba-file ;;
        *) echo "$1" ;;
    esac
}

log "=== portable judgement start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul, BBA_BOOK=$BBA_BOOK"
for vul in $VULS; do
    for a in $ARMS; do
        # bba-gen --provenance reports PONS_FLOOR_WEIGHTS whenever it is set,
        # so a shipped arm must not inherit the previous file arm's export.
        unset PONS_FLOOR_WEIGHTS
        w=$(weights_of "$a")
        [ -z "$w" ] || { PONS_FLOOR_WEIGHTS=$w; export PONS_FLOOR_WEIGHTS; }
        arm "$a" "$vul" --our-floor "$(floor_of "$a")"
    done
    unset PONS_FLOOR_WEIGHTS
    diffpair american-d american   "$vul"
    diffpair bba-d      bba        "$vul"
    diffpair bba        american   "$vul"
    diffpair bba-d      american-d "$vul"
    if [ -n "${AMERICAN_S:-}" ]; then
        diffpair american-s american   "$vul"
        diffpair american-s american-d "$vul"
    fi
done

# Each arm against BBA: G(arm), the bucket ranking and the replay check.  One
# DD cache for all, since the deals are shared.
for a in $ARMS; do
    out="$R/decompose.$a.md"
    [ -s "$out" ] && { log "skip $out (exists)"; continue; }
    log "decompose $a vs BBA (all vuls, book census)"
    w=$(weights_of "$a")
    # shellcheck disable=SC2046
    env ${w:+PONS_FLOOR_WEIGHTS=$w} target/release/examples/bba-decompose \
        $(for vul in $VULS; do printf '%s ' "$R/$a-$vul"; done) \
        --our-floor "$(floor_of "$a")" --dd-cache "$R/dd-cache.json" --report "$out" --book-census \
        >"$R/decompose.$a.txt" 2>>"$R/log"
done
log "=== portable judgement done"
