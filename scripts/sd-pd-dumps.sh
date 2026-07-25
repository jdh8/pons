#!/bin/sh
# sd-pd-dumps.sh — SD-PD re-adjudication over the *stored arm dumps*.
#
# The companion to sd-pd-readjudicate.sh, and methodologically the stronger
# half.  Those runs re-bid, so the shipped book has moved under them and the
# published figures cannot reprint.  These do not: `ab-dump-sd` replays auctions
# recorded weeks ago and only re-scores them, so nothing about today's book
# enters.  **The reproduction gate is live here** — the plain and PD rows must
# match the published numbers in each row's comment before the SD-PD row means
# anything.  A mismatch is dump/schema drift, not a verdict.
#
# Seed: deliberately NOT overridden — ab-lib.sh's sddiff passes no --sd-seed, so
# the published runs used ab-dump-sd's default 20240607 and so do these.  The
# world seed turns out to matter little (modern-negx NV measured +0.875/fired at
# 20240607 vs +0.828 at 1783925001, inside noise), but matching the original
# invocation keeps the gate honest.
#
# Disclosure matters: the blind leader must read the ON arm's auctions under the
# ON arm's system, or the sd numbers credit us for opponents misreading bids
# they would in fact alert.  Each row passes the same --on-ns-* flag its original
# driver used.
#
#   BINDIR=<path>/release/examples setsid nohup scripts/idle-run.sh \
#       scripts/sd-pd-dumps.sh ab-results/sd-pd-dumps \
#       >ab-results/sd-pd-dumps.log 2>&1 </dev/null &
#
# Resumable: a non-empty result file is skipped.
set -eu
R=${1:?usage: sd-pd-dumps.sh RESULTS_DIR}
BINDIR=${BINDIR:?set BINDIR to the release examples directory}
A=${A:-/home/jdh8/src/pons/ab-results}
WORLDS=${WORLDS:-16}
mkdir -p "$R"

# rescore TAG ON_DIR OFF_DIR VUL [flags...]
rescore() {
    tag=$1 on=$2 off=$3 vul=$4
    shift 4
    out="$R/$tag.txt"
    if [ -s "$out" ]; then
        echo "skip $tag (already done)"
        return 0
    fi
    if [ ! -d "$on" ] || [ ! -d "$off" ]; then
        echo "MISSING dumps for $tag ($on / $off) — skipped"
        return 0
    fi
    echo "=== $tag vul=$vul worlds=$WORLDS $(date -Is)"
    "$BINDIR/ab-dump-sd" "$on" "$off" -v "$vul" \
        --sd-worlds "$WORLDS" --show 0 "$@" >"$out.tmp"
    mv "$out.tmp" "$out"
    cat "$out"
}

for vul in none both; do
    # Modern negative doubles (competitive-book P3b‴, SHIPPED default-on).
    # Published: plain +0.0213 NV / +0.0074 vul (CI>0 both), PD -0.0044 / -0.0256
    # (vul CI<0), plain-SD +0.4221 / +0.2881 per divergent board.  The vul-PD
    # loss was written off as a doubling artifact *on the strength of plain SD* —
    # exactly the reading SD-PD exists to check.
    rescore "modern-negx.$vul" \
        "$A/free-bid-answers/free-bid-answers-modern-$vul" \
        "$A/free-bid-answers/free-bid-answers-off-$vul" \
        "$vul" --on-ns-negative-double-shape modern

    # set_floor_rkcb (a7, SHIPPED default-on).  Published: plain +1.01/+1.03 per
    # fired, PD +0.84/+0.77, plain-SD +2.36/+2.93 — "the strongest bracket".
    rescore "floor-rkcb.$vul" \
        "$A/a7/floor-rkcb-on-$vul" "$A/a7/floor-rkcb-off-$vul" "$vul"

    # set_nt_overcall_systems_on (SHIPPED default-on), split by opening kind —
    # the major cells are the plain-wash/PD-negative ones.
    for kind in minor major; do
        rescore "nt-overcall-systems-on.$vul.$kind" \
            "$A/nt-overcall-systems-on/on-$vul-$kind" \
            "$A/nt-overcall-systems-on/off-$vul-$kind" "$vul"
    done

    # FreeBidStyle::Negative (competitive-book P3e, stays OPT-IN) — the inverse
    # case: plain wash, PD loss (-0.0113/-0.0127), and plain SD *won* both vuls
    # (+0.0033/+0.0043, CI>0) yet could not ship it.  If SD-PD keeps that win the
    # opt-in deserves another look; if it erases it, plain SD was flattering an
    # aggressive style and the rejection was right for a better reason.
    rescore "free-bid-negative.$vul" \
        "$A/free-bid-style/free-bid-style-negative-$vul" \
        "$A/free-bid-style/free-bid-style-forcing-$vul" \
        "$vul" --on-ns-free-bid-style negative

    # Batch 2 — the inverse suspects, where plain SD *vetoed* a DD win.
    # set_two_level_minor_overcall_tight (floor 11→15, stays OPT-IN).  Published:
    # plain +0.0015 NV / +0.0061 vul, PD +0.0075 / +0.0131 — but plain SD washed
    # both (−0.0021 ±0.0031 / +0.0025 ±0.0040) and the wash killed it, "for a
    # competitive range sd is the arbiter".  Plain SD is the arm-friendly scorer
    # for the *looser* arm here, so restoring the doubling should move this cell
    # if the wash was the missing punishment on the 11–14 overcalls.
    rescore "two-level-minor-overcall.$vul" \
        "$A/two-level-minor-overcall/on-$vul" \
        "$A/two-level-minor-overcall/off-$vul" \
        "$vul" --on-ns-two-level-minor-overcall-tight
done

echo "=== sd-pd dump re-adjudication done $(date -Is)"
echo "GATE: check each run's plain/PD rows against the published figures in the"
echo "row comment above BEFORE reading its SD-PD row."
