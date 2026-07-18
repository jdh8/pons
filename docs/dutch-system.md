# The Dutch system — campaign ledger

Dutch is a **natural 2/1 built around a wide, non-forcing 1♣** — a "lawyer's
Polish Club" that naturalises the Polish 1♣: Polish constructiveness, but
natural and less restricted. It is a **champion candidate**. We copy
[`american()`](../src/bidding/american.rs) and apply the Dutch diff one
measurable phase at a time; Dutch promotes to the shipped default only if it
measures stronger, at which point `american()` demotes to the ablation baseline
(the WBridge5-ships-a-modded-French model).

Read [docs/bidding-architecture.md](bidding-architecture.md) first (the
book/floor/inference layer cake) and [docs/measurement.md](measurement.md) (no
bidding change ships without an A/B).

## Target system

The full bidding spec — openings, the 1♣ response ladder, opener's relay
rebids, and the deep continuation trees — is transcribed from jdh8's Watermelon
Dutch book in **[dutch-spec.md](dutch-spec.md)** (with pons deviations flagged
inline). In one line: a wide non-forcing **1♣** (11–23 catch-all) with a **1♦!
relay** carrying the awkward and the very strong; **1♦** = 5+♦ or the
singleton-club 4441 (never 3♦; every other 4-diamond hand — incl. (xx)45 —
opens 1♣); five-card majors 10–20; a strong artificial **2♣**; and (Phase 3)
Multi / Muiderberg / UNT at the 2-level replacing the weak twos.

## Decisions

1. **Layout = reuse + override.** `dutch()` is a thin factory that reuses
   american's shared module `register()`s (1NT, slam, XYZ, raises, most rebids,
   most competition/defense) and swaps in Dutch modules only for the divergent
   parts. No fork of the ~19k-line tree.
2. **Family = `Family::NATURAL`.** Opponents defend Dutch as natural for now.
3. **2-level openings = lift the shapes, replace weak twos.** Multi/Muiderberg/UNT
   replace american's weak-two 2♦/2♥/2♠ and strong 2NT; strong balanced 20–23
   moves into the wide 1♣. Reuse the existing `woolsey_multi` /
   `woolsey_muiderberg` / `unusual_2nt` shape builders (lifted out of
   1NT-defense gating), re-banded to weak-two strength. (In this codebase
   "Woolsey" is a *defense to their 1NT*, not opening preempts — only the shape
   builders are reused.)

## The central risk — reader/floor are american-tuned

`inference.rs` (the reader) and `instinct.rs` (the floor) are tied to american's
meanings. Two mechanisms:

- **Rule projection** (`set_alert_reading`, default-on) makes every alerted,
  DSL-authored artificial call self-decode by replaying its own `project` fold.
  So Dutch's strong 2♣, Multi 2♦, Muiderberg 2M, UNT 2NT, and the 1♣–1♦ relay
  decode **for free** once authored with projecting combinators + `.alert(...)`.
- The **hardcoded american reading** decodes the natural, unalerted openings.
  Dutch's natural openings differ by range/shape (1♣ 11–23 2+♣; 1♦ 5+/4414; 1M
  10–20), so they get a **soft misread** (biased HCP band), not a phantom-suit
  disaster. Strategy: rely on projection for the artificial calls, accept soft
  natural misreads, **measure the cost**, fix only the worst offenders (Phase 4).

`instinct()` is keyless and only fires off-book, so **completeness of authored
Dutch nodes** (both sides of every convention) is what keeps the floor from
bidding an artificial opening as natural. Dutch keeps american's 15–17 1NT, so
the floor's transfer-completion still holds.

## Phase ledger

| Phase | Scope | Status |
| --- | --- | --- |
| 0 | Scaffold `dutch()`, re-export, 0.000 baseline | **DONE** |
| 1 | Dutch openings: wide 1♣, 1♦ 5+/4441, 1M 10–20, strong 2♣ | **DONE** (code; A/B pending) |
| 2.1 | Wide-1♣ response table + opener's rebid after the `1♦` relay | **DONE** (code; A/B + gates pending) |
| 2.2 | Deep relay continuations (`1♣-1♦-1M/1NT/2♣/2♦`) + `[1♣,2♣]`/`[1♣,2♦]` continuations | pending |
| 3 | 2-level openings (Multi/Muiderberg/UNT) + strong-2♣ tree | pending |
| 4 | Reader/floor reconciliation + divergent-opening competitive book | pending |
| 5 | Iterate to champion vs BBA/BEN; promote if it wins | pending |

Each phase gates on a paired-seed A/B via `examples/bba-gen` (dutch arm vs
american arm), dual-scored (`ns_score_pd` + `ns_score_contract`), fresh
`SEED_BASE`, run sequentially under `scripts/idle-run.sh`. Preemptive phases
(3) are read knowing DD is blind to obstruction — lean on the sd-lead / PD
bracket, not plain DD alone.

### Phase 2 notes — the wide-1♣ response structure

Spec tables (responder's calls, opener's relay rebids, the deep continuation
trees) live in **[dutch-spec.md](dutch-spec.md)**. Phase 2.1 authored the first
two nodes — `[1♣]` responses and `[1♣,1♦]` opener rebids; the `2NT!` 5-5-minor
rebid is dropped (unreachable in pons — 5-5 minors open 1♦). This section keeps
only the pons-specific encoding choices and the open items.

Encoding choices (each a small, faithful adaptation — validate in the A/B):

- **Relay constraint = `hcp(5..) | len(♣,..3)`** (constructive values, or too
  short in clubs to pass), sitting at weight 0.3 below every natural and above
  `Pass` (0.0). An OR-disjunction projects to the WALL → floors nothing → the
  alert cleanly suppresses the natural-diamond reading with no phantom suit.
- **Weak jump = exactly six, preempt = seven-plus** (`2M` `6..=6`, `3M` `7..`)
  so `2♥♠` and `3♥♠` partition by length rather than overlapping on 6+.
- **`2NT`/`3NT` deduped at 11** — `2NT` 10–11 invite keeps the 11, `3NT`
  encoded 12–15 to-play (doc lists 3NT as 11–15).
- **Opener's `2♣` spans 11–20**, not 11–17, so an 18–20 unbalanced no-4M no-6♣
  five-club hand has a rebid instead of falling to the `Pass` catch-all; opener
  resolves the exact band on the next round (Phase 2.2).
- **1M responses use up-the-line** (bid the cheaper 4-card major first), pure
  DSL, no dependency on american's `spades_first`/`hearts_first`. Longer-major
  discipline (reader-preferred) is a deferred refinement.

Alerts: only the genuinely artificial calls trip the invariant — `1♦!` relay,
opener's `2♦!`/`2NT!`. The natural-but-meaning-inverted calls (`2♦` GF, `2♣`
INV+, weak jumps, minor invites, major preempts) are *also* alerted so
projection self-decodes them (american's hardcoded reader would otherwise read
`2♦` as a weak jump and `2♣` as an inverted raise). Balanced/notrump naturals
(`1M`, `1NT`, `2NT`, `3NT`) are left unalerted — american's read is close enough.

**Open questions / deferrals:**

- **✓ Phase-1 spec discrepancy resolved (2026-07-18):** the online `1D.md`
  argument for opening 1♦ on (xx)45 [4♦5♣] is **stale** — jdh8 is no longer
  following it. In pons, **(xx)45 opens 1♣** (the locked `1♦ = 5+♦ | 4441`
  stands) for simplicity and as an experiment. Consequence: the `2NT!` 5-5-minor
  rebid is unreachable (5-5 minors open 1♦) and was **dropped**.
- **Deep continuations deferred to 2.2:** `1♣-1♦-1M` (support/two-suiter/16+),
  `1♣-1♦-1NT` (a full 18–20 transfer structure, reuses the 1NT machinery),
  `1♣-1♦-2♣`, `1♣-1♦-2♦` (a 21–23 transfer structure). Until authored the floor
  handles responder's third call — a soft misread, measured not fixed blind.
- **`[1♣,2♣]` / `[1♣,2♦]` still american** (inverted-raise / weak-jump
  continuations) under Dutch's natural 2♣/2♦ — overwrite in 2.2.

### Phase 1 notes

`bare_dutch()` takes a full `bare_american()` pair and **overwrites only the
opening node** (`Trie::insert_arc` replaces the classifier at the opening key)
with `dutch::openings::dutch_openings()`; every american continuation is reused
verbatim. Widening was minimal: `insert_uncontested` and `with_instinct_floor`
in `american.rs` → `pub(in crate::bidding)`. Openings live in
`src/bidding/dutch/openings.rs`.

Design choices to validate in the A/B (each defensible from the spec, cheap to
flip):

- **Rule of 20 is a hard gate** (`hcp(band) & rule_of_20()`), so a flat
  sub-R20 minimum passes out — e.g. a 4-3-3-3 twelve-count (12+4+3 = 19).
- **1NT is balanced-only 15–17** (american's wide-6322 5422/6322 shapes open a
  minor instead). A deliberate Dutch choice; wide6322 is a later within-Dutch
  A/B if wanted.
- **No 3rd/4th-seat light (9-count) major openers** — american has them; the
  Dutch spec caps majors at a Rule-of-20 10-count. Watch the passed-out-seat
  boards.
- **Strong balanced 20–21 still opens 2NT** (Phase 1 placeholder); only 22–23
  balanced reaches the wide 1♣ until Phase 3 turns 2NT into UNT.

Guards: `dutch_artificial_calls_are_alerted` (inference.rs) walks the Dutch
constructive book; `dutch::tests::opening_partition` pins the six load-bearing
opening cases (incl. the wide 1♣ hosting a 23-count). `dutch()` is re-exported
as `pons::dutch`.

### Phase 0 notes

Scaffolded `dutch()` as a sibling factory (re-exported `pons::dutch`). The
reuse + override seam opened in Phase 1; widening american internals is done
only as each part diverges.
