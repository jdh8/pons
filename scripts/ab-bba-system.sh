#!/bin/sh
# ab-bba-system.sh — Gate 2 of docs/bba-system.md: BBA's walked book as an
# admissibility mask over our own floor, against the shipped system, on paired
# deals.  Three arms, every one against BBA told the arm's own card:
#
#   american     the shipped 2/1 books over the v6 floor (the control)
#   bba          BBA's walked bands at every walked node, the v6 shell choosing
#                among the admitted calls (design D1, arm A)
#   bba-ladder   the same bands, the floor partition's own owner choosing —
#                the instinct ladder at constructive nodes (D1, arm B)
#
# `bba` − `american` on the same deals is the AGREEMENTS term under one
# judgement; each arm's own headline against BBA (bba-decompose below) is the
# judgement term at equal rules.  The masks read no knob, so an arm differs
# from the control by its book alone.
#
# Needs BBA_BOOK=<probe-bba-book run dir walked with --card none> (the arm's
# bba-gen processes and bba-decompose both read it); on this box also
# BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen (CLAUDE.local.md).
#
#   BBA_BOOK=ab-results/bba-book/<run> JOBS=96 BOARDS=204800 DDSS_THREADS=96 \
#       setsid nohup scripts/idle-run.sh scripts/ab-bba-system.sh ab-results/bba-system \
#       >ab-results/bba-system.log 2>&1 &
#
# Two follow-up cells on the same deals (§9.3): `bba-noceil` is `bba` under
# BBA_BOOK_NO_CEILING_FLOOR=1 (trap 11, the bilans' ceiling glosses dropped),
# and the paired diffs against `american` also write per-board records for
# scripts/bba-node-join.py, the §4 per-node join, rendered as
# $R/join.<arm>.vs.american.md.
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir → a new seed).
R=${1:?usage: ab-bba-system.sh RESULTS_DIR}
: "${BBA_BOOK:?set BBA_BOOK to the walked book the bba arms play}"
export BBA_BOOK
BUILD_EXTRA='--example bba-decompose'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)
echo "$BBA_BOOK" >"$R/bba-book"

log "=== bba system start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul, BBA_BOOK=$BBA_BOOK"
for vul in $VULS; do
    arm american   "$vul" --our-floor american
    arm bba        "$vul" --our-floor bba
    arm bba-ladder "$vul" --our-floor bba-ladder
    (export BBA_BOOK_NO_CEILING_FLOOR=1; arm bba-noceil "$vul" --our-floor bba)
    (DIFF_JSONL=1 diffpair bba        american "$vul")
    (DIFF_JSONL=1 diffpair bba-noceil american "$vul")
    diffpair bba-ladder american   "$vul"
    diffpair bba        bba-ladder "$vul"
    diffpair bba-noceil bba        "$vul"
done

# Each arm against BBA: the headline, the bucket ranking and the Gate 0 census
# on these deals.  One DD cache for the three, since the deals are shared.
for a in american bba bba-ladder bba-noceil; do
    out="$R/decompose.$a.md"
    [ -s "$out" ] && { log "skip $out (exists)"; continue; }
    log "decompose $a vs BBA (both vuls, book census)"
    # The replay must rebuild the arm's own mask, so the knob goes with it
    case $a in
        bba-noceil) floor=bba; noceil=1 ;;
        *) floor=$a; noceil= ;;
    esac
    # shellcheck disable=SC2086
    env ${noceil:+BBA_BOOK_NO_CEILING_FLOOR=1} target/release/examples/bba-decompose \
        $(for vul in $VULS; do printf '%s ' "$R/$a-$vul"; done) \
        --our-floor "$floor" --dd-cache "$R/dd-cache.json" --report "$out" --book-census \
        >"$R/decompose.$a.txt" 2>>"$R/log"
done

# §4: bba − american joined per first-divergence node
for a in bba bba-noceil; do
    out="$R/join.$a.vs.american.md"
    [ -s "$out" ] && { log "skip $out (exists)"; continue; }
    log "per-node join $a vs american"
    # shellcheck disable=SC2046
    python3 scripts/bba-node-join.py --book "$BBA_BOOK" \
        $(for vul in $VULS; do printf '%s ' "$R/diff.$a.vs.american.$vul.jsonl"; done) >"$out"
done
log "=== bba system done"
