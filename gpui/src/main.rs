//! Native [GPUI](https://gpui.rs) front end for pons
//!
//! The engine surface is `pons-web`'s — the same `WebTable` the browser
//! drives, called as plain Rust and answering in JSON.  What native buys is
//! threads: the engine lives on its own thread (the UI never blocks on a
//! bidder or a solver), and double dummy fans out over `pons-dds`'s pool, so
//! the practice oracle prices a thousand reshuffles where the browser does a
//! hundred.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc;

use gpui::prelude::*;
use gpui::{
    App, Application, Bounds, Context, Div, FocusHandle, HighlightStyle, KeyDownEvent,
    SharedString, Stateful, StyledText, Window, WindowBounds, WindowOptions, div, px, rgb, size,
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

const BG: u32 = 0x1e1f22;
const PANEL: u32 = 0x2b2d31;
const BORDER: u32 = 0x45474d;
const TEXT: u32 = 0xe6e6e6;
const DIM: u32 = 0x9a9ca3;
const ACCENT: u32 = 0x2f6fde;
const GOOD: u32 = 0x4ade80;
const BAD: u32 = 0xf87171;

const SEATS: [&str; 4] = ["N", "E", "S", "W"];
const VULS: [&str; 4] = ["none", "ns", "ew", "both"];
const PAIRS: [&str; 2] = ["ns", "ew"];
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
    index: usize,
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

/// Text with its suit symbols in the house colors (orange diamonds, blue clubs)
fn suits(text: impl Into<SharedString>) -> StyledText {
    let text: SharedString = text.into();
    let highlights: Vec<_> = text
        .char_indices()
        .filter_map(|(at, c)| {
            let color = match c {
                '♥' => 0xef4444,
                '♦' => 0xf59e0b,
                '♣' => 0x60a5fa,
                _ => return None,
            };
            let style = HighlightStyle {
                color: Some(rgb(color).into()),
                ..Default::default()
            };
            Some((at..at + c.len_utf8(), style))
        })
        .collect();
    StyledText::new(text).with_highlights(highlights)
}

fn button(id: String, label: impl IntoElement, active: bool, enabled: bool) -> Stateful<Div> {
    div()
        .id(SharedString::from(id))
        .px_2()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(if active { ACCENT } else { PANEL }))
        .when(!enabled, |this| this.opacity(0.35))
        .when(enabled, |this| {
            this.cursor_pointer().hover(|style| style.bg(rgb(ACCENT)))
        })
        .child(label)
}

fn panel() -> Div {
    div()
        .p_2()
        .rounded_md()
        .bg(rgb(PANEL))
        .border_1()
        .border_color(rgb(BORDER))
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
        let cycle = |id: &str, label: String, cx: &mut Context<Self>, step: fn(&mut Self)| {
            button(id.into(), label, false, true).on_click(cx.listener(move |this, _, _, cx| {
                step(this);
                cx.notify();
            }))
        };
        let (seat, dealer, vul) = (SEATS[self.seat], SEATS[self.dealer], VULS[self.vul]);
        let min_hcp = self.min_hcp;

        let controls = div()
            .flex()
            .flex_wrap()
            .gap_2()
            .child(cycle("seat", format!("Seat {seat}"), cx, |this| {
                this.seat = (this.seat + 1) % 4;
            }))
            .child(cycle("dealer", format!("Dealer {dealer}"), cx, |this| {
                this.dealer = (this.dealer + 1) % 4;
            }))
            .child(cycle("vul", format!("Vul {vul}"), cx, |this| {
                this.vul = (this.vul + 1) % 4;
            }))
            .child(cycle("hcp", format!("Min HCP {min_hcp}"), cx, |this| {
                this.min_hcp = (this.min_hcp + 2) % 22;
            }))
            .child(
                button("practice".into(), "Deal practice", false, true).on_click(cx.listener(
                    move |this, _, _, cx| {
                        this.deal(cx, move |table| {
                            table.deal_practice(seat, dealer, vul, min_hcp)
                        });
                    },
                )),
            )
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

        let mut page = div().flex().flex_col().gap_3().child(controls);
        if !self.status.is_empty() {
            page = page.child(div().text_color(rgb(BAD)).child(self.status.clone()));
        }
        let Some(snap) = &self.snap else {
            return page.child(div().text_color(rgb(DIM)).child(
                "Deal a practice board to bid one seat against the bots, a demo board \
                 to watch them, or paste a PBN deal from the clipboard.",
            ));
        };

        let hand = |seat: char| {
            let mut cell = panel().w(px(170.)).flex().flex_col();
            let Some(hand) = snap.hands.get(&seat) else {
                return cell.text_color(rgb(DIM)).child(format!("{seat} — hidden"));
            };
            let you = if snap.seat == Some(seat) {
                " (you)"
            } else {
                ""
            };
            cell = cell.child(
                div()
                    .text_color(rgb(DIM))
                    .child(format!("{seat}{you} · {} HCP", hand.hcp)),
            );
            for (symbol, cards) in [
                ('♠', &hand.spades),
                ('♥', &hand.hearts),
                ('♦', &hand.diamonds),
                ('♣', &hand.clubs),
            ] {
                let cards = if cards.is_empty() { "—" } else { cards };
                cell = cell.child(suits(format!("{symbol} {cards}")));
            }
            cell
        };
        let row = || div().flex().gap_2().justify_center();
        let compass = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(row().child(hand('N')))
            .child(
                row()
                    .child(hand('W'))
                    .child(div().w(px(170.)))
                    .child(hand('E')),
            )
            .child(row().child(hand('S')));

        // The auction, west first: blank cells until the dealer's column
        let lead = COLUMNS.iter().position(|&c| c == snap.dealer).unwrap_or(0);
        let cells: Vec<String> = std::iter::repeat_n(String::new(), lead)
            .chain(snap.auction.iter().cloned())
            .collect();
        let cell = |text: String| div().w(px(48.)).child(suits(text));
        let mut auction = panel().flex().flex_col().child(
            div()
                .flex()
                .text_color(rgb(DIM))
                .children(COLUMNS.map(|seat| cell(seat.to_string()))),
        );
        for line in cells.chunks(4) {
            auction = auction.child(div().flex().children(line.iter().cloned().map(cell)));
        }
        auction = auction.child(
            div()
                .text_color(rgb(DIM))
                .child(format!("Dealer {} · Vul {}", snap.dealer, snap.vul)),
        );

        let mut side = div().flex().flex_col().gap_2().child(auction);

        if snap.your_turn {
            let call = |code: String, cx: &mut Context<Self>| {
                let legal = snap.legal.contains(&code);
                let label = suits(code.clone());
                button(format!("call-{code}"), label, false, legal)
                    .w(px(44.))
                    .when(legal, |this| {
                        this.on_click(cx.listener(move |this, _, _, cx| {
                            this.bid(code.clone(), cx);
                        }))
                    })
            };
            let mut bidding_box = div().flex().flex_col().gap_1();
            for level in 1..=7 {
                bidding_box = bidding_box.child(
                    div()
                        .flex()
                        .gap_1()
                        .children(STRAINS.map(|strain| call(format!("{level}{strain}"), cx))),
                );
            }
            bidding_box = bidding_box.child(
                div()
                    .flex()
                    .gap_1()
                    .children(["P", "X", "XX"].map(|code| call(code.into(), cx))),
            );
            side = side.child(bidding_box);

            if !self.hint.is_empty() {
                let line = self
                    .hint
                    .iter()
                    .map(|h| format!("{} {:.1}±{:.1}", h.strain, h.mean, h.sd))
                    .collect::<Vec<_>>()
                    .join("   ");
                side = side.child(
                    div()
                        .text_color(rgb(DIM))
                        .child("Our tricks, as the net reads the auction:")
                        .child(suits(line)),
                );
            }
        }

        for feedback in &snap.feedback {
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
            let (mark, color) = if feedback.agreed {
                ("✓", GOOD)
            } else {
                ("✗", BAD)
            };
            side = side.child(
                div()
                    .flex()
                    .gap_2()
                    .child(div().text_color(rgb(color)).child(mark))
                    .child(suits(format!(
                        "call {}: you {} — bot: {}",
                        feedback.index + 1,
                        feedback.human,
                        if top.is_empty() { "off book" } else { &top },
                    ))),
            );
        }

        if let Some(contract) = &snap.contract {
            side = side.child(suits(format!("Contract: {contract}")));
        }
        if let Some(oracle) = &self.oracle {
            side = side.child(format!(
                "Oracle, {}/{ORACLE_TOTAL} opponent reshuffles: makes {:.0}% · {:.2} tricks \
                 ({}–{}) · mean score {:+.0}",
                oracle.n,
                oracle.makes_pct,
                oracle.mean_tricks,
                oracle.tricks_min,
                oracle.tricks_max,
                oracle.mean_score,
            ));
        }
        if let Some(dd) = &self.dd {
            let cell = |text: String| div().w(px(36.)).child(suits(text));
            let mut table = panel().flex().flex_col().child(
                div()
                    .flex()
                    .text_color(rgb(DIM))
                    .child(cell(String::new()))
                    .children(dd.seats.map(|seat| cell(seat.to_string()))),
            );
            for row in &dd.rows {
                table = table.child(
                    div()
                        .flex()
                        .child(cell(row.strain.clone()))
                        .children(row.tricks.iter().map(|tricks| cell(tricks.to_string()))),
                );
            }
            side = side.child(table);
            for line in dd.verdict.iter().flatten() {
                side = side.child(suits(line.clone()));
            }
        } else if snap.ended {
            side = side.child(div().text_color(rgb(DIM)).child("Solving double dummy…"));
        }

        page.child(div().flex().flex_wrap().gap_4().child(compass).child(side))
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

        let turn = |id: &str, label: &'static str, to: usize, cx: &mut Context<Self>| {
            button(id.into(), label, false, to != page).on_click(cx.listener(
                move |this, _, _, cx| {
                    this.page = to;
                    cx.notify();
                },
            ))
        };
        let pair = |index: usize, label: &'static str, cx: &mut Context<Self>| {
            button(format!("book-{index}"), label, self.pair == index, true).on_click(cx.listener(
                move |this, _, _, cx| {
                    this.pair = index;
                    this.load_book(cx);
                },
            ))
        };
        let filter = if self.filter.is_empty() {
            "type to filter — suits as c d h s (1nt 2c), Esc clears".to_string()
        } else {
            format!("filter: {}▏", self.filter)
        };
        let bar = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(pair(0, "North–South", cx))
            .child(pair(1, "East–West", cx))
            .child(turn("prev", "‹ Prev", page.saturating_sub(1), cx))
            .child(format!("{}/{pages}", page + 1))
            .child(turn("next", "Next ›", (page + 1).min(pages - 1), cx))
            .child(div().text_color(rgb(DIM)).child(format!(
                "{} of {} nodes · {filter}",
                hits.len(),
                self.nodes.len(),
            )));

        let mut list = div().flex().flex_col().gap_2().child(bar);
        if !self.status.is_empty() {
            list = list.child(div().text_color(rgb(DIM)).child(self.status.clone()));
        }
        for node in hits.into_iter().skip(page * PAGE).take(PAGE) {
            let mut card = panel().flex().flex_col().child(
                div()
                    .flex()
                    .gap_2()
                    .child(suits(node.auction.clone()))
                    .child(div().text_color(rgb(DIM)).child(node.book.clone())),
            );
            if let Some(note) = &node.note {
                card = card.child(div().text_color(rgb(DIM)).child(note.clone()));
            }
            for rule in &node.rules {
                let label = if rule.label.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", rule.label)
                };
                card = card.child(
                    div()
                        .flex()
                        .gap_2()
                        .child(div().w(px(48.)).flex_none().child(suits(rule.call.clone())))
                        .child(
                            div()
                                .w(px(56.))
                                .flex_none()
                                .text_color(rgb(DIM))
                                .child(format!("{:.2}", f32::from(rule.weight) / 100.)),
                        )
                        .child(div().flex_1().child(suits(format!("{}{label}", rule.text)))),
                );
            }
            list = list.child(card);
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

    fn render_settings(&self, cx: &mut Context<Self>) -> Div {
        let pair = |index: usize, label: &'static str, cx: &mut Context<Self>| {
            button(format!("pair-{index}"), label, self.pair == index, true).on_click(cx.listener(
                move |this, _, _, cx| {
                    this.pair = index;
                    cx.notify();
                },
            ))
        };
        let mut page = div().flex().flex_col().gap_2().child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(pair(0, "North–South", cx))
                .child(pair(1, "East–West", cx))
                .child(
                    div()
                        .text_color(rgb(DIM))
                        .child("Changes apply from the next deal."),
                ),
        );
        if !self.status.is_empty() {
            page = page.child(div().text_color(rgb(BAD)).child(self.status.clone()));
        }

        let mut section = "";
        for setting in &self.settings {
            if setting.section != section {
                section = &setting.section;
                page = page.child(div().mt_2().text_color(rgb(DIM)).child(section.to_string()));
            }
            let live = self.live(setting);
            let current = self.value(self.pair, &setting.key);
            let label = if setting.label.is_empty() {
                setting.key.replace('_', " ")
            } else {
                setting.label.clone()
            };
            let option =
                |id: String, text: String, on: bool, value: Value, cx: &mut Context<Self>| {
                    let key = setting.key.clone();
                    button(id, text, on, live).when(live, |this| {
                        this.on_click(cx.listener(move |this, _, _, cx| {
                            this.set(key.clone(), value.clone(), cx);
                        }))
                    })
                };
            let mut row = div().flex().flex_wrap().items_center().gap_2();
            if setting.kind == "toggle" {
                let on = current.as_bool().unwrap_or(false);
                let mark = if on { "☑" } else { "☐" };
                let id = format!("set-{}", setting.key);
                row = row.child(option(
                    id,
                    format!("{mark} {label}"),
                    on,
                    Value::Bool(!on),
                    cx,
                ));
            } else {
                row = row.child(div().w(px(220.)).child(label));
                for variant in &setting.variants {
                    let id = format!("set-{}-{}", setting.key, variant.value);
                    let on = current.as_str() == Some(&variant.value);
                    let value = Value::String(variant.value.clone());
                    row = row.child(option(id, variant.label.clone(), on, value, cx));
                }
            }
            if let (false, Some(requires)) = (live, &setting.requires) {
                row = row.child(
                    div()
                        .text_color(rgb(DIM))
                        .child(format!("needs {requires}")),
                );
            }
            page = page.child(row);
        }
        page
    }
}

impl Render for Pons {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
        let tabs = div()
            .flex()
            .gap_2()
            .p_2()
            .border_b_1()
            .border_color(rgb(BORDER))
            .child(tab(Tab::Play, "Play", cx))
            .child(tab(Tab::Book, "Book", cx))
            .child(tab(Tab::Settings, "Settings", cx));

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
            .bg(rgb(BG))
            .text_color(rgb(TEXT))
            .text_sm()
            .child(tabs)
            .child(
                div()
                    .id("body")
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .p_3()
                    .child(body),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1100.), px(800.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
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
