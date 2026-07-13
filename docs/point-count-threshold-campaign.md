# Point-count threshold campaign

## Why this exists

On 2026-07-14 the `pons` engine gained an opt-in `hcp_plus`-based
[`point_count`](../src/bidding/constraint.rs) scale (HCP + useful shortness + a
bare long-suit-length term, closer to BBO GIB), behind
[`set_new_point_count`](../src/bidding/constraint.rs) — **default off.** It is a
measured win under the realistic single-dummy-lead scorer:

| bracket | NV | Vul |
| --- | --- | --- |
| plain DD | +0.104 ± 0.029 | +0.058 ± 0.039 |
| perfect defense | −0.363 ± 0.037 | −0.443 ± 0.048 |
| **sd-lead (blind lead)** | **+0.279 ± 0.030** | **+0.293 ± 0.040** |

(50k boards/vul, `cargo run --release --example ab-point-count -- --sd`.) The
perfect-defense negative is a DD-pessimism artifact on part-scores — DD finds a
killing lead against every 1NT that a real defender never finds; the sd-lead
bracket is the honest read and says the scale is good.

It ships **opt-in, not default-on**, because it reads **~1–3 points higher on
every *shaped* hand**, while the authored `points(..)` gates are still
denominated in the old scale. Flipping it on today pushes shaped hands past the
invite / game / max thresholds those gates set — a scatter of over-aggressions
(below). This doc is the campaign that re-tunes the gates so the scale can flip
on. **The tests are deliberately left untouched:** refreshing them to the
un-fixed behavior would enshrine the bugs. When default-on is flipped, the bug
gates get fixed first, and only then do the handful of acceptable-aggression
test expectations get updated.

## The mechanism

- **Floors** (`points(lo..)`, `point_count(h) >= N`): fire earlier → more
  aggressive → the measured win. **Safe, no action.**
- **Caps / upper bounds** (`points(lo..=hi)`, `<= N`, invite/game band tops):
  the hand overflows the top → falls into a *gap* (→ Pass) or a
  semantically-wrong stronger bid. **Regression candidates.**

## Clear bugs (fix first)

Surfaced by running the scale default-on against the test suite (23 tests
shifted); the tree is back to opt-in-off, so they live here, not in refreshed
assertions. Each is a genuine mis-bid the aggregate SD win masks (all rare).

**They reduce to two root causes — this is the campaign's leverage:**

- **Root A — `hcp_plus` inflates every *shaped* hand.** Each doubleton/singleton
  adds to the count, so even a plain 6-3-2-2 reads +2 over its HCP — the
  inflation is universal to shape, not special to preempts. It is the intended
  win once a fit is known (shortness = a ruffing value), but **spurious before a
  fit is known and on hands that will never use the shortness**, where it pushes
  weak / sign-off / preemptive shaped hands past invite / game / max thresholds
  they should sit below. Six of the eleven bugs (5, 6, 7, 8, 9, 10) are this.
  `point_count` is context-free, so the clean fix is **gate-level, not a global
  evaluator hack**: the weak / preemptive / sign-off `points(..)` gates should
  gauge raw [`hcp`](../src/bidding/constraint.rs) — they were only ever right
  *by accident* on the old scale, where `points ≈ hcp` for weak flat hands.
  Gauging a **preempt in HCP** is sounder bridge in its own right: it keeps the
  preempt within legal/disciplined bounds and lets partner trust the HCP (e.g.
  bid 3NT on a misfit). (A context-free refinement — drop the shortness term
  when the hand holds a 6+ suit, since a one-suiter does not ruff — would catch
  7, 8, 10 without touching those gates.) The **constructive** invite/game gates
  keep the shortness credit: that is the +0.28 win. So — the answer to "do we
  revisit *all* `points` gates?" — **no, only the weak/preemptive slice.**
- **Root B — legacy-denominated band ceilings and structural holes** that the
  higher scale overflows into a gap or a shape-hiding stronger bid.
  Re-denominate the ceiling or add the missing rung. Bugs 1, 2, 4, 8, 11.
- **Standalone** — bug 3's blast *bid-selection* is a latent bug the new scale
  merely *triggers* (a 6-spade hand blasting 4♦ Texas-to-hearts); fix on its own
  merits, independent of the scale.

### Root B — ceilings and holes

1. **Gladiator club INV/GF split — `inv = 8`, `game = 10`**
   ([defense.rs](../src/bidding/american/defense.rs), `gladiator_advances`). An
   8-HCP invitational 6-club hand (`43.72.K5.KQ9876`) reads 10 = `game`, so it
   force-to-games 3♣ instead of the 2♣ relay; forced through the relay it
   strands in 2♦ (Pass). The INV band collapses. **Fix:** re-denominate
   `inv`/`game` up (~9 / 11–12). Test: `gladiator_club_three_way`.
2. **Opener jump-rebid band top**
   ([rebids.rs](../src/bidding/american/rebids.rs), forcing-1NT jump rung). An
   18-count 6-1-3-3 (`AKQJ72.3.KQ5.J54`, was 17) overflows the jump band into a
   shape-hiding 2NT. **Fix:** raise the jump band's top / the 2NT floor. Test:
   `opener_major_jump_rebid_shows_strength`.
4. **Six-card opener-accept `≥ 18`**
   ([notrump.rs](../src/bidding/american/notrump.rs)). A flat-15 hand with a
   heart doubleton (`AK5.32.AQ74.Q963`) reads 16, so 16 + 2 = 18 accepts game
   instead of passing. **Fix:** raise the accept threshold. Test:
   `sixcard_major_invite`.
8. **Weak-two ↔ 1-opener seam** (Root A *and* B). A clean 9-HCP weak two
   (`53.KQJ732.K42.92`) reads 11 (two doubletons), landing between the weak-two
   ceiling (10) and the 1-opener floor (12) → **Pass**. **Fix:** the weak-two
   gate should read raw HCP (Root A); or widen the seam. Test:
   `test_more_openings`.
11. **`1♥–1♠` rebid table missing a GF jump-shift rung** (structural). An
    18-point 5-5 (`Q2.AK853.4.AQ976`) upgrades past the `points(15..=17)`
    invitational band into a hole and drops to a non-forcing 2♣ responder can
    pass — missing game. **Fix:** add a GF jump-shift rung (mirror
    `with_extras_ladder`). Test: `opener_jumps_to_invitational_three_clubs_over_one_spade`.

### Root A — shaped-hand inflation on weak/sign-off gates

5. **Preempt-4M ace-gate override** ([instinct.rs](../src/bidding/instinct.rs)).
   A KQ-headed 6-bagger with no trump ace (`432.KQJ987.65.32`) reads 8 (was 7),
   crossing the escape/jump point boundary → jumps 4M despite the trump-ace
   gate. Test: `preempt_4m_over_double_jumps_the_long_major`.
6. **Rubens advance floor via partner-suit shortness**
   ([instinct.rs](../src/bidding/instinct.rs)). A bare 8 with a singleton in
   *partner's* overcalled suit (`2.Q32.KQT54.J432`) reads 10 and reaches the
   10-point transfer floor. Test: `rubens_new_suit_transfer`.
7. **XYZ relay sign-off misfire**
   ([xyz.rs](../src/bidding/american/xyz.rs)). After `1♣-1♥-1NT-2♣-2♦`, a 6-HCP
   `x.Qxxx.KJxxxx.xx` reads 8 (singleton + 6-card suit) and raises its own
   *forced* 2♦ sign-off to 3♦ — same strain, higher level, no game ambition.
   Test: `xyz_relay_signs_off_in_diamonds`.
9. **Garbage Stayman → invite**
   ([notrump.rs](../src/bidding/american/notrump.rs)). `Qxxx.Jxxx.Kxxx.x` (6 HCP
   4-4-4-1) reads 8 (singleton) and invites 3♥ instead of the drop-dead Pass
   garbage Stayman is for. Test: `garbage_responder_passes_opener_answer`.
10. **Ogust min → max** ([weak_twos.rs](../src/bidding/american/weak_twos.rs)). A
    6-HCP minimum weak two (`94.QJ8632.K85.72`) reads 8 (worthless doubletons) →
    Ogust `points(8..=10)` max window → answers 3♥ (max) not 3♣ (min),
    collapsing the min/max ladder. Test: `test_ogust_answers_after_two_hearts`.

### Standalone

3. **Six-card major blast-selection bug** ([notrump.rs]). When
   `point_count + length` reaches the blast floor (14), the blast path
   *misfires*: a 6-spade hand blasts 4♦ (Texas *to hearts*), a 6-heart hand
   blasts 4♣. This is a real bid-selection bug the scale merely *triggers*; fix
   it independent of the scale. Test: `sixcard_major_invite`.

## Acceptable aggression (no gate fix — refresh the test expectation when default flips on)

Sound extra aggression the +0.28 rewards; these become test-expectation updates,
not gate fixes, at flip time.

- **Acceptance ladder to game**: a 5-card major / prime hand with a side ruffing
  value now *accepts* game where it invited/declined —
  `limit_raise_accepted_and_declined` (→4♠), `spade_raise_invite_accepted`
  (→4♠), `heart_rebid_preference_structure` (→4♥),
  `responder_invites_with_a_fit_and_eight` (→4♥).
- **Splinter → Jacoby 2NT at 13+**: a singleton + 5-card side suit reads a GF
  raise. `test_splinter_over_one_spade`.
- **Opener jump raise**: a 14-count with a singleton jumps 3♠.
  `test_opener_rebid_raises_spades`.
- **Forcing-1NT 5-5 GF upgrade**: a 5-5 reads 18+ and routes through the
  designed GF-2NT. `opener_jumps_to_invitational_three_clubs` / `_diamonds`
  (and `opener_jumps_to_show_five_five_majors`, whose on/off toggle then goes
  tautological — re-anchor it on a hand that still straddles the band).
- **Rule-of-20 opener**: a sound 11-HCP 5-4 opens as a 12-count (correct); the
  Rule-of-20-*only* demonstration wants a fresh marginal hand.
  `rule_of_20_opens_sound_eleven_counts`.
- **9-count 5-card-major game force**: reads 10, so the game force routes
  through the book not the floor; 3NT outcome unchanged (benign routing).
  `nine_count_five_card_major_forces_game_after_a_transfer`.
- **Rubens transferee 12 → 14**: a 12-count 6-card suit bids game not invite.
  `rubens_transferee_clarifies_with_extras`.

## The re-denomination roster (broader)

Every capped `points(lo..=hi)` range is denominated in old points (`hcp(..)` and
`fifths(..)` ranges are immune — different evaluators). Not all need work:

- **Ladder tiers — mostly safe.** Contiguous strength rungs
  (`points(..6)`/`6..=9`/`10..=12`/13+ in responses.rs, the `12..=15`/`16..=18`
  jump tiers in rebids.rs, raises.rs, the xyz/nmf bands, weak_twos). Overflow =
  the next rung up = the measured aggression. Audit, but expect no change.
- **Floors that re-add length — a small upward re-tune.** A `point_count`-based
  *floor* is normally safe (fires earlier = the win), but one that *also* adds a
  length term double-counts trump length under the new scale (which itself
  carries a long-suit-length term). Measured: the **fit-sum major-game gate**
  ([`FIT_SUM_GAME`](../src/bidding/instinct.rs), default 31 — `point_count +
  partner.min + own_len + partner_shown_len >= t`) has its PD peak move **31 →
  32** under the new scale. Sweep (`ab-fit-sum-game --new-point-count`, 200k×2vul,
  adjacent thresholds, PD is the arbiter for a game gate): raising 30→31 PD
  +0.027/+0.029, **31→32 +0.008/+0.005** (CI-clean both vuls), 32→33
  −0.004/−0.008 — unimodal peak at 32. The move is only **+1** (not the raw
  +1–3 hotness) precisely *because* the gate double-counts `own_len`, so most of
  the inflation self-cancels; the residual notch is the un-double-counted
  shortness term. **Marginal** (+0.005 vul, DD-negative), so this is a low-stakes
  flip-time bump, not a live default change — the default is shared across scales
  and 31 stays optimal under the current (old) default scale. **Action at flip:**
  set `FIT_SUM_GAME` to 32 when `set_new_point_count` goes default-on. Sibling
  `set_floor_slam_entry` (29) likely wants the same one-notch re-probe.
- **Isolated / weak / overcall ceilings — probe.** A ceiling with no stronger
  sibling, so overflow lands in a gap or a wrong bid:
  - **Unusual 2NT `(8, 13)`** (defense.rs `UNUSUAL_NT`): a strong 5-5 minors
    two-suiter has no call above 13 and falls to Pass (`3.3.AJ876.KQ876` reads
    15). Its sisters `meckwell_2c/2d` are already floor-only. Fix: open the top
    (`(8, 37)`) or re-probe via `ab-landy --ns-minors 8` vs `8:13`.
  - Gladiator `inv`/`game` (bug 1); natural/gladiator overcall bands
    (`points(10..=16)`, `points(11..=15)`); the weak-two ↔ 1-opener seam (bug 8).

## Method

- **Gate raw-HCP swap (Root A):** on a weak/preemptive/sign-off gate, replace
  `points(..)` with `hcp(..)`. Cheapest fix, retires most of the list.
- **Re-denominate a ceiling (Root B):** shift the literal up by the shape
  premium (~+1–2), or re-derive from the intended combined-strength target.
- **Re-probe a measured range:** its knob (`set_unusual_notrump_defense`, the
  gladiator toggles, `set_sixcard_invite_floor`, …) sweeps via its `ab-*` runner
  with `set_new_point_count(true)`, `--score plain` + `--score pd`, and `--sd`
  where part-scores dominate.
- Each gate change is a bidding change: measure it (`docs/measurement.md`)
  before it ships. Flip `set_new_point_count` default-on only once the Root-A/B
  gates are fixed and re-measured.
