//! Wasm bindings for the pons web UI
//!
//! One exported [`WebTable`] drives both interactive modes — practice (a human
//! bids one seat against three bots) and demo (bots bid all four) — and a free
//! [`book`] function exports the authored 2/1 books for the browser.  Every
//! method returns a JSON [`Snapshot`] string; the JS side is a thin renderer.
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
use pons::bidding::{Stance, Table};
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
            let tricks = table.get(contract.bid.strain, declarer);
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
                    .map(|seat| table.get(strain, seat))
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
            let tricks = solver.solve(deal)[declarer as usize];
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

    let mut seen: HashSet<usize> = HashSet::new();
    let mut nodes: Vec<NodeJson> = Vec::new();

    for (book, trie) in books {
        for (auction, classifier) in trie.iter() {
            let Some(rules) = classifier.as_rules() else {
                continue;
            };
            // Dedupe by the authored-rules object: shared seat variants of one
            // table classify through the same `Arc` (see `render-book`).
            let id = core::ptr::from_ref(classifier) as *const () as usize;
            if !seen.insert(id) {
                continue;
            }

            nodes.push(NodeJson {
                book,
                auction: if auction.is_empty() {
                    "(opening)".to_string()
                } else {
                    display_calls(&auction).to_string()
                },
                rules: rules
                    .rules()
                    .iter()
                    .map(|rule| RuleJson {
                        call: rule.call().to_string(),
                        weight: rule.weight(),
                        text: rule.describe().to_string(),
                        label: rule.label(),
                    })
                    .collect(),
            });
        }
    }

    serde_json::to_string(&nodes).expect("book serialization")
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
                !node["rules"].as_array().expect("rules").is_empty(),
                "every node has rules",
            );
        }
    }
}
