//! Structural tag reading of a call — the corpus vocabulary, shared.
//!
//! A call's *tags* are terse WBF abbreviations (see
//! `docs/ai-bidder/wbf-abbreviations.md`) naming its meaning — `FG` for a game
//! force, `T/O` for a takeout double, `TRF` for a transfer.  They are **derived
//! structurally** from the auction [`Context`] and the call, the same keyless
//! reading already encoded in [`Inferences`](crate::bidding::inference::Inferences); no
//! tag is stored on a node.
//!
//! This is the single source of truth for that reading, used in two places:
//!
//! - the corpus exporter (`examples/export-corpus`, AI-bidder M0.2) serializes
//!   [`derive`](crate::bidding::tags::derive)'s `(tags, description)` per
//!   `(node, call)`;
//! - the **version-2 feature extractor**
//!   ([`features_v2`](crate::bidding::features::features_v2), AI-bidder M5.1)
//!   multi-hots [`derive_tags`](crate::bidding::tags::derive_tags) of the last
//!   few calls as categorical inputs to the policy net.
//!
//! The exporter knows each node's book from the trie it lives in; the featurizer
//! recovers the same book from the auction with
//! [`infer_book`](crate::bidding::tags::infer_book).

use super::context::Context;
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain};

const ONE_NOTRUMP: Bid = Bid::new(1, Strain::Notrump);

/// The controlled tag vocabulary — exactly the WBF abbreviations
/// [`derive()`] can emit, in a fixed order so the
/// multi-hot index of a tag never moves.
///
/// Drawn from `docs/ai-bidder/wbf-abbreviations.md`.  It is *not* the whole WBF
/// list: every entry here is a dimension the structural reader actually sets, so
/// the V2 feature block carries no dead columns.  The `derive_emits_only_known_tags`
/// test fails loudly if [`derive()`] grows a tag
/// missing from this list — the cue to add it here and bump
/// [`FEATURES_VERSION_V2`](crate::bidding::features::FEATURES_VERSION_V2).
pub const TAGS: [&str; 21] = [
    "NAT", "NF", "F", "FG", "BAL", "STR", "WK", "PRE", // strength / forcing-ness
    "ART", "CUE", "SUPP", "L/S", "SPL", "WJS", // shape / role
    "T/O", "NEG", "PEN", "RDBL", // doubles
    "STAY", "TRF", "QUANT", // notrump conventions
];

/// Number of distinct tags — the width of one call's multi-hot slot.
pub const TAG_COUNT: usize = TAGS.len();

/// Index of `tag` in [`TAGS`], or [`None`] if it is not in the vocabulary.
#[must_use]
pub fn tag_index(tag: &str) -> Option<usize> {
    TAGS.iter().position(|&t| t == tag)
}

/// Set `out[i] = 1.0` for each known tag's index `i`, leaving the rest untouched.
///
/// `out` must be at least [`TAG_COUNT`] long.  Tags outside [`TAGS`] are ignored
/// (the vocabulary is closed by construction; see the test).
pub fn tag_multihot(tags: &[&str], out: &mut [f32]) {
    debug_assert!(out.len() >= TAG_COUNT);
    for &tag in tags {
        if let Some(i) = tag_index(tag) {
            out[i] = 1.0;
        }
    }
}

/// Whether our side has bid any strain yet in this auction.
fn we_bid_anything(ctx: &Context<'_>) -> bool {
    Strain::ASC.into_iter().any(|s| ctx.we_bid(s))
}

/// Recover the corpus *book* a call belongs to from the auction alone.
///
/// The exporter reads the book from the trie a node lives in; the featurizer has
/// only the [`Context`], so it reconstructs the same three-way split from purely
/// mechanical facts:
///
/// - **constructive** — the opponents have only passed (our own auction, incl.
///   the opening);
/// - **competitive** — we have bid and the opponents have since acted;
/// - **defensive** — the opponents have acted and we have not yet bid.
#[must_use]
pub fn infer_book(ctx: &Context<'_>) -> &'static str {
    if ctx.undisturbed() {
        "constructive"
    } else if we_bid_anything(ctx) {
        "competitive"
    } else {
        "defensive"
    }
}

/// Derive a call's `(tags, description)` at a node, structurally.
///
/// First cut: covers the high-confidence structural cases (openings, notrump
/// responses, takeout/negative doubles, cue-bids, raises, rebids, the 2/1
/// game force) and falls back to `NAT` / a generic gloss otherwise.  `book` is
/// the corpus book (`constructive` / `competitive` / `defensive`); pass
/// [`infer_book`] when only the [`Context`] is at hand.
#[must_use]
pub fn derive(book: &str, call: Call, ctx: &Context<'_>) -> (Vec<&'static str>, String) {
    match call {
        Call::Pass => (vec!["NF"], "Pass — below an action.".into()),
        Call::Redouble => (vec!["RDBL"], "Redouble.".into()),
        Call::Double => derive_double(book, ctx),
        Call::Bid(bid) => derive_bid(ctx, bid),
    }
}

/// The tags of a call, without building its prose — the V2 featurizer's path.
#[must_use]
pub fn derive_tags(book: &str, call: Call, ctx: &Context<'_>) -> Vec<&'static str> {
    derive(book, call, ctx).0
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
    if bid.level.get() == 2 && is_two_over_one(ctx, bid) {
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
fn is_two_over_one(ctx: &Context<'_>, bid: Bid) -> bool {
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
    fn tag_index_round_trips_every_tag() {
        for (i, &tag) in TAGS.iter().enumerate() {
            assert_eq!(tag_index(tag), Some(i), "{tag} not at its own index");
        }
        assert_eq!(tag_index("NOPE"), None);
    }

    #[test]
    fn vocabulary_has_no_duplicates() {
        for (i, &tag) in TAGS.iter().enumerate() {
            assert_eq!(tag_index(tag), Some(i), "{tag} appears twice");
        }
    }

    /// Every tag the structural reader can emit must be in [`TAGS`]; otherwise
    /// the V2 multi-hot would silently drop a dimension.  This is the gate that
    /// turns a new tag into a deliberate vocabulary + feature-version bump.
    #[test]
    fn derive_emits_only_known_tags() {
        const EMITTED: [&str; 21] = [
            "NF", "RDBL", "T/O", "NEG", "PEN", "NAT", "BAL", "ART", "STR", "PRE", "WK", "STAY",
            "TRF", "F", "QUANT", "CUE", "SUPP", "L/S", "SPL", "WJS", "FG",
        ];
        for tag in EMITTED {
            assert!(tag_index(tag).is_some(), "{tag} emitted but not in TAGS");
        }
        // And the vocabulary is exactly the emitted set (no dead columns).
        assert_eq!(TAGS.len(), EMITTED.len());
    }

    #[test]
    fn multihot_sets_only_those_tags() {
        let mut out = [0.0f32; TAG_COUNT];
        tag_multihot(&["FG", "NAT"], &mut out);
        assert_eq!(out[tag_index("FG").unwrap()], 1.0);
        assert_eq!(out[tag_index("NAT").unwrap()], 1.0);
        assert_eq!(out.iter().filter(|&&v| v == 1.0).count(), 2);
    }

    #[test]
    fn infer_book_splits_the_three_phases() {
        // Our opening: opponents silent → constructive.
        assert_eq!(infer_book(&ctx(&[])), "constructive");
        // They opened, we have not bid → defensive.
        assert_eq!(infer_book(&ctx(&[bid(1, Strain::Hearts)])), "defensive");
        // We opened, they overcalled → competitive.
        let contested = [bid(1, Strain::Spades), bid(2, Strain::Clubs)];
        assert_eq!(infer_book(&ctx(&contested)), "competitive");
    }

    #[test]
    fn infer_book_makes_derive_read_doubles() {
        // A direct double of their 1♥ opening reads as takeout under inferred book.
        let auction = [bid(1, Strain::Hearts)];
        let c = ctx(&auction);
        assert_eq!(derive_tags(infer_book(&c), Call::Double, &c), vec!["T/O"]);
    }

    #[test]
    fn derives_two_over_one_game_force() {
        // 1♥–P, then 2♣ is a game-forcing two-over-one.
        let auction = [bid(1, Strain::Hearts), Call::Pass];
        let c = ctx(&auction);
        assert_eq!(
            derive_tags("constructive", bid(2, Strain::Clubs), &c),
            vec!["FG", "NAT"]
        );
    }
}
