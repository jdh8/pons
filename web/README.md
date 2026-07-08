# pons-web

The human-facing [pons](..) examples in the browser: practice bidding one seat
against the 2/1 bots, watch them bid a random board, or browse the authored
books.  Everything runs client-side as WebAssembly; there is no server.

Double dummy runs in the browser too, via the pure-Rust
[pons-dds](https://github.com/jdh8/pons-dds) (the native `pons/dd` feature
wraps C++ and cannot target wasm) — but only **after** the auction: a full
5×4 table once the hands are revealed, and on practice boards a fairness
**oracle** that judges the reached contract over reshuffles of the two unseen
opposing hands instead of the one true layout (an actual-layout verdict
*during* practice would be hindsight).

## Build

You need the `wasm32-unknown-unknown` target and a `wasm-bindgen` CLI whose
version matches the `wasm-bindgen` crate in `Cargo.lock`:

```console
rustup target add wasm32-unknown-unknown   # once; or distro pkg (below)
cargo install wasm-pack                    # once

wasm-pack build --release --target web    # writes ./pkg/
```

(Or the two raw steps: `cargo build --release --target wasm32-unknown-unknown`
then `wasm-bindgen target/wasm32-unknown-unknown/release/pons_web.wasm
--out-dir pkg --target web` with a `wasm-bindgen-cli` matching `Cargo.lock`.)

Notes:

- `.cargo/config.toml` clears a global `-Ctarget-cpu=native` for the wasm build.
  Left in place, that flag (harmless on native builds, meaningless for wasm)
  corrupts the module's target features and `wasm-bindgen` then fails with
  `failed to find intrinsics to enable "clone_ref"`.
- `getrandom = { features = ["wasm_js"] }` in `Cargo.toml` names getrandom's
  browser backend so the wasm target compiles — we never call it (the RNG is
  seeded from JS), but the crate still has to name a backend.
- Distro-packaged (non-rustup) Rust can't `rustup target add`; install the
  target's std via the package manager instead — on Fedora,
  `sudo dnf install rust-std-static-wasm32-unknown-unknown` — or use a rustup
  toolchain.

## Run

Serve this directory over HTTP — ES modules and wasm won't load from `file://`:

```console
python3 -m http.server 8137
# open http://localhost:8137/
```

Five tabs:

- **Practice** — pick your seat, dealer, vulnerability, and a minimum HCP,
  then bid with the bidding box; the bots bid the other seats.  After each of
  your calls you see the bot's top-3 picks with probabilities; after the
  auction, all four hands, the final contract, the oracle's verdict over 100
  opponent reshuffles, and the full double-dummy table.
- **Demo** — deal a random board and watch `american()` bid all four seats,
  then see the double-dummy table and the contract's actual-layout verdict.
- **Book** — the authored 2/1 books (constructive/competitive/defensive),
  every node's rules with weights and the constraints' own English
  descriptions, filterable.
- **Edit** — a PBN field two-way-synced with a card palette; build a deal by
  hand, then "Bid it out in Demo".
- **Settings** — toggle bidding conventions, grouped by area.  The whole tab is
  generated from the Rust registry (`describe_options()` in `src/lib.rs`), so a
  convention added there appears here automatically; mutually-exclusive families
  (e.g. defense to their 1NT) render as radio buttons backed by one engine enum.
  Each control flips a thread-local `set_*` knob read when the **next** deal
  rebuilds `american()`, so changes apply from the next Practice/Demo board on.
  See [Settings coverage](#settings-coverage) for what is not exposed yet.

Suit colors are CSS variables in `style.css` (`--club`, `--diamond`, …) —
diamonds are orange on purpose ("red suit" is a bidding-theory term), and the
blue clubs are one line to retune.

## Settings coverage

The Settings tab is the `SETTINGS` registry in `src/lib.rs` — one row per knob,
which `describe_options()` serialises for the JS renderer.  Adding a knob to the
UI is **one registry row** (see the top of that file); adding an engine `set_*`
alone does *not* surface it.  What the registry deliberately leaves out today:

### Not exposed — one-line toggles (just add a registry row)

Boolean conventions with a plain `set_*(bool)` the UI simply hasn't surfaced:

- `set_balanced_1nt_rebid`, `set_splinter_doubled`, `set_second_suit_agreement`
- the takeout-shape discipline family — `set_suppress_5332_takeout`,
  `set_suppress_4432_vs_major`, `set_suppress_4432_vs_minor`,
  `set_suppress_flat_4333_takeout`
- opening / overcall discipline — `set_rule_of_20`, `set_overcall_discipline`,
  `set_passed_hand_overcall`
- `set_direct_landy_penalty_pass` (a direct-Landy sub-option)

### Not exposed — mutually-exclusive families (need a `Setting::Choice` + engine enum)

Each already has an engine enum and wants a radio family like `notrump_defense`;
some are three-way where the UI currently reaches only a subset:

- `set_double_style` — `DoubleStyle` (Takeout / Penalty / PenaltyLight / Optional)
- `set_lebensohl_style` / `set_advance_sohl_style` — `LebensohlStyle`
  (Off / Plain / Transfer); the UI's Lebensohl checkbox reaches only Off/Transfer,
  never Plain
- `set_competitive_4333` — `Competitive4333` (Allow / Suppress / SuppressWithStopper)
- `set_negative_double_shape` — `NegativeDoubleShape` (BothMajors / Modern / Cachalot)
- `set_natural_double_shape` — `DoubleShape` (Balanced / SemiBalanced / Any)
- `set_takeout_support` — `TakeoutSupport`
- `set_unusual_2nt` — `Unusual2nt` (FourFour / FiveFiveAdd / Direct)
- `set_latch_style` — `LatchStyle` (Penalty / Optional)
- `set_fifths_companion` — `FifthsCompanion` (Hcp / Bumrap)
- `set_notrump_minors` — our 1NT-3♣ minor structure (`PUPPET` / `EUROPEAN`)

`NotrumpDefense` (`src/bidding/american/defense.rs`) is the worked example of
this shape: one `Cell<enum>`, a `set_*(enum)` setter, and a `Setting::Choice`
whose `set` maps the registry `value` string onto a variant.

### Not exposed — numeric tuning (needs number/range controls, mostly A/B knobs)

The registry is boolean/enum only; floors, point bands, and range specs have no
control type yet and are driven from the A/B examples, not the UI:

- floors — the `set_*_floor` family (natural double, Woolsey / Meckwell / DONT
  `X`, Texas game, six-card accept/invite, free-bid, preempt, UvU, …)
- bands / ranges — `set_woolsey_points`, `set_natural_overcall_points`,
  `set_landy`, `set_unusual_notrump_defense`, `set_natural_double_weight`
- specs — `set_double_override`, `set_penalty_pass`, `set_doubled_landy_escape`,
  `set_stayman_defense_overcall`

These are convention *tuning* dials (the A/B campaign's knobs), `docs/convention-tuning.md`
territory rather than everyday user settings.

## Deploy

`pkg/`, `index.html`, `app.js`, and `style.css` are all static — push them to
GitHub Pages (`.github/workflows/pages.yml` does exactly this) or any static
host.

## Test

The wasm surface is native-testable without a browser:

```console
cargo test
```
