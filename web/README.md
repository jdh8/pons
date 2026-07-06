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
- **Settings** — toggle bidding conventions (like a calculator's basic vs.
  scientific mode): a dozen common on/off treatments, with "More…" revealing
  the full boolean system grouped by area.  Each toggle flips a thread-local
  `set_*` knob read when the **next** deal rebuilds `american()`, so changes
  apply from the next Practice/Demo board on.

Suit colors are CSS variables in `style.css` (`--club`, `--diamond`, …) —
diamonds are orange on purpose ("red suit" is a bidding-theory term), and the
blue clubs are one line to retune.

## Deploy

`pkg/`, `index.html`, `app.js`, and `style.css` are all static — push them to
GitHub Pages (`.github/workflows/pages.yml` does exactly this) or any static
host.

## Test

The wasm surface is native-testable without a browser:

```console
cargo test
```
