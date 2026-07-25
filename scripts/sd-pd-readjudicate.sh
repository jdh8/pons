#!/bin/sh
# sd-pd-readjudicate.sh — re-adjudicate the verdicts that plain SD decided.
#
# Plain SD (`ns_score_tricks`) relaxes the defenders' opening lead *and* never
# doubles a failing contract — optimistic on both axes, so it flatters
# aggression.  SD-PD (`ns_score_pd_tricks`) reprices the same trick count with
# failures doubled.  Every harness below now prints both.
#
# READ IT AS A FRESH MEASUREMENT, NOT A RESCORE.  The original plan replayed
# each published seed expecting the old plain-SD figure to reprint.  It does
# not: the shipped book has moved since those runs (PointCount default 277059f,
# linearised upgrade b1d3a78, suit-indexed support points, the 2/1 Points13
# gate).  Measured 2026-07-25 on ab-fuzzy-strength at its own published seed:
# divergent 50610 (16.87%) against a published 41261 (13.75%).  Divergence is
# scoring-independent, so that shift is the book, not the scorer — the old
# numbers are a stale baseline and there is nothing to reproduce.
#
# That costs nothing, because the question is *internal to one run*: both SD
# brackets price the same trick counts from the same lead sample, so "does the
# plain-SD win survive the doubling?" is answered by the two adjacent lines of
# a single run's output.  The published figure was only ever a drift check.
#
# Level realism: doubling a failing contract is realistic at partscore and
# game, so SD-PD arbitrates below slam.  At slam it is a stress-test — nobody
# doubles a voluntarily bid six.
#
#   BINDIR=<path>/release/examples setsid nohup scripts/idle-run.sh \
#       scripts/sd-pd-readjudicate.sh ab-results/sd-pd \
#       >ab-results/sd-pd.log 2>&1 </dev/null &
#
# Resumable: a non-empty result file is skipped.  Arms strictly sequential —
# this live-solves, so one run already saturates the box.
set -eu
R=${1:?usage: sd-pd-readjudicate.sh RESULTS_DIR}
BINDIR=${BINDIR:?set BINDIR to the release examples directory}
WORLDS=${WORLDS:-16}
SDSEED=${SDSEED:-1783925001}
mkdir -p "$R"

# WHAT IS *NOT* HERE.  The queue only re-adjudicates knobs where SD-PD arbitrates
# a **bidding decision** — which call to make.  Evaluator-selection A/Bs ("which
# scalar strength gauge is best") are dropped rather than re-measured: the
# `Envelope` already carries several strength axes side by side (hcp,
# support_points, suit_hcp), so the useful move is feeding the net more features,
# not spending DDS-hours electing one fused scalar.  Dropped on that rule:
#   set_fuzzy_points     PointCount-vs-raw-HCP scale election.  Its default was
#                        re-justified since on a *different*, PD-positive
#                        measurement (277059f), so no SD-PD number can move it —
#                        the plain-SD row is stale evidence for a live default.
#   set_gauge_membership sampler-membership gauge, measured bidding-INERT
#                        (0 fired in 409600 boards); and ab-dnf-sd-lead cannot
#                        give an independent SD-PD read (identical contracts).
#   weak-two cccc bands  gauge-band election, same class.
#   RuleOfNFloored, set_new_point_count, Rule-of-20 openings — already superseded.
#
# tag | binary | boards | extra args.  Order is re-adjudication priority: each
# row is a default-on *convention* that plain SD shipped over a negative PD
# bracket.  Board counts match the published runs so the CI is comparable.
run() {
    tag=$1 bin=$2 count=$3
    shift 3
    for vul in none both; do
        out="$R/$tag.$vul.txt"
        if [ -s "$out" ]; then
            echo "skip $tag/$vul (already done)"
            continue
        fi
        echo "=== $tag vul=$vul count=$count worlds=$WORLDS $(date -Is)"
        "$BINDIR/$bin" --count "$count" --vulnerability "$vul" \
            --sd --sd-worlds "$WORLDS" --sd-seed "$SDSEED" "$@" >"$out.tmp"
        mv "$out.tmp" "$out"
        cat "$out"
    done
}

# set_meckstroth_adjunct  — 3m-jump leg: plain wash + PD loss, sd-only
run meckstroth-2nt ab-meckstroth-2nt        400000
# set_forcing_nt_two_suiter — plain wash + PD -0.0017/-0.0010, sd-only
run forcing-nt     ab-forcing-nt-two-suiter 1000000
# set_notrump_minors      — Puppet leg, sd-lead weakly positive
run notrump-minors ab-notrump-minors        400000

echo "=== sd-pd re-adjudication done $(date -Is)"
echo "Verdict per knob: compare the two sd-lead lines.  A plain-SD win that"
echo "collapses to a wash or a loss under SD-PD was the missing doubling."
