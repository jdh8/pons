---
name: author-convention
description: >
  End-to-end checklist for adding or changing a bidding convention/treatment in
  pons — authoring the rules, alerts, inference reading, knob, tests, and the
  A/B measurement that decides whether it ships. Use whenever the user asks to
  add, author, implement, tune, or fix a convention, treatment, response
  structure, defense, or any behavior of the bidding system.
---

# Authoring a bidding convention

Read [docs/bidding-architecture.md](../../../docs/bidding-architecture.md)
before touching `src/bidding`. Ship decisions come from
[docs/measurement.md](../../../docs/measurement.md) via the `measure-ab` skill.
Do the steps in order; skipping one has always cost more than doing it.

**Tuning or fixing a convention that already exists** (finding its best
strength/length range, or chasing down which of its calls leaks IMPs) is a
different loop from authoring a new one — read
[docs/convention-tuning.md](../../../docs/convention-tuning.md) instead of this
checklist. In short: *classify the range before you sweep it* (DD can only tune
a constructive contract-boundary; a competitive overcall/preempt floor hits the
obstruction wall and needs sd-lead), and *bucket by call before blaming the
wall* (a bad bucket is usually a fixable continuation, not a single-dummy loss).
Come back here only if the fix means authoring new nodes — then steps 3–11 apply.

## Checklist

1. **Theory first.** Search authoritative sources for the convention's standard
   shape/strength/continuations; confirm deviations with the user (he knows
   bridge — ask). Check `docs/bidding-theorems.md` and the memory ledger for
   prior verdicts on the same idea: many "obvious" treatments already measured
   as losses, and re-litigating a measured verdict needs new evidence, not new
   enthusiasm.
2. **Find the home.** Constructive structures live in the module for their
   opening (`src/bidding/american/notrump/`, `responses.rs`, …); competition
   over our openings in `competition/`; defenses to their openings in
   `defense/`. Those directories hold **one agreement per file** — its knob,
   its `Rules` builders, its `Package` and its tests — with the parent `.rs` as
   an index; a new convention is normally a new module there, and its module
   doc says what the agreement is and which knob gates it. Reuse existing
   builders/nodes before writing new ones.
3. **Author the WHOLE convention, both sides.** The bid, partner's advances and
   continuations, opener/overcaller rebids, and the contested tails (they
   double it, they overcall it). **The contested tails are the most-skipped
   half, and skipping them has never been free**: registration is suffix-exact,
   so an artificial forcing call whose registered suffixes all end in `-` drops
   to the floor the moment an opponent acts — and the floor has no forcing
   channel, so it passes out a doubled ask or bids a phantom suit. Measured
   twice in one campaign: N1b's doubled stopper-ask passed out in `3♥x`, N1c's
   interference hole at −4.68 PD/fired vul
   (docs/one-notrump-competitive.md). A `systems_on_over_double` rebase often
   covers the whole `(X)` tail for free; the overcalls the opponents actually
   make need their own small tables. A relay whose saved space is never
   spent — or an advance left to the floor — measures as a loss even when the
   idea wins. Watch rule weights: an overlapping cheaper rule (e.g. a
   transfer) can swallow the hands your new rule was written for.
4. **Alert + reading, same change.**
   - Artificial calls get `.alert(Alert("kebab-slug"))`; the invariant test
     `artificial_calls_are_alerted` fails on a missed one. Pick a const name
     that doesn't collide with a toggle thread-local.
   - Add the `Inferences` reading in `src/bidding/inference.rs`: suppress the
     literal natural reading at the artificial bid's index and post-walk narrow
     the real shape. Gate the reading on the convention's toggle. Without it,
     the floor "raises" the phantom suit into a doubled disaster the moment
     opponents intervene past the book.
   - A call constrained to nothing (`hcp(0..)` — a forced puppet) projects
     nothing, so the `artificial_calls_are_alerted` witness cannot see it:
     alert it by hand or the walk reads it as a real suit.  Use
     `.alert_if(agreements.decision.reading.completion_alerts, COMPLETION)`
     for completion-family calls (the uniform doctrine; the invariant
     `completion_readings_admit_the_bidder` covers the lanes), a bespoke
     always-on alert for anything else.
   - If the convention is **theirs** (a `their.*` disclosure), wire the read
     side too: a disclosure that moves only the book leaves the floor bidding
     on a false opponent envelope (docs/one-notrump-competitive.md §N1g). The
     reader must be seat-gated to their calls only.
5. **Gate with an `Agreements` knob** (a field in the matching area struct, or
   `decision.reading` for read-side switches; no thread-locals since 0.11),
   default **off** while measuring, off-state byte-identical. Wire it into the
   relevant `ab-*` example and/or `bba-gen` as a CLI flag.
6. **State shapes with the DSL, not ad-hoc closures.** Two-suiter minimums
   differ by convention (DONT 4-4, Landy 5-4, Michaels 5-5); use the
   `and`/`or` suit-set combinators so the constraint reads like the spec and
   projects correctly. Every rule table ends in a finite catch-all.
7. **Tests.** Unit tests for the new nodes, an `Inferences::read` test for the
   reading, and a full-auction integration test (`tests/american_*.rs`) that
   plays the whole sequence through the real stance — per-node checks miss
   whole families. If a floor rule is involved, verify it actually fires (a
   book node with finite mass shadows it).
8. **Gates:** `cargo fmt`, `cargo test --all-features`,
   `cargo +nightly clippy --all-targets --all-features -- -D warnings`,
   `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`.
9. **Measure** with the `measure-ab` skill. Do not ship on analysis alone.
10. **Ship decision** per the decision table in
    [docs/measurement.md](../../../docs/measurement.md): default-on for a
    plain-DD win or plain-wash+PD-win; rejected treatments stay opt-in with the
    default byte-identical. Flipping a default updates the integration tests
    that encoded the old one.
11. **Disclose it.** `src/bidding/card.rs` generates our `.bbsa` cards from the
    live knob state, so BBA reads our alerts correctly. If BBA's schema has a row
    for the convention, wire it to your knob there (a row is computed *iff*
    something can move it); if it has none, say so in the constant arm. Re-bless
    with `cargo run --example bba-card -- --system american >cards/American.bbsa`
    (and `--system dutch >cards/Dutch.bbsa`), and re-bless
    `tests/fixtures/alert-sites.txt` — the tripwire that fired to send you here.
12. **Record**: CHANGELOG entry with the measured numbers (IMPs/board,
    IMPs/fired, board count, vulnerabilities); ledger row if it's a tracked
    campaign item; propose the commit message (commit directly on `main`).

## Delegation

Mechanical chunks (a rule table from an exact spec, harness plumbing, test
scaffolds) suit cheaper subagents with precise specs — exact constraints,
file ownership, verbatim test cases — so domain theory can't go wrong in the
subagent. Design, weights, floor interplay, and ship decisions stay in the
main loop.
