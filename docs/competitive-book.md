# The competitive-book campaign

Filling the competitive book — auctions where **we open and the opponents come
in**. Read [bidding-architecture.md](bidding-architecture.md) first; every
package here ships (or doesn't) by [measurement.md](measurement.md).

## Why this campaign exists

The rendered book (text and web `#book`) showed an **empty competitive
section** while constructive and defensive were rich. Two causes:

1. **A disclosure artifact.** The competitive book was never empty — ~3,000
   lines of authored competition (cue-raises, negative doubles, support
   doubles, Lebensohl/Transfer-Lebensohl, UvU, the contested-Stayman/transfer
   packages) attach as **guarded fallbacks** (`fallback_all_seats`), and both
   renderers walked only exact trie nodes. Fixed by Workstream 0 below.
2. **Genuine coverage gaps**, tracked in the ledger: their two-suiters over
   our 1M, our contested weak twos and 2♣, overcall responses above 2♠ plus
   free bids, and their takeout double of our 1x (today a bare systems-on
   rebase).

## Wiring idiom (applies to every package)

- **Deeper-key guarded fallbacks** — the Section-5 stolen-Stayman idiom: key
  responder tables at `[open, their_concrete_call]` with `SuffixIs(vec![])`,
  opener continuations at the same key with exact `SuffixIs` suffixes. A
  deeper node's fallback beats every shallower entry (`resolve_at` walks
  deepest-up), so there are no declaration-order races with
  `OvercallAtMost(2♠)` or a `FirstIs(X)` rebase — and the rebase survives as
  the catch-all for every suffix the new guards don't claim.
- **Guarded tables cannot reject-to-floor.** `classify_floored`'s single
  fall-through skips only the *exact-node* classifier; a guarded table with no
  mass returns degenerate logits. Every guarded table is **total** (finite
  catch-all); "everything else systems-on" is guard scope, never rejection.
- **Prefer `len(suit, n..)` over `support(n..)`** in new competitive rules:
  `support` projects nothing, `len` projects tight, and an alerted call
  decodes by its own rule's projection (fallback projection is default-on).
- **Renderability is an invariant.** Every guard and rebase must
  `describe()` (test `competitive_fallbacks_are_renderable`): use `SuffixIs`
  for exact suffixes, `described_guard`/`described_rewrite` around closures.

## Workstream 0 — render the competitive book ✅

Behavior-preserving disclosure fix (render output for existing nodes is
byte-identical; the competitive section went from 0 to ~100 sections):

- `Guard::describe` / `Rewrite::describe` (default `None`); `SuffixIs` guard;
  `described_guard` / `described_rewrite` label wrappers (`fallback.rs`).
- `Trie::fallbacks()` — depth-first enumeration, declaration order within a
  node, Pass child last so seat-fanned entries dedupe to the pass-less key.
- All competition.rs guards converted to `SuffixIs` / described wrappers.
- `render-book` and the web `book()` print guarded sections: heading = node
  auction + guard description, body = rules table or `→ systems on …` note.

Known follow-up (own commit): extend `artificial_calls_are_alerted` over
`Trie::fallbacks()` — fallback-attached rules currently escape that invariant,
and running it may surface genuine missing alerts.

## The packages

Each is a `set_*` knob, default **off** until its A/B ships it, with a
`--ns-*`/`--no-ns-*` switch in `bba-gen`. One knob = one measured change.

### P1 — their two-suiters over our 1M (`set_uvu_over_majors`)

Responder structure over `[1M, (2NT unusual)]` and `[1M, (2M Michaels)]`,
UvU-style: cheaper cue (their lower suit / the other-major cue) = limit+ raise
(alert `comp:uvu-major-raise`), second cue = GF other-major, X = values /
penalty interest, direct raises stay natural-competitive, jump raise
preemptive. Opener reuses `answer_cue_raise`/`answer_cue_minor_raise` for the
limit+ cue. One hand-written reading: suppress the natural walk's "their cue
of our opened suit = natural 5+ suit" misread (unsound vs Michaels), record
the two-suiter shape when the knob is on. Fixes a live misbid: today the
negative-double rule fires over `1♥-(2♥ Michaels)`.
Deferral: their-Michaels-over-our-minors (`1m-(2m)`), same misread.

### P2 — contested weak twos (`set_weak_two_competition`) + strong 2♣ (`set_strong_two_competition`)

Nothing is keyed on `[2x]` today — pure floor. Weak twos: over (X), uncontested
responses ride + business XX 13+ (`comp:weak-two-xx`), Ogust survives via a
systems-on rebase; over an overcall ≤3♠, Ogust-when-legal / penalty-leaning X /
preemptive raises. McCabe = named deferral. Strong 2♣: over (X) systems-on
rebase; over an overcall, natural-GF new suits, X = cards (shadows the floor's
takeout-X — a live bug: a 22+ opener behind a "takeout" double), Pass =
waiting backed by opener's **forced reopening** (finite catch-all X).

### P3 — extended overcall responses (four knobs)

- `set_major_support_double` — support X/XX after `1♥-(P)-1♠-(overcall below
  2♠, or X)`, reusing `support_rules(Spades)`.
- `set_free_bids` — natural free bids inside `over_their_overcall`: 1-level
  new suit 5+ & 6+, 2-level non-jump 5+ & 10+, 1NT 6–10 / 2NT 11–12 with
  stopper.
- `set_negative_double_shape` — enum, `BothMajors` (current, byte-identical
  default) / `Modern` / `Cachalot`. See the theory note below.
- `set_high_overcall_responses` — a second guarded entry for overcalls in
  `2NT < b ≤ 3♠`: neg X through 3♠ (10+), 3NT with stopper, forcing 3-level
  new suits, raises; opener's forced answer to the 3-level neg double.
  4-level cue dropped (X-then-raise or blast) — deferral.

### P4 — over their takeout double (`set_jordan_truscott`)

Responder table at the deeper `[1x, X]` key (total): XX = 10+ no fit
(`comp:value-redouble`), Jordan/Truscott 2NT = 4+ support limit+
(`comp:jordan`), jump raise flips preemptive, 1-level suits forcing, 2-level
suits weak NF (2/1 off over X), 1NT 6–9. The `[1x]` `FirstIs(X)` rebase stays
for every deeper suffix; opener continuations that the rebase would misread
get exact-suffix nodes (`[2NT, P]` → cue-raise answers — else Jordan lands on
Jacoby 2NT; `[3o, P]`; weak `[2x, P]`).

## Negative doubles at the 1-level (theory verdict, 2026-07-06)

- **Sputnik (Roth–Stone 1957/58):** X is the **residual** — 7+ HCP *denying* a
  4-card major biddable at the 1-level; the free 1-level major shows a natural
  4+ (not Modern's 5+, since the double no longer carries the exactly-four
  hand). Over (1♦) → X = ≤3 in *both* majors (4+ in either bids 1♥/1♠); over
  (1♥) → X = ≤3 spades (4+ bids 1♠); over (1♠) → 4+ ♥ (nothing to deny). This
  is the inverse of a major-*showing* double: the free bid carries the major,
  the double is what's left over. `NegativeDoubleShape::Sputnik`, authored +
  measured 2026-07-06 (`--ns-negative-double-shape sputnik`): **ties Modern,
  stays opt-in** — same free-bid blocker, now shown **structural, not a floor
  height** (see the `set_free_bid_floor` sweep in the ledger).
- **Modern (BWS 2017 / Cohen; what BBA plays, untoggleably):** floor 6,
  unbounded. (1♦) → 4+/4+ majors; (1♥) → **exactly** 4♠ (1♠ = 5+); (1♠) → 4+
  ♥; `1♥-(1♠)-X` ≈ 4-4 minors. Through 3♠ or higher. Weak 5-card major →
  double-then-bid, NF. Trap pass requires opener's reopening-X duty.
- **Cachalot / Sardine / Spoutnik rotatif (French; Claire Martel memo):**
  transfer Walsh in competition, over 1♣/1♦ and (1♦)/(1♥) only: X = 4+
  adjacent major, 1♥ = 4+ ♠, 1♠ = takeout hand denying 4♠. Opener's 1-level
  completion = **exactly 3-card support, forcing**; 1NT doesn't deny 3; raise
  = 4. Reverts to natural if they bid again. Most projection-friendly (pure
  per-suit bounds); its headline benefit is right-siding → **DD-blind, the PD
  bracket decides**; no BBA analogue to distill.

Don't author the SEF "4 w/8+ OR 5 w/7–10" disjunction (the OR-projection
wall). Measure as four arms: `BothMajors`+free-bids / `Modern`+free-bids /
`Cachalot` / `Sputnik` (`scripts/sputnik-negx-ab.sh`). Sputnik shares the
free-bid layer, so it inherits the free-bid leak that sank Modern on PD — the
arm tested whether the residual double distributes that leak better than
Modern's exactly-four shape (it ties). The `set_free_bid_floor` sweep
(6→7→8, `scripts/free-bid-floor-ab.sh`) then proved that leak is **structural,
not the floor height**: raising the floor only sheds profitable NV free bids
and leaves the vul-PD losses intact. The family's default-on unblock is a
shape/suit-quality gate on *which* free bids to make, not a strength floor.

## Known deferrals / oddities spotted while authoring

- **`1♥-(1♠)-X` shows 4+ spades** in the shipped `over_their_overcall` rule —
  `other_major` is spades even when spades *is* the overcall, so the "negative
  double" there is really a trump-stack values double. Pre-existing; the
  Modern/Cachalot arms don't touch major openings. Revisit if the P3 forensics
  flag it.
- Modern's opener answer to the minor-opening negative double rides the floor
  safely *because Modern's double shows the major* — the floor's classic
  "negative double = the unbid major" instinct is correct for it. Cachalot's
  and Sputnik's answers are authored: Sputnik's double **denies** the major, so
  the same floor instinct is exactly inverted and (measured) jumped the phantom
  denied suit to a doubled 4♠ until `cachalot_takeout_answer` was wired in.
- P4's XX/Jordan **contested tails** (advancer bids over them) rebase into a
  dead end and land on the floor with the projected floors — authored
  continuations are a follow-up if the buckets drag.
- Balancing-seat two-suiter reading (`[1M, P, P, 2M/2NT]`) is not recorded —
  the P1 reading is direct-seat only, matching the authored nodes.

## Measurement discipline per package

- P1 cues, P3 (all), P4 Jordan/XX: constructive contract-finding — DD-visible,
  normal win/wash ship rules.
- P2a preemptive raises and P4's 3o flip: obstruction-wall — plain-DD ≈ 0 is
  *expected*; score both brackets, bucket by call before judging, and carve a
  dragging preemptive bucket behind a sub-toggle rather than sinking the
  package.
- P2b (2♣ contested) is small-N: judge on IMPs/divergent + worst-board
  forensics.
- Cachalot: PD bracket decisive (right-siding).

## Ledger

| Package | Knob | Status | Verdict (plain / PD, IMPs) |
| --- | --- | --- | --- |
| WS0 renderer | — | **shipped** | render-only, node output byte-identical |
| P1 two-suiters over 1M | `set_uvu_over_majors` | **SHIPPED default-on** | plain **+0.0019/+0.0018** IMPs/bd NV/vul (CI>0), +1.43/+1.58 IMPs/fired, 0.13/0.11% fired; PD +0.0009/+0.0006 (same sign, CI touches 0). 204.8k bd/arm/vul, SEED_BASE 1783284454, sha bc949dc |
| P2a weak twos contested | `set_weak_two_competition` | **measured — stays opt-in**; forensic follow-up before re-measure | plain −0.0012/−0.0015 (wash, CI⊇0); PD **−0.0097/−0.0116** (CI<0), −1.50/−1.94 IMPs/fired, 0.64/0.60% fired. Worst-board buckets: values-X over their overcall (no trump gate — leak), contested Ogust (too eager at 14+ — leak), preemptive raises over (X) (obstruction wall — park for sd-lead). 204.8k bd/arm/vul, SEED_BASE 1783284838, sha bc949dc |
| P2b strong 2♣ contested | `set_strong_two_competition` | **SHIPPED default-on** | plain **+0.0009/+0.0013** IMPs/bd NV/vul, +1.86/+2.79 IMPs/fired; PD **+0.0010/+0.0014**, +2.00/+2.93 IMPs/fired; all four cells CI>0; 0.05% fired. 204.8k bd/arm/vul, SEED_BASE 1783285250, sha bc949dc |
| P3c major support double | `set_major_support_double` | **SHIPPED default-on** (plain-wash + PD-gain row) | plain −0.0004/+0.0004 (wash, CIs⊇0); PD +0.0009/**+0.0016** (vul CI>0), +0.97/+1.69 IMPs/fired; 0.10% fired. 204.8k bd/arm/vul, SEED_BASE 1783285623, sha bc949dc |
| P3b free bids | `set_free_bids` | **measured — stays opt-in**; floor sweep done (P3b′) | vs off: plain +0.29 NV (CI>0) / **−0.30 vul (CI<0)**; PD −0.31/−0.88 (CI<0 both). ~2.0% fired. Worst-board bucket: 1-level free bids + the free 1NT get PD-punished vul. SEED_BASE 1783286814, sha bc949dc |
| P3b′ free-bid floor sweep | `set_free_bid_floor` (default 6) | **measured — floor-height hypothesis REFUTED**; leak is structural, not the 6-count floor | Swept 6→7→8. `free8` vs off still −0.0128 vul-plain / −0.0212 vul-PD / −0.0066 NV-PD (CIs<0) **and** gives up free6's +0.0028 NV-plain win. `free8` vs `free6`: removed 6–7 counts were net-**positive** (NV-plain −0.0026, vul-plain −0.0019, CIs<0), bought nothing at vul-PD (+0.0012, CI⊇0). `modern8` vs `modern6` = same bad trade. Vul weakness is **plain-DD-visible and strength-independent** — a shape/suit-quality gate, not a floor, is the unblock. Knob kept opt-in (default 6 byte-identical). 204.8k bd/arm/vul, SEED_BASE 1783315917, sha c5a0b44, `scripts/free-bid-floor-ab.sh` |
| P3d neg-X shape | `set_negative_double_shape` | **superseded by P3b″′ — Modern SHIPPED default-on** (see below) | Modern vs off: plain +0.47 NV (CI>0) / +0.08 vul (~0); PD **−0.22/−0.63 (CI<0)** — the free-bid floor leak dominates. But **Modern vs free: plain +0.95/+1.36 (CI>0 both vuls)**, PD +0.13/+0.40 — the tighter doubles genuinely win (the floor sweep P3b′ then refuted the "floor fix"; the leak vs off is structural). |
| P3b″ free-bid quality gate | `set_free_bid_quality` | **measured — gate REFUTED as designed; stays opt-in** (default byte-identical) | Vul 1-level free demands 2 of top 3 honors + vul free 1NT dropped, on the Modern base. `modernq` vs `modern`: vul plain **−0.0042 (CI<0)** for vul-PD +0.0033 (CI⊇0) — suppressed *winning* junk frees, left the leak; NV 0 fired (byte-identical at NV, full scale). The anchor waterfall agreed in advance: BBA bids bad-quality 1-level frees at 85%, same as good — top-honors is not the axis. 204.8k bd/arm/vul, SEED_BASE 1783666604, `scripts/free-bid-quality-ab.sh` |
| P3b″′ Modern-complete (forcing free bids + answers) | `set_negative_double_shape` default → `Modern` | **SHIPPED default-on** — the fallback@1/@2 campaign's Fix 1 | Free bids made **forcing one round at both levels** (jdh8 ruling) with opener's Section-4d `answer_free_bid` (raise 3+, cheapest NT w/ stopper 12–14, natural second suit — reverses/3-level 16+, opening-rebid catch-all, no Pass); v1 without answers had opener passing game-going `2♦` out. Modern-complete vs off: plain **+0.0213 NV / +0.0074 vul (CI>0 both)**; PD −0.0044 NV (⊇0) / −0.0256 vul (CI<0); **sd-lead arbiter (16 worlds, disclosure-corrected `--on-ns-negative-double-shape`): +0.4221 NV / +0.2881 vul per divergent bd (CI>0 both, sd>plain)** — the vul-PD loss is the perfect-defense doubling artifact on thin vul games, overruled per the 1NT-systems-on precedent (take 1 undisclosed vs take 2 disclosed differed by <0.01 ⇒ no misread-exploitation). 204.8k bd/arm/vul, SEED_BASE 1783672667, `scripts/free-bid-answers-ab.sh` |
| P3d′ Cachalot arm | `NegativeDoubleShape::Cachalot` | **re-adjudicated with complete books (school tournament Stage A) — LOSES to Modern, stays opt-in** | Books completed first (3f6f790: natural 2-level frees now get `answer_free_bid`; rotation keeps its Section-9 completions). Cachalot vs Modern loses **all six cells**: NV plain −0.0031 / PD −0.0046 / sd −0.0027, vul plain −0.0080 / PD −0.0095 / sd −0.0071 (every CI<0; −0.28…−0.84/fired, ~1.1% fired). The right-siding thesis is refuted on its own bracket — disclosed sd-lead is a loss too, not the redemption. Named leak: the rotated X floors at `hcp(6..)`, so light shapely spade hands (points ≥6 via the length upgrade) that Modern frees with `1♠` orphan to Pass; and X conceals the 4-vs-5 spade distinction, losing the 4♠-over-4♥ push/sacrifice boards. The earlier "≈ Modern" was a both-incomplete comparison. **Refloor postscript (jdh8: "Cachalot is just rotated Sputnik", ed17d04):** the major-showing rotated calls now take `points(free_bid_floor()..)` and the residual takeout `hcp(7..)`, matching Sputnik. The re-run improved every cell but flips none — NV plain −0.0024 / PD −0.0049 / sd −0.0018 (sd now ⊇0), vul plain −0.0073 / PD −0.0100 / sd −0.0057 (5 of 6 still CI<0). The floor seam was a minor component; the residual loss is the rotation itself (spade concealment, 4-vs-5 ambiguity, completion economics — Sputnik with the same floors and hand partition washes vs Modern, so rotation nets ≈−0.005/bd). Stays opt-in at the better floors. 204.8k bd/arm/vul, SEED_BASE 1783679026, sha 3f6f790 (v1) / ed17d04 (refloor), `scripts/school-negx-ab.sh`. |
| P3d″ Sputnik arm | `NegativeDoubleShape::Sputnik` | **re-adjudicated with complete books (school tournament Stage A) — does not clear the flip gate, stays opt-in** | Books completed first (3f6f790: the 2-level raise of a free major demands four trumps — its majors promise only four). Sputnik vs Modern: NV plain +0.0019 (±0.0021, wash) / PD +0.0009 (wash) / **sd +0.0039 (CI>0)**; vul plain −0.0024 / PD −0.0030 / sd −0.0010 (all wash, leaning negative). One CI>0 cell (NV sd) against a vul wash doesn't displace a shipped default — pooled sd +0.0014 (±~0.0018, ⊇0). Reading: the residual double's honest denial profiles slightly better to a blind NV leader; real but sub-gate. 204.8k bd/arm/vul, SEED_BASE 1783679026, sha 3f6f790, `scripts/school-negx-ab.sh`. |
| P3e negative free bids | `FreeBidStyle::Negative` | **measured — does not clear the ship gate, stays opt-in; tempering v2 REFUTED** | Classic NFB (jdh8 spec): 2-level new suits non-forcing `(len 6+ \| len 5+ & 2 top honors) & points(5..=11)` answered WITH Pass (4d′); all stronger long-suit hands start with a widened shapeless-12+ X (the named OR-projection wall) and clarify with a game-forcing new suit, both sides authored (4d″/4d‴). **v1 vs Forcing: plain wash both vuls (−0.0003/−0.0013, CIs⊇0), PD loss both vuls (−0.0113/−0.0127, CI<0), sd WIN both vuls (+0.0033/+0.0043, CI>0, disclosed)** — the bracket-split verdict; no plain win means sd cannot overrule the PD loss (the Fix-1 precedent leaned on plain CI>0 ×2). Worst-board trace named the floor game-jumping the school's major read over the two-way X (`X – 4♥` phantom fits). **v2 tempering (cheap `answer_two_way_double`) made everything worse** — it claimed every X answer and its timid invites lost more ordinary games than the phantom boards cost (plain −0.0096/−0.0185, sd −0.0069/−0.0141, all CI<0; fired 1.44%→1.87%) — REVERTED (5c20e0c). A future NFB continuation campaign would need opener's X-answer policy tuned to match the floor except on the phantom texture. 204.8k bd/arm/vul, SEED_BASE 1783681411, sha 6d8b0ab (v1) / 3d4fac3 (v2), `scripts/free-bid-style-ab.sh`. |
| P3f 2-level free-bid transfers | `FreeBidStyle::Transfer` | **measured — LOSS on all three scorers both vuls, stays opt-in** | Cachalot-style 2-level swaps (exactly-two-slot contexts; opener completes and declares, wrap completes a level higher, pass/raise-inv/cue-FG clarify; lone and three-way slots natural-forcing). Transfer vs Forcing: NV plain −0.0095 / PD −0.0221 / sd −0.0079, vul plain −0.0148 / PD −0.0285 / sd −0.0126 (every CI<0; −0.9…−2.6/fired). The Rubens "DD dead-zero" expectation did not hold — the right-siding never materializes even at sd (the thesis bracket). Named leaks: the unlimited 6+ transfer fires weak hands the Forcing style leaves alone (10+) into 3-level wrap completions, and game-going hands lose a round of natural description (off-arm `2♥–3♥–4♥` becomes on-arm transfer-muddle into 3♠/4♦ misfits). Structural, not a completion gap — the Gladiator precedent caps completion at parity, so no further authoring round. 204.8k bd/arm/vul, SEED_BASE 1783681411, sha 6d8b0ab, `scripts/free-bid-style-ab.sh`. |
| P3a 3-level overcalls | `set_high_overcall_responses` | **measured — stays opt-in**; leak named, re-measure candidate | plain −0.0012/−0.0007, PD −0.0005/−0.0006 (all CIs⊇0), −0.6…−0.2 IMPs/fired, 0.19% fired. Worst-board bucket: the minor-opening 3-level neg X (one-major `or` shape at 10+) is too light — try 12+ or 4-4 both majors and re-measure. 204.8k bd/arm/vul, SEED_BASE 1783286003, sha bc949dc. Follow-up: replace with a **Lebensohl** slow-3NT / fast-cue structure so responder splits competitive-no-game from GF at the 3-level (cures the named 3-level neg-X-too-light leak); keep opt-in until it beats the floor fallthrough. Reuse the existing Lebensohl machinery (A3 `set_lebensohl_style` / `set_advance_sohl_style`). |
| P4 Jordan/Truscott over (X) | `set_jordan_truscott` | **SHIPPED default-on**; the campaign's largest per-board win | plain **+0.0041/+0.0067** IMPs/bd NV/vul, +0.51/+0.83 IMPs/fired; PD **+0.0049/+0.0065**, +0.62/+0.80 IMPs/fired; all four cells CI>0; 0.79/0.81% fired. 204.8k bd/arm/vul, SEED_BASE 1783286386, sha bc949dc |
| P5 competitive long-suit rebid (floor) | `set_competitive_rebid` | **SHIPPED default-on**; the campaign's largest per-board win | Opener/overcaller rebids a 6+ suit we personally bid instead of the floor's forced takeout double; 2-level unconditional, 3-level needs 7 cards or a good six (2 of top 3 honors). plain **+0.047/+0.037** IMPs/bd NV/vul, PD **+0.040/+0.023**; all four cells CI>0; +0.67…+1.37 IMPs/fired, 3.4% fired. Blanket 3-level lost vul (opener-3 PD −0.016) → quality gate flipped it to +0.007, overcaller-3 to a wash. 102.4k bd/arm/vul, SEED_BASE 1783316036. Bucketed by `ab-dump-bucket`. Follow-up: "off-shape X stronger" (a separate hand class — competitive doubles *without* a long suit) is a candidate second treatment. |
| P6 doubled-splinter systems-on | `set_splinter_doubled` | **SHIPPED default-on**; anchor bucket #4 tail | A double of our game-forcing splinter reroutes opener into the competitive book, where — unauthored — it fell to the floor and *passed* the doubled game force (a four-ace monster passing `4♣x` while the field bids `7♠`). A `FirstIs(Double)` rebase keyed at `[1M, P, splinter]` strips the double off the whole subtree so opener + responder's keycard answers resolve systems-on. plain **+0.0059/+0.0079** IMPs/bd NV/vul, PD **+0.0059/+0.0079** (plain ≈ PD — removing *our own* doubled contracts, no artifact); all four CIs>0; +15.4/+17.6 IMPs/fired, 0.04% fired. 204.8k bd/arm/vul, SEED_BASE 1783439089, `scripts/splinter-doubled-ab.sh`. Known tail: a *second* double (of the keycard response) still passes out — 1 board in 79, the standard rebase-tail limitation. |
| alert invariant over fallbacks | — | follow-up | — |
| Rubens-clean transfer advances | `FreeBidStyle::RubensClean` (unbuilt) | **deferred design** — the resumable retry of the lost P3f; see below | — |

## Deferred designs

### Rubens-clean transfer advances (retry of P3f)

**Not built.** A resumable design for a *completion-disciplined* variant of the
transfer advance that P3f (`FreeBidStyle::Transfer`, row above) lost. Ship as a
**new `FreeBidStyle::RubensClean` variant** so it A/Bs head-to-head against
`Forcing` on the existing `scripts/free-bid-style-ab.sh` — do **not** reshape the
lost `Transfer` variant (keep it for the record).

**Goal.** Transfer responses over an overcall of our 1-level opening —
`1♣ (1♠) 2♦ → hearts`. Thesis: a natural suit advance forces one side of the
eternal **forcing-vs-non-forcing** debate; a transfer covers the **union of the
F and NF strength bands** in one call and disambiguates strength on the
follow-up. DD-visible (the orphaned band otherwise reaches a worse contract).

**Why P3f is evidence against the naive form, not just its right-siding.** Over
`1♣ (1♠)` both unbid suits (♦, ♥) sit below the enemy ♠, so a symmetric
next-suit-up transfer inverts them: `2♦ → ♥` completes cleanly at **2♥** ✓, but
`2♥ → ♦` wraps — opener can't descend to 2♦, so completion jumps to **3♦** ✗.
Unioning the NF band into the transfer therefore shoves the *diamond* transferee
to the 3-level = P3f's named **leak #1** (weak band over-commits). P3f's leak #2:
game-forcing hands lose a round of natural description. Both are structural, so a
re-run of the same shape is pointless.

**The fix — completion discipline (between clean-only and symmetric P3f).** The
value lives in *how opener completes*:

- `2♦ → hearts` transfer. **Opener completes 2♥ by default ("when in doubt,
  complete")** — the near-automatic 2-level completion keeps middle hands alive
  in a way a natural NF `2♥` cannot (a natural NF `2♥` can be *passed out*,
  stranding hands that want one more turn). Opener **breaks (bypasses completion)
  only with a strong hand, ~15+.**
- **Advancer** reads the completion: pass 2♥ = NF/weak band; bid on = the
  F/interesting band. The F∪NF union is realized *through the completion round*,
  not by widening the initial call.
- **Break threshold is a knob** (`set_rubens_break_floor`, ~15+) — the primary
  tuning parameter. Beats P3f because the default completion is a **2-level** bid
  (NF band stops low at transfer-then-pass), not a 3-level wrap; the extra round
  gives the F band room to continue (the leak-#2 defense — the open A/B risk).
- **Wrap suit** (`2♥` = diamonds over `1♣ (1♠)`): keep **natural by default**;
  a wrap transfer would be a separate second knob measured on its own.

**Step 0 (prerequisite data) — probe BBA/BEN's `2♥` band + forcing-ness** to set
the transfer floor and the F/NF boundary from data, not a guess. Neither bidder
discloses "forcing", so bucket the actual call at prefix `[1♣, 1♠]` (actor =
responder seat 2) and infer forcing-ness from opener's continuation at
`[1♣, 1♠, 2♥, P]` (seat 0): **Pass (NF) vs bids-again (F)**. BBA: extend
`examples/probe-bba-constraints/main.rs` (its `Bucket`/`render`/`pct` +
continuation `filter` idioms; per-seat conventions via `--conv` — the `.so`
ignores `.bbsa`). BEN: `BenOracle` (`examples/ben-gen/main.rs`, REST subprocess),
continuation-inference only. Caveat: BBA/BEN's natural `2♥` shows only *one* band,
so it teaches at most half the union.

**Authoring seam** (competitive book, ~1 day): the 2-level free-bid region of
`over_their_overcall(opening)` (`competition.rs:1227`, region `:1532-1608`), gated
by `set_free_bid_style` / `FreeBidStyle` (`:675`). Copyable templates —
transfer-Lebensohl / Cachalot. Author **both sides first** (opener completion
`Rules` fn modeled on `transfer_completion` `:2962`; responder clarify fn for the
round-3 F/NF split) before measuring — the P3f ledger shows bare structure loses.
Floor the *target* suit + `.alert(...)` so `project_authored` auto-decodes; add a
manual alert check for the new fallback node (the `artificial_calls_are_alerted`
invariant does not yet traverse fallbacks). Register both sides with
`fallback_all_seats(...)` keyed at `[open, overcall]`, one `SuffixIs` per round.

**Measurement / win bar.** Head-to-head `RubensClean` vs `Forcing` on
`scripts/free-bid-style-ab.sh`, both plain-DD and PD (sd if close), both vuls,
fresh `SEED_BASE`, arms sequential. Must **clear the P3f loss** — at minimum a
plain-DD wash + PD gain (shippable default-on), not a PD-only artifact. Trace the
worst divergent boards to confirm the clean completion actually stops the weak
band low and preserves GF description.
