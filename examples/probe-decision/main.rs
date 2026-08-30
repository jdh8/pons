//! One decision, explained: what the live [`pons::american`] partnership reads
//! about partner and RHO at an auction, which node answers (provenance), and
//! the top logits with the rule that produced each — the CLI twin of
//! `Partnership::explain_call`.
//!
//! ```text
//! cargo run --release --example probe-decision -- "Q93.K43.AKJT.Q42" "1NT (2♠) 2NT - 3♣ - 3♦ -" [none|both|we|they]
//! ```
//!
//! `PROBE_FLOOR=instinct` swaps the shipped net floor for the deterministic one,
//! which is what an anchor `floor#N` row was generated under.
//! `PROBE_NT_HIGH_OVERCALL=1` (or `=transfers`) engages the default-off N3
//! package over their three-level overcall of our `1NT`.
//! `PROBE_THEIR_2D_MULTI=1` declares their `(2♦)` a Multi (§N4) — without it
//! this lane silently probes the natural `(2♦)` leg — and
//! `PROBE_MULTI_WEAK_ESCAPE=6|5` adds the default-off floorless weak escape.
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
        .map(|c| match c.trim_matches(['(', ')']) {
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
    // N3's `1NT (3x)` package is default-off while its A/B runs, so probing
    // its readings needs the knob (`=transfers` also engages the `(3♣)`
    // transfer variant).
    match std::env::var("PROBE_NT_HIGH_OVERCALL").as_deref() {
        Ok("transfers") => {
            agreements.competition.nt_high_overcall_responses = true;
            agreements.competition.nt_3c_transfers = true;
        }
        Ok("1") => agreements.competition.nt_high_overcall_responses = true,
        Ok("0") => agreements.competition.nt_high_overcall_responses = false,
        _ => {}
    }
    // Their `(2♦)` is a *disclosure* (`decision.their`), undeclared by default,
    // and this probe had no channel for one — so every forensic in the N4 lane
    // silently read the natural `(2♦)` leg.  `PROBE_MULTI_WEAK_ESCAPE=6` (or
    // `5`) rides it with the default-off floorless escape.
    match std::env::var("PROBE_THEIR_2D_MULTI").as_deref() {
        Ok("0") => agreements.decision.their.two_diamonds_multi = false,
        Ok(_) => agreements.decision.their.two_diamonds_multi = true,
        Err(_) => {}
    }
    match std::env::var("PROBE_MULTI_WEAK_ESCAPE").as_deref() {
        Ok("0") | Ok("off") => agreements.competition.multi_weak_escape = None,
        Ok(n) => agreements.competition.multi_weak_escape = Some(n.parse().expect("a suit length")),
        Err(_) => {}
    }
    // The Kokish–Kraft whole-table counter (`competition.multi_kokish_kraft`,
    // shipped default-on 2026-08-25), which needs `PROBE_THEIR_2D_MULTI` set
    // to do anything at all.  `=0` falls back to the v7 subtree.
    if std::env::var("PROBE_MULTI_KOKISH_KRAFT").is_ok_and(|v| v == "0") {
        agreements.competition.multi_kokish_kraft = false;
    }
    // The `4m` slam try above a completed K–K minor transfer
    // (`competition.multi_minor_slam_try`, default `15`): `PROBE_MULTI_MINOR_SLAM=off`
    // disarms it, a number re-floors it, and it needs both of the two above to
    // do anything.
    match std::env::var("PROBE_MULTI_MINOR_SLAM").as_deref() {
        Ok("0") | Ok("off") => agreements.competition.multi_minor_slam_try = None,
        Ok(n) => {
            agreements.competition.multi_minor_slam_try = Some(n.parse().expect("a points floor"));
        }
        Err(_) => {}
    }
    // The doubler's natural other major (`competition.multi_doubler_major`,
    // shipped default-on 2026-08-26, §N4-KK residue 4): `=0` withholds it.
    // Needs both of the two above to do anything.
    if std::env::var("PROBE_MULTI_DOUBLER_MAJOR").is_ok_and(|v| v == "0") {
        agreements.competition.multi_doubler_major = false;
    }
    // Responder's P/X information split over their Multi
    // (`competition.multi_px_split`, default off): `PROBE_MULTI_PX_SPLIT=1`
    // arms it.  Implies the natural other major at weight 148 and swaps the
    // delayed `2NT` answer to an acceptance.  Needs the two above it to do
    // anything.
    if std::env::var("PROBE_MULTI_PX_SPLIT").is_ok_and(|v| v == "1") {
        agreements.competition.multi_px_split = true;
    }
    // Opener's notrump out over the doubler's natural other major
    // (`competition.multi_doubler_notrump`, **default on** since 2026-08-27):
    // `PROBE_MULTI_DOUBLER_NOTRUMP=0` disarms it, which is the interesting
    // direction now — that is the answer table whose pass-outs the
    // 2026-08-26 `multi_doubler_major` A/B measured.
    match std::env::var("PROBE_MULTI_DOUBLER_NOTRUMP").as_deref() {
        Ok("0") => agreements.competition.multi_doubler_notrump = false,
        Ok(_) => agreements.competition.multi_doubler_notrump = true,
        Err(_) => {}
    }
    // That notrump out extended down to the 15-count as `2NT` on the `2♠` leg
    // (`competition.multi_doubler_minimum_notrump`, **default on** since
    // 2026-08-27): `PROBE_MULTI_DOUBLER_MINIMUM_NOTRUMP=0` disarms it back to
    // the `hcp(16..)` floor, which is the interesting direction now.
    match std::env::var("PROBE_MULTI_DOUBLER_MINIMUM_NOTRUMP").as_deref() {
        Ok("0") => agreements.competition.multi_doubler_minimum_notrump = false,
        Ok(_) => agreements.competition.multi_doubler_minimum_notrump = true,
        Err(_) => {}
    }
    // Their `(2♣)` is Landy — a *disclosure*, undeclared by default, and until
    // 2026-08-25 this probe had no channel for one, so every N1j forensic
    // silently read the natural `(2♣)` leg: the whole Landy table sat inert and
    // the seat printed `fallback: Some(0)`.  Same class of hole as
    // `PROBE_THEIR_2D_MULTI` above.
    match std::env::var("PROBE_THEIR_2C_LANDY").as_deref() {
        Ok("0") => agreements.decision.their.two_clubs_landy = false,
        Ok(_) => agreements.decision.their.two_clubs_landy = true,
        Err(_) => {}
    }
    // Opener's answer to the N1j Landy `4m` slam try
    // (`competition.landy_minor_slam_answer`, default on).  Needs
    // `PROBE_THEIR_2C_LANDY` to reach anything.
    match std::env::var("PROBE_LANDY_SLAM_ANSWER").as_deref() {
        Ok("0") | Ok("off") => agreements.competition.landy_minor_slam_answer = false,
        Ok(_) => agreements.competition.landy_minor_slam_answer = true,
        Err(_) => {}
    }
    // The Landy doubler's own rebid once their advance names the major
    // (`competition.landy_doubler_rebids`, default off — A/B owed).  Needs
    // `PROBE_THEIR_2C_LANDY` to reach anything.
    match std::env::var("PROBE_LANDY_DOUBLER_REBIDS").as_deref() {
        Ok("0") | Ok("off") => agreements.competition.landy_doubler_rebids = false,
        Ok(_) => agreements.competition.landy_doubler_rebids = true,
        Err(_) => {}
    }
    // The §N1l flip's two smaller arms (`competition.landy_doubler_px`,
    // `landy_doubler_white`, both default off).  `REBIDS` shadows either.
    match std::env::var("PROBE_LANDY_DOUBLER_PX").as_deref() {
        Ok("0") | Ok("off") => agreements.competition.landy_doubler_px = false,
        Ok(_) => agreements.competition.landy_doubler_px = true,
        Err(_) => {}
    }
    match std::env::var("PROBE_LANDY_DOUBLER_WHITE").as_deref() {
        Ok("0") | Ok("off") => agreements.competition.landy_doubler_white = false,
        Ok(_) => agreements.competition.landy_doubler_white = true,
        Err(_) => {}
    }
    // §N1m — **opener's** own seat one call earlier (`competition.landy_opener_px`
    // and its `rungs` companion, both default off).  Needs `PROBE_THEIR_2C_LANDY`.
    match std::env::var("PROBE_LANDY_OPENER_PX").as_deref() {
        Ok("0") | Ok("off") => agreements.competition.landy_opener_px = false,
        Ok(_) => agreements.competition.landy_opener_px = true,
        Err(_) => {}
    }
    match std::env::var("PROBE_LANDY_OPENER_RUNGS").as_deref() {
        Ok("0") | Ok("off") => agreements.competition.landy_opener_rungs = false,
        Ok(_) => agreements.competition.landy_opener_rungs = true,
        Err(_) => {}
    }
    // §N1p — `3NT` denies a four-card major (default off), and the `4M` jam
    // above it (`landy_major_jam`, default **on**, independent of it).  Needs
    // `PROBE_THEIR_2C_LANDY`.
    match std::env::var("PROBE_LANDY_NT_NO_MAJOR").as_deref() {
        Ok("0") | Ok("off") => agreements.competition.landy_notrump_no_major = false,
        Ok(_) => agreements.competition.landy_notrump_no_major = true,
        Err(_) => {}
    }
    match std::env::var("PROBE_LANDY_MAJOR_JAM").as_deref() {
        Ok("0") | Ok("off") => agreements.competition.landy_major_jam = false,
        Ok(_) => agreements.competition.landy_major_jam = true,
        Err(_) => {}
    }
    // The `4m` slam try above a completed *Puppet* minor transfer
    // (`notrump.minor_transfer_slam_try`, default 13): a points floor arms it.
    match std::env::var("PROBE_NT_MINOR_SLAM").as_deref() {
        Ok("0") | Ok("off") => agreements.notrump.minor_transfer_slam_try = None,
        Ok(n) => {
            agreements.notrump.minor_transfer_slam_try = Some(n.parse().expect("a points floor"));
        }
        Err(_) => {}
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
