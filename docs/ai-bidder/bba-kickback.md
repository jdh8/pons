# BBA's Kickback: the extracted trigger rule set

**Question answered here.** With a fit in (say) diamonds, when is the keycard
ask 4♥, when is it 4♠, and when does it stay 4NT? BBA's answer, read from the
decompiled engine and validated against the shipped `.so`: **the ask for trump
T is always the cheapest bid above 4-of-T (♣→4♦, ♦→4♥, ♥→4♠, ♠→4NT) when the
kickback conditions hold; when they fail, 4NT remains ordinary RKCB. It is
never the second step** — 4♠ is never a diamond ask; with hearts agreed, 4♠
*is* the ask (and buys the entire 5-level for the queen/king machinery — the
♠K grand-slam case).

Sources: `EPBot64.dll` decompiled with ilspycmd 9.1.0.7988 (plain VB.NET, no
obfuscation; ~6.5 s — recipe in [bba-floor.md](bba-floor.md) §5.5); methods
cited by name below, all in type `EPBot`. Validation: 39 constructed clause
probes (`examples/probe-bba-kickback.rs`) and a predictive census
(`examples/probe-bba-kickback-census.rs`) against
`vendor/bba/Native-libraries/linux/x64/libEPBot.so`.

## 0. The convention toggles

Three mutually exclusive rows (setting one clears the others), convention ids
80/81/82 (`nazwa_konwencji`, `ModuleCommon`):

```text
Kickback 0123  |  Kickback 0314  |  Kickback 1430
```

Independent of the `Blackwood 0123/0314/1430` rows (ids 40–42): kickback
governs the relocated asks, Blackwood the residual 4NT. Our cards pin all
three kickback rows to 0 ([card.rs](../../src/bidding/card.rs) `SCHEMA`;
"RKCB 1430 into the agreed suit; no Kickback, no Crosswood, no Exclusion").

**FFI trap (new finding).** `epbot_set_conventions(bot, site, name, on)`
takes a *side* (0/1), not a seat: the engine holds `cc = new TYP_SYSTEM[2]`.
Sites 2/3 throw (return −2, swallowed everywhere). `examples/common/oracle.rs`
passes raw seats `[actor, (actor+2)%4]` — functionally correct only because
the out-of-range half of each pair silently no-ops onto the same side.

**Meaning ABI (new finding).** `epbot_get_info_meaning(bot, k, buf, bytes)`
indexes per-seat interpretation records (`Item[k]`); slot `k` holds seat k's
*latest* call's systemic label, refreshed by `set_bid` itself. Read it right
after each call to harvest per-call labels at any depth (the first-round-only
caveat recorded for the deleted `bba-wj-reference` harvest came from calling
`epbot_interpret_bid(bot, x)` with a call *index* — its argument is actually a
bid *code*; don't call it at all). A dealt hand is not required for the
interpretation, only for `get_bid`.

## 1. The interpreter (when does a 4-level bid READ as the ask?)

`interpretuj_blackwooda` tries **Crosswood → Gerber → Kickback → Blackwood**,
first match wins; every interpreted ask is barred from natural use
(`odzywka_zabroniona`). The Kickback arm fires for a 4-level bid in strain
T+1 (`get_kolor_kickback`, plus the retro-agreement fallback in
`get_kolor_domniemany`) when ALL of:

1. **Natural-ambiguity guard**: *neither hand has shown 4+ cards in the
   kickback suit* (`partner.min_dlugosc[T+1] < 4 & CURRENT.min_dlugosc[T+1] < 4`).
   This is BBA's whole answer to "is 4♥ hearts or the ask?": after
   1♦–1♥–3♦, responder's 4♥ is **natural** (the 1♥ showed 4), and the ask
   reverts to 4NT.
2. **Opponents'-suit gate**: the kickback suit is not the opponents' shown
   suit — *unless* trumps are formally agreed (an explicit raise), or
   partner's last call wasn't a pass and (T isn't clubs or a major was
   shown). So over (1♥), 1♦–(1♥)–3♦–…–4♥ **stays the ask**: the raise made
   the agreement formal.
3. **Fit evidence**, any of:
   - the side has *shown* 8+ trumps (`zgloszone_karty ≥ 8`);
   - the side has shown trump length AND (the ask is a jump, or the
     opponents showed length in the kickback suit, or the auction sits at
     3NT);
   - shown minimums sum to exactly 7 with partner ≥ 2 and the bidder's
     length range still open (`min == 7 − partner.min`, `max > min`).
4. **Retro-agreement** (no agreed suit at all): a bare 4♦/4♥/4♠ retro-agrees
   the suit one below when someone showed 4+ there and 8 combined cards are
   arithmetically possible (`partner.min + CURRENT.max ≥ 8`).
5. **HCP gates** (outer, shared with all ace asks): combined strength ≥ 25ish
   (`max_HCP_bidder + min_HCP_partner ≥ 25`, with ≥20 / total-points
   variants), opponents' shown HCP ≤ 18 for Kickback (plain Blackwood
   tolerates ≤ 20; ≤ 22 when they have no suit), opener ≥ 10 HCP if asking.
6. **None of the suppressions**:
   - partner is mid-cue (`feature[144]`), or a cue exchange is live and the
     bid sits below game of the agreed suit — narrow in practice: a partner
     call the engine reads as "calculated" is no cue, and the below-game arm
     can never touch the ♥-ask (4♠ is above 4♥);
   - forcing/semi-forcing 1NT contexts;
   - splinter collisions: the bid matches a responder splinter (2nd call),
     opener splinter (4th), or responder second-round splinter (6th);
   - hearts trumps and 4♠ was the only cue bid available
     (`first_cue == last_cue == 4♠`);
   - spades trumps and the auction is quantitative.
7. 4-level only (`num2 == 4`); an 8-card fit must be arithmetically possible
   (`CURRENT.max + partner.min ≥ 8`); NT agreement excluded. There is no 4♣
   kickback — 4♣ stays Gerber where Gerber applies.

**What 4NT means with a minor agreed and kickback ON**: the Kickback arm gets
`kolor == −1` for 4NT over ♣/♦/♥ trumps, falls through to the Blackwood arm —
ordinary RKCB into the agreed suit (or quantitative where
`quantitative_situation` holds; with ♠ agreed, 4NT itself carries the
`"Kickback 1430, for !S"` label but is functionally the same RKCB).
Kickback ON therefore *adds* asks; it never removes 4NT.

**Exclusion/Crosswood/Gerber precedence**: both Crosswood and Gerber outrank
Kickback in the dispatch; Exclusion (`determine_EXCLUSION`, runs after)
cannot steal a 4-level kickback ask for the same trump suit — only 5-level
jumps can be Exclusion.

## 2. The asker (when does the engine CHOOSE the ask?)

`determine_pytanie_o_asy` scans candidate bids upward from
`max(current+1, 4♣)` and takes the **first** whose interpretation returns the
trump under consideration — so the kickback bid, being cheaper, outranks 4NT
whenever the interpreter would read it (fallback: `4NT` + plain Blackwood).
Whether to ask at all is priced by the bilans: `losing_tricks[T] ≤ 2`, an
8-card fit or `total_points ≥ 34`, and `probable_level ≥ 6` (or the
HCP/missing-keycard ladder). In practice BBA cue-bids en route below game
first (see the asker-side probes): the ask tends to surface after a cue
exchange, not instead of one.

## 3. Answers and continuations

All shared with Blackwood (`odpowiedz_na_asy`): keycards = 4 aces + trump
king, counted for the suit stamped on the ask (`feature[424]`), answers in
**steps above the ask bid**:

| scheme | step 1 | step 2 | step 3 | step 4 |
|---|---|---|---|---|
| 1430 | 1 or 4 | 0 or 3 | 2, no Q | 2 + Q |
| 0314 | 0 or 3 | 1 or 4 | 2, no Q | 2 + Q |
| 0123 | 0 or 4 | 1 or 5 | 2 | 3 |

- the trump queen is *held* when the honor table says so **or the side owns
  10+ trumps** (`posiadane_karty ≥ 10`);
- disclosure strings: `"A=1/5 or 4/5"` etc. (denominator 4 for NT asks);
- **competition collapse**: over interference at or above 5-of-trumps the
  ladder becomes parity — `steps = keycards % 2`, disclosed
  `"0/5 or 2/5"` / `"1/5 or 3/5"` (probe: X = "1/5 or 3/5" over their 5♥);
  under a plain X of the ask the step ladder starts at XX (probe: XX
  carries step 1's `A=1/5 or 4/5`, ROPI-flavored);
- **queen ask** = 1st available non-trump non-NT bid above the answer
  (`get_potencjalne_pytanie_o_dame_krole`); the "2nd available bid" king ask
  exists in code but is gated on `King ask by available bid` — **the
  system-0 default (and the 21GF card) has `King ask by 5NT = 1`, so the
  king ask stays 5NT** and e.g. 5♦ after a 4NT answer is *natural*
  (probe-verified); Gerber forces 5♣.

  **`King ask by available bid` is inert under kickback** — probed, because it
  looks like the natural default once the ask is relocated.  It is not.  Both
  rows crossed on one grand-zone hand, hearts agreed (ask 4♠, answer 4NT, queen
  ask 5♣, reply 5♠ = queen + ♠K):

  | `King ask by 5NT` | `King ask by available bid` | asker |
  | --- | --- | --- |
  | on | off | **5NT** (asks) |
  | off | on | 6♥ (places) |
  | on | on | **5NT** (asks) |
  | off | off | 6♥ (places) |

  The available-bid arm equals the neither arm, so the row never fires; 5NT
  wins when both are set.  Repeated with diamonds agreed, where the 2nd
  available bid would be 5♠ — three steps under six of trump, the most room the
  row can ever have — and it still did not ask (it bid 6NT).  Whatever gates it
  is not satisfied by an ordinary kickback + queen-ask auction.  Note the 6NT:
  BBA prefers the notrump slam to six of a minor once the values are known,
  which is the same "pull to pick a slam" jdh8 arrived at independently.

  Consequence for our own card: we disclose `King ask by available bid` only
  when the relay **and** kickback are both on.  Over a plain 4NT ask our king
  ask is the step above the queen reply, which *is* 5NT in most of those lanes,
  so claiming the available-bid form there describes a call we never make;
- the trump queen is "held" when the honor table says so **or
  `posiadane_karty[trump] ≥ 10`, computed as own actual length + partner's
  bilans-*reconstructed* length** (`MY_HAND.dlugosc + TMP_HAND.dlugosc`) —
  probe-verified boundary: a 4-card raise opposite a probable 6 answers
  "queen yes" on J-fourth; a 3-card raise does not;
- queen answers: signoff in trump without it; with it, cheapest side-suit
  king below 6-of-trump (skipped steps *deny* that king), else an NT bid
  ("queen yes, no side king" — probe: 5NT);
- king answers after 5NT: a **count** ladder (`"K=n"`), enumerated against the
  engine with hearts agreed and only the side kings moved —

  | side kings | 0 | 1 | 2 | 3 |
  | --- | --- | --- | --- | --- |
  | call | 6♣ | 6♦ | **6♥** | **6♠** |

  so it is four strict steps above 5NT, and it **overshoots its own trump suit**
  to keep three kings distinct from two (6♠ with hearts agreed).  Our
  `king_answers` collapses 2-and-3 into a 6♥ catch-all for hearts and cannot
  tell them apart — the distinction the grand actually needs, given
  `probe-trump-queen` puts two side kings at 80.1% against a 56–58% break-even
  while our classic path still demands three.  Per-king answers instead under
  `5NT inviting` (off);
- the alert label rides `ustaw_konwencje`: **`"Kickback 1430, for !D"`**,
  same shape as every ask family.

The ♠K-grand payoff with hearts agreed: ask 4♠, answer 4NT (1/4), queen ask
5♣ with answers below 5♥ that *show or deny specific side kings*, and even
the default 5NT king ask answers at 6♦ = "one king" — still below 6♥. Plain
4NT RKCB spends 4NT+5-level on keycards alone before the 5NT king ask.

## 4. Probe evidence (examples/probe-bba-kickback.rs)

39 constructed cases, each replayed with the `Kickback 1430` row off and on:
**35 pinned expectations pass / 0 fail**, 43 exploratory observations, all
against the shipped `.so`. The load-bearing rows:

- **Ladder + labels**: fed asks read `"Kickback 1430, for !C/!D/!H"` after
  1♣–3♣ / 1♦–3♦ / 1♥–3♥; answers hit the exact 1430 steps with disclosure
  strings `A=1/5 or 4/5`, `A=0/5 or 3/5`, `A=2/5 or 5/5, Q(♦)=0/1`; the
  0314/0123 rows relabel and re-step accordingly (`A=1/5 or 5/5` for 0123).
- **Flag-bites control**: same auction with the row off reads 4♥ as a
  *Splinter* and partner bids 5♣ ("surplus") — the phantom-suit failure mode
  in one row.
- **The asker's choice moves**: after 1♦–3♦–4♣(cue)–4♦, the same monster
  bids **4♥ ("Kickback 1430, for !D") with the row on and 4NT
  ("Blackwood 0314, for !D") with it off**; the club freak over 1♣–3♣ bids
  4♦ vs 4NT. (Also visible: the engine's system-0 default Blackwood is
  **0314**, another engine-default ≠ card case.)
- **Guards**: 1♦–1♥–3♦–4♥, 1♥–2♦–3♦–4♥, 1♥–1♠–3♥–4♠, 1♣–1♦–3♣–4♦ all read
  natural ("bidable suit" / floor) — the shown-4+ guard holds on both sides.
- **4NT residual**: with a minor agreed and kickback on, 4NT reads
  `"Blackwood 0314, for !D/!C"` and is answered on the Blackwood ladder.
- **Cue chains do not kill the ask** when partner's intervening call wasn't a
  recognized cue: 1♦–3♦–4♣–4♦("calculated")–4♥ still reads
  `"Kickback 1430, for !D"` (the suppression needs the engine's own cue flag,
  and its second arm only bites below game of the agreed suit — which
  exempts the ♥-ask 4♠ entirely).
- **Competition**: over (1♥), the formally raised ♦ fit keeps 4♥ as the ask —
  the cue of their suit *is* the keycard ask. Their X of the ask → **XX
  carries the step-1 meaning** (`A=1/5 or 4/5`, ROPI-flavored); their 5♥ over
  the ask → **X = parity** (`1/5 or 3/5`), exactly the collapsed ladder.
- **Jacoby route**: 1♥–2NT–4♠ reads as the ♥ ask (no splinter collision).
- **Wart** (asker/interpreter desync, one case): the club freak's chosen 4♦
  self-labels "Splinter" on the asker's own bot even though the answerer
  reads it as `"Kickback 1430, for !C"` and the auction proceeds correctly
  (C1). Cosmetic in self-play; worth remembering when reading asker-side
  meaning slots.

## 5. Predictive census (examples/probe-bba-kickback-census.rs)

Self-play with `Kickback 1430 = 1` both sides over deals filtered to
"some side holds 24+ combined HCP and an 8-card fit"; every 4♦/4♥/4♠ call
compared against a predictor implementing §1's guard + fit/retro clauses on
the engine's own shown-length state (`info_min/max_length`, plus the
`feature[144]` cue flag).

5000 boards, seed 42: 58 543 calls, **3081 candidate 4♦/4♥/4♠ calls, 224
labeled kickback** (♣ 18 / ♦ 48 / ♥ 158 — the ♥-ask dominates in practice),
plus 214 4NT calls carrying the cosmetic `"Kickback 1430, for !S"` label.
**Predictor agreement 3028/3081 (98.3%) with zero false negatives**: the
engine never fired kickback outside the predicted set, so §1's guard +
fit/retro clauses are a *complete upper bound*. All 53 misses are false
positives where an earlier interpretation arm wins the dispatch before the
Blackwood family is consulted:

| class | ≈count | example label |
|---|---|---|
| cue-bid continuation readings | 25 | `Cue bid, a !D stopper` |
| natural slam-try / self-sufficient jumps | 19 | `slam try`, `bidable suit` |
| competition fit-jumps | 5 | `limit raise or better in !S` |
| misc | 4 | `Unknown bid` |

4NT residuals across the census: 518 calls — 179 `Blackwood`, the rest
answer/queen/king strings, `Quantitative 4NT`, and the ♠-relabeling.
Modeling the remaining 1.7% would mean reimplementing BBA's whole
interpretation dispatch; the classes above are the documented boundary.

## 6. Divergence from human treatments

- **Rubens' Kickback / useful-space principle**: the ladder itself is the
  textbook one. Human partnerships resolve the natural-ambiguity by
  *agreement inventory* (e.g. "kickback applies whenever trumps are agreed;
  the bid is never natural"); BBA instead resolves it *dynamically* by the
  shown-4+ guard — closer to the "cheapest non-natural call asks" school.
  The cost is state: both hands must track shown lengths to know which call
  asks; the benefit is no natural 4♥/4♠ is ever lost.
- **Redwood/Minorwood**: minor-only relocations. BBA's ladder covers hearts
  too (and labels ♠-agreed 4NT "Kickback" cosmetically) — hearts kickback
  at 4♠ is what buys the grand-slam space.
- **DOPI/ROPI/DEPO**: BBA's competition answers collapse to parity (odd/even)
  over 5-of-trump interference, coarser than our shipped DOPI/ROPI/DEPO
  machinery over 4NT (commit 8ba8844).
- **During cue exchanges the ask is suppressed** (the bid stays a cue) —
  many human pairs instead treat the kickback bid as *always* the ask once
  trumps are set, cues be damned. BBA's choice avoids stealing the cheapest
  cue but means the ask's identity depends on whether a cue sequence started.
- **Retro-agreement** is a BBA-ism: a bare 4♥ over a 5+♦-shower retro-agrees
  diamonds. Human kickback needs explicit agreement first.

## 7. The pons rule: jdh8's walk-up ladder

**We differ from BBA on §1.1.** BBA's natural-ambiguity guard *gives up* the
relocation the moment four-of-(T+1) is guarded: after 1♦–1♥–3♦, 4♥ is hearts,
so the ask reverts to 4NT. jdh8's rule instead **keeps walking up** to the
first unguarded suit — 4♠ — and only falls to 4NT when nothing below it is
free. Never worse than BBA, sometimes a step better, and the ask is never
lost.

### 7.1 The ladder

Face-only, exactly like the floor's `face_trump` — no hand, no readings, so
both members provably build the same table (the same guarantee that makes a
4NT ask answerable at all). Three notions, read off the auction below the ask:

- **guarded** — a suit *either* member of our side named naturally, or the
  opponents named at all. A guarded suit keeps its natural meaning at the four
  level; their suit there is a cue. **Hearts is guarded by a spade bid too**
  (2026-08-01), unless the face disproves five of them — see "the undisprovable
  major" below.
- **set** — a suit our side named **twice**: both members (a formal raise), or
  one member twice (1♦–1♥–**3♦**). One bid is no agreement, or `1♦ P 4♥` would
  ask.
- the **`face_trump` veto** — when the face names no trump at all (the notrump
  dichotomy: `1♦ P 3♦ P 3NT P` is a sign-off, so that 4NT is quantitative),
  nothing relocates.

Each set suit, **in ascending rank**, then claims the cheapest *unclaimed
unguarded* suit strictly above it. Whatever goes unclaimed still asks at 4NT.

```text
1♦ P 1♥ P 3♦ P     set {♦}    guarded {♦,♥}   → 4♠ = RKCB(♦)                  †
1♦ P 1♠ P 2♦ P     set {♦}    guarded {♦,♥,♠} → 4NT only (♠ cannot deny ♥)
1♠ P 2♦ P 3♦ P     set {♦}    guarded {♦,♠}   → 4♥ = RKCB(♦)  (5♠+4♦ = 9)
1♥ P 2♦ P 3♦ P 3♥ P set {♦,♥} guarded {♦,♥}   → 4♠ = RKCB(♦), 4NT = RKCB(♥)
1♣ P 2♣ P 2♥ P 3♥ P set {♣,♥} guarded {♣,♥}   → 4♦ = RKCB(♣), 4♠ = RKCB(♥)    †
1♥ (3♦) 4♦ P 4♥ P  set {♥}    guarded {♥,♦}   → 4♠ = RKCB(♥); 4♦ stays a cue  †
1♠ P 3♠ P          set {♠}    guarded {♠}     → 4NT only (nothing above 4♠)
1♦ P               set {}                     → no relocation
1♦ P 3♦ P 3NT P    face veto                  → no relocation
```

**The undisprovable major (2026-08-01, jdh8).** The phase-5 wash's whole
residual loss class was the ladder claiming a call it cannot own: with ♦ set and
♠ bid, 4♥ *is* the ask, so a hand that belongs in the heart game bids 4♥
naturally and both seats' readings then sign off in 5♦ (§7.3.2, board [96], off
a literal void). The doctrine that fixes it is longest-first with ties to the
higher rank: **a spade bid never denies hearts**, because 5-5 majors bid spades,
so a later 4♥ stays plausibly natural and the claim must yield. The escape is
arithmetic: a spade bidder who named a **second** suit has shown 5+4 = 9 cards
and can hold at most four hearts, so `1♠ P 2♦ P 3♦` (opener bid ♠ *and* raised
♦) keeps its relocation. The test is therefore "some member named ♠ and named
no other suit" — still face-only, still reading-free, so both members derive it
identically. No converse: 1♥ *does* deny five spades under the same doctrine.

The price is named and real: on `1♦ P 1♠ P 2♦ P` — a common face — ♥ and ♠ are
both guarded, so the ♦ ask reverts to plain 4NT rather than moving. Redwood is
lost there, not relocated.

**Additive.** 4NT keeps its existing meaning throughout: kickback *adds* asks,
it never removes one (BBA's own posture, §1's "Kickback ON therefore *adds*
asks"). That is what holds the blast radius at zero — no auction pons already
bids changes meaning.

**The price, named — and it is currently zero.** The relocated ask consumes
the cheapest unbid-suit cue: 4♠ over agreed hearts stops being a control bid.
But the floor authors no control-bid *emission* at all (`partner_control_bid`
only responds to partner's), and neither does the book, so today that price is
not paid. It becomes real in the deferred control-bid session (§7.4).

**Open — belongs to the control-bid session.** The rows marked † leave a 4NT
that is redundant (rows 1 and 4) or entirely free (row 3). Additive keeps it
as RKCB *for now*, a scheduling choice and not a verdict; the real question —
quantitative, last train, or a control bid — cannot be answered before pons
has an authored control-bid structure to answer it against, and answering it
moves `face_trump`'s notrump dichotomy with it. Do not settle it inside the
kickback A/B: an arm that changes both the ask *and* the meaning of 4NT cannot
attribute its own result.

### 7.2 Phase ledger

**Phases 2 and 3 were merged, and the order inverted, on 2026-08-01.** The
original plan built hearts first (♥ agreed, ♠ unguarded → 4♠) and deferred
minors. Four findings killed that order:

1. **Hearts-only is not a convention.** Minors-only kickback is
   [**Redwood**](http://www.keycardask.com/redwood.html) — 4♦ asks in ♣, 4♥ in
   ♦, responses in steps above the ask. Kickback is the superset that adds
   ♥→4♠. "Kickback hearts but not minors" is not an agreement anyone plays.
2. **The payoff is a gradient, and hearts is its shallow end.** Count the 1430
   answers that destroy the five-level landing spot in the agreed suit under a
   plain 4NT ask:

   | trump | answers that overshoot 5-of-trump | relocated ask | overshoots |
   |---|---|---|---|
   | ♣ | 5♦, 5♥, 5♠ — **3 of 4** | 4♦ → 4♥/4♠/4NT/5♣ | **0** |
   | ♦ | 5♥, 5♠ — **2 of 4** | 4♥ → 4♠/4NT/5♣/5♦ | **0** |
   | ♥ | 5♠ — **1 of 4** | 4♠ → 4NT/5♣/5♦/5♥ | **0** |
   | ♠ | none — **0 of 4** | (nothing relocates) | — |

   The old phase 2 was scoped to the 1-of-4 row.
3. **The hearts row has no headroom.** [CHANGELOG.md](../../CHANGELOG.md)'s
   M6.4 entry: the shipped majors ask *"ended a clean wash … final round 4
   fired \_\_\_ / 204.8k, delta exactly zero, plain and perfect-defense
   alike."* A hearts-only A/B relocates a call that already measures zero.
   (The fired count is literally missing from that line — an open gap.)
4. **Relocation alone may not rescue minors.** `keycard_trump`'s majors-only
   carve is a measured decision whose stated cause is *strain, not space*:
   "minor and thin 6-2 asks lost to the milestone 6NT power-blast
   (double-dummy monetizes honors at 33-plus)." Kickback does not fix a strain
   verdict — which is exactly why the experiment needs three arms.

| phase | scope | state |
|---|---|---|
| 1 | the ladder as a face-only resolver + unit tests | **done** (`f985e23`) — `kickback_ladder` beside `face_trump`. No bidding change. |
| 2+3 | the floor, **all four suits**, and the carve lifted beside it | **measured 2026-08-01, 3×1M boards.** `set_keycard_minors` **WINS big** and ships; `set_kickback` measures negative but the measurement is *contaminated* — see §7.3. |
| 4 | the authored book in `slam.rs`; competition + disclosure | not started |
| 5 | face-conditional alerts, so the relocation can be priced at all | **done 2026-08-01** — `Rules::face` gate, consulted by `Rule::eval` (−∞) and the three inference consult sites; see §7.3.1's resolution note. Re-measured clean: a **wash** (§7.3.2), knob stays opt-in. |
| 6 | the undisprovable major: the ladder yields the 4♥ claim when a spade bid cannot deny hearts (§7.1) | **done 2026-08-01** — measured §7.3.4: the wash **shrinks but survives** (PD −0.00016, divergence 246 → 216). Shipped inside the opt-in knob as a soundness repair; `set_kickback` stays opt-in. |

### 7.3 The three arms

Two independent knobs, both default **off**, so the coupled change stays
attributable:

| arm | `set_keycard_minors` | `set_kickback` | what it is |
|---|---|---|---|
| **A** `plain` | off | off | today — byte-identical control |
| **B** `minors` | on | off | minor asks at plain 4NT — round 4's losing arm, re-priced |
| **C** `kickback` | on | on | full Kickback: 4♦/4♥ Redwood, 4♠ over hearts |

- **C − A** is the ship decision, and runs first (aggregate before mechanism).
- **C − B** prices the relocation on its own.
- **B − A** re-prices the carve. If B ≈ A the 2026-07-02 verdict has expired;
  if B ≪ A the strain problem is still live and a 6NT exit off the low Redwood
  answers is the named next lever.

(minors off, kickback on) is the abandoned hearts-only slice, reachable as an
arm D if C − B ever needs hearts isolated.

**The result — 3 × 1,000,000 boards, seed 1785546026 shared across arms, vul
none, arms sequential.** IMPs per board, 95% CI in brackets:

| pair | PD | plain DD | sd-declarer | sd + PD | divergent |
|---|---|---|---|---|---|
| **C − A** kickback vs plain | +0.00166 [+0.00069, +0.00262] | +0.00146 [+0.00055, +0.00236] | +0.00121 | +0.00150 | 2554 (0.26%) |
| **B − A** minors vs plain | **+0.00394** [+0.00316, +0.00472] | **+0.00375** [+0.00304, +0.00446] | +0.00253 | +0.00279 | 1840 (0.18%) |
| **C − B** relocation alone | −0.00222 [−0.00284, −0.00159] | −0.00223 [−0.00284, −0.00162] | −0.00175 | −0.00160 | 864 (0.09%) |

Every cell's CI excludes zero. The arms are internally consistent:
(B−A) + (C−B) = +0.00172 against a measured C−A of +0.00166.

**The carve lift ships.** Lifting `keycard_trump`'s majors-only carve wins on
plain DD *and* PD *and* both sd rows — not the PD-only shape that flags a
doubling artifact. The audited boards are the mechanism exactly: `4NT P 5x P 6m`
finding the minor slam (`1♣ P 1♠ P 2♣ P 4NT P 5♣ P 6♣`, +15) and — worth as
much — the ask *declining* one, `5♦ vs 6♦` at **+44 over 4 boards** where arm A
blasted six without the keycards. **The 2026-07-02 round-4 verdict has
expired**: it concluded minors lost to the milestone 6NT power-blast, and on the
2026-08 system the ask beats the blast by 2.04 IMPs per divergent board.

#### 7.3.1 Why C − B does *not* price the relocation

The −0.0022 is a build artifact, not Redwood's price. The 60 audited C − B
boards are dominated by one shape that has nothing to do with keycards:

| A vs B | n | PD |
|---|---|---|
| 4♠ vs 6♠ | 12 | −64 |
| 6♠ vs 4♠ | 8 | −44 |
| 4♥ vs 6♥ | 7 | +13 |
| 6♥ vs 4♥ | 7 | −10 |

```text
1♦ P 1♠ P 2♦ P 4♠ P P P        → kickback plays 4♠, arm B reaches 6♠
```

No suit is bid twice by one side there, so `kickback_ladder` returns all-`None`
and no relocated ask is even reachable. The 4♠ is a natural game — and partner
passes it anyway.

Cause: with the knob on, `KICKBACK_ANSWERS` and the three ask targets install
`.alert(RKCB_FLOOR)` rules on **4♦, 4♥ and 4♠**, whose constraints are `pred`
closures that project to ⊤. `inference.rs` unions the projections of *every*
rule sharing a call, so the natural-4♠ box is unioned with a box promising no
spades and partner's `length(Spades).min` collapses to 0; the structural
`alerted` bit then suppresses the natural walk's lane bookkeeping on top.
Gating rule presence at build time (§7.4) protects arm A and only arm A. **Arm
C carries the poison, and it lands on the two most common contracts in bridge.**

Splitting those 60 boards by whether a **4♥ or 4♠ contract is involved on
either side** sizes it: 38 boards carry **−158 PD / −155 DD**, and the other 22
carry **+7 / +10**. The poison class is *105% of the loss*, and auctions that
never touch a four-major contract sit mildly positive — which is what Redwood is
supposed to look like. (60 of 864 divergent boards, and the split is a proxy: a
genuine relocated-ask auction landing in 4♥ counts in the poison bucket too. A
strong hint, not a measurement.)

There is no cheap patch. `Rule::alert` is a static `Option<Alert>` and the
`alerted` test never evaluates the constraint, so *any* alerted rule on 4♥/4♠
does this — dropping the answer rules would not help, because the ask rules
alone suffice. Pricing the relocation needs a **face-conditional alert**: the
auction is shared, so `keycard_ask_bid(auction, n−2).is_some()` is a legitimate
disclosure predicate, and disclosure that depends only on the face is still
disclosure. That is an inference-layer change (phase 5), not an instinct one,
and it is the gate on every future Kickback number.

**Resolution (2026-08-01, phase 5 built).** `Rule` carries an optional
**face gate** (`Rules::face`, a `Fn(&Context) -> bool` over the auction alone).
The gate is the single source of liveness for both halves: `Rule::eval`
returns −∞ when it fails (the bidder cannot fire a face-dead rule), and the
three inference consult sites — the projection union, the `alerted` bit, and
the announce filter — skip face-dead rules under the bidder's at-the-time
context, so exclusion is sound by construction. The kickback arm attaches the
recognizers' face halves (`keycard_asked_face`, `keycard_asked_over_bid_face`,
and the ladder claim for the three ask targets) to its ask and answer rules;
the plain arm keeps ungated constructors and stays byte-identical (verified:
20k-board smoke, `minors` vs `plain`, pre- and post-change binaries produce
identical output). On the audit shape `1♦ P 1♠ P 2♦ P 4♠` the ladder is
all-`None`, the rules are face-dead, and the natural spade floor survives —
the regression test `kickback_face_gate_keeps_natural_four_spades_natural`
pins it. The re-measure with the gates is §7.3.2: the poison class vanished
outright and the relocation prices as a **wash**. The old −0.0022 must not be
cited as evidence against Redwood — the clean number is −0.0003 with every CI
straddling zero.

#### 7.3.2 The clean price (phase 5 re-measure, 2026-08-01)

**kickback vs minors (the shipped default), 1,000,000 boards, seed 1785558240,
vul none, `--sd`.** IMPs per board, 95% CI in brackets:

| scoring | Δ/board | CI | divergent |
|---|---|---|---|
| PD | −0.00029 | [−0.00062, +0.00004] | 246 (0.025%) |
| plain DD | −0.00029 | [−0.00060, +0.00003] | 246 |
| sd-declarer | −0.00030 | [−0.00061, +0.00001] | 246 |
| sd + PD | −0.00029 | [−0.00062, +0.00004] | 246 |

The build artifact is gone, by every sign at once:

- **Divergence collapsed 864 → 246** — the phantom class was two thirds of the
  old footprint.
- **The poison signature is extinct**: zero boards pair a four-major game
  against the same-suit slam (old audit: 4♠-vs-6♠ alone was 20 boards, −108
  PD). The audit shape `1♦ P 1♠ P 2♦ P 4♠` no longer diverges at all.
- What remains is Redwood *itself*, and it nets to noise: grand-slam churn in
  both directions (6♦-vs-7♦ +66 against 7♦-vs-6♦ −47, 7♣-vs-6♣ +66 against
  6♣-vs-7♣ +55 the other way), the ask's extra room spent going six-over-five
  (6♣-vs-5♣ 21 boards −121, 6♥-vs-5♥ 20 boards −94, partly offset by
  5♥-vs-6♥ +53) — and the one structural price: **the relocated ask eats the
  natural four-major landing spot** (5♦-vs-4♥/4♠ pairs, ~45 boards ≈ −130 PD,
  both directions). With diamonds agreed, 4♥ *is* the ask, and a side that
  belonged in the heart game plays 5♦ instead.

Verdict from the decision table: a wash on plain DD **and** PD — not
shippable default-on, not refuted either. `set_kickback` stays an opt-in
knob. No vul-both run: the flip gate ("both scorings clear zero") was not
met.

**The landing-spot class, audited with hands** (the run replayed with a deal
printer — the divergence is deterministic in the seed): every board in the
class is a **bidder-side agreement collision**, not partner confusion. The
natural-walk 4♥/4♠ rules do not know the ladder has claimed the call, so a
major-freak hand bids it naturally — board 96: `1♦ P 1♠ P 2♦ P` and North,
holding ♠AKQ8754 ♥AT9642 ♦— ♣—, bids 4♥ *meaning hearts* — and both seats'
readings then follow the agreement: partner answers keycards (5♣), and the
bidder's own decode machinery, which also consults the ladder, believes an
ask happened and signs off in "the trump": **5♦ on a diamond void**, while
the baseline table passes 4♥ out and makes it. The auction is internally
consistent and wrong. The revisit lever therefore has two symmetric forms:
the ladder yielding when a four-major game is still live (face-only, cannot
see the freak), or a face gate on the *natural* four-major rules when the
ladder claims the call — the exact mirror of phase 5, and the only form that
can see the collision coming.

**One board of the class was not the ladder's fault at all.** Board 40
(♠AKT862 ♥AKJ92 opposite a weak 2♦) responded **2♥**, suppressing the longer
and higher major, which handed the walk-up a poisoned face: hearts guarded, so
the ♦-ask landed on 4♠ and ate the natural spade game. Traced to the book, not
the floor: [`weak_twos::responses`](../../src/bidding/american/weak_twos.rs)
registered every new suit at weight **1.5** with identical constraints, so the
winner was decided by `Table::next_call`'s tie-break — descending sort, first
*legal* call, i.e. the **cheapest** bid. No length comparison existed in the
node. Repaired 2026-08-01 with the advance side's `longest_unbid` partition
(promoted to `constraint.rs`), shipped **default-on** as doctrine, so it is live
in both arms of the coming re-measure; `set_weak_two_longest_first` ablates it.
The collision needs two qualifying five-card suits opposite a weak two — ~3
boards in 10⁴, hence the enriched probe rather than a random-deal A/B (see
[measurement.md](../measurement.md#enriched-probing--when-the-trigger-is-too-rare-for-random-deals)),
which prices the tie-break at **parity** (PD +0.019/accepted deal, CI
[−0.061, +0.100]): free, as a doctrine repair should be.

The probe's second accept mode found the same misbid **~10× more often** and
not tied at all. Ogust 2NT sits at weight 2.0, above every new suit at 1.5, so
the identical hand with *two* diamonds instead of one asks about diamond
quality; board 40 escaped Ogust only on its singleton. Over 2♦ the major now
outranks the ask (`set_weak_two_major_priority`, **default on**): **PD
+3.048/accepted deal, CI [+2.911, +3.184]; plain DD +1.668, CI [+1.563,
+1.773]**, ≈+0.0021 IMPs/board scaled by the 0.069% trigger density. Both
repairs are live in both arms of the coming re-measure.

#### 7.3.3 The gates reach the default system — and ship free (2026-08-01)

§7.3.1's mechanism was never kickback-specific: the plain 1430 answers
(5♣–5♠) and the ROPI/DOPI/DEPO rules on X/XX/Pass carry `.alert(RKCB_FLOOR)`
and are present in **every** stance, so the shipped default alerted every
floor-classified five-level bid, double and redouble on faces with no ask
anywhere, erasing their natural readings (regression: on
`1♦ P 1♠ P 2♦ P 5♦` partner's diamond floor read as erased, restored by the
gates). `set_keycard_answer_gates` confines those rules to their
recognizers' face windows — the same construction as phase 5, knob-guarded.

Measured `gated` vs `minors` (the then-default), 1M boards a cell, seed
1785560369, both vulnerabilities, `--sd`: **zero divergent boards in either
cell.** The reading is provably tighter and the alerting truthful, and in
self-play it never flips a single call — a perfect NULL, cheaper than SAT's
(which diverged and washed; this does not even diverge). Shipped **on by
default**: the soundness is free. The knob remains for A/B archaeology (off
recovers the pre-2026-08 reading); the plain/minors/kickback arms of
`ab-kickback` disarm it to preserve their originally-measured readings.

#### 7.3.4 The undisprovable major: the wash shrinks but survives (2026-08-01)

§7.3.2's residual class was the ladder claiming 4♥ where a spade bid cannot deny
hearts. Phase 6 makes hearts guarded on those faces (§7.1). Re-measured with the
**kickback arm now carrying `set_keycard_answer_gates`**, so the two arms differ
by exactly `set_kickback` and the verdict speaks directly to a default-on flip:
`kickback` vs `gated` (the shipped default), 1,000,000 boards, seed 1785573096,
vul none, `--sd`:

| scoring | Δ/board | CI | §7.3.2 (un-guarded) |
|---|---|---|---|
| PD | −0.00016 | [−0.00047, +0.00014] | −0.00029 |
| plain DD | −0.00019 | [−0.00049, +0.00011] | −0.00029 |
| sd-declarer | −0.00028 | [−0.00057, +0.00000] | −0.00030 |
| sd + PD | −0.00025 | [−0.00055, +0.00004] | −0.00029 |

Divergence **246 → 216**. The two runs are comparable across the gates because
§7.3.3 measured the gates a perfect NULL (zero divergent boards, both vuls), so
no separate un-guarded arm was built.

Reading: the guard removed about a third of the class it was aimed at and
roughly halved the PD loss, and every CI still straddles zero. **Verdict: a
wash.** The flip gate ("both scorings clear zero") is not met, so no vul-both
run and `set_kickback` stays opt-in — but the guard ships inside the knob
because it is a soundness repair: the ladder must not claim a call whose natural
meaning is still live.

**The residual, audited with hands** (`--show 250` replay of the same seed —
`ab-results/kickback-undisprovable-major-hands.log`; each board mapped to the
arm that *declared* it, since the feature sits N-S at table A and E-W at table
B):

| class | boards | PD |
|---|---|---|
| kickback in 5m, baseline in 4M | 24 | **+57** |
| kickback in 4M, baseline in 5m | 0 | 0 |
| slam churn (either side at 6+) | 190 | **−194** |
| other | 2 | −26 |

**The target class is not merely smaller, it has flipped sign** (§7.3.2: ~45
boards, ≈−130), and the mirror direction is *empty* — no divergent board left
shows a natural four-major colliding with the ask. Phase 6 closed the class it
was aimed at.

What is left is the **opposite** of the phase-5 story. Split the slam churn by
direction: kickback lands **lower** than the baseline on 132 boards (−469 PD)
and higher on 42 (+261). The relocation's own selling point — the answer fits
under five-of-trump, so the pair can sign off — is what loses, because
double-dummy pays for the thin slam the baseline was *forced* into:

```text
board [1]  1♣ P 1♦ P 3♣ P    kickback 4♥ ask → 4NT (0 or 3) → signs off 5♣
                             baseline 4NT ask → 5♦, so clubs cannot be played
                             at the five level → 6♣, which makes
board [3]  1♦ P 1♥ P 3♦ P    kickback 4♠ → 5♣ → 6♦ ;  baseline 4NT → 5♦ → 7♦, making
board [5]  1♥ … 3♥ P 4♥ P    kickback 4♠ → 5♣ → 5♥ ;  baseline 4NT → 5♦ → 6♥, making
```

By trump: hearts **−97** (83 boards, the 4♠ ask), clubs **−62** (75), spades
−53 (4 boards), notrump +20 (12), **diamonds +29** (42) — and the diamond
boards whose auction carried a 4♥ ask are **+10 over 30**. The suit phase 6
repaired is the profitable one; the loss lives in the majors-and-clubs rungs.

**The next lever is therefore not the ladder's claims but the asker's
continuation**: why does it settle at five when the baseline's forced six
makes? That is either a genuine under-bid in the relocated continuations or the
known double-dummy slam bias (the same cause `keycard_trump`'s majors-only carve
was measured against: DD monetizes honors at 33-plus). The sd-declarer row does
not rescue it, which argues the first. Further *exemptions* are ruled out by the
audit — they would cut into the one class that now measures positive.

### 7.4 What the build actually cost

Four things the original work list did not know. The fourth — that gating rule
presence protects the *off* arm only, and the on arm pays with the natural
reading of every 4♦/4♥/4♠ — is §7.3.1, and it is the one that mattered.

- **The alert check is structural, not evaluated.** `inference.rs` computes
  `alerted = rules.iter().any(|r| r.call() == made && r.alert().is_some())`
  without ever running the constraint. An always-present alerted rule on 4♥/4♠
  would therefore suppress the natural reading of *every* floor-classified
  4♥/4♠ even with the knob off. `set_kickback` consequently gates rule
  **presence** at `instinct()` build time — the regime `set_minor_keycard`
  already documents — *and* the recognizer at classification time. A harness
  must arm both: build one stance per arm, and set the flag per call by side.
- **A 4NT that answers a relocated ask is an answer, not a new ask.** With
  hearts agreed 4♠ asks and 4NT is its step 1; unguarded, the asker's own
  partner reads that 4NT as a fresh ask and answers *it*, and the 1.9-weighted
  answer rung outbids the 1.82 signoff. The first smoke run passed
  `1♥ P 2NT P 3NT P 4♥ P 4♠ P 4NT P 5♦` out for −15 IMPs. `keycard_ask_bid`
  carries the carve-out; a plain 4NT ask can never collide this way because all
  four of its rungs are five-level.
- **The recognizer needs the emission's `undisturbed()` bar.** A four-level
  suit bid in a contested auction is a cue or a contract; reading it as an ask
  deals exactly the phantom the alert exists to prevent. 4NT's own recognizer
  stays looser — it has no natural meaning to lose.

Two items from the old work list dropped: `tests/fixtures/alert-sites.txt` does
**not** move (it counts book tries only, never the floor's flat table), and §7.1's
named price — "4♠ over agreed hearts stops being a control bid" — **costs zero
today**, because the floor authors no control-bid emission at all
(`partner_control_bid` only *responds* to partner's) and neither does the book.
The price becomes real only in the deferred control-bid session.

### 7.5 Phase 4 — the book, competition, disclosure

**The book.** `install_rkcb` is absolute-bid across 27 sites; relocating there
means parameterizing it by an `ask: Call` and making `rkcb_answers` /
`asker_after_*` / `king_answers` step-relative — mechanical but wide. Until
then a book node shadows the floor wherever one exists, so the relocation only
reaches un-booked auctions. Note pons has **no queen ask at all** (the queen is
folded into the step-3/step-4 split), so BBA's relocated queen/king machinery
(§3) is a fresh design, not a port.

**Competition.** DOPI/ROPI/DEPO already ride the relocated ask — both step
answers are step-relative and their landing set widens with the ladder — but
the DOPI step is measured from *their* bid, not from the ask, and a lower ask
lets their interference sit at the four level. That arm is authored but
unmeasured.

**Disclosure** is a known hole: BBA's `"Kickback 1430"` row means "ask =
four-of-(T+1) under the shown-4+ guard", which our walk-up rule strictly
*contains* — flipping the row to 1 in [card.rs](../../src/bidding/card.rs)
still under-discloses 4♠-for-♦. Record the divergence in the constant arm
rather than pretending the row fits.

**Measurement**, per [measurement.md](../measurement.md) and the `measure-ab`
skill: `examples/ab-kickback`, duplicate match, **both** plain DD and perfect
defense, fresh `SEED_BASE` shared across arms, `scripts/idle-run.sh`, arms
sequential, no rebuilds in flight. Slam gains are contract-boundary effects
plain DD can see, so a PD-only win here would be a doubling artifact, not a
ship. The motivating measurables are BBA's own: minor slams vetoed below 5m
(the C3 probe — a 2+Q answer lands in 5♣ *as the contract*) and the hearts
grand via the ♠K.

### 7.6 The merged answer — one round, not two (design, not built)

The two-round relay shipped in `472e937`/`2687811` asks the queen, hears a
reply, then asks kings.  jdh8's revision merges them: **the queen ask is also
the king ask**, and partner's single reply carries both.  Below five of trump
the answerer has a full level to work with:

- **5T** — no queen, and not worth six anyway
- **6T** — no queen, but worth six (the ninth trump, a void)
- **a side suit** — queen, plus the king of that suit
- **5NT** — queen, no side king

Where the ask sits *at or above* five of trump the 5T rung is unplayable, so
the weak/strong split collapses and 6T is flatly "no queen"; everything else is
unchanged.

**This is BBA's scheme.**  §3 above, probe-verified: "queen answers: signoff in
trump without it; with it, cheapest side-suit king below 6-of-trump (skipped
steps *deny* that king), else an NT bid".  The revision is therefore a port,
not an invention — and it inherits BBA's refinement that the ladder is
*cheapest-first with denial*, so the reply reads "my cheapest king is this one,
and I hold none below it".  Only the dearer kings stay unknown, which is a
strictly better trade than "one king, count unknown".

The additions over BBA are the 6T buff rung (BBA signs off in trump and has no
way to say "no queen but bid it anyway") and the asker's pull described below.

#### Lane table

Every relay lane, mechanically enumerated (`scripts`-free; the generator lives
in the commit message of this section).  Kickback is **8 of 8**; plain 4NT is
**5 of 8**, breaking exactly where a side suit's cheapest bid climbs above six
of trump:

```text
plain 4NT C  ask 4NT  ans 5C   Q# 5D    BROKEN: Q + DK: 6D above 6C
plain 4NT C  ask 4NT  ans 5D   Q# 5H    BROKEN: Q + DK: 6D above 6C; Q + HK: 6H above 6C
plain 4NT D  ask 4NT  ans 5C   Q# 5D    5H=Q + HK  5S=Q + SK  5NT=Q, no side K  6C=Q + CK  6D=no Q
plain 4NT D  ask 4NT  ans 5D   Q# 5H    BROKEN: Q + HK: 6H above 6D
plain 4NT H  ask 4NT  ans 5C   Q# 5D    5H=no Q, bad for 6  5S=Q + SK  5NT=Q, no side K  6C=Q + CK  6D=Q + DK  6H=no Q, good for 6
plain 4NT H  ask 4NT  ans 5D   Q# 5H    5S=Q + SK  5NT=Q, no side K  6C=Q + CK  6D=Q + DK  6H=no Q
plain 4NT S  ask 4NT  ans 5C   Q# 5D    5H=Q + HK  5S=no Q, bad for 6  5NT=Q, no side K  6C=Q + CK  6D=Q + DK  6S=no Q, good for 6
plain 4NT S  ask 4NT  ans 5D   Q# 5H    5S=no Q, bad for 6  5NT=Q, no side K  6C=Q + CK  6D=Q + DK  6H=Q + HK  6S=no Q, good for 6
kickback  C  ask 4D   ans 4H   Q# 4S    5C=no Q, bad for 6  5D=Q + DK  5H=Q + HK  5S=Q + SK  5NT=Q, no side K  6C=no Q, good for 6
kickback  C  ask 4D   ans 4S   Q# 4NT   5C=no Q, bad for 6  5D=Q + DK  5H=Q + HK  5S=Q + SK  5NT=Q, no side K  6C=no Q, good for 6
kickback  D  ask 4H   ans 4S   Q# 4NT   5C=Q + CK  5D=no Q, bad for 6  5H=Q + HK  5S=Q + SK  5NT=Q, no side K  6D=no Q, good for 6
kickback  D  ask 4H   ans 4NT  Q# 5C    5D=no Q, bad for 6  5H=Q + HK  5S=Q + SK  5NT=Q, no side K  6C=Q + CK  6D=no Q, good for 6
kickback  H  ask 4S   ans 4NT  Q# 5C    5D=Q + DK  5H=no Q, bad for 6  5S=Q + SK  5NT=Q, no side K  6C=Q + CK  6H=no Q, good for 6
kickback  H  ask 4S   ans 5C   Q# 5D    5H=no Q, bad for 6  5S=Q + SK  5NT=Q, no side K  6C=Q + CK  6D=Q + DK  6H=no Q, good for 6
kickback  S  ask 4NT  ans 5C   Q# 5D    5H=Q + HK  5S=no Q, bad for 6  5NT=Q, no side K  6C=Q + CK  6D=Q + DK  6S=no Q, good for 6
kickback  S  ask 4NT  ans 5D   Q# 5H    5S=no Q, bad for 6  5NT=Q, no side K  6C=Q + CK  6D=Q + DK  6H=Q + HK  6S=no Q, good for 6
```

Coverage *rises* against the two-round ladder, which serves 11 lanes: the merged
form picks up plain ♥ after a 0-or-3 and plain ♦ after a 1-or-4 — both excluded
today because the denial rung overshoots five of trump — and loses none, since
the three broken lanes were never served.  One round shorter, wider, and more
informative.

#### What it deletes

`king_asked`, `king_answered`, `king_reply`, `king_ask_here`, `king_total`,
`relay_king_replies`, `asker_after_relay_kings`, and `set_king_zero_jump`
entirely — the zero-king jump answers a question the merged design no longer
asks, because 5NT *is* "queen, no side king" and 6T means "no queen".

#### Two consequences to build deliberately

1. **An asker holding the queen still asks.**  The ask now buys king
   information as well, so `queen_moot` — which today suppresses the relay
   whenever the queen is settled — must narrow to "settled *and* not in the
   grand zone".  Otherwise the strongest hands lose the king ladder.
2. **`Q + 2K` pulls partner's 6T to 6NT.**  Over the "no queen, worth six"
   reply, an asker holding the trump queen and two side kings has the grand
   zone's requirements in its own hand; 6NT picks the slam and leaves seven
   live.  Without this the merged reply is a barrier, and the two-round form's
   own lesson (a placement is not a barrier) has to be re-learned here.

Unmeasured.  Nothing in this section ships without its own A/B, and the arm
that prices it is `queen` against the two-round relay, not against `gated`.
