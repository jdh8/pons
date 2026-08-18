//! One decision, explained: what the live [`pons::american`] partnership reads
//! about partner and RHO at an auction, which node answers (provenance), and
//! the top logits with the rule that produced each — the CLI twin of
//! `Partnership::explain_call`.
//!
//! ```text
//! cargo run --release --example probe-decision -- "Q93.K43.AKJT.Q42" "1NT 2♠ 2NT - 3♣ - 3♦ -" [none|both|we|they]
//! ```
//!
//! `PROBE_FLOOR=instinct` swaps the shipped net floor for the deterministic one,
//! which is what an anchor `floor#N` row was generated under.
//!
//! Born on the N2 forensic (`docs/one-notrump-competitive.md` §N2): opener
//! after the weak Lebensohl sign-off read partner as `hcp 6..37`, every suit
//! `0..13`, and the floor bid `3NT`.

use contract_bridge::Hand;
use contract_bridge::auction::{Call, RelativeVulnerability};
use pons::bidding::agreements::Agreements;
use pons::{american, american_instinct};

fn main() {
    let mut args = std::env::args().skip(1);
    let hand: Hand = args.next().expect("hand").parse().expect("hand parses");
    let auction: Vec<Call> = args
        .next()
        .expect("auction")
        .split_whitespace()
        .map(|c| match c {
            "-" | "P" => Call::Pass,
            "X" => Call::Double,
            "XX" => Call::Redouble,
            other => Call::Bid(other.parse().expect("bid parses")),
        })
        .collect();
    let vul: RelativeVulnerability =
        args.next()
            .map_or(RelativeVulnerability::NONE, |v| match v.as_str() {
                "both" => RelativeVulnerability::ALL,
                "we" | "us" => RelativeVulnerability::WE,
                "they" => RelativeVulnerability::THEY,
                _ => RelativeVulnerability::NONE,
            });
    let mut agreements = Agreements::default();
    // `ReadingScope::All` is the default since 2026-08-16; `PROBE_SCOPE=alerted`
    // (or `none`) shows what the alert gate used to hide.
    match std::env::var("PROBE_SCOPE").as_deref() {
        Ok("all") => {
            agreements.decision.reading.scope = pons::bidding::inference::ReadingScope::All
        }
        Ok("alerted") => {
            agreements.decision.reading.scope = pons::bidding::inference::ReadingScope::Alerted;
        }
        Ok("none") => {
            agreements.decision.reading.scope = pons::bidding::inference::ReadingScope::None
        }
        _ => {}
    }
    // Each made call's strength ceilings, not just its floors — on by default
    // since 2026-08-16, so `PROBE_CEILINGS=0` is the interesting one: it puts
    // the sign-off above back to reading `hcp 6..37`.
    match std::env::var("PROBE_CEILINGS").as_deref() {
        Ok("0") => agreements.decision.reading.strength_ceilings = false,
        Ok("1") => agreements.decision.reading.strength_ceilings = true,
        _ => {}
    }
    // Sibling-gate exclusion on made bids (Phase 4 of
    // docs/authored-reading-handoff.md) is default ON since 2026-08-17, so
    // `PROBE_BID_EXCLUSION=0` is the interesting one: it puts a bid made
    // through one rule back to reading only that rule (plus the natural walk
    // where the rule was a catch-all), instead of also outside every
    // strictly-heavier sibling gate it declined.
    match std::env::var("PROBE_BID_EXCLUSION").as_deref() {
        Ok("0") => agreements.decision.reading.bid_exclusion = false,
        Ok("1") => agreements.decision.reading.bid_exclusion = true,
        _ => {}
    }
    // The instinct force's ceiling read is default ON since 2026-08-16, so
    // `PROBE_FORCING_CEILING=0` is the interesting one: it puts the Lebensohl
    // sign-off above back to forcing us to game on auction shape alone.
    match std::env::var("PROBE_FORCING_CEILING").as_deref() {
        Ok("0") => agreements.decision.instinct.forcing_ceiling_read = false,
        Ok("1") => agreements.decision.instinct.forcing_ceiling_read = true,
        _ => {}
    }
    // `PROBE_UPGRADE_CLOSURE=1` closes `hcp` against `points` through the shape
    // upgrade (C2 of docs/dnf-migration.md): a box whose lengths force balanced
    // reads `points == hcp` instead of carrying the scale's 2-HCP slack.
    match std::env::var("PROBE_UPGRADE_CLOSURE").as_deref() {
        Ok("0") => agreements.decision.reading.upgrade_closure = false,
        Ok("1") => agreements.decision.reading.upgrade_closure = true,
        _ => {}
    }
    // The anchor's instinct arm (`bba-gen --our-floor american-instinct`) is what
    // `boards.jsonl`'s `floor#N` provenance names, so replaying one of its rows
    // needs the same floor: under the shipped net floor a `floor#N` row prints
    // `(floor / no rule)` and the forensic stalls (scripts/anchor-diff.py).
    let system = if std::env::var("PROBE_FLOOR").as_deref() == Ok("instinct") {
        american_instinct(&agreements)
    } else {
        american(&agreements)
    };
    let partnership = system.bind();
    let inf = partnership.infer(vul, &auction);
    let p = inf.partner();
    println!(
        "partner read: hcp {:?} points {:?} lengths ♣{:?} ♦{:?} ♥{:?} ♠{:?}",
        p.strength.hcp, p.strength.points, p.lengths[0], p.lengths[1], p.lengths[2], p.lengths[3]
    );
    let r = inf.rho();
    println!(
        "rho read:     hcp {:?} lengths ♣{:?} ♦{:?} ♥{:?} ♠{:?}",
        r.strength.hcp, r.lengths[0], r.lengths[1], r.lengths[2], r.lengths[3]
    );
    let (logits, prov) = partnership
        .classify_with_provenance(hand, vul, &auction)
        .expect("classifies");
    println!("provenance: {prov:?}");
    let mut top: Vec<(Call, f32)> = (&logits.0)
        .into_iter()
        .filter(|(_, v)| v.is_finite())
        .map(|(c, v)| (c, *v))
        .collect();
    top.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (c, v) in top.iter().take(6) {
        let rule = partnership
            .explain_call(hand, vul, &auction, *c)
            .and_then(|(_, r)| r)
            .map_or(String::from("(floor / no rule)"), |r| {
                format!("rule #{} {:?}: {}", r.index, r.label, r.description)
            });
        println!("  {c:>4} {v:8.3}  {rule}");
    }
}
