#!/bin/sh
# ab-landy-major-jam.sh — §N1p-jam: the `4M` jam rung over their Landy `2♣`,
# measured **standalone** for the first time.
#
#   JOBS=24 PER_SHARD=192000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-major-jam.sh ab-results/landy-major-jam \
#       >ab-results/landy-major-jam.log 2>&1 < /dev/null & disown
#
# Why this run exists.  §N1p built `landy_major_jam` conjoined with
# `landy_notrump_no_major` and could therefore only ever price the pair
# `jam vs nt`.  That pair won all four scorers (+4.295 plain, +1.669 PD,
# +5.541 sd-plain, +3.591 SD-PD IMPs/fired on 1,567 boards) — but it rode an
# arm that lost, and, worse, it measured the wrong substitution:
#
#   with `nt` on   `3NT`@168 denies four-plus of a major and six of one *is*
#                  four-plus, so the six-card major game hand had fallen to the
#                  `X`@145.  `4M` therefore replaced **the double**.
#   standalone     `3NT`@168 is ungated, and `4♠`@172 / `4♥`@171 outrank it, so
#                  `4M` replaces **the game**.
#
# Different experiment; the +5.541 does not transfer.  The conjunct was dropped
# 2026-08-30 and the drop is behaviour-preserving with both knobs on, so §N1p's
# numbers stand.  (The knob doc had claimed the rung "never fires" alone because
# `3NT`@168 swallows the hands.  False — 172 and 171 both outrank 168; it was
# the conjunct that suppressed it, not the ladder.)
#
#   base   today's `main`
#   jam    `competition.landy_major_jam` alone — `4♠`@172 / `4♥`@171 on
#          `len(major, 6..) & points(10..)`, above the **ungated** `3NT`@168 and
#          below the transfers, with `multi_signoff_pass` at `4♠ -` / `4♥ -`.
#          Weak six-carders keep defending through the `X`@145.
#
# The bridge case.  Their `2♣` shows both majors, so our own six-card major sits
# opposite their known length: the outstanding cards break badly and trump
# control matters more than the ninth trick.  Opposite a 15-17 balanced that is
# a 6-2 or 6-3 fit with 25+ combined — the classic "suit game or notrump" fork,
# except that the auction has already told us the suit is breaking 4-1 or worse
# for the notrump.  And `4M` jams a live auction: §N1p measured the candidate
# handing the opponents more room on 72-75% of divergent boards when it
# *doubled* instead of declaring; the jam does the opposite.
#
# Falsifiers, in order.
#   1. **Obstruction is invisible to DD** (the iron rule).  The jam takes the
#      whole four-level away from an opponent pair that has shown both majors,
#      and double-dummy cannot price the guess it forces.  A negative plain-DD
#      row is partly the harness; read the sd-lead pair before the verdict.
#   2. **`4M` may simply be an overbid.**  Read the made/down split of the `4M`
#      contracts before the IMP mean.  `points(10..)` opposite 15-17 is 25+, but
#      the rung has no quality gate at all — a ratty six-bagger with soft
#      values outside is exactly the hand `3NT` was right on.  If the split is
#      the story, the repair is a texture gate, not deletion.
#   3. **The sit forgoes slam.**  `multi_signoff_pass` passes `4M`
#      unconditionally, so the fifteen-plus / six-card-major slice never
#      investigates.  It carried §N1p's winning pair unrelaxed, so it stays for
#      this run; it is the first thing to delete if the arm reads mixed.
#   4. **`--filter-landy` admits only strictly balanced 1NT openers**
#      (§N1's flagged item 5), so the wide-shape slice is invisible to both
#      arms.  It bounds the claim, it does not invalidate it.
#   5. **The slice is thin** — ~0.03% of boards fired in the `jam vs nt`
#      pairing.  A per-fired headline is not a ship gate on its own; read the
#      per-board delta and its CI off docs/measurement.md's decision table.
#
# Scoring: plain AND perfect defense, sd-lead tie-breaks.
# `probe-divergence --gate-opener ours` must read 0 foreign BEFORE any headline.
# Resumable; SEED_BASE persists in $R/landy-major-jam.seed.  Iron rule: do NOT
# edit `src/` while this runs.
R=${1:?usage: ab-landy-major-jam.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-major-jam)
log "=== landy-major-jam SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in none both; do
    arm base "$v" --filter-landy
    arm jam  "$v" --filter-landy --ns-landy-major-jam

    gatepair jam base "$v"
    diffpair jam base "$v"
    sddiff   jam base "$v"
done

log "landy-major-jam done"
