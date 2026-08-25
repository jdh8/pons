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
  partner's shown floor`, so a `13` floor lets only a 16-17 opener ask.

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

1. **N4-KK — BUILT 2026-08-25, A/B pending.**
   `competition.multi_minor_slam_try`, a `points` floor rather than a bool
   (`None` = off). It authors residues 3 and 6 together, because the second is
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
3. **Constructive Puppet and European minors.** Both decline slam by design and
   say so — "the lane places games, it is not a slam try", and
   [`club_no_shortness`](../src/bidding/american/notrump/minor_transfers.rs) is
   named "game-going, slamless". Re-open after (1) and (2) measure, with a probe
   first. The "cost is smallest here" argument is **withdrawn** — see the
   corrected escape-hatch note above.
4. **N3 and Rubensohl.** A different defect (floored, not shadowed). Out of
   scope for this campaign; recorded so nobody folds them in.

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
