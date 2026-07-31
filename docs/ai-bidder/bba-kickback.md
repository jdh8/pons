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
cited by name below, all in type `EPBot`. Validation: 37 constructed clause
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
  (probe-verified); Gerber forces 5♣;
- the trump queen is "held" when the honor table says so **or
  `posiadane_karty[trump] ≥ 10`, computed as own actual length + partner's
  bilans-*reconstructed* length** (`MY_HAND.dlugosc + TMP_HAND.dlugosc`) —
  probe-verified boundary: a 4-card raise opposite a probable 6 answers
  "queen yes" on J-fourth; a 3-card raise does not;
- queen answers: signoff in trump without it; with it, cheapest side-suit
  king below 6-of-trump (skipped steps *deny* that king), else an NT bid
  ("queen yes, no side king" — probe: 5NT);
- king answers after 5NT: a count ladder (`"K=n"`; probe: 6♦ = K=1), or
  per-king under `5NT inviting`;
- the alert label rides `ustaw_konwencje`: **`"Kickback 1430, for !D"`**,
  same shape as every ask family.

The ♠K-grand payoff with hearts agreed: ask 4♠, answer 4NT (1/4), queen ask
5♣ with answers below 5♥ that *show or deny specific side kings*, and even
the default 5NT king ask answers at 6♦ = "one king" — still below 6♥. Plain
4NT RKCB spends 4NT+5-level on keycards alone before the 5NT king ask.

## 4. Probe evidence (examples/probe-bba-kickback.rs)

39 constructed cases, **35 expectations pass / 0 fail** (44 exploratory
observations), all against the shipped `.so`. The load-bearing rows:

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

## 7. Proposed pons trigger rule (design sketch for the follow-up campaign)

Guiding choice: adopt the *static-inventory* school, not BBA's dynamic
shown-4+ guard — our book knows which auctions agree trumps formally, and a
deterministic trigger keeps the floor, the readings, and the disclosure
consistent (a reading knob is a bidding knob under a neural floor).

- Knob `set_kickback` (default off, byte-identical system). When on:
  with a **formally agreed** trump T ∈ {♣, ♦, ♥} below the four level, in an
  uncontested constructive auction, the keycard ask is 4-of-(T+1); 4NT
  reverts to natural/quantitative in those auctions. No relocation for
  spades. Suppress the relocation when the would-be ask suit was bid
  naturally by either hand (mirrors the one BBA guard worth keeping) —
  then 4NT stays RKCB.
- Answers: existing 1430 ladder relative to the ask; queen/king asks
  relocated per §3's available-bid rule; signoff = cheapest trump.
- Competition: reuse the DOPI/ROPI/DEPO family relative to the relocated ask.
- Wiring: `american/slam.rs` (RKCB home; the `Kickback (4♣/4♦), the usual
  remedy, is out of scope` comments mark the entry points), floor awareness
  in `instinct.rs` (`keycard_trump` / `keycard_asked` / `keycard_answer`),
  `Inferences` reading + `.alert("kickback")`, card rows `Kickback 1430 = 1`
  (mutually exclusive rows stay honest), invariant test, and the A/B per
  [measurement.md](../measurement.md) — plain DD + perfect defense, against
  the real routing; the shipped plain-4NT minor keycard
  (`set_minor_keycard`) is the incumbent arm.
- The interesting measurable: BBA's own motivation cases — minor slams
  vetoed below 5m (the C3 probe: a 2+Q answer lands in 5♣ *as the contract*)
  and the hearts grand via the ♠K.
