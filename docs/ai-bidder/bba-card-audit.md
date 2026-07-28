# Audit of `cards/American.bbsa` against `american()`

> **Superseded as a maintenance procedure.** `cards/American.bbsa` is now
> *generated* from the live system by `src/bidding/card.rs`, and a row that a
> knob can move reads that knob rather than being audited after the fact.  This
> audit survives as the record of what each live row means — the evidence the
> generator's row mapping was written from — and as the referent for the two
> rows it still holds constant under stated uncertainty (`Extended acceptance
> after NT`, `Transfers if RHO bids clubs`).

The card declares our 2/1 to the BBA seats (`bba-gen --disclose`). Only rows that
change a BBA call when disclosed are audited here — the
[sensitivity sweep](bba-disclosure-sweep.md) partitions the 257 rows into **33
live** and **224 cosmetic**, over 8406 BBA decisions replayed from the
2026-07-26 anchor at 100% replay agreement.

Referent: `american()` with all 150 `set_*` knobs at crate default.

## Live rows

Sorted as the sweep ranks them. "moved" is decisions whose call changed when the
row was flipped, summed over both values.

| row | card | moved | what `american()` does | src | verdict |
| --- | --- | --- | --- | --- | --- |
| Cappelletti | 0 | 38 | natural defense to their 1NT — penalty `X` (15+ balanced) + natural two-level overcalls | `defense.rs:106` | ✅ |
| Weak natural 2M | 1 | 36 | weak twos in both majors | `weak_twos.rs` | ✅ |
| Raptor 1NT | 0 | 21 | natural 1NT overcall, not 4M+longer-minor | `defense.rs:8` | ✅ |
| 1NT opening natural | 0 | 16 | our 1NT *is* natural, 15-17 balanced | `openings.rs:236` | ✅ (see below) |
| 1NT opening NT style | 1 | 16 | — paired with the row above | — | ✅ (see below) |
| Ghestem | 0 | 14 | not authored | — | ✅ |
| 1NT opening range 12-14 | 0 | 13 | 15-17 (that row is set, and cosmetic) | `openings.rs:236` | ✅ |
| 1NT opening range 13-15 | 0 | 13 | ” | ” | ✅ |
| 1NT opening range 14-16 | 0 | 13 | ” | ” | ✅ |
| Michaels Cuebid | 1 | 12 | authored | `competition.rs` | ✅ |
| 1D opening with 5 cards | 0 | 10 | better minor — `1♦` can be three cards | `openings.rs:222-231` | ✅ |
| Benjamin 2D | 0 | 10 | not authored | — | ✅ |
| French 2D | 0 | 10 | not authored | — | ✅ |
| Multi-Landy | 0 | 10 | `set_landy` defaults to `None` | `defense.rs:97` | ✅ |
| Polish two suiters | 0 | 10 | not authored | — | ✅ |
| Landy | 0 | 6 | ” | `defense.rs:97` | ✅ |
| 1D opening with 4 cards | 0 | 4 | better minor, minimum three | `openings.rs:222-231` | ✅ |
| Checkback | 0 | 4 | XYZ, declared by `Two Way New Minor Forcing = 1` | `xyz.rs:1` | ✅ |
| Garbage Stayman | 1 | 3 | on | `notrump.rs` | ✅ |
| 1N-2N transfer to clubs | 0 | 2 | `2NT` transfers to **diamonds** | `notrump.rs` | ✅ |
| 1N-2N transfer to diamonds | 1 | 2 | ” | ” | ✅ |
| Fit showing jumps | 0 | 2 | not authored | — | ✅ |
| Fourth suit game force | 1 | 2 | FSF on | `american.rs` | ✅ |
| Gazzilli | 0 | 2 | not authored | — | ✅ |
| Mini Splinter | 0 | 2 | not authored | — | ✅ |
| Responsive double | 1 | 2 | authored | `competition.rs` | ✅ |
| Unusual 2NT | 1 | 2 | `set_unusual_notrump_defense` is **on by default** | `defense.rs:175` | ✅ |
| Weak Jump Shifts 2 | 0 | 2 | WJ belongs to `dutch()`, not `american()` | — | ✅ |
| Kickback 0123 | 0 | 1 | RKCB 1430, not Kickback | `slam.rs:1` | ✅ |
| Kickback 0314 | 0 | 1 | ” | ” | ✅ |
| Kickback 1430 | 0 | 1 | ” | ” | ✅ |
| Mixed raise | 0 | 1 | not authored | — | ✅ |
| Support double redouble | 1 | 1 | authored | `competition.rs` | ✅ |

**31 of 33 live rows already describe `american()` correctly.** The card was
written well; what it lacked was a channel, not accuracy.

### The one open pair — resolved: it describes the *responses*

`1NT opening natural = 0` alongside `1NT opening NT style = 1`. Read literally
the first looks wrong — our 1NT *is* a natural 15-17 — and both rows move 16
decisions. Two probes aimed at the opening itself found **no semantic
difference**, because the opening is not what these rows describe.

`probe-bba-sensitivity --explain` names them by printing the auctions they move
rather than counting them. The two rows move the **same six decisions**, mirrored
(`natural=1` ≡ `style=0`): they are one mutually-exclusive radio group, not two
knobs. The authored `(0, 1)` is the "responses are conventional" setting.

```
=1  West KQ.K7432.765.652   after [P P 1NT P 2♥]   X -> P
=1  West J3.A8542.75.AJT8   after [1NT P 2♥]       X -> P
```

Our `2♥` is a transfer to spades. At the authored setting BBA **doubles it** —
the lead-directing double of an artificial bid, showing the suit named — and both
hands are five hearts and little else. Flipped, BBA reads `2♥` as a natural heart
response and passes, since neither hand is close to a takeout double of hearts.
Each behaviour is correct for its assumption.

**So the pair is the switch for Stayman and Jacoby transfers**, not a claim about
the 1NT opening being artificial. The card has **no bare `Stayman` or red-suit
transfer row** — every such row is a variant riding on an assumed base
(`Minor Suit Stayman`, `Extended Stayman`, `Garbage Stayman`, `1N-3C Puppet
Stayman`, the minor-suit transfers, the three `Transfers if RHO …` competition
rules). Modern responses are the default; `1NT opening natural = 1` is what turns
them off. Hence `0` is the honest value, and it is genuinely load-bearing — it
buys BBA the lead-directing double of our transfers.

Naming trap for future readers: `Jacoby 2NT` (row 70) is the game-forcing major
raise and has nothing to do with transfers.

Superseded evidence, kept because it bounds what the rows do *not* control — the
opening's own definition and the band deduced for it are untouched by all four
settings:

`probe-bba-constraints --mode open` (added here — dealer opening, empty prefix),
6000 hands per setting, reading BBA's *own* opening:

| natural | style | BBA's 1NT opening |
| --- | --- | --- |
| 0 | 0 | 15-17, chosen 4.6% (n=275) |
| 0 | 1 | 15-17, chosen 4.6% (n=275) |
| 1 | 0 | 15-17, chosen 4.6% (n=275) |
| 1 | 1 | 15-17, chosen 4.6% (n=275) |

Identical to the hand. So the rows govern *interpretation*, not bidding — which
is what disclosure is for, and readable via `BbaOracle::probe`.
`probe-bba-nt-reading` puts 300 balanced 15-17 hands at the opener's seat and
reads BBA's deduced band for it over `(1NT)`:

| natural | style | deduced HCP | deduced length ♣ ♦ ♥ ♠ |
| --- | --- | --- | --- |
| 0 | 0 | 15-17 | 2-6 2-6 2-5 2-5 |
| 0 | 1 | 15-17 | 2-6 2-6 2-5 2-5 |
| 1 | 0 | 15-17 | 2-6 2-6 2-5 2-5 |
| 1 | 1 | 15-17 | 2-6 2-6 2-5 2-5 |

Also identical — one distinct read per setting, strength *and* shape. BBA already
reads our 1NT correctly as 15-17; that band is carried by the
`1NT opening range 15-17` row, not by these two.

**Verdict: leave both as authored.** The literal reading has no evidence behind
it, and the pair came from BBA's own 21GF card where it is presumably coherent.

Unresolved: the sweep says these rows move 16 real decisions, yet neither probe
locates the mechanism — so they bite somewhere other than a first-seat 1NT
opening or its immediate read. Owed: have the sweep dump *which* decisions moved
rather than only counting them, and read those auctions.

## Cosmetic misstatements — corrected

These claimed conventions with no implementation in `src/bidding/american/`. The
sweep says all three are cosmetic — flipping them moves **zero** decisions — so
correcting them bought documentation honesty, not IMPs. A future EPBot version
could make any of them live, which is the reason to fix them anyway.

| row | was | now | reality |
| --- | --- | --- | --- |
| Exclusion | 1 | **0** | no exclusion keycard authored |
| BROMAD | 1 | **0** | not authored |
| Maximal Doubles | 1 | **0** | not authored |

## Rows that look stale but are not

- **`1N-3M splinter = 0`.** We shipped a `3♥`/`3♠` splinter default-on
  (`NT_SPLINTER` is `Cell::new(true)`), so `0` invites a staleness reading. It is
  correct anyway: BBA's toggle is the **GIB form** — singleton or void in the bid
  suit, 4+ in each other suit, no five-card major — which pins the *other* major
  at exactly four. Ours is the BWS / Polish Club form (`♦` exactly 4, `♣` 5-6).
  Setting the row would misdescribe us. See `notrump.rs:566` and
  [bba-1nt-splinter.md](bba-1nt-splinter.md). The row is cosmetic regardless.
- **`Blackwood 1430 = 1`.** Recorded in 8c3139b as inert *as a bidding flag*
  (EPBot answers keycard in 0314 whatever the card says). The sweep confirms it
  is inert as a *disclosure* flag too — 0 decisions moved. It is still the honest
  declaration: `slam.rs` is 1430 throughout.

## Discrepancy found while auditing

`notrump.rs:572` documents the 1NT splinter as "Off by default pending its A/B",
but `NT_SPLINTER` is `Cell::new(true)` (`notrump.rs:1208`) — the comment predates
the ship and now contradicts the code. Not fixed here; it is outside this audit.
