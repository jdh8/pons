//! Wasm bindings for the pons web UI
//!
//! One exported [`WebTable`] drives both interactive modes — practice (a human
//! bids one seat against three bots) and demo (bots bid all four) — and a free
//! [`book`] function exports the authored 2/1 books for the browser.  Every
//! method returns a JSON `Snapshot` string; the JS side is a thin renderer.
//!
//! Double dummy comes from the pure-Rust `pons-dds` (the native `pons/dd`
//! feature wraps C++ and cannot target wasm), driven strictly on its
//! single-threaded paths.  It is only consulted **after** the auction — a
//! full [`dd_table`][WebTable::dd_table] once all four hands are revealed,
//! and a fairness [`oracle`][WebTable::oracle] that reshuffles the unseen
//! opposing hands instead of judging the one true layout in hindsight.

use std::collections::{BTreeMap, HashSet};

use contract_bridge::auction::{Auction, Call, display_calls};
use contract_bridge::deck::{fill_deals, full_deal};
use contract_bridge::eval::{self, HandEvaluator as _, SimpleEvaluator};
use contract_bridge::{AbsoluteVulnerability, Bid, Builder, FullDeal, Hand, Seat, Strain};
use pons::bidding::american::bare_american;
use pons::bidding::fallback::Fallback;
use pons::bidding::{Stance, Table, american, constraint, inference, instinct};
use pons::scoring::final_contract;
use pons_dds::{Solver, TrickCountTable, solve_deal_on};
use rand::SeedableRng as _;
use rand::rngs::StdRng;
use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;

/// One hand as the UI renders it: a ranks string per suit, plus HCP
#[derive(Serialize)]
struct HandJson {
    spades: String,
    hearts: String,
    diamonds: String,
    clubs: String,
    hcp: u8,
}

impl HandJson {
    fn new(hand: Hand) -> Self {
        use contract_bridge::Suit;
        Self {
            spades: hand[Suit::Spades].to_string(),
            hearts: hand[Suit::Hearts].to_string(),
            diamonds: hand[Suit::Diamonds].to_string(),
            clubs: hand[Suit::Clubs].to_string(),
            hcp: SimpleEvaluator(eval::hcp::<u8>).eval(hand),
        }
    }
}

/// The bot's opinion on one human call, recorded as it was given
#[derive(Serialize, Clone)]
struct Feedback {
    /// 0-based position of the call in the auction
    index: usize,
    /// The call the human chose
    human: String,
    /// Whether the human matched the bot's top pick (or passed off-book)
    agreed: bool,
    /// The bot's top-3 legal calls as `(code, percent)`; empty off-book
    top: Vec<(String, f32)>,
}

/// The legally-visible position, serialized to the JS renderer
#[derive(Serialize)]
struct Snapshot<'a> {
    mode: &'static str,
    dealer: char,
    vul: &'static str,
    seat: Option<char>,
    hands: BTreeMap<char, HandJson>,
    auction: Vec<String>,
    your_turn: bool,
    ended: bool,
    legal: Vec<String>,
    contract: Option<String>,
    feedback: &'a [Feedback],
}

/// One dealt board and its auction state
struct Board {
    table: Table<Stance, Stance>,
    deal: FullDeal,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    /// The human's seat, or [`None`] in demo mode
    human: Option<Seat>,
    auction: Auction,
    feedback: Vec<Feedback>,
    /// Cached double-dummy table, solved on first request after the reveal
    dd: Option<TrickCountTable>,
    /// Oracle statistics accumulated over opponent reshuffles
    oracle: Oracle,
    /// One reused solver for both DD jobs (warm allocation across chunks)
    solver: Option<Solver>,
}

impl Board {
    /// Bid bot seats forward until the human is to act or the auction ends
    fn advance(&mut self) {
        while !self.auction.has_ended() {
            let seat = self.table.seat_to_act(self.auction.len());
            if Some(seat) == self.human {
                break;
            }
            let call = self.table.next_call(self.deal[seat], &self.auction);
            self.auction.push(call);
        }
    }

    /// The bot's ranked top-3 legal calls with softmax percentages
    ///
    /// Port of the CLI feedback in `examples/practice-bidding`: finite logits
    /// only, legal calls only, percent from the full softmax.
    fn top3(&self) -> Vec<(String, f32)> {
        let seat = self.table.seat_to_act(self.auction.len());
        let Some(logits) = self.table.classify(self.deal[seat], &self.auction) else {
            return Vec::new();
        };
        let softmax = logits.softmax();
        let mut scored: Vec<(Call, f32)> = logits
            .iter()
            .filter(|&(_, &logit)| logit.is_finite())
            .filter(|(call, _)| self.auction.can_push(*call).is_ok())
            .map(|(call, &logit)| (call, logit))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("logits are never NaN"));
        scored
            .into_iter()
            .take(3)
            .map(|(call, _)| {
                let prob = softmax.as_ref().map_or(0.0, |sm| *sm.get(call));
                (call.to_string(), 100.0 * prob)
            })
            .collect()
    }

    /// All calls the seat to act may legally make, as display codes
    fn legal(&self) -> Vec<String> {
        if self.auction.has_ended() {
            return Vec::new();
        }
        let bids = (1..=7).flat_map(|level| {
            Strain::ASC
                .into_iter()
                .map(move |strain| Call::Bid(Bid::new(level, strain)))
        });
        [Call::Pass, Call::Double, Call::Redouble]
            .into_iter()
            .chain(bids)
            .filter(|&call| self.auction.can_push(call).is_ok())
            .map(|call| call.to_string())
            .collect()
    }

    fn snapshot(&self) -> Snapshot<'_> {
        let ended = self.auction.has_ended();
        let seat_to_act = self.table.seat_to_act(self.auction.len());

        // Practice shows only the human's hand until the reveal
        let visible = |seat: Seat| ended || self.human.is_none_or(|human| human == seat);
        let hands = Seat::ALL
            .into_iter()
            .filter(|&seat| visible(seat))
            .map(|seat| (seat.letter(), HandJson::new(self.deal[seat])))
            .collect();

        let contract = ended.then(|| match final_contract(&self.auction, self.dealer) {
            Some((contract, declarer)) => format!("{contract} by {}", declarer.letter()),
            None => "Passed out".to_string(),
        });

        Snapshot {
            mode: if self.human.is_some() {
                "practice"
            } else {
                "demo"
            },
            dealer: self.dealer.letter(),
            vul: vul_name(self.vul),
            seat: self.human.map(Seat::letter),
            hands,
            auction: self.auction.iter().map(ToString::to_string).collect(),
            your_turn: !ended && self.human == Some(seat_to_act),
            ended,
            legal: self.legal(),
            contract,
            feedback: &self.feedback,
        }
    }
}

/// Running oracle statistics: the final contract judged over reshuffles of
/// the hands the bidding side never saw
#[derive(Default)]
struct Oracle {
    n: u32,
    makes: u32,
    tricks_sum: u64,
    tricks_min: u8,
    tricks_max: u8,
    score_sum: i64,
}

impl Oracle {
    fn add(&mut self, tricks: u8, makes: bool, human_score: i64) {
        if self.n == 0 {
            self.tricks_min = tricks;
            self.tricks_max = tricks;
        }
        self.n += 1;
        self.makes += u32::from(makes);
        self.tricks_sum += u64::from(tricks);
        self.tricks_min = self.tricks_min.min(tricks);
        self.tricks_max = self.tricks_max.max(tricks);
        self.score_sum += human_score;
    }

    fn stats(&self) -> OracleJson {
        let n = f64::from(self.n.max(1));
        OracleJson {
            n: self.n,
            makes_pct: 100.0 * f64::from(self.makes) / n,
            mean_tricks: self.tricks_sum as f64 / n,
            tricks_min: self.tricks_min,
            tricks_max: self.tricks_max,
            mean_score: self.score_sum as f64 / n,
        }
    }
}

/// Oracle statistics as the UI renders them
#[derive(Serialize)]
struct OracleJson {
    n: u32,
    makes_pct: f64,
    mean_tricks: f64,
    tricks_min: u8,
    tricks_max: u8,
    /// Mean score signed from the human's side
    mean_score: f64,
}

/// Double-dummy table as the UI renders it: rows by strain, columns in
/// `seats` order (west first, matching the auction table)
#[derive(Serialize)]
struct DdJson {
    seats: [char; 4],
    rows: Vec<DdRow>,
    verdict: Option<String>,
}

#[derive(Serialize)]
struct DdRow {
    strain: String,
    /// Tricks per declarer, in `DdJson::seats` order
    tricks: Vec<u8>,
}

/// Browser-sized transposition table (MiB): the native default of 160/256 is
/// a lot to grow a wasm heap by; 64 stays past the sweet spot the solver docs
/// name (16/32 is ~3.5× slower, correctness unaffected at any size).
const TT_MB: (u32, u32) = (64, 128);

/// The vulnerability bit of `seat`'s side
const fn side(seat: Seat) -> AbsoluteVulnerability {
    match seat {
        Seat::North | Seat::South => AbsoluteVulnerability::NS,
        Seat::East | Seat::West => AbsoluteVulnerability::EW,
    }
}

/// Set a [`Builder`] seat by a runtime [`Seat`] value
fn set_seat(builder: Builder, seat: Seat, hand: Hand) -> Builder {
    match seat {
        Seat::North => builder.north(hand),
        Seat::East => builder.east(hand),
        Seat::South => builder.south(hand),
        Seat::West => builder.west(hand),
    }
}

const fn vul_name(vul: AbsoluteVulnerability) -> &'static str {
    match vul.bits() {
        1 => "NS",
        2 => "EW",
        3 => "Both",
        _ => "None",
    }
}

/// A bridge table in the browser: deal, bid, snapshot
#[wasm_bindgen]
pub struct WebTable {
    rng: StdRng,
    board: Option<Board>,
}

#[wasm_bindgen]
impl WebTable {
    /// A fresh table; `seed` is a decimal string from JS (wasm has no entropy)
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(seed: &str) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed.parse().unwrap_or(0)),
            board: None,
        }
    }

    /// Deal a practice board: the human bids `seat`, bots bid the rest
    ///
    /// Unparseable inputs fall back to South / North dealer / no vulnerability.
    pub fn deal_practice(&mut self, seat: &str, dealer: &str, vul: &str, min_hcp: u8) -> String {
        let seat = seat.parse().unwrap_or(Seat::South);
        let deal = self.sample(seat, min_hcp);
        self.deal_with(deal, dealer, vul, Some(seat))
    }

    /// Deal a demo board and let the bots bid it out
    pub fn deal_demo(&mut self, dealer: &str, vul: &str) -> String {
        let deal = full_deal(&mut self.rng);
        self.deal_with(deal, dealer, vul, None)
    }

    /// Bid out a caller-specified deal (from the editor) in demo mode
    ///
    /// `pbn` is the [PBN] deal string the editor emits (`"N:… … … …"`);
    /// returns `"null"` if it does not parse to a full 52-card deal.
    ///
    /// [PBN]: https://www.tistis.nl/pbn/
    pub fn deal_pbn(&mut self, pbn: &str, dealer: &str, vul: &str) -> String {
        match pbn.parse::<FullDeal>() {
            Ok(deal) => self.deal_with(deal, dealer, vul, None),
            Err(_) => "null".to_string(),
        }
    }

    /// The human's call by display code (`"1♥"`, `"P"`, `"X"`, `"XX"`)
    ///
    /// An unparseable or illegal call — or a call out of turn — returns the
    /// snapshot unchanged; the UI prevents these by disabling buttons.
    pub fn bid(&mut self, call: &str) -> String {
        if let Some(board) = &mut self.board
            && !board.auction.has_ended()
            && board.human == Some(board.table.seat_to_act(board.auction.len()))
            && let Ok(call) = call.parse::<Call>()
            && board.auction.can_push(call).is_ok()
        {
            // The bot's opinion must be read before the auction grows
            let top = board.top3();
            let agreed = match top.first() {
                Some((best, _)) => *best == call.to_string(),
                None => call == Call::Pass,
            };
            board.feedback.push(Feedback {
                index: board.auction.len(),
                human: call.to_string(),
                agreed,
                top,
            });
            board.auction.push(call);
            board.advance();
        }
        self.snapshot()
    }

    /// The full double-dummy table of the revealed deal, cached per board
    ///
    /// `"null"` until the auction has ended — the table reads all four
    /// hands, so it exists only once they are on view anyway.  Rows are
    /// strains ♣♦♥♠NT, columns west-first to match the auction table; the
    /// verdict prices the reached contract on the actual layout.
    pub fn dd_table(&mut self) -> String {
        let Some(board) = &mut self.board else {
            return "null".to_string();
        };
        if !board.auction.has_ended() {
            return "null".to_string();
        }

        let solver = board
            .solver
            .get_or_insert_with(|| Solver::with_memory(Strain::Notrump, TT_MB.0, TT_MB.1));
        if board.dd.is_none() {
            board.dd = Some(solve_deal_on(solver, board.deal));
        }
        let table = board.dd.expect("just solved");

        let verdict = final_contract(&board.auction, board.dealer).map(|(contract, declarer)| {
            let tricks = table[contract.bid.strain].get(declarer).get();
            let needed = 6 + contract.bid.level.get();
            let outcome = if tricks >= needed {
                "makes".to_string()
            } else {
                format!("down {}", needed - tricks)
            };
            format!(
                "{contract} by {}: {tricks} tricks — {outcome}",
                declarer.letter()
            )
        });

        const SEAT_COLS: [Seat; 4] = [Seat::West, Seat::North, Seat::East, Seat::South];
        let rows = Strain::ASC
            .into_iter()
            .map(|strain| DdRow {
                strain: strain.to_string(),
                tricks: SEAT_COLS
                    .into_iter()
                    .map(|seat| table[strain].get(seat).get())
                    .collect(),
            })
            .collect();

        let json = DdJson {
            seats: SEAT_COLS.map(Seat::letter),
            rows,
            verdict,
        };
        serde_json::to_string(&json).expect("dd table serialization")
    }

    /// Run `samples` more oracle shuffles and return the running statistics
    ///
    /// The fairness judge for a practice board: the human side's two hands
    /// stay fixed, the opponents' are reshuffled, and the reached contract
    /// is priced double-dummy on each layout — what the contract is worth
    /// on what the bidders could actually know, never the one true layout.
    /// `"null"` unless a practice auction has ended in a contract.
    pub fn oracle(&mut self, samples: u32) -> String {
        let Some(board) = &mut self.board else {
            return "null".to_string();
        };
        let Some(human) = board.human else {
            return "null".to_string();
        };
        if !board.auction.has_ended() {
            return "null".to_string();
        }
        let Some((contract, declarer)) = final_contract(&board.auction, board.dealer) else {
            return "null".to_string();
        };

        let partner = human.partner();
        let partial = set_seat(
            set_seat(Builder::new(), human, board.deal[human]),
            partner,
            board.deal[partner],
        )
        .build_partial()
        .expect("two disjoint 13-card hands form a valid partial deal");

        let strain = contract.bid.strain;
        let solver = board
            .solver
            .get_or_insert_with(|| Solver::with_memory(strain, TT_MB.0, TT_MB.1));
        solver.set_strain(strain);

        let needed = 6 + contract.bid.level.get();
        let declarer_vul = board.vul.contains(side(declarer));
        let human_declaring = side(human) == side(declarer);

        for deal in fill_deals(&mut self.rng, partial).take(samples as usize) {
            let tricks = solver.solve(deal).get(declarer).get();
            let score = i64::from(contract.score(tricks, declarer_vul));
            let human_score = if human_declaring { score } else { -score };
            board.oracle.add(tricks, tricks >= needed, human_score);
        }

        serde_json::to_string(&board.oracle.stats()).expect("oracle serialization")
    }

    /// The current position as JSON (`"null"` before the first deal)
    #[must_use]
    pub fn snapshot(&self) -> String {
        match &self.board {
            Some(board) => {
                serde_json::to_string(&board.snapshot()).expect("snapshot serialization")
            }
            None => "null".to_string(),
        }
    }
}

impl WebTable {
    /// Rejection-sample a deal whose `seat` hand has at least `min_hcp`
    // ponytail: 10 000-attempt cap falls back to the last deal, same as the CLI
    fn sample(&mut self, seat: Seat, min_hcp: u8) -> FullDeal {
        let hcp_eval = SimpleEvaluator(eval::hcp::<u8>);
        let mut candidate = full_deal(&mut self.rng);
        for _ in 1..10_000 {
            if hcp_eval.eval(candidate[seat]) >= min_hcp {
                break;
            }
            candidate = full_deal(&mut self.rng);
        }
        candidate
    }

    /// Seat two 2/1 pairs on `deal` and bid forward to the first decision
    fn deal_with(
        &mut self,
        deal: FullDeal,
        dealer: &str,
        vul: &str,
        human: Option<Seat>,
    ) -> String {
        let dealer = dealer.parse().unwrap_or(Seat::North);
        let vul = vul.parse().unwrap_or(AbsoluteVulnerability::NONE);
        let ns = pons::american();
        let ew = pons::american();
        let mut board = Board {
            table: Table::of_pairs(&ns, &ew, dealer, vul),
            deal,
            dealer,
            vul,
            human,
            auction: Auction::new(),
            feedback: Vec::new(),
            dd: None,
            oracle: Oracle::default(),
            solver: None,
        };
        board.advance();
        self.board = Some(board);
        self.snapshot()
    }
}

/// One authored book node: an auction and its rules, readable
#[derive(Serialize)]
struct NodeJson {
    book: &'static str,
    auction: String,
    rules: Vec<RuleJson>,
    /// Prose for a rule-less entry — a systems-on rebase's summary, or a
    /// computed (non-`Rules`) table's placeholder
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

/// One rule of a node: the call, its weight, and the constraint's own prose
#[derive(Serialize)]
struct RuleJson {
    call: String,
    weight: f32,
    text: String,
    label: &'static str,
}

/// The authored 2/1 books as JSON, for the browser's book tab
///
/// Port of `examples/render-book`: walks the floor-less books and reads each
/// rule's call, weight, and the constraint's own English description, deduping
/// seat variants that share one authored table.
#[wasm_bindgen]
#[must_use]
pub fn book() -> String {
    let pair = bare_american();
    let books: [(&str, &pons::Trie); 3] = [
        ("constructive", &pair.constructive.0),
        ("competitive", &pair.competitive.0),
        ("defensive", &pair.defensive.0),
    ];

    let mut seen: HashSet<(&str, String, usize)> = HashSet::new();
    let mut nodes: Vec<NodeJson> = Vec::new();

    for (book, trie) in books {
        for (auction, classifier) in trie.iter() {
            let Some(rules) = classifier.as_rules() else {
                continue;
            };
            // Dedupe by (book, seat-invariant auction, authored-rules object).
            // Seat variants of one table share an `Arc` under 0–3 leading passes,
            // but the 1NT-overcall graft re-roots that *same* `Arc` below every
            // opening (`(1♣) 1NT`, `(1♦) 1NT`, …); keying on the pointer alone (as
            // `render-book` does) would collapse those distinct advances into one.
            let id = core::ptr::from_ref(classifier) as *const () as usize;
            let heading = match strip_leading_passes(&auction) {
                [] => "(opening)".to_string(),
                canon => display_calls(canon).to_string(),
            };
            if !seen.insert((book, heading.clone(), id)) {
                continue;
            }

            nodes.push(NodeJson {
                book,
                auction: heading,
                rules: rule_json(rules),
                note: None,
            });
        }

        // Guarded fallbacks — the competitive book's whole substance.  The
        // heading folds the guard's description into the auction string (so
        // the text filter sees it); a rebase or computed table renders as a
        // `note`.  Seat variants share one `Arc`: first-seen dedup keeps the
        // canonical pass-less key (`Trie::fallbacks` visits it first).
        for (auction, guard, fallback) in trie.fallbacks() {
            let id = match fallback {
                Fallback::Classify(c) => std::sync::Arc::as_ptr(c).cast::<()>() as usize,
                Fallback::Rebase(r) => std::sync::Arc::as_ptr(r).cast::<()>() as usize,
            };
            let condition = guard
                .describe()
                .unwrap_or_else(|| "(unlabeled guard)".to_string());
            let heading = format!(
                "{} {condition}",
                display_calls(strip_leading_passes(&auction))
            )
            .trim()
            .to_string();
            if !seen.insert((book, heading.clone(), id)) {
                continue;
            }

            let (rules, note) = match fallback {
                Fallback::Classify(classifier) => match classifier.as_rules() {
                    Some(rules) => (rule_json(rules), None),
                    None => (Vec::new(), Some("(computed table)".to_string())),
                },
                Fallback::Rebase(rewrite) => (
                    Vec::new(),
                    Some(format!(
                        "→ {}",
                        rewrite
                            .describe()
                            .unwrap_or_else(|| "(opaque rewrite)".to_string())
                    )),
                ),
            };
            nodes.push(NodeJson {
                book,
                auction: heading,
                rules,
                note,
            });
        }
    }

    serde_json::to_string(&nodes).expect("book serialization")
}

/// The auction with leading passes dropped — the seat-invariant dedup key
///
/// Seat variants of one table are installed under 0–3 leading passes; stripping
/// them collapses those variants while keeping genuinely distinct auctions apart,
/// notably the 1NT-overcall systems-on graft re-rooted below each opening
/// (`(1♦) 1NT` vs `(1♠) 1NT` share the grafted `Arc` but differ here).
fn strip_leading_passes(auction: &[Call]) -> &[Call] {
    let lead = auction.iter().take_while(|&&c| c == Call::Pass).count();
    &auction[lead..]
}

/// The readable form of a node's rules (shared by exact and guarded entries)
fn rule_json(rules: &pons::bidding::Rules) -> Vec<RuleJson> {
    rules
        .rules()
        .iter()
        .map(|rule| RuleJson {
            call: rule.call().to_string(),
            weight: rule.weight(),
            text: rule.describe().to_string(),
            label: rule.label(),
        })
        .collect()
}

/// The Settings-tab registry: one row per user-facing bidding knob
///
/// This table is the **single source of truth** for the Settings tab.
/// [`set_option`] / [`set_choice`] dispatch a call through it and
/// [`describe_options`] serialises it for the JS renderer, so adding a convention
/// to the UI needs only one row here (plus the engine `set_*` it points at) — the
/// old hand-synced JS `CURATED` / `MORE` arrays are gone.  Each `set_*` is a
/// module-level thread-local flag read when a deal rebuilds `american()` in
/// `deal_with`; wasm is single-threaded, so the thread-local is effectively a global.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Setting {
    /// A boolean checkbox.
    Toggle {
        key: &'static str,
        section: &'static str,
        /// Display label, or `""` to humanise the key in JS.
        label: &'static str,
        default: bool,
        #[serde(skip)]
        set: fn(bool),
    },
    /// A mutually-exclusive family, rendered as radio buttons.  Exactly one variant
    /// is active; the engine backs it with a single enum (e.g. [`NotrumpDefense`]).
    ///
    /// [`NotrumpDefense`]: pons::bidding::american::NotrumpDefense
    Choice {
        key: &'static str,
        section: &'static str,
        label: &'static str,
        variants: &'static [Variant],
        /// The `value` of the default variant.
        default: &'static str,
        #[serde(skip)]
        set: fn(&str),
    },
}

/// One radio option of a [`Setting::Choice`].
#[derive(Serialize)]
struct Variant {
    value: &'static str,
    label: &'static str,
}

impl Setting {
    const fn key(&self) -> &'static str {
        match self {
            Setting::Toggle { key, .. } | Setting::Choice { key, .. } => key,
        }
    }
}

/// Terser constructor for the common [`Setting::Toggle`] row.
const fn toggle(
    key: &'static str,
    section: &'static str,
    label: &'static str,
    default: bool,
    set: fn(bool),
) -> Setting {
    Setting::Toggle {
        key,
        section,
        label,
        default,
        set,
    }
}

// Section names; the tab shows them in first-appearance order.
const OPENINGS: &str = "Openings";
const NOTRUMP: &str = "Notrump";
const COMPETITION: &str = "Competition";
const DEFENSE: &str = "Defense to their 1NT";
const REBIDS: &str = "Rebids & responses";
const FLOOR: &str = "Floor (instinct)";
const INFERENCE: &str = "Inference (auction reading)";
const FUZZING: &str = "Fuzzing (hand evaluation)";

/// The `[1NT]` defense family — variants map onto `american::NotrumpDefense`.
static NOTRUMP_DEFENSE_VARIANTS: &[Variant] = &[
    Variant {
        value: "natural",
        label: "Natural",
    },
    Variant {
        value: "direct_dont",
        label: "DONT",
    },
    Variant {
        value: "direct_landy",
        label: "Landy double",
    },
    Variant {
        value: "woolsey",
        label: "Woolsey",
    },
    Variant {
        value: "always_pass",
        label: "Always pass",
    },
];

/// Select the mutually-exclusive 1NT defense from its registry `value`.
fn set_notrump_defense_choice(value: &str) {
    use american::NotrumpDefense;
    // DirectLandy carries a shape flag; select the measured-winning 5-4 form.
    if value == "direct_landy" {
        american::set_direct_landy_double(Some(false));
        return;
    }
    american::set_notrump_defense(match value {
        "direct_dont" => NotrumpDefense::DirectDont,
        "woolsey" => NotrumpDefense::Woolsey,
        "always_pass" => NotrumpDefense::AlwaysPass,
        _ => NotrumpDefense::Natural,
    });
}

/// The 1NT opening shape family — variants map onto `american::NotrumpShape`.
/// Each widens the one before it: balanced only, then also a 5422 with a
/// five-card minor, then also a 6322 with a six-card minor (the shipped default).
static NOTRUMP_SHAPE_VARIANTS: &[Variant] = &[
    Variant {
        value: "balanced",
        label: "Balanced only",
    },
    Variant {
        value: "wide",
        label: "Also 5-card minor (5422)",
    },
    Variant {
        value: "wide6322",
        label: "Also 6-card minor (6322)",
    },
];

/// Select the 1NT opening shape from its registry `value`.
fn set_notrump_shape_choice(value: &str) {
    use american::NotrumpShape;
    american::set_notrump_shape(match value {
        "balanced" => NotrumpShape::Balanced,
        "wide" => NotrumpShape::Wide,
        _ => NotrumpShape::Wide6322,
    });
}

/// The negative-double school over their overcall — variants map onto
/// `american::NegativeDoubleShape`. Only the three shipped-or-playable schools
/// surface; the pre-Modern `BothMajors` rule is not offered.
static NEGATIVE_DOUBLE_VARIANTS: &[Variant] = &[
    Variant {
        value: "modern",
        label: "Modern",
    },
    Variant {
        value: "sputnik",
        label: "Sputnik",
    },
    Variant {
        value: "cachalot",
        label: "Cachalot",
    },
];

/// Select the negative-double school from its registry `value`.
fn set_negative_double_choice(value: &str) {
    use american::NegativeDoubleShape;
    american::set_negative_double_shape(match value {
        "sputnik" => NegativeDoubleShape::Sputnik,
        "cachalot" => NegativeDoubleShape::Cachalot,
        _ => NegativeDoubleShape::Modern,
    });
}

/// Lebensohl as an on/off toggle: on = Transfer Lebensohl (the default package, not
/// the `set_lebensohl` wrapper's lossy `Plain`), off = none.
fn set_lebensohl_toggle(on: bool) {
    use american::LebensohlStyle;
    american::set_lebensohl_style(if on {
        LebensohlStyle::Transfer
    } else {
        LebensohlStyle::Off
    });
}

/// Advancer's Lebensohl (after partner's takeout double is overcalled) as an on/off
/// toggle: on = Transfer Lebensohl (the shipped default), off = none.
fn set_advance_sohl_toggle(on: bool) {
    use american::LebensohlStyle;
    american::set_advance_sohl_style(if on {
        LebensohlStyle::Transfer
    } else {
        LebensohlStyle::Off
    });
}

/// Puppet Stayman as an on/off toggle: on = Puppet (the shipped default, 3♣ Puppet
/// Stayman), off = European transfers (2♠ club transfer, 2NT natural, 3♣ diamond).
fn set_puppet_stayman(on: bool) {
    american::set_notrump_minors(if on {
        american::PUPPET
    } else {
        american::EUROPEAN
    });
}

/// The registry.  Each `default` mirrors its engine `Cell::new(...)` — keep the two
/// in sync by hand when a knob's default changes (there is no automatic guard).
///
/// `rustfmt::skip` keeps every row on one line — rustfmt otherwise explodes each
/// `toggle(...)` whose call exceeds the width into a seven-line block; the table
/// reads far better one-setting-per-line.  Keep new rows one line each.
#[rustfmt::skip]
static SETTINGS: &[Setting] = &[
    // Openings
    toggle("open_one_notrump", OPENINGS, "Open 1NT (15–17)", true, american::set_open_one_notrump),
    Setting::Choice { key: "notrump_shape", section: OPENINGS, label: "1NT opening shape", variants: NOTRUMP_SHAPE_VARIANTS, default: "wide6322", set: set_notrump_shape_choice },
    // Notrump
    toggle("puppet_stayman", NOTRUMP, "Puppet Stayman (3♣)", true, set_puppet_stayman),
    toggle("garbage_stayman", NOTRUMP, "Garbage Stayman", true, american::set_garbage_stayman),
    toggle("transfer_super_accept", NOTRUMP, "", false, american::set_transfer_super_accept),
    toggle("transfer_slam_try", NOTRUMP, "", true, american::set_transfer_slam_try),
    toggle("texas_slam_drive", NOTRUMP, "", true, american::set_texas_slam_drive),
    toggle("transfer_gf_majors", NOTRUMP, "", true, american::set_transfer_gf_majors),
    toggle("transfer_gf_hearts", NOTRUMP, "", true, american::set_transfer_gf_hearts),
    toggle("stayman_both_majors", NOTRUMP, "", true, american::set_stayman_both_majors),
    toggle("stayman_5card_max", NOTRUMP, "", true, american::set_stayman_5card_max),
    toggle("invitational_5card_majors", NOTRUMP, "", true, american::set_invitational_5card_majors),
    toggle("transfer_longer_major", NOTRUMP, "", true, american::set_transfer_longer_major),
    toggle("crawling_stayman", NOTRUMP, "", true, american::set_crawling_stayman),
    toggle("stayman_cue_continuation", NOTRUMP, "", true, american::set_stayman_cue_continuation),
    toggle("stayman_minor_slam_try", NOTRUMP, "", true, american::set_stayman_minor_slam_try),
    // Competition
    toggle("lebensohl", COMPETITION, "Lebensohl (over 1NT interference)", true, set_lebensohl_toggle),
    toggle("advance_lebensohl", COMPETITION, "Lebensohl advancing a double", true, set_advance_sohl_toggle),
    toggle("splinter_doubled", COMPETITION, "", true, american::set_splinter_doubled),
    toggle("passed_hand_overcall", COMPETITION, "", false, american::set_passed_hand_overcall),
    toggle("uvu", COMPETITION, "Unusual vs Unusual", true, american::set_uvu),
    toggle("uvu_over_majors", COMPETITION, "Unusual vs Unusual (over majors)", true, american::set_uvu_over_majors),
    toggle("direct_3nt_stopper", COMPETITION, "", true, american::set_direct_3nt_stopper),
    toggle("cue_raise_answer", COMPETITION, "", true, american::set_cue_raise_answer),
    toggle("cue_minor_raise_answer", COMPETITION, "", true, american::set_cue_minor_raise_answer),
    toggle("major_support_double", COMPETITION, "", true, american::set_major_support_double),
    toggle("high_overcall_responses", COMPETITION, "", false, american::set_high_overcall_responses),
    toggle("jordan_truscott", COMPETITION, "Jordan / Truscott 2NT", true, american::set_jordan_truscott),
    toggle("delayed_cue", COMPETITION, "", false, american::set_delayed_cue),
    toggle("competition_over_stayman", COMPETITION, "", true, american::set_competition_over_stayman),
    toggle("competition_over_minor_transfer", COMPETITION, "", true, american::set_competition_over_minor_transfer),
    toggle("competition_over_diamond_transfer", COMPETITION, "", true, american::set_competition_over_diamond_transfer),
    toggle("defense_to_2d_multi", COMPETITION, "", false, american::set_defense_to_2d_multi),
    toggle("leaping_michaels", COMPETITION, "Leaping Michaels", true, american::set_leaping_michaels),
    toggle("responsive_takeout", COMPETITION, "Responsive doubles", true, american::set_responsive_takeout),
    toggle("rich_advance_double", COMPETITION, "", false, american::set_rich_advance_double),
    toggle("advance_rubens", COMPETITION, "Rubens advances", false, american::set_advance_rubens),
    toggle("nt_overcall_gladiator", COMPETITION, "Gladiator (1NT-overcall advance)", false, american::set_nt_overcall_gladiator),
    // Negative-double school over their overcall — the enum-backed radio family
    Setting::Choice {
        key: "negative_double_shape",
        section: COMPETITION,
        label: "Negative double (over their overcall)",
        variants: NEGATIVE_DOUBLE_VARIANTS,
        default: "modern",
        set: set_negative_double_choice,
    },
    // Defense to their 1NT — the radio family is the enum-backed choice
    Setting::Choice {
        key: "notrump_defense",
        section: DEFENSE,
        label: "Defense system",
        variants: NOTRUMP_DEFENSE_VARIANTS,
        default: "natural",
        set: set_notrump_defense_choice,
    },
    toggle("direct_dont_four_four", DEFENSE, "", true, american::set_direct_dont_four_four),
    toggle("stayman_defense", DEFENSE, "", false, american::set_stayman_defense),
    toggle("transfer_defense", DEFENSE, "", false, american::set_transfer_defense),
    toggle("minor_transfer_defense", DEFENSE, "", false, american::set_minor_transfer_defense),
    // Rebids & responses
    toggle("second_suit_agreement", REBIDS, "", true, american::set_second_suit_agreement),
    toggle("fourth_suit_forcing", REBIDS, "Fourth suit forcing", true, american::set_fourth_suit_forcing),
    toggle("meckstroth_adjunct", REBIDS, "Meckstroth adjunct", true, american::set_meckstroth_adjunct),
    toggle("limit_raise_acceptance", REBIDS, "", true, american::set_limit_raise_acceptance),
    // Floor (instinct)
    toggle("inference_aware", FLOOR, "", true, instinct::set_inference_aware),
    toggle("one_nt_runout", FLOOR, "", true, instinct::set_one_nt_runout),
    toggle("one_nt_runout_universal", FLOOR, "", true, instinct::set_one_nt_runout_universal),
    toggle("settle_floor", FLOOR, "", true, instinct::set_settle_floor),
    toggle("rubens_advances", FLOOR, "", true, instinct::set_rubens_advances),
    toggle("floor_rkcb", FLOOR, "", true, instinct::set_floor_rkcb),
    toggle("penalize_escape_stack", FLOOR, "", true, instinct::set_penalize_escape_stack),
    toggle("penalize_escape_values", FLOOR, "", true, instinct::set_penalize_escape_values),
    toggle("uvu_encircle", FLOOR, "UVU penalty procedure", true, instinct::set_uvu_encircle),
    toggle("penalty_latch", FLOOR, "", true, instinct::set_penalty_latch),
    toggle("penalty_no_pull", FLOOR, "", true, instinct::set_penalty_no_pull),
    toggle("advancer_xx_runout", FLOOR, "", true, instinct::set_advancer_xx_runout),
    toggle("doubler_xx_runout", FLOOR, "", true, instinct::set_doubler_xx_runout),
    // Inference (auction reading)
    toggle("nt_invite_inference", INFERENCE, "", true, inference::set_nt_invite_inference),
    toggle("rubens_transfer_reading", INFERENCE, "", true, inference::set_rubens_transfer_reading),
    toggle("alert_reading", INFERENCE, "", true, inference::set_alert_reading),
    toggle("fallback_projection", INFERENCE, "", true, inference::set_fallback_projection),
    toggle("control_bid_reading", INFERENCE, "", true, inference::set_control_bid_reading),
    toggle("rule_accept", INFERENCE, "", false, inference::set_rule_accept),
    // Fuzzing (hand evaluation)
    toggle("fuzzy_strength", FUZZING, "Fuzzy hand strength", true, constraint::set_fuzzy_strength),
    toggle("fuzzy_points", FUZZING, "", true, constraint::set_fuzzy_points),
    toggle("fuzzy_fifths", FUZZING, "", true, constraint::set_fuzzy_fifths),
];

/// Flip a boolean bidding knob for the **next** deal (the Settings tab).  Unknown
/// keys are a no-op.
#[wasm_bindgen]
pub fn set_option(key: &str, on: bool) {
    if let Some(Setting::Toggle { set, .. }) = SETTINGS.iter().find(|s| s.key() == key) {
        set(on);
    }
}

/// Select a variant of a mutually-exclusive choice (a radio family, e.g. defense to
/// their 1NT) for the **next** deal.  Unknown keys are a no-op.
#[wasm_bindgen]
pub fn set_choice(key: &str, value: &str) {
    if let Some(Setting::Choice { set, .. }) = SETTINGS.iter().find(|s| s.key() == key) {
        set(value);
    }
}

/// The Settings registry as JSON, for the JS renderer to build the tab from
///
/// Each entry carries `{kind, key, section, label, default, variants?}` — `kind`
/// is the internal tag, `"toggle"` (boolean `default`) or `"choice"` (string
/// `default` + a `variants` array).  An empty `label` means "humanise the key in
/// JS".  The renderer and the round-trip test read fields by name, so key order
/// is immaterial.
#[wasm_bindgen]
pub fn describe_options() -> String {
    serde_json::to_string(SETTINGS).expect("settings registry serialises")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(snapshot: &str) -> serde_json::Value {
        serde_json::from_str(snapshot).expect("snapshot is valid JSON")
    }

    #[test]
    fn practice_board_runs_to_completion() {
        let mut table = WebTable::new("12345");
        let mut snap = parse(&table.deal_practice("S", "N", "none", 0));
        assert_eq!(snap["mode"], "practice");
        assert_eq!(snap["seat"], "S");

        let mut human_calls = 0;
        while snap["your_turn"] == true {
            let legal = snap["legal"].as_array().expect("legal is an array");
            assert!(!legal.is_empty(), "legal calls before the auction ends");
            for code in legal {
                let code = code.as_str().expect("legal codes are strings");
                assert!(code.parse::<Call>().is_ok(), "code {code} must re-parse");
            }
            assert_eq!(snap["hands"].as_object().expect("hands").len(), 1);
            snap = parse(&table.bid("P"));
            human_calls += 1;
            assert!(human_calls < 100, "auction must terminate");
        }

        assert_eq!(snap["ended"], true);
        assert!(snap["contract"].is_string());
        assert_eq!(snap["hands"].as_object().expect("hands").len(), 4);
        assert_eq!(
            snap["feedback"].as_array().expect("feedback").len(),
            human_calls,
        );
    }

    #[test]
    fn illegal_and_out_of_turn_bids_are_ignored() {
        let mut table = WebTable::new("7");
        let before = table.deal_practice("S", "S", "ns", 0);
        assert_eq!(table.bid("8♣"), before, "unparseable call is a no-op");
        assert_eq!(table.bid("XX"), before, "illegal call is a no-op");
    }

    #[test]
    fn set_option_reroutes_the_bidding() {
        // North is a balanced 15 — opens 1NT by default, a suit with 1NT off.
        const PBN: &str = "N:AK72.K65.K43.Q82 QJT.AQJ.AQJ.AKJT 986.T987.T98.976 543.432.7652.543";
        let mut table = WebTable::new("1");

        let on = parse(&table.deal_pbn(PBN, "N", "none"));
        assert!(
            on["auction"][0]
                .as_str()
                .expect("opening call")
                .contains('N'),
            "default opens 1NT",
        );

        set_option("open_one_notrump", false);
        let off = parse(&table.deal_pbn(PBN, "N", "none"));
        assert_ne!(
            on["auction"][0], off["auction"][0],
            "toggling the knob changes North's opening",
        );

        set_option("open_one_notrump", true); // restore for a reused test thread
    }

    #[test]
    fn registry_is_well_formed() {
        use std::collections::HashSet;
        // Unique keys — a dup would shadow in the linear find and confuse the UI.
        let mut keys = HashSet::new();
        for setting in SETTINGS {
            assert!(
                keys.insert(setting.key()),
                "duplicate registry key: {}",
                setting.key()
            );
        }
        // describe_options round-trips and matches the table shape one-for-one.
        let json = parse(&describe_options());
        let entries = json.as_array().expect("registry is a JSON array");
        assert_eq!(
            entries.len(),
            SETTINGS.len(),
            "one JSON entry per registry row"
        );
        for entry in entries {
            assert!(entry["key"].is_string() && entry["section"].is_string());
            match entry["kind"].as_str().expect("kind is a string") {
                "toggle" => assert!(entry["default"].is_boolean(), "toggle default is a bool"),
                "choice" => {
                    let default = entry["default"]
                        .as_str()
                        .expect("choice default is a string");
                    let values: Vec<&str> = entry["variants"]
                        .as_array()
                        .expect("choice has variants")
                        .iter()
                        .map(|v| v["value"].as_str().expect("variant value"))
                        .collect();
                    assert!(
                        values.contains(&default),
                        "choice default {default} is a variant"
                    );
                }
                other => panic!("unknown kind {other}"),
            }
        }
    }

    #[test]
    fn set_choice_reroutes_the_defense() {
        // North opens 1NT; East (19 HCP) acts over it — doubles under the natural
        // defense, passes under always-pass.  Selecting the family through set_choice
        // must change East's action.
        const PBN: &str = "N:AK72.K65.K43.Q82 QJT.AQJ.AQJ.AKJT 986.T987.T98.976 543.432.7652.543";
        let mut table = WebTable::new("1");

        set_choice("notrump_defense", "natural");
        let natural = parse(&table.deal_pbn(PBN, "N", "none"));

        set_choice("notrump_defense", "always_pass");
        let always_pass = parse(&table.deal_pbn(PBN, "N", "none"));

        assert_ne!(
            natural["auction"], always_pass["auction"],
            "always-pass defense changes East's action over North's 1NT",
        );

        set_choice("notrump_defense", "natural"); // restore for a reused test thread
    }

    #[test]
    fn set_choice_reroutes_the_notrump_shape() {
        // North is a 16-HCP 6322 with six clubs: opens 1NT under the default
        // wide6322 shape, its minor under balanced-only.  Selecting the family
        // through set_choice must change North's opening.
        const PBN: &str = "N:Q2.K3.AQ4.KQ8765 AKJT9.AQJ.KJT.A9 876.T987.987.JT4 543.6542.6532.32";
        let mut table = WebTable::new("1");

        set_choice("notrump_shape", "wide6322");
        let wide = parse(&table.deal_pbn(PBN, "N", "none"));
        assert!(
            wide["auction"][0]
                .as_str()
                .expect("opening call")
                .contains('N'),
            "wide6322 opens 1NT on a 6322 with a six-card minor",
        );

        set_choice("notrump_shape", "balanced");
        let balanced = parse(&table.deal_pbn(PBN, "N", "none"));
        assert_ne!(
            wide["auction"][0], balanced["auction"][0],
            "balanced-only shape opens a minor, not 1NT",
        );

        set_choice("notrump_shape", "wide6322"); // restore for a reused test thread
    }

    #[test]
    fn demo_board_bids_out() {
        let mut table = WebTable::new("42");
        let snap = parse(&table.deal_demo("W", "both"));
        assert_eq!(snap["mode"], "demo");
        assert_eq!(snap["vul"], "Both");
        assert_eq!(snap["ended"], true);
        assert_eq!(snap["your_turn"], false);
        assert_eq!(snap["hands"].as_object().expect("hands").len(), 4);
        assert!(snap["auction"].as_array().expect("auction").len() >= 4);
        assert!(snap["contract"].is_string());
    }

    #[test]
    fn deal_pbn_bids_out_a_specified_deal() {
        let mut table = WebTable::new("1");
        // A full deal round-trips through the editor's canonical "N:…" form.
        let pbn = "N:AKT86.4.AJ962.K3 Q9432.KQJ8..AQT8 7.AT3.QT753.J764 J5.97652.K84.952";
        let snap = parse(&table.deal_pbn(pbn, "N", "none"));
        assert_eq!(snap["mode"], "demo");
        assert_eq!(snap["ended"], true, "bots bid the specified deal out");
        assert_eq!(snap["hands"].as_object().expect("hands").len(), 4);
        // The North hand is the one we asked for, not a random deal.
        assert_eq!(snap["hands"]["N"]["spades"], "AKT86");
        assert_eq!(snap["hands"]["E"]["diamonds"], "", "East's diamond void");

        assert_eq!(table.deal_pbn("garbage", "N", "none"), "null");
        assert_eq!(
            table.deal_pbn(
                "N:AK.4.AJ962.K3 Q9432.KQJ8..AQT8 7.AT3.QT753.J764 J5.97652.K84.952",
                "N",
                "none"
            ),
            "null",
            "a non-full deal is rejected",
        );
    }

    #[test]
    fn dd_table_solves_revealed_demo_board() {
        let mut table = WebTable::new("42");
        assert_eq!(table.dd_table(), "null", "no board yet");
        let _ = table.deal_demo("N", "none");

        let start = std::time::Instant::now();
        let dd: serde_json::Value = serde_json::from_str(&table.dd_table()).expect("dd JSON");
        eprintln!("dd_table (full 5x4, cold): {:?}", start.elapsed());

        assert_eq!(dd["seats"], serde_json::json!(["W", "N", "E", "S"]));
        let rows = dd["rows"].as_array().expect("rows");
        assert_eq!(rows.len(), 5);
        for row in rows {
            let tricks = row["tricks"].as_array().expect("tricks");
            assert_eq!(tricks.len(), 4);
            assert!(tricks.iter().all(|t| t.as_u64().expect("u8") <= 13));
        }
        // Cached: the second call is the same JSON, instantly
        let again: serde_json::Value =
            serde_json::from_str(&table.dd_table()).expect("cached dd JSON");
        assert_eq!(dd, again);
    }

    #[test]
    fn oracle_accumulates_over_reshuffles() {
        // Seeded so the practice board (human passing throughout) ends in a
        // bot contract: seed 12345 ends in 2NT by N (see the test above).
        let mut table = WebTable::new("12345");
        let mut snap = parse(&table.deal_practice("S", "N", "none", 0));
        for _ in 0..100 {
            if snap["your_turn"] != true {
                break;
            }
            snap = parse(&table.bid("P"));
        }
        assert_eq!(snap["ended"], true);
        if !snap["contract"].is_string() || snap["contract"] == "Passed out" {
            panic!("seed no longer yields a contract; pick a new seed");
        }

        let start = std::time::Instant::now();
        let o: serde_json::Value = serde_json::from_str(&table.oracle(5)).expect("oracle JSON");
        eprintln!("oracle (5 shuffles, 1 strain): {:?}", start.elapsed());

        assert_eq!(o["n"], 5);
        let o2: serde_json::Value = serde_json::from_str(&table.oracle(5)).expect("oracle JSON");
        assert_eq!(o2["n"], 10, "stats accumulate across chunks");
        let pct = o2["makes_pct"].as_f64().expect("pct");
        assert!((0.0..=100.0).contains(&pct));
        assert!(o2["tricks_min"].as_u64() <= o2["tricks_max"].as_u64());
    }

    #[test]
    fn oracle_is_practice_only() {
        let mut table = WebTable::new("42");
        let _ = table.deal_demo("N", "none");
        assert_eq!(
            table.oracle(1),
            "null",
            "demo has no bidding side to be fair to"
        );
    }

    #[test]
    fn book_is_json_with_described_nodes() {
        let nodes: serde_json::Value = serde_json::from_str(&book()).expect("book is valid JSON");
        let nodes = nodes.as_array().expect("book is an array");
        assert!(
            nodes.len() > 100,
            "expected >100 nodes, got {}",
            nodes.len()
        );
        for node in nodes {
            assert!(
                !node["rules"].as_array().expect("rules").is_empty() || node["note"].is_string(),
                "every node has rules or a note: {node}",
            );
        }
    }

    /// The 1NT-overcall systems-on graft renders under **every** opening — not
    /// just the one that wins the pointer dedup.  Each `(1x) 1NT` re-roots the
    /// same grafted `Arc`s, so a book display keyed on the pointer alone showed
    /// only spades; the seat-invariant-auction key restores all four.
    #[test]
    fn book_renders_1nt_overcall_advances_per_opening() {
        let nodes: serde_json::Value = serde_json::from_str(&book()).expect("book is valid JSON");
        let nodes = nodes.as_array().expect("book is an array");
        for opening in ["1♣", "1♦", "1♥", "1♠"] {
            // The advancer's response menu after their opening, our 1NT overcall,
            // RHO pass: "1x 1NT -" (Pass renders as "-").
            let heading = format!("{opening} 1NT -");
            assert!(
                nodes
                    .iter()
                    .any(|node| { node["book"] == "defensive" && node["auction"] == heading }),
                "systems-on advance node {heading:?} must render",
            );
        }
    }

    /// The competitive book renders: guarded fallbacks surface as entries with
    /// the guard's condition folded into the auction heading.
    #[test]
    fn book_renders_the_competitive_fallbacks() {
        let nodes: serde_json::Value = serde_json::from_str(&book()).expect("book is valid JSON");
        let competitive: Vec<&serde_json::Value> = nodes
            .as_array()
            .expect("book is an array")
            .iter()
            .filter(|node| node["book"] == "competitive")
            .collect();
        assert!(
            competitive.len() > 30,
            "expected >30 competitive entries, got {}",
            competitive.len()
        );
        assert!(
            competitive
                .iter()
                .any(|node| node["auction"].as_str().expect("auction").contains("≤2♠")),
            "the direct-seat overcall package renders with its ceiling"
        );
        assert!(
            competitive.iter().any(|node| matches!(
                node["note"].as_str(),
                Some(note) if note.contains("systems on")
            )),
            "a systems-on rebase renders as a note"
        );
    }
}
