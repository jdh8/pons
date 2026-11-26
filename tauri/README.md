# pons-tauri

A desktop client for [pons](..), built on [Tauri](https://tauri.app).  The
window shows the browser client's own page ([`web/`](../web): `index.html`,
`app.js`, `style.css`, embedded at build time), so every tab is here and there
is one UI to maintain.  Only the engine differs: `app.js` finds
`window.__TAURI__` and calls the commands in `src/main.rs` instead of loading
the wasm.

What native buys is threads:

- **Parallel double dummy.**  `pons-dds` fans out over its own thread pool, a
  (deal, strain) pair per task.  The five strains of the double-dummy table
  solve at once, the practice oracle prices 1000 opponent reshuffles a board
  against the browser's 100, and the Evaluate tab's shuffles run in batches of
  100.
- **An engine thread.**  The table, the bidders and the solver calls live off
  the UI thread, so the window never stalls on a deal, a book build or a solve.
  One thread, because `pons-web` keeps the settings in a `thread_local`; the
  page sends its calls one at a time, in order.

Settings are the page's: overrides live in the webview's `localStorage`, apart
from any browser's.

## Build and run

```console
cargo run --release
```

or `cargo desktop` from the repository root.

No Tauri CLI and no wasm build are needed: `cargo` embeds the five files
`tauri.conf.json` lists.  A change to `web/` shows after the next build.

On Linux Tauri links WebKitGTK 4.1.  On Fedora:

```console
sudo dnf install webkit2gtk4.1-devel gtk3-devel libsoup3-devel
```

## Adding an engine call

Three places, all spelled alike: the `pons-web` function, a `#[tauri::command]`
in `src/main.rs` (and its name in `generate_handler!`), and one line in each
half of `loadEngine()` in `web/app.js`.  A Rust parameter `min_hcp` is the key
`minHcp` in JS.

## Test

```console
cargo test
```

The test runs the commands in sequence and proves they share the one engine
thread.  The page itself is not exercised; see the browser client.
