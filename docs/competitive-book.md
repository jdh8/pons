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
| P3d neg-X shape | `set_negative_double_shape` | **measured — BothMajors stays the default** | Modern vs off: plain +0.47 NV (CI>0) / +0.08 vul (~0); PD **−0.22/−0.63 (CI<0)** — the free-bid floor leak dominates. But **Modern vs free: plain +0.95/+1.36 (CI>0 both vuls)**, PD +0.13/+0.40 — the tighter doubles genuinely win (the floor sweep P3b′ then refuted the "floor fix"; the leak vs off is structural). |
| P3d′ Cachalot arm | `NegativeDoubleShape::Cachalot` | **measured — stays opt-in**; ≈ Modern head-to-head | Cachalot vs Modern: NV wash both scorers (±0.04/fired); vul plain −0.26 (~0), PD −0.41 (CI<0). Cachalot vs off ≈ Modern vs off. An engine plays it perfectly and it still only ties Modern on these brackets — revisit with sd-lead (right-siding) after the floor fix. |
| P3d″ Sputnik arm | `NegativeDoubleShape::Sputnik` | **measured — stays opt-in**; ≈ Modern head-to-head (same floor blocker) | Residual double (7+ denying a 1-level-biddable major) + 4+ free major + `cachalot_takeout_answer` for opener. **Sputnik vs Modern: all four cells wash** (+0.001/+0.001 NV, −0.001/−0.003 vul, CIs⊇0). Sputnik vs off: NV plain **+0.012 (CI>0)**, vul plain −0.003 (~0), PD −0.003/**−0.021 (vul CI<0)** — the shared free-bid floor leak, not the shape. Sputnik vs free: plain **+0.005/+0.004 (CI>0)**, PD wash. **v1 was a clear loss** (−0.017 vul-PD vs Modern) from an unauthored opener answer letting the floor jump the phantom *denied* major to a doubled 4♠ — traced, wired `cachalot_takeout_answer`, re-measured to wash. Family floor fix refuted by P3b′ (structural leak). 204.8k bd/arm/vul, SEED_BASE 1783290254, sha ad79b3e, `scripts/sputnik-negx-ab.sh`. |
| P3a 3-level overcalls | `set_high_overcall_responses` | **measured — stays opt-in**; leak named, re-measure candidate | plain −0.0012/−0.0007, PD −0.0005/−0.0006 (all CIs⊇0), −0.6…−0.2 IMPs/fired, 0.19% fired. Worst-board bucket: the minor-opening 3-level neg X (one-major `or` shape at 10+) is too light — try 12+ or 4-4 both majors and re-measure. 204.8k bd/arm/vul, SEED_BASE 1783286003, sha bc949dc |
| P4 Jordan/Truscott over (X) | `set_jordan_truscott` | **SHIPPED default-on**; the campaign's largest per-board win | plain **+0.0041/+0.0067** IMPs/bd NV/vul, +0.51/+0.83 IMPs/fired; PD **+0.0049/+0.0065**, +0.62/+0.80 IMPs/fired; all four cells CI>0; 0.79/0.81% fired. 204.8k bd/arm/vul, SEED_BASE 1783286386, sha bc949dc |
| P5 competitive long-suit rebid (floor) | `set_competitive_rebid` | **SHIPPED default-on**; the campaign's largest per-board win | Opener/overcaller rebids a 6+ suit we personally bid instead of the floor's forced takeout double; 2-level unconditional, 3-level needs 7 cards or a good six (2 of top 3 honors). plain **+0.047/+0.037** IMPs/bd NV/vul, PD **+0.040/+0.023**; all four cells CI>0; +0.67…+1.37 IMPs/fired, 3.4% fired. Blanket 3-level lost vul (opener-3 PD −0.016) → quality gate flipped it to +0.007, overcaller-3 to a wash. 102.4k bd/arm/vul, SEED_BASE 1783316036. Bucketed by `ab-dump-bucket`. Follow-up: "off-shape X stronger" (a separate hand class — competitive doubles *without* a long suit) is a candidate second treatment. |
| alert invariant over fallbacks | — | follow-up | — |
