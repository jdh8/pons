# Retire `artificial()` — complete alert coverage, then drop the heuristic

**Status:** in progress. The retirement-invariant test is landed (ignored); the
alert sweep is the worklist below. Bite off one increment at a time.

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
2. **The natural/conventional taxonomy resolves itself.** The counterexample set
   is *exactly* the shape-bearing calls. A genuinely natural penalty-X / pass
   projects no foreign suit → never in the list. So **alert every hit** is
   correct; no per-call judgement needed. (A natural penalty double that the
   handoff worried about is, by construction, not a counterexample.)
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
| 1 | Michaels cue-bid | `[1♦] 2♦`, `[1♠] 2♠`, `[1♣] 2♣` | [defense.rs](../../src/bidding/american/defense.rs) overcall/cue block | new `MICHAELS` | none |
| 2 | Unusual 2NT | `[1♦] 2NT`, `[1♠] 2NT` | [defense.rs](../../src/bidding/american/defense.rs) `set_unusual_notrump_defense` block | new `UNUSUAL_NT` | check the unusual-NT gated overlay |
| 3 | Leaping Michaels | `[2♥] 4♣/4♦`, `[2♦] 4♦` | [defense.rs:2435](../../src/bidding/american/defense.rs#L2435) `leaping_michaels_advances`, [2485](../../src/bidding/american/defense.rs#L2485) | new `LEAPING_MICHAELS` | `leaping_michaels_enabled()` block |
| 4 | Responsive double | `[1♦ X 2♦] X`, `[1♦ X 3♦] X` | [defense.rs](../../src/bidding/american/defense.rs) `set_responsive_takeout`/`_overcall` blocks | new `RESPONSIVE_X` | `responsive_*()` toggles |
| 5 | Trap pass | `[1♦ X P] P` | [competition.rs](../../src/bidding/american/competition.rs) `set_trap_pass` block (~429) | new `TRAP_PASS` | `trap_pass()` toggle |
| 6 | Transfers over 2NT (opening + 2♣ rebid) | `[2NT P] 3♦/3♥`, `[2♣ P 2♥ P 2NT P] 3♦/3♥` | [notrump.rs:843](../../src/bidding/american/notrump.rs#L843) 2NT-strength structure; [responses.rs:287](../../src/bidding/american/responses.rs#L287) `after_2nt` | reuse `JACOBY`/new `TEXAS`-style | shared by opening & rebid — alert once at the structure |
| 7 | Puppet / two-way-relay continuations | `[1NT P 2♠ P 2NT P] 3♠`, `[1NT P 2♠ P 3♣ P] 3♦`, `[1NT P 3♣ P 3♦ P] 3♥` | [notrump.rs:220](../../src/bidding/american/notrump.rs#L220) `puppet_minors` + continuations / Smolen-style major shows | reuse `PUPPET`/`SMOLEN`/new per-relay | **[notrump.rs:196](../../src/bidding/american/notrump.rs#L196) gated** — add slug to active set |

The table is a map; the **test is the source of truth**. After each increment,
re-run the driver — the count drops by that convention's positions. Some hits
(`nt_structure_artificial` relays) are already suppressed in `Inferences::read`;
alerting them aligns the gate with that suppression and lets both retire together.

## The drop (final increment, the only one measured)

Pre-req: the invariant test is green (zero counterexamples).

1. Un-`#[ignore]` `artificial_calls_are_alerted`.
2. Delete `|| artificial(p, made)` from the gate; delete `fn artificial` + its
   doc comment. `make` no longer needs the `Suit::ASC` scan.
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
