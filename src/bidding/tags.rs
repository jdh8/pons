//! Structural tag reading of a call — the corpus vocabulary, shared.
//!
//! A call's *tags* are terse WBF abbreviations (see
//! `docs/ai-bidder/wbf-abbreviations.md`) naming its meaning — `FG` for a game
//! force, `T/O` for a takeout double, `TRF` for a transfer.  They are **derived
//! structurally** from the auction [`Context`] and the call, the same keyless
//! reading already encoded in [`Inferences`](crate::bidding::inference::Inferences); no
//! tag is stored on a node.
//!
//! This is the single source of truth for that reading.  Its consumer is the
//! corpus exporter (`examples/export-corpus` / `examples/dump-corpus`,
//! AI-bidder M0.2), which serializes
//! [`derive`](crate::bidding::tags::derive)'s `(tags, description)` per
//! `(node, call)`.  The exporter knows each node's *book* (`constructive` /
//! `competitive` / `defensive`) from the trie the node lives in and passes it in.

use super::context::Context;
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain};

const ONE_NOTRUMP: Bid = Bid::new(1, Strain::Notrump);

/// Whether our side has bid any strain yet in this auction.
fn we_bid_anything(ctx: &Context<'_>) -> bool {
    Strain::ASC.into_iter().any(|s| ctx.we_bid(s))
}

/// Derive a call's `(tags, description)` at a node, structurally.
///
/// First cut: covers the high-confidence structural cases (openings, notrump
/// responses, takeout/negative doubles, cue-bids, raises, rebids, the 2/1
/// game force) and falls back to `NAT` / a generic gloss otherwise.  `book` is
/// the corpus book (`constructive` / `competitive` / `defensive`), read from the
/// trie the node lives in.
#[must_use]
pub fn derive(book: &str, call: Call, ctx: &Context<'_>) -> (Vec<&'static str>, String) {
    match call {
        Call::Pass => (vec!["NF"], "Pass — below an action.".into()),
        Call::Redouble => (vec!["RDBL"], "Redouble.".into()),
        Call::Double => derive_double(book, ctx),
        Call::Bid(bid) => derive_bid(ctx, bid),
    }
}

fn derive_double(book: &str, ctx: &Context<'_>) -> (Vec<&'static str>, String) {
    // Defending side's first action over a suit opening: takeout.
    let their_suit_opening = ctx.last_bid().is_some_and(|b| b.strain.is_suit());
    if book == "defensive" && !we_bid_anything(ctx) && their_suit_opening {
        return (vec!["T/O"], "Takeout double of their opening.".into());
    }
    // We opened and they overcalled, responder doubles low: negative.
    if book == "competitive" && we_bid_anything(ctx) {
        return (vec!["NEG"], "Negative double — the unbid suit(s).".into());
    }
    (vec!["PEN"], "Penalty double.".into())
}

fn derive_bid(ctx: &Context<'_>, bid: Bid) -> (Vec<&'static str>, String) {
    // An opening bid: the first non-pass call.
    if ctx.last_bid().is_none() {
        return derive_opening(bid);
    }

    let partner_opened_1nt = ctx.partner_last_bid() == Some(ONE_NOTRUMP);
    if partner_opened_1nt
        && ctx.undisturbed()
        && let Some(record) = derive_over_1nt(bid)
    {
        return record;
    }

    let strain = bid.strain;

    // Cue-bid of a strain the opponents bid.
    if strain.is_suit() && ctx.they_bid(strain) {
        return (vec!["CUE"], "Cue-bid of their suit.".into());
    }

    // Raise of partner's last suit.
    if let Some(suit) = strain.suit() {
        if ctx.partner_last_suit() == Some(suit) {
            let jump = jump_over(ctx, bid);
            let mut tags = vec!["SUPP"];
            if jump >= 1 {
                tags.push("PRE");
            }
            return (tags, "Raise of partner's suit.".into());
        }
        // Rebid of our own suit: extra length.
        if ctx.we_bid(strain) {
            return (vec!["NAT", "L/S"], "Rebid — extra length.".into());
        }
    }

    if strain == Strain::Notrump {
        return (vec!["NAT"], format!("{}NT — natural.", bid.level.get()));
    }

    // A new suit.
    let jump = jump_over(ctx, bid);
    if jump >= 2 {
        return (vec!["SPL"], "Splinter / double jump — shortness.".into());
    }
    if jump == 1 {
        return (vec!["WJS", "WK"], "Weak jump shift.".into());
    }
    // Cheapest new suit. A 2-level new suit over partner's 1-major opening is
    // a game-forcing 2/1.
    if bid.level.get() == 2 && is_american(ctx, bid) {
        return (vec!["FG", "NAT"], "Two-over-one — game forcing.".into());
    }
    (vec!["NAT"], "Natural new suit.".into())
}

fn derive_opening(bid: Bid) -> (Vec<&'static str>, String) {
    match (bid.level.get(), bid.strain) {
        (1, Strain::Notrump) => (vec!["NAT", "BAL"], "1NT opening — balanced.".into()),
        (2, Strain::Notrump) => (vec!["NAT", "BAL"], "2NT opening — balanced.".into()),
        (2, Strain::Clubs) => (vec!["ART", "STR"], "Strong artificial 2♣ opening.".into()),
        (1, s) if s.is_suit() => (vec!["NAT"], "One-level suit opening.".into()),
        (2, s) if s.is_suit() => (vec!["PRE", "WK"], "Weak two opening.".into()),
        (_, s) if s.is_suit() => (vec!["PRE"], "Preemptive opening.".into()),
        _ => (vec!["NAT"], "Opening bid.".into()),
    }
}

/// Responses to partner's 1NT opening (Stayman / Jacoby transfers).
fn derive_over_1nt(bid: Bid) -> Option<(Vec<&'static str>, String)> {
    match (bid.level.get(), bid.strain) {
        (2, Strain::Clubs) => Some((vec!["STAY"], "Stayman.".into())),
        (2, Strain::Diamonds) => Some((vec!["TRF"], "Jacoby transfer to hearts.".into())),
        (2, Strain::Hearts) => Some((vec!["TRF"], "Jacoby transfer to spades.".into())),
        (2, Strain::Spades) => Some((vec!["TRF"], "Minor-suit transfer.".into())),
        (3, s) if s.is_suit() => Some((vec!["NAT", "F"], "Natural, forcing.".into())),
        (_, Strain::Notrump) => Some((vec!["NAT", "QUANT"], "Quantitative notrump.".into())),
        _ => None,
    }
}

/// Levels of jump above the cheapest legal level for the bid's strain.
fn jump_over(ctx: &Context<'_>, bid: Bid) -> u8 {
    ctx.min_level(bid.strain)
        .map_or(0, |min| bid.level.get().saturating_sub(min.get()))
}

/// Whether `bid` is a game-forcing 2/1 over partner's 1-major opening.
fn is_american(ctx: &Context<'_>, bid: Bid) -> bool {
    let Some(partner) = ctx.partner_last_bid() else {
        return false;
    };
    if partner.level.get() != 1 || !matches!(partner.strain, Strain::Hearts | Strain::Spades) {
        return false;
    }
    // A lower-ranking new suit at the two level, not their suit, not ours.
    bid.level.get() == 2
        && bid.strain.is_suit()
        && !ctx.we_bid(bid.strain)
        && bid.strain < partner.strain
}

#[cfg(test)]
mod tests {
    use super::*;
    use contract_bridge::Level;
    use contract_bridge::auction::RelativeVulnerability;

    const fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid {
            level: Level::new(level),
            strain,
        })
    }

    fn ctx(auction: &[Call]) -> Context<'_> {
        Context::new(RelativeVulnerability::NONE, auction)
    }

    #[test]
    fn derive_reads_doubles_by_book() {
        // A direct double of their 1♥ opening, in the defensive book: takeout.
        let auction = [bid(1, Strain::Hearts)];
        let c = ctx(&auction);
        assert_eq!(derive("defensive", Call::Double, &c).0, vec!["T/O"]);

        // We opened, they overcalled: responder's double is negative.
        let contested = [bid(1, Strain::Spades), bid(2, Strain::Clubs)];
        let c = ctx(&contested);
        assert_eq!(derive("competitive", Call::Double, &c).0, vec!["NEG"]);
    }

    #[test]
    fn derives_american_game_force() {
        // 1♥–P, then 2♣ is a game-forcing two-over-one.
        let auction = [bid(1, Strain::Hearts), Call::Pass];
        let c = ctx(&auction);
        assert_eq!(
            derive("constructive", bid(2, Strain::Clubs), &c).0,
            vec!["FG", "NAT"]
        );
    }
}
