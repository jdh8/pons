# How BBA bids where authoring runs out — its *floor*, reverse-engineered

A study of EPBot (BBA's engine) aimed at one question: **what does a mature
bidding engine do when no fully-specified node matches the auction?** This is
the analogue of pons's own [`instinct()`](../../src/bidding/instinct.rs) floor,
and the point is to learn from BBA's structure — explicitly **not** to copy its
book.

Three questions, and the short answers:

1. **How is the floor authored?** As *parametric, generic rules* living in the
   same rule table as the specific ones — suit-variable templates, char-class
   ranges, and constraint-only rules — not a separate fallback module.
2. **How deep does explicit authoring go before the floor takes over?** Shallow.
   Specific literal-auction rules concentrate at 1–3 calls deep and essentially
   vanish past depth 5; the entire deep tail is generic.
3. **Ad-hoc or programmatic?** **Programmatic.** On deep off-book auctions the
   live engine computes a bid that tracks the hand monotonically and labels it
   literally **"calculated bid"**.
4. **What does "calculated" actually compute?** (§5, added 2026-07-20.) It
   reconstructs all four hands from the auction — placing individual honours —
   counts winners and losers analytically on that model, and picks the call by
   expected score. Not a rule table at all.

Reproduce (static side): `python3 scripts/bba_floor_stats.py`. The live-engine
side ran a throwaway `bba-floor-probe` example, since removed. The §5 metadata
side: `strings -n 5 vendor/bba/EPBotNET.dll`.

---

## 0. Does `MB.TXT` even drive the engine?

`vendor/bba/MB.TXT` is a 6094-line plain-text rule DB. First we had to know
whether it is the *runtime* rule source or a stale export. Verdict: **export.**
An `strace -e openat` of `examples/bba-oracle` shows `libEPBot.so` opens **zero**
external data files — not MB.TXT, EVAL.DAT, Comments.txt, or any `.bbsa`, not
even a failed (`ENOENT`) attempt:

```
openat(AT_FDCWD, "…/libEPBot.so", O_RDONLY|O_CLOEXEC) = 3   ← the only vendor/bba open
```

The rules are compiled into the NativeAOT binary. So MB.TXT tells us **what the
authors documented**, and §3's live probe confirms the compiled engine behaves
the same way. (`Comments.txt` is the alert-string vocabulary the `@NNN` tags
reference; the `.bbsa` files are per-system convention on/off toggles. Neither
is loaded at runtime either.)

---

## 1. The rule-line format

Each `MB.TXT` line is five space-separated fields; the auction pattern and its
`~constraint~` are glued together in field 2:

```
leader   pattern~constraint~term   weight   @alert   #index
```

| field | example | meaning |
|---|---|---|
| leader | `3X`, `0d`, `2` | bidding-context / state token (last level · flags like `X`=after double) |
| pattern | `1b.:P:P:X:.#b` | auction template; `:`-separated calls. `P`/`X`/`R`; **lowercase `a`–`h` are suit *variables*** (one rule, all four suits); `[1-7]`/`[^N]` ranges; `.`/`#` wildcards; `START`/`TERMINATE` catch-alls; `(…)*` repetition |
| constraint | `~#0(1,b)>2\|\|#0(2,b)>2~` | hand predicate. `#0(seat,suit)` = a hand-feature lookup (card count; `13` = HCP); `&&`/`\|\|`/comparisons; `$`/`$$` = terminal |
| weight | `0`–`99` | deterministic priority; higher wins |
| @alert | `@479`, `@05` | alert/description id into `Comments.txt` |
| index | `#1350` | node id |

Two things already answer "how do they author": rules are **parametric**
(`#0(seat,suit)` arithmetic computes the decision from the hand, the call is not
a fixed lookup) and **templated** (suit variables generalise one rule across
suits). The DSL is built to *generalise*, not to enumerate.

---

## 2. The floor is two-thirds of the table (static measurement)

Classifying all 6094 lines (`scripts/bba_floor_stats.py`). A line is **generic**
if its auction pattern generalises across auctions/suits (any of the axes
below); **specific** if it matches one concrete call sequence:

```
generic : 4034  (66.2%)
specific: 2059  (33.8%)
```

Non-exclusive generality axes:

| axis | lines | % |
|---|---|---|
| suit variable `a`–`h` (templated over suits) | 2584 | 42.4% |
| char-class `[..]` ranges | 1708 | 28.0% |
| empty pattern (constraint-only, matches any auction) | 1331 | 21.8% |
| Kleene `*`/`+` (variable-length auctions) | 142 | 2.3% |
| `TERMINATE` catch-all | 66 | 1.1% |
| `START` catch-all | 38 | 0.6% |

So **two-thirds of BBA's "book" is parametric**. (An earlier quick grep that put
generic at ~30% missed suit-variable templating and constraint-only rules — the
bulk of the floor.)

**Depth — explicit authoring is shallow.** Colon-segments per pattern, split by
class:

```
depth  1: 2690  (specific 1335 / generic 1355)
depth  2:  798  (specific  277 / generic  521)
depth  3: 1063  (specific  408 / generic  655)
depth  4:  103  (specific   24 / generic   79)
depth  5:   74  (specific   12 / generic   62)
depth 6+:   42  (specific    3 / generic   39)     max depth 15
```

Specific literal nodes live at 1–3 calls and all but disappear past depth 5; the
deep tail is essentially **all generic**. BBA does not hand-spell deep auctions —
it lets parametric rules carry them.

**Constraints carry the computation.** 56.5% of lines have a `~constraint~`,
containing **7470** `#N(...)` hand-feature calls and thousands of arithmetic
operators (`&&` ×9344, `||` ×3842, comparisons ×9k+). The bid is *computed* from
hand features inside the rule.

**Weights are the conflict resolver — bimodal.**

```
 0- 9: 1566      ← the soft floor (broad generic rules)
…
90-99: 2006      ← hard specifics / forcing
median 70, mean 56
```

A big mass near 99 (specific, near-forced) **and** a big mass at 0–9 (broad
generic). The floor rules are deliberately *low weight* so any more-specific
rule outscores them — the generic catch-alls are always present but always lose
to a real node when one exists. That is the whole mechanism that makes a single
broad floor rule safe to leave in the table.

---

## 3. The live engine confirms it (empirical probe)

`bba-floor-probe` (a throwaway, since removed — see Artifacts) drove EPBot down
**deliberately deep, off-book auctions** (where §2 says no specific node exists)
and read back the engine's own call *and its self-description* via
`epbot_interpret_bid` + `epbot_get_info_meaning[_extended]`.

**Probe 1 — HCP sweep on a depth-8 competitive auction**
`1C P 1H 1S 2H 2S 3H 3S`, N to call, hand shape fixed at 2=5=2=4, HCP swept:

```
 hcp  shape   bid   EPBot's meaning for its own call
   2   2524     P
   9   2524     P
  14   2524    4H   calculated bid
  19   2524    4H   calculated bid
  26   2524   4NT   Blackwood 0314, for !H
```

Two findings in one table:
- The call **escalates monotonically with the hand** (P → P → 4H → 4NT). It is
  *computed from hand features*, not a flat table miss → **programmatic floor**.
- EPBot labels its own floor bids **"calculated bid"** — its internal
  fallback descriptor (the string is baked into the binary; it is *not* a
  `Comments.txt` alert). Where a real convention node does fire (4NT over the
  heart fit), the label switches to the named meaning ("Blackwood").

**Probe 2 — fixed 18-HCP hand, growing auction depth**

```
depth  bid   meaning
    3   3H   limit raise or better in !S     ← shallow: a *named systemic* node fires
    6   4H   calculated bid                  ← floor takes over
    9    P   (blank)
   12    X   penalty
```

Exactly the §2 depth story: shallow auctions get a precise, *named* systemic
description; past where specific nodes exist the engine falls to "calculated
bid" — the programmatic floor — yet still produces a sane, graded action.
(The `_extended` range sentences degrade past the first round, the known limit
of EPBot's per-position introspection outside the BBA app; the **label** column
is the reliable signal.)

---

## 4. Contrast with pons's `instinct()` — and what's worth stealing

| | pons `instinct()` | BBA |
|---|---|---|
| floor location | a **separate** keyless `Rules` ladder, attached as a root `Always` fallback | **woven into the same rule table** as authored nodes |
| selection | **strict precedence** — `Trie::resolve` reaches the root last, so instinct only catches what every authored rule missed | **soft weights** — generic floor rules (weight 0–9) and specifics (90–99) coexist; weight breaks the tie |
| authoring unit | per-suit rules in the Constraint DSL | **suit-variable templates** — one rule covers all four suits (42% of the table) |
| conventions on the floor | **none by design** — instinct is all-natural until both sides of a convention are authored | the floor *can* fire conventions (reaches Blackwood as a "calculated bid" continuation) |
| degradation | binary: book hit → book; miss → instinct | graded: best-scoring rule wins, generic or specific |

Worth stealing for **pons's floor** (not its book):

1. **Suit-variable templating.** BBA authors a heart-and-spade (or any-suit) rule
   once with a variable; pons's `Constraint` DSL largely repeats per suit.
   Templating the *floor* over suits would cut authoring without touching the
   book — directly in line with
   [the "smarten the keyless floor" stance](../../CLAUDE.md) and
   `feedback_instinct_floor_over_node_authoring`.
2. **Inline parametric decisions.** BBA's floor *computes* the bid from
   `#0(seat,suit)` arithmetic (HCP/length) rather than enumerating. pons's
   instinct ladder is already context-driven; the takeaway is that a small set of
   parametric rules covers the deep tail that no enumeration can.
3. **Let the floor fire slam machinery — RKCB and control (cue) bids.** Slam
   bidding is inherently *conventional*, and it arises precisely in the deep
   auctions the floor owns: once a fit and extra values appear, the next step is
   a keycard ask or cue bids up the line, not a natural call. A floor that stays
   all-natural (today's `instinct()`, by design) can reach game but
   *systematically stalls below slam* off-book. This is not hypothetical — §3's
   probe shows EPBot's floor bidding **`4NT = Blackwood`** on the depth-8 auction
   at 26 HCP, a keycard ask pons's floor cannot make. The standing objection ("a
   convention is only safe if both partners read it") is already answered by the
   floor's own coherence rule — *instinct decodes instinct* — so a self-consistent
   floor reads its own RKCB/cue-bids identically on both sides; the same argument
   that licenses natural-only calls today licenses these. (The one genuine wrinkle
   is a floor call interpreted by an *on-book* partner — that floor-meets-book seam
   is worth handling, not a reason to forbid the convention.) So the convention-free
   stance is a real limitation to lift, not a virtue to keep.

One **open design choice** — measure, don't assert: strict precedence (a separate
floor, book always wins) vs BBA's single weighted table (generic and specific
rules graded by weight). **Runtime is not the deciding factor** — matching a rule
is negligible beside the double-dummy solver — so decide on *decision quality*,
taking the time to get it right. Soft weights degrade more smoothly (a good
generic rule beats a worse one yet still yields to a real node) at the price of
keeping one global weight scale coherent; pons's strict precedence is simpler to
reason about but binary (book hit → book; miss → instinct, nothing in between).

---

## 5. What "calculated bid" computes — the *bilans* engine

*(Added 2026-07-20. §3 established that the floor is programmatic; this section
is the mechanism.)*

The fallback has a name in the binary: **`bilans`** — Polish for *balance*, the
author's term for a running evaluation of the deal. The method that produces a
floor call is **`odzywka_z_bilansu`**, "bid from the balance", and
`STR_CALCULATED_BID` is the label it hands to `epbot_get_info_meaning`.

**It is not a rule table.** It is a four-stage pipeline: reconstruct the deal,
evaluate it, count tricks, price the contract.

### Stage 1 — reconstruct all four hands from the auction

Per seat, not as point ranges but as *card placement*:

```
calculated_player  calculated_partner  calculated_LHO  calculated_RHO
calculated_honors  arr_used_honor  excluded_honors  EXCLUDED_FIGURES  hidden_honors_B
estimate_dlugosci_partnera         estimate_dlugosci_przeciwnikow
analizuj_potencjalne_dlugosci      determine_remaining_longers
determine_potencjalne_HCP_and_PTS  determine_HCP_przeciwnikow
dodaj_partnerowi_figure            dodaj_przeciwnikowi_figure
mozliwe_dodanie_partnerowi_figury  mozliwe_dodanie_przeciwnikowi_figury
kolejnosc_kolorow_przydzialu_figur (suit order for honour allocation)
TYP_ESTIMATED_PARTNER              TYP_ESTIMATED_OPPONENT
```

It *deals out the other 39 cards*, assigning individual honours to specific
seats, tracking which are already spent and which the auction excludes. There is
even a named heuristic for the awkward case:
`prawdopodobna_figura_partnera_przy_braku_wlasnej_figury_i_siedmiu_kartach`
("partner's probable honour when we hold none and seven cards").

### Stage 2 — evaluate that model

A far richer point count than HCP + shape, with the trump suit distinguished
from side suits and corrections keyed to the *kind of call*:

```
get_pkt_dlugie_atu             length points, trumps
get_pkt_dlugie_bocznego_koloru length points, side suit
get_punkty_przebitkowe         ruffing points
get_pkt_uproszczone_atu/_przebitek   simplified variants
determine_punkty_dodatkowe_NT  extra points, notrump
determine_korekta_HCP          HCP correction
get_korekta_HCP_takeout_double / get_preemtive_korekta_HCP
get_podwyzszenie_HCP_reopening / determine_niewykorzystane_HCP  (unused HCP)
determine_sila_koloru_suma_sily_i_honorow   suit strength = strength + honours
determine_punkty_bilansowe     the balance points themselves
set_honor_table  set_sila_longera_table     (tunable tables)
```

### Stage 3 — count winners and losers, for *both* sides

```
determine_losing_winning_tricks     determine_additional_losing_tricks
components_of_winning_tricks        components_of_losing_tricks
components_of_winning_opponent_tricks  components_of_losing_opponent_tricks
just_13_winning_losing_tricks  direct_tricks  tricks_sum  add_additional_losing
Declarers_tricks  Defenders_Tricks
```

Analytic, on the Stage-1 model. Symmetric: it counts what *they* take too, which
is what lets one pipeline handle both constructive and competitive decisions.

### Stage 4 — pick the call by expected score

```
get_probable_level  get_probable_levels  more_probable_level  less_probable_level
expected_level  expected_score  probable_score  average_score  max_score
expected_double  probable_kontra          (kontra = double)
zalozenia_to_vulnerable  get_max_wysokosc_gry  korekta_kontraktu
potencjalny_zapis_z_naszej_gry           (potential score from our contract)
C_IMP_SCORE  C_MP_SCORE  C_PERCENTAGE50  C_PERCENTAGE70  C_PERCENTAGE90
```

Vulnerability-aware, scoring-form-aware (IMPs vs matchpoints change the answer),
and it prices *being doubled*. The `C_PERCENTAGE50/70/90` constants are the
familiar contract-success thresholds.

### Where it sits in the dispatch

In metadata (file) order, `odzywka_z_bilansu` is declared **among the per-seat
rule dispatchers** — `odzywka1_otwierajacego` (opener), `odzywka1_odpowiadajacego`
(responder), `odzywka1_pierwszego_broniacego` (first defender) — with
`aktualizuj_bilans` ("update the balance") next to it. The balance is maintained
*continuously through the auction*, not spun up on a miss. There is also
`determine_odzywki_zabronione_z_bilansu` — "bids forbidden by the balance" — so
the balance can **veto** a rule-table call, not merely substitute for one.

That reframes §4's contrast table. BBA's floor is not a lower-priority rule
layer; it is a second, always-running evaluator that the rule table sits on top
of.

### Why this is better than `instinct()`

Different kind of object, not a better-tuned one:

| | pons `instinct()` | BBA `bilans` |
|---|---|---|
| shape | feature → bid ladder, ~91 weighted rules, one hop | model → tricks → score, four stages |
| other hands | not modelled | reconstructed per seat, down to individual honours |
| tricks | not counted | winners + losers, both sides |
| level chosen by | rule weight | expected score, vulnerability- and IMP/MP-aware |
| depth behaviour | coverage thins | no coverage notion — it always has an answer |

This is why §3's probe saw P → P → 4♥ → 4NT escalate monotonically with HCP on a
depth-8 auction: that is Stage 4 solving for a level, not a rule table degrading.
And it is why pons's floor stalls below slam off-book while BBA's does not.

Note it is **analytic** — self-contained arithmetic over points, lengths and
winner/loser counts. It is not the double-dummy solver: `bcalconsole` is driven
over pipes (`C_PIPE_DD`, `C_PIPE_PLAY`, errors `201`–`203` about *leads*) for
**card play**, gated by `Playing_Skills`/`Defensive_Skills`. That is the whole
≈190× throughput gap over our sample-and-solve approach.

### Evidence grading

| claim | evidence |
|---|---|
| `calculated bid` is the label of `odzywka_z_bilansu` | **strong** — UTF-16 literal at `.rodata` `0x313c2b`, length-prefixed 14 chars, sitting in the alphabetised `info_meaning` label pool (`artificial`, `forcing`, `penalty`, …) |
| all the method/field names above exist | **strong** — NativeAOT keeps ~4700 reflection names in `libEPBot.so`; `EPBot64.dll` is the same engine as an unobfuscated managed assembly |
| the four stages compose in that order | **inferred** — from names, declaration clustering and the §3 probe. No IL was read |
| the balance is per-seat and continuous | **strong** — four `calculated_*` seats, `aktualizuj_bilans` beside the dispatchers |
| the balance can veto rule-table bids | **moderate** — `determine_odzywki_zabronione_z_bilansu` by name only |
| the bidding path does not call the DD solver | **moderate** — solver strings are all play/lead errors; no link found from the balance cluster |

**To get the actual arithmetic, decompile `vendor/bba/EPBot64.dll` with ILSpy.**
It is a plain managed assembly, not obfuscated; the `.so` is a NativeAOT build of
it (`.comment`: `ilc 10.0.5-servicing.26153.111`). No decompiler on this box yet.

### Correction to §0

§0's conclusion stands and gets stronger. `MB.TXT`, `EVAL.DAT` and `Comments.txt`
are all dated **May 2009** and belong to the legacy `bridge.exe`; grepping ASCII
in all five binaries and UTF-16 in all four PEs for `EVAL.DAT` / `MB.TXT` /
`bbsa` returns **zero hits anywhere**. They are orphaned, not inputs. The
convention names in `*.bbsa` *do* appear verbatim in the `.so`, so those are live
config keys.

`EVAL.DAT` is 29,236 × float32 laid out as pairs — even slots ≈5.8–6.2, odd slots
≈7.7–11.8, `-1.0` × 19,454 as an empty sentinel, max 11.835. Consistent with
(something, expected-tricks), but nothing indexes it. Dead end unless the 2009
`bridge.exe` is worth reversing, which it is not.

One thing §2 got right that matters more now: MB.TXT's rule table is the *2009*
artifact. Our static statistics describe how the authors documented a rule
layer — they say nothing about the balance engine, which is where the strength
actually lives.

---

## 6. The bilans surface, bound and measured  *(session 7A — DONE)*

`libEPBot.so` exports **70** `epbot_*` symbols. [`examples/common/oracle.rs`](../../examples/common/oracle.rs)
now binds **22**, and [`examples/probe-bba-bilans`](../../examples/probe-bba-bilans/main.rs)
dumps them beside our own call, one jsonl row per decision. The oracle is no
longer a one-bit comparator.

### The ABI

Signatures are **confirmed twice over** — never guessed. The managed `EPBotFFI`
shim that NativeAOT compiles into the `.so` ships alongside it as
`vendor/bba/Native-libraries/wasm/EPBotWasm.dll`, and its ECMA-335 metadata
carries full signatures *with parameter names* (parse it with python `dnfile`).
BEN's independent ctypes binding at `src/bba/BBA.py` agrees byte-for-byte.

```
int GetProbableLevels (IntPtr bot, IntPtr buf, int bufBytes, IntPtr countOut)
int Get/SetScoring    (IntPtr bot [, int value])
int GetInfoAlerting   (IntPtr bot, int position)
int GetInfo<X>        (IntPtr bot, int position, IntPtr buf, int bufBytes, IntPtr countOut)
int SetInfo<X>        (IntPtr bot, int position, IntPtr data, int count)
   // <X> ∈ feature, min_length, max_length, probable_length,
   //       strength, stoppers, honors, suit_power
```

Three traps, each of which produces plausible garbage rather than an error:

- The getter's size is in **bytes**; the setter's trailing `count` is in
  **elements**. Verified by round trip, not by reading.
- Status returns are `0` OK, `-1` null handle, `-2` exception, `-3` buffer too
  small. `-1`/`-2` are *also* legitimate `probable_levels` values, so never
  treat a negative there as an error.
- `get_bid` is what drives the `calculated bid` path. Read it **before** the
  level and info blocks or you get stale state.

The per-strain scalar `epbot_get_probable_level(bot, strain)` is deliberately
**not** bound: the array returns the same numbers in one call instead of five,
and it is what revealed the array is 9 entries long rather than the 5 we assumed.

### What the block actually contains

Measured over a 21039-row / 2000-board recon dump, not inferred:

| finding | status |
|---|---|
| **`seats[4 + auction.len() % 4]` is the actor's own hand, exactly** | 21039/21039 |
| positions 0..4 are the public band — never exact | 21039/21039 |
| offsets 1, 2, 3 from the actor's slot are LHO / partner / RHO | HCP bands 17.9 / 15.4 / 12.4 wide |
| `strength[suit]` = that suit's HCP | exact |
| `honors[suit]` = bitmask A 16, K 8, Q 4, J 2, T 1 | exact |
| `probable_length` is effectively **unused** | nonzero on <1% of rows |
| `get_scoring` default | `1`, and setting 0..5 changed no call or level |

BBA's own prose ("positions 0-3 public, 4-7 calculated probable hands") is right
but undersells it: slot `4 + actor` is not a *guess*, it is the actor's real
cards, and RHO is the best-known of the other three because its call is the
freshest evidence. That makes positions 4..8 directly comparable to our own
`Inferences` — the same shape of object `probe-ben-info` builds for BEN.

### `probable_levels` — partly decoded, and a trap

9 entries. Entries 0..5 are the strains ♣ ♦ ♥ ♠ NT, entry 5 is live but
undecoded (0 until partner has bid, then rising with auction depth), entries
6..9 are **always 0**.

The values are **not** a 1–7 contract level. They span −6..7, sit negative early
(a flat 16 opening reads `[-1, -2, -1, -2, -1]`) and climb as the auction firms
up (`[5, 5, 6, 7, 7]` in a slam auction) — an offset from some baseline rather
than a bid. Pinning that scale is the first job of session C.

The strain *ordering* is confirmed: `argmax` over entries 0..4 picks the actor's
own longest suit on **51.3%** of rows against a ~27% chance rate.

> **Trap, and it looks like a real finding.** Correlating `argmax` against the
> final contract strain gives **0.9–4.7%**, far *below* the 20% chance rate.
> That is not evidence the ordering is reversed — it is a selection effect. The
> opponents declare about half the time, and in those auctions the actor's best
> strain is systematically *not* the contract strain. Split by declaring side
> before drawing any conclusion from these numbers.

### Still unbound

`epbot_get_sd_tricks` (its `partnerLongerPtr` is inferred, and it returns two
parallel arrays — a distribution, not a scalar), `epbot_get_opponent_type` (its
`systemNumber` argument is ambiguous), and `_meaning[_extended]` (prose, not
graded signal — §3 already used them).

### Operational note

**EPBot cannot be driven from a `cargo test` thread.** Its NativeAOT runtime
segfaults there, and the *pre-existing* 7-symbol `classify` path does too — this
is why `oracle.rs` has only ever tested pure parsing. The ABI self-check lives
on the main thread instead:

```sh
cargo run --features serde --example probe-bba-bilans -- --self-check
```

It asserts the read path, the `set_info` round trip, the 9-entry width, and the
actor-slot indexing invariant. Run it after any change to the binding.

---

## 7. Focused sessions this opens

Independent; each is its own session with its own entry point.

| # | Area | First chunk | Entry point |
|---|---|---|---|
| ~~A~~ | ~~**Widen the oracle FFI**~~ | **DONE** — see §6. 22 symbols bound including the whole `set_info_*` inject half; `probe`/`probe_with` on `BbaOracle`; recon dump in `examples/probe-bba-bilans`. | — |
| B | **Read the arithmetic** | ILSpy on `vendor/bba/EPBot64.dll`; decompile `odzywka_z_bilansu`, `determine_punkty_bilansowe`, `determine_losing_winning_tricks`. Turns §5's *inferred* rows into *strong*. | needs a decompiler installed |
| ~~C~~ | ~~**A trick model for our floor**~~ | **DONE, differently** — see [`evaluator-net.md`](evaluator-net.md). Stages 2–3 are a small *learned* net rather than authored winners/losers arithmetic: `(own hand, hidden-seat ranges) → DD trick mean and spread`, two heads per target (`μ`, `ln σ`) fit on pre-solved deals by Gaussian negative log-likelihood. It never sees the auction, so it is bidding-system agnostic; and it is scored against sampled double-dummy truth, not against `probable_level` (whose scale stays undecoded — the graded teacher turned out to be unnecessary once DD was the target). | `src/bidding/evaluator.rs` |
| D | **Expected-score level choice** | **Constructive rung DONE** (2026-07-21): the floor's game/slam boundary gates (combined-25/fit-sum-31 games, 33/37 slams, the RKCB entry, the asker's grand) price `P(make)` via `Gaussian::p_at_least` against the IMP break-evens (our failing branch priced *doubled* — a gate prices a call, and bid = PD is the scoring split), vulnerability-aware, behind `set_bilans_floor` (default off; A/B owed — `examples/ab-bilans-floor`, both vulnerabilities). Still open in-row: the full `Σ P(T=k)·score(k)` integration, doubled contracts, and a probabilistic par over all 20 declarer columns for the *competitive* decisions (settle rail, sacrifices) — the natural follow-up session. | `src/bidding/instinct.rs` |
| E | **Relational constraints (`SuitRef`)** | Symbolic auction-bound suit/level refs so `rubens_*` stops being opaque `pred` and keeps `describe`/`project`. Authoring economy + inference recovery, *not* strength. | `src/bidding/constraint.rs` |

Dependency: **A and C are done, and D's constructive rung shipped** — the
break-even probability table is the closed form of the expected-score integral
for undisturbed our-side decisions, so those gates needed none of the heavier
Stage-4 machinery. The live remainder of D is the **competitive extension**: a
probabilistic par from the net's opponent-declarer columns, doubled-contract
pricing, and the settle rail — where the `Σ P(T=k)·score(k)` integration and
disaster tails genuinely matter. Note what C did *not* need: decoding the
`probable_levels` scale, or BBA as a teacher at all. Double-dummy truth on the
actual deal was a better target than a graded opinion, and it comes free with the
`.pdd` stock. **B is optional** and now only matters if D's remaining arithmetic
wants BBA's exact score model. **E is independent** and can run in parallel; it
is the smallest.

Standing caveat from [`../bba-gap-campaign.md`](../bba-gap-campaign.md): deep
auctions are our *smallest* gap family (−6k, vs round-1 −213k). C and D are
justified by the floor's systematic under-bidding — slams stranded, level
selection blind to vulnerability — not by auction depth. And nothing here ships
without the A/B in [`../measurement.md`](../measurement.md).

---

## Artifacts

- [`examples/probe-bba-bilans`](../../examples/probe-bba-bilans/main.rs) — §6's
  recon dump and the ABI self-check (**kept**, unlike the throwaways below).
- The signature parse used python `dnfile` over
  `vendor/bba/Native-libraries/wasm/EPBotWasm.dll`'s ECMA-335 metadata, and
  cross-checked BEN's ctypes binding at `src/bba/BBA.py`. No decompiler needed.
- `scripts/bba_floor_stats.py` — MB.TXT static classifier (throwaway).
- `bba-floor-probe` — live-engine off-book probe (throwaway, since removed).
- Inputs (read-only): `vendor/bba/MB.TXT`, `vendor/bba/Comments.txt`,
  `vendor/bba/Native-libraries/linux/x64/libEPBot.so`.
- §5 used no new code, only shell (§6 is now the bound surface in `oracle.rs`):
  - `strings -n 5 vendor/bba/EPBotNET.dll` — managed metadata, the readable twin.
  - `nm -D --defined-only …/libEPBot.so | grep ' T '` — the 70 `epbot_*` exports.
  - NativeAOT reflection names in the `.so` are length-prefixed UTF-8 with the
    prefix byte at `len * 2`; managed string literals are UTF-16, so plain
    `strings` misses them (this is why `calculated bid` needed a byte scan).
- FFI/ABI reused from `examples/bba-match` + the removed `bba-wj-reference`
  spike (git `7d82918`).
