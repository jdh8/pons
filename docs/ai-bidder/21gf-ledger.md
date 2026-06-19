# 21GF ledger — pons 2/1 vs BBA's `21GF.bbsa`

Target spec for "author pons's 2/1 about as deep as BBA". One row per relevant
`vendor/bba/21GF.bbsa` toggle. The plan that governs this work:
`~/.claude/plans/author-the-2-1-bidding-replicated-mochi.md`.

**Status legend:** `shipped` (authored + tested) · `partial` (some of it) ·
`floor` (handled by `instinct()` only, not authored) · `gap` (absent) ·
`conflict` (pons plays it differently) · `override` (user chose ≠ 21GF) ·
`out` (out of scope).

**Campaign metric:** `examples/bba-match` IMPs/board vs BBA 2/1. Original baseline
**−2.59** (2000 boards); fresh re-measure **−1.997** (4000 boards, CI
[−2.16, −1.83], 81% divergent) — gap already narrowed by recent M6.1 work. Trend
it up as batches land. Worst-board themes: competitive doubles/advances +
balancing/reopening, and slam accuracy (missed grands).

## Locked decisions (from planning dialogue)

1. Mirror `21GF.bbsa` ON toggles; author by IMP-priority; user vetoes per item.
2. Shipped conflicts: keep pons's, A/B head-to-head; switch only if BBA wins.
3. Keycard: **keep 1430** (not 0314).
4. Jump shifts: **keep weak**; fit-showing only as a competition gadget later.
5. Beyond-21GF additions in scope: **plain-4NT minor keycard**, **Garbage Stayman**.
6. Out of scope: Multi 2♦, Benjamin/French 2D, Gambling, Gazzilli, Ghestem,
   Raptor 1NT, plain Landy, Polish/Wilkosz, Kickback/Crosswood.
7. Overrides: **Woolsey** over their 1NT (≠ Cappelletti); **Rubensohl /
   transfer-Lebensohl SEF-2018+** over interference on our 1NT (≠ Lebensohl).
8. **Kickback** postponed to its own session (spec in the plan).

## Constructive — 1NT/2NT responses

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 9 | 1N-2♠ transfer to clubs | shipped (two-way 2♠) | keep | — | — |
| 12 | 1N-3♣ transfer to ♦ | conflict (Puppet) | keep Puppet, A/B | — | — |
| 14 | 1N-3♦ majors | shipped | keep | — | — |
| 18 | 1N-3M splinter | verify | — | — | — |
| 88 | Minor transfers after 2NT | shipped | keep | — | — |
| 98 | Quantitative 4NT | shipped | keep | — | — |
| 109 | Smolen | shipped | keep | — | — |
| 119 | Texas | conflict (SAT 4♣/4♦) | keep SAT, A/B | — | — |
| 55 | Extended acceptance after NT | partial/verify | — | — | — |
| 115 | Super acceptance after NT | partial/verify | — | — | — |
| — | **Garbage Stayman** | gap (in scope) | add (Batch 3) | — | — |

## Constructive — suit raises & responses

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 7 | 1M-3M inviting (limit raise) | shipped | keep | — | — |
| 70 | Jacoby 2NT | shipped | keep | — | — |
| 113 | Splinter | shipped | keep | — | — |
| 68 | Inverted minors | shipped | keep | — | — |
| 37 | Bergen | conflict (limit+J2NT) | A/B (Batch 3) | — | — |
| 89 | Mixed raise | gap/conflict | A/B w/ Bergen | — | — |
| 56 | Fit showing jumps | conflict (weak JS) | keep weak; comp later | — | — |
| 116 | Support 1NT | verify | — | — | — |

## Constructive — opener rebids / checkback

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 57 | Forcing 1NT | shipped | keep | — | — |
| 125 | Two-Way NMF / XYZ | gap (floored) | add, 2NT→3♣ variant (Batch 3) | — | — |
| 58 | Fourth suit forcing | gap (floored) | add (Batch 3) | — | — |
| 124 | Two-way game tries | gap | add (Batch 3) | — | — |
| 52 | Drury | gap | add (Batch 3) | — | — |
| — | Meckstroth adjunct (pons-only) | shipped | keep (complementary to XYZ) | — | — |

## Slam

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 39/40 | Blackwood 0314 / 1430 | shipped 1430 | **keep 1430** | — | — |
| 75 | King ask by 5NT | shipped (majors) | keep | — | — |
| 35 | 5NT pick a slam | verify | — | — | — |
| 64 | Gerber | gap | add (Batch 2) | — | — |
| 53 | Exclusion | gap | add (Batch 2) | — | — |
| 51 | DOPI | gap | add (Batch 2) | — | — |
| 103 | ROPI | gap | add (Batch 2) | — | — |
| 42 | BROMAD | gap (confirm meaning) | Batch 2 | — | — |
| — | **Plain-4NT minor keycard** | shipped (small slam) | keep; grand-in-minor deferred | **+6.80/+8.76 IMPs/div (none/both)** (2M, 46 div) | 99da1b3 |

## Competitive — our opening contested

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 80 | Lebensohl after 1NT | **shipped** | **Transfer Lebensohl** (Cohen) default; plain kept as option | Transfer vs plain **+0.46/+1.24/div** (none/both, 200k); vs floor +0.35/+0.05; (plain vs floor +0.26, Ruben-v1 −1.68); 2NT-role swap true-Rubensohl −0.017/−0.046/board (200k) reverted | bfe5e59 (plain), bee9204 (transfer) |
| 105 | Rubensohl after 1m | floor (Rubens advances) | upgrade (Batch 1) | — | — |
| 100 | Responsive double | partial; overcall-ext tried — DD-negative | **keep floor** (don't ship the light overcall double) | takeout-X-then-raise authored (`defense.rs`); 8+ floor double after partner's *overcall* A/B'd **−0.034/board, −2.37/div** (200k, 1.4% div) → reverted | reverted |
| 83 | Maximal doubles | gap | add (Batch 1) | — | — |
| 71 | Jordan/Truscott 2NT | tried — DD-negative | **keep floor** (don't ship) | full **−1.0/−1.5** IMPs/div; 2NT-only **−4.2/−4.4** (jordan-ab 500k/300k) | reverted |
| 117 | Support double/redouble | shipped | keep | — | — |
| 28/30 | 1X-(Y)-2Z forcing/weak | partial | verify | — | — |
| 122 | Transfers if RHO bids clubs | gap | add (Batch 1) | — | — |

**Transfer Lebensohl (80) — Rubensohl take 2, shipped as default.** The first
Rubensohl attempt lost (−1.68/div) by stranding game hands in partscores. Larry
Cohen's *Transfer Lebensohl* fixes that: after `1NT–(2X)` the 3-level bids are
transfers up the line *through* the adverse suit (over `(2♥)`, `3♦` shows
spades), the cue is Stayman, and a transfer to a suit above theirs is INV+ so
opener is **driven to game** (`4M` with a fit, else `3NT`) — the anti-stranding
rule the user specified. Weak hands keep the plain outlets (natural 2-level,
`2NT` relay, penalty double). A/B (`lebensohl-ab`, `--ns transfer`, 200k
boards/cell): **+0.46/+1.24 IMPs/divergent (none/both) vs plain** (the incumbent
default) and +0.35/+0.05 vs the bare floor — reversing v1's loss. Selected by
`LebensohlStyle` (`set_lebensohl_style`); `Transfer` is the default, `Plain` kept
for the A/B and as a fallback. Unlike the preemptive conventions below, the win
is mostly *constructive* (reaching the right game / strain), which the
DD / perfect-defense measure can see; the right-siding (strong `1NT` hand
declares) is invisible on top, so the table value is higher still.

**Naming + the TransferSmolen experiment (80, follow-up — tried & reverted).**
*Rubensohl* proper makes `2NT` an artificial **club** transfer; what ships keeps
the weak `2NT` **relay**, so it is *Transfer Lebensohl* (Cohen). A **TransferSmolen**
hybrid — Cohen over `(2♠)` but the *standard low-Stayman* structure over `(2♦)`/`(2♥)`
(the bid into their suit is Stayman, `3♣`/`3♦`, freeing a Smolen continuation) — was
authored and A/B'd vs `Transfer`. First pass read −0.7/div but was contaminated by a
too-tight Stayman gate (single-4-card-major hands stranded into a penalty double);
after loosening the gate (fire on one 4-card major) the **fair re-test was
−1.31/−1.76 IMPs/div (none/both, 300k filtered)** — a clear loss. Standard low-Stayman
reaches DD-worse contracts than Cohen's cue=Stayman (e.g. a 5-5 hand routes through
Stayman→denial→`3NT`, missing the 5-3 major game Cohen's transfer-*through* finds), and
Smolen's right-siding is DD-blind. **Reverted.** `lebensohl-ab` kept a cheap
`--filter-dh` shape pre-filter (concentrates `1NT–(2♦/2♥)` boards ~10× so DD
lands on boards that can diverge) + a worst-board auction diagnostic.

**The `2NT`-role A/B (80, follow-up — tried & reverted).** The `2NT`-role swap —
**true Rubensohl** (`2NT` an artificial **club** transfer) vs the relay (Transfer
Lebensohl) — was authored on the Cohen structure and A/B'd vs `Transfer`. Design
(jdh8's rule): every transfer to a suit *below* the overcall is two-way (weak
transfers then passes; strong continues), so `2NT`=clubs is two-way; transfers to
a suit *above* the overcall stay INV+ (weak hands escape with a natural 2-level
bid), identical to `Transfer` — so opener still auto-drives those to game. The
**fair test was −0.017/−0.046 IMPs/board (none/both, 200k filtered each)** — a
clear, consistent loss. Mechanism: for weak hands both arms reach the *same*
contract (right-siding the low-suit partscore is DD-blind), so Rubensohl's only
gain is invisible to DD, while making the low transfers two-way *costs*
`Transfer`'s auto-drive-to-game on invitational hands (opener accepts minimally,
the borderline responder stops short). **Reverted** (kept only as this ledger
note + the `LebensohlStyle` doc); revive from git if a single-dummy measure that
can see right-siding ever shows promise.

**Jordan/Truscott (71) — tried and rejected (DD-negative).** Authored
`1M–(X)–2NT` = limit-raise-or-better + `3M` = preemptive, with opener's decline
path (`2NT`→`3M` sign-off, responder pass/4M) and a sound `2NT` strength
inference; reused the uncontested `major_responses` for every non-Jordan call;
gated by `set_jordan`. A/B'd vs the system-on baseline (`jordan-ab`, contested
seat-swap duplicate, `Family::NATURAL` opponents take out double our major).
Result: **−1.0/−1.5 IMPs/divergent** (full package, 500k boards, none/both) and
**−4.2/−4.4** (the `2NT`-constructive half alone, 300k). Two causes, both inherent
to the harness: (1) the preemptive `3M`'s obstruction value is invisible to the
double-dummy / perfect-defense measure (the solver sees through it — cf.
`texas-vs-sat` "concealment is single-dummy"), while its overbid cost is counted;
(2) making `2NT` limit-or-better diverts 13+ game-forcing raises out of pons's
rich **Jacoby 2NT** machinery (shortness / slam) into a crude `3M/4M` stub,
reaching worse games and missed slams that the doubler punishes. **Reverted** —
the floor's system-on (`2NT` = Jacoby, `3M` = limit raise) stays. Revisit only
under a single-dummy / IMPs-vs-humans measure where preemption actually pays.

## Defensive — their opening

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 43 | Cappelletti | override → **Woolsey** | add (Batch 1) | — | — |
| 84 | Michaels cuebid | shipped | keep | — | — |
| 127 | Unusual 2NT | shipped | keep | — | — |
| 126 | Unusual 1NT | gap | add (Batch 1) | — | — |
| 79 | Leaping Michaels | **shipped, default ON** | keep on | `4♣/4♦` strong 5-5 two-suiters + authored advances; A/B'd **+1.090/+1.452/board** (none/both, 40k filtered, ~24% div) vs prior defense. Inference reader decodes the two-suiter so `american_search` picks the advance by DD (+2.8/board directional, slam-capable). `set_leaping_michaels(false)` to disable. | (this commit) |
| 123 | Two-suit takeout double | gap | add (Batch 1) | — | — |
| 129 | Unusual 4NT | verify | — | — | — |
| 48 | Cue bid | partial | verify | — | — |
| 106 | **Rubensohl after double** (advancer, weak twos; = `Transfer`) | shipped (opt-in, default off) | **keep floor as default**; `Transfer` = best, kept opt-in | Transfer vs off −0.006/+0.084/board (none/both, 200k); Transfer vs Plain +1.85/+2.66/div | a6e7ab9 (`set_advance_sohl_style`) |
| 82 | **Lebensohl after double** (advancer, weak twos; = `Plain`) | measured, rejected | keep floor; `Plain` DD-negative | Plain vs off −0.108/−0.050/board (200k); Pam/Lawrence also rejected (see note) | a6e7ab9 |

**Lebensohl after a takeout double (advancer over a weak two) — measured;
best variant (`Transfer`) kept opt-in.** After `(2X)–X–(P)` the flat `advance_double` ladder can't
distinguish a weak long-suit hand from a constructive one, so the doubler
can't tell when to move. Four sohl structures were authored under the
`(2X)–X–(P)` prefix (reusing the Section-5 builders for `Plain` / `Transfer`,
plus `Pam` = `2NT` shows 5-5 minors and `Lawrence` = three-band
weak/INV/GF strength) and A/B'd on `sohl-after-double-ab` (contested
seat-swap, 200k filtered boards/cell). Headlines: `Transfer` vs floor
**−0.006 / +0.084 IMPs/board** (best, but DD-neutral); `Transfer` vs `Plain`
**+1.85 / +2.66 IMPs/divergent**; `Plain` vs floor **−0.108 / −0.050**;
`Lawrence` vs `Transfer` **−0.053 / −0.092 IMPs/board**; `Pam` vs `Transfer`
**−0.009 / −0.005 IMPs/board**. Mechanism: a takeout double already advertises
the fit (short in their suit, length elsewhere), so the floor's natural
advancing locates most fits — `Transfer`'s right-siding is DD-blind upside,
`Lawrence` loses 5-card-suit *shape* information by collapsing INV into a
single direct-3X slot, and `Pam`'s 5-5-minors trigger is too rare (~0.4 %
divergence) to recover the slot it eats from weak long-clubs. Only `Transfer`
(the best) is kept — behind `set_advance_sohl_style` as an opt-in (default
`Off`, the floor); `Plain` stays as the A/B arm, while `Pam` and `Lawrence`
were rejected and not retained in code; the `sohl-after-double-ab` harness is
kept. Stopper-routing ("slow shows /
fast denies") was *not* tested per user direction (strength was hypothesised
to dominate; the `Lawrence` loss is consistent with that). This **is** toggle
`#106` (`Transfer` = Rubensohl after double) and `#82` (`Plain` = Lebensohl
after double); the "our opening is doubled" responder case is a *separate* BBA
toggle (`Transfers if RHO doubles`), not this one. Revisit only under a
single-dummy measure that can see right-siding.

**Leaping Michaels (79) — shipped opt-in, a clear DD win once the advances were
authored.** Over their weak two, a jump to `4♣`/`4♦` names a 5-5 two-suiter with
game-forcing values: over a major it shows a minor + the *other* major; over `2♦`
the `4♦` cue shows both majors and `4♣` shows clubs + a major. Authored in
`defense_to_weak_two` behind `set_leaping_michaels` (default `Off`), with advancer
continuations in `leaping_michaels_advances` (a fit major game — taking even a
7-card fit, which scores well and makes on ten tricks; else the `5m` minor game;
never a passed-out partscore; over `2♦`, `4♥` is pass-or-correct to opener's
major). A/B on `leaping-michaels-ab` (contested seat-swap, 40k filtered, ~24 %
div): **+1.090 / +1.452 IMPs/board** (none/both) vs the floor.

*The first cut measured −0.6 / −0.9* — but that was the **unauthored advancer**,
not the convention: worst-board analysis showed the instinct floor *passing* the
two-suiter, leaving us in `4m` (or, over the `2♦` cue, declaring the opponents'
diamonds). Authoring the advance flipped the sign by +1.7 IMPs/board. The lesson
is the inverse of the obstruction wall (`#71`/`#100`): a constructive competitive
convention can win big on DD when it reaches a *better strain* — but only if the
whole sequence, advances included, is authored; a half-built convention measures
as a loss for a reason that has nothing to do with the idea.

The authored advance is capped at game. To let the bidder reach the slams a big
two-suiter is *for*, `Inferences::read` now decodes the overcall's two suits
(`leaping_michaels_reading`, post-walk like the Rubens cue), so the constrained
sampler conditions partner correctly and the live double-dummy search bidder
(`american_search`, `--features search`) prices the advance — 4M / 5m / slam — by
cardplay EV. The authored length rules become the fast-floor *prior*; DD disposes.
A directional A/B (search+LM NS vs authored-rules+LM EW, 60 filtered boards, trimmed
64-layout search) measured **+2.8 IMPs/board** for search *on top of* the rule floor,
and the auctions show it reaching the slams the game-capped rules cannot (e.g. a
`6♥` off the `2♦` both-majors cue, a `7♣` grand) — at the cost of a few search
overbids (the small-sample / shortlist noise; a larger run would tighten).
**Shipped default ON** (`Cell::new(true)`); `set_leaping_michaels(false)` recovers
the prior weak-two defense. The plan's "spend runtime for better calls" (M2.3)
makes `american_search` the blessed way to play it — the slam upside lives there,
while the fast floor's authored rules bank the clean +1.09/+1.45.

## Openings

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 20/24/26 | 1NT NT-style / 15-17 / 5422 | shipped (default) | keep | — | — |
| 5 | 1m allows 5M | verify | — | — | — |
| 132 | Weak natural 2♦ | shipped | keep | — | — |
| 133 | Weak natural 2M | shipped | keep | — | — |
| 92 | Namyats | conflict (natural 4-lvl) | A/B (Batch 3) | — | — |
| 33 | 4NT opening | gap (rare) | low priority | — | — |

## Jargon to confirm against authoritative text (at authoring time)

BROMAD (42) · Strength Lawrence structure (114) · Mixed raise (89) ·
Maximal doubles (83) · Super/Extended acceptance after NT (55,115) ·
Unusual 4NT (129) · Support 1NT (116).

## Out of scope (21GF turns OFF; user confirmed skip)

Multi 2♦ (90) · Benjamin 2D (36) · French 2D (60) · Gambling (61) ·
Gazzilli (63) · Ghestem (66) · Raptor 1NT (99) · plain Landy (78) ·
Multi-Landy as a whole (91 — but Woolsey's Multi-Landy *structure* is in via
override) · Polish two-suiters (97) · Wilkosz (134) · Kickback (72-74) /
Crosswood (45-47) · New Minor Forcing (94) / plain Checkback (44) ·
Soloway jump shifts (111-112) · Snapdragon (110) · weak jump shifts off in
21GF (130-131 — pons keeps weak JS regardless).
