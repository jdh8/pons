//! Throwaway: render-book's walk with the Woolsey defense armed
//!
//! `woolsey_package` rides `notrump_defense() == Woolsey`, which is off by
//! default, so the default `render-book` diff sees none of it.  This arms it
//! and prints the alert slug on each line, so an alert change is visible too.
//! Delete after the batch is blessed.

use pons::bidding::american::{NotrumpDefense, american_book, set_notrump_defense};
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
    render("woolsey", || set_notrump_defense(NotrumpDefense::Woolsey));
}

fn render(arm: &str, arms: impl FnOnce()) {
    arms();
    println!("\n########  {arm}  ########");
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
