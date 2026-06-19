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
| — | **Plain-4NT minor keycard** | shipped (small slam) | keep; grand-in-minor deferred | **+6.80/+8.76 IMPs/div (none/both)** (2M, 46 div, optimistic). **PD re-measure (5611eac): +5.41/+7.05 IMPs/div (none/both)** (10M, 202 div) — HOLDS, ship intact; PD trims the gain (rare push-to-failing-slam now doubled) but stays clearly positive. | 99da1b3 |

## Competitive — our opening contested

| # | Toggle | pons status | decision | A/B | commit |
|---|--------|-------------|----------|-----|--------|
| 80 | Lebensohl after 1NT | **shipped** | **TransferSmolen** default (Cohen base + `(2♦)` `3♣`-Stayman/Smolen/Leaping-Michaels); `Transfer`/`Plain`/`Rubensohl` kept as options | Transfer vs plain **+0.46/+1.24/div** (none/both, 200k); vs floor +0.35/+0.05; (plain vs floor +0.26, Ruben-v1 −1.68); 2NT-role swap true-Rubensohl −0.017/−0.046/board (200k). **PD re-val (5611eac): Transfer-vs-plain +0.46/+0.69/div HOLDS (ship decision intact); vs floor FLIPS to −0.66/−0.62/div — PD doubles the failing game-drives, harness-blind to the obstruction value.** **2NT-role swap re-measured under PD: +0.001/−0.023/board (none/both, 200k each) — neutral non-vul, still a clear loss vul; no flip; re-authored as `LebensohlStyle::Rubensohl` opt-in.** **TransferSmolen v2 (`(2♦)`-only: `3♣`-Stayman + Smolen, Jacoby-reshuffled transfers `3♦`→♥/`3♥`→♠/`3♠`→♣, Leaping Michaels `4♣`/`4♦`) vs Transfer (PD, 200k filtered/cell): +0.020/+0.024 board, +2.286/+2.822/div (none/both) → WIN, promoted to default.** **Top-step clubs transfer (`(2♦/2♥)`→`3♠`, `(2♠)`→`3♥` = forced GF 6+♣, completion `3NT`/`5♣`) added — was floored; theory-correct (Cohen wraps the chain to clubs). PD two-binary `transfersmolen` vs floor: −0.0008/−0.0012 board (none/both, 200k filtered/cell), ≈0.04% of boards; the losses are normal making games vs the floor's speculative overcall penalty doubles that perfect defense over-credits → DD-blind to competition, kept in default pending an SD re-measure.** | bfe5e59 (plain), bee9204 (transfer) |
| 105 | Rubensohl after 1m | floor (Rubens advances) | upgrade (Batch 1) | — | — |
| 100 | Responsive double | takeout shipped (toggle); overcall-ext opt-in Off | **keep both as-is under PD** | **PD re-measure (`responsive-ab`, 200k filtered/cell, both vs bare floor):** takeout-X-then-raise (= BBA's `Responsive double`, on in 21GF) **−1.18/−1.89/div** (−0.0003/−0.0006 per raw deal, none/both) → kept shipped (drag negligible + DD-blind obstruction, cf. Lebensohl-vs-floor); overcall-ext (non-standard; nearest = Snapdragon, off in 21GF) **−2.16/−3.53/div** (−0.0020/−0.0032 per raw deal) → still rejected (PD does not rescue the old −0.034/−2.37; *worse* vul). Now behind `set_responsive_takeout` (default on) / `set_responsive_overcall` (default off); defaults byte-identical. | (toggles + `responsive-ab`) |
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

**Naming + the TransferSmolen v1 experiment (80, follow-up — tried & reverted; superseded by v2 below).**
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

**TransferSmolen v2 (80, follow-up — shipped as the new default).** The narrowed
retry the user specified *wins*. It keeps Cohen untouched over `(2♥)`/`(2♠)`/`(2♣)`
and changes only the `(2♦)` branch, where `3♣` sits free below the `3♦` cue: `3♣`
becomes game-forcing Stayman (opener answers `3♥`/`3♠`, or `3♦` to deny — leaving
room for responder's Smolen `3♥`/`3♠`, which shows the 5-4), and the 3-level
transfers shift down to direct Jacoby (`3♦`→♥, `3♥`→♠, `3♠`→♣). The `3♠`→♣ leg is a
*forced* game-force — its completion is `4♣`, so `3♣` can never be the contract.
Two Leaping Michaels jumps are added: `4♦` = both majors 5-5, `4♣` = clubs + a 5+
major (classic shapes from `defense.rs`, but only `points(10..)` — partner opened a
15-17 `1NT`, so the 14+ a silent partner needs drops to ≈8 HCP after the 5-5
distribution upgrade). Key authoring subtlety: a 5-4 GF major hand fits both `3♣`
Stayman and a Jacoby transfer, so Stayman is gated to *exactly* a 4-card major and
weighted above the transfers — otherwise the hand would transfer and Smolen could
never fire. A/B (`lebensohl-ab --ns transfersmolen --ew transfer`, PD, 200k
filtered/cell): **+0.020/+0.024 IMPs/board, +2.286/+2.822 IMPs/divergent
(none/both)** — a clean, consistent win and a full reversal of v1's −1.31/−1.76.
Why it wins where v1 lost: v1's *standard low-Stayman* reached DD-worse contracts
and leaned on DD-blind right-siding; v2 keeps Cohen's transfer-through value over
the majors, *adds* genuine fit-finding the measure can see (5-3 major games via
Stayman+Smolen, 5-5 major games via Leaping Michaels), and only adds nodes over the
`(2♦)` Cohen base. Promoted: the `set_lebensohl_style` default is now
`TransferSmolen`; `Transfer`/`Plain`/`Rubensohl` stay as opt-in arms.

**The top-step clubs transfer (80, follow-up — shipped, theory-correct, DD-marginally-negative).**
Cohen's transfer chain runs *up the line through* the adverse suit, so the highest
3-level step has no suit above it to transfer into and wraps back to **clubs**:
`1NT–(2♦/2♥)–3♠` and `1NT–(2♠)–3♥` are a *forced* game-force transfer to clubs (6+♣,
`points(10..)`; completion `3NT` with a stopper in their suit, else `5♣` — `3♣` is
unplayable below the top step, so game is forced). Previously these fell to the
natural floor, leaving a 6+♣ GF hand with no call: the weak `2NT`→`3♣` relay is
`points(..=8)`, so it cannot carry a game force (bidding it strands the game in `3♣`).
`TransferSmolen` already had the `(2♦)`→`3♠`→♣ leg; this adds the same wrap for
`(2♥)`/`(2♠)` and for plain `Transfer` over `(2♦)`. Lives in the shared
`transfer_lebensohl_responder` builder (guarded `over != Clubs` — over `(2♣)` clubs is
their suit) plus a generalized `clubs_transfer_completion(over)`. A/B (two binaries at
a fixed seed, `--ns transfersmolen --ew off`, PD, 200k filtered/cell): after − before
= **−0.0008/−0.0012 IMPs/board (none/both)**, ≈87 boards changed (0.04%), ≈−1.8/−2.8
IMPs each. The worst boards are textbook DD-blindness: the transfer reaches a normal
making `3NT` (e.g. 27 combined HCP with a running club source), while the floor
instead makes a *speculative penalty double of the overcall* (`2♦×`/`2♥×`) that
perfect double-dummy defense turns into a giant set — the harness over-credits the
defense, exactly the obstruction-blindness flagged for Lebensohl-vs-floor above. Kept
in the default as a theory-correct completion (the bid a 6+♣ GF hand otherwise lacks),
pending a single-dummy re-measure. (Cohen's full *slow-shows-stopper* layer —
`2NT`→`3♣`→cue = Stayman *with* a stopper — is a separate, unimplemented refinement;
`2NT` here is only the weak relay.) `lebensohl-ab` gains `--seed` (deterministic
two-binary runs) and `--only-topstep` (restrict to top-step boards; note it also
catches floor `3♠`-natural auctions, so the clean isolation is the two-binary delta).

**The `2NT`-role A/B (80, follow-up — measured twice, kept opt-in).** The `2NT`-role
swap — **true Rubensohl** (`2NT` an artificial **club** transfer) vs the relay
(Transfer Lebensohl) — was authored on the Cohen structure and A/B'd vs `Transfer`.
Design (jdh8's rule): every transfer to a suit *below* the overcall is two-way (weak
transfers then passes; strong continues), so `2NT`=clubs is two-way; transfers to
a suit *above* the overcall stay INV+ (weak hands escape with a natural 2-level
bid), identical to `Transfer` — so opener still auto-drives those to game. The
**original DD test was −0.017/−0.046 IMPs/board (none/both, 200k filtered each)** — a
clear, consistent loss. Mechanism: for weak hands both arms reach the *same*
contract (right-siding the low-suit partscore is DD-blind), so Rubensohl's only
gain is invisible to DD, while making the low transfers two-way *costs*
`Transfer`'s auto-drive-to-game on invitational hands (opener accepts minimally,
the borderline responder stops short). **Re-measured under perfect-defense scoring
(5611eac, the user's question): +0.001/−0.023 IMPs/board (none/both, 200k filtered
each)** — neutral non-vul, still a clear loss vul. PD only doubles *failing*
contracts; it does not let the harness see *who declares*, so it cannot reward
Rubensohl's right-siding edge — the verdict is unchanged. Per the user's gating
("if the cheap probe stays neutral/negative the full standard ladder won't rescue
it, since its extra structure — Smolen, transfer-into-suit, 3♠-minors — is all
right-siding"), the full standard ledger was **not** authored. The variant is kept
as `LebensohlStyle::Rubensohl` (opt-in via `set_lebensohl_style`; the default stays
`Transfer`) for a future single-dummy / live-search re-measure that can see
right-siding (e.g. `lebensohl-ab --ns rubensohl --ew transfer`).

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
| 79 | Leaping Michaels | **shipped, default ON** | keep on | `4♣/4♦` strong 5-5 two-suiters + authored advances; A/B'd **+1.090/+1.452/board** (none/both, 40k filtered, ~24% div) vs prior defense. Inference reader decodes the two-suiter so `american_search` picks the advance by DD (+2.8/board directional, slam-capable). `set_leaping_michaels(false)` to disable. **PD re-val (5611eac): +1.100/+1.445/board — unchanged (reaches making GF games, almost no failing contracts to double).** | (this commit) |
| 123 | Two-suit takeout double | gap | add (Batch 1) | — | — |
| 129 | Unusual 4NT | verify | — | — | — |
| 48 | Cue bid | partial | verify | — | — |
| 106 | **Sohl after double** (advancer, weak twos) | **shipped, `Transfer` default ON**; true `Rubensohl` opt-in | `Transfer` promoted to default (clear PD win); true `Rubensohl` wired but opt-in | **Deeper PD re-measure (200k filtered/cell): Transfer vs off +0.145/+0.227/board none/both (+1.48/+2.33/div; reps +0.139/+0.234) → PROMOTED to default.** true-`Rubensohl` now wired (2NT=club transfer, two-way low transfers) and measured: vs off +0.140/+0.212, but **Rubensohl vs Transfer −0.007/−0.037/board (2.5% div) → no gain, kept opt-in** (its edge is DD-blind right-siding, same finding as the 1NT context). | (this commit) (`set_advance_sohl_style`) |
| 82 | **Lebensohl after double** (advancer, weak twos; = `Plain`) | measured; opt-in, dominated | `Transfer` (#106) is the default; `Plain` positive but worse | **PD re-measure: Plain vs off FLIPS to +0.089/+0.139/board (200k filtered) — DD-positive (was −0.108/−0.050 optimistic), but dominated by `Transfer` (+0.145/+0.227).** Kept as the `Plain` opt-in / A/B arm. | a6e7ab9 |

**Lebensohl after a takeout double (advancer over a weak two) — measured;
best variant (`Transfer`) PROMOTED to default.** After `(2X)–X–(P)` the flat `advance_double` ladder can't
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
divergence) to recover the slot it eats from weak long-clubs. Stopper-routing ("slow shows /
fast denies") was *not* tested per user direction (strength was hypothesised
to dominate; the `Lawrence` loss is consistent with that). This **is** toggle
`#106` and `#82`; the "our opening is doubled" responder case is a *separate*
BBA toggle (`Transfers if RHO doubles`), not this one.

**Update (this commit) — perfect-defense re-measure + `Transfer` promoted to
default + true `Rubensohl` wired.** The old "DD-neutral → keep `Off`" basis was
an artifact of the optimistic scorer. Under perfect defense (`ns_score`, 200k
filtered boards/cell, both vulnerabilities) the picture is clearly positive and
`Transfer` is promoted from opt-in to the **default** advance-of-double sohl:
`Transfer` vs `Off` **+0.145 / +0.227 IMPs/board** (none/both; reps
+0.139/+0.234, ~9.7% divergent, +1.48/+2.33 IMPs/div). `Plain` (#82) also flips
DD-positive (**+0.089 / +0.139** vs `Off`, was −0.108/−0.050) but stays dominated
by `Transfer`, so it remains the `Plain` opt-in / A/B arm, not the default. True
`Rubensohl` (the fourth `LebensohlStyle`: `2NT` = artificial club transfer, the
low transfers two-way) is now **wired into the `(2X)–X–(P)` context too** (a
verbatim mirror of the Section-5 1NT-context wiring; exposed as
`--ns rubensohl` on `sohl-after-double-ab`): vs `Off` **+0.140 / +0.212**, but
head-to-head **`Rubensohl` vs `Transfer` −0.007 / −0.037 IMPs/board** (only ~2.5%
divergent) — no gain, so it is **kept opt-in** (its edge is DD-blind right-siding,
exactly the 1NT-context finding). Default is now `Transfer`; `Off` / `Plain` /
`Rubensohl` remain selectable via `set_advance_sohl_style`. Revisit `Rubensohl`
only under a single-dummy measure that can see right-siding.

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

**Perfect-defense re-validation sweep (after `ns_score` fix 5611eac).** `ns_score`
now doubles any contract that fails double-dummy (a real defense doubles what it
can beat); the optimistic-undoubled variant is gone. That re-priced every prior
A/B, so the toggle-based competitive conventions were re-measured. PD only changes
scores on divergent boards where one arm reaches a *failing* (now-doubled)
contract, which splits the results cleanly:

- **Convention-vs-convention CHOICES all hold** (both arms share the obstruction
  value, so it cancels and only the constructive/placement edge remains): Leaping
  Michaels on-vs-off **+1.100/+1.445/board** (was +1.090/+1.452, 40k filter);
  Transfer-vs-Plain Lebensohl **+0.46/+0.69/div** (was +0.46/+1.24, both-vul
  shrank, 80k filter-dh); wide-vs-classic 1NT **constructive** **+0.11/+0.32/div**
  (was +0.32 none, `nt-shape-abc` 500k). **No ship decision is overturned.**
- **Convention-vs-NOTHING comparisons flip negative** for the obstructive ones —
  PD sharpens the documented DD-blindness-to-obstruction (the solver sees through
  the obstruction; PD now fully counts the failing-overbid cost): Transfer
  Lebensohl vs the bare floor **−0.66/−0.62/div** (was +0.35/+0.05); wide-vs-classic
  1NT **contested** (`nt-shape-contested`, 100k) **−0.50/−0.61/div** (was
  +0.57/+0.93). Per-board impact is tiny (−0.005…−0.058 — rare auctions). The flip
  is a harness artifact, not a bad convention.
- **sohl-after-double (#106) flipped DD-POSITIVE** — see the row; its opt-in
  rationale is gone (promotion candidate, kept opt-in pending a deeper re-measure).
- **The two-binary *constructive* conventions** (Stayman, 1NT-3♦ #14, Puppet #12,
  SAT #119, M6.1 inferences, minor keycard #75): they reach *making* contracts, so PD
  only doubles the looser baseline's failures → predicted to hold or improve, and they
  shipped with large margins. Re-validating each needs a per-feature worktree rebuild
  (old-arm `--phase bid` → new-arm `--phase score`; the harness file format is stable).
  **minor keycard #75 now empirically re-run under PD: +5.41/+7.05 IMPs/div (none/both,
  10M, 202 div) — HOLDS** (was +6.80/+8.76 optimistic; PD trims but stays clearly
  positive; isolated by reverting just 99da1b3, zero drift in the touched files). The
  other four remain predicted-only (lowest priority; SAT #119 has the most failing-slam
  exposure).

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
