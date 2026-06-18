# Rubensohl after 1m — BBA-vs-BBA A/B (ledger toggle 105)

Research + verification for `21GF.bbsa` toggle **105 "Rubensohl after 1m"**
(deferred authoring item in [`21gf-ledger.md`](21gf-ledger.md)). No pons bidder
change — this measures **BBA against itself** with the one toggle flipped.

## What the convention is

Rubensohl = **Rubens transfers + Lebensohl**, in a contested auction. After our
**1m opening is overcalled at the 2 level**, responder's bids **from 2NT upward
become transfers to the next strain** (below 2NT stays natural). Transfer-then-pass
is weak, transfer-then-continue is invitational+; the point is to show a suit at
once (not be jammed out) and right-side the contract. It does **not** trigger over
a 1-level overcall in BBA — matching the literature's "overcall at the two level".

Verified directly from the engine (`examples/bba-conv-probe`):

| auction | hand | toggle ON | toggle OFF |
|---|---|---|---|
| `1♦-(2♥)-?` | 6-card club one-suiter | **2NT** (transfer→♣) | 3♥ |
| `1♣-(2♥)-?` | 6 diamonds | **3♣** (transfer→♦) | 3♥ |
| `1♦-(1♠)-?` | (1-level overcall) | unchanged | unchanged |

## How we flipped one toggle

EPBot's `libEPBot.so` exposes a **per-seat named-convention API** (signatures
recovered by `objdump`, validated against `21GF.bbsa`):

```c
epbot_convention_index(bot, char* name) -> int          // name → index; -1 bad bot
epbot_get_conventions(bot, int seat, char* name) -> int // 0/1; -1 bad bot
epbot_set_conventions(bot, int seat, char* name, int on) -> int
```

Addressing is **seat + name** (mirroring `set_system_type(bot, seat, system)`),
**not** the convention index — passing the index as the int arg returns −2. Apply
overrides **after** `set_system` (which loads the system defaults). Ground-truth
check: 240/258 boolean toggles round-trip vs `21GF.bbsa`, and `get_bid` genuinely
consults the per-seat flag (control: flipping `Texas` gives 1NT-(P)-4♥ vs 2♥).

### Caveat — the FFI default ≠ the file

The FFI's system-0 ("2/1 Game Force") is **not identical to `21GF.bbsa`**: 18
toggles differ. Rubensohl-after-1m, Cappelletti, Bergen, Maximal Doubles and
Two-Way NMF are ON in the file but **OFF in the engine default**; Multi-Landy and
Checkback are ON in the engine but OFF in the file. So every `bba-match` "BBA 2/1"
number is measured against the **engine default**, not the file exactly.

## The A/B result

`bba-match --our-system 0 --our-conv "Rubensohl after 1m=1" --their-conv
"Rubensohl after 1m=0"` (both sides BBA 2/1, the toggle forced ON for our pair and
OFF for theirs; same deal at both tables, divergent contracts solved double-dummy):

| Vul | Boards | Divergent | IMPs/board (95% CI) |
|---|---|---|---|
| none | 20,000 | 112 (0.56%) | **+0.007** [+0.001, +0.014] |
| both | 20,000 |  99 (0.50%) | **+0.002** [−0.005, +0.009] |

**DD-neutral.** A hair positive non-vul (CI just clears 0), indistinguishable from
zero both-vul. Individual boards swing hard (a 2NT-transfer reaching 6♣ for +10; a
7♠ overreach for −14), but the net is ~nothing.

This fits the DD-blindness theme: a double-dummy solver sees through the
right-siding / concealment that transfers buy (see
`project_preemption-dd-negative`), so transfer conventions tend to measure ~flat
here even when they help at the table. **Steer: authoring toggle 105 has low
expected payoff on the DD/perfect-defense harness** — its real value, if any,
would need a single-dummy measure to show.

## Reproduce

```sh
# single-toggle BBA-vs-BBA A/B (our side ON, their side OFF), DD-scored
cargo run --release --example bba-match -- --count 20000 --our-system 0 \
  --our-conv "Rubensohl after 1m=1" --their-conv "Rubensohl after 1m=0"

# any named convention works; bba-conv-probe shows the ABI ground-truth
# (240/258 round-trip vs 21GF.bbsa) plus the on/off bid divergence
cargo run --example bba-conv-probe
```

The convention-override lever lives in `bba-match` itself: `--our-conv` /
`--their-conv` take `NAME=0|1` (repeatable), so any named toggle can be flipped on
either side and IMP'd. `bba-conv-probe` stays as the ABI reference.
