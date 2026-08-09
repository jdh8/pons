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
use contract_bridge::deal::PartialDeal;
use contract_bridge::deck::{fill_deals, full_deal};
use contract_bridge::eval::{self, HandEvaluator as _, SimpleEvaluator};
use contract_bridge::{
    AbsoluteVulnerability, Bid, Builder, Contract, FullDeal, Hand, Seat, Strain,
};
use pons::bidding::agreements::Agreements;
use pons::bidding::american::american_book;
use pons::bidding::evaluator::trick_estimates;
use pons::bidding::fallback::Fallback;
use pons::bidding::{Relative, Stance, Table, american, inference, instinct};
use pons::scoring::{final_contract, imps};
use pons_dds::{Par, Solver, TrickCountTable, Vulnerability, calculate_par, solve_deal_on};
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

/// One strain's estimate for our side, in tricks
#[derive(Serialize)]
struct HintRow {
    strain: String,
    mean: f32,
    sd: f32,
}

impl Board {
    /// Price every strain for the side to act, off what the auction has shown
    ///
    /// Declarer is whichever of us the net rates higher — the pair picks the
    /// better hand to play it, so the useful number is the max, not our own.
    fn hint(&self) -> Vec<HintRow> {
        let seat = self.table.seat_to_act(self.auction.len());
        let estimates = trick_estimates(self.deal[seat], &self.table.infer(&self.auction));

        Strain::ASC
            .iter()
            .map(|&strain| {
                let ours = [Relative::Me, Relative::Partner]
                    .map(|declarer| estimates.get(strain, declarer));
                // Pick by mean, then report *that* seat's sd — taking the max of
                // each column separately would invent a pairing neither seat has.
                let best = if ours[0].mean >= ours[1].mean {
                    ours[0]
                } else {
                    ours[1]
                };
                HintRow {
                    strain: strain.to_string(),
                    mean: best.mean,
                    sd: best.sd,
                }
            })
            .collect()
    }

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
    /// The score-aware verdict, one string per line (Result / Par / IMP)
    verdict: Option<Vec<String>>,
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

/// The two-letter name of `seat`'s side
const fn side_name(seat: Seat) -> &'static str {
    match seat {
        Seat::North | Seat::South => "NS",
        Seat::East | Seat::West => "EW",
    }
}

/// An NS-signed score printed as `{points} to {side}` (the magnitude sits on
/// the side it favors)
fn to_side(ns_score: i64) -> String {
    if ns_score >= 0 {
        format!("{ns_score} to NS")
    } else {
        format!("{} to EW", -ns_score)
    }
}

/// The over/undertrick suffix of a result: `=`, `+n`, or `-n`
fn result_suffix(diff: i32) -> String {
    match diff {
        0 => "=".to_string(),
        n if n > 0 => format!("+{n}"),
        n => n.to_string(), // negative already carries its own '-'
    }
}

/// The score-aware DD verdict — Result, Par, and IMPs-vs-par lines, every score
/// signed for North-South.
///
/// `reached`/`tricks` are [`Some`] together for a played contract and [`None`]
/// together for a pass-out; par is defined for any deal, so the verdict shows
/// even when the auction passed out (surfacing a makeable game the field let by).
fn verdict_lines(
    reached: Option<(Contract, Seat)>,
    tricks: Option<u8>,
    par: &Par,
    vul: AbsoluteVulnerability,
) -> Vec<String> {
    // Result line + the NS-signed score of the reached contract (0 = passed out).
    let (result, reached_ns) = match (reached, tricks) {
        (Some((contract, declarer)), Some(tricks)) => {
            let needed = 6 + i32::from(contract.bid.level.get());
            let declarer_vul = vul.contains(side(declarer));
            let score = i64::from(contract.score(tricks, declarer_vul));
            let ns = match declarer {
                Seat::North | Seat::South => score,
                Seat::East | Seat::West => -score,
            };
            let line = format!(
                "Result: {} — {contract}{}{}",
                to_side(ns),
                declarer.letter(),
                result_suffix(i32::from(tricks) - needed),
            );
            (line, ns)
        }
        _ => ("Result: Passed out".to_string(), 0),
    };

    // Par line: the par score plus its contracts, deduped by side (a pair
    // expands to both declarers, which collapse to one "NS"/"EW" string).
    let par_line = if par.contracts.is_empty() {
        "Par: Passed out".to_string()
    } else {
        let mut names: Vec<String> = Vec::new();
        for pc in &par.contracts {
            let name = format!(
                "{}{}{}",
                pc.contract,
                side_name(pc.declarer),
                result_suffix(i32::from(pc.overtricks)),
            );
            if !names.contains(&name) {
                names.push(name);
            }
        }
        format!(
            "Par: {} — {}",
            to_side(i64::from(par.score)),
            names.join(", ")
        )
    };

    // IMPs vs par: name a side only when it actually scores.
    let diff = reached_ns - i64::from(par.score);
    let n = imps(diff);
    let imp_line = if n == 0 {
        "0 IMP".to_string()
    } else {
        format!("{} IMP to {}", n.abs(), if diff > 0 { "NS" } else { "EW" })
    };

    vec![result, par_line, imp_line]
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

        // The auction has ended (guarded above), so the verdict always shows —
        // par is defined even for a pass-out.
        let par = calculate_par(
            table,
            Vulnerability::from_bits_truncate(board.vul.bits()),
            board.dealer,
        );
        let reached = final_contract(&board.auction, board.dealer);
        let tricks = reached.map(|(c, d)| table[c.bid.strain].get(d).get());
        let verdict = Some(verdict_lines(reached, tricks, &par, board.vul));

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

    /// The evaluator net's trick estimate for our side, read off the auction
    ///
    /// `null` unless someone is to act on a live auction.  This is the net on
    /// its *training* input — inferences a real auction produced — so unlike a
    /// synthetic maximum-information envelope it carries no distribution
    /// caveat.  Watch `sd` shrink as partner describes their hand: a call that
    /// fails to narrow it is a reading bug, visible without a probe.
    #[must_use]
    pub fn hint(&self) -> String {
        let Some(board) = &self.board else {
            return "null".to_string();
        };
        if board.auction.has_ended() {
            return "null".to_string();
        }
        serde_json::to_string(&board.hint()).expect("hint serialization")
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
        let ns = pons::american(&agreements());
        let ew = pons::american(&agreements());
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
    /// Soft priority in centinats, as the engine stores it; the JS divides by 100
    weight: i16,
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
    let pair = american_book(&agreements());
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

// ---------------------------------------------------------------------------
// The settings value
// ---------------------------------------------------------------------------

// What the user has selected in the Settings tab
//
// The engine used to hold every knob in a `thread_local!` cell of its own, and
// this crate's registry wrote them through `set_*` functions.  Those cells are
// being deleted in favour of one `Agreements` value threaded into
// `american()`, which leaves the *UI's* settings state with nowhere to live —
// so it lives here, where it belongs: a web app holding what the user picked
// is app state, not a hidden configuration channel.
//
// Wasm is single-threaded, so this cell is effectively a global.  It seeds
// from [`Agreements::current`] rather than `default()` while the engine still
// has cells of its own, so a knob this crate has not yet migrated still reads
// through.
thread_local! {
    static AGREEMENTS: std::cell::Cell<Agreements> =
        std::cell::Cell::new(Agreements::current());
}

/// The agreements a deal is bid under
fn agreements() -> Agreements {
    AGREEMENTS.with(std::cell::Cell::get)
}

/// Edit the settings value in place
fn amend(edit: impl FnOnce(&mut Agreements)) {
    AGREEMENTS.with(|cell| {
        let mut value = cell.get();
        edit(&mut value);
        cell.set(value);
    });
}

/// Define a registry row's `set`/`get` pair over one [`Agreements`] field
///
/// One line per knob whose engine cell has been deleted.  The pair keeps the
/// `fn(bool)` / `fn() -> bool` shape [`Setting::Toggle`] stores, so migrating a
/// knob costs a line here and nothing in the 65-row table.
macro_rules! knob {
    ($set:ident, $get:ident, $($field:ident).+ : $ty:ty) => {
        fn $set(value: $ty) {
            amend(|a| a.$($field).+ = value);
        }
        fn $get() -> $ty {
            agreements().$($field).+
        }
    };
}

knob!(set_open_one_notrump, open_one_notrump, opening.open_one_notrump: bool);
knob!(set_notrump_shape, notrump_shape_setting, opening.notrump_shape: american::NotrumpShape);
knob!(set_second_suit_agreement, second_suit_agreement, game_force.second_suit_agreement: bool);
knob!(set_game_backstop, game_backstop_enabled, game_force.game_backstop: bool);
knob!(set_fourth_suit_forcing, fourth_suit_forcing, rebid.fourth_suit_forcing: bool);
knob!(set_meckstroth_adjunct, meckstroth_adjunct, rebid.meckstroth_adjunct: bool);
knob!(set_limit_raise_acceptance, limit_raise_acceptance, response.limit_raise_acceptance: bool);
knob!(set_transfer_super_accept, transfer_super_accept, notrump.transfer_super_accept: bool);
knob!(set_transfer_slam_try, transfer_slam_try, notrump.transfer_slam_try: bool);
knob!(set_texas_slam_drive, texas_slam_drive, notrump.texas_slam_drive: bool);
knob!(set_stayman_both_majors, stayman_both_majors, notrump.stayman_both_majors: bool);
knob!(set_stayman_5card_max, stayman_5card_max, notrump.stayman_5card_max: bool);
knob!(set_invitational_5card_majors, invitational_5card_majors, notrump.invitational_5card_majors: bool);
knob!(set_transfer_longer_major, transfer_longer_major, notrump.transfer_longer_major: bool);
knob!(set_stayman_cue_continuation, stayman_cue_continuation, notrump.stayman_cue_continuation: bool);
knob!(set_stayman_minor_slam_try, stayman_minor_slam_try, notrump.stayman_minor_slam_try: bool);
knob!(set_splinter_doubled, splinter_doubled, competition.splinter_doubled: bool);
knob!(set_uvu, uvu, competition.uvu: bool);
knob!(set_uvu_over_majors, uvu_over_majors, competition.uvu_over_majors: bool);
knob!(set_direct_3nt_stopper, direct_3nt_stopper, competition.direct_3nt_stopper: bool);
knob!(set_cue_raise_answer, cue_raise_answer, competition.cue_raise_answer: bool);
knob!(set_cue_minor_raise_answer, cue_minor_raise_answer, competition.cue_minor_raise_answer: bool);
knob!(set_major_support_double, major_support_double, competition.major_support_double: bool);
knob!(set_high_overcall_responses, high_overcall_responses, competition.high_overcall_responses: bool);
knob!(set_jordan_truscott, jordan_truscott, competition.jordan_truscott: bool);
knob!(set_delayed_cue, delayed_cue, competition.delayed_cue: bool);
knob!(set_competition_over_stayman, competition_over_stayman, competition.competition_over_stayman: bool);
knob!(set_competition_over_minor_transfer, competition_over_minor_transfer, competition.competition_over_minor_transfer: bool);
knob!(set_competition_over_diamond_transfer, competition_over_diamond_transfer, competition.competition_over_diamond_transfer: bool);
knob!(set_defense_to_2d_multi, defense_to_2d_multi, competition.defense_2d_multi: bool);
knob!(set_negative_double_shape, negative_double_shape, competition.negative_double_shape: american::NegativeDoubleShape);
knob!(set_lebensohl_style, lebensohl_style, competition.lebensohl_style: american::LebensohlStyle);

/// The Settings-tab registry: one row per user-facing bidding knob
///
/// This table is the **single source of truth** for the Settings tab.
/// [`set_option`] / [`set_choice`] dispatch a call through it and
/// [`describe_options`] serialises it for the JS renderer, so adding a convention
/// to the UI needs only one row here (plus the engine `set_*` it points at) — the
/// old hand-synced JS `CURATED` / `MORE` arrays are gone.  Each `set_*` is a
/// module-level thread-local flag read when a deal rebuilds `american()` in
/// `deal_with`; wasm is single-threaded, so the thread-local is effectively a global.
/// A row's `requires`: the master this control is dead without.
///
/// Two forms, both resolved in JS against the *current* state of the named row:
/// `"key"` (that toggle must be on) and `"key=value"` (that choice must equal
/// `value`).  The engine has plenty of knobs that read nothing while another is
/// off — `set_advance_rubens` under `set_rich_advance_double`,
/// `set_penalty_no_pull` under the latch, `set_uvu_encircle` under `set_uvu` —
/// and rendering those as equal, independently clickable peers (often in a
/// different section from their master) is a lie the UI tells about what the
/// bidder will do.  A gated row renders disabled until its master is armed.
type Requires = Option<&'static str>;

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
        /// See [`Requires`].
        requires: Requires,
        #[serde(skip)]
        set: fn(bool),
        /// Reads the engine cell `set` writes, so a test can prove `default`
        /// still mirrors it.  See `registry_defaults_match_the_engine`.
        #[serde(skip)]
        get: fn() -> bool,
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
        /// See [`Requires`].
        requires: Requires,
        #[serde(skip)]
        set: fn(&str),
        /// The `value` of the variant the engine currently holds.  See
        /// [`Setting::Toggle::get`].
        #[serde(skip)]
        get: fn() -> &'static str,
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

    /// Test-only: the row's gate. The UI reads the serialised field, not this.
    #[cfg(test)]
    const fn requires(&self) -> Requires {
        match self {
            Setting::Toggle { requires, .. } | Setting::Choice { requires, .. } => *requires,
        }
    }
}

/// Terser constructor for the common ungated [`Setting::Toggle`] row.
const fn toggle(
    key: &'static str,
    section: &'static str,
    label: &'static str,
    default: bool,
    set: fn(bool),
    get: fn() -> bool,
) -> Setting {
    Setting::Toggle {
        key,
        section,
        label,
        default,
        requires: None,
        set,
        get,
    }
}

/// A [`Setting::Toggle`] the engine ignores unless `requires` holds (see
/// [`Requires`]).
const fn gated(
    key: &'static str,
    section: &'static str,
    label: &'static str,
    default: bool,
    set: fn(bool),
    get: fn() -> bool,
    requires: &'static str,
) -> Setting {
    Setting::Toggle {
        key,
        section,
        label,
        default,
        requires: Some(requires),
        set,
        get,
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

/// The `(1NT)` defense family — variants map onto `american::NotrumpDefense`.
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

/// The 1NT defense the engine currently holds, as its registry `value`.
///
/// `DirectLandy` round-trips even though `set_notrump_defense_choice` reaches it
/// through `set_direct_landy_double` — that setter selects the variant too, so
/// the one cell answers for both.  Variants with no registry row read as the
/// default, which is what the radio group can display.
fn get_notrump_defense_choice() -> &'static str {
    use american::NotrumpDefense;
    match american::notrump_defense() {
        NotrumpDefense::DirectDont => "direct_dont",
        NotrumpDefense::Woolsey => "woolsey",
        NotrumpDefense::AlwaysPass => "always_pass",
        NotrumpDefense::DirectLandy => "direct_landy",
        _ => "natural",
    }
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
    set_notrump_shape(match value {
        "balanced" => NotrumpShape::Balanced,
        "wide" => NotrumpShape::Wide,
        _ => NotrumpShape::Wide6322,
    });
}

/// The 1NT opening shape the engine currently holds, as its registry `value`.
fn get_notrump_shape_choice() -> &'static str {
    use american::NotrumpShape;
    match notrump_shape_setting() {
        NotrumpShape::Balanced => "balanced",
        NotrumpShape::Wide => "wide",
        _ => "wide6322",
    }
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
    set_negative_double_shape(match value {
        "sputnik" => NegativeDoubleShape::Sputnik,
        "cachalot" => NegativeDoubleShape::Cachalot,
        _ => NegativeDoubleShape::Modern,
    });
}

/// The negative-double school the engine currently holds, as its registry
/// `value`.  The unoffered pre-Modern `BothMajors` reads as `modern`.
fn get_negative_double_choice() -> &'static str {
    use american::NegativeDoubleShape;
    match negative_double_shape() {
        NegativeDoubleShape::Sputnik => "sputnik",
        NegativeDoubleShape::Cachalot => "cachalot",
        _ => "modern",
    }
}

/// The keycard ask's relocation stance — variants map onto
/// `instinct::RkcbVariant`.  Each widens the one before it: plain 4NT, then
/// the minor asks relocate (Redwood), then hearts relocate to 4♠ as well.
static RKCB_VARIANT_VARIANTS: &[Variant] = &[
    Variant {
        value: "plain",
        label: "Plain 4NT",
    },
    Variant {
        value: "redwood",
        label: "Redwood (relocate minor asks)",
    },
    Variant {
        value: "kickback",
        label: "Kickback (relocate all asks)",
    },
];

/// Select the keycard relocation stance from its registry `value`.
fn set_rkcb_variant_choice(value: &str) {
    use instinct::RkcbVariant;
    instinct::set_rkcb_variant(match value {
        "redwood" => RkcbVariant::Redwood,
        "kickback" => RkcbVariant::Kickback,
        _ => RkcbVariant::Plain,
    });
}

/// The keycard relocation stance the engine currently holds, as its registry
/// `value`.
fn get_rkcb_variant_choice() -> &'static str {
    use instinct::RkcbVariant;
    match instinct::rkcb_variant_now() {
        RkcbVariant::Redwood => "redwood",
        RkcbVariant::Kickback => "kickback",
        _ => "plain",
    }
}

/// Lebensohl as an on/off toggle: on = Transfer Lebensohl (the shipped package),
/// off = none.  `LebensohlStyle::Plain` is deliberately unreachable here — it is a
/// measured-worse arm, kept for A/B only.
fn set_lebensohl_toggle(on: bool) {
    use american::LebensohlStyle;
    set_lebensohl_style(if on {
        LebensohlStyle::Transfer
    } else {
        LebensohlStyle::Off
    });
}

/// Whether Lebensohl is live.  See [`advance_sohl_toggle`] on `Plain`.
fn lebensohl_toggle() -> bool {
    lebensohl_style() != american::LebensohlStyle::Off
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

/// Whether advancer's Lebensohl is live.  `Plain` is unreachable from the UI but
/// counts as on, so an A/B that selected it is not reported as "off".
fn advance_sohl_toggle() -> bool {
    american::advance_sohl_style() != american::LebensohlStyle::Off
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

/// Whether the 1NT minor scheme is Puppet rather than European transfers.
fn puppet_stayman() -> bool {
    american::notrump_minors() == american::PUPPET
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
    toggle("open_one_notrump", OPENINGS, "Open 1NT (15–17)", true, set_open_one_notrump, open_one_notrump),
    Setting::Choice { key: "notrump_shape", section: OPENINGS, label: "1NT opening shape", variants: NOTRUMP_SHAPE_VARIANTS, default: "wide6322", requires: None, set: set_notrump_shape_choice, get: get_notrump_shape_choice },
    // Notrump
    toggle("puppet_stayman", NOTRUMP, "Puppet Stayman (3♣)", true, set_puppet_stayman, puppet_stayman),
    toggle("garbage_stayman", NOTRUMP, "Garbage Stayman", true, american::set_garbage_stayman, american::garbage_stayman),
    toggle("transfer_super_accept", NOTRUMP, "", false, set_transfer_super_accept, transfer_super_accept),
    toggle("transfer_slam_try", NOTRUMP, "", true, set_transfer_slam_try, transfer_slam_try),
    toggle("texas_slam_drive", NOTRUMP, "", true, set_texas_slam_drive, texas_slam_drive),
    toggle("transfer_gf_majors", NOTRUMP, "", true, american::set_transfer_gf_majors, american::transfer_gf_majors),
    gated("transfer_gf_hearts", NOTRUMP, "", true, american::set_transfer_gf_hearts, american::transfer_gf_hearts, "transfer_gf_majors"),
    toggle("stayman_both_majors", NOTRUMP, "", true, set_stayman_both_majors, stayman_both_majors),
    toggle("stayman_5card_max", NOTRUMP, "", true, set_stayman_5card_max, stayman_5card_max),
    toggle("invitational_5card_majors", NOTRUMP, "", true, set_invitational_5card_majors, invitational_5card_majors),
    toggle("transfer_longer_major", NOTRUMP, "", true, set_transfer_longer_major, transfer_longer_major),
    toggle("crawling_stayman", NOTRUMP, "", true, american::set_crawling_stayman, american::crawling_stayman),
    toggle("stayman_cue_continuation", NOTRUMP, "", true, set_stayman_cue_continuation, stayman_cue_continuation),
    toggle("stayman_minor_slam_try", NOTRUMP, "", true, set_stayman_minor_slam_try, stayman_minor_slam_try),
    toggle("nt_splinter", NOTRUMP, "1NT - 3M splinter (short major, ♦4, ♣5–6)", true, american::set_nt_splinter, american::nt_splinter),
    // Competition
    toggle("lebensohl", COMPETITION, "Lebensohl (over 1NT interference)", true, set_lebensohl_toggle, lebensohl_toggle),
    toggle("advance_lebensohl", COMPETITION, "Lebensohl advancing a double", true, set_advance_sohl_toggle, advance_sohl_toggle),
    toggle("splinter_doubled", COMPETITION, "", true, set_splinter_doubled, splinter_doubled),
    toggle("passed_hand_overcall", COMPETITION, "", true, american::set_passed_hand_overcall, american::passed_hand_overcall),
    toggle("uvu", COMPETITION, "Unusual vs Unusual", true, set_uvu, uvu),
    toggle("uvu_over_majors", COMPETITION, "Unusual vs Unusual (over majors)", true, set_uvu_over_majors, uvu_over_majors),
    toggle("direct_3nt_stopper", COMPETITION, "", true, set_direct_3nt_stopper, direct_3nt_stopper),
    toggle("cue_raise_answer", COMPETITION, "", true, set_cue_raise_answer, cue_raise_answer),
    toggle("cue_minor_raise_answer", COMPETITION, "", true, set_cue_minor_raise_answer, cue_minor_raise_answer),
    toggle("major_support_double", COMPETITION, "", true, set_major_support_double, major_support_double),
    toggle("high_overcall_responses", COMPETITION, "", false, set_high_overcall_responses, high_overcall_responses),
    toggle("jordan_truscott", COMPETITION, "Jordan / Truscott 2NT", true, set_jordan_truscott, jordan_truscott),
    toggle("delayed_cue", COMPETITION, "", false, set_delayed_cue, delayed_cue),
    toggle("competition_over_stayman", COMPETITION, "", true, set_competition_over_stayman, competition_over_stayman),
    gated("competition_over_minor_transfer", COMPETITION, "", true, set_competition_over_minor_transfer, competition_over_minor_transfer, "puppet_stayman"),
    gated("competition_over_diamond_transfer", COMPETITION, "", true, set_competition_over_diamond_transfer, competition_over_diamond_transfer, "puppet_stayman"),
    toggle("defense_to_2d_multi", COMPETITION, "", false, set_defense_to_2d_multi, defense_to_2d_multi),
    toggle("leaping_michaels", COMPETITION, "Leaping Michaels", true, american::set_leaping_michaels, american::leaping_michaels_enabled),
    toggle("responsive_takeout", COMPETITION, "Responsive doubles", true, american::set_responsive_takeout, american::responsive_takeout_enabled),
    toggle("rich_advance_double", COMPETITION, "", true, american::set_rich_advance_double, american::rich_advance_double_enabled),
    gated("advance_rubens", COMPETITION, "Rubens advances", false, american::set_advance_rubens, american::advance_rubens_enabled, "rich_advance_double"),
    toggle("nt_overcall_gladiator", COMPETITION, "Gladiator (1NT-overcall advance)", false, american::set_nt_overcall_gladiator, american::nt_overcall_gladiator),
    // Negative-double school over their overcall — the enum-backed radio family
    Setting::Choice {
        key: "negative_double_shape",
        section: COMPETITION,
        label: "Negative double (over their overcall)",
        variants: NEGATIVE_DOUBLE_VARIANTS,
        default: "modern",
        requires: None,
        set: set_negative_double_choice,
        get: get_negative_double_choice,
    },
    // Defense to their 1NT — the radio family is the enum-backed choice
    Setting::Choice {
        key: "notrump_defense",
        section: DEFENSE,
        label: "Defense system",
        variants: NOTRUMP_DEFENSE_VARIANTS,
        default: "natural",
        requires: None,
        set: set_notrump_defense_choice,
        get: get_notrump_defense_choice,
    },
    gated("direct_dont_four_four", DEFENSE, "", true, american::set_direct_dont_four_four, american::direct_dont_four_four, "notrump_defense=direct_dont"),
    toggle("stayman_defense", DEFENSE, "", false, american::set_stayman_defense, american::stayman_defense_enabled),
    toggle("transfer_defense", DEFENSE, "", false, american::set_transfer_defense, american::transfer_defense_enabled),
    toggle("minor_transfer_defense", DEFENSE, "", false, american::set_minor_transfer_defense, american::minor_transfer_defense_enabled),
    // Rebids & responses
    toggle("second_suit_agreement", REBIDS, "", true, set_second_suit_agreement, second_suit_agreement),
    toggle("game_backstop", REBIDS, "2/1 game backstop (retired)", false, set_game_backstop, game_backstop_enabled),
    toggle("fourth_suit_forcing", REBIDS, "Fourth suit forcing", true, set_fourth_suit_forcing, fourth_suit_forcing),
    toggle("meckstroth_adjunct", REBIDS, "Meckstroth adjunct", true, set_meckstroth_adjunct, meckstroth_adjunct),
    toggle("limit_raise_acceptance", REBIDS, "", true, set_limit_raise_acceptance, limit_raise_acceptance),
    // Floor (instinct)
    toggle("one_nt_runout", FLOOR, "", true, instinct::set_one_nt_runout, instinct::one_nt_runout),
    gated("one_nt_runout_universal", FLOOR, "", true, instinct::set_one_nt_runout_universal, instinct::one_nt_runout_universal_enabled, "one_nt_runout"),
    toggle("settle_floor", FLOOR, "", true, instinct::set_settle_floor, instinct::settle_floor_enabled),
    toggle("rubens_advances", FLOOR, "", false, instinct::set_rubens_advances, instinct::rubens_advances_enabled),
    toggle("floor_rkcb", FLOOR, "", true, instinct::set_floor_rkcb, instinct::floor_rkcb),
    // One radio family, not two checkboxes: the old redwood/kickback toggles let
    // the UI show both checked, a state the engine cannot play.  `rkcb_minors`
    // used to sit here too and was dropped for the same reason — either
    // relocation implies the minors' reach (`minor_asks_now`), so the checkbox
    // was inert on two of its six cells.
    Setting::Choice { key: "rkcb_variant", section: FLOOR, label: "Keycard ask relocation", variants: RKCB_VARIANT_VARIANTS, default: "plain", requires: Some("floor_rkcb"), set: set_rkcb_variant_choice, get: get_rkcb_variant_choice },
    toggle("two_over_one_force", FLOOR, "2/1 forces game", true, instinct::set_two_over_one_force, instinct::two_over_one_force),
    gated("penalize_escape_stack", FLOOR, "", true, instinct::set_penalize_escape_stack, instinct::penalize_escape_stack, "one_nt_runout"),
    gated("penalize_escape_values", FLOOR, "", true, instinct::set_penalize_escape_values, instinct::penalize_escape_values, "one_nt_runout"),
    gated("uvu_encircle", FLOOR, "UVU penalty procedure", true, instinct::set_uvu_encircle, instinct::uvu_encircle, "uvu"),
    gated("penalty_latch", FLOOR, "", true, instinct::set_penalty_latch, instinct::penalty_latch_enabled, "notrump_defense=natural"),
    gated("penalty_no_pull", FLOOR, "", true, instinct::set_penalty_no_pull, instinct::penalty_no_pull, "penalty_latch"),
    toggle("advancer_xx_runout", FLOOR, "", true, instinct::set_advancer_xx_runout, instinct::advancer_xx_runout_enabled),
    toggle("doubler_xx_runout", FLOOR, "", true, instinct::set_doubler_xx_runout, instinct::doubler_xx_runout_enabled),
    // Inference (auction reading)
    toggle("nt_invite_inference", INFERENCE, "", true, inference::set_nt_invite_inference, inference::nt_invite_inference),
    gated("rubens_transfer_reading", INFERENCE, "", true, inference::set_rubens_transfer_reading, inference::rubens_transfer_reading, "rubens_advances"),
    toggle("fallback_projection", INFERENCE, "", true, inference::set_fallback_projection, inference::fallback_projection_enabled),
    toggle("control_bid_reading", INFERENCE, "", true, inference::set_control_bid_reading, inference::control_bid_reading),
    toggle("rule_accept", INFERENCE, "", true, inference::set_rule_accept, inference::rule_accept_enabled),
];

/// The in-browser half of `examples/binky`'s benchmark: fix N-S, reshuffle E-W.
///
/// The published `(mu, sigma)` table claims to describe the distribution of
/// double-dummy tricks given the two N-S hands. This checks that claim on *your*
/// hands, and it is an honest check for one specific reason: conditioned on both
/// N-S hands and nothing else, the posterior over the hidden twenty-six cards
/// **is** uniform over East-West splits. No inference, no range envelope, no
/// rule replay — so there is no sampler to be biased.
///
/// Stateful because the transposition table is worth keeping between chunks:
/// JS calls [`run`][Binky::run] in small batches so the page keeps painting,
/// exactly as the practice oracle does.
#[wasm_bindgen]
pub struct Binky {
    partial: PartialDeal,
    /// Notrump alone, or the four suits to take a max over.
    strains: Vec<Strain>,
    solver: Solver,
    rng: StdRng,
    /// Layouts by trick count, 0..=13.
    histogram: [u32; 14],
}

#[wasm_bindgen]
impl Binky {
    /// Fix the two N-S hands; `north`/`south` are `spades.hearts.diamonds.clubs`.
    ///
    /// Returns `None` if either hand is not thirteen valid cards, or if the two
    /// overlap — a caller should surface that rather than solve a nonsense deal.
    /// A **static factory, not a `constructor`**: `new` in JS must yield an
    /// object, so a fallible constructor would have to throw or hand back a
    /// half-built one.
    #[must_use]
    pub fn create(north: &str, south: &str, notrump: bool, seed: &str) -> Option<Binky> {
        let north: Hand = north.parse().ok()?;
        let south: Hand = south.parse().ok()?;
        let partial = set_seat(
            set_seat(Builder::new(), Seat::North, north),
            Seat::South,
            south,
        )
        .build_partial()
        .ok()?;
        // `build_partial` accepts short hands; the shuffle only means anything
        // when both are complete.
        if north.len() != 13 || south.len() != 13 {
            return None;
        }
        let strains = if notrump {
            vec![Strain::Notrump]
        } else {
            Strain::ASC[..4].to_vec()
        };
        Some(Self {
            partial,
            solver: Solver::with_memory(strains[0], TT_MB.0, TT_MB.1),
            strains,
            rng: StdRng::seed_from_u64(seed.parse().unwrap_or(0)),
            histogram: [0; 14],
        })
    }

    /// Solve `samples` more E-W shuffles and return the running verdict as JSON:
    /// `{n, mean, sd, histogram}` — `histogram[k]` is the count of layouts on
    /// which N-S take exactly `k` tricks.
    pub fn run(&mut self, samples: u32) -> String {
        for deal in fill_deals(&mut self.rng, self.partial).take(samples as usize) {
            // The label is the pair's best declarer, and for a suit table also
            // their best suit — the same `max` `examples/binky` fits against.
            let tricks = self
                .strains
                .iter()
                .map(|&strain| {
                    self.solver.set_strain(strain);
                    let row = self.solver.solve(deal);
                    row.get(Seat::North).get().max(row.get(Seat::South).get())
                })
                .max()
                .unwrap_or(0);
            self.histogram[usize::from(tricks).min(13)] += 1;
        }

        let n: u32 = self.histogram.iter().sum();
        let total = f64::from(n).max(1.0);
        let mean = self
            .histogram
            .iter()
            .enumerate()
            .map(|(k, &c)| k as f64 * f64::from(c))
            .sum::<f64>()
            / total;
        let variance = self
            .histogram
            .iter()
            .enumerate()
            .map(|(k, &c)| f64::from(c) * (k as f64 - mean).powi(2))
            .sum::<f64>()
            / total;
        serde_json::to_string(&serde_json::json!({
            "n": n,
            "mean": mean,
            "sd": variance.max(0.0).sqrt(),
            "histogram": self.histogram,
        }))
        .expect("verdict serialization")
    }
}

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
mod tests;
