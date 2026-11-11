//! Native [GPUI](https://gpui.rs) front end for pons
//!
//! The engine surface is `pons-web`'s — the same `WebTable` the browser
//! drives, called as plain Rust and answering in JSON.  What native buys is
//! threads: the engine lives on its own thread (the UI never blocks on a
//! bidder or a solver), and double dummy fans out over `pons-dds`'s pool, so
//! the practice oracle prices a thousand reshuffles where the browser does a
//! hundred.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc;

use gpui::prelude::*;
use gpui::{
    App, Application, Bounds, Context, CursorStyle, Decorations, Div, FocusHandle, Font,
    FontWeight, KeyDownEvent, MouseButton, Pixels, PromptLevel, ResizeEdge, SharedString, Stateful,
    StyledText, TextRun, Window, WindowBounds, WindowDecorations, WindowOptions, div, px, relative,
    rgb, rgba, size,
};
use pons_web::WebTable;
use serde::Deserialize;
use serde_json::Value;

#[cfg(test)]
mod tests;

/// Reshuffles the practice oracle prices per board, and per engine job — a
/// chunk is one parallel batch, small enough that the verdict visibly converges
const ORACLE_TOTAL: u32 = 1000;
const ORACLE_CHUNK: u32 = 100;

/// Book nodes per page
// ponytail: paging, not a virtual list — `gpui::list` if scrolling the whole book matters
const PAGE: usize = 40;

// The web UI's look, copied by hand from `web/style.css` (`:root` and `body`):
// flat felt, card-paper panels, gold for the active choice.  Keep the two in step.
const RESIZE_GRIP: Pixels = px(6.);
const FELT: u32 = 0x2c7a63;
const PAPER: u32 = 0xfdfdf7;
const INK: u32 = 0x222222;
const GOLD: u32 = 0xf5c518;
/// Text straight on the felt
const CHALK: u32 = 0xf2f2f2;
const SPADE: u32 = 0x1a1a1a;
const HEART: u32 = 0xc0392b;
const DIAMOND: u32 = 0xe07b00;
const CLUB: u32 = 0x1e7a45;
/// Outline of a control on the felt, and its hover wash: white at 35% and 12%
const RIM: u32 = 0xffffff59;
const WASH: u32 = 0xffffff1f;
/// Secondary text is the surrounding colour at this opacity, on felt and paper
/// alike — the web's `.hint` / `.count`
const DIM: f32 = 0.65;
// The web's measures, in pixels at its 16px `rem`.  One centred column carries
// every tab, and every row of a tab spans it, so the tabs share their edges.
/// `main`'s `max-width`
const COLUMN: Pixels = px(960.);
/// Between the rows of a tab, and around them
const GAP: Pixels = px(16.);
/// `0.9rem`: controls, counts, the board line
const SMALL: Pixels = px(14.4);
/// Panel titles and table heads
const TINY: Pixels = px(12.);
/// A bidding-box key and the gap between two; three [`WIDE_KEY`]s span five
const KEY: Pixels = px(54.);
const KEY_GAP: Pixels = px(6.);
const WIDE_KEY: Pixels = px(94.);
/// The four suit glyphs of Libertinus Sans, the web's `@font-face`
/// (`assets/SUITS-LICENSE.txt`); every other glyph stays in the system UI font.
/// The embedded file aliases 'm' to the spade, or GPUI's Linux text system
/// rejects the face — see `assets/suits.py`.
const SUIT_FONT: &str = "Libertinus Suits";
/// The face of everything else.  GPUI's Linux text system reads one weight per
/// font file, so GNOME's Adwaita Sans — a single variable file — draws bold as
/// regular; Inter, its upstream, ships a file per weight.  Where Inter is not
/// installed GPUI falls back to the face it would have picked anyway.
const UI_FONT: &str = if cfg!(target_os = "linux") {
    "Inter"
} else {
    ".SystemUIFont"
};

const SEATS: [&str; 4] = ["N", "E", "S", "W"];
const VULS: [&str; 4] = ["none", "ns", "ew", "both"];
const VUL_NAMES: [&str; 4] = ["None", "NS", "EW", "Both"];
/// The floors the *Min HCP* picker offers your hand
// ponytail: eight landmarks, not the web's free 0–37 — GPUI ships no number field
const MIN_HCPS: [u8; 8] = [0, 5, 8, 10, 12, 15, 18, 20];
const PAIRS: [&str; 2] = ["ns", "ew"];
const PAIR_NAMES: [&str; 2] = ["North–South", "East–West"];
const STRAINS: [&str; 5] = ["♣", "♦", "♥", "♠", "NT"];
/// Auction-table column order, west first as in the web client
const COLUMNS: [char; 4] = ['W', 'N', 'E', 'S'];

#[derive(Deserialize)]
struct Hand {
    spades: String,
    hearts: String,
    diamonds: String,
    clubs: String,
    hcp: u8,
}

#[derive(Deserialize)]
struct Feedback {
    human: String,
    agreed: bool,
    authored: bool,
    top: Vec<(String, f32)>,
}

#[derive(Deserialize)]
struct Snapshot {
    mode: String,
    dealer: char,
    vul: String,
    seat: Option<char>,
    hands: BTreeMap<char, Hand>,
    auction: Vec<String>,
    your_turn: bool,
    ended: bool,
    legal: Vec<String>,
    contract: Option<String>,
    feedback: Vec<Feedback>,
}

#[derive(Deserialize)]
struct Hint {
    strain: String,
    mean: f32,
    sd: f32,
}

#[derive(Deserialize)]
struct Dd {
    seats: [char; 4],
    rows: Vec<DdRow>,
    verdict: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct DdRow {
    strain: String,
    tricks: Vec<u8>,
}

#[derive(Deserialize)]
struct Oracle {
    n: u32,
    makes_pct: f64,
    mean_tricks: f64,
    tricks_min: u8,
    tricks_max: u8,
    mean_score: f64,
}

#[derive(Deserialize)]
struct Node {
    book: String,
    auction: String,
    rules: Vec<Rule>,
    note: Option<String>,
    /// What the filter matches: lowercase, suits as letters
    #[serde(skip)]
    haystack: String,
}

#[derive(Deserialize)]
struct Rule {
    call: String,
    weight: i16,
    text: String,
    label: String,
}

#[derive(Deserialize)]
struct Setting {
    kind: String,
    key: String,
    section: String,
    label: String,
    default: Value,
    requires: Option<String>,
    #[serde(default)]
    variants: Vec<Variant>,
}

#[derive(Deserialize)]
struct Variant {
    value: String,
    label: String,
}

/// Per-partnership settings that differ from the registry defaults
type Overrides = [BTreeMap<String, Value>; 2];

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Play,
    Book,
    Settings,
}

type Job = Box<dyn FnOnce(&mut WebTable) + Send>;

struct Pons {
    /// The engine thread's inbox.  One thread owns the table *and* the
    /// settings (`pons-web` keeps them in a `thread_local`), so every engine
    /// call goes through here.
    engine: mpsc::Sender<Job>,
    focus: FocusHandle,
    tab: Tab,
    status: String,

    seat: usize,
    dealer: usize,
    vul: usize,
    min_hcp: u8,
    /// Bumped per deal; a reply from an older board is dropped
    board: u64,
    snap: Option<Snapshot>,
    hint: Vec<Hint>,
    dd: Option<Dd>,
    oracle: Option<Oracle>,

    pair: usize,
    nodes: Vec<Node>,
    filter: String,
    page: usize,

    settings: Vec<Setting>,
    overrides: Overrides,
}

fn config_path() -> Option<PathBuf> {
    let dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from(std::env::var_os("HOME")?).join(".config")))?;
    Some(dir.join("pons").join("settings.json"))
}

/// Suit symbols as letters, lowercase: what a keyboard can type into the filter
fn typeable(text: &str) -> String {
    text.to_lowercase()
        .replace('♣', "c")
        .replace('♦', "d")
        .replace('♥', "h")
        .replace('♠', "s")
}

/// `key` or `key=value`, as the settings registry writes a `requires`
fn satisfies(value: &Value, want: Option<&str>) -> bool {
    match want {
        Some(want) => value.as_str() == Some(want),
        None => value.as_bool() == Some(true),
    }
}

/// Text with its suit symbols in the house colors and the house suit font
///
/// Always on paper: the green club and the near-black spade vanish on the felt.
#[derive(IntoElement)]
struct Suits(SharedString);

fn suits(text: impl Into<SharedString>) -> Suits {
    Suits(text.into())
}

impl RenderOnce for Suits {
    fn render(self, window: &mut Window, _: &mut App) -> impl IntoElement {
        let style = window.text_style();
        let mut runs = Vec::new();
        let mut plain = 0;
        for c in self.0.chars() {
            let color = match c {
                '♠' => SPADE,
                '♥' => HEART,
                '♦' => DIAMOND,
                '♣' => CLUB,
                _ => {
                    plain += c.len_utf8();
                    continue;
                }
            };
            if plain > 0 {
                runs.push(style.to_run(std::mem::take(&mut plain)));
            }
            runs.push(TextRun {
                font: Font {
                    family: SUIT_FONT.into(),
                    ..style.font()
                },
                color: rgb(color).into(),
                ..style.to_run(c.len_utf8())
            });
        }
        if plain > 0 {
            runs.push(style.to_run(plain));
        }
        StyledText::new(self.0).with_runs(runs)
    }
}

/// A quiet control on the felt — the web's `nav button` and `button.secondary`:
/// outlined, gold when it is the active choice
fn button(id: String, label: impl IntoElement, active: bool, enabled: bool) -> Stateful<Div> {
    div()
        .id(SharedString::from(id))
        .px(px(12.))
        .py(px(4.))
        .rounded(px(5.))
        .border_1()
        .text_size(SMALL)
        .border_color(if active { rgb(GOLD) } else { rgba(RIM) })
        .when(active, |this| this.bg(rgb(GOLD)).text_color(rgb(INK)))
        .when(!enabled, |this| this.opacity(0.4))
        .when(enabled, |this| this.cursor_pointer())
        .when(enabled && !active, |this| {
            this.hover(|style| style.bg(rgba(WASH)))
        })
        .child(label)
}

/// The gold action of a tab — the web's `button.primary`
fn primary(id: &'static str, label: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .px(px(16.))
        .py(px(5.))
        .rounded(px(6.))
        .bg(rgb(GOLD))
        .text_color(rgb(INK))
        .font_weight(FontWeight::SEMIBOLD)
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0xffd83a)))
        .child(label)
}

/// One of a few: a named strip of choices, the chosen one gold
///
/// Stands in for the web's `<select>`, which GPUI does not ship.  Every choice
/// is on show, so the strip keeps its width whatever is chosen.
fn picker(
    name: &'static str,
    labels: &[impl ToString],
    chosen: usize,
    cx: &mut Context<Pons>,
    choose: fn(&mut Pons, usize, &mut Context<Pons>),
) -> Div {
    let last = labels.len().saturating_sub(1);
    let choices = labels.iter().enumerate().map(|(index, label)| {
        div()
            .id(SharedString::from(format!("{name}-{index}")))
            .px(px(8.))
            .py(px(3.))
            // The strip's corners do not clip its children; round the ends to fit.
            .when(index == 0, |this| this.rounded_l(px(4.)))
            .when(index == last, |this| this.rounded_r(px(4.)))
            .when(index > 0, |this| this.border_l_1().border_color(rgba(RIM)))
            .when(index == chosen, |this| {
                this.bg(rgb(GOLD)).text_color(rgb(INK))
            })
            .when(index != chosen, |this| {
                this.cursor_pointer()
                    .hover(|style| style.bg(rgba(WASH)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        choose(this, index, cx);
                        cx.notify();
                    }))
            })
            .child(label.to_string())
    });
    let strip = div()
        .flex()
        .rounded(px(5.))
        .border_1()
        .border_color(rgba(RIM))
        .children(choices);
    div()
        .flex()
        .items_center()
        .gap(px(6.))
        .text_size(SMALL)
        .child(name)
        .child(strip)
}

/// A row of controls on the felt — the web's `.controls`
fn controls() -> Div {
    div().flex().flex_wrap().items_center().gap(px(13.))
}

/// A quiet line on the felt under the controls — the web's `.boardinfo`
fn note(text: impl Into<SharedString>) -> Div {
    div().text_size(SMALL).opacity(0.85).child(text.into())
}

/// A bidding-box key: a paper chip, so a suit keeps its colour
fn key(id: String, label: impl IntoElement, enabled: bool) -> Stateful<Div> {
    div()
        .id(SharedString::from(id))
        .py(px(5.))
        .flex()
        .justify_center()
        .rounded(px(6.))
        .bg(rgb(PAPER))
        .text_color(rgb(INK))
        .font_weight(FontWeight::SEMIBOLD)
        .when(!enabled, |this| this.opacity(0.35))
        .when(enabled, |this| {
            this.shadow_sm()
                .cursor_pointer()
                .hover(|style| style.bg(rgb(0xffffff)))
        })
        .child(label)
}

/// One answer on a paper form: a check box, or a radio button when `round`
fn answer(id: String, text: String, on: bool, round: bool, live: bool) -> Stateful<Div> {
    let mark = div()
        .size(px(16.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .border_1()
        .border_color(rgb(if on { CLUB } else { 0x767676 }))
        .map(|this| match (round, on) {
            (true, true) => this
                .rounded_full()
                .child(div().size(px(8.)).rounded_full().bg(rgb(CLUB))),
            (true, false) => this.rounded_full(),
            (false, true) => this
                .rounded(px(3.))
                .bg(rgb(CLUB))
                .text_color(rgb(PAPER))
                .text_size(TINY)
                .line_height(relative(1.))
                .font_weight(FontWeight::BOLD)
                .child("✓"),
            (false, false) => this.rounded(px(3.)),
        });
    div()
        .id(SharedString::from(id))
        .flex()
        .items_center()
        .gap(px(8.))
        .py(px(2.))
        .rounded(px(3.))
        // The web's `.opt.dimmed`: the master knob is off, so the engine ignores this row
        .when(!live, |this| this.opacity(0.45))
        .when(live, |this| {
            this.cursor_pointer()
                .hover(|style| style.bg(rgba(0x0000000f)))
        })
        .child(mark)
        .child(suits(text))
}

/// A registry key as a label — the web client's `humanize`; keep the two in step
fn humanize(key: &str) -> String {
    let words: Vec<String> = key
        .split('_')
        .enumerate()
        .map(|(index, word)| match word {
            "nt" | "xyz" | "rkcb" | "dont" | "hcp" | "gf" | "1nt" | "3nt" | "4m" => {
                word.to_uppercase()
            }
            "uvu" => "UvU".into(),
            "2d" => "2♦".into(),
            _ if index == 0 => {
                let mut chars = word.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_uppercase().chain(chars).collect()
                })
            }
            _ => word.into(),
        })
        .collect();
    words.join(" ")
}

/// A card-paper panel on the felt
fn panel() -> Div {
    div()
        .px(px(14.))
        .py(px(11.))
        .rounded(px(8.))
        .bg(rgb(PAPER))
        .text_color(rgb(INK))
        .shadow_sm()
}

/// What a panel holds, over its content — the web's `.panel-title`
fn title(text: &str) -> Div {
    div()
        .text_size(TINY)
        .opacity(DIM)
        .mb(px(6.))
        .child(text.to_uppercase())
}

fn seat_name(seat: char) -> &'static str {
    match seat {
        'N' => "North",
        'E' => "East",
        'S' => "South",
        _ => "West",
    }
}

/// Seat a hand in `cell`: who and how strong, then the four suit lines
fn seated(cell: Div, seat: char, hand: &Hand) -> Div {
    let head = div()
        .font_weight(FontWeight::BOLD)
        .mb(px(5.))
        .child(format!("{} · {} HCP", seat_name(seat), hand.hcp));
    let lines = [
        ("♠", &hand.spades),
        ("♥", &hand.hearts),
        ("♦", &hand.diamonds),
        ("♣", &hand.clubs),
    ]
    .map(|(symbol, cards)| {
        div()
            .flex()
            .gap(px(8.))
            .text_size(px(16.8))
            .font_weight(FontWeight::SEMIBOLD)
            // A fixed gutter: the four glyphs differ in width, the ranks must not
            .child(
                div()
                    .w(px(16.))
                    .flex()
                    .justify_center()
                    .child(suits(symbol)),
            )
            .child(if cards.is_empty() {
                "—".to_string()
            } else {
                cards.clone()
            })
    });
    cell.flex().flex_col().child(head).children(lines)
}

/// All the hands on show in compass layout: North on top, West and East
/// flanking, South below, each centred in a third of the column
fn compass(hands: &BTreeMap<char, Hand>) -> Div {
    let third = || div().flex_1().flex().justify_center();
    let seat = |seat: char| match hands.get(&seat) {
        Some(hand) => third().child(seated(panel().min_w(px(160.)), seat, hand)),
        None => third(),
    };
    let row = || div().flex().gap(px(10.));
    div()
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(row().child(third()).child(seat('N')).child(third()))
        .child(row().child(seat('W')).child(third()).child(seat('E')))
        .child(row().child(third()).child(seat('S')).child(third()))
}

/// The auction table: four equal columns, west first, blank up to the dealer
fn auction(snap: &Snapshot) -> Div {
    let column = || div().flex_1().flex().justify_center();
    let head = div().flex().children(COLUMNS.map(|seat| {
        let you = snap.seat == Some(seat);
        column()
            .pb(px(4.))
            .border_b_1()
            .text_size(TINY)
            .when(you, |this| {
                this.border_color(rgb(GOLD)).text_color(rgb(0x8a6d00))
            })
            .when(!you, |this| {
                this.border_color(rgba(0x00000026)).opacity(DIM)
            })
            .child(seat_name(seat).to_uppercase())
    }));
    let lead = COLUMNS.iter().position(|&c| c == snap.dealer).unwrap_or(0);
    let mut calls: Vec<&str> = std::iter::repeat_n("", lead)
        .chain(snap.auction.iter().map(String::as_str))
        .collect();
    // A short last line would stretch its cells off the columns
    calls.resize(calls.len().next_multiple_of(4), "");
    let cell = |call: &str| {
        let cell = column().py(px(3.));
        match call {
            "" => cell,
            "P" => cell.opacity(0.45).child("P"),
            "X" | "XX" => cell
                .text_color(rgb(HEART))
                .font_weight(FontWeight::EXTRA_BOLD)
                .child(call.to_string()),
            _ => cell
                .font_weight(FontWeight::SEMIBOLD)
                .child(suits(call.to_string())),
        }
    };
    panel()
        .flex_1()
        .min_w(px(256.))
        .flex()
        .flex_col()
        .child(head)
        .children(
            calls
                .chunks(4)
                .map(|line| div().flex().children(line.iter().map(|call| cell(call)))),
        )
}

impl Pons {
    fn new(cx: &mut Context<Self>) -> Self {
        let (engine, inbox) = mpsc::channel::<Job>();
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos() as u64);
        std::thread::spawn(move || {
            let mut table = WebTable::new(&seed.to_string());
            for job in inbox {
                job(&mut table);
            }
        });

        let overrides: Overrides = config_path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();

        let this = Self {
            engine,
            focus: cx.focus_handle(),
            tab: Tab::Play,
            status: String::new(),
            seat: 2,
            dealer: 0,
            vul: 0,
            min_hcp: 0,
            board: 0,
            snap: None,
            hint: Vec::new(),
            dd: None,
            oracle: None,
            pair: 0,
            nodes: Vec::new(),
            filter: String::new(),
            page: 0,
            settings: Vec::new(),
            overrides,
        };

        // Saved settings first — the inbox is FIFO, so they land before any deal.
        for (pair, saved) in PAIRS.into_iter().zip(this.overrides.clone()) {
            for (key, value) in saved {
                this.apply(pair, key, value);
            }
        }
        this.ask(
            cx,
            |_| pons_web::describe_options(),
            |this, json, _| {
                this.settings = serde_json::from_str(&json).unwrap_or_default();
            },
        );
        this
    }

    /// Run `job` on the engine thread, then `done` with its answer back here
    fn ask<R: Send + 'static>(
        &self,
        cx: &mut Context<Self>,
        job: impl FnOnce(&mut WebTable) -> R + Send + 'static,
        done: impl FnOnce(&mut Self, R, &mut Context<Self>) + 'static,
    ) {
        let (reply, answer) = mpsc::channel();
        let _ = self.engine.send(Box::new(move |table| {
            let _ = reply.send(job(table));
        }));
        cx.spawn(async move |this, cx| {
            // ponytail: a blocking `recv` parks one background thread per pending
            // answer; an async channel if jobs ever queue by the dozen
            let wait = cx.background_executor().spawn(async move { answer.recv() });
            if let Ok(result) = wait.await {
                let _ = this.update(cx, |this, cx| {
                    done(this, result, cx);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Push one setting to the engine; it takes effect at the next deal
    fn apply(&self, pair: &'static str, key: String, value: Value) {
        let _ = self.engine.send(Box::new(move |_| match value {
            Value::Bool(on) => pons_web::set_option(pair, &key, on),
            Value::String(choice) => pons_web::set_choice(pair, &key, &choice),
            _ => {}
        }));
    }

    // --- play ---------------------------------------------------------------

    /// Deal a board with `job`, which answers a snapshot (or `null`)
    fn deal(
        &mut self,
        cx: &mut Context<Self>,
        job: impl FnOnce(&mut WebTable) -> String + Send + 'static,
    ) {
        self.board += 1;
        let board = self.board;
        self.ask(cx, job, move |this, json, cx| {
            if board == this.board {
                this.show(&json, cx);
            }
        });
    }

    /// Take a snapshot and ask for whatever the position now owes: a hint when
    /// it is our turn, the double-dummy table and the oracle once it is over
    fn show(&mut self, json: &str, cx: &mut Context<Self>) {
        let Ok(snap) = serde_json::from_str::<Snapshot>(json) else {
            self.status = "That is not a full 52-card PBN deal.".into();
            return;
        };
        self.status.clear();
        self.hint.clear();
        self.dd = None;
        self.oracle = None;
        let board = self.board;
        if snap.ended {
            self.ask(cx, WebTable::dd_table, move |this, json, _| {
                if board == this.board {
                    this.dd = serde_json::from_str(&json).ok();
                }
            });
            if snap.mode == "practice" {
                self.more_oracle(cx);
            }
        } else if snap.your_turn {
            self.ask(
                cx,
                |table| table.hint(),
                move |this, json, _| {
                    if board == this.board {
                        this.hint = serde_json::from_str(&json).unwrap_or_default();
                    }
                },
            );
        }
        self.snap = Some(snap);
    }

    fn more_oracle(&mut self, cx: &mut Context<Self>) {
        let board = self.board;
        self.ask(
            cx,
            |table| table.oracle(ORACLE_CHUNK),
            move |this, json, cx| {
                if board != this.board {
                    return;
                }
                // `null` (a pass-out) ends the chain
                let Ok(oracle) = serde_json::from_str::<Oracle>(&json) else {
                    return;
                };
                if oracle.n < ORACLE_TOTAL {
                    this.more_oracle(cx);
                }
                this.oracle = Some(oracle);
            },
        );
    }

    fn bid(&mut self, call: String, cx: &mut Context<Self>) {
        let board = self.board;
        self.ask(
            cx,
            move |table| table.bid(&call),
            move |this, json, cx| {
                if board == this.board {
                    this.show(&json, cx);
                }
            },
        );
    }

    fn render_play(&self, cx: &mut Context<Self>) -> Div {
        let (seat, dealer, vul) = (SEATS[self.seat], SEATS[self.dealer], VULS[self.vul]);
        let min_hcp = self.min_hcp;
        let floor = MIN_HCPS.iter().position(|&hcp| hcp == min_hcp);

        let options = controls()
            .child(picker("Seat", &SEATS, self.seat, cx, |this, index, _| {
                this.seat = index;
            }))
            .child(picker(
                "Dealer",
                &SEATS,
                self.dealer,
                cx,
                |this, index, _| {
                    this.dealer = index;
                },
            ))
            .child(picker("Vul", &VUL_NAMES, self.vul, cx, |this, index, _| {
                this.vul = index;
            }))
            .child(picker(
                "Min HCP",
                &MIN_HCPS,
                floor.unwrap_or(0),
                cx,
                |this, index, _| this.min_hcp = MIN_HCPS[index],
            ));
        let actions = controls()
            .child(primary("practice", "Deal practice").on_click(cx.listener(
                move |this, _, _, cx| {
                    this.deal(cx, move |table| {
                        table.deal_practice(seat, dealer, vul, min_hcp)
                    });
                },
            )))
            .child(
                button("demo".into(), "Deal demo", false, true).on_click(cx.listener(
                    move |this, _, _, cx| this.deal(cx, move |table| table.deal_demo(dealer, vul)),
                )),
            )
            .child(
                button("pbn".into(), "Paste PBN", false, true).on_click(cx.listener(
                    move |this, _, _, cx| {
                        let pbn = cx
                            .read_from_clipboard()
                            .and_then(|item| item.text())
                            .unwrap_or_default();
                        this.deal(cx, move |table| table.deal_pbn(pbn.trim(), dealer, vul));
                    },
                )),
            );

        // The web's order from here down: the board line, the table (your hand
        // beside the auction; a demo's four hands above it), the bidding box,
        // the hint, the feedback, then the reveal and its analysis.
        let mut page = div().flex().flex_col().gap(GAP).child(
            div()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(options)
                .child(actions),
        );
        if !self.status.is_empty() {
            page = page.child(panel().text_color(rgb(HEART)).child(self.status.clone()));
        }
        let Some(snap) = &self.snap else {
            return page.child(panel().child(div().opacity(0.6).italic().child(
                "Pick your options and deal: a practice board to bid one seat against the \
                 bots, a demo board to watch them, or a PBN deal from the clipboard.",
            )));
        };
        page = page.child(note(format!(
            "Dealer {} · Vul {}",
            seat_name(snap.dealer),
            snap.vul,
        )));

        let yours = snap
            .seat
            .and_then(|seat| Some((seat, snap.hands.get(&seat)?)));
        let mut table = div().flex().flex_wrap().items_start().gap(GAP);
        match yours {
            Some((seat, hand)) => {
                table = table.child(seated(panel().min_w(px(176.)), seat, hand));
            }
            None => page = page.child(compass(&snap.hands)),
        }
        page = page.child(table.child(auction(snap)));

        if snap.your_turn && !snap.ended {
            let call = |code: String, width: Pixels, cx: &mut Context<Self>| {
                let legal = snap.legal.contains(&code);
                let label = suits(code.clone());
                key(format!("call-{code}"), label, legal)
                    .w(width)
                    .when(legal, |this| {
                        this.on_click(cx.listener(move |this, _, _, cx| {
                            this.bid(code.clone(), cx);
                        }))
                    })
            };
            let line = || div().flex().gap(KEY_GAP);
            let mut bidding_box = div().flex().flex_col().items_center().gap(KEY_GAP);
            for level in 1..=7 {
                bidding_box = bidding_box.child(
                    line()
                        .children(STRAINS.map(|strain| call(format!("{level}{strain}"), KEY, cx))),
                );
            }
            page = page.child(bidding_box.child(
                line().children(["P", "X", "XX"].map(|code| call(code.into(), WIDE_KEY, cx))),
            ));

            if !self.hint.is_empty() {
                let estimates = self.hint.iter().map(|hint| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(suits(hint.strain.clone()))
                        .child(
                            div()
                                .text_size(px(28.8))
                                .line_height(relative(1.1))
                                .font_weight(FontWeight::BOLD)
                                .child(format!("{:.1}", hint.mean)),
                        )
                        .child(div().opacity(0.6).child(format!("± {:.1}", hint.sd)))
                });
                page = page.child(
                    panel()
                        .child(
                            div()
                                .font_weight(FontWeight::BOLD)
                                .mb(px(5.))
                                .child("Our tricks, as the auction reads so far"),
                        )
                        .child(div().flex().flex_wrap().gap(GAP).children(estimates)),
                );
            }
        }

        if !snap.feedback.is_empty() {
            let rows = snap.feedback.iter().map(|feedback| {
                // Ladder in nats when an authored node answered this hand (a
                // precedence), odds when the floor did.
                let top = feedback
                    .top
                    .iter()
                    .map(|(call, value)| {
                        if feedback.authored {
                            format!("{call} {value:.2}")
                        } else {
                            format!("{call} {value:.0}%")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" · ");
                let bot = if top.is_empty() {
                    "book has no opinion (bot would pass)".to_string()
                } else {
                    format!("bot: {top}")
                };
                let (mark, color) = if feedback.agreed {
                    ("✓", CLUB)
                } else {
                    ("✗", HEART)
                };
                div()
                    .flex()
                    .gap(px(6.))
                    .child(
                        div()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(color))
                            .child(mark),
                    )
                    .child(suits(format!("you: {} · {bot}", feedback.human)))
            });
            page = page.child(
                panel()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .text_size(px(15.2))
                    .children(rows),
            );
        }

        if !snap.ended {
            return page;
        }
        if let Some(contract) = &snap.contract {
            // A paper pill, so the strain keeps its colour
            page = page.child(
                div().flex().justify_center().child(
                    div()
                        .px(px(14.))
                        .py(px(4.))
                        .rounded_full()
                        .bg(rgb(PAPER))
                        .text_color(rgb(INK))
                        .shadow_sm()
                        .text_size(px(16.8))
                        .font_weight(FontWeight::BOLD)
                        .child(suits(contract.clone())),
                ),
            );
        }
        if yours.is_some() {
            page = page.child(compass(&snap.hands));
        }
        if let Some(oracle) = &self.oracle {
            page = page.child(panel().child(title("Oracle (opponents reshuffled)")).child(
                format!(
                    "{}/{ORACLE_TOTAL} shuffles: makes {:.0}% · tricks {}/{:.1}/{} · \
                         mean score {:+.0}",
                    oracle.n,
                    oracle.makes_pct,
                    oracle.tricks_min,
                    oracle.mean_tricks,
                    oracle.tricks_max,
                    oracle.mean_score,
                ),
            ));
        }

        let analysis = panel().child(title("Double dummy"));
        let Some(dd) = &self.dd else {
            return page.child(analysis.child(div().opacity(0.6).italic().child("solving…")));
        };
        let cell = || div().w(px(64.)).py(px(2.)).flex().justify_center();
        let head = div()
            .flex()
            .text_size(px(12.8))
            .opacity(0.75)
            .child(cell())
            .children(dd.seats.map(|seat| cell().child(seat_name(seat))));
        let rows = dd.rows.iter().map(|row| {
            div()
                .flex()
                .border_t_1()
                .border_color(rgba(0x00000014))
                .child(cell().child(suits(row.strain.clone())))
                .children(
                    row.tricks
                        .iter()
                        .map(|tricks| cell().child(tricks.to_string())),
                )
        });
        let verdict = dd.verdict.iter().flatten().map(|line| suits(line.clone()));
        page.child(
            analysis
                .child(
                    div().flex().justify_center().child(
                        div()
                            .flex()
                            .flex_col()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(head)
                            .children(rows),
                    ),
                )
                .child(
                    div()
                        .mt(px(6.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .font_weight(FontWeight::BOLD)
                        .children(verdict),
                ),
        )
    }

    // --- book ---------------------------------------------------------------

    fn load_book(&mut self, cx: &mut Context<Self>) {
        let pair = PAIRS[self.pair];
        self.nodes.clear();
        self.page = 0;
        self.status = "Building the book…".into();
        self.ask(
            cx,
            move |_| pons_web::book(pair),
            move |this, json, _| {
                if pair != PAIRS[this.pair] {
                    return;
                }
                this.status.clear();
                this.nodes = serde_json::from_str(&json).unwrap_or_default();
                for node in &mut this.nodes {
                    let rules = node
                        .rules
                        .iter()
                        .map(|rule| format!(" {} {} {}", rule.call, rule.text, rule.label));
                    node.haystack = typeable(
                        &std::iter::once(format!("{} {}", node.book, node.auction))
                            .chain(rules)
                            .collect::<String>(),
                    );
                }
            },
        );
    }

    /// The book tab's filter is typed straight at the window: no text field
    // ponytail: append and backspace only — a real input (gpui's `input.rs`
    // example is the model) if anyone wants a cursor or IME
    fn key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if self.tab != Tab::Book {
            return;
        }
        let stroke = &event.keystroke;
        match stroke.key.as_str() {
            "backspace" => drop(self.filter.pop()),
            "escape" => self.filter.clear(),
            _ if stroke.modifiers.control || stroke.modifiers.platform => return,
            _ => match &stroke.key_char {
                Some(typed) => self.filter.push_str(typed),
                None => return,
            },
        }
        self.page = 0;
        cx.notify();
    }

    fn render_book(&self, cx: &mut Context<Self>) -> Div {
        let needle = typeable(&self.filter);
        let hits: Vec<&Node> = self
            .nodes
            .iter()
            .filter(|node| node.haystack.contains(&needle))
            .collect();
        let pages = hits.len().div_ceil(PAGE).max(1);
        let page = self.page.min(pages - 1);

        // Twice on a long page: beside the filter, and under the last node
        let pager = |place: &str, cx: &mut Context<Self>| {
            let turn = |name: &str, label: &'static str, to: usize| {
                button(format!("{name}-{place}"), label, false, to != page).on_click(cx.listener(
                    move |this, _, _, cx| {
                        this.page = to;
                        cx.notify();
                    },
                ))
            };
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .text_size(SMALL)
                .child(turn("prev", "‹ Prev", page.saturating_sub(1)))
                .child(format!("{} / {pages}", page + 1))
                .child(turn("next", "Next ›", (page + 1).min(pages - 1)))
        };
        // The web's search field, drawn: the keys land on the window (see `key`)
        let filter = div()
            .flex_1()
            .min_w(px(192.))
            .px(px(6.))
            .py(px(3.))
            .rounded(px(5.))
            .border_1()
            .border_color(rgba(RIM))
            .text_size(SMALL)
            .child(if self.filter.is_empty() {
                div()
                    .truncate()
                    .opacity(DIM)
                    .child("Type to filter, e.g. 1nt 2c")
            } else {
                div().truncate().child(format!("{}▏", self.filter))
            });
        let bar = controls()
            .child(picker(
                "Partnership",
                &PAIR_NAMES,
                self.pair,
                cx,
                |this, index, cx| {
                    this.pair = index;
                    this.load_book(cx);
                },
            ))
            .child(filter)
            .child(note(format!(
                "{} of {} nodes",
                hits.len(),
                self.nodes.len(),
            )))
            .child(pager("top", cx));

        let mut list = div().flex().flex_col().gap(GAP).child(bar);
        if !self.status.is_empty() {
            list = list.child(note(self.status.clone()));
        }
        let cards = hits.into_iter().skip(page * PAGE).take(PAGE).map(|node| {
            let badge = match node.book.as_str() {
                "constructive" => CLUB,
                "competitive" => 0xb06000,
                _ => 0x8f2f3a,
            };
            let head = div()
                .flex()
                .items_center()
                .gap(px(10.))
                .mb(px(5.))
                .child(
                    div()
                        .px(px(7.))
                        .py(px(1.))
                        .rounded(px(4.))
                        .bg(rgb(badge))
                        .text_color(rgb(0xffffff))
                        .text_size(px(11.2))
                        .font_weight(FontWeight::BOLD)
                        .child(node.book.to_uppercase()),
                )
                .child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .child(suits(node.auction.clone())),
                );
            // Call, weight, meaning, tag: fixed gutters, so the meanings of a
            // node start on one edge.  The small columns keep the row's line
            // height, which puts them on the first line of a wrapped meaning.
            let rules = node.rules.iter().map(|rule| {
                div()
                    .flex()
                    .items_start()
                    .gap(px(10.))
                    .py(px(2.))
                    .child(
                        div()
                            .w(px(45.))
                            .flex_none()
                            .font_weight(FontWeight::BOLD)
                            .child(suits(rule.call.clone())),
                    )
                    .child(
                        div()
                            .w(px(46.))
                            .flex_none()
                            .opacity(0.5)
                            .text_size(px(12.8))
                            .line_height(px(21.))
                            .child(format!("w{:.2}", f32::from(rule.weight) / 100.)),
                    )
                    .child(div().flex_1().child(suits(rule.text.clone())))
                    .when(!rule.label.is_empty(), |this| {
                        this.child(
                            div()
                                .flex_none()
                                .mt(px(2.))
                                .px(px(6.))
                                .rounded(px(4.))
                                .bg(rgba(0x00000014))
                                .text_size(px(11.5))
                                .line_height(px(17.))
                                .child(rule.label.clone()),
                        )
                    })
            });
            let note = node.note.clone().map(|note| div().py(px(2.)).child(note));
            panel()
                .flex()
                .flex_col()
                .child(head)
                .children(rules)
                .children(note)
        });
        list = list.child(div().flex().flex_col().gap(px(11.)).children(cards));
        if pages > 1 {
            list = list.child(div().flex().justify_center().child(pager("bottom", cx)));
        }
        list
    }

    // --- settings -----------------------------------------------------------

    /// A setting's current value for one partnership: its override, else the default
    fn value(&self, pair: usize, key: &str) -> Value {
        self.overrides[pair].get(key).cloned().unwrap_or_else(|| {
            self.settings
                .iter()
                .find(|setting| setting.key == key)
                .map_or(Value::Null, |setting| setting.default.clone())
        })
    }

    /// Whether the master a row `requires` is armed
    fn live(&self, setting: &Setting) -> bool {
        let Some(requires) = &setting.requires else {
            return true;
        };
        let (pair, requires) = match requires.strip_prefix("opponent:") {
            Some(rest) => (1 - self.pair, rest),
            None => (self.pair, requires.as_str()),
        };
        let (key, want) = match requires.split_once('=') {
            Some((key, want)) => (key, Some(want)),
            None => (requires, None),
        };
        satisfies(&self.value(pair, key), want)
    }

    fn set(&mut self, key: String, value: Value, cx: &mut Context<Self>) {
        let default = self
            .settings
            .iter()
            .find(|setting| setting.key == key)
            .map(|setting| &setting.default);
        if default == Some(&value) {
            self.overrides[self.pair].remove(&key);
        } else {
            self.overrides[self.pair].insert(key.clone(), value.clone());
        }
        self.nodes.clear(); // the Book tab rebuilds from the new agreements
        self.apply(PAIRS[self.pair], key, value);
        self.status = match self.save() {
            Ok(()) => String::new(),
            Err(error) => format!("Settings not saved: {error}"),
        };
        cx.notify();
    }

    fn save(&self) -> std::io::Result<()> {
        let Some(path) = config_path() else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(&self.overrides)?)
    }

    /// Back to the registry defaults for the partnership on show
    fn reset(&mut self, cx: &mut Context<Self>) {
        self.overrides[self.pair].clear();
        for setting in &self.settings {
            self.apply(
                PAIRS[self.pair],
                setting.key.clone(),
                setting.default.clone(),
            );
        }
        self.nodes.clear();
        self.status = match self.save() {
            Ok(()) => String::new(),
            Err(error) => format!("Settings not saved: {error}"),
        };
        cx.notify();
    }

    /// The web client's questionnaire: one paper panel per section, one
    /// question per line — a check box, or a titled family of radio answers
    fn render_settings(&self, cx: &mut Context<Self>) -> Div {
        let mut page = div()
            .flex()
            .flex_col()
            .gap(GAP)
            .child(note(
                "Toggle conventions, then deal in Play — changes apply to the next deal.",
            ))
            .child(controls().child(picker(
                "Partnership",
                &PAIR_NAMES,
                self.pair,
                cx,
                |this, index, _| this.pair = index,
            )));
        if !self.status.is_empty() {
            page = page.child(panel().text_color(rgb(HEART)).child(self.status.clone()));
        }

        let mut sections: Vec<(&str, Div)> = Vec::new();
        for setting in &self.settings {
            if sections
                .last()
                .is_none_or(|(name, _)| *name != setting.section)
            {
                let section = panel()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .text_size(px(15.2))
                    .child(title(&setting.section));
                sections.push((&setting.section, section));
            }
            let live = self.live(setting);
            let current = self.value(self.pair, &setting.key);
            let label = if setting.label.is_empty() {
                humanize(&setting.key)
            } else {
                setting.label.clone()
            };
            let needs = match (live, &setting.requires) {
                (false, Some(requires)) => format!(
                    "  (needs {})",
                    requires
                        .replace("opponent:", "opponent ")
                        .replace('=', ": ")
                ),
                _ => String::new(),
            };
            let option = |id: String,
                          text: String,
                          on: bool,
                          round: bool,
                          value: Value,
                          cx: &mut Context<Self>| {
                let key = setting.key.clone();
                answer(id, format!("{text}{needs}"), on, round, live).when(live, |this| {
                    this.on_click(cx.listener(move |this, _, _, cx| {
                        this.set(key.clone(), value.clone(), cx);
                    }))
                })
            };
            let mut body = sections.pop().expect("a section was just pushed");
            if setting.kind == "toggle" {
                let on = current.as_bool().unwrap_or(false);
                let id = format!("set-{}", setting.key);
                body.1 = body
                    .1
                    .child(option(id, label, on, false, Value::Bool(!on), cx));
            } else {
                body.1 = body.1.child(
                    div()
                        .mt(px(3.))
                        .py(px(2.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(suits(label)),
                );
                for variant in &setting.variants {
                    let id = format!("set-{}-{}", setting.key, variant.value);
                    let on = current.as_str() == Some(&variant.value);
                    let value = Value::String(variant.value.clone());
                    body.1 = body
                        .1
                        .child(option(id, variant.label.clone(), on, true, value, cx));
                }
            }
            sections.push(body);
        }
        page = page.child(
            div()
                .flex()
                .flex_col()
                .gap(px(13.))
                .children(sections.into_iter().map(|(_, section)| section)),
        );

        let name = PAIR_NAMES[self.pair];
        page.child(
            div().flex().child(
                button(
                    "reset".into(),
                    format!("Reset {name} to defaults"),
                    false,
                    true,
                )
                .on_click(cx.listener(move |_, _, window, cx| {
                    let answer = window.prompt(
                        PromptLevel::Warning,
                        &format!("Reset {name} convention settings to defaults?"),
                        None,
                        &["Reset", "Cancel"],
                        cx,
                    );
                    cx.spawn(async move |this, cx| {
                        if answer.await == Ok(0) {
                            let _ = this.update(cx, Self::reset);
                        }
                    })
                    .detach();
                })),
            ),
        )
    }
}

impl Render for Pons {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tab = |tab: Tab, label: &'static str, cx: &mut Context<Self>| {
            button(format!("tab-{label}"), label, self.tab == tab, true).on_click(cx.listener(
                move |this, _, _, cx| {
                    this.tab = tab;
                    this.status.clear();
                    if tab == Tab::Book && this.nodes.is_empty() {
                        this.load_book(cx);
                    }
                    cx.notify();
                },
            ))
        };
        // A window control: a bare glyph, so it does not read as a fourth tab
        let control = |id: &'static str, glyph: &'static str| {
            div()
                .id(id)
                .size(px(28.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .cursor_pointer()
                .hover(|style| style.bg(rgba(WASH)))
                .child(glyph)
        };
        // The web's `header`: the name, then the tabs
        let header = div()
            .flex()
            .items_center()
            .gap(px(6.))
            .px(GAP)
            .py(px(8.))
            .border_b_1()
            .border_color(rgba(0x00000033))
            .child(
                div()
                    .mr(px(18.))
                    .text_size(px(19.2))
                    .font_weight(FontWeight::BOLD)
                    .child("pons"),
            )
            .child(tab(Tab::Play, "Play", cx))
            .child(tab(Tab::Book, "Book", cx))
            .child(tab(Tab::Settings, "Settings", cx))
            // We draw our own decorations (see `main`), so the header is the
            // title bar: the gap drags, a double click zooms.
            .when(
                matches!(window.window_decorations(), Decorations::Client { .. }),
                |row| {
                    row.child(div().flex_1().h_full().on_mouse_down(
                        MouseButton::Left,
                        |event, window, _| {
                            if event.click_count == 2 {
                                window.zoom_window();
                            } else {
                                window.start_window_move();
                            }
                        },
                    ))
                    .child(
                        control("minimize", "–").on_click(|_, window, _| window.minimize_window()),
                    )
                    .child(control("zoom", "□").on_click(|_, window, _| window.zoom_window()))
                    .child(control("close", "×").on_click(|_, window, _| window.remove_window()))
                },
            );

        let body = match self.tab {
            Tab::Play => self.render_play(cx),
            Tab::Book => self.render_book(cx),
            Tab::Settings => self.render_settings(cx),
        };

        div()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event, _, cx| this.key(event, cx)))
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(FELT))
            .text_color(rgb(CHALK))
            .font_family(UI_FONT)
            .text_size(px(15.))
            .line_height(relative(1.4))
            .child(header)
            .child(
                div()
                    .id("body")
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    // The web's `main`: one centred column for every tab
                    .child(div().max_w(COLUMN).mx_auto().p(GAP).child(body)),
            )
            .when(!window.is_maximized(), |root| root.children(grips()))
    }
}

/// The resize handles of a window that draws its own decorations: strips
/// [`RESIZE_GRIP`] wide over the four edges, then squares twice that over the
/// corners. They paint last, so they take the press and the cursor from
/// whatever lies under them.
fn grips() -> impl Iterator<Item = Div> {
    use ResizeEdge::{Bottom, BottomLeft, BottomRight, Left, Right, Top, TopLeft, TopRight};
    // (edge, horizontal anchor, vertical anchor): −1 near, 0 spanning, 1 far.
    [
        (Top, 0, -1),
        (Bottom, 0, 1),
        (Left, -1, 0),
        (Right, 1, 0),
        (TopLeft, -1, -1),
        (TopRight, 1, -1),
        (BottomLeft, -1, 1),
        (BottomRight, 1, 1),
    ]
    .into_iter()
    .map(|(edge, x, y)| {
        let reach = if x != 0 && y != 0 {
            RESIZE_GRIP * 2.
        } else {
            RESIZE_GRIP
        };
        let grip = div().absolute().occlude();
        let grip = match x {
            -1 => grip.left_0().w(reach),
            1 => grip.right_0().w(reach),
            _ => grip.left_0().right_0(),
        };
        let grip = match y {
            -1 => grip.top_0().h(reach),
            1 => grip.bottom_0().h(reach),
            _ => grip.top_0().bottom_0(),
        };
        grip.cursor(match (x, y) {
            (0, _) => CursorStyle::ResizeUpDown,
            (_, 0) => CursorStyle::ResizeLeftRight,
            _ if x == y => CursorStyle::ResizeUpLeftDownRight,
            _ => CursorStyle::ResizeUpRightDownLeft,
        })
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            window.start_window_resize(edge);
            cx.stop_propagation();
        })
    })
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.text_system()
            .add_fonts(vec![Cow::Borrowed(
                include_bytes!("../assets/suits.ttf").as_slice(),
            )])
            .expect("the embedded suit font loads");
        let bounds = Bounds::centered(None, size(px(1100.), px(800.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            // GNOME on Wayland draws no frame, and GPUI then still reports
            // the server decorations it asked for; ask for ours everywhere.
            window_decorations: Some(WindowDecorations::Client),
            ..Default::default()
        };
        cx.open_window(options, |window, cx| {
            let view = cx.new(Pons::new);
            window.focus(&view.read(cx).focus);
            view
        })
        .expect("a window opens");
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
}
