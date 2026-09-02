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
    ab-results/anchor/2026-08-27-3237e037/american-none \
    --dd-cache ab-results/anchor/dd-cache.json
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

### Current — arms `3237e037`, seed 1783375064, 204,800 boards/vulnerability (run 2026-08-27 local)

`ab-results/anchor/2026-08-27-3237e037/american-{none,both}`, the shipping
system, replaying 100.00% of our calls with 0 mismatches; whole-arm plain
**−0.4741 NV / −0.5753 vul** IMPs/board against BBA. This is the **main anchor
series** (seed 1783375064), not the `anchor-confirm` series the 2026-08-23
census read — a different seed, so the two tables are scale for each other,
never deltas. We open 1NT on **6.47%/6.68%** of boards and RHO contests
**12.4%/10.4%** of those, so a contested 1NT is **0.80%/0.69% of all boards**.
The three-level suits are split per RHO suit; `4+` is `3NT` and everything
above it, still one floor-only bucket.

| RHO | boards (NV+vul) | plain total | plain/bd | PD/bd NV | PD/bd vul |
| --- | ---: | ---: | ---: | ---: | ---: |
| `2♣` Landy | 551 | **−275** | −0.50 | −0.27 | +0.29 |
| `2♠` Muiderberg | 430 | −189 | −0.44 | +0.01 | +0.49 |
| **`4+`** (`3NT` and up) | 43 | −126 | **−2.93** | −2.29 | −2.74 |
| **`3♠` preempt** | 88 | −101 | −1.15 | −0.10 | −0.41 |
| `2♥` Muiderberg | 393 | −79 | −0.20 | +0.16 | +1.10 |
| `X` Woolsey | 364 | −62 | −0.17 | +0.88 | +1.20 |
| **`3♣` preempt** | 100 | −54 | −0.54 | −0.78 | +0.24 |
| **`3♥` preempt** | 85 | −6 | −0.07 | +1.03 | +0.30 |
| `2NT` unusual | 118 | +6 | +0.05 | −0.02 | +0.72 |
| **`2♦` Multi** | 794 | **+38** | +0.05 | +0.51 | +0.95 |
| **`3♦` preempt** | 89 | +58 | +0.65 | +0.80 | +0.73 |
| **all contested** | 3055 | −790 | −0.44 / −0.05 | +0.19 | +0.67 |
| **uncontested 1NT** | 23868 | — | **+0.13 / +0.04** | — | — |

**Three findings.**

1. **The lane's whole headroom is ~0.003 IMPs/bd.** Contested costs −0.57 NV /
   −0.09 vul relative to *uncontested*, on 0.80%/0.69% of boards — 0.0046 NV /
   0.0007 vul per board of the arm. Against a −0.47/−0.58 gap to BBA nothing
   here closes an anchor bucket; this is hygiene and disaster removal at the
   standard ship gate, as scoped. The [campaign](bba-gap-campaign.md) says the
   same thing structurally: `1NT (2♦)` fires on 1.17% of table-auctions, its
   `X` on 0.29%, its answer table on 0.06% — below the ±0.02 headline CI by
   construction.
2. **Contested 1NT is not a leak — it is now above the arm's average at both
   vulnerabilities.** −0.44/−0.05 against the arm's own −0.47/−0.58.
   Uncontested 1NT (+0.13/+0.04) is still the better board, and the 1NT
   opening is one of our better boards either way.
3. **`(2♦)` Multi has crossed to positive, and that half of the lane is
   paid-for work.** The largest bucket by boards (794) now reads **+38 plain**
   and +0.51/+0.95 PD. This one is a genuine *paired* move: the same seed's
   previous snapshot (`c5fbee11`, 2026-08-25) reads **−94 NV / +53 vul** on the
   same 439/355 boards, so N4-KK's answer table and the minor-transfer slam
   tries are worth **+79 plain / +42 PD** on this lane — and **every other
   bucket is byte-identical** across the two snapshots (only `2♣` NV moves, by
   2 IMPs). The ranking's top is now `2♣` Landy (**−275 on 551 bd**, −0.50/bd,
   PD −0.27 NV) and `2♠` Muiderberg (−189, −0.44/bd) — the two systems-on
   two-level buckets, both of which were mid-table on the previous seed. Per
   *board* the worst cells are still the rare ones — `4+` −2.93, `3♠` −1.15 —
   and inside `4+` the floor still offers no `X` at all (N3-x). Buckets from
   different seeds are not subtractable; only the `anchor`-series pair above is.

Superseded snapshots — the 2026-08-18 pre-N3 baseline that selected N3, and the
pre-N1 forensic on why the systems-on rebase lost to their Landy `2♣` — are in
[the archive's census history](archive/one-notrump-competitive-closed.md#census-history).

### Current `2NT` reading — small-n wash

The bucket is now **118 boards, +6 plain (+0.05/bd)**, PD −0.02/+0.72 NV/vul,
on per-board CIs of ±1.21/±1.62 that swallow it whole; the signs still
disagree — and the plain half has crossed to positive. The starting
snapshot's forensic pattern was that BBA **doubled** their minors and we bid
on:

```text
us:  1NT 2NT X 3♦ - - -
bba: 1NT 2NT X 3♦ - - X - - -
```

Four re-anchors later there is still no replicated loss — `53a3c254` read
+5 plain on 118 boards, `1e9a47e2` and `053c4fb8` read −26 on 131, and this
snapshot reads +6 on 118, all well inside their CIs — so N6 stays parked.

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
inventory above and by the [ledger](#ledger). The pre-N4e census's top bucket,
`(2♦)` at −744 plain, was re-decomposed on 2026-08-21; its one open item, **N4e**,
**shipped default-on 2026-08-22** and has moved to the [ledger](#ledger)
([§N4e](#n4e--the-floorless-weak-escape-shipped-default-on-2026-08-22-the-six-card-rung-five-refuted)).
N4's v7 counter and its reader had shipped against the *strong* half of that
bucket and its residue was measured out; N4e was responder's sub-5-HCP outlets,
never authored at all. The post-ship replay moves the bucket to −689 plain / +29
PD and closes N4e's owed probe. **N4f** (2026-08-22) then built and measured the
bucket's last named hole and its two reading defects; all three knobs remain
default-off
([§N4f](#n4f--openers-balancing-seat-and-the-two-reading-knobs-measured-2-rounds-2026-08-22-nothing-ships-all-three-stay-opt-in)).
The N4f retrain trigger fired on 2026-08-23, but its BBA-reading v6 twin missed
the pre-registered plain-DD bucket gate and remains opt-in
([retrain round](#the-retrain-trigger-fired-2026-08-23--pd-gain-plain-target-missed-no-ship)).

**N4 is closed (2026-08-23).** Every named item in the `(2♦)` Multi lane has
shipped, been refuted, or been priced below this harness's resolution, and
nothing in it awaits a decision. Three residues are **recorded, not open**:
`competition.multi_balance` is *unresolved below resolution* — ~18 bd of reach
per 230,400, which wants a sub-lane harness or an sd re-measure, **not another
seed**; the two reading knobs are **trigger-gated**, and the first
contested-floor retrain to consume them failed the N4 plain-DD gate, so they
remain off (§N4f keeps the withdrawn correctness-only proposal on record);
and responder's natural **minor** single-suiter over their Multi — 3.9% of
BBA's hands, [§N4f Phase 0](#phase-0-first-two-probes-re-ranked-the-package-before-any-box-was-spent)
— is unbuilt and priced at ≈zero, because opposite 15–17 the contract is
usually `3NT`. **N4-mirror**, the biggest number the lane produced, left for
the defensive campaign and shipped there as M1+M2. Both rows have moved to the
[ledger](#ledger); reopening N4 needs a new bucket, not a new seed.

**And one arrived, and shipped (2026-08-25).** **N4-KK** is not another rung of
the shipped table but a *different published table* for the same object — the
Kokish–Kraft notes, the most complete exact-object package the
[survey](ai-bidder/multi-landy-2d-counter-defense-research.md) found. Its first
A/B failed the isolation gate on the mirror-read leak; the **mirror book**
closed that, and the fresh-seed re-measure gated **0 foreign at both vuls** and
read `win | win` at both-vulnerable over a `wash | wash` NV, so K–K is now the
**default** table against a declared Multi. Residues 3, 4 and 6 are its
follow-up queue ([§N4-KK](#n4-kk--the-kokishkraft-counter-a-whole-table-variant-shipped-default-on-2026-08-25)).

**Re-ranked at `3237e037` (2026-08-27).** The `(2♦)` Multi bucket has crossed
to **+38 plain** and is no longer the lane's top cost; the two open cells the
new census promotes are `4+` (**−2.93/bd**, the worst per-board cell on either
scorer — that is **N3-x**, already queued) and, by total, the two systems-on
two-level buckets `2♣` Landy (−275) and `2♠` Muiderberg (−189), neither of
which has an open package. Both are shipped, tuned lanes whose totals sit
inside per-board CIs of ±0.47–0.76, so this is a re-rank to watch, not a new
package — a fresh seed can reorder them again.

| # | Package | Knob | State |
| --- | --- | --- | --- |
| **N4-KK** | **the Kokish–Kraft whole-table counter to their `(2♦)` Multi** — five changes at once: `X` at `hcp 8+` with no shape promise, a neutral pass with its own delayed *takeout* double, floorless `2NT`→♣ / `3♣`→♦ transfers, `3♠` both minors GF, a *penalty* repeated double, and the uncontested `4M` slam-try tier | `competition.multi_kokish_kraft` (**on**) | **SHIPPED DEFAULT-ON 2026-08-25** — re-measured on a fresh seed after the mirror book fixed the residue-1 leak (`SEED_BASE 1787615025`, SHA `f2ecb3c6`, 230 400 bd/arm/vul): **isolation gate 0 foreign at both vuls** (0/683 and 0/482, against a 55% prior rate), both-vul is the decision table's `win | win` row — plain **+0.0019 ±0.0013** / PD **+0.0023 ±0.0017** (+0.907/+1.102 per fired) — over a `wash | wash` NV (+0.0002 ±0.0012 / +0.0012 ±0.0015), sd-lead agreeing in all four cells and **no negative reading in eight**. Two design-sketch ordering repairs recorded at the rule (`3♠` above `3NT`; `3NT` keeps its stopper gate, else the values double collapses to `points 8..9`), each a one-line reversible sub-arm; residues 3/4/6 are the follow-up queue ([§N4-KK](#n4-kk--the-kokishkraft-counter-a-whole-table-variant-shipped-default-on-2026-08-25)) |
| **N1-lia/rail** | **the envelope-gated new-suit veto on the learned floor** — the *general* fix this lane is parked behind, not a lane node. `new_suit_gate` masks a floored suit bid on at most four cards with at most five announced combined, in every unauthored seat of the default system | `decision.instinct.new_suit_veto` (**off**) | **MEASURED AND REFUTED 2026-09-02** — plain DD −0.0212 ±0.0047 (none) / −0.0164 ±0.0057 (both), PD a wash, seed 1788352713, 204,800 bd/arm/vul; stays opt-in, default byte-identical. Design and evidence in [ai-bidder/new-suit-veto.md](ai-bidder/new-suit-veto.md); jdh8's direction was "try the suppress first — if a general fix happens to solve a local problem, we don't need code for that local problem". **the park is therefore void** — the general fix does not solve the local problem, so §N1-lia's lia4 book changes stand on their own, unblocked and unhelped |
| **N3-x** | **`X` over their `(4x)`** — the floor cannot double above the three level at all (`their_live_bid_at_most(3)`, [instinct.rs:6058](../src/bidding/instinct.rs)), and BBA's advancer sits for our double on **96.7–99.9%** of hands | new (book) | Current `4+` bucket 43 bd / −126 plain / −107 PD, **−2.93/bd** — the lane's worst cell per board on either scorer, on a CI (±3.58/±4.15) that swallows the total. The `(3x)` template does **not** widen — their four-level overcalls are *eight*-card suits, six times rarer than the three-level rows — so this is an uncontested opportunity to size, not a copy |
| **N2d** | relay with a 6+ suit below 6 HCP, over `(2♠)` only (the weak major has no two-level call there) | book | **Re-read 2026-08-21: 25 bd, −77 plain / −52 PD, −3.08/bd** — still the worst hand class in the lane, and negative on both scorers. Contradicts the PD-distilled floor ([`lebensohl_relay_shape`](../src/bidding/american/competition/lebensohl.rs)); against Muiderberg the alternative is a making `2♠`, and BBA at table B bids these hands un-overcalled. Needs the A/B, not a re-derivation |
| N5 | Complete Jacoby, then re-measure | `competition_over_transfer` | default-off on a measured loss *while missing its two most-fired cells* — `(2♥)` over our `2♦` and `(2♠)` over our `2♥`, i.e. they bid the major we are transferring to ([over_our_jacoby.rs:100-103](../src/bidding/american/competition/over_our_jacoby.rs)) — a half-built loss, resumable |
| N3-fit | fresh-seed confirmation of the `4M` fit rung | `nt_high_overcall_x_major_at_four` (**on**) | Round 5 shipped it on one seed (+1.14/+1.69 per fired, 0 foreign) and explicitly owed a confirmation. Rounds 7–8 ran against a `base` that *carries* it, which is not an isolated confirmation — the row is shipped but not settled |
| N6 | `(2NT)` penalty discipline | `uvu_encircle` et al. | 118 bd, +6 plain, CI ±1.21/±1.62 per board and the signs disagree by vulnerability — no replicated loss on any re-anchor since the starting snapshot. Mechanism stays priced: **BBA doubles 46.7%** and cues `3♣` for both majors ([reference](ai-bidder/bba-1nt-counter-defense.md)). Parked |
| N3-three | single-dummy re-measure of the refuted honor half | `nt_high_overcall_x_leave_in_three` (**off**) | Round 7 refuted it at sd-lead −2.44/−2.99 per fired; its `plain wash \| PD win` DD signature is the doubling artifact. Kept opt-in as a house-rule re-measure candidate on its vulnerable PD reading |
| N3-xfer | re-measure the `(3♣)` transfers | `nt_3c_transfers` (**off**) | Two seeds at round 2 and two more at round 3, all four pooled cells positive and all four an order of magnitude inside their CI. Filed as a single-dummy-harness item because what it buys — the invitational five-card major, and right-siding — was believed DD-blind. **Requeued as a plain-DD item 2026-09-01**: the completion moves declarer, and §N1-lia package C measured that half on plain DD |
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

N1j's wide minor transfers are `2NT`→♣ and `3♣`→♦ on a six-card suit and
`points(2..)`: weak signoff through game force, not literally zero-strength.
After the forced completion, `4m` on `points(13..) & len(minor, 6..)` is the
slam try in
[`landy_bba_transfer_rebid`](../src/bidding/american/competition/lebensohl.rs).

**The answer shipped default-on 2026-08-25.** With
`competition.landy_minor_slam_answer = true`, opener bids `4NT` RKCB on
`hcp(16..)` and `5m` otherwise; the full keycard ladder is authored after both
their pass and their double. Its isolated 2.304M-board/arm/vulnerability A/B
had 0 foreign divergences and won all plain-DD, perfect-defense, and sd-lead
cells: plain +5.56/+10.11 and PD +7.11/+11.44 IMPs/fired NV/vulnerable (n =
18/9). The former floor-owned answer remains the explicit off arm. Design,
probe, and measurement: [minor-transfer-slam.md](minor-transfer-slam.md).

### N1l — the doubler's own rebid (`landy_doubler_rebids`, **measured 2026-08-28: mixed, stays off**)

Salvaged from `park/landy-kk` as its own default-off knob; the branch's other
three (`landy_splinter_hcp`, `landy_tail_completion`, `defense_2c_landy_kk`)
stay parked. Every weight and gate below was re-derived against `main`'s code
rather than carried over from the branch's tables.

The seat: our doubler's own rebid after the values `X`, once their advance has
named the major — `1NT (2♣) X (2♥) - -`, `X (2♠) - -`, and the two legs where
their artificial `2♦` escape was pulled to a major. The 2026-08-27 census
prices the branch at 67 bd / −75 IMPs plain, the auction dying after our double.

**The polarity rule this authors, and why it needs a node.** Our subsequent `X`
is penalty after our own `X`, and stays the floor's takeout after our `P`.
Nothing mechanises that. `inference::readers::penalty_x_reading_with_profile`
requires **their** 1NT opening — it scans forward from `opening_index` and
returns `None` on the first non-pass that is a bid — so it returns `None` at
`1NT (2♣)` exactly as it does at `1NT (2♦)`, and `penalty_latch` (default on)
therefore cannot latch anything in either lane whatever its setting. `pdi_latch`
is default off, reading-only and measured inert (`docs/pdi.md:284`). The `1NT
(2♦)` twin's penalty meaning is 100% authored node — the rule's own
`len(major, 4..)` read back through the ordinary projection plus
`.alert(MULTI_PENALTY)` — and this lane copies that, not a latch.

**The table** (`landy_doubler_rebid`, all four legs), `kokish_kraft_doubler_rebid`'s
ladder ported one suit down:

| call | w | gate | note |
| --- | ---: | --- | --- |
| `4NT` | 160 | `hcp(16..)` | quantitative; answer = `multi_quant_answer` (`6NT` on 17+) |
| `X` | 155 | `len(their major, 4..)` | **penalty**, `LANDY_PENALTY` + `.penalty()`; opener sits (`multi_signoff_pass`) |
| `3NT` | 150 | `points(10..) & stopper_in(major)` | |
| `2NT` | 145 | `hcp(8..=9) & stopper_in(major)` | invite; answer = `kokish_kraft_invite_answer` (`3NT` on 16+) |
| `3♣`/`3♦` | 100/99 | `len(minor, 5..)` | natural — replaces the twin's other-major rung, which this opponent's 4-4+ major shape makes impossible |
| `P` | 0 | catch-all | |

Two structural differences from the twin, both probe-driven. There is **no
`ran` fork**: the Landy overcaller passes the preference 94.5%/96.7% of the
time (probed at seat 1, hands filtered to the ones BBA actually overcalls `2♣`
with), so the preference is final and there is no correction to fork on. And
the twin's weight-100 *other major* becomes a natural **minor**, because this
opponent holds both majors; that rung is also the only route for an 8–9
one-suited minor, the wide transfers above it being game-forcing.

**The top two rungs are dead in self-play, and are kept anyway.** Unlike the
Multi twin — whose `3NT`@150 needs *both* major stoppers, so a one-stopper game
hand really does double first — `landy_bba_responder` carries an **ungated**
`3NT`@168 on `points(10..)`. Every 10-plus-point hand bids `3NT` directly and
never doubles, capping the double at nine points. Verified, not assumed:
`probe-call-reading --their-2c-landy --ns-landy-doubler-rebids` reads partner
back as `points 8..9` at every rung of this table. So `4NT`@160 and `3NT`@150
can only fire opposite a partner not bidding this table. They stay because the
table must be **total** — deleted, a strong hand here would take `Pass`@0,
strictly worse than the floor this node shadows. What fires in self-play is
`X` / `2NT` / `3♣` / `3♦` / `Pass`.

**Verified on the shipped tree.** `probe-decision` prints `fallback: Some(0)`
at all four nodes with the knob off and a depth-2 book node with it on;
`probe-call-reading` reads the penalty `X` as `♥ 4..13` and the `2NT` below it
as `♥ 0..3` (denying the double it declined); `smoke-default --count 20000
--seed 1` is byte-identical to `main`.

**As measured (2026-08-28).** `scripts/ab-landy-doubler-rebids.sh`,
`SEED_BASE 1787917699`, sha `ba003a30`, 4,608,000 bd/arm/vul (24 × 192,000),
isolation gate **0 foreign at both vuls**; fired 1.56% none / 1.26% both.
IMPs/fired; every /board CI ≤ ±0.0008:

| cell | DD plain | DD-PD | SD plain | SD-PD |
| --- | ---: | ---: | ---: | ---: |
| none | **+2.365** | −0.759 | **+3.059** | **+0.523** |
| both | **+1.556** | −2.281 | **+2.539** | **−0.741** |

The knob is mixed-direction (one rung adds doubles, the rest bid more), so the
row read is per rung — `probe-divergence --jsonl --imps` on each cell, split by
`call_on`, DD-priced, ±95% CI on the /fired mean:

| rung | none: plain / PD | both: plain / PD |
| --- | --- | --- |
| penalty `X` (~12%) | **+7.489**±.083 / −0.091 | **+9.196**±.094 / −0.148 |
| `2NT` invite (~36%) | +1.888±.055 / −1.667 | +0.733±.088 / **−3.695** |
| `3♣` + `3♦` (~46%) | +1.7..2.0 / −0.6..−0.8 | +0.2..+0.3 / −2.5..−3.1 |
| `-` (table passes, floor bid) (~6%) | −0.445 / +2.929 | +1.206 / **+5.048** |

Falsifier 3 is **refuted**: the penalty `X` is the payoff, not the artifact —
it carries the *entire* vulnerable plain win (+63,332 of +90,066 IMPs), the
addendum arbitrates it on plain DD, and even its double-blind PD column is
flat. Falsifier 1's "small win" shape did arrive, but concentrated: the
authored ladder beats the floor's improvisation exactly where it doubles. **The
drag is the constructive family, vulnerable.** Every constructive rung is
win/loss on the ordinary rows at none but wash-to-thin/loss at both; the
arbiter (SD-PD) flips sign with vulnerability; and the `2NT` invite's declined
half (`2NT` passed out, n=10,051) loses **both scorers** at both vul
(−0.612±.087 plain / −4.313±.130 PD) — the measured mistake is declaring a
thin vulnerable notrump part-score instead of defending their `2♥`, not the
accepted `3NT`s alone.

**Verdict: mixed — not shippable default-on as built; the knob stays off.**
The flip plan is cheap and finished-code-shaped: keep `X`@155 and the
catch-all, tighten or vulnerability-gate the constructive rungs (the `2NT`
invite first), re-measure. A vulnerable band hand would then defend via
`Pass`@0, which the `-` row prices positive on both scorers at both vul
(selection-biased — those are hands the floor chose to bid — but the sign is
encouraging). Opener's max-with-stopper `3NT` jam one seat earlier was
considered against this data and stays with flagged item 1: opener already
declares every notrump ending in both arms (opener bid `1NT` first), so a jam
moves nothing on same-contract boards, and its two live deltas — thin games
added opposite the 7-point doublers, doubler penalty-`X` boards removed —
both point the wrong way here.

### N1l-flip — the two cut-down arms (`landy_doubler_px` **SHIPPED DEFAULT-ON 2026-08-29** / `landy_doubler_white` **not a win, stays off**)

The measurement above is a verdict **per rung**, so the flip is a choice of
*subset*, not a new table. `landy_doubler_rebid` takes a `DoublerLadder` and
three knobs name three subsets of the same four nodes:

| arm | knob | rungs, top to bottom |
| --- | --- | --- |
| `px` | `competition.landy_doubler_px` | `X`@155 `len(major, 4..)` · `Pass`@0 |
| `white` | `competition.landy_doubler_white` | `X`@155 · `3NT`@150 `points(10..) & stopper_in` · `2NT`@145 `hcp(8..=9) & stopper_in & !vulnerable()` · `3♣`@100 / `3♦`@99 `len(minor, 5..) & !vulnerable()` · `Pass`@0 |
| `full` | `competition.landy_doubler_rebids` | the ladder as measured, kept as the comparison arm |

**The axis is vulnerability, not the rung.** Re-reading
`div.reb.vs.base.*.jsonl` grouped by first differing call — the plan's
"adjust before building" step — moved the design. Per fired, plain / PD:

| rung | share | non-vulnerable | vulnerable |
| --- | ---: | --- | --- |
| `2NT` | 36% | +1.888 / −1.667 | +0.733 / **−3.695** |
| `3♣` | 25% | +1.953 / −0.797 | +0.338 / −3.071 |
| `3♦` | 21% | +1.696 / −0.607 | +0.183 / −2.481 |
| `X` | 12% | **+7.489** / −0.091 | **+9.196** / −0.148 |
| `-` (table passes) | 6% | −0.445 / +2.929 | +1.206 / **+5.048** |
| `3NT` | 0.02% | −2.105 / −3.579 (n=19) | +0.222 / −0.222 (n=9) |
| `4NT` | 0% | never fires | never fires |

Every constructive rung flips sign with colour, and the natural minors are the
*cheaper* half white, not the drag: `3♦` costs −0.607 PD per fired against the
`2NT` invitation's −1.667. So the first sketch of this flip — delete the
minors, gate the `2NT` — would have kept the worst white rung and dropped the
best two. `white` gates the whole family instead. Only `4NT` is deleted
outright (it never fired in either cell, because responder's ungated `3NT`@168
caps this double at nine points), and `3NT`@150 stays ungated as the table's
only game rung on 28 fires in 9.2M boards.

**Keeping the minors pays §N1l's completeness debt.** `{path} 3♣ -` and
`{path} 3♦ -` were the Multi-twin hole — authored rungs with a floor-owned
continuation. `landy_minor_rebid_answer` closes it: responder is capped at 8–9
and bid the minor *below* the `2NT`@145 invitation, which denies the stopper
that rung requires, so the stopper must be opener's and 16 opposite 9 is the 25
that bids the game — `3NT`@100 on `hcp(16..) & stopper_in(major)`, else `Pass`.
Total. Note this makes the `full` arm no longer bit-identical to the one
measured on 2026-08-28: it gains the same two answer tables. Its historical
numbers were measured without them.

**Answer tables move with their rungs.** `{path} X -` (opener sits for the
penalty double) is registered by every arm, because every arm carries the `X`;
`{path} 2NT -`, `3♣ -` and `3♦ -` by `white` and `full`; `{path} 4NT -` by
`full` alone. A node with finite mass shadows the floor
([bidding-architecture.md](bidding-architecture.md)), so an answer to a
question no arm asks is not merely dead — it is a live book node standing in
the floor's way.

**The vulnerability gate is real plumbing.** `vulnerable()`
(`constraint.rs`) reads `Context::vul()` and is already used by
`points_by_vul`; `& !vulnerable()` on each constructive constraint is the whole
gate, and it renders in the disclosure (`8–9 HCP, stopper in ♥, and not
(vulnerable)`). It is *our* side's vulnerability, the axis the measurement
moved on.

**What the A/B can and cannot separate.** The arms are generated at `-v none`
and `-v both` only, so the gate makes `white` ≡ `full`-minus-`4NT`-plus-answers
in the white cell and `white` ≡ `px`-plus-`3NT` in the red one. The white
`white vs px` pair therefore isolates the whole constructive family cleanly,
and `probe-divergence --jsonl --imps` split by `call_on` separates the rungs
inside it for free; the red pair is a near-empty consistency check.

**Falsifiers** (`scripts/ab-landy-doubler-flip.sh` states them in full): (1)
the `X` win was selection — both subsets were *chosen from* seed `1787917699`'s
split, so a fresh `SEED_BASE` is mandatory and a flat `px` closes §N1l; (2) the
attribution was wrong and the ladder wins as a whole — read the `white vs px`
pair; (3) vulnerability is the wrong axis and the constructive family is simply
bad; (4) `px` is a pure doubling knob, so plain DD arbitrates and its PD row
keeps the whole cost of the doubles with none of the benefit.

#### The verdict (2026-08-29)

`scripts/ab-landy-doubler-flip.sh`, fresh `SEED_BASE=1787942099`, sha
`de59ad86`, 4.608M boards per arm per vulnerability, **all six isolation gates
0 foreign**.

| pair | vul | plain DD | DD-PD | sd-plain | SD-PD |
| --- | --- | ---: | ---: | ---: | ---: |
| **`px` vs base** | none | **+0.0107** ±.0004 | **+0.0039** ±.0003 | +0.0061 | +0.0000 ±.0003 |
| **`px` vs base** | both | **+0.0142** ±.0004 | **+0.0061** ±.0003 | +0.0100 | **+0.0028** ±.0003 |
| `white` vs base | none | +0.0409 ±.0006 | **−0.0091** ±.0007 | +0.0547 | +0.0140 ±.0007 |
| `white` vs base | both | *(≡ `px`)* | | | |
| `white` vs `px` | none | +0.0302 ±.0005 | **−0.0132** ±.0007 | +0.0484 | +0.0138 ±.0007 |
| `white` vs `px` | both | 0 fired | 0 | 0 | 0 |

**`px` ships default-on.** Every column is a win except the one non-vulnerable
SD-PD wash — the decision table's `win | win` row, with no need for the domain
addendum's rescue.  Off-switch `--no-ns-landy-doubler-px`.

**`white` is not a win and stays off.** `win | loss` at both readings of the
non-vulnerable cell, and vulnerable it is the same arm as `px` (`white vs px`
fires on **0** boards, so the gate leaves only the `3NT`, which responder's
ungated `3NT`@168 caps out of existence).  The sd bracket dissents — sd-plain
+0.0547, SD-PD +0.0140 — which is not nothing in a 1NT lane, where DD's
killing lead is the documented bias and runs against the arm that *declares*
notrump.  A conflict, not a clearance.

#### The falsifiers, answered

1. **Selection — refuted.**  Split by first differing call on the fresh
   stream, the `X` rung prices **+7.554** (none, n=8,341) / **+9.189** (both,
   n=7,007) IMPs/fired plain, against the **+7.489 / +9.196** that selected
   it — inside 1%.  Its PD row stays flat (−0.129 / −0.188), the domain
   addendum's signature.  The rung is real.
2. **Whole-ladder attribution — refuted.**  Vulnerable the pair is empty by
   construction; non-vulnerable `white vs px` reproduces the split's sign
   pattern and sharpens it.  The rungs do not rescue each other.
3. **Colour is the wrong axis — open, and the gate is now known to be
   under-specified.**  `!vulnerable()` reads **our own** vulnerability only,
   and the A/B spans just the symmetric diagonal.  Favourable and unfavourable
   are unmeasured and are where a constructive/doubling split should diverge
   most.  A retry owes a *relative* gate and the two asymmetric cells
   (jdh8, 2026-08-29).
4. **PD blindness — moot.**  `px` wins DD-PD outright at both colours, so the
   excuse was never needed.

#### Two caveats, measured and recorded

**The `Pass`@0 catch-all is wrong and shipped anyway.** It gives the node
finite mass at three trumps or fewer, so it shadows a floor that was *already
acting* there.  Summing every `call_on == "-"` row of the divergence split, the
suppression costs **−14,171 IMPs plain non-vulnerable** (+889 vulnerable, a
wash) — enough to take the white cell from +0.0107 to roughly **+0.0138**.

What the floor does there is **not** a penalty double.  Opener pulls it:

| our length in their major | pulled to `3NT` | `2NT` | `3♦`/`4♠` | converted for penalty |
| --- | ---: | ---: | ---: | ---: |
| 2 (n=2,461) | **49.5%** | 11.0% | 7.7% | 16.7% |
| 3 (n=1,719) | **49.6%** | 17.3% | 18.5% | ~4% |

It is a takeout-shaped values double that **opener answers by declaring
notrump** — which is independently what the oracle's third reading says is
right (`@op` beats `@dbl` in all 36 buckets).  So the lane already had a
working mechanism nobody authored, and both flip arms break it in opposite
directions: `px` ends the auction, `white` rebuilds the same games from the
**wrong side**.  That is the leading candidate mechanism for `white`'s DD-PD
row, and it is untested.

Deleting the catch-all is therefore the **owed follow-up arm** — and it is not
shipped here because of the second caveat.

**Disclosure.** `comp:landy-penalty` publishes *four-plus* of the major their
advance named.  That is honest for the book rung.  With the catch-all gone the
same `X` at the same seat would *also* be the floor's takeout double on three
or fewer, under one published reading claiming four-plus — the phantom-suit
class.  The catch-all currently keeps the floor off that seat, so the conflict
is latent; the no-catch-all arm owes this tag a decision before it can ship.
Recorded in the precedent block in
[`src/bidding/card.rs`](../src/bidding/card.rs).

**Where `white`'s loss actually lives.** `div.white.vs.px.none.jsonl` split by
the rung `white` bids and `px` passes (61,651 divergences, all solved):

| rung | n | share | plain/fired | DD-PD/fired | share of the −60,805 |
| --- | ---: | ---: | ---: | ---: | ---: |
| `2NT` | 27,105 | 44.0% | +2.048 | **−1.921** | **85.6%** |
| `3♣` | 17,878 | 29.0% | +2.407 | −0.381 | 11.2% |
| `3♦` | 16,668 | 27.0% | +2.442 | −0.115 | 3.2% |

The invitation is 44% of the traffic and **86% of the damage**; the natural
minors are plain-positive at +2.4 and PD-neutral to within a rounding error
(`3♦` −0.115).  `3NT` does not appear at all — it fires 28 times in 9.2M
boards, as designed.  So §N1l's *first* sketch had the rung right and the verb
wrong: the `2NT` is the drag, and deleting it (rather than gating the whole
family on colour) is the arm the data now points at — plain ≈ +0.0182,
DD-PD ≈ −0.0019, roughly a sevenfold reduction in the PD cost.  It is still a
`win | loss`, so it does not ship on this evidence either; it is the natural
partner to the right-siding question above, since the `2NT` is exactly the rung
that declares notrump from the wrong hand.

**Not evidence:** BBA's behaviour at *this* seat is unknown.  The
`opener-c-x2h`/`opener-c-x2s` probes read **opener's** seat (§N1m), and the
*"bidable suit"* label is on our **first** `X`.  A position-6 probe is
unrun.

### N1m — **opener's** own rebid over their advance (`landy_opener_px` / `landy_opener_rungs`, **built 2026-08-29, A/B owed**)

`1NT (2♣) X (2♥)` and `X (2♠)` — the seat §N1l's is one call later, and the
seat **§N1k authored a `3NT` at, lost on 2026-08-27, and gave back to the
floor**. Flagged item 1 proposed leaving it there and re-opening it "only as
its own arm after §N1l's verdict". This is that arm, and it is designed off a
probe rather than off a hunch.

#### The oracle (Phase 0)

`examples/probe-landy-opener-oracle/` streams an existing arm dump, keeps the
~2% of boards that reach the seat, solves them, and prices **every contract
opener could steer to** against the contract our live method actually reaches:
defend their major undoubled, defend it **doubled**, `2NT`/`3NT` from either
side of our partnership, a natural `3m`, the unnamed major's `3OM`, and par.
The cut is the design question stated as buckets — opener's length in *their*
major × a stopper in it × opener's HCP.

```text
cargo run --release --features serde --example probe-landy-opener-oracle -- \
    ab-results/landy-doubler-rebids/base-none \
    --dd-cache ab-results/landy-opener-oracle/dd-cache.json --min 50
```

Run on the §N1l base arms (`SEED_BASE 1787917699`): **103,653** seat boards
non-vulnerable (2.25% of 4.608M) and **81,023** vulnerable (1.76%), 105,334
distinct deals solved. Reports in `ab-results/landy-opener-oracle/`.

**What it can and cannot say.** Every candidate is priced as *the contract
opener's call leads to if the auction stops there*: the oracle prices
contracts, not auctions. It cannot see partner pulling, their advancer running
from the double, or the information a bid leaks. It is an upper bound per rung
and a reliable *ordering* between rungs on the same boards. The tell is in the
data itself — `2Mx` beats **par** in the four-trump buckets, which is only
possible because par lets them escape and the oracle does not.

**The seat is the floor's and the floor passes it.** 98.5% non-vulnerable /
99.5% vulnerable, the rest a natural `3♦` on ~1%. That is §N1l's "the auction
dies after our values double", one call earlier.

#### What the oracle says

Plain-DD IMPs/board over today's floor, the **stopper-in-their-major** rows
(the no-stopper rows differ by less than half an IMP except in the `3NT`
column, and never change an ordering):

| len × hcp | `2Mx` | `2NT`@op | `3NT`@op | par |
| --- | ---: | ---: | ---: | ---: |
| **none-vul** 2 × 15 | −3.666 | **+2.130** | +1.137 | +2.659 |
| 2 × 16 | −2.352 | **+2.443** | +2.311 | +3.947 |
| 2 × 17 | −0.992 | +2.491 | **+3.697** | +5.241 |
| 3 × 15 | −1.257 | **+1.371** | +0.478 | +2.900 |
| 3 × 16 | +0.657 | +1.560 | **+1.852** | +4.126 |
| 3 × 17 | +2.462 | +1.462 | **+3.363** | +5.231 |
| **4+** × 15 | **+3.524** | −0.608 | −1.415 | +1.810 |
| **4+** × 16 | **+5.265** | −0.403 | +0.297 | +2.946 |
| **4+** × 17 | **+6.754** | −0.343 | +1.851 | +3.800 |
| **both-vul** 2 × 15 | −4.546 | **+0.441** | −0.658 | +2.287 |
| 2 × 16 | −2.909 | +0.813 | **+1.325** | +4.016 |
| 2 × 17 | −1.418 | +0.973 | **+3.478** | +5.784 |
| 3 × 15 | −1.219 | −0.550 | −1.146 | +2.620 |
| 3 × 16 | +1.091 | −0.458 | **+1.099** | +4.254 |
| 3 × 17 | +3.327 | −0.861 | **+3.334** | +5.759 |
| **4+** × 15 | **+4.872** | −3.411 | −3.506 | +0.573 |
| **4+** × 16 | **+6.688** | −3.567 | −1.145 | +2.006 |
| **4+** × 17 | **+8.111** | −3.716 | +1.093 | +3.328 |

Bucket sizes run 2,086–18,234 boards; every figure's 95% CI is ±0.08…±0.35.

Three readings, and they are the whole design.

1. **The `X` gate is length, and nothing else.** `2Mx` wins *every*
   four-plus-trump bucket at *both* vulnerabilities, on a minimum as well as a
   maximum, with or without a stopper — and its perfect-defense column stays
   flat (−1.2…+0.3), which is the signature of a real penalty double that plain
   DD sees and PD is structurally blind to (measurement.md's domain addendum).
   On a **doubleton** it loses at every strength (−0.7…−4.5). On **three** it is
   negative at 15, marginal at 16, and positive only at 17 — and there `3NT`
   matches or beats it whenever opener has a stopper (+3.363 against +2.462
   white, +3.334 against +3.327 red). So `len(major, 4..)`, no HCP floor and no
   stopper test, and the K–K reference's "three plus good defense" is
   **rejected**.

   One cell pays for that simplicity: three trumps, 17 HCP, **no** stopper —
   `2Mx` +1.959 (n=1,409) white and +2.942 (n=1,049) red, against a rung set
   that passes. That is ~4.5% of the `X`'s total value, and buying it costs
   more than it is worth: `comp:landy-penalty` publishes *four-plus* of that
   major, so a three-card double under the same alert would make the alert
   false and the reading wrong. It would need its own slug, its own reading and
   its own arm. **Residue, recorded, not built.**
2. **Declaring notrump is a non-vulnerable idea, except 16–17 with a stopper.**
   Red, the whole declaring family collapses (over the direct leg: `3NT` +0.008,
   `2NT` −0.583) — but the 16–17-with-a-stopper cells hold up at both colours
   (`3NT` +1.10…+3.48 red). The 15-with-a-stopper cells invert with colour:
   `2NT` is +2.130 / +1.371 white and +0.441 / −0.550 red, so the anchor's old
   "the 15s prefer passing" read is right vulnerable and wrong non-vulnerable.
3. **Opener declares.** `@op` beats `@dbl` in every one of the 36 buckets at
   both vulnerabilities — free right-siding evidence, and unsurprising: opener
   holds the 15–17 and the stoppers, and their major sits under it.

#### Why §N1k lost, in this data

§N1k's `3NT` was gated `hcp(16..) & has_stopper` with nothing above it, and
`has_stopper` is **length-blind**. 17.4% of that gate's traffic is the
four-plus-trump slice, where the oracle prices `3NT` at −1.1…+1.9 against the
double's +5.3…+8.1: the rung was not merely mediocre there, it **forwent
+7.0…+7.8 IMPs/board** by shadowing the floor's delayed penalty double — which
is exactly what §N1k's forensic saw, the OFF arm's floor finding
`X (2♥) - - X` on 3 of the 5 worst plain boards per cell. The oracle explains
the refutation rather than contradicting it, which is the cross-check this
probe was run to pass.

The repair is ordering, not a better constraint: **`X`@150 above the notrump
rungs supplies the ≤3-trump cap `has_stopper` cannot express**, for free.

#### The arms as built

| arm | knob | table |
| --- | --- | --- |
| `px` | `competition.landy_opener_px` | `X`@150 `len(major, 4..)` (`comp:landy-penalty`, `.penalty()`) · `Pass`@0, plus the doubler's sit at `{path} X -` |
| `rungs` | `competition.landy_opener_rungs` (needs `px`) | plus `3NT`@135 `hcp(16..) & stopper_in` · `2NT`@120 `hcp(15..) & stopper_in & !vulnerable()`, each a sign-off the doubler passes |

**Three rungs the plan sketched are absent, not deferred.** A natural `3m` is
dominated by notrump on its *own* boards at both colours (+1.86 against `2NT`'s
+1.99 and `3NT`'s +2.19 white; +0.29 against +1.13 red). `3OM` in the major
they did not name is the **worst of all seven candidates** on its own 2.7%
surface (−0.78 plain, −4.5 PD) — they hold four-plus of it. And the relay leg
(`X (2♦) - (2♥) - -`, verified seat-math; 5.1% / 3.3% of the seat) is a
*balancing* seat where the live method already defends their `2♥` and every
candidate prices negative red, so it is not authored either.

**Their runout over our double stays the floor's** (flagged item 4, decided by
smallest diff). The alert publishes opener's four-plus length, so the floor
decides on true information rather than a phantom, and the §N1l twin one call
later takes the same shape. The alert slug is shared with the doubler's seat
(flagged item 3, default taken): one claim, two seats, and who is still to
speak is a matter for the continuation tables rather than for disclosure.

**Falsifiers** (`scripts/ab-landy-opener.sh` states them in full): (1) the
oracle assumes they sit for the double — if `px` reads flat, split the `X` rows
by their next call before anything else; (2) the *instinct* floor already
doubles here, so anchor intuitions do not transfer (the net floor the A/B
measures passes 98.5%); (3) `rungs` is only meaningful under `px`, and if it
loses, the ≤3-trump cap was the whole story; (4) `px` is a pure doubling knob —
plain DD arbitrates.

#### A measurement caveat this probe turned up

`--filter-landy`'s `is_1nt_opener` gate is **strictly balanced** (no
singleton/void, at most one doubleton). Our 1NT opening is `NotrumpShape::Wide6322`,
which also admits 5m(422) and 6m(322) — both of which have two doubletons. So
**no wide-shape 1NT opener ever enters a `--filter-landy` pool**: the probe
found 15,861 boards with a five-card club suit and 14,900 with five diamonds
(all 5(332)) and **zero** with a six-card minor in 103,653 seat boards. This
does not break any A/B — both arms share the filter and the headline is
IMPs per *accepted* board — but every §N1 verdict measured under it is blind to
that slice, and the "5+ vs 6+" question the `3m` rung was supposed to answer is
**unanswerable in this pool**. Flagged below.

### N1p — an **unlimited** values double (`landy_notrump_no_major` **loss, stays off**; `landy_major_jam` **shipped default-on 2026-08-30**)

Responder's direct seat, `1NT (2♣) ?`. The `X`@145 is constrained `hcp(8..)` —
**unlimited on top in its own constraint** — but the table's ungated `3NT`@168
on bare `points(10..)` outranks it, so every ten-plus-point hand declares and
the double never sees a game hand. Verified, not assumed: `probe-call-reading
--their-2c-landy` reads partner back as `points 8..9`.

This is [flagged item 2](#flagged-not-fixed-n1--reversible-defaults-proposed)
taken up. The flag proposed leaving it because *re-gating* `3NT` "moves every
shape in the lane"; the arm below moves only the shape that matters.

#### Verdict — `nt` loses at both colours; the `jam` rung is a stranded win

Runner `scripts/ab-landy-notrump-shape.sh`, `SEED_BASE=1788005427`,
4,608,000 boards/arm/vul, isolation gate **0 foreign** on all four pairs.

| pair | fired | DD plain | DD-PD | sd-plain | **SD-PD** |
| --- | ---: | ---: | ---: | ---: | ---: |
| `nt vs base` none | 54,987 (1.19%) | **−0.0124** ±.0007 | +0.0181 ±.0008 | **−0.0266** ±.0007 | **−0.0012** ±.0007 |
| `nt vs base` both | 37,778 (0.82%) | **−0.0076** ±.0007 | +0.0195 ±.0008 | — | — |
| `jam vs base` none | 55,093 | −0.0109 | +0.0186 | −0.0247 | −0.0002 |
| `jam vs nt` none | 1,567 (0.03%) | **+0.0015** ±.0001 | **+0.0006** | **+0.0020** ±.0001 | **+0.0013** ±.0001 |

`nt` loses plain DD at **both** vulnerabilities — no colour flip, unlike
§N1l's rungs — and loses the SD-PD arbiter white. Only the DD-PD column is
positive, and falsifier 1 is aimed the wrong way to rescue it: perfect defense
already doubles every failing contract, so **deleting our own penalty double
costs nothing in PD while the extra competitive room still scores**. The
+2.383 IMPs/fired PD row is that artifact, not a result. Both knobs stay off.

The `jam vs nt` pair wins **all four scorers** (+5.541 IMPs/fired sd-plain) on
its 1,567 boards, and the sit node never needed relaxing. **It cannot ship on
that number**, and not merely because it rode a losing arm — it measured the
wrong substitution:

| | what `4M` replaced | measured |
| --- | --- | --- |
| `jam vs nt` | the **`X`**@145 — `nt` gates `3NT`@168 to deny 4+ majors, and six of one *is* four-plus | +5.541/fired sd-plain |
| jam standalone | the **`3NT`**@168 — ungated on `main`, and `4♠`@172 / `4♥`@171 outrank it | **unrun** |

Same hands, different comparison, and the §N1p loss was overwhelmingly *"we
stopped reaching game"* — a cost the standalone jam does not pay, because `4M`
**is** a game. So the +5.541 transfers to nothing and the real question is open.

#### The standalone arm (`landy_major_jam` decoupled, 2026-08-30)

The `deny_major &&` conjunct was dropped from both the rungs and their two sit
nodes. The generalisation is **behaviour-preserving where the two overlap** —
with both knobs on the table is exactly what §N1p measured — so the verdict
above stands unchanged.

It also corrects a doc/code discrepancy: `landy_major_jam`'s knob doc claimed
the rung "never fires" without `landy_notrump_no_major` because the ungated
`3NT`@168 swallows the hands. **False** — 172 and 171 both outrank 168. It was
the conjunct that suppressed the rung, not the weight ladder, which is why
decoupling costs nothing.

**The standalone arm has zero reading drift**, verified not assumed:
`probe-call-reading --their-2c-landy "1N (2C) X -"` returns `points 8..9` with
the same suit ranges on `main` and with `--ns-landy-major-jam`. `3NT`@168 stays
ungated, so the exclusion that caps the double is untouched, and the `4M`
denials are already implied by that cap. This is exactly the mechanism §N1p
tripped over — its 16.0% / 13.4% "bid where the baseline passed" bucket — and
the jam does not touch it.

The bridge case is that their `2♣` shows **both majors**, so our own six-card
major sits opposite known length: the suit breaks badly, trump control beats
the ninth trick, and `4M` takes the four-level away from a pair that has
advertised a fit. §N1p measured the candidate handing the opponents more room
on 72–75% of divergent boards by doubling instead of declaring; the jam does
the opposite.

Runner `scripts/ab-landy-major-jam.sh` (arms `base | jam`, both vulnerabilities,
fresh `SEED_BASE`), render `render-book --their-2c-landy --ns-landy-responder
jam-only --prefix "1NT 2♣"`. Its named risks, in the header: obstruction is
invisible to DD (read the sd pair first); `4M` may simply be an overbid, since
the rung has **no quality gate** and a ratty six-bagger with soft side values is
the hand `3NT` was right on (read the made/down split before the IMP mean); the
sit still forgoes slam on the fifteen-plus slice; the slice is thin (~0.03%).

##### Verdict — an eight-of-eight sweep; **shipped default-on**

`SEED_BASE=1788033942`, sha `52fbc7c1`, 4,608,000 boards/arm/vul, 24 shards.

| scorer | none (1,381 fired, 0.03%) | both (1,013 fired, 0.02%) |
| --- | ---: | ---: |
| DD plain | **+1.443**/fired (+0.0004 ±.0001) | **+1.611**/fired (+0.0004 ±.0001) |
| DD perfect-defense | **+1.635**/fired (+0.0005 ±.0001) | **+1.957**/fired (+0.0004 ±.0001) |
| sd-plain (16 worlds) | **+1.435**/fired (+0.0004 ±.0001) | **+1.600**/fired (+0.0004 ±.0001) |
| **SD-PD** (arbiter) | **+1.558**/fired (+0.0005 ±.0001) | **+1.866**/fired (+0.0004 ±.0001) |

Every cell positive, every CI excluding 0, isolation gate **0 foreign** at both
colours. This is not a doubling artifact — plain DD wins on its own and the PD
column only widens the margin, which is the signature of a *contract* gain, not
of auto-doubles.

The divergence census says the rung is doing exactly one thing, and nothing
else: **100.0%** "a different bid" at both colours — zero boards where an arm
bid and the other passed, zero pass-outs, and game reached in **both** arms on
100.0% of divergent boards. §N1p's fatal buckets are all empty here. Compare:

| bucket | §N1p (`nt`) | the standalone jam |
| --- | ---: | ---: |
| bid where the baseline passed | 16.0% / 13.4% | **0.0% / 0.0%** |
| game reached, baseline only | 81.5% / 84.8% | **0.0% / 0.0%** |
| more room handed to the opponents | 72.3% / 75.0% | **0.0% / 0.0%** (6.2% / 0.2% *less*) |
| declarer changed sides | 91.1% / 94.9% | 4.9% / 0.2% |

So the two arms bid the same auction up to the rung and reach game either way;
the only question priced is `4M` versus `3NT` on a strong six-card major
opposite a pair that has advertised both majors. `4M` wins it by 3:1 in IMPs.

The named risks resolve as follows. **Obstruction**: not needed — the win is
already there on plain DD, and the tiny room asymmetry runs *our* way. **The
overbid**: real but priced. Every one of the five worst boards at every scorer
is the same shape, a making `3NT` traded for a failing `4M` (the 6––4 flat
holdings such as `85.AT9763.6.KQ73` opposite `KQJ.54.KQJ94.AJ4`, where the
notrump has nine top tricks and the heart game has a trump loser plus two).
They cost −11…−16 IMPs each and are outweighed threefold, so a quality gate is
a *tuning* follow-up, not a ship blocker. **The sit's forgone slam**: invisible
at this fire rate; unchanged and still the first thing to relax.

Follow-ups, both optional and both unstarted: a quality gate on the six-card
suit (the losing boards are all texture-poor), and relaxing `multi_signoff_pass`
on the fifteen-plus slice now that the rung ships.

##### Falsifier 2 resolves against the idea, not against a continuation

The divergence split (`probe-divergence --gate-opener ours`) makes the
`3NT`→`X` substitution itself the dominant bucket, not the reading drift at
opener's seat:

| bucket, *our first differing call* | none | both |
| --- | ---: | ---: |
| a different bid (the `3NT`→`X` swap) | 83.6% | 86.4% |
| bid where the baseline passed (reading drift above the `X`) | 16.0% | 13.4% |
| game reached, **baseline only** | 81.5% | 84.8% |
| more room handed to the opponents in the candidate | 72.3% | 75.0% |
| declarer changed sides | 91.1% | 94.9% |

Four of the five worst white boards are a quiet made `3NT` traded for a
competitive train wreck — `off: 1NT 2♣ 3NT - - -` against
`on: 1NT 2♣ X - - 2♠ 4♥ - 4♠ X - - XX - - -`. That is falsifier 2's *"we
defended a making game"* branch, which the runner header marked **idea dead**
rather than repairable.

##### Why: the double is outside the floor's teacher's vocabulary

Flagged item 1 already recorded that BBA labels our `X` over Landy **"bidable
suit"** (`12-17, 5+♣`) and never doubles at opener's seat. A further probe
(jdh8, 2026-08-30) adds that BBA's 1NT opener **does not double and reads the
double as takeout**, contrary to expert practice. So the floor — distilled from
BBA — has no learned concept of a values double in this lane.

§N1l shipped because it authored *both* sides of the call: the doubler's own
rebids are book. §N1p does the opposite — it routes several times more traffic
**into** the double, gives the opponents a free round of bidding they never got
over `3NT`, and leaves both continuations to a floor that misreads the call.
The 72–75% "more room handed to the opponents" is that cost, measured.

**The rule this buys**: in this lane, widening a call's traffic by a *reading*
change is only safe where the floor's teacher shares the concept. Where it does
not, the widening owes authored continuations first. See
[docs/reading-drift-handoff.md](reading-drift-handoff.md) — a reading knob is a
bidding knob under a neural floor.

#### The fix is on the notrump, not on the double

The first sketch promoted the `X` above `3NT`@168. That does buy the same
hands, but "unlimited" then has to mean above the *gated* `3NT`@180 too — and
there is no weight between 180 and the GF both-minors family at 178, so
promoting past the notrump means promoting past the transfers and the splinters
as well. Three separately-defended orderings would move at once: the transfers
outrank the double *by design* ("a six-carder never defends"), and the
two-suited family outranks the transfers so a 6-4 shows the whole picture.

Restricting `3NT` moves exactly the intended traffic and nothing else
(jdh8, 2026-08-29): **`3NT` never gets a four-card major; short stoppers are
welcome; the transfers still outrank the `X`.**

And the gate is not an arbitrary cut chosen to move traffic — it is what `3NT`
*means*. **Bidding `3NT` denies interest in penalising them** (jdh8), and
holding four of a suit they have just shown is exactly interest in penalising
them: their fit in it is at best 4-3, our trumps sit over the overcaller, and
the hand wants to defend. So `len(major, ..=3)` is `3NT`'s own honest
precondition, which is why the reading it publishes (`♥ ≤3 ♠ ≤3`, verified) is
a *narrowing to the truth* rather than a claim the bidder does not honour. The
same statement read the other way is the double's floor: four-plus of their
major is the shape that wants to defend at any strength, which is what
§N1m's oracle then prices at +3.5…+8.1 IMPs/board one seat later.

| | as built |
| --- | --- |
| `nt` | `competition.landy_notrump_no_major` — both `3NT` rungs (@180 and @168) gain `len(♥, ..=3) & len(♠, ..=3)`. Paired `rule` calls, not a conditional constraint: the two constraints are different types (the `landy_doubler_white` idiom) |
| `jam` | `nt` plus `competition.landy_major_jam` — `4♠`@172 / `4♥`@171 on `len(major, 6..) & points(10..)`, above the restricted `3NT`@168 and below the transfers, with `multi_signoff_pass` at `4♠ -` / `4♥ -` |
| `jam-only` | `competition.landy_major_jam` alone — the same two rungs and sits over an **ungated** `3NT`@168, so `4M` substitutes for the game. **This is what ships**, and `nt` stays off |

Where the displaced hands land, in order: a 6+ minor still transfers; the GF
both-minors shapes still fire (4+♦ *and* 4+♣ leaves at most three in each major
anyway, so the only overlap is the 4=1=4=4 splinter); everything else reaches
the `X`@145. `4♠` outranks `4♥` because with 6-6 the better game is `4♠`, and
nothing else satisfies both. The jam rung is natural, so it carries no alert and
`alert-sites.txt` is unchanged.

#### The reading comes for free

`reading.bid_exclusion` intersects each rule with what its strictly heavier
siblings deny, which is the whole mechanism that caps the double today. Narrow
`3NT` and the intersection loosens: the double's published reading widens from
`points 8..9` to *8+ hcp, and with game values four-plus of a major*, and `3NT`
gains an honest denial of major length. **No new slug, no alert change, no
`.bbsa` row, no `comp:landy-penalty`-style disclosure decision** — which is what
separates this from the no-catch-all arm §N1l-flip still owes.

**Free of disclosure cost, not of behavioural cost** — established by the A/B,
recorded here against the claim above. The reading attaches to the *call*, so
the widening also republishes every pre-existing eight-to-nine point values
double as `points 8..37`, and the floor one seat up acts on it. That is the
16.0% / 13.4% "bid where the baseline passed" bucket in the verdict: a real
cost, secondary to the substitution but not zero.

#### Priors

- **N1d** priced taking eight-to-nine point hands *off* this double at
  −0.92/−2.53 PD per fired, and flipping them back at +2.0…+5.1. The double has
  been measured under-fed before.
- **§N1m's oracle** prices defending their major **doubled** at +3.5…+8.1
  IMPs/board in every four-plus-trump bucket at both vulnerabilities, on a
  minimum as well as a maximum, with a stopper or without, PD column flat. That
  is opener's seat, not responder's — it is a prior about *length*, not a
  measurement of this arm.
- The 2026-08-27 census makes `2♣` Landy the lane's **top cost by total**,
  −275 IMPs on 551 boards.

#### Falsifiers

1. **PD is structurally blind to `nt`.** Perfect defense already doubles every
   failing contract, so it keeps the whole cost of a real penalty double and
   none of the benefit. Read `nt` on plain DD with SD-PD as the tie-break.
2. **The displaced `3NT`s were making.** If `nt` loses plain, split the
   divergence by `call_off == "3NT"` and read what replaced it: "we defended a
   making game" kills the idea, "opener pulled the double badly" is a
   continuation defect. The seat above the double is the **floor's** on `main`
   (§N1m is off), and it pulls a values double to `3NT` 49.5% of the time on
   two trumps — so this arm partly measures that floor.
3. **The jam is obstruction, which DD cannot see.** A negative `jam vs nt` on
   plain DD is partly the harness. Read the made/down split of the `4M`
   contracts before the IMP mean.
4. **`--filter-landy` admits only strictly balanced 1NT openers** (flagged item
   5), so the wide-shape slice is invisible to all three arms.

#### The two scales do not leave a hole — checked, not assumed

`3NT`@168 floors on **`points`** and the `X`@145 on **`hcp`** (deliberately —
"defending does not care about distribution"), which looks like it could strand
a shapely hand between them. It cannot. The shipped `ReadingProfile` uses
`PointScale::PointCount`, i.e. `raw_hcp + upgrade`, and `upgrade` is
`unbalanced + (two longest ≥ 10) − wasted`, so it is **capped at 2**. Therefore
`points ≥ 10 ⟹ hcp ≥ 8`, and every hand the gate displaces from `3NT`@168
clears the double's `hcp(8..)`.

The rungs in between are exhaustive too: a displaced hand with a 6+ minor
transfers, a 4=1=4=4 lands on the splinter@176, and everything else reaches the
`X`. **No hand falls to the `Pass`@0 because of this gate** — which is what
makes `nt` a clean substitution of one call for another rather than a mixture
of a substitution and a suppression.

#### Recorded, reversible

The `4♠ -` / `4♥ -` sit is `multi_signoff_pass` — opener passes unconditionally,
so the jam arm **forgoes slam** on the fifteen-plus / six-card-major slice. It
is there because §N1o's forensic caught the floor cue-bidding this lane's
four-level to `6♥` doubled. Proposed reversible default: **keep the sit**, and
delete it first if the jam arm reads mixed. **Settled 2026-08-30**: the jam arm
won all four scorers with the sit in place, so it was never relaxed — and it
carries forward unchanged into the decoupled-`4M` follow-up.

Runner `scripts/ab-landy-notrump-shape.sh`, arms `base | nt | jam` at both
vulnerabilities, `SEED_BASE=1788005427`, `jam vs nt` paired on the same boards
to price the jam rung alone. Renders: `render-book --their-2c-landy
--ns-landy-responder <off|nt|jam> --prefix "1NT 2♣"`. The run was stopped after
`jam-both` began (0 shards written, nothing lost); `jam vs base` and
`jam vs nt` are therefore white-only, and the both-vul `nt vs base` DD cells
were scored off the two completed arms. The sit node was never relaxed — the
jam arm read a win, not mixed.

### N1-lia — Lia's counter-defense: the minor ladder a level down, the doubler unshadowed, Texas at the four level (**packages A and C shipped default-on; B and D measured non-wins; B's refinement measured a loss 2026-09-02 — lane parked behind the floor rail**)

`1NT (2♣)` is the lane's top cost bucket by total (−275 IMPs plain on 551
boards, the census above) and has had no open package at census level since
§N1p closed. **Lia is IntoBridge's AI** — an online service, no code access,
so everything here about her system comes from probing her by hand on
cuebids.com. She plays a counter that is ~80% our shipped §N1j table.

**The original probe of her responder table was wrong, and the "four deltas"
characterisation below is void** (2026-09-01). It read her ladder inverted —
weak transfers at the two level, natural invitations at the three — when she
plays the opposite; the delta called *worse* ("her 5+5+ takeout leaves the 4-4
hand with a singleton major homeless") was a misreading of an **UNBAL 4+♦ 4+♣**
takeout, and the `3NT` = 2-3 majors attribution is now **unconfirmed** rather
than asserted. What survives: the `3NT` delta, whichever system it belongs to,
is `landy_notrump_no_major`, measured a plain-DD loss at both colours — first
with the doubler's known-broken `Pass`@0 catch-all in place, then again on the
repaired seat as package D, where it stayed a non-win. Package B was rebuilt
on the corrected probe and is the only package the correction touches: A, C
and D neither read `defense_2c_landy_lia` nor are gated on it.

Everything ships only on its own A/B: **A** the doubler seat (first — D is
blocked on it, and the −14,171 IMPs live here), **B** the ladder permutation,
**C** the four-level, **D** the `landy_notrump_no_major` re-measure on A's
winner. Six knobs, all default states byte-identical (`smoke-default --count
20000 --seed 1` byte-identical before/after the build, verified twice — once
more after the package-A registration change below, and again after B's
2026-09-01 rebuild).

#### Package A — the full ladder **shipped default-on 2026-08-30**: `landy_doubler_catchall` now **false**, `landy_doubler_three_honors` and `landy_doubler_three_small` both **true**

**Verdict** (`ab-landy-lia-doubler.sh`, SEED_BASE=1788088630, 4.6M
boards/arm/vul, every isolation gate 0-foreign): every adjacent pair on the
cumulative ladder base → nocatch → hon → cells is a **plain-DD win at both
vulnerabilities**, and every sd-lead (16-world) tie-break stays positive —
the whole ladder ships.

| adjacent pair | vul | plain | PD | sd-plain | fired |
| --- | --- | --- | --- | --- | --- |
| nocatch vs base | NV | **+0.0036** ±0.0002 | −0.0027 | +0.0032 | 0.21% |
| nocatch vs base | BV | **+0.0014** ±0.0003 | −0.0044 | +0.0009 ±0.0003 | 0.14% |
| hon vs nocatch | NV | **+0.0008** ±0.0001 | −0.0003 | +0.0006 (+2.03/fired) | 0.03% |
| hon vs nocatch | BV | **+0.0008** ±0.0001 | −0.0004 | +0.0005 (+2.29/fired) | 0.02% |
| cells vs hon | NV | **+0.0104** ±0.0005 | −0.0138 | +0.0007 ±0.0005 | 0.68% |
| cells vs hon | BV | **+0.0118** ±0.0006 | −0.0142 | +0.0020 ±0.0006 | 0.54% |

Falsifier 1 refuted: the −14,171-IMP catch-all cost was no same-seed
artifact — the deletion's sign reproduces on a fresh stream at both colours.
Falsifier 2's loss tail is real (the floor's undisclosed short double does
get sat for the worst boards, −11/−12 IMPs each) but the deletion is a net
win anyway, and the cells shrink the floor's share exactly as the falsifier
asked. Falsifier 3 refuted: the honors cell is positive on its own, and at
+2.0–2.3 sd IMPs/fired it is the ladder's cleanest rung — the sibling lane's
lone-honor caveat did **not** carry to this seat. The small cell's headline
plain margin (+2.0/fired) is mostly the DD-lead artifact — sd-lead shrinks
it to +0.10/+0.37 per fired — but the tie-break holds above zero at both
vulnerabilities, matching the `len3 hon0` sibling prior. PD is negative
throughout: the pre-registered doubling artifact, reported double-blind and
excluded from arbitration per the script header. Package D
(`landy_notrump_no_major`) is now unblocked on the repaired seat — and, run
there, [measured a non-win](#verdict--measured-non-win-2026-09-01-landy_notrump_no_major-stays-default-off):
the repair reached, but it moved the undoubled scorers only.

##### As designed (build record)

The §N1l-flip shipped two caveats; this package takes both up. Deleting the
catch-all (`landy_doubler_catchall=false`) un-shadows the floor's
takeout-shaped values double below the rungs — worth ≈ **+14,171 IMPs plain
non-vulnerable** on the flip stream — and was blocked on disclosure:
`comp:landy-penalty` published *four-plus* while the un-shadowed floor call is
short. **The tag is re-worded** (`competition.rs`, `card.rs`) to **"length or
honour strength in their major"** — one claim across every cell that can fire
under it, so the arms differ only in the rule, never in disclosure. The two
three-card cells then buy exactly-three trumps back by top-honor class
(`X`@154 on `top_honors(2..)`, `X`@153 on `top_honors(..=1)`, both
`.penalty()` under the same tag), priors from the sibling
`nt_high_overcall_x_leave_in` re-slice (`len3 hon0` **+0.62/+1.85** plain per
fired, `len3 hon1` **−0.75/+0.37**; `hon2+` unmeasured).

**As built — the deletion was a silent no-op until the registration moved.**
`Trie::resolve_floored` is a deliberate single fall-through: an **exact
node**'s rejection falls to the floor, but a **guarded fallback**'s rejection
returns its all-−∞ logits unchecked and the driver passes — behaviourally the
same Pass the arm was meant to delete. The doubler tables were `Pattern::after`
guards; they are now `Pattern::node` at the four explicit paths (byte-identical
at defaults, re-verified), and `landy_doubler_cells_split_three_trumps` pins
the floored/not-floored split per arm. Any future "let the floor own it by
deleting the catch-all" change in a **row package** has to cross the same
seam — the trie comment (`trie.rs`, "single fall-through") is the marker.

#### Package B — `defense_2c_landy_lia`: **misprobed, redefined in place, measured a loss, then refined on its own forensic 2026-09-01 — A/B owed**

> **Misprobe annotation (2026-09-01).** Everything in this section down to
> "The repair" is a true record of a **built ladder that no one plays**. The
> 2026-08-31 measured loss and the 2026-09-01 repair are facts about that
> build; they are not facts about Lia's counter, because the probe it was
> built from had her ladder inverted. What she actually plays:
>
> | Call | True Lia | As built (misprobed) |
> | --- | --- | --- |
> | `2♥` | **UNBAL** takeout, 4+♦ 4+♣ | GF takeout 4+♦4+♣ (repair: exact heart doubleton) |
> | `2♠` | **INV+**, 6+♣ (rarely 5) | weak(≤7)/GF two-way, 5+♣ |
> | `2NT` | **INV+**, 6+♦ (rarely 5) | weak transfer, `len(♦,6..) & points(2..)` |
> | `3♣`/`3♦` | **S/O**, 6+ cards | natural 5+ invitations (8–9) |
>
> The unlisted calls (`X`@145, `2♦`@140, `3M`, `3NT`, `Pass`) are confirmed
> unchanged, so the correction is exactly these four rungs. `defense_2c_landy_lia`
> was **redefined in place** — it never shipped, its off state is
> byte-identical, and the old semantics stay pinned by sha (`8a778178`; the
> measured-loss build is `59cd46ee`-control). The rebuild is
> ["Rebuilt as true Lia"](#rebuilt-as-true-lia-2026-09-01) below.
>
> The forensic **transfers**, and it partly predicts well for the corrected
> ladder: her sign-offs demand six cards (defect 2 — 6+ won at both colours,
> exactly-5 flipped on vulnerability), and her INV+ rungs restore the forcing
> channel whose absence was defect 1, the only defect negative at both
> colours. What it predicts badly is the one cell true Lia has no home for:
> the exactly-five 8-9 invitation, the forensic's single biggest win (`3♣`
> +85,613 NV) and its best contested rung (`3♦` +1.576/+1.931).

**Verdict of the built ladder** (SEED_BASE=1788122360, control `59cd46ee` =
package A's ship, 4.6M boards/arm/vul, both isolation gates 0-foreign):

| vul | plain | PD | sd-plain | sd-PD | fired |
| --- | --- | --- | --- | --- | --- |
| NV | **+0.0050** ±0.0012 | −0.0756 ±0.0016 | +0.0374 ±0.0013 | −0.0289 ±0.0016 | 5.90% |
| BV | **−0.0384** ±0.0014 | −0.1210 ±0.0018 | +0.0016 ±0.0014 | −0.0691 ±0.0017 | 4.92% |

Plain DD splits by colour and the both-vul loss is 27σ, so there is no plain
win to ship on; the PD deficit is an order of magnitude past package A's
doubling artifact and cannot be waved through as one. The mechanism runs the
**opposite** way to A's: lia *removes* our penalty doubles (5.51% of divergent
boards against base's 7.79%), and perfect defense is exactly the scorer that
pays for doubles we no longer make. The knob stays default-off, off-state
byte-identical, on the `multi_px_split` precedent.

**The loss decomposes into four named defects, none of them the ladder's
concept.** `probe-divergence --imps` over both divergence sets, bucketed by our
first differing call and by responder's own hand (seat resolved from dealer +
auction; self-validating — all 101,767 boards in the `2♠` bucket come back
holding 5+ clubs, exactly what the rung constrains). Plain IMP totals, NV / BV:

| bucket | plain NV | plain BV | per fired NV/BV |
| --- | --- | --- | --- |
| `3♣` natural club invitation | **+85,613** | **+35,920** | +1.31 / +0.68 |
| `3♦` natural diamond invitation | +14,381 | −14,911 | +0.31 / −0.40 |
| `2♠` club rung | +15,581 | −115,870 | +0.13 / −1.14 |
| `2NT` diamond rung | −23,533 | −23,095 | −1.92 / −2.42 |
| `Pass` (lia passes where base bid) | −35,827 | −32,432 | −2.26 / −2.24 |
| `2♦` | −18,178 | −14,121 | −2.02 / −1.79 |
| `2♥` sole takeout | −12,525 | −10,408 | −4.07 / −5.00 |
| **total** | **+22,996** | **−176,837** | |

**Defect 1 — the contested tails are unauthored, and this is the only defect
negative at both colours.** Every authored lia node requires the opponents to
have passed (`{rung} -`, `{rung} (X)`, `{completed} …`), so an opponent bid
*anywhere* drops the remainder to a floor with no forcing channel — the
`4♣-4♥-5♣-5♦-6♣-6♥` runaways in the worst boards, one landing in `6♥` on a
void.

| rung | quiet NV/BV | contested NV/BV |
| --- | --- | --- |
| `2♠` | +79,531 (+1.29) / −35,081 (−0.57) | −63,950 (−1.13) / −80,789 (−2.00) |
| `2NT` | −1,125 (−0.28) / −1,427 (−0.44) | −22,408 (−2.72) / −21,668 (−3.42) |

The diamond rung is the clean demonstration: uncontested it is nearly free, and
**95%/94% of its entire loss is the contested tail**. Note this is *not* only
the advancer's seat — they pass over `2♠` on 85% of boards, and their immediate
advance is only −26,462 of BV's −115,870; the rest is opponents entering later,
after opener's length answer. The whole contested surface is owed, not one node.

**Defect 2 — the weak five-card sign-off, and it is vulnerability-dependent.**
97% of the club rung's BV loss sits in 0–7 HCP. Within that band, uncontested,
plain per fired (NV / BV):

| weak, quiet | n (BV) | plain/fired NV | plain/fired BV |
| --- | --- | --- | --- |
| exactly 5 clubs | 51,521 | **+1.405** | **−0.803** |
| 6 clubs | 6,187 | +0.993 | +0.668 |
| 7+ clubs | 1,137 | +1.849 | +1.683 |

Six-plus wins at **both** colours; exactly five flips sign with vulnerability —
light five-card sign-offs are profitable white and ruinous red, which is
falsifier 2 confirmed in a sharper form than it was posed (the old N1j transfer
demanded six). That one cell is also the biggest PD cell in the arm:
**−106,027 NV / −249,689 BV**, the latter 45% of the whole arm's PD deficit.

**Defect 3 — the `2NT` cap starves diamonds** (falsifier 4 confirmed): the weak
six-carder with 7+ HCP now passes, worth −35,827 NV / −32,432 BV.
**Defect 4 — the sole `2♥` takeout** is the worst per-fired rung in the ladder
(−4.07/−5.00), on small volume; the 2=3=4=4 merge is the suspect.

At both-vul the first three sum to −176,261 against a −176,837 total, so they
account for essentially the whole deficit — which means the rest of the ladder
is roughly break-even and the restored invitations are a real win.
**Falsifier 1 is not merely refuted but reversed**: the N1c right-siding trade
was *wrong* on plain DD, and unwinding it is the single biggest positive here.

**Repair queue** before any re-measure (fresh seed, control = then-current
`main`), in size order: (1) author the contested tails across both rungs at
every level; (2) gate the weak `2♠` leg to 6+ clubs vulnerable, 5+ white;
(3) restore a rung for the starved weak six-card diamond hands; (4) revisit the
2=3=4=4 merge into `2♥`. Packages C and D are unaffected — neither knob is
gated on `landy_lia` and neither runner passes it, so their control is
unchanged by this loss.

##### As designed (build record) — **superseded, kept as the record of what was measured**

Every rung below is the **misprobed** ladder. It is kept because the loss and
the repair were measured against it, and a verdict is only readable against
the build it scored. The live design is
["Rebuilt as true Lia"](#rebuilt-as-true-lia-2026-09-01).

One arm; a permutation of the same rungs, so it cannot be decomposed. As
rendered (`render-book --their-2c-landy --ns-landy-responder lia --prefix
"1NT 2♣"`):

As first built (2026-08-31, the arm that measured the loss); the three rungs the
2026-09-01 repair moved are marked, with the repaired form beside them:

| Call | As measured | After the repair | vs §N1j |
| --- | --- | --- | --- |
| `3NT`@180/168, `X`@145, `2♦`@140, `3♥`/`3♠`@176/175, `Pass` | unchanged | unchanged | — |
| `2♥`@178 | GF takeout, 4+♦ 4+♣, 2+ in **both** majors | **`len(♥, 2..=2)`** — N1j's exact heart doubleton | the only takeout; the merge is reverted, 2=3=4=4 re-routes to `3NT` |
| `2♠`@174 | 5+♣, weak (≤7) **or** GF (10+) | **`points(..=7) & (len(♣, 6..) \| !vulnerable())`** on the weak leg | was `2NT`→♣ 6+ wide |
| `2NT`@173 | 6+♦ and (7+♦ \| `top_honors(2..)` \| GF) | **`len(♦, 6..) & points(2..)`** — the N1j transfer's own shape gate | was `3♣`→♦ 6+ wide |
| `3♣`@167 / `3♦`@166 | natural 5+ invitations (8-9) | unchanged in the rule; `3♦` now sees only five-card hands, by weight | restored — the N1c right-siding trade unwound **per length** |

The level-down ladder matches BBA's own coherent self-play tree
(`docs/ai-bidder/bba-1nt-landy-tree.md`: `2♠`→♣ 5.9%, `3♣`→♦ 6.8%, direct `X`
0/4,074); §N1j's `2NT`→♣ was aligned to the older actor-only reading that
corpus overturned. Clubs and diamonds are deliberately asymmetric: `2♦` gives
diamonds a cheap natural outlet, clubs have none below 2NT, so `2♠` stays
genuinely two-way.

**No forced completion — opener answers the minor rungs by length**
(`comp:landy-length`, a new always-on alert so the reader decodes the exact
bands instead of the walk's four-card raise floor): the cheap raise = 3+
(`3♣` over `2♠`, `3♦` over `2NT`), the step below it a doubleton (`2NT` over
`2♠`, a contract; `3♣` over `2NT` — balanced with two diamonds implies 3+
clubs). Responder rebids off `landy_bba_transfer_rebid` **verbatim** on the
fit legs; the doubleton legs add one rung — a sign-off in the minor
(`3♣`/`3♦`@60 on `points(..=9)`), and opener sits (`multi_signoff_pass`).
The restored `3♣`/`3♦` invitations get the stack lane's acceptance table
(`landy_minor_invite_answer`: `3NT` from the top with both majors stopped,
else sit) — caught by the build review, not the design: floor-owned, the
seat answered the N1j gadget the natural `3m` replaced with a **phantom
`3♦` transfer completion** on every probed hand, which would have
confounded the arm's own falsifier 1.
The N4-KK `4m` slam try and `landy_slam_answer` (+ RKCB, `-` and `(X)` tails)
re-hang **byte-identical on all four legs**; the P6 residue (accept gate
counts HCP, not controls) is left alone so the ladder arm stays attributable.
Probed readings: `2♠` → partner ♣ 5-13, no spade claim; raise → ♣ 3-6;
doubleton → ♣ exactly 2 / ♦ exactly 2. Two design-sketch ambiguities resolved,
both reversible: *"INV+ may pass"* opener's 2NT is resolved to
**sign-off-always** (the two-way rungs carry no 8-9 band, so the pass had no
traffic — a weight change re-opens it), and the `2NT` GF leg requires **6+♦**
(a GF 5♦4♣ hand rides the takeout, a GF 5♦ balanced hand bids `3NT`).

**After `2♥` the answer priority reverses** (`landy_lia_takeout_answer`): a
four-card minor first (cheapest with both — the guaranteed 4-4 fit is the
takeout's point), `2NT` = spade stopper *specifically*, `2♠` = neither
(asks; `comp:landy-ask`, alerted by hand — vacuous constraint). Lia's takeout
names no short major, so `2NT` answers the one question nothing else in the
structure ever answers — nothing promises hearts, and LHO leads their longer
major; responder resolves hearts with the `3♥` cue
(`landy_bba_takeout_rebid`/`landy_bba_ask_answer` reused verbatim, the asked
major flipped to hearts). Over the `2♠` ask responder needs its own spade
stopper for notrump: `3NT` with both, `3♥` cue with spades only, else `4♣`@20
(the ask-answer's own stopper-dead catch-all one seat over). Splinters and
their raise/`(X)` tails keep the shipped tables; lia's raise tails key the
stopper on the suit **they raised** instead of a short major the takeout no
longer names.

##### The repair (2026-09-01) — **superseded by the probe correction**; its A/B was stopped mid-flight

**Verdict: superseded.** The repair fixed four defects in a ladder nobody
plays, and its A/B (`scripts/ab-landy-lia-repair.sh`, SEED_BASE=1788247951,
control `ce94faeb`) was **stopped** when the probe correction landed. One cell
had completed and is recorded rather than discarded, since it is the only
measurement the repaired build will ever get:

| vul | plain | PD | gates |
| --- | --- | --- | --- |
| NV | **+0.0191** ±0.0011 | **−0.0382** ±0.0014 | 0-foreign |
| BV | *never run* | *never run* | — |

That NV row is the decision table's doubling-artifact shape (plain win, PD
loss) on a mechanism that *bids more* — and the runner's header stated the
arbitration rule for it **wrong**, which is why `ce94faeb` exists. The
arbitration question is now **moot**: there is no both-vul cell, and the
build it scored has been replaced. Nothing is owed on it. The arms stay on
disk under `ab-results/landy-lia-repair/` for forensics.

The four defects it named remain the best evidence available about this lane,
and the rebuild below uses them — see the misprobe annotation for which of
them transfer and which one predicts a loss.


The four defects above are repaired behind the same off-by-default knob (**no
new knobs**; `smoke-default --count 20000 --seed 1` byte-identical against
`main` HEAD, verified twice). Step 0 re-solved the kept divergence sets
(`probe-divergence --imps --jsonl` over `ab-results/landy-lia/`; the August
run's JSONL had been in `/tmp` and was gone) and **two of the plan's three
pre-registered decision rules came back inverted**, so the built repair is not
the one the plan sketched.

**Step 0(a) — the invitations split on length, not on colour.** The rule was
"tighten `3♦` to six-plus iff the exactly-five quiet cell is negative at both
colours". Exactly-five is *positive*, and the six-plus cell is the loser:

| rung × length | quiet NV | quiet BV | contested NV | contested BV |
| --- | --- | --- | --- | --- |
| `3♣` exactly 5 | **+1.796**/fired (+63,697) | +0.756 (+23,210) | +0.818 (+4,243) | −2.721 (−5,521) |
| `3♣` 6+ | +0.768 (+16,365) | +1.035 (+19,876) | +0.369 (+1,308) | −1.394 (−1,645) |
| `3♦` exactly 5 | **+0.920** (+23,105) | −0.459 (−10,032) | +1.576 (+15,504) | +1.931 (+11,701) |
| `3♦` 6+ | **−0.910** (−6,995) | **−0.858** (−5,749) | **−4.668** (−17,233) | **−4.195** (−10,831) |

The plan's rule ("tighten `3♦` to six-plus iff exactly-five quiet is negative at
**both** colours") therefore does **not** fire — exactly-five quiet is +0.920 NV
against −0.459 BV — but the cell it was aimed at is the wrong one. `3♦` **6+**
is negative in all four cells, so the N1c right-siding trade splits on **suit
length**: the natural invitation wins the five-card hand and the transfer's
right-siding wins the six. The six-card hand is given back to the rung above,
and exactly-five stays (its two contested cells, +1.576 / +1.931, are the
package's best). Clubs are positive at both lengths *quiet* at both colours and
are left alone; their one negative cell (exactly-five contested BV, −2.721) is
a contested tail, which defect 1 addresses rather than the rung.

**Step 0(c) — the escape's loss is entirely its tail.** `2♦` is quiet
**+1.110**/fired NV, **+1.360** BV and contested **−2.455** / **−2.106**, on
traffic that is 88% / 91% contested — the most-contested call in the lane, and
the one rung that never even had the `(X)` arm. The plan's follow-on (lift the
cap to `hcp(..=8)`) was then measured to be a **no-op** and is not built: the
escape is `hcp(..=6)` under a `natural_floor` of `(5, 0)`, so it already spans
exactly 5-6, and the passed-out bucket contains **zero** boards at 5 diamonds ×
7-8 HCP at either colour.

**Step 0(e) — defect 3 is a seam between two floors, not a quality judgement.**
The starved hands are 6+ diamonds with **0-4 HCP** (12,956 of 14,156 passed-out
six-card diamond boards, −33,530 plain NV; −30,610 BV): they failed `2NT`'s
quality gate and then failed the `2♦` escape's five-HCP floor. The base arm
bids its wide transfer on 14,153 of the 14,156.

**Step 0(b) — the `2♥` merge convicts itself.** Every one of the 3,069 divergent
`2♥` boards is the merged 2=3=4=4 shape, at −4.074 IMPs/fired quiet NV and
−4.935 BV, ending in a forced `5♣` (−3.832) or `5♦` (−4.324).

**Defect 2 re-confirmed on the fresh solve** at both colours, unchanged from the
August split: weak and quiet, exactly five clubs is **+1.405** IMPs/fired NV and
**−0.803** BV (−41,372 plain, −249,689 PD), while six (+0.993 / +0.668) and
seven-plus (+1.849 / +1.707) win at both.

###### What was built

| # | edit | evidence |
| --- | --- | --- |
| 1 | **the contested surface, both seats plus two the plan did not name** | defect 1, below |
| 2 | weak `2♠` leg → `points(..=7) & (len(♣, 6..) \| !vulnerable())` | defect 2; `over_overcall`'s free-bid idiom, length as the quality term |
| 3′ | `2NT` → `len(♦, 6..) & points(2..)`, the N1j transfer's own shape gate | 0(a) + 0(e), one edit for both |
| 4 | *not built* — the `2♦` cap lift is a measured no-op | 0(c) |
| 5 | `2♥` → `len(♥, 2..=2)`, the merge reverted | 0(b) |

**Defect 1 as built.** The measured failure is not that the floor is silent at
the unauthored nodes — **it is that the floor bids.** Censused over package B's
own `lia` arm, at opener's seat it pushes `4♣` on **72%** of `2♠ (3♠)` boards
and **96%** of `3♣ (3♠)`, `4♦` on 91% of `2♦ (3♥)`, and `3♦` on 91% of
`2♦ (2♥)`. Authoring `Pass`@0 is a decision to sell out, not a no-op, and it is
the substance of the repair. Two tables, and the asymmetry between them is the
design:

* **opener sits** (`landy_lia_overcalled`) — it cannot know which half of a
  two-way rung responder holds, and Pass is safe by the *auction's shape*
  rather than by a game force (their bid cannot be followed by three passes
  without responder speaking again). The exceptions ride only a rung that
  already promised values, i.e. the natural `3♣`/`3♦` invitations: the accept
  where their call left room below `3NT`, and the `len(major, 4..)` penalty
  double `probe-landy-opener-oracle` measured at opener's other seat in this
  lane.
* **responder captains** (`landy_lia_contested_rebid`) — it knows its half, and
  it is the seat that plays *over* the re-entering hand. It carries `3NT` on
  both majors stopped, the penalty double on `hcp(10..) & len(major, 4..)`
  (`hcp`, not `points`: distribution does not defend, and the `2NT` rung's weak
  arm reaches `points(10..)` on a seventh diamond alone), a finite `4m` game
  force so the strong half is **never** stranded, and the misfit leg's `3m`
  escape so a contested tail does not delete a rung the quiet tail offers.

The seat rotation is worth recording because it inverts the obvious reading and
was verified against the arm dumps, not argued: with `O L R A` clockwise,
`2♠ - 3♣ (3♥)` indexes `O L R A O` **`L`** — the hand that re-enters after
opener's length answer is the **overcaller**, whose partner has already passed,
bidding shape into a known 15-17.

Two nodes the plan did not name, both found by pricing the census:

* **the four level.** A first draft stopped the band at three, on the argument
  that the floor's delayed double there (47% of `2♠ (4♥)`, 74% of `2♠ (4♠)`)
  might be right. It is not: those two cells are **−3.434** and **−4.268**
  IMPs/fired, the worst per-fired cells in the whole `2♠` bucket.
* **the balancing seat** `{rung} - {leg} - - ({over})`, where responder passed
  opener's length answer and the *advancer* reopens: −5,509 (`(3♥)`) and −2,838
  (`(3♦)`) plain NV on the club rung alone, about a third of its contested
  deficit, at a node no draft had reached.

###### Flagged, not built

* **`landy_recue_answer`'s `4m`@20 has no answer node**, so
  `2♠ - 3♣ - 3♥ - 4♣ -` is floor-owned — and that floor is the one §N1o caught
  cue-bidding to `6♥` doubled. It is four of the five worst both-vul boards
  (`… 5♣ - 5♦ - 6♣ - 6♥ X`, −16.0 IMPs/fired on 59 NV boards). **The seat is
  shared with the base arm**, which runs away the same way one level lower and
  undoubled, so repairing it lia-gated would make this arm measure a repair the
  control wants too. Owed as its own A/B; the reversible default is a
  `multi_signoff_pass()` sit rail at `{completed} {3M} - {4m} -` in both arms.
* **`defense_2c_landy_lia` was in no `assert_package_invariants` sweep.** Every
  lia row had shipped with no totality check, no weight-tie check and no alert
  check — `landy_counter_package_invariants` now carries a lia × `landy_texas` ×
  `weak_2d_cap` arm.
* **`alert-sites.txt` does not move**, contrary to the repair plan: all four of
  its sections are built at `defense_2c_landy_lia = false`, and no `card.rs` row
  reads any `landy_*` knob. Nothing to re-bless while the knob stays off — which
  held again through the rebuild, `comp:landy-minor` included.
* **The seat immediately after opener's authored sit stays the floor's — six
  node families, not the one this list used to name.** Enumerated from
  `landy_bba_entries`' 201 registrations during the 2026-09-01 pre-flight audit
  of `ab-landy-lia-repair.sh`; all six are lia-only (none reachable in the base
  arm), so none of them cancels in the paired diff:

  | Family | Why it is not covered |
  | --- | --- |
  | `{rung} - {leg} - - (X)` — their **balancing double** | `landy_lia_entries` yields *bids only*, so the balancing loop never registers an `X` arm; `systems_on_over_double` cannot catch it either, because its guard is `s.first() == Some(&Call::Double)` and the first suffix call here is a Pass |
  | `{rung} - {leg} - - ({over}) - -` | responder after opener's authored balance-pass |
  | `2♦ ({over}) - -` | responder after opener's sit on the escape — opener only is registered |
  | `{fit} ({over}) - -` and `{fit} - - ({over}\|X)` | responder after the natural invitation is contested or declined. This is the ladder's **biggest measured win** (`3♣` +85,613 NV) and is contested on ~22% of its traffic; only `{fit} ({over})` and `{fit} ({over}) X -` are registered |
  | `{rung} ({over}) - - X ({run})` | their runout from the penalty double the repair itself newly authors |
  | `{rung} ({over}) - ({over2})` | one round deeper than opener's pass (the family previously listed here) |

  So **defect 1's closure is partial by construction**, and the repair's own
  authored `Pass` is what creates the fresh traffic — the same mechanism the
  repair exists to close, one seat later. Pre-registered in the runner header:
  if the arm reads a wash rather than a win, this is the first place to look,
  and falsifier 1's reading is a lower bound on what authoring the tails buys.
  The reversible default is (a) `landy_lia_entries(leg).map(Some).chain([None])`
  with an `Option<Bid>` in `landy_lia_overcalled`, the shape
  `landy_lia_contested_rebid` already takes, so their balancing `X` gets the
  sit, and (b) `multi_signoff_pass()` at the four responder-follow-up families.
  Not built at the time: it would have changed what that A/B measured.
  **All six are closed by the 2026-09-01 rebuild**, which took exactly those
  defaults once the A/B they would have disturbed was superseded — see
  ["The six flagged families are closed"](#the-six-flagged-families-are-closed).
* **`park/landy-pdi` owes a rebase.** The repair adds two `.penalty()` rows
  (`landy_lia_overcalled`'s invitation double, `landy_lia_contested_rebid`'s),
  and [docs/pdi.md](pdi.md) makes `grep -rn '\.penalty()' src/bidding` the live
  trigger inventory — so both enter it automatically. The branch has no row on
  `main` to annotate (`git branch --list 'park/*'` is its whole index), so the
  note lives here: rebasing it onto this repair widens §N1n's trigger set by
  two lia-gated sites, both inert while the knob is off.

##### Rebuilt as true Lia (2026-09-01) — **this is the build `ab-landy-lia2.sh` measured**

The knob was **redefined in place** — it never shipped, its off state is
byte-identical, and pinning the old semantics by sha is cheaper than a second
knob whose only job is to neutralise the first (the house rule on scaffolding
knobs). Everything below is behind `defense_2c_landy_lia`, still default off.

###### Responder's table

| Call | Weight | Rule | Alert |
| --- | --- | --- | --- |
| `2♥` | 177 | `4+♣ & 4+♦ & len(♥, ..=2) & len(♠, ..=2) & points(8..)` | `comp:landy-tko` |
| `3♥`/`3♠` | **179/178** | splinters, rule unchanged | `comp:landy-spl` |
| `2♠` | 174 | `len(♣, 6..) & points(8..)` | `comp:landy-minor` |
| `2NT` | 173 | `len(♦, 6..) & points(8..)` | `comp:landy-minor` |
| `3♣` | **141** | `len(♣, 6..) & points(..=7)` | none — natural |
| `3♦` | **139** | `len(♦, 6..) & points(..=7)` | none — natural |

Everything else is the shared table verbatim: `3NT`@180/@168, the `4M` jam,
`X`@145, `2♦`@140, `Pass`@0.

Four decisions worth the ink.

**The takeout is spelled as shape, and the shape is the whole delta.** Lia
states *UNBAL, 4+♦ 4+♣* and no point count, so the band is ours (invitational
up, unlimited above; the weak 4-4 minor hand passes). "Unbalanced with 4+4+ in
the minors" is exactly `len(♥, ..=2) & len(♠, ..=2)` — with eight-plus cards in
the minors and no three-card major the holding is nine-plus in the minors, which
is every unbalanced shape in the family and no balanced one. It therefore still
**excludes 2=3=4=4**, the merge the first build let in and the forensic
convicted (−4.074 IMPs/fired, all 3,069 divergent boards that one shape); those
hands re-route to `3NT` by weight, where the base arm plays them. The
misprobe's "her 5+5+ leaves the 4-4 singleton-major hand homeless" was never
her rule, and under the corrected one those hands are *inside* the takeout.

**Which forces the splinters above it.** Lia's shape contains the splinters',
so unlike N1j's exact doubletons the two families overlap and the weights have
to arbitrate: `3♥`/`3♠` move to 179/178 and the takeout to 177, so a
game-forcing 0-1 major makes the more descriptive call and the same shape at
8-9 takes the cheaper takeout. N1j's ordering is untouched.

**The minors invert, and six cards is the rule at both ends.** `2♠`/`2NT` are
*natural* six-card minors — responder declares, no transfer, so
`comp:landy-transfer` is not the tag and `comp:landy-minor` is new. Opener
answers by length (`comp:landy-length`, unchanged) or accepts at `3NT` from the
top of the range with both of their majors stopped — the new rung, and the one
the invitational band cannot do without: a describe-only structure walks a
24-count into `3♣`. The sign-offs one step higher want six too, which is Lia's
own "6+, rarely 5" and also what the forensic said about the *first* build's
five-card weak leg (exactly-5 **+1.405 white / −0.803 red**, 45% of the arm's
both-vul PD deficit; six and seven-plus won at both). With exactly five and a
bust there is now no rung, which is the correction's one predicted loss — see
falsifier 1 below.

**The sign-offs straddle the escape, and that closes defect 3 for free.**
`3♣`@141 sits *above* `2♦`@140 because clubs have no cheaper outlet; `3♦`@139
sits *below* it, so the escape keeps every hand it can take and `3♦` picks up
exactly the ones it refuses — the 6+♦ hands with **0-4 HCP** that failed the
escape's five-HCP `natural_floor` and were package B's "starved diamonds"
(12,956 of 14,156 passed-out boards, −33,530 NV / −30,610 BV), plus the
seven-point hands its `hcp(..=6)` cap excludes. No widening, no knob: rung
order alone. Nothing in the package reads `vulnerable()` any more.

###### Continuations

* **Opener's takeout answer** keeps the reversed priority — four-card minor,
  then `2NT` = spade stopper, then the `2♠` ask — with `3NT`@160 on top.
* **Responder's placements re-banded for INV+.** `landy_bba_pick_rebid`'s
  `5m`@100 and `landy_bba_takeout_rebid`'s `3NT`@100 were game-forcing
  catch-alls; the lia twins (`landy_lia_pick_rebid`, `landy_lia_takeout_rebid`)
  gate them on game values and make `Pass`@0 the finite rung, so 8 opposite a
  minimum stops in `2NT` or `3m` instead of bidding 24-point games.
* **The `2♠` ask's catch-all is `3♣`@20, not `4♣`@20.** Two things were wrong
  with the four-level version and the invitational band made the second fatal:
  it was an unalerted artificial call (vacuous constraint, so the invariant's
  witness could not see it), and it committed to the five level opposite eight
  points. `3♣` names a suit responder holds by the takeout, so it is natural
  and the flagged item retires. It is not a *fit* promise — our `1NT` caps both
  majors at four, so an opener denying four in both minors is 3-3, 3-2 or 2-3
  there and the landing can be 4-2 — and dropping below `3NT` opens a node the
  four-level rung could not: unauthored, the floor bids `3NT` on **every**
  probed hand, in the strain opener's own ask just denied a stopper in. One
  edit answers both: `landy_lia_ask_landing` has opener correct to `3♦` on a
  club doubleton with three diamonds, which turns the 4-2 into a 4-3, and pass
  otherwise. Found by the build's adversarial review.
* **`landy_lia_misfit_rebid`'s `signoff`@60 now carries real traffic.** The
  first build resolved "INV+ may pass" to sign-off-always on the grounds that
  the two-way rungs had no 8-9 band; they do now, and the rung is what stops
  the invitational hand being stranded in a doubled 6-2 notrump.
* **The minor-transfer-slam rule is honoured on the acceptance too.** `2♠`/`2NT`
  are uncapped, so opener's `3NT` can land opposite a slam hand;
  [minor-transfer-slam.md](minor-transfer-slam.md) requires a `4m` rung above
  that `3NT` **with an authored answer**, because an unauthored `4m` reads as
  nothing and the floor's keycard ask is gated on `undisturbed`, which this
  lane never is. `landy_lia_accept_rebid` is the rung; `landy_slam_answer` and
  `slam::rkcb_rows` re-hang on it exactly as on the length legs.
* **`values` is now true on the contested tails of `2♠`/`2NT`.** The first
  build's rungs were bust-or-game-force, so opener had nothing to act on; Lia's
  promise 8+, so opener's `3NT` accept and its four-trump penalty double ride a
  promise the rung makes. Over the six-card sign-offs opener still sits on
  everything. That double needs the mirror of responder's — a `multi_signoff_pass()`
  sit at `{rung} ({over}) X -` and at the balancing `{rung} - {leg} - - ({over}) X -`
  — because the registration that used to cover it rode the invitation rung
  that has become a sign-off. Caught by the build's adversarial review, not by
  the invariants, which do not know a seat is missing.

###### The six flagged families are closed

The 2026-09-01 pre-flight audit found six lia-only node families still reaching
the floor after the repair, and deliberately left them (they would have changed
what the stopped A/B measured). That constraint died with the A/B, so the
rebuild takes the reversible defaults the audit wrote:

| Family | Now |
| --- | --- |
| `{rung} - {leg} - - (X)` — their **balancing double** | `landy_lia_entries(leg).map(Some).chain([None])` and `landy_lia_overcalled` takes `Option<Bid>`; the penalty rung drops, the sit and the accept stay |
| `{rung} - {leg} - - ({over}) - -` | responder sits (`multi_signoff_pass`) — it passed the length answer once |
| `2♦ ({over}\|X) - -` | responder sits; the escape's `(X)` arm exists now too |
| `{3m} ({over}\|X)`, `{3m} ({over}) - -`, `{3m} - - ({over}\|X)` | opener sits under every entry including their double; responder sits wherever the auction returns |
| `{rung} ({over}) - - X ({run})` | opener sits through their runout from responder's penalty double |
| `{rung} ({over}) - ({over2})` | responder captains (`landy_lia_contested_rebid`) — it has not spoken since the rung |

What is still floor-owned is now one seat deeper again, and the honest statement
is that authoring a `Pass` always creates fresh traffic below it; the difference
is that the families above were the ones the census could *price*.

##### Verdict of the rebuild — **measured a loss 2026-09-01; the knob stays default off**

`scripts/ab-landy-lia2.sh`, SEED_BASE 1788264406, control `main` HEAD sha
`32242d63`, 4,608,000 boards/arm/vul, both isolation gates **0 foreign**:

| vul | fired | plain DD | PD |
| --- | --- | --- | --- |
| NV | 143,158 (3.11%) | **−0.0077** ±0.0009 (−35,676; −0.249/fired) | −0.0127 ±0.0011 (−58,701; −0.410/fired) |
| BV | 123,212 (2.67%) | **−0.0059** ±0.0010 (−27,354; −0.222/fired) | −0.0172 ±0.0012 (−79,454; −0.645/fired) |

Plain DD was the runner's pre-registered arbiter at both colours and both cells
are negative, so there was no plain win to ship on. The sd pass was **killed
unrun** — four negative cells need no lead model. Both arm directories and
`imps-{none,both}.jsonl` are kept; everything below is read off them.

**The loss is the diamond leg, and the club leg is a clean win.** Splitting the
divergence by which leg moved (baseline `2NT` = its club transfer, `3♣` = its
diamond transfer; candidate `2♠`/`3♣` clubs, `2NT`/`3♦` diamonds):

| leg | n (NV) | plain NV | plain BV |
| --- | --- | --- | --- |
| club | 57,495 | **+58,256** (+0.0126/bd) | **+63,018** (+0.0137/bd) |
| diamond | 49,380 | **−92,508** (−0.0201/bd) | **−83,517** (−0.0181/bd) |
| rest | 36,283 | −1,424 (−0.0003/bd) | −6,855 (−0.0015/bd) |

`2NT → 3♣` is the whole run's biggest cell (**+46,715 NV / +54,641 BV**), so
unwinding N1c's right-siding trade for a natural club rung is right and the
club leg is kept. Falsifier 1 (the exactly-five 8-9 hand has nowhere to go) is
therefore **not** where the loss went. The five worst cells NV are all diamond
or sell-out: `3♣ → 2♦` −36,875, `3♣ → 3♦` −36,119, `3♦ → -` −22,119,
`3♣ → 2♥` −10,344, `3♣ → 2NT` −8,708.

**Five findings, each of which is a row of the refinement below.**

1. **No weak diamond rung beats the baseline's wide transfer.** Re-solving the
   boards where the baseline bid its `3♣`→♦ transfer, bucketed by responder's
   own diamond holding, every candidate call is negative on plain DD and the
   INV+ rung is the only one inside −1.0 per fired:

   | responder holds | lia bid | n (NV) | plain/fired NV | BV |
   | --- | --- | --- | --- | --- |
   | 6♦ thin, 0-4 HCP | `3♦` | 13,533 | **−1.966** | −1.773 |
   | 6♦ thin, 5+ HCP | `2♦` | 9,223 | **−2.534** | −2.464 |
   | 6♦ two top honors | `2♦` | 1,361 | −2.617 | −2.290 |
   | 7+♦ | `2♦` | 2,345 | **−4.238** | −4.859 |
   | 7+♦ | `3♦` | 2,897 | −2.701 | −2.713 |
   | any | `2NT` | 11,839 | −0.709 … −0.762 | −0.342 … −0.712 |

   This is the "hybrid nobody has built" from the repair queue, priced: the
   diamond leg wants a transfer, and the weak rungs it replaced are worth about
   −2 IMPs/fired each.

2. **The `Pass`@0 sell-outs were selling out to a floor that was right.** Cell
   `3♦ → -` — the baseline competed to `3♦`, the candidate passed — is 14,717
   boards at **−22,119 plain NV / −13,364 BV**, and 14,699 of them are after
   our own `2♦` escape: `1NT (2♣) 2♦ (2♠) -` (6,836 bd, −9,724) and
   `1NT (2♣) 2♦ (2♥) -` (5,895 bd, −11,878). The 2026-09-01 census read the
   floor's behaviour correctly (it does bid) and drew the wrong conclusion from
   it. **The code comment at `lebensohl.rs` calling the floor's `3♦` "a
   law-of-total-tricks violation" was wrong** — the doc/code discrepancy
   flagged to jdh8 on 2026-09-01 is resolved in the measurement's favour, and
   the comment is gone.

3. **Right-siding is not null here, and the first reading of it took the wrong
   column.** Declaring *side* flips on 123 NV / 66 BV boards — that is the
   zero that was recorded. Declarer *seat* flips on **26,664 NV / 27,911 BV**
   same-contract boards, worth **−6,406 (−0.0014/bd) / −10,416 (−0.0023/bd)**
   plain, worst cell `2NT → 3♣` (−3,630 / −5,600). Package C's correction to
   the iron rule applies exactly: DD prices the lead direction, so a seat flip
   is visible and real, and what the natural rungs gave back was declarership.

4. **The by-length answer answered the wrong question.** `comp:landy-length`
   told responder which partscore to pick; an invitational rung's question is
   whether this is a game.

5. **The `X`-versus-takeout cell is genuinely mixed.** Cell `X → 2♥` — the
   baseline doubled, the candidate took out — split by responder's own majors:
   the 8,113 NV / 6,236 BV boards holding two-plus in each major are **+1.854 /
   +1.154 plain** IMPs per fired for the takeout and **−1.305 / −2.169** under
   perfect defense. Plain says take out, PD says double, and the runner's own
   arbitration note says the one PD loss this lane does *not* wave through is a
   mechanism that removes our penalty doubles — which is what taking out with
   values does.

##### Refined on the lia2 forensic (2026-09-01) — **measured a LOSS 2026-09-02 (lia3)**

Same knob, still default off, defaults byte-identical. The refinement keeps the
club leg intact and re-cuts everything the forensic convicted.

###### Responder's table

| Call | Weight | Rule | Alert |
| --- | --- | --- | --- |
| `3♥`/`3♠` | 179/178 | splinters, unchanged | `comp:landy-spl` |
| `2♠` | 174 | `len(♣, 6..) & points(8..)` | `comp:landy-minor` |
| `2NT` | 173 | `len(♦, 6..) & points(8..)` | `comp:landy-minor` |
| `X` | 145 | `hcp(8..) & len(♥, 2..) & len(♠, 2..)` — **narrowed** | `comp:landy-values` |
| `2♥` | **144** | `4+♣ & 4+♦ & points(8..)` — **no major term**, and below the double | `comp:landy-tko` |
| `2♥` | **143** | `len(♣, 5..) & len(♦, 4..=4) & points(4..=7)` — the weak band, **new** | `comp:landy-tko` |
| `3♦` | **142** | `(len(♦, 7..) \| len(♦, 6..) & top_honors(♦, 2..)) & points(..=7)` | none — natural |
| `3♣` | 141 | `len(♣, 6..) & points(..=7)` | none — natural |
| `2♦` | 140 | `len(♦, 5..) & hcp(..=8) & floors` — **ceiling raised** | none — natural |

`3NT`@180/@168, the `4M` jam and `Pass`@0 are the shared table verbatim.

**The takeout moves under the double, and the double gains a shape term.** Lia
states the takeout as shape in the minors and says nothing about the majors, so
neither band carries a major term and the rung *order* does that work instead:
8+ with two-plus in each of their suits doubles, 8+ with a short one takes out,
10+ with a singleton splinters. That is finding 5, resolved for the double —
plain favours the takeout by +1.854/+1.154 per fired and PD favours the double
by −1.305/−2.169, and this lane does not wave through a PD loss whose mechanism
is removing our penalty doubles. **Pre-registered as falsifier 1 of the next
arm**: if the club leg's win shrinks and this is where it went, flip the two
weights back. The deliberate orphan is the 8-9 hand with a short major and not
both minors, which now has no rung.

**The weak takeout band is ours, not hers**, and it exists for the hand no
other rung reaches: five-plus clubs and exactly four diamonds at 4-7, too short
of diamonds for the escape and too short of clubs for `3♣`. Its five-card club
guarantee is what makes opener's `3♣`-before-`3♦` answer priority safe.

**The diamond leg is re-cut three ways** (finding 1). `2NT` carries the whole
INV+ band — it is the least-bad diamond cell in the forensic by a factor of
three — `3♦`@142 re-gates to *excessive* diamonds, and the `2♦` escape's
ceiling rises to eight HCP to take everything they leave ("bid `2♦` if
possible"). `defense_2c_landy_weak_2d_cap` keeps governing the base arm only:
it caps a rung this ladder has re-cut, so crossing them would measure two edits
at once. **Pre-registered residue**: the weak six-card diamond hand is still
off the transfer, and that is the −1.97/fired cell. If the next arm's diamond
leg is still negative, the answer is the wide transfer (`points(2..)` on `2NT`)
and the INV+ gate is what has to go.

###### Continuations

* **Opener answers the minor rungs off one Max-break+ table**
  (`landy_lia_max_break`), the same shape on both legs:

  | Answer | Meaning | Weight |
  | --- | --- | --- |
  | the step below the completion (`2NT` over `2♠`, `3♣` over `2NT`) | super-accept: maximum with three-card support (`comp:landy-super`, **new**) | 161 |
  | `3NT` | maximum, no super-accept — `landy_lia_accept` verbatim | 160 |
  | the completion (`3♣` over `2♠`, `3♦` over `2NT`) | the minimum default, and the finite catch-all | 100 |

  Findings 3 and 4 in one edit: the completion puts **opener** in the contract
  on the common branch, which recovers the right-siding the length table gave
  back, and the two rungs above it answer whether this is a game.
  `comp:landy-length` retires with `landy_lia_minor_answer` and
  `landy_lia_misfit_rebid`. Three-card support is the reversible default on the
  super-accept — opposite the rung's six that is a nine-card fit; flip it to
  `4..` if a probe disagrees.
* **Responder over the super-accept** (`landy_lia_super_rebid`): the `4m` slam
  try, `3NT` on both majors held, else retreat to three of the minor, which is
  the **finite catch-all** rather than a `Pass` — on the diamond leg the
  super-accept is `3♣` and passing it would leave a six-one fit on the table.
  Opener sits for the retreat. The `3NT` gate is `points(9..)`, one below every
  other lia rung, because opener has published `hcp(16..)` here and nowhere
  else; at ten it would strand exactly the hand the super-accept exists to
  find.
* **Responder over the completion**: `landy_bba_transfer_rebid` verbatim — the
  cues, the `4m` slam try, `3NT`, and `Pass`@0 for the invitation declined.
  `landy_recue_answer` rides this leg only; the super-accept's rebid has no
  cues, and a node under a call no arm makes is a book node shadowing the floor
  for nothing.
* **Responder over the `3NT` accept**: `landy_lia_accept_rebid` unchanged, with
  `landy_slam_answer` and `slam::rkcb_rows` re-hanging on all three legs
  ([minor-transfer-slam.md](minor-transfer-slam.md)).
* **The weak takeout band gets one rung**: `3♣`@60 pulls opener's `2NT` on
  `len(♣, 5..) & points(..=7)`, and opener sits for it (`2♥ - 2NT - 3♣ -`).
  Opener denied a four-card minor to bid `2NT`, so it is facing a five-two club
  fit with no values, and sitting it is the one thing the weak hand must not
  do. Unauthored the floor bids a phantom `3♦` there.

###### The contested surface, restricted to the rungs that promise something

Finding 2 as built. `landy_lia_overcalled` loses its `values` parameter and its
`Pass`@0, and its registrations over the weak `3m` sign-offs and the `2♦`
escape are **deleted** — those seats are the floor's again, which is what the
measurement says they should have been. What remains rides `2♠`/`2NT` only and
is three narrow rungs with no catch-all:

* `3NT` — the accept, where their call left room for it below the game.
* `X` — penalty on `len(major, 4..)`, the oracle's gate, at the three level and
  below.
* the **completion**, while their entry has left it cheap enough to be legal —
  new, and the same right-siding argument as the quiet table. In practice it
  fires at `2NT (3♣)` and nowhere else: the club leg's completion is `3♣`
  itself, and every entry above `2♠` is already past it. **A new rung owes a
  new seat**, and the build's adversarial review caught this one: responder is
  uncapped over `2NT (3♣) 3♦`, and left to the floor that seat is sound on
  strength (pass 8-11, `3NT` 12-14) but bids a phantom `4♥` on ~8% of the band,
  heart voids included, with opener answering `4♠` on every hand.
  `landy_lia_contested_rebid` takes it, its `(X)` tail answers verbatim, and
  opener sits for the game-forcing `4m`.

Those registrations are exact **`Pattern::node`s**, not `Pattern::after`
guards. This is package A's finding re-used: `Trie::resolve_floored` returns a
guarded fallback's all-−∞ logits unchecked and the driver passes, so deleting a
`Pass`@0 behind a guard is a silent no-op. `landy_lia_contested_rebid` —
responder's captain table — is unchanged and keeps its `Pass`@0: it is the seat
that knows its strength, and it must never be stranded.

###### Verdict (2026-09-02) — lia3 lost four of four; the lane parks behind a general floor rail

`scripts/ab-landy-lia3.sh`, `SEED_BASE=1788290089`, control `deeb0252`, 4.608M
boards/arm/vul, both isolation gates 0-foreign. Plain DD, the pre-registered
arbiter at both colours, is negative at both:

| vul | fired | plain DD /board | PD /board | SD plain | SD-PD |
| --- | --- | --- | --- | --- | --- |
| NV | 197,920 (4.30%) | **−0.0056** ±0.0011 (−25,685) | −0.0331 ±0.0013 | +0.0058 | −0.0158 |
| BV | 163,430 (3.55%) | **−0.0254** ±0.0012 (−117,094) | −0.0569 ±0.0014 | −0.0090 | −0.0342 |

The knob stays off. Against lia2 (−0.0077/−0.0059) NV is a shade better and
BV four times worse, on a third more fired boards. The VERDICT block in the
runner carries the full forensic (every number below was recomputed by an
independent twelve-claim adversarial pass the same day; its corrections are
folded in). What it says, in order of size:

* **The diamond leg did not move** (−86,528 / −91,463 plain, −0.0188 /
  −0.0198 per board against lia2's −0.0201 / −0.0181) and every one of its
  cells loses to the baseline's wide `3♣` transfer: the six thin diamonds
  that now pass (`3♣ → -`, −35,563 / −34,235 on 13,552 / 12,757 boards), the
  INV+ transfer (`3♣ → 2NT`, −26,135 / −27,514), the escape (`3♣ → 2♦`,
  −19,540 / −16,145) and the excessive-diamond sign-off (`3♣ → 3♦`, −19,274 /
  −17,213). Finding 1's pre-registered residue is answered: no rung set for
  the diamond hand beats the wide transfer, so on that leg **the INV+ gate is
  what has to go**.
* **The club leg's win halved** (+27,990 / +14,627, was +58,256 / +63,018).
  `2NT → 3♣` still carries it (+40,933 / +39,039); the INV+ rung flipped
  (`2NT → 2♠` −6,848 / −11,862, was +14,368 / +14,194) through the Max-break+
  answer (the `2♠ - 2NT` super-accept prefix, −8,890 / −11,951 — finding 4's
  replacement loses) and the contested seats the refinement handed to the
  floor (`2♠ (3♠)` −3,848 plain NV at −4.8 per fired; its `4♠` class −9.9).
* **The weak band is the both-vul catastrophe.** `- → 2♥` (baseline passed,
  lia took out on the 4-7 band) is −2,662 NV but **−42,572 BV** on 28,817
  boards (−107,283 PD). The route is the flagged accept: `2♥ - 3NT` is
  −8,317 / −36,378, and split by responder's HCP (a proxy; the band is in
  points) the 0-7 band is **−14,515 / −38,483** (10,284 / 8,841 boards)
  while the 8-9 band is +6,198 / +2,105. Opener's `3NT`@160 opposite four
  points is doubled and sat.

**The floor: association first, then the fact that holds.** A new tool,
`examples/probe-layer-replay`, re-bids every divergent board through the arm's
own partnership and stamps each of our calls book-or-floor (0 of 361,350 boards
failed to reproduce). Boards on which the learned floor made at least one of
our calls at or after the divergence: NV 124,936 boards, −72,224 plain
(−118,074 PD); BV 94,870, −132,357 (−181,836); boards the book bid to the end
+46,539 / +15,263. That is an association, not an attribution: the **base**
arm's floor is involved on *more* of the same boards (NV 141,958 vs 124,936,
BV 107,264 vs 94,870), the divergent call is the book's on 177,112 / 148,603
boards, and half of the lia floor-involved plain loss sits on boards where the
floor only ever passed (NV −38,515 of −72,224, BV −37,199; PD-positive there).
The fact that holds: **lia doubles the boards on which the floor makes a
substantive, non-pass call** — NV 86,491 against the baseline's 42,994, BV
62,135 against 23,066 — and the cell where lia's floor bids while the
baseline's book or floor only passed is NV 67,239 boards / −29,948 plain /
−165,161 PD, BV 53,103 / −97,643 / −213,559. (Every bucket here is over
divergent boards only; identical boards are absent by construction.) Two
floor classes are worth naming:

* **Phantom suits** — floored suit bids on ≤4 cards where partner's announced
  minimum makes ≤5 combined: `2♠ (3♠) 4♠` on a doubleton (−10 per fired), `3♦
  - - (3♠) 4♦ - 4♠` (−10), `2♥ - 3♣ - - (X) - - 3♦ - 3♥` (−5 to −8).
  `probe-decision` shows the net reading partner correctly (♣6+, 8+ points)
  and bidding `4♠` anyway, logit 9.41 over `4♣`'s 9.34. The mechanism is the
  input, not the reading. The shipped floor's vector (`features_v6`, 176
  values: the disclosable hand summary, a 36-value context block, all four
  seats' announced envelopes, vulnerability, both compact convention cards)
  carries a *we-bid-this-strain* bit per strain and partner's last bid — raw
  call identity, set for every bid artificial or not (`context.rs:616`) — and
  **no alert or tag column** (measured at +0.004 NLL and left out; the
  evaluator's call-identity tail reaches the floor only through
  `competitive_gate` and the accountant). Over `2♠ (3♠)` the net is told "we
  bid spades, partner's last bid was `2♠`, partner has ♣6+", a joint the BBA
  corpus never showed it; the compact card has no lia slot
  (`defense_2c_landy_lia` is never read in `features.rs`), so the regime
  vector is N1j's, under weights from 2026-08-18 (`9fb333f5`). So alerting
  more changes nothing it sees, and masking the strain bit for alerted calls
  would move its inputs off the training distribution the other way — a
  retrain, not a rail.
* **Six-card pushes** — floored `3♦`/`4♦` on the weak rung's own suit over
  their raise (`3♦ - - 3♠ [4♦]` −9.3 per fired; the balancing `- 2♥ - - [3♦]`
  −3.9): level judgment, not phantom, and outside a suit veto.

**Falsifiers.** 1 **does not fire on its own terms**: the club leg's win did
shrink, but it went to `2NT → 2♠` and `2NT → 2♥`, not to `X → 2♥`, which is a
CI-clear plain win (+18,457 / +6,138; PD −3,064 / −14,130, the artifact
shape) — with the caveat the pre-registration missed, that the cell is 100%
short-major 8-9 hands because the 2-2-major hands the flip gave to `X` now
match the baseline's `X` and never diverge (`2♥ → X`: 0 boards), so the
flip's central move is unmeasured. 2 **fires** (above; the recorded repair is
the accept below the minor picks). 3 **not supported on plain DD and
PD-negative**: exactly-five 8-9 diamond hands are NV `X` 4,082 boards +6,007
plain / −2,795 PD, `2♦` 1,851 +2,414 / +1,003, pass 365 +327 (BV `X` +1,470 /
−8,541, `2♦` +402 noise, pass +453; class −1,358 / −9,329 PD) — and the `X`
bucket is not the ladder's `X`: the baseline doubled on every one of those
boards and 3,375 / 3,248 of them diverge only at responder's later floored
`3♦` over `X (2M) - -` (+6,512 plain of the +6,007), the floor's pull. 4
**fires**: `3♦ - - 3♠` 605 boards −5,543 at −9.2 per fired and `3♦ - - 3♥`
759 / −4,130 (BV 139 / −1,355, 154 / −894); over `3♠` the floor pushes `4♦`
on 550 of 605 (always 6+) and, when RHO passes it, always phantom-cues `4♠`
(427 boards, own ≤4 on 415); over `3♥` it passes as often as it pushes (396
vs 363) and the passes cost nearly as much (−1,679 vs −2,451). lia2's −22,119
`3♦ → -` sell-out is gone by **removal**, not reversal — its node `2♦ (2M)`
keeps 2 / 6 boards and the residual cell (1,126 / 271, +244 / +474) sits at
`X (2M)`, floored in both arms — lia3 measures nothing about the floor at
the old node. 5 **partly** (`2♥ (2♠) -` −4,401 /
−3,631, the floor sits at −1.1 / −1.5 per fired; `2♥ (3♥) X` −875 / −7,358
plain but −13,846 / −19,418 PD; nothing here is the takeout's problem, the
band is). Watch list `2♥ (4♥/4♠)`: 344 / 302 boards, +45 / −18 — nothing.
Predictions: diamond leg non-negative **no**; club leg positive yes, halved;
`3♦ → -` flips — removed, see 4; declarer-seat flips recover **no** (same
contract with doubled status, both NS, other seat: −6,728 on 28,345 NV /
−9,091 on 27,378 BV against lia2's −6,910 / −10,323 by the same definition,
flat at −0.0015 / −0.0020 per board).

**Three book mechanisms the verification pass named** (the lia4 list below
is built on them, not on the leg totals):

* **The diamond leg's biggest cell is a hand with no rung.** `3♣ → -` is
  13,534 of 13,552 NV and 12,751 of 12,757 BV boards of responder **0-4 HCP
  with exactly six diamonds**: `2♦`@140 needs five HCP (`floors`), `3♦`@142
  seven diamonds or two top honours, `2NT`@173 eight points — so `Pass`@0.
  −35,552 / −34,223 plain, a PD wash. In lia2 the same class bid `3♦` at
  −1.97 / −1.77 per fired (−26,610 / −22,615): the re-gate cost −8,942 /
  −11,608 *more*. The baseline's wide transfer takes this hand at
  `points(2..)`; that, not the INV+ gate as such, is what "the wide transfer
  wins" means.
* **Both INV+ rungs lose through `landy_lia_super_rebid`'s `retreat`@0.**
  Opener's super-accept relay@161 outranks its own `3NT`@160, so it may hold
  the stoppers; responder's `3NT`@120 then needs both major stoppers and
  `4m`@130 needs 13+, and the 10-12 HCP hand retreats to three of the minor:
  diamonds NV 2,050 boards / −9,737, BV 1,434 / −10,491; clubs NV 2,084 /
  −9,486, BV 1,591 / −10,822 (≈ −19.2k / −21.3k plain) — `3NT` (base) → `3m`
  (lia) on 1,751 / 1,251 boards at −5.5 / −8.3 per fired. This is the club
  leg's flip; the "Max-break+ answer loses" line above is this table, not
  the accept itself.
* **A floored-both cell the reading moved.** `- → X` (NV 8,422 boards /
  −11,801 plain / −32,459 PD; BV 6,200 / −12,162 / −31,976, the third-largest
  PD cell) is 6,558 / 4,977 boards at `X (2♠) - -` where **both** arms are
  floored (lia doubles again, base passes): the narrowed `X` reading moves
  the floor's reopening.

**Colour and the SD seam.** The diamond leg is colour-neutral (−86.5k /
−91.5k); the 4-7 takeout band is the whole colour effect — the `2♥` rung's
0-7 HCP boards are −12,856 NV against **−58,640 BV**, half the BV loss, and at
NV the `2♥ - 3NT` loss is entirely the doubled `3NT`s (3NTx 2,398 / −17,205,
undoubled 6,724 / −873) while BV also loses undoubled (5,918 / −11,810); a
vulnerability gate on the weak band is the obvious untested arm. The NV
sd-lead plain +0.0058 ±0.0011 is a CI-clear win the size of the DD loss (DD −
SD = −0.0114 NV / −0.0164 BV, the opposite sign to package D's +0.014), so the
NV verdict sits inside the lead-model seam; BV loses on all four columns. And
the sd pass ran with no `--on-ns-landy-lia` disclosure — `ab-dump-sd` has none
— the caveat this doc already attaches to D.

**The rail evidence.** Cutting the floored suit bids by the two inputs an
envelope-gated veto would read — the bidder's own length and partner's
announced minimum in the suit, no bid-identity term — the class *floored suit
bid, own ≤4, own + partner-min ≤5* at or after the divergence: on the lia
arm NV 12,513 boards, net **−30,369 plain** (gross −54,120 lost / +23,751
won; −65,663 PD), BV 7,177, net **−33,277** (gross −46,975 / +13,698; −55,043
PD); per call by level, four-level −27,642 / −26,536, three-level −5,241 /
−11,253, five-plus −1,529 / −1,718, two-level nothing (per call, so a board
with two vetoable calls counts twice; the level sum overstates the per-board
net by ~13% — four-level deduplicated 10,686 / −26,687 NV, 5,667 / −24,843
BV). **The base arm on the
same boards** carries one on 6,800 / 3,844 boards and lost on them — **5.2 /
6.5 IMPs per fired**, +35,346 / +25,145 plain net from the candidate's side
(gross +45,554 / +32,010 won against −10,208 / −6,865): the default system's
own floor phantom-bids the same way, lia only put more seats in front of it.
These are the pools a veto would act on, not bounds and not measurements: the
masked call is replaced, not undone; the veto also fires on the ~4.4M
non-divergent boards absent from these files; and a floor change moves both
arms, which is why its A/B is on the default system. The six-card push class
is outside it.

**Disposition (jdh8, 2026-09-02): the general fix first.** The
envelope-gated new-suit veto on the floor — the M6.4-style rail named as this
lane's residue in `docs/archive/one-notrump-competitive-closed.md` since the
2026-08-14 post-ship decompose — gets built and measured on the **default
system** with its own non-inferiority A/B, as its own task. No local nodes
for lia until that has run; if the rail flips the phantom classes here, the
lane re-measures under it (lia4) with the book findings above as its only
book changes: the accept below the minor picks (falsifier 2), the 0-4 HCP
six-diamond hand back on the wide transfer, a values `3NT` in the
super-accept rebid, and a vulnerability gate on the weak band. Recorded and **not built**: restoring
`landy_lia_overcalled`'s `Pass`@0 over the INV+ rungs (lia2 had it, at sha
`32242d63`, and that leg won). Kept: both arm directories, `imps-*.jsonl`,
`layers-*.jsonl` (candidate and base), and the bucket tables under
`ab-results/landy-lia3/`.

###### Owed, and flagged

* **The A/B** — `scripts/ab-landy-lia3.sh`, **run 2026-09-02, lost 4/4 —
  see the verdict above**; the falsifier outcomes are there too. As
  pre-registered: fresh `SEED_BASE`, control = then-current `main`, both
  colours, sized to match the three prior runs. The falsifiers were: the `X`-versus-takeout weight flip (1), opener's accept over
  the two-band takeout (2), the diamond leg's remaining weak-hand residue (3),
  whether handing the weak rungs' contested seats back to the floor is
  worth the −22,119 the sell-outs cost (4), and the contested takeout's own
  floor handoff (5, next bullet). The lia2 verdict block above
  carries the numbers each of them is measured against. On the forensic
  **watch list**, no pre-stated number: `2♥ (4♥/4♠)`, where the floor's
  delayed double was the worst per-fired cell class one rung over
  (`2♠ (4M)`, −3.434/−4.268) but this seat was never priced.
* **The contested `2♥ ({raise})` seat is the floor's (2026-09-02).** The
  pre-launch review caught `landy_bba_takeout_overcalled` riding the two-band
  takeout: its `Pass`@0 was authored safe "because responder's game force
  guarantees another turn", and no lia band is a game force — the weak band
  never bids again, making the `Pass`@0 the sell-out class the lia2 A/B
  convicted, and its free 2NT/3NT-on-a-stopper a game bid opposite a possible
  four-count (the same shape as the flagged `3NT`@160 accept, one seat
  later). The registration is skipped under `lia_takeout` — the same handoff
  as the weak rungs' contested tails; the splinters (`points(10..)`) keep the
  ladder. The old forensic tallied this seat at 16 boards, −35/−10, but the
  weak band raises its frequency. Reversible: drop the `!lia_takeout` guard
  in `landy_bba_entries`. **Falsifier 5**: if the lia3 forensic shows the
  floor selling out or blind-pushing these cells, the seat wants a lia table
  gated by band-consistent values, not the old ladder back. **Outcome: partly**
  — the floor sits on `2♥ (2♠)` at −1.1 / −1.5 per fired, and the doubles at
  both-vul are PD-heavy; the band, not the seat, is the loss.
* **The pre-launch reading gate PASSED (2026-09-02)** — with the correct
  invocation. `probe-call-reading --their-2c-landy --ns-landy-lia` reads every
  re-cut call soundly and tightly: `2♥` 4-9 points, ♣4+, ♦4-5 (the strong
  band's 8-9 cap is genuine rung-order inference — 10+ both-minors hands are
  shape-forced into `X`/`3NT`/splinter first), `2NT` 8+ with ♦6+ ♣≤5, `X` 8-9
  with the 2+2+ major terms (the refinement's owed reading-union item, present
  after all), sign-offs 0-7 on their six-card suits, escape 5-9 on ♦5-6. The
  review's first probe omitted `--their-2c-landy`, under which
  `--ns-landy-lia` is **inert** — `lebensohl_package` registers nothing and
  `1NT (2♣) …` resolves through the systems-on rebase to the constructive
  tables, whose "♦2-2 on `2NT - 3♣`" is that lane's own sound
  diamond-transfer-break description, bidder and reading agreeing via the same
  `Fallback::Rebase`. Half a day chased that as a phantom before the
  cross-build comparison surfaced it. The example now warns on the inert
  combination and its flag doc carries the "needs `--their-2c-landy`" sentence
  its doubler-rebids sibling always had; the ~dozen other `--ns-landy-*`
  modifier flags share the trap and are **flagged, not fixed** — a warn or an
  implied disclosure per flag is a one-line decision each, owed to the flag
  audit, not this package. The A/B harness derives the disclosure from the
  opponent's card, so no measurement was ever affected.
* **Opener's `3NT`@160 accept over `2♥` can now fire opposite the weak band.**
  The accept was authored when the takeout promised 8+, so a maximum meant 24+
  combined; with the 4-7 band under the same call it can be 20. `2♥ - 3NT -`
  is an authored sit, so responder cannot pull it. This is a defect the weak
  band introduced and it is deliberately left standing rather than patched
  blind — the alternative is to weight the accept *below* opener's minor
  picks, which protects the weak hand at the cost of missing the 8-9 × maximum
  game, and choosing between two unmeasured tables on analysis alone is what
  the iron rule forbids. Both are one weight. **Falsifier 2 of the next arm**:
  bucket `2♥ → 3NT` boards by responder's own strength; if the 4-7 half is
  where the takeout's loss is, the accept moves down. **Outcome: fires** —
  the 0-7 band is −14,515 / −38,483 plain against +6,198 / +2,105 for 8-9;
  the accept moves below the minor picks in any lia4.

* **The `3♣` sign-off's contested tail is now the floor's on inference, not on
  measurement.** The convicted cell was the `2♦` escape's tail; the `3♣` one is
  deleted by the same argument but was never priced on its own, and the census
  says the floor pushes `4♣` on 96% of `3♣ (3♠)` boards. Reversible: restore
  the `{3m} ({over})` loop.
* **Six thin diamonds under the escape's five-HCP floor now pass.** `3♦`'s
  re-gate to excessive length reopens part of the old "starved diamonds" seam
  for the 6♦ 0-4 HCP hand — deliberate, since that band measured −1.97/fired on
  the rung it is losing, but it is a pass where the baseline transfers.
* **`landy_recue_answer`'s `4m`@20 still has no answer node** — unchanged,
  still shared with the base arm, still owed its own A/B.
* **`park/landy-pdi` owes a rebase** for the two `.penalty()` rows, unchanged
  by the refinement.
* **`alert-sites.txt` does not move.** `comp:landy-super` is a new slug and
  `comp:landy-length` retires, but the fixture's four sections are all built at
  `defense_2c_landy_lia = false`. Both slugs' disclosure records are in
  `card.rs`, so a ship needs no new decision.
* **Byte-identity**: `smoke-default --count 20000 --seed 1` byte-identical
  against `main` HEAD, verified twice.

#### Package C — `landy_texas`: **shipped default-on 2026-08-31**, an eight-of-eight sweep

Seed 1788181796, control `8dca085a`, 4,608,000 boards/arm/vul, both isolation
gates **0 foreign**. Every cell positive:

| vul | fired | plain | PD | sd-plain | sd-PD |
| --- | --- | --- | --- | --- | --- |
| none | 1280 (0.03%) | **+0.616**/fired (+788) | **+0.816** (+1045) | +0.220 (+281) | +0.305 (+390) |
| both | 931 (0.02%) | **+0.711**/fired (+662) | **+0.996** (+927) | +0.352 (+328) | +0.506 (+471) |

Per board that is +0.0002/+0.0001, because the rung is rare; the per-fired
plain cells are >6.7σ NV / >5.6σ BV off the printed per-board CI bound. Ships
default-on under the decision table's plain-DD-win row, not the non-loss row
the design asked for.

##### The mechanism is the reverse of the one designed for

The package was built expecting a **plain-DD wash** — right-siding invisible
to the harness — carried by the **DD-visible slam reroute**. Both halves came
back inverted, and the correction is worth more than the IMPs:

* **The slam reroute is the invisible half.** Level 5+ is reached on **5 of
  2211** divergent boards, and **0** at both-vul. The 16+ drive is authored and
  correct, but at this frequency it contributed nothing measurable. Falsifier 1
  (the drive overbids) is moot rather than refuted.
* **Right-siding is what the harness priced.** 96.4% NV / 96.3% BV of divergent
  boards are the **same contract played from the other seat** — every one of
  them a declarer-seat flip, none a change of declaring side. That bucket is
  the whole win.

The general lesson, which corrects a load-bearing assumption in the iron rules:
**double dummy is blind to right-siding's *concealment*, not to its *lead
direction*.** The solver prices a specific (contract, declarer) pair, and
North-declaring `4♥` puts East on lead where South-declaring puts West. Those
are different hands, so the trick count honestly differs — leading *through*
the 1NT opener's tenaces versus *into* them is a double-dummy fact. What needs
single dummy is the extra edge from the leader not being able to see the closed
strong hand. sd-lead confirms the split from the other side: with a blind
leader the killing lead is found less often **in both arms**, so the gap
narrows to about a third (+0.220/+0.352) — still positive at both colours,
and the more realistic number of the two.

**Adopted 2026-09-01** (jdh8: "the iron rule is good, it is just the minor claim
that DD is blind to right-siding that needs refinement"). CLAUDE.md's iron rule
and [measurement.md](measurement.md)'s rule list now read *right-siding is
half-visible — DD prices the lead direction, not the concealment, so it sees
the idea iff declarer actually moves*; the "measuring zero is real" clause
survives, narrowed to the no-flip case that §N1o's notrump endings exhibit.
Three live items that deferred on the old premise were requeued: N3-xfer's
`(3♣)` transfers (above), the `(3♣)` transfer design note, and the
splinter-vs-fragment question in
[bba-1nt-splinter.md](ai-bidder/bba-1nt-splinter.md), whose whole argument is
about which defender leads and through what — the half the solver can see.
Historical ledger rows that read "DD-blind right-siding" stand as records of
what was concluded at the time.

Falsifier 2 (the alerted transfer tells the defense the anchor major before the
lead) is **refuted** by the same rows: sd-lead is the scorer that could see an
information leak, and it is positive at both vulnerabilities under both
scorers. Falsifier 3 needed no relaxation — the sit rail costs nothing
visible.

**Residual cost, for anyone who revisits this rung.** The transfer hands the
opponents room on 4.1% NV / 4.0% BV of fired boards (52 / 37 boards, and zero
the other way), and draws 33 / 30 doubles the direct jam never drew — those
boards are the entire worst-5 list in both reports (`1NT 2♣ 4♣ - 4♥ - - 4♠ …`,
where the direct `4♥` had shut the auction). The right-siding gain outweighs it
about 5:1, so this is a refinement target, not a defect: a gate that keeps the
direct jam when the transfer's extra round is likeliest to be used against us.

PD exceeding plain here is **not** the usual doubling artifact and does not
need discounting: perfect defense prices a failing contract as doubled either
way, so its uplift comes from the baseline's wrong-sided contract failing
*more* under a defense that always finds the killing lead. Same direction as
plain, larger magnitude, coherent mechanism.

##### As designed (build record)

The `4♣`/`4♦` seat was floor-owned before this shipped, so the jam declared
from the
wrong side and a 16+ hand cannot look for slam. On, the jam rides South
African Texas — `4♦`@170 → ♠ / `4♣`@169 → ♥ (`4♦` outranks `4♣` as `4♠`@172
outranks `4♥`@171 today: 6-6 wants spades), the uncontested `texas` alert
slug reused (now `pub`), the jam's gate carried verbatim
(`points(landy_texas_floor..) & len(6..)`; the floor knob exists for a later
sweep and is **not** swept here — this package changes only which call
carries the hand). The freed direct `4♥`/`4♠` are the uncontested NF
slam-try tier verbatim (`len 6+ & other ≤4 & hcp(15..=direct_4m_max)`,
`slam_try_answer` + RKCB above); a 16+ hand transfers and drives its own
`4NT` (`texas_slam_drive_rebid` + RKCB above the completion). Completions
answer the `-` and `(X)` tails plus the systems-on rebase; deeper overcall
tails stay the floor's, as the minor transfers' do. No `4♣` collision: the
direct `4♣` and the `4m` slam try sit at different prefixes.

**One deviation from the uncontested twin, deliberate:** the drive seat
carries a `Pass`@0 sit rail. Uncontested that seat belongs to `instinct()`,
which never pulls a completed transfer; this lane's floor is the **learned**
one §N1o caught cue-bidding a dead four-level to `6♥` doubled — the same
measured defect the jam's sit node insures against. Expected verdict shape:
**a plain-DD wash is real, not an artifact** (right-siding is invisible to
the harness by construction); the DD-visible half is the slam reroute. Ships
on a non-loss.

#### Package D — `landy_notrump_no_major` re-measured on A's winner

§N1p's `nt` loss was measured with the broken catch-all in place — its own
falsifier 2 says the doubler's rebid was the floor's, pulling a values double
to `3NT` 49.5% of the time on two trumps, so that arm partly measured the
floor. Package A deleted the shadowing catch-all and authored the three-card
cells, which is the repair that unblocks this re-measure. Control is
post-C `main`, fresh seed.

**New runner, and a deliberate departure from the plan's wording.** The plan
said "the `nt` pair only" of `scripts/ab-landy-notrump-shape.sh`, whose arms
hold `landy_major_jam` **off** on both sides — the framing §N1p measured. Two
things have changed since, so package D runs
`scripts/ab-landy-nt-remeasure.sh` (`base` = bare `main`, `nt` = `+
--ns-landy-notrump-no-major`) instead, and §N1p's runner keeps its own
experiment's record untouched:

* The jam has been default-on since 2026-08-30 and C's Texas since
  2026-08-31, so a strong six-card major now leaves via `4♣`/`4♦`. It never
  reaches `3NT`@168, and `nt` cannot move it either way. Holding the jam off
  would measure a pool `main` no longer routes that way — against the iron rule
  that a ship decision is measured against the real routing.
* The moving pool is therefore the **four- and five-card** major game hands
  only. Narrower than §N1p's, and the whole ship-relevant question.

C is inert in the comparison either way: `landy_texas` is gated on
`landy_major_jam`, and with the jam on it is on identically in both arms.

##### Verdict — **measured non-win 2026-09-01, `landy_notrump_no_major` stays default off**

`scripts/ab-landy-nt-remeasure.sh`, `SEED_BASE=1788191041`, control
`60115871`, 4,608,000 bd/arm/vul. Both isolation gates **0 foreign** (of
54,309 / 36,591 divergent) before any headline.

| column | not vulnerable | both vulnerable |
| --- | --- | --- |
| fired | 54,309 (1.18%) | 36,591 (0.79%) |
| plain DD | **+0.0077** ±0.0007 (+0.652/fired) | **+0.0075** ±0.0008 (+0.943/fired) |
| PD | +0.0145 ±0.0008 (+1.228/fired) | +0.0146 ±0.0008 (+1.835/fired) |
| plain SD (sd-lead) | **−0.0061** ±0.0007 (−0.459/fired) | **−0.0056** ±0.0008 (−0.634/fired) |
| SD-PD | −0.0011 ±0.0008 (−0.084/fired) | +0.0000 ±0.0008 (+0.004/fired) |

(The two sd rows print their own denominator, 61,454 / 40,521 fired, against
the DD rows' 54,309 / 36,591 — `ab-dump-sd` and `probe-divergence` count a
moved board differently. Compare the **IMPs/board** columns, which share one
denominator; the per-fired figures do not.)

**Why this is not a ship, despite two CI-clear positive columns.** Read the
scorers by what each one's synthetic double does to *this* knob's mechanism —
the candidate stops bidding `3NT` and doubles instead, so the baseline's game
is the contract a synthetic `X` lands on:

* **PD is the decision table's `loss | win` row in its literal form** — "it
  credits phantom doubles of contracts we no longer bid". Every failing
  baseline `3NT` is doubled for free; the candidate, which no longer bids
  them, banks the difference. §N1p dismissed the same +0.018/+0.020 as the
  auto-double artifact and this is the same artifact on the same lane.
  Falsifier 1 pre-registered it. Discount the column.
* **The two columns with no synthetic double straddle zero**: plain DD
  +0.0077/+0.0075, plain SD −0.0061/−0.0056. They differ *only* in the lead
  model, and the knob's whole measured effect is smaller than the gap between
  them at both colours.
* **SD-PD — the column §N1p itself named "the arbiter" — carries the artifact
  in the candidate's favour and still cannot clear zero** (−0.0011, +0.0000).
  With a thumb on its scale it reads a wash.

So there is no scorer-independent win, and A's repair did not reverse §N1p:
on the arbiter column the two experiments read **−0.0012** (§N1p) and
**−0.0011 / +0.0000** (D). Falsifier 3 is answered — the repair *did* reach,
and what it bought is visible, but it bought it in the undoubled scorers only:

| | plain DD | plain SD | PD | SD-PD |
| --- | --- | --- | --- | --- |
| §N1p `nt` (NV) | −0.0124 | −0.0266 | +0.0181 | −0.0012 |
| D `nt` (NV) | +0.0077 | −0.0061 | +0.0145 | −0.0011 |
| move | **+0.0201** | **+0.0205** | −0.0036 | +0.0001 |

(A between-experiment comparison across a routing change — §N1p held the jam
off — so suggestive, not a measurement, per the series-break caution in
[measurement.md](measurement.md).) The **lead-model seam is bigger than the
knob**: plain DD reads this knob **+0.0142** above plain SD at §N1p NV,
**+0.0138** at D NV and **+0.0131** at D BV — stable across two experiments,
two routings and two vulnerabilities, and roughly double the plain-DD effect
it is being asked to adjudicate. The sign is the tell: the candidate's whole
mechanism is *declaring `3NT` less often*, and the scorer that hands the
defence a clairvoyant opening lead is the one that likes it. This is the
documented 1NT-end blind-lead seam ("DD pessimistic
for declarer; sd-lead corrects it"), and the knob's plain-DD sign is decided
by the lead model rather than by the treatment.

##### Falsifier 2's split — the gate is a bundled disjunction with opposite-signed halves

`probe-divergence --imps --jsonl`, both vulnerabilities, bucketed by the
**mover's own major lengths** (the knob reads exactly that: `len(♥, ..=3) &
len(♠, ..=3)` on the bidder of `3NT`). `off=3NT` is 74.3%/79.9% of the
divergence and `3NT → X` is 98.9%/99.0% of that bucket, so the substitution
is clean. Inside it, plain-DD IMPs per fired board by the mover's major shape,
short-long:

| majors | n (NV) | plain NV | n (BV) | plain BV |
| --- | --- | --- | --- | --- |
| 4-4 | 1,754 | **+4.743** | 1,288 | **+5.902** |
| 4-5 | 532 | **+3.474** | 393 | **+4.947** |
| 3-4 | 10,258 | **+3.534** | 7,228 | **+4.736** |
| 3-5 | 2,074 | **+3.108** | 1,529 | **+4.504** |
| 2-4 | 12,470 | −0.125 | 9,012 | +0.307 |
| 2-5 | 4,006 | −0.199 | 2,948 | −0.271 |
| 1-4 | 4,734 | **−2.079** | 3,447 | **−1.864** |
| 1-5 | 3,609 | **−1.842** | 2,736 | **−2.304** |

Monotone in the **short** major across all eight cells, replicating
independently at both vulnerabilities, with the sign break between two and
three. The bridge reason is plain: they showed both majors, so a mover holding
3+ in each has them in a genuine misfit with nowhere to run and defending is
right, while a mover with a singleton major has handed them a big fit — the
double gets run out to something making, and `3NT` (where the short major is
partner's problem, not ours) was fine. Summed: the `min major ≥ 3` half is
**+52,861 NV / +50,663 BV** plain IMPs on 14,618 / 10,438 boards, the
`≤ 2` half **−18,838 / −10,758** on 24,819 / 18,143. The bundle nets the
difference, which is why it reads as a small win on the column that likes it.

This is [measurement.md](measurement.md)'s **disjunctive-gate rule** — "when a
candidate gate is a disjunction and any slice gives its disjuncts different
signs, they are two arms, always" — and the bundle is currently burying a
+3.1-to-+5.9-per-fired cell inside a losing one. The split variable is a hand
feature the bidder knows before choosing, so it is a legal gate, not a
post-hoc outcome slice.

**Owed, not done: the narrowed arm.** Today's gate bars `3NT` whenever either
major is 4+. The gate the split asks for bars it only when *both* majors are
3+ **and** one of them is 4+ — the misfit hands — so `3NT` survives on every
hand with a doubleton or singleton major. On plain DD it would move
+0.0115 / +0.0110 IMPs/board against the bundle's +0.0077 / +0.0075, on
roughly a third of the traffic. Whether it clears the lead-model seam is an
A/B and not an extrapolation — though the direction is favourable, since the
seam lives on hands whose `3NT` stands or falls on the opening lead, and those
are exactly the singleton-major hands the narrowing stops moving. Until that
arm runs, `landy_notrump_no_major` stays **default off** and §N1p's flag 2
stays settled.

**Residual worth a line for whoever takes the narrowed arm.** The bucket where
the double did *not* end in defending — `both NS`, 15.4%/14.1% of divergence —
costs **−1.710 / −2.657 per fired**: opener pulls, and we declare something
worse than the `3NT` we gave up. That is a continuation defect on an authored
seat (A's cells), so it is the book's to fix, and it is inside the narrowed
arm's traffic too.

##### Flagged, not fixed: the sd columns disclose no Landy knob

`ab-dump-sd` has `--on-ns-*` disclosure flags for the free-bid, negative-double
and overcall families, and **none for any `landy_*` knob** (`--help | grep -i
landy` is empty). So the blind leader reads both arms with control semantics:
under the ON arm the responder's `X` is wider and its `3NT` narrower, and the
leader is not told. Every sd row in §N1-lia — package A's tie-breaks, C's
corroboration and D's arbiter column — carries that caveat. It does not
invalidate them (the seam's stability across §N1p, which ran under the same
condition, argues it is a lead-model effect rather than a disclosure one), but
it bounds them. Proposed reversible default: **leave the harness alone and
quote the caveat**, since adding flags is a harness change that would
re-baseline the sd column mid-campaign; add `--on-ns-landy-*` when the
narrowed arm above runs, so its sd row is measured with the knob disclosed.

#### The arms

Sequential, fresh `SEED_BASE` per package, 4,608,000 bd/arm/vul at the §N1p
scale, `probe-divergence --gate-opener ours` **0 foreign** before any
headline; every verdict bounded by flagged item 5 (`--filter-landy` admits
strictly balanced openers only). Runners with per-arm falsifiers in their
headers: `scripts/ab-landy-lia-doubler.sh` (A: `base | nocatch | hon | cells`,
adjacent pairs isolate each cell; penalty rungs arbitrated on **plain DD**,
PD double-blind), `scripts/ab-landy-lia.sh` → `ab-landy-lia-repair.sh` →
**`ab-landy-lia2.sh`** (B, three runners: the first two carry VERDICT blocks
for builds the probe correction superseded, the third is the live and
unlaunched one), `scripts/ab-landy-texas.sh`
(C). Renders: `--ns-landy-responder lia` and `--ns-landy-texas` in
`render-book`; every new knob has a `bba-gen`/`probe-call-reading` flag.
Invariants: four new profiles in `gated_profiles_preserve_alert_invariant`
(`their-landy-lia`, `their-landy-texas`, `their-landy-lia-texas`,
`their-landy-doubler-cells`); `cards/*.bbsa` unchanged throughout (the
`comp:landy-ask`/`comp:landy-super`/`comp:landy-minor` no-schema-name records
are in `card.rs`'s precedent block, written at build time (`comp:landy-length`
retired with the by-length answer in the lia2 refinement) — and BBA's schema has
no row for Texas over *their* Landy either, only for our own).
`tests/fixtures/alert-sites.txt` was unchanged while every knob was default-off
and moved once on C's ship, in its `[their-landy]` section only: `texas
80 -> 88` (the eight transfer sites), `completion 696 -> 692`, `rkcb
18404 -> 19232` (the drive's keycard ladder above the completion), with
`[kokish-kraft]` following its anchor. **The default profile did not move** —
the fixture is the standing proof that the knob is inert while their `2♣` is
undeclared or natural.

### Flagged, not fixed (§N1 — reversible defaults proposed)

1. **BBA's opener does not sit at `1NT (2♣) X (2M)` the way it does in the
   Multi lane.** New probes `opener-c-x2h`/`opener-c-x2s` (2026-08-28,
   100,000 hands/vul filtered to the 5,005 BBA opens 1NT with) read **67.3%
   / 67.6% Pass non-vulnerable and 79.3% / 80.5% vulnerable**, with a natural
   `3♣` on four-plus clubs taking the rest; the Multi twins on the same
   sample size pass 91.7–93.0%. `epbot_get_info_meaning` says why: BBA labels
   our `X` **“bidable suit”** over Landy and **“negative double”** over the
   Multi — it has no values double in this lane at all. Full table in
   [the counter-defense research](ai-bidder/landy-2c-counter-defense-research.md).
   What survives for §N1l: BBA never *doubles* at that seat in either lane (no
   `X` bucket over 0.5% in eight cells), which is the evidence the scope call
   rests on. What does not: “opener sits, as in the Multi lane.” Proposed
   reversible default: **leave opener's seat to the floor** — §N1k authored it
   and lost — and re-open it only as its own arm after §N1l's verdict.
   **Acted on 2026-08-29**: that arm is
   [§N1m](#n1m--openers-own-rebid-over-their-advance-landy_opener_px--landy_opener_rungs-built-2026-08-29-ab-owed),
   built off the oracle and default off pending its A/B. The evidence the scope
   call rested on survives unchanged — BBA never doubles at that seat in either
   lane — and the oracle now says that is BBA leaving +2.8…+8.1 IMPs/board on
   the table whenever it holds four of the major it advanced.
2. **`landy_bba_responder`'s `3NT`@168 is ungated on high cards**
   (`points(10..)` alone, no stopper test), which is what caps the values
   double at nine points and kills §N1l's top two rungs in self-play. Proposed
   reversible default: **leave it**; re-gating moves every shape in the lane,
   not just this table's traffic, so it is a wider arm and a separate decision.
   **Acted on 2026-08-29** (jdh8): that arm is
   [§N1p](#n1p--an-unlimited-values-double-landy_notrump_no_major-loss-stays-off-landy_major_jam-shipped-default-on-2026-08-30),
   and it is narrower than the flag feared — the gate is `len(♥, ..=3) &
   len(♠, ..=3)` on the two `3NT` rungs alone, so the stoppers, the transfers
   and the two-suited family keep their orderings and only the four-plus-major
   game hands move. **Measured 2026-08-30 and lost at both colours** — so the
   flag's original proposed default was right, for the reason it gave in
   reverse: re-gating `3NT` moved only this table's traffic, and that traffic
   was better off declaring. The flag is now **settled, not merely proposed**.
   **Re-measured 2026-09-01** as §N1-lia package D, on the seat package A
   repaired, and it stays settled — but the split says *why* more precisely
   than "better off declaring": on plain DD only the traffic with a **short**
   major was, and the whole plain-DD verdict sits inside the lead-model seam.
   The gate bundles that half with a `min major ≥ 3` half worth +3.1…+5.9
   IMPs/fired, and the narrowed gate is this flag's one live descendant.
3. **The `X (2NT)` leg is unauthored on purpose** — opener's `X (2M)` seat was
   too, until §N1m re-opened it; the rest of this item stands.
   After the strong advance the overcaller jumps to `4M` 54.3% of the time and
   to slam another 13.5%; nothing invitational survives. `park/landy-kk` also
   registers a total-pass node at `X (2♦) - (2NT)`; it is **not** salvaged here,
   because a total pass shadows the floor at a seat this plan never scoped.
   Proposed reversible default: **leave the two remaining seats to the
   floor.**
4. **`artificial_calls_are_alerted` cannot cover `LANDY_PENALTY`.**
   `unalerted_artificial` skips `Double`/`Redouble` rules reached through
   row-package fallbacks by design — the node key cannot witness which strain a
   suffix-guarded double doubles. Same hole covers `MULTI_PENALTY` one lane
   over. The guard here is instead an explicit `ReadingScope::Alerted` arm in
   `landy_doubler_rebid_alerts_publish_the_trump_length`, which fails if the
   alert is dropped. Proposed reversible default: **leave the invariant's
   exemption alone**, and keep per-call alerted-scope assertions for penalty
   doubles. §N1m's opener-seat double shares the slug and is guarded the same
   way, in the same test.
5. **`--filter-landy` admits only strictly balanced 1NT openers.**
   `is_1nt_opener` (`examples/bba-gen/main.rs`) requires no singleton/void and
   at most one doubleton, but our shipped opening is
   `NotrumpShape::Wide6322`, which also opens 5m(422) and 6m(322) — both
   two-doubleton shapes. Measured consequence in §N1m's probe: **zero**
   six-card-minor openers in 103,653 seat boards, and every five-card minor a
   5(332). Every §N1 verdict measured under this filter (and §N3's, via
   `--filter-preempt`'s identical gate) is therefore blind to the wide-shape
   slice. It does not invalidate an A/B — both arms share the filter and the
   headline is IMPs per *accepted* board, which the flag's own rustdoc already
   says — but it does mean a rung gated on a long minor cannot be evaluated
   here at all. Proposed reversible default: **leave the filter alone**
   (widening it changes the accepted set, so every arm under it would have to
   be re-generated and no old pair would stay comparable), and instead **state
   the blind spot wherever a shape-gated rung is priced**. A
   `--filter-landy-wide` sibling is the clean fix when a lane actually needs
   the slice.

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
right-siding — believed DD-blind when this was written, but the completion
moves declarer, so under the refined rule the plain scorer credits its
lead-direction half (see the §N1-lia package C verdict below).
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

**Score against the lane on the 2026-08-21 `1e9a47e2` arms** — this is
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

- `1NT (4♥) X - 5♦ - 5♥ (X) - - -` — responder's third call is a five-level cue
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

> The lane's **tree map** — every authored node, who owns each seat, what each
> call reads as, and their pass-or-correct ladder — lives in
> [one-notrump-multi.md](one-notrump-multi.md), regenerable from the book.
> This section keeps the verdicts.

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
| `X` | `DoubleStyle::Optional`: `len(♦, 2..=3) & hcp(8..)`, opener cooperates (pass, or run to a 5-card suit with ≤2♦) | **`hcp(6..)`** (v7; v1–v5 `hcp(8..)` at 143), alerted `comp:multi-values`, weight 130 — below `3NT` 150, `3♠`→♣ 145, the natural `2M` 140 and the relay 135, so a weak 5+ suit still escapes or relays | BBA's own values double (`hcp 5–17`, 41% of its hands, median 9), no diamond claim — the *waiting* call; they name the major, we act on it (`multi_responder_rebid`). Read: **`points 8..`, ♥ ≤4, ♠ ≤4** — the table used to claim `points 6..` and every suit ⊤, stale on both halves. The 8 is `responder_overcall_double_reading`'s hard-coded `DoubleStyle` floor, two points above this rule's own `hcp(6..)`; `reading.their_multi_double_reading` (§N4f) is the opt-in repair. The major caps are sound: by weight ordering a five-card major always escapes or transfers instead |
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
| responder after `X (2♥) - -`, `X (2♥) - (2♠)`, `X (2♠) - -`, `X (2♥) X (2♠)` | `multi_responder_rebid(M, ran)` on the *resolved* major — **v7 (then default)**: `4NT` = `hcp(16..)`; `2♠` = five spades `hcp ≤8` (hearts resolved); `X` = **takeout**, four of the other major and ≤2 of theirs (`comp:multi-takeout`) — in the `ran` shape (`X (2♥) - (2♠)`, `X (2♥) X (2♠)`) four spades and 7+ (`comp:multi-penalty`); `3NT` = `points(10..) & stopper_in(M)`; else pass. (v4: `3NT` stopper / `X` = `len(M, 4..)` penalty / pass.) |
| responder after `X (2M) X -` | sit |
| opener after responder's takeout double (`X (2M) - - X -`) | `multi_takeout_answer(M)`: sit with four of theirs, bid the 4-4 fit (`2♠`/`3♥`), else a four-card minor, else `2NT` |
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
The `X (2♥) - - -` sell-out itself is now plain −2.05 / PD **+2.02** NV
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
bucket. This is the **pre-N4e** re-decomposition of what was left, read with
`probe-1nt-interference --bucket "2♦" --responses 6` on the shipped arms
`ab-results/anchor-confirm/2026-08-21-1e9a47e2/american-{none,both}` (seed
1787064872, 204,800 bd/vulnerability, NV+vul pooled below).

**Read the pre-ship size first.** −744 IMPs over 409,600 boards is **−0.0018
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
| `X (2♥) - - -` sell-out | 48 | −1.94 | **+1.02** | −93 | +49 |
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

#### Post-ship re-anchor (run 2026-08-23 local) — the loop is closed

`ab-results/anchor-confirm/2026-08-22-053c4fb8/american-{none,both}` replays
the same seed, 1787064872, at 204,800 bd/vulnerability with 100.00% replay and
0 mismatches. The `(2♦)` boards are exactly paired with the pre-ship snapshot,
and none overlaps `d54ef73f`'s overcall-strip fix. Cells below are
`boards / plain total / PD total`:

| slice | pre-ship `1e9a47e2` | post-ship `053c4fb8` | delta |
| --- | ---: | ---: | ---: |
| whole `(2♦)` bucket | 816 / −744 / −80 | 816 / **−689 / +29** | 0 / **+55 / +109** |
| responder Pass | 264 / −452 / −226 | 213 / **−301 / −4** | −51 / **+151 / +222** |
| Pass: ≤5 HCP, 6+ suit | 37 / −145 / −179 | **0 / 0 / 0** | −37 / **+145 / +179** |
| `X` | 200 / −203 / +133 | 200 / −203 / +133 | 0 / 0 / 0 |
| `2NT` relay | 79 / −6 / +11 | 110 / −51 / −1 | +31 / −45 / −12 |
| `2♥` | 52 / +5 / +77 | 62 / −90 / −42 | +10 / −95 / −119 |
| `2♠` | 57 / −118 / −71 | 67 / −74 / −53 | +10 / +44 / +18 |

The 51 passes migrate exactly: 31 to `2NT`, 10 to `2♥`, and 10 to `2♠`.
Bucket plain improves from −0.91 to −0.84 per board; PD moves from
+0.01/−0.24 to +0.08/−0.02 NV/vul. The 111-board N4e-owned divergent set at
table A is **+55 plain / +132 PD**. The new `2♥` total is the worst first-call
row at −90/−42, but the N4e-owned boards entering it are **+33/+57**.

No named continuation crosses −100: `2♥ - -` is 31 boards at −49/−66, and
`2♥ (2♠) -` is 16 at −35/−10. One relay-under-interference board is −16/−17;
it is an isolated outlier, not a package. **N4e landed: the lane is done for
now.** N4-mirror is the next adjacent proposal; regime-widening and its v7
floor retrain remain the trigger for the parked reading knob.

**N2d stays parked**, and now gets the pointer the queue row promised: `six`
shipped, so the `(2♠)` twin's case is a pointer to this round, not a run.

## N4f — opener's balancing seat and the two reading knobs (**measured ×2 rounds 2026-08-22: nothing ships; all three stay opt-in**)

The `(2♦)` bucket's one *named* hole plus the two reading defects
[one-notrump-multi.md](one-notrump-multi.md) flagged. All three are built, all
three default off, all three inert while their `2♦` is undeclared
(`smoke-default` `39ca60a2…` byte-identical against `main` HEAD). They were
measured twice with `scripts/ab-2d-multi-balance.sh` — three aligned arms, each
pinning the other two off, `--their-2d-multi --filter-1nt` on every arm — and
nothing shipped.

### Phase 0 first: two probes re-ranked the package before any box was spent

**The takeout double the literature prescribes is not what the anchor plays.**
`probe-bba-constraints --mode custom --seat 0 --calls "1NT 2♦ - 2♥"
--filter-call 1NT` (4000 hands/vul, `--min-share 0.005`) at the unauthored seat:

| seat | BBA |
| --- | --- |
| `1NT (2♦) - (2♥) ?` | Pass **94.2%** · `X` **5.8%** = `hcp(15..=17) & len(♥, 5..) & balanced()` |
| `1NT (2♦) - (2♠) ?` | Pass **92.7%** · `X` **7.3%** = `hcp(15..=17) & len(♠, 5..) & balanced()` |

A trump-length **penalty** double of the suit they named, with **no natural
rung at any share** — not the delayed *takeout* double of Multi theory, and not
the `defense_to_weak_two`-derived table the package was originally designed as.

**And opener's seat is not where the bucket's deficit is born.** BBA acts on 6%
of hands there; 6% of 253 bd is ~15 boards, which cannot carry −426 plain at any
plausible per-board value. Combined with N4e having already absorbed the Pass
row's worst class (37 bd, −145/−179), the residual target is ~227 bd at
**−307 plain / −47 PD** — a plain-only deficit. N4f is disaster removal at the
standard gate, not the bucket's cause. Sized here so no later round re-derives it.

**Responder's first call is not the leak either** (`--mode counter`, 8000 hands
NV): BBA's `4♠`/`4♥` band is `hcp(6..=16) & len(M, 6..)` — a **six**-card major
and the *strong* half, which our `3♦`/`3♥` transfer already reaches — so the
long-major jump this round considered was dropped before it was built. The one
seam the probe does confirm is BBA's natural **minor** single-suiter (`3♣` 2.3%
`hcp 5–12`, `3♦` 1.6% `hcp 4–12`, both median six cards, 3.9% together), which
our table has no home for: `3♣` is Stayman and `3♦` is the heart transfer.
Unbuilt — opposite 15–17 the contract is usually `3NT` — but recorded, because
this campaign had previously dismissed the band as rare and it is not.

### The three knobs

| knob | what it authors | pre-registered verdict row |
| --- | --- | --- |
| `competition.multi_balance` | `1NT (2♦) - (2M) ?`: `X` = `len(M, 5..)` (penalty, alert `comp:multi-penalty`), else pass; plus responder's sits quiet and over their runout. `multi_penalty_answer`'s four trumps raised to five — partner *passed* rather than doubling, so opener is short of the values half and only length acts | mechanism is *doubling more* → measurement.md's domain addendum: plain DD is the arbiter, `plain win \| PD wash` ships (v7's row) |
| `reading.their_multi_advance_reading` | their advance as the whole pass-or-correct ladder: suppression widened to `2♥/2♠/3♥/3♠/4♣/4♦/4♥/4♠` via a Multi-only `multi_advance_ladder` (**not** by widening the shared `advancer_artificial` — the Landy reader shares it). Round 1 also carried `♥3+ & ♠3+` on the jump rungs; **refuted and removed** (below) | moves what the floor *believes*, not what it doubles → PD is a real arbiter, needs `wash \| win` or better |
| `reading.their_multi_double_reading` | `1NT (2♦) X` reads its authored `hcp(6..)` instead of the generic `DoubleStyle` 8+ | as above |

Measured readings, before and after (`probe-call-reading --their-2d-multi`):

| auction | base | armed |
| --- | --- | --- |
| `1N (2D) X (3H)`, RHO | **♥ 6..13** | ⊤ on ♥ alone, then `♥ 3..13 & ♠ 3..13` |
| `1N (2D) X (4D)`, RHO | **♦ 3..13** | `♦` ⊤, `♥ 3..13 & ♠ 3..13` |
| `1N (2D) X (2H)`, RHO | ⊤ | unchanged — a two-level preference can be a singleton |
| `1N (2D) X -`, partner | `points 8..` | `points 6..` |

### Two build notes worth keeping

1. **The suppression half had to be gated too.** A first cut gated only the
   positive claim and widened the ladder unconditionally. `smoke-default` still
   hashed identical — the reader is inert without the disclosure — but every
   *anchor* arm's base would have moved silently, and the A/B's switch would
   have looked like it only added a claim. Both halves now ride the knob.
   The trap generalises: byte-identity of the default system does **not**
   witness isolation for a knob whose lane only exists under a disclosure.
2. **No strength claim is published on the ladder.** The census's advancer
   hands run 1–7 HCP, but `Envelope` has no HCP axis and `points` would fold in
   their distribution (4-4-4-1, 3-4-5-1, 4-3-6-0 …). And `(2♠)` would refuse
   one anyway: `bba-multi-2d.md §2` measures it at `hcp 7–18`, median 11 — the
   *strength-showing* catch-all, not the weak rung. Only `(2♥)` is weak.
   `(4♣)` is included in the ladder on the user's call (`4♣`/`4♦` both land in
   either 4M) but has **zero measured occurrences**; that rung is assumption,
   not evidence, and is flagged as such in `one-notrump-multi.md` open item 3.

### Round 1 (2026-08-22) — measured ×2 seeds: nothing ships, and the positive read is refuted with a mechanism

`ab-results/2d-multi-balance/seed-{1,2}`, `SEED_BASE` 1787402545 / 1787403446,
24 shards × 9,600 = **230,400 bd/arm/vul**, `--their-2d-multi --filter-1nt`,
both vulnerabilities. **All twelve pairs passed `probe-divergence
--gate-opener ours` at 0 foreign** — not one board opened by the other side,
so the mirror-read residue that dogged N4e did not recur.

Headline is IMPs per accepted deal; `/fired` is the conditional mean.

| arm | seed / vul | fired | plain | PD |
| --- | --- | ---: | --- | --- |
| `balance` | 1 / none | 12 | −8, −0.667/fired | −13, −1.083/fired |
| `balance` | 1 / both | 7 | +14, +2.000/fired | +15, +2.143/fired |
| `balance` | 2 / none | 11 | −36, −3.273/fired | −47, −4.273/fired |
| `balance` | 2 / both | 10 | +5, +0.500/fired | −8, −0.800/fired |
| **`advance`** | 1 / none | 40 | **−24** | **−3** |
| **`advance`** | 1 / both | 35 | **−51, −1.457/fired** | **−22, −0.629/fired** |
| **`advance`** | 2 / none | 44 | **−74, −1.682/fired** | **−109, −2.477/fired** |
| **`advance`** | 2 / both | 24 | **−120, −5.000/fired** | **−121, −5.042/fired** |
| `xfloor` | 1 / none | 11 | −10 | +6 |
| `xfloor` | 1 / both | 1 | +10 | +10 |
| `xfloor` | 2 / none | 15 | −6 | −14 |
| `xfloor` | 2 / both | 13 | −39 | −25 |

**`balance` — no verdict, and the ceiling explains why.** 7–12 fired per cell,
signs disagreeing by vulnerability *and* by seed, every per-board CI (±0.0002)
swallowing every effect. The firing rate is not a bug: opener needs five cards
in the major they named, ~6% of hands, exactly matching the anchor's own 5.8%/7.3%
— so the seat's whole reach is ~18 boards per 230,400, and **this knob cannot
be measured at this harness's resolution.** It stays opt-in, unresolved rather
than refuted; an isolated sub-lane harness or an order of magnitude more boards
is what it would take.

**`xfloor` — wash.** 1–15 fired, one cell (`1/both`, n=1) contributing a
±10 IMP swing on a single board. Nothing to read. Opt-in.

**`advance` — negative in all eight cells, and the cause is the positive
claim, not the suppression.** Tracing the worst boards (the iron rule) shows
one repeated shape: our side stops competing because it believes a spade
length the advancer does not have.

```text
[-13 IMP]  N:J98.A98.AQ96.AT3  E:A72.KJT7643.K2.6  S:KQT643..J87.9754  W:5.Q52.T543.KQJ82
  on:  1NT 2♦ 2♠ 3♥ - 4♥ - - -                     South (♠6, ♥void) sells out
  off: 1NT 2♦ 2♠ 3♥ - 4♥ 4♠ - - 5♥ - - X - - -     South saves, they push, we double
```

West's `3♥` there holds **one spade**; on the next-worst board it holds two
(`A3.KQT8763.9.QT8`). The claim `♠3+` is simply false, and it is worth −13 IMPs
a board when it talks a six-card suit out of a save.

`probe-bba-constraints --mode custom --seat 3 --calls "1NT 2♦ 2♠"` (6000 hands
NV) confirms it is systematic, and — usefully — splits the two halves in
opposite directions:

| their call | share | ♥ | ♠ | hcp |
| --- | ---: | --- | --- | --- |
| `3♥` | 38.2% | **2–5 (med 3)** | **2–4 (med 3)** | 7–13 |
| `4♦` | 11.9% | 3–5 | 3–6 | 3–14 |

So the *suppression* half is right and the base read is badly wrong — their
`3♥` is `♥ 2–5`, where the natural walk publishes `♥ 6..13`. The *claim* half
is wrong at both rungs' tails. **The claim is removed**; the knob is now
suppression-only and owes a fresh arm.

**The build lesson, which is the transferable part:** a sound change
(suppression, which only ever removes a possibly-false length) was bundled with
an unsound one (a new positive assertion) behind a single knob, so the A/B
could only say "the bundle loses". Splitting them would have cost one more arm
and identified the culprit directly. *Do not bundle a removal with an
assertion.*

### Round 2 (2026-08-22) — the corrected `advance`, on a clean tree: the false read is also **inert**

`sha=07d135f2` (round 1 ran from an uncommitted tree; this is the citable
round), `ab-results/2d-multi-balance-r2/seed-{1,2}`, `SEED_BASE` 1787406494 /
1787407382, same 230,400 bd/arm/vul. **All twelve pairs 0 foreign** again.

The headline is the **divergence count**, not the IMPs:

| arm | round 1 fired (8 cells) | round 2 fired (8 cells) |
| --- | ---: | ---: |
| `advance` | **143** | **6** |
| `balance` | 40 | 27 |
| `xfloor` | 40 | 52 |

Removing the `♥3+ & ♠3+` claim collapsed the `advance` arm's reach by **96%**.
So essentially the *entire* effect of round 1 was the false assertion, and the
suppression half — which fixes a read that is demonstrably wrong (`♥ 6..13`
published where the advancer holds `♥ 2–5`) — changes **1–2 decisions per
230,400 boards**.

That is the round's real finding, and it is worth more than the verdict:
**a false reading in this lane is very nearly inert.** The contested floor is
not leaning on the advancer's suit length after their Multi, so correcting it
buys almost nothing. Anyone tempted to attack this lane through the read side
should price that first.

| arm | seed / vul | fired | plain | PD |
| --- | --- | ---: | --- | --- |
| `advance` | 1 / none | 2 | +9 | +8 |
| `advance` | 1 / both | 1 | +15 | +18 |
| `advance` | 2 / none | 2 | −13 | −13 |
| `advance` | 2 / both | 1 | −17 | −17 |
| `balance` | 1 / none | 12 | −12 | +4 |
| `balance` | 1 / both | 4 | −31 | −29 |
| `balance` | 2 / none | 7 | +8 | +11 |
| `balance` | 2 / both | 4 | +1 | −6 |
| `xfloor` | 1 / none | 14 | −11 | +5 |
| `xfloor` | 1 / both | 9 | +11 | +34 |
| `xfloor` | 2 / none | 19 | −37 | −27 |
| `xfloor` | 2 / both | 10 | −30 | −16 |

**`advance` — no verdict at n=1–2**, and one that cannot be had from this
harness: a single board swings a cell by ±17 IMPs. Sound by construction,
measured harmless, and inert.

**`balance` — confirmed below resolution.** Its signs flip *between rounds* on
the same cells (round 1 seed-1/vul was +14/+15, round 2 is −31/−29), which is
what noise looks like. Pooled over both rounds — eight cells, 1.84 m boards —
it is **−59 plain / −73 PD IMPs, ≈ −0.00003 IMPs/board**, inside every CI.

**`xfloor` — wash**, seed-1 positive and seed-2 negative in both rounds.

### Disposition — all three opt-in; the one judgment call was withdrawn

All three stay **opt-in, default off**, per the house rule for
rejected-but-interesting treatments. Two are ordinary parks; the third is not.
**Nothing here awaits the user** — the flip proposed below was withdrawn on
2026-08-22 and is kept only as a recorded argument:

- `competition.multi_balance` — **unresolved, not refuted.** Its reach is ~18
  boards per 230,400 because the anchor's own action rate is 6%. Resolving it
  needs a sub-lane harness or an order of magnitude more boards, not another
  seed. A single-dummy re-measure is the cheaper candidate.
- `reading.their_multi_double_reading` — wash; ordinary park.
- `reading.their_multi_advance_reading` — **stays off; no default flip
  proposed.** An earlier draft of this section argued for flipping it on
  because the reading it removes is false (their `3♥` is `♥ 2–5, median 3`
  where we publish `♥ 6..13`) at "no measured cost". **That argument was
  withdrawn**, and it is recorded here because it is a tempting one:

  - It is *ship on analysis alone* wearing a correctness argument. Every
    change feels correct from the inside; that is what the gate is for.
  - "No measured cost" was an overstatement. Six fired boards with ±17 IMP
    single-board swings is **unmeasured**, not harmless.
  - "Correct" is narrower than it sounds: the probe reads *BBA's* advancer at
    *one* forced node. It is correct-against-this-opponent-model, not true.
  - Inertness argues the other way. A falsehood no decision consumes is a
    **latent** bug; flipping it on would hand the next floor retrain an
    unmeasured input, since the regime input reads `Agreements`
    ([card-manifold.md](ai-bidder/card-manifold.md)).

  *Disposition — a trigger, not a default:* flip it when something makes the
  read live — a contested-floor retrain, or a package that consumes the
  advancer's length. The fix is built, tested, probed and documented, so that
  day costs one flag.

### The retrain trigger fired (2026-08-23) — PD gain, plain target missed; no ship

The v6 corpus had been extracted with `Agreements::default()`: BBA supplied the
teacher calls, but the feature reader still treated its `2♦` as natural and its
`2♣` as natural clubs.  `scripts/dump-v6-their.sh` regenerated the exact same
20 shards, deal slices, cells and seeds with `vs_bba_agreements` plus the two
parked Multi readers.  Targets and tags stayed byte-identical in a paired 5k
smoke; 226 of 52,711 feature rows changed.  The full twin has 6,768,279 rows,
4,235,171 contested, and the same 176→256→256→38 recipe, seed 1, 300 epochs and
30-column fold as shipped v6.

The literal scalar CE gate missed narrowly: v6 on its old heldout features is
0.300980140, while the twin on the corrected features is 0.301202792, delta
+0.000222656.  The paired 676,829-row 95% CI is
[`−0.001115893`, `+0.001561204`], a wash.  On the 3,900 rows whose features
actually changed, old-v6 CE moved 0.45695→0.69374 under the corrected reading;
the twin recovered it to 0.47271.  That large train/serve recovery justified
the pre-registered IMP screen without seed or epoch fishing, but it did not
turn the CE gate into a win.

`scripts/ab-v6-their-reading.sh` compared shipped v6 against the twin served
with both readers, 204,800 accepted deals per arm/vulnerability × two fresh
seeds (`1787439593`, `1787440309`), both scorers:

| seed / vul | fired | plain DD | PD |
| --- | ---: | ---: | ---: |
| 1 / none | 19,322 | +0.0035 ±0.0078 | +0.0109 ±0.0096 |
| 1 / both | 16,426 | +0.0011 ±0.0093 | **+0.0179 ±0.0112** |
| 2 / none | 19,633 | −0.0007 ±0.0079 | +0.0008 ±0.0097 |
| 2 / both | 16,545 | +0.0079 ±0.0093 | **+0.0192 ±0.0112** |
| pooled / none | 38,955 | +0.0014 ±0.0056 | +0.0059 ±0.0068 |
| pooled / both | 32,971 | +0.0045 ±0.0066 | **+0.0185 ±0.0079** |

These are whole `--filter-1nt` floor-swap headlines, not N4 attribution.  The
plan's `--gate-opener ours` requirement was inapplicable: a contested-floor
retrain is supposed to move defensive auctions too, and the first NV pair had
12,289 of 19,322 divergences on boards they opened.  That is scope, not the
mirror-reading leak a one-package gate detects.  N4 was therefore cut directly
from the paired records: baseline opener ours, opening `1NT`, their immediately
following call `2♦`; every other accepted deal contributes zero.

| pooled target slice | fired | plain DD | PD |
| --- | ---: | ---: | ---: |
| none | 484 | **−181, −0.00044 ±0.00064** | +316, +0.00077 ±0.00090 |
| both | 326 | +170, +0.00042 ±0.00068 | **+529, +0.00129 ±0.00088** |
| both vulnerabilities | 810 | **−11, −0.000013 ±0.000468** | **+845, +0.001031 ±0.000629** |

The plain target is a wash, not the required improvement, and the signs reverse
by vulnerability.  The mechanism is likewise a redistribution rather than a
clean repair: Pass replacing v6's double gains +438 plain / +1,328 PD, but the
unchanged initial `2♥` row loses −311/−312, the `2♠` row −121/−163, and new
`3♠` actions lose −194/−309.  Exact `bba-decompose --multi-counter` replay on
seed 1 matched **4,007,503 of 4,007,503** candidate calls.

**Disposition.**  The BBA-reading twin and its reader stack remain opt-in; v6
stays shipped and both reading knobs stay default-off.  The whole-anchor run is
skipped because the pre-registered N4 plain gate already failed.  The broad
vulnerable PD win is a credible lead for a separately registered general-floor
experiment, not evidence that this retrain saved N4.

### Out of scope, found while pricing this — the mirror lane

The census's **table-B** panel, which no doc had quoted, prices *our* `2♦`
overcall of *their* 1NT: **1184 boards, 2.6× table A's 461**. Isolated
(`B-only`, our 1NT uncontested or absent):

| BBA's response to our `2♦` | bd | plain/bd | **PD/bd** | plain tot | **PD tot** |
| --- | ---: | ---: | ---: | ---: | ---: |
| **Pass** | 165 | +0.212 | **−3.382** | +35 | **−558** |
| — passed out (`P P P`) | 77 | +0.714 | **−2.961** | +55 | −228 |
| — `P 2NT P` (we advance) | 20 | −1.750 | **−5.300** | −35 | −106 |
| — `P 2♥ P` | 29 | −0.345 | **−3.621** | −10 | −105 |
| `X` | 562 | +0.452 | +0.731 | +254 | +411 |
| `3NT` | 49 | +2.980 | +3.061 | +146 | +150 |

Plain positive, PD −3.4 per board: the **inverse** of the doubling artifact — a
contract only double-dummy declarer play rescues, which perfect defense beats.
−558 PD IMPs over 204,800 boards is **−0.0027 IMPs/bd of the arm**, larger than
the *entire* contested-1NT lane's headroom (0.0053 NV / 0.0014 vul), from one
call. That is our **defense to their 1NT** ([defensive-overcalls.md](defensive-overcalls.md),
`nt_defense.rs`), not this campaign, and nothing here touches it — but it is the
biggest single number this round produced and it should be somebody's package.

**Corrected 2026-08-23 by the N4-mirror forensic** — keep the table above as
the record of what was seen, not as the lane's price.  Three things the panel
row could not show:

- It is **one arm**.  The vulnerable arm's `Pass` row is worse (174 bd, −244
  plain / −971 PD), and the two arms are the *same* 204,800 deals at two
  vulnerabilities.
- It is **one post-hoc sub-bucket**, selected by an opponent action we cannot
  see at bid time.  The whole `2♦` lane, both arms, is **2139 bd, +696 plain /
  −397 PD**; the `X` row alone is +450 / +729 and `3NT` is +319 / +328.
- Roughly **half** of the `Pass` row is table-A: on those boards *we* are the
  1NT opener and no 1NT-defense knob can move them (raw points −4,170 plain /
  −39,520 PD at table A against −4,000 / −39,400 at table B).

The lane's real, gateable leak is the **≤7-HCP tail** that `points(8..=14)`
admits through distribution points, plus the unauthored advance.  Full
forensic, candidates and pre-registered decision rule:
[defensive-overcalls.md](defensive-overcalls.md#defense-to-their-1nt--the-1nt-2-mirror-panel-forensic-2026-08-23).

## N4-KK — the Kokish–Kraft counter, a whole-table variant (**SHIPPED DEFAULT-ON 2026-08-25**)

The one bucket that reopened N4 (see the queue note above): not another rung of
the then-default v7 lane but a **different published table** for the same
object, so it was a new variant rather than a new seed.

`docs/ai-bidder/multi-landy-2d-counter-defense-research.md` surveyed the
counter-defenses to a `1NT (2♦ = one unknown 6+ major)` overcall and found no
consensus — six credible families, differing on the most basic question (is the
immediate `X` values, a major, or a transfer?) and on whether a *later* double
is takeout or penalty. The **Eric Kokish–Beverly Kraft** notes (January 2008,
printed p. 163) carry a table for exactly this object and are the most complete
exact-object package in the survey. `competition.multi_kokish_kraft`
(default **on** since 2026-08-25; `--no-ns-multi-kokish-kraft` falls back to
the v7 table) plays it.

### What moves — five changes at once, deliberately

This is a whole-table swap, registered *instead of* the v7 subtree for
[`landy_bba_entries`][n1j]'s reason: the two disagree on `2NT`, `3♣`, `3♠` and
both delayed doubles, so an overlay would leave v7's rows shadowing these.

| call | v7 (previous default) | K–K (this arm) |
| --- | --- | --- |
| `X` | values, `hcp 6+` (BBA's own band, the 41% workhorse) | invitational-plus values, `hcp 8+`, **no shape promise**; the 6–7 band takes a *designed* neutral pass |
| `-` | nothing authored past opener's floor seat | a **neutral pass with its own delayed table** once they name the major: takeout `X`, natural `2NT`, competitive `3m` |
| `2NT` | the weak Lebensohl relay to `3♣` | **floorless transfer to clubs** (`len ♣ 6+`, no point floor) |
| `3♣` | game-forcing Stayman, Smolen behind it | **floorless transfer to diamonds** |
| `3♠` | forced `3♠`→♣ game force | **both minors, game-forcing, 5-4 or better** |
| second `X` | takeout, four of the other major (v7's one BBA rung that measured positive on both scorers) | **penalty**, v4's trump-length gate — the takeout double moves to the *pass* branch |
| `4♥`/`4♠` | the floor's | the **uncontested direct slam-try tier** copied under the overcall, RKCB ladder included (`hcp 15..=direct_4m_max`, i.e. exactly 15 under the shipped `texas_slam_drive` — see the residue below) |

Unchanged and shared: `3♦`/`3♥` (INV+ transfers to ♥/♠), the weak `2♥`/`2♠`
escape and its whole interfered tail (`multi_weak_escape` composes), Leaping
Michaels `4♣`/`4♦`, and every answer of the double family
(`multi_pass_answer`, `multi_penalty_answer`, `multi_takeout_answer`,
`multi_quant_answer`). `multi_balance` composes — a different seat.
`multi_stopper_ask` goes **inert**: the `3♠` that carried the ask is the
both-minors call here.

The **delayed-double split** is the one structural idea every exact-object
source in the survey agrees on, and the retired v7 table did not have: after an
initial `X`, a second double is cooperative penalty; after an initial *pass*, it
is takeout. v7 has one takeout double and no pass table at all, so the two arms
differ on both halves at once.

### Two design-sketch repairs, both forced by the same thing

The design sketch specified `3NT` as a bare `points(10..)` — "no stopper
requirement (per source)" — ranked at weight 150, above `3♠` (145) and `X`
(130). A bare `points(10..)` **contains every other constructive gate in the
table**, so whatever sits below it is unreachable. Both repairs are recorded at
[`kokish_kraft_responder`](../src/bidding/american/competition/rubensohl.rs)
and are one-line reversible:

1. **`3♠` now outranks `3NT`** (152 vs 150). The both-minors gate implies
   `points(10..)`, so a higher `3NT` made `3♠` dead code rather than a rare
   rung. The source agrees on the merits — its `3NT` is the last-resort gamble,
   the shape calls come first — and the sketch's stated ordering constraints
   (minor transfers above `3NT`/`3♠`/`2M`/`X`; `2M` above `X`; Leaping Michaels
   above the transfers) say nothing about this pair.
2. **`3NT` keeps its both-majors stopper gate**, unchanged from v4–v7. Measured
   bare, it confines the values double to `points 8..9` —
   `probe-call-reading --their-2d-multi "1N (2D) X -"`
   reads exactly that — which contradicts the *same source's* "`X` =
   invitational **or better**" and re-runs at maximum frequency the stopperless
   blast perfect defense priced at **−3.7/−4.3 a board** in N4 v2/v3. Ranking
   `X` above a bare `3NT` does not rescue it either: the survivors are then
   `hcp ≤ 7` hands with distributional points and no transfer — a 7-count
   4-4-4-1 blasting 3NT — which is worse than dead. **Dropping the gate is a
   recorded sub-arm**, owed its own seed, if K–K's letter is wanted measured.

### Build

- Table: `kokish_kraft_responder` and its ten leaf tables in
  [`rubensohl.rs`](../src/bidding/american/competition/rubensohl.rs);
  registration in `kokish_kraft_entries`
  ([`lebensohl.rs`](../src/bidding/american/competition/lebensohl.rs)), which
  the `for over` loop branches to and `continue`s past — the `landy_bba_entries`
  idiom.
- New alert slugs: `comp:kk-values`, `comp:kk-minor-transfer`,
  `comp:kk-two-suiter`, `comp:kk-minors`. The delayed takeout and the repeated
  penalty double **reuse** `comp:multi-takeout` / `comp:multi-penalty`, whose
  claims are identical one branch over. `[kokish-kraft]` is the new
  `tests/fixtures/alert-sites.txt` delta section; `card.rs` records why no
  `.bbsa` row exists (the whole subtree is keyed on a fact about *their* `2♦`,
  which EPBot's schema cannot name).
- Readings come from `.alert(...)` + projection, no hand-written `Inferences`.
  Probed: `X` reads `points 8..` unbounded, the minor transfers read six cards
  with `points 0..`, the delayed `X` reads the other major 4+ with `≤2` of
  theirs.
- Tests: nine `kk_*` cases in `rubensohl/tests.rs` (every rung of responder's
  table, the retired relay/Stayman, the transfers with their two-suiter rebids
  and both competitive tails, the double split, `3♠`, the `4M` tier,
  composition with the escape and `multi_balance`, and inertness without the
  disclosure) plus a tenth pinning the readings; a `kokish_kraft_*` arm in
  `competition/tests.rs`'s package-invariant sweep; two full-auction
  integration tests in `tests/american_competition.rs` (the lane had none); and
  a `their-multi-kokish-kraft` profile in
  `gated_profiles_preserve_alert_invariant`.
- Byte-identity: inert while their `2♦` is undeclared or natural, so the
  default system is unchanged.

### The measurement

Two runs. The first (SHA `78ad4c02`, `SEED_BASE 1787606986`,
`ab-results/2d-multi-kk/`) read the owned lane as the shippable shape but
**failed the isolation gate** at 55% foreign divergence, so no headline could
be quoted from it; that leak was the mirror-read bug, fixed at `29f93561`
([below](#the-mirror-book--why-the-leak-was-not-a-seat-gate)). Its dumps are
dead — the fix moved the v7 control arm, so they no longer pair.

The re-measure is the verdict: `scripts/ab-2d-multi-kk.sh`, SHA `f2ecb3c6`,
**fresh `SEED_BASE 1787615025`**, `ab-results/2d-multi-kk-gated/`, 230 400
boards per arm per vul, both arms `--their-2d-multi` so the table is the only
difference.

**The gate first.** `probe-divergence --gate-opener ours` reads **0 of 683
divergent boards foreign** at none and **0 of 482** at both — 0 of 1165 against
a 55% prior rate. The mirror book holds.

| vul | plain DD | PD | sd-lead plain | sd-lead PD |
| --- | --- | --- | --- | --- |
| none | +0.0002 ±0.0012 | +0.0012 ±0.0015 | +0.0000 ±0.0013 | +0.0009 ±0.0016 |
| **both** | **+0.0019 ±0.0013** | **+0.0023 ±0.0017** | +0.0014 ±0.0014 | +0.0017 ±0.0017 |

IMPs per board, 95% CIs. Per *fired*: plain +0.067 / +0.907 and PD +0.395 /
+1.102 (none/both).

#### By K–K's first countering call

The same clean paired dumps, replayed with `probe-divergence --imps`, attribute
the K–K-minus-v7 swing to K–K's immediate call after `1NT (2♦)`. `reach` is
every board taking that K–K call; `div` is the subset whose final contract
moved. The pooled columns are total IMPs / IMPs per reached call ± 95% CI,
with the zero-swing boards included. They sum exactly to the headline above:
plain **+46 / +437** and PD **+270 / +531** IMPs (none/both).

| K–K call | reach none/both | div none/both | plain total none/both | PD total none/both | pooled plain/reach | pooled PD/reach |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `-` | 884 / 740 | 253 / 185 | −64 / +216 | +187 / +375 | +0.094 ±0.129 | **+0.346 ±0.200** |
| `X` | 335 / 225 | 182 / 125 | −141 / +43 | −19 / +50 | −0.175 ±0.414 | +0.055 ±0.457 |
| `2♥` | 157 / 123 | 20 / 22 | −38 / −2 | −31 / 0 | −0.143 ±0.359 | −0.111 ±0.383 |
| `2♠` | 135 / 105 | 22 / 17 | −2 / +28 | −12 / +32 | +0.108 ±0.311 | +0.083 ±0.320 |
| `2NT` | 143 / 109 | 55 / 34 | +60 / +23 | +60 / +27 | +0.329 ±0.521 | +0.345 ±0.702 |
| `3♣` | 116 / 88 | 99 / 74 | +114 / +92 | +54 / +34 | **+1.010 ±0.798** | +0.431 ±0.937 |
| `3♦` | 90 / 61 | 0 / 0 | 0 / 0 | 0 / 0 | 0 | 0 |
| `3♥` | 92 / 55 | 0 / 0 | 0 / 0 | 0 / 0 | 0 | 0 |
| `3♠` | 43 / 17 | 39 / 15 | +125 / +45 | +42 / +25 | **+2.833 ±1.795** | +1.117 ±2.079 |
| `3NT` | 38 / 28 | 13 / 10 | −8 / −8 | −11 / −12 | −0.242 ±0.736 | −0.348 ±0.856 |
| `4♣` | 11 / 8 | 0 / 0 | 0 / 0 | 0 / 0 | 0 | 0 |
| `4♦` | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | — | — |
| `4♥` | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | — | — |
| `4♠` | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | — | — |

This is **causal accounting for the whole table swap, not fourteen call
ablations**: a same-call row can move on a later K–K continuation or reading,
and a changed-call row inherits its actual v7→K–K counterfactual. The largest
transitions make that selection visible: `X→3♣` gains +195/+144 plain/PD,
`X→3♠` +170/+67 and `-→-` +184/+488, while `3♣→X` loses −309
plain but gains +26 PD and `X→-` loses −79/−219. One seed makes the small
rows descriptive, not separate ship verdicts. These arms predate the separately
measured `multi_minor_slam_try`, so they isolate K–K rather than today's stacked
minor-slam continuation.

#### Against BBA by first call — ranking, not causality

`probe-1nt-interference --bucket "2♦" --responses 1` on the same K–K arms
prices the whole duplicate board against BBA, grouped by our first response.
That is the direct answer to "which calls gained or lost IMPs?", but it carries
the mirror table and every later call, so it **ranks calls rather than isolating
their EV** (the paired table above is the causal package evidence).

| our call | boards none/both | plain IMPs/bd none/both | PD IMPs/bd none/both | plain total none/both | PD total none/both |
| --- | ---: | ---: | ---: | ---: | ---: |
| `-` | 884 / 740 | −0.603 ±0.229 / +0.053 ±0.339 | +0.615 ±0.350 / **+1.622 ±0.503** | −533 / +39 | +544 / +1200 |
| `X` | 335 / 225 | **−1.663 ±0.571** / −0.231 ±0.911 | −0.096 ±0.652 / **+1.676 ±0.995** | −557 / −52 | −32 / +377 |
| `2♥` | 157 / 123 | −0.096 ±0.814 / +0.431 ±1.150 | +1.159 ±0.991 / +1.780 ±1.422 | −15 / +53 | +182 / +219 |
| `2♠` | 135 / 105 | −0.593 ±0.876 / −0.857 ±1.247 | +0.022 ±1.008 / +0.114 ±1.439 | −80 / −90 | +3 / +12 |
| `2NT` | 143 / 109 | **−1.042 ±0.863** / −0.670 ±1.327 | −0.622 ±1.093 / −0.413 ±1.614 | −149 / −73 | −89 / −45 |
| `3♣` | 116 / 88 | +0.172 ±1.056 / −0.091 ±1.586 | +0.871 ±1.268 / +0.693 ±1.826 | +20 / −8 | +101 / +61 |
| `3♦` | 90 / 61 | **+1.700 ±0.945** / **+1.607 ±1.437** | **+1.744 ±1.064** / **+1.672 ±1.528** | +153 / +98 | +157 / +102 |
| `3♥` | 92 / 55 | +0.641 ±0.973 / +0.455 ±1.141 | +0.663 ±1.081 / +0.582 ±1.350 | +59 / +25 | +61 / +32 |
| `3♠` | 43 / 17 | +0.209 ±1.818 / −1.000 ±3.762 | +0.256 ±2.063 / −0.647 ±4.541 | +9 / −17 | +11 / −11 |
| `3NT` | 38 / 28 | +0.763 ±1.211 / +1.036 ±1.874 | +0.474 ±1.273 / +0.357 ±1.984 | +29 / +29 | +18 / +10 |
| `4♣` | 11 / 8 | −0.727 ±3.238 / +2.750 ±5.699 | −0.727 ±3.238 / +2.750 ±5.699 | −8 / +22 | −8 / +22 |
| `4♦`, `4♥`, `4♠` | 0 / 0 | — | — | — | — |

The stable positive row is the heart transfer `3♦`; the large absolute
losses sit in `X`, pass and `2NT`, with pass and `X` reversing sharply under
perfect defense. Those reversals are exactly why the plain and PD columns stay
separate. The tiny four-level rows have no verdict. As above, this snapshot
isolates K–K before `multi_minor_slam_try`; strong `2NT`/`3♣` continuations
in today's stack are measured in [minor-transfer-slam.md](minor-transfer-slam.md).

#### Inside the two big branches — where `X` and `-` actually bleed (2026-08-26)

The two rows above are the lane's largest absolute movers, so the same arms
were cut one call deeper: `probe-1nt-interference <arm> --dd-cache … --bucket
"2♦" --responses 4`, both vulnerabilities, split by our first call, their
advance, opener's answer and responder's rebid. (The probe now writes its
`--dd-cache` back, so the second cut of an arm costs seconds instead of a
132 774-board DD fan-out.) Board counts are NV+both pooled.

**The `X` branch — the loss is responder having nothing to say.**

| responder's rebid, auction resolved | bd | plain | PD |
| --- | ---: | ---: | ---: |
| **passes** — `X 2♥ P P P`, `X 2♥ X 2♠ P`, `X 2♥ P 2♠ P` | 293 | **−824** | +65 |
| **doubles** — `X 2♥ P P X`, `X 2♥ P 2♠ X` | 44 | **+182** | +191 |
| notrump — `2NT` and `3NT` rungs, all four paths | 96 | −34 | −85 |

`probe-1nt-interference --show … --next "X 2♥ P"` and `--next "X 2♥ X"` say why
the pass-outs bleed, and it is the same shape twice: **five of the sixteen
worst `X (2♥) - -` boards and seven of the twelve worst `X (2♥) X (2♠)` boards
are a 4-4 major fit we never find.** We pass out their resolved partscore for
−110 while BBA, holding our cards, doubles for takeout, hears partner's suit
and makes `4M` — eleven IMPs a board, repeated. That is §N4-KK residue 4, and
it is the branch's largest single hole rather than the rare rung the residue
described. Built 2026-08-26 as `competition.multi_doubler_major`, A/B owed.

**Read the PD column before sizing the repair.** Pooled over the three
pass-out rows PD is +65 — a wash, not a loss — because the `4M` games BBA
reaches on 23-25 points often fail against perfect defense. So this is a
plain-DD repair with a PD non-inferiority requirement, not a both-scorer bet,
and the arm's answer table is deliberately more conservative than BBA's: only a
16-count bids game directly, a 15 invites.

**The `-` branch is a net winner and is not the same problem.** Its four cells
are −533 / +39 plain and +544 / +1200 PD (NV/both) — three of four positive,
**+1250 IMPs net** — so the −0.603 NV-plain cell is one half of a
plain-loss/PD-win pair, the signature the campaign already flags as a doubling
artifact. Inside it the two *authored* action rungs of
[`kokish_kraft_delayed`](../src/bidding/american/competition/rubensohl.rs) are
the negative part and passing is the positive part:

| pass-branch rung | bd | plain | PD |
| --- | ---: | ---: | ---: |
| delayed natural `2NT` (`hcp 7..=9` + stopper, reachable band `hcp == 7`) | 49 | −41 | **−166** |
| delayed takeout `X` | 145 | −96 | −110 |
| responder passes | 987 | −286 | **+1230** |

Nothing was changed there. The `2NT` rung is the sharper candidate — −3.4 PD
per board, with a mechanism (a 7-count bidding notrump at 22-24 combined with
their known six-card major live) that residue 5 already predicted — but at 49
boards it is below this harness's resolution, and any pass-branch arm risks
+1200 both-vul PD IMPs to chase a −533 NV-plain cell. **Recorded, not built.**

**Two more residues the review priced, neither built.**

- **The delayed takeout `X` requires four of the other major; the only two
  sources that qualify the shape both *deny* it** (Gilles–Roupoil "sans 4AM";
  Système Jean Christophe "without a four-card major"). K–K itself prints
  "takeout" unqualified, so the shipped gate does not violate its letter, but
  it is on the opposite side of every source that specifies one. Not recorded
  anywhere before now. Against re-gating: `multi_takeout_answer` answers the
  double by *bidding* the other major on four, so denying four would land the
  pair in a 4-3 by construction — the two halves would have to move together.
- **Opener's penalty double at `X (2♥)`** (`multi_penalty_answer`) is the
  worst-scoring opener action in the branch: `X 2♥ X` reads −227 plain / +74 PD
  over 160 boards against `X 2♥ P`'s −386 / +222 over 353, and its tail
  `X 2♥ X 2♠ P` is the single worst row in the census (−4.19 plain per board
  NV). The review argued a seat mechanism — opener sits under the six-card suit
  and is the finesse victim, responder sits over it — plus anti-selection (the
  overcaller's pass rate collapses from 55% to 10% once opener doubles, so the
  double mostly drives them to their real suit undoubled). **The comparison is
  confounded**: opener holding four of their six-card suit *selects* misfits,
  which would score worse with or without the double. It needs its own arm, not
  this census.

**Verdict — ships default-on.** Both-vul is the decision table's `win | win`
row on the two arbitrating scorers, NV is a clean `wash | wash`, the sd-lead
tie-breaker agrees in sign in all four cells, and **no reading of the eight is
negative**. The first run's owned-lane figures (PD +0.285/+0.772 per fired)
reproduced in the *raw* totals at +0.395/+1.102, which is exactly what the
handoff predicted would happen once the foreign boards stopped being counted.
`smoke-default` stays byte-identical at 20 000 boards / seed 1: the knob is
inert until an opponent's Multi is disclosed, so the default system does not
move.

### Known residues — priced by the A/B, not fixed in the build

Six consequences of the design, each traced with `probe-decision` /
`probe-call-reading` during the build review. None is a bug; all are what the
arm was actually testing, and each names its reversible alternative. Residue 1
was fixed rather than priced; **3, 4 and 6 are the follow-up queue**, each owed
its own seed and its own rung — the A/B above prices the table as built, and
folding a rung into it would spend the one clean signal it bought.

**Triaged by jdh8 2026-08-25.** All three queue items keep their place, but two
of the three named alternatives are withdrawn and residue 3 is no longer this
lane's: it is every minor transfer's, and the campaign for it is
[minor-transfer-slam.md](minor-transfer-slam.md). Each item below carries its
ruling.

1. **The mirror lane widens.** ~~The competitive book is keyed by call
   *shape* with no seat gate on the reader side.~~ **FIXED 2026-08-25 — see
   [the mirror book](#the-mirror-book--why-the-leak-was-not-a-seat-gate) below.**
   As measured, when *they* opened `1NT` and *we* overcalled a natural `2♦`,
   their calls decoded off our `1NT (2♦)` counter-table. The shipped lane leaks
   a *strength* claim there; K–K's floorless transfers leak a hard **six-card
   suit**. This was 55% of the A/B's divergence at −1.6/−2.5 PD per foreign
   board, and it is why `probe-divergence --gate-opener ours` must read
   **0 foreign** before any headline is quoted.
2. **The values double loses its `♥ ≤4 / ♠ ≤4` caps.** Base publishes
   `points 8.. ♥ 0..4 ♠ 0..4`; K–K publishes `points 8.. ♥ 0..13 ♠ 0..13` and
   gains `♣ 0..5 ♦ 0..5` instead. Deleting the `2NT` relay removed the rung the
   projector negated the five-card majors from. The reading is *looser, not
   false*, so nothing phantom is claimed — recorded because it is a disclosure
   change the A/B is measuring alongside the bidding.
3. **A strong long minor never doubles.** The transfers are floorless *and*
   uncapped, so a 21-count with six clubs transfers, opener completes
   unconditionally, and the ladder tops out at `3NT` — no slam channel and no
   access to `kokish_kraft_doubler_rebid`. K–K's own transfers are
   invitational-plus; floorless was the design's deliberate change, and putting
   a ceiling on them (transfer below, `X` above) is the reversible alternative.

   **Generalized 2026-08-25 → [minor-transfer-slam.md](minor-transfer-slam.md).**
   jdh8's ruling: this residue is not N4-KK's. *Every* minor transfer in the
   engine topped out at `3NT` or a placed `5m`, the counter to Landy included,
   and the one slam channel that then existed anywhere — N1j's `4m` on
   `points(13..) & len(minor, 6..)`, whose answer still belonged to the floor —
   was the shape to copy here. That is the campaign-opening state; the Landy
   answer and the other played lanes subsequently shipped in
   [minor-transfer-slam.md](minor-transfer-slam.md). **The ceiling alternative is withdrawn**:
   it pushes the strong long minor into the values double (whose reading is
   already the looser one, residue 2) and runs the measured N1h/N1i right-siding
   trade (`3♣ ← 2NT`, −2.19 PD) backwards. The rung is `4m` at weight 151,
   between the lowest two-suiter step and `3NT`.

   **SHIPPED default-on 2026-08-25 as `competition.multi_minor_slam_try =
   Some(15)`.** A `points` floor, not a bool, so the A/B carried three arms
   (`off` / `13` / `15`; `scripts/ab-2d-multi-slam.sh`). Two rounds, gate 0
   foreign in every cell; round 2 is 2.3M bd/arm/vul (`SEED_BASE` 1787642695).
   Both floors beat `off` on all eight cells; `15` reads `t` +2.25/+2.61 plain
   and +1.92/+1.83 PD (NV/both), `13` +1.61/+1.85 and +1.45/+1.33. The two
   floors are **not** separated — the head-to-head is the 13–14 slice and reads
   `t` +0.70 plain over 55 fired after reading the *other* sign in round 1 — so
   `15` ships on the narrower trigger, not on beating `13`. Opener's answer is
   authored against the N1 doctrine, on a probe: floored, that seat offers
   `{6NT, 4♥, Pass}` and takes `4♥`. Full write-up in
   [minor-transfer-slam.md](minor-transfer-slam.md).
4. **The doubler has no takeout.** v7's second `X` is takeout showing four of
   the other major, and it is the one BBA rung that measured positive on *both*
   scorers (+2.4 plain / +1.6 PD per fired NV). K–K's is penalty, so a 12-count
   with four of the *other* major and a doubleton in theirs now passes their
   partscore. This is the delayed-double split, the arm's single biggest known
   risk, and the first thing to trace if the A/B reads a loss.

   **jdh8 rejected this residue 2026-08-25 — it is to be repaired, not priced.**
   Tracing it sharpens the diagnosis: the missing call is not the takeout double,
   it is the **natural other major**. K–K's `X` is "negative and *Stayman-like*",
   and opener already answers it Stayman-style when the advancer passes
   ([`multi_pass_answer`](../src/bidding/american/competition/rubensohl.rs) shows
   a four-card major). When they *bid* instead,
   [`kokish_kraft_doubler_rebid`](../src/bidding/american/competition/rubensohl.rs)
   offers `4NT` / penalty `X` / `3NT` / `2NT` / Pass and **no natural major at
   all** — so a 12-count with four of the other major and no stopper in theirs
   fails every gate and takes the weight-0 Pass. The source lists only the
   non-obvious meanings of a call, so authoring the natural rebid is
   transcription, not deviation.

   Proposed repair, one rung, one A/B, one seed: over their `(2♥)`, `2♠` on
   `len(♠, 4..) & len(♥, ..=2) & hcp(8..)` — the cheap seat; over their `(2♠)`,
   `3♥` on `len(♥, 4..) & len(♠, ..=2) & points(10..)` — a level dearer, so a
   level stronger. In the two `ran` shapes opener has already doubled `(2♥)` on
   four-plus hearts ([`multi_penalty_answer`]), so `3♥` there lands in a known
   4-4 fit and is the strongest of the four. This **keeps** K–K's delayed-double
   split, which every exact-object source in the survey agrees on. Reverting the
   second `X` to v7's takeout is the alternative and contradicts all of them; it
   is not the recommended default.

   **SHIPPED DEFAULT-ON 2026-08-26** (`competition.multi_doubler_major`,
   `scripts/ab-2d-multi-doubler.sh`, `SEED_BASE=1787740671`, 2.304M bd/arm/vul,
   isolation gate 0 foreign at both vuls). The census below found this is not a
   rare rung but the branch's largest single hole, and it corrected the
   proposal in two places.

   | vul | fired | plain DD | PD | sd plain | sd PD |
   | --- | ---: | ---: | ---: | ---: | ---: |
   | none | 787 | +3.344 | +0.610 | +4.412 | +2.168 |
   | both | 510 | +1.927 | **−1.737** | +3.399 | +0.243 |

   The design claim is confirmed exactly: **100% of the 1 297 divergent boards
   are "bid where the baseline passed", 0% the other way** — weight 100 never
   moved a call the shipped table already made — and 336 of the 787 no-vul
   divergences are games the baseline never reached.

   **Shipped on jdh8's ruling with the both-vul PD cell open.** That cell is
   the decision table's `win | loss` row, and the sd-lead tie-breaker rescues
   it only to a wash. Traced per measurement.md step 10, the cause is opener's
   *answer* table, not the rung: four of the five worst both-vul PD boards are
   `2♠` played in a **4-2 or 4-3**, because
   [`kokish_kraft_doubler_major_answer`](../src/bidding/american/competition/rubensohl.rs)
   offers only `4M`/`3♠`/Pass and a 15-17 balanced hand with a stopper in their
   major and short support must pass. The repair (a `3NT`@135) is built under
   `multi_px_split`, and **unbundled 2026-08-26** as
   `competition.multi_doubler_notrump` so it could be priced against the
   shipped default: `scripts/ab-2d-multi-doubler-nt.sh`,
   `SEED_BASE=1787749549`, **4.608M bd/arm/vul** — double this run's, because
   the seat is a subset of its pass-outs (`hcp 16+`, a stopper, short support)
   and its surface measured ~1 in 43 000 against this rung's 1 in 4 500.

   **That repair SHIPPED DEFAULT-ON 2026-08-27, winning all four cells** —
   NV +2.910 plain / +2.096 PD per fired over 167, both-vul **+4.264 /
   +3.264** over 106, 0 foreign on both gates. The hypothesis holds in
   direction (both-vul is the larger cell) but recovers only ~20% of this
   row's deficit in per-board terms, so **this `win | loss` row stays open**
   and the next suspect is the `3♥` leg's gapless `hcp 16+` game answer —
   [multi-doubler-answer-handoff.md](multi-doubler-answer-handoff.md).

   - **"The two `ran` shapes" is true of one of them.** `multi_penalty_answer`
     doubles their `(2M)` on `len(major, 4..)` at weight 150 against a weight-0
     catch-all, so opener's *pass* over `(2♥)` **denies** four hearts as surely
     as its double **shows** four. `X (2♥) X (2♠)` is the known 4-4 and is
     built; `X (2♥) - (2♠)` is excluded, because `3♥` there finds a 4-3 at
     best.
   - **No shortness conjunct and no separate point floor.** Four of *their*
     major already doubles at weight 155, so the ordering supplies the
     `len(major, ..=2)` cap; and the rung sits at **weight 100**, below every
     existing rung, so it fires on exactly today's pass-outs and cannot move a
     call the shipped table already makes. Residue 4's `points(10..)` on the
     `3♥` leg would have killed four of the seven measured 4-4 heart fits
     (their doublers hold 8–9).
   - **`X (2♠) - -` is withheld pending a ruling.** Opener said nothing about
     hearts there, so `3♥` is a four-card suit at the three level opposite
     unknown support, firing only when the spade stopper is missing — the
     misfits. The census gives that leg 25 boards NV+both worth −30 plain and
     +8 PD: no measured loss to repair. One token in `kokish_kraft_entries`
     re-arms it.

   Opener answers with game in the fit from the top of the range (`hcp 16+`),
   the invitational raise where there is room below game, else a pass;
   responder accepts on `points 11+`.
5. **Two rungs of the delayed table are dead in self-play.** Responder reached
   that seat by passing, and under K–K a weak six-card minor does not pass — it
   transfers. So the source's competitive `3♣`/`3♦` fire only opposite a partner
   who is not bidding this table, and the natural `2NT` beside them is really
   `hcp == 7` rather than the `7..=9` the rule spells. Both are consequences of
   (3); the rungs are kept, documented at
   [`kokish_kraft_delayed`](../src/bidding/american/competition/rubensohl.rs),
   because deleting them would silently hand those seats to the floor.

6. **Responder's contested channel is two calls wide.** Over their
   pass-or-correct above a minor transfer, responder has `3NT` (game values
   with their now-named major stopped) and `X` (`hcp 10+` without one) — a
   census of 60,000 deals during review found that without the `X` about *half*
   of all game-forcing transferors were book-forced to pass out their `3M`, so
   the double is load-bearing, not a nicety. What is still missing is the
   shortness hand: 10+ points with a singleton or void in their major wants to
   play our minor, and doubling with a void is the wrong call. The reversible
   candidate is a `5m` rung gated on `len(major, ..=1)`; it is not in this
   build because five of a minor needs eleven tricks and the A/B should price
   the two-call table first.

   **jdh8's ruling 2026-08-25: reroute, do not build a `5m`.** The shortness hand
   already took the transfer to reach this seat, so it wants the transfer
   machinery one round on, not a bespoke rung — the same `4m` residue 3 owes,
   one round later and one level lower than the `5m` proposed. Gated
   `len(major, ..=1) & points(10..)` at weight 145 it slots cleanly between the
   two calls that are there: `3NT` (150) with their major stopped, `4m` (145)
   short in it, `X` (140) with neither, Pass. Eleven tricks become ten, and the
   hand that should never double with a void stops having to.

   **BUILT 2026-08-25, on residue 3's knob and in its arm** — reversing the
   earlier plan to give it a separate seed. This seat is residue 3's *interfered
   tail*, and the iron rule is that a convention ships with its tails; splitting
   them would have measured half a treatment. Opener sits on the placement
   (probed: floored, it answers `4♥`). Their **jump** over the completion,
   `{completed} (4M)`, stays unauthored and is recorded in
   [minor-transfer-slam.md](minor-transfer-slam.md).

### The `P`/`X` information split — `competition.multi_px_split` (MEASURED LOSS 2026-08-27, stays default off)

**Verdict first.** `scripts/ab-2d-multi-px.sh`, `SEED_BASE 1787804916`, sha
`f44b73b9`, 230 400 bd/arm/vul, **isolation gate 0 foreign at both vuls**
(0/50, 0/36). Per fired — NV plain **−0.980** / PD **−1.780**; both-vul plain
**−1.861** / PD **−2.083**. Resolution is 10.56/√n_div = 1.49 (NV) and 1.76
(both), so **three of the four cells are resolved losses and the fourth is a
negative wash**; no cell is positive, and `sddiff` is flat (+0.224/+0.168 NV,
−0.048/+0.279 both, every one inside its ±0.4 CI). The knob stays default off.
`ab-results/2d-multi-px/`.

**The surface came in ~40× thinner than this section predicted.** 50 and 36
divergent boards out of 230 400 — 0.02%, not the "whole X/P frontier" the
script sized for. The exclusion list below is why, and it is the honest
correction to the design note: after the `140`/`180`/`176`/`178`/`150`/`152`
rungs take their share, **8–9 with no four-card major is nearly empty**. The
split is close to inert, and what it does move it moves the wrong way.

**Where the loss comes from** — selected worst tail, so **unverified** as a
population mechanism per [measurement.md](measurement.md) (no full-dump count
was run). Two clusters: (a) hands that used to double and *collect* now pass —
`off: 1NT 2♦ X 2♥ X - - -` against `on: 1NT 2♦ - 2♥ - - -`, four boards at −8
to −11, which is mechanism 1, the constraint itself; (b) the `2NT`→`3♥`/`4♥`
reroute at −10 to −13, going past a making `3NT`, which is mechanism 2's 148.
Cluster (a) landing on the constraint is what argues against splitting the
package into isolating arms: the first mechanism in the ordering below is
already the leading suspect, and the population is too thin to pay for three
more runs. Recorded, not queued.

The design as built, kept for the record:

The census above is two numbers about one decision: the `X` branch's pass-outs
read **−824 plain on 293 boards** and the `-` branch's pass-outs read **+1230
PD on 987**. K–K's double is a flat `hcp 8+` with no shape promise, so the two
branches are split by strength alone and the doubler's own rebid table is left
with nothing to say on a large slice of its population. jdh8's proposal, built
here, splits them by **information** instead:

| call | `multi_kokish_kraft` | `+ multi_px_split` |
| --- | --- | --- |
| `X` | `hcp(8..)` | `hcp(10..) \| (hcp(8..=9) & (len(♥, 4..) \| len(♠, 4..)))` |
| `-` | everything else | everything else — now including 8–9 with no four-card major |

**The hands that move are the complement of the new disjunct, not the
disjunct.** The constraint *retains* 8–9 with a four-card major; what it sheds
— and what therefore changes branch — is **`hcp 8..=9` with no four-card
major**. That slice is narrower still, because every other 8–9 hand already
outranks the double at 130: a five-card major escapes at 140 or transfers at
180, a six-card minor takes the floorless transfer at 176/178, and an 8–9-HCP
hand with 10+ *points* on distribution takes `3NT`@150 or `3♠`@152. No weight
surgery was needed on any shipped rung.

Two things follow structurally, and both ride the same knob:

1. **The doubler's natural other major becomes required, at weight 148.**
   §N4-KK residue 4's rung (`competition.multi_doubler_major`) sits at 100 —
   below everything, so it fires only on today's pass-outs. Under the split the
   8–9 doubler *is* a four-card major, so the rung is re-priced to **148**,
   uniquely between `3NT`@150 and `2NT`@145. The two knobs emit one rung;
   `px_split` owns the weight.
   - **`X (2♠) - -` is re-armed.** The withholding ruling (25 bd, −30 plain /
     +8 PD, "the misfits") was priced under the *wide* `hcp 8+` double. Under
     the split a hearts-only hand at that node is the population that used to
     sell out, so the leg is armed — reversible by its column in
     `kokish_kraft_entries`.
   - **`X (2♥) - (2♠)` stays excluded on both knobs.** The mechanism there is
     opener's *pass* over `(2♥)` denying four hearts (`multi_penalty_answer`
     doubles on `len(major, 4..)`@150 against a weight-0 catch-all), and no
     responder-side split can change what opener said.
   - **The natural `2NT`@145 does not go fully dead** — it survives on exactly
     the excluded leg, where responder holds strictly four hearts, a spade
     stopper and ≤9 points opposite a pass that denied hearts. The rung stays
     per house style; deleting it would hand that seat to the floor.
2. **The delayed `2NT` stops being a `hcp == 7` relic.** Residue 5 above records
   that rung as unreachable across most of its `7..=9` band, because the
   first-turn pass denied `hcp 8+`. Under the split the pass denies 8–9 only
   *with* a four-card major, so the band is real, and opener answers it with
   `kokish_kraft_invite_answer` (game on `hcp 16+`) instead of sitting. The band
   still reaches down to `hcp 7`, so accepting on 16 can reach `3NT` on 23
   combined — a known cost, and one of the two cells the forensic watches.

**The 148 is a reading literal, not only a routing one.** `bid_exclusion` is on
by default, so at weight 100 the natural-major bid denied the `2NT` it declined
(`hcp(8..=9) & stopper_in(major)`) and at 148 it stops denying it — a reading
move riding a weight, exactly the class
[reading-drift-handoff.md](reading-drift-handoff.md) is about.

**Three mechanisms ride this one knob** — it was four until 2026-08-27 —
and `px` vs `base` confounds all three: (1) the double's constraint, (2) the
100→148 re-weight, (3) the `X (2♠) - -` leg re-arm, (4) the delayed `2NT`
acceptance. No arm here separates them; isolating any one is a follow-up arm,
and the ordering above is the order to try if the package measures a loss.

**The fourth was the `3NT`@135 out, and it left this package by winning.**
`competition.multi_doubler_notrump` shipped default-on 2026-08-27, so the rung
is in the `base` arm too and this A/B now isolates the information split
alone — a cleaner experiment than the one designed, and the arms of
`scripts/ab-2d-multi-px.sh` were deliberately left unchanged to keep it.
The old coupling argument still holds mechanically: re-weighting to 148 sends
*more* traffic to that answer table (the 8–9 doublers with a stopper, who used
to bid `2NT`), so the split needs the repair under it — it now inherits it
from the default instead of carrying it. Both knobs still emit one rung
(`multi_px_split || multi_doubler_notrump`), so disarming the default with
`--no-ns-multi-doubler-notrump` would put it back in the `px` arm only.

**And the ladder grew one rung lower the same day, also into `base`.**
`competition.multi_doubler_minimum_notrump` shipped default-on 2026-08-27 (a
win in all four cells, `ab-results/2d-multi-doubler-min-nt/`): the 15-count's
`2NT`@120 on the `2♠` leg plus responder's `3NT`@140 acceptance, the rung below
the `3NT`@135 out. It is gated on the notrump out rather than on the split, so
it too is in both arms and confounds nothing here — the same reasoning, one
point lower. The coupling argument gets *stronger*, not weaker: 148 sends more
stopper-holding 8–9 doublers to that answer table, and the table now answers
them from 15 up rather than from 16 up. Details in
[multi-doubler-answer-handoff.md](multi-doubler-answer-handoff.md) item 2.

**Deliberately not done.** Opener's takeout `X` at `- (2M)` — the pass branch's
mirror of this split — is **skipped on jdh8's ruling**: BBA passes that seat
94.2% / 92.7% and its only action is a trump-length penalty double,
`multi_balance` is twice below resolution, and responder's own delayed takeout
`X` already covers the function. Readings do not move either:
`responder_overcall_double_reading` publishes a flat `points 8..`, which is
still the exact hull of the disjunction.

**Measured**: not yet. Sized at N4-KK scale (230 400 bd/arm/vul) rather than
residue 4's 2.3M, because the divergence surface is the whole 8–9 X/P frontier
rather than one 0.9% rung. Isolation gate first (`probe-divergence
--gate-opener ours`, 0 foreign), both scorers, `sddiff` tie-breaker.

### The mirror book — why the leak was not a seat gate

**Fixed 2026-08-25**, ahead of the re-measure. The diagnosis in the handoff
("keyed by call shape with no seat gate") was wrong in a way worth recording,
because it pointed the repair at the wrong layer.

The K–K table is *already* ownership-keyed: `kokish_kraft_entries` registers
under `Pattern::after("P* 1NT", "(2♦)")`, and an unparenthesized `1NT` is our
side. In the mirror lane the auction is `P* (1NT) 2♦`, which that pattern never
matches for us. Nothing about the table needed a seat gate.

The leak is a **frame flip**. Undeclared opponents are decoded with *our* book,
rebased so that they are "we" (`inference/projection.rs`). Their auction is then
`P* 1NT (2♦)` — genuinely, from their seat — and
`decision.their.two_diamonds_multi`, a fact about **our** opponents, survives
the flip and gets asserted about **theirs**: us. Our natural `2♦` overcall is
not a Multi, and we do not play a counter-defense to ourselves.

No pattern can separate the two frames, because from the bidder's own seat they
are the same auction. The only distinguishing fact is per-side, so the fix is a
per-side book: `System::opponents`, a second build of our own system with
`decision.their` cleared, which their calls decode in. It is built only when
something is declared, so the shipped default carries no second book.

A profile flag would not have reached it. `defense_2d_multi` chooses the
`1NT (2♦)` leg at **build** time — `inference/read.rs` says in as many words
that clearing the classify-time flag "cannot un-compile it" — which is why the
existing local precedent there (the systems-on strip clears `two_clubs_landy`
and `two_diamonds_multi` from the profile) never closed this lane.

Acceptance, both halves:

```sh
# mirror lane — the two arms must now print identical rho reads and logits
PROBE_THEIR_2D_MULTI=1 target/release/examples/probe-decision \
  "AQ54.T8653.7.954" "- 1NT 2♦ X" none
PROBE_THEIR_2D_MULTI=1 PROBE_MULTI_KOKISH_KRAFT=1 \
  target/release/examples/probe-decision "AQ54.T8653.7.954" "- 1NT 2♦ X" none

# owned lane — they must still differ, or the fix turned K–K off
PROBE_THEIR_2D_MULTI=1 target/release/examples/probe-decision \
  "AQ54.T8653.7.954" "- - 1NT 2♦" none
```

**The whole-struct clear, decided 2026-08-25 (jdh8).** The mirror clears the
whole `their` struct, so `two_clubs_landy` goes with it — the same
misapplication one suit lower, which `read.rs` had deliberately declined to fix
at its own site because it moves the Landy campaign's measured base. Fixing it
is correct; the cost is that the Landy campaign's recorded numbers no longer
describe the engine and want re-anchoring. Narrowing
`common::mirror_agreements` to `two_diamonds_multi` alone stays the one-line
reversal.

**Why no leading-pass quantifier could have done this.** A pattern-level
discriminator was considered and rejected: the routed decode cuts the auction at
*their* turn and re-parenthesises it, so the actor index and the fan move
together and the parity of leading passes carries no side information. The
parenthesisation *is* the side marker, and the flip rewrites it by construction.
Only a per-side fact — the agreements — can separate the frames.

Separately, and **not** this change's to fix: `slam::rkcb_rows` offers an
insufficient `5♥` at `… 4NT - 5♥ -` (weight 50, `asker_after_5h`). Registering
the ladder under `1NT (2♦) 4♥` newly exposes it in this lane, but it is
identical in the uncontested tree (`1NT - 4♥ - 4NT - 5♥ -` offers the same call)
and production filters illegal calls at selection. Flagged for the slam module.

### The A/B (2026-08-25, SHA `78ad4c02`, `SEED_BASE 1787606986`, 230 400 bd/arm/vul)

`scripts/ab-2d-multi-kk.sh`, results in `ab-results/2d-multi-kk/`. The
**isolation gate failed exactly as residue 1 predicted** — 843/1530 (none) and
821/1312 (both) divergent boards were opened by *them* — so the script stopped
before quoting raw headlines, by design. The headline is read from the
owner-filtered diffs (`owned.*`, the divergent boards we opened); the foreign
slice is priced separately below.

**Owned lane** (687 fired none / 491 both; per fired, `kk − base`):

| vul | plain DD | PD | SD plain | SD-PD |
| --- | --- | --- | --- | --- |
| none | −0.039 (−27 IMPs, wash) | **+0.285** (+196) | −0.071 | **+0.205** |
| both | +0.305 (+150, wash-to-win) | **+0.772** (+379, sig at the CI edge) | +0.259 | **+0.699** |

Plain wash at both vuls, PD win at both vuls, SD-PD agreeing — the decision
table's `plain wash | PD win` shippable row, *for the lane itself*. The engine
of the win is the **designed neutral pass**: first-diff `−` where v7 doubled
(+141/+335 PD none/both) and `−` where v7 relayed `2NT` (+26/+244). The plain
drag concentrates in `X` replacing v7's `3♣` GF route (−298/−169 plain, PD
≥ 0). The residue tails are visible but not net-negative: `3♠` both-minors on
5-4 drove past `3NT` into their cheap `5♥x` twice at −15…−17, and one floorless
`2NT` transfer freak — the recorded 5-5 fallback and ceiling sub-arms price
those.

**Foreign slice** (they open `1NT`, we overcall a *natural* `2♦`; raw − owned):
plain −55 / PD **−1376** at none (−1.63 per foreign board), plain **−623** /
PD **−2029** at both (−0.76 / −2.47) — at both-vul the leak is a plain-DD loss
too, not a doubling artifact. Raw totals are therefore a clear net loss (none:
−82 plain / −1180 PD; both: −473 / −1650), and a default-on ship today loses.

**Leak mechanism, probed** (`probe-decision`, worst board): with the knob on,
*their negative double of our natural `2♦`* reads as our own K–K `X` — `hcp
8+`, minors ≤ 5, **majors unlimited** — where v7 read `hcp 6+`, majors ≤ 4, a
decent model of what their X actually shows. The poisoned read flips advancer's
floor from `P` (logit 10.5) to `2♥` (11.0), and the cascade ends in doubled
partials (the overcaller even ran to `2♠` on a doubleton). First-diff `2♥`
where v7 passed is alone 238 bd / −321 plain / −1021 PD at none and 246 bd /
−720 / −1416 at both. This is residue 1 made expensive: the fix is an
**ownership gate** — key the K–K table and its readings on *our side having
opened the `1NT`* — after which the gate should read 0 foreign and the script
completes; the owned numbers say the re-measure is then expected to ship. The
`probe-1nt-interference --bucket "2♦" --responses 6` decomposition rides that
re-run.

Recorded follow-ups, each owed its own seed: the `3♠` **5-5** fallback if 5-4
measures badly, the bare stopperless `3NT` sub-arm above, a weight retune
against `probe-decision`, and the `4M` band. On the last:
`direct_4m_max` is `15` under the shipped `notrump.texas_slam_drive`, because
uncontested a 17+ six-card major takes South African Texas and drives its own
RKCB — but under their `(2♦)` those calls are Leaping Michaels, so the 16+ hand
falls back on the `3♦`/`3♥` transfer with its slam try floored. Not a
regression (v7 routes it identically and gains no direct rung at all), and the
fix is one token (`15..=18`), but it is a behaviour change owed its own arm.

[n1j]: archive/one-notrump-competitive-closed.md#n1j--the-bba-ladder-counter-shipped-default-on-2026-08-15

## N2 — Muiderberg `(2♥)/(2♠)`: the lane today

Their `2♥`/`2♠` show exactly five in the major plus a 4+ minor; we answer with
Cohen Transfer Lebensohl (`lebensohl.rs:466`, `rubensohl.rs:98`). The
2026-08-21 census read **`2♥` 409 bd / −99 plain / −0.24 per board** (PD
+0.20/+0.84) and **`2♠` 402 bd / −90 / −0.22** (PD +0.18/+0.48) — mid-table,
and PD-positive at
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

**Re-read on the 2026-08-21 arms** (`1e9a47e2`, `--bucket "2♠" --responses 8`,
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

**Measured through the leak (2026-08-25) — and the base barely moved.** Every
number below was taken before
[the mirror book](#the-mirror-book--why-the-leak-was-not-a-seat-gate)
(`29f93561`), so every arm that declared a disclosure — which is every N1 and N4
row, both vuls — read their calls off our own counter-table. The verdicts are
not withdrawn: each was a paired diff whose *two* arms shared the same leak, and
the N1g row's isolation gate read 0 foreign.

The re-anchor that was owed here **ran at `c5fbee11`** and answers the base
question: the mirror book is **below anchor resolution**. The whole 55-commit
window moved the instinct arm −0.997 → −0.994 plain / −1.131 → −1.121 PD, and
one bucket accounts for all of it — `Defensive / book / round-1`, the M1+M2
overcall ship, at +1,464 plain / +3,958 PD (= +0.0036 / +0.0097 per board,
against headline deltas of +0.0026 / +0.0101; every other bucket nets to noise). `Competitive / book / round-1`, which owns
these rows, did **not** move (−34,664 → −34,729 plain, −38,259 → −38,304 PD).
So the Landy rows moved with the fix in principle, but by less than the anchor
can see; nothing below is restated. Narrowing `common::mirror_agreements` to
`two_diamonds_multi` remains the one-line reversal, and there is now no measured
reason to take it.

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
| N1l the doubler's own rebid ladder | `competition.landy_doubler_rebids` (**off**) | **measured 2026-08-28: mixed, stays off** | SD-PD (the arbiter) **+0.523 none / −0.741 both** IMPs/fired; DD plain wins both cells (+2.365 / +1.556) but the per-rung split attributes the whole vulnerable plain win to the penalty `X`@155 (+9.196/fired, PD double-blind column flat) and the vulnerable loss to the constructive rungs — worst the `2NT` invite (−3.695 PD), whose declined half loses both scorers. Flip plan queued: keep `X` + catch-all, tighten/vul-gate the constructive rungs, re-measure. Seed `1787917699`, sha `ba003a30` | [§N1l](#n1l--the-doublers-own-rebid-landy_doubler_rebids-measured-2026-08-28-mixed-stays-off); `scripts/ab-landy-doubler-rebids.sh` |
| **N1l-flip** the cut-down doubler ladder | `competition.landy_doubler_px` (**ON**, off-switch `--no-ns-landy-doubler-px`), `landy_doubler_white` (**off**) | **`px` SHIPPED DEFAULT-ON 2026-08-29; `white` not a win** | `px` plain **+0.0107 ±.0004** NV / **+0.0142 ±.0004** vul, PD +0.0039/+0.0061, sd-plain +0.0061/+0.0100, SD-PD +0.0000/+0.0028 — a win on every column bar the one NV SD-PD wash, all six gates 0 foreign. **Selection refuted**: the `X` rung re-prices **+7.554/+9.189** IMPs/fired against the +7.489/+9.196 that selected it. `white` is `win | loss` (plain +0.0409, **DD-PD −0.0091** NV; vulnerable it *is* `px` — `white vs px` fires 0 boards), sd bracket dissenting (+0.0547/+0.0140). Two caveats shipped open: the **`Pass`@0 catch-all costs −14,171 IMPs plain NV** by shadowing a floor takeout double opener pulls to `3NT` 49.5% of the time, and `comp:landy-penalty` publishes four-plus while that floor call is short — deleting the catch-all is the owed arm and owes the tag a decision. `white`'s `!vulnerable()` reads our own colour only; asymmetric vuls unmeasured. Seed `1787942099`, sha `de59ad86` | [§N1l-flip](#n1l-flip--the-two-cut-down-arms-landy_doubler_px-shipped-default-on-2026-08-29--landy_doubler_white-not-a-win-stays-off); `scripts/ab-landy-doubler-flip.sh` |
| **N1m** opener's own rebid over their advance | `competition.landy_opener_px`, `landy_opener_rungs` (**both off**) | **built 2026-08-29, A/B owed** | no verdict. The seat §N1k lost at, re-opened as its own arm per flagged item 1. Designed off `probe-landy-opener-oracle` (103,653 + 81,023 seat boards, 105,334 deals solved): defending their major **doubled** wins every four-plus-trump bucket at both vuls (+2.8…+8.1 IMPs/bd over the floor, PD flat) and loses on two or three, so `len(major, 4..)` is the whole gate. `X`@150 above the notrump rungs supplies the ≤3-trump cap `has_stopper` could not — 17.4% of §N1k's gate was four-trump hands where its `3NT` forwent +7.0…+7.8. `3m`, `3OM` and the relay leg all priced out and are absent | [§N1m](#n1m--openers-own-rebid-over-their-advance-landy_opener_px--landy_opener_rungs-built-2026-08-29-ab-owed); `scripts/ab-landy-opener.sh` |
| **N1p** an unlimited values double | `competition.landy_notrump_no_major` (**off**), `landy_major_jam` (**ON**, off-switch `--no-ns-landy-major-jam`) | **`nt` measured loss 2026-08-30, stays off; the decoupled `4M` jam SHIPPED DEFAULT-ON 2026-08-30** | `nt vs base` **−0.0124** none / **−0.0076** both on plain DD, **−0.0012** on the SD-PD arbiter; DD-PD +0.018/+0.020 is the auto-double artifact. No colour flip. The divergence split makes the `3NT`→`X` swap the dominant bucket (83.6%/86.4%), with game reached in the baseline only 81.5%/84.8% of the time and 72–75% more room handed to the opponents — falsifier 2's *idea dead* branch. `jam vs nt` wins all four scorers (+5.541 IMPs/fired sd-plain) on 1,567 boards but rode the losing arm and measured the wrong substitution, so the rung was **decoupled** and re-measured standalone: an **eight-of-eight sweep** (`scripts/ab-landy-major-jam.sh`, `SEED_BASE=1788033942`, sha `52fbc7c1` — DD plain +1.443/+1.611 IMPs/fired, SD-PD +1.558/+1.866, every CI excluding 0, both gates 0 foreign), shipped default-on. Flagged item 2 taken up: the `X`@145 is `hcp(8..)` — unlimited in its constraint — but the ungated `3NT`@168 outranks it, so `probe-call-reading` reads partner back as `points 8..9`. The widened reading falls out of `bid_exclusion`, so **no new slug and no disclosure decision**. **Re-measured 2026-09-01** on the repaired seat as §N1-lia package D and it stays off: plain DD flips to +0.0077/+0.0075 but plain SD stays −0.0061/−0.0056 and the SD-PD arbiter reads −0.0011/+0.0000 against §N1p's −0.0012 — the seam between the two plain columns (DD reads +0.014 above SD, stable across both experiments) is bigger than the knob. D's split found the reason this table keeps reading marginal: the gate bundles opposite-signed halves, and the `min major ≥ 3` half alone is worth +3.1…+5.9 IMPs/fired | [§N1p](#n1p--an-unlimited-values-double-landy_notrump_no_major-loss-stays-off-landy_major_jam-shipped-default-on-2026-08-30); `scripts/ab-landy-notrump-shape.sh`; `scripts/ab-landy-major-jam.sh` |
| **N1-lia** Lia's counter-defense (four packages) | `competition.landy_doubler_catchall` (**off 2026-08-30**), `landy_doubler_three_honors`, `landy_doubler_three_small` (**both on 2026-08-30**), `defense_2c_landy_lia` (**off**), `landy_texas` (**on 2026-08-31**), `landy_texas_floor` (**10**) | **packages A and C shipped default-on (2026-08-30, 2026-08-31); B's build was MISPROBED, redefined in place 2026-09-01 as Lia's real ladder, then MEASURED A LOSS 2026-09-01, REFINED on its own forensic and the refinement MEASURED A LOSS 2026-09-02 (lia3) — stays off, lane parked behind the general floor rail; D measured a non-win 2026-09-01 (stays off)** | **A: the full ladder wins** — every adjacent pair a plain win at both vuls (nocatch +0.0036/+0.0014, hon +0.0008/+0.0008, cells +0.0104/+0.0118 IMPs/board NV/BV), every sd tie-break positive; PD negative throughout is the pre-registered doubling artifact. Falsifiers 1 and 3 refuted; the sibling lone-honor caveat did not carry. Package A takes up both §N1l-flip caveats: the catch-all deletion (≈ +14,171 IMPs plain NV on the flip stream) unblocked by re-wording `comp:landy-penalty` to "length or honour strength in their major", plus two three-card `X` cells split by `top_honors` (sibling priors: `len3 hon0` +0.62/+1.85, `hon1` −0.75/+0.37, `hon2+` unmeasured). Build finding worth the row: the deletion was a **silent no-op** until the doubler tables moved from `Pattern::after` guards to exact `Pattern::node`s — `Trie::resolve_floored`'s single fall-through returns a guarded fallback's all-−∞ logits unchecked, so only an exact node's rejection reaches the floor. **B: the probe was wrong, and both its A/Bs measured nobody's system.** Lia is IntoBridge's AI, probed by hand on cuebids.com, and the original read had her responder ladder **inverted** — weak transfers at the two level and natural invitations at the three, where she plays an UNBAL 4+♣4+♦ takeout at `2♥`, natural INV+ six-card minors at `2♠`/`2NT`, and six-card sign-offs at `3♣`/`3♦` (the unlisted `X`@145, `2♦`@140, `3M`, `3NT` and `Pass` are confirmed unchanged). The knob was **redefined in place 2026-09-01** — it never shipped, its off state is byte-identical, and the superseded semantics are pinned by sha (`8a778178`; measured-loss build `59cd46ee`-control), which beats a scaffolding knob whose only job is to neutralise the first. The rebuild spells the takeout as shape (`len(♥,..=2) & len(♠,..=2)`, unbalanced by construction with 4+4+ minors, so the convicted 2=3=4=4 merge stays out) and therefore moves the splinters **above** it (179/178 vs 177), inverts the minors to natural six-card rungs under a new `comp:landy-minor` slug with opener accepting at `3NT` from the top, and drops the sign-offs to **straddle** the weak `2♦`@140 (`3♣`@141 / `3♦`@139) — which closes defect 3's starved diamonds by rung order alone and removes every `vulnerable()` term from the package. Responder's placements are re-banded for INV+ (`Pass` replaces two game-forcing catch-alls), the `2♠` ask's unalerted `4♣`@20 becomes a natural `3♣`@20, the minor-transfer-slam rule is honoured above opener's acceptance, and **all six flagged lia-only node families are closed** (the constraint that kept them open — not disturbing the in-flight A/B — died with it). The correction's one predicted loss is pre-registered as falsifier 1 of the new runner: the exactly-five 8-9 hand has no rung under Lia, and that band was the built ladder's biggest measured win. **The two superseded verdicts, kept as records of what they scored:** the first build (seed 1788122360, plain **+0.0050** ±0.0012 NV / **−0.0384** ±0.0014 BV, PD −0.0756/−0.1210, gates 0-foreign) and its four-defect repair, whose A/B was **STOPPED mid-flight** on the probe correction with one cell done (seed 1788247951, control `ce94faeb`, NV plain **+0.0191** ±0.0011 / PD **−0.0382** ±0.0014, gates 0-foreign; BV never run). That NV row is the doubling-artifact shape on a bid-more mechanism and the stopped runner's header stated its arbitration rule wrong — **moot**, since there is no both-vul cell and no surviving build; no decision is owed. The forensic below still stands as evidence about the lane, and the campaign section says which defects transfer. Forensic (`probe-divergence --imps`, bucketed by first differing call and responder's hand) splits the deficit into four named defects, none of them the concept: the **contested tails are unauthored** (every lia node requires the opponents to have passed, so any opponent bid drops the rest to a floor with no forcing channel — 95%/94% of the diamond rung's whole loss, and −63,950/−80,789 on the club rung), the **weak five-card sign-off** is vulnerability-dependent (uncontested weak: exactly-5 clubs **+1.405 NV / −0.803 BV** per fired while 6c and 7+c win at both colours — falsifier 2 confirmed sharper than posed, and that one cell is 45% of BV's PD deficit), the `2NT` cap **starves diamonds** into passing (−35,827/−32,432, falsifier 4), and the sole `2♥` takeout is the worst per-fired rung (−4.07/−5.00). **Falsifier 1 is reversed, not merely refuted**: the N1c right-siding trade was wrong on plain DD — the restored natural invitations are the ladder's biggest win (`3♣` **+85,613 NV / +35,920 BV**). At BV the first three defects sum to −176,261 of a −176,837 total, so the rest of the ladder is roughly break-even. Repair queue in size order: contested tails, a 6+ club floor for the weak `2♠` leg when vulnerable, a rung for the starved diamonds, the 2=3=4=4 merge. **All four built 2026-09-01** (`ab-landy-lia-repair.sh`, A/B owed, defaults byte-identical ×2), and re-solving the kept divergence sets inverted two of the repair plan's three pre-registered rules: the restored invitations split on **length** not colour (`3♦` +0.920 IMPs/fired quiet at exactly five, −0.910 at six-plus), and the starved diamonds are a **0-4 HCP** seam between `2NT`'s quality gate and the `2♦` escape's five-HCP `natural_floor`, not a 7+ HCP cap problem — so `2NT` takes the N1j transfer's own shape gate back (`len(♦,6..) & points(2..)`) and the plan's `2♦` cap lift is a measured no-op, not built. Defect 1's mechanism is that the floor **bids** at the unauthored nodes (`4♣` on 72% of `2♠ (3♠)`, 96% of `3♣ (3♠)`), so the repair is mostly authored `Pass`: opener sits, responder captains. Two nodes the plan never named were added on the census — the four-level entries (floor −3.4/−4.3 per fired) and the balancing seat (−8,347 NV on the club rung). Flagged and deliberately **not** repaired: `landy_recue_answer`'s `4m`@20 has no answer node, which is four of the five worst both-vul boards, but that seat is shared with the control arm and owes its own A/B. **B measured 2026-09-01 and LOST** (`ab-landy-lia2.sh`, seed 1788264406, control `32242d63`, 4.608M bd/arm/vul, gates 0-foreign): plain **−0.0077 ±0.0009** NV / **−0.0059 ±0.0010** BV, PD −0.0127/−0.0172; plain was the pre-registered arbiter at both colours, sd killed unrun. The forensic splits it by leg and the **club leg wins** (+0.0126/+0.0137 per board; `2NT → 3♣` is the run's biggest cell at +46,715/+54,641) while the **diamond leg loses** (−0.0201/−0.0181) — so the ladder was **refined, not reverted**, on five measured findings: (1) no weak diamond rung beats the baseline's wide transfer (6♦ thin 0-4 HCP → `3♦` −1.966/fired, 6♦ 5+ HCP → `2♦` −2.534, 7+♦ → `2♦` −4.238, against `2NT` at −0.709…−0.762), so `2NT` carries the whole INV+ band, `3♦`@142 re-gates to **excessive** diamonds and the `2♦` escape's ceiling rises to eight HCP; (2) the `Pass`@0 sell-outs were selling out to a floor that was **right** — cell `3♦ → -` is 14,717 boards at −22,119 NV / −13,364 BV, 14,699 of them after our own `2♦` — so `landy_lia_overcalled` loses its registrations over the weak rungs and the escape *and* its own catch-all, at exact `Pattern::node`s (package A's silent-no-op trap), and the code comment calling the floor's `3♦` "a law-of-total-tricks violation" is **withdrawn**, resolving that flagged doc/code discrepancy against the comment; (3) **right-siding is not null** — the earlier "0 of 18,170 declarer flips" read the declaring-*side* column (123 NV / 66 BV); declarer *seat* flips on 26,664/27,911 same-contract boards worth −0.0014/−0.0023 per board, worst cell `2NT → 3♣`, so the Max-break+ table makes the **completion** opener's minimum default and takes declarership back; (4) the by-length answer answered the wrong question, so `comp:landy-length` retires for `comp:landy-super` (super-accept / `3NT` / completion); (5) the `X`-vs-takeout cell is genuinely mixed — the 8,113/6,236 boards with two-plus in each major are **+1.854/+1.154 plain** for the takeout and **−1.305/−2.169 PD** — and the refinement gives them to the **double** (narrowed to `len(♥,2..) & len(♠,2..)`, with the takeout dropped below it in two bands and no major term), pre-registered as the next arm's falsifier 1. **The refinement (lia3) LOST 4/4 on 2026-09-02** (seed 1788290089, control `deeb0252`, gates 0-foreign): plain **−0.0056 ±0.0011** NV / **−0.0254 ±0.0012** BV, PD −0.0331/−0.0569, SD-PD −0.0158/−0.0342. The diamond leg did not move (−86,528/−91,463, every cell losing to the wide transfer — the INV+ gate is what has to go there), the club leg's win halved (+27,990/+14,627, the INV+ rung flipping through the Max-break+ answer and the floored contested seats), and the weak 4-7 band is the both-vul catastrophe (`- → 2♥` −42,572 plain / −107,283 PD, routed through the `3NT`@160 accept — falsifier 2 fires; 1 and 3 refuted; 4 fires, the floor pushing `4♦` then phantom-cueing `4♠`; 5 partly). lia doubles the boards on which the floor makes a substantive call (86,491 vs 42,994 NV; `examples/probe-layer-replay`), the biggest diamond cell is the 0-4 HCP six-diamond hand with no rung and both INV+ rungs lose through the super-accept's `retreat`@0, and the named floor class is **phantom suits** — floored bids on ≤4 cards with ≤5 announced combined, a net −30,369/−33,277 plain pool on the lia arm and **+35,346/+25,145 against the baseline's own floor on the same boards** (the floor's vector carries a raw we-bid-this-strain bit and partner's last bid, no alert column, so an artificial `2♠` reads as spades bid). Disposition: the envelope-gated new-suit veto (the lane's 2026-08-14 residue) is built and measured on the default system first; lia4 only under it. **C: shipped default-on** (seed 1788181796, gates 0-foreign) — an eight-of-eight sweep, plain **+0.616 NV / +0.711 BV** per fired, PD +0.816/+0.996, sd-lead +0.220/+0.352 plain and +0.305/+0.506 PD; fires on 0.03%/0.02% of boards so the per-board move is +0.0002, >6.7σ/>5.6σ off the printed CI bound. **Its mechanism is the reverse of the design's**: the DD-visible half was supposed to be the slam reroute, which reaches the five level on 5 of 2211 divergent boards (0 at both-vul), while right-siding was supposed to be invisible — instead 96.4%/96.3% of divergent boards are the *same contract from the other seat*, and that bucket is the whole win. The correction to the iron rule: DD is blind to right-siding's **concealment**, not to its **lead direction** — a different declarer puts a different defender on lead and the solver prices that honestly; sd-lead, where the leader is blind in both arms, keeps the sign at a third the size. Falsifier 2 (the alerted transfer leaks the anchor major) refuted by exactly those sd rows; falsifier 1 moot; PD>plain here is not the doubling artifact but the baseline's wrong-sided contract failing harder under a defense that always finds the lead. Residual: the transfer hands the opponents room on ~4% of fired boards (52/37, zero the other way) and draws 33/30 doubles the direct jam never drew — the entire worst-5 list, outweighed ~5:1. Defaults byte-identical while off (`smoke-default` ×2); C's ship moves `alert-sites.txt`'s `[their-landy]` section only. **D: measured non-win** (seed 1788191041, control `60115871`, gates 0-foreign) — `landy_notrump_no_major` stays default off, and A's repair did not reverse §N1p. Plain DD **+0.0077 NV / +0.0075 BV** and PD +0.0145/+0.0146 are both CI-clear positive, but PD is the decision table's `loss \| win` row verbatim (synthetic doubles of the baseline `3NT`s the candidate no longer bids, pre-registered as falsifier 1), and the two columns with no synthetic double **straddle zero** — plain SD −0.0061/−0.0056 against plain DD's +0.0077/+0.0075. On SD-PD, the column §N1p itself named the arbiter, the two experiments read −0.0012 (§N1p) and −0.0011/+0.0000 (D): the artifact leans the candidate's way and it still cannot clear zero. The **lead-model seam is bigger than the knob** — plain DD reads it +0.0142/+0.0138/+0.0131 above plain SD across §N1p NV, D NV and D BV, stable across two routings, and roughly double the effect it is adjudicating; the sign is the tell, since the candidate's mechanism is declaring `3NT` less and the clairvoyant-lead scorer is the one that likes it. Falsifier 3 answered: the repair reached, moving both *undoubled* columns +0.020 and both doubled columns ≈0. **Falsifier 2 returned a third branch neither posed** — the gate is a bundled disjunction whose halves have opposite signs. Bucketed by the mover's own major lengths (`off=3NT` is 74.3%/79.9% of divergence, `3NT → X` 98.9%/99.0% of it), plain IMPs/fired are monotone in the **short** major and break sign between two and three: 4-4 +4.74/+5.90, 3-4 +3.53/+4.74, 3-5 +3.11/+4.50, 4-5 +3.47/+4.95 against 2-4 −0.13/+0.31, 2-5 −0.20/−0.27, 1-4 −2.08/−1.86, 1-5 −1.84/−2.30 — +52,861/+50,663 IMPs in the `min major ≥ 3` half against −18,838/−10,758 in the `≤ 2` half. They showed both majors, so 3+ in each is a misfit with nowhere to run and a singleton is a big fit for them; the narrowed gate (bar `3NT` only when both majors are 3+ and one is 4+) is owed as its own arm under the disjunctive-gate rule, worth +0.0115/+0.0110 plain on a third of the traffic. Residual for that arm: the `both NS` bucket, where opener pulls the double and we declare something worse, costs −1.710/−2.657 per fired on 15.4%/14.1% of divergence. Flagged: `ab-dump-sd` has **no `--on-ns-landy-*` disclosure flag**, so every §N1-lia sd row reads both arms with control semantics | [§N1-lia](#n1-lia--lias-counter-defense-the-minor-ladder-a-level-down-the-doubler-unshadowed-texas-at-the-four-level-packages-a-and-c-shipped-default-on-b-and-d-measured-non-wins-b-refined-on-its-own-forensic-ab-owed); `scripts/ab-landy-lia-doubler.sh`, `ab-landy-lia.sh`, `ab-landy-lia-repair.sh`, `ab-landy-lia2.sh` (B's measured loss, VERDICT block + forensic), **`ab-landy-lia3.sh`** (the refinement's measured loss, VERDICT block + forensic; `examples/probe-layer-replay` is its provenance tool), `ab-landy-texas.sh`, `ab-landy-nt-remeasure.sh` |
| N4 their `(2♦)` as a Multi | `their.two_diamonds_multi` — disclosure; engine default undeclared | **SHIPPED 2026-08-15, v7 of seven rounds** | v7 vs base ×3 seeds, owned: NV `plain wash \| PD win` (+0.00100 ±0.00067), vul plain **+0.00061 ±0.00056** \| PD +0.00061 ±0.00069, both-vul pool `win \| win`; paired vs v4 better on 3 of 4 cells. Every raw headline was 60–70% foreign — verdicts are owner-split | [§N4](#n4--their-2-as-a-multi-shipped-2026-08-15--v7-seven-rounds-default-on-vs-bba-via-the-census); [v1–v6](archive/one-notrump-competitive-closed.md#n4--measurement-rounds-v1v6) |
| N4 residue — Multi reader / stopper ask | `reading.their_multi_reading` (**on**), `competition.multi_stopper_ask` (**Off**) | reader **SHIPPED DEFAULT-ON 2026-08-16**; ask **REFUTED as a default** | reader `plain wash \| PD win` ×3 seeds — −29 plain / **+643 PD** over 1.3824m boards, 0 foreign on every pair. Both stopper modes landed on `plain win \| PD wash` (the artifact row) and tied with each other, so no combined arm ran | [§N4 residue](#n4-residue--reader-shipped-stopper-ask-stays-opt-in-measured-2026-08-16) |
| **N4-KK** Kokish–Kraft whole-table counter | `competition.multi_kokish_kraft` (**on**) | **SHIPPED DEFAULT-ON 2026-08-25** | Re-measure on a fresh seed after the mirror book (`SEED_BASE 1787615025`, SHA `f2ecb3c6`, 230 400 bd/arm/vul): **isolation gate 0 foreign at both vuls** — 0/683 and 0/482 against a 55% prior rate. Both-vul `win \| win`: plain **+0.0019 ±0.0013**, PD **+0.0023 ±0.0017** (+0.907/+1.102 per fired); NV `wash \| wash` (+0.0002 ±0.0012 / +0.0012 ±0.0015); sd-lead agrees in all four cells. **No negative reading in eight.** The first run (`1787606986`) was 55% foreign and its dumps are dead — the fix moved the v7 control arm | [§N4-KK](#n4-kk--the-kokishkraft-counter-a-whole-table-variant-shipped-default-on-2026-08-25) |
| N4e floorless weak escape over their Multi | `competition.multi_weak_escape` (**`Some(6)`**) | **SHIPPED DEFAULT-ON 2026-08-22 at `Some(6)`; `Some(5)` REFUTED as a default** | `plain wash \| PD win` ×2 seeds, 921,600 bd per cell-set: pooled DD plain +0.00028 ±0.00039, **PD +0.00063 ±0.00049**, sd plain +0.00013 ±0.00041, **SD-PD +0.00052 ±0.00049** — all four PD and all four SD-PD cells non-negative, one negative reading in sixteen (−0.0003, inside CI). `five` is the doubling-artifact row (pooled plain **+0.00085 ±0.00051**, PD **−0.00030 ±0.00069**, vulnerable PD CI-clear negative on **both** seeds) and paired vs `six` is CI-clear worse on PD (**−0.00095 ±0.00048**). Two rounds were lost to the systems-on strip leaking the Multi table into the 1NT-overcall lane (26/260, 27/267 foreign); after the 2026-08-22 fix, 8 of 12 pair-cells gated **0 foreign** and 4 carried exactly **1** board of the campaign's mirror-read leak | [§N4e](#n4e--the-floorless-weak-escape-shipped-default-on-2026-08-22-the-six-card-rung-five-refuted) |
| N4b `(2♦)` diamond penalty double | `competition.two_diamond_double` (**`None`**) | **measured 2026-08-15 — sweep NULL, opt-in** | all 28 raw cells CI-clear positive and **all of it a leak** (84.9% foreign); owned subset has no CI-clear cell in 28. Two findings kept: the **alert is what makes a gate a reading**, and the "they sit 43%" claim was retracted (0 of 141 on our-opened boards) | [closed §N4b](archive/one-notrump-competitive-closed.md#n4b--the-2-diamond-penalty-double-built-2026-08-15-sweeping) |
| N4f opener's balancing seat + the two Multi reading knobs | `competition.multi_balance`, `reading.their_multi_advance_reading`, `reading.their_multi_double_reading` (**all off**) | **measured ×2 rounds 2026-08-22 — nothing ships; all three opt-in** | 4 seed-sets, 24 pairs, **0 foreign on every one**. `balance` is **below this harness's resolution** — ~18 bd of reach per 230,400, signs flipping between rounds, pooled ≈ −0.00003 IMPs/bd; unresolved, wants reach (a sub-lane harness or sd), not another seed. `xfloor` is a wash. `advance`'s round-1 loss was its bundled `♥3+ & ♠3+` claim — probed false, removed; suppression-only then diverged on **6 boards in 1.84 m**, so the false read is **inert**. Its correctness-only default flip was **withdrawn**: a trigger, not a default | [§N4f](#n4f--openers-balancing-seat-and-the-two-reading-knobs-measured-2-rounds-2026-08-22-nothing-ships-all-three-stay-opt-in) |
| N4g BBA-reading v6 retrain | `american_v6_their` + both parked Multi readers (**all opt-in**) | **measured 2026-08-23 — N4 plain gate missed; no ship** | Matched-corpus CE +0.000223, paired CI a wash. Two-seed `--filter-1nt` screen is broad-positive (vul PD **+0.0185 ±0.0079**), but the owned `(2♦)` slice is **−11 plain / +845 PD** over 819,200 accepted deals: plain `−0.000013 ±0.000468`, not the pre-registered improvement. The whole anchor was skipped. Exact replay 4,007,503/4,007,503 | [retrain round](#the-retrain-trigger-fired-2026-08-23--pd-gain-plain-target-missed-no-ship) |
| N4-mirror our `2♦` overcall of *their* 1NT | `natural_overcall_hcp_floor` = 8, `natural_overcall_advance_enabled` (**both on**) — the defensive lane's knobs | **handed over 2026-08-23; M1+M2 SHIPPED DEFAULT-ON there the same day** | The forensic corrected the −558 headline to the whole lane, both arms: **2139 bd, +696 plain / −397 PD**. M1 (HCP floor) + M2 (the unauthored advance) measured as one package, 204.8k bd/arm/vul × 2 seeds, **0 foreign in all four cells**: plain DD +0.0011/+0.0068 and +0.0041/+0.0096, **SD-PD +0.0086/+0.0164 and +0.0100/+0.0202, every cell CI-clear**. The contested tail `(1NT) 2x (X)` was censused and closed **below resolution**; `k = 9` and the 8-vs-11 floor gap stand as *leave it* | [defensive-overcalls.md](defensive-overcalls.md#defense-to-their-1nt--the-1nt-2-mirror-panel-forensic-2026-08-23) |
| N3 `(3♣)`–`(3♠)` preempt of our 1NT | `nt_high_overcall_responses` (**on**), `nt_high_overcall_3nt_stopper` (**off**), `nt_3c_transfers` (**off**) | **SHIPPED DEFAULT-ON 2026-08-18** | owned plain **+0.00208 ±0.00126** NV / **+0.00293 ±0.00160** vul, PD +0.00079/+0.00180, sd agreeing on all four cells, **zero negative cells in sixteen readings**. The private `3NT` bit shipped **off** (+2.37/+1.65 per fired, 0 foreign); the BBA-style double continuation was **refuted and removed** (all eight cells negative) | [§N3 — measurement rounds](archive/one-notrump-competitive-closed.md#n3--measurement-rounds) |
| N3 opener's answer to the takeout `X` | `nt_high_overcall_x_major_at_four` (**on**), `_x_leave_in` (**on**), `_x_leave_in_three` (**off**) | fit rung **SHIPPED 2026-08-19**; leave-in **SHIPPED 2026-08-20** (v2, `len(over, 4..)`); honor half **REFUTED** | fit rung 4/4 DD cells CI-clear (+1.14/+1.69 per fired), 0 foreign of 1103/1340. Leave-in `length` **CI-clear positive in all eight cells** — plain +0.0071 ±0.0009 NV / +0.0104 ±0.0012 vul (+2.38/+3.41 per fired), 0 foreign — and **replicated on a fresh seed** at +0.0074/+0.0111 with every suit DD-positive, so no suit gate exists. `three` sd-lead **−2.44/−2.99 per fired**: bundled as one gate the package would have measured as a loss at both vuls | [§N3 — measurement rounds](archive/one-notrump-competitive-closed.md#n3--measurement-rounds) |

### Memory compaction notes (2026-08-16)

Moved verbatim to
[the archive](archive/one-notrump-competitive-closed.md#memory-compaction-notes-2026-08-16): the refuted stopperless
`3NT` escape gate, the opt-in gambling games over `1NT (X)`, the historical
ship commits, and the superseded statements to ignore if met in old notes.
