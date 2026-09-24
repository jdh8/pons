# Competitive 1NT — closed package history

> **Archived 2026-08-19, extended 2026-08-21 and 2026-09-24.** This file holds
> the closed N1 Landy campaign (N1–N1j; its 2026-08-28 → 2026-09-24 sequel
> §N1l–§N1r is in [one-notrump-competitive-landy.md](one-notrump-competitive-landy.md)),
> N4's superseded measurement rounds v1–v6, the N4b
> diamond-double sweep, the superseded **census snapshots**, N3's eight
> **measurement rounds**, N2's **pre-fix census**, and the 2026-08-16 memory
> compaction notes, and — since 2026-09-24 — the shipped-state sections of N3,
> N4 (v7, its reader, N4e, N4f, N4-KK with `multi_px_split` and the mirror
> book) and N2's 2026-08-21 re-read. The live opponent model, the current
> census, the coverage inventory, each lane's state today, measurement
> discipline, the open queue, and the ledger remain in
> [one-notrump-competitive.md](../one-notrump-competitive.md).
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
collectible and plain DD can see it.~~ **Retracted 2026-08-15 ([§N4](#n4--their-2-as-a-multi-shipped-2026-08-15--v7-seven-rounds-default-on-vs-bba-via-the-census)):** the
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
   floor offers no `X` over `(4x)` at all, so see [§N3's flagged list](#flagged-not-fixed-floor-defects-reversible-defaults-proposed).

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
[§N3](../one-notrump-competitive.md#n3--their-33-preempt-of-our-1nt);
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

## N3 — the shipped tables (2026-08-18)

> Moved from the live doc 2026-09-24. The verdicts are in
> [§N3 — measurement rounds](#n3--measurement-rounds) above; the lane's open
> residue is in [the live doc](../one-notrump-competitive.md#n3--their-33-preempt-of-our-1nt).

### What was wrong

Nothing in the book keyed `1NT (3…)`; `high_overcall_responses` covers suit
openings only. Three floor defects followed, each visible in `probe-decision`:

- Responder's new suit **read as nothing**. `- 1NT 3♣ 3♠ -` gave opener
  partner `hcp 0..37`, every suit `0..13`, and opener passed — on a board where
  `6♠` was cold. It is worse than passive: on the same hand at responder's seat
  the floor's own top call over `(3♣)` was **`4♥` on a doubleton** (a phantom
  suit at game).
- The double fired on the wrong hands — 6–7 HCP balanced over `(3♥)` (opener
  then drove to `4♠`), while a 9–11 4-4-4-1 over `(3♣)` had no call at all and
  passed.
- The floor blasted `6♣`/`5♦` on 8–11 HCP over `(3♠)`.

BBA's side is the reference in
[bba-1nt-counter-defense.md](../ai-bidder/bba-1nt-counter-defense.md): its
three-level overcalls are natural seven-card preempts (`hcp 4–10`), and its
responder plays new suit natural 5+ `hcp 7–18`, `3NT` 9–17 with no stopper
gate, `X` = 4+♠ over `(3♥)` / balanced values over `(3m)`, `4m` weak natural
6+. So the lane needs an ordinary competitive scheme, not a counter-defense —
nothing here keys on a disclosure.

### Responder's table (`nt_over_high_overcall`)

Strength floors are the Lebensohl lane's — opposite 15–17, `points(10..)` is
game. The book owns responder's first call and opener's one answer; everything
deeper is the floor's (§"The book/floor line").

| call | weight | constraint | note |
| --- | ---: | --- | --- |
| `3y`, y above their suit | 180 + rank | `len(y, 5..) & points(10..) & at_least_as_long(y, ·)` vs each rival above their suit | natural, game-forcing |
| `4M`, M above their suit | 160 + rank | `len(M, 6..) & points(6..=9)` | natural, to play — the weak twin of `3M` |
| `4♥` over `(3♠)` | 160 | `(len(♥, 6..) & points(6..)) \| (len(♥, 5..) & points(9..))` | the only natural call left, so it carries the strong hands too |
| `X` | 150 | over `(3♥)` `len(♠, 4..)`, over `(3♠)` `len(♥, 4..)`, over `(3m)` `len(♥, 4..) \| len(♠, 4..)`; all `& points(8..)` | takeout, the 4-4 major finder; `.alert(NEGATIVE_DOUBLE)` |
| `3NT` | 140 | `author_direct_3nt`, fed the lane's **own** bit — `nt_high_overcall_3nt_stopper` (default **off**) is substituted for `direct_3nt_stopper` in a local `Agreements` ([nt_high_overcall.rs:29](../../src/bidding/american/competition/nt_high_overcall.rs)) | to play |
| `4m`, m below their suit | 120 + rank | `len(m, 5..) & points(10..) & at_least_as_long(m, ·)` when both minors are below theirs | natural, forcing |
| `Pass` | 0 | `hcp(0..)` | the finite catch-all |

**"+ rank" is load-bearing, and so is `at_least_as_long`.** Without them a
natural-suit family at one weight is decided by the call encoding's iteration
order, which picks the *lower* suit: the census board `AT9754.AT732.A.A` over
`(3♣)` — six spades, five hearts — bid `3♥`. `at_least_as_long` keeps a 6-5
out of its five-carder, and the rank-ordered weight breaks the genuine 5-5 tie
*upward*, which is also the better bridge (it leaves the lower suit biddable as
a correction under partner's raise). The same pair runs on the `4m` rungs and
on the `(3♣)` transfers. It pays a reading dividend too: after `3♠`, partner
now reads `♦ ..6` and `♥ ..6` alongside `♠ 5..`.

**The one deviation from the plan of record**: `4m` is priced at **120, under
`3NT` and `X`**, not the planned 170. At 170 a hand with five clubs, four
spades and game values bids `4♣` over `(3♥)` — bypassing both the 4-4 major
and the game we are trying to reach. Below `3NT` the rung fires exactly where
it should: 10+ points, a five-card minor, no four-card major, and no stopper.
The consequence is that in the `nostop` arm (`direct_3nt_stopper false`) `4m`
is inert, since `3NT` then admits every 10+ hand — which is what "partner holds
the stopper" means, and the A/B prices it.

### Opener's three answers

- `1NT (3x) 3y -` — a **major** `y`: `4y` 150 `len(y, 3..)`, `3NT` 140
  `stopper_in_their_suits()`, catch-all `4y` 100. A **minor** `y` (only `3♦`
  over `(3♣)`): `3NT` 150 with the stopper, `3M` 140 `len(M, 4..)` for each
  unbid major, `4y` 130 `len(y, 3..)`, catch-all `3NT` 100. A force must be
  answered, so both arms end finite.
- `1NT (3x) 4m -` — `3NT` is already gone: `5m` 140 `len(m, 3..)`, `4M` 130
  `len(M, 5..)`, catch-all `5m` 100. Responder denied both a five-card suit
  above theirs and a four-card major, so opener's major is at best 5-3 and the
  eight-card minor fit wins the tie.
- `1NT (3x) X -` — `high_overcall.rs`'s negative-double answer re-floored for
  15–17, plus the two rungs the 2026-08-19 census decomposition bought. In
  weight order: the shown major jumped to game with `points(17..)` (150); that
  major at its **cheapest legal** level with four — `3M` (140) when it clears
  their suit, `4M` (140) when it does not
  (`nt_high_overcall_x_major_at_four`, **default on**, the rung their `(3♠)`
  leaves missing); **`Pass` (135)** with `len(over, 4..)`, converting the
  takeout double to penalty (`nt_high_overcall_x_leave_in`, **default on**);
  `3NT` 130 on a stopper; the three-card tolerance (30 at the three level / 25
  at the four); catch-all `3NT` 15. Every major row is `at_least_as_long`-
  guarded whenever their overcall leaves both majors live, so a 5-4 answers in
  its five-carder (round 4). The leave-in's honor half —
  `nt_high_overcall_x_leave_in_three`, `len3 & top_honors(2..)` — is **refuted
  and default-off**: at three cards an A/K/Q in their suit *is* the stopper, so
  passing spends a real `3NT`; at four the suit was never running anyway and
  the same honors are pure defensive tricks.

### The `(3♣)` transfer variant (`nt_3c_transfers`)

`(3♣)` is the one three-level overcall that leaves steps below `3NT`. The arm
replaces the natural three-level rows in the `(3♣)` instance only:

| call | weight | constraint | reading |
| --- | ---: | --- | --- |
| `3♦` | 180 | `len(♥, 5..) & points(9..) & at_least_as_long(♥, ♠)` | transfer to ♥, INV+ |
| `3♥` | **181** | `len(♠, 5..) & points(9..) & at_least_as_long(♠, ♥)` | transfer to ♠, INV+ |
| `3♠` | 145 | `len(♦, 6..) & points(10..)` | transfer to ♦, GF |

As over `1NT (2♦)`, responder with the long minor **and** a stopper in their
suit prefers direct `3NT`; without one, the top-step transfer wins over the
general stopperless `3NT` fallback.

All three `.alert(LEBENSOHL_TRANSFER)`, so `project_authored` decodes the rule's
own constraint — no new reader. INV+ is driven to game at the completion (the
"displaced bid is GF" simplification), so there is no min/max split and no
responder second call to author. What it buys is the invitational five-card
major, which the natural table can only show as `X` or a pass, plus
right-siding — believed DD-blind when this was written, but the completion
moves declarer, so under the refined rule the plain scorer credits its
lead-direction half (see the §N1-lia package C verdict below).
**BBA plays all three naturally** (per-suit census recorded in
[bba-1nt-counter-defense.md](../ai-bidder/bba-1nt-counter-defense.md)
§`(3♣)` per call), so the arm is judged on its own merit, not on alignment.

Two build traps the design walked into and out of:

- The top-step minor transfer now shares `rubensohl::minor_transfer_completion`
  with `1NT (2♦) 3♠`, with the minors swapped: `3NT` with a stopper in their
  suit, otherwise five of the target minor as the finite catch-all. Thus
  `1NT (3♣) 3♠ -` is `3NT` with a club stopper, otherwise `5♦`; the
  three-level completion is below the artificial `3♠` and therefore illegal.
- The interfered tails are authored per the iron rule: their `X` of a transfer
  steals no room, so the completion re-registers verbatim; their `(4♣)` raise
  takes every step, so opener completes at the four level with tolerance
  (150) and doubles for values otherwise (100). Everything past that is floor.

### Measurement — eight rounds, archived

The table and its answer ladder were measured in eight rounds between
2026-08-18 and 2026-08-21, every headline preceded by
`probe-divergence --gate-opener ours`. The verdicts are pinned to their shas
and seeds and live in
[§N3 — measurement rounds](one-notrump-competitive-closed.md#n3--measurement-rounds);
the [ledger](../one-notrump-competitive.md#ledger) carries their numbers. What the rounds settled:

| round | question | verdict |
| --- | --- | --- |
| 1 (2026-08-18) | the table itself, `stop ↔ base` | **SHIPPED default-on** — owned plain +0.0021/+0.0029, no negative cell in sixteen readings |
| — | `stop` vs `nostop`, the *shared* `3NT` stopper bit | two lanes summed and disagreeing (+2.20 here, −1.23 PD in the advance lane), so this table got its **own** bit |
| 2 (2026-08-18) | that private bit, and the `(3♣)` transfers | bit **SHIPPED off** (+2.37/+1.65 per fired, 0 foreign on all four cells); transfers a wash ×2 seeds, opt-in |
| 3 (2026-08-19) | the top step re-cut as `1NT (2♦) 3♠`'s minor-swapped twin | still `wash \| wash`; `nt_3c_transfers` stays opt-in, default byte-identical |
| — (2026-08-19) | BBA-style double continuation (cheapest major, else `3♦`/`4♦`) | **REFUTED and removed** — all eight cells negative, all eight CIs excluding zero; `4♦ ← 3NT` missed the games. Do not retry the whole continuation |
| 4 (2026-08-19) | the answer tables' cross-call weight ties (encoding order picked hearts) | **SHIPPED** as a repair at a pre-pinned gate; 0 foreign of 252/221. The IMPs came from the *reading* the `at_least_as_long` guard publishes, not the 5-4 call change |
| 5 (2026-08-19) | opener's answer to the takeout double: the `4M` fit rung, and a v1 leave-in | fit rung **SHIPPED default-on** (4/4 DD cells CI-clear, 0 foreign); leave-in v1 **REFUTED** — sd-lead CI-clear negative in all four cells at −1.75…−2.06 per fired |
| 6 (2026-08-20) | re-slice v1's own dumps by opener's holding in their suit | the loss is **length**-shaped, not honor-shaped; the honor axis runs backwards because at three cards an A/K/Q *is* the stopper. v2 splits into two knobs |
| 7 (2026-08-20) | `length` and `three` as separate arms, fresh seed | `length` **SHIPPED default-on** — CI-clear positive in all eight cells (+2.38/+3.41 per fired), 0 foreign, 13/13 buckets DD-positive; `three` **REFUTED**, opt-in |
| 8 (2026-08-21) | is any suit-dependent gate worth authoring? | **no** — every suit DD-positive at both vulnerabilities, the round-7 gradient was noise around a real spades-best tilt that never crosses zero, the spade-only widening of `_three` sd-negative in every suit, and the leave-in replicated CI-clear on an independent seed |

**Score against the lane on the 2026-08-21 `1e9a47e2` arms** — this is
attribution and not a causal A/B: each number is the whole board's swing and
carries the mirrored table.

*Paired, same seed 1787064872.* Against the post-ship 2026-08-18 arms
(`9cfb464b`), which reproduce their published totals exactly, the four authored
suits move **410 bd: −273 → −192 plain, −186 → −73 PD**; with the floor-only
`4+` bucket, **448 bd: −316 → −235 plain, −231 → −118 PD**, i.e. −0.71 → −0.52
plain and −0.52 → −0.26 per board. `4+` itself is unchanged at 38 bd / −43 /
−45, so every IMP of the movement is in the authored buckets — the answer
refinements of rounds 4–7.

*Cross-seed, for scale only.* The pre-ship snapshot (`53a3c254`, seed
1783375064) had the four suits at 362 bd / −345 plain. Do **not** subtract two
anchor totals across seeds to estimate the package's value; the isolated A/Bs
in the archive are the causal evidence.

### Disclosure — what BBA is told

No card row exists for this lane. `card.rs`'s `SCHEMA` has no slot for
"responses to their three-level overcall of our 1NT", and grepping it turns up
nothing between `Lebensohl after 1NT` / `Rubensohl after 1NT` (both the *two*-level
lane) and the preempt rows. The remaining transfer variant is default-off, so
it does not change the default alert fixture; it adds `comp:lebensohl-transfer`
and `completion`. The transfer arm additionally has
no honest card row to set, the same record as the Landy and Multi counters: a
treatment EPBot's schema does not name, so it is as invisible to BBA as
`Not defined = 0`. Accepted for measurement; if it ever ships, that asymmetry
belongs in its ship row. The removed BBA arm used
`comp:nt-high-bba-placement` during its experiment.

### Flagged, not fixed (floor defects; reversible defaults proposed)

- `1NT (4♥) X - 5♦ - 5♥ (X) - - -` — responder's third call is a five-level cue
  the floor then passes (2 bd, −34). Out of the book's line; the proposed
  repair is a floor rail "no cue above game unless slam-forcing", for the floor
  campaign.
- Their `(4x)` overcalls: **re-priced 2026-08-19** — see
  [The v2 queue, re-priced](one-notrump-competitive-closed.md#the-v2-queue-re-priced-probe--fresh-seed-census-2026-08-19).
  The fresh-seed bucket is 38 bd / −43 plain / −45 PD with a CI that swallows it, and the trigger is an *eight*-card suit, so the `(3x)`
  template does not widen. The floor still offers **no `X` over `(4x)` at all**
  (`their_live_bid_at_most(3)`, `instinct.rs:6058`), and BBA's advancer sits for
  that double on 96.7–99.9%, so a book `X` there is the surviving item.
- We **never** overcall a 1NT opening at the three level (table B: 0 boards) —
  BBA does on 1.5% of its 1NT-defense hands. Obstruction is DD-blind, so a
  preempt package there is a single-dummy harness item, not this one.
- `(3♥) 3NT` on a singleton in their suit (11 bd, −23) was the
  `direct_3nt_stopper` question; round 2 answered it (the lane's own bit,
  shipped off).
- **Dead rows, deliberately left**: in `nt_answer_double` the `4M@25`
  three-card-tolerance rung is unreachable whenever that major clears their
  suit — the identical `3M@30` rung outranks it on exactly the same hands — so
  it is dead in the `(3♣)` and `(3♦)` tables and for spades in the `(3♥)` one.
  Deleting them is **not** inert: an unreachable row still joins the *reading*
  of its call, so dropping `4♥@25 len(♥, 3..)` would narrow a made `4♥` from
  "three-plus hearts" to "four and a maximum" and move calls elsewhere.
  Proposed reversible default: **leave them**, and revisit only inside a reading
  A/B.
- Opener's answer to a forcing **major** never shows the *other* four-card
  major (`1NT (3♣) 3♥ -` with four spades bids `4♥` or `3NT`, never `3♠`).
  Deliberate — one book answer, floor beyond — but it is a real 4-4 miss when
  responder is 5-4.

### Open residue

Round 8 closed the answer-to-the-double thread. What is left, and where it
lives:

- **`X` over their `(4x)`** — the surviving `(4x)` item, queued as **N3-x** in
  [the live queue](../one-notrump-competitive.md#package-queue--open-work-ranked-by-the-census). The `(3x)` template does not widen to it.
- **Fresh-seed confirmation of the `4M` fit rung** — owed since round 5, queued
  as **N3-fit**. Rounds 7–8 ran against a `base` that carries the rung, which
  confirms nothing about the rung itself.
- **`nt_high_overcall_x_leave_in_three`** — refuted, opt-in, a single-dummy
  re-measure candidate (**N3-three**).
- **`nt_3c_transfers`** — measured wash on four seeds, opt-in; what it buys is
  DD-blind (**N3-xfer**).
- **Reading drift** — 11.8%/9.6% of the leave-in's divergences move only a
  *later* call, −0.56 DD / −0.63 sd per fired vulnerable against +1.41/+1.15
  NV. Pooled positive, but a real vulnerable cost inside a shipped win. Owned
  by [reading-drift-handoff.md](../reading-drift-handoff.md).
- **The penalty pass is *not* open.** Round 8's closing sentence still lists it
  among the remaining items; that line is stale. It shipped as the
  `len(over, 4..)` leave-in in round 7 and replicated in round 8. Proposed
  reversible default: treat it as closed, and read round 8's list as naming
  `_three` (the refuted honor half) rather than the pass itself.

## N4 — their `(2♦)` as a Multi (**SHIPPED 2026-08-15 — v7, seven rounds; default-on vs BBA via the census**)

> The lane's **tree map** — every authored node, who owns each seat, what each
> call reads as, and their pass-or-correct ladder — lives in
> [one-notrump-multi.md](../one-notrump-multi.md), regenerable from the book.
> This section keeps the verdicts.

The rebuild of the deleted `defense_2d_multi`, on the disclosure channel N1
uses and with the continuations gated. Not the natural table
[bba-multi-2d.md §4](../ai-bidder/bba-multi-2d.md) sketched — that arm was **not
run** (recorded here so nobody assumes it was): the shipped Transfer-Lebensohl
`(2♦)` leg keeps its constructive calls, and only what named *diamonds* moves.

### Engagement — `their.two_diamonds_multi`

The second field of `TheirDisclosures`: their `2♦` is a Multi, one unknown
six-card major (BBA's 2/1 reference: `hcp 9–18`, median 13,
[bba-multi-2d.md](../ai-bidder/bba-multi-2d.md)). Undeclared keeps the natural
leg, byte-identical (smoke `18aba5ce…` re-verified from the worktree — that
constant is **stale**: `smoke-default` was re-based to `cf583ff5…` on
2026-08-16 by the strength-ceiling ship
([authored-reading-handoff.md](../authored-reading-handoff.md)) and has moved
again since. Re-verify byte-identity by diffing the dump against `main` HEAD,
not against a quoted digest).
`bba-gen --their-2d-multi` arms it; `their_2d_multi` derives it from an
explicit `Multi-Landy` row at face value and otherwise uses the shipped 2/1
census default. `--their-2d-multi false` names the pre-N4 arm.
`bba-decompose --multi-counter` replays a candidate dump; `web`
`declare_their_2d_multi`; `probe-call-reading --their-2d-multi`.

Either/or with the natural leg (`defense_2d_multi` in `lebensohl.rs`), never
an overlay: the deleted first build gated responder's table and left the
continuations natural, so opener answered a natural `3♦` with a transfer
completion. Only the Transfer style has a `(2♦)` leg to re-key; Plain keeps its
natural table under the declaration.

### The table (`multi_2d_responder`, `rubensohl.rs`)

`stayman_2d_constructive` is shared verbatim with the natural leg — `3♣`
Stayman + Smolen, `3♦`→♥, `3♥`→♠, `3♠`→♣, Leaping Michaels `4♣`/`4♦` — the fits
it hunts are in the major they do *not* hold. What moves:

| call | natural leg | Multi leg | why |
| --- | --- | --- | --- |
| `X` | `DoubleStyle::Optional`: `len(♦, 2..=3) & hcp(8..)`, opener cooperates (pass, or run to a 5-card suit with ≤2♦) | **`hcp(6..)`** (v7; v1–v5 `hcp(8..)` at 143), alerted `comp:multi-values`, weight 130 — below `3NT` 150, `3♠`→♣ 145, the natural `2M` 140 and the relay 135, so a weak 5+ suit still escapes or relays | BBA's own values double (`hcp 5–17`, 41% of its hands, median 9), no diamond claim — the *waiting* call; they name the major, we act on it (`multi_responder_rebid`). Read: **`points 8..`, ♥ ≤4, ♠ ≤4** — the table used to claim `points 6..` and every suit ⊤, stale on both halves. The 8 is `responder_overcall_double_reading`'s hard-coded `DoubleStyle` floor, two points above this rule's own `hcp(6..)`; `reading.their_multi_double_reading` (§N4f) is the opt-in repair. The major caps are sound: by weight ordering a five-card major always escapes or transfers instead |
| direct `3NT` | `points(10..) & stopper_in(♦)` | `points(10..) & stopper_in(♥) & stopper_in(♠)` | the blast that needs no more information; one major open → double first |
| `2NT` relay shape | 5+ in ♣/♥/♠, `hcp 6+` | 5+ in **any** suit, `hcp 6+` | diamonds are ours to sign off in |
| after `2NT - 3♣ -` | `3♥`/`3♠` (5+) or pass | **`3♦`**/`3♥`/`3♠` (5+) or pass (`multi_relay_rebid`) | the rung the natural leg cannot have |
| `3♠`→♣ completion | `3NT` with a ♦ stopper else `5♣` | `3NT` outright | no stopper to key on; `5♣` on a 6-2 fit is the worse guess |
| opener over `X -` | cooperate | a four-card major (`2♥` first), else **pass** (`multi_pass_answer`, v7 — BBA shows the major and cues `3♦` otherwise; the cue is the pass) | a seat BBA's advancer never gives (0.0% at `advance-x`) — see below |
| opener over `X (2♥)` / `X (2♠)` | floor | **`X` = `len(M, 4..)`** alerted `comp:multi-penalty`, else **pass** (`multi_penalty_answer`) | nominally penalty; when the overcaller's major is the other one they correct, and partner has been told where our trumps are. Read: `♥ 4..` / `♠ 4..` (probed) |
| `two_diamond_double` | armed = diamond penalty double | ignored | a diamond penalty double of a Multi is the gate N4b measured null |

That was v1's line: everything deeper the floor's. The design round (grilled
2026-08-15) had settled *against* copying nodes for the pass-or-correct
positions (a finite node shadows the floor; the seat differs — after `- (2♥)`
*opener* acts), against `(3M)` continuations (BBA's advancer never bids one),
and for the read side — their `2♦` as `6+♥ ∪ 6+♠` in the floor's envelopes,
the N1g pattern with a union box — as a **separate second A/B**. The
measurement overturned the first of those (below): the final table (v4) also
authors, all under the same gate —

| seat | table |
| --- | --- |
| responder after `X (2♥) - -`, `X (2♥) - (2♠)`, `X (2♠) - -`, `X (2♥) X (2♠)` | `multi_responder_rebid(M, ran)` on the *resolved* major — **v7 (then default)**: `4NT` = `hcp(16..)`; `2♠` = five spades `hcp ≤8` (hearts resolved); `X` = **takeout**, four of the other major and ≤2 of theirs (`comp:multi-takeout`) — in the `ran` shape (`X (2♥) - (2♠)`, `X (2♥) X (2♠)`) four spades and 7+ (`comp:multi-penalty`); `3NT` = `points(10..) & stopper_in(M)`; else pass. (v4: `3NT` stopper / `X` = `len(M, 4..)` penalty / pass.) |
| responder after `X (2M) X -` | sit |
| opener after responder's takeout double (`X (2M) - - X -`) | `multi_takeout_answer(M)`: sit with four of theirs, bid the 4-4 fit (`2♠`/`3♥`), else a four-card minor, else `2NT` |
| opener after responder's `ran`-shape penalty double, `2♠`, `4NT` | sit / sit / `6NT` with 17 else pass (`multi_quant_answer`) |
| both after the overcaller's `2NT` heart relay over `2♠` (`X (2♠) X/- (2NT)`) | pass |
| the doubled relay: `2NT (X)`, `2NT (X) 3♣ -`, `2NT - 3♣ (X)` | completion / `multi_relay_rebid` |
| opener over every relay sign-off (`3♦`/`3♥`/`3♠`, all three relay paths), their X of it, their bid over it; responder over their balance | pass (`multi_signoff_pass`, `Pattern::up_to … 7♠`) |

Their `(3M)` jumps, the advancer's `4♦`, and everything past these stay the
floor's.

### Two facts the build corrected

1. **BBA's advancer never passes our double.** The N4b write-up's "they sit
   43% (6 of 14)" was the *foreign* lane — BBA's responder doubling **our**
   `2♦` overcall and *our* advancer passing — mis-read as ours. Re-counted on
   the N4b `len5` and `base` dumps by opener's side: on our-opened
   `1NT (2♦) X` boards the advancer passed **0 of 141 / 0 of 339**; and the new
   `probe-bba-constraints --mode advance-x` (seat 3 over `1NT 2♦ X`, our side
   declared natural as `bba-gen` models us) is **`2♠` 66.9% / `2♥` 33.0% /
   pass 0.0%** — identical to the undoubled relay. So opener's `X -` sit is a
   node BBA never reaches; it stays as the human-partner default and Q3 of the
   design round ("show a four-card major instead") has nothing to decompose
   against BBA. Conditional on our double, the advancer leans weak — the N4b
   `len5` arm saw `2♥` 136 / `2♠` 5 on our-opened boards.

   **The two splits are inverted, and both are right (checked 2026-08-21).**
   The probe reproduces unchanged on the shipped build — NV `2♠` 67.0% / `2♥`
   33.0%, vul `2♠` 73.0% / `2♥` 26.9%, pass 0.0% — while the census arms'
   *realized* auctions over our double are `2♥` **155 / 200 (77.5%)**, `2♠` 45
   (22.5%), pass 0. They measure different populations: the probe deals the
   advancer at random, and its `2♠` bucket is `hcp 6–18` (median 11) against
   `2♥`'s `hcp 2–14` (median 6) — an advancer that strong rarely coexists with
   our `hcp(6..)` double opposite a 15-17 opener. **Size the `ran` shapes off
   the conditional split (`2♥` 77.5%), not the probe's unconditional one.**
   No code changes on this: v7's `ran` tables already carry both legs, and the
   reversible default if it is ever re-litigated is to leave them as built.
2. **The advancer's split is by strength, not shape**, and the correction
   mechanics are BBA's own: over the weak `2♥` the overcaller passes with
   hearts (36.7%), corrects to `2♠` with spades (49.8%), jumps `3M` with a
   seven-carder (13.5%) — `--mode rebid-d-x2h`, i.e. after our double; over the
   invitational `2♠` it bids `2NT` as a heart relay, never `3♥`. There is no
   advancer `3♥`/`3♠` in any probed table.

**Rounds v1–v6:** [archived measurement trail](one-notrump-competitive-closed.md#n4--measurement-rounds-v1v6).

### v7 — BBA's structure minus the artifacts (**measured 2026-08-15 ×3 seeds: SHIPPED** — `2d-multi-v7`, `-v7s2`, `-v7s3`)

Same run shape (v4 base arms by symlink, Multi arm regenerated, priced vs
base and paired vs v4), owned boards, 691.2k bd/vul:

| v7 vs | vul | n | plain /bd | PD /bd | row |
| --- | --- | ---: | ---: | ---: | --- |
| base | NV | 1162 | +0.00019 ±0.00053 | **+0.00100 ±0.00067** | `plain wash \| PD win` — **ship** |
| base | vul | 835 | **+0.00061 ±0.00056** | +0.00061 ±0.00069 | plain win (CI-clear by 0.00005) \| PD wash leaning + |
| base | both pooled | 1997 | **+0.00040 ±0.00039** | **+0.00081 ±0.00048** | win \| win |
| v4 (paired) | NV | 501 | **+0.00075 ±0.00031** | +0.00020 ±0.00038 | better |
| v4 (paired) | vul | 377 | **+0.00041 ±0.00034** | −0.00022 ±0.00043 | plain better, PD wash |

The vul row is `win | wash` — the "doubling artifact, suspect" row as written,
but its domain addendum applies: v7's mechanism is *doubling them more* (the
takeout X, +2.4/+1.6 per fired NV and +2.2/+0.6 vul against v4, and the
penalty passes it produces), the case where PD is blind to the benefit and
plain DD is the arbiter — and PD is not negative anyway. No cell of the
eight is negative; the NV headline is the ship row by the letter; the
both-vul pool is `win | win`. **Shipped.**

What is left on the table (v7 vs base, NV, plain / PD per fired): the
sell-out after they *run* to `2♠` — `X (2♥) - (2♠) -` **−4.03 / −0.57**
(n=65) and `X (2♥) X (2♠) -` −3.61 / +0.56 (n=59) — is the 10–12 hand with
no spade stopper and fewer than four spades, still passing; BBA's blind
`3NT` there was the one blind blast PD tolerated (+0.09 NV, −1.79 vul, plain
+3.9/+3.1). The honest stopper-ask cue (`3♠`) is measured in the residue
round below.
The `X (2♥) - - -` sell-out itself is now plain −2.05 / PD **+2.02** NV
and +0.32 / **+5.01** vul — perfect defense wants us defending BBA's `2♥`,
and the takeout X takes the hands that should not.

Ship mechanics: `their_2d_multi`'s bottom arm is the 2/1 census default
(`--their-2d-multi false` is the pre-ship arm; an explicit Landy-family
declaration without `Multi-Landy` reads as a declared no-Multi),
`vs_bba_agreements` sets `two_diamonds_multi` (so `bba-decompose` replays it
by default — `--multi-counter false` for dumps generated before), the
`[their-landy]` alert-sites anchor arms it (three slugs: `comp:multi-values`
4, `comp:multi-penalty` 16, `comp:multi-takeout` 8; the fenced relay tail
moves `comp:lebensohl-completion` 24 → 44 and `completion` 696 → 680), and
`card.rs` records why no EPBot schema row exists. Engine default stays
undeclared: smoke `18aba5ce…` unchanged.

### Verdict — v7 shipped; v4's numbers, and the read-side follow-up, for the record

v4 (three seeds, owned): **vul `plain wash | PD win`**; **NV `PD win | plain
−0.00055 ±0.00050`** — opt-in by the letter of the gate. Its per-call
decomposition (above) put the whole NV plain deficit on the doubler's second
turn; v6 mimicked BBA's second turn whole and found BBA's takeout double
real and BBA's game bids the DD-declarer artifact; v7 kept the one and
dropped the others and clears the gate. sd-lead could not arbitrate rounds
v1–v7: `ab-dump-sd` has no owner split, and those arms were 60–70% foreign,
so the raw sd is leak-inflated like every other raw number here. **That
caveat does not extend to the residue round** — its pairs are 0-foreign, its
sd files were written and never reported, and they are now in
[§N4 residue](#n4-residue--reader-shipped-stopper-ask-stays-opt-in-measured-2026-08-16).

What the seven rounds established, beyond the numbers: **the floor cannot
hold any seat of this structure** — it sold out with 10+, raised a weak
sign-off to game, pulled both sides' penalty doubles, and cued their relay —
because it read their `2♦` as diamonds and their `2M` as natural. Every one
of those seats is now a book node (the "copy nodes" the design round argued
against and the measurement demanded), and the read-side follow-up (their
`2♦` as `6+♥ ∪ 6+♠`) now gives the remaining floor decisions the same
fact.

### N4 residue — reader shipped; stopper ask stays opt-in (**measured 2026-08-16**)

The temporary reader fires only when our side opened `1NT`, the opponents'
first action is their disclosed Multi `2♦`, and
`reading.their_multi_reading` is on. It suppresses both the natural-diamond
read and the advancer's first `2♥`/`2♠` pass-or-correct read, then intersects
the same exact two-box union into sampler and announced inference:
`{ ♥6+ } ∪ { ♠6+ }`. It claims no strength, minor length, or other-major
length. The systems-on overcall strip **declines that one shape outright**
(2026-08-22), so `(1x) 1NT (2♦)` cannot enter this lane. Before then it only
cleared the *profile* flag, which stops this hand reader but cannot un-compile
the book's Multi table — the leak N4e's isolation gate caught; see
[§N4e](#n4e--the-floorless-weak-escape-shipped-default-on-2026-08-22-the-six-card-rung-five-refuted).
It is temporary until a declared-opponent profile can project the opponents'
own authored book.

The independently gated `competition.multi_stopper_ask` has three modes:
`Off`, `FitSearch`, and `OpenerPlaces`. In only the two `ran=true` corrections
to spades, responder may bid alerted `3♠` with 10–12 points, at most three
spades, and no spade stopper. Opener bids `3NT` with a stopper; otherwise it
uses the ordinary deterministic longest-side-suit choice. `FitSearch` lets
responder pass `4♥`, raise a known minor fit to game, or name a remaining
four-card side suit, with the lone `4♣–4♦` branch placed in `5♦` with support
and `5♣` otherwise. `OpenerPlaces` chooses `4♥` or `5m` immediately.

Their double of the ask rebases to the same answers and continuations. Over
their `4♠`, opener doubles with a stopper or four spades and otherwise makes
a forcing pass; after two opposing passes responder names its longest
four-plus side suit at the five level. All resulting games, penalty doubles,
and doubled signoffs are book-owned terminal passes, fenced from the floor.

`scripts/ab-2d-multi-residue.sh` ran four aligned arms — shipped v7,
reader-only, `FitSearch`-only, and `OpenerPlaces`-only — with the other residue
knob pinned off. Three independent seeds (1786812881, 1786813975,
1786815052), 230.4k accepted boards per arm/vulnerability/seed, both
vulnerabilities, `--filter-1nt`, plain DD and PD: 691.2k boards per table row.
Every pair/seed/vulnerability passed `probe-divergence --gate-opener ours`
with **zero foreign boards**.

| arm vs shipped v7 | vul | fired | plain /bd | PD /bd | verdict |
| --- | --- | ---: | ---: | ---: | --- |
| reader | NV | 321 | −0.0001 ±0.0003 | +0.0004 ±0.0004 | wash \| wash |
| reader | both | 174 | +0.0001 ±0.0003 | **+0.0006 ±0.0004** | wash \| win |
| `FitSearch` | NV | 84 | **+0.0006 ±0.0002** | +0.0001 ±0.0002 | win \| wash |
| `FitSearch` | both | 58 | **+0.0004 ±0.0002** | −0.0001 ±0.0002 | win \| wash |
| `OpenerPlaces` | NV | 84 | **+0.0006 ±0.0002** | +0.0001 ±0.0002 | win \| wash |
| `OpenerPlaces` | both | 58 | **+0.0004 ±0.0002** | −0.0001 ±0.0002 | win \| wash |

Across both vulnerabilities the reader summed **−29 plain / +643 PD IMPs**
on 1.3824m boards: no honest-score downside and a CI-clear pooled PD gain.
That is the repository's `plain wash | PD win` ship row, so
`their_multi_reading` is default-on (still inert without the disclosure).

Both stopper continuations instead land on the table's `plain win | PD wash`
doubling-artifact row, so the ask remains default `Off`. Their direct paired
comparison was a statistical tie: NV only two contracts differed
(`FitSearch − OpenerPlaces` +4 plain / −4 PD IMPs); vulnerable, none did.
Because no stopper mode passed independently, there is no selected reader +
stopper stack and the conditional combined confirmation arm was skipped.

#### The sd-lead third scorer — written 2026-08-16, reported 2026-08-21

`ab-2d-multi-residue.sh` called `sddiff` on every pair and the files were
never read (`ab-results/2d-multi-residue/seed-{1,2,3}/sd.*.txt`, 16 worlds,
230.4k bd per cell). Unlike the v1–v7 arms these pairs are **0 foreign**, so
the leak-inflation caveat does not apply and the numbers stand as a third
scorer:

| pair | vul | seed 1 | seed 2 | seed 3 |
| --- | --- | --- | --- | --- |
| `FitSearch` vs base, sd-plain | NV | **+0.0008 ±0.0004** | **+0.0007 ±0.0004** | **+0.0006 ±0.0003** |
| `FitSearch` vs base, sd-plain | both | **+0.0007 ±0.0004** | +0.0003 ±0.0003 | +0.0003 ±0.0003 |
| `FitSearch` vs base, sd-PD | NV | +0.0005 ±0.0004 | +0.0002 ±0.0004 | +0.0001 ±0.0004 |
| `FitSearch` vs base, sd-PD | both | +0.0004 ±0.0004 | −0.0001 ±0.0004 | −0.0001 ±0.0004 |
| reader vs base, sd-plain | NV | −0.0005 ±0.0006 | +0.0002 ±0.0006 | +0.0002 ±0.0006 |
| reader vs base, sd-PD | NV | +0.0001 ±0.0006 | +0.0004 ±0.0006 | **+0.0007 ±0.0007** |
| reader vs base, sd-PD | both | +0.0000 ±0.0007 | +0.0006 ±0.0006 | +0.0005 ±0.0006 |

`OpenerPlaces` is within ±4 IMPs of `FitSearch` in every cell (their direct
`search vs place` sd pairs fire on 0–4 boards), so the two remain tied on this
scorer as well.

**The verdicts do not move.** `FitSearch` is sd-plain positive in all six
cells — three CI-clear at NV, one CI-clear vulnerable, two vulnerable cells
(+0.0003 ±0.0003) sitting on the boundary — and sd-PD wash-to-positive. That
is the *same* `plain win | PD wash` doubling-artifact signature on a third
scorer, not an escape from it, so **`multi_stopper_ask` stays default `Off`**.
The reader's sd is a wash on plain and leans positive on PD, consistent with
the DD reading that shipped it. What changes is only the record: the sentence
in the verdict above claiming sd "could not arbitrate any round" was wrong
about this round.


### N4e — the floorless weak escape (**SHIPPED DEFAULT-ON 2026-08-22; the six-card rung, five refuted**)

The v7 counter and its reader shipped against the strong half of the `(2♦)`
bucket. This is the **pre-N4e** re-decomposition of what was left, read with
`probe-1nt-interference --bucket "2♦" --responses 6` on the shipped arms
`ab-results/anchor-confirm/2026-08-21-1e9a47e2/american-{none,both}` (seed
1787064872, 204,800 bd/vulnerability, NV+vul pooled below).

**Read the pre-ship size first.** −744 IMPs over 409,600 boards is **−0.0018
IMPs/board of the arm** — 0.3% of the −0.48/−0.59 gap to BBA — and per *board*
`(2♦)` is not the worst cell: `3♠` −1.49, `4+` −1.13, `3♣` −1.09 all beat it.
`(2♦)` leads the bucket table because their Multi fires twice as often as
anything else (816 of 3,115 contested boards), not because we handle it
uniquely badly. This is hygiene at the standard ship gate.

#### By responder's first call (816 bd)

| our response | bd | plain tot | plain/bd | PD tot | PD/bd | reading |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| **Pass** | 264 | **−452** | −1.71 | −226 | **−0.86** | real on both scorers |
| **`X`** | 200 | **−203** | −1.02 | **+133** | **+0.67** | plain loss / PD win — artifact |
| **`2♠`** | 57 | **−118** | −2.07 | −71 | −1.25 | n too small to separate from `2♥` |
| `3♣` | 45 | −15 | −0.33 | −27 | −0.60 | — |
| `2NT` relay | 79 | −6 | −0.08 | +11 | +0.14 | **the relay lane is free** |
| `3NT` | 6 | −4 | | −11 | | — |
| `3♥` | 61 | −2 | −0.03 | +6 | | — |
| `2♥` | 52 | +5 | +0.10 | +77 | +1.48 | — |
| `3♦` | 37 | +37 | +1.00 | +22 | +0.59 | — |
| other | 15 | +14 | | +6 | | — |

Three rows carry **−773**; everything else nets **+29**.

#### Pass, split by responder's hand (the probe's own classifier)

| class | bd | plain/bd | PD/bd | plain tot | PD tot |
| --- | ---: | ---: | ---: | ---: | ---: |
| **≤5 hcp, 6+ suit** | 37 | **−3.92** | **−4.84** | −145 | −179 |
| ≤5 hcp, 5-card suit | 97 | −1.73 | −0.84 | −168 | −81 |
| ≤7 hcp, no 5-card suit | 130 | −1.07 | **+0.26** | −139 | +34 |

The 6+-suit class is the worst per-board cell anywhere in this campaign —
worse than N2d's `(2♠)` twin (25 bd, −3.08/−2.08) and 1.9× its total. The
bottom row is PD-positive, i.e. mostly the DD-declarer artifact.

#### `X`, split by the advancer and opener's answer

| continuation | bd | plain/bd | PD/bd | plain | PD |
| --- | ---: | ---: | ---: | ---: | ---: |
| `X (2♥) - - -` sell-out | 48 | −1.94 | **+1.02** | −93 | +49 |
| `X (2♥) - (2♠) -` | 22 | −2.00 | +1.18 | −44 | +26 |
| **`X (2♥) X (2♠) -`** | 35 | **−2.40** | **−0.80** | −84 | −28 |
| `X (2♠) …` (whole leg) | 45 | +0.82 | +2.13 | +37 | +96 |

The heart leg loses, the spade leg wins, and every sub-cell except
`X (2♥) X (2♠)` is PD-positive. **Only that one is real on both scorers**:
responder doubles, the advancer bids the weak `2♥`, opener makes the penalty
double with four hearts (`multi_penalty_answer`), they run to `2♠`, and
`multi_responder_rebid(Spades, ran=true)` passes. It is already owned by
`multi_stopper_ask`, which stays `Off` (§N4 residue), so **the `X` lane gets
no new work**: −1.02 plain but **+0.67** PD, the sell-outs are the
DD-declarer artifact, and perfect defense says our pass is right.

#### The `2♠` row is not separable from `2♥`

−2.07 ± ~1.5 against `2♥`'s +0.10 ± ~1.8 → a difference of 2.2 ± 2.3. Pooled,
the natural two-level escape is **109 bd, −1.04 plain / +0.06 PD**. Same shape
as N3 round 8's refuted suit gradient, so **no suit gate is authored** on the
natural escape.

#### Root cause — in code

`multi_2d_responder` ([rubensohl.rs](../../src/bidding/american/competition/rubensohl.rs))
has these finite rows, in weight order: `4♣`/`4♦` Leaping Michaels
`points(10..)`, `3♣` Stayman `points(10..)`, `3♦`/`3♥` transfers `points(9..)`,
`3NT` `points(10..)`, `3♠`→♣ `points(10..)`, natural `2♥`/`2♠`
`len(M,5..) & points(..=8) & hcp(5..)`, `2NT` relay
`points(..=8) & (5+ any suit) & hcp(6..)`, `X` `hcp(6..)`, `Pass` 0.

**Below 5 HCP, `Pass` is the only finite row in the table.** Between 5 and 6
HCP it is the only row unless responder holds a five-card *major*:

- 4 hcp with 6 hearts → pass (`96.KJT975.82.972`, −13)
- 4 hcp with 6 clubs → pass (`J.8542.96.QJ9853`, −11)
- 2 hcp with 7 diamonds → pass (`4.T63.QT97542.86`, −11)
- **5 hcp with 6 clubs → pass, one HCP under the relay floor** (`632.7.QJT.Q97532`, −9)

Meanwhile the `2NT` relay — where those hands want to go — measures **−0.08
plain / +0.14 PD** for the hands that clear its floor, and its landing spots
are all authored (`multi_relay_rebid` gives `3♦`/`3♥`/`3♠` or a pass in `3♣`;
`multi_signoff_pass` keeps opener quiet).

The standing objection is `lebensohl_relay_shape`'s **PD-distilled** 6-HCP
floor ([lebensohl.rs](../../src/bidding/american/competition/lebensohl.rs)): a
sampled double-dummy gate "declines nearly every sub-6 hand — pushing a
near-bust to the 3 level loses on DD, even with a 6-card suit". Two reasons
that does not settle *this* lane: it was distilled over a **natural** overcall
whose suit is known, and it is a **DD** gate, the regime that systematically
flatters defending. The census's PD column — −4.84/bd on the 6+-suit class —
is the perfect-defense answer to the same question, and it says the pass is
losing. Which of the two is right is the A/B's job, and it is why the
five-card band gets its own arm.

Second gap: **`1NT (2♦) 2♥/2♠ (anything)` had no book node.** Only
`{their} 2M -` was wired (opener's sign-off raise, under `natural_floor`);
their competition over our escape fell to the floor, which in the dumps bid
`4♥` (their suit) over partner's `4♣` for −1100 doubled, and doubled `4♠` into
twelve tricks. The iron rule ("complete the convention — both sides'
continuations **and the interfered tails**") was unmet, and widening the
escape sends more traffic into it.

#### The package — `competition.multi_weak_escape: Option<u8>`

**One field, three states** — `None` (default, byte-identical), `Some(6)`,
`Some(5)`: the minimum suit length that may act with **no HCP floor** over
their *declared* Multi. One knob rather than two booleans, so the two arms of
the A/B are the same measured change at two settings. Three rungs move
together:

| rung | change under `Some(n)` |
| --- | --- |
| natural `2♥`/`2♠` (140) | `len(M, n..)` with no HCP floor, alongside the existing `len(M,5..) & hcp(5..)`. At `n = 5` this simply removes the floor for five-card majors |
| `2NT` relay (135) | `multi_relay_shape()` is unioned with `len(any, n..)`, no HCP floor — the only outlet a six-card *minor* or diamond suit has, since the natural escape is majors-only |
| opener's sign-off raise | `lebensohl_signoff_raise` is fed `0` instead of `natural_floor_hcp` (5), so its `23 − resp_floor` game bar rises exactly as far as the reading `project_authored` publishes falls (`hcp 5..8` → `hcp 0..8`). Getting that pair out of step is the reading-drift failure mode, not a cosmetic detail |

A six-card major is itself evidence their Multi is the *other* major, which is
what makes a two-level escape safer here than over a natural overcall — hence
the length gate rather than a flat floor removal.

The same knob authors the **interfered tail** (`multi_escape_overcalled`):
`1NT (2♦) 2M (X | 2♠ | 2NT | 3♣/3♦/3♥/3♠)`, opener's one answer, on N1f's
`landy_cue_overcalled` doctrine one level lower — the competitive raise on a
fit with a maximum, the values double when there is no raise to make, and
Pass for everything else, which is *safe* because responder has shown a long
suit and at most eight. Their double is a pure sit: running a known 5-3 fit
out of a doubled two-level partial is the disaster the escape was authored to
avoid. Above `3♠`, and everything past opener's answer, stays floor.

Verified on the four census hands (`probe-decision`, with the new
`PROBE_THEIR_2D_MULTI` / `PROBE_MULTI_WEAK_ESCAPE` env vars — before them
*every forensic in this lane silently probed the natural `(2♦)` leg*):

| hand | `None` | `Some(6)` | `Some(5)` |
| --- | --- | --- | --- |
| `632.7.QJT.Q97532` (5 hcp, 6♣) | `P` only | **`2NT`** 1.350 | `2NT` |
| `96.KJT975.82.972` (4 hcp, 6♥) | `P` only | **`2♥`** 1.400 | `2♥` |
| `4.T63.QT97542.86` (2 hcp, 7♦) | `P` only | **`2NT`** 1.350 | `2NT` |
| `JT987.2.T5.86542` (5♠/5♣, 1 hcp) | `P` only | **`P`** (arm isolation) | **`2♠`** 1.400 |

The default system is byte-identical: `smoke-default` was run at `main` HEAD
and at the built tree and the dumps hash the same. The alert invariant holds
on a new `their-multi-escape` gated profile (the relay keeps `LEBENSOHL_RELAY`;
the natural `2M`, the raise and the values double are all natural).

#### The A/B — `scripts/ab-2d-multi-escape.sh`, two arms

`bba-gen --filter-1nt --their-2d-multi --ns-multi-weak-escape off|6|5`, arms
**sequential**, one fresh `SEED_BASE=$(date +%s)` shared across all three,
never rebuild in flight.

| arm | setting | priced against | target cell |
| --- | --- | --- | --- |
| `base` | `None` | — | — |
| `six` | `Some(6)` | `base` | 37 bd, −145 plain / −179 PD |
| `five` | `Some(5)` | `base`, and **paired** vs `six` | adds the 97-bd five-card class (−168 / −81) |

Both vulnerabilities, plain **and** PD, headline as IMPs per *accepted* deal
with `per-board = conditional mean × trigger density` alongside;
`probe-divergence --gate-opener ours` must read **0 foreign** on every pair
before any headline is quoted. The `five` vs `six` paired comparison is what
says whether the five-card band earns its own default — N3 rounds 6–7 are the
precedent for splitting rather than bundling, and a `six` win plus a `five`
loss is a live and expected outcome.

On the winning arm, re-run `probe-1nt-interference --bucket "2♦" --responses 6`:
the Pass row's `≤5 hcp, 6+ suit` class should shrink toward zero boards and the
`2NT`/`2♥`/`2♠` rows should absorb them without going negative.

#### Round 1 (2026-08-21) — the A/B never ran: the strip leaked the Multi table

Two launches, both killed by the pre-registered isolation gate. Neither
produced a headline; neither vulnerable half ran.

| run | `SEED_BASE` | boards/arm | divergent | foreign (`theirs` opened) |
| --- | --- | --- | --- | --- |
| `ab-results/2d-multi-escape-gatefail/` | 1787325027 | 230,400 (NV only) | 260 (0.11%) | **26 (10.0%)** |
| `ab-results/2d-multi-escape/` | 1787327781 | 230,400 (NV only) | 267 (0.12%) | **27 (10.1%)** |

Every foreign board is the same shape — `(1x) 1NT (2♦) …`, our `1NT` as an
*overcall*:

```
opened 1♦ by them
  on:  - 1♦ 1NT 2♦ 2♠ 3♦ 3♠ - - 4♦ - - -
  off: - 1♦ 1NT 2♦ 2♠ 3♦ 3♠ - - 4♦ - - 4♠ 5♦ X - - -
```

**Root cause — a reading seam, not the package.** `defensive()` grafts the
opening-1NT book below each `(1t) 1NT`, and `systems_on_overcall_strip` re-reads
the lane as `P* 1NT (2♦) …` so the advancer's Stayman/transfers decode off that
structure. But the strip re-keys into the **main competition book**, whose
`1NT (2♦)` leg is chosen Multi-or-natural at *build* time
(`defense_2d_multi`, `american/competition/lebensohl.rs`) — so the strip's
existing `profile.their.two_diamonds_multi = false` (`inference/read.rs`) stops
the hand reader but cannot un-compile the Multi table. The whole Multi leg,
N4e's floorless escape included, was published in a lane where their `2♦` is a
*response* to their own opening, and the inference-aware floor bid it:
partner's `2♠` in `1♥ 1NT 2♦ 2♠ 3♦` read `points 5..=8` at `None` and
`points 0..=8` at `Some(6)`.

**The graft was never the channel.** `register_one_nt` authors only uncontested
keys — `[1NT, 2♦]` is not even a prefix of the grafted trie — so nothing below
`(1x) 1NT (2♦)` is authored at all; every call there is the floor's, steered by
the reading. (A first attempt that cleared the disclosure inside the graft was a
no-op for the same reason: no build-time row in `register_one_nt` reads
`decision.their`.)

**Fix (2026-08-22).** The strip declines that one shape: under a declared
`their.two_diamonds_multi`, `(1x) 1NT (2♦) …` is not stripped, and the natural
walk reads their real diamonds instead. Default system byte-identical
(`smoke-default` `39ca60a2…`, and the declaration is off by default); pinned by
`multi_weak_escape_stays_out_of_the_overcall_lane`.

**What it costs, and the alternative.** Declining loses the *borrowed* natural
Lebensohl reading in that one sub-lane: with the Multi declared, partner's `2♠`
now reads `points 0..=37, ♠ 4..=13` where it read `5..=8, ♠ 5..=13` before, in
**both** arms. The arms therefore agree (the gate is the point), but the
anchor's absolute base in this lane moves. The alternative that keeps the
natural leg is a second, Multi-blind competition book carried on the
`Partnership` for the strip to read against (`opponents: Option<Arc<Partnership>>`
is the existing precedent) — ~50 lines plus a second bound book under N4
configs, and it must be built *after* the floors or floor-authored calls read as
nothing. Deferred, not refuted.

**Sibling, flagged not fixed.** `their.two_clubs_landy` is the same
misapplication one suit lower — `(1♦) 1NT (2♣)` gets the Landy counter-defense
though their `2♣` there is a response. Left at today's behavior deliberately:
clearing it moves the Landy campaign's measured base. User's call.

#### Round 2 (2026-08-22) — measured ×2 seeds: `six` ships, `five` is refuted

`sha=d54ef73f`, 24 shards × 9,600 = **230,400 bd/arm/vul**, `--filter-1nt
--their-2d-multi`, both vulnerabilities, arms sequential.
`SEED_BASE` **1787340263** (`ab-results/2d-multi-escape`) and **1787341972**
(`-s2`). Headline is IMPs per accepted deal — `--filter-1nt` is applied before
any bidding, so the 230,400 boards of an arm *are* the accepted deals.

**`six` (`Some(6)`) vs `base` — `plain wash | PD win`, replicated.**

| seed / vul | fired | DD plain | DD PD | sd plain | **SD-PD** |
| --- | --- | --- | --- | --- | --- |
| 1 / none | 244 (0.11%) | +174, +0.0008 ±0.0008 | **+312, +0.0014 ±0.0010** | +123, +0.0005 ±0.0008 | **+270, +0.0012 ±0.0010** |
| 1 / both | 183 (0.08%) | +29, +0.0001 ±0.0008 | +103, +0.0004 ±0.0010 | −70, −0.0003 ±0.0008 | +5, +0.0000 ±0.0010 |
| 2 / none | 207 (0.09%) | +38, +0.0002 ±0.0007 | +116, +0.0005 ±0.0009 | +43, +0.0002 ±0.0008 | +139, +0.0006 ±0.0009 |
| 2 / both | 165 (0.07%) | −2, −0.0000 ±0.0008 | +50, +0.0002 ±0.0010 | +32, +0.0001 ±0.0009 | +80, +0.0003 ±0.0010 |
| **pooled** | 799 | **+239, +0.00028 ±0.00039** | **+581, +0.00063 ±0.00049** | +128, +0.00013 ±0.00041 | **+494, +0.00052 ±0.00049** |

All four PD cells and all four SD-PD cells non-negative; **one** negative
reading in sixteen (sd plain, seed 1 vul, −0.0003, well inside its CI). The
knob's mechanism is *bidding more*, not doubling more, so the domain addendum
does not apply and PD is a real arbiter: this is the `wash | win` one-sided bet
— never loses on the honest scorer, gains when they punish — with the SD-PD
bracket agreeing at +0.00052 ±0.00049. **Shipped default-on.**

**`five` (`Some(5)`) vs `base` — the doubling-artifact row.**

| seed / vul | fired | DD plain | DD PD | sd plain | SD-PD |
| --- | --- | --- | --- | --- | --- |
| 1 / none | 525 (0.23%) | +429, +0.0019 ±0.0010 | +272, +0.0012 ±0.0013 | +453, +0.0020 ±0.0010 | +384, +0.0017 ±0.0013 |
| 1 / both | 428 (0.19%) | −25, −0.0001 ±0.0011 | **−377, −0.0016 ±0.0015** | +112, +0.0005 ±0.0011 | −156, −0.0007 ±0.0015 |
| 2 / none | 493 (0.21%) | +410, +0.0018 ±0.0009 | +206, +0.0009 ±0.0012 | +517, +0.0022 ±0.0010 | +383, +0.0017 ±0.0012 |
| 2 / both | 420 (0.18%) | −38, −0.0002 ±0.0011 | **−386, −0.0017 ±0.0015** | +47, +0.0002 ±0.0012 | −290, −0.0013 ±0.0015 |
| **pooled** | 1866 | **+776, +0.00085 ±0.00051** | **−285, −0.00030 ±0.00069** | +1129, +0.00122 ±0.00054 | +321, +0.00035 ±0.00069 |

A CI-clear plain win that PD erases, with the vulnerable PD cell **CI-clear
negative on both seeds** — the artifact row verbatim. Not shipped.

**`five` paired against `six` — `six` is the right rung.**

| seed / vul | fired | DD plain | DD PD | SD-PD |
| --- | --- | --- | --- | --- |
| 1 / none | 282 (0.12%) | +255, +0.0011 ±0.0006 | −40, −0.0002 ±0.0008 | +111, +0.0005 ±0.0008 |
| 1 / both | 246 (0.11%) | −54, −0.0002 ±0.0007 | **−480, −0.0021 ±0.0011** | −202, −0.0009 ±0.0010 |
| 2 / none | 289 (0.13%) | +371, +0.0016 ±0.0006 | +89, +0.0004 ±0.0008 | +220, +0.0010 ±0.0008 |
| 2 / both | 256 (0.11%) | −37, −0.0002 ±0.0008 | **−437, −0.0019 ±0.0011** | −262, −0.0011 ±0.0010 |
| **pooled** | 1073 | +535, +0.00057 ±0.00034 | **−868, −0.00095 ±0.00048** | −133, −0.00013 ±0.00045 |

The pre-registered split landed exactly as written: the five-card band is what
`lebensohl_relay_shape`'s PD-distilled floor was distilled against, and freeing
it buys plain-DD contracts that a competent doubler collects. **`Some(5)` stays
opt-in** — the same `Option<u8>`, third state.

**Gates.** After the strip fix, **8 of 12 pair-cells read 0 foreign** and four
read exactly **1**: `six:base` none 1/244 and `five:base` none 1/525 on seed 1,
`six:base` both 1/165 and `five:base` both 1/420 on seed 2. Every one is the
same board class — the campaign's **mirror-read leak** on the swapped axis:

```
- 1NT 2♦ 2NT - 3♣ 3♦ X - - -      (on)   3♦x South
- 1NT 2♦ 2NT - 3♣ - 3NT - - -     (off)  3NT East
```

*They* opened `1NT` and *we* overcalled `2♦`, but the trie key `P* 1NT (2♦) 2NT`
is shape-identical to ours and `Phase::of` on that prefix routes it to the
competition book, so their `2NT` decodes off **our** `multi_2d_responder` relay
row — which this knob widens (`points ..=8` reads `6..=8` off and `0..=8` on).
`readers.rs`'s hand reader has exactly this seat gate
(`their_disclosed_overcall` requires the `1NT` to be ours); the book node has
none, and the general cure is the declared-opponent book. Priced: the seed-1
NV board is worth **+3 of the divergent set's +174 plain / +312 PD**, so the
owner split moves NV plain +0.00076 → +0.00074 and PD +0.00135 → +0.00134 —
inside their CIs, no cell changes verdict. Quoted here rather than gated on,
by the same reading N1c shipped under at 1 of 132.

#### Post-ship re-anchor (run 2026-08-23 local) — the loop is closed

`ab-results/anchor-confirm/2026-08-22-053c4fb8/american-{none,both}` replays
the same seed, 1787064872, at 204,800 bd/vulnerability with 100.00% replay and
0 mismatches. The `(2♦)` boards are exactly paired with the pre-ship snapshot,
and none overlaps `d54ef73f`'s overcall-strip fix. Cells below are
`boards / plain total / PD total`:

| slice | pre-ship `1e9a47e2` | post-ship `053c4fb8` | delta |
| --- | ---: | ---: | ---: |
| whole `(2♦)` bucket | 816 / −744 / −80 | 816 / **−689 / +29** | 0 / **+55 / +109** |
| responder Pass | 264 / −452 / −226 | 213 / **−301 / −4** | −51 / **+151 / +222** |
| Pass: ≤5 HCP, 6+ suit | 37 / −145 / −179 | **0 / 0 / 0** | −37 / **+145 / +179** |
| `X` | 200 / −203 / +133 | 200 / −203 / +133 | 0 / 0 / 0 |
| `2NT` relay | 79 / −6 / +11 | 110 / −51 / −1 | +31 / −45 / −12 |
| `2♥` | 52 / +5 / +77 | 62 / −90 / −42 | +10 / −95 / −119 |
| `2♠` | 57 / −118 / −71 | 67 / −74 / −53 | +10 / +44 / +18 |

The 51 passes migrate exactly: 31 to `2NT`, 10 to `2♥`, and 10 to `2♠`.
Bucket plain improves from −0.91 to −0.84 per board; PD moves from
+0.01/−0.24 to +0.08/−0.02 NV/vul. The 111-board N4e-owned divergent set at
table A is **+55 plain / +132 PD**. The new `2♥` total is the worst first-call
row at −90/−42, but the N4e-owned boards entering it are **+33/+57**.

No named continuation crosses −100: `2♥ - -` is 31 boards at −49/−66, and
`2♥ (2♠) -` is 16 at −35/−10. One relay-under-interference board is −16/−17;
it is an isolated outlier, not a package. **N4e landed: the lane is done for
now.** N4-mirror is the next adjacent proposal; regime-widening and its v7
floor retrain remain the trigger for the parked reading knob.

**N2d stays parked**, and now gets the pointer the queue row promised: `six`
shipped, so the `(2♠)` twin's case is a pointer to this round, not a run.


## N4f — opener's balancing seat and the two reading knobs (**measured ×2 rounds 2026-08-22: nothing ships; all three stay opt-in**)

The `(2♦)` bucket's one *named* hole plus the two reading defects
[one-notrump-multi.md](../one-notrump-multi.md) flagged. All three are built, all
three default off, all three inert while their `2♦` is undeclared
(`smoke-default` `39ca60a2…` byte-identical against `main` HEAD). They were
measured twice with `scripts/ab-2d-multi-balance.sh` — three aligned arms, each
pinning the other two off, `--their-2d-multi --filter-1nt` on every arm — and
nothing shipped.

### Phase 0 first: two probes re-ranked the package before any box was spent

**The takeout double the literature prescribes is not what the anchor plays.**
`probe-bba-constraints --mode custom --seat 0 --calls "1NT 2♦ - 2♥"
--filter-call 1NT` (4000 hands/vul, `--min-share 0.005`) at the unauthored seat:

| seat | BBA |
| --- | --- |
| `1NT (2♦) - (2♥) ?` | Pass **94.2%** · `X` **5.8%** = `hcp(15..=17) & len(♥, 5..) & balanced()` |
| `1NT (2♦) - (2♠) ?` | Pass **92.7%** · `X` **7.3%** = `hcp(15..=17) & len(♠, 5..) & balanced()` |

A trump-length **penalty** double of the suit they named, with **no natural
rung at any share** — not the delayed *takeout* double of Multi theory, and not
the `defense_to_weak_two`-derived table the package was originally designed as.

**And opener's seat is not where the bucket's deficit is born.** BBA acts on 6%
of hands there; 6% of 253 bd is ~15 boards, which cannot carry −426 plain at any
plausible per-board value. Combined with N4e having already absorbed the Pass
row's worst class (37 bd, −145/−179), the residual target is ~227 bd at
**−307 plain / −47 PD** — a plain-only deficit. N4f is disaster removal at the
standard gate, not the bucket's cause. Sized here so no later round re-derives it.

**Responder's first call is not the leak either** (`--mode counter`, 8000 hands
NV): BBA's `4♠`/`4♥` band is `hcp(6..=16) & len(M, 6..)` — a **six**-card major
and the *strong* half, which our `3♦`/`3♥` transfer already reaches — so the
long-major jump this round considered was dropped before it was built. The one
seam the probe does confirm is BBA's natural **minor** single-suiter (`3♣` 2.3%
`hcp 5–12`, `3♦` 1.6% `hcp 4–12`, both median six cards, 3.9% together), which
our table has no home for: `3♣` is Stayman and `3♦` is the heart transfer.
Unbuilt — opposite 15–17 the contract is usually `3NT` — but recorded, because
this campaign had previously dismissed the band as rare and it is not.

### The three knobs

| knob | what it authors | pre-registered verdict row |
| --- | --- | --- |
| `competition.multi_balance` | `1NT (2♦) - (2M) ?`: `X` = `len(M, 5..)` (penalty, alert `comp:multi-penalty`), else pass; plus responder's sits quiet and over their runout. `multi_penalty_answer`'s four trumps raised to five — partner *passed* rather than doubling, so opener is short of the values half and only length acts | mechanism is *doubling more* → measurement.md's domain addendum: plain DD is the arbiter, `plain win \| PD wash` ships (v7's row) |
| `reading.their_multi_advance_reading` | their advance as the whole pass-or-correct ladder: suppression widened to `2♥/2♠/3♥/3♠/4♣/4♦/4♥/4♠` via a Multi-only `multi_advance_ladder` (**not** by widening the shared `advancer_artificial` — the Landy reader shares it). Round 1 also carried `♥3+ & ♠3+` on the jump rungs; **refuted and removed** (below) | moves what the floor *believes*, not what it doubles → PD is a real arbiter, needs `wash \| win` or better |
| `reading.their_multi_double_reading` | `1NT (2♦) X` reads its authored `hcp(6..)` instead of the generic `DoubleStyle` 8+ | as above |

Measured readings, before and after (`probe-call-reading --their-2d-multi`):

| auction | base | armed |
| --- | --- | --- |
| `1N (2D) X (3H)`, RHO | **♥ 6..13** | ⊤ on ♥ alone, then `♥ 3..13 & ♠ 3..13` |
| `1N (2D) X (4D)`, RHO | **♦ 3..13** | `♦` ⊤, `♥ 3..13 & ♠ 3..13` |
| `1N (2D) X (2H)`, RHO | ⊤ | unchanged — a two-level preference can be a singleton |
| `1N (2D) X -`, partner | `points 8..` | `points 6..` |

### Two build notes worth keeping

1. **The suppression half had to be gated too.** A first cut gated only the
   positive claim and widened the ladder unconditionally. `smoke-default` still
   hashed identical — the reader is inert without the disclosure — but every
   *anchor* arm's base would have moved silently, and the A/B's switch would
   have looked like it only added a claim. Both halves now ride the knob.
   The trap generalises: byte-identity of the default system does **not**
   witness isolation for a knob whose lane only exists under a disclosure.
2. **No strength claim is published on the ladder.** The census's advancer
   hands run 1–7 HCP, but `Envelope` has no HCP axis and `points` would fold in
   their distribution (4-4-4-1, 3-4-5-1, 4-3-6-0 …). And `(2♠)` would refuse
   one anyway: `bba-multi-2d.md §2` measures it at `hcp 7–18`, median 11 — the
   *strength-showing* catch-all, not the weak rung. Only `(2♥)` is weak.
   `(4♣)` is included in the ladder on the user's call (`4♣`/`4♦` both land in
   either 4M) but has **zero measured occurrences**; that rung is assumption,
   not evidence, and is flagged as such in `one-notrump-multi.md` open item 3.

### Round 1 (2026-08-22) — measured ×2 seeds: nothing ships, and the positive read is refuted with a mechanism

`ab-results/2d-multi-balance/seed-{1,2}`, `SEED_BASE` 1787402545 / 1787403446,
24 shards × 9,600 = **230,400 bd/arm/vul**, `--their-2d-multi --filter-1nt`,
both vulnerabilities. **All twelve pairs passed `probe-divergence
--gate-opener ours` at 0 foreign** — not one board opened by the other side,
so the mirror-read residue that dogged N4e did not recur.

Headline is IMPs per accepted deal; `/fired` is the conditional mean.

| arm | seed / vul | fired | plain | PD |
| --- | --- | ---: | --- | --- |
| `balance` | 1 / none | 12 | −8, −0.667/fired | −13, −1.083/fired |
| `balance` | 1 / both | 7 | +14, +2.000/fired | +15, +2.143/fired |
| `balance` | 2 / none | 11 | −36, −3.273/fired | −47, −4.273/fired |
| `balance` | 2 / both | 10 | +5, +0.500/fired | −8, −0.800/fired |
| **`advance`** | 1 / none | 40 | **−24** | **−3** |
| **`advance`** | 1 / both | 35 | **−51, −1.457/fired** | **−22, −0.629/fired** |
| **`advance`** | 2 / none | 44 | **−74, −1.682/fired** | **−109, −2.477/fired** |
| **`advance`** | 2 / both | 24 | **−120, −5.000/fired** | **−121, −5.042/fired** |
| `xfloor` | 1 / none | 11 | −10 | +6 |
| `xfloor` | 1 / both | 1 | +10 | +10 |
| `xfloor` | 2 / none | 15 | −6 | −14 |
| `xfloor` | 2 / both | 13 | −39 | −25 |

**`balance` — no verdict, and the ceiling explains why.** 7–12 fired per cell,
signs disagreeing by vulnerability *and* by seed, every per-board CI (±0.0002)
swallowing every effect. The firing rate is not a bug: opener needs five cards
in the major they named, ~6% of hands, exactly matching the anchor's own 5.8%/7.3%
— so the seat's whole reach is ~18 boards per 230,400, and **this knob cannot
be measured at this harness's resolution.** It stays opt-in, unresolved rather
than refuted; an isolated sub-lane harness or an order of magnitude more boards
is what it would take.

**`xfloor` — wash.** 1–15 fired, one cell (`1/both`, n=1) contributing a
±10 IMP swing on a single board. Nothing to read. Opt-in.

**`advance` — negative in all eight cells, and the cause is the positive
claim, not the suppression.** Tracing the worst boards (the iron rule) shows
one repeated shape: our side stops competing because it believes a spade
length the advancer does not have.

```text
[-13 IMP]  N:J98.A98.AQ96.AT3  E:A72.KJT7643.K2.6  S:KQT643..J87.9754  W:5.Q52.T543.KQJ82
  on:  1NT 2♦ 2♠ 3♥ - 4♥ - - -                     South (♠6, ♥void) sells out
  off: 1NT 2♦ 2♠ 3♥ - 4♥ 4♠ - - 5♥ - - X - - -     South saves, they push, we double
```

West's `3♥` there holds **one spade**; on the next-worst board it holds two
(`A3.KQT8763.9.QT8`). The claim `♠3+` is simply false, and it is worth −13 IMPs
a board when it talks a six-card suit out of a save.

`probe-bba-constraints --mode custom --seat 3 --calls "1NT 2♦ 2♠"` (6000 hands
NV) confirms it is systematic, and — usefully — splits the two halves in
opposite directions:

| their call | share | ♥ | ♠ | hcp |
| --- | ---: | --- | --- | --- |
| `3♥` | 38.2% | **2–5 (med 3)** | **2–4 (med 3)** | 7–13 |
| `4♦` | 11.9% | 3–5 | 3–6 | 3–14 |

So the *suppression* half is right and the base read is badly wrong — their
`3♥` is `♥ 2–5`, where the natural walk publishes `♥ 6..13`. The *claim* half
is wrong at both rungs' tails. **The claim is removed**; the knob is now
suppression-only and owes a fresh arm.

**The build lesson, which is the transferable part:** a sound change
(suppression, which only ever removes a possibly-false length) was bundled with
an unsound one (a new positive assertion) behind a single knob, so the A/B
could only say "the bundle loses". Splitting them would have cost one more arm
and identified the culprit directly. *Do not bundle a removal with an
assertion.*

### Round 2 (2026-08-22) — the corrected `advance`, on a clean tree: the false read is also **inert**

`sha=07d135f2` (round 1 ran from an uncommitted tree; this is the citable
round), `ab-results/2d-multi-balance-r2/seed-{1,2}`, `SEED_BASE` 1787406494 /
1787407382, same 230,400 bd/arm/vul. **All twelve pairs 0 foreign** again.

The headline is the **divergence count**, not the IMPs:

| arm | round 1 fired (8 cells) | round 2 fired (8 cells) |
| --- | ---: | ---: |
| `advance` | **143** | **6** |
| `balance` | 40 | 27 |
| `xfloor` | 40 | 52 |

Removing the `♥3+ & ♠3+` claim collapsed the `advance` arm's reach by **96%**.
So essentially the *entire* effect of round 1 was the false assertion, and the
suppression half — which fixes a read that is demonstrably wrong (`♥ 6..13`
published where the advancer holds `♥ 2–5`) — changes **1–2 decisions per
230,400 boards**.

That is the round's real finding, and it is worth more than the verdict:
**a false reading in this lane is very nearly inert.** The contested floor is
not leaning on the advancer's suit length after their Multi, so correcting it
buys almost nothing. Anyone tempted to attack this lane through the read side
should price that first.

| arm | seed / vul | fired | plain | PD |
| --- | --- | ---: | --- | --- |
| `advance` | 1 / none | 2 | +9 | +8 |
| `advance` | 1 / both | 1 | +15 | +18 |
| `advance` | 2 / none | 2 | −13 | −13 |
| `advance` | 2 / both | 1 | −17 | −17 |
| `balance` | 1 / none | 12 | −12 | +4 |
| `balance` | 1 / both | 4 | −31 | −29 |
| `balance` | 2 / none | 7 | +8 | +11 |
| `balance` | 2 / both | 4 | +1 | −6 |
| `xfloor` | 1 / none | 14 | −11 | +5 |
| `xfloor` | 1 / both | 9 | +11 | +34 |
| `xfloor` | 2 / none | 19 | −37 | −27 |
| `xfloor` | 2 / both | 10 | −30 | −16 |

**`advance` — no verdict at n=1–2**, and one that cannot be had from this
harness: a single board swings a cell by ±17 IMPs. Sound by construction,
measured harmless, and inert.

**`balance` — confirmed below resolution.** Its signs flip *between rounds* on
the same cells (round 1 seed-1/vul was +14/+15, round 2 is −31/−29), which is
what noise looks like. Pooled over both rounds — eight cells, 1.84 m boards —
it is **−59 plain / −73 PD IMPs, ≈ −0.00003 IMPs/board**, inside every CI.

**`xfloor` — wash**, seed-1 positive and seed-2 negative in both rounds.

### Disposition — all three opt-in; the one judgment call was withdrawn

All three stay **opt-in, default off**, per the house rule for
rejected-but-interesting treatments. Two are ordinary parks; the third is not.
**Nothing here awaits the user** — the flip proposed below was withdrawn on
2026-08-22 and is kept only as a recorded argument:

- `competition.multi_balance` — **unresolved, not refuted.** Its reach is ~18
  boards per 230,400 because the anchor's own action rate is 6%. Resolving it
  needs a sub-lane harness or an order of magnitude more boards, not another
  seed. A single-dummy re-measure is the cheaper candidate.
- `reading.their_multi_double_reading` — wash; ordinary park.
- `reading.their_multi_advance_reading` — **stays off; no default flip
  proposed.** An earlier draft of this section argued for flipping it on
  because the reading it removes is false (their `3♥` is `♥ 2–5, median 3`
  where we publish `♥ 6..13`) at "no measured cost". **That argument was
  withdrawn**, and it is recorded here because it is a tempting one:

  - It is *ship on analysis alone* wearing a correctness argument. Every
    change feels correct from the inside; that is what the gate is for.
  - "No measured cost" was an overstatement. Six fired boards with ±17 IMP
    single-board swings is **unmeasured**, not harmless.
  - "Correct" is narrower than it sounds: the probe reads *BBA's* advancer at
    *one* forced node. It is correct-against-this-opponent-model, not true.
  - Inertness argues the other way. A falsehood no decision consumes is a
    **latent** bug; flipping it on would hand the next floor retrain an
    unmeasured input, since the regime input reads `Agreements`
    ([card-manifold.md](../ai-bidder/card-manifold.md)).

  *Disposition — a trigger, not a default:* flip it when something makes the
  read live — a contested-floor retrain, or a package that consumes the
  advancer's length. The fix is built, tested, probed and documented, so that
  day costs one flag.

### The retrain trigger fired (2026-08-23) — PD gain, plain target missed; no ship

The v6 corpus had been extracted with `Agreements::default()`: BBA supplied the
teacher calls, but the feature reader still treated its `2♦` as natural and its
`2♣` as natural clubs.  `scripts/dump-v6-their.sh` regenerated the exact same
20 shards, deal slices, cells and seeds with `vs_bba_agreements` plus the two
parked Multi readers.  Targets and tags stayed byte-identical in a paired 5k
smoke; 226 of 52,711 feature rows changed.  The full twin has 6,768,279 rows,
4,235,171 contested, and the same 176→256→256→38 recipe, seed 1, 300 epochs and
30-column fold as shipped v6.

The literal scalar CE gate missed narrowly: v6 on its old heldout features is
0.300980140, while the twin on the corrected features is 0.301202792, delta
+0.000222656.  The paired 676,829-row 95% CI is
[`−0.001115893`, `+0.001561204`], a wash.  On the 3,900 rows whose features
actually changed, old-v6 CE moved 0.45695→0.69374 under the corrected reading;
the twin recovered it to 0.47271.  That large train/serve recovery justified
the pre-registered IMP screen without seed or epoch fishing, but it did not
turn the CE gate into a win.

`scripts/ab-v6-their-reading.sh` compared shipped v6 against the twin served
with both readers, 204,800 accepted deals per arm/vulnerability × two fresh
seeds (`1787439593`, `1787440309`), both scorers:

| seed / vul | fired | plain DD | PD |
| --- | ---: | ---: | ---: |
| 1 / none | 19,322 | +0.0035 ±0.0078 | +0.0109 ±0.0096 |
| 1 / both | 16,426 | +0.0011 ±0.0093 | **+0.0179 ±0.0112** |
| 2 / none | 19,633 | −0.0007 ±0.0079 | +0.0008 ±0.0097 |
| 2 / both | 16,545 | +0.0079 ±0.0093 | **+0.0192 ±0.0112** |
| pooled / none | 38,955 | +0.0014 ±0.0056 | +0.0059 ±0.0068 |
| pooled / both | 32,971 | +0.0045 ±0.0066 | **+0.0185 ±0.0079** |

These are whole `--filter-1nt` floor-swap headlines, not N4 attribution.  The
plan's `--gate-opener ours` requirement was inapplicable: a contested-floor
retrain is supposed to move defensive auctions too, and the first NV pair had
12,289 of 19,322 divergences on boards they opened.  That is scope, not the
mirror-reading leak a one-package gate detects.  N4 was therefore cut directly
from the paired records: baseline opener ours, opening `1NT`, their immediately
following call `2♦`; every other accepted deal contributes zero.

| pooled target slice | fired | plain DD | PD |
| --- | ---: | ---: | ---: |
| none | 484 | **−181, −0.00044 ±0.00064** | +316, +0.00077 ±0.00090 |
| both | 326 | +170, +0.00042 ±0.00068 | **+529, +0.00129 ±0.00088** |
| both vulnerabilities | 810 | **−11, −0.000013 ±0.000468** | **+845, +0.001031 ±0.000629** |

The plain target is a wash, not the required improvement, and the signs reverse
by vulnerability.  The mechanism is likewise a redistribution rather than a
clean repair: Pass replacing v6's double gains +438 plain / +1,328 PD, but the
unchanged initial `2♥` row loses −311/−312, the `2♠` row −121/−163, and new
`3♠` actions lose −194/−309.  Exact `bba-decompose --multi-counter` replay on
seed 1 matched **4,007,503 of 4,007,503** candidate calls.

**Disposition.**  The BBA-reading twin and its reader stack remain opt-in; v6
stays shipped and both reading knobs stay default-off.  The whole-anchor run is
skipped because the pre-registered N4 plain gate already failed.  The broad
vulnerable PD win is a credible lead for a separately registered general-floor
experiment, not evidence that this retrain saved N4.

### Out of scope, found while pricing this — the mirror lane

The census's **table-B** panel, which no doc had quoted, prices *our* `2♦`
overcall of *their* 1NT: **1184 boards, 2.6× table A's 461**. Isolated
(`B-only`, our 1NT uncontested or absent):

| BBA's response to our `2♦` | bd | plain/bd | **PD/bd** | plain tot | **PD tot** |
| --- | ---: | ---: | ---: | ---: | ---: |
| **Pass** | 165 | +0.212 | **−3.382** | +35 | **−558** |
| — passed out (`P P P`) | 77 | +0.714 | **−2.961** | +55 | −228 |
| — `P 2NT P` (we advance) | 20 | −1.750 | **−5.300** | −35 | −106 |
| — `P 2♥ P` | 29 | −0.345 | **−3.621** | −10 | −105 |
| `X` | 562 | +0.452 | +0.731 | +254 | +411 |
| `3NT` | 49 | +2.980 | +3.061 | +146 | +150 |

Plain positive, PD −3.4 per board: the **inverse** of the doubling artifact — a
contract only double-dummy declarer play rescues, which perfect defense beats.
−558 PD IMPs over 204,800 boards is **−0.0027 IMPs/bd of the arm**, larger than
the *entire* contested-1NT lane's headroom (0.0053 NV / 0.0014 vul), from one
call. That is our **defense to their 1NT** ([defensive-overcalls.md](../defensive-overcalls.md),
`nt_defense.rs`), not this campaign, and nothing here touches it — but it is the
biggest single number this round produced and it should be somebody's package.

**Corrected 2026-08-23 by the N4-mirror forensic** — keep the table above as
the record of what was seen, not as the lane's price.  Three things the panel
row could not show:

- It is **one arm**.  The vulnerable arm's `Pass` row is worse (174 bd, −244
  plain / −971 PD), and the two arms are the *same* 204,800 deals at two
  vulnerabilities.
- It is **one post-hoc sub-bucket**, selected by an opponent action we cannot
  see at bid time.  The whole `2♦` lane, both arms, is **2139 bd, +696 plain /
  −397 PD**; the `X` row alone is +450 / +729 and `3NT` is +319 / +328.
- Roughly **half** of the `Pass` row is table-A: on those boards *we* are the
  1NT opener and no 1NT-defense knob can move them (raw points −4,170 plain /
  −39,520 PD at table A against −4,000 / −39,400 at table B).

The lane's real, gateable leak is the **≤7-HCP tail** that `points(8..=14)`
admits through distribution points, plus the unauthored advance.  Full
forensic, candidates and pre-registered decision rule:
[defensive-overcalls.md](../defensive-overcalls.md#defense-to-their-1nt--the-1nt-2-mirror-panel-forensic-2026-08-23).


## N4-KK — the Kokish–Kraft counter, a whole-table variant (**SHIPPED DEFAULT-ON 2026-08-25**)

The one bucket that reopened N4 (see the queue note above): not another rung of
the then-default v7 lane but a **different published table** for the same
object, so it was a new variant rather than a new seed.

`docs/ai-bidder/multi-landy-2d-counter-defense-research.md` surveyed the
counter-defenses to a `1NT (2♦ = one unknown 6+ major)` overcall and found no
consensus — six credible families, differing on the most basic question (is the
immediate `X` values, a major, or a transfer?) and on whether a *later* double
is takeout or penalty. The **Eric Kokish–Beverly Kraft** notes (January 2008,
printed p. 163) carry a table for exactly this object and are the most complete
exact-object package in the survey. `competition.multi_kokish_kraft`
(default **on** since 2026-08-25; `--no-ns-multi-kokish-kraft` falls back to
the v7 table) plays it.

### What moves — five changes at once, deliberately

This is a whole-table swap, registered *instead of* the v7 subtree for
[`landy_bba_entries`][n1j]'s reason: the two disagree on `2NT`, `3♣`, `3♠` and
both delayed doubles, so an overlay would leave v7's rows shadowing these.

| call | v7 (previous default) | K–K (this arm) |
| --- | --- | --- |
| `X` | values, `hcp 6+` (BBA's own band, the 41% workhorse) | invitational-plus values, `hcp 8+`, **no shape promise**; the 6–7 band takes a *designed* neutral pass |
| `-` | nothing authored past opener's floor seat | a **neutral pass with its own delayed table** once they name the major: takeout `X`, natural `2NT`, competitive `3m` |
| `2NT` | the weak Lebensohl relay to `3♣` | **floorless transfer to clubs** (`len ♣ 6+`, no point floor) |
| `3♣` | game-forcing Stayman, Smolen behind it | **floorless transfer to diamonds** |
| `3♠` | forced `3♠`→♣ game force | **both minors, game-forcing, 5-4 or better** |
| second `X` | takeout, four of the other major (v7's one BBA rung that measured positive on both scorers) | **penalty**, v4's trump-length gate — the takeout double moves to the *pass* branch |
| `4♥`/`4♠` | the floor's | the **uncontested direct slam-try tier** copied under the overcall, RKCB ladder included (`hcp 15..=direct_4m_max`, i.e. exactly 15 under the shipped `texas_slam_drive` — see the residue below) |

Unchanged and shared: `3♦`/`3♥` (INV+ transfers to ♥/♠), the weak `2♥`/`2♠`
escape and its whole interfered tail (`multi_weak_escape` composes), Leaping
Michaels `4♣`/`4♦`, and every answer of the double family
(`multi_pass_answer`, `multi_penalty_answer`, `multi_takeout_answer`,
`multi_quant_answer`). `multi_balance` composes — a different seat.
`multi_stopper_ask` goes **inert**: the `3♠` that carried the ask is the
both-minors call here.

The **delayed-double split** is the one structural idea every exact-object
source in the survey agrees on, and the retired v7 table did not have: after an
initial `X`, a second double is cooperative penalty; after an initial *pass*, it
is takeout. v7 has one takeout double and no pass table at all, so the two arms
differ on both halves at once.

### Two design-sketch repairs, both forced by the same thing

The design sketch specified `3NT` as a bare `points(10..)` — "no stopper
requirement (per source)" — ranked at weight 150, above `3♠` (145) and `X`
(130). A bare `points(10..)` **contains every other constructive gate in the
table**, so whatever sits below it is unreachable. Both repairs are recorded at
[`kokish_kraft_responder`](../../src/bidding/american/competition/rubensohl.rs)
and are one-line reversible:

1. **`3♠` now outranks `3NT`** (152 vs 150). The both-minors gate implies
   `points(10..)`, so a higher `3NT` made `3♠` dead code rather than a rare
   rung. The source agrees on the merits — its `3NT` is the last-resort gamble,
   the shape calls come first — and the sketch's stated ordering constraints
   (minor transfers above `3NT`/`3♠`/`2M`/`X`; `2M` above `X`; Leaping Michaels
   above the transfers) say nothing about this pair.
2. **`3NT` keeps its both-majors stopper gate**, unchanged from v4–v7. Measured
   bare, it confines the values double to `points 8..9` —
   `probe-call-reading --their-2d-multi "1N (2D) X -"`
   reads exactly that — which contradicts the *same source's* "`X` =
   invitational **or better**" and re-runs at maximum frequency the stopperless
   blast perfect defense priced at **−3.7/−4.3 a board** in N4 v2/v3. Ranking
   `X` above a bare `3NT` does not rescue it either: the survivors are then
   `hcp ≤ 7` hands with distributional points and no transfer — a 7-count
   4-4-4-1 blasting 3NT — which is worse than dead. **Dropping the gate is a
   recorded sub-arm**, owed its own seed, if K–K's letter is wanted measured.

### Build

- Table: `kokish_kraft_responder` and its ten leaf tables in
  [`rubensohl.rs`](../../src/bidding/american/competition/rubensohl.rs);
  registration in `kokish_kraft_entries`
  ([`lebensohl.rs`](../../src/bidding/american/competition/lebensohl.rs)), which
  the `for over` loop branches to and `continue`s past — the `landy_bba_entries`
  idiom.
- New alert slugs: `comp:kk-values`, `comp:kk-minor-transfer`,
  `comp:kk-two-suiter`, `comp:kk-minors`. The delayed takeout and the repeated
  penalty double **reuse** `comp:multi-takeout` / `comp:multi-penalty`, whose
  claims are identical one branch over. `[kokish-kraft]` is the new
  `tests/fixtures/alert-sites.txt` delta section; `card.rs` records why no
  `.bbsa` row exists (the whole subtree is keyed on a fact about *their* `2♦`,
  which EPBot's schema cannot name).
- Readings come from `.alert(...)` + projection, no hand-written `Inferences`.
  Probed: `X` reads `points 8..` unbounded, the minor transfers read six cards
  with `points 0..`, the delayed `X` reads the other major 4+ with `≤2` of
  theirs.
- Tests: nine `kk_*` cases in `rubensohl/tests.rs` (every rung of responder's
  table, the retired relay/Stayman, the transfers with their two-suiter rebids
  and both competitive tails, the double split, `3♠`, the `4M` tier,
  composition with the escape and `multi_balance`, and inertness without the
  disclosure) plus a tenth pinning the readings; a `kokish_kraft_*` arm in
  `competition/tests.rs`'s package-invariant sweep; two full-auction
  integration tests in `tests/american_competition.rs` (the lane had none); and
  a `their-multi-kokish-kraft` profile in
  `gated_profiles_preserve_alert_invariant`.
- Byte-identity: inert while their `2♦` is undeclared or natural, so the
  default system is unchanged.

### The measurement

Two runs. The first (SHA `78ad4c02`, `SEED_BASE 1787606986`,
`ab-results/2d-multi-kk/`) read the owned lane as the shippable shape but
**failed the isolation gate** at 55% foreign divergence, so no headline could
be quoted from it; that leak was the mirror-read bug, fixed at `29f93561`
([below](#the-mirror-book--why-the-leak-was-not-a-seat-gate)). Its dumps are
dead — the fix moved the v7 control arm, so they no longer pair.

The re-measure is the verdict: `scripts/ab-2d-multi-kk.sh`, SHA `f2ecb3c6`,
**fresh `SEED_BASE 1787615025`**, `ab-results/2d-multi-kk-gated/`, 230 400
boards per arm per vul, both arms `--their-2d-multi` so the table is the only
difference.

**The gate first.** `probe-divergence --gate-opener ours` reads **0 of 683
divergent boards foreign** at none and **0 of 482** at both — 0 of 1165 against
a 55% prior rate. The mirror book holds.

| vul | plain DD | PD | sd-lead plain | sd-lead PD |
| --- | --- | --- | --- | --- |
| none | +0.0002 ±0.0012 | +0.0012 ±0.0015 | +0.0000 ±0.0013 | +0.0009 ±0.0016 |
| **both** | **+0.0019 ±0.0013** | **+0.0023 ±0.0017** | +0.0014 ±0.0014 | +0.0017 ±0.0017 |

IMPs per board, 95% CIs. Per *fired*: plain +0.067 / +0.907 and PD +0.395 /
+1.102 (none/both).

#### By K–K's first countering call

The same clean paired dumps, replayed with `probe-divergence --imps`, attribute
the K–K-minus-v7 swing to K–K's immediate call after `1NT (2♦)`. `reach` is
every board taking that K–K call; `div` is the subset whose final contract
moved. The pooled columns are total IMPs / IMPs per reached call ± 95% CI,
with the zero-swing boards included. They sum exactly to the headline above:
plain **+46 / +437** and PD **+270 / +531** IMPs (none/both).

| K–K call | reach none/both | div none/both | plain total none/both | PD total none/both | pooled plain/reach | pooled PD/reach |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `-` | 884 / 740 | 253 / 185 | −64 / +216 | +187 / +375 | +0.094 ±0.129 | **+0.346 ±0.200** |
| `X` | 335 / 225 | 182 / 125 | −141 / +43 | −19 / +50 | −0.175 ±0.414 | +0.055 ±0.457 |
| `2♥` | 157 / 123 | 20 / 22 | −38 / −2 | −31 / 0 | −0.143 ±0.359 | −0.111 ±0.383 |
| `2♠` | 135 / 105 | 22 / 17 | −2 / +28 | −12 / +32 | +0.108 ±0.311 | +0.083 ±0.320 |
| `2NT` | 143 / 109 | 55 / 34 | +60 / +23 | +60 / +27 | +0.329 ±0.521 | +0.345 ±0.702 |
| `3♣` | 116 / 88 | 99 / 74 | +114 / +92 | +54 / +34 | **+1.010 ±0.798** | +0.431 ±0.937 |
| `3♦` | 90 / 61 | 0 / 0 | 0 / 0 | 0 / 0 | 0 | 0 |
| `3♥` | 92 / 55 | 0 / 0 | 0 / 0 | 0 / 0 | 0 | 0 |
| `3♠` | 43 / 17 | 39 / 15 | +125 / +45 | +42 / +25 | **+2.833 ±1.795** | +1.117 ±2.079 |
| `3NT` | 38 / 28 | 13 / 10 | −8 / −8 | −11 / −12 | −0.242 ±0.736 | −0.348 ±0.856 |
| `4♣` | 11 / 8 | 0 / 0 | 0 / 0 | 0 / 0 | 0 | 0 |
| `4♦` | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | — | — |
| `4♥` | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | — | — |
| `4♠` | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | — | — |

This is **causal accounting for the whole table swap, not fourteen call
ablations**: a same-call row can move on a later K–K continuation or reading,
and a changed-call row inherits its actual v7→K–K counterfactual. The largest
transitions make that selection visible: `X→3♣` gains +195/+144 plain/PD,
`X→3♠` +170/+67 and `-→-` +184/+488, while `3♣→X` loses −309
plain but gains +26 PD and `X→-` loses −79/−219. One seed makes the small
rows descriptive, not separate ship verdicts. These arms predate the separately
measured `multi_minor_slam_try`, so they isolate K–K rather than today's stacked
minor-slam continuation.

#### Against BBA by first call — ranking, not causality

`probe-1nt-interference --bucket "2♦" --responses 1` on the same K–K arms
prices the whole duplicate board against BBA, grouped by our first response.
That is the direct answer to "which calls gained or lost IMPs?", but it carries
the mirror table and every later call, so it **ranks calls rather than isolating
their EV** (the paired table above is the causal package evidence).

| our call | boards none/both | plain IMPs/bd none/both | PD IMPs/bd none/both | plain total none/both | PD total none/both |
| --- | ---: | ---: | ---: | ---: | ---: |
| `-` | 884 / 740 | −0.603 ±0.229 / +0.053 ±0.339 | +0.615 ±0.350 / **+1.622 ±0.503** | −533 / +39 | +544 / +1200 |
| `X` | 335 / 225 | **−1.663 ±0.571** / −0.231 ±0.911 | −0.096 ±0.652 / **+1.676 ±0.995** | −557 / −52 | −32 / +377 |
| `2♥` | 157 / 123 | −0.096 ±0.814 / +0.431 ±1.150 | +1.159 ±0.991 / +1.780 ±1.422 | −15 / +53 | +182 / +219 |
| `2♠` | 135 / 105 | −0.593 ±0.876 / −0.857 ±1.247 | +0.022 ±1.008 / +0.114 ±1.439 | −80 / −90 | +3 / +12 |
| `2NT` | 143 / 109 | **−1.042 ±0.863** / −0.670 ±1.327 | −0.622 ±1.093 / −0.413 ±1.614 | −149 / −73 | −89 / −45 |
| `3♣` | 116 / 88 | +0.172 ±1.056 / −0.091 ±1.586 | +0.871 ±1.268 / +0.693 ±1.826 | +20 / −8 | +101 / +61 |
| `3♦` | 90 / 61 | **+1.700 ±0.945** / **+1.607 ±1.437** | **+1.744 ±1.064** / **+1.672 ±1.528** | +153 / +98 | +157 / +102 |
| `3♥` | 92 / 55 | +0.641 ±0.973 / +0.455 ±1.141 | +0.663 ±1.081 / +0.582 ±1.350 | +59 / +25 | +61 / +32 |
| `3♠` | 43 / 17 | +0.209 ±1.818 / −1.000 ±3.762 | +0.256 ±2.063 / −0.647 ±4.541 | +9 / −17 | +11 / −11 |
| `3NT` | 38 / 28 | +0.763 ±1.211 / +1.036 ±1.874 | +0.474 ±1.273 / +0.357 ±1.984 | +29 / +29 | +18 / +10 |
| `4♣` | 11 / 8 | −0.727 ±3.238 / +2.750 ±5.699 | −0.727 ±3.238 / +2.750 ±5.699 | −8 / +22 | −8 / +22 |
| `4♦`, `4♥`, `4♠` | 0 / 0 | — | — | — | — |

The stable positive row is the heart transfer `3♦`; the large absolute
losses sit in `X`, pass and `2NT`, with pass and `X` reversing sharply under
perfect defense. Those reversals are exactly why the plain and PD columns stay
separate. The tiny four-level rows have no verdict. As above, this snapshot
isolates K–K before `multi_minor_slam_try`; strong `2NT`/`3♣` continuations
in today's stack are measured in [minor-transfer-slam.md](../minor-transfer-slam.md).

#### Inside the two big branches — where `X` and `-` actually bleed (2026-08-26)

The two rows above are the lane's largest absolute movers, so the same arms
were cut one call deeper: `probe-1nt-interference <arm> --dd-cache … --bucket
"2♦" --responses 4`, both vulnerabilities, split by our first call, their
advance, opener's answer and responder's rebid. (The probe now writes its
`--dd-cache` back, so the second cut of an arm costs seconds instead of a
132 774-board DD fan-out.) Board counts are NV+both pooled.

**The `X` branch — the loss is responder having nothing to say.**

| responder's rebid, auction resolved | bd | plain | PD |
| --- | ---: | ---: | ---: |
| **passes** — `X 2♥ P P P`, `X 2♥ X 2♠ P`, `X 2♥ P 2♠ P` | 293 | **−824** | +65 |
| **doubles** — `X 2♥ P P X`, `X 2♥ P 2♠ X` | 44 | **+182** | +191 |
| notrump — `2NT` and `3NT` rungs, all four paths | 96 | −34 | −85 |

`probe-1nt-interference --show … --next "X 2♥ P"` and `--next "X 2♥ X"` say why
the pass-outs bleed, and it is the same shape twice: **five of the sixteen
worst `X (2♥) - -` boards and seven of the twelve worst `X (2♥) X (2♠)` boards
are a 4-4 major fit we never find.** We pass out their resolved partscore for
−110 while BBA, holding our cards, doubles for takeout, hears partner's suit
and makes `4M` — eleven IMPs a board, repeated. That is §N4-KK residue 4, and
it is the branch's largest single hole rather than the rare rung the residue
described. Built 2026-08-26 as `competition.multi_doubler_major`, A/B owed.

**Read the PD column before sizing the repair.** Pooled over the three
pass-out rows PD is +65 — a wash, not a loss — because the `4M` games BBA
reaches on 23-25 points often fail against perfect defense. So this is a
plain-DD repair with a PD non-inferiority requirement, not a both-scorer bet,
and the arm's answer table is deliberately more conservative than BBA's: only a
16-count bids game directly, a 15 invites.

**The `-` branch is a net winner and is not the same problem.** Its four cells
are −533 / +39 plain and +544 / +1200 PD (NV/both) — three of four positive,
**+1250 IMPs net** — so the −0.603 NV-plain cell is one half of a
plain-loss/PD-win pair, the signature the campaign already flags as a doubling
artifact. Inside it the two *authored* action rungs of
[`kokish_kraft_delayed`](../../src/bidding/american/competition/rubensohl.rs) are
the negative part and passing is the positive part:

| pass-branch rung | bd | plain | PD |
| --- | ---: | ---: | ---: |
| delayed natural `2NT` (`hcp 7..=9` + stopper, reachable band `hcp == 7`) | 49 | −41 | **−166** |
| delayed takeout `X` | 145 | −96 | −110 |
| responder passes | 987 | −286 | **+1230** |

Nothing was changed there. The `2NT` rung is the sharper candidate — −3.4 PD
per board, with a mechanism (a 7-count bidding notrump at 22-24 combined with
their known six-card major live) that residue 5 already predicted — but at 49
boards it is below this harness's resolution, and any pass-branch arm risks
+1200 both-vul PD IMPs to chase a −533 NV-plain cell. **Recorded, not built.**

**Two more residues the review priced, neither built.**

- **The delayed takeout `X` requires four of the other major; the only two
  sources that qualify the shape both *deny* it** (Gilles–Roupoil "sans 4AM";
  Système Jean Christophe "without a four-card major"). K–K itself prints
  "takeout" unqualified, so the shipped gate does not violate its letter, but
  it is on the opposite side of every source that specifies one. Not recorded
  anywhere before now. Against re-gating: `multi_takeout_answer` answers the
  double by *bidding* the other major on four, so denying four would land the
  pair in a 4-3 by construction — the two halves would have to move together.
- **Opener's penalty double at `X (2♥)`** (`multi_penalty_answer`) is the
  worst-scoring opener action in the branch: `X 2♥ X` reads −227 plain / +74 PD
  over 160 boards against `X 2♥ P`'s −386 / +222 over 353, and its tail
  `X 2♥ X 2♠ P` is the single worst row in the census (−4.19 plain per board
  NV). The review argued a seat mechanism — opener sits under the six-card suit
  and is the finesse victim, responder sits over it — plus anti-selection (the
  overcaller's pass rate collapses from 55% to 10% once opener doubles, so the
  double mostly drives them to their real suit undoubled). **The comparison is
  confounded**: opener holding four of their six-card suit *selects* misfits,
  which would score worse with or without the double. It needs its own arm, not
  this census.

**Verdict — ships default-on.** Both-vul is the decision table's `win | win`
row on the two arbitrating scorers, NV is a clean `wash | wash`, the sd-lead
tie-breaker agrees in sign in all four cells, and **no reading of the eight is
negative**. The first run's owned-lane figures (PD +0.285/+0.772 per fired)
reproduced in the *raw* totals at +0.395/+1.102, which is exactly what the
handoff predicted would happen once the foreign boards stopped being counted.
`smoke-default` stays byte-identical at 20 000 boards / seed 1: the knob is
inert until an opponent's Multi is disclosed, so the default system does not
move.

### Known residues — priced by the A/B, not fixed in the build

Six consequences of the design, each traced with `probe-decision` /
`probe-call-reading` during the build review. None is a bug; all are what the
arm was actually testing, and each names its reversible alternative. Residue 1
was fixed rather than priced; **3, 4 and 6 are the follow-up queue**, each owed
its own seed and its own rung — the A/B above prices the table as built, and
folding a rung into it would spend the one clean signal it bought.

**Triaged by jdh8 2026-08-25.** All three queue items keep their place, but two
of the three named alternatives are withdrawn and residue 3 is no longer this
lane's: it is every minor transfer's, and the campaign for it is
[minor-transfer-slam.md](../minor-transfer-slam.md). Each item below carries its
ruling.

1. **The mirror lane widens.** ~~The competitive book is keyed by call
   *shape* with no seat gate on the reader side.~~ **FIXED 2026-08-25 — see
   [the mirror book](#the-mirror-book--why-the-leak-was-not-a-seat-gate) below.**
   As measured, when *they* opened `1NT` and *we* overcalled a natural `2♦`,
   their calls decoded off our `1NT (2♦)` counter-table. The shipped lane leaks
   a *strength* claim there; K–K's floorless transfers leak a hard **six-card
   suit**. This was 55% of the A/B's divergence at −1.6/−2.5 PD per foreign
   board, and it is why `probe-divergence --gate-opener ours` must read
   **0 foreign** before any headline is quoted.
2. **The values double loses its `♥ ≤4 / ♠ ≤4` caps.** Base publishes
   `points 8.. ♥ 0..4 ♠ 0..4`; K–K publishes `points 8.. ♥ 0..13 ♠ 0..13` and
   gains `♣ 0..5 ♦ 0..5` instead. Deleting the `2NT` relay removed the rung the
   projector negated the five-card majors from. The reading is *looser, not
   false*, so nothing phantom is claimed — recorded because it is a disclosure
   change the A/B is measuring alongside the bidding.
3. **A strong long minor never doubles.** The transfers are floorless *and*
   uncapped, so a 21-count with six clubs transfers, opener completes
   unconditionally, and the ladder tops out at `3NT` — no slam channel and no
   access to `kokish_kraft_doubler_rebid`. K–K's own transfers are
   invitational-plus; floorless was the design's deliberate change, and putting
   a ceiling on them (transfer below, `X` above) is the reversible alternative.

   **Generalized 2026-08-25 → [minor-transfer-slam.md](../minor-transfer-slam.md).**
   jdh8's ruling: this residue is not N4-KK's. *Every* minor transfer in the
   engine topped out at `3NT` or a placed `5m`, the counter to Landy included,
   and the one slam channel that then existed anywhere — N1j's `4m` on
   `points(13..) & len(minor, 6..)`, whose answer still belonged to the floor —
   was the shape to copy here. That is the campaign-opening state; the Landy
   answer and the other played lanes subsequently shipped in
   [minor-transfer-slam.md](../minor-transfer-slam.md). **The ceiling alternative is withdrawn**:
   it pushes the strong long minor into the values double (whose reading is
   already the looser one, residue 2) and runs the measured N1h/N1i right-siding
   trade (`3♣ ← 2NT`, −2.19 PD) backwards. The rung is `4m` at weight 151,
   between the lowest two-suiter step and `3NT`.

   **SHIPPED default-on 2026-08-25 as `competition.multi_minor_slam_try =
   Some(15)`.** A `points` floor, not a bool, so the A/B carried three arms
   (`off` / `13` / `15`; `scripts/ab-2d-multi-slam.sh`). Two rounds, gate 0
   foreign in every cell; round 2 is 2.3M bd/arm/vul (`SEED_BASE` 1787642695).
   Both floors beat `off` on all eight cells; `15` reads `t` +2.25/+2.61 plain
   and +1.92/+1.83 PD (NV/both), `13` +1.61/+1.85 and +1.45/+1.33. The two
   floors are **not** separated — the head-to-head is the 13–14 slice and reads
   `t` +0.70 plain over 55 fired after reading the *other* sign in round 1 — so
   `15` ships on the narrower trigger, not on beating `13`. Opener's answer is
   authored against the N1 doctrine, on a probe: floored, that seat offers
   `{6NT, 4♥, Pass}` and takes `4♥`. Full write-up in
   [minor-transfer-slam.md](../minor-transfer-slam.md).
4. **The doubler has no takeout.** v7's second `X` is takeout showing four of
   the other major, and it is the one BBA rung that measured positive on *both*
   scorers (+2.4 plain / +1.6 PD per fired NV). K–K's is penalty, so a 12-count
   with four of the *other* major and a doubleton in theirs now passes their
   partscore. This is the delayed-double split, the arm's single biggest known
   risk, and the first thing to trace if the A/B reads a loss.

   **jdh8 rejected this residue 2026-08-25 — it is to be repaired, not priced.**
   Tracing it sharpens the diagnosis: the missing call is not the takeout double,
   it is the **natural other major**. K–K's `X` is "negative and *Stayman-like*",
   and opener already answers it Stayman-style when the advancer passes
   ([`multi_pass_answer`](../../src/bidding/american/competition/rubensohl.rs) shows
   a four-card major). When they *bid* instead,
   [`kokish_kraft_doubler_rebid`](../../src/bidding/american/competition/rubensohl.rs)
   offers `4NT` / penalty `X` / `3NT` / `2NT` / Pass and **no natural major at
   all** — so a 12-count with four of the other major and no stopper in theirs
   fails every gate and takes the weight-0 Pass. The source lists only the
   non-obvious meanings of a call, so authoring the natural rebid is
   transcription, not deviation.

   Proposed repair, one rung, one A/B, one seed: over their `(2♥)`, `2♠` on
   `len(♠, 4..) & len(♥, ..=2) & hcp(8..)` — the cheap seat; over their `(2♠)`,
   `3♥` on `len(♥, 4..) & len(♠, ..=2) & points(10..)` — a level dearer, so a
   level stronger. In the two `ran` shapes opener has already doubled `(2♥)` on
   four-plus hearts ([`multi_penalty_answer`]), so `3♥` there lands in a known
   4-4 fit and is the strongest of the four. This **keeps** K–K's delayed-double
   split, which every exact-object source in the survey agrees on. Reverting the
   second `X` to v7's takeout is the alternative and contradicts all of them; it
   is not the recommended default.

   **SHIPPED DEFAULT-ON 2026-08-26** (`competition.multi_doubler_major`,
   `scripts/ab-2d-multi-doubler.sh`, `SEED_BASE=1787740671`, 2.304M bd/arm/vul,
   isolation gate 0 foreign at both vuls). The census below found this is not a
   rare rung but the branch's largest single hole, and it corrected the
   proposal in two places.

   | vul | fired | plain DD | PD | sd plain | sd PD |
   | --- | ---: | ---: | ---: | ---: | ---: |
   | none | 787 | +3.344 | +0.610 | +4.412 | +2.168 |
   | both | 510 | +1.927 | **−1.737** | +3.399 | +0.243 |

   The design claim is confirmed exactly: **100% of the 1 297 divergent boards
   are "bid where the baseline passed", 0% the other way** — weight 100 never
   moved a call the shipped table already made — and 336 of the 787 no-vul
   divergences are games the baseline never reached.

   **Shipped on jdh8's ruling with the both-vul PD cell open.** That cell is
   the decision table's `win | loss` row, and the sd-lead tie-breaker rescues
   it only to a wash. Traced per measurement.md step 10, the cause is opener's
   *answer* table, not the rung: four of the five worst both-vul PD boards are
   `2♠` played in a **4-2 or 4-3**, because
   [`kokish_kraft_doubler_major_answer`](../../src/bidding/american/competition/rubensohl.rs)
   offers only `4M`/`3♠`/Pass and a 15-17 balanced hand with a stopper in their
   major and short support must pass. The repair (a `3NT`@135) is built under
   `multi_px_split`, and **unbundled 2026-08-26** as
   `competition.multi_doubler_notrump` so it could be priced against the
   shipped default: `scripts/ab-2d-multi-doubler-nt.sh`,
   `SEED_BASE=1787749549`, **4.608M bd/arm/vul** — double this run's, because
   the seat is a subset of its pass-outs (`hcp 16+`, a stopper, short support)
   and its surface measured ~1 in 43 000 against this rung's 1 in 4 500.

   **That repair SHIPPED DEFAULT-ON 2026-08-27, winning all four cells** —
   NV +2.910 plain / +2.096 PD per fired over 167, both-vul **+4.264 /
   +3.264** over 106, 0 foreign on both gates. The hypothesis holds in
   direction (both-vul is the larger cell) but recovers only ~20% of this
   row's deficit in per-board terms, so **this `win | loss` row stays open**
   and the next suspect is the `3♥` leg's gapless `hcp 16+` game answer —
   [multi-doubler-answer-handoff.md](../multi-doubler-answer-handoff.md).

   - **"The two `ran` shapes" is true of one of them.** `multi_penalty_answer`
     doubles their `(2M)` on `len(major, 4..)` at weight 150 against a weight-0
     catch-all, so opener's *pass* over `(2♥)` **denies** four hearts as surely
     as its double **shows** four. `X (2♥) X (2♠)` is the known 4-4 and is
     built; `X (2♥) - (2♠)` is excluded, because `3♥` there finds a 4-3 at
     best.
   - **No shortness conjunct and no separate point floor.** Four of *their*
     major already doubles at weight 155, so the ordering supplies the
     `len(major, ..=2)` cap; and the rung sits at **weight 100**, below every
     existing rung, so it fires on exactly today's pass-outs and cannot move a
     call the shipped table already makes. Residue 4's `points(10..)` on the
     `3♥` leg would have killed four of the seven measured 4-4 heart fits
     (their doublers hold 8–9).
   - **`X (2♠) - -` is withheld pending a ruling.** Opener said nothing about
     hearts there, so `3♥` is a four-card suit at the three level opposite
     unknown support, firing only when the spade stopper is missing — the
     misfits. The census gives that leg 25 boards NV+both worth −30 plain and
     +8 PD: no measured loss to repair. One token in `kokish_kraft_entries`
     re-arms it.

   Opener answers with game in the fit from the top of the range (`hcp 16+`),
   the invitational raise where there is room below game, else a pass;
   responder accepts on `points 11+`.
5. **Two rungs of the delayed table are dead in self-play.** Responder reached
   that seat by passing, and under K–K a weak six-card minor does not pass — it
   transfers. So the source's competitive `3♣`/`3♦` fire only opposite a partner
   who is not bidding this table, and the natural `2NT` beside them is really
   `hcp == 7` rather than the `7..=9` the rule spells. Both are consequences of
   (3); the rungs are kept, documented at
   [`kokish_kraft_delayed`](../../src/bidding/american/competition/rubensohl.rs),
   because deleting them would silently hand those seats to the floor.

6. **Responder's contested channel is two calls wide.** Over their
   pass-or-correct above a minor transfer, responder has `3NT` (game values
   with their now-named major stopped) and `X` (`hcp 10+` without one) — a
   census of 60,000 deals during review found that without the `X` about *half*
   of all game-forcing transferors were book-forced to pass out their `3M`, so
   the double is load-bearing, not a nicety. What is still missing is the
   shortness hand: 10+ points with a singleton or void in their major wants to
   play our minor, and doubling with a void is the wrong call. The reversible
   candidate is a `5m` rung gated on `len(major, ..=1)`; it is not in this
   build because five of a minor needs eleven tricks and the A/B should price
   the two-call table first.

   **jdh8's ruling 2026-08-25: reroute, do not build a `5m`.** The shortness hand
   already took the transfer to reach this seat, so it wants the transfer
   machinery one round on, not a bespoke rung — the same `4m` residue 3 owes,
   one round later and one level lower than the `5m` proposed. Gated
   `len(major, ..=1) & points(10..)` at weight 145 it slots cleanly between the
   two calls that are there: `3NT` (150) with their major stopped, `4m` (145)
   short in it, `X` (140) with neither, Pass. Eleven tricks become ten, and the
   hand that should never double with a void stops having to.

   **BUILT 2026-08-25, on residue 3's knob and in its arm** — reversing the
   earlier plan to give it a separate seed. This seat is residue 3's *interfered
   tail*, and the iron rule is that a convention ships with its tails; splitting
   them would have measured half a treatment. Opener sits on the placement
   (probed: floored, it answers `4♥`). Their **jump** over the completion,
   `{completed} (4M)`, stays unauthored and is recorded in
   [minor-transfer-slam.md](../minor-transfer-slam.md).

### The `P`/`X` information split — `competition.multi_px_split` (MEASURED LOSS 2026-08-27, stays default off)

**Verdict first.** `scripts/ab-2d-multi-px.sh`, `SEED_BASE 1787804916`, sha
`f44b73b9`, 230 400 bd/arm/vul, **isolation gate 0 foreign at both vuls**
(0/50, 0/36). Per fired — NV plain **−0.980** / PD **−1.780**; both-vul plain
**−1.861** / PD **−2.083**. Resolution is 10.56/√n_div = 1.49 (NV) and 1.76
(both), so **three of the four cells are resolved losses and the fourth is a
negative wash**; no cell is positive, and `sddiff` is flat (+0.224/+0.168 NV,
−0.048/+0.279 both, every one inside its ±0.4 CI). The knob stays default off.
`ab-results/2d-multi-px/`.

**The surface came in ~40× thinner than this section predicted.** 50 and 36
divergent boards out of 230 400 — 0.02%, not the "whole X/P frontier" the
script sized for. The exclusion list below is why, and it is the honest
correction to the design note: after the `140`/`180`/`176`/`178`/`150`/`152`
rungs take their share, **8–9 with no four-card major is nearly empty**. The
split is close to inert, and what it does move it moves the wrong way.

**Where the loss comes from** — selected worst tail, so **unverified** as a
population mechanism per [measurement.md](../measurement.md) (no full-dump count
was run). Two clusters: (a) hands that used to double and *collect* now pass —
`off: 1NT 2♦ X 2♥ X - - -` against `on: 1NT 2♦ - 2♥ - - -`, four boards at −8
to −11, which is mechanism 1, the constraint itself; (b) the `2NT`→`3♥`/`4♥`
reroute at −10 to −13, going past a making `3NT`, which is mechanism 2's 148.
Cluster (a) landing on the constraint is what argues against splitting the
package into isolating arms: the first mechanism in the ordering below is
already the leading suspect, and the population is too thin to pay for three
more runs. Recorded, not queued.

The design as built, kept for the record:

The census above is two numbers about one decision: the `X` branch's pass-outs
read **−824 plain on 293 boards** and the `-` branch's pass-outs read **+1230
PD on 987**. K–K's double is a flat `hcp 8+` with no shape promise, so the two
branches are split by strength alone and the doubler's own rebid table is left
with nothing to say on a large slice of its population. jdh8's proposal, built
here, splits them by **information** instead:

| call | `multi_kokish_kraft` | `+ multi_px_split` |
| --- | --- | --- |
| `X` | `hcp(8..)` | `hcp(10..) \| (hcp(8..=9) & (len(♥, 4..) \| len(♠, 4..)))` |
| `-` | everything else | everything else — now including 8–9 with no four-card major |

**The hands that move are the complement of the new disjunct, not the
disjunct.** The constraint *retains* 8–9 with a four-card major; what it sheds
— and what therefore changes branch — is **`hcp 8..=9` with no four-card
major**. That slice is narrower still, because every other 8–9 hand already
outranks the double at 130: a five-card major escapes at 140 or transfers at
180, a six-card minor takes the floorless transfer at 176/178, and an 8–9-HCP
hand with 10+ *points* on distribution takes `3NT`@150 or `3♠`@152. No weight
surgery was needed on any shipped rung.

Two things follow structurally, and both ride the same knob:

1. **The doubler's natural other major becomes required, at weight 148.**
   §N4-KK residue 4's rung (`competition.multi_doubler_major`) sits at 100 —
   below everything, so it fires only on today's pass-outs. Under the split the
   8–9 doubler *is* a four-card major, so the rung is re-priced to **148**,
   uniquely between `3NT`@150 and `2NT`@145. The two knobs emit one rung;
   `px_split` owns the weight.
   - **`X (2♠) - -` is re-armed.** The withholding ruling (25 bd, −30 plain /
     +8 PD, "the misfits") was priced under the *wide* `hcp 8+` double. Under
     the split a hearts-only hand at that node is the population that used to
     sell out, so the leg is armed — reversible by its column in
     `kokish_kraft_entries`.
   - **`X (2♥) - (2♠)` stays excluded on both knobs.** The mechanism there is
     opener's *pass* over `(2♥)` denying four hearts (`multi_penalty_answer`
     doubles on `len(major, 4..)`@150 against a weight-0 catch-all), and no
     responder-side split can change what opener said.
   - **The natural `2NT`@145 does not go fully dead** — it survives on exactly
     the excluded leg, where responder holds strictly four hearts, a spade
     stopper and ≤9 points opposite a pass that denied hearts. The rung stays
     per house style; deleting it would hand that seat to the floor.
2. **The delayed `2NT` stops being a `hcp == 7` relic.** Residue 5 above records
   that rung as unreachable across most of its `7..=9` band, because the
   first-turn pass denied `hcp 8+`. Under the split the pass denies 8–9 only
   *with* a four-card major, so the band is real, and opener answers it with
   `kokish_kraft_invite_answer` (game on `hcp 16+`) instead of sitting. The band
   still reaches down to `hcp 7`, so accepting on 16 can reach `3NT` on 23
   combined — a known cost, and one of the two cells the forensic watches.

**The 148 is a reading literal, not only a routing one.** `bid_exclusion` is on
by default, so at weight 100 the natural-major bid denied the `2NT` it declined
(`hcp(8..=9) & stopper_in(major)`) and at 148 it stops denying it — a reading
move riding a weight, exactly the class
[reading-drift-handoff.md](../reading-drift-handoff.md) is about.

**Three mechanisms ride this one knob** — it was four until 2026-08-27 —
and `px` vs `base` confounds all three: (1) the double's constraint, (2) the
100→148 re-weight, (3) the `X (2♠) - -` leg re-arm, (4) the delayed `2NT`
acceptance. No arm here separates them; isolating any one is a follow-up arm,
and the ordering above is the order to try if the package measures a loss.

**The fourth was the `3NT`@135 out, and it left this package by winning.**
`competition.multi_doubler_notrump` shipped default-on 2026-08-27, so the rung
is in the `base` arm too and this A/B now isolates the information split
alone — a cleaner experiment than the one designed, and the arms of
`scripts/ab-2d-multi-px.sh` were deliberately left unchanged to keep it.
The old coupling argument still holds mechanically: re-weighting to 148 sends
*more* traffic to that answer table (the 8–9 doublers with a stopper, who used
to bid `2NT`), so the split needs the repair under it — it now inherits it
from the default instead of carrying it. Both knobs still emit one rung
(`multi_px_split || multi_doubler_notrump`), so disarming the default with
`--no-ns-multi-doubler-notrump` would put it back in the `px` arm only.

**And the ladder grew one rung lower the same day, also into `base`.**
`competition.multi_doubler_minimum_notrump` shipped default-on 2026-08-27 (a
win in all four cells, `ab-results/2d-multi-doubler-min-nt/`): the 15-count's
`2NT`@120 on the `2♠` leg plus responder's `3NT`@140 acceptance, the rung below
the `3NT`@135 out. It is gated on the notrump out rather than on the split, so
it too is in both arms and confounds nothing here — the same reasoning, one
point lower. The coupling argument gets *stronger*, not weaker: 148 sends more
stopper-holding 8–9 doublers to that answer table, and the table now answers
them from 15 up rather than from 16 up. Details in
[multi-doubler-answer-handoff.md](../multi-doubler-answer-handoff.md) item 2.

**Deliberately not done.** Opener's takeout `X` at `- (2M)` — the pass branch's
mirror of this split — is **skipped on jdh8's ruling**: BBA passes that seat
94.2% / 92.7% and its only action is a trump-length penalty double,
`multi_balance` is twice below resolution, and responder's own delayed takeout
`X` already covers the function. Readings do not move either:
`responder_overcall_double_reading` publishes a flat `points 8..`, which is
still the exact hull of the disjunction.

**Measured**: not yet. Sized at N4-KK scale (230 400 bd/arm/vul) rather than
residue 4's 2.3M, because the divergence surface is the whole 8–9 X/P frontier
rather than one 0.9% rung. Isolation gate first (`probe-divergence
--gate-opener ours`, 0 foreign), both scorers, `sddiff` tie-breaker.

### The mirror book — why the leak was not a seat gate

**Fixed 2026-08-25**, ahead of the re-measure. The diagnosis in the handoff
("keyed by call shape with no seat gate") was wrong in a way worth recording,
because it pointed the repair at the wrong layer.

The K–K table is *already* ownership-keyed: `kokish_kraft_entries` registers
under `Pattern::after("P* 1NT", "(2♦)")`, and an unparenthesized `1NT` is our
side. In the mirror lane the auction is `P* (1NT) 2♦`, which that pattern never
matches for us. Nothing about the table needed a seat gate.

The leak is a **frame flip**. Undeclared opponents are decoded with *our* book,
rebased so that they are "we" (`inference/projection.rs`). Their auction is then
`P* 1NT (2♦)` — genuinely, from their seat — and
`decision.their.two_diamonds_multi`, a fact about **our** opponents, survives
the flip and gets asserted about **theirs**: us. Our natural `2♦` overcall is
not a Multi, and we do not play a counter-defense to ourselves.

No pattern can separate the two frames, because from the bidder's own seat they
are the same auction. The only distinguishing fact is per-side, so the fix is a
per-side book: `System::opponents`, a second build of our own system with
`decision.their` cleared, which their calls decode in. It is built only when
something is declared, so the shipped default carries no second book.

A profile flag would not have reached it. `defense_2d_multi` chooses the
`1NT (2♦)` leg at **build** time — `inference/read.rs` says in as many words
that clearing the classify-time flag "cannot un-compile it" — which is why the
existing local precedent there (the systems-on strip clears `two_clubs_landy`
and `two_diamonds_multi` from the profile) never closed this lane.

Acceptance, both halves:

```sh
# mirror lane — the two arms must now print identical rho reads and logits
PROBE_THEIR_2D_MULTI=1 target/release/examples/probe-decision \
  "AQ54.T8653.7.954" "- 1NT 2♦ X" none
PROBE_THEIR_2D_MULTI=1 PROBE_MULTI_KOKISH_KRAFT=1 \
  target/release/examples/probe-decision "AQ54.T8653.7.954" "- 1NT 2♦ X" none

# owned lane — they must still differ, or the fix turned K–K off
PROBE_THEIR_2D_MULTI=1 target/release/examples/probe-decision \
  "AQ54.T8653.7.954" "- - 1NT 2♦" none
```

**The whole-struct clear, decided 2026-08-25 (jdh8).** The mirror clears the
whole `their` struct, so `two_clubs_landy` goes with it — the same
misapplication one suit lower, which `read.rs` had deliberately declined to fix
at its own site because it moves the Landy campaign's measured base. Fixing it
is correct; the cost is that the Landy campaign's recorded numbers no longer
describe the engine and want re-anchoring. Narrowing
`common::mirror_agreements` to `two_diamonds_multi` alone stays the one-line
reversal.

**Why no leading-pass quantifier could have done this.** A pattern-level
discriminator was considered and rejected: the routed decode cuts the auction at
*their* turn and re-parenthesises it, so the actor index and the fan move
together and the parity of leading passes carries no side information. The
parenthesisation *is* the side marker, and the flip rewrites it by construction.
Only a per-side fact — the agreements — can separate the frames.

Separately, and **not** this change's to fix: `slam::rkcb_rows` offers an
insufficient `5♥` at `… 4NT - 5♥ -` (weight 50, `asker_after_5h`). Registering
the ladder under `1NT (2♦) 4♥` newly exposes it in this lane, but it is
identical in the uncontested tree (`1NT - 4♥ - 4NT - 5♥ -` offers the same call)
and production filters illegal calls at selection. Flagged for the slam module.

### The A/B (2026-08-25, SHA `78ad4c02`, `SEED_BASE 1787606986`, 230 400 bd/arm/vul)

`scripts/ab-2d-multi-kk.sh`, results in `ab-results/2d-multi-kk/`. The
**isolation gate failed exactly as residue 1 predicted** — 843/1530 (none) and
821/1312 (both) divergent boards were opened by *them* — so the script stopped
before quoting raw headlines, by design. The headline is read from the
owner-filtered diffs (`owned.*`, the divergent boards we opened); the foreign
slice is priced separately below.

**Owned lane** (687 fired none / 491 both; per fired, `kk − base`):

| vul | plain DD | PD | SD plain | SD-PD |
| --- | --- | --- | --- | --- |
| none | −0.039 (−27 IMPs, wash) | **+0.285** (+196) | −0.071 | **+0.205** |
| both | +0.305 (+150, wash-to-win) | **+0.772** (+379, sig at the CI edge) | +0.259 | **+0.699** |

Plain wash at both vuls, PD win at both vuls, SD-PD agreeing — the decision
table's `plain wash | PD win` shippable row, *for the lane itself*. The engine
of the win is the **designed neutral pass**: first-diff `−` where v7 doubled
(+141/+335 PD none/both) and `−` where v7 relayed `2NT` (+26/+244). The plain
drag concentrates in `X` replacing v7's `3♣` GF route (−298/−169 plain, PD
≥ 0). The residue tails are visible but not net-negative: `3♠` both-minors on
5-4 drove past `3NT` into their cheap `5♥x` twice at −15…−17, and one floorless
`2NT` transfer freak — the recorded 5-5 fallback and ceiling sub-arms price
those.

**Foreign slice** (they open `1NT`, we overcall a *natural* `2♦`; raw − owned):
plain −55 / PD **−1376** at none (−1.63 per foreign board), plain **−623** /
PD **−2029** at both (−0.76 / −2.47) — at both-vul the leak is a plain-DD loss
too, not a doubling artifact. Raw totals are therefore a clear net loss (none:
−82 plain / −1180 PD; both: −473 / −1650), and a default-on ship today loses.

**Leak mechanism, probed** (`probe-decision`, worst board): with the knob on,
*their negative double of our natural `2♦`* reads as our own K–K `X` — `hcp
8+`, minors ≤ 5, **majors unlimited** — where v7 read `hcp 6+`, majors ≤ 4, a
decent model of what their X actually shows. The poisoned read flips advancer's
floor from `P` (logit 10.5) to `2♥` (11.0), and the cascade ends in doubled
partials (the overcaller even ran to `2♠` on a doubleton). First-diff `2♥`
where v7 passed is alone 238 bd / −321 plain / −1021 PD at none and 246 bd /
−720 / −1416 at both. This is residue 1 made expensive: the fix is an
**ownership gate** — key the K–K table and its readings on *our side having
opened the `1NT`* — after which the gate should read 0 foreign and the script
completes; the owned numbers say the re-measure is then expected to ship. The
`probe-1nt-interference --bucket "2♦" --responses 6` decomposition rides that
re-run.

Recorded follow-ups, each owed its own seed: the `3♠` **5-5** fallback if 5-4
measures badly, the bare stopperless `3NT` sub-arm above, a weight retune
against `probe-decision`, and the `4M` band. On the last:
`direct_4m_max` is `15` under the shipped `notrump.texas_slam_drive`, because
uncontested a 17+ six-card major takes South African Texas and drives its own
RKCB — but under their `(2♦)` those calls are Leaping Michaels, so the 16+ hand
falls back on the `3♦`/`3♥` transfer with its slam try floored. Not a
regression (v7 routes it identically and gains no direct rung at all), and the
fix is one token (`15..=18`), but it is a behaviour change owed its own arm.

[n1j]: #n1j--the-bba-ladder-counter-shipped-default-on-2026-08-15


## N2 — the 2026-08-21 re-read by response

Re-read on the 2026-08-21 arms (`1e9a47e2`, `--bucket "2♠" --responses 8`,
NV+vul pooled, 402 boards). All three signs the 2026-08-15 census established
replicate:

| our response to `(2♠)` | bd | plain | PD | plain/bd |
| --- | ---: | ---: | ---: | ---: |
| **Pass** | 228 | **−231** | +73 | −1.01 |
| **`2NT` relay** | 45 | −32 | **−68** | −0.71 |
| `3♦` (→♥) | 33 | +37 | +29 | +1.12 |
| `3♠` cue (Stayman) | 29 | +46 | +55 | +1.59 |
| **`X`** | 62 | **+86** | +63 | **+1.39** |

`X` still wins, the relay still loses on both scorers, and Pass is a plain
loss that perfect defense nearly recovers (their `2♠` fails and PD doubles it).
Splitting the passes by hand class ranks the two open packages:

| why responder passed over `(2♠)` | bd | plain | PD | plain/bd |
| --- | ---: | ---: | ---: | ---: |
| `≤5 hcp, 6+ suit` — the relay's `hcp 6` floor (**N2d**) | 25 | **−77** | −52 | **−3.08** |
| `≤5 hcp, 5-card suit` — relay floor | 68 | −121 | −59 | −1.78 |
| `≤7 hcp, no 5-card suit` — nothing to say | 116 | −44 | +93 | −0.38 |
| `8+ hcp, 0-1 or 4+ in theirs` — no call at all (**N2c**) | 19 | **+11** | **+91** | +0.58 |

**N2d replicates** as the worst hand class in the lane, on both scorers.
**N2c does not**: the class that motivated it is now positive on both scorers,
so its queue row is demoted to *parked pending replication* rather than closed
— 19 boards is small enough that either sign is seed noise.
