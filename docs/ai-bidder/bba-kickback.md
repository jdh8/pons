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
Sites 2/3 throw (return −2, swallowed everywhere). `examples/common/oracle/mod.rs`
passes raw seats `[actor, (actor+2)%4]` — functionally correct only because
the out-of-range half of each pair silently no-ops onto the same side.

**Confirmed by measurement 2026-08-02, and it survived a wrong correction.**
`docs/ai-bidder/configured-net.md` briefly claimed the opposite (seat + name,
−2 blamed on passing a convention index). `examples/probe-set-conv` settles it:
indices 2+ return −2 from the *getter* as well, so there are two slots, not
four. `oracle.rs` now addresses the side directly. The same probe found the
sharper point — an **unknown name returns 0** and reads back 0, so no return
code can catch a mistyped row; only a read-back can.

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

  Consequence for our own card: we disclose `King ask by available bid` as **0
  always**, and carry our king ask on `King ask by 5NT` instead — even under
  kickback, where ours *is* relocated to the second relay.  Setting an inert row
  discloses nothing; setting the row BBA acts on at least tells it a king ask
  exists.  The relocation stays undisclosed, because BBA's schema cannot express
  a king ask derived from the ladder — its own is anchored at 5NT absolutely.

  **The king ask does not relocate under kickback**, and the flags cannot make
  it.  The hearts lane cannot tell — 5NT *is* the cheapest call above a 5♠
  reply — so the question was settled with diamonds agreed (ask 4♥, answer 4♠,
  queen ask 5♣, reply 5♥ = queen + ♥K), where a relocated ask would be 5♠:

  | lane | rows on | cheapest call above the reply | BBA's king ask |
  | --- | --- | --- | --- |
  | ♥ kickback | 5NT | 5NT | 5NT (indistinguishable) |
  | ♥ kickback | both | 5NT | 5NT (indistinguishable) |
  | ♦ kickback | 5NT | **5♠** | **5NT** — skips a step |
  | ♦ kickback | both | **5♠** | **5NT** — skips a step |

  So 5NT is an absolute anchor, not a rung derived from the ladder.  That fits
  the available-bid row meaning the *2nd* available bid — a dearer ask, not a
  relocated one — and explains why turning it on never buys anything;
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

## 7. The pons ladder: BBA's rule, adopted (the walk-up retired 2026-08-02)

**We no longer differ from BBA on §1.1.** jdh8's original rule kept **walking
up** to the first unguarded suit — after 1♦–1♥–3♦ the ♦ ask sat at 4♠ — and
only fell to 4NT when nothing below it was free: never worse than BBA on paper,
sometimes a step better, and the ask never lost. It is retired, because a
relocated ask two suits above the trump is unrecognisable to anything that has
not built the same table, and one seat mistaking it for a natural bid or a cue
costs a slam while the prize for being right is one or two steps of room —
**the saving is always stormed by the misunderstanding** (jdh8, 2026-08-02).
Each set suit now claims **four of the next suit up, and nothing else**; an
occupied rung falls back to 4NT rather than walking on. The history below
(§7.2–§7.3) predates the switch and reads in walk-up terms.

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

Each set suit, **in ascending rank**, then claims **four of the next suit up,
and nothing else** — if that one call is guarded or already claimed, the suit
does not relocate and asks at 4NT (BBA's §1.1 rule; the walk-up's "cheapest
unguarded suit above" is retired, see the section head).

```text
1♦ P 1♥ P 3♦ P     set {♦}    guarded {♦,♥}   → 4NT only (♥ guarded, no walking)
1♦ P 1♠ P 2♦ P     set {♦}    guarded {♦,♥,♠} → 4NT only (♠ cannot deny ♥)
1♠ P 2♦ P 3♦ P     set {♦}    guarded {♦,♠}   → 4♥ = RKCB(♦)  (5♠+4♦ = 9)
1♥ P 2♦ P 3♦ P 3♥ P set {♦,♥} guarded {♦,♥}   → 4♠ = RKCB(♥), 4NT = RKCB(♦)
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
that is redundant (one set suit, its ask relocated) or entirely free (both
asks relocated off it). Additive keeps it
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
| 6 | the merged queen+king answer, and its collision guard | **done 2026-08-02**, default-on (§7.6). The guard that makes plain `set_kickback` measurable at all. |
| 7 | the floor's kickback twin, and the first clean cell | **measured 2026-08-02, 2×10M boards.** Plain DD flips sign across vulnerability; 95% of the divergence is below slam, so the cell prices the *retrained net*, not the relocation — see §7.8. Knob stays opt-in. |
| 8 | the **fair** cell — one net, the card as an input | **measured 2026-08-03, 2×2M boards.** The relocation alone is a **loss**: plain DD −0.0105/−0.0092, sd-declarer −0.0088/−0.0073, PD parity. Every relocated lane loses. See §7.13. |
| 9 | the revert | **done 2026-08-03** — `set_kickback` back to opt-in, default byte-identical. The case is closed until a scorer can fight DD's slam optimism the way sd-lead fights its defensive optimism; see §7.14. |
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

### 7.6 The merged answer — one round, not two (**built 2026-08-02**)

The two-round relay shipped in `472e937`/`2687811` asks the queen, hears a
reply, then asks kings.  jdh8's revision merges them: **the queen ask is also
the king ask**, and partner's single reply carries both.  Below five of trump
the answerer has a full level to work with:

- **5T** — no queen, and not worth six anyway
- **6T** — no queen, but worth six (the ninth trump, a void)
- **a side suit** — queen, plus the king of that suit
- **5NT** — queen, no side king

The ask must land **strictly below** five of trump.  The first cut of this
design allowed the ask to land *on* it, on the reasoning that the 5T rung was
unplayable there anyway and only the weak/strong split was lost.  That is
wrong, and the error is worth recording: the answerer reads the **face**, so a
5♥ ask with hearts agreed is indistinguishable from a 5♥ signoff, and partner
would raise a signoff to six.  The two lanes it appeared to buy are exactly the
two it breaks.

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

Coverage is **11 lanes**, the same set the two-round ladder served — the ask
must clear five of trump either way, and `successor(ask) <= 5T` and `ask < 5T`
are the same condition.  The two rows the table shows for plain ♥ after a 0-or-3
and plain ♦ after a 1-or-4 are *not* served: their ask lands on five of trump
(marked BROKEN in the generator's later pass).  What the merge buys is not
width but **depth and length** — a king named in the same round the queen is,
one round shorter, with "skipped steps deny" carrying strictly more than a
count would.

#### What it deletes

`relay_ladder`, `queen_buff_reply`, `king_rung` and `set_king_zero_jump`
entirely — the zero-king jump answers a question the merged design no longer
asks, because 5NT *is* "queen, no side king" and 6T means "no queen".  The
`king_*` decoders survive, repointed at the **second relay** below.

#### The second relay — where kickback pays twice

Partner's reply names its *cheapest* king, so one king in the asker's own hand
already makes the two the grand gate wants and seven is bid on the spot.  With
none, the second king is the whole question, and the asker **relays again**:

```text
ask = successor(reply)   more = successor(ask)   none = 6T      (requires more < 6T)
```

Two rungs, because the gate counts kings and does not name them; "none" is six
of trump, a contract rather than a code.  Being a *step* above the reply rather
than an absolute 5NT is the point jdh8 made and the reason this is worth
building: **relocating the keycard ask buys room twice**, once for the queen and
once again here.  Example: kickback ♦, reply 5♣ (queen + ♣K) → relay 5♦, and
5♥ says there is another king.

#### Two consequences, and what became of them

1. **An asker holding the queen still asks.**  The ask now buys king
   information as well, so `queen_moot` would have to narrow to "settled *and*
   not in the grand zone".  **Deferred, deliberately.**  The grand gate is
   `combined_points(37)` *and* the net's verdict; an asker holding the trump
   queen inside that band is rare enough that the lane cannot be measured, and
   opening it forces the asker to re-decide every "no queen" reply (which it
   would otherwise pass) on hands where the queen is in its own hand.  A
   `ponytail:` note, not a hole.
2. **`Q + 2K` pulls partner's 6T to 6NT.**  **Dead as designed.**  6T is the
   *no-queen* reply, and an asker holding the queen never relays (consequence 1),
   so the position cannot arise.  It would return only if consequence 1 is built.

**Measured 2026-08-02, and it ships default-on** (`ab-kickback`, 10M boards a
cell, seed 1785588007 — deliberately the two-round run's seed, so both
encodings meet the same baselines on identical deals).  Per divergent board:

| cell | | two-round | merged |
| --- | --- | --- | --- |
| vs `gated`, NV | PD / DD | +0.52 / +0.08 | +0.25 / **+0.13** |
| vs `gated`, vul | PD / DD | −0.15 / −0.24 | **+0.12 / +0.02** |
| vs `kickback`, NV | PD / DD | +0.67 / +0.24 | **+0.76 / +0.33** |
| vs `kickback`, vul | PD / DD | +0.03 / −0.05 | **+0.30 / +0.20** |

Merging wins 7 of the 8 matched comparisons.  The `kickback` NV cell clears
zero on **plain DD** unaided (+0.00024/board, CI [+0.00007, +0.00041]) — the
decision table's strongest verdict — and the vulnerable cell is a plain wash
with a PD win.  No cell loses on either scorer.

The gain is concentrated exactly where the mechanism predicts: **the vulnerable
cells**, which the two-round ladder lost outright on plain DD.  A round saved
is a step lower, and a step lower is worth most at 100 a trick.  The lone
regression — PD vs `gated` NV, +0.25 against +0.52 — is the lane with least
room to save.

Note the second-order result: the relay is worth about twice as much under
kickback as under plain 4NT, so it is now an argument for the relocation
itself.  Chaining the two experiments across their shared seed puts
`kickback-queen` roughly +0.00015/board plain DD ahead of `gated-queen`, where
kickback alone had re-measured a wash.  That is arithmetic across two A/Bs, not
a measured cell — §7.7's arm, when someone writes it.

### 7.7 Open follow-ups

Recorded, not built.  Each owes its own A/B.

**The collision is in the bidder, not the ladder** (probed 2026-08-02).  The
repaired ladder (§7.6's guard) still measured −0.76 PD / −0.80 DD per divergent
board, and the residue localised sharply: 354 of 2090 divergent boards land in
*different strains* in the two arms, hearts against diamonds in both directions,
carrying ~58% of the loss.  Tracing them gives one auction:

```
1♦  P  1♠  P  2♦  P  4♥  P  P  P        ← passed out, 171 boards, −551 DD IMPs
```

Diamonds is set, hearts is unguarded, so the ladder claims 4♥ for the diamond
ask — but the hand *bidding* 4♥ is 6-6 in the majors and **void in diamonds**
(♠AQJT83 ♥QT9875 ♦— ♣6).  It means hearts.  The answerer reads an ask, the
natural continuation is face-gated off, and on 171 boards nothing fires at all.

The instinct was to blame the ladder — to stop a *simple* rebid from setting the
suit, so `1♦ P 1♠ P 2♦` claims nothing.  **The probe refutes that.**  Three cases
added to `probe-bba-kickback` (`collision A/B/C`) put both auctions to EPBot with
`Kickback 1430` on:

| face | BBA's label for 4♥ | BBA's answer |
| --- | --- | --- |
| `1♦ P 1♠ P 2♦ P 4♥` | `Kickback 1430, for !D` | 5♦ (`A=2/5 or 5/5, Q(D)=1`) |
| `1♦ P 1♠ P 3♦ P 4♥` | `Kickback 1430, for !D` | 5♦ |

BBA claims 4♥ after a simple rebid exactly as we do; narrowing the ladder would
have diverged from BBA in both auctions to fix a defect that is not there.

BBA escapes the collision **on the emitter side**.  Handed the 6-6 hand over
`1♦ P 1♠ P 2♦`, EPBot bids **2♥** (`bidable suit`) — the second suit cheaply, at
the two level.  It never jumps to 4♥, so 4♥ is never natural there and no
conflict can arise.  BBA did not disambiguate the ambiguity; it structured the
auction so the ambiguity has no instance.

Our floor jumps instead, and the reason is in the artifact: `american_bba.json`
records `card: engine defaults`, i.e. a teacher with kickback **off**, and
`kickback` appears in no data-gen script.  The net has never seen a board where
4♥ meant diamond keycards; every training row taught it that call is natural or
a splinter.  No ladder rule, reader, or guard can repair that — the ladder fixes
what a bid *means*, the net chooses what gets *bid*.  The next move is therefore
a **retrained twin**, `dump-teacher --teacher bba --conv "Kickback 1430=1"`,
selected inside `classify_bba` under `kickback_now()` exactly as
`evaluator_v3_exclusion` is selected under `pass_exclusion_reading` — knob-off
byte-identical.  Residual risk to watch: BBA reverts to 4NT precisely where
jdh8's ladder *walks up* (after `1♦ P 1♥ P 3♦` we ask 4♠, BBA asks 4NT), so a
BBA-taught net will call that 4♠ a cue while our reader calls it an ask — the
same disease, in the lane the walk-up exists for.

**A system-config block in the features** (designed 2026-08-02, not built).  The
twin exists because one net cannot serve both systems, and the reason is sharper
than "it needs more data".  A single net trained on both regimes
(`dump-teacher --mix-kickback`, 866k rows alternating by board) is a **better
net by every aggregate** — val CE 0.4004 against the twin's 0.4431 and the plain
net's 0.4518 — and it *still* bids the phantom 4♥ (the
`the_six_six_hand_stops_jumping_into_the_relocated_ask` acceptance test passes
for the twin and fails for it).

The regime is not in the features **at the moment the call is chosen**.
`features_v3`'s forty `Inferences` floats describe the auction *so far*, and
`1♦ P 1♠ P 2♦` is three natural bids in either system — so both regimes present
that decision with byte-identical inputs and contradictory targets (2♥ from the
kickback teacher, 4♥ from the plain one) and the net can only average them.  The
readings *do* diverge, but one ply too late: only once a relocated ask has been
made, which is the decision we needed right.  Separate weights are how a regime
bit gets expressed without widening the pinned vector.

The principled fix is a fifth block, **prior to the auction rather than derived
from it**:

| Block | Start | Len |
| --- | --- | --- |
| Disclosable hand | 0 | 10 |
| Context | 10 | 36 |
| Inferences | 46 | 40 |
| Vulnerability | 86 | 2 |
| **System config** | **88** | *N* |

Encode it as the **`.bbsa` card rows** (jdh8): the card already *is* the
configuration, it is the shared vocabulary with the teacher, and it is
disclosable by construction — `Kickback 1430 = 1` is a row we hand BBA.  Pruning
`cards/American.bbsa`'s 258 rows drops 123 `Not defined` and 2 meta rows, leaving
**133**.  Take all 133 rather than the three that currently vary: the card
enumerates every convention BBA knows, so the block never needs widening again —
a future knob flips an existing row instead of forcing another version bump.
Constant inputs cost a bias term and one wider gemv (~34k extra first-layer
weights), nothing measurable.

Two things to design around.  **The card cannot express pons-only knobs** — there
is no queen row in `American.bbsa` at all, and only 18 of the card's rows are
driven by a live knob against 31 hardcoded constants — so the block needs a small
pons-only extension beside the card-derived part.  The queen gap is not BBA
lacking the convention: BBA relays for the queen and discloses it
(`probe-bba-kickback` asserts the label, reading back `hearts queen ask` /
`no !H queen`).  The schema carries only *toggleable* conventions, and BBA's
relay is unconditional, so there is nothing to switch and no row.  Ours is
knob-gated, and a card must describe an A/B arm, so it needs a row BBA does not.
jdh8's proposal is to carry the gaps on the 123 pruned `Not defined` rows
(South African Texas, the queen relay),
and the probe says that is safe: `Not defined = 1` **sticks** — `get_conv` reads
it back — and BBA bids the probe deal identically, so the rows are spare rather
than aliases onto something real.

The slot should then be *renamed*, not kept as `Not defined` and read
positionally.  A name EPBot does not know is a silent no-op (`South African
Texas = 1`: set does nothing, get returns 0), so an honestly-named row is exactly
as invisible to BBA as the filler it replaces — and positional meaning would not
survive anyway, because BEN's `BBA.py::load_ccs` keys a **dict** by name and
collapses all 123 `Not defined` rows into a single entry.  Comments are not an
option either: `load_ccs` unpacks `split(' = ')` into exactly two under a bare
`except` that calls `sys.exit(1)`, so a `#` line kills the process.  Shipped as
`PONS_SCHEMA` in `card.rs` (`South African Texas = 1`,
`Queen ask by available bid = queen_ask_now()`), spending one filler slot apiece
so the card holds its 258-line length.  The one hazard is a name EPBot *does*
know — it would stick and flip a real convention — so a test asserts disjointness
from `SCHEMA`.  And a bump means retraining
the **default** floor, so shipping it needs an A/B proving the default has not
regressed, which at these divergence rates is hours per cell.  Deferred until a
*second* knob needs the same treatment, or until kickback ships and wants to be
on without a knob; one net per knob is 2^n and does not scale, but n is 1 today.

**Kickback was disclosed as OFF even when it was on** (fixed 2026-08-02).
`card.rs` hardcoded `"Kickback 0123" | "Kickback 0314" | "Kickback 1430" => 0`,
so `cards/American.bbsa` told BBA we do not play it whatever `set_kickback` said.
Against BBA that is an undisclosed convention: the opponents defend a system
description in which our 4♥ is natural.  It did not touch the pons-vs-pons A/B
(no BBA in that loop), but it invalidated any kickback-vs-BBA anchor.
`"Kickback 1430"` now rides `kickback_now()`; the default stays `0` because the
knob is default-off, so the golden cards are unchanged by it.

`"King ask by available bid"` rides `queen_ask_now()` in the same change, which
*does* move the goldens (`0 → 1` in both `American.bbsa` and `Dutch.bbsa`, the
knob being default-on).  It is the honest row for the queen relay's king ask —
the step above the queen answer rather than always 5NT — and it is free, because
this row is inert in BBA: probed both ways and crossed against `King ask by 5NT`,
turning it on is byte-identical to setting no king-ask row at all.

**The relocation itself is unmeasured.**  Every kickback number on record —
including the "wash" — was taken while the answer/ask collision of §7.6 was
live, because the guard that prevents it sat entirely behind `set_queen_ask`.
Plain `set_kickback` therefore ran with no protection at all beyond the 4NT
carve-out.  Nothing measured before 2026-08-02 says anything about the
relocation on its own; it all says something about the relocation plus a
phantom-suit generator.  Re-measure before quoting any of it.  **Done —
§7.8 is that re-measure**, and §7.9 splits it per trump.

**Nine HCP-axis reading leaks, recorded not resolved.**  Shipping the relay
default-on moved `authored_calls_read_what_they_gate`'s HCP pin from 11/0 to
20/9.  The nine are the same three calls in each book column: the asker's
continuations over a 1430 answer, which **gate** on `19+ HCP` (the grand-zone
strength bar) but **read** as keycard counts and "the queen cannot change the
call".  The reading is the honest one — it says what the call shows.  The HCP
conjunct is a strength floor the reading deliberately does not project, so the
meter scores it a leak.  Closing it means either projecting the bar, which
over-narrows partner's hand at every keycard answer, or dropping it, which lets
the relay fire without the values.  Neither is obviously right; it wants its
own decision, not a silent pin bump.

**The two king asks disagree.**  The relay bids seven on **two** of the three
side kings; the classic 5NT path (`asker_after_6c`) demands all **three**.
`probe-trump-queen`'s grand table says two is right — 80.1% ± 2.5 against a
grand break-even near 56–58%, where one king is 66.2% ± 11.2 and does not
survive the slam-boundary shave.  The 5NT path is a shipped path, so it wants
its own arm rather than a quiet edit.

**All three side kings may be a 7NT signal, not a seven-of-trump signal**
(jdh8, 2026-08-02).  The argument is not "notrump takes the same tricks" — it
is that **notrump is the safer contract**, which is the opposite of the usual
intuition and is why this is worth measuring.

Holding AK in every suit, no opening lead can dislodge a stopper: we win trick
one whatever they choose, and the risk of *not having enough tricks* is
already priced out by the keycard, queen and king count.  What is left is the
risk a suit contract carries and notrump does not — a **ruff**.  Cash the ace
of a side suit into a defender who is void with a trump left and the grand
dies on a hand where every high card was ours.  7NT cannot be beaten that way.
So on the hands this trigger fires, the dominant failure mode is a ruff, not a
missing trick, and notrump removes it outright.

The gate is therefore the **fit shape**, for two reasons pointing the same way:
a 4-4 fit earns a thirteenth trick notrump cannot (the 5-3 side suit gives
discards), *and* a 4-4 fit is the shape least exposed to the ruff, because the
side suits are shorter and a defender's void is less likely.  So 4-4 keeps
playing in the suit; at 5-3 or longer both arguments flip toward notrump.

Size it before building it: over a *major* grand the gain is ten points
(1520 against 1510), which rounds to **zero IMPs**.  Over a *minor* grand it is
eighty (1520/1440 non-vulnerable, 2220/2140 vulnerable) — **2 IMPs, both
vulnerabilities**.  So this is a minor-suit treatment, which is precisely where
the relocated ladder has the room to explore a grand at all, and it should be
measured on minor lanes only.  Expect double-dummy to flatter notrump grands
(DD never misguesses a two-way finesse); score with perfect defense and apply
the slam-boundary shave before believing a thin win.

### 7.8 The retrained-twin cell (measured 2026-08-02) — the package, not the convention

The first clean measurement of the relocation *after* §7.6's collision guard
shipped default-on, and the first with the floor's kickback twin
(`american_bba_kickback`, §7.7) selected by the same knob.  Arm A is
`kickback + queen relay + twin net`; arm B is the shipped default,
`plain 4NT + queen relay + the old net`.  10M boards per cell, seed
`1785623878`, arms sequential under `scripts/idle-run.sh`.

| cell | divergent | `ns_score_pd` /board | `ns_score_cnt` (plain DD) /board |
|---|---|---|---|
| not vulnerable | 31.70% | **+0.0723** [+0.0695, +0.0750] | **+0.0062** [+0.0040, +0.0083] |
| both vulnerable | 27.97% | **+0.0438** [+0.0406, +0.0470] | **−0.0078** [−0.0104, −0.0052] |

Plain DD **changes sign across vulnerability**, both intervals excluding zero.
Off the decision table that is not shippable default-on, and `set_kickback`
stays opt-in.

**The verdict that matters is not the number, it is the divergence rate.**  A
census of the 100k sampled divergent boards per cell, bucketed by the higher of
the two arms' contract levels:

| level | share (vul / NV) | plain DD per board (vul) | (NV) |
|---|---|---|---|
| partscore 1–3 | 40.6% / 36.8% | −0.06 | −0.03 |
| game 4–5 | 54.7% / 58.2% | +0.02 | +0.05 |
| slam 6 | 4.6% / 4.8% | +0.21 | −0.09 |
| grand 7 | 0.1% / 0.1% | +0.70 | −0.04 |

**Ninety-five percent of the divergent boards never reach the six-level.**  A
relocated keycard ask cannot act below slam, so it cannot be what produced a
28–32% divergence; the twin net rewrote the whole system.  The largest movers
are `S4↔S3`, `N3↔S4`, `H3↔H4` — ordinary partscore and game judgement with no
keycard in the auction.  The 4.7% slice where kickback *can* act contributes
+970 DD IMPs vulnerable and −413 non-vulnerable over ~4700 boards each: sign
inconsistent, i.e. nothing.

So the cell measures the **package** — and the qualitative attribution is
already settled by the census without a second experiment.  The ±0.008 is the
retrained net, not the convention.  A dedicated attribution arm (a plain net
retrained on the same recipe, same seeds, kickback-blind teacher, against the
shipped old net) would put a number on the net's own contribution; it is worth
running before any future kickback cell is quoted, because until it exists
*every* twin-selecting arm carries the same confound.

**Design note for whoever runs the next one.**  The confound is structural, not
an oversight: one knob has to drive both halves, because a net distilled from a
kickback-blind teacher keeps bidding a natural 4♥ into the relocated ask (§7.7).
Kickback cannot be measured with the old net beside it.  The way out is to
subtract the net's contribution separately, not to try to hold it fixed.

**A PD/DD sign split on the undoubled majority.**  In the vulnerable cell the
79.9% of divergent boards where *neither* arm doubles score PD +0.15/board
against plain DD −0.04/board.  That is not the usual doubling artifact —
the artifact lives on boards where the arms disagree about doubling, and this
slice has none — but the two scorers disagree in sign on the largest slice,
so neither reading of the PD gain is corroborated.

### 7.9 Per-trump attribution — RETRACTED, and what replaced it

**The first version of this section was wrong, and the way it was wrong is
worth keeping.** It bucketed §7.8's divergent boards by the strain of the
**final contract** and read the buckets as kickback lanes, reporting ♣ +0.78 /
+0.56 plain DD per board against ♦ −0.38 / −0.74 and ♥ −0.26 / −0.52, with ♠ —
which kickback never relocates — as a control at ~0. It concluded that clubs
carried the whole win. None of that is supported.

Bucketing by final contract strain does not identify the lane a keycard ask was
made in. It slices *every* divergent board by where the auction happened to
land, so under a knob that also swaps the floor's weights it slices the **net's
rewrite of the whole system** by strain. The ♠ row reading ~0 was not a control
passing; ♠ is simply the strain where the net's changes happened to cancel.

**The instrument that replaced it.** `examples/ab-kickback` now buckets by the
ask itself, using `instinct::keycard_ask_at` — the trump a keycard ask was made
in and whether it was relocated — over **all** divergent boards rather than a
100k prefix. Two traps had to be cleared first:

- `keycard_ask_at` reads the knob, so the arm that produced the auction must be
  armed before the scan or a relocated ask reads as no ask at all.
- **Both arms bid at every table.** The feature sits N-S at table A and E-W at
  table B, so an auction is a conversation between the two arms; a scan that
  ignores *who* called attributes the opponents' asks to whichever arm it
  happened to arm. The census filters by the asking seat.

**The result, 200k boards, vulnerability none, seed 1785623878,
`kickback-queen` against `queen`:**

| bucket | boards | share | PD/board | plain DD/board |
|---|---|---|---|---|
| **no keycard ask by either arm** | 59240 | **93.7%** | +0.202 | −0.024 |
| a 4NT ask, some lane | 1675 | 2.7% | −2.62 | −1.48 |
| an ask the baseline made and the feature did not | 2253 | 3.6% | +2.63 | +1.37 |
| **a relocated ask — every lane, ♣ ♦ ♥ together** | **35** | **0.055%** | −0.46 | −0.66 |

**The relocated ask fires on 35 of 63,203 divergent boards, and moves −16 PD
and −23 plain-DD IMPs in a cell that moved +13,522 and −850.** Kickback is not
carrying this measurement in any lane, including clubs. The `no keycard ask`
row alone holds 89% of the cell's PD gain: what §7.8 measured is the retrained
twin, and §7.8's caution was if anything understated.

**Independently corroborated.** §7.2's phase-6 cell measured `kickback` against
`gated` — one knob, no net swap — at 1M boards and found **216 divergent
boards**. That implies the relocation changes an auction on roughly 2 boards
per 10,000, which is the same order as the 35-in-200,000 seen here by a
completely different route. Two measurements built years of reasoning apart
agree that the trigger is rare.

**What this costs the campaign.** A random-deal A/B is the wrong instrument for
a 0.02% trigger: 10M boards buys only ~1,750 relocated-ask boards, which is
~440 per lane and around ±0.6 IMPs of resolution — enough for a pooled
statement, not a per-lane one. `docs/measurement.md`'s enriched probing is the
right tool for a trigger this rare (accept on raw hands *before* the bidder),
and the minors-only arm §7.9's retracted version proposed should wait for it
rather than consume another 13 machine-hours at this density.

**The lesson, stated plainly so the next census inherits it.** A bucket keyed
on an *outcome* (the contract) cannot attribute a *cause* (the convention) when
something else in the arm moves every outcome. The fix is to key on the
mechanism — here, the ask — and to include the bucket where the mechanism never
fired, because that bucket is the size of everything the analysis cannot claim.

### 7.10 Shipped default-on (2026-08-02) — **REVERTED 2026-08-03, see §7.14**

`set_kickback` defaults to **on** at jdh8's call, and the knob remains for the
off-arm. The measured basis is §7.8: a PD win in both cells (+0.0723 NV,
+0.0438 vulnerable per board) against a plain-DD split (+0.0062 NV, **−0.0078
vulnerable**, both intervals excluding zero). That is a vulnerable plain-DD
loss, which the decision table would normally hold opt-in; it ships anyway as a
judgement call, and this paragraph is the record of the trade rather than a
claim that the table was met.

Two consequences worth stating outright:

1. **The floor's weights move with it.** `classify_bba` serves the kickback
   twin whenever the knob is on, so the default floor is now the twin — the
   package §7.8 measured, not the relocation alone.
2. **The convention card now discloses it.** `Kickback 1430` rides
   `kickback_now()`, so both golden cards move `0 → 1`. Every future
   BBA anchor therefore defends against a system description that names the
   convention, where previously the row said we played a natural 4♥.

### 7.11 The knob cull (2026-08-02)

Six RKCB flags became four deletions and one merge. The rule that decided each
one: **a knob has to name a stance a partnership could actually play.** A flag
whose off arm is a broken build is not an agreement — it is a bug with a
switch on it, and every arm it adds to a harness is an arm that can be
mispaired.

| flag | verdict | why |
| --- | --- | --- |
| `set_keycard_answer_gates` | **deleted** | "off" is §7.3.1's union poison — a natural 5♦ read as a keycard answer. Bid-inert: **0 divergent boards over 1M×2**, 3 over 200k |
| `set_queen_ask` | **deleted** | "off" is unbuildable against the only opponent we measure against. `Kickback 1430=1` is a real EPBot toggle, but nothing makes EPBot ask kings at the available bid, and its queen relay is unconditional — no retrain produces an off arm to play into |
| `set_queen_fit`, `set_queen_buff_fit` | **deleted → constants** | development tuning, settled at ten and nine. The `probe-trump-queen` evidence stands; only runtime mutability goes |
| `set_rkcb_announce` | **deleted** | announced 11+ points with an ask that fires on less — a *false* disclosure, not an inert one. Pilot measured a wash |
| `set_minor_keycard` + `set_keycard_minors` | **merged → `set_rkcb_minors`** | one agreement, two layers: two of the four stances were unplayable (a book asking on a minor over a floor that cannot answer, and the reverse) |

`set_kickback` survives the cull, and it is worth saying why when four of its
neighbours did not: its off arm is the ladder every 2/1 pair in the world
actually plays, and it is the arm the BBA anchor is measured against.

**What the harness lost.** `examples/ab-kickback` drops `gated`, `queen` and
`kickback-queen` — with the gates and the relay unconditional, all three were
byte-identical to arms that remain. Six arms collapse to three (`plain`,
`minors`, `kickback`). The mispairing that corrupted a census earlier in this
campaign is now structurally unrepresentable, which was the point.

**What it did not touch.** Nothing here moves a bid in the shipped default —
every deleted flag was already at its default value. The cull is a claim about
the *option surface*, not a bidding change, so it carries no A/B of its own;
the cells the deleted knobs measured stay in this ledger and in
`docs/bidding-options.md` §A7.

### 7.12 The census, run for real — and the bucket that turned out to be empty

§7.9 retracted the per-trump cut and named its replacement. This is that
replacement actually executed. The earlier figures quoted for it came from a
**stale binary** — the build had failed and the exit code came from `tail` in a
pipe, so the run silently used the previous binary and printed the previous
labels. Both are re-run here.

**200k boards, seed 1785623878, vul none, `kickback` vs `plain`.** Divergence
63,411 of 200,000 (31.7%); PD **+0.0705/board** [+0.0511, +0.0899], plain DD
**−0.0014/board** [−0.0169, +0.0140] — parity. Consistent with §7.8's 10M cell
(+0.0723 PD NV) at this sample size.

| bucket | boards | share | PD/bd | DD/bd |
| --- | ---: | ---: | ---: | ---: |
| no keycard ask (net alone) | 59,258 | 93.5% | +0.203 | −0.024 |
| ♥ ask only in baseline | 1,033 | 1.6% | +3.094 | +1.854 |
| ♠ 4NT (no claim) | 722 | 1.1% | −1.939 | −1.096 |
| ♥ 4NT (no claim) | 504 | 0.8% | −3.512 | −2.200 |
| ♠ ask only in baseline | 464 | 0.7% | +2.332 | +1.062 |
| ♣ ask only in baseline | 408 | 0.6% | +3.515 | +1.958 |
| ♣ 4NT (no claim) | 365 | 0.6% | −1.745 | −0.466 |
| ♦ ask only in baseline | 338 | 0.5% | +1.074 | +0.012 |
| ♦ 4NT (no claim) | 212 | 0.3% | −1.179 | −0.344 |
| ♦ relocated | 46 | 0.1% | +0.870 | +0.826 |
| ♣ relocated | 43 | 0.1% | +1.116 | +1.116 |
| ♥ relocated | 18 | 0.0% | −2.278 | −2.278 |

**The relocated ask fires on 107 boards of 63,411 divergent — 0.17%.** The
0.055% quoted in §7.9 was the stale binary's figure; the order of magnitude
survives, and so does the conclusion: kickback is near-inert at random-deal
density, and 93.5% of divergent boards saw no ask from either side.

**♠ is the control lane, by construction.** The ladder is ♣→4♦, ♦→4♥, ♥→4♠,
♠→4NT — spades has nowhere to relocate to, so `kickback_ladder[Spades]` is
never `Some` and every spade ask necessarily lands in "♠ 4NT (no claim)". There
is no "♠ relocated" row and there cannot be. Which makes its **−1.939 PD/board
over 722 boards** the sharpest statement of the confound in this campaign: a
lane where the relocation provably cannot have moved a single call still posts
a large number, because the knob swaps the floor's weights too. Any per-lane
reading here is the twin net until proven otherwise.

**The "ladder offered" bucket is empty — and the first version of it was
wrong.** The bucket was meant to size phase 4: a 4NT ask made while the ladder
was offering a relocation is a seat the convention was available for and did
not get. The first implementation asked whether the ladder claimed *any* suit,
not the ask's own trump, and duly reported 129 such boards at ≈−1.9 PD/board.
That is the §7.9 error repeated — bucketing by a label that does not identify
the lane. A spade ask belongs at 4NT, so a spade-trump 4NT beside an unrelated
club claim is not a missed relocation at all. Keyed to the ask's own trump
(`kickback_offered_at(auction, index, trump)`), the bucket **vanishes**, and the
loose rows reconcile exactly into the "(no claim)" rows: ♠ 699+23=722,
♥ 474+30=504, ♣ 316+49=365, ♦ 185+27=212.

So: **wherever the ladder claims the ask's trump, the ask relocates — 107 of
107.** The book does not override the ladder *at the ask*.

**What this does not settle.** Book shadowing has a second form the census
cannot see: the book bidding game directly and suppressing the ask altogether,
which is what `probe-kickback-lane` showed (`4♥ [AUTHORED]`, no ask anywhere).
That lands in the 93.5% "no keycard ask" bucket, indistinguishable from the
overwhelming majority where not asking is correct. **Phase 4's prize therefore
remains unmeasured**, and it needs a different instrument than a bucket: count
the positions where the ladder claims a trump *and* slam entry is reached *and*
no ask was made.

**Byte-identity, as a method.** `examples/smoke-default` dumps the shipped
default's auctions on seeded deals and takes no knobs. Built at two commits and
diffed, it answers "did this refactor move a bid?" outright. The knob cull
(§7.11) is identical over **200,000 boards** covering ~4,980 4NT asks, 6,757
slam auctions and 329 grands. Rayon is safe there *because* it takes no knobs —
the thread-locals are `const`-initialised to the shipped defaults, so a worker
starts out holding the system under test; a harness arming a non-default knob
cannot do this, which is why `ab-kickback` re-arms per call.

### 7.13 The fair cell, at last — and the relocation is a loss (2026-08-03)

Everything above §7.12 prices *a package*: `set_kickback` moved the rules and
swapped the floor's weights, so no arm could separate the two. The configured
net closes that hole by construction — one artifact, the convention card as an
*input*, so the arms differ by a card row and share every weight. The design and
its acceptance gates are [`configured-net.md`](configured-net.md); this section
records what gate 2 said about **this** convention.

`ab-kickback --feature v4-kickback --baseline v4`, 2,000,000 fresh boards per
vulnerability, seed 1785708870:

| vul | divergent | plain DD | perfect defense | sd-declarer (400k) |
| --- | ---: | ---: | ---: | ---: |
| none | 4.78% | **−0.0105 ± 0.0018** | +0.0006 ± 0.0022 | **−0.0088 ± 0.0041** |
| both | 4.15% | **−0.0092 ± 0.0021** | +0.0026 ± 0.0026 | **−0.0073 ± 0.0049** |

Three scorers: two losses, one parity, no win. And the ask-bucketed census —
§7.9's instrument, the one that survives — is sharper than the aggregate. On the
boards where a relocated ask actually fires, **every lane loses at both
vulnerabilities**: ♥ −1.09 over 391 boards, ♦ −3.76 over 230, ♣ −1.28 over 144.

**The DD-blindness defence was tested and failed.** It is the strongest
objection available, and it is a real effect elsewhere: kickback exists to
*stop* at five of trump when partner denies a keycard, and double dummy sees all
52 cards, so it never lets a thin slam go down — it charges the arm that stopped
and credits the arm that punted. PD sitting at parity looked consistent with
that. The sd row is the scorer that does let a thin slam fail, and it still
reads negative at both vulnerabilities with both intervals excluding zero.

> **Caveat (added 2026-08-03, when §7.15 reopened the diagnosis):** the sd
> row above is a 400k **aggregate** over a divergent set in which 92%+ of
> boards saw no keycard ask at all — it mostly re-priced the card-row
> perturbation of the net, not the relocation, and there is no sd-lead row in
> this table at all (right-siding unpriced). "Tested and failed" is therefore
> recorded stronger than its evidence. The ask-bucketed rescore under the
> full instrument set — including the sd-blend, the calibrated slam scorer
> §7.14 said did not exist — is §7.15's step 0; quote that, not this
> aggregate, for or against the relocation.

**What is not in doubt is the ladder's arithmetic.** §7.2's table is correct: a
relocated ask genuinely brings the overshooting answers to zero, in all three
lanes, which is why this was built. The measurement says the room it buys costs
more than it returns — the 4♦/4♥/4♠ faces the ladder claims are among the most
common natural calls in bridge, and the bidder gives up more by not having them
than the asker gains by stopping accurately. That is a claim about *this*
system's floor, not about kickback as bridge theory.

### 7.14 Reverted to opt-in, and the case closed (2026-08-03)

`set_kickback` is **off by default** again, one day after §7.10 turned it on.
The shipped default is the knob-off arm in every regime the knob reaches: rule
presence is gated at `instinct()` build time, the recognizers read it at
classification time, `classify_bba` serves the plain artifact again, and the
card stops disclosing `Kickback 1430` (the only row that moved in either golden
`.bbsa`).

The evidence §7.10 shipped on is superseded, not merely outweighed. It was a
*package* price — a two-week-newer net plus a convention — and gate 1 measured
the net's share of it at +0.19/+0.25 plain DD, far more than the +0.0705 PD the
package showed. What is left for the convention alone is §7.13's loss.

**The case is closed until a scorer can fight DD's slam optimism.** The one
argument that could reopen it is the DD-blindness defence, and it has been
tested and failed on the only instrument currently available. What would change
that is a *new instrument*, not a new argument: the sd-lead scorer exists
because plain DD was too kind to declarer on opening lead, and slams want the
same treatment — something that makes double dummy stop finding the winning line
in a 6-2 fit off a keycard. Until such a scorer exists, do not re-measure
kickback; it will keep reading negative for reasons the harness cannot see past.

The two decisions were one package. The other half — making the configured net
the default floor — is **not taken here**: reverting the knob restores a stance
the shipped twins already serve correctly, so it stands alone. Gate 1's verdict
keeps until that decision is made on its own.

**Knob surface since 2026-08-03:** `set_kickback` and `set_redwood` were folded
into one enum knob, `set_rkcb_variant(RkcbVariant)` with `Plain` (the shipped
default) / `Redwood` / `Kickback` — the bool pair encoded four cells for three
stances, since kickback implies the Redwood scope and hearts-only is
unrepresentable by design. Every `set_kickback`/`set_redwood` mention above is
history and reads as `set_rkcb_variant(Kickback)`/`(Redwood)` today; the
regimes, weights selection, and card row are unchanged
(`docs/bidding-options.md` §A7 and its encoding audit).

> Superseded in part by §7.15: the scorer this section said did not exist was
> built the next day, the cell was re-measured under it, and the verdict is
> *not* rescue — the loss decomposes into two build defects no scorer can see
> past. "Do not re-measure until a scorer exists" is discharged; the operative
> guidance is §7.15's.

### 7.15 The instrument arrived, and the diagnosis changed (2026-08-04)

The calibrated slam scorer §7.14 demanded now exists — the **sd-blend**
(`docs/measurement.md`, "the slam bracket"): a λ(level) logit-space mixture of
the sd-lead optimist endpoint and the sd-playout pessimist endpoint, fitted to
Pavlicek's after-lead table, validated by `probe-slam-battery`. The fair cell
was re-run from seed 1785708870 (reproduction: NV −0.01041, vul −0.00905 per
board plain DD, both inside §7.13's intervals), dumped
(`ab-kickback --dump`), and every divergent board was re-priced under nine
instruments with the ask-bucketed census (`--rescore --sd-ask-only`; sd rows
priced on the ~8% of divergent boards where either arm asked).

**The verdict is not rescue.** Per-divergent-board means, both cells:

| lane (pooled NV+vul) | boards | plain DD | playout | blend | mechanism |
| --- | ---: | ---: | ---: | ---: | --- |
| no keycard ask | 164,210 | −0.19 | — | — | card-row perturbation of the net (PD **positive**) |
| ♥ relocated | 391 | −1.13 | −0.35 | −0.53 | the only DD-optimism candidate; NV playout is a wash (+0.03), vul is not (−0.73) |
| ♦ relocated | 113 | −2.04 | −2.60 | −1.94 | **eaten 4♥** — 103/113 boards |
| ♣ relocated | 144 | −1.53 | −3.24 | −3.02 | **grand-blast continuation** in the freed room |

Three mechanisms, none of them a scorer question:

1. **The eaten 4♥ (♦ lane, and hiding in the no-ask bucket).** Exactly the
   classical objection: the baseline bids 4♥ and *plays the heart game*; the
   feature's 4♥ is the diamond keycard ask, partner answers, the pair lands in
   5♦+. 90% of the ♦ lane's loss in both cells, negative under every
   instrument including PD — game-made-versus-slam-try-wrecked is DD-fair, so
   no clairvoyance correction can touch it. The same denial also operates
   *without* an ask: on no-ask boards whose auctions are identical until the
   exact ply where the baseline bids 4♥ and the feature deviates (with the
   side having bid diamonds), the feature loses 2.19/bd NV and 3.32/bd vul —
   against a mirror control (baseline deviates off the feature's 4♥) of only
   1.39/1.65. The asymmetric excess, ~−650 IMPs NV and ~−1,130 vul, is the
   silent half of the eaten-4♥ bill; combined with the asked lane it is
   ~3.5% of the NV cell's loss and ~7% of the vul cell's, and it is
   PD-negative — a genuine bidding leak masked in §7.13's aggregate by
   PD-positive perturbation noise.
2. **The grand-blast continuation (♣ lane).** The relocated answer decodes
   correctly; what fails is the continuation in the very room the relocation
   wins. Exhibit (top loser in both cells): `1♣–1♠–3♣–4♦–5♣ (2+Q)–7♣` down
   one off the ♠K, while the baseline hears the *identical message* as
   `4NT–5♠ (2+Q)` and signs off in 6♣ making. Below 6♣ the feature asker has
   space its grand logic overbids into; the cramped baseline does the boring
   right thing. Heavy-tailed — three grand blasts are a third of the lane.
3. **The ♥ lane is the residue** — no eaten-4♠ signature (3/391 boards), and
   the one lane where DD optimism is even arguable: NV flips to a wash under
   the playout, vul does not, pooled playout −0.35 on thin n. If anything in
   §7 deserves an enriched probe it is this lane alone — but see below.

**What the instrument contributed** is separation, not absolution: it proved
the ♦ and ♣ lanes lose for reasons that survive the pessimist bound, and
identified the ♥ lane as the only place DD's slam optimism plausibly
overcharges the relocation. §7.14's premise ("kickback keeps reading negative
for reasons the harness cannot see past") is retired — the harness now sees
past DD, and the relocation still loses, for *named* reasons.

**The reopen path is a build change, not a measurement change:**

- a **claim guard** on the ♦ ask: do not relocate onto 4♥ while hearts are a
  live strain for the partnership (either side has shown hearts, or diamond
  agreement is soft) — the guard's cost is re-admitting 4NT's overshooting
  answers on exactly those faces, which §7.2 prices;
- **grand discipline over relocated answers**: the asker's continuation in
  the freed room must be authored (king-ask usage, a cap at small slam
  without third-round-control confirmation), not left to the floor's grand
  heuristics;
- then re-run this cell. The enriched probe (`probe-kickback-relocation`,
  §7.9's tool) is **held** until those fixes exist — probing the current
  build would measure two defects this section already names.

The no-ask bucket's −0.19/bd (PD-positive) remains the headline's bulk and is
the card row perturbing the net — part of the knob's price under the
configured-net design, but not evidence about relocation mechanics. Artifacts:
dumps and rescore logs under `/mnt/hdd-data/jdh8/pons-ab-results/`
(`kickback-fair-cell-{none,both}/`, `kickback-rescore-{none,both}.log`).
