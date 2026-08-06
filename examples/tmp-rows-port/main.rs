//! Throwaway: the constructive port campaign's knob-armed, **no-dedup** book dump
//!
//! `render-book` dedupes nodes by classifier pointer identity, so it cannot be
//! the campaign's only oracle: a port trades cross-key `Arc` sharing for one
//! `Arc` per pattern, which changes *how many times* render-book prints a table
//! without changing what the book bids.  This dump keys on the auction instead,
//! so it sees every node at every seat and is blind to allocation identity — a
//! strictly stronger byte-identity check.
//!
//! It also arms one knob per run.  Almost every gate in the port tail is
//! default-*on*, so its rows are already in the default dump and the default
//! pair of campaign shas is structurally blind to whether `if crawling_stayman()
//! { … }` became `gate: crawling_stayman` or was inlined away.  The *off* arm is
//! what proves the gate read survived the port, which is why almost every arm
//! below is a `(false)`.
//!
//! With no argument it lists the arms, so the sweep loop has one source of
//! truth and a shell array can never go stale:
//!
//! ```text
//! P=./target/release/examples/tmp-rows-port
//! for a in $($P); do "$P" "$a" > "$D/before/$a"; done
//! ```
//!
//! Delete at T1, with the campaign's other throwaways.

use pons::bidding::american::american_book;
use pons::bidding::fallback::Fallback;
use pons::bidding::rules::Rules;
use pons::bidding::trie::Trie;

/// The arms, one per gate the port's batches translate
///
/// Every arm must move its knob **away from** the `Cell::new(…)` default in its
/// `thread_local!` block, or its dump equals `default`'s and proves nothing.
/// The retired `tmp-rows-rkcb` shipped four such arms — `game-tries` and
/// `limit-raise` set knobs to their own `Cell::new(true)` defaults, `kickback`
/// armed a floor knob no book node reads — and all four hashed identical to
/// `default` under a blessing that said "diff empty".  The sweep's non-vacuity
/// loop now catches that mechanically.
const ARMS: &[(&str, fn())] = &[
    ("default", || {}),
    // N1 — notrump.rs 3096–3238
    ("no-stayman-cue", || {
        pons::bidding::american::set_stayman_cue_continuation(false)
    }),
    ("no-stayman-minor-slam", || {
        pons::bidding::american::set_stayman_minor_slam_try(false)
    }),
    ("no-crawling", || {
        pons::bidding::american::set_crawling_stayman(false)
    }),
    // N2 — notrump.rs 3239–3429
    ("no-inv-5card", || {
        pons::bidding::american::set_invitational_5card_majors(false)
    }),
    // `transfer_slam_try` alone is vacuous: its two sites are
    // `transfer_slam_try() && !transfer_gf_hearts()` (already false, hearts is
    // on) and `transfer_slam_try() || transfer_gf_majors()` (already true).  It
    // is observable only with both GF knobs off, so its arm is the triple.
    ("no-transfer-suite", || {
        pons::bidding::american::set_transfer_slam_try(false);
        pons::bidding::american::set_transfer_gf_majors(false);
        pons::bidding::american::set_transfer_gf_hearts(false);
    }),
    ("no-transfer-gf-majors", || {
        pons::bidding::american::set_transfer_gf_majors(false)
    }),
    ("no-transfer-gf-hearts", || {
        pons::bidding::american::set_transfer_gf_hearts(false)
    }),
    // `sixcard_invite_active()` is `sixcard_invite_floor() < texas_game_floor()`,
    // 13 < 14 by default — a *numeric* gate, so its off arm closes the band
    // rather than flipping a bool.
    ("no-sixcard-invite", || {
        pons::bidding::american::set_sixcard_invite_floor(14)
    }),
    // N3 — notrump.rs 3430–3534.  `european` is the anti-gate of the `puppet`
    // local, and covers N4's two `else` arms as well as N3's one.
    ("european", || {
        pons::bidding::american::set_notrump_minors(pons::bidding::american::EUROPEAN)
    }),
    ("no-both-majors", || {
        pons::bidding::american::set_stayman_both_majors(false)
    }),
    ("no-5card-max", || {
        pons::bidding::american::set_stayman_5card_max(false)
    }),
    ("no-nt-splinter", || {
        pons::bidding::american::set_nt_splinter(false)
    }),
    // N4 — notrump.rs 3535–3608
    ("no-texas-slam-drive", || {
        pons::bidding::american::set_texas_slam_drive(false)
    }),
    // N5 — `register_two_nt_and_rebids` reads no gate at all.
    // R1 — rebids.rs.  All ten translated knobs ship on, so their off arms
    // exercise the construction-time gates.  The Meckstroth adjunct and
    // major-rebid tails remain on while isolating their respective child knobs.
    ("no-meckstroth", || {
        pons::bidding::american::set_meckstroth_adjunct(false)
    }),
    ("no-meckstroth-minor-jumps", || {
        pons::bidding::american::set_meckstroth_minor_jumps(false)
    }),
    ("no-forcing-nt-two-suiter", || {
        pons::bidding::american::set_forcing_nt_two_suiter(false)
    }),
    ("no-balanced-1nt-rebid", || {
        pons::bidding::american::set_balanced_1nt_rebid(false)
    }),
    ("no-opener-extras-ladder", || {
        pons::bidding::american::set_opener_extras_ladder(false)
    }),
    ("no-opener-major-jump-rebid", || {
        pons::bidding::american::set_opener_major_jump_rebid(false)
    }),
    ("no-major-rebid-tails", || {
        pons::bidding::american::set_major_rebid_tails(false)
    }),
    ("no-fourth-suit-forcing", || {
        pons::bidding::american::set_fourth_suit_forcing(false)
    }),
    ("no-nt-invite-hcp", || {
        pons::bidding::american::set_nt_invite_hcp(false)
    }),
    ("no-up-the-line", || {
        pons::bidding::american::set_up_the_line(false)
    }),
    // S1 — strong_two.rs.  The same knob drives the minor-raise ask tables
    // and their RKCB answer subtrees, so its off arm proves both halves move
    // together through the row port.
    ("no-minor-keycard", || {
        pons::bidding::instinct::set_rkcb_minors(false)
    }),
    // P1 — responses.rs.  The other translated gate is the choice-of-games
    // continuation after 1M–3NT; its off arm must remove both major nodes.
    ("no-major-choice-of-games", || {
        pons::bidding::american::set_major_choice_of_games(false)
    }),
    // G1 — game_force.rs.  The opener-third table and its RKCB answer tree
    // deliberately have different gates, while the second-suit agreement
    // moves its table and answer tree together.  The retired backstop is the
    // positive arm because it defaults off.
    ("no-opener-third", || {
        pons::bidding::american::set_opener_third(false)
    }),
    ("no-second-suit-agreement", || {
        pons::bidding::american::set_second_suit_agreement(false)
    }),
    ("game-backstop", || {
        pons::bidding::american::set_game_backstop(true)
    }),
];

/// Dump one table, **including each rule's alert**
///
/// `label()` is a human-readable note, not the alert, and neither `render-book`
/// nor this dump used to print `alert()` at all — so the campaign's oracle was
/// blind to an alert being dropped in translation.  That is not hypothetical:
/// the alert is load-bearing twice over, once for disclosure and once through
/// `Rules::gated`, which retains an alerted rule only while its alert is
/// active.  N1–N3 were safe (they added and removed zero `.alert(` lines, and
/// alerts live only inside table builders the port never edits), but the column
/// is cheap and the blindness was not worth keeping.
fn print_rules(book: &str, auction: &str, kind: &str, rules: &Rules) {
    for rule in rules.rules() {
        println!(
            "{book}\t{auction}\t{kind}\t{}\t{:.3}\t{}\t{}\t{:?}",
            rule.call(),
            rule.weight(),
            rule.describe(),
            rule.label(),
            rule.alert(),
        );
    }
}

fn main() {
    let Some(arm) = std::env::args().nth(1) else {
        for (name, _) in ARMS {
            println!("{name}");
        }
        return;
    };
    let (_, engage) = ARMS
        .iter()
        .find(|(name, _)| *name == arm)
        .unwrap_or_else(|| panic!("unknown arm {arm:?}"));
    engage();

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
