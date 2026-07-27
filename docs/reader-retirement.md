# Retiring the hand-written disjunction readers

Campaign: replace each hand-written convention reader in
`src/bidding/inference.rs` with the authored rules' own DNF projection, one
measured chop at a time. This is the correctly-aimed half of "reading DNF
obsoletes the special cases" — the *readers* are the pre-DNF legacy, not the
alerts (the alert is what makes projection decoding sound: it gates the
decode, drives the natural-walk suppression, and selects rule variants at
build time; see [bidding-architecture.md](bidding-architecture.md)
§Disclosure).

The ledger at the bottom records verdicts. Nothing here is byte-identical by
construction — a retirement changes shipped readings — so every chop runs the
full [measurement.md](measurement.md) A/B.

## Why the readers exist, and why they can die

They predate `Dnf`. `Inferences` once held only per-seat interval envelopes,
which cannot carry a disjunction ("five spades *or* five hearts"), so
conventions whose meaning is an `Or` got hand-written readers that pin the
expressible half (suppress the natural walk, narrow the other suits) and let
the sampler deal the residual. Post-FLIP (`set_dnf_reading` default-on,
2026-07-23), the generic path carries the whole meaning: `project_authored`
unions each authoring rule's `project` fold via `Dnf::disjoin`, and the
sampler tests membership per box (`Inferences::admits`). A reader whose
knowledge is expressible as its rules' boxes is redundant machinery — and a
second copy of the convention's meaning that can drift from the rules (the
phantom-suit class of bug).

## Inventory

All in `src/bidding/inference.rs` (grep the names; line numbers drift). Each
is gated by its convention's own enable knob — confirm the exact wiring when
its chop starts.

| reader | what it reads |
| --- | --- |
| `rubens_reading` | Rubens advances of our overcall |
| `landy_advance_suppress` | advances of our Landy 2♣ |
| `multi_reading` | the Multi 2♦ in our Woolsey defense |
| `two_suiter_reading` | their Michaels/Unusual over our 1M |
| `gladiator_reading` | Gladiator responses to our 1NT overcall |
| `woolsey_x_reading` | our Woolsey X (4M + longer minor) |
| `responder_overcall_double_reading` | responder's X of their overcall |
| `penalty_latch_double_reading` | penalty doubles under the latch |
| `dont_reading` | our DONT over their 1NT |
| `meckwell_reading` | our Meckwell over their 1NT |

Out of scope here: the FBM census's six `as_rules() == None` classifiers
(the seat-fanned `[1NT 2♣]` closure ×4 and the two root `(always)` floors).
Those are invisible to projection *entirely* — converting them is the
[sampled-projection](ai-bidder/sampled-projection.md) campaign's ground.

## The migration rule (per reader)

1. **Express the meaning on the authoring rules.** The rule's constraint must
   project the convention's whole claim: DSL where it folds exactly
   (`and`/`or` suit-sets, staircases, `shapes`), `dnf_upgrade(legacy, boxes)`
   where a leg is context-sensitive or the composite projects loose — boxes
   pinned statically to the node's own suits (the Jacoby idiom;
   authoring-time boxes use `union`, never `disjoin`). The `.alert(...)`
   stays on the rule — it triggers the decode and the suppression.
2. **Diff the reader's residue before deleting.** What does it do beyond the
   projection? Extra suppression indices, two-sided narrowing (candidate:
   `project_band`), advance-side effects (`landy_advance_suppress`),
   persistent state (`penalty_latch_double_reading`'s latch). Whatever the
   boxes can't carry stays hand-written — a partial retirement is fine; a
   silent semantics drop is not.
3. **Knob per reader**, default keeping the reader, so the shipped system
   stays byte-identical until the A/B says otherwise.
4. **Measure — plain + PD, both vuls, real routing.** The C1/F caution
   applies with force: the frozen nets are calibrated to the *current*
   readings, so a true tighter reading can lose through them (C1 lost
   −0.037/−0.067 on a truthful closure; the F flip won only after the
   evaluator was retrained on knob-on readings). If the worst tails trace to
   net-priced decisions, budget the evaluator-twin recipe (F2b) before
   declaring the chop dead.
5. **Trace before declaring.** Worst divergent boards first
   (`ab-dump-diff --show`); the usual culprits are a residue semantic missed
   in step 2 or an unauthored continuation newly exposed.

## Ledger

| # | reader | chop | verdict |
| --- | --- | --- | --- |
| — | (none yet) | | |
