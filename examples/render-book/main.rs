//! Book pretty-printer (AI-bidder M4)
//!
//! Walks the floor-less 2/1 books ([`american_book`]) and prints every
//! authored node as readable prose: each auction, then per rule its call,
//! weight, and the **constraint's own** English description
//! ([`Rule::describe`][pons::bidding::Rules]).  Unlike the corpus exporter's
//! structural gloss, the meaning here is read straight from the logic the book
//! bids on, so author and reader cannot drift.
//!
//! Guarded fallbacks — the competitive book's whole substance — print after
//! the exact nodes: the heading is the node's auction plus the guard's own
//! description (a [`SuffixIs`][pons::bidding::fallback::SuffixIs] guard reads
//! like more auction), the body is the attached rules table, or the rebase's
//! summary for a systems-on rewrite.
//!
//! A rule with no readable constraint (a bare
//! [`pred`][pons::bidding::constraint::pred]) prints `(opaque condition)`; the
//! stderr summary counts them as a coverage metric — labeling such predicates
//! with [`described`][pons::bidding::constraint::described] drives it to zero.
//! Unlabeled guards are counted the same way.
//!
//! Run with `cargo run --example render-book` (pipe to a pager — it is long).
//! `--prefix "1NT 2♦"` cuts it to one lane's subtree, `--no-ns-multi-kokish-kraft`
//! swaps that lane back from the shipped §N4-KK table to v7, and `--their-2d-multi`
//! declares their `2♦` a Multi first, so the N4 tables are in the book at all
//! ([docs/one-notrump-multi.md](../../docs/one-notrump-multi.md)).

use clap::Parser;
use pons::bidding::american::american_book;
use pons::bidding::constraint::Description;
use pons::bidding::fallback::Fallback;
use pons::bidding::rules::Rules;
use pons::bidding::trie::Trie;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Parser)]
struct Args {
    /// Only print nodes whose auction starts with this, e.g. `"1NT 2♦"`
    #[arg(long, default_value = "")]
    prefix: String,

    /// Declare their `2♦` a Multi (`their.two_diamonds_multi`), which swaps the
    /// natural `(2♦)` leg for the N4 tables
    #[arg(long, default_value_t = false)]
    their_2d_multi: bool,

    /// Fall back to the v7 N4 tables, disabling the shipped Kokish–Kraft
    /// counter (`competition.multi_kokish_kraft`) — needs `--their-2d-multi`
    /// to do anything (§N4-KK)
    #[arg(long, default_value_t = false)]
    no_ns_multi_kokish_kraft: bool,

    /// The `points` floor of the `4m` slam try above a completed Kokish–Kraft
    /// minor transfer: `off` (default), or a floor — `13` is
    /// `landy_bba_transfer_rebid`'s own rung, `15` the narrow arm
    ///
    /// Also authors opener's answer (`4NT` RKCB on a maximum, else `5m`) and,
    /// on the same switch, the shortness `4m` when they compete over the
    /// completion (§N4-KK residues 3 and 6, `docs/minor-transfer-slam.md`).
    /// Needs the Kokish–Kraft counter and their declared `(2♦)` Multi to do
    /// anything.
    #[arg(long, default_value = "off", value_name = "off|13|15")]
    ns_multi_minor_slam_try: String,
}

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
    let args = Args::parse();
    let mut agreements = pons::bidding::agreements::Agreements::default();
    agreements.decision.their.two_diamonds_multi = args.their_2d_multi;
    agreements.competition.multi_kokish_kraft = !args.no_ns_multi_kokish_kraft;
    agreements.competition.multi_minor_slam_try = match args.ns_multi_minor_slam_try.as_str() {
        "off" => None,
        n => Some(
            n.parse()
                .expect("--ns-multi-minor-slam-try must be off or a points floor"),
        ),
    };
    let system = american_book(&agreements);
    let books: [(&str, &Trie); 3] = [
        ("constructive", &system.constructive.0),
        ("competitive", &system.competitive.0),
        ("defensive", &system.defensive.0),
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
            let auction_str = if auction.is_empty() {
                "—  (opening)".to_string()
            } else {
                contract_bridge::auction::display_calls(&auction).to_string()
            };
            if !auction_str.starts_with(&args.prefix) {
                continue;
            }
            // Dedupe by the authored-rules object: shared seat variants of one
            // table classify through the same `Arc` (see `export-corpus`).
            // After the filter, so `--prefix` keeps the first *matching* key
            // rather than losing a table to an earlier non-matching one.
            let id = core::ptr::from_ref(classifier) as *const () as usize;
            if !seen.insert(id) {
                continue;
            }
            nodes += 1;
            println!("\n{auction_str}");
            print_rules(rules, &mut opaque);
        }

        // Guarded fallbacks: the same walk, headed by node auction + guard
        // description.  Seat variants share one `Arc` — first-seen dedup keeps
        // the canonical pass-less key (`Trie::fallbacks` visits it first).
        for (auction, guard, fallback) in trie.fallbacks() {
            let auction_str = contract_bridge::auction::display_calls(&auction).to_string();
            // Filter on the whole heading: a `SuffixIs` guard reads like more
            // auction, so `1NT 2♦ X -` is the node `1NT` plus the guard's text.
            let condition = guard.describe();
            let heading = format!(
                "{auction_str} {}",
                condition.as_deref().unwrap_or("(unlabeled guard)")
            );
            let heading = heading.trim();
            if !heading.starts_with(&args.prefix) {
                continue;
            }
            let id = match fallback {
                Fallback::Classify(c) => Arc::as_ptr(c).cast::<()>() as usize,
                Fallback::Rebase(r) => Arc::as_ptr(r).cast::<()>() as usize,
            };
            if !seen.insert(id) {
                continue;
            }
            sections += 1;
            unlabeled += usize::from(condition.is_none());
            println!("\n{heading}");

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
        "\nrender-book: {nodes} authored nodes and {sections} guarded sections printed, \
         {opaque} rules still opaque, {unlabeled} guards unlabeled."
    );
}
