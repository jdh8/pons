# Retiring the hand-written disjunction readers

Campaign: replace each hand-written convention reader in
`src/bidding/inference.rs` with the authored rules' own DNF projection, one
measured chop at a time. This is the correctly-aimed half of "reading DNF
obsoletes the special cases" — the *readers* are the pre-DNF legacy, not the
alerts (the alert is what makes projection decoding sound: it gates the
decode, drives the natural-walk suppression, and selects rule variants at
build time; see [bidding-architecture.md](bidding-architecture.md)
§Disclosure).

The ledger at the bottom records verdicts. A retirement normally changes
shipped readings, so the default is the full [measurement.md](measurement.md)
A/B — but chop 1 established that this is not true *by construction*: where the
reader's narrowing is provably a subset of what the projection already folds
in, the chop is a no-op and the A/B has nothing to measure. See
[the subset escape](#the-subset-escape) below for when that applies.

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
its chop starts. The `blocker` column is what stands between the reader and its
chop; four of them are one missing `.alert(...)` away.

| reader | what it reads | blocker |
| --- | --- | --- |
| ~~`two_suiter_reading`~~ | ~~their Michaels/Unusual over our 1M~~ | **retired, ledger 1** |
| `meckwell_reading` | our Meckwell over their 1NT | `meckwell_x_advance`'s 2♣ relay is unalerted, so the projection cannot suppress it |
| `dont_reading` | our DONT over their 1NT | same, ×4 (`passed_dont_{x,2c,2d,2h}_advance`) |
| `woolsey_x_reading` | our Woolsey X (4M + longer minor) | same (`woolsey_x_advance`'s 2♣) |
| `multi_reading` | the Multi 2♦ in our Woolsey defense | same (`multi_advances` pass-or-correct) |
| `landy_advance_suppress` | advances of our Landy 2♣ | same, plus `equal_majors` is an opaque `equal_length` predicate that projects nothing |
| `rubens_reading` | Rubens advances of our overcall | two knobs (`rubens_advances_enabled` + `rubens_transfer_reading`); the only reader touching the `support_points` axis; cue recording is not side-gated while the transfer's is |
| `gladiator_reading` | Gladiator responses to our 1NT overcall | the `(2♣)`→`Pass` auction rebase and self-recursion, which no box can carry. **Also drifts**: stamps `points 0..9` on the relay while the rule's third arm is `points(game..)` = 10+, deleting the projection's GF box. Fix that first, on its own A/B |
| `responder_overcall_double_reading` | responder's X of their overcall | no knob at all, and its `points ≥ 8` is a hand-derived intersection across three `DoubleStyle` variants — a real authoring job, not a delete |
| `penalty_latch_double_reading` | penalty doubles under the latch | reconstructs a latch by carrying `last_suit_bid` across calls, and its `penalty_x_reading` helper has an agreement contract with the floor (`instinct.rs`). Retire last, or never |

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

## The subset escape

Steps 3 and 4 (knob, then A/B) buy protection against a *changed* shipped
reading. Some readers change nothing, and then the knob is a switch whose two
positions are indistinguishable and the A/B is a run that prints zero. The
reason is structural: `project_authored`'s overlay hull is folded into
`players` **before** any reader's post-walk recording block (`inference.rs`,
the `for (seat, projected) in overlay.iter().enumerate()` loop), and
`Envelope::narrow_length` / `narrow_points` are plain per-axis
`Range::intersect`. So a reader whose every narrowing is already implied by
`hull(overlay)` is running an idempotent intersect.

A chop qualifies as a **no-op** when all four hold:

1. Every axis the reader narrows is at least as narrow in `hull(overlay)` —
   check the authoring rule's projection, not its prose.
2. The authoring node exists for every auction the reader fires on, including
   the leading-pass seat fan (`insert_all_seats`).
3. No sibling rule shares the call unalerted — `project_call` unions all rules
   sharing the made call, so an unalerted catch-all would hull the floor away.
4. Every seat that reads it is covered: the opponents' call via the table-alert
   walk, the same call own-side via the exact-node/fallback walk. Both use the
   same `relative_of`, so attribution matches.

Then skip the knob and the A/B. Instead: pin the subset property in a test
across every seat and seat-fan (stronger than an A/B, which only shows the
divergent set was empty *on those seeds*), and diff
`probe-call-reading` before and after for the empirical confirmation at zero
DDS cost. If that diff moves, the analysis is wrong — stop and escalate to the
knob and the full A/B.

Anything that fails one of the four is a normal chop: knob, A/B, steps 3-5.

## Ledger

| # | reader | chop | verdict |
| --- | --- | --- | --- |
| 1 | `two_suiter_reading` | Deleted whole — suppression and recording together (~85 lines). Their Michaels cue of our 1M and their unusual `(2NT)` now read solely from `project_authored`'s table-alert decode of the `.alert(MICHAELS)` / `.alert(UNUSUAL)` rules in `defense_to_suit`. `set_uvu_over_majors` keeps its book half only | **NO-OP, adopted unmeasured** (the subset escape). Michaels projects to two boxes `{om≥5, ♣≥5, pts≥8} ∪ {om≥5, ♦≥5, pts≥8}`, hull `{om≥5, pts≥8}`; unusual to one box `{♣≥5, ♦≥5, pts≥8}`. Both hulls **contain** the reader's whole claim and add the rule's `pts ≥ 8`, and the boxes pin the unknown Michaels minor the reader conceded it could not. Residue: **none** — the only suppression target (the cue) is alerted, so `project_call` sets the bit anyway, and the `(2NT)` never needed one. Verified by `retired_two_suiter_reader_is_subsumed_by_the_projection` (5 auctions × both reading seats) and a byte-identical `probe-call-reading` diff over 9 auctions. Loss confined to keyless contexts and the `--no-ns-table-alert-reading` off arm, where it is sound-but-looser (arguably a fix — that arm silently kept half the disclosure the flag claims to remove) |
