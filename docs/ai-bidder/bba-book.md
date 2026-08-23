# BBA's book: what every call means, by its rules

**Question answered here.** BBA/EPBot is the anchor
[the gap campaign](../bba-gap-campaign.md) measures against, and until now
nobody had mapped its *book*: what each call means by its **rules**, as opposed
to what its bilans floor does with a hand. Every decode so far was an island —
[the 1NT lanes](bba-1nt-defense.md), [the keycard family](bba-kickback.md),
[the bilans arithmetic](bba-floor.md) — and
[takeout-double-layers.md](../takeout-double-layers.md):99 records that not even
a one-of-a-suit opening had a live census.

This is the systematic walk. It rests on one fact: **EPBot will tell you what a
call means without being dealt a hand.** `epbot_get_info_meaning` returns the
engine's own name for the rule that matched — `Jacoby 2NT`, `Drury`,
`takeout double`, `Multi-Landy, both majors` — and returns `calculated bid` when
no rule matched and the bilans floor chose the call. Replay an auction onto a
hand-free bot and the label is the book, read straight off the engine.

## Status

| piece | state |
| --- | --- |
| Interpretation ABI, bound and self-checked | **shipped** — `BbaOracle::interpret` and friends |
| The walker, renderer, statistics, self-play reach corpus | **shipped** — [`examples/probe-bba-book`](../../examples/probe-bba-book/main.rs) |
| Sharded runner | **shipped** — [`scripts/bba-book.sh`](../../scripts/bba-book.sh) |
| Dictionaries: convention ids, `feature` slots, label emission paths | **shipped** — §4, baked into the example with a drift check |
| The book/floor boundary, located in the decompiled engine | **shipped** — §2 |
| Interpreter-vs-bidder containment cross-check | **measured** — 873/10 711 calls outside the hand-free reading, §2 |
| The walk's own bound (ceilings are not enough) | **measured** — §3 |
| The book by region | **shipped** — §5, from a 55 792-node run |
| Six suspected `american()` divergences | **probed and triaged** — §5.7, reproducible script checked in |
| Generated-card truthfulness | **corrected; default vectors and shipped model output unchanged** — §6.1; ids 25/116 wait for a raw-card feature bump |

No **pons** bidding behaviour changes from this work, so no A/B is owed; the
fixed-seed `smoke-default` dump is byte-identical.  The card row-order fix does
change what **BBA** plays in `1X-(1Y)-2Z` lanes, so it is an anchor series break
for that slice. Anything authored *from* these probes goes through
[measurement.md](../measurement.md) like everything else.

## Reproduce

```sh
# The ABI and its six invariances (a)-(f).
cargo run --release --features serde --example probe-bba-book -- --self-check

# EPBot's convention table, checked against the copy baked into the example.
cargo run --release --features serde --example probe-bba-book -- --conventions 200

# The reach corpus: BBA against BBA, every auction prefix counted.
#   SEED_BASE=$(date +%s); one shard per core, offset by COUNT (see below).
cargo run --release --features serde --example probe-bba-book -- \
    --selfplay 10000 --seed "$SEED_BASE" --output corpus/reach-"$SEED_BASE".jsonl

# Does the hand-free reading contain hands BBA actually bids this way?
cargo run --release --features serde --example probe-bba-book -- \
    --crosscheck 1000 --seed 1787487734

# The walk, sharded across every core.
RUN=ab-results/bba-book/$(date +%F)-$(git rev-parse --short HEAD)
scripts/idle-run.sh scripts/bba-book.sh "$RUN" --corpus corpus --min-reach 2

# Read one lane back.
cargo run --release --features serde --example probe-bba-book -- \
    --render "$RUN" --prefix "1♠ (2♥)"
cargo run --release --features serde --example probe-bba-book -- --stats "$RUN"

# The six §5.7 questions and the two silent-on card rows.
scripts/bba-book-divergence.sh
```

**Seed hygiene.** `common::seeded_deals` seeds board *i* as `seed + i`, so
`--selfplay` shards must be offset by their **count**, not by 1 — unlike
`bba-gen`, whose own rng makes `seed + i` disjoint. Shard *j* takes
`--seed $((SEED_BASE + j * COUNT))`.

**The card is a build input.** Every figure below is BBA playing
[`cards/American.bbsa`](../../cards/American.bbsa) on **all four seats** — the
book BBA plays when told our agreements. `--card none` gives the engine's own
2/1 defaults instead, and the two differ (§5). `scripts/bba-book.sh` records the
card, the SHA and the flags in `RUN` beside the dump.

## 1. Method: reading the book instead of playing it

`BbaOracle::interpret(vul, auction)` builds a fresh bot, deals it **nothing**
(`epbot_new_hand` with an empty holdings string), replays the auction with
`set_bid`, and reads the last caller's slot. `epbot_get_bid` is never called, so
the bilans engine never runs — which is why one reading costs ≈ 1.9 ms and a
book walk is possible at all. Compare `BbaOracle::probe`, which is the opposite
read: what the *floor* does with a hand.

Six things had to be true for that to be the right instrument. All six are
checked by `--self-check` and all six hold:

| | check | verdict |
| --- | --- | --- |
| (a) | a hand-free reading equals a dealt-hand reading | **120/120 labels.** The extended prose differs on 5/120 — all of them `calculated bid`, where with no rule to quote the prose paraphrases the hand model instead. The label, which is what the walk records, never moves. |
| (b) | the reading does not depend on which seat is looking | **45/45**, labels *and* all four public blocks |
| (c) | the reading does not depend on vulnerability | 15/15 on the battery, and **19 438/19 438** across a four-vulnerability walk of the whole `1♠` subtree — not one reading differs. See below. |
| (d) | what `extended` and the convention getters add | `extended` is `h` and `n` respelled in English — dropped from the dump by default (`--extended` restores it), halving it |
| (e) | the buffer-too-small status is real | `-3` at 4 bytes, `0` at 32; `read_str` retries once at 16 KiB |
| (f) | one bot replayed equals a fresh bot per prefix | **47/47** — EPBot carries no state across `set_bid` that a fresh replay would miss |
| (g) | the vulnerability argument reaches the engine at all | **3/400** opening calls move between green and red. Live, but barely — see below |

(b) and (f) together are what make the walk cheap and shardable: a node's
reading is a pure function of its auction.

**Vulnerability is not an axis of BBA's reading.** (c) is a strong null: 19 438
children of the `1♠` subtree, read at all four vulnerabilities, produced 19 438
single readings — zero variance, not even in a point band. That is worth a 4×
saving on every run, but only if the argument is live at all, which is what (g)
exists to rule out: BBA's *bidding* does move with vulnerability, on 3 of 400
opening hands. Live, and weak — EPBot's preempt style keys on suit quality, not
on the colours — which is exactly why its reader can afford to ignore it.

So the shipped walk runs `--vuls none`. A periodic four-vulnerability re-read of
one subtree is the check that keeps that honest; the dedupe machinery stays in
the dump schema (`Reading::v` is a vulnerability bitmask) so turning the sweep
back on costs nothing but time.

## 2. The book/floor boundary

`"calculated bid"` is `ModuleCommon.STR_CALCULATED_BID`, and the line that
emits it is the whole partition:

```csharp
// EPBot.cs:56569, in odzywka_z_bilansu_exit_function — every return path
// of the bilans engine funnels through here (61 call sites, all inside
// odzywka_z_bilansu).
if (Item[14].znaczenie == null)
{
    Item[14].znaczenie = "calculated bid";
    set_feature(14, 417, 1);          // F_ODZYWKA_BILANSOWA
}
```

If any book rule already wrote a label into the staging slot, the bilans keeps
that label and the call counts as a **book** call. If the slot is still empty,
the call is the **floor's**. That `if` is the boundary.

The reading side has a mirror at `EPBot.cs:2903`, inside `interpretuj_odzywke`:
when the phase-specific `*_interpretacja` produces nothing, the interpreter runs
its own cascade — `interpretuj_kontre_rekontre` → `zgloszenie_*_interpretacja` →
`set_en_passant` → `set_cue_bid` → `set_bilansowe_ba` →
`interpretuj_kolor_przeciwnika` → `set_wywiad_bezatutowy` →
`interpretuj_kolor_bilansowy` — and stamps `"calculated bid"` at `:3013` if the
slot is still empty. **BBA reads an unrecognised call exactly the way it makes
one.**

### Two caveats that bound what the label proves

**The label is a sound but incomplete floor detector.** Every `calculated bid`
is the floor. Not every floor call says `calculated bid`, because the reading
bilans has *named* exits as well as the anonymous one, and those names are
strings the book also uses:

| site | label it writes | also written by |
| --- | --- | --- |
| `interpretuj_kolor_bilansowy`:3124 | `calculated bid` | — |
| `interpretuj_kolor_bilansowy`:3129, :3487 | **`bidable suit`** | 197 other sites, most of them real `*_interpretacja` readers |
| `set_bilansowe_ba`:31001 | `calculated bid` | — |
| `set_bilansowe_ba`:30993, :30997 | **`balanced`** | — |
| `set_bilansowe_ba`:30985 | **`stopper !X …`** | — |

`:3124` and `:3129` are the two arms of one `if` — the suit is too short to read
as a bidable suit, or it is not. So a `bidable suit` reading is the floor's
natural-suit reader *or* a book reader, indistinguishable by string, and the
whole `stopper !X` family is the floor's notrump reader wearing a name. The two
dominant labels of the census (`bidable suit` 31%, the `stopper` family 12%) are
exactly the ambiguous ones, so **§5.6's `calculated bid` share is a lower bound
on how often BBA improvises.**

It bounds that and nothing else. The label answers *why BBA chose this call*; it
says nothing about *what the call means*, and the meaning is rule-shaped at
every node — `meaning_extended` returns a points band and four length bands
whatever the label. `1♠ - 2♠` is `calculated bid` **and** 7-9 with 3+♠, which is
our simple raise verbatim. So "can pons express this as a rule" cannot key the
partition: it is true of the whole census, `calculated bid` included, and would
collapse the column to zero. The key stays `calculated bid`, and §5.6 names it
for what it counts — a **no-rule share**, not a share of readings outside our
grammar.

**`feature[417]` is invisible to the reader.** The flag is raised on `Item[14]`,
the bidding side's staging slot; the walk reads positions 0..3. Measured over
47 720 readings of the census dump: `feature[417]` appears **zero** times, on
`calculated bid` children and book children alike. There is no machine-readable
floor bit on the interpretation path, only the string.

### Interpreter versus bidder: 8.15% fall outside the reading

`--crosscheck` bids fresh boards BBA against BBA, reads each resulting auction
with one hand-free `interpret_path`, and asks whether the hand that actually
made each call satisfies the reading's HCP band and changed suit-length bands —
exactly the `h`/`n` fields the renderer prints.  Dealer and vulnerability rotate
as in `--selfplay`; vulnerability is swapped into the dealer-as-position-0
frame when an E/W seat deals.

On **1 000 boards / 10 711 decisions**, seed `1787487734`, **873 calls (8.150%)**
fall outside their hand-free reading.  Aggregated by the interpreter's label,
the largest counts are unlabelled calls 303/6 295 (4.81%), `calculated bid`
205/855 (23.98%), `bidable suit` 145/1 174 (12.35%), and `takeout double`
34/96 (35.42%).  The failures are substantive rather than formatting noise:
BBA sometimes opens a 10-HCP five-card major outside the printed 11–20,
doubles directly with 10 where its reading says 12+, and uses its
`1M-3M inviting` call on 9 HCP where the reading says 10–11.

This does **not** move the label census or its no-rule share: the cross-check
replays the very call BBA made, and `calculated bid` still says which reader
path fired.  It bounds every numerical range in §5 instead: those ranges are
what BBA's interpreter declares, not a guarantee that its bidder stays inside
them.  A bidder-label disagreement rate is unavailable non-circularly.  On a
hand-holding bot the outgoing meaning slot is empty on its first decision and
stale on later ones after `get_bid`; only `set_bid` refreshes it, by invoking
the interpreter whose label is under test.  The existing dealt-vs-hand-free
interpreter check remains 120/120 labels, but it cannot answer the bidder-label
question.

## 3. The ceilings, and why they are not the bound

The campaign's ceilings are **`4♠` constructive, `3NT` contested**, judged by the
*child's* family — how many sides have made a non-pass call. So `1♠ - 4♠` is
inside the ceiling and `1♠ (4♥)` is a dead end. A dead end's meaning is still
read and recorded; only its subtree is skipped.

**They cap the level, not the length, and that is not enough.** A contested
auction under a `3NT` ceiling still branches on every pass and every remaining
bid below `3NT`. Measured: a walk of the single node
`1♠ (1NT) 2♣ (2♦) 2♥ (2♠) 2NT` — one depth-6 node — passed 2000 descendants
without terminating. The `1♠` census grew 1 → 13 → 93 → 511 → 2329 → 10301 nodes
over depths 1..6, a branching factor of ≈ 4.5 that decays only slowly, and the
`calculated bid` gate prunes just 18% of expansions.

So the walk takes its bound from **BBA's own play**. `--selfplay` bids out
boards BBA-against-BBA and counts every auction prefix; `--reach-depth D` then
expands a child longer than `D` only when the corpus reaches it `--min-reach`
times. The book stays exhaustive to `D`; below it the walk follows where BBA
actually goes.

A 120 000-board corpus (12 shards × 10 000, ~4 min on 12 cores) reaches
**287 194** distinct prefixes, of which **82 286** at reach ≥ 2:

| reach ≥ | prefixes | depth 4 | 6 | 8 | 10 | 12 | 16 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 287 194 | 4 493 | 16 062 | 30 483 | 32 761 | 24 587 | 8 954 |
| 2 | 82 286 | 3 164 | 8 586 | 11 667 | 9 215 | 4 820 | 860 |
| 5 | 26 691 | 1 902 | 3 740 | 3 537 | 2 431 | 1 020 | 109 |
| 25 | 4 885 | 678 | 719 | 546 | 318 | 77 | 5 |

`--min-reach 2` is the shipped default gate: a prefix seen once in 120 000
boards is a fluke, and dropping it cuts the walk 3.5× for nothing.

**The shipped configuration**, then, and what each part of it buys:

| flag | value | why |
| --- | --- | --- |
| `--reach-depth` | 4 | the whole book exhaustive through the advancer's first call |
| `--min-reach` | 2 | 82 286 prefixes rather than 287 194, dropping only the flukes |
| `--vuls` | `none` | measured worth nothing, costs 4× |
| `--extended` | off | measured derivable from `h` and `n`, costs half the dump |
| `FRONTIER` | 3 | the root process walks 184 nodes in 10 s and hands out 1 121 shards |
| `--card` | `cards/American.bbsa` | on all four seats: the book BBA plays when told our agreements |

`scripts/bba-book.sh` copies the walker **into the run directory** and shards
from that copy, so a rebuild mid-run cannot swap the program under the dump and
the dump ships with the binary that made it.

## 4. Dictionaries

Extracted from `vendor/bba/EPBot64.dll`, decompiled read-only with `ilspycmd`
9.1 (recipe in [bba-floor.md](bba-floor.md) §5.5), and cross-checked against the
shipped `.so` wherever the live library will answer.

### 4.1 Convention ids — 0..172

`epbot_convention_name(bot, index)` enumerates the whole table off the **live
library**, so this one needs no decompile at all:
`probe-bba-book --conventions 200`. Ids 173.. read `Not defined` — the filler
slots [`card.rs`](../../src/bidding/card.rs)'s `PONS_SCHEMA` deliberately parks
our own convention names on.

The table is baked into the example as `CONVENTIONS`, because it is what turns a
raw slot delta into `Kickback 1430` and the renderer must work on a dump alone.
`--conventions` diffs the baked copy against the live one and **fails** if the
vendored engine has moved underneath the dumps.

### 4.2 `feature[512]` — three disjoint regions

`ModuleCommon` declares a `const` name for every slot the engine touches; the
C# compiler inlines them, which is why the decompile shows bare literals.

| region | meaning |
| --- | --- |
| `0..173` | one flag per **convention** — the index *is* the convention id (`ustaw_konwencje` writes `feature[id] = 1`) |
| `173..300`, `325..400` | dead space; no read, no write |
| `300..325` | per-call bidding facts (`F_*`) |
| `400..512` | per-hand and per-auction state (`F_*`) |

135 named slots in the two `F_*` regions are baked into the example as
`FEATURE_NAMES`, so a delta in a dump prints as `force_partner=13` rather than
`f411=13`. The names are EPBot's own constants with `F_` stripped, Polish and
all, so a slot greps straight back to the decompile.

The ones a book reader meets constantly:

| slot | name | what it says |
| --- | --- | --- |
| 402 / 403 | `min_hcp` / `max_hcp` | the HCP band — a fresh seat is `0..37`, **not** `0..0` |
| 404 / 405 | `min_pkt` / `max_pkt` | total points, band `0..42` |
| 400 / 401 / 509 / 432 | `odzywka` / `pierwsza_odzywka` / `ostatnia_odzywka` / `poprzednia_odzywka` | this / first / last / previous call, as a bid index |
| 411 | `force_partner` | the bid index partner is forced to, `0` = not forcing |
| 314 / 437 / 444 | `alerting` / `alert` / `artificial_bid` | the alert triple, all three written together |
| 417 | `odzywka_bilansowa` | the floor flag — bidding side only, see §2 |
| 443 / 480 / 308 | `game_forcing` / `blocking_bid` / `preemptive` | force and preempt flags |
| 424 | `kolor_domniemany` | the presumed trump strain, `4` = NT/none |
| 425 / 441 | `zadane_pytanie_o_asy` / `rodzaj_zadanego_pytania_o_asy` | the keycard ask's **bid index**, and separately the **kind** of ask (a convention id) |
| 511 | `used_convention` | **the convention that fired** — `ustaw_konwencje` stamps it, and it is what distinguishes `Blackwood 1430` from `Kickback 1430` where the label cannot |
| 442 | `przeskok` | how far the call jumped |

Bid indices are `0`=P, `1`=X, `2`=XX, then `5·level + strain` with
`0=♣ 1=♦ 2=♥ 3=♠ 4=NT`, so `5`=1♣ … `39`=7NT. Suit index `4` is the
no-suit/notrump sentinel everywhere.

### 4.3 `Item[22]` — which hand model a reading belongs to

| index | role |
| --- | --- |
| `0..4` | the four seats' **public** state — what the auction has established, and all `interpret` reads |
| `4..8` | the *calculated probable hand* per seat — Stage 1's output, what [`probe`](bba-floor.md) reads |
| `8..12` | hidden |
| 12 / 13 / 14 | the call this bot **decided**, the call being **interpreted**, the bid choosers' **staging** slot |
| 20 / 21 | balance-engine result / the blank template `restore_Items` copies over everything each auction |

### 4.4 The label vocabulary

1029 emission sites, **293 distinct labels reachable**, by four paths:

| path | sites | how the label is formed |
| --- | --- | --- |
| direct `znaczenie = <expr>` | 698 (63 of them `null`, a slot reset) | unconditional overwrite |
| `ustaw_konwencje(…)` | 138 | `nazwa_konwencji(id)`, optionally `+ ", " + extra` — this is where `Jacoby 2NT`, `Drury`, `Multi-Landy, both majors`, `Cue bid, a !H stopper`, `Two way game tries` come from |
| `set_alert_artificial_force_partner` | 102 | sets the alert triple, then **overwrites** the label (default `"artificial"`) |
| `set_alert` | 91 | same alert writes, label **only if still null** — first writer wins |

Labels are frequently *built*, not literal: `strain_icon[suit]` interpolates, so
`5+ !S`, `shortness  !C` and `A=2/5 or 5/5, Q(!H)=1` never appear in the string
pool. The renderer maps `!C !D !H !S !N` to `♣ ♦ ♥ ♠ NT`.

Most-emitted: `bidable suit` 194 sites, `strong` 44, `calculated bid` 31,
`artificial` 31 (plus 108 more from the two alert helpers' defaults),
`accepts contract` 27, `to the partner's longer` 27, `minimum` 25, `support` 23.

### 4.5 Polish glossary

The engine is written in Polish. Enough to navigate it:

| | | | |
| --- | --- | --- | --- |
| `odzywka` call/bid | `licytacja` auction | `otwierajacy` opener | `odpowiadajacy` responder |
| `broniacy` defender | `obrona` defence | `wejscie` overcall | `otwarcie` the opening |
| `kolor` suit | `starszy` major | `mlodszy` minor | `ba`/`bezatut` notrump |
| `dlugosc` length | `krotkosc` shortness | `sklad` shape | `czworka` 4-card suit |
| `kontra` double | `rekontra` redouble | `wywolawcza` takeout | `karna` penalty |
| `blokujaca` preemptive | `przeskok` jump | `rewers`/`odwrotka` reverse | `powtorzenie` rebid |
| `uzgodniony` agreed | `sfitowany` fitted | `forsujacy` forcing | `inwit` invitation |
| `sila` strength | `pkt` points | `figura` honour | `pytanie o` ask for |
| `as`/`krol`/`dama` A/K/Q | `zgloszenie` showing | `znaczenie` meaning | `bilans` the balance engine |
| `interpretuj` interpret | `nazwa` name | `konwencja` convention | `wybrany gracz` chosen player |
| `ustaw` set | `aktualizuj` update | `wyklucz` exclude | `zapis` score |
| `wysokosc` level | `szczebel` step/rung | `koncowka` game contract | `rezygnacja` declining |

Unresolved: `wywiad` (a notrump probe or a stopper ask?), `expas`, `awers`, and
the `pro` of `odzywka_po_pro`.

### 4.6 Engine shape

1063 methods, 64 441 lines of body, 992 of them in `EPBot64/EPBot.cs`. Entry
point `EPBot.get_bid()` at `EPBot.cs:2525`. The dominant idiom is a
**writer / gate / reader** triple: `X` chooses the bid, `X_situation` asks
whether the auction is right for it, `X_interpretacja` decodes it when someone
else makes it. 95 writer↔reader name-twins, 16 writer↔gate. The 14 top-level
dispatchers are `odzywka{1,2,3}_{otwierajacego, odpowiadajacego,
pierwszego_broniacego, drugiego_broniacego}` plus `odzywka4_{otwierajacego,
odpowiadajacego}`.

`licz_*` and `oblicz_*` — names an earlier note guessed at — do **not** exist;
counting is `determine_*` (writes a table) and `get_*` (returns a value).

## 5. The book

The shipped run: **55 792 nodes, 224 MB, 4.0 KB/node**, one process per subtree
across 1 121 shards, **9 minutes** wall clock on 16 cores under `idle-run`. The
renderer walks it from the root with **0 dangling children**, so every expanded
call has its node. 1 375 446 readings, **156 distinct labels**.

`ab-results/bba-book/2026-08-23-08c54312-dirty` (the tree is outside git — the
`/mnt/hdd-data` symlink). Look a lane up rather than reading tables:

```sh
probe-bba-book --render "$RUN" --prefix="1♠ (2♥)"      # note --prefix=, see below
```

**Use `--prefix=`, not `--prefix `.** A third- or fourth-seat auction begins with
a pass (`- - 1♠`), and a space-separated value starting with `-` is a flag to
any argument parser. The walker sets `allow_hyphen_values`, so both forms work
now; the shard runner uses `--prefix=` regardless.

### 5.0 The card *is* the book

The single most important thing the walk establishes. Every label below is BBA
playing [`cards/American.bbsa`](../../cards/American.bbsa), and that is a
different system from EPBot's own 2/1 defaults. Four auctions, read both ways:

| auction | `--card none` (EPBot defaults) | `--card cards/American.bbsa` | the row that moved |
| --- | --- | --- | --- |
| `1♠ - 2NT` | `Jacoby 2NT` | `Jacoby 2NT` | — (`Jacoby 2NT = 1` both ways) |
| `1♠ - 2♠ - 3♣` | `Two way game tries` | `Cue bid, a ♣ stopper` | `Two way game tries = 0` |
| `1NT (2♣)` | `Multi-Landy, both majors` | `bidable suit`, 12-17 5+♣ | `Multi-Landy = 0` |
| `- - 1♠ - 2♣` | `Drury` | `bidable suit`, 5-10 6+♣ | `Drury = 0` |

All four left-hand readings are exactly what
[bba-1nt-counter-defense.md](bba-1nt-counter-defense.md),
[bba-kickback.md](bba-kickback.md) and the earlier probes recorded — so the
instrument reproduces every prior decode — and three of the four **change** once
we disclose our own card. Anything measured against the anchor is measured
against the right-hand column.

Likewise `4NT` as an *opening* reads `Not defined` rather than `4NT opening`,
because our card sets `4NT opening = 0`.

### 5.1 Openings — `--prefix=""`

| call | BBA reads it as | label |
| --- | --- | --- |
| `1♣` / `1♦` | 11-21, 3+, **≤4♥ and ≤4♠** | `3+ ♣` / `3+ ♦` |
| `1♥` / `1♠` | 11-20, 5+ | `5+ ♥` / `5+ ♠` |
| `1NT` | 15-17, **2-6♣ 2-6♦**, 2-5♥ 2-5♠ | `NT style` |
| `2♣` | 19+ | `strong`, alerted |
| `2♦` | 4-10, 6-7♦, ≤3♥ ≤3♠ | `Weak natural 2D` — **not Multi** (id 110 is one flag away; its book is walked in [bba-multi-2d-opening.md](bba-multi-2d-opening.md)) |
| `2♥` / `2♠` | 4-10, 6-7 | `Weak natural 2M` |
| `2NT` | 20-21, four stoppers | `NT style` |
| `3♣`-`3♠` | 4-10, 7-8 | `preemptive` |
| `3NT` | **25-27**, four stoppers | `NT style` — not gambling |
| `4♣`-`4♠` | 4-10, 8-9 (8-11 in a major) | `preemptive` |
| `4NT` and up | — | `Not defined` (`4NT opening = 0`) |
| `P` | ≤11 | *(no label)* |

The `1NT` shape band is the one to notice: **a 6-card minor is inside BBA's 1NT**
(`1NT opening shape 6 minor = 1` on our card), and a 5-card major is not.

### 5.2 Responses to `1♠` — `--prefix="1♠ -"`

| call | BBA reads it as | label |
| --- | --- | --- |
| `1NT` | 5-12, ≤3♠ | `forcing 1NT`, alerted |
| `2♣` / `2♦` / `2♥` | 12-29, 4+ (5+♥), ≤4♠ | `Shape Bergen structure` |
| **`2♠`** | 7-9, 3+♠ | **`calculated bid`** — the floor's |
| `2NT` | 11+, 4+♠, ≤4♥ | `Jacoby 2NT`, alerted |
| `3♣` / `3♦` / `3♥` | 4-6, 7+, ≤2♠ | `Weak Jump Shifts 3`, alerted |
| `3♠` | **10-12**, 4+♠ | `1M-3M inviting` — invitational, not preemptive |
| `3NT` | 13-15, 3♠, three stoppers | `support` |
| `4♣` / `4♦` / `4♥` | 12-20, ≤1 in the suit, 3+♠ | `Splinter`, alerted |
| `4♠` | 4-8, 4+♠ | `preemptive` |
| `P` | ≤5 | *(no label)* |

By a **passed** hand (`--prefix="- - 1♠ -"`) the structure changes on its own:
`1NT` becomes `balanced` 6-11 rather than forcing, `3♠` narrows to 10-11, and
`2♣` is a natural 5-10 6+♣ — our card has `Drury = 0`.

### 5.3 Responses to `1NT` — `--prefix="1NT -"`

`2♣` `Garbage Stayman` · `2♦`/`2♥` transfers · `2♠` `1N-2S transfer to clubs` ·
`2NT` `1N-2N transfer to diamonds` · `3♣` `1N-3C Puppet Stayman` · `3♦` both
majors 5-5, 6+ · `3♥`/`3♠` `1N-3M splinter` (9-25, singleton, 4 of the other) ·
**`3NT` `calculated bid`** · `4♦`/`4♥` `Texas` · `4♠` `Minors`.

### 5.4 Opener's rebid after a 2/1 — `--prefix="1♠ - 2♣ -"`

`2♠` and `2NT` are `minimum` (11-15); `3♣` is `support` 4+♣; `3♥`/`3♠` are
`bidable suit` **15-20**, so the jump rebid is the strength split; `3NT` is
`balanced` 17-20; `4♦`/`4♥` are `Splinter` with club support; `4♣` is `slam try`.
`P` reads 11-12 with 5♠ — BBA's `2♣` response is not unconditionally forcing.

### 5.5 Contested and defensive

The walk does not distinguish "our opening, they act" from "their opening, we
act": a node is an auction, and all four public blocks are read at every node.
`1♠ (2♥)` is one node carrying both readings.

`--prefix="1♠ (2♥)"`, the opener's side to act: `2♠` is the floor's again
(6-9 3+♠); `2NT` and `3NT` are `stopper ♥`; `3♥` is
`limit raise or better in ♠`, alerted; `X` is `takeout double` **10+**, against
**12+** for a direct double of an opening (`--prefix="1♠"`).

`--prefix="1♥ (X)"`: `1NT` is `Support 1NT` — **7-9 with 3+♥**, not natural;
`2NT` is `Jordan Truscott 2NT` 9+ 4+♥; `XX` is `penalty` 10+; the suit bids are
natural `bidable suit`.

`--prefix="1♠ (1NT)"`: `X` is `penalty` 10+, `2♠` and `3NT` are the floor's.

### 5.6 The book/floor partition

| family | labelled readings | `calculated bid` | no-rule share |
| --- | ---: | ---: | ---: |
| opening | 144 | 0 | 0.0% |
| constructive | 368 878 | 54 747 | **12.9%** |
| contested | 818 317 | 133 360 | **14.0%** |

**No-rule share, not floor share.** The column counts nodes where no authored
rule fired and the bilans chose the call, and both figures are **lower bounds** —
§2's generic-label caveat hides floor exits behind book strings. What the column
is *not* is a count of readings pons could not author: every node's meaning
arrives as a points band and four length bands, `calculated bid` nodes included,
so reading-expressibility is 100% here and carries no information. The axis that
separates BBA from us is the **choice**, not the meaning.

Verdicts over 1 375 446 children: 1 071 588 above a ceiling, 213 221 stopped at
the reach gate, 55 791 expanded, 24 322 floor dead ends, 10 524 auction ends.

**The headline is where the floor sits, not how big it is.** BBA's *simple raise*
is `calculated bid` — `1♠ - 2♠`, `1♠ (2♥) 2♠`, `1♠ (1NT) 2♠`, all of them — and
so is `1NT - 3NT`. Those are not obscure corners: the reach corpus puts
`1♠ - 2♠` on 0.75% of boards and `1♠ - 1NT` on 2.27%. The most ordinary
constructive calls in bridge are the ones BBA has no rule for and hands to its
bilans engine — and they are the ones pons authors as rules. Anything that
models "BBA's book" as the thing to beat is modelling the wrong half of these
auctions.

### 5.7 Six suspected divergences from `american()`, probed

[`scripts/bba-book-divergence.sh`](../../scripts/bba-book-divergence.sh)
replays the exact nodes and representative hands.  Nothing here changes
`american()`; the two surviving leads still owe fresh-seed A/Bs under
[measurement.md](../measurement.md) and live in
[21gf-ledger.md](21gf-ledger.md).

| observation from BBA's reading | what `american()` actually bids in the probe | verdict |
| --- | --- | --- |
| `1NT` admits a 15-17 hand with a six-card minor | `1NT` on all four 6m/6322 hands | **matches** — the card row is truthful |
| `3NT` opens 25-27 balanced | `2♣` on all four hands | **diverges**, but too rare and unsurprising to promote |
| `1M - 3M` is a 10-12 four-card raise | 10/11 support points bid `3M`; 12+ shortness bids Jacoby `2NT`; weaker hands bid `2M`/`4M` | **matches categorically** — `1M-3M inviting = 1` is truthful |
| after `1M (X)`, BBA's `1NT` shows 7-9 and 3+ support | pons raises `2M` with each 7-9 three-card-support hand and uses `1NT` only on the no-fit hand | **diverges; A/B lead** |
| BBA's takeout double starts at 12 direct; its negative double after `1♠ (2♥)` starts at 10 with 3+ in both minors, ≤4♥ and ≤2♠ | pons also starts at 12 direct. At the response seat its `X` instead requires 4+♥ and starts at 8: it doubles four 8-11 overlap hands, but passes all five BBA-shaped 2=2=4=5 controls from 8 through 12 | **diverges in shape and strength; A/B lead, but not a floor-only experiment** |
| BBA may pass `1♠ - 2♣` with an 11-12 minimum | pons rebids on all four route-valid tested minima; no pass | **diverges**, but copying one row would contradict pons's 2/1-GF premise |

## 6. Corrections to facts pons already records

**Decided 2026-08-23** — the user took every proposed default; the
"resolution" column says what was done. Rows 7 and 8 grew into the card audit
of §6.1, which found more than the two rows it set out to check.

| # | recorded | what the decompile says | resolution |
| --- | --- | --- | --- |
| 1 | `feature[144]` = "partner mid-cue" ([bba-kickback.md](bba-kickback.md):89, :270; `oracle/mod.rs`) | 144 is `CONVENTION_SPLINTER`, written only by `set_SPLINTER*`. The real cue flag is **52**, `CONVENTION_CUE_BID`. EPBot *does* treat 144 as cue-equivalent in `get_cue_bid` and `interpretuj_blackwooda`, so the operational use is defensible — but a dump keyed on 144 misses every real cue bid. | **both**, by the user's reading of the theory: a splinter *promises* a control and opens control bidding — it is a stricter control bid, not a different thing — so 144 is cue-equivalent by design (and it is what makes splinters useful beside other forcing raises, which carry the non-splinter slam tries). The engine's own Kickback suppression (`interpretuj_blackwooda`:3712) reads exactly `144 OR (52 AND NOT 77 AND below game)` — the splinter flag outright, the cue flag only below game of the agreed suit and never after Jacoby 2NT. Relabelled in [bba-kickback.md](bba-kickback.md) §1/§5 and the census predictor now mirrors all three terms |
| 2 | `feature[406]` = aces | counts **keycards** — the trump king is routed into 406 by `aktualizuj_partnera_LHO_RHO`:30017 | done — `SeatInfo` doc comment |
| 3 | `feature[425]` = "asking-bid code" | 425 is the ask's **bid index**; the *kind* of ask is **441**. Two EPBot slots behind one pons field. | done — it was only ever a doc comment; now reads 425 = bid index, 441 = kind |
| 4 | `feature[319]`: "BBA swaps the meaning of −1 and 0" | EPBot has no swap: −1 never asked, 0 denies, 1 holds. The appearance of a swap is `SeatInfo::features` keeping only **nonzero** entries, so a meaningful `0` is indistinguishable from an absent slot — the same hazard hits 411, 314, 406, 407. **`probe-bba-book`'s own deltas are unaffected**: it walks the union of the before and after key sets, so a slot falling to 0 is recorded. | done — the swap claim is gone and the hazard is on the `features` field |
| 5 | `SeatInfo::hcp_range` defaults a missing slot to 0 | a fresh seat's `403` is **37**, not 0, so `(0, 0)` would be a false "max 0 HCP" reading. Harmless today because 37 ≠ 0 survives the filter. | done — `hcp_range` |
| 6 | `epbot_get_used_conventions(bot, item)`, semantics unknown | the argument is a **convention id** and the return a *cumulative* count of times it fired on this bot. Bound correctly now as `BbaOracle::convention_usage`; it reports `1NT` → `22:1NT opening range 15-17`, `2♦` → `168:Weak natural 2D`, `1♠ - 2♠ - 3♣` → `52:Cue bid`. It stays silent on some auctions whose label *does* name a convention (`1♠ - 2NT` → `Jacoby 2NT`), so it is not a substitute for `feature[511]`. | confirmed by the user: it is the "conventions used" tally BBA's UI displays — statistics, not state. Left as a diagnostic; `feature[511]` is the reading |
| 7 | `cards/American.bbsa` discloses what we think it does | `Direct Jump Cuebid` and `Transfers if RHO passes` exist in `vendor/ben/21GF.bbsa` (hence in our card) but **not** in `nazwa_konwencji`: the `.bbsa` schema is BBA's *UI* name list, not the engine's, and `epbot_set_conventions` drops them silently. `verify_card` cannot see it while both are written 0. 42 engine ids are absent from our card; 6 (13, 48, 53, 146, 147, 172) have no bidding-code read at all. | **audited** — §6.1, with `probe-bba-book --effective 175` as the reproducible form |
| 8 | — | Setter side effects nobody has measured: mutual exclusion makes `.bbsa` **row order** significant; id 131 (ROPI) is unsettable and hard-wired off; id 140 (SMOLEN) is forced off under Acol; setting 70 forces 69 on. | **transcribed** — §6.2, the whole setter |

### 6.1 The card audit

`probe-bba-book --effective 175` writes `cards/American.bbsa` onto a fresh bot
exactly as [`interpret`](../../examples/common/oracle/mod.rs) does and reads
every id back. Three things separate *what we wrote* from *what BBA plays*:

**Rows the engine has no id for — inert either way.** `Direct Jump Cuebid` and
`Transfers if RHO passes` are BBA's *UI* rows, inherited from BEN's
`21GF.bbsa`. The engine's `convention_index` does not know the names, so
`epbot_set_conventions` drops them whether written 0 or 1 (a `1` would at least
trip `verify_card`; a `0` passes silently). What they stand for:
`Transfers if RHO passes` is the uncontested baseline beside the real ids
`Transfers if RHO doubles` (155) and `Transfers if RHO bids clubs` (156) — the
engine needs no id because plain Jacoby covers it. `Direct Jump Cuebid` is the
UI's *header* for a six-id radio group the engine does have — over their major
opening `(1M) 3M` as **Gambling** (solid suit, stopper ask) / **Minor** / **Strong**
(95–97), over their minor `(1m) 3m` as **Gambling** / **Majors** / **Preempt**
(100–102). Our card writes none of the six, all read 0, and BBA plays none —
truthful, since we author none either.

**Ids our card never writes — 42 of them.** Every one reads 0 after the card
lands except the seven below, which `set_system_type(0)` (the 2/1 seed) or the
constructor's `initialize_CONVENTIONS` turns **on** and nothing in the card
turns off:

| engine id | name | on because | truthful? |
| --- | --- | --- | --- |
| 25 | `1NT opening shape 5 major` | system-type seed | **yes** — `Wide6322` admits 5M(332) |
| 116 | `NMF after 2NT rebid` | system-type seed | **no** — the exact responder node has no `3♣`; after a forced `3♣` and pass, the generic floor bids `3NT` on every tested opener rather than giving an NMF answer |
| 88 / 89 / 90 | `Lavinthal from void / on ace / to void` | constructor | card-play signals; no bidding read |
| 98 / 99 | `Mark on queen / king` | constructor | card-play signals; no bidding read |

A silent-on convention is disclosure we never made.  The values are now known:
id 25 should be `1`, id 116 should be `0`.  Adding two generated rows today,
however, would change `LEN_CARD_ROWS` 135→137, `LEN_CARD` 140→142 and the raw
v4 feature width 368→372.  That raw-card ABI still has a checked-in artifact,
so both explicit rows wait for its next feature-ABI bump, retrain or retirement.
The shipped v6 floor reads compact agreement features by row name and is not
silently depending on those positions.

**Rows whose final state differs from what we wrote — three pairs, row order
did it.** `verify_card` writes and reads each row in turn, so a pair rule that
fires on a *later* row is invisible to it:

| rows (in card order) | wrote | BBA plays | why |
| --- | --- | --- | --- |
| `1X-(1Y)-2Z weak` (30), `1X-(1Y)-2Z strong` (29) | 0, 0 | **1**, 0 | each write sets its twin to `!value`; writing weak first makes strong the final `0`, which flips weak to `1`. BBA offers no "neither" state; weak is nearer to our forcing free bid than the old effective strong setting |
| `Fourth suit` (63), `Fourth suit game force` (64) | 1, 1 | **0**, 1 | writing `64 = 1` clears 63. Harmless: 4SF-GF is what a 2/1 card means |
| `King ask by 5NT` (83), `King ask by 5NT inviting` (84), `King ask by available bid` (85) | 1, 0, 1 | **0**, 0, 1 | writing `85 = 1` clears 83 and 84. `EPBot.cs:43964` reads 85 to place the king ask on the next step instead of 5NT, which is exactly `instinct::king_relay`; the stale `card.rs` comment was corrected, with no value change |

The weak/strong order is now corrected in `SCHEMA`, and the generated American
and Dutch cards were re-blessed.  Their literal bit vectors remain byte-identical
because both rows contain `0`; `smoke-default --count 20000 --seed 1` is likewise
byte-identical, SHA-256
`eccf17bc3e2818be26a97750eb74949d9af4075a09eb5e9f735cd2508e1a3ed0`.
Unequal foreign cards do attach the two raw positions to the opposite names,
but both corresponding v4 input columns are folded to zero in the shipped W1.
`--effective` correctly flags weak row 30 as `wrote 0, plays 1`: the complement
setter makes that honest final state impossible to spell without an override.

### 6.2 The convention setter, transcribed

`EPBot.conventions` (set), `EPBot.cs:1101–1400`. A name resolves through
`convention_index`; a numeric string is taken as the id. Out of `0..=255`:
ignored. Then, in this order:

1. **Hard-wired.** 131 (ROPI) returns without writing — unsettable, always
   off. 140 (SMOLEN) is forced off under Acol (`system_type == 4`).
2. **Radio groups — a `0` write is ignored.** Writing `1` clears the group and
   sets the one; writing `0` returns without touching anything, so a member can
   only be turned off by turning a sibling on:
   - 2♦ openings: 38 Benjamin, 59 Flannery, 65 French, 110 Multi, 121
     Precision, 146 Strong natural 2D, 168 Weak natural 2D, 171 Wilkosz
   - two-level jump shifts: 142 Soloway, 143 Soloway Extended, 148 Strong, 166 Weak
   - three-level jump shifts / raises: 39 Bergen, 76 Inviting, 105 Mini
     Splinter, 125 Reverse Bergen, 149 Strong, 167 Weak
   - 1NT range: 19 (12–14), 20 (13–15), 21 (14–16), 22 (15–17)
   - `1N-3D`: 11 majors, 12 minors, 13 natural, 14 splinter
   - after 2NT, with a twist: 33/34 (transfer to clubs/diamonds) are written
     **as a pair** and clear 106/107/108 (Minor Suit Slam Try / Stayman /
     Transfers after 2NT) on a `1`; any of 106–108 on a `1` clears 33, 34 and
     the other two. Here a `0` **does** write.
3. **Everything else writes, then a twin rule runs:**
   - complement pairs (`twin = !value` — a `0` turns the twin **on**): 3/4
     (`1M-3M blocking/inviting`; under Acol 4 is forced on), 17/18 (`1NT
     natural/NT style`), 29/30 (`1X-(1Y)-2Z strong/weak`), 138/145 (`Shape
     Bergen/Strength Lawrence`)
   - exclusive sets (`others &&= !value` — a `1` clears the others, a `0` is
     inert): 37/72 (5NT pick a slam / Grand Slam Force), 63/64 (Fourth suit /
     game force), 40/41/42 (Blackwood 0123/0314/1430), 49/50/51 (Crosswood),
     80/81/82 (Kickback), 83/84/85 (King ask by 5NT / inviting / available
     bid), 55/126 (Drury / Reverse), 62/137 (Forcing / Semi-forcing 1NT),
     46/115/132/159 (Checkback / NMF / Roudi / Two-way NMF), 45/87/111
     (Cappelletti / Landy / Multi-Landy), 92/133, 93/134, 94/135 (each
     Lebensohl vs its Rubensohl twin), 129/139 (Reverse / plain Smith Echo),
     95/96/97 and 100/101/102 (the two `Direct Jump Cuebid` triples), 120/169
     (`Polish two suiters` / `Weak natural 2M`, and 169 also clears 110 Multi),
     0/1 (`1D opening with 4 / 5 cards`; a complement pair under Polish Club,
     `system_type == 2`), and the `1N-2S … 1N-3C` ladder 5–10 as a chain where
     each id clears only its **neighbours**: 5 Minor Suit Stayman, 6 `2S`
     transfer to clubs, 7 `2N` transfer to clubs, 8 `2N` transfer to diamonds,
     9 `3C` transfer to diamonds, 10 `3C` Puppet Stayman
   - one implication: 70 (`Gerber only for NT openings`) on forces 69 (Gerber)
     on; 2 (`1m opening allows 5M`) is forced on under Acol.

Seeds, for completeness: the constructor calls `set_system_type(0)` and then
turns on 30, 46, 54, 111, 83, 92, 93, 94, 55, 25, 6, 9, 108, 41, 145, 2, 63,
155, 156, 13, 35, 37, 44, 52, 56, 60, 69, 75, 77, 78, 104, 109, 112, 122, 131,
140, 144, 150, 151, 152, 154, 157, 158, 161, 162, 164, 88, 89, 90, 98, 99;
`set_system_type(0)` (2/1) additionally sets 62, 125, 4, 168, 169, 148, 2, 28,
18, 22, 24, 25, 116 on and 0, 1 off. A card row only ever *overrides* a seed;
an omitted id keeps it.

## 7. Open work

- **Shard balance, deferred until it matters.** The all-pass frontier key carries
  an entire seat's tree in one process (3 142 nodes, 173 s), but the complete
  walk still finished in nine minutes. Re-shard oversized keys only when a
  deeper `--reach-depth` or lower `--min-reach` makes that tail dominate wall
  time; a blanket `FRONTIER=4` would create roughly 18 000 shards for no present
  payoff.
