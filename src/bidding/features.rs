//! Versioned feature extractor for the AI instinct bidder
//!
//! Converts a bridge hand and its auction [`Context`] into a fixed-size
//! `Vec<f32>` suitable for input to a neural network.  Every value is
//! normalised so that the expected range is roughly `[0.0, 1.0]`; the exact
//! layout is pinned by [`FEATURES_VERSION`] so that a model trained on one
//! version cannot be accidentally loaded under another.
//!
//! # Layout (version 1)
//!
//! | Block           | Start | Len |
//! |-----------------|-------|-----|
//! | Per-suit hand   |     0 |  76 |
//! | Global hand     |    76 |   6 |
//! | Context         |    82 |  36 |
//! | Inferences      |   118 |  40 |
//! | Vulnerability   |   158 |   2 |
//! | **Total**       |       | **160** |

use super::context::Context;
use super::inference::{Inferences, Relative};
use crate::bidding::constraint::upgrade;
use contract_bridge::auction::RelativeVulnerability;
use contract_bridge::eval::{self, HandEvaluator, SimpleEvaluator};
use contract_bridge::{Hand, Holding, Penalty, Rank, Strain, Suit};

/// Layout version tag — bump whenever the feature vector layout changes
pub const FEATURES_VERSION: u32 = 1;

/// Number of `f32` values returned by [`features`]
pub const FEATURES_LEN: usize = 160;

// ── Block offsets (used in tests and as documentation) ──────────────────────

/// Offset of the per-suit hand block (76 values)
pub const OFFSET_HAND: usize = 0;
/// Length of the per-suit hand block
pub const LEN_HAND: usize = 76;

/// Offset of the global hand block (6 values)
pub const OFFSET_GLOBAL: usize = 76;
/// Length of the global hand block
pub const LEN_GLOBAL: usize = 6;

/// Offset of the context block (36 values)
pub const OFFSET_CONTEXT: usize = 82;
/// Length of the context block
pub const LEN_CONTEXT: usize = 36;

/// Offset of the inferences block (40 values)
pub const OFFSET_INFERENCES: usize = 118;
/// Length of the inferences block
pub const LEN_INFERENCES: usize = 40;

/// Offset of the vulnerability block (2 values)
pub const OFFSET_VUL: usize = 158;
/// Length of the vulnerability block
pub const LEN_VUL: usize = 2;

// ── Rank constants in descending order for per-suit encoding ─────────────────

/// Ranks in the high-to-low order used for the 13 per-suit rank bits
const RANKS_HIGH_TO_LOW: [Rank; 13] = [
    Rank::A,
    Rank::K,
    Rank::Q,
    Rank::J,
    Rank::T,
    Rank::new(9),
    Rank::new(8),
    Rank::new(7),
    Rank::new(6),
    Rank::new(5),
    Rank::new(4),
    Rank::new(3),
    Rank::new(2),
];

// ── Private helpers ───────────────────────────────────────────────────────────

/// Balanced shape: every suit ≥ 2 cards and at most one doubleton
fn is_balanced(hand: Hand) -> bool {
    let lengths = Suit::ASC.map(|suit| hand[suit].len());
    lengths.iter().all(|&l| l >= 2) && lengths.iter().filter(|&&l| l == 2).count() <= 1
}

/// HCP of a single holding (A=4, K=3, Q=2, J=1)
fn holding_hcp(holding: Holding) -> u8 {
    4 * u8::from(holding.contains(Rank::A))
        + 3 * u8::from(holding.contains(Rank::K))
        + 2 * u8::from(holding.contains(Rank::Q))
        + u8::from(holding.contains(Rank::J))
}

/// Whether a holding stops the suit (A, Kx, Qxx, Jxxx)
fn has_stopper(holding: Holding) -> bool {
    holding.contains(Rank::A)
        || (holding.contains(Rank::K) && holding.len() >= 2)
        || (holding.contains(Rank::Q) && holding.len() >= 3)
        || (holding.contains(Rank::J) && holding.len() >= 4)
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

/// Extract a fixed-size feature vector from a hand and auction context
///
/// Returns exactly [`FEATURES_LEN`] `f32` values laid out as documented in the
/// module-level table.  All values are finite and normalised to roughly
/// `[0.0, 1.0]`.
#[must_use]
pub fn features(hand: Hand, context: &Context<'_>) -> Vec<f32> {
    let mut out = Vec::with_capacity(FEATURES_LEN);

    // ── Block 1: per-suit hand (76 values) ──────────────────────────────────
    for suit in Suit::ASC {
        let holding = hand[suit];
        let len = holding.len();

        // 1–13: rank indicator bits, high to low
        for &rank in &RANKS_HIGH_TO_LOW {
            out.push(f32::from(holding.contains(rank)));
        }

        // 14: len/13
        out.push(len as f32 / 13.0);

        // 15: suit_hcp/10
        out.push(holding_hcp(holding) as f32 / 10.0);

        // 16: top_honors/3 (count of A, K, Q present)
        let top = u8::from(holding.contains(Rank::A))
            + u8::from(holding.contains(Rank::K))
            + u8::from(holding.contains(Rank::Q));
        out.push(top as f32 / 3.0);

        // 17: stopper bit
        out.push(f32::from(has_stopper(holding)));

        // 18: is-major bit
        out.push(f32::from(matches!(suit, Suit::Hearts | Suit::Spades)));

        // 19: strain_rank/3 = (suit as u8) / 3.0
        out.push(suit as u8 as f32 / 3.0);
    }

    // ── Block 2: global hand (6 values) ─────────────────────────────────────
    let hcp = SimpleEvaluator(eval::hcp::<u8>).eval(hand);
    let up = upgrade(hand);
    let points = hcp + up;

    out.push(hcp as f32 / 40.0);
    out.push(points as f32 / 40.0);
    out.push(eval::FIFTHS.eval(hand) as f32 / 40.0);
    out.push(eval::cccc(hand) as f32 / 40.0);
    out.push(eval::NLTC.eval(hand) as f32 / 13.0);
    out.push(f32::from(is_balanced(hand)));

    // ── Block 3: context (36 values) ────────────────────────────────────────

    // our_strains: 5 bits
    for strain in Strain::ASC {
        out.push(f32::from(context.we_bid(strain)));
    }

    // their_strains: 5 bits
    for strain in Strain::ASC {
        out.push(f32::from(context.they_bid(strain)));
    }

    // contract-to-beat: 7 values
    push_bid_encoding(&mut out, context.last_bid());

    // partner's last bid: 7 values
    push_bid_encoding(&mut out, context.partner_last_bid());

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

    // seat one-hot (4 values): index = auction.len() % 4
    let seat_idx = context.auction().len() % 4;
    for i in 0..4 {
        out.push(f32::from(i == seat_idx));
    }

    // we-opened bit: 1 value
    let we_opened = match context.opener_seat() {
        Some(seat) => {
            let opening_index = seat as usize - 1;
            (context.auction().len() - opening_index).is_multiple_of(2)
        }
        None => false,
    };
    out.push(f32::from(we_opened));

    // ── Block 4: inferences (40 values) ─────────────────────────────────────
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

    // ── Block 5: vulnerability (2 values) ───────────────────────────────────
    let v = context.vul();
    out.push(f32::from(v.contains(RelativeVulnerability::WE)));
    out.push(f32::from(v.contains(RelativeVulnerability::THEY)));

    debug_assert_eq!(out.len(), FEATURES_LEN);
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
        assert_eq!(OFFSET_HAND, 0);
        assert_eq!(LEN_HAND, 76);
        assert_eq!(OFFSET_GLOBAL, OFFSET_HAND + LEN_HAND);
        assert_eq!(LEN_GLOBAL, 6);
        assert_eq!(OFFSET_CONTEXT, OFFSET_GLOBAL + LEN_GLOBAL);
        assert_eq!(LEN_CONTEXT, 36);
        assert_eq!(OFFSET_INFERENCES, OFFSET_CONTEXT + LEN_CONTEXT);
        assert_eq!(LEN_INFERENCES, 40);
        assert_eq!(OFFSET_VUL, OFFSET_INFERENCES + LEN_INFERENCES);
        assert_eq!(LEN_VUL, 2);
        assert_eq!(OFFSET_VUL + LEN_VUL, FEATURES_LEN);
    }

    #[test]
    fn length_is_correct_for_empty_auction() {
        let ctx = empty_context();
        let h = hand("AKQ32.K532.QJ4.9");
        let f = features(h, &ctx);
        assert_eq!(f.len(), FEATURES_LEN);
    }

    #[test]
    fn length_is_correct_for_contested_auction() {
        let auction = [
            bid(1, Strain::Hearts),
            bid(1, Strain::Spades),
            bid(2, Strain::Hearts),
        ];
        let ctx = Context::new(RelativeVulnerability::WE, &auction);
        let h = hand("AQ32.K53.QJ4.A92");
        let f = features(h, &ctx);
        assert_eq!(f.len(), FEATURES_LEN);
    }

    #[test]
    fn all_values_are_finite_and_in_range() {
        let auction = [
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Double,
        ];
        let ctx = Context::new(RelativeVulnerability::ALL, &auction);
        let h = hand("AKQ32.K532.QJ4.9");
        let f = features(h, &ctx);
        for (i, &v) in f.iter().enumerate() {
            assert!(v.is_finite(), "feature[{i}] is not finite: {v}");
            assert!(v >= 0.0, "feature[{i}] is negative: {v}");
            assert!(v <= 1.5, "feature[{i}] exceeds 1.5: {v}");
        }
    }

    #[test]
    fn empty_auction_known_values() {
        let ctx = empty_context();
        let h = hand("AKQ32.K532.QJ4.9");
        let f = features(h, &ctx);

        // Seat one-hot: auction.len()=0, so index 0 → f[OFFSET_CONTEXT + 24] = 1.0
        // Context layout: 5 our_strains + 5 their_strains + 7 last_bid + 7 partner + 3 penalty + 1 undisturbed + 1 passed + 1 partner_passed + 1 leading + 4 seat + 1 we_opened = 36
        // seat one-hot starts at OFFSET_CONTEXT + 5+5+7+7+3+1+1+1+1 = OFFSET_CONTEXT + 31
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
    fn spade_rank_bits_for_known_hand() {
        // "AKQ32.K532.QJ4.9" → spades = AKQ32 (A,K,Q,3,2)
        let h = hand("AKQ32.K532.QJ4.9");
        let ctx = empty_context();
        let f = features(h, &ctx);

        // Spades is suit index 3 in Suit::ASC, so its block starts at OFFSET_HAND + 3*19
        let spade_start = OFFSET_HAND + 3 * 19;
        // Ranks HIGH→LOW: A=1, K=1, Q=1, J=0, T=0, 9=0, 8=0, 7=0, 6=0, 5=0, 4=0, 3=1, 2=1
        assert_eq!(f[spade_start], 1.0, "A of spades");
        assert_eq!(f[spade_start + 1], 1.0, "K of spades");
        assert_eq!(f[spade_start + 2], 1.0, "Q of spades");
        assert_eq!(f[spade_start + 3], 0.0, "J of spades");
        assert_eq!(f[spade_start + 4], 0.0, "T of spades");
        // 9,8,7,6,5,4 all 0
        for i in 5..11 {
            assert_eq!(f[spade_start + i], 0.0, "rank bit {i} of spades");
        }
        assert_eq!(f[spade_start + 11], 1.0, "3 of spades");
        assert_eq!(f[spade_start + 12], 1.0, "2 of spades");

        // len/13 = 5/13
        assert!(
            (f[spade_start + 13] - 5.0 / 13.0).abs() < 1e-6,
            "spades len/13"
        );

        // stopper bit: has A, so 1.0
        assert_eq!(f[spade_start + 16], 1.0, "spades stopper");

        // is-major = 1.0
        assert_eq!(f[spade_start + 17], 1.0, "spades is-major");
    }

    #[test]
    fn balanced_bit_for_balanced_hand() {
        // 4333 shape — balanced
        let h = hand("AQ32.K53.QJ4.A92");
        let ctx = empty_context();
        let f = features(h, &ctx);
        assert_eq!(f[OFFSET_GLOBAL + 5], 1.0, "balanced bit for 4333");

        // 5431 shape — unbalanced (has singleton)
        let unbalanced = hand("AKQ32.K532.QJ4.9");
        let f2 = features(unbalanced, &ctx);
        assert_eq!(f2[OFFSET_GLOBAL + 5], 0.0, "balanced bit for 5431");
    }

    #[test]
    fn vulnerability_bits() {
        let ctx_we = Context::new(RelativeVulnerability::WE, &[]);
        let h = hand("AQ32.K53.QJ4.A92");
        let f = features(h, &ctx_we);
        assert_eq!(f[OFFSET_VUL], 1.0, "WE vul bit");
        assert_eq!(f[OFFSET_VUL + 1], 0.0, "THEY vul bit");

        let ctx_all = Context::new(RelativeVulnerability::ALL, &[]);
        let f2 = features(h, &ctx_all);
        assert_eq!(f2[OFFSET_VUL], 1.0);
        assert_eq!(f2[OFFSET_VUL + 1], 1.0);
    }

    #[test]
    fn we_opened_bit() {
        let h = hand("AQ32.K53.QJ4.A92");

        // Empty auction: no opener → 0.0
        let f0 = features(h, &empty_context());
        let we_opened_offset = OFFSET_CONTEXT + 35; // last value in context block
        assert_eq!(f0[we_opened_offset], 0.0, "no opener → 0.0");

        // We opened (seat 1, actor index = 1 call, parity even → we-opened)
        // After [1♠]: auction.len()=1, opening_index=0, (1-0)%2=1 ≠ 0 → they opened
        let auction_they = [bid(1, Strain::Spades)];
        let ctx_they = Context::new(RelativeVulnerability::NONE, &auction_they);
        let f1 = features(h, &ctx_they);
        assert_eq!(f1[we_opened_offset], 0.0, "they opened (RHO opened)");

        // After [1♠, P]: auction.len()=2, opening_index=0, (2-0)%2=0 → we opened
        let auction_we = [bid(1, Strain::Spades), Call::Pass];
        let ctx_we = Context::new(RelativeVulnerability::NONE, &auction_we);
        let f2 = features(h, &ctx_we);
        assert_eq!(f2[we_opened_offset], 1.0, "we opened (partner opened)");
    }

    #[test]
    fn penalty_one_hot() {
        let h = hand("AQ32.K53.QJ4.A92");
        let penalty_offset = OFFSET_CONTEXT + 5 + 5 + 7 + 7;

        // Undoubled (default)
        let f0 = features(h, &empty_context());
        assert_eq!(f0[penalty_offset], 1.0, "undoubled");
        assert_eq!(f0[penalty_offset + 1], 0.0);
        assert_eq!(f0[penalty_offset + 2], 0.0);

        // Doubled
        let auction_x = [bid(1, Strain::Spades), Call::Double];
        let ctx_x = Context::new(RelativeVulnerability::NONE, &auction_x);
        let f1 = features(h, &ctx_x);
        assert_eq!(f1[penalty_offset], 0.0);
        assert_eq!(f1[penalty_offset + 1], 1.0, "doubled");
        assert_eq!(f1[penalty_offset + 2], 0.0);

        // Redoubled
        let auction_xx = [bid(1, Strain::Spades), Call::Double, Call::Redouble];
        let ctx_xx = Context::new(RelativeVulnerability::NONE, &auction_xx);
        let f2 = features(h, &ctx_xx);
        assert_eq!(f2[penalty_offset], 0.0);
        assert_eq!(f2[penalty_offset + 1], 0.0);
        assert_eq!(f2[penalty_offset + 2], 1.0, "redoubled");
    }
}
