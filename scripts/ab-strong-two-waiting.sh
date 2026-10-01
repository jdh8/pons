#!/bin/sh
# ab-strong-two-waiting.sh — `2♦` waiting replaces the `2♥` double negative
# over our strong `2♣` (`decision.strong_two_waiting`, docs/next-steps.md
# item 2, the strong `2♣` lane).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-strong-two-waiting.sh ab-results/strong-two-waiting \
#       >ab-results/strong-two-waiting.log 2>&1 < /dev/null &
#
# What is being tested.  Our `2♣ - 2♥` is the 0–3 HCP double negative and
# `2♦` (4+) forces game; there is no heart positive.  BBA (EPBot defaults,
# `probe-bba-book --card none`, 2026-10-01) waits with `2♦` on everything
# short of a positive, bids natural positives (`2♥` included), and passes
# opener's `2M` rebid on ≤5 without support.  The 2026-09-30 anchor prices
# our `2♥` bust against its `2♦` at 680 rows, −1.7k plain / −1.2k PD per
# 409,600 boards, and our `2♦` against its heart positive at 169 rows, −0.5k.
#
#   on   the default since 2026-10-01: `2♦` on 0+, `2♥` a positive like `2♠`;
#        over opener's `2M` a 0–3 hand passes without three-card support and
#        bids `4M` with it, over `3m` it passes; the floor's `2♣` game force
#        fires only on a positive
#   off  `--no-ns-strong-two-waiting`, the double negative
#
# SHIPPED default-on 2026-10-01 on rounds 3–4 (below), with the reading
# mirror pinned to the double negative (jdh8's call: the pin's ~80 ms per
# bind is accepted).
#
# Round 1 (ab-results/strong-two-waiting, seed 1790831619): a wash, plain
# +0.0005 / +0.0009, PD +0.0004 / −0.0001.  The worst boards were opener
# passing `2♦` (no catch-all once the floor's force lifted) and opener selling
# out over their interference; round 2 (`-r2`, seed 1790832249) added the
# catch-all and kept opener forced, and read plain −0.0004 / −0.0005, PD
# +0.0001 / −0.0007 — but 63 / 104 of its divergent boards were *their* 2♣:
# the reading mirror decoded BBA's `2♣ - 2♦` with our knob.  Both rounds are
# contaminated; `common::mirror_agreements` now resets the knob, and the
# isolation gate below fails the run if a board they opened moves.
#
# Round 3 (`-r3`, seed 1790832953, gate passed): plain +0.0003 ±0.0008 /
# +0.0007 ±0.0010, PD +0.0001 / +0.0006, 199 / 210 fired.  Round 4 (`-r4`,
# seed 1790833515, gate passed): plain +0.0011 ±0.0008 / +0.0018 ±0.0010, PD
# +0.0008 ±0.0008 / +0.0017 ±0.0011, 184 / 191 fired.  Pooled (409,600
# boards/vul): plain +0.0007 ±0.0006 / +0.0013 ±0.0007, PD +0.0004 ±0.0006 /
# +0.0011 ±0.0008 — every cell non-negative, plain DD a win.  Worst boards:
# opener's floor after a natural positive (`2♣ - 2♥ - 4♠ - 6NT`), unauthored.
#
# The mirror pin, priced (`-natural`, seed 1790834184, a BBA_GEN build
# without the `mirror_agreements` reset, no gate): decoding *their*
# `2♣ - 2♦` as waiting too reads plain −0.0005 / −0.0011, PD −0.0005 /
# −0.0014; pooled with round 2 plain −0.0005 / −0.0008, PD −0.0002 /
# −0.0011.  The truer reading of BBA's 2♣ costs our defence ~0.002 per
# board under the v6 floor; the pin costs ~80 ms per system bind (a mirror
# book on every build that turns the knob on).
#
# Hypothesis: a win on both scorers, ≈ +0.002–0.004 per board; the surface is
# ~0.5% of boards.  A loss points at the bust passing opener's `2M` (opener's
# game-in-hand hands have no jump rebid) before the response table.
R=${1:?usage: ab-strong-two-waiting.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for strong-two-waiting)

log "=== strong-two-waiting A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-strong-two-waiting
    arm on  "$vul"
    gatepair on off "$vul"
    diffpair on off "$vul"
done
log "=== strong-two-waiting A/B done"
