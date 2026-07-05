# pons-web

The human-facing [pons](..) examples in the browser: practice bidding one seat
against the 2/1 bots, watch them bid a random board, or browse the authored
books.  Everything runs client-side as WebAssembly; there is no server — and
no double-dummy solver (that is native C++, and actual-layout verdicts are
hindsight anyway).

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

Three tabs:

- **Practice** — pick your seat, dealer, vulnerability, and a minimum HCP,
  then bid with the bidding box; the bots bid the other seats.  After each of
  your calls you see the bot's top-3 picks with probabilities; after the
  auction, all four hands and the final contract.
- **Demo** — deal a random board and watch `american()` bid all four seats.
- **Book** — the authored 2/1 books (constructive/competitive/defensive),
  every node's rules with weights and the constraints' own English
  descriptions, filterable.

Suit colors are CSS variables in `style.css` (`--club`, `--diamond`, …) —
diamonds are orange on purpose ("red suit" is a bidding-theory term), and the
dark-blue clubs are one line to retune.

## Deploy

`pkg/`, `index.html`, `app.js`, and `style.css` are all static — push them to
GitHub Pages (`.github/workflows/pages.yml` does exactly this) or any static
host.

## Test

The wasm surface is native-testable without a browser:

```console
cargo test
```
