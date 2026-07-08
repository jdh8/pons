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
alone does *not* surface it.

The registry is **curated by measurement**: it offers every convention that A/B's
as a win or a wash, and hides options that measure *worse* — the engine keeps
those as opt-in re-measure knobs, but a player is never offered a setting that
loses.  So an absent knob is usually a deliberate omission, not a gap.
`NotrumpDefense` (`src/bidding/american/defense.rs`) is the worked example of a
radio family: one `Cell<enum>`, a `set_*(enum)` setter, and a `Setting::Choice`
whose `set` maps the registry `value` string onto a variant.

### Hidden because the option measures worse

Real conventions whose *enabled* setting lost an A/B (plain-DD negative, or a
perfect-defense loss that erases a plain win); still in the engine as opt-in
knobs, just not offered:

- single toggles — `set_long_minor_force` (−7.12 IMPs/fired), `set_free_bids`,
  `set_competition_over_transfer`, `set_diamond_transfer_defense`,
  `set_responsive_overcall`, the gambling / preempt-over-double family,
  `set_notrump_balancing`, `set_weak_two_competition`, `set_minor_min_to_3nt`
- evaluator sweeps the default beats — `set_one_notrump_fifths`, `set_landy_hcp`
- Meckwell defense + satellites (`set_meckwell`, `set_meckwell_x_four_four`,
  `set_meckwell_minor_major_44`) — 0% Nash support, a decisive loss
- the losing *variants* of the enum families below

### Mutually-exclusive families — only the not-worse variants are offered

Each family is one engine enum.  A variant becomes a radio option (or on/off
toggle) only where it measures no worse than the default; where every alternative
lost, the family stays a fixed default with no control:

- **offered** — defense to their 1NT (`notrump_defense`: Natural / DONT /
  Landy-double / Woolsey / Always-pass — the DirectLandy **5-4** form is a
  measured win; the 4-4 form and Meckwell are omitted as losses); our 1NT minor
  responses (`notrump_minors`: Puppet / European); advancer's Lebensohl (on/off
  over `set_advance_sohl_style`)
- **fixed — every alternative lost** — `set_double_style` (`Penalty` −1.59,
  `Takeout` −2.14 IMPs/div), `set_competitive_4333`, `set_negative_double_shape`,
  `set_natural_double_shape` (`Any` −0.70), `set_unusual_2nt` (the `FourFour`
  relay lost to the `Direct` default), `set_takeout_support`, `set_latch_style`
  (`Optional` is only a wash), `set_fifths_companion` (an internal evaluator
  gauge).  Lebensohl's `Plain` middle is likewise omitted — `Transfer` dominates
  it, so the `lebensohl` toggle is Off/Transfer only.

### Not exposed — numeric tuning (needs a range control, and it's dev-path)

The registry is boolean/enum only; floors, point bands, and range specs have no
control type yet and are driven from the A/B examples, not the UI:

- floors — the `set_*_floor` family (natural double, Woolsey / Meckwell / DONT
  `X`, Texas game, six-card accept/invite, free-bid, preempt, UvU, …)
- bands / ranges — `set_woolsey_points`, `set_natural_overcall_points`,
  `set_landy`, `set_unusual_notrump_defense`, `set_natural_double_weight`
- specs — `set_double_override`, `set_penalty_pass`, `set_doubled_landy_escape`,
  `set_stayman_defense_overcall`

These are convention *tuning* dials (the A/B campaign's knobs),
`docs/convention-tuning.md` territory rather than everyday user settings;
`<input type="range">` is the natural control if they ever move to the UI.

## Deploy

`pkg/`, `index.html`, `app.js`, and `style.css` are all static — push them to
GitHub Pages (`.github/workflows/pages.yml` does exactly this) or any static
host.

## Test

The wasm surface is native-testable without a browser:

```console
cargo test
```
