# Competitive 1NT — closed package history

> **Archived 2026-08-19.** This file holds the closed N1 Landy campaign, N4's
> superseded measurement rounds v1–v6, and the N4b diamond-double sweep. The
> live opponent model, census, current package tables, measurement discipline,
> queue, and ledger remain in
> [one-notrump-competitive.md](../one-notrump-competitive.md).

## N1* — the Landy `(2♣)` counter (**SHIPPED DEFAULT-ON 2026-08-14**)

The census's top loser, closed in five measured rounds in one day. This
section is the 2026-08-14 shipped state; the exploration that produced it is
digested at the end, and every measured verdict lives in the
[ledger](../one-notrump-competitive.md#ledger).

> **Superseded as the default 2026-08-15 by [§N1j](#n1j--the-bba-ladder-counter-shipped-default-on-2026-08-15)** —
> the BBA-ladder table now rides the bare declaration; the stack below
> remains fully wired behind `defense_2c_landy_bba = false`
> (`bba-gen --defense-2c-landy-bba false`) and is the A/B baseline N1j was
> measured against. The engagement and disclosure subsections here still
> govern both tables; the lane-wide mirror-leak gate remains in the live
> [measurement discipline](../one-notrump-competitive.md#the-mirror-read-leak--open-defect-gated).

### Engagement — a disclosure, not a knob

What their `2♣` means is a fact about the opponents, so the engagement bit is
**`their.two_clubs_landy`** in `Agreements::their` — the disclosure channel,
never our own knob space (`competition.defense_2c_landy` existed for one day
and was deleted; `defense_2d_multi` was deleted outright). Undeclared
defaults to natural — the systems-on rebase — which self-play demands: our own
tables' `2♣` overcalls *are* natural.

A harness that knows its opponent derives the declaration. `bba-gen`'s
`their_2c_landy` plays explicit `--their-card`/`--their-conv` Landy-family
rows at **face value** (a declared no-Landy set reads natural — deviations
behind a declaration are *their* infraction) and, with no declaration,
defaults the 2/1 reference to Landy from its **measured behavior**, because
its own card lies: `21GF.bbsa` declares `Cappelletti=1, Landy=0,
Multi-Landy=0` while the engine bids Multi-Landy regardless. `bba-decompose`
applies the same correction when replaying dumps: `--landy-counter false` for
pre-N1 dumps, `--landy-stack false` for dumps generated between the
base-counter ship and the stack ship.

Structure knobs ride the declaration: `defense_2c_landy_transfer` (implies
the cues, `defense_2c_landy_cues`) plus the three repairs
`defense_2c_landy_cue_floor`, `_fit_answers`, `_competition` (each implies
`_transfer`) — **all default true**. They engage only under the declaration,
so the default *system* is byte-identical: `smoke-default --count 20000
--seed 1` SHA-256
`8ea2f5678a733cfe3ead79411d9cb31b8e95d37de52236e597fc38f9dec82bbb`, unchanged
by every ship in this package.  (That constant later moved **outside** the
package — `reading.completion_alerts` shipped default-on 2026-08-14 (94daa30)
and re-based the default dump.  That constant was
`18aba5ce4d7d7e3b5fe3f26a453da96a53ae0a239f1bd56dfa201ae84034b60a` from N1j
until `reading.strength_ceilings` + `DecisionProfile::legacy_view` shipped
default-on 2026-08-16; the current constant is
`cf583ff5f46d7e7ffdf0ab065dcb285680a6b7d865df42cf5e139f0b74ab7b90`.) `bba-gen`'s stack flags are `Option<bool>`
(unset = engine default; a pre-ship arm is spelled
`--defense-2c-landy-<knob> false`).

⚠ **A cue-constraint edit is a reading edit.** The lane-wide
[mirror-read leak](../one-notrump-competitive.md#the-mirror-read-leak--open-defect-gated)
reflects the cue rows onto auctions *they* open; gate every arm pair.

**Disclosure to BBA**: its `.bbsa` schema has no row for our counter to
*their* Landy over *our* 1NT — nothing to wire in `card.rs`; golden cards and
`alert-sites.txt` unchanged.

### Responder's table over `1NT (2♣)`

`landy_responder` + overlays, `competition/lebensohl.rs`. Either/or with the
systems-on rebase, **not** an overlay — leaving the rebase registered would
remap the values `X` onto stolen Stayman a round later — the ungated-continuation
bug that sank the deleted Multi counter.

| Call | Meaning | Weight |
| --- | --- | --- |
| `3NT` | game values, **both majors stopped, no six-card minor** | 180 |
| `3♣` / `3♦` | **INV (8-9), 6+ suit** | 176 / 175 |
| `2♥` / `2♠` (cue) | **INV+, `points(10..)`**, 5+ clubs / 5+ diamonds — alert `comp:landy-cue` | 173 / 172 |
| `3NT` | game values, ungated | 170 |
| `X` | **values**, `hcp(8..)`, penalty-oriented — alert `comp:landy-values` | 145 |
| `2♦` | weak natural, 5+, `points(..=9)` + the `natural_floor` | 140 |
| `2NT` | **transfer to clubs, weak 6+** — alert `comp:landy-transfer`; projects `len(♣,6..) & points(..=9)`, tight enough for `project_authored` | 110 |
| Pass | finite catch-all | 0 |

Design points, each measured or smoke-found:

- **The gated `3NT` outranks the cues** because opener declares any notrump
  contract (opener bid 1NT — Law 54), so responder's direct `3NT` costs no
  siding; denying a six-card minor sends sources of tricks through the cue.
- **`3NT` takes no stopper gate**: their `2♣` promises no clubs, and demanding
  a major stopper is no use — they hold both.
- **`X` floors on `hcp`, not `points`**: defending does not care about
  distribution; shapely weak hands belong in `2♦`/the transfer.
- **Cue floor 10** (N1d): at weight 173 against the double's 145, an 8+ floor
  took every 8-9 five-card-minor hand off the values double, worth
  −0.92/−2.53 PD per fired; flipping them back (cue→X, 55-60% of the repair's
  divergence) paid +2.0…+5.1 PD per fired.
- **`2NT` transfer**: their `2♣` is artificial, so clubs are ours;
  transferring puts the weak escape a level lower *and* right-sides it into
  the 15-17 hand. The package's biggest earner, and mostly a **new** call —
  the weak six-card club hand had no call at all under the base counter.
  Completion reuses `complete_lebensohl_relay()`, natural in the target and
  unalerted (`complete_advance_transfer` doctrine); responder passes it. The
  natural `2NT` invite it displaced carried almost nothing — the values `X`
  outranked it on every 8+ hcp hand.
- **`3m` INV is answered by the uncontested invite's own size decision**
  (`size_ask_accept_floor`, default 16): `3NT` from the top with both majors
  stopped, else sit — minor game is out of reach of a combined 23-26.

### Opener's answers

Natural calls (`landy_natural_answers`). Authored after the first A/B traced
its loss entirely to their absence: a call the book leaves unanswered is
phantom-completed by the floor as the default-system gadget it replaced.

| After | Opener |
| --- | --- |
| `X -` | Pass — sitting for the values double |
| `2♦ -` | Pass, always (`lebensohl_signoff_raise` doctrine) |
| `2NT -` | forced `3♣` (transfer completion) |
| `3♣`/`3♦ -` | `3NT` at `hcp(16..)` with both majors stopped, else Pass |
| `3NT -` | no node — audited clean |

Cue answers (`landy_cue_answer`, with N1e's fit answers): **level carries
strength — cheap is minimum — and every raise or ask promises 3+**. The
notrump rungs absorb the doubleton (*both majors stopped, or ≤2 support*) and
the terminal catch-all is `2NT`, so the 5-2 raise (measured −10/−8 PD per
fired) cannot be manufactured. A stopper is guaranteed only alongside a fit;
responder knows which story a rung told from the rung itself.

| Opener | Shows | Weight |
| --- | --- | --- |
| `3NT` | maximum — both majors stopped, or ≤2 support | 160 |
| `3♥` / `3♠` | maximum, asks for the stopper opener lacks, 3+ in the minor | 155 |
| `4m` | maximum, 3+, neither major stopped | 150 |
| `2NT` | minimum — both stopped or ≤2 support (terminal catch-all) | 145 |
| `2♠` | minimum ask — club cue only, the one rung below the 3-level | 140 |
| `3m` | minimum raise, 3+ | 100 |

Responder answers an ask by showing the stopper (cheaply on a minimum, so
opener still judges game) or retreating to the minor made safe. Over the
minimum `3m`, responder may **re-cue** `3♥`/`3♠` with a game force and a
stopper worry — opener bids `3NT` holding it, else takes the minor. Over
`2NT`, pass or `3NT`. **`4♣`/`4♦` over opener's minimum rebids is a slam try**
(13+ with a six-card suit); opener's continuation is deliberately the floor's —
a `4m` *suit* contract lets the floor cue-bid on to slam where a notrump rung
dies in `3NT`. Every other rung is authored down to the placing call, because
`Inferences` has no forcing channel: a rung left to the floor reads as bare
"5+ ♣, 8+ points" with no notion of an invitation.

### Competition over the counter (N1f)

Three shapes; everything deeper stays the floor's, deliberately:

- **Their `X` of a cue or ask** takes no room, so the answer is **verbatim**
  (the immediate table re-registered on the `(X)` suffix), and every deeper
  X-then-bid tail is stripped back onto the clean subtree by a
  `systems_on_over_double` rebase — the contested-Stayman idiom, one entry
  covering asks, rebids and re-cues.
- **Their raise over a cue** (`2♠`/`3♥`/`3♠`): compressed ladder — `3NT` =
  both stopped + maximum, the raise = 3+ by size, Pass = the rest (safe
  because responder is INV+ and guaranteed another turn).
- **The doubled club transfer** is still completed, sign-off intact.

`(4♠)`, overcalls of opener's answers, and everything deeper: floor.

### Ship evidence

Two ships, both at the standard gate; full per-version verdicts in the
[ledger](../one-notrump-competitive.md#ledger).

**The base counter** (N1, `their.two_clubs_landy` + `landy_natural_answers`):
plain wash + PD CI-clear win in both vuls — NV PD +0.0032 ±0.0028, vul
+0.0043 ±0.0032 (76.8k bd/arm/vul), **confirmed at 3× n** (PD +0.0032/+0.0028,
230.4k bd/arm/vul) after a single-seed non-replication scare.

**The stack** (N1c transfer + N1d/e/f repairs) against the shipped base,
pooled over seeds 1786694464 + 1786695954, 460.8k bd/vul — the package's
first `win | win`:

| `f↔on` pooled | plain DD | PD |
| --- | --- | --- |
| NV all | **+0.00068 ±0.00062** | +0.00075 ±0.00077 |
| NV ours | **+0.00091 ±0.00052** | **+0.00077 ±0.00064** |
| vul all | **+0.00085 ±0.00072** | **+0.00100 ±0.00087** |
| vul ours | **+0.00075 ±0.00058** | +0.00060 ±0.00070 |

Six of eight DD cells CI-clear positive, 8/8 sd cells positive at both seeds,
**no negative cell in 24 readings**. Increment attribution
(230.4k bd/arm/vul, seed 1786694464):

| increment | fired (NV/vul) | verdict |
| --- | --- | --- |
| `d↔xfer` cue floor | 169 / 136 | **the engine**: plain wash + PD **+0.0009 ±0.0008** NV / **+0.0015 ±0.0009** vul (+1.21/+2.49 per fired) |
| `e↔d` doubleton-NT answers | 3 / 1 | no population left post-floor; ships on naturalness (raises promise 3+; the alternative was a priced −10/−8 PD 5-2) |
| `f↔e` interfered tails | 17 / 11 | CI-wide wash; ships as the iron rule's convention-completion |

`ab-landy-counter.sh`'s arms are spelled in post-flip terms (landy-f *is* the
default; landy-on switches the stack off); the confirm pair is
`scripts/ab-landy-confirm.sh`.

### Residue

- **Their *second* call still floors us** — **diagnosed and half-closed by
  N1g** (next section): N1f's worst board was
  `1NT (2♣) 2♥ (2♠) 3♣ (3♠) 4♦ - 4♠ X` — the floor bidding a phantom `4♠`
  off an envelope that claimed LHO held five clubs (−17 PD). The wiring
  fixes the inputs (and the N1g probes show phantom contracts being
  *corrected*); whatever phantom-bidding remains after a post-ship decompose
  is the true floor-discipline residue — an M6.4-style rail
  (conversation-in-motion → instinct, or an envelope-gated new-suit veto
  scoped off agreed fits), not another node ring.
  - **Post-ship decompose RUN 2026-08-14**
    (`ab-results/landy-postship-decompose/`, `lane-residue.py`; arms
    regenerated at HEAD on the three N1g ship seeds — the 27003c6 dumps do
    **not** replay at 94daa30, 122/2.26M calls moved). 2 653 lane boards in
    460.8k enriched; **148 failing our-side contracts (97 deals, −1 256 PD
    duplicate swing** — table-B share included): 84 hopeless 3NTs (game
    judgment, a different lever), 64 suit contracts of which **17 sat on
    ≤5-card combined holdings — the phantom class persists** (worst repeat
    offender: `1NT (2♣) 2♥ (2♠) 3♣ (3♠) 4♣ - 4♥` on a 2–2 fit, −18 PD;
    also `{cue} - 2NT - 4♦ - 4♠` on 4–1, the slam-try continuation the book
    deliberately leaves to the floor). By reader coverage: **118 boards /
    −921 PD had only *covered* their-calls** (true-envelope floor
    indiscipline — several phantom majors bid *into* suits the 2♣ envelope
    already gives the opponents), 30 / −335 PD followed an unread
    their-call. **Verdict: rail first** — the false-envelope share is the
    minority, and an envelope-gated new-suit veto catches both classes; the
    `their_profile` split stays the structural upgrade path. **Deleting the
    cue-response nodes is CLOSED**: responses-only deletion breaks
    convention completeness, whole-tree deletion re-litigates the shipped
    win|win, and the floor beneath still phantom-bids. The alert-derivation
    campaign (94daa30) is orthogonal to this lane — all three of its pieces
    act on *our* rules' alerts.
- **The `3♣`→`2♥` row is CLOSED** (re-probed 2026-08-14 against the shipped
  stack on the N1g dumps, `read-on ↔ landy-on`): the pre-stack −1.09/−3.26 PD
  loss is gone — vul both **+3.12 plain / +2.38 PD per fired**, NV a small
  mixed wash (+0.42/−0.25), n=20 flips. No forcing-3m arm is warranted; the
  residual worst boards are the known `{cue}`→`4♥`-instead-of-3NT artifact,
  2-3 per vul.
- `{cue} - 3NT -` and `{cue} - 4m -` (opener's maximum rungs) remain
  unauthored; never surfaced in a dump.

### N1g — the read-side wiring (**SHIPPED DEFAULT-ON 2026-08-14**)

Decomposing the residue found **the floor's inputs are lies in this lane**:
`their.two_clubs_landy` had *zero read-side consumers*.  The disclosure moved
the book only; opponent-call decoding falls back to our own profile
([read.rs:333-335](../../src/bidding/inference/read.rs)), whose shipped defaults
read their `2♣` through the natural walk as **5+♣, 8+** — so on every board
of this lane the learned floor's LHO envelope (its then-live `features_v5` inference
block) claimed five clubs while LHO held both majors.  The residue boards are
exactly the floor's boards; the phantom `4♠` was bid off a false deal
picture, before any question of the net's weights.

The wiring, `ReadingProfile::their_landy_reading` (**default on**; the
pre-ship arm is `bba-gen --ns-their-landy-read false`): under the declaration
their `2♣` reads 4-4+ in the majors with no strength claim, their
`2♦`/`2♥`/`2♠` advances and direct `3M` raises are natural-suppressed (a
preference plays on a doubleton; the `3M` would otherwise read as a weak-jump
six-carder).  Implemented as a seat-gated hand reader
(`inference/readers.rs::their_landy_reading`) that fires only when the `1NT`
opener is on the *reader's* side — our own `2♣` overcalls cannot match — and
that does **not** extrapolate through the systems-on strip.  The disclosure
itself re-homed to `DecisionProfile::their` per the dual-read house rule,
proven byte-identical (smoke `8ea2f567…` unchanged).

**Ship evidence** (three seeds pooled, 230.4k bd/vul, `read-on ↔ read-off`,
`scripts/ab-landy-read.sh`, seeds 1786704432 / 1786705413 / 1786705763):
plain **wash** (NV −0.00051 ±0.00072, vul +0.00001 ±0.00078), PD **win both
vuls** (NV **+0.00104 ±0.00097**, vul **+0.00112 ±0.00104**; ≈ +1.0/+1.5 per
fired at 0.07–0.11% fired), sd agreeing in sign (sd-PD +0.00065/+0.00076,
sd-plain wash) — the decision table's `plain-wash + PD-win` ship row.  The
**isolation gate passed at zero foreign boards in both vuls** — the first
pair in this campaign to do so.  Mechanism (from the divergence probes): a
conservative shift off true envelopes — fewer thin NV games/slams (plain DD,
the optimism bound, dislikes exactly those; PD likes them), and partner's
phantom-`4♥` contracts *corrected* to the real fit (+17 PD boards).

Two lessons paid for en route: **seed 1 of the fixed build showed a CI-clear
NV-plain loss that seeds 2–3 refuted** (single-seed negatives are not design
inputs — again), and **v1 of the reader leaked through the systems-on
strip**: in `(1♣) 1NT (2♣)` lanes the strip re-reads our 1NT overcall as an
opening, the seat gate passed, and their *responder's* 2♣ read as Landy.  The
v1 worst boards were all this leak; the fix pins the disclosure out of the
strip recursion (`read.rs`, regression-tested).

A sibling defect found in the same sweep: the forced `3♣` completion of a
sohl `2NT` relay is `hcp(0..)` — it projects nothing, dodges the alert
invariant's artificiality witness, and reads as **four real clubs** where no
blanket covers it.  That lane is *advance-sohl* (their weak two, our takeout
`X`, the relay), not this one — after our own `1NT` opening the walk blankets
the whole structure, so plain Lebensohl and the N1c transfer completion are
latent.  The knob grew into the family `reading.completion_alerts`
(2026-08-14, superseding `lebensohl_completion_alert`; **shipped default-on
the same day** — `scripts/ab-completion-alerts.sh`, unfiltered, pooled over
three seeds at 614.4k boards/cell: vul plain +0.0005 ±0.0004 and vul PD
+0.0006 ±0.0005 both CI-clear, NV positive, sd sign-agreed): it alerts the
puppet (decodes ⊤, suppresses the club read)
and the rest of the completion family with it.  Never fold its arm into
N1g's.

### N1h / N1i — the minor rungs re-priced (**both REFUTED, both opt-in**)

Two arms over the same four rungs, measured 2026-08-15 against the shipped
stack on three shared seeds (230.4k bd/vul, `ab-results/landy-low{,-v2,-v3}`;
the `low-off` baseline is shared and was verified board-for-board identical
before reuse):

| | `2♥`/`2♠` cue | `3♣`/`3♦` | `2♦` | `2NT` |
| --- | --- | --- | --- | --- |
| shipped | `points(10..)` | `points(8..=9)` | `points(..=9)` | `points(2..=9)` |
| **N1h** `defense_2c_landy_low_minors` | `points(9..)` | `points(7..=8)` | — | — |
| **N1i** `defense_2c_landy_hcp_rungs` | `hcp(9..)` | `hcp(7..=8)` | `hcp(..=6)` | `hcp(..=6)` |

| verdict | NV plain | NV PD | vul plain | vul PD |
| --- | --- | --- | --- | --- |
| N1h | +0.00036 ±0.00051 | −0.00044 ±0.00066 | +0.00002 ±0.00061 | **−0.00081 ±0.00074** |
| N1i | −0.00029 ±0.00043 | −0.00039 ±0.00062 | −0.00014 ±0.00052 | −0.00036 ±0.00068 |

N1h lands on `plain wash | PD loss` (the mirror of N1g's ship row); N1i has no
CI-clear cell in either direction with every cell leaning negative. Neither
ships. Both `probe-divergence` decomposes leak on the cue-constraint mirror
(10-13% foreign), so the per-row figures below are ours-only.

**The durable finding: the cue floor is settled.** `cue ← X` measured negative
in both arms — N1h −1.80 PD/fired over 96 boards, N1i −2.96/−4.04 over 46 —
and N1d originally measured the same migration at **+2.0…+5.1** going the
other way. Three independent experiments agree that hands do not belong on the
cue at the values double's expense. `defense_2c_landy_cue_floor`'s
`points(10..)` is not to be probed again.

Three smaller rows worth keeping, all ours-only, PD per fired:

- **`Pass ← 2♦` +2.40 over 52 boards** (N1i), positive at both vuls, plain a
  wash — the weak five-card-diamond `2♦` on a 7-9 point hand may be worth less
  than passing. Per seed +4.50 / +1.33 / −1.09, so a lead, not a result; the
  isolated arm would be `hcp(..=6)` on `2♦` alone.
- **`3♦ ← 2♦` +3.11, plain +3.96 over 27 boards** (N1h) — the 7-point six-card
  diamond is worth an invitation. Its club twin is not (`3♣ ← 2NT` −2.19: the
  transfer's right-siding is worth more), which is why the two minors are not
  symmetric here.
- **`cue ← 3♣` −2.88 over 26 boards** (N1h) — shifting the `3m` band whole
  rather than lowering only its floor cost real IMPs.


### N1j — the BBA-ladder counter (**SHIPPED DEFAULT-ON 2026-08-15**)

With the rung lane closed, the next probe was the *shape* of the table.  The
shipped stack beats BBA partly on gadgets the anchor's model of us cannot
represent — an exploit-flavored win, which matters now that BBA's role is
exploit guard for the BEN campaign.  N1j re-shapes responder's whole table to
the structure BBA itself plays as a 1NT opener facing Landy
([bba-1nt-counter-defense.md](../ai-bidder/bba-1nt-counter-defense.md)), and its
ship gate was pinned **before the run** as non-inferiority — zero CI-clear
negative cells across pooled {NV,vul}×{plain,PD} — because the rationale is
structural alignment, not IMPs.  `defense_2c_landy_bba`; the N1b–N1i
structure knobs are **inert** under it, and the stack stays wired behind
`--defense-2c-landy-bba false` as the measured baseline.

| Call | Meaning | Weight |
| --- | --- | --- |
| `3NT` | game values, both majors stopped, no six-card minor | 180 |
| `2♥` / `2♠` | **GF takeout**, 4+♦ 4+♣, exactly two in the bid major (2-2 bids `2♥`, so `2♠` = 2=3=4=4) — alert `comp:landy-tko` | 178/177 |
| `3♥` / `3♠` | **GF splinter**, 4+♦ 4+♣, 0-1 in the bid major — alert `comp:landy-spl` | 176/175 |
| `2NT` / `3♣` | transfers to ♣/♦, 6+, **any strength** (weak sign-off through GF) — alert `comp:landy-transfer` | 174/173 |
| `3NT` | game values, ungated | 168 |
| `X` | values `hcp(8..)` — the stack's row **byte-identical** | 145 |
| `2♦` | weak natural 5+ — `hcp(..=6)` under the shipped cap (`defense_2c_landy_weak_2d_cap`), `points(..=9)` without it | 140 |
| Pass | finite catch-all | 0 |

Deviations from BBA verbatim, each deliberate: the values `X` stays (BBA
never doubles Landy; N1d/N1h/N1i all defended the row), the club transfer
sits on `2NT` rather than BBA's `2♠` (the takeout pair spends both major
cues), and the GF both-minors family is ours (BBA has no call for the hand).
The two-suiter family outranks the transfers so a 6-4 hand shows the whole
picture; a hand with a doubleton in one major and 0-1 in the other splinters.
No `6NT` blast (BBA's 2.9%): opposite our 15-17 with a live overcall, an 18+
responder is arithmetic-impossible in the lane.

**Continuations** (the authored minimum): opener answers a takeout/splinter
in **notrump with the bid (short) major stopped or no four-card minor** —
responder knows its own holding in the unbid major, so opener answers only
the unknown, and the minor-less branch doubles as the forced catch-all — else
picks a 4+ minor cheapest-first, denying that stopper.  Over the notrump
answer responder's cue of the *other* major asks it (3NT holding it, else a
four-level minor, floor continues); over a three-level pick responder places
(3NT on its own double stopper / `4m` slam re-open at 14+ / `5m` on the
guaranteed 4-4).  The wide transfers complete forced — the `3♦` completion
joins the `completion_alerts` family — and responder's rebid shows the one
major stopper held (`landy_recue_answer` supplies 3NT with the other), `4m`
slam-tries at 13+, else 3NT; the invitational one-suiter deliberately dies at
the completed three level, the N1h/N1i trade of the invite for right-siding.
Tails are the N1f idiom: doubled calls answered verbatim plus the systems-on
rebase, raises get a compressed ladder (NT = stopper / minor pick / Pass,
safe under the game force), doubled transfers still complete.

**The reading ceiling — why "aligned" cannot mean "readable".**  The
disclosure channel for this lane is real and live: `Transfers if RHO bids
clubs = 1` (row 122) is on our generated cards
([card.rs](../../src/bidding/card.rs), emitted from `lebensohl_style != Off`)
and pushed per-side into EPBot's model of us.  But it projects our
**uncontested Puppet scheme** onto the counter lane, so BBA decodes our
counter calls as: `2♦` → Jacoby-♥, `2♥` → Jacoby-♠, `2♠` → ♣-transfer,
`2NT` → ♦-transfer, `3♣` → Puppet Stayman — regardless of what we author.
Exact readability would need European minors uncontested (out of scope;
lying on the card is not an option).  The alignment claim is therefore
**structural** — we play the ladder shape the anchor itself chose, with no
rungs it cannot conceive of — not literal.  Found en route and flagged in
code, not fixed: `bba-gen`'s `--advertise-natural`/`--advertise-landy`
oracle never receives `.with_opponents(disclosure)`, so those lanes model us
as playing BBA beyond the three advertised rows; a blind fix risks the card
push clobbering the advertisement (row-push order unverified).

**Ship evidence** (pooled seeds 1786753231 / 1786753518 / 1786753808,
230.4k bd/vul, 76.8k bd/arm/vul/seed, enriched `--filter-1nt`,
`scripts/ab-landy-bba.sh`):

| pair | NV plain | NV PD | vul plain | vul PD |
| --- | --- | --- | --- | --- |
| `bba-on ↔ bba-off` (the ladder) | +0.00083 ±0.00085 | +0.00083 ±0.00110 | +0.00080 ±0.00100 | +0.00073 ±0.00123 |
| `bba-cap ↔ bba-on` (the 2♦ cap) | −0.00003 ±0.00027 | **+0.00037 ±0.00033** | +0.00017 ±0.00024 | **+0.00050 ±0.00035** |

- **The ladder ships at its pinned gate and beats it**: zero CI-clear
  negative cells, and *all eight* DD cells (plus all eight sd cells) lean
  positive — NV plain misses CI-clear by 0.00002.  Fired 280/234 (NV/vul).
- **The cap ships at the standard gate**, not the relaxed one: plain wash +
  PD CI-clear win at both vuls, sd sign-agreed, **isolation gate 0 foreign
  boards both vuls** (the campaign's second after N1g), and every divergence
  is the predicted `2♦ → Pass` row (×59 pooled, +2.58/+4.54 PD per fired) —
  the N1i lead (+2.40) replicated as a result.
- **The `2M ← X` guard passed vacuously**: not one hand left the values
  double for the takeout family in 460.8k boards.  The family's boards came
  off the old *cues* (`2♥/2♠ → 3♥/3♠/2NT` rows, all plain-positive), so the
  three-experiments finding (`cue ← X` negative) was never touched.
- **Movers** (ours-only, PD per fired): `2♦ → 3♣` **+5.18/+6.06** (×17/×16)
  — the diamond transfer's right-siding is the bundle's engine, mooting the
  N1h 3♦-invite lead; `3♦ → 3♣` +1.48/+2.27 (×31/×26); the cue→family rows
  all positive.  Costs: `3♣ → 2NT` (INV clubs riding the wide transfer)
  −0.15 NV / +1.14 vul ×40/×35 — a wash; `Pass → 3♣` (new weak transfers)
  plain-positive, PD-mixed (+1.94/−0.06 NV, +1.54/−0.79 vul) — the
  obstruction shape plain DD likes.
- **Mirror leak as predicted**: 36%/38% foreign (the ladder deletes the cue
  constraints), foreign PD sums −21/−29 — depressing the headline, so the
  ours-only figures are stronger (NV +182 plain / +215 PD over 180 boards;
  vul +171/+202 over 146).  Same shape as N1d/N1f; the `their_profile`
  split stays the structural fix.

Bookkeeping: default system byte-identical through both flips (smoke
`18aba5ce…` verified against clean HEAD by stash/pop before the run and
re-verified after the flip); golden cards and `alert-sites.txt`'s default
section unchanged, the `[their-landy]` fixture section re-blessed (cue
64→24, tko/spl 0→8, transfer 4→8, completion 32→40); `comp:landy-tko`/
`comp:landy-spl` recorded in `card.rs` as schema-inexpressible.  Replay:
`bba-gen --defense-2c-landy-bba false` is the pre-N1j stack arm,
`bba-decompose --landy-bba false` replays between-ships dumps.

### How it got here — exploration digest

Five measured rounds, all 2026-08-14; numbers in the ledger, probe files in
`ab-results/landy-*`.

1. **The first A/B lost all six cells** — not the idea, two leaks: opener's
   answers were unauthored (the floor phantom-completed each natural call as
   the gadget it replaced: Jacoby `2♥` 82% over the weak `2♦`, Puppet `3♦`
   85% over `3♣`), and the census had misread systems-on's minor transfers,
   which were *winning* the minor-partial boards. `landy_natural_answers`
   closed leak 1 and the base counter shipped `wash | win`.
2. **The UvU-style GF cue overlay (N1b) washed four times.** The `1♣ (2♣)`
   analogy held as *isomorphic, not identical* — expert counter-Landy
   structures (Cohen, Walker) independently reproduce the values-`X` +
   GF-minor-cues core, but the 15-17 captaincy re-spends the raise half; the
   minor-opening side of the skeleton is P7 in
   [competitive-book.md](../competitive-book.md). `probe-divergence` decomposed
   the wash into four effects with different signs: the weak `3♣` escape was
   the earner, the cues the losers — first a sub-game cue answer (missed
   slams), then the poached values double, then the fit forensic (5-2 raises
   at −10/−8 PD per fired; interference dropping mid-convention auctions to a
   floor with no forcing channel).
3. **N1c re-spent the rungs the decomposition named** — weak escape → `2NT`
   transfer, weak `3♦` deleted, natural `2NT` invite deleted, direct 3m →
   INV six-carders — and was the first arm to substantially pass the
   isolation gate (0.8% foreign boards vs N1b's 27%).
4. **N1d/e/f repaired the cue** (floor 10, doubleton-NT answers, interfered
   tails), and the stack went `win | win` pooled over two seeds — the whole
   five-arm, two-vul final round ran in 19 minutes off the enriched filter.

Lessons the next package inherits: **decompose a wash before theorising** —
none of this package's washes was one effect; a **single-seed negative is not
a design input** (a vul-PD −0.0010 that drove a day of worry did not
replicate); and an artificial call is not complete until both sides'
continuations *and the interfered tails* are authored — every loss this
package ever measured was an unauthored continuation, never the idea.


## N4 — measurement rounds v1–v6

### Measurement

`scripts/ab-2d-multi.sh`: `base` (natural leg) vs `multi`, both vuls,
`--filter-1nt` on both, 230.4k bd/arm/vul, plain + PD + sd. Doubling half
judged on plain DD; the re-keyed constructive calls on the standard gate.
`probe-divergence --gate-opener ours` before the headline. Verdict: see the
[ledger](../one-notrump-competitive.md#ledger).

### v1 — measured 2026-08-15: **LOSS on the owned boards, and the raw headline is the mirror leak again**

`ab-results/2d-multi`, SEED_BASE 1786786643, 230.4k bd/arm/vul, sha 1b621b5+dirty.
Raw headline: NV plain −0.0001 ±0.0012 / PD **+0.0035 ±0.0016**, vul plain
**+0.0017 ±0.0014** / PD **+0.0052 ±0.0018** — the ship row on its face.
`probe-divergence --gate-opener ours` **FAILS: 383 of 588 (65%) NV, 333 of 482
(69%) vul foreign** — their double of *our* `2♦` overcall read through our
now-alerted values double (N4b's leak, one call over). Priced by opener's side
(`--imps --jsonl`, per accepted board):

| vul | subset | n | plain/bd | PD/bd | plain/fired | PD/fired |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| NV | **ours** | 205 | **−0.00088 ±0.00075** | −0.00066 ±0.00102 | −0.99 | −0.75 |
| NV | theirs | 383 | +0.00077 ±0.00095 | +0.00419 ±0.00126 | +0.46 | +2.52 |
| vul | **ours** | 149 | **−0.00131 ±0.00081** | **−0.00159 ±0.00104** | −2.03 | −2.46 |
| vul | theirs | 333 | +0.00301 ±0.00118 | +0.00674 ±0.00150 | +2.08 | +4.67 |

Two mechanisms, both **unauthored seats**, not the calls themselves
(buckets = responder's first call, off → on, our-opened boards, NV/vul):

1. **`3NT` → `X`** (55/33 boards, plain **−3.8/−5.3 per board**, PD +0.9/+0.2):
   the game hand with one major open doubled to wait, and then *responder's
   second call* — the floor's — **sold out at the two level with 10+**
   (`1NT (2♦) X (2♥) - (-) -`, `… X (2♥) X (2♠) -`: 42 of 62 ended in `2♠`/`2♥`
   undoubled). The waiting double is only as good as the seat that has to
   act on the answer, and that seat reads their `2♦` as diamonds.
2. **pass → `2NT` relay** (77/58 boards, plain −0.2/−2.3, PD **−3.4/−6.1**):
   not the relay — **opener raised the weak `3♦` sign-off to `3NT`** on 45 of
   52 (NV) / 48 of 69 (vul) relay boards; `2NT - 3♣ - 3♦ -` had no book node
   and the floor bid game opposite a hand that had just shown ≤ 8. The
   unauthored doubled tail (`3♣ (X)` passed with five diamonds, −18) was 2
   boards.

Winners, small: the values double on hands the Optional gate passed
(`- 2♥ → X 2♥`, +0.05/+1.94 plain, +0.45/+2.61 PD), the blast on ex-doublers
(`X → 3NT`, +5.5/+10.0 plain), opener's trump double node (+0.9/−0.3,
PD +3.8/+1.0).

**v2 (built, running):** `3NT` = `points(10..)` unconditional (plain DD is the
arbiter and preferred the blast by 3.8–5.3 a board; PD called it a wash), so
the double is in practice the 8-9 hand; opener **passes** every relay
sign-off (`multi_signoff_pass`, all three relay paths × ♦/♥/♠); the doubled
relay tail authored (`2NT (X)`, `2NT (X) 3♣ -`, `2NT - 3♣ (X)`).

### v2 — measured 2026-08-15: **owned boards vul plain win, NV wash-to-PD-loss; two more floor seats named**

`ab-results/2d-multi-v2`, SEED_BASE 1786787534, same shape. Raw again leak-
inflated (349/528 NV, 310/471 vul foreign). Owned:

| vul | n | plain/bd | PD/bd | plain/fired | PD/fired |
| --- | ---: | ---: | ---: | ---: | ---: |
| NV | 179 | +0.00001 ±0.00059 | −0.00082 ±0.00079 | +0.02 | −1.05 |
| vul | 161 | **+0.00089 ±0.00070** | +0.00076 ±0.00087 | +1.27 | +1.09 |

The v1 mechanisms are gone: `3NT` blasts (`X → 3NT` +1.6/+1.6 plain), the
relay is wash-to-positive (`- 2♥ → 2NT` +0.5/+0.8 plain, −0.8/+0.4 PD),
and opener's trump-double node is the package's engine vul (`X 2♥ → X 2♥`
**+3.7 plain / +4.5 PD per board**, n=35). What is left, all NV, is again
seats the floor still owns after the double: **the floor pulls the penalty
doubles it cannot read** — `X (2♠) X (P) 3♥` (responder pulling opener's
double of 2♠ to 3♥, opener raising to 4♥), `X (2♠) - (-) X (P) 4♥` (opener
pulling responder's double), the overcaller's `2NT` heart relay cued as `3♠`;
`X 2♠` boards −59 plain / −114 PD on 14 boards. And after the relay
sign-off, their competition: `3♦ - - (3♠) 4♣ - 4♥` — responder correcting a
weak sign-off to a four-level phantom.

**v3 (built, running):** the double family's continuations authored — responder
after `X (2♥) - (-)` / `X (2♥) - (2♠)` / `X (2♠) - (-)` doubles with four of the
*resolved* major else passes (`multi_penalty_answer` again), sits over opener's
double, doubles the correction over it; opener sits for every responder
penalty double; the heart-relay `2NT` nodes and every relay sign-off's
competition (their X, their bid, their balance) fenced with passes. The
"consequent doubles are nominal penalty" structure the design named, now
authored to the seat that has to hold it — the floor could not.

### v3 — measured 2026-08-15: **owned plain win both vuls, PD wash — one PD-negative rung left**

`ab-results/2d-multi-v3`, SEED_BASE fresh, same shape; foreign 369/617 NV,
344/525 vul (raw NV plain +0.0023 ±0.0012 / PD +0.0052 ±0.0016; vul
+0.0043 ±0.0015 / +0.0070 ±0.0018; sd raw +0.0015/+0.0024 plain, +0.0039/+0.0049
PD — all leak-inflated). Owned:

| vul | n | plain/bd | PD/bd | plain/fired | PD/fired |
| --- | ---: | ---: | ---: | ---: | ---: |
| NV | 248 | **+0.00088 ±0.00071** | +0.00008 ±0.00083 | +0.82 | +0.07 |
| vul | 181 | **+0.00125 ±0.00080** | +0.00039 ±0.00092 | +1.60 | +0.50 |

Every authored seat now pays: relay `- 2♥ → 2NT` +0.66/+0.59 NV (vul
−0.06/−0.98), `- 2♠ → 2NT` **+2.77/+1.13** NV, +2.18/−0.18 vul; opener's trump
double `X 2♥ → X 2♥` +0.68/+1.19 NV, **+2.92/+2.42** vul (n=53/36); the new
8-9 doubles `- 2♥ → X 2♥` −0.51/−0.86 NV, +1.36/+1.14 vul. The one rung
negative on perfect defense is the **blind `3NT` blast on the ex-Optional
doublers** (`X 2♥ → 3NT`: +1.63/+1.56 plain, **−3.70/−4.31 PD**, n=27/16) —
the DD-fragile stopperless game; without it PD would read +118/+160 IMPs.
Plain-win + PD-wash is the artifact row, so:

**v4 (built, running):** direct `3NT` back to *both majors stopped*; the game
hand with a major open doubles, and its authored second call
(`multi_responder_rebid`, at every resolved node) bids `3NT` with a stopper
in the *named* suit, doubles with four trumps, else passes — the v1 idea with
the seat that killed it authored instead of floored.

### v4 — measured 2026-08-15: **owned `plain wash | PD win` on both vuls — the ship row** (seed 1; pooled verdict below)

`ab-results/2d-multi-v4`, seed 1. Foreign 392/737 NV, 328/561 vul (raw NV
+0.0013 ±0.0013 plain / +0.0064 ±0.0017 PD; vul +0.0035 ±0.0015 / +0.0077
±0.0018 — leak-inflated as ever). Owned:

| vul | n | plain/bd | PD/bd | plain/fired | PD/fired |
| --- | ---: | ---: | ---: | ---: | ---: |
| NV | 345 | −0.00005 ±0.00088 | **+0.00161 ±0.00106** | −0.04 | +1.08 |
| vul | 233 | +0.00052 ±0.00092 | **+0.00112 ±0.00108** | +0.51 | +1.10 |

The engine is the very bucket v1 lost: `3NT → X 2♥` — the game hand with a
major open doubles, hears the suit, and places — plain −0.72/+0.37 but
**PD +3.45/+4.79 per board** (n=65/43), against v1's −3.8/−5.3 plain when the
placing seat was the floor's. Opener's trump double `X 2♥ → X 2♥` −1.08/+1.55
plain, +0.66/+1.93 PD; the relay `- 2M → 2NT` positive on both scorers both
vuls (+1.27/+3.00 plain, +1.24/+1.50 PD NV; +0.83/+1.56, +0.23/−0.34 vul); the
8-9 doubles `- 2♥ → X 2♥` −0.96/−0.45 NV, +0.71/+1.58 vul. Nothing left with
a CI-clear negative sign.

**v4 pooled, three seeds** (`2d-multi-v4`, `-v4s2`, `-v4s3`; 691.2k bd/vul), owned:

| vul | n | plain/bd | PD/bd | plain/fired | PD/fired |
| --- | ---: | ---: | ---: | ---: | ---: |
| NV | 961 | **−0.00055 ±0.00050** | **+0.00083 ±0.00059** | −0.39 | +0.60 |
| vul | 663 | +0.00025 ±0.00052 | **+0.00084 ±0.00061** | +0.26 | +0.87 |

Vul is the ship row; NV is a PD win over a plain loss that just clears its
CI (seed 2 −0.00104 ±0.00085, seed 3 −0.00055 ±0.00086, seed 1 −0.00005).
Decomposed by what happens after our `X` and their `2M` (pooled, NV): the
**sell-out** — opener passes, overcaller passes, *responder passes* — is 309
boards at **plain −2.53 / PD +0.82** per board, and its cousins (`- 2♠ -`
−3.75, `X 2♠ -` −3.56) the same sign; every path that *acts* is plain-positive
(`X - -` responder's penalty double sat +2.21/+1.10, `- - X` +4.63/+0.31,
`- - 3NT` +4.20/+1.00). Plain DD wants the 8-9 hand to declare something; PD
is content to defend. **v5:** the rebid table gains a natural `2NT` invite
(`points(8..) & stopper_in(M)`, below `3NT`/`X`), opener answering from the
top of the range (`multi_invite_answer`, the uncontested `size_ask_accept_floor`).

### v5 — measured 2026-08-15 ×3 seeds: **REFUTED, reverted** (`2d-multi-v5`, `-v5s2`, `-v5s3`)

The invite bought thin games perfect defense refuses: `- - 2NT` PD −0.90 NV /
**−4.82 vul** per invite (plain +1.22 / −1.50), and the pooled owned verdict
fell to a four-way wash — NV plain −0.00048 ±0.00051 / PD +0.00010 ±0.00061,
vul −0.00005 ±0.00054 / +0.00009 ±0.00062. The DD-declarer artifact in one
rung; v4's PD win was the thing worth keeping. Code reverted to v4
(`multi_invite_answer` deleted; the rebid table's sell-out documented).

### v4 decomposed per call — the double is PD's best call and plain's whole loss

Owned boards, v4 pooled three seeds (691.2k bd/vul), by responder's first
call in the Multi arm (`/bd` = the call's contribution to the headline):

| call | vul | n | plain /fired | plain /bd | PD /fired | PD /bd |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| **X** | NV | 615 | −1.36 | **−0.00121 ±0.00044** | +0.60 | +0.00054 ±0.00049 |
| `2NT` relay | NV | 304 | +1.13 | +0.00050 ±0.00021 | +0.53 | +0.00023 |
| `3NT` | NV | 20 | +5.70 | +0.00016 | +2.40 | +0.00007 |
| `3♠`→♣ | NV | 22 | 0 | 0 | −0.18 | 0 |
| **X** | vul | 396 | −0.18 | −0.00011 ±0.00045 | +1.63 | **+0.00094 ±0.00049** |
| `2NT` relay | vul | 238 | +0.45 | +0.00015 | −0.37 | −0.00013 |
| `3NT` | vul | 16 | +6.00 | +0.00014 | −0.69 | −0.00002 |
| `3♠`→♣ | vul | 13 | +3.23 | +0.00006 | +2.46 | +0.00005 |

The double itself is fine — where it ends in a penalty pass (`2♥x`/`2♠x`) it
is +4.7 plain a board. The loss is **responder's rebid after their
pass-or-correct**, v4's `3NT (stopper) / X (four trumps) / pass`:

| seat | responder hand | n (NV) | plain | PD |
| --- | --- | ---: | ---: | ---: |
| `X (2♥) - (-) ?` passes | 8–9, 4–5 spades, no ♥ stopper | 109 | **−2.9** | −0.4 |
| same | 10–12, 2–3 spades, no ♥ stopper | 63 | **−3.2** | +0.8 |
| same | 8–9, ♥ stopper | 53 | +0.8 | +2.0 |
| `X (2♥) - (2♠) ?` / `X (2♥) X (2♠) ?` passes | 10–12, 2–3 hearts, no ♠ stopper | 68 | **−3.8** | 0.0 |
| same | 8–9, 2–3 hearts, no ♠ stopper | 33 | **−3.0** | **−1.7** |
| `X (2♠) - (-) ?` passes | 8–9 | 28 | −0.4 | +1.3 |

Two holes: no natural spade bid once hearts are resolved (opener's pass over
`2♥` already denied four hearts), and 10–12 without a stopper in the resolved
major sells out at the two-level holding 25–27 combined. Vulnerable the same
seats are plain-smaller and PD-positive (BBA's vulnerable `2♥` is worth
defending) except the spades-resolved 8–9 no-stopper hand, negative on both.

### v6 — BBA's own second-turn structure mimicked whole (**measured 2026-08-15 ×3 seeds: plain win / PD LOSS — the artifact row** — `2d-multi-v6`, `-v6s2`, `-v6s3`)

The user's call: mimic BBA's double *and what it bids after it*, with a pass
where BBA cues `3♦`. Probed at the seats BBA's advancer actually gives us
(`probe-bba-constraints --mode opener-d-x2h|opener-d-x2s|counter-d-x2h|
counter-d-x2h2s|counter-d-x2s`, plus the new `--mode custom --seat/--calls/
--filter-call/--filter-prefix` for one-off nodes; findings distilled in
[bba-multi-2d.md §3a](../ai-bidder/bba-multi-2d.md)):

- **Their X is takeout/Stayman-shaped, yes** — `hcp 5–17` (median 9), and at
  the unreachable `X -` node opener shows a four-card major, else cues `3♦`,
  never passes. But over the pass-or-correct **BBA's opener passes 92%**
  (`2NT` with a 17-count 6%, `2♠` with five 2%; it never doubles `2M`) and
  **the doubler describes at its second turn**:

  | after `X (2♥) - (-)` | share | BBA's hand |
  | --- | ---: | --- |
  | `3NT` | 29.5% | `hcp 9–15`, **no stopper gate** (3–4 hearts, 2–4 spades) |
  | Pass | 26.8% | `hcp 5–9` |
  | `X` | 12.6% | **exactly four spades, 1–2 hearts**, `hcp 6–17` — labelled "reopening double", i.e. takeout showing the other major |
  | `2NT` | 8.0% | `hcp 8–9`, natural invite |
  | `4NT` | 7.6% | `hcp 16–21`, quantitative |
  | `2♠` | 5.9% | five spades, `hcp 6–8` |
  | `3♠` | 5.2% | four spades, 2–3 hearts, `hcp 9–13` |
  | `3♣`/`3♦` | 1.5% each | 5+, `hcp 7–13` (median 8) |

  After `X (2♠) - (-)` the mirror (X = 4–5 hearts, 1–2 spades; no `3♥`/`2♥`
  analogue above 1%). After `X (2♥) - (2♠)` — the weak advancer's
  pass-or-correct corrected to spades — the double is **penalty** (spades
  3–5, median 4, `hcp 5–16`, 33%), `3NT` 9–15, `2NT` 8–10, the rest as above.
  BBA's opener over the takeout double is opaque (`2NT` 34% even holding four
  of the other major, `3m` with four, a penalty pass with 4+ of theirs, never
  the 4-4 fit).

The v6 table (`multi_responder_rebid(M, ran)`, per resolved major, `ran` =
the corrected-to-spades shape) mimics responder rung for rung — `4NT` 16+,
`2♠` five weak spades (hearts resolved), `X` = four of the other major and
≤2 of theirs (`comp:multi-takeout`; in the `ran` shape four spades and 7+,
`comp:multi-penalty`), `3♠` = four spades with heart length 9–13, **`3NT` =
`hcp 9–15` blind**, `3m` = 5+ and 7–8, `2NT` = 8–9, pass. First-turn `X` drops
to BBA's band, `hcp(6..)`, weighted *below* the natural `2M` and the relay so
weak 5+ suits still escape. Opener: over `X -` a four-card major (hearts
first) else **pass** (BBA's `3♦` cue replaced, `multi_pass_answer`); over
`X (2M)` the v4 four-trump double stays (its sat path measured +2.5/+1.4 NV,
+4.1/+1.9 vul, the one place v4 beat BBA's pass); over the takeout double
sit with four of theirs, bid the 4-4 fit, else a four-card minor, else `2NT`
(`multi_takeout_answer`); `3♠` → `4♠` with four else `3NT`; `2NT` → the Landy
invite answer; `4NT` → `6NT` with 17; `2♠`/`3m` → pass; the `ran` double
sat. Everything past those is the floor's. Unit tests re-pinned
(`multi_double_family_continuations_are_book_nodes` walks the whole family).

Run: v4's base arms are re-used by symlink (the default system is
byte-identical, smoke `18aba5ce…` re-verified after the edit — the same code
on the same seeds, not a stale control), only the Multi arm regenerates per
seed, and each seed is priced both against base and **paired against v4's
Multi arm** (`probe-divergence multi-v6 multi-v4`).


**v6 measured** (three seeds, owned boards, 691.2k bd/vul): **plain win /
PD loss** — the DD-declarer-artifact row, both vuls:

| v6 vs | vul | n | plain /bd | PD /bd |
| --- | --- | ---: | ---: | ---: |
| base | NV | 1300 | **+0.00224 ±0.00054** | −0.00062 ±0.00069 |
| base | vul | 963 | **+0.00110 ±0.00059** | **−0.00163 ±0.00075** |
| v4 (paired) | NV | 1098 | **+0.00287 ±0.00051** | **−0.00139 ±0.00064** |
| v4 (paired) | vul | 763 | +0.00097 ±0.00054 | **−0.00248 ±0.00069** |

Per rung, v6 vs v4 (plain / PD per fired, NV then vul):

| rung | n | NV | vul | verdict |
| --- | ---: | --- | --- | --- |
| takeout `X` after `X (2♥) - (-)` | 116 / 82 | **+2.44 / +1.58** | **+2.22 / +0.62** | the real gain — both scorers, both vuls |
| takeout `X` after `X (2♠) - (-)` | 41 / 42 | +1.83 / +0.29 | +0.33 / −1.12 | wash |
| blind `3NT` after `X (2♥) - (-)` | 166 / 99 | +1.83 / **−2.45** | +0.16 / **−4.64** | artifact |
| blind `3NT` after `X (2♥) X (2♠)` | 100 / 67 | +2.20 / **−2.46** | +0.73 / **−4.15** | artifact |
| blind `3NT` after `X (2♥) - (2♠)` | 116 / 73 | +3.95 / +0.09 | +3.08 / −1.79 | artifact-leaning |
| `2NT` invite (all seats) | 128 / 84 | +0.3 / **−3.2** | −1.9 / **−6.3** | v5's refutation, again |
| `3♠` try | 23 / 17 | +2.00 / −2.30 | +1.53 / −3.59 | artifact |
| `3♣` / `3♦` natural | 38 / 26 | +2.5 / +0.7 | +1.7 / −0.9 | wash, tiny |
| first-turn re-order (`2NT` relay / `2M` before `X`) | 212 / 161 | +1.6 / −0.2 | +0.8 / −1.0 | wash |

So BBA's *double* is right and BBA's *game bids* are what double-dummy
likes and perfect defense refuses — the third time this package has measured
that (v2/v3 blind 3NT, v5 invite). **v7** keeps BBA's structure minus those
rungs: `X` = takeout of the resolved major (four of the other, ≤2 of theirs;
penalty spade length in the `ran` shape), `2♠` five weak spades, `4NT` 16+,
`3NT` back to v4's `points(10..) & stopper_in(M)`, pass the rest; first-turn
`X` stays BBA's `hcp(6..)`; opener's takeout answer, quant answer and the
sits stay, the invite/try/3m answers go with their calls.

## N4b — the `(2♦)` diamond penalty double (**built 2026-08-15, sweeping**)

The cheap half of N4: it needs no disclosure and no Multi package, because a
length+quality penalty double of `2♦` is sound *either* way — over a natural `2♦`
it is a textbook penalty double, and over the Multi they cannot sit on it.

### What was wrong

Responder's double of `(2♦)` was `len(♦, 2..=3) & hcp(8..)` —
`DoubleStyle::Optional` via `responder_double` ([rubensohl.rs](../../src/bidding/american/competition/rubensohl.rs)),
a *cooperative* double asking opener to decide. Against the reference opponent
that gate names a suit nobody holds: BBA's `2♦` is a Multi, a single-suited
six-card major of ≈12-15 ([bba-multi-2d.md](../ai-bidder/bba-multi-2d.md)). Opener's
answer, `opener_cooperates_optional(♦)`, was diamond-keyed too.

And the structure leaves responder **no way to bid diamonds below `3NT`** — `3♦`
is the Jacoby transfer to hearts. The double is the only channel there is.

### The knob

`competition.two_diamond_double: Option<(min_len, min_suit_hcp, hcp_floor)>`,
default `None` (byte-identical: smoke `18aba5ce…` = HEAD). Armed, responder's `X`
becomes `len(♦, min..) & suit_hcp(♦, q..) & hcp(floor..)` and opener sits
(`opener_leaves_in_penalty_double`). Harness: `bba-gen --ns-2d-double LEN:SUITHCP:HCP`.

### The alert is load-bearing — this is the N1g trap again

The first build left the double **unalerted**, on the theory that a natural call
needs no alert and the rule's own projection would carry the length. Measured on
`probe-call-reading "1N (2D) X (P)"`, it read **`points 8..` with every suit ⊤,
armed and unarmed identically** — `project_authored` decodes *alerted calls only*.
Opener would have competed over their runout blind to the suit it was just told
about, which is exactly the phantom the N1g wiring existed to kill. With
`TWO_DIAMOND_PENALTY` attached it reads `points 9.., ♦ 5..13`. Regression test:
`the_two_diamond_double_reads_as_diamonds`.

**Rule this generalises to: a gate is not a reading. If a knob's whole value is
information the floor needs, probe the reading before measuring anything.**

### Trigger shape (20k `--filter-1nt` boards, gate `5:4:9`)

| quantity | count |
| --- | --- |
| our `1NT (2♦)` lanes | 393 (2.0% of filtered boards) |
| we double | 14 |
| **they pass our double** | **6 of 14 (43%)** |
| boards diverging from baseline | 0.70% |

~~The 43% is the load-bearing number: the probe's advancer census
(`2♠` 67% / `2♥` 33%, no Pass) is taken over `1NT - 2♦ -`, and does **not**
carry over to `1NT (2♦) X` — they *do* sit. So the penalty is genuinely
collectible and plain DD can see it.~~ **Retracted 2026-08-15 ([§N4](../one-notrump-competitive.md#n4--their-2-as-a-multi-shipped-2026-08-15--v7-seven-rounds-default-on-vs-bba-via-the-census)):** the
14 fires were counted over both lanes, and the six "sits" were the *foreign*
one — BBA's responder doubling **our** `2♦` overcall and *our* advancer
passing. Split by opener's side, BBA's advancer passed our double **0 of 141**
times in this arm (0 of 339 in `base`), and `--mode advance-x` confirms 0.0%.
The Multi overcaller's side never sits; the penalty was never collectible.

### The sweep — **NULL, and the headline is a leak** (2026-08-15)

`scripts/ab-2d-double.sh`, centred on `5:0:9` with one axis moving at a time
(length 4/5/6, floor 8/9/11, suit quality 0/4/6) — eight arms, two vuls, 230.4k
bd/arm/vul, `--filter-1nt` on every arm, SEED_BASE 1786733434, sha 392f7d2+dirty.

**Raw headline: all 28 cells CI-clear positive**, plain +0.0016…+0.0048/bd, PD
+0.0037…+0.0086/bd, sd agreeing in sign. Four to eight times N1g's shipped
effect — which is what made it obviously wrong.

Two tells, before any celebration:

1. **No axis separates.** `len4` and `hcp11` sit at opposite ends of two
   different dials and are the two *best* arms. Divergence counts barely move
   with the gate (791 fired at ♦4+, 784 at ♦6+) — a gate that does nothing to
   the fire rate is not what is being measured.
2. **`probe-divergence --gate-opener ours` FAILS: 652 of 768 divergent boards
   (84.9%) were opened by *them*** — boards where our `1NT (2♦)` node cannot
   fire at all.

The leak is the documented `their_profile` mirror fallback
([read.rs](../../src/bidding/inference/read.rs)). On a board *they* open 1NT and we
overcall `2♦`, their partner's double is read through **our** `two_diamond_double`
agreement — so it now says "5+ diamonds, 9+" and our advance changes. Almost all
the measured IMPs come from there.

Priced on the boards the package actually owns (per-arm, `opener_ours` only):

| arm | vul | n ours | plain/bd | PD/bd | plain/fired |
| --- | --- | ---: | ---: | ---: | ---: |
| len4 | none | 157 | −0.00061 ±0.00063 | −0.00038 ±0.00074 | −0.90 |
| len5 | none | 116 | +0.00024 ±0.00049 | −0.00020 ±0.00061 | +0.48 |
| len6 | none | 97 | +0.00039 ±0.00044 | −0.00046 ±0.00055 | +0.92 |
| hcp8 | none | 152 | +0.00009 ±0.00055 | −0.00036 ±0.00072 | +0.14 |
| hcp11 | none | 91 | +0.00043 ±0.00042 | −0.00032 ±0.00052 | +1.09 |
| qual6 | none | 91 | +0.00036 ±0.00041 | −0.00036 ±0.00051 | +0.90 |

(vul is the same shape, uniformly weaker; 80–91% foreign in every cell.)

**Verdict: a wash on its own domain — not one CI-clear cell in 28.** Plain leans
faintly positive, PD faintly negative, at n=62–157 owned boards per cell. Stays
**opt-in**, default byte-identical. The only signal, weak and uncertain: **tighter
is better** — `hcp11`, `len6`, `qual6` lead on owned plain/fired, and `len4` is
the only arm negative on both scorers. If this is resumed, start at `6:6:11`, not
at the centre, and buy power: three seeds at 460.8k would take the owned n from
~100 to ~600/cell.

### The real find is the leak

Reading *a defender's double of a `2♦` overcall* as diamonds-and-values is worth
**+0.0016…+0.0048 plain / up to +0.0086 PD** — far more than the convention it
leaked out of. That is a fact about a call **they** make, so it belongs on the
`their` disclosure channel with its own reader and its own A/B (the N1/N1g split),
not as a side effect of our agreement. Logged as a candidate, not a result: it has
never been measured as itself.

**Rule this generalises to: run `--gate-opener ours` before reading the
headline, not after.** A CI-clear win several times the size of anything else in
the campaign is evidence of a leak until the gate says otherwise.

### Orphaning — checked, not the story

This is a *replacement*: every 2-3-diamond eight-count that doubles today stops
doubling, and its outs are shut (`2♥`/`2♠` want a five-card major, the `2NT` relay
wants a long suit, `3♦` is the transfer), so it passes. 76.8% of divergences are
"passed where the baseline bid". That is the orphaning, and on the owned subset it
prices to roughly nothing — it is neither the win nor a hidden loss.
