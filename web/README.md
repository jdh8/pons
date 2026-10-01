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

Eight tabs, grouped Play / System / Deal tools / Probability:

- **Practice** — pick your seat, dealer, vulnerability, and a minimum HCP,
  then bid with the bidding box; the bots bid the other seats.  After each of
  your calls you see the bot's top-3 picks with probabilities; after the
  auction, all four hands, the final contract, the oracle's verdict over 100
  opponent reshuffles, and the full double-dummy table.
- **Demo** — deal a random board and watch `american()` bid all four seats,
  then see the double-dummy table and the contract's actual-layout verdict.
- **Book** — the authored 2/1 books (constructive/competitive/defensive),
  every node's rules with weights and the constraints' own English
  descriptions, filterable.  An NS/EW selector chooses which partnership's
  current book is shown.
- **Edit** — a PBN field two-way-synced with a card palette; build a deal by
  hand, then "Bid it out in Demo".
- **Odds** (`#odds`) — a shape-points probability calculator: a union of boxes
  (four suit cells plus a points range, one row per box), priced exactly by
  convolving per-suit holding censuses over the 560 shapes.  The points
  column's header picks its gauge: HCP, UP (upgraded points, HCP + `upgrade` —
  the bidder's scale), and support points with each trump — the censuses come
  from the wasm's `point_census()`, built with the crate's own evaluators, so
  the gauges cannot drift from the bidder's.
- **Partner** (`#partner`) — partner's hand given yours, exactly, after Pavlicek's
  [Companion Hand Calculator](https://www.rpbridge.net/cgi-bin/xch1.pl) — but
  each hand is **a union of boxes** (`15-17` with `freak < 3`: a 1NT opener).
  Partner's *known* boxes condition, the *query* boxes ask; each of the three
  tables picks its own points gauge.  Pure JS: per suit the two hands draw
  disjoint holdings (the honors A–T by identity, the eight spots by count),
  giving a joint census of (your HCP, partner's HCP) per length pair; four
  suits convolve into a 2-D table per shape pair, whose state also carries
  which boxes every suit so far admits.  The other gauges ride beside a hand's
  HCP as small sums — the wasted suits behind `up`, the short side suits'
  extra behind each `sp` — widening that hand's axis.  Dimensions no box reads
  are collapsed, so a named hand or one shape answers in milliseconds.  Suits
  no box tells apart permute freely, so one shape pair is counted for its
  whole orbit — 24 to one when the boxes are points only (`16+` opposite
  `0-7`: half a second).  A wide union of your shapes whose boxes name every
  suit is seconds, and several times that for each further gauge read — hence
  a Compute button.

  Both tabs speak one **box grammar**:
  - A suit cell is a length range (`5+`, `4-6`, `3`, blank = any) or a
    **holding**: a regex over the suit written high to low and matched whole —
    `A K Q J T` by name, `x` a card 2–9, `.` any card — so `AKxx`, `AKx*`,
    `AK...+` (the AK in a 5+ suit), `.*K.*` (the king anywhere), `[^A]*` (no
    ace), `[AKQ]{2}.{3,}` (two of the top three, 5+).  A digit is only ever a
    count: `Q4` is malformed.  Pasting a whole hand (`AKT52.K83.94.762`) into a
    suit cell fills the row.
  - The **where** field is a condition on the box's own hand: the lengths
    `s h d c` (or `♠ ♥ ♦ ♣`), a suit's own HCP `s.hcp` (`d.hcp = 4`: the ♦A),
    the gauges `hcp`, `up`, `sps` `sph` `spd` `spc` (or `sp♠` …; support
    points by trump), `p` for the gauge the points column is on, and `freak`,
    Pavlicek's freakness (`freak < 3` = 4333/4432/5332).  Integers, `+ −`,
    `max()`, `min()`, chainable comparisons (`s >= h >= d`), then `!`, `&`, `|`
    (or `not`/`and`/`or`) and a comma, an *and* that binds loosest — e.g.
    `s > h`, `♠ + ♠.hcp >= 9`, `hcp >= 10 & up >= 12`, `p + max(s, h) >= 20`.
    Typed like Rust: a number is never a condition.
  - In Partner, a partner box also reads your hand under `my.` — `my.s`,
    `my.hcp`, `my.up`, `my.s.hcp`, `my.freak`, and `my.p` for your table's
    gauge (`s + my.s >= 8`: a spade fit; `up + my.up >= 25`).
- **Settings** — toggle bidding conventions, grouped by area.  The whole tab is
  generated from the Rust registry (`describe_options()` in `src/lib.rs`), so a
  convention added there appears here automatically; mutually-exclusive families
  (e.g. defense to their 1NT) render as radio buttons backed by one engine enum.
  NS and EW have separate profiles; each control edits the selected partnership
  and changes apply when the next Practice/Demo board rebuilds both systems.
  The two profiles are also disclosed to each other, so each side reads the
  opponent's artificial calls from the opponent's actual book.
  See [Settings coverage](#settings-coverage) for what is not exposed yet.

Suit colors are CSS variables in `style.css` (`--club`, `--diamond`, …) —
diamonds are orange on purpose ("red suit" is a bidding-theory term), and the
blue clubs are one line to retune.

## Settings coverage

The Settings tab is the `SETTINGS` registry in `src/lib.rs` — one row per knob,
which `describe_options()` serialises for the JS renderer.  Adding a knob to the
UI is **one registry row** (see the top of that file); adding an engine `set_*`
alone does *not* surface it.

Only overrides are stored in `localStorage`, separately for NS and EW.  A
pre-split global value is copied to both profiles on first load, preserving the
old symmetric behavior.  Opponent disclosures such as Woolsey's Landy `2♣`
and Multi `2♦` are derived from the other profile rather than user-editable.

The registry is **curated by measurement**: it offers every convention that A/B's
as a win or a wash, and hides options that measure *worse* — the engine keeps
those as opt-in re-measure knobs, but a player is never offered a setting that
loses.  So an absent knob is usually a deliberate omission, not a gap.
`NotrumpDefense` (`src/bidding/american/defense.rs`) is the worked example of a
radio family: one enum field and a `Setting::Choice` whose `set` maps the
registry `value` string onto a variant.

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
  (`Optional` is only a wash), `DecisionProfile::fifths_companion` (an internal evaluator
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
