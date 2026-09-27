# Defensive auctions — how BBA and BEN bid them, and what pons can lever

Research distillation, 2026-09-27. "Defensive" = they open, we act: direct
seat, advances, the doubler's/overcaller's rebid, balancing, and defences to
their preempts and 1NT. Three sources, read side by side:

- **BBA** — the shipped book dump (`ab-results/bba-book/2026-08-23-08c54312-dirty`,
  rendered with `probe-bba-book --render … --prefix=…`) plus a fresh `ilspycmd`
  decompile of `vendor/bba/EPBot64.dll` (63 616 lines; the same decompile
  [bba-floor.md](ai-bidder/bba-floor.md) cites, so `EPBot.cs:N` below lines up
  with that doc). `MB.TXT` is a 2009 export the engine never loads; where the
  two disagree the decompile wins (see §5).
- **BEN** — `~/ben` at v0.8.8.4, `src/botbidder.py`, and the declared card
  `vendor/ben/BEN-21GF.bbsa`. Mechanism map in
  [ben-architecture.md](ben-architecture.md).
- **pons** — `src/bidding/american/defense/`, rendered with `render-book`, at
  `113983fa`.

## 1. Headline

**The defensive floor cells are pons's largest hole vs BEN, and BBA's book
is a ready-made spec for each of them.** Tier-S ranking of 2026-09-16
([ben-gap-campaign.md](ben-gap-campaign.md) §"ranking"):

| bucket | boards | plain | per board | PD |
| --- | --- | --- | --- | --- |
| Defensive / book / round-1 | 2 199 | −3 218 | −1.46 | −2 963 |
| Defensive / floor / round-2 | 1 076 | −1 801 | −1.67 | −1 367 |
| Defensive / floor / round-1 | 759 | −1 542 | **−2.03** | −909 |
| Competitive / book / round-1 | 710 | −1 451 | −2.04 | −969 |

The two floor cells together (−3 343) outweigh the book cell, and the round-1
floor cell is the worst per board of any bucket. Defensive lanes run 2.0–2.5×
worse per board vs BEN than vs BBA. Round-1 floor calls are exactly the lanes
pons has **zero book nodes** for: the balancing seat `(1x) - -`, their 3-level
preempt `(3x)`, their strong `(2♣)`, and the 3-level jump overcall. Round-2
floor calls are the advances pons left to the floor by design: every advance
of a natural overcall `(1x) 1y -` / `(1x) 2y -` (with the floor's Rubens
transfers **off**, a measured loss), the weak-jump advance, and most
interfered tails.

BBA authors all of these (§2). BEN has no defensive module at all, but in
Tier S it special-cases exactly the passout and post-preempt seats where pons
is floor-only (§3).

> **Corrected by the §7 prefix split (2026-09-27).** The lane list in the
> paragraph above was inferred from pons's missing nodes, not from the rows.
> The rows disagree: `family` is the actor's *own* round, so round-1 floor
> is mostly advances and fourth-seat calls. The balancing seat sits in its
> own `balancing` family, and there, like `(3x)` direct, it is PD-positive.
> L1 and L2 fail their gates, and L3's lanes lose to BEN but not to BBA.

## 2. BBA's defensive book

### 2.1 Design

Each seat is one dispatcher — `odzywka1_pierwszego_broniacego` (direct and
balancing overcaller, `EPBot.cs:6868`), `odzywka1_drugiego_broniacego`
(advancer, `:8036`), `odzywka2_pierwszego_broniacego` (doubler's/overcaller's
rebid, `:7373`). Each first fills a veto table, then tries **named writers in
fixed order** (Leaping Michaels → Ghestem → Michaels → 1NT-defence gadgets →
Unusual NT → `wejscie_zaporowe` preempts → reopening 2NT → jump cue-bids → 1NT
overcall → 2NT over a weak two → balancing 1NT → natural minor → takeout /
reopening double → strong-suit entry, `:7145-7362`), returning at the first
writer that fires; only if nothing wrote does the seat fall to the bilans
(`odzywka_z_bilansu`, `:7367`, advancer `:8773`, rebid `:7786`).

Two design facts worth copying:

- **Gadgets are book; the plain raise is floor.** The advancer's simple raise
  of an overcall, the doubler's raise of the advance, and the contested simple
  raise all render as `calculated bid`. The cue, the NT rungs, the preemptive
  jump raise, splinters, responsive double and Rubensohl are book.
- **The bilans can veto book calls** (`determine_odzywki_zabronione_z_bilansu`,
  `:54854`): every bid below its own minimum level, 3NT after a cue, cue-bids
  in suits our side holds < 8 cards. The advancer additionally vetoes 1NT on
  `probable_level[NT] < 0` (`:8144`), and the takeout double's minimum is
  `max(min_HCP_odzywki, …)` where `min_HCP_odzywki` is the bilans' floor
  (`:45230-45236`).

### 2.2 Direct seat over `(1x)`

| call | band | source |
| --- | --- | --- |
| 1-level suit | 8–17, 5+ (7 HCP if nobody on our side has called) | `:17631-17645` |
| 2-level suit | `max(…, 12)` HCP, 5+; a 6-card suit −2 but never below 12 | `:17639` |
| quality gate | `quality(suit) + 2·(points − 10) > 8 + 2·level` | MB.TXT:977, [defensive-overcalls.md](defensive-overcalls.md) §O4 |
| 1NT | 15–17 HCP, `HCP_USEFUL ≤ 17`, stopper, ≤ 5422, not 4441, majors ≤ 8 cards | `:7068`, `:7301` |
| X | **12+**, 3+ in each unbid, ≤ 4 in theirs | `:27510`, `:27556` |
| weak jump 2M | 4–10, 6–7 cards | render |
| 3-level jump | preemptive 4–10, 7–8 cards | render |
| Michaels / UNT | 8–29 / 9–29, 5-5; UNT needs `3 + (vul_us − vul_them)` honours per minor | `:3948` |
| 3NT | 22–29 with a stopper | render |

Preempt gate `wejscie_zaporowe` (`:3897`): `zar_points ≥ 18` and
`HCP_USEFUL[suit] ≥ min_preemptive_HCP + korekta + 3·(7 − len)` for a 3-level
jump, `3·(6 − len)` for a weak 2M, `2·(8 − len)` for 4m — **three HCP per
missing card**. `min_preemptive_HCP = 4 + (2·vul_us − vul_them)`, capped 10
(`:25397-25401`); +2 when they opened 15+ and we passed; +3 as a passed hand;
10–15 in fourth seat (`:25375-25405`).

Takeout-double HCP correction (`get_korekta_HCP_takeout_double`,
`:48000-48047`): +2 per Q / +1 per J in their suit when LHO holds 4+ there,
−1 for a singleton without honour, +1 for 4333, `+max(14 − HCP, 0)` per
doubleton in an unbid major. Direct-seat strength (`jest_sila_kontry`,
`:45417-45460`): −2/−1 for 4+ cards or a singleton in their suit, +1 for 4333
opposite a 17+ opening, `+vulnerable` when the deck is thin.

Live statistic: `takeout double` is BBA's worst reader-vs-bidder label — 35 %
of its own doubles fall outside the 12+ band, and it doubles on 10
([bba-book.md](ai-bidder/bba-book.md) §3.3).

### 2.3 Takeout-double layering and the doubler's rebid

| situation | minimum | source |
| --- | --- | --- |
| direct | 12 | `:27556` |
| reopening, 1-level | **8** (max 17 as a first call; `10 + vul` when their suit is agreed) | `:45254`, `:27381`, `:45266` |
| over their 18+ opening | 16; over a strong 1♣ 14 | `:45274`, `:45269` |
| second double | `max(min + 3, 17)` | `:45235-45236` |

Doubler's rebid over a minimum advance, `(1♥) X - 1♠ -` (render): `1NT` 18–20
stopper, `2♣/2♦` 16–29 5+, `2♥` cue 17+, `2♠` raise 15–20 4+ (**floor**),
`2NT` 21–22, `3♣/3♦` 18–29 6+, `3NT` 23–29, `P` 12–19. After opener rebids,
`(1♦) X - 1♠ (2♦)`: second `X` 17+, `3♦` cue 17+, `2♠` 15–20, `P` 12–22.

### 2.4 Advances

**Of a takeout double** `(1♥) X -` (render): cheapest suit ≤ 9 4+ (2-level ≤
10); `1NT` 6–10 stopper; cue **10+** `[strength cue bid]`; jump `2♠` **7–12**
4+ (invitational, not weak); `3m` 8–12 5+; `2NT`/`3NT` 11–12 / 12–18 stopper;
`P` ≤ 12 with 4+♥ (penalty pass). After `(1♦) X (XX)` the same ladder with
`2♣` widened to ≤ 12 and the cue `[artificial]`. Code: 1NT vetoed on `HCP ≤ 6
− (X/XX flag)` (`:8144`); with ≤ 8 HCP and a 4-card major the minor is vetoed
so the major is bid (`:8148`); Lebensohl after partner doubles their raise
(`:8060-8069`); responsive double needs LHO opened, partner's first call not
pass/1NT, both opponents bid (`:37908-37913`).

**Of a natural overcall** `(1♦) 1♠ -` (render):

| call | band |
| --- | --- |
| 2♠ raise | 7–12, 3+ — **floor** |
| cue 2♦ | 13+ (9+ over a 2-level overcall), 3+ trumps, `[limit raise or better]` |
| 3♠ | preemptive 3–6, 3+; 4♠ preemptive 4–13, 4+ |
| 1NT / 2NT / 3NT | 10–14 stopper ≤ 2♠ / 14–16 / 16–22 |
| new suit | 2♣ 14–21 5+, 2♥ 9–21 5+; jumps 16–21 |
| 4♣ | splinter 15–23, ≤ 1♣, 3+♠ |
| 4NT | RKCB 1430, 20+ |

**Contested advance** `(1♥) 1♠ (2♥)`: `2♠` 7–11 3+ (floor); `3♥` cue 11+ 3+♠;
`3♠` preemptive 3–6; `X` `[Responsive double]` **11+, 4–5 in each unbid
minor, 1–3♥, ≤ 2♠**; `2NT` 15–16 stopper; `3m` 9–19 6+; `3NT` 16–22; `4♣`
splinter 14–22; `P` ≤ 16.

**Of the 1NT overcall** `(1♥) 1NT -`: systems fully on — Garbage Stayman,
`2♦` **natural 7–14 5+♦** (no transfer into their suit), `2♥` = transfer to ♠,
`2♠`/`2NT` minor transfers, `3♣` Puppet 10+, `3♦` 5-5 majors, `3M` splinters
9–14, `4♦` Texas, `4NT` quantitative 15–17, `P` ≤ 9.

### 2.5 Balancing seat `(1♠) - -`

Gate `get_balancing_situation` (`:31555-31562`): exactly two passes, no X/XX,
level 1 or below 2NT with their agreed suit. Balancing turns `wejscie_zaporowe`
off (`:3902`), so jumps become intermediate.

| call | band | source |
| --- | --- | --- |
| X | `[reopening double]` **8+**, 3+ each unbid, ≤ 3 in theirs | `:45254` |
| 1NT | 12–15, stopper | `:7311` |
| 2♣/2♦/2♥ | 10–15, 5+ | render |
| 2♠ | Michaels 11–29 | render |
| 2NT | **19–21**, four stoppers (16–18 when reopening at the 2-level) | `:7222`, `:7227` |
| 3-level jump | **13–15, 7+** intermediate | `:3902` |
| 3NT | 22–29 | render |
| P | ≤ 14 | render |

Advances of the balancing double `(1♠) - - X -`: `1NT` 10–14; `2♣/2♦/2♥` ≤ 14
4+; `2♠` cue 14; `2NT` 15; `3x` 12–13 5+; `3NT` 16; `P` ≤ 14 with 4+♠.
Reopening HCP is **boosted** by wasted values (`get_podwyzszenie_HCP_reopening`,
`:47946-47993`): +3 for a singleton K in their suit, +2 per Q, +1 per J,
`len(their suit) − 2` at the 2-level, plus any undertrick deficit the bilans
computes.

### 2.6 Their preempts

| call | over `(2♠)` | over `(3♦)` |
| --- | --- | --- |
| 2NT | 15–17, stopper (`set_reka_ba(12,14,15,17)`, `:7303-7306`) | — |
| new suit | 12–36, 5+ | 12–36, 5+; 4♣ 14–36 |
| cue | Michaels 12–36 | 4♦ Michaels 13+, 5-5 majors |
| 3NT | 22–36, stopper | **17–36**, stopper |
| X | 12+, 3+ each unbid, ≤ 4 theirs | 12+, ≤ 4♦, 3+ elsewhere |
| 4m | Leaping Michaels 13+ 5-5 | 4NT Unusual 16+ |
| P | ≤ 16 | ≤ 18 |

Advances of `(2♠) X -`: `2NT/3♣/3♦` Rubensohl transfers (≤ 24), `3♥` 13–24 4+,
`3♠` 12+ stopper-ask, `3NT` 12–18 stopper, `P` 3+♠ `[accepts double]`, `4♥`
13–24.

### 2.7 Their 1NT

The dump has **our card** on all four seats, so `(1NT)` shows BBA playing our
natural defence: `2x` 12–17 5+, `2NT` Unusual 9–25, `X` takeout 13+ 3+ in
every suit, `P` ≤ 17. BBA's native defence is Woolsey Multi-Landy
([bba-1nt-defense.md](ai-bidder/bba-1nt-defense.md)); our Woolsey trails
BBA's by 0.23 IMPs/board NV plain on the identical card — a continuation
quality gap ([gto-1nt-defense.md](ai-bidder/gto-1nt-defense.md) §"identical
card").

## 3. BEN's defensive bidding

No module: every defensive call runs the same policy → candidates → sample →
DD rollout → BBA blend pipeline (`botbidder.py:224`). The declared card
(`BEN-21GF.bbsa`) is stock 2/1 with Michaels, UNT, Leaping Michaels,
Lebensohl after a double of a weak two, gambling jump cue-bids, responsive and
maximal doubles, fit jumps, Jordan 2NT, and **Cappelletti** over 1NT.
Overcall strength, 1NT-overcall range and takeout shape are not card rows;
the BBA-8730 teacher's defaults apply (BEN's training file was generated with
BBA, `ben:scripts/training/README.md:111-114`; "GIB" is the 2/1 flavour label).

### 3.1 The Tier-S patches that matter defensively

Tier F (`BEN-21GF-F.conf`, `search_threshold = -1`) returns the first legal
argmax at `botbidder.py:886-914` — **before** every patch below. Every Tier-F
A/B pons has run therefore never saw the parts of BEN that make it a
balancing and doubling opponent.

| patch | mechanism | `botbidder.py` |
| --- | --- | --- |
| passout | `no_bids > 3 and last == PASS` → `min_candidates = 2`, threshold one rung lower | `:920-925` |
| "assume we're doubled" | a below-threshold non-pass in passout has negative EV **halved** or `−100` subtracted | `:394-402` |
| passout X | drop the best 25 % of sampled boards; charge 100 (200 if they are vul) unless EV clears +200 / +100 | `:405-444` |
| passout XX | charge 200 / 400 | `:446-454` |
| any low-confidence X | `insta_score < 0.5` after 4+ calls → −100 (−200 above the 5-level); `< 0.1` → −200 | `:455-477` |
| penalty X sanity | BBA reads it as penalty → docked for a singleton/void in trumps and for 0–1 controls | `:363-390` |
| after their preempt | BBA labels the bid weak/preempt → force 2 candidates, threshold two rungs lower, within the first 4 calls ("the model is lacking some bidding") | `:937-948` |
| rescue | replace a final PASS if ≥ 20 samples show ≥ 500 gain; always evaluated in passout | `:124-184`, `:590-810` |
| NN trust | 200× undisturbed, 60× in competition — and `undisturbed` **ignores an opponents' double** (`bidding.py:132-134`) | `:355-358` |

Two quirks read from source, not in [ben-architecture.md](ben-architecture.md):
`passout` is dealer-parity dependent (it misses `(1x) - -` unless dealer
passed first, and fires for opener after `1♠ - 2♠ -`), and there is **no
sacrifice logic** (`:432`).

### 3.2 Why BEN wins defensive boards (per the docs)

- Its Tier-F policy is ruly at every depth: LHO-overcall nodes ceiling
  96–99 %, advancer nodes 92–100 % ([ben-gap-campaign.md](ben-gap-campaign.md)
  §distillation probes) — the *entry* decisions are rules pons can write.
- The rest is Tier-S DD search, ≈ 0.8–1.1 IMPs/board globally, "not
  authorable".
- It preempts and jumps only with a good suit — honour concentration flips
  2M to PASS.
- It doubles directly where BBA structurally cannot — which is also why a
  doubling-driven PD win vs BEN needs the meta check before it ships
  ([defensive-overcalls.md](defensive-overcalls.md) §O4-vul).

## 4. The gap — pons today, per lane

| lane | pons | BBA | BEN | lever |
| --- | --- | --- | --- | --- |
| balancing `(1x) - -` | **0 nodes**, floor; decided out of scope | full ladder §2.5 | passout patch (Tier S) | **L1** |
| their `(3x)` | **0 nodes**, floor | full table §2.6 | forced 2nd candidate | **L2** |
| advances of a natural overcall `(1x) 1y -` | **0 nodes**; floor's Rubens off (LOSS 07-31) | cue = limit+, NT rungs, preemptive jump raise, new suit, splinter | ruly 92–100 % | **L3** |
| contested advance `(1x) 1y (2x)` | 0 nodes; responsive-overcall X opt-in (NV win, vul loss) | responsive X 11+ both unbid, cue 11+, preemptive raise | — | L3 |
| Michaels / UNT doubled, `(2t) X (3t)`, `(1NT) 2t (X)` | floor | authored | — | L3 tails |
| doubler's rebid `(1x) X - 1y -` | floor by default (ladder only behind `defensive_seam_split`, wash) | §2.3 | — | re-measure with more boards |
| direct X | 12+ shapely / 18+ any | 12 direct, 8 reopening, 17 second; HCP corrections §2.2 | mixed sign on `P → X` | L4 |
| 2-level overcall | 11+ pts, no quality atom; tight variants refuted / meta-vetoed | 12 HCP floor + quality gate | passes all 572 traced boards | none — both references agree, the harness cannot price it |
| 3-level jump overcall | 0 nodes (O6 owed) | 4–10, 7–8 cards, 3 HCP/missing card | DD-negative lane | O6 as owed |
| weak two `(2x)` | full package, vul bands shipped | §2.6 | — | done |
| advances of `(2x) X` | transfer Sohl shipped | Rubensohl | Lebensohl | done |
| their 1NT | natural default, M1+M2 shipped | Woolsey native | Cappelletti declared | Woolsey continuations, if reprioritised |
| their Multi 2♦ opening | none; reads as a natural weak 2♦ | none | n/a (2/1) | none today |
| penalty-X sanity | accountant + four floor double rules gated ≤ 3-level | bilans | trump-control + control-count dock | **L5** |
| rescue / final-contract check | none | none | Tier S | M8 search, deferred |

## 5. Discrepancies flagged (not resolved)

1. **Balancing "out of scope by decision"** ([defensive-overcalls.md](defensive-overcalls.md)
   §out-of-scope; [takeout-double-layers.md](takeout-double-layers.md)
   §out-of-scope) vs the Tier-S `Defensive/floor/round-1` cell being the worst
   per-board bucket of all. Proposed reversible default: reopen as L1 behind a
   knob, default off until measured.
2. **MB.TXT vs the live engine**: second double 18 (MB.TXT:3728) vs 17
   (`:45235`); 1NT overcall 16–18 (MB.TXT:3466) vs 15–17 (`:7068`).
   [defensive-overcalls.md](defensive-overcalls.md) §"Reference behavior" and
   [takeout-double-layers.md](takeout-double-layers.md) §"Current state vs BBA"
   quote the 2009 numbers. Proposed: annotate those sections with the
   decompile values; no code change.
3. **Their Multi 2♦ opening** is read as a natural weak 2♦ by
   `weak_two_defense`; `TheirDisclosures::two_diamonds_multi` reaches only the
   `1NT (2♦)` lane. Neither reference opens Multi, so zero KR1 value today.
   Proposed: leave, note in the module doc.
4. **PD-scorer definition mismatch** (`ns_score_bid` vs `ns_score_pd`, ≈ 0.07
   IMPs/board on the M32 headline) is still owed to jdh8; every PD number
   above carries it.
5. **BEN passout quirks** (§3.1) — [ben-architecture.md](ben-architecture.md)
   §4's "balancing/passout" line is rosier than the code. Proposed: one
   sentence there pointing here.

## 6. Levers, ranked

Each lever names its gate — the probe that must confirm the bucket before
authoring, per the measurement iron rules.

- **L1 — author the balancing seat with BBA's ladder** (§2.5): X 8+ with
  wasted-value boost, 1NT 12–15, 2NT 19–21, intermediate 3-level jumps 13–15
  7+, natural suits 10–15, plus the advances of the balancing double. One
  package, one knob, both references. Gate: split `Defensive/floor/round-1`
  by prefix in the Tier-S decompose and confirm `(1x) - -` boards carry the
  loss. Expected mechanism: BEN's passout patch is a search-side answer to
  the same hole; BBA's is a rule-side one we can copy directly.
- **L2 — author `(3x)`** (§2.6): X 12+ ≤ 4 theirs, 3NT 17+ stopper, suits
  12+ 5+, 4m Michaels 13+, 4NT 16+. Cheapest package here; same gate as L1.
- **L3 — author advances of a natural overcall** (§2.4) including the
  contested `(1x) 1y (2x)` tail and the doubled Michaels/UNT tails: cue =
  limit raise or better, 1NT/2NT/3NT stopper rungs, preemptive jump raise,
  constructive new suit, splinter; keep the plain raise as BBA's 7–12 3+
  catch-all band. This is the `Defensive/floor/round-2` cell. Gate: prefix
  split of that cell. Risk: a book node shadows the v6 floor, which is
  BBA-distilled and may already play some of this — the prefix split says
  whether the floor is losing there.
- **L4 — BBA's takeout-double corrections as an evaluator term** (§2.2):
  wasted honours in their suit, the 4333 bonus, the unbid-major doubleton
  term, the reopening boost. Fits `takeout_double_shape_ok` as a points
  adjustment rather than a shape gate. Gate: the `P → X` mixed-sign slice
  needs a split by these features first.
- **L5 — BEN's penalty-X sanity as a floor rail**: veto a penalty double with
  a singleton/void in trumps or 0–1 controls; charge a low-confidence X. Same
  family as the shipped junk-action rails. Gate: census how often the floor's
  doubles fail these tests on divergent boards.
- **Not a lever under the rules**: BEN's alert blindness and
  `opponent_model = bidder_model` make artificial defensive calls
  systematically mis-read — an exploit, and BBA is the exploit guard.

## 7. Gate probe results (2026-09-27)

**Method.** The auction prefix up to `div_index` is identical at both tables.
So each `boards.jsonl` row joins its shard board on `(vul, seed, board)`, and
the prefix gives the lane. The rows are floor-provenance `Defensive` rows,
normalised per 1 000 boards. BEN is the Tier-S anchor
`ben-anchor/2026-09-13-daa8bf4a` (20k boards). BBA is the **shipping** arm of
`anchor/2026-09-26-7e0bc648` (409.6k boards). Its rows were re-decomposed
with `--our-floor american --jsonl` at that commit (replay 100.00%) and kept
as `boards-american.jsonl` in the snapshot. `anchor.sh` wrote no rows for
that arm before this probe.

| lane (floor) | BEN bd | plain | PD | BBA bd | plain | PD |
| --- | --- | --- | --- | --- | --- | --- |
| `(1x) 1y (z)` | 6.30 | −15.10 | −10.80 | 5.51 | −1.59 | **+4.27** |
| `(1x) 1y -` | 6.00 | −14.00 | −9.05 | 3.15 | −1.64 | **+0.74** |
| `(1x) 2y -` | 4.75 | −11.75 | −7.00 | 4.05 | −1.18 | **+2.28** |
| `(2x) - (y)` | 2.65 | −11.45 | −12.45 | 1.80 | −3.25 | −3.68 |
| `(1NT) - (y)` | 2.00 | −7.50 | −8.95 | 2.58 | −1.59 | −3.75 |
| `(2x)` direct | 1.05 | −3.70 | −4.25 | 0.83 | −1.06 | −1.12 |
| `(3x) - -` | 1.65 | −2.65 | −1.75 | 1.23 | +0.22 | +0.55 |

In the BEN rows (absolute counts), the balancing seat `(1x) - -` is 153
boards at −106 plain / **+90 PD**, the passout-seat half of the
`Defensive/floor/balancing` cell (374 bd, −373 / +20). `(3x)` direct is
73 boards at −2 / **+131**.

**Verdicts.**

- **L1 fails its gate.** The balancing loss is plain-only and PD-positive:
  a doubling artifact, not a bidding hole. The "out of scope" decision
  stands, so §5 item 1 needs no action.
- **L2 fails its gate.** `(3x)` has no loss to recover.
- **L3's premise is refuted.** Against BBA, the v6 floor already plays
  these lanes at plain ≈ −1.5 and PD positive per 1 000 boards. It is
  BBA-distilled and has learned BBA's ladder, so authoring that ladder as
  a book node can at best copy the floor, and it risks the shadowing and
  over-reach that sank the Rubens layer (07-31). The BEN loss is
  BEN-specific. In the 405 BEN boards the pattern is *we pass or raise
  once, BEN raises higher or cue-raises*: `P` vs `3y` over `(1x) 2y -` is
  44 bd, −61 / −30, and `P` vs `4y` over `(1x) 2y (z)` is 11 bd, −57 / −19.
  That is a competitive-raise aggression question, not a missing
  structure, and its BBA-guard price is the open risk.
- **Two small lanes lose to both references:** fourth seat over their
  weak-two response `(2x) - (y)` and over their 1NT response
  `(1NT) - (2x)`. Each is a ≤ 0.004 IMPs/board ceiling vs BBA. The 1NT
  one already has a BBA template on file (fourth-seat Stayman/transfer
  defence).

**(a) traced: the raise-aggression gap is diffuse, not a lever (2026-09-27).**
I split the L3 rows where we and the reference disagree only about
passing vs raising partner's suit. The split is by clean or contested,
our call vs theirs, support (3 / 4+) and HCP band. The gap vs BEN
spreads over about 40 cells. Only one cell is coherent across both
references: **clean `(1x) 1y -`, we pass, they raise to `2y`, 3-card
support, 8–10 HCP.** Examples: `KT9.KQ2.J83.9865` and `AK3.T42.QJ853.T2`
over `(1♦) 1♠ -`, and `QJ54.Q92.QJ74.J8` over `(1♣) 1♥ -`. The v6 floor
passes these, and both references' ladders raise. Its price per 1 000
boards is 0.85 bd, −3.4 PD (−4.0/bd) vs BEN, and 0.14 bd, −0.29 PD
(−2.1/bd) vs BBA. That is a ceiling of ≈ +0.003 IMPs/board vs BEN and
≈ +0.0003 vs BBA, below what a Tier-F A/B resolves. Next in size, and
BEN-only: preemptive `3y` on 4+ support with 0–5 HCP (−1.65 PD) and
`3y` over a two-level overcall on 3 trumps and 8–10 HCP (−1.55 PD). Both
are PD-positive or flat vs BBA. The rest are cells under 1 PD each with
mixed signs.

Verdict: no authorable or railable lever. The advance-lane gap vs BEN is
BEN's judgment spread thin, consistent with the ≈ 0.8–1.1 IMPs/board that
[ben-gap-campaign.md](ben-gap-campaign.md) attributes to its search. The
one coherent cell, the passed 3-card raise, joins the owed retrain
targets as a labelled example rather than a rail. A rail would be
re-arbitrated at the next retrain anyway (junk-action rail series).
