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

```
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

`examples/bba-toggle-ab` (a copy of `bba-match` + a convention override applied to
all four seats; our pair = toggle ON, theirs = OFF; same deal at both tables,
divergent contracts solved double-dummy):

| Vul | Boards | Divergent | IMPs/board (95% CI) |
|---|---|---|---|
| none | 20,000 | 112 (0.56%) | **+0.007** [+0.001, +0.014] |
| both | 20,000 |  99 (0.50%) | **+0.002** [−0.005, +0.009] |

**DD-neutral.** A hair positive non-vul (CI just clears 0), indistinguishable from
zero both-vul. Individual boards swing hard (a 2NT-transfer reaching 6♣ for +10; a
7♠ overreach for −14), but the net is ~nothing.

This matches the ledger's prior result that **Rubensohl-after-1NT lost to plain
Lebensohl** (toggle 80, commit `bfe5e59`), and the DD-blindness theme: a
double-dummy solver sees through the right-siding/concealment that transfers buy
(see `project_preemption-dd-negative`). **Steer: authoring toggle 105 has low
expected payoff on the DD/perfect-defense harness** — defer to a single-dummy
measure if its real value is to show.

## Reproduce

```
BBA_LIB=vendor/bba/Native-libraries/linux/x64/libEPBot.so \
  cargo run --release --example bba-toggle-ab -- --count 20000
# TOGGLE=<other convention name> picks a different toggle to isolate
cargo run --example bba-conv-probe   # ABI ground-truth + on/off bid probe
```

`bba-toggle-ab` is a throwaway copy of `bba-match`; fold the convention-override
into `bba-match` proper (add `epbot_set_conventions` + a `--enable`/`--disable`
flag) once it's wanted as standing infrastructure.
