//! Throwaway: render-book's walk with every batch-6 knob armed
//!
//! Batch 6 ports `insert_sohl_over` / `insert_advance_of_double` to row
//! producers.  Both consumers ride default-off knobs — `set_advance_sohl_style`
//! and `set_nt_overcall_gladiator` — so the default `render-book` diff sees
//! neither.  This arms all three branches (`Plain`, `Transfer` + delayed cue,
//! Gladiator) and prints the alert slug on each line.  Delete after the batch
//! is blessed.

use pons::bidding::american::{
    LebensohlStyle, american_book, set_advance_sohl_style, set_delayed_cue,
    set_nt_overcall_gladiator,
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
    // Arm 1: the Plain sohl advance of a weak-two double.  Arm 2: Transfer, with
    // the stopper-split delayed cue on.  Arm 3: Gladiator, whose (2♦/2♥/2♠) tail
    // is the other `sohl_rows_over` consumer.
    render("advance-sohl-plain", || {
        set_advance_sohl_style(LebensohlStyle::Plain)
    });
    render("advance-sohl-transfer+delayed-cue", || {
        set_advance_sohl_style(LebensohlStyle::Transfer);
        set_delayed_cue(true);
    });
    render("gladiator", || set_nt_overcall_gladiator(true));
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
