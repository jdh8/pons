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
| `(2♣)` | undeclared: systems-on rebase + stolen-Stayman `X`; declared Landy: **the N1 counter, every seat authored** ([§N1](#n1--the-landy-2-counter-shipped-default-on-2026-08-15)) | `lebensohl.rs:388-446`; `landy_*` in `lebensohl.rs` |
| `(2♦)` | Transfer Lebensohl's Stayman/Smolen/Jacoby/Leaping-Michaels leg | `lebensohl.rs:450-462`, `rubensohl.rs:332`; continuations `lebensohl.rs:584-608` |
| `(2♥)`, `(2♠)` | Cohen Transfer Lebensohl | `lebensohl.rs:466`, `rubensohl.rs:98`; continuations `:550-566`, `:573-578` |
| `(2NT)` | Unusual-vs-Unusual | `uvu.rs:139`, `:21`, `:145-161` |
| `(3♣)`–`(3♠)` | **book, complete** — N3's forcing three-level suit / `4M` / takeout `X` / `3NT` table and opener's one answer to each (`nt_high_overcall_responses`, **shipped default-on 2026-08-18**); the `(3♣)` transfer variant rides `nt_3c_transfers` (opt-in) | `nt_high_overcall.rs` |
| `3NT` and up | **floor** — `high_overcall_responses` covers suit openings only | `high_overcall.rs:152` |

The Multi counter is [§N4](#n4--their-2-as-a-multi)
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

Shipped and refuted packages have left this table; the coverage inventory
above and the [ledger](#ledger) represent them, and each lane's section below
says what is still owed. The `(2♦)` Multi lane is **closed** (2026-08-23,
reopened once for N4-KK and closed again 2026-08-27); the `(2♣)` Landy lane
finished its §N1r queue on 2026-09-24 and shipped §N1s (the Wilkosz `3♦`) the
same day. The census's two open cells are `4+`
(**−2.93/bd**, N3-x) and, by total, the two systems-on two-level buckets `2♣`
(−275) and `2♠` (−189), both inside per-board CIs of ±0.47–0.76 — a re-rank to
watch, not a package.

| # | Package | Knob | State |
| --- | --- | --- | --- |
| **N3-x** | **`X` over their `(4x)`** — the floor cannot double above the three level at all (`their_live_bid_at_most(3)`, [instinct.rs:6058](../src/bidding/instinct.rs)), and BBA's advancer sits for our double on **96.7–99.9%** of hands | new (book) | `4+` bucket 43 bd / −126 plain / −107 PD, **−2.93/bd** — the lane's worst cell per board on either scorer, on a CI (±3.58/±4.15) that swallows the total. The `(3x)` template does **not** widen: their four-level overcalls are *eight*-card suits, six times rarer |
| **N2d** | relay with a 6+ suit below 6 HCP, over `(2♠)` only | book | **Re-read 2026-08-21: 25 bd, −77 plain / −52 PD, −3.08/bd** — the worst hand class in the lane on both scorers. Contradicts the PD-distilled floor ([`lebensohl_relay_shape`](../src/bidding/american/competition/lebensohl.rs)); needs the A/B, not a re-derivation |
| N5 | Complete Jacoby, then re-measure | `competition_over_transfer` (**off**) | measured loss *while missing its two most-fired cells* — `(2♥)` over our `2♦` and `(2♠)` over our `2♥` ([over_our_jacoby.rs:100-103](../src/bidding/american/competition/over_our_jacoby.rs)). A half-built loss, resumable |
| N3-fit | fresh-seed confirmation of the `4M` fit rung | `nt_high_overcall_x_major_at_four` (**on**) | Round 5 shipped it on one seed (+1.14/+1.69 per fired, 0 foreign) and owed a confirmation; rounds 7–8 ran on a `base` that carries it. Shipped but not settled |
| N6 | `(2NT)` penalty discipline | `uvu_encircle` et al. | 118 bd, +6 plain, no replicated loss on four re-anchors. Mechanism stays priced: **BBA doubles 46.7%** and cues `3♣` for both majors ([reference](ai-bidder/bba-1nt-counter-defense.md)). Parked |
| N3-three | single-dummy re-measure of the refuted honor half | `nt_high_overcall_x_leave_in_three` (**off**) | Round 7 refuted it at sd-lead −2.44/−2.99 per fired; its `plain wash \| PD win` DD signature is the doubling artifact. Opt-in house-rule candidate |
| N3-xfer | re-measure the `(3♣)` transfers | `nt_3c_transfers` (**off**) | Four seeds, all pooled cells positive and an order of magnitude inside their CI. **Requeued as a plain-DD item 2026-09-01**: the completion moves declarer, and §N1-lia package C measured that half on plain DD |
| N2c | the no-call 8–9 count with 0-1 / 4+ in their suit | book | **Re-read 2026-08-21: 19 bd, +11 plain / +91 PD** — positive on both scorers. Parked pending replication; n is small enough that either sign is seed noise |
| N7 | Absent responses contested | new | Puppet `3♣`, `3♦`, splinters, `3NT`, Texas, `4NT` — rarest in the system |
| N8 | Delete `1NT - 2NT - 3♣ - 3♦ -`'s `pass_out` node — the redundant half of the pair | knobless (book node) | Inherited 2026-08-18 from the [authored-reading campaign](authored-reading-handoff.md). Probed 2026-08-17: with the node removed the floor offers **`P` alone** on every hand, so the node buys nothing; its twin `1NT - 2♠ - 2NT - 3♣ -` blasts **`4♣` 1.200 over `P` 0.000**, so the pre-registered "any hand still blasts → leave both" rule kept both. This is the deletion of the redundant one alone; the twin's repair is the floor-side settle rail ([archive/dutch-system.md](archive/dutch-system.md#the-wj-floor-campaign--bbas-polish-club-as-dutchs-teacher)) |
| — | reading drift after the N3 leave-in: 11.8%/9.6% of divergences move a *later* call only, −0.56 DD / −0.63 sd per fired vulnerable | — | Owned by [reading-drift-handoff.md](reading-drift-handoff.md). Pooled positive; the vulnerable cost is real |

## N1 — the Landy `(2♣)` counter (**SHIPPED DEFAULT-ON 2026-08-15**)

N1 shipped on 2026-08-14; N1j's BBA-ladder table superseded its original
stack as the default on 2026-08-15, and a second campaign (2026-08-28 →
2026-09-24) authored every seat above and beside it. The engagement is
`their.two_clubs_landy` — a disclosure, not a knob; undeclared stays natural
(the systems-on rebase). The generated BBA card can disclose only
`Transfers if RHO bids clubs = 1`, so the alignment is structural, not
literal. Histories: [N1–N1j](archive/one-notrump-competitive-closed.md#n1--the-landy-2-counter-shipped-default-on-2026-08-14)
(tables, measurement, exploration trail) and
[§N1l–§N1r](archive/one-notrump-competitive-landy.md) (every later verdict,
build record and forensic).

**The shipped stack**, seat by seat (all **on** unless marked; every off
knob's default state is byte-identical):

| Seat | Knobs | What ships |
| --- | --- | --- |
| responder's table | `defense_2c_landy_bba`, `defense_2c_landy_weak_2d_cap` | N1j's ladder: `X` values, `2♦` escape capped `hcp(..=6)`, wide minor transfers `2NT`→♣ / `3♣`→♦ on `points(2..)`, splinters `3♥/3♠`@179/178, `3NT`@180 (both stoppers) / @168, Texas `4♣/4♦`, direct `4M` at exactly 15 |
| the game-forcing Wilkosz | `landy_wilkosz` (§N1s) | `3♦`@181 = 5-5 with a major, `points(10..)`, dead at favourable; opener names a three-card major, responder raises or retreats to `3NT`, opener finds the other major; both sides' tails over their `(X)`/`(3M)` |
| the two-level majors | `defense_2c_landy_strength_majors`, `_strength_doubles`, `_strength_nv_invite`, `landy_recue_signoff` (§N1q) | sorted by **strength**, not shortness: `2♠`@177 the 4+4+ INV+ band, `2♥`@141 the weak five-four band beside the escape; `Pass` rails at the floor-phantom nodes; the 8-9 half of `2♠` gated `!vulnerable()`; the doubles' polarities over their raises |
| `3NT` vs the values `X` | `landy_notrump_no_major_favourable` (§N1r step 0); `landy_notrump_no_major` (**off**, all colours) | game hands with a four-card major double instead of declaring, **at favourable only** — the colour flip §N1p and §N1-lia D missed by never running the asymmetric cells |
| the splinter seat | `landy_splinter_rebids`, `landy_splinter_tails` (§N1r row 1); `landy_splinter_stopper` (**off**, measured non-win) | responder's rebids over opener's answer (`Pass` over `3NT`, `5m` over `4m`, the `(X)` twins — the floor's phantom `4♠` is gone) and both sides' calls over their `(X)`/`(4M)` of the passed `3NT` |
| the values doubler's rebids | `landy_doubler_px` (§N1l-flip), `landy_doubler_catchall` (**false**), `_three_honors`, `_three_small` (§N1-lia A), `landy_doubler_game` (§N1r row 9); `landy_doubler_rebids`, `_white` (**off**) | penalty `X` on four-plus of their major, the two three-card cells split by top honours, **no catch-all** (the floor's takeout-shaped double is un-shadowed), and at favourable the `3NT`@150 / natural `3m` rebids over their `(2M)` runout |
| opener's rebid over their advance | `landy_opener_px`, `landy_opener_rungs` (§N1m) | penalty `X`@150 on `len(major, 4..)` above the notrump rungs; `px` alone is refused by the sd bracket |
| the four level | `landy_texas` (`landy_texas_floor` 10), `landy_major_jam` (§N1p) | Texas transfers over the counter (a right-siding win DD *does* see, via lead direction) and the direct `4M` jam |
| the minor-transfer slam try | `landy_minor_slam_answer` | `4m` on `points(13..) & len(minor, 6..)` after the completion; opener `4NT` RKCB on `hcp(16..)` else `5m` ([minor-transfer-slam.md](minor-transfer-slam.md)) |
| Lia's ladder | `defense_2c_landy_lia` (**off**) | lost twice (lia2, lia3); the general floor rail it was parked behind was itself refuted, so the park is void and lia4 stands alone, unblocked and unhelped |

**Still open, all below resolution or deferred:**

- `landy_notrump_no_major`'s narrowed gate (`min major ≥ 3` only): §N1-lia
  D's split priced it at +0.0115/+0.0110 plain on a third of the traffic, but
  §N1r step 0b's misfit gate lost at both-vul, so it is at most a favourable-only
  refinement of the shipped knob.
- `landy_splinter_stopper` refined by exact stopper type (jdh8 2026-09-24:
  stays opt-in, the lead seam judged real).
- §N1r rows 4 (direct `4♥` short in spades) and 8 (`4m` above the §N1q rail):
  **closed unbuilt 2026-09-24**, ≈ +0.00004 / 0.00007 per board.
- **Flagged, not fixed** (reversible defaults, [full text](archive/one-notrump-competitive-landy.md#flagged-not-fixed-n1--reversible-defaults-proposed)):
  the `X (2NT)` leg stays floor-owned (after the strong advance the overcaller
  jumps to `4M` 54.3% of the time); `artificial_calls_are_alerted` cannot
  cover `LANDY_PENALTY` (explicit alerted-scope assertions guard it instead);
  `--filter-landy` admits only strictly balanced openers, so a long-minor rung
  cannot be priced under it — state the blind spot, do not widen the filter.
- **Not evidence:** BBA's behaviour at the doubler's rebid seat is unknown;
  the `opener-c-x2*` probes read opener's seat. A position-6 probe is unrun.

## N2 — Muiderberg `(2♥)/(2♠)`: the lane today

Their `2♥`/`2♠` show exactly five in the major plus a 4+ minor; we answer with
Cohen Transfer Lebensohl (`lebensohl.rs:466`, `rubensohl.rs:98`). Mid-table on
the census (`2♠` −189 / `2♥` −79 plain), PD-positive at both vulnerabilities.

The 2026-08-15 census by response put the lane's headroom in *our own weak
calls being unread*, not in BBA's plain Lebensohl. Two of three fixes shipped
2026-08-16 — **N2e** `instinct.forcing_ceiling_read` and **N2b**
(`ReadingProfile::strength_ceilings` + `ReadingScope::All`); **N2a**, a book
node for `{relay} 3♦ -`, stays parked because it would shadow the floor. The
2026-08-21 re-read replicated every sign: `X` wins (+1.39/bd), the `2NT` relay
loses on both scorers, and Pass is a plain loss PD nearly recovers. Split by
hand class, **N2d** (≤5 HCP with a 6+ suit, −3.08/bd) replicates as the worst
class in the lane and **N2c** does not (now +11/+91) — both in the queue.
Tables: [the pre-fix census](archive/one-notrump-competitive-closed.md#n2--the-pre-fix-census-2026-08-15)
and [the re-read](archive/one-notrump-competitive-closed.md#n2--the-2026-08-21-re-read-by-response).
The reading defect it exposed is the whole book's:
[authored-reading-handoff.md](authored-reading-handoff.md).

## N3 — their `(3♣)`–`(3♠)` preempt of our 1NT

**Shipped default-on 2026-08-18** (`nt_high_overcall_responses`), opener's
answers to the takeout `X` on 2026-08-19/20. Code
[nt_high_overcall.rs](../src/bidding/american/competition/nt_high_overcall.rs),
keyed `P* 1NT (3x)`; runner `scripts/ab-nt-high-overcall.sh`. Nothing keys on
a disclosure — BBA's three-level overcalls are natural seven-card preempts.

| Knob | Default | Verdict |
| --- | --- | --- |
| `nt_high_overcall_responses` | **on** | responder's table: forcing new suit, weak `4M`, takeout `X`, `3NT`; opener's one answer to each |
| `nt_high_overcall_x_major_at_four` | **on** | opener's `4M` fit rung over the double (N3-fit: one seed, confirmation owed) |
| `nt_high_overcall_x_leave_in` | **on** | the length leave-in `len(over, 4..)`, replicated on a fresh seed |
| `nt_high_overcall_x_leave_in_three` | off | the honor half, refuted at sd-lead (N3-three) |
| `nt_high_overcall_3nt_stopper` | off | the lane's private stopper bit, +2.37/+1.65 per fired but shipped off |
| `nt_3c_transfers` | off | the `(3♣)` transfer variant, four-seed wash (N3-xfer) |

Tables, the three floor defects it replaced, disclosure and the dead-row note:
[the shipped tables](archive/one-notrump-competitive-closed.md#n3--the-shipped-tables-2026-08-18);
the eight rounds: [measurement rounds](archive/one-notrump-competitive-closed.md#n3--measurement-rounds).

**Open residue:** N3-x, N3-fit, N3-three, N3-xfer and the reading drift, all in
the queue. **Flagged, not fixed** (floor defects, reversible defaults proposed):
a five-level cue the floor passes (`1NT (4♥) X - 5♦ - 5♥ (X) - - -`, a floor
rail item); we never overcall a 1NT opening at the three level while BBA does
on 1.5% (DD-blind obstruction, a single-dummy item); `nt_answer_double`'s
unreachable `4M@25` rows are **deliberately left** — deleting them narrows the
call's reading; opener's answer to a forcing major never shows the other
four-card major (one book answer, floor beyond). The penalty pass is *not*
open: it shipped as the length leave-in in round 7.

## N4 — their `(2♦)` as a Multi

**Closed 2026-08-23**; reopened for N4-KK, closed again 2026-08-27. The
lane's tree map is [one-notrump-multi.md](one-notrump-multi.md); every
verdict, build record and residue is in the archive:
[§N4 v7 + reader](archive/one-notrump-competitive-closed.md#n4--their-2-as-a-multi-shipped-2026-08-15--v7-seven-rounds-default-on-vs-bba-via-the-census),
[§N4e](archive/one-notrump-competitive-closed.md#n4e--the-floorless-weak-escape-shipped-default-on-2026-08-22-the-six-card-rung-five-refuted),
[§N4f](archive/one-notrump-competitive-closed.md#n4f--openers-balancing-seat-and-the-two-reading-knobs-measured-2-rounds-2026-08-22-nothing-ships-all-three-stay-opt-in),
[§N4-KK](archive/one-notrump-competitive-closed.md#n4-kk--the-kokishkraft-counter-a-whole-table-variant-shipped-default-on-2026-08-25).

| Knob | Default | Verdict |
| --- | --- | --- |
| `their.two_diamonds_multi` | disclosure; `bba-gen` derives it from the census | engages the counter; undeclared keeps the natural Transfer-Lebensohl leg, either/or |
| `competition.multi_kokish_kraft` | **on** (2026-08-25) | the Kokish–Kraft whole-table counter replaces v7: `X` at `hcp 8+`, neutral pass with a delayed takeout `X`, floorless minor transfers, `3♠` both minors GF, a penalty repeated double, the `4M` slam-try tier |
| `competition.multi_weak_escape` | `Some(6)` (2026-08-22) | responder's floorless weak escape; `Some(5)` refuted |
| `reading.their_multi_reading` | **on** (2026-08-16) | the floor reads their `2♦` as `{♥6+} ∪ {♠6+}` |
| `competition.multi_doubler_major`, `_notrump`, `_minimum_notrump` | **on** (2026-08-26/27) | opener's answer table to the K–K doubler ([handoff](multi-doubler-answer-handoff.md)) |
| `competition.multi_stopper_ask`, `multi_px_split`, `multi_balance`, `reading.their_multi_advance_reading`, `_double_reading`, `two_diamond_double` | off | refuted, below resolution, or trigger-gated (the BBA-reading v6 retrain missed the plain gate) |

The **mirror book** (2026-08-25) closed the campaign's mirror-read leak: the
leak was a frame flip, not a missing seat gate — undeclared opponents decode
with *our* book rebased to their seat, so `their.two_diamonds_multi` got
asserted about our own natural `2♦` overcall. `System::opponents` is a second
build of our system with `decision.their` cleared (`common::mirror_agreements`),
built only when something is declared; the K–K re-measure then gated 0
foreign. Narrowing the mirror to `two_diamonds_multi` is the one-line
reversal, and the `c5fbee11` re-anchor found no measured reason to take it. Three residues are **recorded, not
open**: `multi_balance` (~18 bd of reach per 230,400 — wants a sub-lane
harness or sd, not another seed), the two reading knobs (trigger-gated on a
retrain that passes the plain gate), and responder's natural minor
single-suiter (3.9% of BBA's hands, priced ≈ zero opposite 15–17). Reopening
N4 needs a new bucket, not a new seed.

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
the full measurement trails are in the archives, linked per row. Numbers are
the final pooled verdict, IMPs per board unless marked per fired; every row
since 2026-08-28 is 4.608M bd/arm/vul with all isolation gates 0 foreign
unless stated.

**Measured through the leak (rows before 2026-08-25).** Every N1 and N4 row
before the [mirror book](archive/one-notrump-competitive-closed.md#the-mirror-book--why-the-leak-was-not-a-seat-gate)
(`29f93561`) read their calls off our own counter-table. The verdicts stand:
each was a paired diff whose two arms shared the leak, and the `c5fbee11`
re-anchor put the mirror book **below anchor resolution** (`Competitive /
book / round-1` −34,664 → −34,729 plain).

| Package | Knob (default) | Status | Final pooled verdict | Trail |
| --- | --- | --- | --- | --- |
| census tool | — | **shipped** | read-only; picked N1 over the pre-census guess | [§census](#the-census--what-each-interference-call-actually-costs) |
| N1 Landy `(2♣)` counter | `their.two_clubs_landy` — a disclosure, not a knob | **SHIPPED 2026-08-14** | `plain wash \| PD win`, confirmed at 3× n: NV plain −0.0002 ±0.0013 / PD **+0.0032 ±0.0017**, vul plain +0.0003 ±0.0015 / PD **+0.0028 ±0.0019**. v1 lost all six cells on unauthored opener answers | [closed §N1](archive/one-notrump-competitive-closed.md#n1--the-landy-2-counter-shipped-default-on-2026-08-14) |
| N1b GF minor cues | `defense_2c_landy_cues` (**off**) | measured ×4 2026-08-14, **opt-in** | v4 `win \| wash` — the artifact row: NV plain **+0.0016 ±0.0010** / PD +0.0001, vul **+0.0014 ±0.0012** / −0.0000 | [closed §N1](archive/one-notrump-competitive-closed.md#n1--the-landy-2-counter-shipped-default-on-2026-08-14) |
| N1c club transfer + INV minors | `defense_2c_landy_transfer` (**on**, via the stack) | **SHIPPED DEFAULT-ON 2026-08-14** | ×2 seeds: plain **+0.0013 ±0.0007** NV / +0.0007 ±0.0008 vul, sd **+0.0018/+0.0011** | [closed §N1](archive/one-notrump-competitive-closed.md#n1--the-landy-2-counter-shipped-default-on-2026-08-14) |
| N1d/e/f cue repairs | `defense_2c_landy_cue_floor`, `_fit_answers`, `_competition` (all **on**) | **SHIPPED DEFAULT-ON 2026-08-14** | the package's first `win \| win`: ours-only NV plain **+0.00091 ±0.00052** / PD **+0.00077 ±0.00064**, vul **+0.00075 ±0.00058** / +0.00060; 8/8 sd cells positive. **N1d is the engine** (cue→X, +2.0…+5.1 PD/fired) | [closed §N1](archive/one-notrump-competitive-closed.md#n1--the-landy-2-counter-shipped-default-on-2026-08-14) |
| N1g Landy read-side wiring | `reading.their_landy_reading` (**on**) | **SHIPPED DEFAULT-ON 2026-08-14** | `plain wash \| PD win` ×3 seeds: NV PD **+0.00104 ±0.00097**, vul PD **+0.00112 ±0.00104**. **Isolation gate 0 foreign — the campaign's first** | [closed §N1g](archive/one-notrump-competitive-closed.md#n1g--the-read-side-wiring-shipped-default-on-2026-08-14) |
| N1h / N1i minor-rung re-pricing | `defense_2c_landy_low_minors`, `defense_2c_landy_hcp_rungs` (**both off**) | **both REFUTED 2026-08-15** | N1h `plain wash \| PD loss` (vul PD **−0.00081 ±0.00074**); N1i no CI-clear cell, all eight leaning negative. **N1d's cue floor is settled — do not probe it again** | [closed §N1h / N1i](archive/one-notrump-competitive-closed.md#n1h--n1i--the-minor-rungs-re-priced-both-refuted-both-opt-in) |
| N1j BBA-ladder counter + weak-`2♦` cap | `defense_2c_landy_bba`, `defense_2c_landy_weak_2d_cap` (**both on**) | **both SHIPPED DEFAULT-ON 2026-08-15** | ladder at a pre-pinned non-inferiority gate: `wash \| wash`, all 16 cells leaning positive. Cap: NV PD **+0.00037 ±0.00033**, vul **+0.00050 ±0.00035**, 0 foreign | [closed §N1j](archive/one-notrump-competitive-closed.md#n1j--the-bba-ladder-counter-shipped-default-on-2026-08-15) |
| N1 minor-transfer slam try | `competition.landy_minor_slam_answer` (**on**) | **SHIPPED DEFAULT-ON 2026-08-25** | 2.304M bd/arm/vul, 0 foreign, every DD/PD/sd cell a win: plain +5.56/+10.11, PD +7.11/+11.44 IMPs/fired NV/vul (n = 18/9) | [minor-transfer-slam.md](minor-transfer-slam.md) |
| N1l the doubler's own rebid ladder | `competition.landy_doubler_rebids` (**off**) | **measured 2026-08-28: mixed, stays off** | SD-PD **+0.523 none / −0.741 both** per fired; DD plain wins both (+2.365/+1.556) but the whole vulnerable win is the penalty `X`@155 (+9.196/fired) and the vulnerable loss is the constructive rungs, worst the `2NT` invite (−3.695 PD). Seed 1787917699, sha `ba003a30` | [§N1l](archive/one-notrump-competitive-landy.md#n1l--the-doublers-own-rebid-landy_doubler_rebids-measured-2026-08-28-mixed-stays-off); `scripts/ab-landy-doubler-rebids.sh` |
| N1l-flip the cut-down doubler ladder | `competition.landy_doubler_px` (**on**), `landy_doubler_white` (**off**) | **`px` SHIPPED DEFAULT-ON 2026-08-29; `white` not a win** | `px` plain **+0.0107 / +0.0142** NV/vul, PD +0.0039/+0.0061, SD-PD +0.0000/+0.0028 — `win \| win`; selection refuted (the `X` rung re-prices +7.554/+9.189 vs +7.489/+9.196). `white` is `win \| loss` NV (DD-PD −0.0091) and ≡ `px` vulnerable. Two caveats shipped open and later taken up by §N1-lia A: the `Pass`@0 catch-all cost −14,171 plain NV by shadowing a floor takeout double, and `comp:landy-penalty` published four-plus over it. Seed 1787942099, sha `de59ad86` | [§N1l-flip](archive/one-notrump-competitive-landy.md#n1l-flip--the-two-cut-down-arms-landy_doubler_px-shipped-default-on-2026-08-29--landy_doubler_white-not-a-win-stays-off); `scripts/ab-landy-doubler-flip.sh` |
| N1m opener's own rebid over their advance | `competition.landy_opener_px`, `landy_opener_rungs` (**both on**) | **SHIPPED DEFAULT-ON 2026-09-16 as a package; `px` alone refused** | `px + rungs` plain DD **+0.0140 / +0.0220**, DD-PD +0.0251/+0.0341, sd-plain +0.0004/+0.0030, **SD-PD +0.0095/+0.0135** — non-negative on all four scorers at both colours. `px` alone: DD +0.0099/+0.0249 but sd-plain **−0.0234** white — its catch-all "passed where the baseline bid" on 85.8% of divergence. Designed off `probe-landy-opener-oracle`: defending their major doubled wins every four-plus-trump bucket (+2.8…+8.1/bd), so `len(major, 4..)` is the whole gate. Seed 1789547524, sha `00c0421e` | [§N1m](archive/one-notrump-competitive-landy.md#n1m--openers-own-rebid-over-their-advance-landy_opener_px--landy_opener_rungs-shipped-default-on-2026-09-16-as-a-package); `scripts/ab-landy-opener.sh` |
| N1p an unlimited values double | `competition.landy_notrump_no_major` (**off**), `landy_major_jam` (**on**) | **`nt` measured loss 2026-08-30; the decoupled `4M` jam SHIPPED DEFAULT-ON 2026-08-30** | `nt` **−0.0124 / −0.0076** plain, **−0.0012** SD-PD; DD-PD +0.018/+0.020 is the auto-double artifact. `jam` standalone: an eight-of-eight sweep, DD plain +1.443/+1.611 per fired, SD-PD +1.558/+1.866 (seed 1788033942, sha `52fbc7c1`). The `X`@145 reads `points 8..9` because the ungated `3NT`@168 outranks it — no new slug. Re-measured as §N1-lia D, still off | [§N1p](archive/one-notrump-competitive-landy.md#n1p--an-unlimited-values-double-landy_notrump_no_major-loss-stays-off-landy_major_jam-shipped-default-on-2026-08-30); `scripts/ab-landy-notrump-shape.sh`, `ab-landy-major-jam.sh` |
| N1-lia A the doubler seat un-shadowed | `landy_doubler_catchall` (**false**), `landy_doubler_three_honors`, `_three_small` (**on**) | **SHIPPED DEFAULT-ON 2026-08-30** | every adjacent pair a plain win at both colours (nocatch +0.0036/+0.0014, hon +0.0008/+0.0008, cells +0.0104/+0.0118), every sd tie-break positive; PD negative throughout is the pre-registered doubling artifact. Build finding: the catch-all deletion was a **silent no-op** until the tables moved from `Pattern::after` guards to exact `Pattern::node`s (`Trie::resolve_floored`'s single fall-through). Seed 1788088630 | [§N1-lia A](archive/one-notrump-competitive-landy.md#package-a--the-full-ladder-shipped-default-on-2026-08-30-landy_doubler_catchall-now-false-landy_doubler_three_honors-and-landy_doubler_three_small-both-true); `scripts/ab-landy-lia-doubler.sh` |
| N1-lia B Lia's ladder | `defense_2c_landy_lia` (**off**) | **misprobed, rebuilt as true Lia, MEASURED LOSS ×2 (lia2 2026-09-01, lia3 2026-09-02)** | lia2 plain **−0.0077 / −0.0059**, PD −0.0127/−0.0172 (seed 1788264406): the club leg wins (+0.0126/+0.0137), the diamond leg loses (−0.0201/−0.0181). lia3 (refined on that forensic) lost 4/4: plain **−0.0056 / −0.0254**, SD-PD −0.0158/−0.0342 (seed 1788290089); the weak 4-7 band is the both-vul catastrophe and the named floor class is **phantom suits** (floored bids on ≤4 cards, +35,346/+25,145 against the baseline's own floor). Lane parked behind the general new-suit veto rail, which was itself refuted — the park is void; lia4 stands alone | [§N1-lia B](archive/one-notrump-competitive-landy.md#package-b--defense_2c_landy_lia-misprobed-redefined-in-place-measured-a-loss-then-refined-on-its-own-forensic-2026-09-01--ab-owed); `scripts/ab-landy-lia2.sh`, `ab-landy-lia3.sh`, `examples/probe-layer-replay` |
| N1-lia/rail the envelope-gated new-suit veto | `decision.instinct.new_suit_veto` (**off**) | **MEASURED AND REFUTED 2026-09-02** | plain DD **−0.0212 ±0.0047** (none) / **−0.0164 ±0.0057** (both), PD a wash, seed 1788352713, 204,800 bd/arm/vul. The general fix does not solve the local problem | [ai-bidder/new-suit-veto.md](ai-bidder/new-suit-veto.md) |
| N1-lia C Texas over the counter | `landy_texas` (**on**), `landy_texas_floor` (**10**) | **SHIPPED DEFAULT-ON 2026-08-31** | eight-of-eight sweep, plain **+0.616 / +0.711** per fired, PD +0.816/+0.996, sd-lead +0.220/+0.352; fires on 0.03%/0.02%. **Mechanism reversed from the design's**: 96% of divergence is the same contract from the other seat — DD sees right-siding's *lead direction*, not its concealment. Seed 1788181796 | [§N1-lia C](archive/one-notrump-competitive-landy.md#package-c--landy_texas-shipped-default-on-2026-08-31-an-eight-of-eight-sweep); `scripts/ab-landy-texas.sh` |
| N1-lia D `landy_notrump_no_major` on A's winner | `competition.landy_notrump_no_major` (**off**) | **measured non-win 2026-09-01** | plain DD +0.0077/+0.0075 but plain SD −0.0061/−0.0056 and SD-PD −0.0011/+0.0000; the DD−SD seam (+0.014) is bigger than the knob. The gate bundles opposite-signed halves: `min major ≥ 3` is worth +3.1…+5.9/fired, `≤ 2` loses. Seed 1788191041 | [§N1-lia D](archive/one-notrump-competitive-landy.md#package-d--landy_notrump_no_major-re-measured-on-as-winner); `scripts/ab-landy-nt-remeasure.sh` |
| N1q the two-level majors sorted by strength | `defense_2c_landy_strength_majors` (**on** 2026-09-20), `_strength_doubles`, `landy_recue_signoff`, `_strength_nv_invite` (**on** 2026-09-21) | **SHIPPED DEFAULT-ON** on the third arm, five runs | run 3 `str` vs base plain **+0.0078 / +0.0050**, PD +0.0070/+0.0026, sd-lead +0.0096/+0.0075 — `win \| win` both colours. Runs 1–2 lost/washed on three families of floor phantoms at unauthored nodes; the third arm's `Pass` rails fixed them. Run 4: `rail` plain +0.0012/+0.0009, `strx` +0.0000/+0.0001 with sd 4/4 positive; run 5 `nv2` plain +0.0018 ±0.0003 both-vul, NV 0 divergent. Census: the weak 5-4 band is 1.77% of all boards, the INV+ 4-4 band 2.67% | [§N1q](archive/one-notrump-competitive-landy.md#n1q--the-two-level-majors-sorted-by-strength-not-by-shortness-defense_2c_landy_strength_majors-shipped-default-on-2026-09-20-on-the-third-arm-_doubles-and-the-landy_recue_signoff-rail-on-since-2026-09-21); `scripts/ab-landy-strength.sh`, `ab-landy-strength-residue.sh` |
| N1r step 0 `3NT` vs the values `X` by colour | `landy_notrump_no_major_favourable` (**on** 2026-09-22) | **SHIPPED DEFAULT-ON at favourable only** | DD plain / DD PD / sd plain / sd PD: **ew +0.0147 / +0.0127 / +0.0042 / +0.0023** (all CI-clear); ns −0.0107/−0.0144/−0.0206/−0.0235; none and both lose on sd. Face-gated (`Rules::face`), so a colour term never drifts the `X`'s reading. Seed 1789977169, control `ab8bc884` | [step 0](archive/one-notrump-competitive-landy.md#step-0-verdict--3nt-vs-the-values-x-all-four-colours-landy_notrump_no_major_favourable-shipped-default-on-2026-09-22); `scripts/ab-landy-nt-vs-x.sh` |
| N1r step 0b the misfit-only `3NT` gate | none — dropped | **MEASURED LOSS 2026-09-22**, code removed | vs base at both +0.0002 / −0.0065 / −0.0017 / −0.0069; vs the full gate at ew −0.0100/−0.0118/−0.0022/−0.0034 (dominated). 64% of divergence at both is opener's continuation under the widened `X` reading — the reading tax of any narrowed `3NT` | [step 0b](archive/one-notrump-competitive-landy.md#step-0b-verdict--the-misfit-only-3nt-gate-measured-loss-2026-09-22-knob-landy_notrump_no_major_misfit-and-runner-dropped-nothing-shipped) |
| N1r row 1 the splinter seat | `landy_splinter_rebids` (**on**), `landy_splinter_tails` (**on**), `landy_splinter_stopper` (**off**) | **census 2026-09-23; arm 1 and tails SHIPPED DEFAULT-ON 2026-09-23; arm 2 measured non-win** | census (`probe-landy-splinter-oracle`): wastage is the wrong axis, slam dead by frequency, the leak is responder's floor `4♠` over opener's `3NT`. `rebids` **+0.0055 / +0.0078 / +0.0053 / +0.0072** none, **+0.0039 / +0.0047 / +0.0035 / +0.0042** both. `tails` +0.0030/+0.0024/+0.0009/+0.0003 none, +0.0006/+0.0007/+0.0003/+0.0004 both (round 2). `stopper` DD +0.0020/+0.0019, sd −0.0003/−0.0008 — a declare-less-`3NT` knob, sd-lead arbitrates. Seed 1789977169 | [census](archive/one-notrump-competitive-landy.md#row-1-step-0-verdict--the-splinter-census-2026-09-23-wastage-is-the-wrong-axis-the-leak-is-responders-floor-4-over-openers-3nt), [arm 1](archive/one-notrump-competitive-landy.md#row-1-arm-1-verdict--responders-rebids-over-openers-answer-landy_splinter_rebids-shipped-default-on-2026-09-23), [arm 2](archive/one-notrump-competitive-landy.md#row-1-arm-2-verdict--openers-3nt-needs-a-double-stopper-landy_splinter_stopper-measured-non-win-2026-09-23-stays-opt-in-default-off), [tails](archive/one-notrump-competitive-landy.md#row-1-tails-verdict--their-action-over-the-passed-3nt-landy_splinter_tails-shipped-default-on-2026-09-23); `scripts/ab-landy-splinter-{rebids,stopper,tails}.sh` |
| N1r row 9 the values doubler's rebids over their `(2M)` runout | `landy_doubler_game` (**on** 2026-09-23) | **SHIPPED DEFAULT-ON** at favourable | `game` vs base at ew **+0.0085 / +0.0117 / +0.0078 / +0.0103**, every CI clear; other colours byte-identical by face gate. `3NT`@150 `points(10..)`, authored five-card `3m` so `X – 3m` reads `8..9`. Seed 1790142988, control `9d6a51ae` | [row 9](archive/one-notrump-competitive-landy.md#row-9-verdict--the-values-doublers-rebids-over-their-2m-runout-landy_doubler_game-shipped-default-on-2026-09-23); `scripts/ab-landy-doubler-game.sh` |
| N1r rows 2 and 5 | none — `examples/probe-landy-notrump-stopper` | **CENSUS 2026-09-23, both DEAD** | row 2 (`3♦` for the `3NT`@168 hand missing a stopper): the scheme vs live **−0.0006 / −0.0011**; opener covers the missing major on 88%. Row 5 (quantitative `4NT`): responder 16–17 on 200 / 52 boards in 4.6M, ≈ 0 | [row 2](archive/one-notrump-competitive-landy.md#row-2-step-0-verdict--the-3nt168-stopper-census-2026-09-23-dead-at-step-0-3nt-is-the-right-game-even-without-the-stopper), [row 5](archive/one-notrump-competitive-landy.md#row-5-step-0-verdict--the-quantitative-4nt-census-2026-09-23-dead-at-step-0-responder-16-never-bids-3nt) |
| N1s `1NT (2♣) 3♦` = GF Wilkosz | `landy_wilkosz` (**on** 2026-09-24) | **SHIPPED DEFAULT-ON** | census (`probe-landy-wilkosz-oracle`) `fit` +0.00061 / +0.00058 plain none / both, ceiling +0.0010, favourable loses to the values `X` → face-gated off there. A/B **+0.0005 / +0.0007 / +0.0003 / +0.0004** none, **+0.0004 / +0.0006 / +0.0002 / +0.0003** both, every CI clear, 1,483 / 991 fired. Seed 1789977169, control `375066af` | [§N1s](archive/one-notrump-competitive-landy.md#n1s--1nt-2-3--game-forcing-wilkosz-census-2026-09-24-landy_wilkosz-shipped-default-on-2026-09-24); `scripts/ab-landy-wilkosz.sh` |
| N4 their `(2♦)` as a Multi | `their.two_diamonds_multi` — disclosure | **SHIPPED 2026-08-15, v7 of seven rounds** | v7 vs base ×3 seeds, owned: NV `plain wash \| PD win` (+0.00100 ±0.00067), vul plain **+0.00061 ±0.00056**; both-vul pool `win \| win`. Every raw headline was 60–70% foreign — verdicts are owner-split | [§N4](archive/one-notrump-competitive-closed.md#n4--their-2-as-a-multi-shipped-2026-08-15--v7-seven-rounds-default-on-vs-bba-via-the-census); [v1–v6](archive/one-notrump-competitive-closed.md#n4--measurement-rounds-v1v6) |
| N4 residue — Multi reader / stopper ask | `reading.their_multi_reading` (**on**), `competition.multi_stopper_ask` (**off**) | reader **SHIPPED DEFAULT-ON 2026-08-16**; ask **REFUTED as a default** | reader `plain wash \| PD win` ×3 seeds — −29 plain / **+643 PD** over 1.3824M boards, 0 foreign. Both stopper modes landed on the artifact row | [§N4 residue](archive/one-notrump-competitive-closed.md#n4-residue--reader-shipped-stopper-ask-stays-opt-in-measured-2026-08-16) |
| N4e floorless weak escape | `competition.multi_weak_escape` (**`Some(6)`**) | **SHIPPED DEFAULT-ON 2026-08-22; `Some(5)` REFUTED** | `plain wash \| PD win` ×2 seeds: pooled plain +0.00028 ±0.00039, **PD +0.00063 ±0.00049**, **SD-PD +0.00052 ±0.00049**. `five` is the doubling-artifact row and CI-clear worse than `six` on PD (−0.00095 ±0.00048). Two rounds lost to the systems-on strip leaking the Multi table into the 1NT-overcall lane | [§N4e](archive/one-notrump-competitive-closed.md#n4e--the-floorless-weak-escape-shipped-default-on-2026-08-22-the-six-card-rung-five-refuted) |
| N4b `(2♦)` diamond penalty double | `competition.two_diamond_double` (**`None`**) | **measured 2026-08-15 — sweep NULL, opt-in** | all 28 raw cells CI-clear positive and **all of it a leak** (84.9% foreign); owned subset has no CI-clear cell. **The alert is what makes a gate a reading** | [closed §N4b](archive/one-notrump-competitive-closed.md#n4b--the-2-diamond-penalty-double-built-2026-08-15-sweeping) |
| N4f opener's balancing seat + the two Multi reading knobs | `competition.multi_balance`, `reading.their_multi_advance_reading`, `_double_reading` (**all off**) | **measured ×2 rounds 2026-08-22 — nothing ships** | 24 pairs, 0 foreign. `balance` is below resolution (~18 bd of reach per 230,400); `advance`'s false `♥3+ & ♠3+` claim was removed and the read is **inert** (6 boards in 1.84M) | [§N4f](archive/one-notrump-competitive-closed.md#n4f--openers-balancing-seat-and-the-two-reading-knobs-measured-2-rounds-2026-08-22-nothing-ships-all-three-stay-opt-in) |
| N4g BBA-reading v6 retrain | `american_v6_their` + both Multi readers (**opt-in**) | **measured 2026-08-23 — N4 plain gate missed** | owned `(2♦)` slice −11 plain / +845 PD over 819,200 accepted deals: plain −0.000013 ±0.000468, not the pre-registered improvement | [retrain round](archive/one-notrump-competitive-closed.md#the-retrain-trigger-fired-2026-08-23--pd-gain-plain-target-missed-no-ship) |
| N4-KK Kokish–Kraft whole-table counter | `competition.multi_kokish_kraft` (**on**) | **SHIPPED DEFAULT-ON 2026-08-25** | fresh seed after the mirror book (1787615025, sha `f2ecb3c6`, 230,400 bd/arm/vul): **0 foreign at both vuls**; both-vul `win \| win` plain **+0.0019 ±0.0013** / PD **+0.0023 ±0.0017**, NV `wash \| wash`, sd agreeing in all four cells. The first run was 55% foreign and its dumps are dead | [§N4-KK](archive/one-notrump-competitive-closed.md#n4-kk--the-kokishkraft-counter-a-whole-table-variant-shipped-default-on-2026-08-25) |
| N4-KK `P`/`X` information split | `competition.multi_px_split` (**off**) | **MEASURED LOSS 2026-08-27** | per fired NV plain **−0.980** / PD **−1.780**, both-vul **−1.861 / −2.083**; the surface came in ~40× thinner than designed (50/36 divergent boards of 230,400). Seed 1787804916, sha `f44b73b9` | [§px split](archive/one-notrump-competitive-closed.md#the-px-information-split--competitionmulti_px_split-measured-loss-2026-08-27-stays-default-off) |
| N4-KK opener's answer to the doubler | `multi_doubler_major`, `_notrump`, `_minimum_notrump` (**all on**) | **SHIPPED DEFAULT-ON 2026-08-26/27**, each 4/4 cells | the both-vul PD cell of `multi_doubler_major` shipped open; the later two won every cell and completed the `2♠` leg's ladder | [multi-doubler-answer-handoff.md](multi-doubler-answer-handoff.md) |
| N4-mirror our `2♦` overcall of *their* 1NT | `natural_overcall_hcp_floor` = 8, `natural_overcall_advance_enabled` (**on**) | **handed over 2026-08-23; M1+M2 SHIPPED there the same day** | 204.8k bd/arm/vul × 2 seeds, 0 foreign: plain DD +0.0011/+0.0068 and +0.0041/+0.0096, **SD-PD +0.0086/+0.0164 and +0.0100/+0.0202, every cell CI-clear** | [defensive-overcalls.md](defensive-overcalls.md#defense-to-their-1nt--the-1nt-2-mirror-panel-forensic-2026-08-23) |
| N3 `(3♣)`–`(3♠)` preempt of our 1NT | `nt_high_overcall_responses` (**on**), `nt_high_overcall_3nt_stopper` (**off**), `nt_3c_transfers` (**off**) | **SHIPPED DEFAULT-ON 2026-08-18** | owned plain **+0.00208 ±0.00126** NV / **+0.00293 ±0.00160** vul, PD +0.00079/+0.00180, zero negative cells in sixteen. The private `3NT` bit shipped off; the BBA-style double continuation refuted and removed | [§N3 rounds](archive/one-notrump-competitive-closed.md#n3--measurement-rounds) |
| N3 opener's answer to the takeout `X` | `nt_high_overcall_x_major_at_four` (**on**), `_x_leave_in` (**on**), `_x_leave_in_three` (**off**) | fit rung **SHIPPED 2026-08-19**; leave-in **SHIPPED 2026-08-20**; honor half **REFUTED** | fit rung 4/4 CI-clear (+1.14/+1.69 per fired). Leave-in `length` CI-clear in all eight cells, plain +0.0071/+0.0104 (+2.38/+3.41 per fired), replicated on a fresh seed with every suit positive. `three` sd-lead **−2.44/−2.99 per fired** | [§N3 rounds](archive/one-notrump-competitive-closed.md#n3--measurement-rounds) |
