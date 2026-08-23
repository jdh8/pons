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
| The walk's own bound (ceilings are not enough) | **measured** — §3 |
| The book by region | **shipped** — §5, from a 55 792-node run |

No bidding behaviour changes from this work: it is reference and tooling only,
so no A/B is owed. Anything authored *from* it goes through
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

# The walk, sharded across every core.
RUN=ab-results/bba-book/$(date +%F)-$(git rev-parse --short HEAD)
scripts/idle-run.sh scripts/bba-book.sh "$RUN" --corpus corpus --min-reach 2

# Read one lane back.
cargo run --release --features serde --example probe-bba-book -- \
    --render "$RUN" --prefix "1♠ (2♥)"
cargo run --release --features serde --example probe-bba-book -- --stats "$RUN"
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
whole `stopper !X` family is the floor's notrump reader wearing a name. **Every
floor share in §5 is therefore a lower bound**, and the two dominant labels of
the census (`bidable suit` 31%, the `stopper` family 12%) are the reason.

Proposed default, reversible either way: keep the partition keyed on
`calculated bid` alone, as it is now, and report the generic-label share beside
it rather than folding the two together. Folding them would be one flag
(`--floor-labels`) and would shrink the walk considerably — but it would also
silently reclassify every genuine natural book rule as floor, and no evidence
here separates the two. **Left for the user to decide.**

**`feature[417]` is invisible to the reader.** The flag is raised on `Item[14]`,
the bidding side's staging slot; the walk reads positions 0..3. Measured over
47 720 readings of the census dump: `feature[417]` appears **zero** times, on
`calculated bid` children and book children alike. There is no machine-readable
floor bit on the interpretation path, only the string.

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
| `2♦` | 4-10, 6-7♦, ≤3♥ ≤3♠ | `Weak natural 2D` — **not Multi** |
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

| family | book readings | `calculated bid` | floor share |
| --- | ---: | ---: | ---: |
| opening | 144 | 0 | 0.0% |
| constructive | 368 878 | 54 747 | **12.9%** |
| contested | 818 317 | 133 360 | **14.0%** |

Both figures are **lower bounds** — §2's generic-label caveat. Verdicts over
1 375 446 children: 1 071 588 above a ceiling, 213 221 stopped at the reach gate,
55 791 expanded, 24 322 floor dead ends, 10 524 auction ends.

**The headline is where the floor sits, not how big it is.** BBA's *simple raise*
is `calculated bid` — `1♠ - 2♠`, `1♠ (2♥) 2♠`, `1♠ (1NT) 2♠`, all of them — and
so is `1NT - 3NT`. Those are not obscure corners: the reach corpus puts
`1♠ - 2♠` on 0.75% of boards and `1♠ - 1NT` on 2.27%. The most ordinary
constructive calls in bridge are the ones BBA has no rule for and hands to its
bilans engine. Anything that models "BBA's book" as the thing to beat is
modelling the wrong half of these auctions.

### 5.7 Divergences from `american()` worth an A/B

Observations only — nothing here is a proposal, and each would go through
[measurement.md](../measurement.md). Cross-reference
[21gf-ledger.md](21gf-ledger.md).

1. **`1NT` with a 6-card minor.** BBA opens 15-17 `1NT` on 2-6 clubs or
   diamonds. Where we open the minor, the anchor is already in notrump.
2. **`3NT` opening = 25-27 balanced**, four stoppers. We should check what we do
   with that hand and whether it matters at all (it is rare enough that this may
   be a curiosity rather than a lead).
3. **`1M - 3M` is invitational** (10-12, `1M-3M inviting = 1`), not preemptive.
   That is a card row we chose; the blocking alternative is `1M-3M blocking`.
4. **`Support 1NT`** over a takeout double — 7-9 with three-card support, in
   place of a natural 1NT. [takeout-double-layers.md](../takeout-double-layers.md)
   has no entry for this seat.
5. **The takeout double's floor moves with the seat**: 12+ direct over an
   opening, 10+ in the `1♠ (2♥)` sandwich. Compare
   [takeout-double-layers.md](../takeout-double-layers.md), which has the
   4-4-major rung table but no seat-dependent floor.
6. **`2♣` response is not unconditionally forcing** — `1♠ - 2♣ - P` reads 11-12
   with 5♠.

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

| id | name | on because | truthful? |
| --- | --- | --- | --- |
| 25 | `1NT opening shape 5 major` | system-type seed | **yes** — `Wide6322` admits 5M(332) |
| 116 | `NMF after 2NT rebid` | system-type seed | **unchecked** — we have no authored `1m - 1M - 2NT - 3♣`; the floor bids it |
| 88 / 89 / 90 | `Lavinthal from void / on ace / to void` | constructor | card-play signals; no bidding read |
| 98 / 99 | `Mark on queen / king` | constructor | card-play signals; no bidding read |

A silent-on convention is disclosure we never made. The fix is one generated row
apiece in `card.rs` (`1NT opening shape 5 major = 1`, `NMF after 2NT rebid = 0`
or `1`, once someone decides what we play there); the engine accepts any name
it knows, whether or not BBA's UI lists it. **Left as a flagged default** — 25
is already truthful and 116 needs a probe of the floor's actual 3♣.

**Rows whose final state differs from what we wrote — three pairs, row order
did it.** `verify_card` writes and reads each row in turn, so a pair rule that
fires on a *later* row is invisible to it:

| rows (in card order) | wrote | BBA plays | why |
| --- | --- | --- | --- |
| `1X-(1Y)-2Z strong` (29), `1X-(1Y)-2Z weak` (30) | 0, 0 | **1**, 0 | each write sets its twin to `!value`; the last row written wins and flips the other. We meant "neither" — row 28 `1X-(Y)-2Z forcing = 1` is the agreement — and got "strong" |
| `Fourth suit` (63), `Fourth suit game force` (64) | 1, 1 | **0**, 1 | writing `64 = 1` clears 63. Harmless: 4SF-GF is what a 2/1 card means |
| `King ask by 5NT` (83), `King ask by 5NT inviting` (84), `King ask by available bid` (85) | 1, 0, 1 | **0**, 0, 1 | writing `85 = 1` clears 83 and 84. `card.rs`'s comment says 85 is *inert* and "83 stays 1 because that is the row BBA acts on" — the decompile disagrees on both counts: the setter clears 83, and `EPBot.cs:43964` reads 85 to place the king ask on the next step instead of 5NT. The *state* is nevertheless the truthful one — our king ask **is** the step above the queen reply (`instinct::king_relay`) — so only the comment is wrong. Flagged, not edited |

Row 29's flip is the one that misdescribes us. Proposed default: write
`1X-(1Y)-2Z weak = 0` **before** `1X-(1Y)-2Z strong = 0` in `SCHEMA` (the
final write then leaves strong 0 / weak 1, which is at least the nearer of the
two to a forcing free bid), or drop both rows and let the seed's `weak = 1`
stand. Either is a card change, so it waits for the user.

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

- **The generic-label question of §2** — whether `bidable suit` / `balanced` /
  `stopper !X` should count as floor. The whole floor-share number turns on it.
- The six divergences of §5.7 are observations. Each needs a probe before it is
  an experiment, and none has one.
- §6.1's two flagged card changes: the `1X-(1Y)-2Z` row order (we disclose
  "strong" and mean "neither") and the two silent-on seeds (25, 116) that the
  card should write explicitly. Both are `card.rs` changes and wait for the
  user; the `King ask` comment in `card.rs` is wrong about the engine but the
  state it leaves is truthful.
- **Shard balance.** The frontier splits on auction length, so the all-pass key
  `- - -` carries an entire seat's tree in one process (3 142 nodes, 173 s — it
  finished eight minutes after the other 1 120 shards). `FRONTIER=4` would split
  it at the cost of ~18 000 shards; re-sharding oversized keys only would be
  better, and is unwritten.
- The walk reads the **interpreter's** book. §2 shows the interpreter mirrors the
  bidder's fallback, but the two are separate code paths and nothing here
  measures where they disagree. A `--selfplay` cross-check — does BBA's own call
  at a node carry the label the walk predicts? — would close it.
