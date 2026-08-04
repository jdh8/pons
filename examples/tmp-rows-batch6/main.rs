//! Throwaway: render-book's walk with every batch-6 knob armed
//!
//! The default `render-book` diff cannot see Section 9, which rides
//! `NegativeDoubleShape::Cachalot` (default `Modern`).  This arms it — with
//! the contested-`X` sub-knob explicitly on — and prints the alert slug on
//! each line, so an alert change is visible too.  Delete after the batch is
//! blessed.

use pons::bidding::american::{
    NegativeDoubleShape, american_book, set_cachalot_contested_x, set_negative_double_shape,
};
use pons::bidding::fallback::Fallback;
use pons::bidding::rules::Rules;
use pons::bidding::trie::Trie;
use std::collections::HashSet;
use std::sync::Arc;

fn print_rules(rules: &Rules) {
    for rule in rules.rules() {
        println!(
            "    {:>6}  w{:<4} {} [{}] <{:?}>",
            format!("{}", rule.call()),
            format!("{:.1}", rule.weight()),
            rule.describe(),
            rule.label(),
            rule.alert(),
        );
    }
}

fn main() {
    set_negative_double_shape(NegativeDoubleShape::Cachalot);
    set_cachalot_contested_x(true);

    let pair = american_book();
    let books: [(&str, &Trie); 3] = [
        ("constructive", &pair.constructive.0),
        ("competitive", &pair.competitive.0),
        ("defensive", &pair.defensive.0),
    ];

    let mut seen: HashSet<usize> = HashSet::new();
    for (book, trie) in books {
        println!("\n═════════════════  {book}  ═════════════════");
        for (auction, classifier) in trie.iter() {
            let Some(rules) = classifier.as_rules() else {
                continue;
            };
            let id = core::ptr::from_ref(classifier) as *const () as usize;
            if !seen.insert(id) {
                continue;
            }
            println!("\n{}", contract_bridge::auction::display_calls(&auction));
            print_rules(rules);
        }
        for (auction, guard, fallback) in trie.fallbacks() {
            let id = match fallback {
                Fallback::Classify(c) => Arc::as_ptr(c).cast::<()>() as usize,
                Fallback::Rebase(r) => Arc::as_ptr(r).cast::<()>() as usize,
            };
            if !seen.insert(id) {
                continue;
            }
            let condition = guard
                .describe()
                .unwrap_or_else(|| "(unlabeled)".to_string());
            println!(
                "\n{} {condition}",
                contract_bridge::auction::display_calls(&auction)
            );
            match fallback {
                Fallback::Classify(classifier) => match classifier.as_rules() {
                    Some(rules) => print_rules(rules),
                    None => println!("    (computed table)"),
                },
                Fallback::Rebase(rewrite) => println!(
                    "    → {}",
                    rewrite.describe().unwrap_or_else(|| "(opaque)".to_string())
                ),
            }
        }
    }
}
