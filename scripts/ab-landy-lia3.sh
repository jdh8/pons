#!/bin/sh
# ab-landy-lia3.sh — §N1-lia package B, REFINED on the lia2 forensic: keep the
# club leg, re-cut the diamond leg, delete the convicted sell-outs.
#
#   JOBS=24 BOARDS=4608000 setsid nohup scripts/idle-run.sh \
#       scripts/ab-landy-lia3.sh ab-results/landy-lia3 \
#       >ab-results/landy-lia3.log 2>&1 < /dev/null & disown
#
# Third measurement of `competition.defense_2c_landy_lia` (never shipped;
# redefined in place again, precedent 32242d63).  The lia2 loss and the
# five findings this arm is built on are the VERDICT block of
# `ab-landy-lia2.sh`; the full ladder spec is the campaign doc's "Refined on
# the lia2 forensic" section (docs/one-notrump-competitive.md §N1-lia).
# In one screen:
#
#   base  today's `main` — the shipped N1j BBA ladder
#   lia   club leg kept (2♠ INV+ clubs, 3♣ weak); diamond leg split three ways
#         (2♦ escape to 8 HCP / 3♦ excessive-diamond sign-off / 2NT = INV+
#         diamond transfer); X narrowed to 2+2+ majors above a two-band 2♥
#         minors takeout (8+, and a weak 5♣/4=♦ band at 4-7); Max-break+
#         answers the INV+ rungs (super-accept `comp:landy-super` / 3NT /
#         completion as opener's minimum default, recovering the lia2
#         right-siding loss); the weak rungs' contested seats and the two-band
#         takeout's contested seat are the FLOOR's (the −22,119 sell-out
#         conviction and its 2026-09-02 pre-launch sibling).
#
# READING GATE (pre-launch, 2026-09-02): PASSED.  `probe-call-reading
# --their-2c-landy --ns-landy-lia` reads every re-cut call soundly and
# tightly — `2♥` 4-9 pts ♣4+ ♦4-5 (the strong band's 8-9 cap is rung-order
# inference: 10+ is shape-forced into X/3NT/splinter first), `2NT` 8+ ♦6+
# ♣≤5, `X` 8-9 with the 2+2+ majors, the sign-offs their bands.  A first
# probe without `--their-2c-landy` read the systems-on rebase (constructive
# tables — a ♦2-2 "phantom" that is that lane's sound self-description) and
# burned an afternoon; the example warns on the inert combination now.  The
# A/B harness derives the disclosure from the opponent, so arm wiring is
# unaffected.
#
# SIZING.  4,608,000 boards/arm/vul (24 × 192,000), byte-for-byte the sizing
# of all three prior runs, so the four are power-comparable; MDE ~0.002
# IMPs/board.  The knob is OFF by default, so it needs a plain-DD **win**:
# both colours must clear zero, a wash leaves it off.
#
# PRE-REGISTERED ARBITRATION — inherited from lia2 verbatim: plain DD is
# primary at BOTH colours, PD is a reported column; the one PD loss not waved
# through is a mechanism that removes our penalty doubles, and this arm's X
# keeps them by construction (finding 5).  sd only if plain is worth reading.
#
# PREDICTIONS the arm stands or falls on (leg split per the lia2 forensic):
#   * diamond-leg cells flip non-negative (was −0.0201 NV / −0.0181 BV per bd);
#   * club-leg cells stay positive (was +0.0126 / +0.0137);
#   * the `3♦ → -` sell-out cell flips (was −22,119 plain NV);
#   * same-contract declarer-seat flips recover toward zero (was −0.0014 /
#     −0.0023 per bd — the completion is opener's minimum default now).
#
# FALSIFIERS, numbered as in the campaign doc's "Owed, and flagged":
#   1. **X-vs-takeout weight flip.**  If the club leg's win shrinks and the
#      `X → 2♥` cell is where it went, flip X@145 / takeout@144 back.
#   2. **Opener's 3NT@160 accept over the two-band 2♥.**  Bucket `2♥ → 3NT`
#      boards by responder's own strength; if the 4-7 band is where the
#      takeout loses, the accept moves below the minor picks.
#   3. **The diamond leg's remaining weak residue.**  Exactly-five 8-9 diamond
#      hands (no rung: X needs 2+2+, 2♦ caps at 8 HCP) must stay a tolerable
#      residue; a loss concentrated there is Lia's rung set, not this ladder.
#   4. **The weak rungs' contested handoff.**  The floor must actually collect
#      the −22,119 the sell-outs cost; if those cells stay negative with the
#      book gone, the floor needs the rail, not the book back.
#   5. **The contested takeout handoff.**  `2♥ ({raise})` cells: if the floor
#      sells out or blind-pushes there, the seat wants a lia table gated by
#      band-consistent values — not the old GF-doctrine ladder back.
#   WATCH (no pre-stated number): `2♥ (4♥/4♠)` — the floor's delayed double
#   was the worst per-fired class one rung over and this seat is unpriced.
#
# Scoring: plain AND perfect defense off docs/measurement.md's decision table.
# `probe-divergence --gate-opener ours` must read 0 foreign BEFORE any
# headline.  Resumable; SEED_BASE persists in $R/landy-lia3.seed.  Resume with
# the same two env vars — `JOBS=24 BOARDS=4608000`, the launch line verbatim.
# Iron rule: do NOT edit `src/` or run any cargo build while this runs.
#
# The bucket forensic is manual and post-hoc, over the kept arm dirs:
#
#   ./target/release/examples/probe-divergence \
#       $R/lia-both $R/base-both --imps --jsonl $R/imps-both.jsonl
#
R=${1:?usage: ab-landy-lia3.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-lia3)
log "=== landy-lia3 SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

# Plain-DD first, sd second — lia2's two-pass order: primary cells land in
# ~3 h, the lead-model column follows only if the headline earns it.
for v in none both; do
    arm base "$v" --filter-landy
    arm lia  "$v" --filter-landy --ns-landy-lia

    gatepair lia base "$v"
    diffpair lia base "$v"
done

for v in none both; do
    sddiff lia base "$v"
done

log "landy-lia3 done"
