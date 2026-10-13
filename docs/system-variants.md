# System variants: `basic` and `advanced`

**Status: built and measured 2026-10-14 (commit 44283685); verdict in [§5a](#5a-verdict-2026-10-14).** `Agreements::basic()` /
`advanced()`, `american_basic()` / `american_advanced()`, the `--ns-preset`
harness flag, `cards/American-Basic.bbsa`, the web `system_preset` row and
the two pins (`basic_differs_from_default_only_on_the_listed_fields`,
`basic_alert_set_is_the_whitelist`) are on `main`; the default system is
byte-identical (`smoke-default --count 20000 --seed 1` vs HEAD).  jdh8's
decisions are recorded in [§7](#7-decisions-owed-to-jdh8); the hole census
that reshaped §4 is in [§4a](#4a-the-hole-census-2026-10-14).
`scripts/ab-system-variants.sh` runs the three arms of [§5](#5-measurement).

## 1. The idea

IntoBridge ships many of its systems in a **basic** and an **advanced**
version, and the basic one is what a beginning player can sit down with
(their robot even changes its lead agreements between the two: fourth-best
against suits in the basic systems, third/fifth in the advanced ones). Do the
same for `american()`:

- **`basic`** — the 2/1 core with the **deeply convoluted conventions
  dropped**; every other call natural. jdh8 picks which to drop (§4 is the
  candidate list); the test is *how much a beginner must memorise*, not
  *expert consensus*. (The first draft of this doc used Bridge World
  Standard 2017 as the reference; that is the wrong card — Wikipedia's own
  summary calls BWS 2/1 "with many advanced treatments", its complete text
  is paywalled, and a consensus of experts is what `advanced` already
  approximates. BWS stays in [§8](#8-sources) as a *not this*.) The nearest
  public teaching card is the ACBL SAYC booklet; BBO's own split is the
  precedent: BBO Basic ≈ SAYC, BBO Advanced ≈ 2/1.
- **`advanced`** — the champion: every convention that measured a win is on.

**Framing (jdh8, 2026-10-14, after the measurement): `basic` is an ace
playing a beginner's system.** The card is simple; the judgement is the
same precise engine as `advanced`. The two variants differ only in what a
player must memorise, never in how well the robot judges — and the §5a
numbers say the judgement is worth fifteen times the card.

Today `american()` is `advanced` in all but name. Per the ship policy in
[bidding-options.md](bidding-options.md) a measured win is default-on, a wash
is opt-in, a loss is off, so `Agreements::default()` *is* "all winning
conventions". The whole idea therefore reduces to **one new `Agreements`
value**, `basic`, and names for both.

"Simple" has an objective proxy in this codebase: the set of **alerted
calls** in the book. Every artificial call carries an `.alert(...)` (the
`artificial_calls_are_alerted` invariant), and a beginner at the web table
reads exactly those alerts. So `basic` owes a test that its alert set equals
a fixed whitelist — Stayman, Jacoby transfers, `2♣`/`2♦` waiting, negative
and takeout doubles, RKCB, Jacoby 2NT, splinters, cue raises, Michaels and
the unusual 2NT, plus whatever [§7](#7-decisions-owed-to-jdh8) adds. That
makes "basic" a checked property, not a vibe, and a new default-on gadget
cannot silently join it.

## 2. Mechanism — a variant is an `Agreements` value, not a factory

The architecture already says it ([bidding-architecture.md § Knobs](bidding-architecture.md#knobs)):
"an A/B arm is one agreements value and one partnership". A system variant is
the same thing with a name. So:

| Surface | `advanced` | `basic` |
| --- | --- | --- |
| `Agreements` | `Agreements::default()` (alias `advanced()`, documented, no second body) | `Agreements::basic()` — `default()` with the fields of [§4](#4-the-knob-partition) overwritten |
| Factory | `american()`; `american_advanced()` is `american(&Agreements::advanced())` | `american_basic()` is `american(&Agreements::basic())` — one call each, no second body |
| Harness (`bba-gen`, `render-book`, `probe-call-reading`) | no flag | `--ns-preset basic`, applied **after** the `--ns-*` overlay (`common::apply_preset`). Every binary assigns each knob flag unconditionally, defaults included, so a preset laid down first would be clobbered back to the shipped system; the preset therefore overrides the knob flags and keeps only `decision.their`. The seat name stays `american`, so `--our-floor american-instinct --ns-preset basic` is the floor ablation with no third seat name |
| Card | `cards/American.bbsa` | `cards/American-Basic.bbsa`, blessed by `bba-card --system american-basic`, pinned by the same staleness test; it differs from the advanced card in ten rows (Checkback, Garbage Stayman, Jordan, Leaping Michaels, NMF, responsive double, Reverse Drury, Rubensohl after double, support double, two-way NMF) |
| Web | the individual toggles | the `system_preset` `Choice` row at the top of `SETTINGS` (section "System"): basic / advanced / custom. `set` rewrites every knob but `decision.their`; `get` reports `custom` when the live vector equals neither preset. A beginner starts on `basic` and turns gadgets on one at a time |
| Floor | v6 net | v6 net (see [§3](#3-the-floor-is-the-real-cost)) |

What this deliberately does **not** build, with the lesson that forbids it:

- **No second factory, book, module tree or `Family`.** The retired `dutch()`
  ([archive/dutch-system.md](archive/dutch-system.md)) was a reuse-and-override
  factory and still never beat `american()` as a whole; every win it found was
  a per-gadget knob. A system is a knob vector. Keep it one.
- **No new knobs for the split itself.** `basic` is assembled from knobs that
  exist. Where a beginner card wants a state we have not authored (§4's
  "hole" rows), that is a normal `author-convention` ticket, measured on
  `american()` like every other.
- **No "basic = everything off".** An off-state that is unauthored falls to the
  floor, and a net floor with no book above it is not "natural", it is
  whatever the net learned ([bidding-architecture.md](bidding-architecture.md)
  shadowing rule). Rule: **`basic` picks the natural *authored* state of each
  knob**. Verified instance: `LebensohlStyle::Off` is "responder falls to the
  instinct floor (no Lebensohl node)" — a hole, so `basic` keeps `Plain`
  until a natural `1NT (2x)` table is authored, even though Lebensohl is not
  beginner material.

The default system stays byte-identical: the only touched default path is a
`Default` impl that gains a sibling constructor. Proof at ship time is the
standard seeded `smoke-default` diff ([measurement.md](measurement.md) item 12).

## 3. The floor is the real cost

The contested books stand on the v6 net, whose convention-regime input is
`ConventionCard::capture(agreements)` — both sides, 28 compact slots, of which
**13 per side are live** after the export fold ([card-manifold.md](ai-bidder/card-manifold.md)):
`dutch`, `relocating`, `nmf`, `xyz`, `jordan`, the `shape` / `defense` /
`lebensohl` one-hots, `minors`, `landy`. Everything else on the card is a
corpus-constant column, folded to zero: the net cannot see it.

Consequences for `basic`:

1. The gadgets `basic` turns off that sit on **live** slots (XYZ, NMF,
   Jordan, 1NT minor scheme) are regimes the net was trained across. The
   floor follows them for free, and the sibling-factory rule holds by
   construction (same net on the declared and undeclared paths, because
   `basic` is reached through `american()`).
2. The gadgets on **folded** slots (support doubles, responsive doubles,
   Leaping Michaels, splinters over 1NT, Garbage Stayman…) are invisible to
   the net. Its *auction* inputs are still honest, because readings come from
   the book that `basic` builds, so partner's calls are read as `basic`
   means them. What is stale is only the prior over what *we* would have
   done, i.e. the floor may continue an auction as if the advanced gadget
   were available. This is the reading-knob-under-a-neural-floor mechanism
   that sank 5542 (2026-09-24), in a milder form: there a reading changed
   under the net; here only an unseen regime bit does.
3. The deterministic `instinct()` floor sees no regime at all and plays
   natural; it was the pre-swap baseline. `american_instinct(&basic())` is
   the honest comparison arm when 2 is suspected on a traced board.

Ranked response, cheapest first:

- **Ship `basic` on the v6 net and measure** ([§5](#5-measurement)). If the
  trace of its worst boards shows the floor continuing advanced gadgets, the
  fix is a rail or an authored natural continuation, both knob-measurable.
- A **`basic` corpus** (one `dump` run under `basic`, same teacher) and a
  retrain that thaws the folded slots. Retrain-gated, so deferred with the
  rest ([next-steps.md § Owed](next-steps.md#owed--deferred)). Side benefit:
  this is the "second system's corpus" M5.3 has waited on
  ([ai-bidder/plan.md](ai-bidder/plan.md)), at a fraction of Polish Club's
  cost, because the reader and the net stay american-tuned.

## 4. The knob partition — decided 2026-10-14

jdh8 took the defaults: §4 as drafted, keeping inverted minors, splinters
and Ogust, dropping Jordan 2NT; Texas is a transfer without a knob (only
its slam drive, `texas_slam_drive`, is a knob) so it stays by the
authored-state rule.  The rows below are the draft; the field names the
constructor actually moves, and the three rows the hole census reversed,
are in [§4a](#4a-the-hole-census-2026-10-14).  The pin
`basic_differs_from_default_only_on_the_listed_fields` is the live list.

Criterion per row: **is it deeply convoluted?** The proxy is the size of the
continuation tree a player must carry — count a gadget's authored nodes and
alerted calls with `render-book --prefix <auction>`; a gadget whose tree is
one artificial call and a natural answer (Stayman, a transfer) is cheap, one
whose tree is a ladder of relays and pass-or-correct steps (the N4 Multi
structure, Transfer Lebensohl, the Stayman slam tries) is convoluted. Rows
below are **candidates**; the final list is jdh8's call
([§7](#7-decisions-owed-to-jdh8)), and "line call" marks the borderline ones. Default-on natural knobs (the `fold into
base` rows of [bidding-options.md](bidding-options.md)) are untouched in both
variants unless listed. Numeric ranges (Tier B) are untouched: a beginner
card and the shipped system agree on 1NT 15–17, 2NT 20–21, `2♣` with `2♦`
waiting. **hole** marks a row whose natural off-state must first be checked
to be authored (else it is an `author-convention` ticket, not a flip).

### Stays on in `basic`

2/1 game forcing, forcing 1NT, Jacoby 2NT with shortness rebids, limit
raises, splinters (**line call**), inverted minors (**line call** — SAYC does
not, most 2/1 courses do), weak jump shifts, weak twos with one asking bid
(**line call**: Ogust vs feature), Stayman, Jacoby transfers, `floor_rkcb`
with `rkcb_variant = Plain` (1430 is fixed in the asker tables; a beginner
0314 would be a new knob — not worth it), negative doubles, takeout doubles,
cue-bid raises, Michaels and the unusual 2NT, 1NT overcall systems-on,
Lebensohl `Plain` (kept only because `Off` is a hole, see §2),
`strong_two_waiting`.

### Switched to the natural authored state in `basic`

| Knob (today's default) | `basic` | Note |
| --- | --- | --- |
| `decision.reading.xyz` (on), `rebid.new_minor_forcing` (off) | both off | **hole**: verify `1m - 1M - 1NT - 2♣` is an authored natural call with both off; if not, NMF on (it pairs with `up_the_line`, which shipped *coupled* with XYZ — joint win, each alone a loss — so NMF + `up_the_line` must be measured, not assumed) |
| `competition.lebensohl_style` (Transfer) | `Plain` | `Off` is a floor hole |
| `decision.reading.notrump_minors` (Puppet) | **hole** | both authored schemes are artificial; a natural `1NT - 3m` is the beginner call and is unauthored |
| `notrump.garbage_stayman`, `crawling_stayman`, `stayman_both_majors`, `stayman_cue_continuation`, `stayman_minor_slam_try`, `transfer_gf_majors`, `transfer_gf_hearts`, `transfer_longer_major`, `invitational_5card_majors`, `diamond_splinter`, `minor_transfer_slam_try`, `transfer_slam_try`, `texas_slam_drive` (on) | off | the 1NT gadget layer; Stayman promises 8+, transfers complete, Texas is a **line call** |
| `competition.competition_over_stayman`, `competition_over_minor_transfer`, `competition_over_diamond_transfer` (on) | off | authored interfered tails of the gadgets they follow |
| `competition.uvu`, `uvu_over_majors`, `one_nt_runout`, `one_nt_runout_universal`, `advancer_xx_runout`, `doubler_xx_runout`, `penalize_escape_*` (on) | **hole** | a beginner plays "system on over their double"; verify the off-state of the runout family is that and not a floor hole |
| `competition.multi_kokish_kraft`, `multi_minor_slam_try`, `landy_minor_slam_answer` (on) | off | our N4 structure vs their Multi; the natural leg stays. Their-card disclosures (`decision.their.*`) are about *them* and are unchanged |
| `competition.jordan_truscott`, `major_support_double`, `splinter_doubled`, `second_suit_agreement`, `two_over_one_fit`, `limit_raise_acceptance`, `major_choice_of_games`, `forcing_nt_two_suiter` (on) | off | intermediate structure; each off-state **hole**-checked |
| `defense.leaping_michaels_enabled`, `responsive_takeout_enabled`, `rich_advance_double`, `longest_first_advance`, `advance_minor_jump`, `advance_2nt_continuation` (on) | off | advance structure beyond a first course |
| `notrump_defense` (Natural) | `Natural` | a beginner's defense to 1NT is natural overcalls; nothing to flip |
| `response.drury`, `drury_splinters` (on) | off | not first-course |
| `response.forcing_nt_jump_shifts` (on), `rebid.meckstroth_adjunct` (off) | both off | opener's jump shift is natural and strong |
| `opening.weak_two_eval` | unchanged | a gate, not an agreement |

### 4a. The hole census (2026-10-14)

`render-book` under both presets, every authored node diffed: advanced
7887 nodes, basic 5445, nothing basic-only.  Checked prefix by prefix
before the flips were kept:

| Row | Finding | Decision |
| --- | --- | --- |
| `xyz` off | `1x - 1y - 1z` responder rebid **unauthored** (0 nodes, the knob doc says so) | `rebid.new_minor_forcing` **on** for the four `1m - 1M - 1NT` slots (its alert `new-minor-forcing` joins the whitelist); the six other one-level auctions XYZ owned are a hole, floor-owned |
| `splinter_doubled` off | opener's rebid over a doubled splinter falls to the floor and passes (the knob's own reason to exist) | **stays on** — an interfered tail of a kept convention, not a gadget |
| `second_suit_agreement` off | `1M - 2r - 2x - 3x` opener's third call falls to the floor | **stays on** — same reason |
| `forcing_nt_jump_shifts` off | the knob **is** the natural strong jump shift (on = `3x` natural 18+, off with Meckstroth off = no 18+ side-suit rebid at all: an 18-count bids `2♣` as a minimum) | **stays on**; the draft row had it backwards |
| `limit_raise_acceptance` off | `1M - 3M -` opener has no table (44 nodes gone) and falls to `instinct()` | off — the constructive floor is deterministic and natural; the ladder's whole win was its keycard ask |
| `jordan_truscott` off | systems on: their `X` is stripped to a pass | off, natural |
| `lebensohl_style = Plain` | `1NT (2♥)` keeps 6 of 10 nodes | as designed (`Off` is a hole) |
| `notrump_minors` | both schemes artificial | Puppet stays (hole: a natural `1NT - 3m`) |
| runout family (`decision.instinct.one_nt_runout*`, `advancer_xx_runout`, `penalize_escape_*`, `uvu*`) | floor rules, not book agreements | unchanged (hole: "system on over their double" unauthored) |
| `major_support_double` off | the minor-opening support doubles have no knob and stay on, so basic plays support doubles after `1m` but not after `1♥ - 1♠` | off as drafted; **flagged** — a one-line revert if jdh8 wants consistency |
| Smolen, `rebid_checkback`, `fourth_suit_forcing` | alerted, no knob or a shipped win with no natural off-state | stay (`smolen` is a line call with no knob) |

Nodes that vanish, by prefix (advanced-only, largest first): `1♠ 1NT -`
641 (the 1NT-overcall gadget layer), `1♠ - 1NT` 571 and `1♥ - 1NT` 349
(the forcing-NT two-suiter tails), the passed-hand lane 326, `1NT - 2♣`
243 and `1NT - 2♥` 184 (Stayman/transfer gadgets), the takeout-double
advances (`1x X -`) 142.  These are the gadgets' absences, which is the
point; run 1's trace ranks the ones that cost.

Knobs **already opt-in** (the 53 wash/loss rows) stay off in both variants,
so `advanced` ≡ `default()` with no edits. If a future win is shipped
default-on, `advanced` moves with it; `basic` does not unless it passes the
whitelist of §1. Two tests pin this: **`basic()` differs from `default()`
only on fields listed here**, and **`basic`'s alert set equals the
whitelist**.

## 5. Measurement

`basic` is a bidding change for whoever selects it, so it owes an A/B before
the name ships ([measurement.md](measurement.md)). Three runs, in order, one
`SEED_BASE`, arms sequential on the shared box — `scripts/ab-system-variants.sh`
(arms `advanced`, `basic`, `basic-instinct`, all vs BBA 2/1 on paired deals;
run 2026-10-14 into `ab-results/system-variants`, verdict in [§5a](#5a-verdict-2026-10-14)):

1. **`american-basic` vs `american`, self-play**, both scorers. Expected: a
   loss — that is the point. It is the first measurement of the whole gadget
   layer's worth, a number no campaign has produced (each knob was measured
   against the rest of the layer, never the sum). It is also the number a
   beginner is told when the web preset is explained: "advanced is worth
   about *x* IMPs/board over basic".
2. **`american-basic` vs BBA on its 2/1 card** (`scripts/anchor.sh` with the
   new seat name). This reads how good our *natural* bidding and the floor's
   judgement are once gadgets stop paying. Buckets where `basic` loses and
   `advanced` does not are the gadgets' real homes; buckets where both lose
   are judgement and belong to the floor campaigns.
3. **`american-instinct` with `basic()` vs `american-basic`**: the floor
   ablation under the natural book, the §3 question answered directly.

Ship gate for the name: run 1 traced (worst divergent boards are gadget
absences, not unauthored holes), `smoke-default` byte-identical, the card
golden re-blessed, `cd web && cargo test`.

### 5a. Verdict (2026-10-14)

`scripts/ab-system-variants.sh`, SEED_BASE 1791913599, 204,800 boards per
arm per vulnerability, all arms vs BBA 2/1 on its card with ours generated
from the arm's agreements (`told: generated`), 33 minutes wall-clock.

| Pair | Vul | Fired | Plain DD IMPs/board | PD IMPs/board | IMPs/fired (plain / PD) |
| --- | --- | --- | --- | --- | --- |
| basic vs advanced | none | 11.48% | −0.0179 ±0.0086 | −0.0699 ±0.0094 | −0.156 / −0.608 |
| basic vs advanced | both | 10.64% | −0.0244 ±0.0107 | −0.0797 ±0.0119 | −0.230 / −0.749 |
| basic-instinct vs basic | none | 26.74% | −0.2943 ±0.0133 | −0.3378 ±0.0150 | −1.101 / −1.263 |
| basic-instinct vs basic | both | 24.73% | −0.3696 ±0.0165 | −0.4803 ±0.0187 | −1.495 / −1.942 |

Read:

- **The gadget layer is worth about 0.02 IMPs/board plain, 0.07–0.08 PD**
  (run 1; every CI clear of zero). That is the first measurement of the
  layer as a sum. The PD figure is three to four times the plain one, so a
  good part of what the gadgets buy is *not being doubled* — the doubling
  bracket the PD scorer prices. The number a beginner is told: "advanced is
  worth roughly a fiftieth of an IMP per board over basic, more if your
  opponents double well".
- **The v6 net floor is not the problem** (run 3): under the natural book
  it beats `instinct()` by 0.29–0.48 IMPs/board, fifteen times the gadget
  layer. The §3 worry — that a net trained with the gadgets on would play
  the natural book dishonestly — does not show at this resolution; the
  basic corpus retrain stays deferred with the rest.
- The worst run-1 boards are high-level contested doublings (`2♠ - 2NT (4NT)
  - (5♥) X`, a 2/1 auction they sacrifice over) where **BBA's own action
  differs** because the card it is told differs — a disclosure effect, not
  an authored hole. The per-node attribution is below.

**Attribution** (`ab-dump-bucket --by node|side`, files
`ab-results/system-variants/bucket.*.txt`). The loss is diffuse: the top 30
of ~15k node buckets carry 23–27% of the PD loss. By side:

| Side | NV plain / PD IMPs | Vul plain / PD IMPs | Reading |
| --- | --- | --- | --- |
| we open, we diverge | −3643 / −10159 | −5387 / −12648 | the whole plain loss: constructive gadgets |
| they open, we diverge | +58 / −4101 | +347 / −3584 | plain **wash**, PD loss: the defensive gadgets (Leaping Michaels, responsive and rich advance doubles) pay only in the doubling bracket |
| they diverge (either opener) | −91 / −47 | +35 / −92 | disclosure noise, negligible |

The named node clusters, both vulnerabilities, worst PD first:

1. **The six one-level auctions XYZ owned that NMF does not cover** —
   `1♣ - 1♦ - 1♥ -`, `1♦ - 1♥ - 1♠ -`, `1♣ - 1♥ - 1♠ -` and their
   passed-hand twins, responder passing or raising (`2♥ vs -`, `2♠ vs -`,
   `2♠ vs 2♣`): about −1,200 PD NV, −1,500 vul across the cluster, at
   −3 to −8 PD per fired. **This is the §4a hole and the one ticket the
   run produced**: an authored natural invitation (a raise with
   invitational values, `2NT`) over the third one-level bid, knob-measured
   on `american()`.
2. Gadget absences, which are the point — Garbage Stayman (`1NT - ⟨- vs
   2♣⟩`, ~−200/−260 PD), Reverse Drury (`- - 1♠ - ⟨1NT vs 2♣⟩`,
   −133/−267), NMF vs XYZ over `1m - 1M - 1NT` (−115/−141 each slot),
   Jordan 2NT (`1♠ X ⟨1NT vs -⟩`, −122/−227), the 1NT-overcall cue
   (`1NT - 2♥ ⟨X vs -⟩`).
3. Disclosure effects: `2♠ - 2NT (4NT)` and `1♠ - 2NT (4NT)` where BBA's
   own action over our card differs, a handful of boards at −8 to −10 each.

Ship gate for the name: run 1 traced — the worst buckets are gadget absences
plus one recorded hole (item 1), `smoke-default` byte-identical, the card
golden blessed, `cd web && cargo test` green. **Shipped.**

## 6. What it buys

- **A table a beginner can sit at, with an expert partner.** Fewer alerts
  to read, every call natural unless whitelisted, a preset instead of 220
  toggles — and the same judgement under it, so a learner's results are
  within ~0.02 IMPs/board of the champion's. The web
  registry is already "a player-facing system chooser"
  ([bidding-options.md](bidding-options.md)); two presets and `custom` make
  it one.
- **A curriculum for free.** Each default-on gadget already carries its
  measured verdict in [bidding-options.md](bidding-options.md); a learner
  on `basic` turns one on and sees what it is worth. The option audit
  becomes the lesson plan.
- **A clean ablation of the whole gadget layer** (run 1), plus a
  per-bucket map of where gadgets pay (run 2).
- **A second disclosed card** for BBA to defend against, and a second-system
  corpus for M5.3 when retrains resume, at knob cost.
- **A KR2 win**: the ship policy becomes legible as a name. "Advanced ships
  all winning conventions" is the policy restated, so it costs nothing and
  cannot drift.

## 7. Decisions owed to jdh8 — taken 2026-10-14

jdh8: "Go" on the recommended defaults below, plus an `american_basic()`
factory and `american_advanced()` as the alias of `american` (the shipped
default under its IntoBridge name).  Kept here as the record.

1. **Which conventions to drop.** §4 is the candidate list ranked by
   tree size; the **line call** rows (inverted minors, splinters, Ogust vs
   feature, Texas, Jordan 2NT) are the ones a reasonable player keeps or
   drops either way. No reference text is needed; `render-book` sizes each
   tree if a number helps.
2. **Names.** `basic` / `advanced` as asked (IntoBridge's words), or
   `beginner` / `champion`? The seat name `american-basic` follows
   `american-instinct`'s pattern.
3. **The hole rows.** Author the natural states first (a natural `1NT - 3m`,
   a natural `1NT (2x)` table, the plain new-minor rebid, system-on over
   their double), each as its own knob and A/B on `american()`, or let
   `basic` ship with the nearest authored state and record the gaps?
   Recommendation: ship with the nearest authored state; the holes are
   their own tickets and run 1's trace will rank them.
4. **Floor for `basic`**: the shared v6 net (recommended, §3) or
   `instinct()`?
5. **Leads and carding.** IntoBridge's basic/advanced also differ in
   *defensive* agreements. Out of scope for a bidding variant, noted so the
   name is not over-promised.

## 8. Sources

- IntoBridge, *Lia's Carding Explained*: the robot leads fourth-best against
  suits in the basic systems and third/fifth in the advanced ones — the
  basic/advanced split is system-wide, not just bidding.
  <https://intobridge.com/blog/lias-carding-explained/>. IntoBridge's system
  cards themselves are in-app, not on the public site (checked 2026-10-12).
- BBO forum, *What's the difference between 2/1 and SAYC*: BBO Basic ≈ SAYC,
  BBO Advanced ≈ 2/1 — the precedent for a two-tier split.
  <https://forumstest.bridgebase.com/forums/topic/21952-whats-the-difference-between-21-and-sayc>
- Wikipedia, *Bridge World Standard*: BWS "shares similarities with Standard
  American and 2/1 game forcing, but with many advanced treatments" — why
  BWS is *not* the `basic` reference.
  <https://en.wikipedia.org/wiki/Bridge_World_Standard>; the 2017 complete
  system is behind The Bridge World's login
  (<https://bridgeworld.com/pages/readingroom/bws/bwscompletesystem.html>);
  Jim Loy's public summary is of the 1994 edition
  (<https://www.sao.ru/hq/dim/bridge/biddings/natural/loybws.html>).
