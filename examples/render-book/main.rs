//! Book pretty-printer (AI-bidder M4)
//!
//! Walks the floor-less 2/1 books ([`bare_two_over_one`]) and prints every
//! authored node as readable prose: each auction, then per rule its call,
//! weight, and the **constraint's own** English description
//! ([`Rule::describe`][pons::bidding::Rules]).  Unlike the corpus exporter's
//! structural gloss, the meaning here is read straight from the logic the book
//! bids on, so author and reader cannot drift.
//!
//! A rule with no readable constraint (a bare
//! [`pred`][pons::bidding::constraint::pred]) prints `(opaque condition)`; the
//! stderr summary counts them as a coverage metric — labeling such predicates
//! with [`described`][pons::bidding::constraint::described] drives it to zero.
//!
//! Run with `cargo run --example render-book` (pipe to a pager — it is long).

use pons::bidding::constraint::Description;
use pons::bidding::trie::Trie;
use pons::bidding::two_over_one::bare_two_over_one;
use std::collections::HashSet;

fn main() {
    let pair = bare_two_over_one();
    let books: [(&str, &Trie); 3] = [
        ("constructive", &pair.constructive.0),
        ("competitive", &pair.competitive.0),
        ("defensive", &pair.defensive.0),
    ];

    let mut seen: HashSet<usize> = HashSet::new();
    let mut nodes = 0usize;
    let mut opaque = 0usize;

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
                auction
                    .iter()
                    .map(|call| format!("{call}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            println!("\n{auction_str}");

            for rule in rules.rules() {
                let description = rule.describe();
                if matches!(description, Description::Opaque) {
                    opaque += 1;
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
    }

    eprintln!("\nrender-book: {nodes} authored nodes printed, {opaque} rules still opaque.");
}
