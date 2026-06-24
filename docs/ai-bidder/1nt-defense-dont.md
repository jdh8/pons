# 1NT defense: the penalty-X is DD-negative — try DONT (conventional X) next

**Status: RESOLVED (2026-06-24).** Direct-seat DONT was built (`set_direct_dont`,
opt-in/off) and — contrary to this doc's "expect it to lose" framing —
**matched/beat natural** on the honest measure once two levers were added: a
**six-card one-suiter minimum** (`set_direct_dont_one_suiter_min`, the marginal
five-card one-suiters pass instead of conceding through the X relay) and a
**doubled-`2♣`-relay escape** (the doubler names its real suit instead of sitting
in a `2♣x` misfit — the dominant leak this doc didn't foresee). Result vs natural
(−0.187 nv / −0.480 bv): DONT-6+escape = **−0.196 nv (tie) / −0.408 bv (+0.072
win)** — first artificial 1NT defense to reach parity here. **Kept opt-in, natural
stays default** (jdh8's call; the DONT-vs-natural verdict is still single-dummy —
the obstruction wall). Full write-up: `CHANGELOG.md` + memory
`project_natural-1nt-defense.md`. The rest of this doc is the original handoff plan,
kept for the reasoning.

## TL;DR

- Our natural defense to their 1NT is **penalty `X` (15+) + natural overcalls**.
  On double-dummy the **penalty `X` is the loser**, and it is *pre-existing*
  (reproduced bit-for-bit on a clean `HEAD` tree, nothing from the chase session
  involved).
- Root cause: doubling their 1NT **provokes a runout backed by their 15-17
  opener**, which *makes* double-dummy. We'd usually have done better defending
  1NT *undoubled* (we hold 15+ over the opener, so 1NT often fails). The penalty
  `X`'s value is **single-dummy** (they sit and die, or misguess the scramble) —
  invisible to a perfect-play harness.
- **Next hypothesis (next session):** DONT, where `X` = a *one-suiter* (relay
  `2♣`, then correct) — a **constructive/competitive** call, not an attempt to
  collect a penalty. Does a conventional `X` sidestep the penalty-trap on DD?
  **Temper expectations:** DONT was already DD-*lost* as a direct defense in
  earlier work (see memory below) — the *new* angle is to measure it cleanly in
  self-play, per-action, and specifically watch whether the `X`/one-suiter bucket
  stops bleeding the way the penalty `X` does.

## The numbers (so you don't re-derive them)

All self-play, `american()` both sides, plain-DD (`ns_score_contract`), via
`examples/ab-landy`. Natural defense vs **always-pass** (the truest do-nothing
baseline), 60k filtered boards, seed `20260622`, vul none:

| action | boards | IMPs/action-board |
|---|---|---|
| `2♠` | 2071 | **+0.016** |
| `2♣` | 1158 | **+0.018** |
| `2♥` | 2241 | −0.139 |
| `2♦` | 2326 | −0.386 |
| **`X` (penalty)** | **771** | **−2.095** |

Overall natural-vs-always-pass: **−0.323 IMPs/divergent** (−0.046/filtered-board).
The DD ordering is brutal and counterintuitive:

> **always-pass  >  natural defense  >  instinct floor**

i.e. *doing nothing* over their 1NT wins double-dummy — any action just helps
them locate their best making spot, and DD lets them escape perfectly. This is
why `bba-match --isolate-defense --advertise-natural` (honest) sits at
**−0.19/board (none) / −0.47 (both)** with the deficit concentrated in the
**Pass + X** buckets.

Reproduce (clean tree, no changes needed):

```text
cargo run --release --example ab-landy -- --count 60000 --filter \
  --ns-majors "" --ns-minors "" --ns-natural on --ew-always-pass on
```

### The chase that was tried and reverted

A "recursive penalty chase" (mirror of the shipped doubled-1NT chase: after our
X, keep doubling their escape) was built and measured, then **reverted**:

- self-play: stack arm **−1.07**, values arm **−1.92** IMPs/divergent;
  vs BBA the X bucket went **−1.37 → −1.57**.
- make/down tally: stack doubles go **down 75%** of the time (sound bridge!), but
  the 25% that *make* cost ~5× — a doubled 2-level overtrick is +670 to them vs
  +100 to us for down-one (**partscore-doubling asymmetry**).
- "double iff expected score ≥ ~50" is the right gate, but it needs a trick
  estimate the keyless floor can't make (it can't see their trump split). That
  estimate is the **DD search layer**'s job (`american_search`, M2.3:
  shortlist-by-net-prior + `ev_all`). Conclusion: don't author the speculative
  penalty double in the fast floor.

**Lesson the chase confirmed:** you cannot recover the penalty `X`'s loss by
doubling *more*. The whole premise — "our X makes us the strong side" — is
DD-false, because the actual 15-17 opener is still declaring the runout.

## Why DONT is the next move

DONT (Disturb Opponents' NoTrump) uses `X` as a **one-suiter** takeout, not a
penalty:

- `X` = a one-suiter (any) → advancer relays `2♣`, doubler passes/corrects.
- `2♣` = clubs + a higher suit (≥5-4).
- `2♦` = diamonds + a major (≥5-4).
- `2♥` = both majors (≥5-4); `2♠` = natural spades.

The hypothesis: because DONT's `X` is **constructive competition** (find our fit,
contest the partscore) rather than a penalty try, it should not provoke the
opener-backed-runout loss the penalty `X` suffers — at worst it competes us to a
making/failing partscore of *our own*, scored honestly. Whether that nets above
always-passing on DD is the open question.

**Caveat (read first):** memory `project_natural-1nt-defense.md` records DONT (and
Landy, Meckwell) as **DD-lost** direct defenses already. So this is a *re-measure
with a sharper lens*, not a fresh idea — the new value is (a) isolating the `X`/
one-suiter bucket to see if it avoids the penalty-X bleed, and (b) the obstruction
-wall framing: even a DD-negative DONT may be fine single-dummy (it pressures,
it competes), which DD can't score. If DONT also loses on DD, that is *expected*
and not disqualifying — it would confirm the wall, and the real test is SD.

## What already exists in the code (reuse this)

DONT is implemented **only for passed hands** today
(`[P,P,P,1NT]`), gated on `passed_hand()`:

- `PassedHandDefense::Dont` enum + `set_passed_hand_defense(Some(Dont))` —
  [src/bidding/american/defense.rs:354](../../src/bidding/american/defense.rs#L354),
  [:386](../../src/bidding/american/defense.rs#L386).
- DONT **shape predicates** (directly reusable):
  `dont_two_clubs` / `dont_two_diamonds` / `dont_double` (one-suiter) —
  [:784](../../src/bidding/american/defense.rs#L784),
  [:801](../../src/bidding/american/defense.rs#L801),
  [:814](../../src/bidding/american/defense.rs#L814).
- DONT **advances** (relay `2♣` over `X`, pass/correct over `2♣`/`2♦`/`2♥`) —
  [:1028-1083](../../src/bidding/american/defense.rs#L1028-L1083). These are
  keyed at the passed-hand `[P,P,P,1NT,...]` seat (`insert` site near
  [:1835](../../src/bidding/american/defense.rs#L1835)).
- The natural (penalty-X) defense lives in `defense_to_notrump()` —
  [:672](../../src/bidding/american/defense.rs#L672) — wired at all seats via
  `insert_all_seats(&mut d, &[notrump], 3, defense_to_notrump())` (~`:1685`).

## What to build next session

1. **Direct-seat DONT** behind a knob (e.g. `set_direct_dont(bool)` or extend the
   defense-style selection). Reuse the `dont_*` shape predicates and the advance
   functions; ungate them from `passed_hand()` for the direct `[1NT]` seat (and
   the balancing seat if `notrump_balancing`). The X = one-suiter / `2♣` relay
   continuation already exists — the work is mostly *re-keying* it from the
   passed-hand auction to the direct-seat auction and gating the natural
   penalty-X off when DONT is on (mutually exclusive defenses).
2. Decide the `2♠` and `2NT` slots (natural spades; `2NT` = both minors per the
   memory's UvU, or leave to the floor).

## How to measure (and the trap to avoid)

`examples/ab-landy` is the harness — it builds **two books** and seats them per
side, and prints the **per-action breakdown** (the `X` row is what matters here).
It already has a `--ns-passed-dbl dont` flag (passed-hand only); add a
direct-DONT flag alongside it.

- **Baselines:** measure DONT vs **always-pass** (`--ew-always-pass on`) for
  absolute worth, *and* vs the **natural penalty-X defense** (`--ew-natural on`,
  default) for the head-to-head "is conventional-X better than penalty-X".
- **Score both ways:** `--score plain` (the standard) **and** `--score pd`
  (perfect-defense doubling). A competitive overbid that plain-DD lets off is
  doubled under `pd`; a `pd`-only win is a doubling artifact, a `plain`-only win
  is suspect the other way. (See memory `reference_pd-vs-plain-dd-bracket.md`.)
- **Read `IMPs/divergent` and the per-action `X` row**, not just the headline —
  the question is specifically whether the conventional `X`/one-suiter bucket
  stops bleeding.
- **The obstruction wall (do not forget):** this is a *double-dummy* harness; it
  is **blind to single-dummy obstruction** (preempts, light competitive doubles,
  the penalty-X's "they sit and die"). A DD-negative DONT is *not* proof DONT is
  bad at the table. If DONT loses on DD like everything else here, the honest
  next step is a **single-dummy measure**, not another DD tweak. See memory
  `project_preemption-dd-negative.md`.

## Pointers / memories

- `project_natural-1nt-defense.md` — the natural defense status; DONT/Landy/
  Meckwell all DD-lost; `--advertise-natural` honest numbers; the Pass+X deficit.
- `project_preemption-dd-negative.md` — the obstruction wall (DD blind to SD).
- `reference_pd-vs-plain-dd-bracket.md` — score competitive A/Bs with PD **and**
  plain-DD.
- `project_passed-hand-1nt-defense.md` — the passed-hand defense (where DONT and
  the Landy-double live today).
- `project_one-nt-doubled-runout.md` / `project_one-nt-runout-phase2.md` — the
  *shipped* doubled-1NT chase (Case A) that the reverted Case-B chase mirrored;
  note Case A **wins** because there the runners are the weak defenders, not an
  opener-backed pair — the asymmetry that doomed Case B.
