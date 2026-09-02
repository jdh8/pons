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
#   ./target/release/examples/probe-layer-replay $R/lia-both \
#       --jsonl $R/imps-both.jsonl --out $R/layers-both.jsonl --ns-landy-lia
#   python3 scripts/divergence-buckets.py $R/imps-both.jsonl
#   python3 scripts/divergence-layers.py  $R/imps-both.jsonl $R/layers-both.jsonl [veto]
#
# ============================ VERDICT ============================
#
# **MEASURED A LOSS 2026-09-02.  `defense_2c_landy_lia` stays default off;
# the lane is PARKED behind a general floor rail (see the end).**
# SEED_BASE=1788290089, control sha deeb0252 (`main` HEAD), 4,608,000
# boards/arm/vul, both isolation gates **0 foreign / PASSED**.
#
#   vul | fired          | plain DD           | PD                 | sd plain | sd-PD
#   ----+----------------+--------------------+--------------------+----------+---------
#   NV  | 197,920 (4.30%)| -0.0056 +-0.0011   | -0.0331 +-0.0013   | +0.0058  | -0.0158
#       |                | (-25,685; -0.130/f)| (-152,539; -0.771/f)|          |
#   BV  | 163,430 (3.55%)| -0.0254 +-0.0012   | -0.0569 +-0.0014   | -0.0090  | -0.0342
#       |                | (-117,094; -0.716/f)| (-262,399; -1.606/f)|         |
#
# Plain DD is the pre-registered arbiter at both colours and both cells are
# negative; the one positive column (NV plain sd) is the column measurement.md
# says cannot overrule a PD loss, and sd-PD is negative at both colours too.
# Against lia2 (-0.0077 / -0.0059): NV a shade better, BV four times worse,
# and the arm fires on a third more boards.  Everything below is read off
# `imps-{none,both}.jsonl` (kept), the per-call provenance replay
# `layers-{none,both}.jsonl` / `layers-base-{none,both}.jsonl`
# (`examples/probe-layer-replay`, new for this forensic: it re-bids every
# divergent board's candidate auction and stamps each of our calls book or
# floor; 0 of 361,350 boards failed to reproduce) and the tables
# `bucket-*.txt`, `layers-*.txt`, `veto-*.txt`, `veto-*-extra.txt`.  Every
# number here was recomputed independently by a twelve-claim adversarial
# pass (2026-09-02); the corrections it forced are folded in.
#
# WHERE THE LOSS IS (plain IMPs, NV / BV; the lia2 leg split's definition):
#
#   leg      | n (NV / BV)     | plain NV / BV        | lia2 (NV / BV)
#   ---------+-----------------+----------------------+-------------------
#   diamond  | 55,383 / 47,593 | -86,528 / -91,463    | -92,508 / -83,517
#   club     | 57,838 / 48,667 | +27,990 / +14,627    | +58,256 / +63,018
#   rest     | 84,699 / 67,170 | +32,853 / -40,258    |  -1,424 /  -6,855
#
# 1. **The diamond leg is unchanged by the re-cut** (-0.0188 / -0.0198 per
#    board vs -0.0201 / -0.0181), and every one of its cells is negative
#    against the baseline's wide `3♣` transfer: `3♣ -> -` (the six thin
#    diamonds that now pass) -35,563 / -34,235 on 13,552 / 12,757 boards,
#    `3♣ -> 2NT` (the INV+ transfer) -26,135 / -27,514, `3♣ -> 2♦` -19,540 /
#    -16,145, `3♣ -> 3♦` -19,274 / -17,213.  Finding 1's pre-registered
#    residue is answered: no rung set for the diamond hand beats the wide
#    transfer, so on that leg the INV+ gate is what has to go.
# 2. **The club leg's win halved** (-54% NV / -78% BV).  `2NT -> 3♣` still
#    carries it (+40,933 / +39,039) but the INV+ rung flipped: `2NT -> 2♠`
#    -6,848 / -11,862 (was +14,368 / +14,194), via the Max-break+ answer
#    (`2♠ - 2NT` prefix -8,890 / -11,951, finding 4's replacement loses) and
#    the contested seats the refinement handed to the floor (`2♠ (3♠)`
#    -3,848 at -4.8 per fired NV; its `4♠` class -9.9).
# 3. **The weak band is the both-vul catastrophe.**  `- -> 2♥` (baseline
#    passed, lia took out on the 4-7 band) is -2,662 NV but **-42,572 BV**
#    on 28,817 boards (-107,283 PD), and the route is the flagged accept:
#    `2♥ - 3NT` -8,317 / -36,378, split by responder's HCP (a proxy — the
#    band is in points) into the 0-7 band at **-14,515 / -38,483** (10,284 /
#    8,841 boards) and the 8-9 band at +6,198 / +2,105.  Opener's `3NT`@160
#    opposite four points is doubled and sat (`2♥ - 3NT - - X` then our
#    authored pass: -17,679 / -26,917).
#
# THE FLOOR — ASSOCIATION, THEN THE FACT THAT HOLDS.  Boards on which the
# learned floor made at least one of our calls at or after the divergence:
# NV 124,936 boards, -72,224 plain (-118,074 PD); BV 94,870, -132,357
# (-181,836); boards the book bid to the end +46,539 / +15,263 plain.  That
# is an association, not an attribution: the BASE arm's floor is involved on
# MORE of the same boards (NV 141,958 vs 124,936; BV 107,264 vs 94,870), the
# divergent call is the book's on 177,112 / 148,603 boards, and half of the
# lia floor-involved plain loss sits on boards where the floor only ever
# PASSED (NV -38,515 of -72,224, BV -37,199; PD-positive there).  The fact
# that holds: lia DOUBLES the boards on which the floor makes a substantive
# (non-pass) call — NV 86,491 vs the baseline's 42,994, BV 62,135 vs 23,066
# — and the cell where lia's floor bids while the baseline's book or floor
# only passed is NV 67,239 boards / -29,948 plain / -165,161 PD, BV 53,103 /
# -97,643 / -213,559.  Every bucket in this block is over divergent boards
# only; identical boards are absent by construction.  The floored classes
# worth naming:
#   * the sits over their double of the `3NT` accept (above);
#   * **phantom suits** — floored suit bids on <=4 cards with partner's
#     announced minimum making <=5 combined: `2♠ (3♠) 4♠` on a doubleton
#     (-10 per fired), `3♦ - - (3♠) 4♦ - 4♠` (-10), `2♥ - 3♣ - - (X) - - 3♦
#     - 3♥` (-5 to -8).  probe-decision shows the net reading partner
#     correctly (♣6+, 8+) and bidding 4♠ anyway, logit 9.41 over 4♣'s 9.34.
#     The mechanism is the input, not the reading: the shipped floor's
#     vector (`features_v6`, 176 values) carries the four announced
#     envelopes, a we-bid-this-strain bit per strain and partner's last bid
#     — raw call identity (`context.rs:616` sets the bit for every bid,
#     artificial or not) — and no alert or tag column (the +0.004 NLL they
#     measured was not worth the coupling).  Over `2♠ (3♠)` it is told "we
#     bid spades, partner's last bid was 2♠, partner has ♣6+", a joint the
#     BBA corpus never showed it — and the compact card has no lia slot
#     (`defense_2c_landy_lia` is never read in features.rs), so the regime
#     vector is N1j's, under weights from 2026-08-18 (`9fb333f5`).  Alerting
#     more changes nothing the net sees; masking the strain bit for alerted
#     calls would shift its inputs off the training distribution the other
#     way and needs a retrain;
#   * **six-card pushes** — floored `3♦`/`4♦` on the weak rung's own suit
#     over their raise (`3♦ - - 3♠ [4♦]` -9.3 per fired; the balancing `- 2♥
#     - - [3♦]` -3.9), the level-judgment class, not phantom.
#
# FALSIFIERS (numbered as above):
#   1. **Does not fire on its own terms.**  The club leg's win did shrink,
#      but it went to `2NT -> 2♠` and `2NT -> 2♥`, not to `X -> 2♥`, which
#      is +18,457 / +6,138 plain (z +27 / +8; PD -3,064 / -14,130, the
#      artifact shape).  Caveat the pre-registration missed: that cell is
#      100% short-major 8-9 hands — the 2-2-major hands the flip gave to `X`
#      now match the baseline's `X` and never diverge, so the flip itself is
#      unmeasured (`2♥ -> X`: 0 boards).  The weights stand by default.
#   2. **FIRES.**  The 0-7 band under the accept is the takeout's whole loss
#      (item 3).  The recorded repair is the accept below the minor picks.
#   3. **Not supported on plain DD; PD-negative.**  Exactly-five 8-9 diamond
#      hands: NV `X` 4,082 boards +6,007 plain / -2,795 PD, `2♦` 1,851
#      +2,414 / +1,003, pass 365 +327; BV `X` +1,470 / -8,541, `2♦` +402
#      (noise), pass +453; the class is -1,358 / -9,329 PD.  And the `X`
#      bucket is not the ladder's `X` at all — the baseline doubled on every
#      one of those boards, and 3,375 / 3,248 of them diverge only at
#      responder's later floored `3♦` over `X (2M) - -` (+6,512 plain of the
#      +6,007): the floor's pull, unmeasured by design.
#   4. **FIRES.**  The weak rungs' contested tails: `3♦ - - 3♠` 605 boards
#      -5,543 (-9.2 per fired), `3♦ - - 3♥` 759 / -4,130 (BV 139 / -1,355,
#      154 / -894).  Over `3♠` the floor pushes `4♦` (550 / 605, always on
#      6+) and, when RHO passes it, always phantom-cues `4♠` (427 boards, own
#      <=4 on 415); over `3♥` it passes as often as it pushes (396 vs 363)
#      and the passes cost nearly as much (-1,679 vs -2,451).  lia2's
#      -22,119 `3♦ -> -` sell-out is gone by REMOVAL, not reversal: its node
#      `2♦ (2M)` keeps 2 / 6 boards, and the residual cell (1,126 / 271
#      boards, +244 / +474) sits at `X (2M)`, floored in both arms — lia3
#      measures nothing about the floor at the old node.
#   5. **Partly.**  `2♥ (2♠) -` -4,401 / -3,631 (-1.1 / -1.5 per fired, the
#      floor sits); `2♥ (2♠) X` +1,877 / -4,384; `2♥ (3♥) X` -875 / -7,358
#      plain but -13,846 / -19,418 PD.  Nothing here is the takeout's problem
#      — the band is.
#   WATCH `2♥ (4♥/4♠)`: 344 / 302 boards, +45 / -18 plain.  Nothing.
#
# PREDICTIONS: diamond leg non-negative — **no**; club leg positive — yes,
# halved; `3♦ -> -` flips — removed, see 4; declarer-seat flips recover —
# **no** (same contract, doubled status included, NS both arms, other seat:
# -6,728 on 28,345 NV / -9,091 on 27,378 BV against lia2's -6,910 on 26,496 /
# -10,323 on 27,816 by the same definition — flat at -0.0015 / -0.0020 per
# board).
#
# THREE BOOK MECHANISMS THE VERIFICATION PASS NAMED (the doc's next-arm
# list is built on them, not on the leg totals):
#   a. **The diamond leg's biggest cell is a hand with no rung.**  `3♣ -> -`
#      is 13,534 / 13,552 NV and 12,751 / 12,757 BV boards of responder
#      **0-4 HCP with exactly six diamonds**: `2♦`@140 needs five HCP
#      (`floors`), `3♦`@142 seven diamonds or two top honours, `2NT`@173
#      eight points — so `Pass`@0.  -35,552 / -34,223 plain, a PD wash.  In
#      lia2 the same class bid `3♦` at -1.97 / -1.77 per fired (-26,610 /
#      -22,615): the re-gate cost -8,942 / -11,608 MORE.  The baseline's wide
#      transfer takes this hand at `points(2..)`; that, not the INV+ gate as
#      such, is what "the wide transfer wins" means.
#   b. **Both INV+ rungs lose through `landy_lia_super_rebid`'s retreat@0.**
#      Opener's super-accept relay@161 outranks its own `3NT`@160, so it may
#      hold the stoppers; responder's `3NT`@120 then needs both major
#      stoppers and `4m`@130 needs 13+, and the 10-12 HCP hand retreats to
#      three of the minor: diamonds NV 2,050 boards / -9,737, BV 1,434 /
#      -10,491; clubs NV 2,084 / -9,486, BV 1,591 / -10,822 (~ -19.2k /
#      -21.3k plain) — `3NT` (base) -> `3m` (lia) on 1,751 / 1,251 boards at
#      -5.5 / -8.3 per fired.  This is the club leg's flip, and the
#      "Max-break+ answer loses" line above is this table, not the accept.
#   c. **A floored-both cell the reading moved.**  `- -> X` (NV 8,422 boards
#      / -11,801 plain / -32,459 PD; BV 6,200 / -12,162 / -31,976, the
#      third-largest PD cell) is 6,558 / 4,977 boards at `X (2♠) - -` where
#      BOTH arms are floored (lia doubles again, base passes): the narrowed
#      `X` reading moves the floor's reopening.
#   Colour: the diamond leg is colour-neutral (-86.5k / -91.5k); the 4-7
#   takeout band is the whole colour effect — the `2♥` rung's 0-7 HCP boards
#   are -12,856 NV against **-58,640 BV**, half the BV loss, and at NV the
#   `2♥ - 3NT` loss is entirely the doubled `3NT`s (3NTx 2,398 / -17,205;
#   undoubled 6,724 / -873) while BV also loses undoubled (5,918 / -11,810).
#   A vulnerability gate on the weak band is the obvious untested arm.
#   SD: the NV sd-lead plain +0.0058 +-0.0011 is a CI-clear win the size of
#   the DD loss (DD - SD = -0.0114 NV / -0.0164 BV, the opposite sign to D's
#   +0.014), so the NV verdict sits inside the lead-model seam; BV loses on
#   all four columns.  And the sd pass ran with no `--on-ns-landy-lia`
#   disclosure — `ab-dump-sd` has none — the caveat the doc already attaches
#   to package D.
#
# THE RAIL EVIDENCE — why the lane parks behind a general fix.  Cutting the
# floored suit bids by the two inputs an envelope-gated veto would read — the
# bidder's own length and partner's announced minimum in the suit, no bid-
# identity term at all — the class "floored suit bid, own <=4, own +
# partner-min <=5" on boards at or after the divergence:
#
#   * lia arm: NV 12,513 boards, net **-30,369 plain** (gross -54,120 lost /
#     +23,751 won; PD -65,663 net), BV 7,177, net **-33,277** (gross -46,975
#     / +13,698; PD -55,043).  Per call by level (a board counts once per
#     call): four-level -27,642 / -26,536, three-level -5,241 / -11,253,
#     five-plus -1,529 / -1,718, two-level ~0 — per-call, so a board with
#     two vetoable calls counts twice; summed over levels this overstates the
#     per-board net by ~13% (four-level deduplicated: 10,686 / -26,687 NV,
#     5,667 / -24,843 BV).
#   * **the BASE arm on the same boards**: 6,800 / 3,844 boards carry one and
#     the baseline lost on them — +35,346 / +25,145 plain net from the
#     candidate's side (gross +45,554 / +32,010 won by the candidate against
#     -10,208 / -6,865), **5.2 / 6.5 IMPs per fired**.  The default system's
#     own floor phantom-bids the same way; lia only put more seats in front
#     of it.
#   These are the pools a veto would act on, not bounds and not measurements:
#     the masked call is replaced, not undone; the veto also fires on the
#     ~4.4M non-divergent boards absent from these files; and a floor change
#     moves both arms, which is why its A/B is on the default system.  The
#     six-card push class is outside it.
#
# DISPOSITION (jdh8, 2026-09-02): try the general fix first — the
# envelope-gated new-suit veto on the floor (the residue named in
# docs/archive/one-notrump-competitive-closed.md since 2026-08-14), measured
# on the default system with its own non-inferiority A/B.  No local nodes for
# lia until that has run; if the rail flips the phantom classes here, the
# lane re-measures under it (lia4) with the book findings above as its only
# book changes: the accept below the picks (falsifier 2), the 0-4 HCP
# six-diamond hand back on the wide transfer (a), a values `3NT` in the
# super-accept rebid (b), and a vulnerability gate on the weak band (colour).  Recorded, not built: restoring `landy_lia_overcalled`'s
# `Pass`@0 over the INV+ rungs (lia2 had it, sha 32242d63, and that leg won).
# =================================================================
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
