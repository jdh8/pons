# Retire `artificial()` — complete alert coverage, then drop the heuristic

**Status: DONE.** The structural `artificial()` decode fallback has been dropped
from the gate; alerts carry the "decode this call" signal exhaustively. The
retirement-invariant test (`artificial_calls_are_alerted`) is now un-ignored and
green — a **permanent regression guard** that fails if a future artificial bid is
added without an `.alert(...)`.

**History:** #1 Michaels (`aa237be`), #2 Unusual 2NT (`955fada`), #3 Leaping
Michaels (`842da31`), #4 responsive doubles + the Pass/Double half (`artificial()`
went bid-only; #5's trap pass naturalized). #6 transfers over 2NT
(`Alert("jacoby-transfer")`) + #7 puppet/relay continuations
(`splinter`/`puppet`/`slam-try`/`smolen`) swept the last 68, then the bid drop.
Worklist 172 → 120 → 68 → **0**.

**The drop was a provable identity, not a measured wash.** The gate projects the
*union* of a node's rules for a call; whenever that union floors a non-named suit
(the structural witness), every matching rule floors it too → every matching rule
is alerted (invariant green) → the `alerted` term is already true. So
`alerted || artificial` and `alerted` fire on exactly the same calls under the
default `alert_reading() = on`. No BBA run needed. (With `alert_reading` off these
calls go undecoded, where the fallback formerly caught them; `alert_reading` stays
the master switch.) The `artificial` fn survives `#[cfg(test)]`-only as the
invariant guard's witness.

## Goal

Make `Rule.alert()` the **complete** "decode this call" signal so the structural
fallback `artificial()` can be deleted from the decode gate. This is the move
modern bridge made with doubles: **"X is self-alerting"** (a structural-category
exemption) was retired in favor of **alert-by-disclosed-meaning**. `artificial()`
is our self-alerting rule; alerts are the meaning.

## The gate (one site)

[src/bidding/inference.rs](../../src/bidding/inference.rs), `project_authored`:

```rust
let alerted = alert_reading()
    && rules.rules().iter().any(|r| r.call() == made && r.alert().is_some());
if let Some(projection) = projection.filter(|p| alerted || artificial(p, made)) {
```

- `artificial(p, made)`: the projection floors a suit ≠ `made`'s strain (Jacoby
  2♦→♥, Landy 2♣→44 majors). The structural witness.
- **Bid-only (done).** `artificial()` now returns `false` for Pass/Double: the
  witness is inverted for no-suit calls — a trap pass / penalty double floors the
  *opponents'* suit precisely because it wants to **defend** the contract on the
  table, which is natural. So Pass/Double are alert-only today; the remaining
  `|| artificial(p, made)` speaks for **bids** only.
- Retirement = delete `|| artificial(p, made)`, then delete `fn artificial` + its
  doc. Alerts alone then carry the signal.

## What the worklist test found (and where the original handoff was wrong)

The driver is `artificial_calls_are_alerted` in
[inference.rs](../../src/bidding/inference.rs) (`#[ignore]`, run with
`cargo test --all-features artificial_calls_are_alerted -- --ignored --nocapture`).
It walks every authored rule in all three `american()` tries and prints every
call where `artificial(project(rule), call) && !alerted`.

**172 counterexamples** across ~12 distinct authoring rules. Three corrections to
the prior handoff's assumptions:

1. **Doubles and passes ARE caught.** The handoff claimed `artificial()` is
   "always false for a double". False: our **trap-pass** (`[1♦ X P] P`, floors
   4+ in their suit) and **responsive doubles** (`[1♦ X 2♦] X`) carry shape
   projections. 32 doubles + 20 passes are in the list.
2. **No-suit calls need per-category judgement, not blind alerting.** *(Corrects
   the prior "alert every hit".)* The "floors a suit it did not name" witness is
   **inverted** for Pass/Double: a call that names no suit floors the *opponents'*
   suit precisely when it wants to **defend** the contract on the table — a trap
   pass, a penalty double — which is natural, not artificial. So the counterexample
   set is **not** the artificial set; it mixes the genuinely artificial doubles
   (responsive/takeout — they ask partner to pick a suit) with natural defend-it
   calls. The fix: `artificial()` is **bid-only**; the artificial doubles carry an
   alert (#4), the defend-it passes naturalize (the settle floor reads "pass = play
   the top bid"), and penalty doubles stay on their post-walk readers
   (`penalty_x_reading` / `penalty_latch_double_reading`, independent of the gate).
3. **Adding alerts is a decode no-op until the drop.** While `artificial()` stays
   in the gate, the `|| artificial(...)` already keeps every one of these calls,
   so `.alert(...)` changes only alert *disclosure*, not the read. The behavior
   change happens at the **drop**, which is a **true wash iff alerts cover exactly
   the artificial-set** — they will, by construction. This is the safety property
   that lets us sweep incrementally with no measurable risk, and measure once at
   the end.

### Gating footgun

Some `.gated()` blocks key on the alert slug and **silently drop** a rule whose
alert is not in the active set:

- [defense.rs:1533](../../src/bidding/american/defense.rs#L1533) `.gated(|t| alerts.contains(&t))`
- [notrump.rs:196](../../src/bidding/american/notrump.rs#L196) `.gated(|alert| alert != dormant_minors())`

When alerting a rule inside such a block, **add the new slug to that block's
active set** (or place the alert outside the gated region). The invariant test +
existing convention tests will catch a dropped rule.

## Worklist — one increment per convention

Each increment: add `.alert(SLUG)` to the rule(s), handle gating, run the
convention's own unit tests + the invariant test (it should shrink). No IMP
measurement per increment — adding alerts is a decode no-op (finding 3). Order is
low-risk first. Slugs follow the `Alert("ns:slug")` house style; reuse existing
constants where one already exists.

| # | Convention | Sample auctions | Source | Alert | Gating |
|---|---|---|---|---|---|
| ✅1 | Michaels cue-bid | `[1♦] 2♦`, `[1♠] 2♠`, `[1♣] 2♣` | [defense.rs](../../src/bidding/american/defense.rs) overcall/cue block | `MICHAELS` (`aa237be`) | none |
| ✅2 | Unusual 2NT | `[1♦] 2NT`, `[1♠] 2NT` | ungated tail of `defense_to_suit` (NOT the 1NT-defense `unusual_2nt()` — already alerted) | `UNUSUAL` `"unusual-2nt"` (`955fada`) — named `UNUSUAL` to dodge the `set_unusual_notrump_defense` thread-local | none (outside the `active_alerts()` gate) |
| ✅3 | Leaping Michaels | `[2♥] 4♣/4♦`, `[2♦] 4♦` | `defense_to_weak_two` LM block (overcalls). The `leaping_michaels_advances` continuations project no foreign suit → not in the worklist | `LEAPING` `"leaping-michaels"` (`842da31`) — named `LEAPING` to dodge the `leaping_michaels_enabled` thread-local | `leaping_michaels_enabled()` only; outside `active_alerts()` |
| ✅4 | Responsive double (takeout family) | `[1♦ X 2♦] X`, `[1♦ X 3♦] X` | [defense.rs](../../src/bidding/american/defense.rs) `responsive_doubles` / `responsive_overcall_doubles` | `RESPONSIVE` `"responsive-double"` — asks partner to pick a suit (artificial) | `responsive_*_enabled()` toggles |
| ~~5~~ | ~~Trap pass~~ → **natural, not alerted** | `[1♦ X P] P` | naturalized by bid-only `artificial()`; the settle floor reads "pass = play the top bid" — the trap pass *defends* the doubled contract, so it is not artificial. (The resp-3NT trap in `competition.rs set_trap_pass` was never a counterexample: it floors HCP, not length.) | — (no alert) | — |
| ✅6 | Transfers over 2NT (opening + 2♣ rebid) | `[2NT P] 3♦/3♥`, `[2♣ P 2♥ P 2NT P] 3♦/3♥` | `two_notrump_responses` (the 3♦/3♥ transfers only — 3♣ Stayman is an OR-disjunction the witness never flags) | reused `JACOBY` (`"jacoby-transfer"`) | none — outside the `.gated()` block |
| ✅7 | Puppet / two-way-relay continuations | `[1NT 2♠ 2NT] 3♦/3♥/3♠/3NT`, `[1NT 2♠ 3♣] 3♦/3♥/3♠`, `[1NT 2♣ 2M] 3OM`, `[1NT 3♣ 3♦] 3♥/3♠` | `two_spade_over_min`/`_max` club splinters → `SPLINTER`; slamless 6♣ `3NT` → `PUPPET`; `stayman_major_rebid` 3OM slam try → `SLAM_TRY`; `puppet_deny_rebid` 4-4 hunt → `SMOLEN` | new `SPLINTER`/`SLAM_TRY`, reused `PUPPET`/`SMOLEN` | none — the continuation nodes are plain `insert_uncontested`, not the `.gated()` response node |

The table is a map; the **test is the source of truth**. After each increment,
re-run the driver — the count drops by that convention's positions. Some hits
(`nt_structure_artificial` relays) are already suppressed in `Inferences::read`;
alerting them aligns the gate with that suppression and lets both retire together.

## The drop (final increment, the only one measured)

**The Pass/Double half already dropped** (this increment): `artificial()` is
bid-only, so passes/doubles are alert-only today. The one behavior change it
carried is the **trap pass naturalizing** — verify in the measurement below
(watch `[1♦ X P] P` and `1NT-(2M)-P-(P)` reopening boards). What remains is the
**bid-only** drop after #6–#7.

Pre-req: the invariant test is green (zero counterexamples — all remaining are bids).

1. Un-`#[ignore]` `artificial_calls_are_alerted`.
2. Delete `|| artificial(p, made)` from the gate; delete `fn artificial` + its
   doc comment (now a bid-only `Suit::ASC` scan).
3. The `alerted` term no longer needs the `alert_reading()` toggle as a *gate*
   for these calls (they're authored-alerted now) — but keep `alert_reading()` as
   the master switch unless a separate decision retires it.
4. **Measure** the drop: paired BBA A/B, `--filter-1nt`, `ab-dump-diff` plain +
   pd (template: `scripts/bba-gen-parallel.sh` + `scripts/idle-run.sh`,
   sequential — never idle-run parallel jobs). Expect a **wash** — that is the
   proof the invariant held. A regression means a missed alert (a counterexample
   the sample missed); the invariant test + the regressing board localize it.
   Ship per the **plain-win-or-wash + PD-positive** rule; hard opt-in only for a
   plain loss.

## Why it's worth it

1. **Honest disclosure** — alerts are real obligations; the sweep corrects the
   alert *output*, not just internal decoding (the `--advertise-natural` family).
2. **One fewer heuristic** — `artificial()` is structural and can misfire; once
   alerts are complete it is provably redundant.
3. **Aligns gate with `read` suppression** — the `nt_structure_artificial` and
   `*_reading` overlays already encode "this call is conventional" by hand;
   alerts make that one declaration on the authoring rule.

## Anchors

- Gate + `artificial()`: [inference.rs](../../src/bidding/inference.rs) `project_authored`
- Driver test: `artificial_calls_are_alerted` (same file, `#[ignore]`)
- `Alert` type + `.alert()` builder/getter: [rules.rs](../../src/bidding/rules.rs)
- Memory: `project_rule-projection`, `project_family-alert-split`,
  `project_per-call-alert-tags`
</content>
</invoke>
