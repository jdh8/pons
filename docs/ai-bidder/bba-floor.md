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

Reproduce (static side): `python3 scripts/bba_floor_stats.py`. The live-engine
side ran a throwaway `bba-floor-probe` example, since removed.

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

`examples/bba-floor-probe` drives EPBot down **deliberately deep, off-book
auctions** (where §2 says no specific node exists) and reads back the engine's
own call *and its self-description* via `epbot_interpret_bid` +
`epbot_get_info_meaning[_extended]`.

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

## Artifacts

- `scripts/bba_floor_stats.py` — MB.TXT static classifier (throwaway).
- `bba-floor-probe` — live-engine off-book probe (throwaway, since removed).
- Inputs (read-only): `vendor/bba/MB.TXT`, `vendor/bba/Comments.txt`,
  `vendor/bba/Native-libraries/linux/x64/libEPBot.so`.
- FFI/ABI reused from `examples/bba-match` + the removed `bba-wj-reference`
  spike (git `7d82918`).
