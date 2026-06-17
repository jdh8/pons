//! Corpus exporter (AI-bidder M0.2)
//!
//! Walks the floorless 2/1 books ([`bare_american`]) and emits one JSONL
//! record per `(node, call)` the books authorise:
//!
//! ```text
//! {"book":"constructive","auction":["1H","P"],"call":"2C",
//!  "weight":1.0,"tags":["FG","NAT"],"label":"",
//!  "constraint":"12–21 points, and 5+ ♦","description":"…"}
//! ```
//!
//! Tags are drawn from the WBF abbreviation vocabulary (see
//! `docs/ai-bidder/wbf-abbreviations.md`) and **derived structurally** from the
//! auction + call: the keyless reading already encoded in
//! [`Inferences`][pons::bidding::Inferences] and [`Context`].
//!
//! The `constraint` field is the **truthful** render of the call's
//! highest-weight rule, read straight from its [`Constraint`] via
//! [`Rule::describe`][pons::bidding::rules::Rule::describe] (AI-bidder M4) — not
//! a structural guess.  The `description` field then prefers, in order: a
//! hand-authored [`label`][pons::bidding::rules::Rule::label] (`Rules::note`);
//! else the truthful `constraint` render; else, only when the constraint is a
//! bare opaque predicate, a structurally-templated gloss.  So prose is truthful
//! by default, with human overrides and an opaque last resort.
//!
//! [`Constraint`]: pons::bidding::constraint::Constraint
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
use pons::bidding::american::bare_american;
use pons::bidding::constraint::Description;
use pons::bidding::context::Context;
use pons::bidding::tags::derive;
use pons::bidding::trie::Trie;
use std::collections::HashSet;

/// One auction prefix the books classify, plus the rules found there.
struct Node<'a> {
    system: &'static str,
    book: &'static str,
    auction: Vec<Call>,
    rules: &'a pons::bidding::Rules,
}

fn main() {
    let system = "american";
    let pair = bare_american();
    let books: [(&'static str, &Trie); 3] = [
        ("constructive", &pair.constructive.0),
        ("competitive", &pair.competitive.0),
        ("defensive", &pair.defensive.0),
    ];

    let mut seen: HashSet<usize> = HashSet::new();
    let mut nodes = 0usize;
    let mut records = 0usize;
    let mut specific = 0usize;
    let mut opaque = 0usize;
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
                system,
                book,
                auction: auction.to_vec(),
                rules,
            };
            for record in node_records(&node) {
                // A "specific" tag is anything beyond the NAT/NF fallback.
                if !matches!(record.tags.as_slice(), ["NAT"] | ["NF"]) {
                    specific += 1;
                }
                if record.opaque {
                    opaque += 1;
                }
                per_book[book_index] += 1;
                records += 1;
                println!("{}", record.to_json());
            }
        }
    }

    eprintln!(
        "export-corpus: system {system}, {nodes} authored nodes, {records} \
         (node,call) records ({} constructive, {} competitive, {} defensive), \
         {specific} with a specific (non-NAT/NF) tag, {opaque} with an opaque \
         constraint.",
        per_book[0], per_book[1], per_book[2]
    );
}

/// A single corpus record.
struct Record {
    system: &'static str,
    book: &'static str,
    auction: Vec<Call>,
    call: Call,
    weight: f32,
    tags: Vec<&'static str>,
    label: &'static str,
    /// Truthful render of the representative rule's constraint (`describe`).
    constraint: String,
    description: String,
    /// Whether that constraint was a bare opaque predicate (a coverage metric,
    /// not serialized): true means `description` fell back to the gloss.
    opaque: bool,
}

impl Record {
    fn to_json(&self) -> String {
        let auction: Vec<String> = self.auction.iter().map(|c| format!("{c}")).collect();
        serde_json::json!({
            "system": self.system,
            "book": self.book,
            "auction": auction,
            "call": format!("{}", self.call),
            "weight": self.weight,
            "tags": self.tags,
            "label": self.label,
            "constraint": self.constraint,
            "description": self.description,
        })
        .to_string()
    }
}

/// Collapse a node's rules to one record per distinct call (highest weight wins
/// as the representative), tagging each from the auction + call.
fn node_records(node: &Node<'_>) -> Vec<Record> {
    let ctx = Context::new(RelativeVulnerability::NONE, &node.auction);

    // Best (weight, label, constraint description) per call, first-seen order.
    // The representative is the highest-weight rule for the call.
    let mut order: Vec<Call> = Vec::new();
    let mut best: std::collections::HashMap<Call, (f32, &'static str, Description)> =
        std::collections::HashMap::new();
    for rule in node.rules.rules() {
        let entry = best.entry(rule.call()).or_insert_with(|| {
            order.push(rule.call());
            (f32::NEG_INFINITY, "", Description::Opaque)
        });
        if rule.weight() > entry.0 {
            *entry = (rule.weight(), rule.label(), rule.describe());
        } else if entry.1.is_empty() && !rule.label().is_empty() {
            // Keep a label even if a higher-weight sibling rule lacked one.
            entry.1 = rule.label();
        }
    }

    order
        .into_iter()
        .map(|call| {
            let (weight, label, description) = best.remove(&call).expect("call was seen");
            let (tags, derived) = derive(node.book, call, &ctx);
            let constraint = description.to_string();
            let opaque = matches!(description, Description::Opaque);
            // Prefer a human label, then the truthful constraint, then the gloss.
            let prose = if !label.is_empty() {
                label.to_string()
            } else if opaque {
                derived
            } else {
                constraint.clone()
            };
            Record {
                system: node.system,
                book: node.book,
                auction: node.auction.clone(),
                call,
                weight,
                tags,
                label,
                constraint,
                description: prose,
                opaque,
            }
        })
        .collect()
}
