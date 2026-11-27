# Strawberry Polish Club: the `strawberry` system

**Status: first cut built 2026-11-26, measurement queued, verdict owed
([§6](#6-measurement)).** `pons::strawberry()`, `strawberry_instinct()` and
`strawberry_book()`, the seat names `strawberry` / `strawberry-instinct` /
`strawberry-book` in `bba-gen`, `render-book --system strawberry`,
`card::strawberry_card` and `scripts/ab-strawberry.sh` are on `main`.
Authored: the opening table and the Polish `1♣` through opener's rebid.
Everything else is borrowed from the house book or owed
([§3](#3-chapter-ledger)). `american()` is untouched: its two visibility
changes move no rule.

The spec is jdh8's book, <https://polish.club/> (source in
`~/src/polish.club/src`, one page per agreement; page names below are its
files). The census was read at its commit `cde7a81`.

## 1. What it is

A second system, not a variant. [system-variants.md §2](system-variants.md#2-mechanism--a-variant-is-an-agreements-value-not-a-factory)
rules that a variant of `american()` is an `Agreements` value and never a
factory, and that still holds for variants. Strawberry is not one (jdh8,
2026-11-26: "it's on its own"; BWS 2017 is 2/1, this is a Polish club): nearly
every opening differs, so it has its own factory and its own modules under
`src/bidding/strawberry/`, one per chapter of the book.

This re-opens the "Add Polish Club" entry parked in
[next-steps.md](next-steps.md). The three earlier attempts are in the record:
`polish_club()` and `american_strawberry()` (dropped, CHANGELOG) and `dutch()`
([archive/dutch-system.md](archive/dutch-system.md)). What they paid for
applies here unchanged: the reader and the learned floor are american-tuned,
and a half-built system measures as a loss until both sides of each
convention are authored.

## 2. Assembly

`strawberry(&Agreements)` builds a `System` from four parts:

| Book | Source |
| --- | --- |
| Constructive | `strawberry::openings` and `strawberry::one_club`, then the house subtrees under each opening in `BORROWED` |
| Competitive | the house subtrees under each opening in `BORROWED`; nothing under `1♣` |
| Defensive | the house book, whole (jdh8, 2026-11-26: the defensive system is not coupled to the constructive one) |
| Mirror (how we read *their* calls) | the house book: an undeclared opponent does not play a Polish club |

- **Borrowed chapters.** `BORROWED` is `1♦`, `1♥`, `1♠`, `1NT`. Each is a
  `Trie::graft` of the house book's subtree in all four seats, so a chapter
  not yet authored is bid as `american()` bids it and a comparison with
  `american()` prices exactly the authored chapters. It is not the book's
  meaning (`1M - 2♣` is a natural game force there, a relay in the book), so
  a chapter leaves the list in the commit that authors it. `1NT` stays: the
  book's notrump structure is the house's ([§4](#4-census)). The test
  `the_house_book_keys_every_fallback_under_an_opening` guards the graft: a
  house fallback keyed above an opening call would not follow it.
- **Floors.** `strawberry()` stands on the shipped v6 net under the same
  convention card as `american()`; `strawberry_instinct()` on the
  deterministic ladder. No card slot expresses a Polish club (slot 0 is the
  Watermelon wide `1♣`), so the net is told nothing and reads our `1♣` from
  the opening rule's projection. A board they open is therefore bid exactly
  as `american()` bids it (test
  `borrowed_chapters_and_the_defense_are_the_house_book`).
- **Readings.** `ReadingScope::All` is the default, so every authored call
  reads off its rule: the `1♣` as the union of its three hand types, `1♦` as
  11–20, `1M` as 11–16. Only calls that show a suit they do not name are
  alerted (`strawberry:*` slugs); `strawberry_artificial_calls_are_alerted`
  walks both books.
- **Disclosure to BBA.** `strawberry_card` is the house card under the WJ
  header (EPBot system 2), the only channel that says our `1♣` is forcing and
  may be short. What it gets wrong: WJ's `1♦` and `1M` run to 17, ours to 20
  and 16; WJ opens a Precision `2♣`, a Multi and two-suited `2M`, calls we do
  not make yet.

## 3. Chapter ledger

| Chapter (page) | State | Notes |
| --- | --- | --- |
| Openings (`Openings.md`) | **authored**, one level | `1♣!`, `1♦`, `1M`, `1NT`, `3x`. Late-seat majors from 9. No two-level opening, gambling `3NT`, Namyats, `4M` or `4NT`: those hands pass or open `3x` |
| `1♣` responses (`1C.md`) | **authored** | both tables; the `4♣!`/`4♦!` Namyats responses are owed |
| Negative `1♦` (`1C/1D.md`) | **authored**, two rounds | opener's rebid, responder over `1M` / `2♣` / `2♦!`, opener over `1M - 1NT`. Owed: `3♥!`, `3♠!`, `3NT!` two-suiters, the Muppet tree over `2♦!`, systems on over the 18–20 `1NT` (a verbatim graft would be three points off; the floor bids it for now), opener's pass of a passed hand's `1♦` |
| Positive `1M` (`1C/1M.md`) | **authored**, opener's rebid | owed: opener's splinters, the solid-club `3NT` |
| Odwrotka (`1C/1M/2D.md`) | **authored**, reverse 445566 | the page's recommended strong-`2♥` relay structure is owed; opener places game only |
| Checkback Gladiator (`1C/1M/CheckbackGladiator.md`) | **interim** | `1♣ - 1♥ - 1♠`: the natural rows and the game-forcing `2♦!` with its answers. The `2♣!` checkback is owed, so `1NT` also takes the minimum with four spades and the balanced invitation. The `1♣ - 1M - 1NT` tables are the floor's |
| Inviting `2M` (`1C/1M/2M.md`) | floor | |
| `1♣ - 1NT`, `2NT` (`1C/1NT.md`) | **authored**, natural rows | the `2♦!` relay and `3♦!` slam try wait for their answers |
| Swapped `2♣`/`2♦` (`1C/2CD.md`) | **authored** | responder places a flat game (capped at 15 points; stronger hands bid on with the floor); opener's splinters over `2♦!` owed |
| Reverse Flannery (`1C/2H.md`) | **interim** | opener bids game directly; `2NT!` lebensohl and the `3♣!`/`3♦!` asks are owed |
| `1♣` competition (`1C/Competition.md`, `Opener.md`) | floor | nothing is grafted under `1♣`: the house tables read it as natural clubs |
| `1♦` (`1D*.md`) | borrowed | |
| `1♥`, `1♠` (`1H*.md`, `1S*.md`) | borrowed | |
| `1NT` (`1NT*.md`) | borrowed for good | |
| Preempts (`2C.md`, `2D.md`, `2M.md`, `2NT.md`, `4x.md`) | owed | |
| Slam, cuebids, BTU vs Unusual | the house's | |
| Defense (`Defense/*.md`) | out of scope | a separate defensive book if ported, never part of this factory |

**Closing tables.** An artificial call is never left unanswered: where the
page gives no continuation, or the continuation is owed, `one_club.rs`
answers with two rows, to play (`closing_entries`, `place_game`,
`after_odwrotka_step`). Before they existed the floor passed `1♣ - 3♦!`,
`1♣ - 2♠!` and the forcing `1♣ - 1♥ - 1♠` (the last with 21 opposite 9).
They are not the book; each is replaced with its chapter.

## 4. Census

Four readers, 2026-11-26, over `render-book` and the source at `f61d3699`,
one per group of pages; classes as in [bws-2017.md §3](bws-2017.md#3-conformance-census).
Rows were not re-checked one by one: treat each as a lead until its prefix is
rendered.

| Pages | Against the house book | Reading |
| --- | --- | --- |
| `1NT.md`, `1NT/2C.md`, `1NT/2R.md` | mostly SAME | Stayman, Jacoby, `2♠` clubs-or-size, `2NT` diamonds, Puppet `3♣`, `3♦` both majors, South African Texas, the quantitative `4NT`, the 5♥-4♠ invitation through `2♦ - 2♥ - 2NT`. PARTIAL: Puppet and garbage/crawling Stayman are narrower, the `3M` splinters only the 3=1=4=5 family. DIFFERS: direct `4M` (a slam try, the book's sign-off), `2♠ - 3♣ - 3♦`, `2♥ - 2♠ - 3♥`/`4♥`. HOLE: `5NT`, the graded super-accepts, opener's `4♣`/`4♦` over `3♦` |
| `1NT/Competition.md` | PARTIAL | `(2♦)`/`(2♠)` Transfer Lebensohl and `(2NT)` match; `(2♥)` has `3♦`/`3♥` transposed; the runouts over `(X)` and the Landy counter differ |
| `1D*.md` | mostly DIFFERS | SAME: `1M`, `3NT`, opener's raises, jump shifts and splinters. The house plays inverted raises (the book: standard with mixed), weak jump shifts (Reverse Flannery `2♥!`, `2♠!` clubs), a natural strong `3♣` (the artificial reverse) |
| `1H*.md`, `1S*.md` | mostly DIFFERS | SAME: `1♠` over `1♥`, splinters, `3NT` choice of games, Reverse Drury with the `2♦` relay, Truscott `2NT` over their double. The house plays a forcing `1NT`, natural 2/1s (the `2♣!` relay, the transfer raise, `1♠ - 2♦!`), Jacoby (Stenberg), limit `3M` (preemptive) |
| `2D.md` | KNOB, drifted | `opening.multi_two_diamonds` with the champion table. Its `2NT` answers no longer match the page ([§7](#7-decisions-owed-to-jdh8)) |
| `2C.md`, `2M.md`, `2NT.md`, `4x.md` | DIFFERS / HOLE | the house `2♣` and `2NT` are strong; no Muiderberg, gambling `3NT`, Namyats, `4M` or `4NT` opening |
| `Slam*.md`, `Cuebids.md`, `BTUvsUnusual.md` | PARTIAL | 1430 and the queen ask match; Kickback only for hearts (`rkcb_variant`), no Turbo, no control bids bid; unusual-vs-unusual cues are the other way round |
| `CheckbackGladiator.md` | PARTIAL | XYZ covers the same slots with `2♦` game forcing; its `2♣` puppet and direct `2x` are the classic meanings the page swaps; no `2NT` club transfer |
| `Defense/*.md` | out of scope | recorded for a future defensive book: SAME for overcalls, the takeout double and its advances, Michaels, the unusual `2NT`, Leaping Michaels; KNOB for Rubens advances, Gladiator over `(1M) 1NT`, Landy, Woolsey; HOLE for the reopening and passed-hand tables and everything over their Multi |

Knob states the readers found closer to the book than the default, none
moved here (the borrowed chapters stay byte-identical to the house's):
`rebid.semiforcing_notrump`, `rebid.one_diamond_two_clubs`,
`notrump.stayman_both_majors` and `stayman_5card_max` off,
`competition.nt_3c_transfers`, `competition.uvu_natural_floor` 5,
`opening.multi_two_diamonds`.

## 5. Holes, ranked

By how often the auction comes up, most frequent first. Each is one chapter
authored whole: both sides, and the tails where they double or overcall.

1. **Competition over `1♣`** (`1C/Competition.md`, `Opener.md`): the transfer
   responses over `(X)` and the one-level overcalls, Landik over `(1NT)`,
   opener's rebids. Today the net answers a `1♣` it was trained to read as
   clubs; in the smoke a 13-count passed `1♣ (1♠)`.
   `competition.cachalot_contested_x` is the nearest house code.
2. **Checkback Gladiator** under `1♣`: the `2♣!` lane, then the three
   `1♣ - 1M - 1NT` tables; `1D/1M/CheckbackGladiator.md` shares the scheme.
3. **`1♥` and `1♠`**: the non-forcing `1NT`, the `2♣!` relay, the transfer
   raise, Stenberg, opener's limited rebids; then their competition.
4. **`1♦`**: standard and mixed raises, Reverse Flannery, the artificial
   `3♣` reverse, the wide range's rebids.
5. **The two level**: Ekren, the Multi re-read against the page, Muiderberg,
   the minors `2NT`, their fourth-seat forms. Double-dummy is blind to what a
   preempt obstructs ([measurement.md](measurement.md)).
6. **Strong hands**: the Muppet tree over `2♦!`, systems on over the 18–20
   `1NT`, the two-suited jumps, slam tries after Odwrotka and the minor game
   forces. In the smoke the floor leapt to a grand slam opposite readings
   with no ceiling (`1♣ - 1♠ - 2NT - 7NT`: responder, with 16, opposite a
   17+ rebid).
7. **The asks withheld**: `1♣ - 1NT - 2♦!`, `1♣ - 2NT - 3♦!`, the Flannery
   asks, the strong-`2♥` Odwrotka structure, opener's splinters.
8. **Gambling `3NT`, Namyats, `4M`, `4NT`**, and Namyats over `1♣`.

Beyond the book: the floor was never trained on these auctions (the
`strawberry-instinct` arm prices it), and a Strawberry corpus is retrain work,
deferred with the rest ([next-steps.md](next-steps.md)).

## 6. Measurement

`scripts/ab-strawberry.sh`, three arms on one seed, vs BBA 2/1, two cells:

| Arm | Flags | Reads |
| --- | --- | --- |
| `advanced` | none | the control, `american()` |
| `strawberry` | `--our-floor strawberry` | the system, `diffpair strawberry advanced` |
| `strawberry-instinct` | `--our-floor strawberry-instinct` | the floor under it, `diffpair strawberry-instinct strawberry` |

Plain DD is the verdict and PD the guard ([measurement.md](measurement.md)).
A loss is expected and on plan for a half-built system; what the run is for
is `ab-dump-bucket --by node`, which re-ranks §5 by measured cost. Because
the borrowed chapters and the defense are the house's, every divergent board
starts at an opening call or under `1♣`. Two things the run cannot say: BBA
defends a WJ card that misstates our `1♦` and `1M` ranges, and double-dummy
is blind to what the nebulous `1♣` concealed.

**Read 2026-11-27** (pueue task 43, snapshot `fae1bb9a` on `f61d3699`,
seed 1795771032426658059, 204,800 boards per arm per cell, results in
`ab-results/strawberry/`). IMPs a board, 95% CI in brackets:

| Pair | Fired | Plain none | Plain both | PD none | PD both |
| --- | --- | --- | --- | --- | --- |
| `strawberry` vs `advanced` | 18.3% / 18.5% | −0.0640 [±0.0103] | −0.0835 [±0.0132] | −0.0870 [±0.0118] | −0.0995 [±0.0153] |
| `strawberry-instinct` vs `strawberry` | 27.7% / 26.0% | −0.3371 [±0.0133] | −0.4133 [±0.0168] | −0.3424 [±0.0152] | −0.4941 [±0.0191] |

- **The system: a loss, on plan** for a first cut with half its chapters
  borrowed. About 0.07 IMPs a board plain, −0.35 to −0.45 per fired board;
  plain carries most of it, so this is bidding, not the doubling bracket.
- **The floor under it is worth more than under `american`**: the learned
  floor beats the ladder by +0.337 / +0.413 plain and +0.342 / +0.494 PD,
  where on `american` the M32 v6 floor's paired worth is about +0.265 plain
  and +0.392 PD ([bba-gap-campaign.md](bba-gap-campaign.md)). Never trained
  on a Polish `1♣`, it still covers the unauthored ground better than the
  ladder does; that is no reason to leave the holes to it.
- **Attribution owed**: `ab-dump-bucket --by side|node` for both pairs,
  queued as pueue task 44 (files `ab-results/strawberry/bucket.*.txt`,
  the `instinct` ones prefixed `bucket.instinct.`). It re-ranks §5.

The borrowed `1♥`/`1♠` chapters took the semiforcing `1NT` with the house
default on 2026-11-27, after this run; the book's own `1M - 1NT` is
non-forcing (§5 item 3), so that moves toward it.

Checked before queueing: `american` is byte-identical to `f61d3699`
(`smoke-default --count 20000 --seed 1`, dev profile, both dumps
`a7c90606…`); 3,000 self-play boards against the house system pass no
forcing call; 64 boards against BBA run with the generated card.

## 7. Decisions owed to jdh8

Every choice the first cut made that the book does not make for it. The
defaults are in place and each is reversible where the last column says;
functions are named, since line numbers drift. "Points" is the book's
upgraded count (`constraint::points`).

### 7.1 Taken by jdh8, 2026-11-26

| Decision | Consequence |
| --- | --- |
| Strawberry is its own system, not packages or a preset on `american` | its own factory and module tree; `Agreements` stays the house's knob vector, read by the borrowed and shared parts |
| The defensive system is not coupled to the constructive one | the house `Defensive` book, whole; `Defense/*.md` is not ported into this factory |
| The name `strawberry` (offered with a question mark) | used for the factory, the module, the seat names and the card; renaming is mechanical |

### 7.2 Structure

| # | Question | In place | Alternative | Reverse at |
| --- | --- | --- | --- | --- |
| S1 | What bids an unauthored chapter? | the house book, grafted (`1♦`, `1♥`, `1♠`) | the floor alone: purer, weaker, and the A/B would then price the floor's responses, not the authored chapters | `BORROWED` in `strawberry.rs` |
| S2 | Which floor does `strawberry()` ship on? | the v6 net, as `american()` | the instinct ladder (`strawberry_instinct()`); the A/B's third arm decides | `strawberry()` |
| S3 | Is the net told about the Polish `1♣`? | no: the house card, slot 0 off | set slot 0 (the Watermelon wide `1♣`, the nearest regime); it also moves boards they open | `strawberry()`, the `ConventionCard` |
| S4 | What card does BBA get? | the WJ header over the house rows | the 2/1 header, which says `1♣` is natural; or a hand-written `.bbsa` | `card::strawberry_card` |
| S5 | An artificial call the book leaves unanswered, or whose answers are owed | answered by a two-row closing table, to play | leave it to the floor, which passed `1♣ - 3♦!` and `1♣ - 2♠!` in the smoke | `close`, `closing_entries` |
| S6 | An artificial ask whose answers are not authored | withheld; its hands stay in the natural rows | bid it and let the floor answer | the module doc of `one_club.rs` lists them |
| S7 | Systems on over the 18–20 `1NT` rebid | the floor's, natural | graft the house `1NT` tree verbatim: right conventions, every strength gate three points off | add a graft in `strawberry::book` |
| S8 | No two-level opening is authored: what do those hands do? | pass, or `3x` on seven cards | borrow the house weak twos until Ekren, the Multi and Muiderberg exist | `strawberry::openings` |
| S9 | The measurement | 204,800 boards per arm per cell, cells none and both, vs BBA on its 2/1 card | four cells, or more boards, once a chapter is close enough for δ to matter | `scripts/ab-strawberry.sh`, `BOARDS`, `VULS` |

### 7.3 The opening table

| # | The book | In place | Alternative | Reverse at |
| --- | --- | --- | --- | --- |
| O1 | `1♥♠` "11–16 points" | upgraded points, floor 10 HCP: a shapely 10 opens, and a 16-HCP unbalanced hand counts 17 and opens `1♣` (in the smoke, ♠832 ♥AKJ963 ♦A8 ♣A3) | read 11–16 as HCP | the `1M` rules in `openings` |
| O2 | `1♦` "11–20"; balanced hands need 12, an 11 needs playing strength | 12–20 points, floor 10 HCP, so an unbalanced 11 HCP opens and a balanced 11 passes | an explicit playing-strength test | the `1♦` rule |
| O3 | 17–20 with a five-card major and diamonds: `1♦` or `1♣`? | five or more diamonds (six beside a six-card major): `1♦`; only four: `1♣` | every 17+ five-card major opens `1♣` | `long` and `canape` in `openings` |
| O4 | 18–20 balanced with five diamonds: `1♦` ("5+♦") or `1♣` then `1NT` (the notrump ladder)? | `1♦`, also on 5♦422 and 6♦322 | `1♣`, as 2=2=4=5 does | `long` |
| O5 | Late seats: `1M` 9–16; light openings are for balanced and wasted hands, shapely ones open at the two level | every five-card major from 9 HCP in third and fourth seat (a 9–10 `1♥` needs three spades), since no two-level opening exists | hold the light opening to balanced hands once the two level is authored | the late-seat rules |
| O6 | `1NT` 15–17 with a five-card major | opens `1NT` ahead of `1M` in every seat (the house prefers `1M` in late seats) | the house order | weights 200 against 165/155 |
| O7 | `3x` "PRE, 7+" | the house rule: seven cards, under 12 points, not in fourth seat | the book's own bands, unstated | the `3x` loop |

### 7.4 Over the Polish `1♣`

| # | The book | In place | Alternative | Reverse at |
| --- | --- | --- | --- | --- |
| R1 | `2♣!`/`2♦!` "FG, 5+ minor"; the passed-hand rows deny a major, these do not | 12+ points; a five-card minor is bid ahead of a four-card major | deny a four-card major, as the passed hand does | `responses` |
| R2 | both minors, game forcing | `2♣!` (diamonds) on 5-5 and on 5♦6♣ | split by length | the `ponytail:` note in `responses` |
| R3 | `1NT` 8–10, `2NT` 10–11 | a 10 bids `1NT` | a 10 bids `2NT`, or split by quality | weights in `shared` |
| R4 | invitations `2♠!`, `3♣` "9–11" | points, and no four-card major | HCP; allow a major | `responses`, `shared` |
| R5 | opener's raise: "INV, 15–17" (`1C/1M.md`) against "INV 15–16, FG 17+" (`Competition/Opener.md`) | invitation 15–16, game force 17+ | 15–17, with Odwrotka from 18 | `after_positive` |
| R6 | Odwrotka "FG, 3+" | every 17+ with three or more, four-card support included; no splinter, no solid-club `3NT` | author the splinters and send four-card support there | `after_positive` |
| R7 | two Odwrotka structures; the page recommends the strong-`2♥` relay | reverse 445566: one table closes the convention; opener places game, capped at 19 | the strong-`2♥` relay and its three follow-up tables | `odwrotka_steps`, `after_odwrotka_step` |
| R8 | `1♣ - 1♦ - 1M` "11–19", `2♦!` "21+" | `1M` runs to 20 for 4M-5♣ and 4=4=1=4 | `2♦!` from 20 | `after_negative` |
| R9 | `1♣ - 1♦ - 1NT` "18–20, 2–5♠, 2–5♥" against `1M` "17–19: 5+" | `1NT` on every hand of that shape, a five-card major included | `1M` first on five | weights 260 against 155 in `after_negative` |
| R10 | the two-suited `3♥!`, `3♠!`, `3NT!` | not authored: those hands rebid `2♦!` or `2M` | author them with their continuations | `after_negative` |
| R11 | `1♣ - 1♦ - 2♦!`: "force to game as over `2NT`, else sign off" | 0–3 HCP signs off (`2♥`, `2♠`, `2NT`); 4+ is the floor's until Muppet is authored | another game-force line | `after_strong_rebid` |
| R12 | `1♣ - 1♠ - 1NT` "2–3♥" | the weak notrump with four hearts rebids `1NT` too, so the call reads 2–4 hearts | none on the page: `2♥` is game forcing there | `after_positive` |
| R13 | `1♣ - 1NT`: `2NT` "INV, 14–15", `3NT` "S/O, 16+" | as written, with no ceiling on `3NT`, so a slam hand signs off; the `2♦!` relay is withheld | cap `3NT` and leave the rest to the floor | `after_notrump` |
| R14 | `1♣ - 2NT - 3NT` "S/O", no figure | 14+ HCP | 13+ | `after_jump_notrump` |
| R15 | swapped minors: "11–14 or 18+" cheap, 15–17 jumps | as written; responder then bids a flat game, capped at 15 points | natural continuations and slam tries | `after_swapped_minor`, `place_game` |
| R16 | Reverse Flannery: game through the `3♣!`/`3♦!` asks | game bid directly on 17+ with a fit, `3NT` on 18+ HCP without one; invitations 15–16; pass on four hearts, else `2♠` | the asks and the `2NT!` lebensohl with their answers | `after_flannery` |
| R17 | `1♣ - 1♥ - 1♠`: the `2♣!` checkback carries the minimum with four spades and the balanced invitation | `1NT` takes both (to 12 points); `2♥`/`2♠` invite on 11–12; 13+ bids `3♠`, `3♥` or the game-forcing `2♦!` | the `2♣!` lane | `after_one_spade` |

### 7.5 Discrepancies met

Not resolved here; each needs your reading.

1. **The book against itself: Namyats.** `Openings.md` has `4♣!` semi-solid
   with 0–1 outside ace or king and `4♦!` solid with none; `4x.md` has
   `4♣` solid and `4♦` semi-solid. Neither is authored. Proposed default:
   follow `4x.md`, the convention's own page.
2. **The book against itself: opener's raise.** R5 above.
3. **Code against the book: the Multi.** `american/multi.rs` (champion)
   answers `2♦ - 2NT` maximum-first: `3♣`/`3♦` 8–10 and `3♥`/`3♠` 4–7, each
   naming the suit held. `2D.md` since 2026-09-13 has `3♣!`/`3♦!` minimum,
   `3♥!`/`3♠!` game forcing in the other major, `3NT!` a solid suit, and
   relays over the minimum answers. Proposed default: leave the measured
   knob alone and author the page's table in Strawberry's two-level module.
4. **House code, met by the readers, not touched.** `defense/overcall.rs`
   says seven-card preempts "remain with the floor" under a node whose
   finite pass shadows it; the systems-on graft under `(1♠) 1NT` renders
   `2♥` as a transfer to their suit.
