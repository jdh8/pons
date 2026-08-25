# Competitive 1NT — closed package history

> **Archived 2026-08-19, extended 2026-08-21.** This file holds the closed N1
> Landy campaign, N4's superseded measurement rounds v1–v6, the N4b
> diamond-double sweep, the superseded **census snapshots**, N3's eight
> **measurement rounds**, N2's **pre-fix census**, and the 2026-08-16 memory
> compaction notes. The live opponent model, the current census, the shipped
> package tables, measurement discipline, the open queue, and the ledger remain
> in [one-notrump-competitive.md](../one-notrump-competitive.md).
>
> **What lives here is verdicts** — A/B results pinned to a sha and a seed, and
> the snapshots each package was chosen against. They are permanent facts and
> are never re-run for freshness; only *state* numbers (the census, the score
> against a lane) are refreshed, and those live in the live doc.

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
   (`1NT (2♦) X (2♥) - - -`, `… X (2♥) X (2♠) -`: 42 of 62 ended in `2♠`/`2♥`
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
doubles it cannot read** — `X (2♠) X - 3♥` (responder pulling opener's
double of 2♠ to 3♥, opener raising to 4♥), `X (2♠) - - X - 4♥` (opener
pulling responder's double), the overcaller's `2NT` heart relay cued as `3♠`;
`X 2♠` boards −59 plain / −114 PD on 14 boards. And after the relay
sign-off, their competition: `3♦ - - (3♠) 4♣ - 4♥` — responder correcting a
weak sign-off to a four-level phantom.

**v3 (built, running):** the double family's continuations authored — responder
after `X (2♥) - -` / `X (2♥) - (2♠)` / `X (2♠) - -` doubles with four of the
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
| `X (2♥) - - ?` passes | 8–9, 4–5 spades, no ♥ stopper | 109 | **−2.9** | −0.4 |
| same | 10–12, 2–3 spades, no ♥ stopper | 63 | **−3.2** | +0.8 |
| same | 8–9, ♥ stopper | 53 | +0.8 | +2.0 |
| `X (2♥) - (2♠) ?` / `X (2♥) X (2♠) ?` passes | 10–12, 2–3 hearts, no ♠ stopper | 68 | **−3.8** | 0.0 |
| same | 8–9, 2–3 hearts, no ♠ stopper | 33 | **−3.0** | **−1.7** |
| `X (2♠) - - ?` passes | 8–9 | 28 | −0.4 | +1.3 |

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

  | after `X (2♥) - -` | share | BBA's hand |
  | --- | ---: | --- |
  | `3NT` | 29.5% | `hcp 9–15`, **no stopper gate** (3–4 hearts, 2–4 spades) |
  | Pass | 26.8% | `hcp 5–9` |
  | `X` | 12.6% | **exactly four spades, 1–2 hearts**, `hcp 6–17` — labelled "reopening double", i.e. takeout showing the other major |
  | `2NT` | 8.0% | `hcp 8–9`, natural invite |
  | `4NT` | 7.6% | `hcp 16–21`, quantitative |
  | `2♠` | 5.9% | five spades, `hcp 6–8` |
  | `3♠` | 5.2% | four spades, 2–3 hearts, `hcp 9–13` |
  | `3♣`/`3♦` | 1.5% each | 5+, `hcp 7–13` (median 8) |

  After `X (2♠) - -` the mirror (X = 4–5 hearts, 1–2 spades; no `3♥`/`2♥`
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
| takeout `X` after `X (2♥) - -` | 116 / 82 | **+2.44 / +1.58** | **+2.22 / +0.62** | the real gain — both scorers, both vuls |
| takeout `X` after `X (2♠) - -` | 41 / 42 | +1.83 / +0.29 | +0.33 / −1.12 | wash |
| blind `3NT` after `X (2♥) - -` | 166 / 99 | +1.83 / **−2.45** | +0.16 / **−4.64** | artifact |
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
`probe-call-reading "1N (2D) X -"`, it read **`points 8..` with every suit ⊤,
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

## Census history

Two superseded snapshots of the whole-lane census, kept because they are what
each package was chosen against. The **live** census — refreshed per re-anchor
— is in
[one-notrump-competitive.md §The census](../one-notrump-competitive.md#the-census--what-each-interference-call-actually-costs).
Census rows rank; they never isolate, and none of them is a verdict.

### 2026-08-18 pre-N3 baseline, anchor `2026-08-17-53a3c254`, seed 1783375064, 204,800 boards/vul

We open 1NT on **6.5%/6.7%** of boards; RHO contests **12.4%/10.4%** of those
(NV/vul) — so a contested 1NT is **0.80%/0.69% of all boards**.

The three-level suits are split per RHO suit since 2026-08-18 (the N3
deliverable); `4+` is `3NT` and everything above it, still one floor-only bucket.

| RHO | boards (NV+vul) | plain total | plain/bd | PD/bd NV | PD/bd vul |
| --- | --- | --- | --- | --- | --- |
| `2♦` Multi | 794 | −245 | −0.31 | +0.15 | +0.54 |
| `2♠` Muiderberg | 430 | −219 | −0.51 | −0.16 | +0.36 |
| `2♣` Landy | 551 | −213 | −0.39 | −0.10 | +0.42 |
| **`3♣` preempt** | 100 | **−192** | **−1.92** | **−1.78** | **−0.62** |
| `X` Woolsey | 364 | −183 | −0.50 | +0.51 | +0.74 |
| **`4+`** (`3NT` and up) | 43 | −89 | −2.07 | −1.33 | −1.74 |
| `2♥` Muiderberg | 393 | −77 | −0.20 | +0.08 | +1.07 |
| **`3♥` preempt** | 85 | −75 | −0.88 | +0.13 | −1.70 |
| **`3♦` preempt** | 89 | −43 | −0.48 | +0.53 | −0.23 |
| **`3♠` preempt** | 88 | −35 | −0.40 | +0.50 | +0.89 |
| `2NT` unusual | 118 | +5 | +0.04 | −0.23 | +0.48 |
| **all contested** | 3055 | −1366 | −0.61 / −0.26 | +0.01 | **+0.43** |
| **uncontested 1NT** | 23868 | — | **+0.13 / +0.01** | — | — |

At this pre-ship snapshot the four three-level suits are 362 boards and −345
plain between them — the family was the top loser, and `3♣` alone out-cost
every two-level call per board by a factor of three. N3's post-ship fresh-seed
census is recorded in [§N3](#post-ship-fresh-seed-anchor-check-2026-08-18).

**Three findings.**

1. **The lane's whole headroom is ~0.004 IMPs/bd.** Contested costs
   −0.74 NV / −0.27 vul relative to *uncontested*, on 0.80%/0.69% of boards.
   Nothing here closes an anchor gap; this is hygiene and disaster removal at
   the standard ship gate, as scoped.
2. **Contested 1NT is above the instinct anchor's board average**, not a leak —
   −0.61/−0.26 against −0.90/−1.09. The 1NT opening is one of our better boards
   even when contested.
3. **The pre-N3 three-level lane is where both scorers lose.** `3♣` is −1.92
   plain/bd with PD negative at both vulnerabilities, `4+` worse per board on
   43 boards, and `3♥` swings PD −1.70 vul; only `3♠` is PD-positive on both.
   With the shipped Landy package present, `2♣` is −0.39 plain pooled and
   −0.10/+0.42 PD. `X` remains fine (−0.28 plain vul, PD +0.74), and `2♦` is
   mild (PD +0.15/+0.54). N3 authors the four three-level suits; `4+` stays
   floor-only, and inside it **`(4♥)` alone is −118 plain / −126 PD** (the
   worst-board dump's own tally; the rest of `4+` nets positive) — but the
   floor offers no `X` over `(4x)` at all, so see [§N3's flagged list](../one-notrump-competitive.md#flagged-not-fixed-floor-defects-reversible-defaults-proposed).

### Historical mechanism — why `2♣` lost before N1 shipped

The analysis below is from the starting `2026-08-12-ea2cde9-dirty` snapshot,
where `2♣` carried the largest loss (−406 plain IMPs, −0.74/bd). It motivated
N1; the current census above includes that package's shipped repairs.

Before N1, over their `2♣` we played a **systems-on rebase**
([lebensohl.rs:388-405](../../src/bidding/american/competition/lebensohl.rs)):
their `2♣` was stripped to a Pass and our whole uncontested response structure
went live, with `X` transplanted onto the stolen `2♣` Stayman
([lebensohl.rs:416-425](../../src/bidding/american/competition/lebensohl.rs)).
Against a *natural* club overcall that is sound and standard — `2♣` is the one
overcall that costs no space.

Against **Landy** it was actively bad. The worst boards showed the structure
firing into a hand that had just shown both majors:

```text
us:  - 1NT 2♣ 2NT - 3♦ 3♠ 4♦ - 4♠ X - - -      [−18 IMPs]
us:  1NT 2♣ 2♦ 3♣ X 3♠ - 4♠ X - - -            [−10 IMPs]
us:  1NT 2♣ 2NT 3♥ X - 3NT - - -               [−10 IMPs]
```

`2♦` is a Jacoby transfer **to hearts** — one of the two suits they hold. `2NT`
and `2♠` are the minor transfers, pure constructive asks that hand them a free
run at their fit. `X` asks for a four-card major against a hand holding both.
Two of the eight worst boards end in `4♠` doubled.

## N3 — measurement rounds

Their `(3♣)`–`(3♠)` preempt of our 1NT: the pre-ship decomposition, the
post-ship anchor check, and the eight measured rounds that ran from the
2026-08-18 ship to the 2026-08-21 close. The shipped tables, the disclosure
note and the surviving residue stay live in
[§N3](../one-notrump-competitive.md#n3--their-33-preempt-of-our-1nt-shipped-default-on-2026-08-18);
what follows is the evidence, at the sha and seed each round was run at.

### The pre-ship census, decomposed (anchor `2026-08-17-53a3c254`, 204,800 bd/vul)

The `3+` bucket split is now the probe's own (`probe-1nt-interference` labels
three-level suits per suit since 2026-08-18), so the table in §census above is
the deliverable; the worst cells per RHO suit, from the `--show 400` dumps:

| RHO | bd | plain | PD | worst cells (RHO × our call) |
| --- | ---: | ---: | ---: | --- |
| `3♣` | 100 | −192 | −120 | `3♠` 25 bd −86 (opener passes / 3NT over a 6-carder), Pass 41 bd −61 (4441 9–11 with no call), `3♦` 9 bd −38 |
| `3♥` | 85 | −75 | −73 | `X` 27 bd −23 / **−74 PD** (X on 6–7 HCP, opener `4♠`), `3NT` 11 bd −23 (singleton in their suit) |
| `3♦` | 89 | −43 | +14 | `3♥` 19 bd −65 (`3♥ - - -` passed out on 10–11 HCP), `X` 6 bd −52 (6–8 HCP) |
| `3♠` | 88 | −35 | +62 | Pass 39 bd −65 / +36; floor blasts `6♣`/`5♦` on 8–11 HCP; `X` +57, `4♥` +58 (the winners) |

### Post-ship fresh-seed anchor check (2026-08-18)

The shipping arms in `ab-results/anchor-confirm/2026-08-18-9cfb464b`, fresh seed
`1787064872`, 204,800 boards/vulnerability, replay 100.00% with 0 mismatches.
At the shipped defaults (responses on, private `3NT` stopper gate off,
`(3♣)` transfers off), the N3 buckets are:

| RHO | bd | plain | PD | plain/bd | PD/bd |
| --- | ---: | ---: | ---: | ---: | ---: |
| `3♣` | 140 | −170 | −163 | −1.21 | −1.16 |
| `3♥` | 105 | −11 | +45 | −0.10 | +0.43 |
| `3♦` | 93 | +29 | +48 | +0.31 | +0.52 |
| `3♠` | 72 | −121 | −116 | −1.68 | −1.61 |
| **all four** | **410** | **−273** | **−186** | **−0.67** | **−0.45** |

This is an attribution check, not another treatment A/B: the swing is the
whole board, the mirrored table is present, and the seed differs from the
pre-ship snapshot. In particular, `3♠` moved from PD-positive on the series
seed to −1.44/−1.76 PD per board NV/vul here, while the isolated package A/B
was positive on both scorers. Do not subtract the two anchor totals to estimate
N3's value; the owned `stop ↔ base` A/B below remains the causal ship evidence.

### Measurement — the ship row (2026-08-18)

`scripts/ab-nt-high-overcall.sh`, `SEED_BASE=1787055415`, sha `69cd39a1`+dirty,
230,400 bd/arm/vul, `--filter-1nt` on every arm. Three arms: `base` (knob off),
`stop` (on, `direct_3nt_stopper` as shipped), `nostop` (on, the shared stopper
bit dropped).

**The package (`stop ↔ base`) — owned boards** (`probe-divergence`, split on
`opener_ours`):

| vul | fired | plain/bd | PD/bd | plain/fired | PD/fired |
| --- | ---: | --- | --- | ---: | ---: |
| none | 435 (0.19%) | **+0.00208 ±0.00126** | +0.00079 ±0.00145 | +1.103 | +0.416 |
| both | 460 (0.20%) | **+0.00293 ±0.00160** | +0.00180 ±0.00182 | +1.470 | +0.900 |

Single-dummy leads (whole arm, 16 worlds): plain **+0.0019 ±0.0013** NV /
**+0.0028 ±0.0016** vul, PD +0.0008 ±0.0014 / +0.0015 ±0.0018. **Sixteen
readings, no negative cell.** Plain is CI-clear on both vuls, perfect defense
keeps 38%/61% of it with the same sign — it does not *erase* the win, which is
what the decision table's artifact row is about, and this package's added double
is a **takeout** double opener always answers, not a penalty double.

**Isolation gate: 16 NV / 12 vul foreign boards** (`--gate-opener ours`), which
is a hard fail and a small one — 3.5% / 2.5%. The mechanism is worth recording
because it is *not* the mirror-read leak the other packages hit. The classifier
is clean: over `1♠ 1NT 3♠` — our 1NT an **overcall**, not an opening — the node
does not fire and the floor answers, exactly as authored. The **reader** does
fire: `1♠ 1NT 3♠ 4♣ -` reads partner as `♣ 5.., points 10.., ♥ ..3` from this
table's rule, because the inference walk keys a made call from the caller's own
`1NT` while `classify` keys from the auction's start. Priced: NV foreign is
**−1 plain / +4 PD IMPs on 16 boards** (noise), vul foreign is +47/+43 on 12
(+3.9/fired, ~6% of the plain total). The owned figures above are the verdict
either way, and they stay CI-clear. The read is not obviously *wrong* either —
our 1NT overcall is 15–18 balanced and partner's `4♣` over their `3♠` really is
a long minor — but the scope mismatch belongs in
[authored-reading-handoff.md](../authored-reading-handoff.md)'s inventory.

### `stop` vs `nostop` — why the shared stopper bit was not flipped

`nostop ↔ stop` looks like a win on plain (NV **+0.00067 ±0.00062**, vul
+0.00040 ±0.00079) and a wash on PD. It is **two lanes summed**, and they
disagree — `--gate-opener ours` fails at 44/121 NV and 39/116 vul, and the
foreign boards are all `2M X - 3NT`: `direct_3nt_stopper` also governs
**advancing partner's takeout double of a weak two** (`american/defense.rs`
reuses the Lebensohl builders verbatim). Split:

| subset | NV plain/fired | NV PD/fired | vul plain/fired | vul PD/fired |
| --- | ---: | ---: | ---: | ---: |
| our 1NT opened (this lane) | **+2.195** | +0.662 | **+1.623** | +0.377 |
| everything else (the advance lane) | −0.318 | **−1.227** | −0.846 | **−1.923** |

So this lane wants no gate and the other lane wants it kept — which is why the
three-level table got its **own** bit, `competition.nt_high_overcall_3nt_stopper`,
rather than a flip of the shared one.

### Round 2 — the private bit **SHIPPED OFF**, the `(3♣)` transfers stay opt-in (2026-08-18)

Two increments over the shipped default, each against the reused `stop` arm —
whose boards were checked byte-identical to a default-flag regeneration before
reuse (only the recorded `gen_args` metadata differs). Two seeds, 1787055415 and
1787060609, 230,400 bd/arm/vul each.

**`nogate` — `nt_high_overcall_3nt_stopper false`, SHIPPED default-off.**
Pooled over both seeds (460,800 bd/vul):

| vul | fired | plain/bd | PD/bd | plain/fired | PD/fired |
| --- | ---: | --- | --- | ---: | ---: |
| none | 127 | **+0.00065 ±0.00036** | **+0.00043 ±0.00041** | +2.370 | +1.543 |
| both | 145 | **+0.00052 ±0.00047** | +0.00016 ±0.00053 | +1.648 | +0.510 |

Three of four DD cells CI-clear, the fourth wash-positive; single-dummy leads
positive on all eight per-seed cells (+0.0007…+0.0011 plain, +0.0005…+0.0006 PD),
five of them CI-clear. **`probe-divergence --gate-opener ours` passes at 0 foreign
on all four seed × vulnerability cells** — the campaign's third clean gate, and
exactly what the private bit was for. The size matches round 1's prediction for
this lane (+2.20/+1.62 plain per fired) to within noise.

Note `smoke-default` does **not** move on this flip (`39ca60a2…` unchanged): the
lane fires on 0.03% of `--filter-1nt` boards and the smoke set is unfiltered, so
zero hits in 20,000 auctions is the expected count. The A/B is the only witness
here; a byte-identity smoke is not evidence of inertness at this firing rate.

**`xfer` — the `(3♣)` transfers, measured WASH across two seeds, stays opt-in.**
Owned boards (6–8 foreign per cell, the same reader-scope leak, sign-flipping
between seeds):

| vul | fired | plain/bd | PD/bd | plain/fired | PD/fired |
| --- | ---: | --- | --- | ---: | ---: |
| none | 174 | +0.00002 ±0.00026 | +0.00007 ±0.00029 | +0.057 | +0.172 |
| both | 172 | +0.00007 ±0.00034 | +0.00009 ±0.00037 | +0.198 | +0.244 |

All four pooled cells positive and all four an order of magnitude inside their
CI. Seed 1 looked like a win at vul (plain +0.0003, PD +0.0004, PD > plain — the
right-siding signature); **seed 2 reversed it** (plain −0.0004, PD −0.0005), and
the pool is flat. That is the decision table's `wash | wash, a convention
trialled against natural` row: **stays opt-in**, default off, finished code with
its measurement paid.

Both `xfer` arms were measured against a **gated** `3NT` baseline, since they
ran before the `nogate` flip. Round 3 below pays that fresh-baseline caveat.

### Round 3 — top-step minor symmetry, still opt-in (2026-08-19)

The owed fresh-baseline run makes `1NT (3♣) 3♠` exactly the minor-swapped
twin of `1NT (2♦) 3♠`: responder now shows **6+♦** (not 5+), and opener
bids `3NT` with a club stopper, otherwise `5♦` (replacing the old
`3NT`/`4♦` table). Responder's club stopper instead selects direct `3NT`, as
in the `(2♦)` tree. The major transfers are unchanged and still share
`rubensohl::transfer_completion`: `4M` with three-card support, otherwise
`3NT`, including the doubled-transfer tail.

Fresh seeds `1787072350` / `1787073219`, sha `4740bcc3`+dirty, 230,400
boards/arm/vulnerability/seed, `--filter-1nt`; new `xfer` versus the current
shipped `stop` baseline (`nt_high_overcall_3nt_stopper false`). Owned boards:

| vul | fired | plain/bd | PD/bd | plain/fired | PD/fired |
| --- | ---: | --- | --- | ---: | ---: |
| none | 180 | +0.00008 ±0.00027 | +0.00008 ±0.00030 | +0.200 | +0.200 |
| both | 176 | +0.00019 ±0.00033 | +0.00020 ±0.00037 | +0.500 | +0.528 |

Every cell leans positive and every CI contains zero: still **wash | wash**.
The raw SD pair also leans positive in all four pooled cells, but the exact
top-step `3♠`→♦ branch fired on only 3 NV + 2 vulnerable owned boards. The
isolation gate found 6/4 and 8/9 foreign divergences by seed (the known
reader-scope leak); they are excluded above. This is still a convention
trialled against natural, so a wash keeps `nt_3c_transfers` opt-in/default-off.
The shipped system is byte-identical: `smoke-default --count 20000 --seed 1`
stays `39ca60a251e03e558cfe44659b44ae45b1fe296d806e90cb3ed1cc9338bf72cd`.

### BBA-style double continuation — refuted (2026-08-19)

A temporary experimental arm kept our responder's existing takeout-double
constraint fixed and changed only the continuation. Opener showed a four-card
major at the cheapest level; with none, it copied BBA's `3♦` over `(3♣)` / `4♦`
otherwise, and responder placed. This isolated the continuation from BBA's
different direct-double ranges.

Fresh seed `1787121438`, sha `e6819181`+dirty, 230,400 filtered boards per arm
and vulnerability (the temporary `ROUND=3` arm in
`scripts/ab-nt-high-overcall.sh`, since removed):

| vul | fired | plain/bd | PD/bd | sd plain/bd | sd-PD/bd |
| --- | ---: | ---: | ---: | ---: | ---: |
| none | 109 | **−0.0012 ±0.0006** | **−0.0012 ±0.0008** | **−0.0012 ±0.0006** | **−0.0013 ±0.0007** |
| both | 112 | **−0.0017 ±0.0008** | **−0.0015 ±0.0010** | **−0.0021 ±0.0009** | **−0.0020 ±0.0010** |

Every cell is negative and all eight CIs exclude zero. Isolation is clean:
221/221 divergences were on boards we opened. The mechanism is one row:
`4♦ ← 3NT` lost **−310/−348 plain/PD** over 55 NV boards and **−420/−429**
over 57 vulnerable boards. The candidate missed game where the baseline made
one on 42/109 and 50/112 divergences. By overcall suit, only `(3♣)` avoided a
replicated loss (NV +26/+43, vulnerable −11/+3 raw IMPs); `(3♦)`, `(3♥)`, and
`(3♠)` were negative on both scorers and vulnerabilities. The anchor's bad
`(3♠) X` attribution therefore did not identify `3NT` as a causal leak. The
arm, knob, test, and harness were removed after measurement; do not retry the
whole continuation.

### Round 4 — the answer tables' cross-call weight ties (2026-08-19)

**Pre-pinned before the run** (N1j precedent; rationale = structural
alignment, not an expected gain).

*The defect.* All three of opener's answer tables price the two majors' rows at
one weight — `nt_answer_double`'s `4M@150` / `3M@140` / `3M@30` / `4M@25`,
`nt_answer_forcing_suit`'s minor arm `3M@140`, and `nt_answer_forcing_minor`'s
`4M@130`. Production keeps the *first strict* maximum in call-encoding order,
so on a cross-call tie the **encoding** decides and hearts always wins: opener
with four hearts and five spades answers the takeout double `3♥`. The same bug
class was fixed on the responder side at ship ("+ rank is load-bearing"), and
`weight_tie_report` never saw it — that invariant only meters ties on the
*same* call. The test helper `best_call_with` used `max_by`, which keeps the
*last* maximum and so resolved ties the opposite way from production, hiding
the defect from any pinned test.

*The repair.* Each major's rows carry `at_least_as_long(major, rival)` whenever
their overcall leaves both majors live; with only one live major there is no
rival and no guard. A genuine 4-4 still fires both rows and still answers in
hearts (byte-identical), a 5-4 now answers in its five-carder. `best_call_with`
now reduces with a strict `>`, matching production, and
`the_double_answer_picks_the_longer_major` pins all four cases.

*The gate, pinned before reading any number.* Both arms under
`--filter-preempt`, fresh `SEED_BASE`, arms sequential, no rebuild in flight,
`fix` versus `base` at 716,800 boards per arm per vulnerability. **Ship iff no
CI-clear negative cell across {NV, vul} × {plain, PD}.** Any CI-clear negative
cell → revert and log. No knob: this is a repair, not a treatment, so the two
arms are two *binaries* built from the same tree with and without the `src/`
patch — both carrying the same `--filter-preempt`, so their accepted deal sets
are identical per seed.

*Measured — **SHIPPED**.* `ab-results/nt-answer-tie/`, seed `1787144117`, sha
`7f8fa998`+patch, 716,800 boards/arm/vulnerability, 28 shards × 25,600:

| vul | fired | plain/bd | PD/bd | plain/fired | PD/fired | sd plain/fired | sd-PD/fired |
| --- | ---: | --- | --- | ---: | ---: | ---: | ---: |
| none | 252 (0.04%) | +0.0002 ±0.0003 | +0.0002 ±0.0003 | +0.508 | +0.520 | +0.646 | +0.719 |
| both | 221 (0.03%) | +0.0001 ±0.0003 | +0.0001 ±0.0003 | +0.416 | +0.267 | +0.221 | +0.032 |

Eight of eight cells lean positive and none is CI-clear negative: the pinned
gate passes and the repair ships. `probe-divergence --gate-opener ours` is
**0 of 252** and **0 of 221** foreign — perfect isolation, as a book row keyed
`P* 1NT (3x) X -` should be.

The mechanism is not the one the defect description predicts. Only 6.0% / 3.6%
of divergent boards are "a different bid"; **85.7% / 88.7% are "passed where the
baseline bid"**, and game is reached in both arms on 94.4% / 100.0%. The guard
is doing most of its work through the **reading**: `4♥ | ♥ at least as long as
♠` tells responder opener is not hiding five spades, so responder stops
correcting to `4♠` over a 4-4 answer. The call-level 5-4 repair is real but
rare; the reading it publishes is what the IMPs came from.

*`smoke-default` cannot see this lane.* The default-system hash is unchanged
(`39ca60a251e03e558cfe44659b44ae45b1fe296d806e90cb3ed1cc9338bf72cd`,
`--count 20000 --seed 1`) — but that is **not** an inertness proof here: we never
overcall a 1NT opening at the three level, so a self-play smoke never reaches
`1NT (3x)` at all. The A/B above is the whole evidence.

### The v2 queue, re-priced (probe + fresh-seed census, 2026-08-19)

The N3 residue was queued against an opponent whose side of the lane had never
been probed. It has been now — advancer tables, sit-vs-rescue over our double,
the preemptor's second turn, and the `(4x)`/`(3NT)` triggers, in
[bba-1nt-counter-defense.md](../ai-bidder/bba-1nt-counter-defense.md) §"Their side
of the lane". Four items move.

**1. `(3NT)` is closed — no trigger.** BBA never bids `3NT` directly over our
1NT: the row does not exist at 200,000 hands per vulnerability, at either
vulnerability. Nothing to counter.

**2. `(4x)` is re-priced down, twice.** On the fresh-seed anchor the whole `4+`
bucket is **38 bd / −43 plain / −45 PD** (NV 23 bd −22/−27, vul 15 bd −21/−18),
not the 43 bd / −89 of the series seed; per board −1.13 ±2.78, a CI that
swallows the total. And the trigger is not a widened `(3x)`: BBA's four-level
overcalls are **eight**-card suits (`4♥` 0.049%, `4♠` 0.046%, `4♦` 0.012%,
`4♣` 0.006% of hands), six times rarer than the three-level rows, with `5♣` and
`5♦` (also 8+) as common as `(4M)`. Widening the `(3x)` template to `(4x)`
would author for a hand class the template does not describe. What survives is
narrower and better supported: the advancer **sits for our double of a `(4x)`
on 96.7–99.9%** of hands, and our floor cannot double above the three level at
all, so a book `X` over `(4x)` is an uncontested opportunity — parked as a
sized item, not the top of the queue.

**3. The penalty pass survives its probe, and the realized rate is stronger than
the probe's.** The item was queued on an unsourced "BBA sits over some doubles".
It does, and then some. Per random advancer hand the probe reads 88.2% Pass over
a minor and ~50% over a major; counted over **14,120 realized `1NT (3x)` boards**
of a `--filter-preempt` arm — where the advancer's hand is conditioned on our
side holding 23+ — it is **99.7% / 100.0% / 97.1% / 98.3%** over
`(3♣)`/`(3♦)`/`(3♥)`/`(3♠)`. If opener leaves the double in, we defend the
doubled three-level contract essentially every time. This is not the `(2♦)` lane,
where the runout was unconditional and the item died without a run.

**4. The `X (4z)` tail is closed — it does not happen.** The preemptor never bids
again (six two-ply probe lanes, 99.4–100% Pass on filtered hands), and the
advancer, once conditioned, acts over our double on **0.0–2.4%** of realized
boards. The node `P* 1NT (3x) X (4z)` would own a tail that is two boards in a
thousand. Removed from the queue; what is left of the tail is the advancer's
`(4M)` over our **`3NT`** (6.7% over `(3♥)`, 9.1% over `(3♠)`), which is a
different node and still floor-owned.

#### What the census says instead — the two cells worth authoring

Per-cell decomposition of the fresh-seed anchor (`--bucket … --responses 8`,
both vulnerabilities pooled, boards / plain / PD):

| cell | bd | plain | PD | mechanism |
| --- | ---: | ---: | ---: | --- |
| `(3♠) X - 3NT` | 20 | **−94** | **−123** | opener bids `3NT` on one stopper facing a *seven*-card suit — and does it **holding four hearts**, the suit responder's takeout double promised |
| `(3♣) 3♠` | 16 | **−54** | **−57** | the force is answered `4♠` and dies: slam missed on a 5-5 11-count (`4♠+3`, BBA bid `6♠`), or `4♠` on a 5-3 where `3NT` was the make |
| `(3♣)` Pass | 43 | −67 | −14 | responder has no call; PD nearly recovers it |
| `(3♣) X` | 29 | −30 | −51 | |
| `(3♣) 3♥` | 30 | −12 | −13 | |
| `(3♣) 3♦` | 18 | +9 | +6 | |
| `(3♠) 4♥` | 18 | +37 | +38 | the authored four-level rung, and the lane's best cell |

The `(3♠) X - 3NT` cell is the largest single loss in N3 and has a one-row
cause. Over `(3♠)` the cheap `3M@140` rung does not exist — hearts are *below*
their suit — so `nt_answer_double`'s ladder runs `4♥@150` (four hearts **and**
17+ points), `3NT@130` (one stopper), `4♥@25` (three-card tolerance). Opener
with four hearts and 15–16 therefore bids `3NT` and buries the known 4-4 fit:
on the worst NV board opener held `K5.QJ84.A92.KQ92` opposite `Q6.A972.K8643.63`
and `3NT` went **three down** while `♥` was worth nine tricks. The repair is to
give the shown major its **cheapest legal** rung — four when three is gone —
above `3NT`, not to replace `3NT` everywhere.

This is *not* the refuted BBA-style continuation. That arm bundled the same
cheapest-level major with "no major → `3♦`/`4♦`", and its own decomposition put
the whole loss on `4♦ ← 3NT` (−310/−348 and −420/−429). Over `(3♠)` "no major →
`4♦`" is the arm's dominant branch, so the `(3♠)` column being negative there
prices the `4♦` substitution, not the `4♥` rung. The un-bundled half is
untested, and the census cell it targets is the lane's biggest.

### Round 5 — opener's answer to the takeout double: the fit rung **ships**, the leave-in is **refuted** (2026-08-19)

Two knobs, one control, one seed. `ab-results/nt-answer-x-v2/`, seed
`1787145997`, sha `7f8fa998`+patch (round 4 shipped), **716,800 boards per arm
per vulnerability** under the new `--filter-preempt`, 28 shards × 25,600.
`scripts/ab-nt-high-overcall.sh` `ROUND=4`.

*Read the per-board figures against `--filter-preempt`'s density, not
`--filter-1nt`'s.* The `1NT (3x)` lane is **13.7%** of accepted boards here
against 0.60% there, so these per-board numbers are ~23× more concentrated than
the round-1/2 rows above and are **not** comparable to them. Per-fired is.

#### `fit` — `nt_high_overcall_x_major_at_four`, **SHIPPED DEFAULT-ON**

| vul | fired | plain/bd | PD/bd | plain/fired | PD/fired | sd plain/fired | sd-PD/fired |
| --- | ---: | --- | --- | ---: | ---: | ---: | ---: |
| none | 1103 (0.15%) | **+0.0018 ±0.0007** | **+0.0034 ±0.0008** | +1.141 | +2.182 | +0.243 | +0.889 |
| both | 1340 (0.19%) | **+0.0032 ±0.0009** | **+0.0062 ±0.0011** | +1.687 | +3.322 | +0.352 | +1.578 |

Four of four double-dummy cells CI-clear positive, all four sd-lead cells
positive, `probe-divergence --gate-opener ours` **0 foreign of 1103 / 1340**.
This is `win | win` on the decision table, so it ships default-on. One rung:
over `(3♠)`, `4♥` at 140 with four hearts — the fit responder's takeout double
promised, which the ladder previously buried under `3NT@130` because hearts sit
*below* their suit and the cheap `3M` rung does not exist there.

The census cell it targets (`(3♠) X - 3NT`, 20 bd / −94 plain / −123 PD) is
the one the "BBA-style double continuation" arm also touched and lost on. The
difference is the un-bundling: that arm replaced `3NT` with `3♦`/`4♦` when
opener had *no* four-card major, and its own decomposition put the whole loss on
`4♦ ← 3NT`. Keeping `3NT` for the no-major hands and adding only the fit rung
turns the same cell from a −4.4/board loser into a +1.1/+1.7-per-fired winner.
**A fresh-seed confirmation is owed** before this row is treated as settled.

#### `pass` — `nt_high_overcall_x_leave_in`, **REFUTED, kept opt-in**

| vul | fired | plain/bd | PD/bd | sd plain/bd | sd-PD/bd |
| --- | ---: | --- | --- | --- | --- |
| none | 9157 (1.28%) | **−0.0048 ±0.0019** | +0.0078 ±0.0021 | **−0.0263 ±0.0019** | **−0.0184 ±0.0021** |
| both | 10493 (1.46%) | +0.0072 ±0.0026 | +0.0295 ±0.0028 | **−0.0257 ±0.0027** | **−0.0099 ±0.0027** |

Double dummy splits by vulnerability — a CI-clear plain **loss** NV, a CI-clear
plain win vulnerable — and perfect defense is a large win in both. That pattern
is precisely the doubling artifact [measurement.md](../measurement.md) names, and
the **sd-lead tie-breaker settles it: CI-clear negative in all four cells**,
−1.75 to −2.06 IMPs per fired. Isolation was clean (0 foreign of 9157 / 10493),
so this is the treatment, not a leak.

The bridge reading of the split is the honest one: with 15–17 opposite a
takeout double's 8+, we hold 23+ and belong in **game**, not defending a doubled
three-level partscore for +200. The vulnerable column is the exception that
proves it — +500 instead of +200 is what flips double-dummy's sign, and even
that does not survive a realistic opening lead. The knob stays default-off; the
only live follow-up is a **vulnerability-gated** variant, and it inherits the
sd-lead result as its prior.  Round 6 below re-slices these same dumps and
finds a second, better-supported follow-up: the loss is not uniform, and a
length gate keeps the part that pays. Probe evidence that the leave-in *can* fire (the
advancer sits 97–100% of realized boards) was correct and irrelevant: the
question was never whether they run, it was whether defending beats bidding.

### Round 6 — the leave-in re-sliced (2026-08-20)

Round 5 refuted the v1 gate as one number. Before discarding the idea, the
**existing** dumps (`ab-results/nt-answer-x-v2/{pass,base}-{none,both}`, seed
1787145997, 716,800 bd/arm/vul) were re-scored — no new bidding run — through a
new `--by holding` bucket key shared by `ab-dump-bucket` and `ab-dump-sd`
(`common::holding_key`). The key is opener's holding in **their** seven-card
suit: `len` {≤2, 3, 4+} × `top_honors` {0, 1, 2+} × whether the `4M` fit rung
fires.

The window is exact for *this* gate: 9157/9157 (NV) and 10493/10493 (vul)
divergent boards key to a real bucket, ON always passing and OFF always
bidding, so the `(other)` bucket reads zero. `len0-1` never appears and never
will — a balanced 15–17 opener holds no singleton or void.

**Do not read `(other)` as the isolation gate.** It collects genuinely foreign
boards *and* in-lane boards whose first divergence is downstream of opener's
answer, and only the first is a leak. v1's gate was wide enough (pass on ≤1
honor) that every board reaching the table diverged right at the answer, so
`(other)` happened to be zero; the narrow v2 gate leaves opener's call
unchanged on most boards it reaches and moves only the *reading*, which
surfaces as a later divergence — 12.7% / 9.4% in Round 7, every one of them
still a `1NT (3x) X` auction we opened. Test foreignness explicitly.

**Read the `no4M` rows, not the totals.** The `pass` arm predates the fit rung,
so its dumps still route to `Pass` on boards where today's shipped
`nt_high_overcall_x_major_at_four` (weight 140) outbids `Pass` (weight 135).
Those boards cannot diverge in Round 7 — both arms bid `4M` — so every figure
below is the `no4M` subset, which is what a fresh A/B against today's default
will actually see. The `4M` block is quoted separately as finding 3.

**IMPs per fired, NV / vulnerable** (`no4M` subset; n = NV/vul):

| bucket | n | DD plain | DD PD | sd plain | sd PD |
| --- | ---: | --- | --- | --- | --- |
| whole subset | 7870 / 8916 | −0.25 / +0.76 | +0.80 / +2.33 | −1.93 / −1.55 | −1.25 / −0.44 |
| `len2 hon1` | 2165 / 2476 | −1.05 / −0.46 | +0.22 / +1.66 | −2.53 / −2.70 | −1.76 / −1.28 |
| `len2 hon0` | 980 / 1162 | +0.15 / +0.88 | +0.88 / +1.73 | −1.01 / −0.54 | −0.39 / +0.21 |
| `len3 hon1` | 2995 / 3287 | −0.75 / +0.37 | +0.27 / +2.05 | −2.66 / −2.33 | −2.06 / −1.18 |
| `len3 hon0` | 698 / 843 | +0.62 / +1.85 | +2.02 / +3.42 | −0.88 / −0.16 | +0.21 / +1.19 |
| **`len4+`** | **1032 / 1148** | **+1.92 / +3.58** | **+2.68 / +4.36** | **−0.13 / +1.09** | **+0.33 / +1.65** |

Three findings, in order of what they cost:

1. **The whole surviving case is length.** `len4+` is the best cell at both
   vulnerabilities on every scorer, and the only one that clears Round 5's
   −2.06 IMPs/fired sd prior: flat NV (−0.13) and genuinely positive vulnerable
   (+1.09), against the subset's −1.93 / −1.55. It is 13% of the subset, so v1
   spent roughly seven bad boards to buy each good one.

2. **The honor axis runs the wrong way.** At fixed length every measured honor
   step *costs*, on both scorers: `len3 hon0` +0.62 vs `len3 hon1` −0.75 (DD,
   NV) and −0.88 vs −2.66 (sd, NV); `len2` repeats it. The mechanism is
   `has_stopper` — A, Kx, Qxx, or Jxxx — so at three cards an A/K/Q in their
   suit **is** the stopper, and honors there mark the boards where the `3NT` we
   gave up was a stopper-backed game rather than a punt. `hon2+` is genuinely
   unmeasured (v1's gate was `top_honors(..=1)`), so it is not refuted; but the
   trend extrapolates to the *worst* three-card cell, not the best.

3. **v1's headline partly priced boards `main` no longer passes.** The `4M`
   block is DD −1.14 / −1.02 and sd −2.69 / −2.92 per fired — the worst block
   in the slice — and the fit rung shipped in Round 5 already outbids it. Any
   re-measure must use today's default as `base`, which Round 7 does.

**Consequence — v2 is two knobs, not one.** `nt_high_overcall_x_leave_in` is
re-gated to `len(over, 4..)`; the honor disjunct becomes its own
`nt_high_overcall_x_leave_in_three` (`len4+ | (len3 & hon2+)`). Finding 2 says
the two disjuncts have opposite signs, so bundling them would let a win ship
the bad half or a loss bury the good one. Round 7 runs them as separate arms
(`scripts/ab-nt-high-overcall.sh`, `ROUND=5`: `base` / `length` / `three`), with
`three vs length` reading the extension's own price. A 4000-board preflight
confirms the arms are nested as designed — `length` diverges from `base` on 9
boards, `three` on 17, `three` from `length` on the remaining 8.

**Reproduction caveat.** The DD tables re-sum to Round 5's published headline
exactly (9157 fired, plain −3431, PD +5558). The **sd** totals do not: −18,645
(NV) / −18,472 (vul) against the published −18,835, i.e. −2.036 vs −2.057 per
fired, ~1% off. Cause, not noise: in the ON arm *they* declare the doubled
partscore, so the opening leader is **our** side and our own book feeds the
blind lead, and `main` has moved since Round 5 (the weight-tie guard
`1ecac19d`, the fit rung `30ea36ba`). Recorded as reproduced-to-1%-with-cause;
every conclusion above is a *within-slice* contrast, which that drift does not
touch.

**In-sample warning.** The re-gate was chosen on these dumps, so it is
in-sample and ships nothing until Round 7's fresh-seed A/B confirms it
out-of-sample on plain DD, per [measurement.md](../measurement.md)'s domain
addendum for a knob whose mechanism is adding doubles.

### Round 7 — the length leave-in **SHIPS DEFAULT-ON**, the honor half refuted (2026-08-20)

`ab-results/nt-answer-x-v3`, `SEED_BASE=1787169600`, sha `14acdd1f`, 28 x 25,600
= 716,800 bd/arm/vul, `--filter-preempt`, three arms against today's default
(`base`, which already carries the `4M` fit rung):

- `length` — `nt_high_overcall_x_leave_in` re-gated to `len(over, 4..)`
- `three` — ...plus `nt_high_overcall_x_leave_in_three`, the full v2 candidate

**IMPs per fired, NV / vulnerable:**

| pairing | fired | plain DD | PD | sd plain | sd PD |
| --- | ---: | --- | --- | --- | --- |
| **`length` vs `base`** | 2124 / 2191 | **+2.383 / +3.410** | +2.885 / +4.074 | **+0.558 / +0.817** | +0.837 / +1.244 |
| `three` vs `base` | 3298 / 3472 | +1.384 / +2.166 | +1.930 / +2.966 | **−0.546 / −0.630** | −0.218 / −0.109 |
| `three` vs `length` | 1211 / 1298 | −0.405 / +0.012 | +0.200 / +1.033 | **−2.438 / −2.990** | −2.094 / −2.406 |

Per board with CI: `length` vs `base` plain **+0.0071 ±0.0009** NV / **+0.0104
±0.0012** vul, PD +0.0085 ±0.0010 / +0.0125 ±0.0013, sd-plain +0.0017 ±0.0009 /
+0.0025 ±0.0011, sd-PD +0.0025 ±0.0010 / +0.0038 ±0.0012.

#### `length` — **SHIPPED DEFAULT-ON**

CI-clear positive in **all eight cells**. No wash, no negative cell, no scorer
disagreement, both vulnerabilities — the ship condition met without needing the
decision table's tie-break rules, and the arbiter column (plain DD, since the
mechanism is adding doubles) is the strongest of the four. v1 on this same lane
was CI-clear *negative* in all four sd cells. Same convention, window and
opponent model: the gate was reading the wrong feature.

**Isolation: 0 foreign boards at both vulnerabilities** (2138 NV / 2201 vul
divergences, every one a `1NT (3x) X` auction we opened). This is one of the
campaign's few clean isolation gates.

Bucketed by `--by holding`, **13 of 13 buckets positive on DD at both
vulnerabilities**; on sd, 12 of 13 NV (only `(3♣) len4+ hon1` at −0.31 plain,
PD-positive) and 12 of 12 vulnerable.

Two things the bucket table shows that no earlier measurement could:

1. **`hon2+` is the best cell, not a passenger.** `len4+` with two or more of
   A/K/Q in their suit reads +4.21 (NV) / +4.63 (vul) plain per fired, the top
   cell at both vulnerabilities. v1's `top_honors(..=1)` gate structurally
   excluded it, so it appears in no prior dump and Round 6's slice could not
   see it — roughly half of `length`'s fired boards are therefore genuinely
   out-of-sample even against the slice that motivated the design.

   This resolves the apparent contradiction with Round 6's finding 2. At
   **three** cards an A/K/Q in their suit *is* the stopper (`has_stopper` = A,
   Kx, Qxx, Jxxx), so passing spends a real stopper-backed `3NT`. At **four**
   they hold seven and we hold four: the suit was never running against `3NT`
   anyway, the stopper question is moot, and the same honors become pure
   defensive tricks. Honors hurt at three and help at four for one reason.

2. **A suit gradient.** The leave-in pays most against the highest overcall:
   sd plain per fired at `hon2+` is `(3♠)` +1.67 / `(3♥)` +0.21 / `(3♦)` +0.09
   / `(3♣)` +0.26 NV, and `(3♠)` +1.46 / `(3♣)` +1.79 vul. Over `(3♠)` opener's
   alternatives are genuinely bad — `3NT` wants a spade stopper we do not have
   holding four small, everything else is at the four level. Over `(3♣)`,
   `3NT` is cheap and often right. **In-sample on this run**; a suit-dependent
   gate is a Round 8 question, not a conclusion.

Also confirmed: **no `4M` bucket appears at all**. In Round 6's slice the `4M`
block was 1287/1577 boards and the worst in the table; with the fit rung in
`base` those boards are bid identically in both arms and never diverge. Round
6's finding 3 was right, and running against today's default rather than the
pre-ship arm is what made it visible.

#### `three` — **REFUTED, kept opt-in**

Priced in isolation against the shipped gate, sd-lead is CI-clear negative at
both vulnerabilities: **−2.44 / −2.99 IMPs per fired** — v1's own headline
magnitude (−1.75 to −2.06), reproduced on fresh seeds. The honor half is not
the weaker disjunct; it *is* the v1 loss.

Its DD signature is `plain wash | PD win` (+0.0000 ±0.0009 plain vulnerable,
**+0.0019 ±0.0009** PD), which the standard decision table would ship
default-on. The domain addendum blocks it, and this is the cleanest example the
campaign has produced of why that addendum exists: the knob's mechanism is
*adding doubles*, and a double-dummy defender never misdefends exactly the
doubled contracts it creates. Plain DD arbitrates, sd-lead breaks ties, PD is a
double-blind column that neither rescues nor kills.

Added to the length gate the extension **inverts the package**: `three` vs
`base` is a CI-clear sd loss at both vulnerabilities where `length` vs `base`
is a CI-clear win. **Bundled as one gate — which is what the v2 plan originally
specified — this run would have returned a refutation at both vulnerabilities,
and a +3.4 IMPs/fired winner would have been thrown away inside it.** The
general rule: when a candidate gate is a disjunction and the slice gives its
disjuncts different signs, they are separate arms, always.

Kept as an opt-in knob (house rule for rejected-but-interesting treatments) and
a single-dummy re-measure candidate on its vulnerable PD reading.

#### Flagged, not fixed — reading drift is the one negative cell

12.7% (NV) / 9.4% (vul) of divergences are boards where opener's call is
**unchanged** and only a later call moves. Adding a `Pass` rule at weight 135
narrows the complement — `3NT` in that seat now also denies four of their suit
— so partner's inference shifts and a downstream slam try or double changes.
This is [reading-drift-handoff.md](../reading-drift-handoff.md)'s subject: a rule
addition is never reading-neutral when a call's meaning is read off the bidder.

Those boards are the only negative cell in the vulnerable slice: **−0.97 plain
/ −0.72 PD per fired** on 197 boards, against +0.90 / +0.23 NV. Pooled it is a
wash (+44 IMPs plain across both vuls) and it does not threaten the headline,
but it is a real vulnerable cost inside a shipped win. Not fixed here; recorded
for the reading-drift queue.

### Round 8 — the suit gate **REFUTED**, the uniform leave-in **replicated** (2026-08-21)

`ab-results/nt-answer-x-v4`, `SEED_BASE=1787252714`, sha `9f8b7975`, 28 x
25,600 = 716,800 bd/arm/vul, `--filter-preempt`, `ROUND=8` in
`scripts/ab-nt-high-overcall.sh`. Two arms:

- `base` — today's shipped default (the leave-in on, `len(over, 4..)`)
- `noleave` — `--ns-nt-high-overcall-x-leave-in false`, the pre-Round-7 ladder

**Why two arms answer a four-way question.** Round 7's suit gradient was
in-sample; the obvious follow-up — one arm per candidate suit gate — is
unnecessary here because the overcall suits **partition** the fired set: every
divergent board's window is `1NT (3x) X -` with exactly one `(3x)`, and the
`Pass` row's presence on a `(3♥)` board never consults the `(3♣)` table. So a
hypothetical arm with the leave-in gated to any suit subset would bid every
board identically to `base` on its in-subset boards and identically to
`noleave` elsewhere — its paired diff vs `base` is byte-for-byte a suit-bucket
subset of `base vs noleave`. One diff, bucketed, prices all fourteen candidate
narrowings at once, and the Round-7 lesson about bundled disjuncts does not
bite because the "disjuncts" live on disjoint boards. Read with:

```sh
ab-dump-bucket $R/base-VUL $R/noleave-VUL --by holding
ab-dump-sd     $R/base-VUL $R/noleave-VUL -v VUL --sd-worlds 16 --show 0 --by holding
```

(`ON` must be the leave-in arm = `base`; the `(other)` bucket must read zero,
and `probe-divergence --gate-opener ours` runs before the headline as usual.)

**Decision rule, pre-registered.** A suit-gate knob is authored only if some
suit reads CI-clear negative out-of-sample — plain DD the arbiter (the
mechanism keeps doubles in), sd-lead the tie-break, PD a double-blind column.

#### Verdict — no suit earns a gate; nothing is authored

**Headline (fresh seed): all eight cells CI-clear positive again**, NV /
vulnerable per board: plain **+0.0074 ±0.0009 / +0.0111 ±0.0012** (+2.48 /
+3.52 per fired, 2143 / 2252 fired), PD +0.0093 ±0.0010 / +0.0135 ±0.0013,
sd-plain +0.0015 ±0.0009 / +0.0026 ±0.0011, sd-PD +0.0026 ±0.0009 / +0.0044
±0.0012 — Round 7's +0.0071/+0.0104 plain and +0.0017/+0.0025 sd-plain
reproduced on an independent seed. **Isolation: 0 foreign of 2143 / 2252**
(`probe-divergence --gate-opener ours` passes at both vulnerabilities).

**Per suit (the Round-8 question), `len4+` cells pooled, per fired:**

| suit | fired NV/vul | DD plain NV | DD plain vul | sd plain NV | sd plain vul |
| --- | --- | ---: | ---: | ---: | ---: |
| `(3♣)` | 388 / 382 | +2.32 | +4.13 | −0.37 | +0.66 |
| `(3♦)` | 523 / 509 | +1.92 | +3.30 | −0.01 | +0.92 |
| `(3♥)` | 541 / 643 | +2.63 | +4.11 | +0.32 | +0.79 |
| `(3♠)` | 454 / 513 | +3.64 | +4.20 | +1.72 | +1.56 |

**Every suit is solidly DD-positive at both vulnerabilities** — 12 of 12
suit-by-honor buckets NV and 12 of 12 vulnerable. The in-sample gradient's
*shape* replicates on sd at NV (`(3♠)` clearly best, `(3♣)` mildly negative
at −0.37/fired ≈ −0.0002/bd, far inside the CI and sd-PD-positive) and
**vanishes vulnerable**, where `(3♣)` is +0.66. No suit is CI-clear negative
on any column, so per the pre-registered rule **no suit-gate knob exists**:
the uniform `len(over, 4..)` gate stands, and the Round-7 gradient goes down
as sampling noise around a real spades-best tilt that never crosses zero.

**The spade-only widening is dead too.** Re-slicing Round 7's `three vs
length` dumps (`ab-results/nt-answer-x-v3`) by suit: the `len3 hon2+`
extension is sd-negative in **every** suit at both vulnerabilities — spades
least bad and still **−1.82 / −2.48** per fired (373/411 boards), hearts worst
at −2.75 / −3.37, PD agreeing everywhere, 100% of divergences keyed (no
`(other)`). No suit ever buys that arm; `_three` stays a refuted opt-in.

**Reading drift replicates as the one soft spot.** The `(other)` bucket —
opener's call unchanged, a later call moved — is 11.8% / 9.6% of divergences:
NV +1.41 DD / +1.15 sd per fired, vulnerable **−0.56 DD / −0.63 sd**, Round
7's exact pattern (+0.90 / −0.97). Pooled it is positive and it threatens
nothing, but the vulnerable drift cost is real and stays on the
[reading-drift](../reading-drift-handoff.md) queue.

Round 8 closes N3's answer-to-the-double thread: the remaining queue items
are the drift cell (owned by the reading-drift campaign), `(4x)` widening,
the penalty pass, and the transfer re-measure.

## N2 — the pre-fix census (2026-08-15)

Muiderberg `(2♥)`/`(2♠)`, split by response off the pre-fix
`2026-08-12-ea2cde9-dirty` anchor arm. This census is what selected N2a/N2b/N2e
and queued N2c/N2d; N2e and N2b shipped on 2026-08-16 and N2a is parked, so the
table below is the **motivating** snapshot, not the current score. The live N2
status and the refreshed N2c/N2d evidence are in
[§N2](../one-notrump-competitive.md#n2--muiderberg-22-the-lane-today).

This section preserves the pre-fix `2026-08-12-ea2cde9-dirty` anchor arm
(204,800 boards/vul, deal-keyed DD cache), split one call deeper with
`probe-1nt-interference --bucket 2♠ --responses 8`: table A by **our**
response to their Muiderberg (and by response / advancer / opener), table B by
**BBA's** response to *our* natural `2M` overcall of its 1NT.  IMPs are ours
(table-A NS) on both tables, so a negative table-B row is BBA's gain.  Same
attribution ceiling as the current census — these rank, they do not isolate.

### Table A — our response, pooled NV + vul

| lane | our call | boards | plain | PD | plain/bd |
| --- | --- | ---: | ---: | ---: | ---: |
| `(2♠)` (430 bd, −284) | **Pass** | 242 | **−282** | +47 | −1.17 |
| | **`2NT` relay** | 68 | **−114** | **−184** | −1.68 |
| | `3♦` (→♥) | 56 | −15 | −46 | −0.27 |
| | **`X`** | 43 | **+110** | +97 | **+2.56** |
| | other | 21 | +17 | +4 | |
| `(2♥)` (393 bd, −43) | Pass | 194 | −107 | +153 | −0.55 |
| | **`2NT` relay** | 21 | **−45** | **−105** | −2.14 |
| | `2♠` natural | 50 | +7 | −5 | |
| | `3♦` (→♠) | 45 | +6 | −11 | |
| | `X` | 58 | +54 | +86 | +0.93 |
| | other | 25 | +42 | +45 | |

Three signs are consistent across all four cells (lane × vul):

1. **`X` wins everywhere** (+1.8 / +3.5 / +0.3 / +1.8 per board).  The Optional
   double (`2-3` in their suit, 8+) followed by opener sitting (`X P P`: +2.2 /
   +4.0 / +0.4 / +2.4) is the lane's best call.  BBA's own `X` here shows the
   other major and is a *loser* for BBA over `(2♥)` (table B `X` +357 for us).
2. **The `2NT` relay loses everywhere** (−0.9 / −2.6 / −0.9 / −3.5 plain,
   −1.9 / −3.7 / −3.8 / −6.3 PD).  Its own decomposition, `(2♠)` both vuls:
   sign-off `3♥` then opener passes −45 (20 bd); relay then pass `3♣` −42
   (21 bd); **sign-off `3♦` then opener bids `3NT` on 16 of 18 boards, −52
   plain / −125 PD** across all four cells — see the mechanism below.
3. **Pass** loses NV plain (−1.65 / −1.01 per board), is a wash vul, and is
   PD-*positive* vul (their `2M` fails and PD doubles it).  Its hand classes,
   `(2♠)` NV+vul: `≤5 hcp with a 6+ suit` (the relay's 6-HCP floor) **31 bd,
   −120 plain, −3.9/bd** — the single worst class in the lane, `2♠` making
   opposite our 9-11-trick heart/diamond spots (BBA at table B, un-overcalled,
   transfers there); `≤7 hcp, no 5-card suit` 109 bd, −43 (nothing to say —
   the obstruction the Muiderberg buys); `≤5 hcp, 5-card suit` 89 bd, −54;
   `8+ hcp with 0-1 or 4+ in their suit` **11 bd, −53** — hands with **no
   call at all**: `X` needs 2-3 trumps, the relay needs `points ≤ 8`
   (a 6-card suit's upgrade pushes an 8-count to 9), the club transfer needs
   `10+`.  `T.JT6.AQ85.QT963` and `4.K92.K97.Q98542` passed.

### Table B — BBA's response to our natural `2M`, pooled NV + vul

BBA plays plain Lebensohl here ([counter-defense](../ai-bidder/bba-1nt-counter-defense.md)):

| lane | BBA's call | boards | plain | note |
| --- | --- | ---: | ---: | --- |
| `(2♠)` (2291 bd) | Pass | 880 | −772 | our overcall failing (vul −1.48/bd; PD −4.8/bd = the auto-double) — the defensive-overcall lane's business, not this one |
| | `X` (= ♥4+) | 734 | −151 | BBA's one gain from a call |
| | `3♠` cue | 178 | **+216** | |
| | `3♥`, `2NT` relay, `3♦`, `4♥` | 138 / 145 / 70 / 16 | +72 / +66 / +7 / +68 | every constructive call is ours to gain |
| `(2♥)` (2423 bd) | Pass | 798 | −483 | same |
| | `X` (= ♠4+) | 790 | **+357** | |
| | `3♥` cue, `3♦`, `3NT`, `3♣`, `2NT` | 146 / 75 / 65 / 91 / 201 | +194 / +102 / +49 / +10 / +18 | |
| | `2♠` natural | 140 | −93 | |

BBA's Lebensohl earns it nothing on its constructive calls; its edge is our
overcalls going down (and, over `(2♠)`, its takeout `X`).

### Opener facing BBA's advances — not a leak

Advancer acts on ~15% of table-A boards.  Opener over the artificial `2NT`
minor-ask (`P 2NT P`) is a wash: 81 boards, −16 / −8 / +1 / 0.  Every other
advancer row is single digits of boards.  **The opener defects are after our
own weak calls**, whose ceilings the floor cannot see:

- `2NT - 3♣ - 3♦ -` → opener `3NT` (16/18, above);
- `2NT (3♥) X` — opener doubles their raise of the relay, 5 bd, −25 / −54 PD;
- `2♠ (3♥) 3♠` (`(2♥)` lane) — opener competes over our weak natural `2♠`,
  12 bd, mixed.

### The mechanism — weak calls read as unlimited

`probe-decision "Q93.K43.AKJT.Q42" "1NT 2♠ 2NT - 3♣ - 3♦ -"` reads partner as
**`hcp 6..37, points 6..37, every suit 0..13`**, provenance `depth 0,
fallback Some(0)` — the floor — and bids `3NT` 1.400 over Pass 0.  Two causes:

1. **`Points::project` and `Hcp::project` are floor-only** by design
   ([constraint.rs](../../src/bidding/constraint.rs), "floor only, matching every
   hand-written reader"; the two-sided `project_band` serves only the *pass*
   reading).  So `points(..=8)` on the relay and on the natural 2-level call
   projects to `0..37`; a weak sign-off is read as unlimited by every net
   downstream.  The uncontested 1NT structure is protected by the hand-coded
   notrump walk (`1NT - 2♣ - 2♦ - 2NT -` reads `8..9`); the Lebensohl lane has
   no such reader, so only the relay's `hcp(6..)` floor survives.  `1NT 2♥ 2♠ -`
   (our weak natural `2♠`) reads as **nothing at all**.
2. **The sign-off's own length is dropped too** — the reading of responder's
   `3♦`/`3♥` after `2NT - 3♣` is wrong on both axes.  The natural walk
   *blankets* every suit bid on the opening side after a 1NT opening except a
   lane's first three-level call (`nt_blanket` in
   [read.rs](../../src/bidding/inference/read.rs) — right for the uncontested
   transfer structure, where a lane's second bid is a completion), so the
   sign-off can only be read from its authored rule
   `min_level_is(3, ♦) & len(♦, 5..)`; but that rule is natural (unalerted),
   and the shipped `ReadingScope::Alerted` decodes **alerted** rules only.
   The call falls between the two regimes — the
   [reading-drift](../reading-drift-handoff.md) story exactly.  Verified with
   `PROBE_SCOPE=all probe-decision …` (`ReadingScope::All`, unmeasured):
   ♦ `5..13` comes back, `1NT 2♥ 2♠ -` regains ♠ `5..13` and `hcp 5+`, but the
   ceiling stays `..37` (cause 1) and **the floor still bids `3NT` 1.400** (and
   `4♦` 1.200) — the missing ceiling is the binding defect, the missing length
   an independent one.
3. **The relay's minor sign-off has no opener node.**  `lebensohl_signoff_raise`
   is wired for the major sign-off only (`(2♠) 2NT - 3♣ - 3♥ -`); `3♦` falls to
   the floor, which — reading an unlimited partner opposite 15-17 — bids `3NT`.

### N2 packages, from the census

The evidence column is the 2026-08-17 re-price and is frozen here. N2e and N2b **shipped 2026-08-16**, N2a is parked, and N2c/N2d stay open with re-read evidence in the live
[package queue](../one-notrump-competitive.md#package-queue--open-work-ranked-by-the-census).

| # | Package | Class | Evidence | Note |
| --- | --- | ---: | ---: | --- |
| **N2a** | opener **passes** the relay's minor sign-off — `{relay} 3♦ -` over `(2♥)`/`(2♠)` (the relay-then-pass-`3♣` is already terminal), a `landy_signoff_answer`-style node | book, one node | −52 plain / −125 PD on 18 bd, 16 of them the same wrong call | cheapest, cleanest; also gates the `2NT (3♥) X` |
| **N2e** | teach `opener_forced_past_invitation` ([instinct.rs:3820](../../src/bidding/instinct.rs)) that a Lebensohl **sign-off** is not a game force | floor, one predicate | traced 2026-08-16: the predicate is *"our strong 1NT + partner's last call is a three-level non-notrump bid"*, pure auction shape | **the actual cause of the `3NT`.** It sets `forced_to_game`, so the rail bypasses the net *and* `auction_forces_game()` pre-satisfies the game-milestone `Or` — `combined_hcp` never runs. Verified: a 12-HCP opener still bids `3NT`; the only hand-dependent gate is `stopper_in_their_suits()`. Smaller than N2a. **SHIPPED default-on 2026-08-16** as `instinct.forcing_ceiling_read` (the force also requires partner's read `points` ceiling to reach 10 — the direct three-level bid's `points(10..)`, not `nt_responder_game_floor`, which is 9 and would never fire). Probe: `P 9.001` beats `3NT 7.792`. A/B 3 seeds × 204,800 bd/arm/vul, **12/12 cells positive**, +0.0001 plain / +0.0003 PD both vulnerabilities, firing 0.01% at +2 to +6 IMPs per fired board. ⚠ it does **not** fix "every sign-off lane at once" — only alerted ceilings project, so under the shipped scope it reaches this lane alone; the other floor-workaround nodes are Phase 2 work (reach census in the handoff) |
| N2b | read the relay / sign-off / natural-2-level **ceilings** (a two-sided strength projection at these rules, or a Lebensohl reader) **and lengths** (`ReadingScope::All`, or exempt the sohl lane from `nt_blanket`) | reading | the general defect behind N2a and the `X` of their raise; touches every weak call in the system | **SHIPPED 2026-08-16, both halves.** Ceilings as `ReadingProfile::strength_ceilings` (soundness-proved book-wide; the raw whole-book arm was a 4-cell wash leaning plain-DD-negative, so it shipped on the nets-shielded arm — 4/4 cells positive); lengths as `ReadingScope::All`, which took the whole `nt_blanket` question with it (12/12 cells positive after the side-blind-strip forensic). Neither moves **this** node: nothing in the floor or the book read a strength ceiling at the time (the handoff's consumer census). Necessary, not sufficient; N2e is the sufficient half |
| N2c | the no-call 8-9-count with 0-1 / 4+ in their suit — widen the relay to `points ≤ 9` with a 6-card suit, or let a singleton double | book | **Current re-price:** 11 bd, −34 (`2026-08-17-53a3c254`) | small n; the Optional > Takeout verdict was measured pons-vs-pons |
| N2d | relay with a 6+ suit below 6 HCP (over `(2♠)` only, where the weak major has no 2-level call) | book | **Current re-price:** 31 bd, −126, −4.1/bd (`2026-08-17-53a3c254`) | contradicts the PD-distilled floor ([`lebensohl_relay_shape`](../../src/bidding/american/competition/lebensohl.rs)), measured pons-vs-pons; against Muiderberg the alternative is a making `2♠`, and BBA at table B bids these hands un-overcalled. Needs the A/B, not a re-derivation |

Nothing here is BBA-alignment (BBA's plain Lebensohl earns nothing at table
B); the lane's headroom is our own weak calls being *unread*.

The reading defect is the whole book's, not N2's — the campaign to fix it is
[authored-reading-handoff.md](../authored-reading-handoff.md), with this lane as
its testbed (N2a stays a book node in its own right).

**Correction (2026-08-16).** The census read the `3NT` as a consequence of the
weak call reading *unlimited*. Phase 1 of the handoff built the two-sided
reading, proved it sound book-wide, and measured it — and the `3NT` does not
move. The reading was genuinely wrong and is now right, but it was never what
decided this node: `opener_forced_past_invitation` forces to game off the
*shape* of partner's call, and no floor rule or authored gate reads a strength
ceiling at all. N2e is the fix; N2b is the prerequisite that makes a
ceiling-reading rule expressible.

## Memory compaction notes (2026-08-16)

- **Stopperless `3NT` escape gate — REFUTED and removed.** Requiring a
  stopper when advancing partner's double fired on 1.79% of 1.6m
  `--filter-1nt` boards and lost plain DD **−0.020 IMPs/board
  (−1.12/fired)** while gaining PD **+0.086 (+4.79/fired)**. BBA usually left
  the failing escapes undoubled; the PD-only gain was a doubling artifact.
- **Gambling games over `1NT (X)` remain opt-in (`b87e314`).** The BBA
  follow-up (128k boards/arm, 19 fired) lost **−4.6/−6.1 plain** and
  **−5.8/−7.4 PD** IMPs/fired (NV/vul): BBA passed 10 of the 19 business
  redoubles, and those boards accounted for the entire −111-IMP loss. The
  gamble won on the roughly six boards where BBA ran, so run-prone opponents
  remain the explicit re-open condition.
- Historical ship commits not otherwise recorded in checked-in prose:
  contested Stayman `98c6c21`, Stayman-defense `6:14` calibration `9312402`;
  Jacoby-transfer competition `c60f96f`; doubled-1NT runout Phase 1
  `5d06184`; penalty-latch persistence `5a2433d` and immediate advancer
  XX-runout `782a4aa`; responder penalty leave-in `ee0077b`, Optional-double
  default `bf6e5cd`, and defensive latch-style arm `cc35135`.
- Superseded statements to ignore if met in old notes: N2's relay-signoff
  `3NT` is caused by `opener_forced_past_invitation`, not a lost ceiling; the
  N4 Multi migration shipped (the stopper ask was measured and refused as a
  default, §N4 residue); diamond-transfer competition is Side A on / Side B
  opt-in; the doubler XX-runout is default-on; Phase 2 shipped the escape
  penalty doubles; and the double style is **Optional > Penalty > Takeout**
  (Optional shipped), not Takeout.
