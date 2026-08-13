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
    ab-results/anchor/2026-08-12-ea2cde9-dirty/american-none \
    --dd-cache ab-results/anchor/dd-cache.json
# add --bucket "2♣" --show 8 for the worst boards of one bucket
```

Bucket the **shipped** arm (`american-*`, the v5 floor), not the
`american-instinct` reference arm.

**Attribution ceiling — read this before quoting a number.** The swing is the
*board's* IMPs, not the interference decision's. On these deals the same 1NT
hand opens at table B too, so the mirrored board carries our *defense to their
1NT* as well, and every later call is in there. The buckets **rank**; they do
not isolate. Isolation is a package's own A/B (`bba-gen --filter-1nt`, one
knob). The confound is broadly common across buckets, which is what leaves the
ranking usable.

### 2026-08-14, anchor `2026-08-12-ea2cde9-dirty`, 204,800 boards/vul

We open 1NT on **6.5%/6.7%** of boards; RHO contests **12.4%/10.4%** of those
(NV/vul) — so a contested 1NT is **0.80%/0.69% of all boards**.

| RHO | boards (NV+vul) | plain total | plain/bd | PD/bd NV | PD/bd vul |
| --- | --- | --- | --- | --- | --- |
| **`2♣`** Landy | 551 | **−406** | **−0.74** | **−0.70** | +0.08 |
| `2♦` Multi | 794 | −322 | −0.41 | −0.02 | +0.90 |
| `2♠` Muiderberg | 430 | −284 | −0.66 | −0.58 | +0.30 |
| `3+` | 405 | −254 | −0.63 | +0.01 | −0.13 |
| `X` Woolsey | 364 | −187 | −0.51 | −0.08 | +0.97 |
| `2NT` unusual | 118 | −139 | **−1.18** | −0.72 | −0.86 |
| `2♥` Muiderberg | 393 | −43 | −0.11 | −0.09 | +1.06 |
| **all contested** | 3055 | −1635 | −0.74 / −0.30 | −0.26 | **+0.48** |
| **uncontested 1NT** | 23868 | — | **+0.09 / −0.02** | — | — |

**Three findings.**

1. **The lane's whole headroom is ~0.004 IMPs/bd.** Contested costs
   −0.82 NV / −0.28 vul relative to *uncontested*, on 0.80%/0.69% of boards.
   Nothing here closes an anchor gap; this is hygiene and disaster removal at
   the standard ship gate, as scoped.
2. **Contested 1NT is above board average**, not a leak — −0.74/−0.30 against
   an anchor board average of −0.95/−1.16. The 1NT opening is one of our better
   boards even when contested.
3. **`2♣` is the one bucket that loses on both scorers** (plain −0.74, PD −0.70
   NV) and carries the largest total. `X` is *fine* (−0.05/bd vul, PD +0.97) —
   the floor's runout pays even though their double is Woolsey, not penalty, so
   that open question is closed. `2♦` is mild (PD −0.02 NV, **+0.90** vul).

### Named mechanism — why `2♣` loses

Over their `2♣` we play a **systems-on rebase**
([lebensohl.rs:388-405](../src/bidding/american/competition/lebensohl.rs)):
their `2♣` is stripped to a Pass and our whole uncontested response structure
goes live, with `X` transplanted onto the stolen `2♣` Stayman
([lebensohl.rs:416-425](../src/bidding/american/competition/lebensohl.rs)).
Against a *natural* club overcall that is sound and standard — `2♣` is the one
overcall that costs no space.

Against **Landy** it is actively bad. The worst boards show the structure
firing into a hand that has just shown both majors:

```text
us:  - 1NT 2♣ 2NT - 3♦ 3♠ 4♦ - 4♠ X - - -      [−18 IMPs]
us:  1NT 2♣ 2♦ 3♣ X 3♠ - 4♠ X - - -            [−10 IMPs]
us:  1NT 2♣ 2NT 3♥ X - 3NT - - -               [−10 IMPs]
```

`2♦` is a Jacoby transfer **to hearts** — one of the two suits they hold. `2NT`
and `2♠` are the minor transfers, pure constructive asks that hand them a free
run at their fit. `X` asks for a four-card major against a hand holding both.
Two of the eight worst boards end in `4♠` doubled.

### Second observation — `2NT` (worst per-board, small n)

118 boards pooled, so the CI is enormous, but the sign is consistent across
vulnerabilities (−1.18/bd plain, −0.72/−0.86 PD). The forensic pattern is that
BBA **doubles** their minors and we bid on:

```text
us:  1NT 2NT X 3♦ - - -
bba: 1NT 2NT X 3♦ - - X - - -
```

Worth a look once the top buckets are done; not actionable at this n.

## Coverage inventory

### Lane 1 — `1NT (X/2x)`, they interfere directly

| RHO | Owner | Anchor |
| --- | --- | --- |
| `(X)` | **floor, complete** — escape, business XX, 2NT scramble, SOS, balancing runout, encircling doubles | `instinct.rs:4641-4956`; `set_one_nt_runout` default-on, +0.039/+0.053 plain, 1.58% fired |
| `(2♣)` | systems-on rebase + stolen-Stayman `X` | `lebensohl.rs:388-405`, `:416-425`, `:436-446` |
| `(2♦)` | Transfer Lebensohl's Stayman/Smolen/Jacoby/Leaping-Michaels leg | `lebensohl.rs:450-462`, `rubensohl.rs:332`; continuations `lebensohl.rs:584-608` |
| `(2♥)`, `(2♠)` | Cohen Transfer Lebensohl | `lebensohl.rs:466`, `rubensohl.rs:98`; continuations `:550-566`, `:573-578` |
| `(2NT)` | Unusual-vs-Unusual | `uvu.rs:139`, `:21`, `:145-161` |
| `(3♣)`+ | **floor** — `high_overcall_responses` covers suit openings only | `high_overcall.rs:152` |

Multi counter exists but is **half-built**: `defense_2d_multi`
(`agreements.rs:171`, default `false`) swaps in `multi_responder`
(`lebensohl.rs:256`), but the continuation block at `lebensohl.rs:584` fires on
`style == Transfer && over == Diamonds` **without checking the Multi flag** —
so with the knob on, opener answers a natural `3♦` with
`transfer_completion(Hearts, ♦)`. Same mismatch for `Plain` + Multi at `:540`.
Dormant only because the knob defaults off.

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
| **N1** | **Landy `(2♣)` counter** | new | top loser, both scorers; mechanism named above |
| N2 | Muiderberg `(2♠)` calibration | — | −0.66/bd, PD −0.58 NV |
| N3 | `(3+)` overcalls of our 1NT | new | floor-only today; −0.63/bd |
| N4 | Multi `(2♦)` — finish + measure | `defense_2d_multi` | exists, half-built, gate bug above; plain-only loss |
| N5 | Complete Jacoby, re-measure | `competition_over_transfer` | default-off on a measured loss *while missing its two most-fired cells* — a half-built loss, resumable |
| N6 | `(2NT)` penalty discipline | `uvu_encircle` et al. | worst per-board rate, n=118 — needs boards before it needs code |
| N7 | Absent responses contested | new | Puppet `3♣`, `3♦`, splinters, `3NT`, Texas, `4NT` — rarest in the system |

## N1 — the Landy `(2♣)` counter (authored 2026-08-14, `defense_2c_landy`, default off)

Responder's table at `1NT (2♣)`, replacing the systems-on rebase
(`landy_responder`, `competition/lebensohl.rs`):

| Call | Meaning | Weight |
| --- | --- | --- |
| `3♣` / `3♦` | natural forcing, **6+** suit, `points(10..)` | 175 |
| `3NT` | game values, `points(10..)`, **no stopper gate** | 170 |
| `X` | **values**, `hcp(8..)`, penalty-oriented — alert `comp:landy-values` | 145 |
| `2♦` | weak natural, 5+, `points(..=9)` + the `natural_floor` | 140 |
| `2NT` | natural invite, `points(8..=9)` | 130 |
| Pass | finite catch-all | 0 |

Opener's single answer: `1NT (2♣) X -` → **Pass**, sitting for the values
double. Everything deeper is the floor's, which is where we want it — the floor
already encircles their escape from our double
(`penalize_escape_stack`/`penalize_escape_values`, both default-on).

**Three design points, two of them found by the smoke run rather than by
reasoning:**

1. **Either/or with the rebase, not an overlay.** Leaving the systems-on rebase
   registered would strip their `2♣` and remap our values `X` back onto the
   stolen `2♣` Stayman one round later, routing `1NT (2♣) X (2♥)` into the
   contested-Stayman package. That is the same defect as the `defense_2d_multi`
   gate bug above, so the knob switches the whole node.
2. **`3NT` takes no stopper gate.** The first draft used `author_direct_3nt`,
   which keys its stopper *and* its trap-pass test on the overcall's suit — but
   their `2♣` is artificial and promises no clubs, so a club stopper is not the
   question and a club honour-stack is no reason to trap. Demanding a *major*
   stopper is no use either: they hold both. Leaving `3NT` at plain game values
   also keeps it where systems-on had it, so the A/B tests the double rather
   than 3NT discipline.
3. **`X` is the residual, floored on `hcp` not `points`.** It ranks below every
   hand with an offensive direction (the 6-card minors, `3NT`) and above those
   without (`2♦`, `2NT`). Defending does not care about distribution, and a
   `points` floor would drag the shapely weak hands that belong in `2♦` into a
   double.

The 3-level minors are ranked *above* `3NT` on purpose: with both majors against
us, whether `3NT` is playable turns on opener's major holdings, which only opener
can see, so showing a six-card source of tricks and letting opener choose beats
guessing. Opener's reply to that is the floor's, and reads correctly because the
bid is natural.

**Smoke, 4,000 `--filter-1nt` boards paired (seed 424242, vul none):** 46 of
8,000 tables diverge (0.57%). Responder's call over their `2♣` moves
`2♠` **11 → 0** — the two-way minor transfer fired at a both-majors overcall is
gone — with those hands redistributing to Pass (+11) and `3NT` (+9). Sample
divergence, the census mechanism reversing:

```text
on : 1NT 2♣ X - - -                 (we double and defend)
off: 1NT 2♣ X - 2♦ 2♠ - - -         (opener answers stolen Stayman; they find 2♠)
```

Not a verdict — `scripts/ab-landy-counter.sh` is the A/B, unrun.

**Inertness proven**: `smoke-default --count 20000 --seed 1` SHA-256
`8ea2f5678a733cfe3ead79411d9cb31b8e95d37de52236e597fc38f9dec82bbb`, identical at
HEAD and with the change. The default system is byte-identical.

**Disclosure**: BBA's `.bbsa` schema has rows for whether *we* play Landy /
Multi-Landy against *their* 1NT, but none for our counter to *their* Landy over
*our* 1NT. Nothing to wire in `card.rs`; the golden cards and
`alert-sites.txt` are unchanged (the knob is default-off, so the alert site is
not in the default system).

## N1b — the GF minor cues (`defense_2c_landy_cues`, authored 2026-08-14, default off)

The overlay that makes N1's remaining free space earn its keep, and the
falsifiable half of the **`1♣ (2♣)` analogy** (the theory that a counter to a
both-majors overcall of a balanced-ish partner should carry the
Unusual-vs-Unusual cue skeleton — see the verdict below). A pure addition to
`landy_responder`; requires `defense_2c_landy`:

| Call | Meaning | Weight |
| --- | --- | --- |
| `2♥` (cue) | GF, **5+ clubs** — alert `comp:landy-cue` | 173 |
| `2♠` (cue) | GF, **5+ diamonds** — alert `comp:landy-cue` | 172 |

Ranked below the 6-card forcing naturals (a one-suiter still shows its source
of tricks at 175) and above the ungated `3NT` (170), so the *5-card* game
hands — which under N1 must guess a stopperless `3NT` or stretch the values
`X` — start low and let opener place the contract. Opener's one answer
(`landy_cue_answer`): `2NT` with both majors stopped, else raise the named
minor (the raise is the finite catch-all — opener is balanced, so it never
lands on fewer than two). Everything deeper is the floor's.

**Documentary basis (web survey 2026-08-14).** No published source draws the
`1♣ (2♣)` analogy outright — the UvU literature scopes itself to suit
openings — but the strongest expert counter-Landy structures independently
reproduce its two load-bearing components: `X` = values / "can double at
least one of their suits" (Cohen, Walker — N1's authored `X`), and cues = the
two unshown suits, exactly this scheme (Cohen's advanced structure: `2♥` = GF
clubs, `2♠` = GF diamonds). Where the record diverges is where the captaincy
disanalogy predicts: over a wide-range nebulous `1♣` the UvU cues grade
*raises* and the double stays informational (WJ2005 Sputnik), while opposite
a narrow 15-17 the raise half is re-spent on the `2NT`/`3NT` ladder and the
penalty axis is promoted. So the theory holds as *isomorphic, not identical*,
and the published cue meanings are four-way incompatible (minors / stoppers /
natural / other-major Stayman) — which is what the third A/B arm decides.
The minor-opening side of the same skeleton is P7 in
[competitive-book.md](competitive-book.md) (`set_uvu_over_minors`) — authored
for coherence, unmeasurable vs the anchor (BBA never cues over a minor;
def1-c/d probes 2026-08-14).

**Deferred candidate N1c**: a Lebensohl `2NT` relay (weak sign-offs at `3♣`/
`3♦`, hands that under N1/N1b must pass), the one lit-standard component both
arms lack. It would move the natural invite, so it is its own package and its
own A/B, only worth boards if the census still shows weak-minor passes
leaking after N1/N1b ship or die.

**Inertness**: `smoke-default --count 20000 --seed 1` SHA-256 `8ea2f567…`
identical at HEAD with both knobs off — the N1 reference hash, unchanged.

## Measurement discipline

- **Counter-defense isolation gate:** on identical seeded deals, configuring
  the candidate against a natural defense must leave the auction dump
  byte-identical to the natural baseline.  Require natural interference to
  occur and the targeted artificial-defense arm to diverge, so the check is
  not vacuous; a face-call-wide reinterpretation (`2♣` always means Landy)
  fails this gate.
- **Enriched probing** is the default here: `bba-gen --filter-1nt` (raw-hand
  gate, balanced 15-17 somewhere, applied *before* any bidding). Headline is
  IMPs per **accepted** deal; publish `per-board = conditional mean × trigger
  density` alongside and scale the CI the same way. Compare IMPs/divergent, not
  IMPs/board.
- One knob = one measured change; arms **sequential**, fresh
  `SEED_BASE=$(date +%s)` per experiment shared across its arms, never rebuild
  in flight.
- Any **reading** change is a second, separate A/B on the same enriched boards,
  so a loss attributes to calls or to reading, never to their sum.
- Ship rule: standard gate. Plain-DD wash + PD gain ships default-on; a CI-clear
  plain loss stays opt-in with the default byte-identical, and the leak gets
  named in the ledger.

## Ledger

| Package | Knob | Status | Verdict (plain / PD, IMPs) |
| --- | --- | --- | --- |
| census tool | — | **shipped** | read-only; picked N1 over the pre-census guess |
| N1 Landy `(2♣)` counter | `defense_2c_landy` | **authored, A/B queued** — default off, inertness proven | fires 0.57% of tables on `--filter-1nt`; `scripts/ab-landy-counter.sh` |
| N1b GF minor cues | `defense_2c_landy_cues` | **authored, third A/B arm queued** — default off, rides N1's knob, inertness proven (same smoke SHA) | `scripts/ab-landy-counter.sh` now 3 arms: off / N1 / N1+cues — on↔off prices the counter, cues↔on prices the analogy's delta alone |
