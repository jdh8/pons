//! Versioned feature extractor for the AI instinct bidder
//!
//! Converts a bridge hand and its auction [`Context`] into a fixed-size
//! `Vec<f32>` suitable for input to a neural network.  Every value is
//! normalised so that the expected range is roughly `[0.0, 1.0]`; the exact
//! layout is pinned by [`FEATURES_VERSION_V3`] so that a model trained on one
//! version cannot be accidentally loaded under another.
//!
//! # Layout (version 3 — the restrictive, fully disclosable vector)
//!
//! | Block                | Start | Len |
//! |----------------------|-------|-----|
//! | Disclosable hand     |     0 |  10 |
//! | Context              |    10 |  36 |
//! | Inferences           |    46 |  40 |
//! | Vulnerability        |    86 |   2 |
//! | **Total**            |       | **88** |

use super::context::Context;
use super::inference::{Inferences, Relative};
use crate::bidding::constraint::upgrade;
use contract_bridge::auction::RelativeVulnerability;
use contract_bridge::eval::{self, HandEvaluator, SimpleEvaluator};
use contract_bridge::{Hand, Holding, Penalty, Rank, Strain, Suit};

/// Layout version tag for the restrictive *disclosable* extractor [`features_v3`]
pub const FEATURES_VERSION_V3: u32 = 3;

/// Length of the restrictive hand block in [`features_v3`]: 4 suits ×
/// `{len, suit_hcp}` (8) plus global `{hcp, shape}` (2).
pub const LEN_HAND_V3: usize = 10;

/// Number of `f32` values returned by [`features_v3`]: a disclosable-only hand
/// summary ([`LEN_HAND_V3`]) plus the shared context/inferences/vulnerability
/// blocks.
pub const FEATURES_LEN_V3: usize = LEN_HAND_V3 + LEN_CONTEXT + LEN_INFERENCES + LEN_VUL;

// ── Block offsets (used in tests and as documentation) ──────────────────────

/// Offset of the context block (36 values)
pub const OFFSET_CONTEXT: usize = LEN_HAND_V3;
/// Length of the context block
pub const LEN_CONTEXT: usize = 36;

/// Offset of the inferences block (40 values)
pub const OFFSET_INFERENCES: usize = OFFSET_CONTEXT + LEN_CONTEXT;
/// Length of the inferences block
pub const LEN_INFERENCES: usize = 40;

/// Offset of the vulnerability block (2 values)
pub const OFFSET_VUL: usize = OFFSET_INFERENCES + LEN_INFERENCES;
/// Length of the vulnerability block
pub const LEN_VUL: usize = 2;

// ── Private helpers ───────────────────────────────────────────────────────────

/// HCP of a single holding (A=4, K=3, Q=2, J=1)
fn holding_hcp(holding: Holding) -> u8 {
    4 * u8::from(holding.contains(Rank::A))
        + 3 * u8::from(holding.contains(Rank::K))
        + 2 * u8::from(holding.contains(Rank::Q))
        + u8::from(holding.contains(Rank::J))
}

/// Push a 7-value bid encoding: [present, level/7, strain one-hot ×5]
fn push_bid_encoding(out: &mut Vec<f32>, bid: Option<contract_bridge::Bid>) {
    match bid {
        None => {
            out.push(0.0); // present
            out.push(0.0); // level/7
            for _ in Strain::ASC {
                out.push(0.0);
            }
        }
        Some(b) => {
            out.push(1.0); // present
            out.push(b.level.get() as f32 / 7.0);
            for strain in Strain::ASC {
                out.push(f32::from(b.strain == strain));
            }
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Push the auction-context, inferences, and vulnerability blocks (36 + 40 + 2
/// = 78 values) — the disclosable, hand-shape-independent tail of
/// [`features_v3`].
///
/// Everything here is derivable from the *public* auction and the partnership's
/// disclosed agreements (the [`Inferences`] ranges), so it stays in the
/// restrictive v3 vector unchanged.
fn push_context(out: &mut Vec<f32>, context: &Context<'_>) {
    // ── Context (36 values) ─────────────────────────────────────────────────

    // our_strains: 5 bits
    for strain in Strain::ASC {
        out.push(f32::from(context.we_bid(strain)));
    }

    // their_strains: 5 bits
    for strain in Strain::ASC {
        out.push(f32::from(context.they_bid(strain)));
    }

    // contract-to-beat: 7 values
    push_bid_encoding(out, context.last_bid());

    // partner's last bid: 7 values
    push_bid_encoding(out, context.partner_last_bid());

    // penalty one-hot: 3 values [Undoubled, Doubled, Redoubled]
    let penalty = context.penalty();
    out.push(f32::from(penalty == Penalty::Undoubled));
    out.push(f32::from(penalty == Penalty::Doubled));
    out.push(f32::from(penalty == Penalty::Redoubled));

    // undisturbed, passed_hand, partner_passed_hand: 3 values
    out.push(f32::from(context.undisturbed()));
    out.push(f32::from(context.passed_hand()));
    out.push(f32::from(context.partner_passed_hand()));

    // leading_passes (capped at 3): 1 value
    out.push((context.leading_passes().min(3) as f32) / 3.0);

    // seat one-hot (4 values): index = auction.len() % 4 (seat relative to dealer)
    let seat_idx = context.auction().len() % 4;
    for i in 0..4 {
        out.push(f32::from(i == seat_idx));
    }

    // we-opened bit: 1 value
    out.push(f32::from(context.we_opened()));

    // ── Inferences (40 values) ──────────────────────────────────────────────
    let inf = Inferences::read(context);

    for who in [
        Relative::Me,
        Relative::Lho,
        Relative::Partner,
        Relative::Rho,
    ] {
        let player = inf.get(who);
        for suit in Suit::ASC {
            let range = player.length(suit);
            out.push(range.min as f32 / 13.0);
            out.push(range.max as f32 / 13.0);
        }
        out.push(player.points.min as f32 / 37.0);
        out.push(player.points.max as f32 / 37.0);
    }

    // ── Vulnerability (2 values) ────────────────────────────────────────────
    let v = context.vul();
    out.push(f32::from(v.contains(RelativeVulnerability::WE)));
    out.push(f32::from(v.contains(RelativeVulnerability::THEY)));
}

/// Extract the **restrictive, fully disclosable** feature vector (AI-bidder v3)
///
/// Bridge ethics require full disclosure: a call is explained to opponents by
/// the partnership's *agreement*, never by the bidder's specific cards.
/// Agreements are defined over summary abstractions — so this extractor drops
/// every card-specific value (per-suit rank bits, top-honor count, stopper bit)
/// and keeps only what a bidder could disclose:
///
/// - per suit (4 × 2): `len/13`, `suit_hcp/10` (suit quality);
/// - global (2): `hcp/40`, `shape/2` where `shape = points − hcp` is the
///   crate's fuzzy distribution [`upgrade`] (0–2; the detailed shape is already
///   carried by the four suit lengths);
/// - the shared context, inferences, and vulnerability blocks (the
///   `push_context` tail) — all derived from the public auction and the
///   disclosed agreement ranges.
///
/// Seat (relative to dealer) and relative vulnerability are already inside those
/// shared blocks, so they are not repeated here.  Returns exactly
/// [`FEATURES_LEN_V3`] finite values normalised to roughly `[0.0, 1.0]`.
#[must_use]
pub fn features_v3(hand: Hand, context: &Context<'_>) -> Vec<f32> {
    let mut out = Vec::with_capacity(FEATURES_LEN_V3);

    // ── Restrictive hand block (10 values) ──────────────────────────────────
    // Per suit: length and suit HCP only — no rank/honor/stopper card detail.
    for suit in Suit::ASC {
        let holding = hand[suit];
        out.push(holding.len() as f32 / 13.0);
        out.push(holding_hcp(holding) as f32 / 10.0);
    }

    // Global strength: HCP and shape (= points − HCP = the fuzzy upgrade, 0–2).
    let hcp = SimpleEvaluator(eval::hcp::<u8>).eval(hand);
    let shape = upgrade(hand);
    out.push(hcp as f32 / 40.0);
    out.push(shape as f32 / 2.0);

    debug_assert_eq!(out.len(), LEN_HAND_V3);

    // ── Shared context / inferences / vulnerability (78 values) ─────────────
    push_context(&mut out, context);

    debug_assert_eq!(out.len(), FEATURES_LEN_V3);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use contract_bridge::auction::{Call, RelativeVulnerability};
    use contract_bridge::{Bid, Level, Strain};

    const fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid {
            level: Level::new(level),
            strain,
        })
    }

    fn hand(s: &str) -> Hand {
        s.parse().expect("valid test hand")
    }

    fn empty_context() -> Context<'static> {
        Context::new(RelativeVulnerability::NONE, &[])
    }

    #[test]
    fn block_offsets_are_consistent() {
        assert_eq!(LEN_HAND_V3, 10);
        assert_eq!(OFFSET_CONTEXT, LEN_HAND_V3);
        assert_eq!(LEN_CONTEXT, 36);
        assert_eq!(OFFSET_INFERENCES, OFFSET_CONTEXT + LEN_CONTEXT);
        assert_eq!(LEN_INFERENCES, 40);
        assert_eq!(OFFSET_VUL, OFFSET_INFERENCES + LEN_INFERENCES);
        assert_eq!(LEN_VUL, 2);
        assert_eq!(OFFSET_VUL + LEN_VUL, FEATURES_LEN_V3);
    }

    #[test]
    fn length_is_correct_for_contested_auction() {
        let auction = [
            bid(1, Strain::Hearts),
            bid(1, Strain::Spades),
            bid(2, Strain::Hearts),
        ];
        let ctx = Context::new(RelativeVulnerability::WE, &auction);
        let f = features_v3(hand("AQ32.K53.QJ4.A92"), &ctx);
        assert_eq!(f.len(), FEATURES_LEN_V3);
    }

    #[test]
    fn v3_length_and_range() {
        // v3 is 88 floats: a 10-value restrictive hand block + the 78-value
        // shared context/inferences/vul tail.
        assert_eq!(FEATURES_LEN_V3, 88);
        let auction = [
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Double,
        ];
        for ctx in [
            empty_context(),
            Context::new(RelativeVulnerability::ALL, &auction),
        ] {
            let f = features_v3(hand("AKQ32.K532.QJ4.9"), &ctx);
            assert_eq!(f.len(), FEATURES_LEN_V3);
            for (i, &v) in f.iter().enumerate() {
                assert!(v.is_finite() && (0.0..=1.5).contains(&v), "v3[{i}] = {v}");
            }
        }
    }

    #[test]
    fn empty_auction_known_values() {
        let ctx = empty_context();
        let f = features_v3(hand("AKQ32.K532.QJ4.9"), &ctx);

        // Context layout: 5 our_strains + 5 their_strains + 7 last_bid + 7 partner
        // + 3 penalty + 1 undisturbed + 1 passed + 1 partner_passed + 1 leading
        // + 4 seat + 1 we_opened = 36.
        // Seat one-hot: auction.len() = 0, so index 0 is set.
        let seat_one_hot_start = OFFSET_CONTEXT + 5 + 5 + 7 + 7 + 3 + 1 + 1 + 1 + 1;
        assert_eq!(f[seat_one_hot_start], 1.0, "seat index 0 should be 1.0");
        assert_eq!(f[seat_one_hot_start + 1], 0.0);
        assert_eq!(f[seat_one_hot_start + 2], 0.0);
        assert_eq!(f[seat_one_hot_start + 3], 0.0);

        // Vulnerability: both 0.0 (NONE)
        assert_eq!(f[OFFSET_VUL], 0.0, "WE vul should be 0.0");
        assert_eq!(f[OFFSET_VUL + 1], 0.0, "THEY vul should be 0.0");

        // contract-to-beat present bit = 0.0
        let last_bid_start = OFFSET_CONTEXT + 5 + 5;
        assert_eq!(f[last_bid_start], 0.0, "contract-to-beat present bit");

        // undisturbed = 1.0 for empty auction
        let undisturbed_offset = OFFSET_CONTEXT + 5 + 5 + 7 + 7 + 3;
        assert_eq!(f[undisturbed_offset], 1.0, "undisturbed should be 1.0");
    }

    #[test]
    fn disclosable_hand_block_for_known_hand() {
        // "AKQ32.K532.QJ4.9" — Suit::ASC order is clubs, diamonds, hearts, spades.
        let f = features_v3(hand("AKQ32.K532.QJ4.9"), &empty_context());

        // Clubs: singleton 9, no HCP.
        assert!((f[0] - 1.0 / 13.0).abs() < 1e-6, "clubs len/13");
        assert_eq!(f[1], 0.0, "clubs suit_hcp");
        // Diamonds: QJ4 = 3 cards, 3 HCP.
        assert!((f[2] - 3.0 / 13.0).abs() < 1e-6, "diamonds len/13");
        assert!((f[3] - 3.0 / 10.0).abs() < 1e-6, "diamonds suit_hcp");
        // Hearts: K532 = 4 cards, 3 HCP.
        assert!((f[4] - 4.0 / 13.0).abs() < 1e-6, "hearts len/13");
        assert!((f[5] - 3.0 / 10.0).abs() < 1e-6, "hearts suit_hcp");
        // Spades: AKQ32 = 5 cards, 9 HCP.
        assert!((f[6] - 5.0 / 13.0).abs() < 1e-6, "spades len/13");
        assert!((f[7] - 9.0 / 10.0).abs() < 1e-6, "spades suit_hcp");
        // Global: 15 HCP, then the fuzzy shape upgrade scaled by 2.
        assert!((f[8] - 15.0 / 40.0).abs() < 1e-6, "hcp/40");
        assert!((0.0..=1.0).contains(&f[9]), "shape/2 in range");
    }

    #[test]
    fn vulnerability_bits() {
        let h = hand("AQ32.K53.QJ4.A92");
        let ctx_we = Context::new(RelativeVulnerability::WE, &[]);
        let f = features_v3(h, &ctx_we);
        assert_eq!(f[OFFSET_VUL], 1.0, "WE vul bit");
        assert_eq!(f[OFFSET_VUL + 1], 0.0, "THEY vul bit");

        let ctx_all = Context::new(RelativeVulnerability::ALL, &[]);
        let f2 = features_v3(h, &ctx_all);
        assert_eq!(f2[OFFSET_VUL], 1.0);
        assert_eq!(f2[OFFSET_VUL + 1], 1.0);
    }

    #[test]
    fn we_opened_bit() {
        let h = hand("AQ32.K53.QJ4.A92");
        let we_opened_offset = OFFSET_CONTEXT + 35; // last value in context block

        // Empty auction: no opener → 0.0
        let f0 = features_v3(h, &empty_context());
        assert_eq!(f0[we_opened_offset], 0.0, "no opener → 0.0");

        // After [1♠]: auction.len()=1, opening_index=0, (1-0)%2=1 ≠ 0 → they opened
        let auction_they = [bid(1, Strain::Spades)];
        let ctx_they = Context::new(RelativeVulnerability::NONE, &auction_they);
        let f1 = features_v3(h, &ctx_they);
        assert_eq!(f1[we_opened_offset], 0.0, "they opened (RHO opened)");

        // After [1♠, P]: auction.len()=2, opening_index=0, (2-0)%2=0 → we opened
        let auction_we = [bid(1, Strain::Spades), Call::Pass];
        let ctx_we = Context::new(RelativeVulnerability::NONE, &auction_we);
        let f2 = features_v3(h, &ctx_we);
        assert_eq!(f2[we_opened_offset], 1.0, "we opened (partner opened)");
    }

    #[test]
    fn penalty_one_hot() {
        let h = hand("AQ32.K53.QJ4.A92");
        let penalty_offset = OFFSET_CONTEXT + 5 + 5 + 7 + 7;

        // Undoubled (default)
        let f0 = features_v3(h, &empty_context());
        assert_eq!(f0[penalty_offset], 1.0, "undoubled");
        assert_eq!(f0[penalty_offset + 1], 0.0);
        assert_eq!(f0[penalty_offset + 2], 0.0);

        // Doubled
        let auction_x = [bid(1, Strain::Spades), Call::Double];
        let ctx_x = Context::new(RelativeVulnerability::NONE, &auction_x);
        let f1 = features_v3(h, &ctx_x);
        assert_eq!(f1[penalty_offset], 0.0);
        assert_eq!(f1[penalty_offset + 1], 1.0, "doubled");
        assert_eq!(f1[penalty_offset + 2], 0.0);

        // Redoubled
        let auction_xx = [bid(1, Strain::Spades), Call::Double, Call::Redouble];
        let ctx_xx = Context::new(RelativeVulnerability::NONE, &auction_xx);
        let f2 = features_v3(h, &ctx_xx);
        assert_eq!(f2[penalty_offset], 0.0);
        assert_eq!(f2[penalty_offset + 1], 0.0);
        assert_eq!(f2[penalty_offset + 2], 1.0, "redoubled");
    }
}
