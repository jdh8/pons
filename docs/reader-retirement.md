# Retiring the hand-written disjunction readers

Campaign: replace each hand-written convention reader in
`src/bidding/inference/readers.rs` with the authored rules' own envelope-union projection,
one measured chop at a time. This is the correctly-aimed half of "envelope-union
reading obsoletes the special cases" — the *readers* are the pre-campaign legacy, not the
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

They predate `EnvelopeUnion`. `Inferences` once held only per-seat interval envelopes,
which cannot carry a disjunction ("five spades *or* five hearts"), so
conventions whose meaning is an `Or` got hand-written readers that pin the
expressible half (suppress the natural walk, narrow the other suits) and let
the sampler deal the residual. Post-FLIP (`set_envelope_union_reading` default-on,
2026-07-23), the generic path carries the whole meaning: `project_authored`
unions each authoring rule's `project` fold via `EnvelopeUnion::disjoin`, and the
sampler tests membership per box (`Inferences::admits`). A reader whose
knowledge is expressible as its rules' boxes is redundant machinery — and a
second copy of the convention's meaning that can drift from the rules (the
phantom-suit class of bug).

## Inventory

All in `src/bidding/inference/readers.rs`, with their tests in
`src/bidding/inference/readers/tests.rs` (grep the names; line numbers drift). Each
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
| `rubens_reading` | Rubens advances of our overcall | two knobs (`rubens_advances_enabled` + `rubens_transfer_reading`); the only reader touching the `support_points` axis. **Silent under the default** since the layer lost its re-measure (2026-07-31) — the reader shares `rubens_advances_enabled`. Reachable only through `--ns-rubens`, where its length claims are measured unsound on *both* sides; see §The Rubens layer below |
| `gladiator_reading` | Gladiator responses to our 1NT overcall | the `(2♣)`→`Pass` auction rebase and self-recursion, which no box can carry — a permanent **partial** retirement. The relay's `points 0..9` drift is **fixed** (2026-07-31): the band is gone, the projection's union carries the strength. No A/B was owed — `set_nt_overcall_gladiator` is default-off, so the change is byte-identical in the shipped system; see §The Gladiator relay below |
| `responder_overcall_double_reading` | responder's X of their overcall | no knob at all, and its `points ≥ 8` is a hand-derived intersection across three `DoubleStyle` variants — a real authoring job, not a delete |
| `penalty_latch_double_reading` | penalty doubles under the latch | reconstructs a latch by carrying `last_suit_bid` across calls, and its `penalty_x_reading` helper has an agreement contract with the floor (`instinct.rs`). Retire last, or never |

Out of scope here: the FBM census's six `as_rules() == None` classifiers
(the seat-fanned `1NT (2♣)` closure ×4 and the two root `(always)` floors).
Those are invisible to projection *entirely* — converting them is the
[sampled-projection](ai-bidder/sampled-projection.md) campaign's ground.

## The migration rule (per reader)

1. **Express the meaning on the authoring rules.** The rule's constraint must
   project the convention's whole claim: DSL where it folds exactly
   (`and`/`or` suit-sets, staircases, `shapes`), `envelope_union_upgrade(legacy, boxes)`
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
`players` by the `for (seat, projected) in overlay.iter().enumerate()` loop in
`inference/read.rs`, and `Envelope::narrow_length` / `narrow_points` are plain
per-axis `Range::intersect`. So a reader whose every narrowing is already
implied by `hull(overlay)` is running an idempotent intersect.

**Order matters, because `Range::intersect` is not a meet.** On disjoint
bounds it *widens to the span* rather than going empty (`inference/envelope.rs`,
`Range::intersect`), so the operation is non-associative exactly in the case the
subset argument is about. A reader that narrows *before* the fold can therefore
see a range the fold has not yet tightened, conflict with it, and widen — and
the same narrowing applied after the fold would not. The post-walk blocks run in
this order:

| # | Block | Relative to the fold |
| --- | --- | --- |
| 1 | `rubens_cue` | **before** |
| 2 | `rubens_transfer` | **before** |
| 3 | the overlay fold | — |
| 4–8 | `multi`, `woolsey_x`, `dont`, `meckwell`, `gladiator` | after |
| 9–11 | `penalty_x`, `penalty_latch_doubles`, `overcall_double` | after |

So the escape covers the nine blocks after the fold. The two Rubens blocks are
**not** eligible for it: chop them the normal way (knob, A/B, steps 3-5)
regardless of how the subset analysis comes out. That is moot today — the
Rubens chop is deferred on its own measurement (below) — but it binds if it is
ever reopened, and it binds for any new reader wired ahead of the fold.

A chop qualifies as a **no-op** when all five hold:

1. The reader's recording block runs **after** the overlay fold — see the table
   above.
2. Every axis the reader narrows is at least as narrow in `hull(overlay)` —
   check the authoring rule's projection, not its prose.
3. The authoring node exists for every auction the reader fires on, including
   the declarative leading-pass seat fan (`P*`).
4. No sibling rule shares the call unalerted — `project_call` unions all rules
   sharing the made call, so an unalerted catch-all would hull the floor away.
5. Every seat that reads it is covered: the opponents' call via the table-alert
   walk, the same call own-side via the exact-node/fallback walk. Both use the
   same `relative_of`, so attribution matches.

Then skip the knob and the A/B. Instead: pin the subset property in a test
across every seat and seat-fan (stronger than an A/B, which only shows the
divergent set was empty *on those seeds*), and diff
`probe-call-reading` before and after for the empirical confirmation at zero
DDS cost. If that diff moves, the analysis is wrong — stop and escalate to the
knob and the full A/B.

Anything that fails one of the five is a normal chop: knob, A/B, steps 3-5.

## The Rubens layer — measured, chop deferred

`rubens_reading` was queued as a normal chop. Probing it first (the
`probe-bba-constraints` `ucb-*` / `rub-ch` modes, 20k advancer hands per
auction per arm) found something the chop rule does not cover: **the reader's
claims are false, and the convention under them may not be live.**

What the reader asserts, against what actually happens (violation = the
reading's box excludes the truth; the ambient rates from
[deviation-panel.md](deviation-panel.md) are 8.2/8.3% for LHO/RHO and 3.3%
for partner):

| claim | regime | BBA | ours |
| --- | --- | --- | --- |
| `len(Y) ≥ 3` | two-level cue, `Y` a **major** (`1♠ (2♥)`) | 0.0% | 2.8% |
| `len(Y) ≥ 3` | two-level cue, `Y` a **minor** (`1♠ (2♦)`, `1♠ (2♣)`, `1♦ (2♣)`) | 19–28% | 5.9–87.5% |
| `SP(Y) ≥ 10` | two-level cue, `Y` a major | 7.6% | 17.3% |
| `SP(Y) ≥ 10` | two-level cue, `Y` a minor | 0.0% | 0.0% |
| `len(Y) ≥ 3` | one-level transfer into partner's suit | *(our side only)* | 93.3% |
| `points ≥ 10` | one-level transfer into partner's suit | *(our side only)* | 0.0% |

Two things fall out.

1. **The cue's meaning is structure-determined, not universal.** Ours is
   authored as a raise (`rubens_cue_raise & support(3..) & points(10..)`);
   BBA's is a strong general force, because it has no transfer layer to carry
   the strong hands and so must cue with them — short in partner's suit one
   time in four. Over a *major* both sides mean the raise (support is worth
   showing); over a *minor* the cue is half a stopper ask. The band that would
   be sound for the major cue is `8..`, not `10..` (violation 0.1% BBA /
   3.1% ours at 8, vs 7.6% / 17.3% at 10).
2. **The layer is largely dead where it is supposed to own the node.** Over
   `1♣ (1♥) -`, our own bidder picks the `2♦` transfer-into-partner's-hearts
   **0.4%** of the time and those hands hold **6–7 diamonds and 1–2 hearts** —
   natural diamonds; the `2♣` "transfer to diamonds" holds no diamond suit
   (median 3) and is the floor cueing; and the natural `2♥` raise the transfer
   layer exists to abolish is alive at 13.4%. The authored rule is `−∞` for
   those hands, the floor calls anyway, and the reader decodes the floor's
   natural call as an artificial one. That is the phantom-suit class, on our
   own partner.

So the reader's unsoundness is downstream of a convention the bidder mostly
does not play, and `set_rubens_advances` has never been measured against the
natural ladder it replaced — [bidding-options.md](bidding-options.md) files it
as *"baseline default; knob is the A/B off-arm"*. **The layer A/B comes first**
(`scripts/ab-rubens.sh`, `--no-ns-rubens` vs default, plain + PD, both vuls):
the knob silences `rubens_reading` too, so a wash or a loss retires the reader
along with the convention and every leg above becomes moot. Only a win makes
the leg repairs worth authoring.

Generalisation worth carrying to the other nine: a reader is only as sound as
the *bidder*, not the rule it transcribes — read a call off the bidder before
trusting its authored meaning
([sampled-projection.md](ai-bidder/sampled-projection.md)).

**Verdict (2026-07-31): the layer lost on re-measure, and is now default-off.**
It won M6.3 (2026-07-02: plain +0.0016 ±0.0015, CI excluding zero; PD −0.0009
wash; 1144 fired) with both sides' continuations authored. Re-run on the
current system it reverses, and fires 5× less often — 204,800
bd/arm/vul, SEED_BASE 1785426828, sha `4485555` — plain −0.0009 ±0.0009 NV /
−0.0008 ±0.0011 vul, PD −0.0014 ±0.0011 / −0.0014 ±0.0013, all four cells
negative and both PD CIs clear of zero; fired 0.11%/0.09%. The tail is
over-reach, not one unauthored continuation (15 of 60 worst boards involve a
double at all). So `rubens_reading` is now **silent under the default** and its
three unsound legs are moot: the chop becomes a straight deletion whenever the
knob itself is retired, and until then the reader is only reachable through
`--ns-rubens`. Nothing here needs authoring.

## The Gladiator relay — the band deleted, the arm kept

`gladiator_reading`'s `Relay` arm stamped `points 0..=9` ("weak-or-invitational,
< game") on the advancer's `2♣`. The rule authoring that relay is a **three-way
disjunction** whose third arm is `points(game..) & balanced() & len(o, 3..=3) &
!flat_4333()` — a game-forcing hand with exactly three in the unbid major, whose
entry that arm is to the delayed cue (`gladiator_relay_continuation` authors the
cue `points(inv..)`, unbounded above). The stamp lands on the walk hull before
`assemble`, which folds `EnvelopeUnion::from(players[i]).intersect(&overlay[i])`, so
`0..=9 ∩ 10..` emptied exactly that box: a **wrong** box, not a loose one.

Deleted, with no A/B owed — `set_nt_overcall_gladiator` is `Cell::new(false)`
and `gladiator_reading` returns `None` on its first line when the knob is off,
so the shipped system cannot reach the code (the §subset escape's spirit: a run
that must print zero is a run you did not need).

Three findings worth keeping:

- **Our Gladiator is not the reference's.** The card we play
  (<https://www.bridgewebs.com/crowborough/NT%20Responses.htm>) defines `2♣` as
  weak-or-invitational *only*, and parks the 3-card-major ask in a **direct**
  `2♦` Extended Stayman (INV+). That is Gladiator over a 1NT *opening*. Over our
  1NT *overcall* `2♦` is natural and the cue is Stayman, so the relay has to
  absorb the ask — which is why its third arm is game-forcing and why `0..=9`
  was wrong as an agreement, whatever the bidder happens to do.
- **Arm 3 is weight-shadowed, deliberately.** At `0.5` the relay loses to `3NT`
  (`1.2`) and to the `3♣`/`3♦`/`3O` naturals (`1.3`), all of which admit the same
  hands, so no hand plays it: the arm is **read, never played**. Promoting it
  (its own rule at a higher weight, or gating the shadowing calls) was rejected
  — the box is too confined to yield enough hands for a verdict, so the change
  could never be adjudicated. Pinned as a divergence in
  `gladiator_advances_follow_the_card`, not hidden.
- **A separate defect the sweep turned up — now fixed.** The card's *natural*
  game-forcing advances are authored `len(suit, 5..) & points(game..)`, but the
  natural walk read a jump advance as a weak **6+** with no strength, so every
  five-card GF advancer was excluded by its own reading (`KQT.K8.AJT64.QJ4` bids
  `3♦`, read `♦ 6..=13`; 16 of 256 random advancers hit it). Fixed 2026-07-31 in
  the walk, not here: `over_one_notrump` now also recognises our 1NT *overcall*,
  which systems-on got for free through `systems_on_overcall_strip` and
  Gladiator did not. Default byte-identical (the strip covers every auction the
  new clause does, whenever it is on). This is regime 2 of
  [reading-drift-handoff.md](reading-drift-handoff.md) — authored-but-natural
  calls, which no envelope-union projection reads and no static test compares.

Pinned by three new tests: `gladiator_advances_follow_the_card` (one hand per
advance and per relay continuation, replaying the **bidder** — so a floor that
drifts under the structure goes red instead of silently ceasing to fire),
`gladiator_readings_admit_the_bidder` (256 seeded hands over the quiet, relay
and doubled branches; **every** call the bidder makes must be admitted by the
reading it produces — unscoped since the walk fix) and
`gladiator_runs_out_of_the_doubled_overcall` (§below). The second is the
behavioural analogue of `authored_rules_eval_within_projection`, which cannot
reach this table: that sweep walks the shipped tries, and `gladiator_advances`
is only in one with the knob on.

## The doubled overcall — the strip is load-bearing for the floor

`(1M) 1NT (X)` had no book node under Gladiator; the comment said the
instinct-floor runout would answer it. It does — under `american_instinct()`,
identically in both arms. Under the shipped **distilled** floor it did not:
Gladiator turns off `systems_on_overcall_strip` (its advances differ, so the
strip identity fails), the net was distilled on the stripped picture, and fed
the unstripped one it escaped a **1-count to `3♥` doubled**. That is ≈40% of the
treatment's measured loss, and it is the mechanism behind the `vs-X-*` buckets.

`gladiator_doubled_runout` now authors the node — `XX` on values, else run to a
five-plus suit, never into their major, else sit — because a finite book node
shadows the floor and stops the runout depending on what the net makes of a
picture it never saw. The general lesson (a reading knob is a bidding knob under
a neural floor) is written up in
[reading-drift-handoff.md](reading-drift-handoff.md).

Two more nodes were authored the same way, and the rest of the tree was probed
and deliberately left alone. The floor answered a *weak `2O` signoff* off the
relay by raising on three trumps or bidding `3NT` opposite a hand that had
denied 8 points (`gladiator_relay_signoff_answer`: pass, or `3O` on four trumps
and 18), and answered **Leaping Michaels `4♣` with `5NT`**
(`gladiator_leaping_answer`: the major game on a three-card fit, else five of
the minor). Every remaining leaf is "advancer passes the game opposite a limited
hand", where the floor is right on all six probed and a bare `Pass` node would
only shadow its slam machinery — pinned either way by
`gladiator_continuations_are_authored_to_the_leaf`.

## Ledger

| # | reader | chop | verdict |
| --- | --- | --- | --- |
| 1 | `two_suiter_reading` | Deleted whole — suppression and recording together (~85 lines). Their Michaels cue of our 1M and their unusual `(2NT)` now read solely from `project_authored`'s table-alert decode of the `.alert(MICHAELS)` / `.alert(UNUSUAL)` rules in `defense_to_suit`. `set_uvu_over_majors` keeps its book half only | **NO-OP, adopted unmeasured** (the subset escape). Michaels projects to two boxes `{om≥5, ♣≥5, pts≥8} ∪ {om≥5, ♦≥5, pts≥8}`, hull `{om≥5, pts≥8}`; unusual to one box `{♣≥5, ♦≥5, pts≥8}`. Both hulls **contain** the reader's whole claim and add the rule's `pts ≥ 8`, and the boxes pin the unknown Michaels minor the reader conceded it could not. Residue: **none** — the only suppression target (the cue) is alerted, so `project_call` sets the bit anyway, and the `(2NT)` never needed one. Verified by `retired_two_suiter_reader_is_subsumed_by_the_projection` (5 auctions × both reading seats) and a byte-identical `probe-call-reading` diff over 9 auctions. Loss confined to keyless contexts and the `--no-ns-table-alert-reading` off arm, where it is sound-but-looser (arguably a fix — that arm silently kept half the disclosure the flag claims to remove) |
