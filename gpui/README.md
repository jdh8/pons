# pons-gpui

A native desktop client for [pons](..), built on [GPUI](https://gpui.rs).  It
drives the same engine surface as the browser client ([`web/`](../web)) — the
`WebTable` session, the book walk, the Settings registry — called as plain Rust
instead of through WebAssembly.

What native buys is threads:

- **Parallel double dummy.**  `pons-dds` fans out over its own thread pool, a
  (deal, strain) pair per task.  The five strains of the double-dummy table
  solve at once, and the practice oracle prices its opponent reshuffles in
  parallel batches — 1000 per board here, against the browser's 100.
- **An engine thread.**  The table, the bidders and the solver calls live off
  the UI thread, so the window never stalls on a deal, a book build or a solve.

## Build and run

```console
cargo run --release
```

On Linux GPUI links `libxkbcommon`, `libxkbcommon-x11` and `libxcb`, and wants
a Vulkan driver at run time.  On Fedora:

```console
sudo dnf install libxkbcommon-devel libxkbcommon-x11-devel libxcb-devel
```

Install [Inter](https://rsms.me/inter/) (`sudo dnf install rsms-inter-fonts`)
for bold text: GPUI's Linux text system reads one weight per font file, so a
variable-only UI font such as GNOME's Adwaita Sans draws every weight as
regular.  Without Inter the client falls back to whatever GPUI finds.

`Cargo.lock` pins `libc` to 0.2.189: GPUI 0.2.2 reaches `xattr` 0.2 through its
HTTP client, and that reads `libc::ENOATTR`, which 0.2.190 removed on Linux.
Drop the pin when a GPUI release moves off it.

## Tabs

- **Play** — the web client's Practice and Demo in one.  *Seat*, *Dealer*,
  *Vul* and *Min HCP* are strips of choices, the chosen one gold.  *Deal
  practice* seats you against the bots, with the bidding box, the evaluator
  net's trick hint, and the bot's top-3 after each of your calls; *Deal demo*
  lets the bots bid all four seats;
  *Paste PBN* bids out the deal on the clipboard.  After the auction: all four
  hands, the double-dummy table, the Result / Par / IMP verdict, and on a
  practice board the oracle.
- **Book** — the authored books of either partnership.  Type to filter, with
  suits as letters (`1nt 2c`); Esc clears.
- **Settings** — generated from the same registry as the web tab, one profile
  per partnership.  Overrides are saved to
  `$XDG_CONFIG_HOME/pons/settings.json` (`~/.config` by default) and apply from
  the next deal.

Not ported: the web client's Edit, Binky, Odds and Partner tabs.  Each needs a
text field GPUI does not ship, and Odds and Partner are computed in
`web/app.js`, not in Rust.

## Test

```console
cargo test
```

The tests bid a board to the end and parse every JSON shape the client reads,
so a renamed field in `web/src/lib.rs` fails here rather than in the window.
