# Minor-suit transfers — the missing slam channel

**The campaign opened with every lane in which responder transfers to a minor
topping out at `3NT`, or at a `5m` opener placed unasked.** The engine's one
slam channel above a completed minor transfer was a single `4m` call in the
Landy counter — and that call had no authored answer, so the seat it created
belonged to the floor.

**Shipped 2026-08-25.** All three in-scope played lanes are now answered and
default-on: K–K at `competition.multi_minor_slam_try = Some(15)`, constructive
Puppet at `notrump.minor_transfer_slam_try = Some(13)`, and Landy at
`competition.landy_minor_slam_answer = true`. All three answer packages include
their complete RKCB continuations after both a clean pass and `(X)`. Their A/Bs
ran from code base `4eb925c2` plus the then-uncommitted
campaign worktree. jdh8's ruling on the constructive lanes — *"slamless looks
wrong"* — opened queue item 3; his ruling on the wider residues — *"the blast
radius looks large for constructive bidding, document them instead"* — kept
them open at the 2026-08-25 boundary. C1 closed the next day; the others remain
open below.

**C1 shipped 2026-08-26.** `notrump.minor_transfer_slam_fit = true` now lets
the exactly-5♦, 4+♣ member of the Puppet `2NT` class use the existing `4♦`
slam try after opener's `3♦`; the `3♣` denial remains six-card-only. Two fresh
same-floor 8M-board rounds won every plain-DD and perfect-defense cell.

**Ship-tail audit.** Completing the previously missing RKCB rows after their
double does not change the preserved A/B samples. The constructive harness
forces every opponent call to Pass, so `(X)` was structurally unreachable in
both 8M-board seeds. Full raw candidate-arm scans found zero doubled `4m` tries
in either Landy cell; all 27 divergent candidate auctions were also inspected.
The same scan found zero in both K–K rounds and all four `13`/`15` cells; all
351 divergent candidate auctions were inspected. Quiet-tail witnesses in both
corpora confirm the scan matched the stored auction encoding. The new tests pin
the completed tails; no same-seed rerun is owed for this structural repair.

Opened 2026-08-25 out of [§N4-KK residue 3](one-notrump-competitive.md#n4-kk--the-kokishkraft-counter-a-whole-table-variant-shipped-default-on-2026-08-25):
jdh8's ruling is that the residue belongs to every minor transfer, not that
lane, and the Landy counter is the lane that already half-solved it.

## Why this is one problem and not nine

A minor transfer buys right-siding and one step of room. The completion is
**forced and unconditional** — `hcp(0..)`, alerted only to stop the natural walk
reading the puppet as a suit ([`complete_lebensohl_relay`](../src/bidding/american/competition/lebensohl.rs),
[`kokish_kraft_minor_completion`](../src/bidding/american/competition/rubensohl.rs)) —
so it carries nothing back, and everything the transferor still wants to say has
to fit in the rungs above it.

At campaign opening, every lane spent those rungs on **shape** (second suits,
stopper cues, splinters) or on **placement** (`3NT`, `5m`). None spent one on
**size**: the only strength boundary any had was the game line. The transfer
was therefore wide at the bottom — a weak sign-off rode it — and wide at the
top — a 21-count rode it too — and after the completion nothing told them apart.

The floor could not rescue the seat. A rebid table with a finite catch-all
**shadows** the floor ([bidding-architecture.md](bidding-architecture.md)), and
every campaign-opening table below ended in `Pass, 0, hcp(0..)` or `3NT, 100,
hcp(0..)`. A call the table did not spell sat at `NEG_INFINITY` and could not be
made at all.

## The campaign-opening census (2026-08-25)

This table preserves the defect as it stood before the campaign builds; the
Queue and measurement sections record the shipped answers.

| lane | the transfer | band | above the completion | top of the ladder |
| --- | --- | --- | --- | --- |
| Constructive Puppet (default) | `1NT - 2♠` (→♣) / `1NT - 2NT` (→♦) | none / none; the game boundary is a hardcoded `8` at every site | splinter into the shortness, else `3NT` | opener places `3NT` / **`5m`**, total — [`pick_game_over_diamond_splinter`](../src/bidding/american/notrump/minor_transfers.rs) |
| Constructive European (opt-in) | `2♠` (→♣) / `3♣` (→♦) | none / none | the same `8` | **`3NT` in both minors** — no splinter arm, no `5m` at all. **Not a defect — see below** |
| N1j Landy `(2♣)` (default on) | `2NT` (→♣) / `3♣` (→♦), `len 6.. & points(2..)` | 2 / none | stopper cue `3♥`/`3♠` (10+), **`4m` (13+, six)**, `3NT` (10+), Pass | **`4m`**, and then the floor — [`landy_bba_transfer_rebid`](../src/bidding/american/competition/lebensohl.rs) |
| N1c legacy Landy stack (arm) | `2NT` (→♣) only, `points(2..=9)` | 2 / **9 — capped** | terminal | `3♣`, forced pass |
| §N1-lia Landy `(2♣)` (opt-in, `defense_2c_landy_lia`; **built 2026-09-01, later than this census**) | **not a transfer** — `2♠` = natural 6+♣, `2NT` = natural 6+♦ | 8 / **none** | opener answers by length **or accepts at `3NT`**; over the length legs the N1j rebid verbatim, over the acceptance `4m` (13+, six) | **`4m`** with `landy_slam_answer` + RKCB on **both** — the length legs and the acceptance — [`landy_lia_accept_rebid`](../src/bidding/american/competition/lebensohl.rs) |
| N4-KK `(2♦)` Multi (default on) | `2NT` (→♣) / `3♣` (→♦), `len 6..` and **no point term** | floorless / none | two-suiter steps (10+), `3NT` (10+), Pass | **`3NT`** — [`kokish_kraft_transfer_rebid`](../src/bidding/american/competition/rubensohl.rs) |
| N4-KK, they compete over it | same transfer | — | `3NT` (10+ with a stopper), `X` (`hcp 10+`), Pass | **`3NT`**, or their partscore doubled |
| N3 `(3♣)` transfer variant (opt-in) | `3♠` (→♦), `points(10..)` | 10 (GF) / none | **no transferor-rebid node at all** — the seat is *floored*, not shadowed | `3NT`, else `5♦` |
| Rubensohl `(2♥)`/`(2♠)` (default) | `3♣` (→♦), top step (→♣) | 9 / 10 | **no transferor-rebid node at all** | `3♦` (a partscore) or `3NT` |
| Gladiator, after our 1NT overcall (opt-in) | `2NT` (→♣) | **`points(..inv)` — capped** | — | `3♣` sign-off |

One row postdates the census and is marked as such. §N1-lia's rungs are not
transfers at all — responder declares its own six-card minor — but the rule
this document produced binds them anyway, because what it is really about is an
**uncapped minor rung whose ladder runs out**: opener's `3NT` acceptance can
land opposite a hand that wanted slam, and an unauthored `4m` above it reads as
nothing while the floor's keycard ask is gated on `undisturbed`. The lane is
disturbed by construction, so the `4m` and its answer are authored on the
acceptance as well as on the length legs. Recorded here as the "Landy port" the
queue below asked for; it is owed a measurement, not a decision.

### The European arm is out of scope — corrected 2026-08-25

An earlier draft of this census called the European arm "the more exposed of
the two" constructive lanes, on the grounds that it has no `5m` at all. That
reads it as a system we play. **It is not.**
[`notrump::european`](../src/bidding/american/notrump/european.rs) says so in
its own module doc: the transfers are pinned to EPBot's measured buckets and
"this is an **opponent model**, not a system we play: fidelity to EPBot is the
acceptance test, so the tables track the probe even where a soundness argument
would author something else."

So the European lane's missing `5m` is a *fidelity property*, and adding our
`4m` to it would be a regression — it would model an opponent who does not
exist. The two lanes share
[`diamond_transfer_game`](../src/bidding/american/notrump/minor_transfers.rs),
which is exactly where the leak would have happened; the slam try is threaded
in as a parameter and European passes `None` explicitly, with the reason in a
comment beside the call.

The lanes that are **not** defective are therefore three: the two capped ones
(N1c, Gladiator), where nothing strong ever transfers so nothing is stranded,
and the European arm, which is an opponent model. N3 and
Rubensohl are a *different* problem — a floored seat, not a shadowed one — and
belong in their own campaign, because fixing them means registering a node and
taking a seat away from the floor.

### The escape hatch is not open — corrected 2026-08-25

An earlier draft of this document, and of
[one-notrump-constructive.md](one-notrump-constructive.md), claimed the
constructive lane was the mild case because "a strong long minor need not
transfer at all: the direct quantitative `4NT` is still on the table". **That is
false.** The quantitative `4NT` is weight 120 and the minor transfers are weight
130, and the classes overlap, so the long-minor hand transfers. Probed:
`A32.32.AKQ876.K2` (16 HCP, six diamonds) at `1NT -` gives `2NT 1.300` over
`4NT 1.200`; `A32.32.K2.AKQ876` gives `2♠ 1.300` over `4NT 1.200`. The hatch is
open only to hands that are not long-minor hands, which is to say not these.

The same trap is worse under an overcall, where the direct `4NT` slot is gone
entirely (K–K's `4♣`/`4♦` are Leaping Michaels) **and** the transfer out-ranks
the values double, 176/178 against 130 — so the strong hand is routed into the
transfer and stranded there with no access to the quantitative `4NT` behind the
double. Probed: `32.AK2.A2.AKQJ32`, a 21-count, bids `2NT`.

## Three designs, all in-house

1. **Capped** — Gladiator, N1c. The transfer is the weak hand only;
   invitational-plus routes elsewhere. Nothing is missing because nothing strong
   ever transfers.
2. **Wide, with a `4m` slam try** — N1j Landy. The transfer takes every hand and
   the strength is spoken one round later.
3. **Wide, with nothing above `3NT`** — N4-KK and both constructive lanes at
   campaign opening.

Only (3) was a defect: a hand arrived in a seat where its values had no call.
This campaign moved the played lanes to (2).

## The doctrine, and where it breaks

N1 wrote the rule down:

> **`4♣`/`4♦` over opener's minimum rebids is a slam try** (13+ with a six-card
> suit); opener's continuation is deliberately the floor's — a `4m` *suit*
> contract lets the floor cue-bid on to slam where a notrump rung dies in `3NT`.
>
> — [closed N1 history](archive/one-notrump-competitive-closed.md), repeated at
> [`landy_bba_transfer_rebid`](../src/bidding/american/competition/lebensohl.rs)
> ("the cue stack's measured 6♦ lesson") and
> [`landy_bba_ask_answer`](../src/bidding/american/competition/lebensohl.rs)
> ("the slam-exploration doctrine")

The first half holds. **The second half does not, and it was never probed.**
With the rung authored at `1NT (2♦) 2NT - 3♣ - 4♣ -`, opener's whole floor
vocabulary is `{6NT 1.600, 4♥ 1.500, Pass 0.000}` — `4♥` being a contract in the
suit their Multi showed — and a minimum takes the `4♥`. There is no `5♣` and no
keycard ask on offer at all.

Two reasons, one of them structural:

- The deterministic floor's `4NT` ask is gated on `Context::undisturbed` — "the
  opponents have made nothing but passes" — so **it can never keycard in a
  competitive lane**. K–K, Landy, N3 and Rubensohl are all disturbed by
  construction. The doctrine's premise cannot hold in any of them.
- Even uncontested, the ask carries `combined_points(29)` against `own +
  partner's shown floor`, so a `13` floor lets only a 16-17 opener ask — and if
  the `4m` is *unauthored*, partner's shown floor is **zero**, which no opener
  can make up. Probed uncontested 2026-08-25: the ask is not merely rationed
  there, it is absent. See §"The constructive lane is worse, not milder".

**So the rule gains a second half:** a `4m` slam try owes an authored answer.
`american::slam::rkcb_rows(prefix, trump)` is already reachable from
`competition::lebensohl` with no visibility change — `lebensohl.rs` already runs
a full ladder for the direct `4M` tier — and it handles minor trumps.

## The rule this campaign shipped

> **A minor transfer that is not capped owes a `4m` rung above its `3NT`, and
> that rung owes an authored answer.**

One rung, one A/B, one seed — except where a rung and its interfered tail are
the same treatment, which ship together.

## Queue

1. **N4-KK — SHIPPED default-on 2026-08-25 at `Some(15)`.** ✅ Two rounds
   below; queue item closed.
   `competition.multi_minor_slam_try`, a `points` floor rather than a bool
   (`None` = off), default `Some(15)`. It authors residues 3 and 6 together, because the second is
   the first's interfered tail:
   - [`kokish_kraft_transfer_rebid`](../src/bidding/american/competition/rubensohl.rs)
     gains `4m` on `points(N..) & len(minor, 6..)` at w151, between the lowest
     two-suiter step (152) and `3NT` (150);
   - [`kokish_kraft_slam_answer`](../src/bidding/american/competition/rubensohl.rs)
     is opener's: `4NT` RKCB on `hcp(16..)`, else `5m`, plus `slam::rkcb_rows`
     after their pass or double.
     The `16` is a **constant across both arms**, so the arms differ in
     responder's floor and nowhere else;
   - [`kokish_kraft_transfer_overcalled`](../src/bidding/american/competition/rubensohl.rs)
     gains the shortness `4m` on `len(major, ..=1) & len(minor, 6..) &
     points(10..)` at w145, between `3NT` (150) and the penalty `X` (140), with
     an authored sit over it — jdh8's reroute in place of the `5m` residue 6
     first proposed. Eleven tricks become ten.

   Three arms — `off` / `13` / `15` — via `scripts/ab-2d-multi-slam.sh`. The
   floor is a payload because Landy's `13` is skimmed by stopper cues at
   w150/149 that this table does not have, so the same number fires on a
   materially wider class here.
2. **Landy — SHIPPED default-on 2026-08-25.** ✅
   This is a *fix*, not a copy: `landy_bba_transfer_rebid`'s `4m` had shipped
   default-on with **no authored answer**, the same floored seat this campaign
   found one lane over.

   **Result:** gate 0 foreign in both cells; eight readings, eight positive;
   +5.6/+7.1 IMPs per fired NV and +10.1/+11.4 both-vul, on n = 18 and 9. Full
   write-up and the single loss mechanism in
   [Landy round 1](#landy-round-1--measured-2026-08-25-seed-1787662790--win-thin--shipped)
   below.

   `competition.landy_minor_slam_answer`, a bool, now defaults **on**. It
   authors one seat and nothing else:
   [`landy_slam_answer`](../src/bidding/american/competition/lebensohl.rs) —
   `4NT` RKCB on `hcp(16..)`, else `5m`. The answer **and the complete
   `slam::rkcb_rows` ladder** are registered at `{completed} 4m -` and at
   `{completed} 4m (X)`; their double takes no room, so the package answers
   verbatim. The rung itself was untouched by the A/B.

   **What is deliberately not an arm: the responder floor.** Landy's `13` stays
   `13`. The two tables differ exactly where it would matter — Landy's `4m` is
   skimmed from above by stopper cues at w150/149 that K–K does not have — so
   K–K's shipped `15` does not transplant back by symmetry, and sweeping the
   floor here is a separate seed. The `16` in the answer is likewise a
   constant across arms: this experiment prices the *answer*.

   Runner: `scripts/ab-landy-minor-slam.sh`, two arms (`base` / `ans`), both
   vulnerabilities, `--filter-1nt`, gate `probe-divergence --gate-opener ours`
   at 0 foreign — run at `SEED_BASE 1787662790`, 2.304M retained
   boards/arm/vul, from `4eb925c2` plus the campaign worktree.

   Probed at the seat before authoring (`PROBE_THEIR_2C_LANDY=1
   probe-decision "AQ32.KQ54.A4.K32" "1NT (2♣) 2NT - 3♣ - 4♣ -"`): `base` gives
   `{6NT 1.600, 4♥ 1.500, Pass 0.000}` and a balanced 16 takes the `4♥` — a
   contract in a major their Landy `2♣` advertised. With the arm on, `4NT
   1.600`, `fallback: Some(7)` → a book node.
3. **Constructive Puppet minors — SHIPPED default-on at `Some(13)`
   2026-08-25.** ✅
   jdh8's ruling on the probe below was *"slamless looks wrong"*, which settles
   the open design question this item used to carry: the lanes name themselves
   "game-going, slamless"
   ([`club_no_shortness`](../src/bidding/american/notrump/minor_transfers.rs)),
   and that is the defect, not the design.

   `notrump.minor_transfer_slam_try`, a `points` floor (`None` = off), now
   defaults to **`Some(13)`**. Four rungs ship together because they are one
   call in the four seats a Puppet minor transfer can reach:

   | seat | table | the rung |
   | --- | --- | --- |
   | `1NT - 2♠ - 2NT -` | `two_spade_over_min` | `4♣` |
   | `1NT - 2♠ - 3♣ -` | `two_spade_over_max` | `4♣` |
   | `1NT - 2NT - 3♦ -` | `diamond_transfer_game` | `4♦` |
   | `1NT - 2NT - 3♣ -` | `diamond_transfer_correct` | `4♦` |

   All four are `minor_slam_try(minor, floor)` = `len(minor, 6..) &
   points(floor..)` at **weight 95** — deliberately *below* the splinters (100)
   and *above* `3NT` (90). So a slam hand that holds a shortness still
   splinters, which is the more informative call, and only the flat slam hand
   gives `3NT` up. Opener's answer is
   [`minor_slam_answer`](../src/bidding/american/notrump/minor_transfers.rs):
   `4NT` RKCB on `size_ask_accept_floor` (16), else `5m`. At all four seats,
   both the answer and the complete `slam::rkcb_rows` ladder are registered
   after a clean pass and after `(X)`.

   C1 later widened only the supported `3♦` seat to `two_notrump_class() &
   points(floor..)`. The other three gates remain `len(minor, 6..)`, and the
   answer tree above is unchanged.

   The European arm passes `None` — it is an opponent model, see above.

   Harness: `ab-nt-splinter --minor-slam N`, a third mode on the lane's own
   uncontested self-play harness (opponents silenced, plain + PD + sd-lead).
   Self-play for the same reason the splinter arms are: BBA has no counterpart
   toggle, so a gain measured against it would price misinformation.

   Two independent 8M-board rounds measured — see §"Constructive round 1"
   below. **All 16 DD+PD cells were positive; all 32 cells were positive when
   the SD-lead readings are included.** The runs used `4eb925c2` plus the
   campaign worktree.

4. **The survey — RUN 2026-08-25, 18 floored seats.** ✅ See §"The survey"
   below. Eleven of the eighteen are the Landy `(2♣)` family. Nothing is fixed
   from it yet, and the choice between "author eighteen answers" and "relax the
   `undisturbed` gate" is jdh8's, not taken.

5. **N3 and Rubensohl.** A different defect (floored, not shadowed). Out of
   scope for this campaign; recorded so nobody folds them in.

## Round 1 — measured 2026-08-25, `SEED_BASE 1787641731`

`scripts/ab-2d-multi-slam.sh`, SHA `8f1303e4`, `ab-results/2d-multi-slam/`,
230 400 boards per arm per vul, three arms.

**The gate first.** `probe-divergence --gate-opener ours` reads **0 foreign** in
all four cells — 13/6 divergent boards for `s13` (none/both) and 9/4 for `s15`,
100% "ours opened" in every one. The mirror book holds.

| arm | vul | fired | plain DD | PD | sd-lead | per fired |
| --- | --- | --- | --- | --- | --- | --- |
| s13 vs base | none | 13 | **+0.0004 ±0.0003** | **+0.0004 ±0.0003** | agrees | +7.54 |
| s13 vs base | both | 6 | +0.0002 ±0.0002 | +0.0002 ±0.0002 | agrees | +8.83 |
| s15 vs base | none | 9 | **+0.0003 ±0.0002** | **+0.0003 ±0.0002** | agrees | +8.89 |
| s15 vs base | both | 4 | +0.0001 ±0.0002 | +0.0001 ±0.0002 | agrees | +7.25 |
| s15 vs s13 | none | 4 | −0.0001 ±0.0002 | −0.0001 | agrees | −4.50 |
| s15 vs s13 | both | 2 | −0.0001 ±0.0001 | −0.0001 | agrees | −12.00 |

IMPs per board, 95% CIs. **No negative cell in eight** against `base`; NV is the
decision table's `win | win` row on both arbitrating scorers and both-vul is a
clean `wash | wash` — the same shape K–K itself shipped on, mirrored across the
vulnerabilities.

**"The floor is `13`, not `15`" — WITHDRAWN by round 2.** Round 1 read
`s15 vs s13` negative at both vuls on 6 fired boards and this section concluded
that Landy's number survived the transplant. Round 2 reads the *same comparison
positive* at both vuls. Neither reading is significant (round 2: `t` +0.70
plain / +0.43 PD). The head-to-head was never measured; it was 6 boards of
noise given a story. Left standing as the worked example of what `n = 19` buys.

**The mechanism is visible in the tail.** `2NT - 3♣ - 4♣ - 4NT - 5♦ - 6♣`
making where `base` stopped in `3NT` (+9); `3♣ - 3♦ - 4♦ - 5♦ - 6♣` (+10); and
the contested rung earning its keep at `3♣ 3♥ 4♣ - - 4♥ - - X`, doubling them
into the `4♥` the shipped table had to pass (+3). The one loss is a `6♦` reached
off `4♦ - 4NT - 5♠` that fails (−11).

**Not shippable on this alone: `n = 19`.** The rung fires on 19 of 460 800
boards for `s13` — **0.004%**, some 35× rarer than the K–K arm that contains
it — so all eight readings rest on nineteen boards, and a 95% CI computed over
230 400 boards of which 230 381 are exactly zero is a normal approximation with
almost nothing to approximate. The direction is consistent and the effect is
large where it fires; the *precision* is not there. Round 2 is the same three
arms at 10× the boards on a fresh seed (`ab-results/2d-multi-slam-x10/`,
`SEED_BASE 1787642695`), which costs about two hours and multiplies the fired
count by ten.

## Round 2 — measured 2026-08-25, `SEED_BASE 1787642695` — SHIPPED `15`

`scripts/ab-2d-multi-slam.sh`, SHA `8f1303e4`, `ab-results/2d-multi-slam-x10/`,
**2 304 000 boards per arm per vul** (10× round 1), same three arms, fresh seed.
Wall clock 2h15m.

**The gate first.** 0 foreign in all four cells again — 130/54 divergent for
`s13` (none/both), 95/40 for `s15`.

Per-board means are all `+0.0000`–`+0.0001 IMPs/board` with CIs at the printed
precision, which is the wrong frame for a rung that fires on 0.004% of boards.
The right frame is the fired boards, so these are sums and a one-sample `t` on
the fired deltas:

| arm | vul | fired | ΣIMP plain | per fired | `t` | ΣIMP PD | `t` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `13` | none | 130 | +140 | +1.08 | +1.61 | +135 | +1.45 |
| `13` | both | 54 | +121 | +2.24 | +1.85 | +92 | +1.33 |
| **`15`** | none | 95 | **+162** | **+1.71** | **+2.25** | +150 | +1.92 |
| **`15`** | both | 40 | **+147** | **+3.67** | **+2.61** | +111 | +1.83 |

sd-lead (16 worlds) agrees in sign in all four base cells. **All eight cells
positive**, `15` the stronger arm on every one of its four.

**Read one cell at a time — the vuls are not independent.** `base-none` and
`base-both` are generated from the same `SEED_BASE` and hold *identical deals*
(verified: `shard-0.json` board 0 matches); only the pricing and the bidding
differ. Pooling the two vuls would treat one set of deals as two samples and
inflate every `t` by about √2. Nothing here is pooled.

**The two floors are still not separated, and this is the real finding.**
`15 ⊂ 13`, so `s15 vs s13` *is* the 13–14 slice in isolation: 55 fired boards,
+42 IMPs plain (`t` +0.70) and +27 PD (`t` +0.43) — after round 1 read the
opposite sign at `t` ≈ −0.5. Per-fired standard deviation is ~8 IMPs and the
slice's per-fired mean is under 1, so separating the floors at 2σ would need
roughly 250 fired boards in the *slice*, i.e. another 10× on top of round 2.
Not worth buying: the slice is a wash whichever way it points.

**So `15` ships on its own merits, not on beating `13`** — the narrower trigger,
the cleaner win on all four of its cells, and 27% fewer fires for more total
IMPs. `13` is a legitimate alternative and stays one keystroke away
(`--ns-multi-minor-slam-try 13`, `PROBE_MULTI_MINOR_SLAM=13`).

**What this did *not* settle:** whether Landy's `13` should also become `15`.
The two tables differ exactly where the payload was made a payload — Landy's
rung is skimmed by stopper cues at w150/149 that K–K does not have — so the
number does not transplant back by symmetry. Queue item 2 priced the missing
answer only; Landy's responder floor remains `13`, and any floor sweep is a
separate future question.

## Constructive round 1 — measured 2026-08-25, seed `1787661918`

`ab-nt-splinter --minor-slam N --sd`, **8 000 000 boards per cell**, floors
`13`/`15` × vulnerability none/both, `ab-results/nt-minor-slam/`. Opponents
silenced (the harness's constructive mode), so this prices constructive value
only. Perfect defense still adds synthetic doubles of failing contracts; plain
DD remains the shipping gate. Wall clock ~25 minutes for all four cells
including the single-dummy pass — the lane is uncontested, so it needs no
opponent to cooperate and the run is cheap in a way the K–K experiment was not.
Run code was base `4eb925c2` plus the campaign worktree.

| arm | vul | fired | plain /fired | PD /fired | sd-lead plain /div | sd-lead PD /div |
| --- | --- | --- | --- | --- | --- | --- |
| `13` | none | 2184 | **+2.02** | +1.94 | +2.84 | +2.80 |
| `13` | both | 2184 | **+2.66** | +2.59 | +3.73 | +3.69 |
| `15` | none | 834 | **+5.05** | +4.92 | +6.02 | +5.99 |
| `15` | both | 834 | **+6.29** | +6.18 | +7.56 | +7.53 |

**Eight DD+PD cells and eight SD-lead cells, all positive.** Per-board the arms
are +0.0005 to +0.0010 IMPs/board. Fire rate is 0.027% at `13` and 0.010% at
`15` — some 7× and 2.5× the K–K rung's respectively. SD-lead is corroboration,
not slam insurance: it fixes the opening lead while leaving declarer
double-dummy perfect, an optimistic upper bound at slam. The ship decision
rests on the plain-DD and perfect-defense cells.

The analytic slam-optimism gate clears without needing the missing dumps. At
Pavlicek's maximal `q = 3%`, a making small-slam swing loses 6%; duplicate IMPs
are capped at 24, so even the deliberately impossible assumption that every
fired board is a positive +24 slam costs at most 1.44 IMPs per fire. Applied to
the shipped `13` arm, all eight DD/PD on/off cells stay positive; the weakest
residual is **+1089.04 IMPs**. This bound proves the on/off ship, not the thin
`13`-against-`15` slice.

**Whether the rung ships is not in doubt. Which floor is.** The arms are nested
(`15 ⊂ 13`), so their difference *is* the 13–14 points slice in isolation:
1350 fired boards, **+201 IMPs plain / +127 PD (none)** and **+561 / +492
(both)**. Positive on all four readings — unlike the K–K slice, which was a
wash that changed sign between rounds — but small per fire (+0.15 and +0.42
IMPs/fired plain). `ab-nt-splinter` preserved aggregate summaries, not the
per-board dumps, so the slice's exact conditional variance and `t` statistic
cannot be reconstructed; importing K–K's spread would not supply that missing
evidence. So:

- `13` takes **more total IMPs** (+4413/+5807 plain vs `15`'s +4212/+5246),
  which is the KR1 metric;
- `15` takes **2.5-3× more per fire** and gives `3NT` up 62% less often.

Round 1 alone does not settle that, so it was replicated.

## Constructive round 2 — measured 2026-08-25, seed `1787662387`

Same four cells, fresh seed, 8M boards each, `ab-results/nt-minor-slam-r2/`.
**The two seeds are independent** — unlike the two vulnerability cells within a
seed, which share deals and must be read one at a time.

| arm | vul | fired | ΣIMP plain | /fired | ΣIMP PD | /fired |
| --- | --- | --- | --- | --- | --- | --- |
| `13` | none | 2142 | +4749 | +2.22 | +4637 | +2.16 |
| `13` | both | 2142 | +6224 | +2.91 | +6102 | +2.85 |
| `15` | none | 830 | +4258 | +5.13 | +4211 | +5.07 |
| `15` | both | 830 | +5319 | +6.41 | +5273 | +6.35 |

**Across both rounds: 16/16 DD+PD cells positive; 32/32 positive including
SD-lead** (2 seeds × 2 floors × 2 vulnerabilities × 2 scorers, repeated with
SD-lead). Round 2 is slightly *stronger* than round 1 in every DD+PD cell. As
above, SD-lead corroborates the sign but is not slam insurance or the basis of
the verdict.

### The floor: `13`, not `15` — the opposite of the K–K lane

The arms are nested, so their difference is the 13–14 points slice alone:

| seed | vul | n | plain | /fired | PD | /fired |
| --- | --- | --- | --- | --- | --- | --- |
| r1 | none | 1350 | +201 | +0.15 | +127 | +0.09 |
| r1 | both | 1350 | +561 | +0.42 | +492 | +0.36 |
| r2 | none | 1312 | +491 | +0.37 | +426 | +0.32 |
| r2 | both | 1312 | +905 | +0.69 | +829 | +0.63 |

**Eight readings, eight positive**, and pooling the two independent seeds gives
n = 2662 at **+0.26 IMPs/fired NV and +0.55 both-vul** (plain; +0.21/+0.50 PD).
The harness did not preserve per-board dumps, so it is impossible to recover an
exact conditional variance or `t` statistic for the `13`-versus-`15` slice.
The ship evidence is the KR1 aggregate itself: `13` takes more total IMPs than
`15` in all eight seed/vulnerability/scorer cells, and the slice never once
reads negative. No variance estimate borrowed from K–K is used.

**Why this lane answers differently from K–K, where `15` won.** There the `4m`
had to out-bid a *contested* `3NT` under pressure and every rung was bought
with the notrump game in a lane the opponents had already entered. Here the
auction is quiet: opener can decline to `5m` safely, and the 13-14 hand
opposite 15-17 is 28-31 combined, which is a minor-suit game hand often enough
to be worth the `3NT` it gives up. The floors are lane properties, not a
constant of the treatment — the same lesson the K–K round-2 write-up drew from
the other side.

**Shipped default-on at `Some(13)` on 2026-08-25.** The exact aggregate sums and
all eight positive `13`-versus-`15` slice signs select the floor; the missing
per-board dumps limit the precision claim, not the KR1 ship decision.

## C1 supported 54 extension — measured 2026-08-26 — SHIPPED

`ab-nt-splinter --minor-slam 13 --minor-slam-fit`, 8M boards per cell,
opponents silenced, two fresh seeds. Both arms kept the shipped 13-point slam
floor; control required six diamonds and treatment admitted the whole
`two_notrump_class()` only after opener's supported `3♦`. Run code was HEAD
`0b60dcaf` plus this worktree; results are in
`ab-results/nt-minor-slam-fit/`.

| seed | vul | fired | plain IMPs / fired (95% CI) | PD IMPs / fired (95% CI) |
| --- | --- | ---: | ---: | ---: |
| `1787678038` | none | 493 | +488 / **+0.99 ±0.71** | +492 / **+1.00 ±0.74** |
| `1787678038` | both | 493 | +688 / **+1.40 ±0.85** | +690 / **+1.40 ±0.89** |
| `1787678117` | none | 526 | +736 / **+1.40 ±0.69** | +705 / **+1.34 ±0.71** |
| `1787678117` | both | 526 | +991 / **+1.88 ±0.83** | +960 / **+1.83 ±0.85** |

All eight readings are positive and every conditional CI excludes zero: the
decision table's win/win row on both seeds and vulnerabilities. Each seed's
two vulnerability cells share deals and are read separately. The exact-new-fire
predicate excludes every pre-existing six-card `4m` try; fired and divergent
counts are equal in all four cells. Fired rates were **0.0061625%** in round 1
and **0.0065750%** in round 2. All-board plain/PD means and 95% CIs, in table
order, were **+0.000061 ± 0.000044 / +0.000062 ± 0.000046**,
**+0.000086 ± 0.000053 / +0.000086 ± 0.000055**,
**+0.000092 ± 0.000046 / +0.000088 ± 0.000047**, and
**+0.000124 ± 0.000056 / +0.000120 ± 0.000057 IMPs/board**.

The full divergent dumps also clear the slam-optimism gate. Applying the
maximal Pavlicek shave—6% of every positive small-slam IMP and 20% of every
positive grand-slam IMP—leaves the four plain/PD pairs at
**+379.5/+382.6**, **+555.8/+556.6**, **+605.5/+574.3**, and
**+832.0/+800.7 IMPs**, in table order. This is deliberately conservative:
it discounts every positive slam swing whether or not that board carried the
full DD-play bias.

The worst-board trace found the expected cost—minimum opener sometimes signs
off in `5♦` when `3NT` was better. The `5♦` bucket lost in all four cells
(plain/PD **−340/−311**, **−349/−315**, **−477/−485**, and
**−515/−521 IMPs**); gains from `6♦`, `6NT`, and `7NT` dominate those
signoff losses. No new continuation was authored: the existing
`4♦ -` / `4♦ (X)` answer and full RKCB ladder are keyed on the auction, not
responder's shape. The natural `4♦` rule's authored projection admits the
5♦4+♣ bidder, pinned by the end-to-end test. Dutch and the systems-on natural
1NT-overcall graft inherit the same constructive package; the explicit false
arm remains available.

**Verdict: default on.** `notrump.minor_transfer_slam_fit = true`; false
restores the measured six-card control.

## Landy round 1 — measured 2026-08-25, seed `1787662790` — WIN, thin — SHIPPED

`scripts/ab-landy-minor-slam.sh`, two arms (`base` = the former table, with the
`4m` rung but no answer; `ans` = the shipped authored answer), 2.304M retained
boards per arm per vulnerability, `--filter-1nt` on both. The stored run used
the then-positive candidate flag; the current runner expresses the same pair
as `base --no-ns-landy-minor-slam-answer` versus default `ans`. Run code was
base `4eb925c2` plus the campaign worktree.

**Isolation gate first, as the runner's header demands: PASSED in both cells** —
0 of 18 (NV) and 0 of 9 (both) divergent boards opened by the other side, 100%
"ours", no board where the candidate bid over a pass or passed over a bid. Every
divergence is a different call at the seat this arm authors, which is what a
one-seat arm should look like.

| vul | n | plain | /fired | PD | /fired |
| --- | --- | --- | --- | --- | --- |
| none | 18 | +100 | +5.56 | +128 | +7.11 |
| both | 9 | +91 | +10.11 | +103 | +11.44 |

The preserved arm dumps support a conditional one-sample test on the fired-set
IMP differences: NV `t = +2.68` plain and `+3.13` PD (`n = 18`); both-vul
`t = +5.48` plain and `+4.97` PD (`n = 9`). This is the relevant uncertainty
statement for the authored seat, rather than a rounded CI over millions of
non-firing filtered boards.

The control arm's 1NT filter scanned 12,153,068 raw deals per vulnerability to
retain 2.304M boards. Thus the divergences are rare in raw traffic — about one
per 675,000 raw deals NV and one per 1.35M both-vul — while their sums are about
+0.000008/+0.000011 IMP per raw deal NV and +0.000007/+0.000008 both-vul
(plain/PD). The density is context, not a substitute for the conditional test.

The raw dumps also support the exact slam-optimism gate. At Pavlicek's maximal
`q = 3%`, discounting only DD-making small-slam swings leaves
**+96.16/+123.98 IMPs NV and +88.48/+100.42 both-vul** (plain/PD). All four
on/off cells remain wins by wide margins.

SD-lead (16 worlds) fires on a slightly different set and agrees in sign on
both cells: NV n=28, +3.50 plain / +4.04 PD per fired; both-vul n=18, +5.83
plain / +6.22 PD. It is not slam insurance: the opening lead is fixed while
declarer remains double-dummy perfect, so this is an optimistic upper bound at
slam. The verdict rests on the positive plain-DD and PD cells and their
conditional evidence; SD-lead only corroborates the sign.

### The only loss mechanism: `hcp(16..)` is the wrong gate

Both-vul has **zero losing boards** (its five worst are +0, +3, +7, +12, +12).
NV has exactly three, all −11, and all one shape:

```
[-11] N:A5.AKJ8.QJ3.T532 KJT97.QT6432.6.K 8643.5.AK.AQJ984 Q2.97.T987542.76
  ans:  - 1NT 2♣ 2NT - 3♣ - 4♣ - 5♣ - - -
  base: - 1NT 2♣ 2NT - 3♣ - 4♣ - 4♥ - 4♠ - 5♣ - 6♣ - - -
```

Opener holds `A5.AKJ8.QJ3.T532` — **15 HCP, but two aces and `AKJ8`**. Our
answer reads 15 < 16 and signs off in `5♣`; the *floor*, left alone, cue-bids
`4♥` and reaches `6♣`, which makes. The other two −11s are the same board in
different clothes.

So the floor is not helpless above this rung after all — it cannot RKCB
(`undisturbed()` is false), but it can cue. What the answer does is **replace a
cue ladder that sometimes works with an HCP gate that sometimes doesn't**, and
it wins overall because the gains are larger and more frequent than the three
losses: `base`'s failure mode is `6NT` off a cashing suit, or a contract in a
major their Landy `2♣` advertised, and those cost more than a missed `6♣`.

**The fix this points at is not a floor sweep.** `13`-vs-`15`-style tuning
cannot separate these hands: the losers are *15-counts with two aces*, which any
HCP threshold either takes with the flat 15s or rejects with them. The gate
wants controls, not points. That is a new design question for the whole
`4m`-answer family (K–K's shipped `16` has the same shape), and it is filed in
the ledger below rather than started here.

**Shipped default-on.** `competition.landy_minor_slam_answer = true` closes the
previously unauthored seat after both `-` and `(X)`, including the complete RKCB
ladder in each tail.

## The constructive lane is worse, not milder — probed 2026-08-25

The campaign opened on the assumption that the constructive lanes were the mild
case: uncontested, `Context::undisturbed` holds, so `instinct`'s `4NT` keycard
ask is *available*, and N1's "leave the `4m` to the floor" doctrine ought to
work there even though it does not in a competitive lane. **Both halves of that
are false.** Two probes, both on `american()` at its shipped defaults:

**1. Responder's seat is fully shadowed — there is no slam try to leave to
anyone.** At `1NT - 2♠ - 3♣ -` (Puppet transfer to clubs, completed), holding
`32.K42.A2.AKQJ32` — 17 HCP, six clubs to the AKQJ, facing a 16–17 notrump, so
**33–34 combined** — the whole vocabulary is:

```
3NT  0.900  rule #4: (exactly 8 HCP, and balanced), or (6+ ♣, 8+ HCP, 2+ ♦, 2+ ♥, and 2+ ♠)
```

One call, from a book node whose gate is `8+ HCP`. A 17-count and an 8-count
take the same bid, and the floor is never consulted — `provenance` reads
`fallback: None`. This is the same shadowing the K–K lane had, one layer
deeper: there the ladder at least offered two-suiter steps.

**2. The floor would not rescue it even if the node were widened.** At
`1NT - 2♠ - 3♣ - 4♣ -`, opener holding `AQ32.KQ54.A4.K32`, the floor's entire
vocabulary is:

```
4♥  1.500
P   0.000
```

No `4NT`. It takes `4♥` on a balanced 16 facing a club slam try — the *same
wrong answer* the disturbed K–K lane gave, though `undisturbed()` is satisfied
here and the keycard ask is nominally available.

**So `undisturbed` was necessary but not sufficient — and the two gates in
§"The doctrine, and where it breaks" turn out to be one gate.** Uncontested,
partner's read at that seat is `hcp 0..37`: the `4♣` reads as **nothing**,
because no book node authors it. The ask carries `combined_points(29)` against
`own + partner's shown floor`, and an unread call shows a floor of **zero**, so
a 16-count sums to 16 and the ask is suppressed — not rationed, *absent*. The
reading gate and the points gate are the same gate, reached through the shown
floor. (The disturbed lane at least read `hcp 6..8` off the K–K transfer, and
still answered `4♥`, because `undisturbed` fails there independently.) This is
the "a call reads as nothing" failure of
[sampled-projection.md](ai-bidder/sampled-projection.md), reached from the
opposite direction.

The consequence for the rule this campaign shipped: **an unauthored `4m` is
not a slam try anywhere, contested or not.** The rung and its answer have to be
authored together in every lane — which all three shipped packages now do,
including their clean and `(X)` answer-plus-RKCB tails.

## The survey — 18 floored four-level seats, 2026-08-25

Queue item 4 of the original handoff asked for "a survey of every competitive
4-level rung with no authored answer". It was run: nine readers over the whole
competitive and defensive book, each finding adversarially refuted by an
independent verifier. **30 suspects, 24 survived, 18 distinct seats.** Every
one of them is a call that invites a continuation, sitting at a node with no
registered answer — `Provenance { depth: 0, fallback: Some(0) }`, the root
floor.

| the unanswered seat | table that offers the call | gate |
| --- | --- | --- |
| `1NT (2♣) 2♥ - 2NT - 3♠ - 4♣ -` | [`landy_bba_ask_answer`](../src/bidding/american/competition/lebensohl.rs) | `len(minor, 4..)` |
| `1NT (2♣) 2♥ - 3♣ - 4♣ -` | [`landy_bba_pick_rebid`](../src/bidding/american/competition/lebensohl.rs) | `points(14..)` |
| `1NT (2♣) 2♥ - 2NT - 4♣ -` | [`landy_bba_takeout_rebid`](../src/bidding/american/competition/lebensohl.rs) | `points(14..) & len(minor, 5..)` |
| `1NT (2♣) 3♥ - 4♣ -` | [`landy_bba_takeout_answer`](../src/bidding/american/competition/lebensohl.rs) | `len(minor, 4..)` |
| `1NT (2♣) 2NT - 3♣ - 3♥ - 4♣ -` | [`landy_recue_answer`](../src/bidding/american/competition/lebensohl.rs) | `hcp(0..)` |
| `1NT (2♣) 2♥ (3♠) 4♣ -` | [`landy_bba_takeout_overcalled`](../src/bidding/american/competition/lebensohl.rs) | `len(minor, 4..)` |
| `1NT (2♣) 2♥ - 4♣ -` | [`landy_cue_answer`](../src/bidding/american/competition/lebensohl.rs) | `tolerance.clone() & hcp(max..)` |
| `1NT (2♣) 2♥ - 3♥ - 4♣ -` | [`landy_ask_answer`](../src/bidding/american/competition/lebensohl.rs) | `hcp(0..)` |
| `1NT (2♣) 3♣ - 4♣ -` | [`landy_minor_answer`](../src/bidding/american/competition/lebensohl.rs) | `len(minor, 3..)` |
| `1NT (2♣) 2♥ - 2♠ - 4♣ -` | [`landy_ask_answer`](../src/bidding/american/competition/lebensohl.rs) | `points(10..)` |
| `1NT (2♣) 2♥ - 3♣ - 3♠ - 4♣ -` | [`landy_recue_answer`](../src/bidding/american/competition/lebensohl.rs) | `hcp(0..)` |
| `1NT (2♦) 4♥ (X)` | [`kokish_kraft_responder`](../src/bidding/american/competition/rubensohl.rs) | `len(major, 6..) & len(other, ..5) & hcp(15.…` |
| `1NT (2♦) 4♣ - 4♦ - 4♥ -` | [`lm_2d_clubs_major`](../src/bidding/american/competition/rubensohl.rs) | `len(Suit::Hearts, 5..)` |
| `1NT (2♦) 4♣ - 4♦ - 4♠ -` | [`lm_2d_clubs_major`](../src/bidding/american/competition/rubensohl.rs) | `len(Suit::Spades, 5..)` |
| `(2♥) X - 3♥ - 4♣ -` | [`cue_stayman_answer_no_stopper`](../src/bidding/american/competition/rubensohl.rs) | `len(minor, 4..) & min_level_is(4, m)` |
| `1NT (3♣) 3♠ (4♣) 4♦ -` | [`nt_3c_transfer_squeezed`](../src/bidding/american/competition/nt_high_overcall.rs) | `len(target, 3..)` |
| `1♣ (1♠) 2♥ - 4♦ -` | [`free_transfer_completion`](../src/bidding/american/competition/free_bids.rs) | `len(shown, 4..) & points(15..)` |
| `1♣ (2♦) 2♠ - 3♥ - 4♦ -` | [`free_transfer_clarify`](../src/bidding/american/competition/free_bids.rs) | `points(13..)` |

**Eleven of the eighteen are the Landy `(2♣)` family** — the N1j arm and the
legacy cue stack both — which makes this one defect with eleven exits, not
eleven defects. Its signature is a doc comment: *"opener's continuation is
deliberately the floor's"*, *"the floor continues"*, *"the floor takes over"*,
*"the floor drives on from there"*. That sentence is the N1 slam-exploration
doctrine, and §"The doctrine, and where it breaks" above is why it does not
hold. Every place it was written down is a place to look.

Two spot-checks run by hand rather than by agent, both in the **default** N1j
arm, both `PROBE_THEIR_2C_LANDY=1`:

- `1NT (2♣) 2♥ - 3♣ - 4♣ -` (opener's slam-zone rung, `points(14..)`) —
  `depth: 0, fallback: Some(0)`, vocabulary `{6NT 1.600, 4♥ 1.500, Pass}`.
  The floor takes `4♥`.
- `1NT (2♣) 2♥ - 2NT - 3♠ - 4♣ -` — same root floor, and the top two calls are
  `4♥ 7.784` and `4♠ 7.526`: **both of the majors their Landy `2♣`
  advertised**, bid by a side that has just been told about them.

The verifiers flagged that a few rows sit in non-default arms (the legacy
`defense_2c_landy_*` stack is superseded by N1j) and that one — `1NT (2♣) 2♥ -
2♠ - 4♣ -` — is in a table the shipped system does not register at all. Those
are recorded as-found rather than pruned, because the census in this document
already treats a wired-but-dormant arm as in scope and exonerates lanes on
substantive grounds, not on arm-ness.

### What the survey does not settle

**Whether to fix these one rung at a time, or fix the gate.** Every one of the
eighteen is floored for the same reason, and there are two shapes of repair:

1. **An authored answer per rung** — what this campaign has done three times
   now (K–K, Landy's transfer lane and the constructive lane all shipped).
   Safe, measurable one at a time, and eighteen times as much work.
2. **Relax `Context::undisturbed` on the floor's `4NT` ask.** One change
   covering every lane at once — but it is a *floor* change under a learned
   regime, so it needs its own non-inferiority proof, and it does not fix the
   second gate (an unauthored call still shows a floor of zero, so
   `combined_points(29)` still cannot be met). It would have to move with a
   reading change.

jdh8 left both unstarted for this ship. **Neither is started.**

## Decided — do not re-litigate

- **Not a ceiling on the transfer.** Residue 3's stated alternative ("transfer
  below, `X` above") pushes the strong long minor into the values double, whose
  reading is already the looser one (§N4-KK residue 2), and it spends the
  right-siding the wide transfer was built to buy — the measured N1h/N1i trade
  (`3♣ ← 2NT`, **−2.19 PD**) bought right-siding by *deleting* the invitational
  rungs, so re-imposing a band boundary above the transfer runs it backwards.
- **Not `5m`.** Eleven tricks against ten, and the floor cannot cue-bid below a
  contract it is handed. `4m` is the cheapest call that is still a suit contract.
- **Not floored.** Opener's answer is authored. See the doctrine section — this
  is the one place the campaign departs from N1, and it departs on a probe.

## Residues — open, and flagged

**Session boundary, 2026-08-25.** jdh8 stopped authoring here — *"the blast
radius looks large for constructive bidding; document them instead of dealing
with them in this session."* C1 was resumed and shipped the next day. The
still-open rows below are **documented, not started**. C1, C3 and P1 are
retained as closed ship-decision history; every other row remains open with its
proposed reversible default unchanged.

| # | item | lane | blast radius | proposed default |
| --- | --- | --- | --- | --- |
| C1 | **CLOSED — the supported `2NT` class's exactly-5♦, 4+♣ member can slam-try** | constructive | one rule's shape gate, one node | **SHIPPED `minor_transfer_slam_fit = true`** |
| C2 | the splinter branch is still slamless | constructive | a new rung **plus** a size channel through the shipped splinter lane | leave |
| C3 | **CLOSED — ship `minor_transfer_slam_try`** | constructive | **moves the default constructive system** | **SHIPPED `Some(13)`** |
| C4 | `two_spade_over_min`/`two_spade_over_max` have no finite catch-all | constructive | invariant break, benign today, propagates by copy-paste | leave, record |
| C5 | `size_ask_accept_floor` is 16 uncontested vs a hardcoded 17 contested | constructive | one constant, two sites | tighten the doc comment; file the `17` as a sweep candidate |
| P1 | **CLOSED — ship `landy_minor_slam_answer`** | competitive | one seat in one contested lane | **SHIPPED `true`**; gate read 0 foreign |
| P2 | the 18 floored four-level seats, and the fork below | competitive | 18 rungs, **or** one floor-gate change under a learned regime | neither started |
| P3 | Landy's `{completed} 4m (4M)` — their jump over the slam try | competitive | one tail | leave, record |
| P4 | K–K's `{completed} (4M)` — their jump over the completion | competitive | one tail | leave, record |
| P5 | `nt_overcall_systems_on` grafts the Puppet trie below `(1x) 1NT`, where `over_our_minor_transfer` cannot see it | both | a prefix generalization | leave; **but see the checked note below** |
| P6 | the `4m` answer's accept gate counts **HCP, not controls** — the whole family (K–K's `16`, Landy's `16`, the constructive `size_ask_accept_floor`, 16) | all | a shared design change across three shipped answers | leave, record; the Landy round-1 losers are its evidence |

**P5, checked 2026-08-25 — the new rung is fine there.** The concern was that
the graft would carry the `4m` rung into `(1x) 1NT` *without* its answer,
manufacturing a fresh floored seat of exactly the kind this campaign exists to
close. It does not. Probed with the knob armed: `(1♦) 1NT - 2♠ - 3♣ -` offers
`4♣ 0.950` at `depth: 7, fallback: None`, and `(1♦) 1NT - 2♠ - 3♣ - 4♣ -`
answers `4NT 1.000` at `depth: 9, fallback: None`. Rung and answer graft
together. P5 remains open for what it originally named —
`over_our_minor_transfer` keying `P* 1NT - 2♠` and so missing the grafted seat
when *they* compete — which is untouched by this campaign.

### The details

- **Which constructive floor.** Settled for this ship at `13`: it beat `15` on
  total IMPs in all eight exact DD+PD slice aggregates across the two seeds.
  The missing per-board dumps prevent an exact conditional `t`, as recorded
  above, but do not change those sums.
- **C1 — the `2NT` class's supported exactly-5♦, 4+♣ member now slam-tries.** The
  pre-change witness `K2.A3.AKQ32.AQ32` had only `3NT 0.900` at
  `1NT - 2NT - 3♦ -`; with the measured default it bids `4♦ 0.950`. The same
  hand still bids `3NT` after opener's `3♣` denial. See the C1 round above.
- **The splinter branch is still slamless, in both constructive lanes.** The
  `4m` rung sits *under* the splinters by design, so a slam-values hand holding
  a shortness splinters — and then
  [`pick_game_over_club_splinter`](../src/bidding/american/notrump/minor_transfers.rs)
  places `3NT` or `5m` and the auction is over. Responder showed `8+` and is
  unlimited above that, so opener cannot tell 8 from 18 and the *better* slam
  shape is the one still stranded. Fixing it is a second rung with its own
  gate (opener needs a size channel over the splinter, or responder needs a way
  back in over the placement), so it is not this arm's. *Proposed default:
  leave, record; the `4m` arm has shipped, but this remains a separate rung and
  size-channel experiment.*
- **Their jump over the Landy `4m` is unauthored** — `landy_minor_slam_answer`
  keys `{completed} 4m -` and `{completed} 4m (X)` only, so `(4♥)`/`(4♠)` over
  the slam try drops to the floor. Exactly the K–K residue below, one lane
  over, and recorded for the same reason. *Proposed default: leave, record.*
- **`probe-decision` had no channel for `their.two_clubs_landy` — FIXED
  2026-08-25.** Until this session every N1j forensic silently read their `2♣`
  as *natural*: the whole Landy table sat inert and the seat printed
  `fallback: Some(0)`, which reads exactly like "the floor owns this" and is
  not. Same class of hole as the `PROBE_THEIR_2D_MULTI` one the N4 campaign
  found, and it means N1j probe results recorded before this date are suspect
  unless they named the disclosure. The knob is now `PROBE_THEIR_2C_LANDY`.
- **`{completed} (4M)` is unauthored.** `kokish_kraft_transfer_overcalled` is
  keyed to `(3♥)`/`(3♠)` only, so their *jump* over the completion drops to the
  floor. Not in this arm; recorded.
- **`two_spade_over_min` / `two_spade_over_max` have no finite catch-all** —
  every rule requires `len(♣,6..)` or `hcp(8..=8) & balanced()`, and
  `A3.A32.K43.AQJ76` gets zero candidate calls. Benign today (such a hand could
  not have bid `2♠`) but it breaks the invariant, and anything copying the shape
  propagates it. *Proposed default: leave, record; adding a rung is a bidding
  change.*
- **`size_ask_accept_floor` is `16` uncontested and hardcoded `17` contested**
  (`over_our_minor_transfer.rs`). *Proposed default: tighten the doc comment,
  file the contested `17` as a sweep candidate; moving either is an A/B.*
- **A default-on lane is missing from the census.** `nt_overcall_systems_on`
  grafts the whole 1NT response trie below `(1x) 1NT`, carrying the Puppet minor
  transfers into a seat `over_our_minor_transfer` cannot see (it keys
  `P* 1NT - 2♠`). `(1♦) 1NT - 2NT (X)` is entirely floored. A prefix
  generalization, not new theory — arguably cheaper than any rung here.
