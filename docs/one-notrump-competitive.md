# The competitive-1NT campaign

Our 1NT opening when the opponents come in — both lanes: `1NT (X/2x)` (they
interfere directly) and `1NT - resp (..)` (they interfere after our response).
Read [bidding-architecture.md](bidding-architecture.md) first; every package
here ships (or doesn't) by [measurement.md](measurement.md).

Distinct from [competitive-book.md](competitive-book.md), whose P1–P6 are suit
openings: this lane has a different **opponent model** (their overcalls are
artificial) and a different **book/floor partition** (the floor owns the whole
`(X)` runout and every deep continuation).

## Opponent model — and the assumption we accept

BBA/EPBot, the anchor opponent, defends a 1NT opening with Woolsey
"Multi-Landy". **Every call is artificial**
(`reference_bba-1nt-defense`, distilled by sample-and-probe; BEN is a distilled
BBA-8730 and plays the same family):

| Their call | Woolsey Multi-Landy | Our book treats it as |
| --- | --- | --- |
| `X` | 4-card major **+ a longer (5–6) minor**, 12–19. **No penalty double exists** — a flat 22-count passes | a double to run from (the floor's runout) |
| `2♣` | **both majors**, ≥ 5-4 | natural clubs → **full systems-on rebase** |
| `2♦` | **Multi** — a 6+ *single* major, symmetric ♥/♠ | natural diamonds → Transfer Lebensohl's `(2♦)` leg |
| `2♥` / `2♠` | **Muiderberg** — exactly 5 in the major + a 4+ minor | natural 5+ major → Cohen Transfer Lebensohl |
| natural 2m | **never bid** — a 6-card minor one-suiter passes | — |

The mirror of that table — what BBA does as the **1NT opener** facing each of
those calls — is distilled in
[bba-1nt-counter-defense.md](ai-bidder/bba-1nt-counter-defense.md). Its two
load-bearing readings for this campaign: over their Landy `2♣` BBA plays a
**notrump ladder with no double and only the minor transfers**, and over
Muiderberg it plays **plain Lebensohl** with a takeout `X` showing the other
major. It routes on the opponents' declared system only in the `2♣` lane
(59.4% of the call distribution moves; every other lane ≤ 8%).

**Stated assumption.** We optimize for an artificial-defense field and accept
being wrong against a DONT/Cappelletti `2♦` that holds real diamonds. Routing
by the opponents' declared system is the `--declare-their-book` channel, which
measured plain-wash / **PD −0.0070** and is default-off; its own post-mortem
says the misreading was *"never the binding constraint — the calls it affects
are ones whose continuations are thin; reopening means authoring those
continuations first."* That is this campaign, so the reading channel stays out
of it until the calls are authored.

## The census — what each interference call actually costs

`examples/probe-1nt-interference` splits the 1NT slice of the anchor's
`Competitive / book / round-1` bucket by RHO's call. It reads an existing
anchor arm off the deal-keyed DD cache — seconds, no generation, no new solve:

```bash
cargo run --release --features serde --example probe-1nt-interference -- \
    ab-results/anchor-confirm/2026-08-21-1e9a47e2/american-none \
    --dd-cache ab-results/anchor-confirm/dd-cache.json
# --bucket "2♣" --show 8       the worst boards of one bucket
# --bucket "2♠" --responses 8  that bucket by our response, and by hand class
# --bucket "2♦" --responses 6  the N4e decomposition (§N4e); 6 keeps the 6-board rows
```

Bucket the **shipped** arm (`american-*`, the v6 floor), not the
`american-instinct` reference arm.

**Attribution ceiling — read this before quoting a number.** The swing is the
*board's* IMPs, not the interference decision's. On these deals the same 1NT
hand opens at table B too, so the mirrored board carries our *defense to their
1NT* as well, and every later call is in there. The buckets **rank**; they do
not isolate. Isolation is a package's own A/B (`bba-gen --filter-1nt`, one
knob). The confound is broadly common across buckets, which is what leaves the
ranking usable.

### Current — arms `1e9a47e2`, seed 1787064872, 204,800 boards/vulnerability (2026-08-21)

`ab-results/anchor-confirm/2026-08-21-1e9a47e2/american-{none,both}`, the
shipping system, replaying 100.00% of our calls with 0 mismatches; whole-arm
plain **−0.4819 NV / −0.5872 vul** IMPs/board against BBA. We open 1NT on
**6.33%/6.53%** of boards and RHO contests **12.9%/10.8%** of those, so a
contested 1NT is **0.82%/0.70% of all boards**. The three-level suits are split
per RHO suit; `4+` is `3NT` and everything above it, still one floor-only
bucket.

| RHO | boards (NV+vul) | plain total | plain/bd | PD/bd NV | PD/bd vul |
| --- | ---: | ---: | ---: | ---: | ---: |
| **`2♦` Multi** | 816 | **−744** | −0.91 | +0.01 | −0.24 |
| **`3♣` preempt** | 140 | −152 | −1.09 | −1.00 | −0.94 |
| **`3♠` preempt** | 72 | −107 | **−1.49** | −0.91 | −1.11 |
| `2♣` Landy | 573 | −102 | −0.18 | +0.67 | +0.95 |
| `2♥` Muiderberg | 409 | −99 | −0.24 | +0.20 | +0.84 |
| `2♠` Muiderberg | 402 | −90 | −0.22 | +0.18 | +0.48 |
| **`4+`** (`3NT` and up) | 38 | −43 | −1.13 | −1.17 | −1.20 |
| `2NT` unusual | 131 | −26 | −0.20 | −0.12 | +0.53 |
| **`3♥` preempt** | 105 | +7 | +0.07 | −0.11 | +1.02 |
| **`3♦` preempt** | 93 | +60 | +0.65 | +0.32 | +1.46 |
| `X` Woolsey | 336 | **+103** | +0.31 | +0.94 | +1.79 |
| **all contested** | 3115 | −1193 | −0.52 / −0.22 | +0.20 | +0.51 |
| **uncontested 1NT** | 23223 | — | **+0.12 / −0.02** | — | — |

**Three findings.**

1. **The lane's whole headroom is ~0.003 IMPs/bd.** Contested costs −0.65 NV /
   −0.20 vul relative to *uncontested*, on 0.82%/0.70% of boards — 0.0053 NV /
   0.0014 vul per board of the arm. Against a −0.48/−0.59 gap to BBA nothing
   here closes an anchor bucket; this is hygiene and disaster removal at the
   standard ship gate, as scoped.
2. **Contested 1NT is not a leak.** −0.52/−0.22 against the arm's own
   −0.48/−0.59: a shade below average NV, well above it vulnerable. Uncontested
   1NT (+0.12/−0.02) is the better board either way, and the 1NT opening stays
   one of our better boards even when contested.
3. **`(2♦)` Multi is now the lane's whole loss.** At **−744 plain on 816
   boards** it is five times the next bucket's total and PD-negative
   vulnerable — and N4's v7 counter and its reader already shipped against it,
   so nothing in the queue below owns it. Everything N3 and N1 touched has
   moved: the four three-level suits pool to **410 bd / −192 plain / −73 PD**
   (paired against the same seed's post-ship baseline of −273 / −186 —
   [§N3](#measurement--eight-rounds-archived)), `X` is now **+103 plain,
   +0.31/bd, PD +0.94/+1.79** where the pre-N3 snapshot had −183 plain,
   −0.50/bd, PD +0.51/+0.74 — the plain half crossed, the PD half was already
   positive — and `2♣` is −0.18 per board. Per *board* the worst cells are
   still the rare ones — `3♠` −1.49, `4+` −1.13, `3♣` −1.09 — and inside `4+`
   the floor still offers no `X` at all. Buckets from different seeds are not subtractable; the
   pre-N3 figures are scale, not deltas.

Superseded snapshots — the 2026-08-18 pre-N3 baseline that selected N3, and the
pre-N1 forensic on why the systems-on rebase lost to their Landy `2♣` — are in
[the archive's census history](archive/one-notrump-competitive-closed.md#census-history).

### Current `2NT` reading — small-n wash

The bucket is now **131 boards, −26 plain (−0.20/bd)**, PD −0.12/+0.53 NV/vul,
on per-board CIs of ±1.36/±1.70 that swallow it whole; the signs still
disagree. The starting snapshot's forensic pattern was that BBA **doubled**
their minors and we bid on:

```text
us:  1NT 2NT X 3♦ - - -
bba: 1NT 2NT X 3♦ - - X - - -
```

Two re-anchors later there is still no replicated loss — `53a3c254` read
+5 plain on 118 boards, this one −26 on 131, both well inside their CIs —
so N6 stays parked.

## Coverage inventory

### Lane 1 — `1NT (X/2x)`, they interfere directly

| RHO | Owner | Anchor |
| --- | --- | --- |
| `(X)` | **floor, complete** — escape, business XX, 2NT scramble, SOS, balancing runout, encircling doubles | `instinct.rs:4641-4956`; `set_one_nt_runout` default-on, +0.039/+0.053 plain, 1.58% fired |
| `(2♣)` | systems-on rebase + stolen-Stayman `X` | `lebensohl.rs:388-405`, `:416-425`, `:436-446` |
| `(2♦)` | Transfer Lebensohl's Stayman/Smolen/Jacoby/Leaping-Michaels leg | `lebensohl.rs:450-462`, `rubensohl.rs:332`; continuations `lebensohl.rs:584-608` |
| `(2♥)`, `(2♠)` | Cohen Transfer Lebensohl | `lebensohl.rs:466`, `rubensohl.rs:98`; continuations `:550-566`, `:573-578` |
| `(2NT)` | Unusual-vs-Unusual | `uvu.rs:139`, `:21`, `:145-161` |
| `(3♣)`–`(3♠)` | **book, complete** — N3's forcing three-level suit / `4M` / takeout `X` / `3NT` table and opener's one answer to each (`nt_high_overcall_responses`, **shipped default-on 2026-08-18**); the `(3♣)` transfer variant rides `nt_3c_transfers` (opt-in) | `nt_high_overcall.rs` |
| `3NT` and up | **floor** — `high_overcall_responses` covers suit openings only | `high_overcall.rs:152` |

The Multi counter is [§N4](#n4--their-2-as-a-multi-shipped-2026-08-15--v7-seven-rounds-default-on-vs-bba-via-the-census)
(`their.two_diamonds_multi`, **shipped 2026-08-15** — engine undeclared, `bba-gen`
derives it from the census like `their_2c_landy`). Its predecessor
`competition.defense_2d_multi` + `multi_responder` were **deleted 2026-08-15**:
never measured, and half-built — the continuation block fired on
`style == Transfer && over == Diamonds` **without checking the Multi flag**, so
with the knob on opener answered a natural `3♦` with
`transfer_completion(Hearts, ♦)` (same mismatch for `Plain` + Multi). N4 gates
the whole leg on the disclosure, either/or with the natural one.

### Lane 2 — `1NT - resp (..)`, they interfere after our response

Four packages, all keyed `P* 1NT - <our call>` and installed at
`competition.rs:304-318`:

| Our response | Coverage | Knob (default) |
| --- | --- | --- |
| `2♣` Stayman | authored; **2-level overcalls only** — `(2NT)`, `(3♣)`, `(3♦)`+ absent. Has the double-of-opener's-answer node | `competition_over_stayman` (**on**) |
| `2♦`/`2♥` Jacoby | partial: missing `(2♥)` over `2♦` and `(2♠)` over `2♥` — *they bid the major we are transferring to* ([over_our_jacoby.rs:100-103](../src/bidding/american/competition/over_our_jacoby.rs)) — and `(3♥)`/`(3♠)` | `competition_over_transfer` (**off**, measured loss) |
| `2♠` two-way minor | authored `(2NT)`–`(3♠)` | `competition_over_minor_transfer` (**on**, PUPPET-gated) |
| `2NT` ♦ transfer | authored `(3♣)`–`(3♠)` | `competition_over_diamond_transfer` (**on**, PUPPET-gated) |
| `3♣` Puppet, `3♦` both majors, `3♥/3♠` splinter, `3NT`, Texas `4♣/4♦`, direct `4M`, quantitative `4NT` | **absent** — floor | — |

Also absent everywhere but Stayman: the double of opener's answer
(`1NT - 2♦ - 2♥ (X)`), and *any* overcall of opener's answer.

## The book/floor line

**The book owns responder's first contested call and opener's one answer.
Everything deeper is floor, permanently.** "Complete" never means depth — a
book node with finite mass shadows the floor, and the floor is where deep
contested continuations get smarter. The deepest authored suffix in this lane
is 5 calls; the most common runout point is after opener's transfer completion
(`1NT (2♥) 3♠ - 4♠ -`), and it stays there.

## Package queue — open work, ranked by the census

Shipped packages have left this table; they are represented by the coverage
inventory above and by the [ledger](#ledger). The census's top bucket, `(2♦)`
at −744 plain, was re-decomposed on 2026-08-21; its one open item, **N4e**,
**shipped default-on 2026-08-22** and has moved to the [ledger](#ledger)
([§N4e](#n4e--the-floorless-weak-escape-shipped-default-on-2026-08-22-the-six-card-rung-five-refuted)).
N4's v7 counter and its reader had shipped against the *strong* half of that
bucket and its residue was measured out; N4e was responder's sub-5-HCP outlets,
never authored at all. The bucket has no open item again.

| # | Package | Knob | State |
| --- | --- | --- | --- |
| **N3-x** | **`X` over their `(4x)`** — the floor cannot double above the three level at all (`their_live_bid_at_most(3)`, [instinct.rs:6058](../src/bidding/instinct.rs)), and BBA's advancer sits for our double on **96.7–99.9%** of hands | new (book) | Current `4+` bucket 38 bd / −43 plain / −45 PD, −1.13/bd on a CI that swallows the total. The `(3x)` template does **not** widen — their four-level overcalls are *eight*-card suits, six times rarer than the three-level rows — so this is an uncontested opportunity to size, not a copy |
| **N2d** | relay with a 6+ suit below 6 HCP, over `(2♠)` only (the weak major has no two-level call there) | book | **Re-read 2026-08-21: 25 bd, −77 plain / −52 PD, −3.08/bd** — still the worst hand class in the lane, and negative on both scorers. Contradicts the PD-distilled floor ([`lebensohl_relay_shape`](../src/bidding/american/competition/lebensohl.rs)); against Muiderberg the alternative is a making `2♠`, and BBA at table B bids these hands un-overcalled. Needs the A/B, not a re-derivation |
| N5 | Complete Jacoby, then re-measure | `competition_over_transfer` | default-off on a measured loss *while missing its two most-fired cells* — `(2♥)` over our `2♦` and `(2♠)` over our `2♥`, i.e. they bid the major we are transferring to ([over_our_jacoby.rs:100-103](../src/bidding/american/competition/over_our_jacoby.rs)) — a half-built loss, resumable |
| N3-fit | fresh-seed confirmation of the `4M` fit rung | `nt_high_overcall_x_major_at_four` (**on**) | Round 5 shipped it on one seed (+1.14/+1.69 per fired, 0 foreign) and explicitly owed a confirmation. Rounds 7–8 ran against a `base` that *carries* it, which is not an isolated confirmation — the row is shipped but not settled |
| N6 | `(2NT)` penalty discipline | `uvu_encircle` et al. | 131 bd, −26 plain, CI ±1.36/±1.70 per board and the signs disagree by vulnerability — no replicated loss on either re-anchor since the starting snapshot. Mechanism stays priced: **BBA doubles 46.7%** and cues `3♣` for both majors ([reference](ai-bidder/bba-1nt-counter-defense.md)). Parked |
| N3-three | single-dummy re-measure of the refuted honor half | `nt_high_overcall_x_leave_in_three` (**off**) | Round 7 refuted it at sd-lead −2.44/−2.99 per fired; its `plain wash \| PD win` DD signature is the doubling artifact. Kept opt-in as a house-rule re-measure candidate on its vulnerable PD reading |
| N3-xfer | re-measure the `(3♣)` transfers | `nt_3c_transfers` (**off**) | Two seeds at round 2 and two more at round 3, all four pooled cells positive and all four an order of magnitude inside their CI. What it buys — the invitational five-card major, and right-siding — is DD-blind, so this is a single-dummy-harness item |
| N2c | the no-call 8–9 count with 0-1 / 4+ in their suit | book | **Re-read 2026-08-21: 19 bd, +11 plain / +91 PD.** The class that motivated it now reads *positive on both scorers*. Demoted to **parked pending replication**, not closed — n is small enough that either sign is seed noise |
| N7 | Absent responses contested | new | Puppet `3♣`, `3♦`, splinters, `3NT`, Texas, `4NT` — rarest in the system |
| N8 | Delete `1NT - 2NT - 3♣ - 3♦ -`'s `pass_out` node — the redundant half of the pair | knobless (book node) | **Inherited 2026-08-18** from the closed [authored-reading campaign](authored-reading-handoff.md)'s Phase 2 row. Probed 2026-08-17: with the node removed the auction falls to the root fallback and the floor offers **`P` alone** on every hand at both vulnerabilities, so the node buys nothing — but its twin `1NT - 2♠ - 2NT - 3♣ -` blasts **`4♣` 1.200 over `P` 0.000** (the floor's support-raise fires on partner's *sign-off*, never reading the `points 0..9` cap the same reading supplies). The pre-registered rule was "any hand still blasts → leave both", so both stayed. This is the deletion of the *redundant* one alone, and it owes its own arm; the twin's real repair is the floor-side settle rail ([dutch-system.md](dutch-system.md#the-wj-floor-campaign--bbas-polish-club-as-dutchs-teacher)), not a book node |
| — | reading drift after the leave-in: 11.8%/9.6% of divergences move a *later* call only, −0.56 DD / −0.63 sd per fired vulnerable | — | Owned by [reading-drift-handoff.md](reading-drift-handoff.md), not by this campaign. Pooled it is positive and threatens nothing; the vulnerable cost is real |

## N1 — the Landy `(2♣)` counter (**SHIPPED DEFAULT-ON 2026-08-15**)

N1 shipped on 2026-08-14; N1j's BBA-ladder table superseded its original
stack as the default on 2026-08-15. The current engagement and disclosure are:

| Setting | Shipped behaviour |
| --- | --- |
| `their.two_clubs_landy` | disclosure gate; declared Landy engages the counter, undeclared stays natural |
| `defense_2c_landy_bba` | **on**; the anchor-aligned table replaces the N1b–N1i stack |
| `defense_2c_landy_weak_2d_cap` | **on**; the natural `2♦` escape is capped at `hcp(..=6)` |
| `reading.their_landy_reading` | **on**; the floor reads their `2♣` as both majors |

The generated BBA card can disclose only `Transfers if RHO bids clubs = 1`,
not the exact counter table, so the alignment is structural rather than
literal. The old stack remains wired behind
`--defense-2c-landy-bba false`. See the
[closed N1 history](archive/one-notrump-competitive-closed.md#n1--the-landy-2-counter-shipped-default-on-2026-08-14) for its tables, measurement, and exploration trail.

## N3 — their `(3♣)`–`(3♠)` preempt of our 1NT (**SHIPPED DEFAULT-ON 2026-08-18**)

Knob `competition.nt_high_overcall_responses` (the table, **default on since
2026-08-18**) and `competition.nt_3c_transfers` (the `(3♣)` transfer variant,
default off); `bba-gen --ns-nt-high-overcall` / `--ns-nt-3c-transfers`, plus the new
`--ns-direct-3nt-stopper` for the gate arm. Code:
[nt_high_overcall.rs](../src/bidding/american/competition/nt_high_overcall.rs),
a sibling of `high_overcall.rs` keyed `P* 1NT (3x)`. Runner:
`scripts/ab-nt-high-overcall.sh`.

### What was wrong

Nothing in the book keyed `1NT (3…)`; `high_overcall_responses` covers suit
openings only. Three floor defects followed, each visible in `probe-decision`:

- Responder's new suit **read as nothing**. `- 1NT 3♣ 3♠ -` gave opener
  partner `hcp 0..37`, every suit `0..13`, and opener passed — on a board where
  `6♠` was cold. It is worse than passive: on the same hand at responder's seat
  the floor's own top call over `(3♣)` was **`4♥` on a doubleton** (a phantom
  suit at game).
- The double fired on the wrong hands — 6–7 HCP balanced over `(3♥)` (opener
  then drove to `4♠`), while a 9–11 4-4-4-1 over `(3♣)` had no call at all and
  passed.
- The floor blasted `6♣`/`5♦` on 8–11 HCP over `(3♠)`.

BBA's side is the reference in
[bba-1nt-counter-defense.md](ai-bidder/bba-1nt-counter-defense.md): its
three-level overcalls are natural seven-card preempts (`hcp 4–10`), and its
responder plays new suit natural 5+ `hcp 7–18`, `3NT` 9–17 with no stopper
gate, `X` = 4+♠ over `(3♥)` / balanced values over `(3m)`, `4m` weak natural
6+. So the lane needs an ordinary competitive scheme, not a counter-defense —
nothing here keys on a disclosure.

### Responder's table (`nt_over_high_overcall`)

Strength floors are the Lebensohl lane's — opposite 15–17, `points(10..)` is
game. The book owns responder's first call and opener's one answer; everything
deeper is the floor's (§"The book/floor line").

| call | weight | constraint | note |
| --- | ---: | --- | --- |
| `3y`, y above their suit | 180 + rank | `len(y, 5..) & points(10..) & at_least_as_long(y, ·)` vs each rival above their suit | natural, game-forcing |
| `4M`, M above their suit | 160 + rank | `len(M, 6..) & points(6..=9)` | natural, to play — the weak twin of `3M` |
| `4♥` over `(3♠)` | 160 | `(len(♥, 6..) & points(6..)) \| (len(♥, 5..) & points(9..))` | the only natural call left, so it carries the strong hands too |
| `X` | 150 | over `(3♥)` `len(♠, 4..)`, over `(3♠)` `len(♥, 4..)`, over `(3m)` `len(♥, 4..) \| len(♠, 4..)`; all `& points(8..)` | takeout, the 4-4 major finder; `.alert(NEGATIVE_DOUBLE)` |
| `3NT` | 140 | `author_direct_3nt`, fed the lane's **own** bit — `nt_high_overcall_3nt_stopper` (default **off**) is substituted for `direct_3nt_stopper` in a local `Agreements` ([nt_high_overcall.rs:29](../src/bidding/american/competition/nt_high_overcall.rs)) | to play |
| `4m`, m below their suit | 120 + rank | `len(m, 5..) & points(10..) & at_least_as_long(m, ·)` when both minors are below theirs | natural, forcing |
| `Pass` | 0 | `hcp(0..)` | the finite catch-all |

**"+ rank" is load-bearing, and so is `at_least_as_long`.** Without them a
natural-suit family at one weight is decided by the call encoding's iteration
order, which picks the *lower* suit: the census board `AT9754.AT732.A.A` over
`(3♣)` — six spades, five hearts — bid `3♥`. `at_least_as_long` keeps a 6-5
out of its five-carder, and the rank-ordered weight breaks the genuine 5-5 tie
*upward*, which is also the better bridge (it leaves the lower suit biddable as
a correction under partner's raise). The same pair runs on the `4m` rungs and
on the `(3♣)` transfers. It pays a reading dividend too: after `3♠`, partner
now reads `♦ ..6` and `♥ ..6` alongside `♠ 5..`.

**The one deviation from the plan of record**: `4m` is priced at **120, under
`3NT` and `X`**, not the planned 170. At 170 a hand with five clubs, four
spades and game values bids `4♣` over `(3♥)` — bypassing both the 4-4 major
and the game we are trying to reach. Below `3NT` the rung fires exactly where
it should: 10+ points, a five-card minor, no four-card major, and no stopper.
The consequence is that in the `nostop` arm (`direct_3nt_stopper false`) `4m`
is inert, since `3NT` then admits every 10+ hand — which is what "partner holds
the stopper" means, and the A/B prices it.

### Opener's three answers

- `1NT (3x) 3y -` — a **major** `y`: `4y` 150 `len(y, 3..)`, `3NT` 140
  `stopper_in_their_suits()`, catch-all `4y` 100. A **minor** `y` (only `3♦`
  over `(3♣)`): `3NT` 150 with the stopper, `3M` 140 `len(M, 4..)` for each
  unbid major, `4y` 130 `len(y, 3..)`, catch-all `3NT` 100. A force must be
  answered, so both arms end finite.
- `1NT (3x) 4m -` — `3NT` is already gone: `5m` 140 `len(m, 3..)`, `4M` 130
  `len(M, 5..)`, catch-all `5m` 100. Responder denied both a five-card suit
  above theirs and a four-card major, so opener's major is at best 5-3 and the
  eight-card minor fit wins the tie.
- `1NT (3x) X -` — `high_overcall.rs`'s negative-double answer re-floored for
  15–17, plus the two rungs the 2026-08-19 census decomposition bought. In
  weight order: the shown major jumped to game with `points(17..)` (150); that
  major at its **cheapest legal** level with four — `3M` (140) when it clears
  their suit, `4M` (140) when it does not
  (`nt_high_overcall_x_major_at_four`, **default on**, the rung their `(3♠)`
  leaves missing); **`Pass` (135)** with `len(over, 4..)`, converting the
  takeout double to penalty (`nt_high_overcall_x_leave_in`, **default on**);
  `3NT` 130 on a stopper; the three-card tolerance (30 at the three level / 25
  at the four); catch-all `3NT` 15. Every major row is `at_least_as_long`-
  guarded whenever their overcall leaves both majors live, so a 5-4 answers in
  its five-carder (round 4). The leave-in's honor half —
  `nt_high_overcall_x_leave_in_three`, `len3 & top_honors(2..)` — is **refuted
  and default-off**: at three cards an A/K/Q in their suit *is* the stopper, so
  passing spends a real `3NT`; at four the suit was never running anyway and
  the same honors are pure defensive tricks.

### The `(3♣)` transfer variant (`nt_3c_transfers`)

`(3♣)` is the one three-level overcall that leaves steps below `3NT`. The arm
replaces the natural three-level rows in the `(3♣)` instance only:

| call | weight | constraint | reading |
| --- | ---: | --- | --- |
| `3♦` | 180 | `len(♥, 5..) & points(9..) & at_least_as_long(♥, ♠)` | transfer to ♥, INV+ |
| `3♥` | **181** | `len(♠, 5..) & points(9..) & at_least_as_long(♠, ♥)` | transfer to ♠, INV+ |
| `3♠` | 145 | `len(♦, 6..) & points(10..)` | transfer to ♦, GF |

As over `1NT (2♦)`, responder with the long minor **and** a stopper in their
suit prefers direct `3NT`; without one, the top-step transfer wins over the
general stopperless `3NT` fallback.

All three `.alert(LEBENSOHL_TRANSFER)`, so `project_authored` decodes the rule's
own constraint — no new reader. INV+ is driven to game at the completion (the
"displaced bid is GF" simplification), so there is no min/max split and no
responder second call to author. What it buys is the invitational five-card
major, which the natural table can only show as `X` or a pass, plus
right-siding — DD-blind, so the plain scorer will not credit it.
**BBA plays all three naturally** (per-suit census recorded in
[bba-1nt-counter-defense.md](ai-bidder/bba-1nt-counter-defense.md)
§`(3♣)` per call), so the arm is judged on its own merit, not on alignment.

Two build traps the design walked into and out of:

- The top-step minor transfer now shares `rubensohl::minor_transfer_completion`
  with `1NT (2♦) 3♠`, with the minors swapped: `3NT` with a stopper in their
  suit, otherwise five of the target minor as the finite catch-all. Thus
  `1NT (3♣) 3♠ -` is `3NT` with a club stopper, otherwise `5♦`; the
  three-level completion is below the artificial `3♠` and therefore illegal.
- The interfered tails are authored per the iron rule: their `X` of a transfer
  steals no room, so the completion re-registers verbatim; their `(4♣)` raise
  takes every step, so opener completes at the four level with tolerance
  (150) and doubles for values otherwise (100). Everything past that is floor.

### Measurement — eight rounds, archived

The table and its answer ladder were measured in eight rounds between
2026-08-18 and 2026-08-21, every headline preceded by
`probe-divergence --gate-opener ours`. The verdicts are pinned to their shas
and seeds and live in
[§N3 — measurement rounds](archive/one-notrump-competitive-closed.md#n3--measurement-rounds);
the [ledger](#ledger) carries their numbers. What the rounds settled:

| round | question | verdict |
| --- | --- | --- |
| 1 (2026-08-18) | the table itself, `stop ↔ base` | **SHIPPED default-on** — owned plain +0.0021/+0.0029, no negative cell in sixteen readings |
| — | `stop` vs `nostop`, the *shared* `3NT` stopper bit | two lanes summed and disagreeing (+2.20 here, −1.23 PD in the advance lane), so this table got its **own** bit |
| 2 (2026-08-18) | that private bit, and the `(3♣)` transfers | bit **SHIPPED off** (+2.37/+1.65 per fired, 0 foreign on all four cells); transfers a wash ×2 seeds, opt-in |
| 3 (2026-08-19) | the top step re-cut as `1NT (2♦) 3♠`'s minor-swapped twin | still `wash \| wash`; `nt_3c_transfers` stays opt-in, default byte-identical |
| — (2026-08-19) | BBA-style double continuation (cheapest major, else `3♦`/`4♦`) | **REFUTED and removed** — all eight cells negative, all eight CIs excluding zero; `4♦ ← 3NT` missed the games. Do not retry the whole continuation |
| 4 (2026-08-19) | the answer tables' cross-call weight ties (encoding order picked hearts) | **SHIPPED** as a repair at a pre-pinned gate; 0 foreign of 252/221. The IMPs came from the *reading* the `at_least_as_long` guard publishes, not the 5-4 call change |
| 5 (2026-08-19) | opener's answer to the takeout double: the `4M` fit rung, and a v1 leave-in | fit rung **SHIPPED default-on** (4/4 DD cells CI-clear, 0 foreign); leave-in v1 **REFUTED** — sd-lead CI-clear negative in all four cells at −1.75…−2.06 per fired |
| 6 (2026-08-20) | re-slice v1's own dumps by opener's holding in their suit | the loss is **length**-shaped, not honor-shaped; the honor axis runs backwards because at three cards an A/K/Q *is* the stopper. v2 splits into two knobs |
| 7 (2026-08-20) | `length` and `three` as separate arms, fresh seed | `length` **SHIPPED default-on** — CI-clear positive in all eight cells (+2.38/+3.41 per fired), 0 foreign, 13/13 buckets DD-positive; `three` **REFUTED**, opt-in |
| 8 (2026-08-21) | is any suit-dependent gate worth authoring? | **no** — every suit DD-positive at both vulnerabilities, the round-7 gradient was noise around a real spades-best tilt that never crosses zero, the spade-only widening of `_three` sd-negative in every suit, and the leave-in replicated CI-clear on an independent seed |

**Current score against the lane** — the census arms above, so this is
attribution and not a causal A/B: each number is the whole board's swing and
carries the mirrored table.

*Paired, same seed 1787064872.* Against the post-ship 2026-08-18 arms
(`9cfb464b`), which reproduce their published totals exactly, the four authored
suits move **410 bd: −273 → −192 plain, −186 → −73 PD**; with the floor-only
`4+` bucket, **448 bd: −316 → −235 plain, −231 → −118 PD**, i.e. −0.71 → −0.52
plain and −0.52 → −0.26 per board. `4+` itself is unchanged at 38 bd / −43 /
−45, so every IMP of the movement is in the authored buckets — the answer
refinements of rounds 4–7.

*Cross-seed, for scale only.* The pre-ship snapshot (`53a3c254`, seed
1783375064) had the four suits at 362 bd / −345 plain. Do **not** subtract two
anchor totals across seeds to estimate the package's value; the isolated A/Bs
in the archive are the causal evidence.

### Disclosure — what BBA is told

No card row exists for this lane. `card.rs`'s `SCHEMA` has no slot for
"responses to their three-level overcall of our 1NT", and grepping it turns up
nothing between `Lebensohl after 1NT` / `Rubensohl after 1NT` (both the *two*-level
lane) and the preempt rows. The remaining transfer variant is default-off, so
it does not change the default alert fixture; it adds `comp:lebensohl-transfer`
and `completion`. The transfer arm additionally has
no honest card row to set, the same record as the Landy and Multi counters: a
treatment EPBot's schema does not name, so it is as invisible to BBA as
`Not defined = 0`. Accepted for measurement; if it ever ships, that asymmetry
belongs in its ship row. The removed BBA arm used
`comp:nt-high-bba-placement` during its experiment.

### Flagged, not fixed (floor defects; reversible defaults proposed)

- `1NT (4♥) X - 5♦ - 5♥ X - - -` — responder's third call is a five-level cue
  the floor then passes (2 bd, −34). Out of the book's line; the proposed
  repair is a floor rail "no cue above game unless slam-forcing", for the floor
  campaign.
- Their `(4x)` overcalls: **re-priced 2026-08-19** — see
  [The v2 queue, re-priced](archive/one-notrump-competitive-closed.md#the-v2-queue-re-priced-probe--fresh-seed-census-2026-08-19).
  The fresh-seed bucket is 38 bd / −43 plain / −45 PD with a CI that swallows it, and the trigger is an *eight*-card suit, so the `(3x)`
  template does not widen. The floor still offers **no `X` over `(4x)` at all**
  (`their_live_bid_at_most(3)`, `instinct.rs:6058`), and BBA's advancer sits for
  that double on 96.7–99.9%, so a book `X` there is the surviving item.
- We **never** overcall a 1NT opening at the three level (table B: 0 boards) —
  BBA does on 1.5% of its 1NT-defense hands. Obstruction is DD-blind, so a
  preempt package there is a single-dummy harness item, not this one.
- `(3♥) 3NT` on a singleton in their suit (11 bd, −23) was the
  `direct_3nt_stopper` question; round 2 answered it (the lane's own bit,
  shipped off).
- **Dead rows, deliberately left**: in `nt_answer_double` the `4M@25`
  three-card-tolerance rung is unreachable whenever that major clears their
  suit — the identical `3M@30` rung outranks it on exactly the same hands — so
  it is dead in the `(3♣)` and `(3♦)` tables and for spades in the `(3♥)` one.
  Deleting them is **not** inert: an unreachable row still joins the *reading*
  of its call, so dropping `4♥@25 len(♥, 3..)` would narrow a made `4♥` from
  "three-plus hearts" to "four and a maximum" and move calls elsewhere.
  Proposed reversible default: **leave them**, and revisit only inside a reading
  A/B.
- Opener's answer to a forcing **major** never shows the *other* four-card
  major (`1NT (3♣) 3♥ -` with four spades bids `4♥` or `3NT`, never `3♠`).
  Deliberate — one book answer, floor beyond — but it is a real 4-4 miss when
  responder is 5-4.

### Open residue

Round 8 closed the answer-to-the-double thread. What is left, and where it
lives:

- **`X` over their `(4x)`** — the surviving `(4x)` item, queued as **N3-x**
  above. The `(3x)` template does not widen to it.
- **Fresh-seed confirmation of the `4M` fit rung** — owed since round 5, queued
  as **N3-fit**. Rounds 7–8 ran against a `base` that carries the rung, which
  confirms nothing about the rung itself.
- **`nt_high_overcall_x_leave_in_three`** — refuted, opt-in, a single-dummy
  re-measure candidate (**N3-three**).
- **`nt_3c_transfers`** — measured wash on four seeds, opt-in; what it buys is
  DD-blind (**N3-xfer**).
- **Reading drift** — 11.8%/9.6% of the leave-in's divergences move only a
  *later* call, −0.56 DD / −0.63 sd per fired vulnerable against +1.41/+1.15
  NV. Pooled positive, but a real vulnerable cost inside a shipped win. Owned
  by [reading-drift-handoff.md](reading-drift-handoff.md).
- **The penalty pass is *not* open.** Round 8's closing sentence still lists it
  among the remaining items; that line is stale. It shipped as the
  `len(over, 4..)` leave-in in round 7 and replicated in round 8. Proposed
  reversible default: treat it as closed, and read round 8's list as naming
  `_three` (the refuted honor half) rather than the pass itself.

## N4 — their `(2♦)` as a Multi (**SHIPPED 2026-08-15 — v7, seven rounds; default-on vs BBA via the census**)

The rebuild of the deleted `defense_2d_multi`, on the disclosure channel N1
uses and with the continuations gated. Not the natural table
[bba-multi-2d.md §4](ai-bidder/bba-multi-2d.md) sketched — that arm was **not
run** (recorded here so nobody assumes it was): the shipped Transfer-Lebensohl
`(2♦)` leg keeps its constructive calls, and only what named *diamonds* moves.

### Engagement — `their.two_diamonds_multi`

The second field of `TheirDisclosures`: their `2♦` is a Multi, one unknown
six-card major (BBA's 2/1 reference: `hcp 9–18`, median 13,
[bba-multi-2d.md](ai-bidder/bba-multi-2d.md)). Undeclared keeps the natural
leg, byte-identical (smoke `18aba5ce…` re-verified from the worktree — that
constant is **stale**: `smoke-default` was re-based to `cf583ff5…` on
2026-08-16 by the strength-ceiling ship
([authored-reading-handoff.md](authored-reading-handoff.md)) and has moved
again since. Re-verify byte-identity by diffing the dump against `main` HEAD,
not against a quoted digest).
`bba-gen --their-2d-multi` arms it; `their_2d_multi` derives it from an
explicit `Multi-Landy` row at face value and otherwise uses the shipped 2/1
census default. `--their-2d-multi false` names the pre-N4 arm.
`bba-decompose --multi-counter` replays a candidate dump; `web`
`declare_their_2d_multi`; `probe-call-reading --their-2d-multi`.

Either/or with the natural leg (`defense_2d_multi` in `lebensohl.rs`), never
an overlay: the deleted first build gated responder's table and left the
continuations natural, so opener answered a natural `3♦` with a transfer
completion. Only the Transfer style has a `(2♦)` leg to re-key; Plain keeps its
natural table under the declaration.

### The table (`multi_2d_responder`, `rubensohl.rs`)

`stayman_2d_constructive` is shared verbatim with the natural leg — `3♣`
Stayman + Smolen, `3♦`→♥, `3♥`→♠, `3♠`→♣, Leaping Michaels `4♣`/`4♦` — the fits
it hunts are in the major they do *not* hold. What moves:

| call | natural leg | Multi leg | why |
| --- | --- | --- | --- |
| `X` | `DoubleStyle::Optional`: `len(♦, 2..=3) & hcp(8..)`, opener cooperates (pass, or run to a 5-card suit with ≤2♦) | **`hcp(6..)`** (v7; v1–v5 `hcp(8..)` at 143), alerted `comp:multi-values`, weight 130 — below `3NT` 150, `3♠`→♣ 145, the natural `2M` 140 and the relay 135, so a weak 5+ suit still escapes or relays | BBA's own values double (`hcp 5–17`, 41% of its hands, median 9), no diamond claim — the *waiting* call; they name the major, we act on it (`multi_responder_rebid`). Read: `points 6..`, every suit ⊤ |
| direct `3NT` | `points(10..) & stopper_in(♦)` | `points(10..) & stopper_in(♥) & stopper_in(♠)` | the blast that needs no more information; one major open → double first |
| `2NT` relay shape | 5+ in ♣/♥/♠, `hcp 6+` | 5+ in **any** suit, `hcp 6+` | diamonds are ours to sign off in |
| after `2NT - 3♣ -` | `3♥`/`3♠` (5+) or pass | **`3♦`**/`3♥`/`3♠` (5+) or pass (`multi_relay_rebid`) | the rung the natural leg cannot have |
| `3♠`→♣ completion | `3NT` with a ♦ stopper else `5♣` | `3NT` outright | no stopper to key on; `5♣` on a 6-2 fit is the worse guess |
| opener over `X -` | cooperate | a four-card major (`2♥` first), else **pass** (`multi_pass_answer`, v7 — BBA shows the major and cues `3♦` otherwise; the cue is the pass) | a seat BBA's advancer never gives (0.0% at `advance-x`) — see below |
| opener over `X (2♥)` / `X (2♠)` | floor | **`X` = `len(M, 4..)`** alerted `comp:multi-penalty`, else **pass** (`multi_penalty_answer`) | nominally penalty; when the overcaller's major is the other one they correct, and partner has been told where our trumps are. Read: `♥ 4..` / `♠ 4..` (probed) |
| `two_diamond_double` | armed = diamond penalty double | ignored | a diamond penalty double of a Multi is the gate N4b measured null |

That was v1's line: everything deeper the floor's. The design round (grilled
2026-08-15) had settled *against* copying nodes for the pass-or-correct
positions (a finite node shadows the floor; the seat differs — after `- (2♥)`
*opener* acts), against `(3M)` continuations (BBA's advancer never bids one),
and for the read side — their `2♦` as `6+♥ ∪ 6+♠` in the floor's envelopes,
the N1g pattern with a union box — as a **separate second A/B**. The
measurement overturned the first of those (below): the final table (v4) also
authors, all under the same gate —

| seat | table |
| --- | --- |
| responder after `X (2♥) - (-)`, `X (2♥) - (2♠)`, `X (2♠) - (-)`, `X (2♥) X (2♠)` | `multi_responder_rebid(M, ran)` on the *resolved* major — **v7 (shipped)**: `4NT` = `hcp(16..)`; `2♠` = five spades `hcp ≤8` (hearts resolved); `X` = **takeout**, four of the other major and ≤2 of theirs (`comp:multi-takeout`) — in the `ran` shape (`X (2♥) - (2♠)`, `X (2♥) X (2♠)`) four spades and 7+ (`comp:multi-penalty`); `3NT` = `points(10..) & stopper_in(M)`; else pass. (v4: `3NT` stopper / `X` = `len(M, 4..)` penalty / pass.) |
| responder after `X (2M) X -` | sit |
| opener after responder's takeout double (`X (2M) - (-) X -`) | `multi_takeout_answer(M)`: sit with four of theirs, bid the 4-4 fit (`2♠`/`3♥`), else a four-card minor, else `2NT` |
| opener after responder's `ran`-shape penalty double, `2♠`, `4NT` | sit / sit / `6NT` with 17 else pass (`multi_quant_answer`) |
| both after the overcaller's `2NT` heart relay over `2♠` (`X (2♠) X/- (2NT)`) | pass |
| the doubled relay: `2NT (X)`, `2NT (X) 3♣ -`, `2NT - 3♣ (X)` | completion / `multi_relay_rebid` |
| opener over every relay sign-off (`3♦`/`3♥`/`3♠`, all three relay paths), their X of it, their bid over it; responder over their balance | pass (`multi_signoff_pass`, `Pattern::up_to … 7♠`) |

Their `(3M)` jumps, the advancer's `4♦`, and everything past these stay the
floor's.

### Two facts the build corrected

1. **BBA's advancer never passes our double.** The N4b write-up's "they sit
   43% (6 of 14)" was the *foreign* lane — BBA's responder doubling **our**
   `2♦` overcall and *our* advancer passing — mis-read as ours. Re-counted on
   the N4b `len5` and `base` dumps by opener's side: on our-opened
   `1NT (2♦) X` boards the advancer passed **0 of 141 / 0 of 339**; and the new
   `probe-bba-constraints --mode advance-x` (seat 3 over `1NT 2♦ X`, our side
   declared natural as `bba-gen` models us) is **`2♠` 66.9% / `2♥` 33.0% /
   pass 0.0%** — identical to the undoubled relay. So opener's `X -` sit is a
   node BBA never reaches; it stays as the human-partner default and Q3 of the
   design round ("show a four-card major instead") has nothing to decompose
   against BBA. Conditional on our double, the advancer leans weak — the N4b
   `len5` arm saw `2♥` 136 / `2♠` 5 on our-opened boards.

   **The two splits are inverted, and both are right (checked 2026-08-21).**
   The probe reproduces unchanged on the shipped build — NV `2♠` 67.0% / `2♥`
   33.0%, vul `2♠` 73.0% / `2♥` 26.9%, pass 0.0% — while the census arms'
   *realized* auctions over our double are `2♥` **155 / 200 (77.5%)**, `2♠` 45
   (22.5%), pass 0. They measure different populations: the probe deals the
   advancer at random, and its `2♠` bucket is `hcp 6–18` (median 11) against
   `2♥`'s `hcp 2–14` (median 6) — an advancer that strong rarely coexists with
   our `hcp(6..)` double opposite a 15-17 opener. **Size the `ran` shapes off
   the conditional split (`2♥` 77.5%), not the probe's unconditional one.**
   No code changes on this: v7's `ran` tables already carry both legs, and the
   reversible default if it is ever re-litigated is to leave them as built.
2. **The advancer's split is by strength, not shape**, and the correction
   mechanics are BBA's own: over the weak `2♥` the overcaller passes with
   hearts (36.7%), corrects to `2♠` with spades (49.8%), jumps `3M` with a
   seven-carder (13.5%) — `--mode rebid-d-x2h`, i.e. after our double; over the
   invitational `2♠` it bids `2NT` as a heart relay, never `3♥`. There is no
   advancer `3♥`/`3♠` in any probed table.

**Rounds v1–v6:** [archived measurement trail](archive/one-notrump-competitive-closed.md#n4--measurement-rounds-v1v6).

### v7 — BBA's structure minus the artifacts (**measured 2026-08-15 ×3 seeds: SHIPPED** — `2d-multi-v7`, `-v7s2`, `-v7s3`)

Same run shape (v4 base arms by symlink, Multi arm regenerated, priced vs
base and paired vs v4), owned boards, 691.2k bd/vul:

| v7 vs | vul | n | plain /bd | PD /bd | row |
| --- | --- | ---: | ---: | ---: | --- |
| base | NV | 1162 | +0.00019 ±0.00053 | **+0.00100 ±0.00067** | `plain wash \| PD win` — **ship** |
| base | vul | 835 | **+0.00061 ±0.00056** | +0.00061 ±0.00069 | plain win (CI-clear by 0.00005) \| PD wash leaning + |
| base | both pooled | 1997 | **+0.00040 ±0.00039** | **+0.00081 ±0.00048** | win \| win |
| v4 (paired) | NV | 501 | **+0.00075 ±0.00031** | +0.00020 ±0.00038 | better |
| v4 (paired) | vul | 377 | **+0.00041 ±0.00034** | −0.00022 ±0.00043 | plain better, PD wash |

The vul row is `win | wash` — the "doubling artifact, suspect" row as written,
but its domain addendum applies: v7's mechanism is *doubling them more* (the
takeout X, +2.4/+1.6 per fired NV and +2.2/+0.6 vul against v4, and the
penalty passes it produces), the case where PD is blind to the benefit and
plain DD is the arbiter — and PD is not negative anyway. No cell of the
eight is negative; the NV headline is the ship row by the letter; the
both-vul pool is `win | win`. **Shipped.**

What is left on the table (v7 vs base, NV, plain / PD per fired): the
sell-out after they *run* to `2♠` — `X (2♥) - (2♠) -` **−4.03 / −0.57**
(n=65) and `X (2♥) X (2♠) -` −3.61 / +0.56 (n=59) — is the 10–12 hand with
no spade stopper and fewer than four spades, still passing; BBA's blind
`3NT` there was the one blind blast PD tolerated (+0.09 NV, −1.79 vul, plain
+3.9/+3.1). The honest stopper-ask cue (`3♠`) is measured in the residue
round below.
The `X (2♥) - (-) -` sell-out itself is now plain −2.05 / PD **+2.02** NV
and +0.32 / **+5.01** vul — perfect defense wants us defending BBA's `2♥`,
and the takeout X takes the hands that should not.

Ship mechanics: `their_2d_multi`'s bottom arm is the 2/1 census default
(`--their-2d-multi false` is the pre-ship arm; an explicit Landy-family
declaration without `Multi-Landy` reads as a declared no-Multi),
`vs_bba_agreements` sets `two_diamonds_multi` (so `bba-decompose` replays it
by default — `--multi-counter false` for dumps generated before), the
`[their-landy]` alert-sites anchor arms it (three slugs: `comp:multi-values`
4, `comp:multi-penalty` 16, `comp:multi-takeout` 8; the fenced relay tail
moves `comp:lebensohl-completion` 24 → 44 and `completion` 696 → 680), and
`card.rs` records why no EPBot schema row exists. Engine default stays
undeclared: smoke `18aba5ce…` unchanged.

### Verdict — v7 shipped; v4's numbers, and the read-side follow-up, for the record

v4 (three seeds, owned): **vul `plain wash | PD win`**; **NV `PD win | plain
−0.00055 ±0.00050`** — opt-in by the letter of the gate. Its per-call
decomposition (above) put the whole NV plain deficit on the doubler's second
turn; v6 mimicked BBA's second turn whole and found BBA's takeout double
real and BBA's game bids the DD-declarer artifact; v7 kept the one and
dropped the others and clears the gate. sd-lead could not arbitrate rounds
v1–v7: `ab-dump-sd` has no owner split, and those arms were 60–70% foreign,
so the raw sd is leak-inflated like every other raw number here. **That
caveat does not extend to the residue round** — its pairs are 0-foreign, its
sd files were written and never reported, and they are now in
[§N4 residue](#n4-residue--reader-shipped-stopper-ask-stays-opt-in-measured-2026-08-16).

What the seven rounds established, beyond the numbers: **the floor cannot
hold any seat of this structure** — it sold out with 10+, raised a weak
sign-off to game, pulled both sides' penalty doubles, and cued their relay —
because it read their `2♦` as diamonds and their `2M` as natural. Every one
of those seats is now a book node (the "copy nodes" the design round argued
against and the measurement demanded), and the read-side follow-up (their
`2♦` as `6+♥ ∪ 6+♠`) now gives the remaining floor decisions the same
fact.

### N4 residue — reader shipped; stopper ask stays opt-in (**measured 2026-08-16**)

The temporary reader fires only when our side opened `1NT`, the opponents'
first action is their disclosed Multi `2♦`, and
`reading.their_multi_reading` is on. It suppresses both the natural-diamond
read and the advancer's first `2♥`/`2♠` pass-or-correct read, then intersects
the same exact two-box union into sampler and announced inference:
`{ ♥6+ } ∪ { ♠6+ }`. It claims no strength, minor length, or other-major
length. The systems-on overcall strip **declines that one shape outright**
(2026-08-22), so `(1x) 1NT (2♦)` cannot enter this lane. Before then it only
cleared the *profile* flag, which stops this hand reader but cannot un-compile
the book's Multi table — the leak N4e's isolation gate caught; see
[§N4e](#n4e--the-floorless-weak-escape-shipped-default-on-2026-08-22-the-six-card-rung-five-refuted).
It is temporary until a declared-opponent profile can project the opponents'
own authored book.

The independently gated `competition.multi_stopper_ask` has three modes:
`Off`, `FitSearch`, and `OpenerPlaces`. In only the two `ran=true` corrections
to spades, responder may bid alerted `3♠` with 10–12 points, at most three
spades, and no spade stopper. Opener bids `3NT` with a stopper; otherwise it
uses the ordinary deterministic longest-side-suit choice. `FitSearch` lets
responder pass `4♥`, raise a known minor fit to game, or name a remaining
four-card side suit, with the lone `4♣–4♦` branch placed in `5♦` with support
and `5♣` otherwise. `OpenerPlaces` chooses `4♥` or `5m` immediately.

Their double of the ask rebases to the same answers and continuations. Over
their `4♠`, opener doubles with a stopper or four spades and otherwise makes
a forcing pass; after two opposing passes responder names its longest
four-plus side suit at the five level. All resulting games, penalty doubles,
and doubled signoffs are book-owned terminal passes, fenced from the floor.

`scripts/ab-2d-multi-residue.sh` ran four aligned arms — shipped v7,
reader-only, `FitSearch`-only, and `OpenerPlaces`-only — with the other residue
knob pinned off. Three independent seeds (1786812881, 1786813975,
1786815052), 230.4k accepted boards per arm/vulnerability/seed, both
vulnerabilities, `--filter-1nt`, plain DD and PD: 691.2k boards per table row.
Every pair/seed/vulnerability passed `probe-divergence --gate-opener ours`
with **zero foreign boards**.

| arm vs shipped v7 | vul | fired | plain /bd | PD /bd | verdict |
| --- | --- | ---: | ---: | ---: | --- |
| reader | NV | 321 | −0.0001 ±0.0003 | +0.0004 ±0.0004 | wash \| wash |
| reader | both | 174 | +0.0001 ±0.0003 | **+0.0006 ±0.0004** | wash \| win |
| `FitSearch` | NV | 84 | **+0.0006 ±0.0002** | +0.0001 ±0.0002 | win \| wash |
| `FitSearch` | both | 58 | **+0.0004 ±0.0002** | −0.0001 ±0.0002 | win \| wash |
| `OpenerPlaces` | NV | 84 | **+0.0006 ±0.0002** | +0.0001 ±0.0002 | win \| wash |
| `OpenerPlaces` | both | 58 | **+0.0004 ±0.0002** | −0.0001 ±0.0002 | win \| wash |

Across both vulnerabilities the reader summed **−29 plain / +643 PD IMPs**
on 1.3824m boards: no honest-score downside and a CI-clear pooled PD gain.
That is the repository's `plain wash | PD win` ship row, so
`their_multi_reading` is default-on (still inert without the disclosure).

Both stopper continuations instead land on the table's `plain win | PD wash`
doubling-artifact row, so the ask remains default `Off`. Their direct paired
comparison was a statistical tie: NV only two contracts differed
(`FitSearch − OpenerPlaces` +4 plain / −4 PD IMPs); vulnerable, none did.
Because no stopper mode passed independently, there is no selected reader +
stopper stack and the conditional combined confirmation arm was skipped.

#### The sd-lead third scorer — written 2026-08-16, reported 2026-08-21

`ab-2d-multi-residue.sh` called `sddiff` on every pair and the files were
never read (`ab-results/2d-multi-residue/seed-{1,2,3}/sd.*.txt`, 16 worlds,
230.4k bd per cell). Unlike the v1–v7 arms these pairs are **0 foreign**, so
the leak-inflation caveat does not apply and the numbers stand as a third
scorer:

| pair | vul | seed 1 | seed 2 | seed 3 |
| --- | --- | --- | --- | --- |
| `FitSearch` vs base, sd-plain | NV | **+0.0008 ±0.0004** | **+0.0007 ±0.0004** | **+0.0006 ±0.0003** |
| `FitSearch` vs base, sd-plain | both | **+0.0007 ±0.0004** | +0.0003 ±0.0003 | +0.0003 ±0.0003 |
| `FitSearch` vs base, sd-PD | NV | +0.0005 ±0.0004 | +0.0002 ±0.0004 | +0.0001 ±0.0004 |
| `FitSearch` vs base, sd-PD | both | +0.0004 ±0.0004 | −0.0001 ±0.0004 | −0.0001 ±0.0004 |
| reader vs base, sd-plain | NV | −0.0005 ±0.0006 | +0.0002 ±0.0006 | +0.0002 ±0.0006 |
| reader vs base, sd-PD | NV | +0.0001 ±0.0006 | +0.0004 ±0.0006 | **+0.0007 ±0.0007** |
| reader vs base, sd-PD | both | +0.0000 ±0.0007 | +0.0006 ±0.0006 | +0.0005 ±0.0006 |

`OpenerPlaces` is within ±4 IMPs of `FitSearch` in every cell (their direct
`search vs place` sd pairs fire on 0–4 boards), so the two remain tied on this
scorer as well.

**The verdicts do not move.** `FitSearch` is sd-plain positive in all six
cells — three CI-clear at NV, one CI-clear vulnerable, two vulnerable cells
(+0.0003 ±0.0003) sitting on the boundary — and sd-PD wash-to-positive. That
is the *same* `plain win | PD wash` doubling-artifact signature on a third
scorer, not an escape from it, so **`multi_stopper_ask` stays default `Off`**.
The reader's sd is a wash on plain and leans positive on PD, consistent with
the DD reading that shipped it. What changes is only the record: the sentence
in the verdict above claiming sd "could not arbitrate any round" was wrong
about this round.

### N4e — the floorless weak escape (**SHIPPED DEFAULT-ON 2026-08-22; the six-card rung, five refuted**)

The v7 counter and its reader shipped against the strong half of the `(2♦)`
bucket. This is the re-decomposition of what is left, read with
`probe-1nt-interference --bucket "2♦" --responses 6` on the shipped arms
`ab-results/anchor-confirm/2026-08-21-1e9a47e2/american-{none,both}` (seed
1787064872, 204,800 bd/vulnerability, NV+vul pooled below).

**Read the size first.** −744 IMPs over 409,600 boards is **−0.0018
IMPs/board of the arm** — 0.3% of the −0.48/−0.59 gap to BBA — and per *board*
`(2♦)` is not the worst cell: `3♠` −1.49, `4+` −1.13, `3♣` −1.09 all beat it.
`(2♦)` leads the bucket table because their Multi fires twice as often as
anything else (816 of 3,115 contested boards), not because we handle it
uniquely badly. This is hygiene at the standard ship gate.

#### By responder's first call (816 bd)

| our response | bd | plain tot | plain/bd | PD tot | PD/bd | reading |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| **Pass** | 264 | **−452** | −1.71 | −226 | **−0.86** | real on both scorers |
| **`X`** | 200 | **−203** | −1.02 | **+133** | **+0.67** | plain loss / PD win — artifact |
| **`2♠`** | 57 | **−118** | −2.07 | −71 | −1.25 | n too small to separate from `2♥` |
| `3♣` | 45 | −15 | −0.33 | −27 | −0.60 | — |
| `2NT` relay | 79 | −6 | −0.08 | +11 | +0.14 | **the relay lane is free** |
| `3NT` | 6 | −4 | | −11 | | — |
| `3♥` | 61 | −2 | −0.03 | +6 | | — |
| `2♥` | 52 | +5 | +0.10 | +77 | +1.48 | — |
| `3♦` | 37 | +37 | +1.00 | +22 | +0.59 | — |
| other | 15 | +14 | | +6 | | — |

Three rows carry **−773**; everything else nets **+29**.

#### Pass, split by responder's hand (the probe's own classifier)

| class | bd | plain/bd | PD/bd | plain tot | PD tot |
| --- | ---: | ---: | ---: | ---: | ---: |
| **≤5 hcp, 6+ suit** | 37 | **−3.92** | **−4.84** | −145 | −179 |
| ≤5 hcp, 5-card suit | 97 | −1.73 | −0.84 | −168 | −81 |
| ≤7 hcp, no 5-card suit | 130 | −1.07 | **+0.26** | −139 | +34 |

The 6+-suit class is the worst per-board cell anywhere in this campaign —
worse than N2d's `(2♠)` twin (25 bd, −3.08/−2.08) and 1.9× its total. The
bottom row is PD-positive, i.e. mostly the DD-declarer artifact.

#### `X`, split by the advancer and opener's answer

| continuation | bd | plain/bd | PD/bd | plain | PD |
| --- | ---: | ---: | ---: | ---: | ---: |
| `X (2♥) - (-) -` sell-out | 48 | −1.94 | **+1.02** | −93 | +49 |
| `X (2♥) - (2♠) -` | 22 | −2.00 | +1.18 | −44 | +26 |
| **`X (2♥) X (2♠) -`** | 35 | **−2.40** | **−0.80** | −84 | −28 |
| `X (2♠) …` (whole leg) | 45 | +0.82 | +2.13 | +37 | +96 |

The heart leg loses, the spade leg wins, and every sub-cell except
`X (2♥) X (2♠)` is PD-positive. **Only that one is real on both scorers**:
responder doubles, the advancer bids the weak `2♥`, opener makes the penalty
double with four hearts (`multi_penalty_answer`), they run to `2♠`, and
`multi_responder_rebid(Spades, ran=true)` passes. It is already owned by
`multi_stopper_ask`, which stays `Off` (§N4 residue), so **the `X` lane gets
no new work**: −1.02 plain but **+0.67** PD, the sell-outs are the
DD-declarer artifact, and perfect defense says our pass is right.

#### The `2♠` row is not separable from `2♥`

−2.07 ± ~1.5 against `2♥`'s +0.10 ± ~1.8 → a difference of 2.2 ± 2.3. Pooled,
the natural two-level escape is **109 bd, −1.04 plain / +0.06 PD**. Same shape
as N3 round 8's refuted suit gradient, so **no suit gate is authored** on the
natural escape.

#### Root cause — in code

`multi_2d_responder` ([rubensohl.rs](../src/bidding/american/competition/rubensohl.rs))
has these finite rows, in weight order: `4♣`/`4♦` Leaping Michaels
`points(10..)`, `3♣` Stayman `points(10..)`, `3♦`/`3♥` transfers `points(9..)`,
`3NT` `points(10..)`, `3♠`→♣ `points(10..)`, natural `2♥`/`2♠`
`len(M,5..) & points(..=8) & hcp(5..)`, `2NT` relay
`points(..=8) & (5+ any suit) & hcp(6..)`, `X` `hcp(6..)`, `Pass` 0.

**Below 5 HCP, `Pass` is the only finite row in the table.** Between 5 and 6
HCP it is the only row unless responder holds a five-card *major*:

- 4 hcp with 6 hearts → pass (`96.KJT975.82.972`, −13)
- 4 hcp with 6 clubs → pass (`J.8542.96.QJ9853`, −11)
- 2 hcp with 7 diamonds → pass (`4.T63.QT97542.86`, −11)
- **5 hcp with 6 clubs → pass, one HCP under the relay floor** (`632.7.QJT.Q97532`, −9)

Meanwhile the `2NT` relay — where those hands want to go — measures **−0.08
plain / +0.14 PD** for the hands that clear its floor, and its landing spots
are all authored (`multi_relay_rebid` gives `3♦`/`3♥`/`3♠` or a pass in `3♣`;
`multi_signoff_pass` keeps opener quiet).

The standing objection is `lebensohl_relay_shape`'s **PD-distilled** 6-HCP
floor ([lebensohl.rs](../src/bidding/american/competition/lebensohl.rs)): a
sampled double-dummy gate "declines nearly every sub-6 hand — pushing a
near-bust to the 3 level loses on DD, even with a 6-card suit". Two reasons
that does not settle *this* lane: it was distilled over a **natural** overcall
whose suit is known, and it is a **DD** gate, the regime that systematically
flatters defending. The census's PD column — −4.84/bd on the 6+-suit class —
is the perfect-defense answer to the same question, and it says the pass is
losing. Which of the two is right is the A/B's job, and it is why the
five-card band gets its own arm.

Second gap: **`1NT (2♦) 2♥/2♠ (anything)` had no book node.** Only
`{their} 2M -` was wired (opener's sign-off raise, under `natural_floor`);
their competition over our escape fell to the floor, which in the dumps bid
`4♥` (their suit) over partner's `4♣` for −1100 doubled, and doubled `4♠` into
twelve tricks. The iron rule ("complete the convention — both sides'
continuations **and the interfered tails**") was unmet, and widening the
escape sends more traffic into it.

#### The package — `competition.multi_weak_escape: Option<u8>`

**One field, three states** — `None` (default, byte-identical), `Some(6)`,
`Some(5)`: the minimum suit length that may act with **no HCP floor** over
their *declared* Multi. One knob rather than two booleans, so the two arms of
the A/B are the same measured change at two settings. Three rungs move
together:

| rung | change under `Some(n)` |
| --- | --- |
| natural `2♥`/`2♠` (140) | `len(M, n..)` with no HCP floor, alongside the existing `len(M,5..) & hcp(5..)`. At `n = 5` this simply removes the floor for five-card majors |
| `2NT` relay (135) | `multi_relay_shape()` is unioned with `len(any, n..)`, no HCP floor — the only outlet a six-card *minor* or diamond suit has, since the natural escape is majors-only |
| opener's sign-off raise | `lebensohl_signoff_raise` is fed `0` instead of `natural_floor_hcp` (5), so its `23 − resp_floor` game bar rises exactly as far as the reading `project_authored` publishes falls (`hcp 5..8` → `hcp 0..8`). Getting that pair out of step is the reading-drift failure mode, not a cosmetic detail |

A six-card major is itself evidence their Multi is the *other* major, which is
what makes a two-level escape safer here than over a natural overcall — hence
the length gate rather than a flat floor removal.

The same knob authors the **interfered tail** (`multi_escape_overcalled`):
`1NT (2♦) 2M (X | 2♠ | 2NT | 3♣/3♦/3♥/3♠)`, opener's one answer, on N1f's
`landy_cue_overcalled` doctrine one level lower — the competitive raise on a
fit with a maximum, the values double when there is no raise to make, and
Pass for everything else, which is *safe* because responder has shown a long
suit and at most eight. Their double is a pure sit: running a known 5-3 fit
out of a doubled two-level partial is the disaster the escape was authored to
avoid. Above `3♠`, and everything past opener's answer, stays floor.

Verified on the four census hands (`probe-decision`, with the new
`PROBE_THEIR_2D_MULTI` / `PROBE_MULTI_WEAK_ESCAPE` env vars — before them
*every forensic in this lane silently probed the natural `(2♦)` leg*):

| hand | `None` | `Some(6)` | `Some(5)` |
| --- | --- | --- | --- |
| `632.7.QJT.Q97532` (5 hcp, 6♣) | `P` only | **`2NT`** 1.350 | `2NT` |
| `96.KJT975.82.972` (4 hcp, 6♥) | `P` only | **`2♥`** 1.400 | `2♥` |
| `4.T63.QT97542.86` (2 hcp, 7♦) | `P` only | **`2NT`** 1.350 | `2NT` |
| `JT987.2.T5.86542` (5♠/5♣, 1 hcp) | `P` only | **`P`** (arm isolation) | **`2♠`** 1.400 |

The default system is byte-identical: `smoke-default` was run at `main` HEAD
and at the built tree and the dumps hash the same. The alert invariant holds
on a new `their-multi-escape` gated profile (the relay keeps `LEBENSOHL_RELAY`;
the natural `2M`, the raise and the values double are all natural).

#### The A/B — `scripts/ab-2d-multi-escape.sh`, two arms

`bba-gen --filter-1nt --their-2d-multi --ns-multi-weak-escape off|6|5`, arms
**sequential**, one fresh `SEED_BASE=$(date +%s)` shared across all three,
never rebuild in flight.

| arm | setting | priced against | target cell |
| --- | --- | --- | --- |
| `base` | `None` | — | — |
| `six` | `Some(6)` | `base` | 37 bd, −145 plain / −179 PD |
| `five` | `Some(5)` | `base`, and **paired** vs `six` | adds the 97-bd five-card class (−168 / −81) |

Both vulnerabilities, plain **and** PD, headline as IMPs per *accepted* deal
with `per-board = conditional mean × trigger density` alongside;
`probe-divergence --gate-opener ours` must read **0 foreign** on every pair
before any headline is quoted. The `five` vs `six` paired comparison is what
says whether the five-card band earns its own default — N3 rounds 6–7 are the
precedent for splitting rather than bundling, and a `six` win plus a `five`
loss is a live and expected outcome.

On the winning arm, re-run `probe-1nt-interference --bucket "2♦" --responses 6`:
the Pass row's `≤5 hcp, 6+ suit` class should shrink toward zero boards and the
`2NT`/`2♥`/`2♠` rows should absorb them without going negative.

#### Round 1 (2026-08-21) — the A/B never ran: the strip leaked the Multi table

Two launches, both killed by the pre-registered isolation gate. Neither
produced a headline; neither vulnerable half ran.

| run | `SEED_BASE` | boards/arm | divergent | foreign (`theirs` opened) |
| --- | --- | --- | --- | --- |
| `ab-results/2d-multi-escape-gatefail/` | 1787325027 | 230,400 (NV only) | 260 (0.11%) | **26 (10.0%)** |
| `ab-results/2d-multi-escape/` | 1787327781 | 230,400 (NV only) | 267 (0.12%) | **27 (10.1%)** |

Every foreign board is the same shape — `(1x) 1NT (2♦) …`, our `1NT` as an
*overcall*:

```
opened 1♦ by them
  on:  - 1♦ 1NT 2♦ 2♠ 3♦ 3♠ - - 4♦ - - -
  off: - 1♦ 1NT 2♦ 2♠ 3♦ 3♠ - - 4♦ - - 4♠ 5♦ X - - -
```

**Root cause — a reading seam, not the package.** `defensive()` grafts the
opening-1NT book below each `(1t) 1NT`, and `systems_on_overcall_strip` re-reads
the lane as `P* 1NT (2♦) …` so the advancer's Stayman/transfers decode off that
structure. But the strip re-keys into the **main competition book**, whose
`1NT (2♦)` leg is chosen Multi-or-natural at *build* time
(`defense_2d_multi`, `american/competition/lebensohl.rs`) — so the strip's
existing `profile.their.two_diamonds_multi = false` (`inference/read.rs`) stops
the hand reader but cannot un-compile the Multi table. The whole Multi leg,
N4e's floorless escape included, was published in a lane where their `2♦` is a
*response* to their own opening, and the inference-aware floor bid it:
partner's `2♠` in `1♥ 1NT 2♦ 2♠ 3♦` read `points 5..=8` at `None` and
`points 0..=8` at `Some(6)`.

**The graft was never the channel.** `register_one_nt` authors only uncontested
keys — `[1NT, 2♦]` is not even a prefix of the grafted trie — so nothing below
`(1x) 1NT (2♦)` is authored at all; every call there is the floor's, steered by
the reading. (A first attempt that cleared the disclosure inside the graft was a
no-op for the same reason: no build-time row in `register_one_nt` reads
`decision.their`.)

**Fix (2026-08-22).** The strip declines that one shape: under a declared
`their.two_diamonds_multi`, `(1x) 1NT (2♦) …` is not stripped, and the natural
walk reads their real diamonds instead. Default system byte-identical
(`smoke-default` `39ca60a2…`, and the declaration is off by default); pinned by
`multi_weak_escape_stays_out_of_the_overcall_lane`.

**What it costs, and the alternative.** Declining loses the *borrowed* natural
Lebensohl reading in that one sub-lane: with the Multi declared, partner's `2♠`
now reads `points 0..=37, ♠ 4..=13` where it read `5..=8, ♠ 5..=13` before, in
**both** arms. The arms therefore agree (the gate is the point), but the
anchor's absolute base in this lane moves. The alternative that keeps the
natural leg is a second, Multi-blind competition book carried on the
`Partnership` for the strip to read against (`opponents: Option<Arc<Partnership>>`
is the existing precedent) — ~50 lines plus a second bound book under N4
configs, and it must be built *after* the floors or floor-authored calls read as
nothing. Deferred, not refuted.

**Sibling, flagged not fixed.** `their.two_clubs_landy` is the same
misapplication one suit lower — `(1♦) 1NT (2♣)` gets the Landy counter-defense
though their `2♣` there is a response. Left at today's behavior deliberately:
clearing it moves the Landy campaign's measured base. User's call.

#### Round 2 (2026-08-22) — measured ×2 seeds: `six` ships, `five` is refuted

`sha=d54ef73f`, 24 shards × 9,600 = **230,400 bd/arm/vul**, `--filter-1nt
--their-2d-multi`, both vulnerabilities, arms sequential.
`SEED_BASE` **1787340263** (`ab-results/2d-multi-escape`) and **1787341972**
(`-s2`). Headline is IMPs per accepted deal — `--filter-1nt` is applied before
any bidding, so the 230,400 boards of an arm *are* the accepted deals.

**`six` (`Some(6)`) vs `base` — `plain wash | PD win`, replicated.**

| seed / vul | fired | DD plain | DD PD | sd plain | **SD-PD** |
| --- | --- | --- | --- | --- | --- |
| 1 / none | 244 (0.11%) | +174, +0.0008 ±0.0008 | **+312, +0.0014 ±0.0010** | +123, +0.0005 ±0.0008 | **+270, +0.0012 ±0.0010** |
| 1 / both | 183 (0.08%) | +29, +0.0001 ±0.0008 | +103, +0.0004 ±0.0010 | −70, −0.0003 ±0.0008 | +5, +0.0000 ±0.0010 |
| 2 / none | 207 (0.09%) | +38, +0.0002 ±0.0007 | +116, +0.0005 ±0.0009 | +43, +0.0002 ±0.0008 | +139, +0.0006 ±0.0009 |
| 2 / both | 165 (0.07%) | −2, −0.0000 ±0.0008 | +50, +0.0002 ±0.0010 | +32, +0.0001 ±0.0009 | +80, +0.0003 ±0.0010 |
| **pooled** | 799 | **+239, +0.00028 ±0.00039** | **+581, +0.00063 ±0.00049** | +128, +0.00013 ±0.00041 | **+494, +0.00052 ±0.00049** |

All four PD cells and all four SD-PD cells non-negative; **one** negative
reading in sixteen (sd plain, seed 1 vul, −0.0003, well inside its CI). The
knob's mechanism is *bidding more*, not doubling more, so the domain addendum
does not apply and PD is a real arbiter: this is the `wash | win` one-sided bet
— never loses on the honest scorer, gains when they punish — with the SD-PD
bracket agreeing at +0.00052 ±0.00049. **Shipped default-on.**

**`five` (`Some(5)`) vs `base` — the doubling-artifact row.**

| seed / vul | fired | DD plain | DD PD | sd plain | SD-PD |
| --- | --- | --- | --- | --- | --- |
| 1 / none | 525 (0.23%) | +429, +0.0019 ±0.0010 | +272, +0.0012 ±0.0013 | +453, +0.0020 ±0.0010 | +384, +0.0017 ±0.0013 |
| 1 / both | 428 (0.19%) | −25, −0.0001 ±0.0011 | **−377, −0.0016 ±0.0015** | +112, +0.0005 ±0.0011 | −156, −0.0007 ±0.0015 |
| 2 / none | 493 (0.21%) | +410, +0.0018 ±0.0009 | +206, +0.0009 ±0.0012 | +517, +0.0022 ±0.0010 | +383, +0.0017 ±0.0012 |
| 2 / both | 420 (0.18%) | −38, −0.0002 ±0.0011 | **−386, −0.0017 ±0.0015** | +47, +0.0002 ±0.0012 | −290, −0.0013 ±0.0015 |
| **pooled** | 1866 | **+776, +0.00085 ±0.00051** | **−285, −0.00030 ±0.00069** | +1129, +0.00122 ±0.00054 | +321, +0.00035 ±0.00069 |

A CI-clear plain win that PD erases, with the vulnerable PD cell **CI-clear
negative on both seeds** — the artifact row verbatim. Not shipped.

**`five` paired against `six` — `six` is the right rung.**

| seed / vul | fired | DD plain | DD PD | SD-PD |
| --- | --- | --- | --- | --- |
| 1 / none | 282 (0.12%) | +255, +0.0011 ±0.0006 | −40, −0.0002 ±0.0008 | +111, +0.0005 ±0.0008 |
| 1 / both | 246 (0.11%) | −54, −0.0002 ±0.0007 | **−480, −0.0021 ±0.0011** | −202, −0.0009 ±0.0010 |
| 2 / none | 289 (0.13%) | +371, +0.0016 ±0.0006 | +89, +0.0004 ±0.0008 | +220, +0.0010 ±0.0008 |
| 2 / both | 256 (0.11%) | −37, −0.0002 ±0.0008 | **−437, −0.0019 ±0.0011** | −262, −0.0011 ±0.0010 |
| **pooled** | 1073 | +535, +0.00057 ±0.00034 | **−868, −0.00095 ±0.00048** | −133, −0.00013 ±0.00045 |

The pre-registered split landed exactly as written: the five-card band is what
`lebensohl_relay_shape`'s PD-distilled floor was distilled against, and freeing
it buys plain-DD contracts that a competent doubler collects. **`Some(5)` stays
opt-in** — the same `Option<u8>`, third state.

**Gates.** After the strip fix, **8 of 12 pair-cells read 0 foreign** and four
read exactly **1**: `six:base` none 1/244 and `five:base` none 1/525 on seed 1,
`six:base` both 1/165 and `five:base` both 1/420 on seed 2. Every one is the
same board class — the campaign's **mirror-read leak** on the swapped axis:

```
- 1NT 2♦ 2NT - 3♣ 3♦ X - - -      (on)   3♦x South
- 1NT 2♦ 2NT - 3♣ - 3NT - - -     (off)  3NT East
```

*They* opened `1NT` and *we* overcalled `2♦`, but the trie key `P* 1NT (2♦) 2NT`
is shape-identical to ours and `Phase::of` on that prefix routes it to the
competition book, so their `2NT` decodes off **our** `multi_2d_responder` relay
row — which this knob widens (`points ..=8` reads `6..=8` off and `0..=8` on).
`readers.rs`'s hand reader has exactly this seat gate
(`their_disclosed_overcall` requires the `1NT` to be ours); the book node has
none, and the general cure is the declared-opponent book. Priced: the seed-1
NV board is worth **+3 of the divergent set's +174 plain / +312 PD**, so the
owner split moves NV plain +0.00076 → +0.00074 and PD +0.00135 → +0.00134 —
inside their CIs, no cell changes verdict. Quoted here rather than gated on,
by the same reading N1c shipped under at 1 of 132.

**Owed.** Re-run `probe-1nt-interference --bucket "2♦" --responses 6` on a
fresh anchor arm carrying the shipped default: the Pass row's `≤5 hcp, 6+ suit`
class should shrink toward zero boards and the `2NT`/`2♥`/`2♠` rows should
absorb them without going negative.

**N2d stays parked**, and now gets the pointer the queue row promised: `six`
shipped, so the `(2♠)` twin's case is a pointer to this round, not a run.

## N2 — Muiderberg `(2♥)/(2♠)`: the lane today

Their `2♥`/`2♠` show exactly five in the major plus a 4+ minor; we answer with
Cohen Transfer Lebensohl (`lebensohl.rs:466`, `rubensohl.rs:98`). Current
census: **`2♥` 409 bd / −99 plain / −0.24 per board** (PD +0.20/+0.84) and
**`2♠` 402 bd / −90 / −0.22** (PD +0.18/+0.48) — mid-table, and PD-positive at
both vulnerabilities in both lanes.

**Status.** The 2026-08-15 census by response put the lane's headroom in *our
own weak calls being unread*, not in BBA's plain Lebensohl, which earns nothing
at table B. Two of the three fixes shipped on 2026-08-16: **N2e**,
`instinct.forcing_ceiling_read` — the actual cause of opener's `3NT` over the
relay's minor sign-off was `opener_forced_past_invitation` forcing to game off
the *shape* of partner's call, not a lost ceiling — and **N2b**, the reading
half (`ReadingProfile::strength_ceilings` plus `ReadingScope::All`, which took
the whole `nt_blanket` question with it). **N2a**, a book node for
`{relay} 3♦ -`, stays **parked**: it would now shadow the floor that handles
that seat. The census tables, the mechanism, and the 2026-08-16 correction are
archived in
[§N2 — the pre-fix census](archive/one-notrump-competitive-closed.md#n2--the-pre-fix-census-2026-08-15).

**Re-read on the current arms** (`1e9a47e2`, `--bucket "2♠" --responses 8`,
NV+vul pooled, 402 boards). All three signs the 2026-08-15 census established
replicate:

| our response to `(2♠)` | bd | plain | PD | plain/bd |
| --- | ---: | ---: | ---: | ---: |
| **Pass** | 228 | **−231** | +73 | −1.01 |
| **`2NT` relay** | 45 | −32 | **−68** | −0.71 |
| `3♦` (→♥) | 33 | +37 | +29 | +1.12 |
| `3♠` cue (Stayman) | 29 | +46 | +55 | +1.59 |
| **`X`** | 62 | **+86** | +63 | **+1.39** |

`X` still wins, the relay still loses on both scorers, and Pass is a plain
loss that perfect defense nearly recovers (their `2♠` fails and PD doubles it).
Splitting the passes by hand class ranks the two open packages:

| why responder passed over `(2♠)` | bd | plain | PD | plain/bd |
| --- | ---: | ---: | ---: | ---: |
| `≤5 hcp, 6+ suit` — the relay's `hcp 6` floor (**N2d**) | 25 | **−77** | −52 | **−3.08** |
| `≤5 hcp, 5-card suit` — relay floor | 68 | −121 | −59 | −1.78 |
| `≤7 hcp, no 5-card suit` — nothing to say | 116 | −44 | +93 | −0.38 |
| `8+ hcp, 0-1 or 4+ in theirs` — no call at all (**N2c**) | 19 | **+11** | **+91** | +0.58 |

**N2d replicates** as the worst hand class in the lane, on both scorers.
**N2c does not**: the class that motivated it is now positive on both scorers,
so its queue row is demoted to *parked pending replication* rather than closed
— 19 boards is small enough that either sign is seed noise.

The reading defect the census exposed is the whole book's, not N2's; its
campaign is [authored-reading-handoff.md](authored-reading-handoff.md), with
this lane as its testbed.

## Measurement discipline

### The mirror-read leak — open defect, gated

`read.rs` gates its 1NT sites on parity **relative to the opener, not to us**
([read.rs:386-389](../src/bidding/inference/read.rs)), and `their_profile`
falls back to *our own* profile whenever no foreign book is declared
([read.rs:333-335](../src/bidding/inference/read.rs)) — every arm in this
campaign. So when **they** open 1NT and **we** overcall `2♣` (our own Landy),
their next call is read through *our* counter table: under the house rule that
a knob picks what we bid, not how we read their bidding, the symmetric
fallback quietly makes **every counter knob a reading knob as well**.

Measured at 21–43% of divergent boards across this package's A/Bs. The IMP
impact was neutral-to-depressing on every headline (no verdict flipped; the
our-opened subsets are the honest figures, and they were *stronger*). **Now
enforced**: `probe-divergence --gate-opener ours` exits non-zero unless every
divergent board was opened by our side (`theirs` for a defensive package).
Pure book edits (`e↔d`, `f↔e`) pass at zero foreign boards; cue-*constraint*
edits fail. Run it on every arm pair in this lane; the real fix is splitting
`their_profile` from our own.

- **Counter-defense isolation gate:** on identical seeded deals, configuring
  the candidate against a natural defense must leave the auction dump
  byte-identical to the natural baseline.  Require natural interference to
  occur and the targeted artificial-defense arm to diverge, so the check is
  not vacuous; a face-call-wide reinterpretation (`2♣` always means Landy)
  fails this gate.  **Ownership half, automated:**
  `probe-divergence --gate-opener ours` fails the pair unless every divergent
  board was opened by our side — the half that caught nothing until
  2026-08-14 because it was discipline with no implementation.
- **Enriched probing** is the default here: `bba-gen --filter-1nt` (raw-hand
  gate, balanced 15-17 somewhere, applied *before* any bidding). Headline is
  IMPs per **accepted** deal; publish `per-board = conditional mean × trigger
  density` alongside and scale the CI the same way. Compare IMPs/divergent, not
  IMPs/board.
- One knob = one measured change; arms **sequential**, fresh
  `SEED_BASE=$(date +%s)` per experiment shared across its arms, never rebuild
  in flight.
- Any **reading** change is a second, separate A/B on the same enriched boards,
  so a loss attributes to calls or to reading, never to their sum. A counter
  knob is *also* a reading change until the mirror-read leak above is closed.
- **Decompose a wash before theorising about it.**
  [`probe-divergence`](../examples/probe-divergence/main.rs) pairs two arm dirs
  already on disk and classifies every divergent board — who bid differently
  first, whether a game was reached in one arm only, whether declarer swapped
  sides, how much room the opponents got — with `--jsonl` for per-board records
  and `--imps` to price a bucket. It reproduced this campaign's published
  headline exactly and then split it into three populations with different
  signs. Counting needs no solver at all.
- Ship rule: standard gate. Plain-DD wash + PD gain ships default-on; a CI-clear
  plain loss stays opt-in with the default byte-identical, and the leak gets
  named in the ledger.

## Ledger

Every row here is **closed**. Open work is the [queue](#package-queue--open-work-ranked-by-the-census);
the full measurement trails are in the archive, linked per row. Numbers are the
final pooled verdict, IMPs per board unless marked per fired.

| Package | Knob (default) | Status | Final pooled verdict | Trail |
| --- | --- | --- | --- | --- |
| census tool | — | **shipped** | read-only; picked N1 over the pre-census guess | [§census](#the-census--what-each-interference-call-actually-costs) |
| N1 Landy `(2♣)` counter | `their.two_clubs_landy` — a disclosure, not a knob | **SHIPPED 2026-08-14** | `plain wash \| PD win`, confirmed at 3× n: NV plain −0.0002 ±0.0013 / PD **+0.0032 ±0.0017**, vul plain +0.0003 ±0.0015 / PD **+0.0028 ±0.0019**. v1 lost all six cells on unauthored opener answers (phantom Jacoby 82%, phantom Puppet 85%) | [closed §N1](archive/one-notrump-competitive-closed.md#n1--the-landy-2-counter-shipped-default-on-2026-08-14) |
| N1b GF minor cues | `defense_2c_landy_cues` (**off**) | measured ×4 2026-08-14, **opt-in** | v4 `win \| wash` — the artifact row: NV plain **+0.0016 ±0.0010** / PD +0.0001 ±0.0013, vul **+0.0014 ±0.0012** / −0.0000 ±0.0014. Decomposes into four independent effects that replicate across vuls | [closed §N1](archive/one-notrump-competitive-closed.md#n1--the-landy-2-counter-shipped-default-on-2026-08-14) |
| N1c club transfer + INV minors | `defense_2c_landy_transfer` (**on**, via the stack) | **SHIPPED DEFAULT-ON 2026-08-14** | `xfer↔on` ×2 seeds: plain **+0.0013 ±0.0007** NV / +0.0007 ±0.0008 vul, sd **+0.0018/+0.0011**; 1 of 132 divergences foreign | [closed §N1](archive/one-notrump-competitive-closed.md#n1--the-landy-2-counter-shipped-default-on-2026-08-14) |
| N1d/e/f cue repairs | `defense_2c_landy_cue_floor`, `_fit_answers`, `_competition` (all **on**) | **SHIPPED DEFAULT-ON 2026-08-14** | the package's first `win \| win`: `f↔on` ours-only NV plain **+0.00091 ±0.00052** / PD **+0.00077 ±0.00064**, vul **+0.00075 ±0.00058** / +0.00060 ±0.00070; 8/8 sd cells positive, no negative cell in 24 readings. **N1d is the engine** (cue→X, +2.0…+5.1 PD/fired) | [closed §N1](archive/one-notrump-competitive-closed.md#n1--the-landy-2-counter-shipped-default-on-2026-08-14) |
| N1g Landy read-side wiring | `reading.their_landy_reading` (**on**) | **SHIPPED DEFAULT-ON 2026-08-14** | `plain wash \| PD win` ×3 seeds: NV plain −0.00051 ±0.00072 / PD **+0.00104 ±0.00097**, vul +0.00001 ±0.00078 / PD **+0.00112 ±0.00104**. **Isolation gate 0 foreign — the campaign's first** | [closed §N1g](archive/one-notrump-competitive-closed.md#n1g--the-read-side-wiring-shipped-default-on-2026-08-14) |
| N1h / N1i minor-rung re-pricing | `defense_2c_landy_low_minors`, `defense_2c_landy_hcp_rungs` (**both off**) | **both REFUTED 2026-08-15, lane closed** | N1h `plain wash \| PD loss` (vul PD **−0.00081 ±0.00074**); N1i no CI-clear cell, all eight leaning negative. `cue ← X` negative in both, so **N1d's cue floor is settled — do not probe it again** | [closed §N1h / N1i](archive/one-notrump-competitive-closed.md#n1h--n1i--the-minor-rungs-re-priced-both-refuted-both-opt-in) |
| N1j BBA-ladder counter + weak-`2♦` cap | `defense_2c_landy_bba`, `defense_2c_landy_weak_2d_cap` (**both on**) | **both SHIPPED DEFAULT-ON 2026-08-15** | ladder at a **pre-pinned non-inferiority gate**: `wash \| wash`, all 16 DD+sd cells leaning positive (NV plain +0.00083 ±0.00085). Cap at the standard gate: NV PD **+0.00037 ±0.00033**, vul **+0.00050 ±0.00035**, **0 foreign** | [closed §N1j](archive/one-notrump-competitive-closed.md#n1j--the-bba-ladder-counter-shipped-default-on-2026-08-15) |
| N4 their `(2♦)` as a Multi | `their.two_diamonds_multi` — disclosure; engine default undeclared | **SHIPPED 2026-08-15, v7 of seven rounds** | v7 vs base ×3 seeds, owned: NV `plain wash \| PD win` (+0.00100 ±0.00067), vul plain **+0.00061 ±0.00056** \| PD +0.00061 ±0.00069, both-vul pool `win \| win`; paired vs v4 better on 3 of 4 cells. Every raw headline was 60–70% foreign — verdicts are owner-split | [§N4](#n4--their-2-as-a-multi-shipped-2026-08-15--v7-seven-rounds-default-on-vs-bba-via-the-census); [v1–v6](archive/one-notrump-competitive-closed.md#n4--measurement-rounds-v1v6) |
| N4 residue — Multi reader / stopper ask | `reading.their_multi_reading` (**on**), `competition.multi_stopper_ask` (**Off**) | reader **SHIPPED DEFAULT-ON 2026-08-16**; ask **REFUTED as a default** | reader `plain wash \| PD win` ×3 seeds — −29 plain / **+643 PD** over 1.3824m boards, 0 foreign on every pair. Both stopper modes landed on `plain win \| PD wash` (the artifact row) and tied with each other, so no combined arm ran | [§N4 residue](#n4-residue--reader-shipped-stopper-ask-stays-opt-in-measured-2026-08-16) |
| N4e floorless weak escape over their Multi | `competition.multi_weak_escape` (**`Some(6)`**) | **SHIPPED DEFAULT-ON 2026-08-22 at `Some(6)`; `Some(5)` REFUTED as a default** | `plain wash \| PD win` ×2 seeds, 921,600 bd per cell-set: pooled DD plain +0.00028 ±0.00039, **PD +0.00063 ±0.00049**, sd plain +0.00013 ±0.00041, **SD-PD +0.00052 ±0.00049** — all four PD and all four SD-PD cells non-negative, one negative reading in sixteen (−0.0003, inside CI). `five` is the doubling-artifact row (pooled plain **+0.00085 ±0.00051**, PD **−0.00030 ±0.00069**, vulnerable PD CI-clear negative on **both** seeds) and paired vs `six` is CI-clear worse on PD (**−0.00095 ±0.00048**). Two rounds were lost to the systems-on strip leaking the Multi table into the 1NT-overcall lane (26/260, 27/267 foreign); after the 2026-08-22 fix, 8 of 12 pair-cells gated **0 foreign** and 4 carried exactly **1** board of the campaign's mirror-read leak | [§N4e](#n4e--the-floorless-weak-escape-shipped-default-on-2026-08-22-the-six-card-rung-five-refuted) |
| N4b `(2♦)` diamond penalty double | `competition.two_diamond_double` (**`None`**) | **measured 2026-08-15 — sweep NULL, opt-in** | all 28 raw cells CI-clear positive and **all of it a leak** (84.9% foreign); owned subset has no CI-clear cell in 28. Two findings kept: the **alert is what makes a gate a reading**, and the "they sit 43%" claim was retracted (0 of 141 on our-opened boards) | [closed §N4b](archive/one-notrump-competitive-closed.md#n4b--the-2-diamond-penalty-double-built-2026-08-15-sweeping) |
| N3 `(3♣)`–`(3♠)` preempt of our 1NT | `nt_high_overcall_responses` (**on**), `nt_high_overcall_3nt_stopper` (**off**), `nt_3c_transfers` (**off**) | **SHIPPED DEFAULT-ON 2026-08-18** | owned plain **+0.00208 ±0.00126** NV / **+0.00293 ±0.00160** vul, PD +0.00079/+0.00180, sd agreeing on all four cells, **zero negative cells in sixteen readings**. The private `3NT` bit shipped **off** (+2.37/+1.65 per fired, 0 foreign); the BBA-style double continuation was **refuted and removed** (all eight cells negative) | [§N3 — measurement rounds](archive/one-notrump-competitive-closed.md#n3--measurement-rounds) |
| N3 opener's answer to the takeout `X` | `nt_high_overcall_x_major_at_four` (**on**), `_x_leave_in` (**on**), `_x_leave_in_three` (**off**) | fit rung **SHIPPED 2026-08-19**; leave-in **SHIPPED 2026-08-20** (v2, `len(over, 4..)`); honor half **REFUTED** | fit rung 4/4 DD cells CI-clear (+1.14/+1.69 per fired), 0 foreign of 1103/1340. Leave-in `length` **CI-clear positive in all eight cells** — plain +0.0071 ±0.0009 NV / +0.0104 ±0.0012 vul (+2.38/+3.41 per fired), 0 foreign — and **replicated on a fresh seed** at +0.0074/+0.0111 with every suit DD-positive, so no suit gate exists. `three` sd-lead **−2.44/−2.99 per fired**: bundled as one gate the package would have measured as a loss at both vuls | [§N3 — measurement rounds](archive/one-notrump-competitive-closed.md#n3--measurement-rounds) |

### Memory compaction notes (2026-08-16)

Moved verbatim to
[the archive](archive/one-notrump-competitive-closed.md#memory-compaction-notes-2026-08-16): the refuted stopperless
`3NT` escape gate, the opt-in gambling games over `1NT (X)`, the historical
ship commits, and the superseded statements to ignore if met in old notes.
