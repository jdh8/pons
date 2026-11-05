#!/bin/sh
# ab-portable-judgement-2.sh — Phase 2 of docs/portable-judgement.md: three
# nets × three books on paired deals, every arm against BBA told the arm's own
# card.  Arm `<net>-<domain>`:
#
#   nets     american-d  AMERICAN_D, the K = 8 on the american relabel cut
#            bba-d       BBA_D, the same recipe on the bba relabel cut
#            union-d     UNION_D, the same recipe on both cuts (D3 order)
#   domains  am          american-file: our shipped 2/1 books
#            bba         bba-file over BBA_BOOK (the --card none walk)
#            pc          bba-file over PC_BOOK (the --card 2 walk, Polish
#                        Club), against BBA playing WJ too (`--system 2`):
#                        the walk seats WJ on all four seats and the bba
#                        floors read their calls off it (§8)
#
# No net trained on pc: it is the held-out book.  The reads (§4 Phase 2):
# transfer symmetry (bba-d-am vs american-d-am, american-d-bba vs bba-d-bba),
# the union's price (union-d vs the single-domain net on am and on bba) and
# the held-out cell (the three nets on pc).
#
#   BBA_BOOK=ab-results/bba-book/<none walk> PC_BOOK=ab-results/bba-book/<pc walk> \
#       AMERICAN_D=a1.f32,... BBA_D=b1.f32,... UNION_D=u1.f32,... BOARDS=204800 \
#       scripts/idle-run.sh scripts/ab-portable-judgement-2.sh ab-results/portable-judgement-2
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir → a new seed).
R=${1:?usage: ab-portable-judgement-2.sh RESULTS_DIR}
: "${BBA_BOOK:?set BBA_BOOK to the --card none walk}"
: "${PC_BOOK:?set PC_BOOK to the --card 2 walk}"
: "${AMERICAN_D:?set AMERICAN_D}" "${BBA_D:?set BBA_D}" "${UNION_D:?set UNION_D}"
BUILD_EXTRA='--example bba-decompose'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)
NONE_BOOK=$BBA_BOOK
printf '%s\n%s\n' "$NONE_BOOK" "$PC_BOOK" >"$R/bba-book"

# Pin each net's blobs as ab-portable-judgement.sh does.
for a in american-d="$AMERICAN_D" bba-d="$BBA_D" union-d="$UNION_D"; do
    n=${a%%=*}
    f=$R/weights.$n
    W=$(echo "${a#*=}" | tr , '\n' | xargs sha256sum)
    [ -s "$f" ] || echo "$W" >"$f"
    [ "$(cat "$f")" = "$W" ] || { echo "ab-portable-judgement-2: $n in $R was generated with other blobs (see $f)" >&2; exit 1; }
    log "net $n: ${a#*=}"
done
NETS="american-d bba-d union-d"
DOMAINS="am bba pc"

weights_of() {
    case $1 in
        american-d-*) echo "$AMERICAN_D" ;;
        bba-d-*) echo "$BBA_D" ;;
        union-d-*) echo "$UNION_D" ;;
    esac
}
# setup ARM — export the arm's net and book; FLOOR and THEIRS its flags
# (called in this shell, not $(...), so the exports reach `arm`)
setup() {
    PONS_FLOOR_WEIGHTS=$(weights_of "$1")
    export PONS_FLOOR_WEIGHTS
    THEIRS=
    case $1 in
        *-am) FLOOR=american-file ;;
        *-bba) FLOOR=bba-file BBA_BOOK=$NONE_BOOK ;;
        *-pc) FLOOR=bba-file BBA_BOOK=$PC_BOOK THEIRS='--system 2' ;;
    esac
    export BBA_BOOK
}

log "=== portable judgement 2 start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul, none=$NONE_BOOK pc=$PC_BOOK"
for vul in $VULS; do
    for d in $DOMAINS; do
        for n in $NETS; do
            setup "$n-$d"
            # shellcheck disable=SC2086
            arm "$n-$d" "$vul" --our-floor "$FLOOR" $THEIRS
        done
    done
    diffpair bba-d-am      american-d-am  "$vul"
    diffpair american-d-bba bba-d-bba     "$vul"
    diffpair union-d-am    american-d-am  "$vul"
    diffpair union-d-bba   bba-d-bba      "$vul"
    diffpair union-d-pc    american-d-pc  "$vul"
    diffpair union-d-pc    bba-d-pc       "$vul"
    diffpair bba-d-pc      american-d-pc  "$vul"
done

# Each arm against BBA: G(arm), the bucket ranking and the replay check.  One
# DD cache for all, since a table is a function of the deal alone.
for d in $DOMAINS; do
    for n in $NETS; do
        a=$n-$d
        out="$R/decompose.$a.md"
        [ -s "$out" ] && { log "skip $out (exists)"; continue; }
        log "decompose $a vs BBA (all vuls, book census)"
        setup "$a"
        # shellcheck disable=SC2046
        target/release/examples/bba-decompose \
            $(for vul in $VULS; do printf '%s ' "$R/$a-$vul"; done) \
            --our-floor "$FLOOR" --dd-cache "$R/dd-cache.json" \
            --report "$out" --book-census >"$R/decompose.$a.txt" 2>>"$R/log"
    done
done
log "=== portable judgement 2 done"
