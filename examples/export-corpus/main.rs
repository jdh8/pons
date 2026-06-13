//! Corpus exporter (AI-bidder M0.2)
//!
//! Walks the floorless 2/1 books ([`bare_two_over_one`]) and emits one JSONL
//! record per `(node, call)` the books authorise:
//!
//! ```text
//! {"book":"constructive","auction":["1H","P"],"call":"2C",
//!  "weight":1.0,"tags":["FG","NAT"],"label":"","description":"…"}
//! ```
//!
//! Tags are drawn from the WBF abbreviation vocabulary (see
//! `docs/ai-bidder/wbf-abbreviations.md`) and **derived structurally** from the
//! auction + call: the keyless reading already encoded in
//! [`Inferences`][pons::bidding::Inferences] and [`Context`].  Where a rule
//! carries a hand-authored [`label`][pons::bidding::rules::Rule::label]
//! (`Rules::note`), that label becomes the `description`; otherwise a templated
//! description is generated.  This is the "auto-derive + patch" hybrid: the
//! machine tags every node, humans patch the prose where it matters.
//!
//! Records are deduplicated by authored-rules identity, so the four seat
//! variants of a shared opening table (keyed under 0–3 leading passes) yield one
//! record-set, not four.  Run with `cargo run --example export-corpus`; JSONL
//! goes to stdout, a summary to stderr.
//!
//! # Known coverage (first cut)
//!
//! - The **competitive** book is almost entirely "system-on" rebases and
//!   guarded fallbacks rather than standalone [`Rules`][pons::bidding::Rules]
//!   nodes, so it contributes few direct records; its meanings live in the
//!   constructive core its rebases resolve into.
//! - **Deep artificial continuations** (RKCB step responses, BTU relays) are
//!   tagged coarsely by the keyless structural reading — these are the prime
//!   targets for a hand-authored [`note`][pons::bidding::Rules::note] label,
//!   which overrides the derived description.

use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Strain};
use pons::bidding::context::Context;
use pons::bidding::trie::Trie;
use pons::bidding::two_over_one::bare_two_over_one;
use std::collections::HashSet;

/// One auction prefix the books classify, plus the rules found there.
struct Node<'a> {
    book: &'static str,
    auction: Vec<Call>,
    rules: &'a pons::bidding::Rules,
}

fn main() {
    let pair = bare_two_over_one();
    let books: [(&'static str, &Trie); 3] = [
        ("constructive", &pair.constructive.0),
        ("competitive", &pair.competitive.0),
        ("defensive", &pair.defensive.0),
    ];

    let mut seen: HashSet<usize> = HashSet::new();
    let mut nodes = 0usize;
    let mut records = 0usize;
    let mut specific = 0usize;
    let mut per_book = [0usize; 3];

    for (book_index, (book, trie)) in books.into_iter().enumerate() {
        for (auction, classifier) in trie.iter() {
            let Some(rules) = classifier.as_rules() else {
                continue;
            };
            // Dedupe by the authored-rules object: shared seat variants of one
            // opening/response table classify through the same `Arc`.
            let id = core::ptr::from_ref(classifier) as *const () as usize;
            if !seen.insert(id) {
                continue;
            }
            nodes += 1;
            let node = Node {
                book,
                auction: auction.to_vec(),
                rules,
            };
            for record in node_records(&node) {
                // A "specific" tag is anything beyond the NAT/NF fallback.
                if !matches!(record.tags.as_slice(), ["NAT"] | ["NF"]) {
                    specific += 1;
                }
                per_book[book_index] += 1;
                records += 1;
                println!("{}", record.to_json());
            }
        }
    }

    eprintln!(
        "export-corpus: {nodes} authored nodes, {records} (node,call) records \
         ({} constructive, {} competitive, {} defensive), {specific} with a \
         specific (non-NAT/NF) tag.",
        per_book[0], per_book[1], per_book[2]
    );
}

/// A single corpus record.
struct Record {
    book: &'static str,
    auction: Vec<Call>,
    call: Call,
    weight: f32,
    tags: Vec<&'static str>,
    label: &'static str,
    description: String,
}

impl Record {
    fn to_json(&self) -> String {
        let auction: Vec<String> = self.auction.iter().map(|c| format!("{c}")).collect();
        serde_json::json!({
            "book": self.book,
            "auction": auction,
            "call": format!("{}", self.call),
            "weight": self.weight,
            "tags": self.tags,
            "label": self.label,
            "description": self.description,
        })
        .to_string()
    }
}

/// Collapse a node's rules to one record per distinct call (highest weight wins
/// as the representative), tagging each from the auction + call.
fn node_records(node: &Node<'_>) -> Vec<Record> {
    let ctx = Context::new(RelativeVulnerability::NONE, &node.auction);

    // Best (weight, label) per call, in first-seen order.
    let mut order: Vec<Call> = Vec::new();
    let mut best: std::collections::HashMap<Call, (f32, &'static str)> =
        std::collections::HashMap::new();
    for rule in node.rules.rules() {
        let entry = best.entry(rule.call()).or_insert_with(|| {
            order.push(rule.call());
            (f32::NEG_INFINITY, "")
        });
        if rule.weight() > entry.0 {
            *entry = (rule.weight(), rule.label());
        } else if entry.1.is_empty() && !rule.label().is_empty() {
            // Keep a label even if a higher-weight sibling rule lacked one.
            entry.1 = rule.label();
        }
    }

    order
        .into_iter()
        .map(|call| {
            let (weight, label) = best[&call];
            let (tags, derived) = derive(node.book, call, &ctx);
            let description = if label.is_empty() {
                derived
            } else {
                label.to_string()
            };
            Record {
                book: node.book,
                auction: node.auction.clone(),
                call,
                weight,
                tags,
                label,
                description,
            }
        })
        .collect()
}

const ONE_NOTRUMP: Bid = Bid::new(1, Strain::Notrump);

/// Whether our side has bid any strain yet in this auction.
fn we_bid_anything(ctx: &Context<'_>) -> bool {
    Strain::ASC.into_iter().any(|s| ctx.we_bid(s))
}

/// Derive `(tags, description)` for a call at a node, structurally.
///
/// First cut: covers the high-confidence structural cases (openings, notrump
/// responses, takeout/negative doubles, cue-bids, raises, rebids, the 2/1
/// game force) and falls back to `NAT` / a generic gloss otherwise.
fn derive(book: &'static str, call: Call, ctx: &Context<'_>) -> (Vec<&'static str>, String) {
    match call {
        Call::Pass => (vec!["NF"], "Pass — below an action.".into()),
        Call::Redouble => (vec!["RDBL"], "Redouble.".into()),
        Call::Double => derive_double(book, ctx),
        Call::Bid(bid) => derive_bid(ctx, bid),
    }
}

fn derive_double(book: &'static str, ctx: &Context<'_>) -> (Vec<&'static str>, String) {
    // Defending side's first action over a suit opening: takeout.
    let their_suit_opening = ctx.last_bid().is_some_and(|b| b.strain.is_suit());
    if book == "defensive" && !we_bid_anything(ctx) && their_suit_opening {
        return (vec!["T/O"], "Takeout double of their opening.".into());
    }
    // We opened and they overcalled, responder doubles low: negative.
    if book == "competitive" && we_bid_anything(ctx) {
        return (vec!["NEG"], "Negative double — the unbid suit(s).".into());
    }
    (vec!["PEN"], "Penalty double.".into())
}

fn derive_bid(ctx: &Context<'_>, bid: Bid) -> (Vec<&'static str>, String) {
    // An opening bid: the first non-pass call.
    if ctx.last_bid().is_none() {
        return derive_opening(bid);
    }

    let partner_opened_1nt = ctx.partner_last_bid() == Some(ONE_NOTRUMP);
    if partner_opened_1nt
        && ctx.undisturbed()
        && let Some(record) = derive_over_1nt(bid)
    {
        return record;
    }

    let strain = bid.strain;

    // Cue-bid of a strain the opponents bid.
    if strain.is_suit() && ctx.they_bid(strain) {
        return (vec!["CUE"], "Cue-bid of their suit.".into());
    }

    // Raise of partner's last suit.
    if let Some(suit) = strain.suit() {
        if ctx.partner_last_suit() == Some(suit) {
            let jump = jump_over(ctx, bid);
            let mut tags = vec!["SUPP"];
            if jump >= 1 {
                tags.push("PRE");
            }
            return (tags, "Raise of partner's suit.".into());
        }
        // Rebid of our own suit: extra length.
        if ctx.we_bid(strain) {
            return (vec!["NAT", "L/S"], "Rebid — extra length.".into());
        }
    }

    if strain == Strain::Notrump {
        return (vec!["NAT"], format!("{}NT — natural.", bid.level.get()));
    }

    // A new suit.
    let jump = jump_over(ctx, bid);
    if jump >= 2 {
        return (vec!["SPL"], "Splinter / double jump — shortness.".into());
    }
    if jump == 1 {
        return (vec!["WJS", "WK"], "Weak jump shift.".into());
    }
    // Cheapest new suit. A 2-level new suit over partner's 1-major opening is
    // a game-forcing 2/1.
    if bid.level.get() == 2 && is_two_over_one(ctx, bid) {
        return (vec!["FG", "NAT"], "Two-over-one — game forcing.".into());
    }
    (vec!["NAT"], "Natural new suit.".into())
}

fn derive_opening(bid: Bid) -> (Vec<&'static str>, String) {
    match (bid.level.get(), bid.strain) {
        (1, Strain::Notrump) => (vec!["NAT", "BAL"], "1NT opening — balanced.".into()),
        (2, Strain::Notrump) => (vec!["NAT", "BAL"], "2NT opening — balanced.".into()),
        (2, Strain::Clubs) => (vec!["ART", "STR"], "Strong artificial 2♣ opening.".into()),
        (1, s) if s.is_suit() => (vec!["NAT"], "One-level suit opening.".into()),
        (2, s) if s.is_suit() => (vec!["PRE", "WK"], "Weak two opening.".into()),
        (_, s) if s.is_suit() => (vec!["PRE"], "Preemptive opening.".into()),
        _ => (vec!["NAT"], "Opening bid.".into()),
    }
}

/// Responses to partner's 1NT opening (Stayman / Jacoby transfers).
fn derive_over_1nt(bid: Bid) -> Option<(Vec<&'static str>, String)> {
    match (bid.level.get(), bid.strain) {
        (2, Strain::Clubs) => Some((vec!["STAY"], "Stayman.".into())),
        (2, Strain::Diamonds) => Some((vec!["TRF"], "Jacoby transfer to hearts.".into())),
        (2, Strain::Hearts) => Some((vec!["TRF"], "Jacoby transfer to spades.".into())),
        (2, Strain::Spades) => Some((vec!["TRF"], "Minor-suit transfer.".into())),
        (3, s) if s.is_suit() => Some((vec!["NAT", "F"], "Natural, forcing.".into())),
        (_, Strain::Notrump) => Some((vec!["NAT", "QUANT"], "Quantitative notrump.".into())),
        _ => None,
    }
}

/// Levels of jump above the cheapest legal level for the bid's strain.
fn jump_over(ctx: &Context<'_>, bid: Bid) -> u8 {
    ctx.min_level(bid.strain)
        .map_or(0, |min| bid.level.get().saturating_sub(min.get()))
}

/// Whether `bid` is a game-forcing 2/1 over partner's 1-major opening.
fn is_two_over_one(ctx: &Context<'_>, bid: Bid) -> bool {
    let Some(partner) = ctx.partner_last_bid() else {
        return false;
    };
    if partner.level.get() != 1 || !matches!(partner.strain, Strain::Hearts | Strain::Spades) {
        return false;
    }
    // A lower-ranking new suit at the two level, not their suit, not ours.
    bid.level.get() == 2
        && bid.strain.is_suit()
        && !ctx.we_bid(bid.strain)
        && bid.strain < partner.strain
}
