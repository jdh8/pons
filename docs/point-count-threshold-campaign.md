# Point-count threshold campaign

> **Superseded (2026-07-14).** The global `set_new_point_count` flip this campaign
> was written to re-tune is **deleted**. Its durable, fit-known fraction shipped
> instead as `support_points` — a named, fit-known-only evaluator (HCP + useful
> shortness) wired into the raise / fit-raise / floor-fit-sum gates, default on
> (plain DD +0.033/+0.053, PD +0.005/+0.020, sd-lead +0.052, all CIs clear 0).
> Scoping to fit-known dodged "Root A" by construction — no gate-by-gate re-tune
> needed — so this campaign is closed. `FIT_SUM_GAME` was re-probed under the
> shipped scale (2026-07-14) and **31 holds** (below). Root B (ceilings/holes)
> remains the one open follow-up under `support_points`.
>
> **Successor shipped (2026-07-14):** the fit-*unknown* gates moved too — the
> global `points` scale is now the **rule of N+8** (see
> [the deprecation A/B/C](#the-points-deprecation-abc-2026-07-14--rule-of-n8-shipped)
> at the bottom); legacy is the `set_point_scale(PointScale::PointCount)`
> opt-out.

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
  carries a long-suit-length term). Measured, and **31 holds** under the shipped
  `support_points`. The **fit-sum major-game gate**
  ([`FIT_SUM_GAME`](../src/bidding/instinct.rs), default 31 — `support_point_count
  + partner.min + own_len + partner_shown_len >= t`) was re-probed twice:
  - *Under the deleted global `set_new_point_count`* (broad, all gates hot): the
    PD peak moved 31 → 32, but only marginally (30→31 +0.027/+0.029, 31→32
    **+0.008/+0.005**, 32→33 −0.004/−0.008). This is the number the old breadcrumb
    was written from.
  - *Under the shipped fit-known-only `support_points`* (narrower — the actual
    routing): 32-vs-31 is NV PD **+0.004** CI [+0.001, +0.007] (barely ahead), Vul
    PD **−0.004** CI [−0.008, +0.000] (**parity/behind**), DD −0.016/−0.027. Not a
    clean both-vuls win → **the bump is refuted; 31 is the peak.**

  The move was only ever **+1** (not the raw +1–3 hotness) *because* the gate
  double-counts `own_len`, so most inflation self-cancels; the narrower shipped
  scale absorbs even that residual notch and the peak lands back at 31 — exactly
  the double-count argument. Sibling `set_floor_slam_entry` (29) is the same
  length-re-adding shape; if ever re-probed, expect the same "no bump" verdict.
  (`ab-fit-sum-game --support-points`, 200k×2vul, PD is the arbiter for a game
  gate; DD is monotone-worse as you raise a game gate and blind to doubling.)
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

## The `points` deprecation A/B/C (2026-07-14) — rule of N+8 shipped

The successor campaign: instead of re-tuning gates to a hotter scale, swap the
scale under all 446 `points()` gates at once and let the ranges keep their
authored meaning. Knob: `set_point_scale` (`PointScale::{PointCount, Hcp,
RuleOfN}`) inside the `point_count` scalar — gates, sampler acceptance, and
floor combined-counts move together, so the gates-vs-sampler confound cannot
arise. `RuleOfN` = raw HCP + two longest suit lengths − 8 (`points(12..)` ⟺
Rule of 20); bounds vs legacy: −1 (flat 4-3-3-3 only) to +4 (veto-blocked
extreme shapes).

**Stage 1 — 1M pre-solved boards/vul** (`/nfs2/jdh8/24.pdd` rows 0..1M NV,
1M..2M vul; stored 20-cell tables serve both scorers, zero live solving;
paired arms on the same slices):

| arm vs legacy | plain DD NV | plain DD Vul | PD NV | PD Vul |
| --- | --- | --- | --- | --- |
| B `Hcp` | **−0.0981 ± 0.0041** | **−0.1053 ± 0.0055** | +0.0414 | +0.0464 |
| C `RuleOfN` | **+0.0313 ± 0.0042** | **+0.0453 ± 0.0057** | −0.0377 | −0.0255 |

B is dead on arrival — a plain-DD loss whose PD positive is the doubling
artifact (re-confirming the A6 fuzzy-points verdict at 20× the sample). C is
the `support_points` signature (plain win + PD dip), so the sd-lead bracket
decides.

**Stage 2 — sd-lead tiebreak, 50k boards/vul live** (seed 1784042788): NV
**+0.0475 ± 0.0190**, vul **+0.0635 ± 0.0254** IMPs/board — both CIs clear of
zero. **Shipped default-on**; legacy is the opt-out
(`set_point_scale(PointScale::PointCount)`), and C's PD dip is ~10× shallower
than the deleted `hcp_plus` global flip's.

**Fallout triage (the Root-A/B taxonomy, applied):**

- *Bug gates fixed before test refreshes:* the strong 2♣ gained an `hcp(22..)`
  leg — a flat 22-count reads 21 points and `points(22..)` alone demotes a
  game force to a passable 1♣; unbalanced 22-HCP hands already read 22+, so
  the union is exact cover for the flat hole. The 1NT/2NT opening readings
  give their point floor back 1 via `flat_hcp_slack()` (shared with
  `Hcp::project`): an HCP-gated call's flat 4-3-3-3 minimum reads one under on
  the new scale.
- *Absorbed:* `set_rule_of_20(false)` no longer bites on the default scale —
  Rule of 20 *is* `points(12..)` there by identity; the knob still governs the
  legacy opt-out arm and the inference floor.
- *Sound aggression (test hands refreshed, gates untouched):* every remaining
  failure was a flat 4-3-3-3 boundary hand — 17-count Lebensohl max, 15-count
  transfer-invite accept, 14-count Rubens drive / NMF max, 10-count raise
  invite, 8-count reverse-raise, 6-count response — now correctly reading one
  lower and declining. That is the curse of 4-3-3-3 built into the scale.

**Consumed slices (never replay):** `24.pdd` rows 0..4,000,000 (stage 1 rows
0..2M, remnant report rows 2M..4M; both arms paired within each slice).

## The remnant report (2026-07-15) — where legacy `points` still wins

The shipped config (rule of N+8 + the 2♣/reading fixes) re-measured vs legacy
on fresh slices (`24.pdd` rows 2M..3M NV, 3M..4M vul, 1M boards/vul,
`--show 40`): plain DD **+0.0252 ± 0.0042 NV / +0.0334 ± 0.0057 vul** — the
ship verdict holds and the commit-3 fixes cost nothing. Remnant = a
first-divergence bucket where legacy wins with its per-bucket 95% CI clear of
zero (`⚠ remnant` in the runner output). The flagged buckets group into seven
families; they total ≈ −8k NV / −10k vul IMPs per 1M boards, so ~−0.01
IMPs/board of the scale's win is still on the table behind legacy-favoring
gates.

| family (flagged buckets, both directions) | ≈IMPs NV / vul per 1M | gate | prescription |
| --- | --- | --- | --- |
| **Weak-two band** — `[] 2♥→P`, `[] P→2♥`, `[] 2♠→P`, `[P] P→2♠`, … all seats | −2.0k / −3.1k | `len(suit, 6..=6) & points(5..=10)` ([openings.rs](../src/bidding/american/openings.rs)) | Root A: the band shifted down ~1–2 HCP both edges (a 6-card suit reads +1..+2, and legacy's wasted-honor veto did real work). Re-denominate on raw HCP: `hcp(5..=10)`-ish, sweep the edges. |
| **Quantitative 6NT** — `[2♣ P 2NT P 3NT P] 6NT↔P`, every rotation, **both directions lose** | −1.9k / −2.0k | no-fit NT slam `combined_points(33)`/`(37)` ([instinct.rs:2949-2960](../src/bidding/instinct.rs#L2949-L2960)) | A *notrump* slam has no ruffs — long-suit length is the wrong currency, and legacy wins both flip directions. Gauge raw HCP (+ partner floor) for the NT 6/7 gates; echoes the NT-invite-evaluator null (raw HCP wins at NT boundaries). |
| **2/1 response band** — `[1♠ P] 2♣↔1NT`, `2♦↔1NT`, `2♥↔1NT`, passed-hand variants | −1.5k / −2.1k | two-over-one `len(x, 4..) & points(13..)` vs residual 1NT `points(6..)` ([responses.rs:219](../src/bidding/american/responses.rs#L219)) | Both directions lose: flat 13s belong in the game force, shaped 11s belong in 1NT. The GF entry is shape-indifferent → `hcp` leg (union, like the 2♣ fix), sweep 12/13. |
| **One-level opening seam** — `[] P→1♣/1♦` (freaks), `[] 1♣/1♦→P` (flat 12s), all seats; NV-heavy | −2.3k / ~0 | `points(12..=21)` + Pass `points(..12)` ([openings.rs](../src/bidding/american/openings.rs)) | Two legs: flat 12-HCP now reads 11 and passes (add the `hcp(12..)` union leg, mirror of the 2♣ fix); sub-10-HCP freaks (11+ cards in two suits) now open where even the rule-of-20 light rules required `hcp(10..=11)` (add an `hcp(10..)` floor to the light seam). |
| **Competitive X ↔ bid seams** — `[1♦] X→1♠`, `[1♣ P 1♥] X→1♠`, `[P 1♠] 2♣→X`, neg-X families | −1.5k / −2.8k scattered | takeout/negative-double and free-bid bands in [competition.rs](../src/bidding/american/competition.rs) | Scattered small buckets, no one dominant gate; probe per docs/convention-tuning.md forensics before touching bands. |
| **2NT rebid-invite seam** — `[1♥ P 1♠ P 2♦ P] 2NT↔P` | — / −0.5k | responder's 2NT invite after two suits | NT-oriented invite → HCP gauge; probe. |
| **Weak-two ask answer** — `[2♦ P 2NT P] 3♣→3♥` | −0.2k / — | weak-two max/min answer band | Same Root A as the opening band; fix with it. |

Every prescription is expressible as an `hcp(..)` swap or an `hcp` union leg —
the `legacy_points(range)` pin combinator was never needed (YAGNI held). Each
fix is a bidding change: measure it per [docs/measurement.md](measurement.md)
on a **fresh** `.pdd` slice (cursor: `24.pdd` row 4,000,000) before it ships.
Note the harness subtlety: an `hcp` swap changes the *legacy arm's* behavior
too (legacy `points ⊇ hcp` on floors), so fix-vs-shipped is the honest A/B —
both arms on `RuleOfN`, differing only in the gate — not fix-vs-legacy.

## The 4333-floor A/B (2026-07-15) — the flat downgrade blocked, shipped

jdh8's follow-up idea: since the gates stayed on `points`, block the scale's
only downgrade — let `points` floor at raw HCP on flat 4-3-3-3. On the
rule-of-N+8 scale that is one moved parenthesis: `hcp + max(0, L2 − 8)`
instead of `max(0, hcp + L2 − 8)` (only 4-3-3-3 has L2 = 7 < 8), the new
`PointScale::RuleOfNFloored`. Measured **fix-vs-shipped** (both arms
otherwise rule of N+8, per the remnant-report rule; `ab-point-count
--candidate rule-floored --baseline rule`):

| stage | slice (`24.pdd`) | plain DD | PD | sd-lead |
| --- | --- | --- | --- | --- |
| 1M boards NV | rows 4M..5M | **+0.0129 ± 0.0020** | −0.0407 ± 0.0027 | — |
| 1M boards vul | rows 5M..6M | −0.0007 ± 0.0027 (wash) | −0.0595 ± 0.0034 | — |
| 50k sd NV | rows 6.00M..6.05M | +0.0112 ± 0.0088 | −0.0361 ± 0.0117 | **+0.0316 ± 0.0090** |
| 50k sd vul | rows 6.05M..6.10M | −0.0028 ± 0.0125 | −0.0587 ± 0.0155 | **+0.0258 ± 0.0125** |

Same signature as the scale's own ship (plain win/wash + PD dip, sd-lead
positive both vuls with CIs clear) → **shipped default-on**; plain
`RuleOfN` stays opt-in. Forensics (`--show 12`): vul, the worst buckets were
the *opening seam* — the floor opens flat 12-counts that plain rule-of-N+8
passes, and plain DD dislikes that vulnerable (−0.5..−0.7/board on those
boards) but sd-lead nets it positive; NV, the worst buckets were
competitive-X/redouble machinery and a Texas-then-4NT quantitative creep that
PD over-punishes. Note the vul opening-seam forensic *contradicts* the
remnant report's NV-heavy `hcp(12..)` opening prescription in direction —
that remnant fix, if pursued, should re-measure against **this** default
(the floor already restores flat 12-HCP openings).

Fallout: with the downgrade gone `flat_hcp_slack()` is 0 by default, so the
1NT/2NT readings return to exact 15–18/19–23 and the 2♣ `hcp(22..)` leg is
redundant-but-exact — both mechanisms stay for the plain-`RuleOfN` opt-in
arm. Test churn was exactly the four flat-reads-−1 encodings from the ship
commit, reverted to their pre-rule expectations.

### Remnant report re-run vs the floored default (2026-07-15)

Floored-vs-legacy on fresh slices (`24.pdd` rows 6.1M..7.1M NV, 7.1M..8.1M
vul, 1M boards/vul, `--show 40`): plain DD **+0.0377 ± 0.0039 NV /
+0.0347 ± 0.0052 vul** — coherent with pre-floor ship-vs-legacy plus the
floor's own fix-vs-shipped delta (+0.0252 + 0.0129 ≈ +0.0381;
+0.0334 − 0.0007 ≈ +0.0327). Flagged remnant totals shrank from ≈ −8k / −10k
to **−6.7k NV / −8.3k vul** per 1M boards. Family status changes:

- **One-level opening seam — CLEARED but for the freak leg.** The flat-12
  buckets (`[] 1♣/1♦ → P` + mirrors, −2.3k NV) vanished — the floor *is*
  that fix, the `hcp(12..)` union-leg prescription is confirmed moot. Only
  `[P P] P → 1♦` ×138 (−155 NV) survives: the sub-10-HCP freak leg
  (`hcp(10..)` floor on the light seam) is still open, now minor.
- **Quantitative 6NT — dropped out of the flagged set entirely** (zero
  buckets in either top-40; was −1.9k/−2.0k). The pass-direction losses were
  4333 responders reading −1 opposite `combined_points(33)`; the floor
  restored them. The raw-HCP-gauge prescription is downgraded from remnant
  to probe-if-bored.
- **Weak-two band, 2/1 response band, competitive-X seams, 2NT rebid-invite,
  weak-two ask answer — all stand** (no 4333 exposure for the weak-two
  families; both 2/1 directions still flagged). The redouble-then-game
  buckets (`[1M X XX P] game → P`, ×13–14 at −10..−17/board) now flag both
  vuls — the floor A/B's NV forensic made visible; part of the
  competitive-X family.

### 2/1 response band — FIXED (2026-07-15)

The prescription shipped, sharpened by jdh8's fit-split idea: the major 2/1
entry became `hcp(13..) | (support(3..) & support_points(13..))` —
shape-indifferent without a fit (`set_two_over_one_gate` = `Hcp13`; shaped
11-12s return to the forcing 1NT), `support_points` with exactly three-card
support (`set_two_over_one_fit`; the fit is privately known, opener promised
five, so the 2/1 is a priced preparation for `4M`) — plus the `1M – 3NT`
choice-of-games response carving out the flat (4333) 12-15s
(`set_major_choice_of_games`). Self-play `ab-major-continuations`, 1M
boards/vul/arm: the gate+fit pair plain **+0.0033/+0.0048** NV/vul, PD
**+0.0070/+0.0087**; the full package with the 3NT plain
**+0.0039/+0.0059**, PD **+0.0074/+0.0096** — all CIs clear, both scorers,
both vuls, ~4× the −1.5k/−2.1k the remnant report priced for this family.
The paired `hcp12`-vs-`hcp13` head-to-head kept 13 (hcp12's vul plain edge
+0.0026 came with PD −0.0020/−0.0034 — the thin-game doubling signature; an
sd-lead probe could still revisit vul-only). Details: CHANGELOG 2026-07-15,
`scripts/two-over-one-ab.sh`.

### Weak-two band — REJECTED, the wall (2026-07-15)

The prescription (Root A, `hcp(5..=10)` for the six-card weak-two opening,
`set_weak_two_hcp`; Ogust min/max deliberately left on `points`, the fit-known
leg — responder's 2NT promised support, mirroring the 2/1 hcp/support-points
split) is **sound bridge but does not ship**. Fix-vs-shipped
(`ab-point-count --weak-two-hcp 5:10`, both arms floored, `scripts/weak-two-ab.sh`):

| bracket | NV | Vul |
| --- | --- | --- |
| plain DD (1M) | +0.0017 ± 0.0017 | +0.0011 ± 0.0023 |
| perfect defense (1M) | +0.0131 ± 0.0022 | +0.0099 ± 0.0027 |
| **sd-lead (50k)** | **−0.0045 ± 0.0080** | **−0.0018 ± 0.0108** |

The signature is inverted from every shippable point-count win (plain wash, PD
positive, **sd-lead negative** — sd sits *below* both DD brackets, not between
them). A weak two is a **preempt = competitive range**, so per
[convention-tuning.md](convention-tuning.md) sd-lead is the arbiter and it is a
wash-to-loss: the marginal weak twos (the sound 9–10 HCP shapely hands the HCP
band adds, that floored `points` read 11–12 and passed) **over-disclose to the
opponents' blind opening lead** — the one bias plain DD and PD miss and sd-lead
prices. The plain-DD −2.0k/−3.1k the remnant report flagged is therefore the
disclosure/obstruction wall, not a fixable gauge. Forensics (`--show 40`): the
2♦ weak two is the biggest loser both directions/vuls; a **major-only carve**
(2♥/2♠ hcp, 2♦ `points`) measured **strictly worse** — plain-DD vul −0.0033
(CI clear) and sd-vul −0.0113. `set_weak_two_hcp` stays opt-in (default
byte-identical) as a single-dummy re-measure candidate; the `#1b` weak-two ask
answer folds in (same fit-known `points` reasoning — no change). New harness
capability: `ab-point-count` now builds two books for build-time gate knobs
(`Arms::WeakTwoHcp`), reusable for the remaining remnant families.

**Slice ledger: `24.pdd` rows 0..12,300,000 consumed; cursor at 12,300,000**
(2/1 through 8.1M; weak-two all-suits 8.1M–10.2M, major-only carve
10.2M–12.3M).

## The remnant close-out (2026-07-15) — competitive-X forensics + the last families

The forensic pass the competitive-X family was waiting for ("no one dominant
gate; probe before touching bands").  Method: replay the floored-vs-legacy
remnant run (rows 6.1M/7.1M — replay-for-tracing, not re-measurement) with
`--show 2000`, parse the worst-board dumps per sub-family, resolve which arm
made which call by seat parity (candidate sits EW at the `off` table), and
read the acting hand's HCP/shape/points on each divergent board.  The
`X ↔ bid` family decomposed into **four mechanisms**, none of them the
negative double (the shipped negX is `hcp(8..)`, scale-invariant):

1. **The overcall / double-first partition edge** (the `[1♦] X→1♠`,
   `[P 1♠] 2♣→X`, sandwich and passed-seat cousins — both mirror directions
   CI-flagged).  Weights make the effective partition "overcall until the
   band top (17), double first above it" — and *both* faces of that edge were
   `points`-denominated ([defense.rs](../src/bidding/american/defense.rs)
   `points(8..=17)` bands / `points(17..)` strong tier).  Rule-of-N+8 reads a
   5-4 fourteen-count 17+, so shaped 14–17 HCP hands (one dump board: a
   **nine-card** spade suit reading 18) route into X-first auctions and lose
   to the natural overcall.  "Too strong to overcall" is a defensive-trick
   promise — a high-card statement.  **Fix: `set_strong_double_hcp(18)`** —
   strong tier `hcp(18..)`, every overcall band top `hcp(..18)`, floors stay
   `points` (the obstruction win).  The 17-HCP shaped hands — the forensic
   winners — keep overcalling.
2. **Redouble-then-game `[1M X XX P]`** — the report's single worst
   per-board family (vul −16..−17 IMPs/board, near-deterministic; ≈−2.6k NV /
   −3.1k vul per 1M over all `X XX` buckets).  Not a gauge bug: an
   **unauthored continuation**.  Section 11 authors responder's XX but no
   opener answer, so the `FirstIs(Double)` systems-on rebase strips both the
   double and the redouble and opener replays *uncontested* — partner's shown
   10+ reads as silence, and the floor re-prices shaped minimums as
   game-going (stopperless 3NT off a 12-count 5-6).  **Fix:
   `set_redouble_answer`** — a pass-only authored node.  The first draft
   carried a "pure playing strength" 2M escape (6+ suit, ≤13 HCP); the smoke
   A/B measured that rung **−11 IMPs/fired** and it was deleted — a long-suit
   minimum is exactly the hand that wants to sit (one-of-a-suit redoubled
   makes with overtricks), and any pull reopens the auction for their runout.
   The forensic dump also shows the **doubler's side** sitting out a making
   `1M xx` after `[1M X XX P P]` — a separate defensive-side node candidate,
   parked pending post-fix forensics.
3. **Garbage Michaels** (`[1♥] 2♥→P` and mirrors, ≈−2.1k in the NV dump).
   Michaels and the Unusual 2NT are documented "8+ HCP" but were gauged
   `points(8..)`: a 5-HCP 6-6 freak reads 9, cues at weight 2.0, and eats
   −800 penalty doubles (−17..−21/board).  **Fix:
   `set_two_suiter_hcp_floor(8)`** — an `hcp(8..)` leg on both, making the
   documented floor real.
4. **Legacy's 4441 upgrade** (small, parked): legacy points reached the 17+
   tier on 16-HCP 4441s with length in *their* suit (singleton upgrades);
   rule-of-N+8's L2 term is blind to 4441 (L2 = 8 → +0), so those hands now
   pass.  Rare; revisit only if post-fix forensics still flag it.

Also visible at `--show 2000` (the original `--show 40` cut them off), and
deliberately **not** chased: the `[1NT] 2♥↔P` natural-1NT-defense buckets
(that band is sd-tuned — the plain-DD remnant is the known wall; if anything,
`set_natural_overcall_points` is an sd re-sweep candidate under the new
scale) and the `[] 1♠↔2♠` weak-two↔1-opener seam (the weak-two family's
edge; a `points(5..=10) | hcp(5..=9)` union band would re-admit the shapely
sub-10s, but the weak-two sd verdict prices exactly that class as
over-disclosure — parked).

The two small families fixed alongside:

- **2NT rebid-invite** `[1♥ P 1♠ P 2♦ P] 2NT↔P` (now flagged both vuls,
  both mirror directions): responder's one no-fit rung in
  `responder_after_minor_rebid` was `points(10..=12)` — a notrump invite
  priced in ruffs it never takes.  **Fix: `set_nt_invite_hcp`** —
  `hcp(10..=12)`; the fit-showing 3♥/3m invites keep `points` (the 2/1
  hcp/support-points split, again).
- **Freak opening leg** `[P P] P→1♦` ×138 (−155 NV): `points(12..) &
  hcp(..10)` ⟺ eleven-plus cards in two suits, so a 9-HCP 6-5 (reads 12)
  walks in the sound-opening front door; legacy passed or preempted.  **Fix:
  `set_opening_hcp_floor(10)`** on the four `points(12..=21)` openings; the
  rule-of-20 light rules already carry `hcp(10..=11)`.  The mirror leg
  (11-HCP 4441s that legacy's singleton upgrade opened and the new scale
  correctly passes) is the deposed upgrade scale re-litigated — left alone.

All five fixes are build-time knobs, measured fix-vs-shipped through
`ab-point-count --fix <spec>` (the `Arms::GateFix` two-book path generalizing
`Arms::WeakTwoHcp`), both arms on the shipped floored scale
(`scripts/remnant-fixes-ab.sh`, 1M boards/vul plain+PD each, 50k/vul sd-lead
for the two competitive ranges where sd is the arbiter):

| fix | plain DD NV / vul | PD NV / vul | sd-lead NV / vul | verdict |
| --- | --- | --- | --- | --- |
| `strong-double-hcp:18` | **+0.0105 ± 0.0012 / +0.0115 ± 0.0016** | +0.0114 / +0.0126 | **+0.0159 ± 0.0054 / +0.0115 ± 0.0072** | **default-on** |
| `redouble-answer` | **+0.0056 ± 0.0005 / +0.0078 ± 0.0007** | +0.0058 / +0.0080 | — (constructive) | **default-on** |
| `two-suiter-hcp:8` | **+0.0023 ± 0.0008 / +0.0031 ± 0.0010** | +0.0028 / +0.0036 | +0.0024 ± 0.0035 / +0.0046 ± 0.0043 | **default-on** |
| `nt-invite-hcp` | **+0.0018 ± 0.0003 / +0.0022 ± 0.0005** | +0.0028 / +0.0032 | — (constructive) | **default-on** |
| `opening-hcp-floor:10` | +0.0000 ± 0.0003 / +0.0000 ± 0.0003 | wash / wash | — | **wash → opt-in** |

Four ship default-on with **every bracket positive and no PD dip anywhere** —
these are gate-precision fixes, not aggression trades, so plain, PD, and sd
agree.  The redouble answer runs +10.7/+14.0 IMPs *per divergent board*
(rare, huge); the strong-double partition is the largest total (+10.5k/+11.5k
IMPs per 1M boards — several times the −1.5k/−2.8k the report priced for the
whole X↔bid family, because the fix also repairs boards where *both* scales
mis-partitioned).  The opening floor is a genuine wash — the −155 IMPs/1M
freak family sits below a 1M-board A/B's resolution — so the sound-bridge
knob stays opt-in and the family closes as *wash*.

The two-suiter sd NV CI spans zero (+0.0024 ± 0.0035) with vul clear; that is
consistency, not the weak-two wall signature (sd there sat *below* both DD
brackets — here it sits between/above).  Plain + PD + sd all point the same
way, so it ships.

### Composite verification (floored-vs-legacy, all four defaults on)

Fresh slices (`24.pdd` rows 22.5M..23.5M NV, 23.5M..24.5M vul, 1M boards/vul,
`--show 40`): plain DD **+0.0473 ± 0.0037 NV / +0.0515 ± 0.0050 vul** — up
from +0.0377/+0.0347 at the previous re-run.  (Fix-vs-shipped gains do not
add linearly here: a build-time fix lands in *both* books of the
scale-vs-scale comparison, so the delta grows only by what each fix removes
of legacy's relative edge.)  Bucket check: the redouble-then-game,
garbage-Michaels, 2NT-invite, and direct-seat X↔bid buckets are **gone**
from the flagged set.  Standing, as expected: the weak-two band and Ogust
buckets (closed as the disclosure wall), the `[1NT]` natural-defense buckets
(the sd-tuned wall), and the opening-seam trickle (wash-priced).

One real residual: the X↔bid seam persists in the **sandwich**
(`[1♦ P 1♥] 1♠→X`, −5.1/board ×52) and **balancing** (`[1♦ P P] X→1♠`)
seats.  Those actions come from the instinct floor, not `defense_to_suit`
(which only serves the direct seat — the seat the fix closed), so the
prescription is a floor change: apply the same HCP partition to the floor's
overcall-vs-double choice.  Parked as the campaign's follow-up; floor
changes touch every auction and deserve their own measured pass.

### Family ledger — every remnant family now has a verdict

| family | verdict |
| --- | --- |
| One-level opening seam | **CLEARED** (4333 floor) + freak leg **wash** (`set_opening_hcp_floor` opt-in) |
| Quantitative 6NT | **CLEARED** (4333 floor); raw-HCP gauge parked probe-if-bored |
| 2/1 response band | **FIXED** (fit-split, e416a9d) |
| Weak-two band + Ogust answer | **WALL** (disclosure; `set_weak_two_hcp` opt-in sd re-measure candidate) |
| Competitive X ↔ bid: direct seat | **FIXED** (`set_strong_double_hcp(18)` default-on) |
| Competitive X ↔ bid: redouble-then-game | **FIXED** (`set_redouble_answer` default-on; doubler-side sit-out parked) |
| Garbage Michaels / UNT | **FIXED** (`set_two_suiter_hcp_floor(8)` default-on) |
| 2NT rebid-invite | **FIXED** (`set_nt_invite_hcp` default-on) |
| Competitive X ↔ bid: sandwich/balancing seats | **OPEN — floor follow-up** (HCP-partition the floor's overcall/double choice) |
| Natural-1NT-defense buckets | **WALL** (sd-tuned band; `set_natural_overcall_points` sd re-sweep candidate under the new scale) |
| Weak-two ↔ 1-opener seam | **WALL's edge** (union band would re-admit the sd-punished class; parked) |

**Slice ledger: `24.pdd` rows 0..24,500,000 consumed; cursor at 24,500,000**
(remnant fixes 12.3M–22.3M plain+PD, 22.3M–22.5M sd, composite re-run
22.5M–24.5M).
