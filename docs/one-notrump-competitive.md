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
    ab-results/anchor/2026-08-17-53a3c254/american-none \
    --dd-cache ab-results/anchor/dd-cache.json
# add --bucket "2♣" --show 8 for the worst boards of one bucket
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

### 2026-08-18 pre-N3 baseline, anchor `2026-08-17-53a3c254`, seed 1783375064, 204,800 boards/vul

We open 1NT on **6.5%/6.7%** of boards; RHO contests **12.4%/10.4%** of those
(NV/vul) — so a contested 1NT is **0.80%/0.69% of all boards**.

The three-level suits are split per RHO suit since 2026-08-18 (the N3
deliverable); `4+` is `3NT` and everything above it, still one floor-only bucket.

| RHO | boards (NV+vul) | plain total | plain/bd | PD/bd NV | PD/bd vul |
| --- | --- | --- | --- | --- | --- |
| `2♦` Multi | 794 | −245 | −0.31 | +0.15 | +0.54 |
| `2♠` Muiderberg | 430 | −219 | −0.51 | −0.16 | +0.36 |
| `2♣` Landy | 551 | −213 | −0.39 | −0.10 | +0.42 |
| **`3♣` preempt** | 100 | **−192** | **−1.92** | **−1.78** | **−0.62** |
| `X` Woolsey | 364 | −183 | −0.50 | +0.51 | +0.74 |
| **`4+`** (`3NT` and up) | 43 | −89 | −2.07 | −1.33 | −1.74 |
| `2♥` Muiderberg | 393 | −77 | −0.20 | +0.08 | +1.07 |
| **`3♥` preempt** | 85 | −75 | −0.88 | +0.13 | −1.70 |
| **`3♦` preempt** | 89 | −43 | −0.48 | +0.53 | −0.23 |
| **`3♠` preempt** | 88 | −35 | −0.40 | +0.50 | +0.89 |
| `2NT` unusual | 118 | +5 | +0.04 | −0.23 | +0.48 |
| **all contested** | 3055 | −1366 | −0.61 / −0.26 | +0.01 | **+0.43** |
| **uncontested 1NT** | 23868 | — | **+0.13 / +0.01** | — | — |

At this pre-ship snapshot the four three-level suits are 362 boards and −345
plain between them — the family was the top loser, and `3♣` alone out-cost
every two-level call per board by a factor of three. N3's post-ship fresh-seed
census is recorded in [§N3](#post-ship-fresh-seed-anchor-check-2026-08-18).

**Three findings.**

1. **The lane's whole headroom is ~0.004 IMPs/bd.** Contested costs
   −0.74 NV / −0.27 vul relative to *uncontested*, on 0.80%/0.69% of boards.
   Nothing here closes an anchor gap; this is hygiene and disaster removal at
   the standard ship gate, as scoped.
2. **Contested 1NT is above the instinct anchor's board average**, not a leak —
   −0.61/−0.26 against −0.90/−1.09. The 1NT opening is one of our better boards
   even when contested.
3. **The pre-N3 three-level lane is where both scorers lose.** `3♣` is −1.92
   plain/bd with PD negative at both vulnerabilities, `4+` worse per board on
   43 boards, and `3♥` swings PD −1.70 vul; only `3♠` is PD-positive on both.
   With the shipped Landy package present, `2♣` is −0.39 plain pooled and
   −0.10/+0.42 PD. `X` remains fine (−0.28 plain vul, PD +0.74), and `2♦` is
   mild (PD +0.15/+0.54). N3 authors the four three-level suits; `4+` stays
   floor-only, and inside it **`(4♥)` alone is −118 plain / −126 PD** (the
   worst-board dump's own tally; the rest of `4+` nets positive) — but the
   floor offers no `X` over `(4x)` at all, so see §N3's flagged list.

### Historical mechanism — why `2♣` lost before N1 shipped

The analysis below is from the starting `2026-08-12-ea2cde9-dirty` snapshot,
where `2♣` carried the largest loss (−406 plain IMPs, −0.74/bd). It motivated
N1; the current census above includes that package's shipped repairs.

Before N1, over their `2♣` we played a **systems-on rebase**
([lebensohl.rs:388-405](../src/bidding/american/competition/lebensohl.rs)):
their `2♣` was stripped to a Pass and our whole uncontested response structure
went live, with `X` transplanted onto the stolen `2♣` Stayman
([lebensohl.rs:416-425](../src/bidding/american/competition/lebensohl.rs)).
Against a *natural* club overcall that is sound and standard — `2♣` is the one
overcall that costs no space.

Against **Landy** it was actively bad. The worst boards showed the structure
firing into a hand that had just shown both majors:

```text
us:  - 1NT 2♣ 2NT - 3♦ 3♠ 4♦ - 4♠ X - - -      [−18 IMPs]
us:  1NT 2♣ 2♦ 3♣ X 3♠ - 4♠ X - - -            [−10 IMPs]
us:  1NT 2♣ 2NT 3♥ X - 3NT - - -               [−10 IMPs]
```

`2♦` is a Jacoby transfer **to hearts** — one of the two suits they hold. `2NT`
and `2♠` are the minor transfers, pure constructive asks that hand them a free
run at their fit. `X` asks for a four-card major against a hand holding both.
Two of the eight worst boards end in `4♠` doubled.

### Current `2NT` reading — small-n wash

The same 118 boards now total +5 plain IMPs (+0.04/bd), with PD −0.23/+0.48
NV/vul. The CI is enormous and the signs disagree. The starting snapshot's
forensic pattern was that BBA **doubled** their minors and we bid on:

```text
us:  1NT 2NT X 3♦ - - -
bba: 1NT 2NT X 3♦ - - X - - -
```

The re-anchor does not replicate a loss, so N6 stays parked.

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

## Package queue — ranked by the census

| # | Package | Knob | State |
| --- | --- | --- | --- |
| **[N1](archive/one-notrump-competitive-closed.md#n1--the-landy-2-counter-shipped-default-on-2026-08-14)** | **Landy `(2♣)` counter + N1c/d/e/f stack** | `their.two_clubs_landy` | **SHIPPED 2026-08-14** (base `wash \| win`, stack `win \| win`); was the top loser, both scorers |
| N1g | Landy **read-side** wiring — their `2♣` = majors in the floor's envelopes | `reading.their_landy_reading` | **SHIPPED DEFAULT-ON 2026-08-14** (`plain wash \| PD win` ×3 seeds, isolation gate 0 foreign); see [closed §N1g](archive/one-notrump-competitive-closed.md#n1g--the-read-side-wiring-shipped-default-on-2026-08-14) |
| N1h / N1i | Landy counter's minor rungs re-priced — a point lower, then regraded on `hcp` | `defense_2c_landy_low_minors`, `defense_2c_landy_hcp_rungs` | **both REFUTED 2026-08-15, both opt-in; lane closed.** `cue ← X` negative in both, so N1d's cue floor is settled — see [closed §N1h / N1i](archive/one-notrump-competitive-closed.md#n1h--n1i--the-minor-rungs-re-priced-both-refuted-both-opt-in) |
| **N1j** | **BBA-ladder counter** — the anchor-aligned table, replacing the stack — **+ the weak-2♦ cap** | `defense_2c_landy_bba`, `defense_2c_landy_weak_2d_cap` | **both SHIPPED DEFAULT-ON 2026-08-15** — the ladder at its pinned non-inferiority gate (`wash \| wash`, all eight DD cells leaning positive), the cap at the standard gate (`plain wash \| PD win`, 0 foreign); see [closed §N1j](archive/one-notrump-competitive-closed.md#n1j--the-bba-ladder-counter-shipped-default-on-2026-08-15) |
| N2 | Muiderberg `(2♠)` calibration | — | Current census −0.51/bd, PD −0.16/+0.36 NV/vul; **pre-fix census by response run 2026-08-15 (§N2)**: `X` wins, the `2NT` relay and Pass lose, opener bids `3NT` over the relay's minor sign-off 16/18. **Cause corrected 2026-08-16**: not the unlimited reading (built and measured as `strength_ceilings`, the node does not move) but `opener_forced_past_invitation`, which forces to game off any three-level suit bid — **N2e, now SHIPPED default-on as `instinct.forcing_ceiling_read`** (3 seeds, 12/12 cells positive, +0.0001 plain / +0.0003 PD). N2a stays parked (it would shadow the floor that now handles this seat); N2c/N2d queued and re-priced below. BBA's plain Lebensohl earns nothing at table B |
| **N3** | **`(3♣)`–`(3♠)` overcalls of our 1NT** — responder's one call and opener's one answer | `nt_high_overcall_responses` (**on**), `nt_high_overcall_3nt_stopper` (**off** — no gate), `nt_3c_transfers` (off) | **SHIPPED DEFAULT-ON 2026-08-18** — owned plain **+0.0021/+0.0029 IMPs/board** (NV/vul, both CI-clear), PD +0.0008/+0.0018, single-dummy agreeing in sign on all four cells and **zero negative cells in sixteen readings**. The BBA-style double continuation was later **refuted on every scorer and removed**: its `4♦` placement missed too many games. Pre-ship census: 362 bd / −345 plain; post-ship fresh seed: 410 bd / −273 plain / −186 PD; current same-deal census after the answer refinements: **410 bd / −192 plain / −73 PD** (or 448 bd / −235 / −118 including `4+`). The anchor rows rank and attribute; the isolated A/B remains the verdict. Their calls are **natural 7-card preempts**, so this is an ordinary competitive scheme. See §N3 |
| N4 | Multi `(2♦)` — the Transfer leg re-keyed for a Multi, the double family and relay authored to the seat; **v7 = BBA's own second-turn structure minus its PD-refused game bids** (takeout X of the resolved major, `hcp 6+` values double) | `their.two_diamonds_multi` (disclosure) | **SHIPPED 2026-08-15 (seven rounds, v7 pooled 3 seeds): NV `plain wash \| PD win` (+0.00100 ±0.00067), vul plain +0.00061 ±0.00056 \| PD +0.00061 ±0.00069; both-vul pool win\|win; paired vs v4 better on 3 of 4 cells** — census default in `their_2d_multi` + `vs_bba_agreements`; see [§N4](#n4--their-2-as-a-multi-shipped-2026-08-15--v7-seven-rounds-default-on-vs-bba-via-the-census) |
| N4 residue | Read their disclosed Multi as exactly `6+♥ ∪ 6+♠`; test the honest `3♠` stopper ask after the correction to spades | `reading.their_multi_reading`, `competition.multi_stopper_ask` | **Reader SHIPPED DEFAULT-ON 2026-08-16** (`plain wash \| PD win`, 0 foreign). **Stopper ask REFUTED as a default** (`plain win \| PD wash` for both continuations); `FitSearch` and `OpenerPlaces` remain explicit opt-ins, default `Off`. See §N4 residue below |
| N4b | `(2♦)` **diamond penalty double** — the cheap half of N4, no disclosure needed | `two_diamond_double` | **measured 2026-08-15 — sweep NULL, stays opt-in.** Raw headline was CI-clear positive in all 28 cells but **84.9% foreign** (isolation gate failed); owned subset is a wash. Spun off a real candidate: reading *their* double of a `2♦` overcall as diamonds — see [closed §N4b](archive/one-notrump-competitive-closed.md#n4b--the-2-diamond-penalty-double-built-2026-08-15-sweeping) |
| N5 | Complete Jacoby, re-measure | `competition_over_transfer` | default-off on a measured loss *while missing its two most-fired cells* — a half-built loss, resumable |
| N6 | `(2NT)` penalty discipline | `uvu_encircle` et al. | Current +0.04 plain/bd pooled, PD −0.23/+0.48 NV/vul, n=118 — no replicated loss. Mechanism remains priced: **BBA doubles 46.7%** and cues `3♣` for both majors ([reference](ai-bidder/bba-1nt-counter-defense.md)) |
| N7 | Absent responses contested | new | Puppet `3♣`, `3♦`, splinters, `3NT`, Texas, `4NT` — rarest in the system |
| N8 | Delete `1NT - 2NT - 3♣ - 3♦ -`'s `pass_out` node — the redundant half of the pair | knobless (book node) | **Inherited 2026-08-18** from the closed [authored-reading campaign](authored-reading-handoff.md)'s Phase 2 row. Probed 2026-08-17: with the node removed the auction falls to the root fallback and the floor offers **`P` alone** on every hand at both vulnerabilities, so the node buys nothing — but its twin `1NT - 2♠ - 2NT - 3♣ -` blasts **`4♣` 1.200 over `P` 0.000** (the floor's support-raise fires on partner's *sign-off*, never reading the `points 0..9` cap the same reading supplies). The pre-registered rule was "any hand still blasts → leave both", so both stayed. This is the deletion of the *redundant* one alone, and it owes its own arm; the twin's real repair is the floor-side settle rail ([dutch-system.md](dutch-system.md#the-wj-floor-campaign--bbas-polish-club-as-dutchs-teacher)), not a book node |

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

### The pre-ship census, decomposed (anchor `2026-08-17-53a3c254`, 204,800 bd/vul)

The `3+` bucket split is now the probe's own (`probe-1nt-interference` labels
three-level suits per suit since 2026-08-18), so the table in §census above is
the deliverable; the worst cells per RHO suit, from the `--show 400` dumps:

| RHO | bd | plain | PD | worst cells (RHO × our call) |
| --- | ---: | ---: | ---: | --- |
| `3♣` | 100 | −192 | −120 | `3♠` 25 bd −86 (opener passes / 3NT over a 6-carder), Pass 41 bd −61 (4441 9–11 with no call), `3♦` 9 bd −38 |
| `3♥` | 85 | −75 | −73 | `X` 27 bd −23 / **−74 PD** (X on 6–7 HCP, opener `4♠`), `3NT` 11 bd −23 (singleton in their suit) |
| `3♦` | 89 | −43 | +14 | `3♥` 19 bd −65 (`3♥ - - -` passed out on 10–11 HCP), `X` 6 bd −52 (6–8 HCP) |
| `3♠` | 88 | −35 | +62 | Pass 39 bd −65 / +36; floor blasts `6♣`/`5♦` on 8–11 HCP; `X` +57, `4♥` +58 (the winners) |

### Post-ship fresh-seed anchor check (2026-08-18)

The shipping arms in `ab-results/anchor-confirm/2026-08-18-9cfb464b`, fresh seed
`1787064872`, 204,800 boards/vulnerability, replay 100.00% with 0 mismatches.
At the shipped defaults (responses on, private `3NT` stopper gate off,
`(3♣)` transfers off), the N3 buckets are:

| RHO | bd | plain | PD | plain/bd | PD/bd |
| --- | ---: | ---: | ---: | ---: | ---: |
| `3♣` | 140 | −170 | −163 | −1.21 | −1.16 |
| `3♥` | 105 | −11 | +45 | −0.10 | +0.43 |
| `3♦` | 93 | +29 | +48 | +0.31 | +0.52 |
| `3♠` | 72 | −121 | −116 | −1.68 | −1.61 |
| **all four** | **410** | **−273** | **−186** | **−0.67** | **−0.45** |

This is an attribution check, not another treatment A/B: the swing is the
whole board, the mirrored table is present, and the seed differs from the
pre-ship snapshot. In particular, `3♠` moved from PD-positive on the series
seed to −1.44/−1.76 PD per board NV/vul here, while the isolated package A/B
was positive on both scorers. Do not subtract the two anchor totals to estimate
N3's value; the owned `stop ↔ base` A/B below remains the causal ship evidence.

### Current-HEAD paired `(3♣+)` census (2026-08-21)

The current shipping arms in
`ab-results/anchor-confirm/2026-08-21-1e9a47e2`, HEAD `1e9a47e2`, replay the
same seed `1787064872` and 32 × 6,400 boards/vulnerability as the 2026-08-18
snapshot.  This time `(3♣+)` includes the four three-level suits **and** the
probe's `4+` bucket (`3NT` and higher).  Per-bucket means carry their 95% CI
half-widths:

| RHO, NV | bd | plain | PD | plain/bd | PD/bd |
| --- | ---: | ---: | ---: | ---: | ---: |
| `3♣` | 70 | −76 | −70 | −1.086 ±1.449 | −1.000 ±1.695 |
| `3♦` | 47 | +5 | +15 | +0.106 ±1.498 | +0.319 ±1.744 |
| `3♥` | 47 | −23 | −5 | −0.489 ±1.278 | −0.106 ±1.425 |
| `3♠` | 34 | −50 | −31 | −1.471 ±2.014 | −0.912 ±2.120 |
| `4+` | 23 | −22 | −27 | −0.957 ±2.784 | −1.174 ±3.025 |

| RHO, vulnerable | bd | plain | PD | plain/bd | PD/bd |
| --- | ---: | ---: | ---: | ---: | ---: |
| `3♣` | 70 | −76 | −66 | −1.086 ±1.796 | −0.943 ±2.082 |
| `3♦` | 46 | +55 | +67 | +1.196 ±1.980 | +1.457 ±2.252 |
| `3♥` | 58 | +30 | +59 | +0.517 ±1.593 | +1.017 ±1.797 |
| `3♠` | 38 | −57 | −42 | −1.500 ±2.485 | −1.105 ±2.806 |
| `4+` | 15 | −21 | −18 | −1.400 ±4.673 | −1.200 ±4.974 |

| `(3♣+)` | bd | plain | PD | plain/bd | PD/bd | Δ plain vs 2026-08-18 | Δ PD vs 2026-08-18 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| NV | 221 | −166 | −118 | −0.75 | −0.53 | +28 | +42 |
| vulnerable | 227 | −69 | 0 | −0.30 | 0.00 | +53 | +71 |
| **pooled** | **448** | **−235** | **−118** | **−0.52** | **−0.26** | **+81** | **+113** |

The old arms reproduce the published four-suit total exactly (410 boards,
−273 plain / −186 PD); adding their unchanged `4+` row gives the paired
`(3♣+)` baseline of 448 boards, −316 plain / −231 PD.  The current score is
therefore −0.71 → −0.52 plain and −0.52 → −0.26 PD per board.  `4+` itself is
unchanged (38 boards, −43 / −45); the movement is entirely in the authored
three-level-suit buckets.

This remains an attribution census, not a causal A/B: each number is the whole
board's swing and includes the mirrored table.  It updates our current score
against the lane but does not replace the isolated A/Bs that shipped the N3
package and its answer refinements.

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
| `3NT` | 140 | `author_direct_3nt` — rides `direct_3nt_stopper` | to play |
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
  15–17: the shown major at its cheapest level with four (140), jumped to game
  with `points(17..)` (150), `3NT` 130 on a stopper, the three-card tolerance
  (30 at the three level / 25 at the four), catch-all `3NT` 15. **No penalty
  pass in v1** — BBA sits over some doubles and perfect defense may want it;
  logged as residue, a v2 knob.

### The `(3♣)` transfer variant (`nt_3c_transfers`)

`(3♣)` is the one three-level overcall that leaves steps below `3NT`. The arm
replaces the natural three-level rows in the `(3♣)` instance only:

| call | weight | constraint | reading |
| --- | ---: | --- | --- |
| `3♦` | 180 | `len(♥, 5..) & points(9..)` | transfer to ♥, INV+ |
| `3♥` | 180 | `len(♠, 5..) & points(9..)` | transfer to ♠, INV+ |
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

### Measurement — the ship row (2026-08-18)

`scripts/ab-nt-high-overcall.sh`, `SEED_BASE=1787055415`, sha `69cd39a1`+dirty,
230,400 bd/arm/vul, `--filter-1nt` on every arm. Three arms: `base` (knob off),
`stop` (on, `direct_3nt_stopper` as shipped), `nostop` (on, the shared stopper
bit dropped).

**The package (`stop ↔ base`) — owned boards** (`probe-divergence`, split on
`opener_ours`):

| vul | fired | plain/bd | PD/bd | plain/fired | PD/fired |
| --- | ---: | --- | --- | ---: | ---: |
| none | 435 (0.19%) | **+0.00208 ±0.00126** | +0.00079 ±0.00145 | +1.103 | +0.416 |
| both | 460 (0.20%) | **+0.00293 ±0.00160** | +0.00180 ±0.00182 | +1.470 | +0.900 |

Single-dummy leads (whole arm, 16 worlds): plain **+0.0019 ±0.0013** NV /
**+0.0028 ±0.0016** vul, PD +0.0008 ±0.0014 / +0.0015 ±0.0018. **Sixteen
readings, no negative cell.** Plain is CI-clear on both vuls, perfect defense
keeps 38%/61% of it with the same sign — it does not *erase* the win, which is
what the decision table's artifact row is about, and this package's added double
is a **takeout** double opener always answers, not a penalty double.

**Isolation gate: 16 NV / 12 vul foreign boards** (`--gate-opener ours`), which
is a hard fail and a small one — 3.5% / 2.5%. The mechanism is worth recording
because it is *not* the mirror-read leak the other packages hit. The classifier
is clean: over `1♠ 1NT 3♠` — our 1NT an **overcall**, not an opening — the node
does not fire and the floor answers, exactly as authored. The **reader** does
fire: `1♠ 1NT 3♠ 4♣ -` reads partner as `♣ 5.., points 10.., ♥ ..3` from this
table's rule, because the inference walk keys a made call from the caller's own
`1NT` while `classify` keys from the auction's start. Priced: NV foreign is
**−1 plain / +4 PD IMPs on 16 boards** (noise), vul foreign is +47/+43 on 12
(+3.9/fired, ~6% of the plain total). The owned figures above are the verdict
either way, and they stay CI-clear. The read is not obviously *wrong* either —
our 1NT overcall is 15–18 balanced and partner's `4♣` over their `3♠` really is
a long minor — but the scope mismatch belongs in
[authored-reading-handoff.md](authored-reading-handoff.md)'s inventory.

### `stop` vs `nostop` — why the shared stopper bit was not flipped

`nostop ↔ stop` looks like a win on plain (NV **+0.00067 ±0.00062**, vul
+0.00040 ±0.00079) and a wash on PD. It is **two lanes summed**, and they
disagree — `--gate-opener ours` fails at 44/121 NV and 39/116 vul, and the
foreign boards are all `2M X - 3NT`: `direct_3nt_stopper` also governs
**advancing partner's takeout double of a weak two** (`american/defense.rs`
reuses the Lebensohl builders verbatim). Split:

| subset | NV plain/fired | NV PD/fired | vul plain/fired | vul PD/fired |
| --- | ---: | ---: | ---: | ---: |
| our 1NT opened (this lane) | **+2.195** | +0.662 | **+1.623** | +0.377 |
| everything else (the advance lane) | −0.318 | **−1.227** | −0.846 | **−1.923** |

So this lane wants no gate and the other lane wants it kept — which is why the
three-level table got its **own** bit, `competition.nt_high_overcall_3nt_stopper`,
rather than a flip of the shared one.

### Round 2 — the private bit **SHIPPED OFF**, the `(3♣)` transfers stay opt-in (2026-08-18)

Two increments over the shipped default, each against the reused `stop` arm —
whose boards were checked byte-identical to a default-flag regeneration before
reuse (only the recorded `gen_args` metadata differs). Two seeds, 1787055415 and
1787060609, 230,400 bd/arm/vul each.

**`nogate` — `nt_high_overcall_3nt_stopper false`, SHIPPED default-off.**
Pooled over both seeds (460,800 bd/vul):

| vul | fired | plain/bd | PD/bd | plain/fired | PD/fired |
| --- | ---: | --- | --- | ---: | ---: |
| none | 127 | **+0.00065 ±0.00036** | **+0.00043 ±0.00041** | +2.370 | +1.543 |
| both | 145 | **+0.00052 ±0.00047** | +0.00016 ±0.00053 | +1.648 | +0.510 |

Three of four DD cells CI-clear, the fourth wash-positive; single-dummy leads
positive on all eight per-seed cells (+0.0007…+0.0011 plain, +0.0005…+0.0006 PD),
five of them CI-clear. **`probe-divergence --gate-opener ours` passes at 0 foreign
on all four seed × vulnerability cells** — the campaign's third clean gate, and
exactly what the private bit was for. The size matches round 1's prediction for
this lane (+2.20/+1.62 plain per fired) to within noise.

Note `smoke-default` does **not** move on this flip (`39ca60a2…` unchanged): the
lane fires on 0.03% of `--filter-1nt` boards and the smoke set is unfiltered, so
zero hits in 20,000 auctions is the expected count. The A/B is the only witness
here; a byte-identity smoke is not evidence of inertness at this firing rate.

**`xfer` — the `(3♣)` transfers, measured WASH across two seeds, stays opt-in.**
Owned boards (6–8 foreign per cell, the same reader-scope leak, sign-flipping
between seeds):

| vul | fired | plain/bd | PD/bd | plain/fired | PD/fired |
| --- | ---: | --- | --- | ---: | ---: |
| none | 174 | +0.00002 ±0.00026 | +0.00007 ±0.00029 | +0.057 | +0.172 |
| both | 172 | +0.00007 ±0.00034 | +0.00009 ±0.00037 | +0.198 | +0.244 |

All four pooled cells positive and all four an order of magnitude inside their
CI. Seed 1 looked like a win at vul (plain +0.0003, PD +0.0004, PD > plain — the
right-siding signature); **seed 2 reversed it** (plain −0.0004, PD −0.0005), and
the pool is flat. That is the decision table's `wash | wash, a convention
trialled against natural` row: **stays opt-in**, default off, finished code with
its measurement paid.

Both `xfer` arms were measured against a **gated** `3NT` baseline, since they
ran before the `nogate` flip. Round 3 below pays that fresh-baseline caveat.

### Round 3 — top-step minor symmetry, still opt-in (2026-08-19)

The owed fresh-baseline run makes `1NT (3♣) 3♠` exactly the minor-swapped
twin of `1NT (2♦) 3♠`: responder now shows **6+♦** (not 5+), and opener
bids `3NT` with a club stopper, otherwise `5♦` (replacing the old
`3NT`/`4♦` table). Responder's club stopper instead selects direct `3NT`, as
in the `(2♦)` tree. The major transfers are unchanged and still share
`rubensohl::transfer_completion`: `4M` with three-card support, otherwise
`3NT`, including the doubled-transfer tail.

Fresh seeds `1787072350` / `1787073219`, sha `4740bcc3`+dirty, 230,400
boards/arm/vulnerability/seed, `--filter-1nt`; new `xfer` versus the current
shipped `stop` baseline (`nt_high_overcall_3nt_stopper false`). Owned boards:

| vul | fired | plain/bd | PD/bd | plain/fired | PD/fired |
| --- | ---: | --- | --- | ---: | ---: |
| none | 180 | +0.00008 ±0.00027 | +0.00008 ±0.00030 | +0.200 | +0.200 |
| both | 176 | +0.00019 ±0.00033 | +0.00020 ±0.00037 | +0.500 | +0.528 |

Every cell leans positive and every CI contains zero: still **wash | wash**.
The raw SD pair also leans positive in all four pooled cells, but the exact
top-step `3♠`→♦ branch fired on only 3 NV + 2 vulnerable owned boards. The
isolation gate found 6/4 and 8/9 foreign divergences by seed (the known
reader-scope leak); they are excluded above. This is still a convention
trialled against natural, so a wash keeps `nt_3c_transfers` opt-in/default-off.
The shipped system is byte-identical: `smoke-default --count 20000 --seed 1`
stays `39ca60a251e03e558cfe44659b44ae45b1fe296d806e90cb3ed1cc9338bf72cd`.

### BBA-style double continuation — refuted (2026-08-19)

A temporary experimental arm kept our responder's existing takeout-double
constraint fixed and changed only the continuation. Opener showed a four-card
major at the cheapest level; with none, it copied BBA's `3♦` over `(3♣)` / `4♦`
otherwise, and responder placed. This isolated the continuation from BBA's
different direct-double ranges.

Fresh seed `1787121438`, sha `e6819181`+dirty, 230,400 filtered boards per arm
and vulnerability (the temporary `ROUND=3` arm in
`scripts/ab-nt-high-overcall.sh`, since removed):

| vul | fired | plain/bd | PD/bd | sd plain/bd | sd-PD/bd |
| --- | ---: | ---: | ---: | ---: | ---: |
| none | 109 | **−0.0012 ±0.0006** | **−0.0012 ±0.0008** | **−0.0012 ±0.0006** | **−0.0013 ±0.0007** |
| both | 112 | **−0.0017 ±0.0008** | **−0.0015 ±0.0010** | **−0.0021 ±0.0009** | **−0.0020 ±0.0010** |

Every cell is negative and all eight CIs exclude zero. Isolation is clean:
221/221 divergences were on boards we opened. The mechanism is one row:
`4♦ ← 3NT` lost **−310/−348 plain/PD** over 55 NV boards and **−420/−429**
over 57 vulnerable boards. The candidate missed game where the baseline made
one on 42/109 and 50/112 divergences. By overcall suit, only `(3♣)` avoided a
replicated loss (NV +26/+43, vulnerable −11/+3 raw IMPs); `(3♦)`, `(3♥)`, and
`(3♠)` were negative on both scorers and vulnerabilities. The anchor's bad
`(3♠) X` attribution therefore did not identify `3NT` as a causal leak. The
arm, knob, test, and harness were removed after measurement; do not retry the
whole continuation.

### Round 4 — the answer tables' cross-call weight ties (2026-08-19)

**Pre-pinned before the run** (N1j precedent; rationale = structural
alignment, not an expected gain).

*The defect.* All three of opener's answer tables price the two majors' rows at
one weight — `nt_answer_double`'s `4M@150` / `3M@140` / `3M@30` / `4M@25`,
`nt_answer_forcing_suit`'s minor arm `3M@140`, and `nt_answer_forcing_minor`'s
`4M@130`. Production keeps the *first strict* maximum in call-encoding order,
so on a cross-call tie the **encoding** decides and hearts always wins: opener
with four hearts and five spades answers the takeout double `3♥`. The same bug
class was fixed on the responder side at ship ("+ rank is load-bearing"), and
`weight_tie_report` never saw it — that invariant only meters ties on the
*same* call. The test helper `best_call_with` used `max_by`, which keeps the
*last* maximum and so resolved ties the opposite way from production, hiding
the defect from any pinned test.

*The repair.* Each major's rows carry `at_least_as_long(major, rival)` whenever
their overcall leaves both majors live; with only one live major there is no
rival and no guard. A genuine 4-4 still fires both rows and still answers in
hearts (byte-identical), a 5-4 now answers in its five-carder. `best_call_with`
now reduces with a strict `>`, matching production, and
`the_double_answer_picks_the_longer_major` pins all four cases.

*The gate, pinned before reading any number.* Both arms under
`--filter-preempt`, fresh `SEED_BASE`, arms sequential, no rebuild in flight,
`fix` versus `base` at 716,800 boards per arm per vulnerability. **Ship iff no
CI-clear negative cell across {NV, vul} × {plain, PD}.** Any CI-clear negative
cell → revert and log. No knob: this is a repair, not a treatment, so the two
arms are two *binaries* built from the same tree with and without the `src/`
patch — both carrying the same `--filter-preempt`, so their accepted deal sets
are identical per seed.

*Measured — **SHIPPED**.* `ab-results/nt-answer-tie/`, seed `1787144117`, sha
`7f8fa998`+patch, 716,800 boards/arm/vulnerability, 28 shards × 25,600:

| vul | fired | plain/bd | PD/bd | plain/fired | PD/fired | sd plain/fired | sd-PD/fired |
| --- | ---: | --- | --- | ---: | ---: | ---: | ---: |
| none | 252 (0.04%) | +0.0002 ±0.0003 | +0.0002 ±0.0003 | +0.508 | +0.520 | +0.646 | +0.719 |
| both | 221 (0.03%) | +0.0001 ±0.0003 | +0.0001 ±0.0003 | +0.416 | +0.267 | +0.221 | +0.032 |

Eight of eight cells lean positive and none is CI-clear negative: the pinned
gate passes and the repair ships. `probe-divergence --gate-opener ours` is
**0 of 252** and **0 of 221** foreign — perfect isolation, as a book row keyed
`P* 1NT (3x) X -` should be.

The mechanism is not the one the defect description predicts. Only 6.0% / 3.6%
of divergent boards are "a different bid"; **85.7% / 88.7% are "passed where the
baseline bid"**, and game is reached in both arms on 94.4% / 100.0%. The guard
is doing most of its work through the **reading**: `4♥ | ♥ at least as long as
♠` tells responder opener is not hiding five spades, so responder stops
correcting to `4♠` over a 4-4 answer. The call-level 5-4 repair is real but
rare; the reading it publishes is what the IMPs came from.

*`smoke-default` cannot see this lane.* The default-system hash is unchanged
(`39ca60a251e03e558cfe44659b44ae45b1fe296d806e90cb3ed1cc9338bf72cd`,
`--count 20000 --seed 1`) — but that is **not** an inertness proof here: we never
overcall a 1NT opening at the three level, so a self-play smoke never reaches
`1NT (3x)` at all. The A/B above is the whole evidence.

### The v2 queue, re-priced (probe + fresh-seed census, 2026-08-19)

The N3 residue was queued against an opponent whose side of the lane had never
been probed. It has been now — advancer tables, sit-vs-rescue over our double,
the preemptor's second turn, and the `(4x)`/`(3NT)` triggers, in
[bba-1nt-counter-defense.md](ai-bidder/bba-1nt-counter-defense.md) §"Their side
of the lane". Four items move.

**1. `(3NT)` is closed — no trigger.** BBA never bids `3NT` directly over our
1NT: the row does not exist at 200,000 hands per vulnerability, at either
vulnerability. Nothing to counter.

**2. `(4x)` is re-priced down, twice.** On the fresh-seed anchor the whole `4+`
bucket is **38 bd / −43 plain / −45 PD** (NV 23 bd −22/−27, vul 15 bd −21/−18),
not the 43 bd / −89 of the series seed; per board −1.13 ±2.78, a CI that
swallows the total. And the trigger is not a widened `(3x)`: BBA's four-level
overcalls are **eight**-card suits (`4♥` 0.049%, `4♠` 0.046%, `4♦` 0.012%,
`4♣` 0.006% of hands), six times rarer than the three-level rows, with `5♣` and
`5♦` (also 8+) as common as `(4M)`. Widening the `(3x)` template to `(4x)`
would author for a hand class the template does not describe. What survives is
narrower and better supported: the advancer **sits for our double of a `(4x)`
on 96.7–99.9%** of hands, and our floor cannot double above the three level at
all, so a book `X` over `(4x)` is an uncontested opportunity — parked as a
sized item, not the top of the queue.

**3. The penalty pass survives its probe, and the realized rate is stronger than
the probe's.** The item was queued on an unsourced "BBA sits over some doubles".
It does, and then some. Per random advancer hand the probe reads 88.2% Pass over
a minor and ~50% over a major; counted over **14,120 realized `1NT (3x)` boards**
of a `--filter-preempt` arm — where the advancer's hand is conditioned on our
side holding 23+ — it is **99.7% / 100.0% / 97.1% / 98.3%** over
`(3♣)`/`(3♦)`/`(3♥)`/`(3♠)`. If opener leaves the double in, we defend the
doubled three-level contract essentially every time. This is not the `(2♦)` lane,
where the runout was unconditional and the item died without a run.

**4. The `X (4z)` tail is closed — it does not happen.** The preemptor never bids
again (six two-ply probe lanes, 99.4–100% Pass on filtered hands), and the
advancer, once conditioned, acts over our double on **0.0–2.4%** of realized
boards. The node `P* 1NT (3x) X (4z)` would own a tail that is two boards in a
thousand. Removed from the queue; what is left of the tail is the advancer's
`(4M)` over our **`3NT`** (6.7% over `(3♥)`, 9.1% over `(3♠)`), which is a
different node and still floor-owned.

#### What the census says instead — the two cells worth authoring

Per-cell decomposition of the fresh-seed anchor (`--bucket … --responses 8`,
both vulnerabilities pooled, boards / plain / PD):

| cell | bd | plain | PD | mechanism |
| --- | ---: | ---: | ---: | --- |
| `(3♠) X (P) 3NT` | 20 | **−94** | **−123** | opener bids `3NT` on one stopper facing a *seven*-card suit — and does it **holding four hearts**, the suit responder's takeout double promised |
| `(3♣) 3♠` | 16 | **−54** | **−57** | the force is answered `4♠` and dies: slam missed on a 5-5 11-count (`4♠+3`, BBA bid `6♠`), or `4♠` on a 5-3 where `3NT` was the make |
| `(3♣)` Pass | 43 | −67 | −14 | responder has no call; PD nearly recovers it |
| `(3♣) X` | 29 | −30 | −51 | |
| `(3♣) 3♥` | 30 | −12 | −13 | |
| `(3♣) 3♦` | 18 | +9 | +6 | |
| `(3♠) 4♥` | 18 | +37 | +38 | the authored four-level rung, and the lane's best cell |

The `(3♠) X (P) 3NT` cell is the largest single loss in N3 and has a one-row
cause. Over `(3♠)` the cheap `3M@140` rung does not exist — hearts are *below*
their suit — so `nt_answer_double`'s ladder runs `4♥@150` (four hearts **and**
17+ points), `3NT@130` (one stopper), `4♥@25` (three-card tolerance). Opener
with four hearts and 15–16 therefore bids `3NT` and buries the known 4-4 fit:
on the worst NV board opener held `K5.QJ84.A92.KQ92` opposite `Q6.A972.K8643.63`
and `3NT` went **three down** while `♥` was worth nine tricks. The repair is to
give the shown major its **cheapest legal** rung — four when three is gone —
above `3NT`, not to replace `3NT` everywhere.

This is *not* the refuted BBA-style continuation. That arm bundled the same
cheapest-level major with "no major → `3♦`/`4♦`", and its own decomposition put
the whole loss on `4♦ ← 3NT` (−310/−348 and −420/−429). Over `(3♠)` "no major →
`4♦`" is the arm's dominant branch, so the `(3♠)` column being negative there
prices the `4♦` substitution, not the `4♥` rung. The un-bundled half is
untested, and the census cell it targets is the lane's biggest.

### Round 5 — opener's answer to the takeout double: the fit rung **ships**, the leave-in is **refuted** (2026-08-19)

Two knobs, one control, one seed. `ab-results/nt-answer-x-v2/`, seed
`1787145997`, sha `7f8fa998`+patch (round 4 shipped), **716,800 boards per arm
per vulnerability** under the new `--filter-preempt`, 28 shards × 25,600.
`scripts/ab-nt-high-overcall.sh` `ROUND=4`.

*Read the per-board figures against `--filter-preempt`'s density, not
`--filter-1nt`'s.* The `1NT (3x)` lane is **13.7%** of accepted boards here
against 0.60% there, so these per-board numbers are ~23× more concentrated than
the round-1/2 rows above and are **not** comparable to them. Per-fired is.

#### `fit` — `nt_high_overcall_x_major_at_four`, **SHIPPED DEFAULT-ON**

| vul | fired | plain/bd | PD/bd | plain/fired | PD/fired | sd plain/fired | sd-PD/fired |
| --- | ---: | --- | --- | ---: | ---: | ---: | ---: |
| none | 1103 (0.15%) | **+0.0018 ±0.0007** | **+0.0034 ±0.0008** | +1.141 | +2.182 | +0.243 | +0.889 |
| both | 1340 (0.19%) | **+0.0032 ±0.0009** | **+0.0062 ±0.0011** | +1.687 | +3.322 | +0.352 | +1.578 |

Four of four double-dummy cells CI-clear positive, all four sd-lead cells
positive, `probe-divergence --gate-opener ours` **0 foreign of 1103 / 1340**.
This is `win | win` on the decision table, so it ships default-on. One rung:
over `(3♠)`, `4♥` at 140 with four hearts — the fit responder's takeout double
promised, which the ladder previously buried under `3NT@130` because hearts sit
*below* their suit and the cheap `3M` rung does not exist there.

The census cell it targets (`(3♠) X (P) 3NT`, 20 bd / −94 plain / −123 PD) is
the one the "BBA-style double continuation" arm also touched and lost on. The
difference is the un-bundling: that arm replaced `3NT` with `3♦`/`4♦` when
opener had *no* four-card major, and its own decomposition put the whole loss on
`4♦ ← 3NT`. Keeping `3NT` for the no-major hands and adding only the fit rung
turns the same cell from a −4.4/board loser into a +1.1/+1.7-per-fired winner.
**A fresh-seed confirmation is owed** before this row is treated as settled.

#### `pass` — `nt_high_overcall_x_leave_in`, **REFUTED, kept opt-in**

| vul | fired | plain/bd | PD/bd | sd plain/bd | sd-PD/bd |
| --- | ---: | --- | --- | --- | --- |
| none | 9157 (1.28%) | **−0.0048 ±0.0019** | +0.0078 ±0.0021 | **−0.0263 ±0.0019** | **−0.0184 ±0.0021** |
| both | 10493 (1.46%) | +0.0072 ±0.0026 | +0.0295 ±0.0028 | **−0.0257 ±0.0027** | **−0.0099 ±0.0027** |

Double dummy splits by vulnerability — a CI-clear plain **loss** NV, a CI-clear
plain win vulnerable — and perfect defense is a large win in both. That pattern
is precisely the doubling artifact [measurement.md](measurement.md) names, and
the **sd-lead tie-breaker settles it: CI-clear negative in all four cells**,
−1.75 to −2.06 IMPs per fired. Isolation was clean (0 foreign of 9157 / 10493),
so this is the treatment, not a leak.

The bridge reading of the split is the honest one: with 15–17 opposite a
takeout double's 8+, we hold 23+ and belong in **game**, not defending a doubled
three-level partscore for +200. The vulnerable column is the exception that
proves it — +500 instead of +200 is what flips double-dummy's sign, and even
that does not survive a realistic opening lead. The knob stays default-off; the
only live follow-up is a **vulnerability-gated** variant, and it inherits the
sd-lead result as its prior.  Round 6 below re-slices these same dumps and
finds a second, better-supported follow-up: the loss is not uniform, and a
length gate keeps the part that pays. Probe evidence that the leave-in *can* fire (the
advancer sits 97–100% of realized boards) was correct and irrelevant: the
question was never whether they run, it was whether defending beats bidding.

### Round 6 — the leave-in re-sliced (2026-08-20)

Round 5 refuted the v1 gate as one number. Before discarding the idea, the
**existing** dumps (`ab-results/nt-answer-x-v2/{pass,base}-{none,both}`, seed
1787145997, 716,800 bd/arm/vul) were re-scored — no new bidding run — through a
new `--by holding` bucket key shared by `ab-dump-bucket` and `ab-dump-sd`
(`common::holding_key`). The key is opener's holding in **their** seven-card
suit: `len` {≤2, 3, 4+} × `top_honors` {0, 1, 2+} × whether the `4M` fit rung
fires.

The window is exact for *this* gate: 9157/9157 (NV) and 10493/10493 (vul)
divergent boards key to a real bucket, ON always passing and OFF always
bidding, so the `(other)` bucket reads zero. `len0-1` never appears and never
will — a balanced 15–17 opener holds no singleton or void.

**Do not read `(other)` as the isolation gate.** It collects genuinely foreign
boards *and* in-lane boards whose first divergence is downstream of opener's
answer, and only the first is a leak. v1's gate was wide enough (pass on ≤1
honor) that every board reaching the table diverged right at the answer, so
`(other)` happened to be zero; the narrow v2 gate leaves opener's call
unchanged on most boards it reaches and moves only the *reading*, which
surfaces as a later divergence — 12.7% / 9.4% in Round 7, every one of them
still a `1NT (3x) X` auction we opened. Test foreignness explicitly.

**Read the `no4M` rows, not the totals.** The `pass` arm predates the fit rung,
so its dumps still route to `Pass` on boards where today's shipped
`nt_high_overcall_x_major_at_four` (weight 140) outbids `Pass` (weight 135).
Those boards cannot diverge in Round 7 — both arms bid `4M` — so every figure
below is the `no4M` subset, which is what a fresh A/B against today's default
will actually see. The `4M` block is quoted separately as finding 3.

**IMPs per fired, NV / vulnerable** (`no4M` subset; n = NV/vul):

| bucket | n | DD plain | DD PD | sd plain | sd PD |
| --- | ---: | --- | --- | --- | --- |
| whole subset | 7870 / 8916 | −0.25 / +0.76 | +0.80 / +2.33 | −1.93 / −1.55 | −1.25 / −0.44 |
| `len2 hon1` | 2165 / 2476 | −1.05 / −0.46 | +0.22 / +1.66 | −2.53 / −2.70 | −1.76 / −1.28 |
| `len2 hon0` | 980 / 1162 | +0.15 / +0.88 | +0.88 / +1.73 | −1.01 / −0.54 | −0.39 / +0.21 |
| `len3 hon1` | 2995 / 3287 | −0.75 / +0.37 | +0.27 / +2.05 | −2.66 / −2.33 | −2.06 / −1.18 |
| `len3 hon0` | 698 / 843 | +0.62 / +1.85 | +2.02 / +3.42 | −0.88 / −0.16 | +0.21 / +1.19 |
| **`len4+`** | **1032 / 1148** | **+1.92 / +3.58** | **+2.68 / +4.36** | **−0.13 / +1.09** | **+0.33 / +1.65** |

Three findings, in order of what they cost:

1. **The whole surviving case is length.** `len4+` is the best cell at both
   vulnerabilities on every scorer, and the only one that clears Round 5's
   −2.06 IMPs/fired sd prior: flat NV (−0.13) and genuinely positive vulnerable
   (+1.09), against the subset's −1.93 / −1.55. It is 13% of the subset, so v1
   spent roughly seven bad boards to buy each good one.

2. **The honor axis runs the wrong way.** At fixed length every measured honor
   step *costs*, on both scorers: `len3 hon0` +0.62 vs `len3 hon1` −0.75 (DD,
   NV) and −0.88 vs −2.66 (sd, NV); `len2` repeats it. The mechanism is
   `has_stopper` — A, Kx, Qxx, or Jxxx — so at three cards an A/K/Q in their
   suit **is** the stopper, and honors there mark the boards where the `3NT` we
   gave up was a stopper-backed game rather than a punt. `hon2+` is genuinely
   unmeasured (v1's gate was `top_honors(..=1)`), so it is not refuted; but the
   trend extrapolates to the *worst* three-card cell, not the best.

3. **v1's headline partly priced boards `main` no longer passes.** The `4M`
   block is DD −1.14 / −1.02 and sd −2.69 / −2.92 per fired — the worst block
   in the slice — and the fit rung shipped in Round 5 already outbids it. Any
   re-measure must use today's default as `base`, which Round 7 does.

**Consequence — v2 is two knobs, not one.** `nt_high_overcall_x_leave_in` is
re-gated to `len(over, 4..)`; the honor disjunct becomes its own
`nt_high_overcall_x_leave_in_three` (`len4+ | (len3 & hon2+)`). Finding 2 says
the two disjuncts have opposite signs, so bundling them would let a win ship
the bad half or a loss bury the good one. Round 7 runs them as separate arms
(`scripts/ab-nt-high-overcall.sh`, `ROUND=5`: `base` / `length` / `three`), with
`three vs length` reading the extension's own price. A 4000-board preflight
confirms the arms are nested as designed — `length` diverges from `base` on 9
boards, `three` on 17, `three` from `length` on the remaining 8.

**Reproduction caveat.** The DD tables re-sum to Round 5's published headline
exactly (9157 fired, plain −3431, PD +5558). The **sd** totals do not: −18,645
(NV) / −18,472 (vul) against the published −18,835, i.e. −2.036 vs −2.057 per
fired, ~1% off. Cause, not noise: in the ON arm *they* declare the doubled
partscore, so the opening leader is **our** side and our own book feeds the
blind lead, and `main` has moved since Round 5 (the weight-tie guard
`1ecac19d`, the fit rung `30ea36ba`). Recorded as reproduced-to-1%-with-cause;
every conclusion above is a *within-slice* contrast, which that drift does not
touch.

**In-sample warning.** The re-gate was chosen on these dumps, so it is
in-sample and ships nothing until Round 7's fresh-seed A/B confirms it
out-of-sample on plain DD, per [measurement.md](measurement.md)'s domain
addendum for a knob whose mechanism is adding doubles.

### Round 7 — the length leave-in **SHIPS DEFAULT-ON**, the honor half refuted (2026-08-20)

`ab-results/nt-answer-x-v3`, `SEED_BASE=1787169600`, sha `14acdd1f`, 28 x 25,600
= 716,800 bd/arm/vul, `--filter-preempt`, three arms against today's default
(`base`, which already carries the `4M` fit rung):

- `length` — `nt_high_overcall_x_leave_in` re-gated to `len(over, 4..)`
- `three` — ...plus `nt_high_overcall_x_leave_in_three`, the full v2 candidate

**IMPs per fired, NV / vulnerable:**

| pairing | fired | plain DD | PD | sd plain | sd PD |
| --- | ---: | --- | --- | --- | --- |
| **`length` vs `base`** | 2124 / 2191 | **+2.383 / +3.410** | +2.885 / +4.074 | **+0.558 / +0.817** | +0.837 / +1.244 |
| `three` vs `base` | 3298 / 3472 | +1.384 / +2.166 | +1.930 / +2.966 | **−0.546 / −0.630** | −0.218 / −0.109 |
| `three` vs `length` | 1211 / 1298 | −0.405 / +0.012 | +0.200 / +1.033 | **−2.438 / −2.990** | −2.094 / −2.406 |

Per board with CI: `length` vs `base` plain **+0.0071 ±0.0009** NV / **+0.0104
±0.0012** vul, PD +0.0085 ±0.0010 / +0.0125 ±0.0013, sd-plain +0.0017 ±0.0009 /
+0.0025 ±0.0011, sd-PD +0.0025 ±0.0010 / +0.0038 ±0.0012.

#### `length` — **SHIPPED DEFAULT-ON**

CI-clear positive in **all eight cells**. No wash, no negative cell, no scorer
disagreement, both vulnerabilities — the ship condition met without needing the
decision table's tie-break rules, and the arbiter column (plain DD, since the
mechanism is adding doubles) is the strongest of the four. v1 on this same lane
was CI-clear *negative* in all four sd cells. Same convention, window and
opponent model: the gate was reading the wrong feature.

**Isolation: 0 foreign boards at both vulnerabilities** (2138 NV / 2201 vul
divergences, every one a `1NT (3x) X` auction we opened). This is one of the
campaign's few clean isolation gates.

Bucketed by `--by holding`, **13 of 13 buckets positive on DD at both
vulnerabilities**; on sd, 12 of 13 NV (only `(3♣) len4+ hon1` at −0.31 plain,
PD-positive) and 12 of 12 vulnerable.

Two things the bucket table shows that no earlier measurement could:

1. **`hon2+` is the best cell, not a passenger.** `len4+` with two or more of
   A/K/Q in their suit reads +4.21 (NV) / +4.63 (vul) plain per fired, the top
   cell at both vulnerabilities. v1's `top_honors(..=1)` gate structurally
   excluded it, so it appears in no prior dump and Round 6's slice could not
   see it — roughly half of `length`'s fired boards are therefore genuinely
   out-of-sample even against the slice that motivated the design.

   This resolves the apparent contradiction with Round 6's finding 2. At
   **three** cards an A/K/Q in their suit *is* the stopper (`has_stopper` = A,
   Kx, Qxx, Jxxx), so passing spends a real stopper-backed `3NT`. At **four**
   they hold seven and we hold four: the suit was never running against `3NT`
   anyway, the stopper question is moot, and the same honors become pure
   defensive tricks. Honors hurt at three and help at four for one reason.

2. **A suit gradient.** The leave-in pays most against the highest overcall:
   sd plain per fired at `hon2+` is `(3♠)` +1.67 / `(3♥)` +0.21 / `(3♦)` +0.09
   / `(3♣)` +0.26 NV, and `(3♠)` +1.46 / `(3♣)` +1.79 vul. Over `(3♠)` opener's
   alternatives are genuinely bad — `3NT` wants a spade stopper we do not have
   holding four small, everything else is at the four level. Over `(3♣)`,
   `3NT` is cheap and often right. **In-sample on this run**; a suit-dependent
   gate is a Round 8 question, not a conclusion.

Also confirmed: **no `4M` bucket appears at all**. In Round 6's slice the `4M`
block was 1287/1577 boards and the worst in the table; with the fit rung in
`base` those boards are bid identically in both arms and never diverge. Round
6's finding 3 was right, and running against today's default rather than the
pre-ship arm is what made it visible.

#### `three` — **REFUTED, kept opt-in**

Priced in isolation against the shipped gate, sd-lead is CI-clear negative at
both vulnerabilities: **−2.44 / −2.99 IMPs per fired** — v1's own headline
magnitude (−1.75 to −2.06), reproduced on fresh seeds. The honor half is not
the weaker disjunct; it *is* the v1 loss.

Its DD signature is `plain wash | PD win` (+0.0000 ±0.0009 plain vulnerable,
**+0.0019 ±0.0009** PD), which the standard decision table would ship
default-on. The domain addendum blocks it, and this is the cleanest example the
campaign has produced of why that addendum exists: the knob's mechanism is
*adding doubles*, and a double-dummy defender never misdefends exactly the
doubled contracts it creates. Plain DD arbitrates, sd-lead breaks ties, PD is a
double-blind column that neither rescues nor kills.

Added to the length gate the extension **inverts the package**: `three` vs
`base` is a CI-clear sd loss at both vulnerabilities where `length` vs `base`
is a CI-clear win. **Bundled as one gate — which is what the v2 plan originally
specified — this run would have returned a refutation at both vulnerabilities,
and a +3.4 IMPs/fired winner would have been thrown away inside it.** The
general rule: when a candidate gate is a disjunction and the slice gives its
disjuncts different signs, they are separate arms, always.

Kept as an opt-in knob (house rule for rejected-but-interesting treatments) and
a single-dummy re-measure candidate on its vulnerable PD reading.

#### Flagged, not fixed — reading drift is the one negative cell

12.7% (NV) / 9.4% (vul) of divergences are boards where opener's call is
**unchanged** and only a later call moves. Adding a `Pass` rule at weight 135
narrows the complement — `3NT` in that seat now also denies four of their suit
— so partner's inference shifts and a downstream slam try or double changes.
This is [reading-drift-handoff.md](reading-drift-handoff.md)'s subject: a rule
addition is never reading-neutral when a call's meaning is read off the bidder.

Those boards are the only negative cell in the vulnerable slice: **−0.97 plain
/ −0.72 PD per fired** on 197 boards, against +0.90 / +0.23 NV. Pooled it is a
wash (+44 IMPs plain across both vuls) and it does not threaten the headline,
but it is a real vulnerable cost inside a shipped win. Not fixed here; recorded
for the reading-drift queue.

### Round 8 — the suit gradient out-of-sample (**IN FLIGHT 2026-08-21**)

`ab-results/nt-answer-x-v4`, `SEED_BASE=1787252714`, sha `3364aa3c` plus the
uncommitted Round-8 runner, 28 x 25,600 = 716,800 bd/arm/vul,
`--filter-preempt`, `ROUND=8` in `scripts/ab-nt-high-overcall.sh`. Two arms:

- `base` — today's shipped default (the leave-in on, `len(over, 4..)`)
- `noleave` — `--ns-nt-high-overcall-x-leave-in false`, the pre-Round-7 ladder

**Why two arms answer a four-way question.** Round 7's suit gradient was
in-sample; the obvious follow-up — one arm per candidate suit gate — is
unnecessary here because the overcall suits **partition** the fired set: every
divergent board's window is `1NT (3x) X -` with exactly one `(3x)`, and the
`Pass` row's presence on a `(3♥)` board never consults the `(3♣)` table. So a
hypothetical arm with the leave-in gated to any suit subset would bid every
board identically to `base` on its in-subset boards and identically to
`noleave` elsewhere — its paired diff vs `base` is byte-for-byte a suit-bucket
subset of `base vs noleave`. One diff, bucketed, prices all fourteen candidate
narrowings at once, and the Round-7 lesson about bundled disjuncts does not
bite because the "disjuncts" live on disjoint boards. Read with:

```sh
ab-dump-bucket $R/base-VUL $R/noleave-VUL --by holding
ab-dump-sd     $R/base-VUL $R/noleave-VUL -v VUL --sd-worlds 16 --show 0 --by holding
```

(`ON` must be the leave-in arm = `base`; the `(other)` bucket must read zero,
and `probe-divergence --gate-opener ours` runs before the headline as usual.)

**Decision rule, pre-registered.** A suit-gate knob is authored only if some
suit reads CI-clear negative out-of-sample — plain DD the arbiter (the
mechanism keeps doubles in), sd-lead the tie-break, PD a double-blind column.
Round 7's cells were DD-positive for every suit at both vulnerabilities, so
the expected verdict is that the uniform gate stands; the gradient's NV sd
spread (`(3♠)` +1.67 vs `(3♦)` +0.09 at `hon2+`) is what earns the check.
Pool a second seed before concluding if the per-suit CIs straddle zero.

**Deliberately excluded: the spade-only widening.** The refuted `three`
extension peaked vs `(3♠)` in the same in-sample gradient. A widening is a new
gate, not a subset of the shipped one, so it would need its own arm — and
whether it ever earns one is decided first by re-slicing Round 7's existing
`three vs length` dumps (`ab-results/nt-answer-x-v3`) with `--by holding`:
only a positive spade bucket there buys the arm.

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
- Their `(4x)` overcalls: **re-priced 2026-08-19** — see "The v2 queue,
  re-priced" above. The fresh-seed bucket is 38 bd / −43 plain / −45 PD with a
  CI that swallows it, and the trigger is an *eight*-card suit, so the `(3x)`
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
leg, byte-identical (smoke `18aba5ce…` re-verified from the worktree).
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
dropped the others and clears the gate. sd-lead could not arbitrate any
round: `ab-dump-sd` has no owner split and the raw sd is leak-inflated like
everything else here.

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
length. The systems-on overcall strip clears the disclosure, so
`(1x) 1NT (2♦)` cannot enter this lane. It is temporary until a declared-
opponent profile can project the opponents' own authored book.

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

## N2 — Muiderberg `(2♥)/(2♠)`: the census by response (2026-08-15)

This section preserves the pre-fix `2026-08-12-ea2cde9-dirty` anchor arm
(204,800 boards/vul, deal-keyed DD cache), split one call deeper with
`probe-1nt-interference --bucket 2♠ --responses 8`: table A by **our**
response to their Muiderberg (and by response / advancer / opener), table B by
**BBA's** response to *our* natural `2M` overcall of its 1NT.  IMPs are ours
(table-A NS) on both tables, so a negative table-B row is BBA's gain.  Same
attribution ceiling as the current census — these rank, they do not isolate.

### Table A — our response, pooled NV + vul

| lane | our call | boards | plain | PD | plain/bd |
| --- | --- | ---: | ---: | ---: | ---: |
| `(2♠)` (430 bd, −284) | **Pass** | 242 | **−282** | +47 | −1.17 |
| | **`2NT` relay** | 68 | **−114** | **−184** | −1.68 |
| | `3♦` (→♥) | 56 | −15 | −46 | −0.27 |
| | **`X`** | 43 | **+110** | +97 | **+2.56** |
| | other | 21 | +17 | +4 | |
| `(2♥)` (393 bd, −43) | Pass | 194 | −107 | +153 | −0.55 |
| | **`2NT` relay** | 21 | **−45** | **−105** | −2.14 |
| | `2♠` natural | 50 | +7 | −5 | |
| | `3♦` (→♠) | 45 | +6 | −11 | |
| | `X` | 58 | +54 | +86 | +0.93 |
| | other | 25 | +42 | +45 | |

Three signs are consistent across all four cells (lane × vul):

1. **`X` wins everywhere** (+1.8 / +3.5 / +0.3 / +1.8 per board).  The Optional
   double (`2-3` in their suit, 8+) followed by opener sitting (`X P P`: +2.2 /
   +4.0 / +0.4 / +2.4) is the lane's best call.  BBA's own `X` here shows the
   other major and is a *loser* for BBA over `(2♥)` (table B `X` +357 for us).
2. **The `2NT` relay loses everywhere** (−0.9 / −2.6 / −0.9 / −3.5 plain,
   −1.9 / −3.7 / −3.8 / −6.3 PD).  Its own decomposition, `(2♠)` both vuls:
   sign-off `3♥` then opener passes −45 (20 bd); relay then pass `3♣` −42
   (21 bd); **sign-off `3♦` then opener bids `3NT` on 16 of 18 boards, −52
   plain / −125 PD** across all four cells — see the mechanism below.
3. **Pass** loses NV plain (−1.65 / −1.01 per board), is a wash vul, and is
   PD-*positive* vul (their `2M` fails and PD doubles it).  Its hand classes,
   `(2♠)` NV+vul: `≤5 hcp with a 6+ suit` (the relay's 6-HCP floor) **31 bd,
   −120 plain, −3.9/bd** — the single worst class in the lane, `2♠` making
   opposite our 9-11-trick heart/diamond spots (BBA at table B, un-overcalled,
   transfers there); `≤7 hcp, no 5-card suit` 109 bd, −43 (nothing to say —
   the obstruction the Muiderberg buys); `≤5 hcp, 5-card suit` 89 bd, −54;
   `8+ hcp with 0-1 or 4+ in their suit` **11 bd, −53** — hands with **no
   call at all**: `X` needs 2-3 trumps, the relay needs `points ≤ 8`
   (a 6-card suit's upgrade pushes an 8-count to 9), the club transfer needs
   `10+`.  `T.JT6.AQ85.QT963` and `4.K92.K97.Q98542` passed.

### Table B — BBA's response to our natural `2M`, pooled NV + vul

BBA plays plain Lebensohl here ([counter-defense](ai-bidder/bba-1nt-counter-defense.md)):

| lane | BBA's call | boards | plain | note |
| --- | --- | ---: | ---: | --- |
| `(2♠)` (2291 bd) | Pass | 880 | −772 | our overcall failing (vul −1.48/bd; PD −4.8/bd = the auto-double) — the defensive-overcall lane's business, not this one |
| | `X` (= ♥4+) | 734 | −151 | BBA's one gain from a call |
| | `3♠` cue | 178 | **+216** | |
| | `3♥`, `2NT` relay, `3♦`, `4♥` | 138 / 145 / 70 / 16 | +72 / +66 / +7 / +68 | every constructive call is ours to gain |
| `(2♥)` (2423 bd) | Pass | 798 | −483 | same |
| | `X` (= ♠4+) | 790 | **+357** | |
| | `3♥` cue, `3♦`, `3NT`, `3♣`, `2NT` | 146 / 75 / 65 / 91 / 201 | +194 / +102 / +49 / +10 / +18 | |
| | `2♠` natural | 140 | −93 | |

BBA's Lebensohl earns it nothing on its constructive calls; its edge is our
overcalls going down (and, over `(2♠)`, its takeout `X`).

### Opener facing BBA's advances — not a leak

Advancer acts on ~15% of table-A boards.  Opener over the artificial `2NT`
minor-ask (`P 2NT P`) is a wash: 81 boards, −16 / −8 / +1 / 0.  Every other
advancer row is single digits of boards.  **The opener defects are after our
own weak calls**, whose ceilings the floor cannot see:

- `2NT - 3♣ - 3♦ -` → opener `3NT` (16/18, above);
- `2NT (3♥) X` — opener doubles their raise of the relay, 5 bd, −25 / −54 PD;
- `2♠ (3♥) 3♠` (`(2♥)` lane) — opener competes over our weak natural `2♠`,
  12 bd, mixed.

### The mechanism — weak calls read as unlimited

`probe-decision "Q93.K43.AKJT.Q42" "1NT 2♠ 2NT - 3♣ - 3♦ -"` reads partner as
**`hcp 6..37, points 6..37, every suit 0..13`**, provenance `depth 0,
fallback Some(0)` — the floor — and bids `3NT` 1.400 over Pass 0.  Two causes:

1. **`Points::project` and `Hcp::project` are floor-only** by design
   ([constraint.rs](../src/bidding/constraint.rs), "floor only, matching every
   hand-written reader"; the two-sided `project_band` serves only the *pass*
   reading).  So `points(..=8)` on the relay and on the natural 2-level call
   projects to `0..37`; a weak sign-off is read as unlimited by every net
   downstream.  The uncontested 1NT structure is protected by the hand-coded
   notrump walk (`1NT - 2♣ - 2♦ - 2NT -` reads `8..9`); the Lebensohl lane has
   no such reader, so only the relay's `hcp(6..)` floor survives.  `1NT 2♥ 2♠ -`
   (our weak natural `2♠`) reads as **nothing at all**.
2. **The sign-off's own length is dropped too** — the reading of responder's
   `3♦`/`3♥` after `2NT - 3♣` is wrong on both axes.  The natural walk
   *blankets* every suit bid on the opening side after a 1NT opening except a
   lane's first three-level call (`nt_blanket` in
   [read.rs](../src/bidding/inference/read.rs) — right for the uncontested
   transfer structure, where a lane's second bid is a completion), so the
   sign-off can only be read from its authored rule
   `min_level_is(3, ♦) & len(♦, 5..)`; but that rule is natural (unalerted),
   and the shipped `ReadingScope::Alerted` decodes **alerted** rules only.
   The call falls between the two regimes — the
   [reading-drift](reading-drift-handoff.md) story exactly.  Verified with
   `PROBE_SCOPE=all probe-decision …` (`ReadingScope::All`, unmeasured):
   ♦ `5..13` comes back, `1NT 2♥ 2♠ -` regains ♠ `5..13` and `hcp 5+`, but the
   ceiling stays `..37` (cause 1) and **the floor still bids `3NT` 1.400** (and
   `4♦` 1.200) — the missing ceiling is the binding defect, the missing length
   an independent one.
3. **The relay's minor sign-off has no opener node.**  `lebensohl_signoff_raise`
   is wired for the major sign-off only (`(2♠) 2NT - 3♣ - 3♥ -`); `3♦` falls to
   the floor, which — reading an unlimited partner opposite 15-17 — bids `3NT`.

### N2 packages, from the census

| # | Package | Class | Evidence | Note |
| --- | --- | ---: | ---: | --- |
| **N2a** | opener **passes** the relay's minor sign-off — `{relay} 3♦ -` over `(2♥)`/`(2♠)` (the relay-then-pass-`3♣` is already terminal), a `landy_signoff_answer`-style node | book, one node | −52 plain / −125 PD on 18 bd, 16 of them the same wrong call | cheapest, cleanest; also gates the `2NT (3♥) X` |
| **N2e** | teach `opener_forced_past_invitation` ([instinct.rs:3820](../src/bidding/instinct.rs)) that a Lebensohl **sign-off** is not a game force | floor, one predicate | traced 2026-08-16: the predicate is *"our strong 1NT + partner's last call is a three-level non-notrump bid"*, pure auction shape | **the actual cause of the `3NT`.** It sets `forced_to_game`, so the rail bypasses the net *and* `auction_forces_game()` pre-satisfies the game-milestone `Or` — `combined_hcp` never runs. Verified: a 12-HCP opener still bids `3NT`; the only hand-dependent gate is `stopper_in_their_suits()`. Smaller than N2a. **SHIPPED default-on 2026-08-16** as `instinct.forcing_ceiling_read` (the force also requires partner's read `points` ceiling to reach 10 — the direct three-level bid's `points(10..)`, not `nt_responder_game_floor`, which is 9 and would never fire). Probe: `P 9.001` beats `3NT 7.792`. A/B 3 seeds × 204,800 bd/arm/vul, **12/12 cells positive**, +0.0001 plain / +0.0003 PD both vulnerabilities, firing 0.01% at +2 to +6 IMPs per fired board. ⚠ it does **not** fix "every sign-off lane at once" — only alerted ceilings project, so under the shipped scope it reaches this lane alone; the other floor-workaround nodes are Phase 2 work (reach census in the handoff) |
| N2b | read the relay / sign-off / natural-2-level **ceilings** (a two-sided strength projection at these rules, or a Lebensohl reader) **and lengths** (`ReadingScope::All`, or exempt the sohl lane from `nt_blanket`) | reading | the general defect behind N2a and the `X` of their raise; touches every weak call in the system | **SHIPPED 2026-08-16, both halves.** Ceilings as `ReadingProfile::strength_ceilings` (soundness-proved book-wide; the raw whole-book arm was a 4-cell wash leaning plain-DD-negative, so it shipped on the nets-shielded arm — 4/4 cells positive); lengths as `ReadingScope::All`, which took the whole `nt_blanket` question with it (12/12 cells positive after the side-blind-strip forensic). Neither moves **this** node: nothing in the floor or the book read a strength ceiling at the time (the handoff's consumer census). Necessary, not sufficient; N2e is the sufficient half |
| N2c | the no-call 8-9-count with 0-1 / 4+ in their suit — widen the relay to `points ≤ 9` with a 6-card suit, or let a singleton double | book | **Current re-price:** 11 bd, −34 (`2026-08-17-53a3c254`) | small n; the Optional > Takeout verdict was measured pons-vs-pons |
| N2d | relay with a 6+ suit below 6 HCP (over `(2♠)` only, where the weak major has no 2-level call) | book | **Current re-price:** 31 bd, −126, −4.1/bd (`2026-08-17-53a3c254`) | contradicts the PD-distilled floor ([`lebensohl_relay_shape`](../src/bidding/american/competition/lebensohl.rs)), measured pons-vs-pons; against Muiderberg the alternative is a making `2♠`, and BBA at table B bids these hands un-overcalled. Needs the A/B, not a re-derivation |

Nothing here is BBA-alignment (BBA's plain Lebensohl earns nothing at table
B); the lane's headroom is our own weak calls being *unread*.

The reading defect is the whole book's, not N2's — the campaign to fix it is
[authored-reading-handoff.md](authored-reading-handoff.md), with this lane as
its testbed (N2a stays a book node in its own right).

**Correction (2026-08-16).** The census read the `3NT` as a consequence of the
weak call reading *unlimited*. Phase 1 of the handoff built the two-sided
reading, proved it sound book-wide, and measured it — and the `3NT` does not
move. The reading was genuinely wrong and is now right, but it was never what
decided this node: `opener_forced_past_invitation` forces to game off the
*shape* of partner's call, and no floor rule or authored gate reads a strength
ceiling at all. N2e is the fix; N2b is the prerequisite that makes a
ceiling-reading rule expressible.

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

| Package | Knob | Status | Verdict (plain / PD, IMPs) |
| --- | --- | --- | --- |
| census tool | — | **shipped** | read-only; picked N1 over the pre-census guess |
| [N1 Landy `(2♣)` counter](archive/one-notrump-competitive-closed.md#n1--the-landy-2-counter-shipped-default-on-2026-08-14) | `their.two_clubs_landy` (disclosure, not a knob; `defense_2c_landy` deleted) | **SHIPPED 2026-08-14** — engine undeclared=natural; `bba-gen` derives the declaration (2/1 reference → Landy, its card lies) and `bba-decompose` replays it; re-homing proven board-identical on the measured arm | **v2 (with `landy_natural_answers`, full audit fix)** on↔off: NV plain +0.0005 ±0.0022 / PD **+0.0032 ±0.0028** (+1.10/fired) / sd −0.0006 ±0.0025, SD-PD +0.0017 ±0.0030; vul plain +0.0013 ±0.0026 / PD **+0.0043 ±0.0032** (+1.65/fired) / sd +0.0001 ±0.0029, SD-PD +0.0030 ±0.0034. 0.26–0.30% fired, 76.8k bd/arm/vul, SEED_BASE 1786644715, sha 40a0946 — plain wash + PD CI-clear both vuls = ship. **Confirmed at 3× n** (230.4k bd/arm/vul, SEED_BASE 1786653231): NV plain −0.0002 ±0.0013 / PD **+0.0032 ±0.0017**, vul plain +0.0003 ±0.0015 / PD **+0.0028 ±0.0019**, sd-PD +0.0019/+0.0015 — the v3a run's non-replication was seed noise; NV is the stronger vul. **v1 (sha 8bc465a, SEED_BASE 1786642613): LOSS all six cells** (NV plain −0.0050 ±0.0024, vul −0.0049 ±0.0028, every CI<0) — leak 1: opener's answers unauthored, audit: phantom Jacoby `2♥` 82% over `2♦`, phantom minor-transfers 23% over `2NT`, phantom Puppet `3♦` 85% over `3♣`, passed force 62% over `3♦` (only `3NT` clean); leak 2: the census misread (systems-on's minor transfers were winning the minor-partial boards). |
| [N1b GF minor cues](archive/one-notrump-competitive-closed.md#n1--the-landy-2-counter-shipped-default-on-2026-08-14) | `defense_2c_landy_cues` | **measured 2026-08-14 ×4 — stays opt-in, but v4 is the first arm with a CI-clear positive and no negative cell**. **v4** (INV+ cues + level-as-strength stopper asks, 230.4k bd/arm/vul, SEED_BASE 1786653231, sha 8873e9c+dirty): NV plain **+0.0016 ±0.0010** / PD +0.0001 ±0.0013, vul plain **+0.0014 ±0.0012** / PD −0.0000 ±0.0014, sd plain **+0.0018/+0.0024**, SD-PD +0.0006 / **+0.0016 ±0.0015**. Lands on `win \| wash` (artifact row); SD splits vul-real / NV-artifact. `probe-divergence` decomposes it into four independent effects, replicating across both vuls (per fired, we-opened, NV/vul): `3♣` weak clubs **+3.41/+2.98 plain, +0.71/+0.61 PD**; `2♠` diamond cue **+1.87/+1.93**, +0.48/+0.12; `2♥` club cue −0.17/+0.06, **−0.76/−1.04 PD**; `3♦` weak diamonds −1.07/+0.55, **−1.71/−0.55 PD**. `2♥`'s whole loss is **9 boards** where we declare *hearts* — `{cue} - {ask} (X)` passed out (every registration ends in `-`) and the floor bidding `4♥` itself on non-book continuations; the other 162 are +81/−22 = wash. Mirror leak persists at 25–31%, PD-positive, `--gate-opener ours` fails. v2 = the *full* UvU skeleton (cues carry all GF one-suiters, direct 3m weak) on the fixed base; the `probe-divergence` post-mortem decomposed the v2 wash — cues −1.76 plain/−1.90 PD per fired (missed *slams*, from a sub-game cue answer), weak escapes +0.54/−1.59 (PD-negative vul only = going for a number), and 38% of divergences on boards the opponents opened (the mirror-read leak). v3a = opener's cue answer restored to `landy_minor_answer` (game level), −0.34…−0.62/fired, still every cell negative | **v2 (full skeleton, N1-win run)** cues↔on: NV plain −0.0005 ±0.0013 / PD −0.0006 ±0.0018, vul plain −0.0007 ±0.0016 / PD −0.0012 ±0.0021, sd −0.0004/−0.0012 (−0.43…−1.25/fired, 0.10–0.11% fired, all CIs ⊇ 0, every cell leaning negative). **v1 (pure cue addition, N1-loss run):** NV plain −0.0001 / PD +0.0004, vul plain +0.0001 / PD +0.0005, sd negative — unpriceable next to phantom sibling answers. |
| [N1c club transfer + INV minors](archive/one-notrump-competitive-closed.md#n1--the-landy-2-counter-shipped-default-on-2026-08-14) | `defense_2c_landy_transfer` (implies the cues) | **measured 2026-08-14 — opt-in for a day on the artifact row, then SHIPPED DEFAULT-ON 2026-08-14 as part of the N1d/e/f stack (next row), whose pooled `win \| win` retired the PD hesitation.** Increment over N1b (`xfer↔cues`, 230.4k bd/arm/vul, SEED_BASE 1786657996, sha f313f3d+dirty, 0.06–0.07% fired): plain wash + **PD +0.0008 ±0.0008 NV / +0.0011 ±0.0008 vul** (+1.10/+1.90 per fired), SD wash both. **Passes the isolation gate in substance** — 1 of 132 divergent boards opened by them (0.8%) vs 27% for N1b. Against the *shipped* counter (`xfer↔on`), pooled over two seeds (1786657996 + 1786659297, 460.8k bd/vul): plain **+0.0013 ±0.0007** NV / +0.0007 ±0.0008 vul, PD +0.0005 ±0.0009 / −0.0005 ±0.0010, plain SD **+0.0018 ±0.0008** / **+0.0011 ±0.0009**, SD-PD **+0.0012 ±0.0009** / +0.0000 ±0.0010 — four CI-clear positives, no CI-clear negative; seed 1's vul-PD −0.0010 did NOT replicate (−0.0001). Opt-in because plain/SD-win + PD-wash is the artifact row. Residue named: the cue poaches the values double (`X`→`2♠` −3.83 PD/fired vul, `X`→`2♥` −2.63) because it is `points(8..)` at weight 173 against X's 145 — N1c wins by pulling hands off it (`2♠`→`3♦` +6.06, `3♦`→pass +3.38, `2♥`→`3♣` +2.22, transfer +0.67). **N1d = raise the cue floor to `points(10..)`.** | `xfer↔cues` NV plain +0.0002 ±0.0006 / PD +0.0008 ±0.0008 / sd −0.0002 / sd-PD +0.0002; vul plain +0.0003 ±0.0006 / PD +0.0011 ±0.0008 / sd −0.0003 / sd-PD +0.0002. |
| [N1d/e/f cue repairs](archive/one-notrump-competitive-closed.md#n1--the-landy-2-counter-shipped-default-on-2026-08-14) | `defense_2c_landy_cue_floor` + `_fit_answers` + `_competition` (each implies `_transfer`; all four now default **true**) | **SHIPPED DEFAULT-ON 2026-08-14** — the package's first `win \| win`. Stack vs shipped base (`f↔on`), pooled seeds 1786694464 + 1786695954, 460.8k bd/vul: **six of eight DD cells CI-clear positive, 8/8 sd cells positive, no negative cell in 24 readings** (table in §Ship evidence). Engages only under the `their.two_clubs_landy` declaration — default system byte-identical, smoke `8ea2f567…` unchanged; `bba-gen` stack flags are `Option<bool>` (pre-ship arm = `--defense-2c-landy-<knob> false`), `bba-decompose --landy-stack false` replays between-ships dumps. Increment attribution: **N1d is the engine** (`d↔xfer` plain wash + PD **+0.0009 ±0.0008** NV / **+0.0015 ±0.0009** vul, cue→X = 55-60% of divergence at +2.0…+5.1 PD/fired — the poached-double rows reversed); N1e fired 3+1 boards post-floor (ships on naturalness: raises promise 3+); N1f the expected CI-wide wash (ships as the iron rule's convention-completion). Isolation gate: e/f pass at 0 foreign; d and f↔on fail at 18-43% (the cue-constraint mirror leak), foreign boards *depress* the headline — our-opened figures are stronger. Residue: their **second** call still floors us (phantom `4♠` one level deeper, −17 PD, 1 board); the `3♣`→`2♥` GF-six-carder row unread against the shipped stack. | `f↔on` pooled: NV plain **+0.00068 ±0.00062** / PD +0.00075 ±0.00077, vul plain **+0.00085 ±0.00072** / PD **+0.00100 ±0.00087**; ours-only NV plain **+0.00091 ±0.00052** / PD **+0.00077 ±0.00064**, vul plain **+0.00075 ±0.00058** / PD +0.00060 ±0.00070. |
| [N1g read-side wiring](archive/one-notrump-competitive-closed.md#n1g--the-read-side-wiring-shipped-default-on-2026-08-14) | `reading.their_landy_reading` (default **true**) | **SHIPPED DEFAULT-ON 2026-08-14** — the disclosure finally read: their `2♣` = ♥4+/♠4+ (no strength claim), advances + direct-3M suppressed, via a seat-gated hand reader that cannot fire on our own `2♣` and does not extrapolate through the systems-on strip (v1 leaked there — `(1♣) 1NT (2♣)` read responder's 2♣ as Landy; fixed + regression test). `TheirDisclosures` re-homed to `DecisionProfile::their`, byte-identical. Pooled 3 seeds (1786704432/1786705413/1786705763, 230.4k bd/vul, 0.07-0.11% fired): plain wash, **PD win both vuls**, sd agreeing in sign — the `plain-wash \| PD-win` ship row. **Isolation gate: 0 foreign boards, both vuls — the campaign's first.** Mechanism: conservative shift off true envelopes (fewer thin NV games; phantom `4♥` corrected to the real fit). Fixed-build seed 1 showed a CI-clear NV-plain loss that seeds 2-3 refuted. The `3♣`→`2♥` re-probe rode the same dumps and closed (wash-to-win). | pooled: NV plain −0.00051 ±0.00072 / PD **+0.00104 ±0.00097**, vul plain +0.00001 ±0.00078 / PD **+0.00112 ±0.00104**; sd-plain −0.00053/−0.00024, sd-PD +0.00065/+0.00076. |
| [N4b `(2♦)` diamond penalty double](archive/one-notrump-competitive-closed.md#n4b--the-2-diamond-penalty-double-built-2026-08-15-sweeping) | `competition.two_diamond_double` (`Option<(min_len, min_suit_hcp, hcp_floor)>`, default `None`) | **measured 2026-08-15 — sweep NULL, stays opt-in, default byte-identical (`18aba5ce…`).** Eight arms (length 4/5/6, floor 8/9/11, quality 0/4/6) around `5:0:9`, two vuls, 230.4k bd/arm/vul, SEED_BASE 1786733434, `scripts/ab-2d-double.sh`. Raw: **all 28 cells CI-clear positive** (plain +0.0016…+0.0048, PD +0.0037…+0.0086) — and **all of it a leak**: `--gate-opener ours` fails at 652/768 (84.9%) foreign, the `their_profile` mirror fallback reading *their* double of our `2♦` overcall through *our* agreement. Owned subset: **no CI-clear cell in 28**, plain +0.0001…+0.0004 (len4 −0.0006), PD −0.0002…−0.0006, n=62–157/cell. Two build findings kept: the **alert is what makes the gate a reading** (unalerted it read `points 8..`, every suit ⊤, identical armed and unarmed — `project_authored` decodes alerted calls only), and over `1NT (2♦) X` ~~they sit 43%~~ — **retracted 2026-08-15**: the count mixed the foreign lane; on our-opened boards the advancer passed 0 of 141 (§N4). Direction if resumed: tighter, start `6:6:11`, buy power (3 seeds × 460.8k → owned n ~600/cell). | owned, NV: len5 plain +0.00024 ±0.00049 / PD −0.00020 ±0.00061; hcp11 plain +0.00043 ±0.00042 / PD −0.00032 ±0.00052; len4 plain **−0.00061 ±0.00063** / PD −0.00038 ±0.00074. Raw (leaked) len5 NV plain +0.0025 ±0.0014 / PD +0.0054 ±0.0017. |
| N4 their `(2♦)` as a Multi | `their.two_diamonds_multi` (disclosure; `bba-gen` derives it from the 2/1 census like `their_2c_landy`, `--their-2d-multi false` = pre-ship arm; engine default undeclared) | **SHIPPED 2026-08-15 — v7, seven rounds** (`scripts/ab-2d-multi.sh`, 230.4k bd/arm/vul, `ab-results/2d-multi{,-v2,-v3,-v4,-v4s2,-v4s3,-v5,-v5s2,-v5s3,-v6,-v6s2,-v6s3,-v7,-v7s2,-v7s3}`). Every raw headline was 60-70% foreign (the mirror-read leak on their double of *our* `2♦`); verdicts are owner-split. v1 (waiting X, both-stopper 3NT, floored continuations) **LOSS** both scorers — the floor sold out with 10+ and raised the relay sign-off to 3NT; v2/v3 (blind 3NT, sign-off passes, doubled tail, double-family + relay fences) plain **win** both vuls / PD wash — the blind blast PD −3.7/−4.3; **v4** (both-stopper blast + authored second call `multi_responder_rebid`) pooled 3 seeds: **vul plain wash \| PD win, NV PD win \| plain loss by 0.00005** — the 8-9 sell-out (plain −2.5/bd, PD +0.8); v5 (natural 2NT invite there) **REFUTED** — PD −0.9/−4.8 per invite, four-way wash; reverted to v4 (opt-in by the letter of the gate). **v4 decomposed per call: X = PD's best call and the whole plain loss, all of it the doubler's second turn.** v6 (BBA's own second turn mimicked whole — takeout X, blind 3NT 9–15, 2NT invite 8–9, 3♠ try, 3m, 4NT; first-turn X `hcp 6+`) plain win / **PD loss** both vuls — the takeout X real (+2.4/+1.6 per fired), the game bids the artifact (PD −2.5 to −6.9); **v7** = v6 minus the game bids (3NT back to stopper-gated) **SHIPPED**: NV `plain wash \| PD win`, vul plain win \| PD wash-leaning-+, both-vul pool win\|win, paired vs v4 better on 3 of 4 cells. | v7 vs base owned: NV plain +0.00019 ±0.00053 / PD **+0.00100 ±0.00067**; vul plain **+0.00061 ±0.00056** / PD +0.00061 ±0.00069; pooled vuls plain **+0.00040 ±0.00039** / PD **+0.00081 ±0.00048**. v7 vs v4 paired: NV **+0.00075 ±0.00031** / +0.00020 ±0.00038, vul **+0.00041 ±0.00034** / −0.00022 ±0.00043. v6 vs base: NV +0.00224 ±0.00054 / −0.00062 ±0.00069, vul +0.00110 ±0.00059 / **−0.00163 ±0.00075**. v4: NV **−0.00055 ±0.00050** / **+0.00083 ±0.00059**, vul +0.00025 ±0.00052 / **+0.00084 ±0.00061**. |
| [N1h / N1i minor-rung re-pricing](archive/one-notrump-competitive-closed.md#n1h--n1i--the-minor-rungs-re-priced-both-refuted-both-opt-in) | `defense_2c_landy_low_minors`, `defense_2c_landy_hcp_rungs` | **both REFUTED 2026-08-15, both opt-in — the lane is closed.** Three shared seeds, 230.4k bd/vul, shared `low-off` baseline (verified board-identical before reuse); `ab-results/landy-low{,-v2,-v3}`, `scripts/ab-landy-rungs.sh`. N1h (cue `points(9..)`, `3m` `points(7..=8)`) = `plain wash \| PD loss`, vul PD **−0.00081 ±0.00074**. N1i (cue `hcp(9..)`, `3m` `hcp(7..=8)`, `2♦`/`2NT` `hcp(..=6)`) = no CI-clear cell, all eight leaning negative. **`cue ← X` negative in both** (−1.80 ×96, −2.96/−4.04 ×46) against N1d's original +2.0…+5.1 the other way — the cue floor is settled, do not probe it again. Leads recorded but not pursued: `Pass ← 2♦` +2.40 PD ×52 (per-seed +4.50/+1.33/−1.09), `3♦ ← 2♦` +3.96 plain/+3.11 PD ×27, `3♣ ← 2NT` −2.19 PD (the transfer's right-siding wins), `cue ← 3♣` −2.88 (shifting a band whole costs more than lowering its floor). | N1h pooled: NV plain +0.00036 ±0.00051 / PD −0.00044 ±0.00066, vul plain +0.00002 ±0.00061 / PD −0.00081 ±0.00074. N1i pooled: NV plain −0.00029 ±0.00043 / PD −0.00039 ±0.00062, vul plain −0.00014 ±0.00052 / PD −0.00036 ±0.00068; sd both arms ≈0. |
| [N1j BBA-ladder counter + weak-2♦ cap](archive/one-notrump-competitive-closed.md#n1j--the-bba-ladder-counter-shipped-default-on-2026-08-15) | `defense_2c_landy_bba`, `defense_2c_landy_weak_2d_cap` | **both SHIPPED DEFAULT-ON 2026-08-15** — the anchor-aligned table replacing the stack (which stays wired behind `--defense-2c-landy-bba false`).  The ladder shipped at its **pre-pinned non-inferiority gate** (rationale: structural alignment; a wash ships) and beat it — zero CI-clear negatives, all 16 DD+sd cells leaning positive; the `2M ← X` guard passed **vacuously** (no hand left the values double for the takeout family, the three-experiments finding untouched); mirror leak 36-38% foreign, depressing (ours-only stronger: NV +182/+215, vul +171/+202 raw IMPs).  The cap shipped at the **standard** gate: plain wash \| PD win both vuls, sd sign-agreed, isolation gate **0 foreign** (second ever), every divergence the predicted `2♦ → Pass` (+2.58/+4.54 PD per fired ×59 — the N1i lead confirmed).  Engine: `2♦ → 3♣` diamond-transfer right-siding (+5.18/+6.06 PD per fired).  Smoke `18aba5ce…` unchanged through the flip; `[their-landy]` fixture re-blessed; 3 seeds 1786753231/1786753518/1786753808, 230.4k bd/vul, `scripts/ab-landy-bba.sh`.  See [closed §N1j](archive/one-notrump-competitive-closed.md#n1j--the-bba-ladder-counter-shipped-default-on-2026-08-15). | ladder (`on↔off`) pooled: NV plain +0.00083 ±0.00085 / PD +0.00083 ±0.00110, vul plain +0.00080 ±0.00100 / PD +0.00073 ±0.00123; sd-plain +0.00080/+0.00103, sd-PD +0.00070/+0.00113. cap (`cap↔on`) pooled: NV plain −0.00003 ±0.00027 / PD **+0.00037 ±0.00033**, vul plain +0.00017 ±0.00024 / PD **+0.00050 ±0.00035**; sd-PD +0.00017/+0.00033. |
| N3 `(3♣)`–`(3♠)` preempt of our 1NT | `competition.nt_high_overcall_responses` (default **true**), `competition.nt_high_overcall_3nt_stopper` (**false** — no gate), `competition.nt_3c_transfers` (**false**) | **SHIPPED DEFAULT-ON 2026-08-18** (`scripts/ab-nt-high-overcall.sh`, `SEED_BASE=1787055415`, 230,400 bd/arm/vul). Owned plain CI-clear both vuls, PD positive both, sd sign-agreeing on all four cells. The private 3NT gate shipped off; `(3♣)` transfers remain a measured wash. The BBA-style double continuation was **REFUTED AND REMOVED 2026-08-19**: NV −0.0012 plain/PD, vul −0.0017/−0.0015, with sd agreeing and all eight CIs below zero; clean isolation. Its `4♦ ← 3NT` row missed game and lost −310/−348 NV, −420/−429 vul. See §N3 for the full history and mechanism. | owned package NV plain **+0.00208 ±0.00126** / PD +0.00079 ±0.00145; owned vul plain **+0.00293 ±0.00160** / PD +0.00180 ±0.00182 |
| N3 opener's answer to responder's takeout `X` | `competition.nt_high_overcall_x_major_at_four` (**true**), `nt_high_overcall_x_leave_in` (**true**), `nt_high_overcall_x_leave_in_three` (**false**) | Round 5 (2026-08-19): the **fit rung SHIPPED DEFAULT-ON**; the **leave-in v1 REFUTED** — sd-lead CI-clear negative in all four cells at −1.75…−2.06 IMPs/fired, clean isolation (0 foreign of 9157/10493). Round 6 (2026-08-20) re-sliced those same dumps by opener's holding in their suit (`--by holding`): the loss is **not uniform** — `len4+` (on the boards the shipped fit rung does not take) is +1.92/+3.58 plain DD and −0.13/+1.09 sd per fired, the honor axis runs the *wrong* way (`has_stopper` = A/Kx/Qxx/Jxxx, so honors at three cards mark the boards whose 3NT was real), and the `4M` block that the shipped fit rung now takes was the worst in the slice. v2 therefore re-gates the knob to `len(over, 4..)` and puts the honor disjunct behind its own `_three` bit so the A/B prices them **separately**. Round 7 (2026-08-20, `nt-answer-x-v3`, seed 1787169600, 716,800 bd/arm/vul) confirmed it out-of-sample: **`length` SHIPPED DEFAULT-ON — CI-clear positive in all eight cells** (plain +0.0071 ±0.0009 NV / +0.0104 ±0.0012 vul = +2.38/+3.41 per fired; PD +0.0085/+0.0125; sd-plain +0.0017/+0.0025; sd-PD +0.0025/+0.0038), **0 foreign boards** at both vuls, 13/13 buckets DD-positive. The strongest cell is `len4+ hon2+` (+4.21/+4.63), which v1's `top_honors(..=1)` gate structurally excluded — honors hurt at three cards (they *are* the stopper) and help at four (the suit cannot run anyway). **`three` REFUTED, kept opt-in**: sd-lead −2.44/−2.99 per fired in isolation, v1's own magnitude; its `plain wash \| PD win` DD signature is the doubling artifact the domain addendum names. Bundled as one gate the package would have measured as a loss at both vuls. Open: reading drift on 12.7%/9.4% of divergences is the only negative cell (−0.97 plain/fired vul). |


### Memory compaction notes (2026-08-16)

- **Stopperless `3NT` escape gate — REFUTED and removed.** Requiring a
  stopper when advancing partner's double fired on 1.79% of 1.6m
  `--filter-1nt` boards and lost plain DD **−0.020 IMPs/board
  (−1.12/fired)** while gaining PD **+0.086 (+4.79/fired)**. BBA usually left
  the failing escapes undoubled; the PD-only gain was a doubling artifact.
- **Gambling games over `1NT (X)` remain opt-in (`b87e314`).** The BBA
  follow-up (128k boards/arm, 19 fired) lost **−4.6/−6.1 plain** and
  **−5.8/−7.4 PD** IMPs/fired (NV/vul): BBA passed 10 of the 19 business
  redoubles, and those boards accounted for the entire −111-IMP loss. The
  gamble won on the roughly six boards where BBA ran, so run-prone opponents
  remain the explicit re-open condition.
- Historical ship commits not otherwise recorded in checked-in prose:
  contested Stayman `98c6c21`, Stayman-defense `6:14` calibration `9312402`;
  Jacoby-transfer competition `c60f96f`; doubled-1NT runout Phase 1
  `5d06184`; penalty-latch persistence `5a2433d` and immediate advancer
  XX-runout `782a4aa`; responder penalty leave-in `ee0077b`, Optional-double
  default `bf6e5cd`, and defensive latch-style arm `cc35135`.
- Superseded statements to ignore if met in old notes: N2's relay-signoff
  `3NT` is caused by `opener_forced_past_invitation`, not a lost ceiling; the
  N4 Multi migration shipped (the stopper ask was measured and refused as a
  default, §N4 residue); diamond-transfer competition is Side A on / Side B
  opt-in; the doubler XX-runout is default-on; Phase 2 shipped the escape
  penalty doubles; and the double style is **Optional > Penalty > Takeout**
  (Optional shipped), not Takeout.
