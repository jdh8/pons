# Minor-suit transfers — the missing slam channel

**Every lane in which responder transfers to a minor tops out at `3NT`, or at a
`5m` opener placed unasked.** The engine's one slam channel above a completed
minor transfer is a single `4m` call in the Landy counter — and that call has no
authored answer, so the seat it creates belongs to the floor.

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

Every lane spent those rungs on **shape** (second suits, stopper cues,
splinters) or on **placement** (`3NT`, `5m`). None spent one on **size**: the
only strength boundary any of them has is the game line. The transfer is
therefore wide at the bottom — a weak sign-off rides it — and wide at the top —
a 21-count rides it too — and after the completion nothing tells them apart.

The floor cannot rescue the seat. A rebid table with a finite catch-all
**shadows** the floor ([bidding-architecture.md](bidding-architecture.md)), and
every table below ends in `Pass, 0, hcp(0..)` or `3NT, 100, hcp(0..)`. A call the
table does not spell sits at `NEG_INFINITY` and cannot be made at all.

## The census (2026-08-25)

| lane | the transfer | band | above the completion | top of the ladder |
| --- | --- | --- | --- | --- |
| Constructive Puppet (default) | `1NT - 2♠` (→♣) / `1NT - 2NT` (→♦) | none / none; the game boundary is a hardcoded `8` at every site | splinter into the shortness, else `3NT` | opener places `3NT` / **`5m`**, total — [`pick_game_over_diamond_splinter`](../src/bidding/american/notrump/minor_transfers.rs) |
| Constructive European (opt-in) | `2♠` (→♣) / `3♣` (→♦) | none / none | the same `8` | **`3NT` in both minors** — no splinter arm, no `5m` at all |
| N1j Landy `(2♣)` (default on) | `2NT` (→♣) / `3♣` (→♦), `len 6.. & points(2..)` | 2 / none | stopper cue `3♥`/`3♠` (10+), **`4m` (13+, six)**, `3NT` (10+), Pass | **`4m`**, and then the floor — [`landy_bba_transfer_rebid`](../src/bidding/american/competition/lebensohl.rs) |
| N1c legacy Landy stack (arm) | `2NT` (→♣) only, `points(2..=9)` | 2 / **9 — capped** | terminal | `3♣`, forced pass |
| N4-KK `(2♦)` Multi (default on) | `2NT` (→♣) / `3♣` (→♦), `len 6..` and **no point term** | floorless / none | two-suiter steps (10+), `3NT` (10+), Pass | **`3NT`** — [`kokish_kraft_transfer_rebid`](../src/bidding/american/competition/rubensohl.rs) |
| N4-KK, they compete over it | same transfer | — | `3NT` (10+ with a stopper), `X` (`hcp 10+`), Pass | **`3NT`**, or their partscore doubled |
| N3 `(3♣)` transfer variant (opt-in) | `3♠` (→♦), `points(10..)` | 10 (GF) / none | **no transferor-rebid node at all** — the seat is *floored*, not shadowed | `3NT`, else `5♦` |
| Rubensohl `(2♥)`/`(2♠)` (default) | `3♣` (→♦), top step (→♣) | 9 / 10 | **no transferor-rebid node at all** | `3♦` (a partscore) or `3NT` |
| Gladiator, after our 1NT overcall (opt-in) | `2NT` (→♣) | **`points(..inv)` — capped** | — | `3♣` sign-off |

The two lanes that are **not** defective are the two capped ones (N1c,
Gladiator): nothing strong ever transfers, so nothing is stranded. N3 and
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
3. **Wide, with nothing above `3NT`** — N4-KK and both constructive lanes.

Only (3) is a defect: a hand arrives in a seat where its values have no call.
This campaign picks (2).

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

## The rule this campaign proposes

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
     is opener's: `4NT` RKCB on `hcp(16..)`, else `5m`, plus `slam::rkcb_rows`.
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
2. **Port the winner back to Landy.** jdh8's call, and it is a *fix*, not a
   copy: `landy_bba_transfer_rebid`'s `4m` is shipped default-on today with **no
   authored answer**, which is the same floored seat this campaign found, one
   lane over. Owed its own seed.
3. **Constructive Puppet and European minors — PROBED 2026-08-25, and the
   defect is real.** Both lanes decline slam by design and say so — "the lane
   places games, it is not a slam try", and
   [`club_no_shortness`](../src/bidding/american/notrump/minor_transfers.rs) is
   named "game-going, slamless". The probe that item asked for has now been run;
   see §"The constructive lane is worse, not milder" below. The "cost is
   smallest here" argument is **withdrawn** — see the corrected escape-hatch note
   above — and so is "probe first": the probe is done, what is owed now is the
   design and its A/B.
4. **N3 and Rubensohl.** A different defect (floored, not shadowed). Out of
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

**What this does *not* settle:** whether Landy's `13` should also become `15`.
The two tables differ exactly where the payload was made a payload — Landy's
rung is skimmed by stopper cues at w150/149 that K–K does not have — so the
number does not transplant back by symmetry. Queue item 2 owns it, on its own
seed.

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

The consequence for the rule this campaign proposes: **an unauthored `4m` is
not a slam try anywhere, contested or not.** The rung and its answer have to be
authored together in every lane — which is what the N4-KK build did, and what
the Landy port and the constructive design both still owe.

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

## Open, and flagged

- **Which floor.** That is the A/B's third arm, not a decision to take in
  advance. `13` is Landy's; `15` only bypasses `3NT` at 28-30 combined.
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
