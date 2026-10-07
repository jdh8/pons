# Fourth suit forcing

**Status (2026-10-07):** 4SF is **not** a system-wide agreement.  Authored in
the `1♥ - 1♠` lane only; the ten `1x - 1y - 1z` one-level prefixes belong to
XYZ by decision; the reverses were measured and closed as a wash (parked on
`park/reverse-weak-responses`); the net floor already plays BBA's reverse
structure, so the remaining two-level nodes are not a book gap.  The census
below is a read-only walk of `src/bidding/american` at anchor `25aea82c`
(102k boards), nothing measured except where it says so.  Ledger row 58 in
[ai-bidder/21gf-ledger.md](ai-bidder/21gf-ledger.md) points here.

## Our book, lane by lane

- **Authored 4SF: the `1♥ - 1♠` lane only** — `2♣ - 2♦` (`points` 12+,
  answers + game-only placements, no slam try: 82 boards −760) and
  `2♦ - 3♣` (HCP 13+, `rebid.diamond_rebid_fourth_suit`, shipped a77ecbe4).
  The two are not symmetric (`points` vs HCP; the clubs answer table has no
  four-card `3♦` rung).
- **The four `1x - 1y - 1z` lanes belong to XYZ** (`xyz.rs`), whose table is
  total, so the floor never fires there.  `1♣ - 1♥ - 1♠ - 2♦` is the XYZ
  game force and so *is* 4SF by coincidence; `1♦ - 1♥ - 1♠ - 2♣` is the XYZ
  relay (invitational), correct and not a gap.  **Gap:** responder's
  placement after every XYZ game-force answer is the floor's, and opener's
  `2NT` answer promises no stopper in the fourth suit.  The commonest
  prefixes in the census.
- **Reverses** (`1♣ - 1♥ - 2♦`, `1♣ - 1♠ - 2♦` / `2♥`, `1♦ - 1♠ - 2♥`, 17+):
  responder's whole node is the floor — no 4SF, no Ingberman / Lebensohl
  `2NT`; the instinct floor is natural only (`instinct.rs:18-25`).  Measured
  below.
- `1♦ - 1M - 2♣`: our opener never reaches it by default
  (`one_diamond_two_clubs` off, a wash); with it on the fourth suit is the
  floor's, by design (`rebids.rs:378-382`).
- **Opener's jump shifts** (18+, `extras_ladder.rs`): responder's node is
  wholly floor.  Rarest.
- **Disclosure:** `card.rs` sets "Fourth suit" / "Fourth suit game force" to
  1 whenever either knob is on, so the card claims 4SF everywhere while the
  book plays it in one lane.  Owes a correction once the agreement is
  decided: either author 4SF as a system rule (every three-suit round-2
  node, one shared answer/placement scheme) or narrow the card row.  **jdh8
  to decide** — proposed default: keep the row (BBA reads our fourth suit as
  4SF in every lane regardless, and the floor's natural reading only matters
  to us), author the XYZ placements next.
- **Ours in practice, same anchor:** two lanes bid it (`1♥ - 1♠ - 2♣ - 2♦`
  39 % at 10–22, `1♣ - 1♥ - 1♠ - 2♦` 42 % — the XYZ force), the rest never
  (`1♥ - 1♠ - 2♦` 0 / 2014 before a77ecbe4, `1♠ - 2♣ - 2♥` 0 / 929,
  `1♠ - 2♦ - 2♥` 0 / 447, every reverse 0); our `1♥ - 2♣ - 2♦ - 2♠` is the
  natural four-spade bid, not a probe.  The 2/1 lanes agree with BBA
  (natural) only by omission — the tables have no fourth-suit rung.

## How BBA and BEN play it

- **BBA** (book walk 2026-10-07, one node per prefix — recipe in
  [ai-bidder/bba-book.md](ai-bidder/bba-book.md) § Reproduce — on our card):
  `Fourth suit game force` fires on **every** three-suit round-2 node, the
  one-level lanes included (`1♣ - 1♦ - 1♥ - 1♠`, `1♦ - 1♥ - 1♠ - 2♣`),
  reading **14+ HCP**; after a **reverse** it reads **9+**, beside a 6–8
  `2NT` and 6–9 simple preferences (BBA's reverse structure is "2NT weak",
  not Lebensohl).  Where the auction is already game-forcing the fourth suit
  is **natural**: `1♠ - 2♣ - 2♦ - 2♥` is 4–6 hearts 13+, `1♠ - 2♦ - 2♥ - 3♣`
  4–6 clubs 13+.  Opener's answers are natural — the delayed three-card
  raise of responder's major first (`2♠` 11–17 over `1♥ - 1♠ - 2♣ - 2♦`), a
  six-card rebid, the fourth suit raised with four, `2NT` / `3NT` with it
  stopped, jump answers 14+.  Practice (the 25aea82c anchor, 102k boards):
  25–30 % of each non-reverse lane at **11–12+ HCP, median 13** (the book's
  14 is its hand-free reading), 8 % for the three-level `3♣` over
  `1♥ - 1♠ - 2♦` (median 16), 40–66 % after a reverse (median 9–12); 3NT the
  commonest final everywhere.
- **BBA does not play XYZ**: our card's `Two Way New Minor Forcing = 1` fires
  only over a `1NT` rebid (`2♣` 6–12 relay, `2♦` 13+), so over `1x - 1y - 1z`
  BBA reads our relay `1♦ - 1♥ - 1♠ - 2♣` as a 14+ game force and our game
  force `1♣ - 1♦ - 1♠ - 2♦` as a 6–9 diamond rebid.  A competition-only
  disclosure gap with no card row to fix.
- **BEN** is distilled BBA on `BEN-21GF.bbsa` (`Fourth suit = 0`, `Fourth
  suit game force = 1`), and its anchor dumps (tiers F and S, ~10k boards
  each) show the same book: the fourth suit on 23–35 % of each lane at 13–20
  HCP (median 14–15), the three-level `3♣` over `1♥ - 1♠ - 2♦` only on
  15–17, the one-level `1♠` over `1♣ - 1♦ - 1♥` on 13–18; answers and
  placements as BBA's.

## Decisions

- **2026-10-07 (jdh8): XYZ takes precedence over 4SF** on the ten one-level
  prefixes.  The open 4SF nodes are therefore the two-level ones: the
  reverses (BBA: 9+ GF beside a weak `2NT`), and the `1♦ - 1M - 2♣` /
  `1♦ - 1♠ - 2♥`-type non-reverse nodes we never reach or author.
- The disclosure row (above) awaits jdh8.

## Reverses, measured 2026-10-07 — closed as a wash

The census priced the *instinct* arms; on the shipping (net-floor) arms the
four reverses are 943 boards, −372 plain / −111 PD per 409,600, with 43
forcing reverses passed by the floor.  BBA's whole structure authored in
full (fourth suit 9+ GF with opener's answers and responder's placement,
`2NT` 6–8, `2M` / `3m` / `3y` 6–9, `3NT` 9–14, heart raise and keycard ask)
**lost** against the net floor on two seeds (1791362460 plain −0.0004 /
−0.0007; with a `6NT` rung on 15+, 1791363144 plain −0.0002 / −0.0003):
every loss a game or slam the floor's shape evaluation bid better than an
HCP band — its `3NT`, `6NT`, `7NT` jumps make — the fourth suit itself at
zero, every gain a weak hand with a fit the floor had passed.  Cut to that
weak table (`rebid.reverse_weak_responses`, ≤7 HCP, partial, 8+ and the
misfit the floor's) it is a wash on two seeds (1791366826, 1791367325:
pooled plain +0.0001 / +0.0000, PD +0.0001 / +0.0000, every cell inside its
CI; ~20 boards fire per 204,800).

| Park | Numbers | Flip plan |
| --- | --- | --- |
| `park/reverse-weak-responses` (code, knob, tests, `scripts/ab-reverse-weak-responses.sh` and CHANGELOG entry live there, nothing on `main`) | wash, two seeds, pooled plain +0.0001 / +0.0000, PD +0.0001 / +0.0000 | the weak heart raise on five-card support loses to the floor's direct `4♥` on every seed; add a `4♥` rung on 5+ support (or drop the raise), rebase, re-measure |

Two findings on the way:

1. **A partial table is unsound under the systems-on rebase** — their
   double stripped, the uncontested key found, the rejection answered as a
   pass of the forcing reverse (−59 IMPs on one seed, three boards).  The
   fix is a `Pattern::guarded` row admitting only `context.undisturbed()`,
   listed in `KNOWN_PARTIAL_TABLES` (`rows.rs`); that is now the idiom for
   any book node that means to leave hands to the floor
   ([bidding-architecture.md](bidding-architecture.md) § Resolution and
   shadowing).
2. **The net floor already plays BBA's reverse structure** (it is distilled
   from it), so the 4SF program's remaining two-level nodes are not a book
   gap.

Results in `ab-results/reverse-fourth-suit-v{1,2}` and
`ab-results/reverse-weak-responses{,-2,-v3,-v3-2,-v4,-v4-2}`.

## Owed

- The `1♥ - 1♠ - 2♣ - 2♦` tail places only at game.  Re-counted on the
  shipping arm: 757 boards, −1,114 plain per 409,600, our game against
  BBA's slam or grand 104 of them, −909 (by deal: clubs 17, hearts 14,
  spades 8, notrump 10).  The major-fit keycard ask
  (`rebid.fourth_suit_keycard`, 16+ points, two keycards) measured a
  **wash** 2026-10-07 (two seeds, every cell +0.0001 to +0.0003 and inside
  its CI; CHANGELOG) and stays opt-in.  Untouched: the club slams (a `4NT`
  names one suit per node, so clubs need their own ask — a `4♣` set-trump
  or Kickback) and the no-fit notrump slams (a quantitative `4NT`).
- The card row / ledger row 58 correction, once the disclosure decision
  lands.
- **Flag — `1♠ - 2♣ - 2♦ - 4♥`:** 22 boards, responder 11–12 HCP with a
  spade void and 6–7 hearts, the floor's natural jump in the fourth suit,
  passed 22 / 22.  Proposed default: a natural `2♥` rung (BBA's reading) in
  `responder_rebid`, measured with the lane.  Low priority.
- The park's flip plan (table above).
