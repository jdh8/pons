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
//! `--prefix "1NT (2♦)"` cuts it to one lane's subtree, `--no-ns-multi-kokish-kraft`
//! swaps that lane back from the shipped §N4-KK table to v7, and `--their-2d-multi`
//! declares their `2♦` a Multi first, so the N4 tables are in the book at all
//! ([docs/one-notrump-multi.md](../../docs/one-notrump-multi.md)).  The `(2♣)`
//! lane is the same shape: `--their-2c-landy` declares their `2♣` a Landy, so
//! the N1j counter tables are in the book at all
//! ([docs/one-notrump-competitive.md](../../docs/one-notrump-competitive.md)).

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
    /// Only print nodes whose auction starts with this, e.g. `"1NT (2♦)"`
    #[arg(long, default_value = "")]
    prefix: String,

    /// Declare their `2♦` a Multi (`their.two_diamonds_multi`), which swaps the
    /// natural `(2♦)` leg for the N4 tables
    #[arg(long, default_value_t = false)]
    their_2d_multi: bool,

    /// Declare their `2♣` a Landy (`their.two_clubs_landy`), which swaps the
    /// natural `(2♣)` leg for the N1j counter tables
    #[arg(long, default_value_t = false)]
    their_2c_landy: bool,

    /// Fall back to the v7 N4 tables, disabling the shipped Kokish–Kraft
    /// counter (`competition.multi_kokish_kraft`) — needs `--their-2d-multi`
    /// to do anything (§N4-KK)
    #[arg(long, default_value_t = false)]
    no_ns_multi_kokish_kraft: bool,

    /// The `points` floor of the `4m` slam try above a completed Kokish–Kraft
    /// minor transfer: a `points` floor (default `15`), or `off` — `13` is
    /// `landy_bba_transfer_rebid`'s own rung, `15` the narrow arm
    ///
    /// Also authors opener's answer (`4NT` RKCB on a maximum, else `5m`) and,
    /// on the same switch, the shortness `4m` when they compete over the
    /// completion (§N4-KK residues 3 and 6, `docs/minor-transfer-slam.md`).
    /// Needs the Kokish–Kraft counter and their declared `(2♦)` Multi to do
    /// anything.
    #[arg(long, default_value = "15", value_name = "off|13|15")]
    ns_multi_minor_slam_try: String,

    /// Withhold the Kokish–Kraft doubler's **natural bid of the other major**
    /// once their pass-or-correct resolves theirs
    ///
    /// Turns *off* the rung shipped default-on 2026-08-26
    /// (`competition.multi_doubler_major`): `2♠` over their `(2♥)`, `3♥` over
    /// their `(2♠)`, on four-plus of the other major at weight 100 — below
    /// every existing rung, so it fires only on the hands that would otherwise
    /// pass out their resolved partscore.  Withheld from `X (2♥) - (2♠)`,
    /// where opener's pass has already denied four hearts.  Opener answers with
    /// game from the top of the range, the invitational raise where there is
    /// room, else a pass.  This is the control arm of
    /// `scripts/ab-2d-multi-doubler.sh`.
    #[arg(long, default_value_t = false)]
    no_ns_multi_doubler_major: bool,
    /// Split responder's `P`/`X` over their `(2♦)` Multi by **information**:
    /// `X` = game values, or an invitation with a four-card major
    ///
    /// `hcp(10..) | (hcp(8..=9) & (len(♥, 4..) | len(♠, 4..)))` in place of
    /// Kokish–Kraft's flat `hcp 8+`.  The 8–9-no-major hands take the neutral
    /// pass instead, where the delayed `2NT` becomes a live invitation (opener
    /// accepts on `hcp 16+`), and the doubler's natural other major becomes
    /// required at weight 148 — above the natural `2NT` — on three of the four
    /// resolved paths.  Implies `--ns-multi-doubler-major`'s rung and
    /// re-weights it; the two emit one rung.  Needs the Kokish–Kraft counter
    /// and their declared `(2♦)` Multi to do anything.
    #[arg(long, default_value_t = false)]
    ns_multi_px_split: bool,

    /// The `4m` slam try above a completed **Puppet** minor transfer
    /// (`1NT - 2♠`→♣, `1NT - 2NT`→♦): a `points` floor (default `13`), or `off`
    ///
    /// The shipped constructive twin of
    /// `--ns-multi-minor-slam-try`.  Authors the rung in all four Puppet seats
    /// plus opener's answer (`4NT` RKCB on `size_ask_accept_floor`, else `5m`).
    /// The European arm is an opponent model and never carries it.
    #[arg(long, default_value = "13", value_name = "off|POINTS")]
    ns_minor_transfer_slam_try: String,

    /// Leave opener's N1j Landy `4m` slam try to the floor instead of using
    /// the shipped authored answer (`1NT (2♣) 2NT - 3♣ - 4♣ -`)
    ///
    /// The rung itself has shipped since N1; this restores the former
    /// floor-owned seat above it — and the floor can never keycard in a
    /// disturbed auction (`docs/minor-transfer-slam.md`).
    #[arg(long, default_value_t = false)]
    no_ns_landy_minor_slam_answer: bool,

    /// Author opener's notrump out over the N1j Landy doubler's advanced major
    /// (`1NT (2♣) X (2♥)`, `1NT (2♣) X (2♠)`)
    ///
    /// `competition.landy_doubler_notrump`, default **off**: `3NT` on
    /// `hcp(16..) & stopper_in(their major)` where the seat is the floor's
    /// today and it passes.  The treatment arm of
    /// `scripts/ab-landy-doubler-nt.sh`.
    #[arg(long, default_value_t = false)]
    ns_landy_doubler_notrump: bool,

    /// Author the N1j Landy doubler's own rebid once their advance has named
    /// the major (`1NT (2♣) X (2♥) - -` and its three siblings)
    ///
    /// `competition.landy_doubler_rebids`, default **off**: the Kokish–Kraft
    /// doubler ladder ported one suit down — `4NT` quantitative, the penalty
    /// `X` on four of their major, `3NT`, the `2NT` invitation and natural
    /// `3♣`/`3♦` — where the seat is the floor's today and the auction dies.
    #[arg(long, default_value_t = false)]
    ns_landy_doubler_rebids: bool,

    /// Grade the N1j Landy both-minor splinters on high cards, not points
    ///
    /// `competition.landy_splinter_hcp`, default **off**: `3♥`/`3♠` take
    /// `hcp(10..)` instead of `points(10..)`, so a shortness-inflated 8-9 HCP
    /// hand doubles rather than splintering.  The only branch of the `(2♣)`
    /// lane negative on both scorers (23 bd, −18 plain / −32 PD).
    #[arg(long, default_value_t = false)]
    ns_landy_splinter_hcp: bool,

    /// Author the N1j Landy lane's three unfinished tails as one batch
    ///
    /// `competition.landy_tail_completion`, default **off**: their overcall of
    /// our minor transfer (`3♣ (3♠)`, 6 bd at −38 PD), the five floored
    /// four-level seats of the 2026-08-25 survey plus their RKCB ladders, and
    /// the manufactured-`4♣` repair in `landy_bba_ask_answer`.
    #[arg(long, default_value_t = false)]
    ns_landy_tail_completion: bool,

    /// Play the full Kokish–Kraft minor core over their `(2♣)` Landy
    ///
    /// `competition.defense_2c_landy_kk`, default **off**: `2♥`/`2♠` both
    /// minors split by strength, `2NT` a weak either-minor escape relay,
    /// `3♣`/`3♦` game-forcing cross-transfers.  Replaces the shipped N1j ladder's
    /// minor structure; `3NT`, the values `X`, the weak `2♦` and the splinters
    /// carry over verbatim.  Wants `--ns-landy-doubler-rebids` under it — 8-9
    /// one-suited minors have no direct call in this table.
    #[arg(long, default_value_t = false)]
    ns_defense_2c_landy_kk: bool,
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
    let mut args = Args::parse();
    args.prefix.retain(|c| !matches!(c, '(' | ')'));
    let mut agreements = pons::bidding::agreements::Agreements::default();
    agreements.decision.their.two_diamonds_multi = args.their_2d_multi;
    agreements.decision.their.two_clubs_landy = args.their_2c_landy;
    agreements.competition.multi_kokish_kraft = !args.no_ns_multi_kokish_kraft;
    agreements.competition.multi_minor_slam_try = match args.ns_multi_minor_slam_try.as_str() {
        "off" => None,
        n => Some(
            n.parse()
                .expect("--ns-multi-minor-slam-try must be off or a points floor"),
        ),
    };
    agreements.competition.multi_doubler_major = !args.no_ns_multi_doubler_major;
    agreements.competition.multi_px_split = args.ns_multi_px_split;
    agreements.notrump.minor_transfer_slam_try = match args.ns_minor_transfer_slam_try.as_str() {
        "off" => None,
        n => Some(
            n.parse()
                .expect("--ns-minor-transfer-slam-try must be off or a points floor"),
        ),
    };
    agreements.competition.landy_minor_slam_answer = !args.no_ns_landy_minor_slam_answer;
    agreements.competition.landy_doubler_notrump = args.ns_landy_doubler_notrump;
    agreements.competition.landy_doubler_rebids = args.ns_landy_doubler_rebids;
    agreements.competition.landy_splinter_hcp = args.ns_landy_splinter_hcp;
    agreements.competition.landy_tail_completion = args.ns_landy_tail_completion;
    agreements.competition.defense_2c_landy_kk = args.ns_defense_2c_landy_kk;
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
