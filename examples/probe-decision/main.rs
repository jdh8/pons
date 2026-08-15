//! One decision, explained: what the live [`pons::american`] partnership reads
//! about partner and RHO at an auction, which node answers (provenance), and
//! the top logits with the rule that produced each — the CLI twin of
//! `Partnership::explain_call`.
//!
//! ```text
//! cargo run --release --example probe-decision -- "Q93.K43.AKJT.Q42" "1NT 2♠ 2NT - 3♣ - 3♦ -" [none|both|we|they]
//! ```
//!
//! Born on the N2 forensic (`docs/one-notrump-competitive.md` §N2): opener
//! after the weak Lebensohl sign-off read partner as `hcp 6..37`, every suit
//! `0..13`, and the floor bid `3NT`.

use contract_bridge::Hand;
use contract_bridge::auction::{Call, RelativeVulnerability};
use pons::american;
use pons::bidding::agreements::Agreements;

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
    let partnership = american(&Agreements::default()).bind();
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
