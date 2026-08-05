//! Throwaway: a **no-dedup** book dump, one line per rule, knob-armed.
//!
//! `render-book` dedupes nodes by classifier pointer identity, so it cannot be
//! the oracle for the RKCB row port: the port trades cross-key `Arc` sharing
//! (`shared_king_answers` spans four answer paths) for one `Arc` per pattern,
//! which changes *how many times* render-book prints a table without changing
//! what the book bids.  This dump keys on the auction instead, so it sees every
//! node at every seat and is blind to allocation identity — a strictly stronger
//! byte-identity check.
//!
//! ```text
//! cargo run --release --example tmp-rows-rkcb -- <arm> | sort > arm.txt
//! ```
//!
//! Delete after the batch is blessed.

use pons::bidding::american::american_book;
use pons::bidding::fallback::Fallback;
use pons::bidding::rules::Rules;
use pons::bidding::trie::Trie;

fn print_rules(book: &str, auction: &str, kind: &str, rules: &Rules) {
    for rule in rules.rules() {
        println!(
            "{book}\t{auction}\t{kind}\t{}\t{:.3}\t{}\t{}",
            rule.call(),
            rule.weight(),
            rule.describe(),
            rule.label(),
        );
    }
}

fn main() {
    match std::env::args().nth(1).unwrap_or_default().as_str() {
        "default" => {}
        "rkcb-minors" => pons::bidding::instinct::set_rkcb_minors(true),
        "no-rkcb-minors" => pons::bidding::instinct::set_rkcb_minors(false),
        "kickback" => pons::bidding::instinct::set_rkcb_variant(
            pons::bidding::instinct::RkcbVariant::Kickback,
        ),
        "game-tries" => pons::bidding::american::set_major_game_tries(true),
        "limit-raise" => pons::bidding::american::set_limit_raise_acceptance(true),
        "no-opener-third" => pons::bidding::american::set_opener_third(false),
        other => panic!("unknown arm {other:?}"),
    }

    let pair = american_book();
    let books: [(&str, &Trie); 3] = [
        ("constructive", &pair.constructive.0),
        ("competitive", &pair.competitive.0),
        ("defensive", &pair.defensive.0),
    ];

    for (book, trie) in books {
        for (auction, classifier) in trie.iter() {
            let Some(rules) = classifier.as_rules() else {
                continue;
            };
            let auction = contract_bridge::auction::display_calls(&auction).to_string();
            print_rules(book, &auction, "node", rules);
        }
        for (auction, guard, fallback) in trie.fallbacks() {
            let auction = contract_bridge::auction::display_calls(&auction).to_string();
            let condition = guard
                .describe()
                .unwrap_or_else(|| "(unlabeled)".to_string());
            let kind = format!("guard[{condition}]");
            match fallback {
                Fallback::Classify(classifier) => match classifier.as_rules() {
                    Some(rules) => print_rules(book, &auction, &kind, rules),
                    None => println!("{book}\t{auction}\t{kind}\t(computed table)"),
                },
                Fallback::Rebase(rewrite) => {
                    let summary = rewrite.describe().unwrap_or_else(|| "(opaque)".to_string());
                    println!("{book}\t{auction}\t{kind}\t→ {summary}");
                }
            }
        }
    }
}
