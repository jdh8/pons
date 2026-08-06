//! Dutch book pretty-printer
//!
//! Walks the floor-less Dutch books ([`dutch_book`]) and prints every authored
//! node as readable prose, exactly like `render-book`.  This is the Dutch
//! declarative-row port's byte-identity harness.
//!
//! Run with `cargo run --example render-dutch-book` (pipe to a pager — it is
//! long).

use pons::bidding::constraint::Description;
use pons::bidding::fallback::Fallback;
use pons::bidding::rules::Rules;
use pons::bidding::trie::Trie;
use pons::dutch_book;
use std::collections::HashSet;
use std::sync::Arc;

fn print_rules(rules: &Rules, opaque: &mut usize) {
    for rule in rules.rules() {
        let description = rule.describe();
        if matches!(description, Description::Opaque) {
            *opaque += 1;
        }
        let label = rule.label();
        let note = if label.is_empty() {
            String::new()
        } else {
            format!("   [{label}]")
        };
        let call = format!("{}", rule.call());
        let weight = format!("{:.1}", rule.weight());
        println!("    {call:>6}  w{weight:<4} {description}{note}");
    }
}

fn main() {
    let pair = dutch_book();
    let books: [(&str, &Trie); 3] = [
        ("constructive", &pair.constructive.0),
        ("competitive", &pair.competitive.0),
        ("defensive", &pair.defensive.0),
    ];

    let mut seen: HashSet<usize> = HashSet::new();
    let mut nodes = 0usize;
    let mut sections = 0usize;
    let mut opaque = 0usize;
    let mut unlabeled = 0usize;

    for (book, trie) in books {
        println!("\n═════════════════  {book}  ═════════════════");
        for (auction, classifier) in trie.iter() {
            let Some(rules) = classifier.as_rules() else {
                continue;
            };
            // Dedupe by the authored-rules object: shared seat variants of one
            // table classify through the same `Arc` (see `export-corpus`).
            let id = core::ptr::from_ref(classifier) as *const () as usize;
            if !seen.insert(id) {
                continue;
            }
            nodes += 1;

            let auction_str = if auction.is_empty() {
                "—  (opening)".to_string()
            } else {
                contract_bridge::auction::display_calls(&auction).to_string()
            };
            println!("\n{auction_str}");
            print_rules(rules, &mut opaque);
        }

        // Guarded fallbacks: the same walk, headed by node auction + guard
        // description.  Seat variants share one `Arc` — first-seen dedup keeps
        // the canonical pass-less key (`Trie::fallbacks` visits it first).
        for (auction, guard, fallback) in trie.fallbacks() {
            let id = match fallback {
                Fallback::Classify(c) => Arc::as_ptr(c).cast::<()>() as usize,
                Fallback::Rebase(r) => Arc::as_ptr(r).cast::<()>() as usize,
            };
            if !seen.insert(id) {
                continue;
            }
            sections += 1;

            let condition = guard.describe().unwrap_or_else(|| {
                unlabeled += 1;
                "(unlabeled guard)".to_string()
            });
            let auction_str = contract_bridge::auction::display_calls(&auction).to_string();
            let heading = format!("{auction_str} {condition}");
            println!("\n{}", heading.trim());

            match fallback {
                Fallback::Classify(classifier) => match classifier.as_rules() {
                    Some(rules) => print_rules(rules, &mut opaque),
                    None => println!("    (computed table)"),
                },
                Fallback::Rebase(rewrite) => {
                    let summary = rewrite.describe().unwrap_or_else(|| {
                        unlabeled += 1;
                        "(opaque rewrite)".to_string()
                    });
                    println!("    → {summary}");
                }
            }
        }
    }

    eprintln!(
        "\nrender-dutch-book: {nodes} authored nodes and {sections} guarded sections printed, \
         {opaque} rules still opaque, {unlabeled} guards unlabeled."
    );
}
