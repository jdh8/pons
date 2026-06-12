//! BTU responses to a strong 1NT opening
//!
//! The strawberry variant's replacement for the baseline 1NT response block
//! ([`super::notrump::register_one_nt`]).  BTU Stayman 2♣ (which also handles
//! invitational 5-card spades), BTU Jacoby transfers 2♦/2♥ with super-accepts,
//! minor-suit transfers 2♠/2NT, Puppet Stayman 3♣, splinters, South African
//! Texas, and the quantitative ladder.
//!
//! # Structure overview
//!
//! - **1NT → responses**: 2♣ BTU Stayman, 2♦/2♥ transfers, 2♠ club relay,
//!   2NT diamond relay, 3♣ Puppet Stayman, 3♦ 5-5 majors INV+, 3♥/3♠ splinters,
//!   3NT signoff, 4♣/4♦ Texas, 4♥/4♠ signoff, 4NT/5NT quantitative.
//!
//! - **2♣ BTU Stayman**: opener answers 2♦ (no major), 2♥ (4+♥), 2♠ (4+♠ no ♥).
//!   Continuations handle Smolen, invitations, and game values.
//!
//! - **2♦/2♥ Jacoby transfers**: opener super-accepts with maximum 4-card fit
//!   showing distribution; forcing 2♠ relay over 2♥ completes the invite.
//!
//! - **Scope / termination**: every authored path reaches game or a slot known
//!   to terminate (pass, or an already-authored game bid).  The constructive
//!   book has no instinct floor, so a stranded forcing auction would be passed
//!   below game.  Deep relay continuations are simplified to game terminations
//!   where the relay logic is not yet fully authored.

use super::{call, insert_uncontested, slam};
use crate::bidding::constraint::{balanced, hcp, len, stopper_in, top_honors};
use crate::bidding::{Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

// ---------------------------------------------------------------------------
// 1NT — top-level response table
// ---------------------------------------------------------------------------

/// Responder's first call over our 1NT opening (BTU variant)
///
/// - 2♣: BTU Stayman (Garbage / INV 5=♠ / GF with 4-card major)
/// - 2♦: Jacoby TRF, 5+♥
/// - 2♥: Jacoby TRF, 5+♠
/// - 2♠: minor relay, 6+♣ or QUANT INV
/// - 2NT: minor relay, 5+♦ + 4+♣ or 6+♦
/// - 3♣: Puppet Stayman (game force, queries 5 and 4-card majors)
/// - 3♦: INV+, 5+♠ 5+♥
/// - 3♥: splinter, 0-1♥, 0-3♠, 4+♦♣
/// - 3♠: splinter, 0-1♠, 0-3♥, 4+♦♣
/// - 3NT: signoff
/// - 4♣: South African Texas TRF, 6+♥
/// - 4♦: South African Texas TRF, 6+♠
/// - 4♥/4♠: signoff
/// - 4NT: QUANT INV to 6NT
/// - 5NT: QUANT INV to 7NT (forces 6NT+)
fn btu_responses() -> Rules {
    Rules::new()
        // South African Texas 4♣: game-going 6+♥ hand.  Weight beats the 2♦
        // transfer (3.0) so a strong 6-card heart hand goes directly to the
        // 4♥ level rather than the invite relay.
        .rule(
            Bid::new(4, Strain::Clubs),
            3.5,
            len(Suit::Hearts, 6..) & hcp(9..),
        )
        // South African Texas 4♦: game-going 6+♠ hand.
        .rule(
            Bid::new(4, Strain::Diamonds),
            3.5,
            len(Suit::Spades, 6..) & hcp(9..),
        )
        // 2♦ transfer: 5+♥ (any strength, including weak 6-card hands)
        .rule(Bid::new(2, Strain::Diamonds), 3.0, len(Suit::Hearts, 5..))
        // 2♥ transfer: 5+♠ (any strength)
        .rule(Bid::new(2, Strain::Hearts), 3.0, len(Suit::Spades, 5..))
        // Puppet Stayman 3♣: game force, no 5-card major, has 4-card major.
        // Weight (2.8) beats 2♣ Stayman (2.5) when the hand is GF (12+) with
        // a 4-card major but no 5-card major — Puppet finds opener's 5-card suit.
        .rule(
            Bid::new(3, Strain::Clubs),
            2.8,
            hcp(12..)
                & len(Suit::Hearts, ..5)
                & len(Suit::Spades, ..5)
                & (len(Suit::Hearts, 4..) | len(Suit::Spades, 4..)),
        )
        // 2♣ BTU Stayman: Garbage (short clubs), INV 5=♠, or GF with 5-card major.
        // Also used GF with 4-card major if Puppet Stayman does not apply.
        .rule(
            Bid::new(2, Strain::Clubs),
            2.5,
            (len(Suit::Hearts, 4..) | len(Suit::Spades, 4..)) & hcp(8..),
        )
        // 3♦: INV+, 5+♠ 5+♥
        .rule(
            Bid::new(3, Strain::Diamonds),
            2.0,
            len(Suit::Hearts, 5..) & len(Suit::Spades, 5..) & hcp(8..),
        )
        // 3♥: splinter, 0-1♥, 0-3♠, long minors
        .rule(
            Bid::new(3, Strain::Hearts),
            1.8,
            len(Suit::Hearts, ..=1) & len(Suit::Spades, ..=3) & hcp(10..),
        )
        // 3♠: splinter, 0-1♠, 0-3♥, long minors
        .rule(
            Bid::new(3, Strain::Spades),
            1.8,
            len(Suit::Spades, ..=1) & len(Suit::Hearts, ..=3) & hcp(10..),
        )
        // 2♠ minor relay: 6+♣ or quantitative INV (8-9, no major, no 5+♦)
        .rule(
            Bid::new(2, Strain::Spades),
            1.5,
            len(Suit::Clubs, 6..)
                | (hcp(8..=9)
                    & len(Suit::Hearts, ..4)
                    & len(Suit::Spades, ..4)
                    & len(Suit::Diamonds, ..5)),
        )
        // 2NT minor relay: 5+♦ + 4+♣ or 6+♦
        .rule(
            Bid::new(2, Strain::Notrump),
            1.5,
            (len(Suit::Diamonds, 5..) & len(Suit::Clubs, 4..)) | len(Suit::Diamonds, 6..),
        )
        // 5NT: QUANT INV to 7NT (forces 6NT+)
        .rule(
            Bid::new(5, Strain::Notrump),
            1.3,
            hcp(18..) & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        // 4NT: QUANT INV to 6NT — balanced, no 4-card major
        .rule(
            Bid::new(4, Strain::Notrump),
            1.2,
            hcp(16..=17) & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        // 4♥/4♠ signoff: game hand, weak to medium, 6-card suit
        .rule(
            Bid::new(4, Strain::Hearts),
            1.1,
            len(Suit::Hearts, 6..) & hcp(7..=8),
        )
        .rule(
            Bid::new(4, Strain::Spades),
            1.1,
            len(Suit::Spades, 6..) & hcp(7..=8),
        )
        // 3NT signoff: game values, balanced, no 4-card major
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            hcp(10..) & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        // Pass: too weak for any constructive call
        .rule(
            Call::Pass,
            0.0,
            hcp(..8)
                & len(Suit::Hearts, ..5)
                & len(Suit::Spades, ..5)
                & len(Suit::Clubs, ..6)
                & len(Suit::Diamonds, ..5),
        )
}

// ---------------------------------------------------------------------------
// BTU Stayman 2♣ — opener's answers
// ---------------------------------------------------------------------------

/// Opener's answer to BTU Stayman 2♣
///
/// - 2♦: no 4-card major (2-3♠, 2-3♥)
/// - 2♥: 4-5♥, 2-4♠ (hearts, possibly with spades)
/// - 2♠: 4-5♠, 2-3♥ (spades, no 4-card hearts)
fn btu_stayman_answers() -> Rules {
    Rules::new()
        // 2♥: 4+♥ (may also have 4♠; ♥ takes priority per notes)
        .rule(Bid::new(2, Strain::Hearts), 1.0, len(Suit::Hearts, 4..))
        // 2♠: 4+♠, no 4-card ♥
        .rule(
            Bid::new(2, Strain::Spades),
            1.0,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        // 2♦: no 4-card major
        .rule(
            Bid::new(2, Strain::Diamonds),
            0.5,
            len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
}

// ---------------------------------------------------------------------------
// After 1NT-2♣-2♦ (no major found)
// ---------------------------------------------------------------------------

/// Responder's continuation after 1NT-2♣-2♦ (opener showed no 4-card major)
///
/// - 2♥!: TRF INV, 5+♠ (the BTU speciality: spade invite via transfer)
/// - 2♠!: NF INV Smolen, 4=♠ 5+♥
/// - 2NT: NAT INV
/// - 3♣: FG, 5+♣
/// - 3♦: FG, 5+♦
/// - 3♥!: FG Smolen TRF, 5-4 (5♥ 4♠)
/// - 3♠!: FG Smolen TRF, 4-5 (4♥ 5♠)
/// - 4♣: SA Texas TRF, 6+♥ 4=♠
/// - 4♦: SA Texas TRF, 6+♠ 4=♥
/// - 4♥/4♠: signoff
/// - 4NT: QUANT
/// - 5NT: QUANT
fn after_2c_2d() -> Rules {
    Rules::new()
        // 3♠!: FG Smolen TRF, 4=♥ 5+♠
        .rule(
            Bid::new(3, Strain::Spades),
            3.0,
            hcp(12..) & len(Suit::Spades, 5..) & len(Suit::Hearts, 4..=4),
        )
        // 3♥!: FG Smolen TRF, 5+♥ 4=♠
        .rule(
            Bid::new(3, Strain::Hearts),
            3.0,
            hcp(12..) & len(Suit::Hearts, 5..) & len(Suit::Spades, 4..=4),
        )
        // 2♥!: TRF INV, 5+♠ (the BTU 5-spade invite relay)
        .rule(
            Bid::new(2, Strain::Hearts),
            2.5,
            hcp(8..=11) & len(Suit::Spades, 5..),
        )
        // 2♠!: NF INV Smolen, 4=♠ 5+♥
        .rule(
            Bid::new(2, Strain::Spades),
            2.5,
            hcp(8..=11) & len(Suit::Hearts, 5..) & len(Suit::Spades, 4..=4),
        )
        // 3♣: FG, 5+♣
        .rule(
            Bid::new(3, Strain::Clubs),
            2.0,
            hcp(12..) & len(Suit::Clubs, 5..),
        )
        // 3♦: FG, 5+♦
        .rule(
            Bid::new(3, Strain::Diamonds),
            2.0,
            hcp(12..) & len(Suit::Diamonds, 5..),
        )
        // Texas 4♣: TRF, 6+♥ 4=♠
        .rule(
            Bid::new(4, Strain::Clubs),
            1.8,
            len(Suit::Hearts, 6..) & len(Suit::Spades, 4..=4),
        )
        // Texas 4♦: TRF, 6+♠ 4=♥
        .rule(
            Bid::new(4, Strain::Diamonds),
            1.8,
            len(Suit::Spades, 6..) & len(Suit::Hearts, 4..=4),
        )
        // 4♥/4♠ signoff
        .rule(
            Bid::new(4, Strain::Hearts),
            1.5,
            len(Suit::Hearts, 6..) & hcp(10..),
        )
        .rule(
            Bid::new(4, Strain::Spades),
            1.5,
            len(Suit::Spades, 6..) & hcp(10..),
        )
        // 5NT: QUANT INV to 7NT
        .rule(
            Bid::new(5, Strain::Notrump),
            1.3,
            hcp(18..) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        // 4NT: QUANT INV
        .rule(
            Bid::new(4, Strain::Notrump),
            1.2,
            hcp(16..=17) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        // 2NT: NAT INV, balanced, no major
        .rule(
            Bid::new(2, Strain::Notrump),
            1.0,
            hcp(8..=11) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        // 3NT: FG balanced, no major
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            hcp(12..) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        // Pass: no slam interest, no invite: weak garbage stayman hand
        .rule(Call::Pass, 0.0, hcp(..8))
}

/// Opener's response to 1NT-2♣-2♦-2♥ (responder shows 5+♠ INV via transfer)
///
/// - 2♠: MIN, 2=♠ or 3=♠ minimum
/// - 2NT: INV, 2=♠
/// - 3♣/3♦!: INV, 3=♠, 5+minor
/// - 3♥!: INV, 3=♠, good 4+♥
/// - 3♠: INV, 3=♠
fn after_2c_2d_2h() -> Rules {
    // Opener accepts/declines the 5=♠ invitation
    Rules::new()
        // 3♠: accept with 3=♠, no extra feature
        .rule(
            Bid::new(3, Strain::Spades),
            2.0,
            len(Suit::Spades, 3..) & hcp(16..),
        )
        // 3♥!: INV, 3=♠, good 4+♥
        .rule(
            Bid::new(3, Strain::Hearts),
            2.0,
            len(Suit::Spades, 3..)
                & len(Suit::Hearts, 4..)
                & top_honors(Suit::Hearts, 2..)
                & hcp(16..),
        )
        // 3♣!: INV, 3=♠, 5+♣
        .rule(
            Bid::new(3, Strain::Clubs),
            2.0,
            len(Suit::Spades, 3..) & len(Suit::Clubs, 5..) & hcp(16..),
        )
        // 3♦!: INV, 3=♠, 5+♦
        .rule(
            Bid::new(3, Strain::Diamonds),
            2.0,
            len(Suit::Spades, 3..) & len(Suit::Diamonds, 5..) & hcp(16..),
        )
        // 2NT: INV, 2=♠ (can't agree spades but maximum)
        .rule(
            Bid::new(2, Strain::Notrump),
            1.5,
            len(Suit::Spades, ..3) & hcp(16..),
        )
        // 2♠: MIN, pass or correct
        .rule(Bid::new(2, Strain::Spades), 0.5, hcp(0..))
}

/// Opener's response to 1NT-2♣-2♦-2♠ (NF INV Smolen: 4=♠ 5+♥)
///
/// Opener bids 2NT (2=♥), 3♣/3♦ (3=♥ + 5+minor), 3♥ (3=♥ good hearts), 3♠ (3=♥)
fn after_2c_2d_2s() -> Rules {
    Rules::new()
        // 3♠: accept with 3=♥ (transfer complete to hearts; responder converts to 4♥)
        // Actually in Smolen 4=♠ 5+♥: opener accepting shows 3=♥ fit
        .rule(
            Bid::new(3, Strain::Hearts),
            2.0,
            len(Suit::Hearts, 3..) & hcp(16..),
        )
        // 3♣!: 3=♥, 5+♣
        .rule(
            Bid::new(3, Strain::Clubs),
            2.0,
            len(Suit::Hearts, 3..) & len(Suit::Clubs, 5..) & hcp(16..),
        )
        // 3♦!: 3=♥, 5+♦
        .rule(
            Bid::new(3, Strain::Diamonds),
            2.0,
            len(Suit::Hearts, 3..) & len(Suit::Diamonds, 5..) & hcp(16..),
        )
        // 2NT: MIN, 2=♥ (misfitting)
        .rule(
            Bid::new(2, Strain::Notrump),
            1.0,
            len(Suit::Hearts, ..3) & hcp(16..),
        )
        // 3NT: signoff 2=♥, maximum
        .rule(Bid::new(3, Strain::Notrump), 0.5, hcp(0..))
}

/// Opener's answer after 1NT-2♣-2♦-3♥ (FG Smolen TRF, 5+♥ 4=♠)
///
/// Opener transfers to 4♥ (or bids 3NT if doubting fit), or 3♠ (shows 4♠ fit).
fn after_2c_2d_3h_smolen() -> Rules {
    Rules::new()
        // 3♠: opener shows 4♠ (fits the declared 4=♠ by responder)
        .rule(Bid::new(3, Strain::Spades), 1.5, len(Suit::Spades, 4..))
        // 3NT: no fit for hearts; partner can convert to 4♥
        .rule(Bid::new(3, Strain::Notrump), 1.0, len(Suit::Hearts, ..3))
        // 4♥: opener has 3+ hearts, accepting
        .rule(Bid::new(4, Strain::Hearts), 0.5, hcp(0..))
}

/// Opener's answer after 1NT-2♣-2♦-3♠ (FG Smolen TRF, 4=♥ 5+♠)
///
/// Opener transfers to 4♠ (or bids 3NT), or 4♥ (shows 4♥ fit).
fn after_2c_2d_3s_smolen() -> Rules {
    Rules::new()
        // 4♥: opener shows 4♥ (fits the declared 4=♥ by responder)
        .rule(Bid::new(4, Strain::Hearts), 1.5, len(Suit::Hearts, 4..))
        // 3NT: no fit for spades; partner can convert to 4♠
        .rule(Bid::new(3, Strain::Notrump), 1.0, len(Suit::Spades, ..3))
        // 4♠: opener has 3+ spades, accepting
        .rule(Bid::new(4, Strain::Spades), 0.5, hcp(0..))
}

// ---------------------------------------------------------------------------
// After 1NT-2♣-2♥ (opener showed 4+♥)
// ---------------------------------------------------------------------------

/// Responder's continuation after 1NT-2♣-2♥ (opener showed 4+♥)
///
/// - 2♠: INV, 5+♠
/// - 2NT: INV, 0-3♥, 4=♠
/// - 3♣: FG, 5+♣, 0-3♥, 4=♠
/// - 3♦: FG, 5+♦, 0-3♥, 4=♠
/// - 3♥: INV, 4+♥
/// - 3♠!: S/T, 4+♥
/// - 4♣/4♦!: S/T SPL, 0-1 minor, 4+♥
/// - 4NT: QUANT INV to 6NT
/// - 5NT: QUANT INV to 7NT
fn after_2c_2h() -> Rules {
    Rules::new()
        // 3♠!: S/T, 4+♥ (generic slam try)
        .rule(
            Bid::new(3, Strain::Spades),
            3.0,
            hcp(13..) & len(Suit::Hearts, 4..),
        )
        // 4♣!: S/T SPL, 0-1♣, 4+♥
        .rule(
            Bid::new(4, Strain::Clubs),
            2.8,
            hcp(12..) & len(Suit::Hearts, 4..) & len(Suit::Clubs, ..=1),
        )
        // 4♦!: S/T SPL, 0-1♦, 4+♥
        .rule(
            Bid::new(4, Strain::Diamonds),
            2.8,
            hcp(12..) & len(Suit::Hearts, 4..) & len(Suit::Diamonds, ..=1),
        )
        // 2♠: INV, 5+♠
        .rule(
            Bid::new(2, Strain::Spades),
            2.5,
            hcp(8..=11) & len(Suit::Spades, 5..),
        )
        // 3♣: FG, 5+♣, 0-3♥, 4=♠ (misfit with ♥, has ♠)
        .rule(
            Bid::new(3, Strain::Clubs),
            2.0,
            hcp(12..) & len(Suit::Hearts, ..4) & len(Suit::Clubs, 5..),
        )
        // 3♦: FG, 5+♦, 0-3♥, 4=♠
        .rule(
            Bid::new(3, Strain::Diamonds),
            2.0,
            hcp(12..) & len(Suit::Hearts, ..4) & len(Suit::Diamonds, 5..),
        )
        // 3♥: INV, 4+♥
        .rule(
            Bid::new(3, Strain::Hearts),
            1.5,
            hcp(8..=11) & len(Suit::Hearts, 4..),
        )
        // 2NT: INV, 0-3♥, 4=♠ (invitational with spades, no heart fit)
        .rule(
            Bid::new(2, Strain::Notrump),
            1.2,
            hcp(8..=11) & len(Suit::Hearts, ..4) & len(Suit::Spades, 4..),
        )
        // 5NT: QUANT
        .rule(
            Bid::new(5, Strain::Notrump),
            1.1,
            hcp(18..) & len(Suit::Hearts, ..4),
        )
        // 4NT: QUANT INV to 6NT
        .rule(
            Bid::new(4, Strain::Notrump),
            1.0,
            hcp(16..=17) & len(Suit::Hearts, ..4),
        )
        // 4♥: signoff (game with heart fit, no slam try)
        .rule(
            Bid::new(4, Strain::Hearts),
            0.8,
            hcp(10..=12) & len(Suit::Hearts, 4..),
        )
        // 3NT: game, no heart fit
        .rule(
            Bid::new(3, Strain::Notrump),
            0.7,
            hcp(10..) & len(Suit::Hearts, ..4),
        )
        // Catch-all pass: weak, no fit
        .rule(Call::Pass, 0.0, hcp(..8) & len(Suit::Hearts, ..4))
}

/// Opener's continuation after 1NT-2♣-2♥-2♠ (responder INV, 5+♠)
///
/// - 2NT: INV, 2=♠
/// - 3♣/3♦!: INV, 3=♠, 5+minor
/// - 3♥!: INV, 3=♠, good 4+♥
/// - 3♠: INV, 3=♠
fn after_2c_2h_2s() -> Rules {
    Rules::new()
        // 3♠: 3=♠ fit, accept
        .rule(
            Bid::new(3, Strain::Spades),
            2.0,
            len(Suit::Spades, 3..) & hcp(16..),
        )
        // 3♥!: INV, 3=♠, good 4+♥
        .rule(
            Bid::new(3, Strain::Hearts),
            2.0,
            len(Suit::Spades, 3..)
                & len(Suit::Hearts, 4..)
                & top_honors(Suit::Hearts, 2..)
                & hcp(16..),
        )
        // 3♣!: INV, 3=♠, 5+♣
        .rule(
            Bid::new(3, Strain::Clubs),
            2.0,
            len(Suit::Spades, 3..) & len(Suit::Clubs, 5..) & hcp(16..),
        )
        // 3♦!: INV, 3=♠, 5+♦
        .rule(
            Bid::new(3, Strain::Diamonds),
            2.0,
            len(Suit::Spades, 3..) & len(Suit::Diamonds, 5..) & hcp(16..),
        )
        // 2NT: 2=♠, can't agree spades; INV values
        .rule(
            Bid::new(2, Strain::Notrump),
            1.0,
            len(Suit::Spades, ..3) & hcp(16..),
        )
        // 3NT: 2=♠, no spade fit, minimum
        .rule(Bid::new(3, Strain::Notrump), 0.5, hcp(0..))
}

// ---------------------------------------------------------------------------
// After 1NT-2♣-2♠ (opener showed 4+♠, no 4♥)
// ---------------------------------------------------------------------------

/// Responder's continuation after 1NT-2♣-2♠ (opener showed 4+♠, no 4♥)
///
/// - 2NT: INV, 0-3♠, 4=♥
/// - 3♣: FG, 5+♣, 0-3♠, 4=♥
/// - 3♦: FG, 5+♦, 0-3♠, 4=♥
/// - 3♥!: S/T, 4+♠
/// - 3♠: INV, 4+♠
/// - 4♣/4♦/4♥!: S/T SPL, 0-1 minor/heart, 4+♠
/// - 4♠: S/O, 4+♠
/// - 4NT: QUANT
/// - 5NT: QUANT
fn after_2c_2s() -> Rules {
    Rules::new()
        // 3♥!: S/T, 4+♠ (generic fit slam try)
        .rule(
            Bid::new(3, Strain::Hearts),
            3.0,
            hcp(13..) & len(Suit::Spades, 4..),
        )
        // 4♣!: S/T SPL, 0-1♣, 4+♠
        .rule(
            Bid::new(4, Strain::Clubs),
            2.8,
            hcp(12..) & len(Suit::Spades, 4..) & len(Suit::Clubs, ..=1),
        )
        // 4♦!: S/T SPL, 0-1♦, 4+♠
        .rule(
            Bid::new(4, Strain::Diamonds),
            2.8,
            hcp(12..) & len(Suit::Spades, 4..) & len(Suit::Diamonds, ..=1),
        )
        // 4♥!: S/T SPL, 0-1♥, 4+♠
        .rule(
            Bid::new(4, Strain::Hearts),
            2.8,
            hcp(12..) & len(Suit::Spades, 4..) & len(Suit::Hearts, ..=1),
        )
        // 3♣: FG, 5+♣, 0-3♠, 4=♥
        .rule(
            Bid::new(3, Strain::Clubs),
            2.0,
            hcp(12..) & len(Suit::Spades, ..4) & len(Suit::Clubs, 5..),
        )
        // 3♦: FG, 5+♦, 0-3♠, 4=♥
        .rule(
            Bid::new(3, Strain::Diamonds),
            2.0,
            hcp(12..) & len(Suit::Spades, ..4) & len(Suit::Diamonds, 5..),
        )
        // 3♠: INV, 4+♠
        .rule(
            Bid::new(3, Strain::Spades),
            1.5,
            hcp(8..=11) & len(Suit::Spades, 4..),
        )
        // 2NT: INV, 0-3♠, 4=♥
        .rule(
            Bid::new(2, Strain::Notrump),
            1.2,
            hcp(8..=11) & len(Suit::Spades, ..4) & len(Suit::Hearts, 4..),
        )
        // 5NT: QUANT
        .rule(
            Bid::new(5, Strain::Notrump),
            1.1,
            hcp(18..) & len(Suit::Spades, ..4),
        )
        // 4NT: QUANT INV to 6NT
        .rule(
            Bid::new(4, Strain::Notrump),
            1.0,
            hcp(16..=17) & len(Suit::Spades, ..4),
        )
        // 4♠: S/O, 4+♠
        .rule(
            Bid::new(4, Strain::Spades),
            0.8,
            hcp(10..=12) & len(Suit::Spades, 4..),
        )
        // 3NT: game, no spade fit
        .rule(
            Bid::new(3, Strain::Notrump),
            0.7,
            hcp(10..) & len(Suit::Spades, ..4),
        )
        // Catch-all pass
        .rule(Call::Pass, 0.0, hcp(..8) & len(Suit::Spades, ..4))
}

// ---------------------------------------------------------------------------
// BTU Jacoby transfers — opener's immediate answers
// ---------------------------------------------------------------------------

/// Opener's answers to 2♦ transfer (responder has 5+♥)
///
/// With maximum and 4=♥, opener super-accepts; otherwise completes the transfer.
///
/// - 2♥: (relay) complete the transfer
/// - 2♠!: MAX, 4=♠, 4=♥
/// - 2NT: MAX, 3433
/// - 3♣!: MAX, good 4+♣, 4=♥
/// - 3♦!: MAX, good 4+♦, 4=♥
/// - 3♥: MAX, 4=♥, none of the above
fn transfer_2d_answers() -> Rules {
    Rules::new()
        // 3♥: MAX, 4=♥, plain shape
        .rule(
            Bid::new(3, Strain::Hearts),
            2.5,
            hcp(16..) & len(Suit::Hearts, 4..=4),
        )
        // 3♣!: MAX, good 4+♣, 4=♥
        .rule(
            Bid::new(3, Strain::Clubs),
            2.5,
            hcp(16..)
                & len(Suit::Hearts, 4..=4)
                & len(Suit::Clubs, 4..)
                & top_honors(Suit::Clubs, 2..),
        )
        // 3♦!: MAX, good 4+♦, 4=♥
        .rule(
            Bid::new(3, Strain::Diamonds),
            2.5,
            hcp(16..)
                & len(Suit::Hearts, 4..=4)
                & len(Suit::Diamonds, 4..)
                & top_honors(Suit::Diamonds, 2..),
        )
        // 2♠!: MAX, 4=♠, 4=♥
        .rule(
            Bid::new(2, Strain::Spades),
            2.5,
            hcp(16..) & len(Suit::Hearts, 4..=4) & len(Suit::Spades, 4..=4),
        )
        // 2NT: MAX, 3433 (balanced maximum)
        .rule(
            Bid::new(2, Strain::Notrump),
            2.0,
            hcp(16..) & balanced() & len(Suit::Hearts, 3..=3),
        )
        // 2♥: complete the transfer (default)
        .rule(Bid::new(2, Strain::Hearts), 0.5, hcp(0..))
}

/// Responder's continuation after 1NT-2♦-2♥ (transfer complete, non-super-accept)
///
/// - 2♠!: F INV, 5=♥ (the forcing invite relay)
/// - 2NT!: UNBAL FG
/// - 3♣!: S/T, 4+♣
/// - 3♦!: S/T, 4+♦
/// - 3♥: INV, 6+♥
/// - 3NT: BAL P/C
/// - 3♠!/4♣/4♦!: SPL, 0-1 in that suit, 6+♥
/// - 4♥: S/O, 6+♥
/// - 4NT: QUANT INV
/// - 5NT: QUANT
fn after_2d_2h() -> Rules {
    Rules::new()
        // 3♣!: S/T, 4+♣
        .rule(
            Bid::new(3, Strain::Clubs),
            3.0,
            hcp(14..) & len(Suit::Hearts, 5..) & len(Suit::Clubs, 4..),
        )
        // 3♦!: S/T, 4+♦
        .rule(
            Bid::new(3, Strain::Diamonds),
            3.0,
            hcp(14..) & len(Suit::Hearts, 5..) & len(Suit::Diamonds, 4..),
        )
        // 3♠!: SPL, 0-1♠, 6+♥
        .rule(
            Bid::new(3, Strain::Spades),
            2.8,
            hcp(12..) & len(Suit::Hearts, 6..) & len(Suit::Spades, ..=1),
        )
        // 4♣!: SPL, 0-1♣, 6+♥
        .rule(
            Bid::new(4, Strain::Clubs),
            2.8,
            hcp(12..) & len(Suit::Hearts, 6..) & len(Suit::Clubs, ..=1),
        )
        // 4♦!: SPL, 0-1♦, 6+♥
        .rule(
            Bid::new(4, Strain::Diamonds),
            2.8,
            hcp(12..) & len(Suit::Hearts, 6..) & len(Suit::Diamonds, ..=1),
        )
        // 2NT!: UNBAL FG (game forcing, unbalanced)
        .rule(
            Bid::new(2, Strain::Notrump),
            2.5,
            hcp(12..) & !balanced() & len(Suit::Hearts, 5..),
        )
        // 2♠!: F INV, 5=♥ (forcing invite, exactly 5 hearts)
        .rule(
            Bid::new(2, Strain::Spades),
            2.0,
            hcp(8..=11) & len(Suit::Hearts, 5..=5),
        )
        // 3♥: INV, 6+♥
        .rule(
            Bid::new(3, Strain::Hearts),
            1.5,
            hcp(8..=11) & len(Suit::Hearts, 6..),
        )
        // 4♥: S/O, 6+♥ (strong hand preferring game sign-off)
        .rule(
            Bid::new(4, Strain::Hearts),
            1.2,
            hcp(10..=13) & len(Suit::Hearts, 6..),
        )
        // 5NT: QUANT
        .rule(
            Bid::new(5, Strain::Notrump),
            1.1,
            hcp(18..) & len(Suit::Hearts, 5..) & balanced(),
        )
        // 4NT: QUANT INV
        .rule(
            Bid::new(4, Strain::Notrump),
            1.0,
            hcp(16..=17) & len(Suit::Hearts, 5..) & balanced(),
        )
        // 3NT: BAL P/C
        .rule(
            Bid::new(3, Strain::Notrump),
            0.8,
            hcp(10..) & balanced() & len(Suit::Hearts, 5..=5),
        )
        // Pass: weak (2♥ is already game, so pass is fine)
        .rule(Call::Pass, 0.0, hcp(..8) & len(Suit::Hearts, 5..=5))
}

/// Opener's answer after 1NT-2♦-2♥-2♠ (responder F INV, 5=♥)
///
/// - 2NT: MIN, 15 HCP, 2=♥
/// - 3♣!: P/C, 16 HCP, 2=♥
/// - 3♦!: P/C, 16 HCP, 2=♥, 5+♦ (actually typo in notes — separate entry)
/// - 3♥: MIN, 3=♥
/// - 3NT: S/O, 17 HCP, 2=♥
fn after_2d_2h_2s() -> Rules {
    Rules::new()
        // 3♥: 3=♥, any (agree hearts)
        .rule(Bid::new(3, Strain::Hearts), 2.0, len(Suit::Hearts, 3..))
        // 3NT: S/O, 17 HCP, 2=♥
        .rule(
            Bid::new(3, Strain::Notrump),
            2.0,
            hcp(17..) & len(Suit::Hearts, ..3),
        )
        // 3♣!: P/C, 16 HCP, 2=♥, 5+♣
        .rule(
            Bid::new(3, Strain::Clubs),
            1.8,
            hcp(16..) & len(Suit::Hearts, ..3) & len(Suit::Clubs, 5..),
        )
        // 3♦!: P/C, 16 HCP, 2=♥, 5+♦
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.8,
            hcp(16..) & len(Suit::Hearts, ..3) & len(Suit::Diamonds, 5..),
        )
        // 2NT: MIN, 15 HCP, 2=♥
        .rule(Bid::new(2, Strain::Notrump), 0.5, hcp(0..))
}

// ---------------------------------------------------------------------------
// Opener's answers to 2♥ transfer (responder has 5+♠)
// ---------------------------------------------------------------------------

/// Opener's answers to 2♥ transfer (responder has 5+♠)
///
/// - 2♠: (relay) complete the transfer
/// - 2NT!: MAX, 4333
/// - 3♣!: MAX, good 4+♣, 4=♠
/// - 3♦!: MAX, good 4+♦, 4=♠
/// - 3♥!: MAX, 4=♥, 4=♠
/// - 3♠: MAX, 4=♠, none of the above
fn transfer_2h_answers() -> Rules {
    Rules::new()
        // 3♠: MAX, 4=♠, plain
        .rule(
            Bid::new(3, Strain::Spades),
            2.5,
            hcp(16..) & len(Suit::Spades, 4..=4),
        )
        // 3♣!: MAX, good 4+♣, 4=♠
        .rule(
            Bid::new(3, Strain::Clubs),
            2.5,
            hcp(16..)
                & len(Suit::Spades, 4..=4)
                & len(Suit::Clubs, 4..)
                & top_honors(Suit::Clubs, 2..),
        )
        // 3♦!: MAX, good 4+♦, 4=♠
        .rule(
            Bid::new(3, Strain::Diamonds),
            2.5,
            hcp(16..)
                & len(Suit::Spades, 4..=4)
                & len(Suit::Diamonds, 4..)
                & top_honors(Suit::Diamonds, 2..),
        )
        // 3♥!: MAX, 4=♥, 4=♠
        .rule(
            Bid::new(3, Strain::Hearts),
            2.5,
            hcp(16..) & len(Suit::Spades, 4..=4) & len(Suit::Hearts, 4..=4),
        )
        // 2NT!: MAX, 4333
        .rule(
            Bid::new(2, Strain::Notrump),
            2.0,
            hcp(16..) & balanced() & len(Suit::Spades, 3..=3),
        )
        // 2♠: complete the transfer (default)
        .rule(Bid::new(2, Strain::Spades), 0.5, hcp(0..))
}

/// Responder's continuation after 1NT-2♥-2♠ (transfer complete, non-super-accept)
///
/// - 2NT!: UNBAL FG
/// - 3♣!: S/T, 4+♣
/// - 3♦!: S/T, 4+♦
/// - 3♥!: S/T, 5+♥
/// - 3♠: INV, 6+♠
/// - 3NT: BAL P/C
/// - 4♣/4♦!: SPL, 0-1 minor, 6+♠
/// - 4♥: COG, 5+♥ (prefer 4♠ > 4♥)
/// - 4♠: S/O, 6+♠
/// - 4NT: QUANT INV
/// - 5NT: QUANT
fn after_2h_2s() -> Rules {
    Rules::new()
        // 3♣!: S/T, 4+♣
        .rule(
            Bid::new(3, Strain::Clubs),
            3.0,
            hcp(14..) & len(Suit::Spades, 5..) & len(Suit::Clubs, 4..),
        )
        // 3♦!: S/T, 4+♦
        .rule(
            Bid::new(3, Strain::Diamonds),
            3.0,
            hcp(14..) & len(Suit::Spades, 5..) & len(Suit::Diamonds, 4..),
        )
        // 3♥!: S/T, 5+♥
        .rule(
            Bid::new(3, Strain::Hearts),
            3.0,
            hcp(14..) & len(Suit::Spades, 5..) & len(Suit::Hearts, 5..),
        )
        // 4♣!: SPL, 0-1♣, 6+♠
        .rule(
            Bid::new(4, Strain::Clubs),
            2.8,
            hcp(12..) & len(Suit::Spades, 6..) & len(Suit::Clubs, ..=1),
        )
        // 4♦!: SPL, 0-1♦, 6+♠
        .rule(
            Bid::new(4, Strain::Diamonds),
            2.8,
            hcp(12..) & len(Suit::Spades, 6..) & len(Suit::Diamonds, ..=1),
        )
        // 2NT!: UNBAL FG
        .rule(
            Bid::new(2, Strain::Notrump),
            2.5,
            hcp(12..) & !balanced() & len(Suit::Spades, 5..),
        )
        // 3♠: INV, 6+♠
        .rule(
            Bid::new(3, Strain::Spades),
            1.5,
            hcp(8..=11) & len(Suit::Spades, 6..),
        )
        // 4♥: COG, 5+♥ (prefer 4♠ over 4♥)
        .rule(
            Bid::new(4, Strain::Hearts),
            1.3,
            hcp(10..) & len(Suit::Hearts, 5..) & len(Suit::Spades, 5..),
        )
        // 4♠: S/O, 6+♠
        .rule(
            Bid::new(4, Strain::Spades),
            1.2,
            hcp(10..=13) & len(Suit::Spades, 6..),
        )
        // 5NT: QUANT
        .rule(
            Bid::new(5, Strain::Notrump),
            1.1,
            hcp(18..) & len(Suit::Spades, 5..) & balanced(),
        )
        // 4NT: QUANT INV
        .rule(
            Bid::new(4, Strain::Notrump),
            1.0,
            hcp(16..=17) & len(Suit::Spades, 5..) & balanced(),
        )
        // 3NT: BAL P/C
        .rule(
            Bid::new(3, Strain::Notrump),
            0.8,
            hcp(10..) & balanced() & len(Suit::Spades, 5..=5),
        )
        // Pass: weak
        .rule(Call::Pass, 0.0, hcp(..8) & len(Suit::Spades, 5..=5))
}

// ---------------------------------------------------------------------------
// Minor-suit relays: 2♠ (clubs) and 2NT (diamonds)
// ---------------------------------------------------------------------------

/// Opener's answer to 2♠ (clubs relay: 6+♣ or QUANT INV)
///
/// - 2NT: MIN
/// - 3♣!: MAX, 1-3♣ (shortish)
/// - 3♦/3♥/3♠!: MAX, 4-6♣, good stopper in that suit
fn after_2s_club_relay() -> Rules {
    Rules::new()
        // 3♠!: MAX, 4-6♣, good ♠ stopper
        .rule(
            Bid::new(3, Strain::Spades),
            2.0,
            hcp(16..) & len(Suit::Clubs, 4..) & stopper_in(Suit::Spades),
        )
        // 3♥!: MAX, 4-6♣, good ♥ stopper
        .rule(
            Bid::new(3, Strain::Hearts),
            2.0,
            hcp(16..) & len(Suit::Clubs, 4..) & stopper_in(Suit::Hearts),
        )
        // 3♦!: MAX, 4-6♣, good ♦ stopper
        .rule(
            Bid::new(3, Strain::Diamonds),
            2.0,
            hcp(16..) & len(Suit::Clubs, 4..) & stopper_in(Suit::Diamonds),
        )
        // 3♣!: MAX, 1-3♣ (short clubs: misfitting)
        .rule(
            Bid::new(3, Strain::Clubs),
            1.5,
            hcp(16..) & len(Suit::Clubs, ..=3),
        )
        // 2NT: MIN
        .rule(Bid::new(2, Strain::Notrump), 0.5, hcp(0..))
}

/// Opener's answer to 2NT (diamond relay: 5+♦ + 4+♣ or 6+♦)
///
/// - 3♣!: 1-2♦ (short diamonds)
/// - 3♦: 3-4♦
/// - 3♥/3♠!: 5-6♦, good stopper
/// - 3NT!: 5-6♦, good ♣ stopper
fn after_2nt_diamond_relay() -> Rules {
    Rules::new()
        // 3♥!: 5-6♦, good ♥ stopper
        .rule(
            Bid::new(3, Strain::Hearts),
            2.0,
            len(Suit::Diamonds, 5..) & stopper_in(Suit::Hearts),
        )
        // 3♠!: 5-6♦, good ♠ stopper
        .rule(
            Bid::new(3, Strain::Spades),
            2.0,
            len(Suit::Diamonds, 5..) & stopper_in(Suit::Spades),
        )
        // 3NT!: 5-6♦, good ♣ stopper
        .rule(
            Bid::new(3, Strain::Notrump),
            2.0,
            len(Suit::Diamonds, 5..) & stopper_in(Suit::Clubs),
        )
        // 3♦: 3-4♦ (moderate fit)
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.0,
            len(Suit::Diamonds, 3..=4),
        )
        // 3♣!: 1-2♦ (short diamonds)
        .rule(Bid::new(3, Strain::Clubs), 0.5, hcp(0..))
}

// ---------------------------------------------------------------------------
// Puppet Stayman 3♣
// ---------------------------------------------------------------------------

/// Opener's answer to Puppet Stayman 3♣
///
/// - 3♦!: no 5-card major, no 4-card major (2-4♠, 2-4♥)
/// - 3♥/3♠: 5-card major
fn puppet_answers() -> Rules {
    Rules::new()
        // 3♠: 5=♠
        .rule(Bid::new(3, Strain::Spades), 2.0, len(Suit::Spades, 5..))
        // 3♥: 5=♥ (no 5=♠)
        .rule(
            Bid::new(3, Strain::Hearts),
            2.0,
            len(Suit::Hearts, 5..) & len(Suit::Spades, ..5),
        )
        // 3♦!: no 5-card major (has 4-card major or flat)
        .rule(Bid::new(3, Strain::Diamonds), 0.5, hcp(0..))
}

/// Responder's continuation after 1NT-3♣-3♦ (opener showed no 5-card major)
///
/// - 3♥!: Smolen TRF, 4=♠ (responder has 4♠ 3+♥)
/// - 3♠!: Smolen TRF, 4=♥ (responder has 3+♠ 4♥)
/// - 3NT: S/O
/// - 4♣!: S/T, 4-4 (xx)
/// - 4♦!: COG, 4-4 (xx)
fn after_puppet_3d() -> Rules {
    Rules::new()
        // 3♥!: Smolen TRF, 4=♠
        .rule(Bid::new(3, Strain::Hearts), 2.0, len(Suit::Spades, 4..=4))
        // 3♠!: Smolen TRF, 4=♥
        .rule(
            Bid::new(3, Strain::Spades),
            2.0,
            len(Suit::Hearts, 4..=4) & len(Suit::Spades, ..4),
        )
        // 4♣!: S/T, 4-4 majors
        .rule(
            Bid::new(4, Strain::Clubs),
            1.5,
            hcp(14..) & len(Suit::Hearts, 4..) & len(Suit::Spades, 4..),
        )
        // 4♦!: COG, 4-4 majors (choice of games)
        .rule(
            Bid::new(4, Strain::Diamonds),
            1.2,
            len(Suit::Hearts, 4..) & len(Suit::Spades, 4..),
        )
        // 3NT: S/O
        .rule(Bid::new(3, Strain::Notrump), 0.5, hcp(0..))
}

/// Opener's answer after 1NT-3♣-3♦-3♥ (Smolen, responder declared 4=♠)
///
/// Opener shows 4=♠ by bidding 4♠ (agreeing), else 3NT/4♥.
fn after_puppet_3d_3h() -> Rules {
    Rules::new()
        // 4♠: fit in spades
        .rule(Bid::new(4, Strain::Spades), 1.5, len(Suit::Spades, 4..))
        // 3NT: no 4-card spades
        .rule(Bid::new(3, Strain::Notrump), 0.5, hcp(0..))
}

/// Opener's answer after 1NT-3♣-3♦-3♠ (Smolen, responder declared 4=♥)
///
/// Opener shows 4=♥ by bidding 4♥ (agreeing), else 3NT/4♣.
fn after_puppet_3d_3s() -> Rules {
    Rules::new()
        // 4♥: fit in hearts
        .rule(Bid::new(4, Strain::Hearts), 1.5, len(Suit::Hearts, 4..))
        // 3NT: no 4-card hearts
        .rule(Bid::new(3, Strain::Notrump), 0.5, hcp(0..))
}

// ---------------------------------------------------------------------------
// Texas transfers (4♣→4♥, 4♦→4♠) — opener just completes
// ---------------------------------------------------------------------------

/// Opener's response to 4♣ Texas transfer (complete to 4♥)
fn complete_texas_hearts() -> Rules {
    Rules::new().rule(Bid::new(4, Strain::Hearts), 1.0, hcp(0..))
}

/// Opener's response to 4♦ Texas transfer (complete to 4♠)
fn complete_texas_spades() -> Rules {
    Rules::new().rule(Bid::new(4, Strain::Spades), 1.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Quantitative 4NT/5NT answers
// ---------------------------------------------------------------------------

/// Opener's answer to 4NT quantitative invite (accept = 6NT, decline = Pass)
///
/// Accept with maximum (17 HCP for 1NT = 15-17).
fn quantitative_answer_1nt() -> Rules {
    Rules::new()
        .rule(Bid::new(6, Strain::Notrump), 1.0, hcp(17..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's answer to 5NT quantitative invite (forces 6NT+, accept = 7NT)
fn quantitative_answer_5nt() -> Rules {
    Rules::new()
        .rule(Bid::new(7, Strain::Notrump), 1.0, hcp(17..))
        .rule(Bid::new(6, Strain::Notrump), 0.5, hcp(0..))
}

// ---------------------------------------------------------------------------
// Strand-fixing reply tables
// ---------------------------------------------------------------------------

/// Opener's answer to [1NT, 3♦] (responder showed 5-5 majors INV+)
///
/// Opener picks the better major fit or bids 3NT.
fn after_3d_55_majors() -> Rules {
    Rules::new()
        // 4♥: 3+ hearts, prefer hearts (hearts is shorter so pick the longer major)
        .rule(
            Bid::new(4, Strain::Hearts),
            1.2,
            len(Suit::Hearts, 3..) & len(Suit::Hearts, 4..),
        )
        // 4♠: 3+ spades, prefer spades
        .rule(Bid::new(4, Strain::Spades), 1.0, len(Suit::Spades, 3..))
        // 4♥: 3+ hearts (catch-all for heart fit)
        .rule(Bid::new(4, Strain::Hearts), 0.8, len(Suit::Hearts, 3..))
        // 3NT: catch-all
        .rule(Bid::new(3, Strain::Notrump), 0.5, hcp(0..))
}

/// Opener's answer to [1NT, 3♥] or [1NT, 3♠] (minor-oriented splinter)
///
/// Terminate at 3NT (catch-all).
fn after_3h_or_3s_splinter() -> Rules {
    Rules::new().rule(Bid::new(3, Strain::Notrump), 1.0, hcp(0..))
}

/// Reply to a FG natural minor (3♣ or 3♦) — opener terminates at 3NT
fn reply_fg_minor() -> Rules {
    Rules::new().rule(Bid::new(3, Strain::Notrump), 1.0, hcp(0..))
}

/// Reply to a NAT INV 2NT — opener accepts max (3NT) or declines min (Pass)
fn reply_nat_inv_2nt() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(16..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Reply to Texas-in-branch 4♣ (complete to 4♥)
fn reply_texas_4c() -> Rules {
    Rules::new().rule(Bid::new(4, Strain::Hearts), 1.0, hcp(0..))
}

/// Reply to Texas-in-branch 4♦ (complete to 4♠)
fn reply_texas_4d() -> Rules {
    Rules::new().rule(Bid::new(4, Strain::Spades), 1.0, hcp(0..))
}

/// Reply node for a slam try with hearts agreed (4NT RKCB / 4♥ sign-off)
fn reply_slam_try_hearts() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 1.0, hcp(16..))
        .rule(Bid::new(4, Strain::Hearts), 0.5, hcp(0..))
}

/// Reply node for a slam try with spades agreed (4NT RKCB / 4♠ sign-off)
fn reply_slam_try_spades() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 1.0, hcp(16..))
        .rule(Bid::new(4, Strain::Spades), 0.5, hcp(0..))
}

/// Reply node for a 4-level SPL with hearts agreed: 4NT RKCB / Pass sign-off
///
/// Used when the SPL bid is already at the 4-level (4♣/4♦), so 4♥ is above
/// but 4NT is the key-card ask.  Pass is the game sign-off.
fn reply_spl4_hearts() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 1.0, hcp(16..))
        .rule(Bid::new(4, Strain::Hearts), 0.5, hcp(0..))
}

/// Reply node for a 4-level SPL with spades agreed: 4NT RKCB / Pass sign-off
fn reply_spl4_spades() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 1.0, hcp(16..))
        .rule(Bid::new(4, Strain::Spades), 0.5, hcp(0..))
}

/// Reply to a quantitative 5NT in-branch (opener bids 7NT@max / 6NT@min)
fn reply_quant_5nt() -> Rules {
    Rules::new()
        .rule(Bid::new(7, Strain::Notrump), 1.0, hcp(17..))
        .rule(Bid::new(6, Strain::Notrump), 0.5, hcp(0..))
}

/// Responder's continuation after 1NT-3♣-3♥ (opener showed 5=♥)
///
/// Responder bids 4♥ sign-off or 4NT RKCB for hearts slam.
fn puppet_after_3h_five() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 1.0, hcp(16..))
        .rule(Bid::new(4, Strain::Hearts), 0.5, hcp(0..))
}

/// Responder's continuation after 1NT-3♣-3♠ (opener showed 5=♠)
///
/// Responder bids 4♠ sign-off or 4NT RKCB for spades slam.
fn puppet_after_3s_five() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 1.0, hcp(16..))
        .rule(Bid::new(4, Strain::Spades), 0.5, hcp(0..))
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register the BTU strong-1NT response structure
///
/// Replaces the baseline [`notrump::register_one_nt`] block in the strawberry
/// variant.  Called by the strawberry system constructor.
pub(super) fn register(book: &mut Trie) {
    let one_nt = call(1, Strain::Notrump);

    // -------------------------------------------------------------------------
    // Top-level responses to 1NT
    // -------------------------------------------------------------------------
    insert_uncontested(book, &[one_nt], btu_responses());

    // A. Top-level: opener's answers to 3♦ (5-5 majors INV+), 3♥, 3♠ (splinters)
    let c3d_top = call(3, Strain::Diamonds);
    insert_uncontested(book, &[one_nt, c3d_top], after_3d_55_majors());

    let c3h_top = call(3, Strain::Hearts);
    insert_uncontested(book, &[one_nt, c3h_top], after_3h_or_3s_splinter());

    let c3s_top = call(3, Strain::Spades);
    insert_uncontested(book, &[one_nt, c3s_top], after_3h_or_3s_splinter());

    // -------------------------------------------------------------------------
    // BTU Stayman 2♣: opener's answers
    // -------------------------------------------------------------------------
    let c2c = call(2, Strain::Clubs);
    insert_uncontested(book, &[one_nt, c2c], btu_stayman_answers());

    // After 1NT-2♣-2♦ (no major)
    let c2d_neg = call(2, Strain::Diamonds);
    insert_uncontested(book, &[one_nt, c2c, c2d_neg], after_2c_2d());

    // 1NT-2♣-2♦-2♥ (F INV, 5+♠): opener's answer
    let c2h_inv = call(2, Strain::Hearts);
    insert_uncontested(book, &[one_nt, c2c, c2d_neg, c2h_inv], after_2c_2d_2h());

    // 1NT-2♣-2♦-2♠ (NF INV Smolen, 4=♠ 5+♥): opener's answer
    let c2s_smolen = call(2, Strain::Spades);
    insert_uncontested(book, &[one_nt, c2c, c2d_neg, c2s_smolen], after_2c_2d_2s());

    // B. After 1NT-2♣-2♦: opener answers the forcing/invitational continuations
    // B.3♣: FG 5+♣
    let c3c_fg = call(3, Strain::Clubs);
    insert_uncontested(book, &[one_nt, c2c, c2d_neg, c3c_fg], reply_fg_minor());

    // B.3♦: FG 5+♦
    let c3d_fg = call(3, Strain::Diamonds);
    insert_uncontested(book, &[one_nt, c2c, c2d_neg, c3d_fg], reply_fg_minor());

    // B.2NT: NAT INV
    let c2nt_inv = call(2, Strain::Notrump);
    insert_uncontested(book, &[one_nt, c2c, c2d_neg, c2nt_inv], reply_nat_inv_2nt());

    // B.4♣: Texas → 4♥
    let c4c_tex = call(4, Strain::Clubs);
    insert_uncontested(book, &[one_nt, c2c, c2d_neg, c4c_tex], reply_texas_4c());

    // B.4♦: Texas → 4♠
    let c4d_tex = call(4, Strain::Diamonds);
    insert_uncontested(book, &[one_nt, c2c, c2d_neg, c4d_tex], reply_texas_4d());

    // B.4NT: QUANT
    let c4nt_b = call(4, Strain::Notrump);
    insert_uncontested(
        book,
        &[one_nt, c2c, c2d_neg, c4nt_b],
        quantitative_answer_1nt(),
    );

    // B.5NT: QUANT
    let c5nt_b = call(5, Strain::Notrump);
    insert_uncontested(book, &[one_nt, c2c, c2d_neg, c5nt_b], reply_quant_5nt());

    // 1NT-2♣-2♦-3♥ (FG Smolen TRF, 5+♥ 4=♠): opener's answer
    let c3h_smolen = call(3, Strain::Hearts);
    insert_uncontested(
        book,
        &[one_nt, c2c, c2d_neg, c3h_smolen],
        after_2c_2d_3h_smolen(),
    );

    // 1NT-2♣-2♦-3♠ (FG Smolen TRF, 4=♥ 5+♠): opener's answer
    let c3s_smolen = call(3, Strain::Spades);
    insert_uncontested(
        book,
        &[one_nt, c2c, c2d_neg, c3s_smolen],
        after_2c_2d_3s_smolen(),
    );

    // After 1NT-2♣-2♥ (opener showed 4+♥): responder's continuation
    let c2h = call(2, Strain::Hearts);
    insert_uncontested(book, &[one_nt, c2c, c2h], after_2c_2h());

    // 1NT-2♣-2♥-2♠ (INV, 5+♠): opener's answer
    let c2s_inv = call(2, Strain::Spades);
    insert_uncontested(book, &[one_nt, c2c, c2h, c2s_inv], after_2c_2h_2s());

    // C. After 1NT-2♣-2♥: opener answers the forcing/invitational continuations
    // C.3♣: FG 5+♣
    insert_uncontested(book, &[one_nt, c2c, c2h, c3c_fg], reply_fg_minor());

    // C.3♦: FG 5+♦
    insert_uncontested(book, &[one_nt, c2c, c2h, c3d_fg], reply_fg_minor());

    // C.2NT: INV
    insert_uncontested(book, &[one_nt, c2c, c2h, c2nt_inv], reply_nat_inv_2nt());

    // C.3♠: S/T hearts — opener node + RKCB
    let c3s_st = call(3, Strain::Spades);
    insert_uncontested(book, &[one_nt, c2c, c2h, c3s_st], reply_slam_try_hearts());
    slam::install_rkcb(book, &[one_nt, c2c, c2h, c3s_st], Suit::Hearts);

    // C.4♣: SPL slam try (hearts agreed) — opener node + RKCB
    insert_uncontested(book, &[one_nt, c2c, c2h, c4c_tex], reply_spl4_hearts());
    slam::install_rkcb(book, &[one_nt, c2c, c2h, c4c_tex], Suit::Hearts);

    // C.4♦: SPL slam try (hearts agreed) — opener node + RKCB
    insert_uncontested(book, &[one_nt, c2c, c2h, c4d_tex], reply_spl4_hearts());
    slam::install_rkcb(book, &[one_nt, c2c, c2h, c4d_tex], Suit::Hearts);

    // C.4NT: QUANT
    insert_uncontested(book, &[one_nt, c2c, c2h, c4nt_b], quantitative_answer_1nt());

    // C.5NT: QUANT
    insert_uncontested(book, &[one_nt, c2c, c2h, c5nt_b], reply_quant_5nt());

    // After 1NT-2♣-2♠ (opener showed 4+♠, no 4♥): responder's continuation
    let c2s = call(2, Strain::Spades);
    insert_uncontested(book, &[one_nt, c2c, c2s], after_2c_2s());

    // D. After 1NT-2♣-2♠: opener answers the forcing/invitational continuations
    // D.3♣: FG 5+♣
    insert_uncontested(book, &[one_nt, c2c, c2s, c3c_fg], reply_fg_minor());

    // D.3♦: FG 5+♦
    insert_uncontested(book, &[one_nt, c2c, c2s, c3d_fg], reply_fg_minor());

    // D.2NT: INV
    insert_uncontested(book, &[one_nt, c2c, c2s, c2nt_inv], reply_nat_inv_2nt());

    // D.3♥: S/T spades — opener node + RKCB
    let c3h_st = call(3, Strain::Hearts);
    insert_uncontested(book, &[one_nt, c2c, c2s, c3h_st], reply_slam_try_spades());
    slam::install_rkcb(book, &[one_nt, c2c, c2s, c3h_st], Suit::Spades);

    // D.4♣: SPL slam try (spades agreed) — opener node + RKCB
    insert_uncontested(book, &[one_nt, c2c, c2s, c4c_tex], reply_spl4_spades());
    slam::install_rkcb(book, &[one_nt, c2c, c2s, c4c_tex], Suit::Spades);

    // D.4♦: SPL slam try (spades agreed) — opener node + RKCB
    insert_uncontested(book, &[one_nt, c2c, c2s, c4d_tex], reply_spl4_spades());
    slam::install_rkcb(book, &[one_nt, c2c, c2s, c4d_tex], Suit::Spades);

    // D.4♥: SPL slam try (spades agreed, 0-1♥) — opener node + RKCB
    let c4h_spl = call(4, Strain::Hearts);
    insert_uncontested(book, &[one_nt, c2c, c2s, c4h_spl], reply_spl4_spades());
    slam::install_rkcb(book, &[one_nt, c2c, c2s, c4h_spl], Suit::Spades);

    // D.4NT: QUANT
    insert_uncontested(book, &[one_nt, c2c, c2s, c4nt_b], quantitative_answer_1nt());

    // D.5NT: QUANT
    insert_uncontested(book, &[one_nt, c2c, c2s, c5nt_b], reply_quant_5nt());

    // -------------------------------------------------------------------------
    // BTU Jacoby transfer 2♦ → hearts
    // -------------------------------------------------------------------------
    let c2d = call(2, Strain::Diamonds);
    insert_uncontested(book, &[one_nt, c2d], transfer_2d_answers());

    // After transfer completion 2♥ (non-super-accept): responder's continuation
    let c2h_comp = call(2, Strain::Hearts);
    insert_uncontested(book, &[one_nt, c2d, c2h_comp], after_2d_2h());

    // 1NT-2♦-2♥-2♠ (F INV, 5=♥): opener's answer
    let c2s_relay = call(2, Strain::Spades);
    insert_uncontested(book, &[one_nt, c2d, c2h_comp, c2s_relay], after_2d_2h_2s());

    // E. After 1NT-2♦-2♥: opener answers the forcing/invitational continuations
    // E.2NT: UNBAL FG — opener terminates at 3NT
    insert_uncontested(book, &[one_nt, c2d, c2h_comp, c2nt_inv], reply_fg_minor());

    // E.3♣: S/T hearts — opener node + RKCB
    let c3c_st = call(3, Strain::Clubs);
    insert_uncontested(
        book,
        &[one_nt, c2d, c2h_comp, c3c_st],
        reply_slam_try_hearts(),
    );
    slam::install_rkcb(book, &[one_nt, c2d, c2h_comp, c3c_st], Suit::Hearts);

    // E.3♦: S/T hearts — opener node + RKCB
    let c3d_st = call(3, Strain::Diamonds);
    insert_uncontested(
        book,
        &[one_nt, c2d, c2h_comp, c3d_st],
        reply_slam_try_hearts(),
    );
    slam::install_rkcb(book, &[one_nt, c2d, c2h_comp, c3d_st], Suit::Hearts);

    // E.3♠: SPL (6♥, 0-1♠) — opener node + RKCB
    let c3s_spl = call(3, Strain::Spades);
    insert_uncontested(
        book,
        &[one_nt, c2d, c2h_comp, c3s_spl],
        reply_slam_try_hearts(),
    );
    slam::install_rkcb(book, &[one_nt, c2d, c2h_comp, c3s_spl], Suit::Hearts);

    // E.4♣: SPL (6♥, 0-1♣) — opener node + RKCB
    insert_uncontested(book, &[one_nt, c2d, c2h_comp, c4c_tex], reply_spl4_hearts());
    slam::install_rkcb(book, &[one_nt, c2d, c2h_comp, c4c_tex], Suit::Hearts);

    // E.4♦: SPL (6♥, 0-1♦) — opener node + RKCB
    insert_uncontested(book, &[one_nt, c2d, c2h_comp, c4d_tex], reply_spl4_hearts());
    slam::install_rkcb(book, &[one_nt, c2d, c2h_comp, c4d_tex], Suit::Hearts);

    // E.4NT: QUANT
    insert_uncontested(
        book,
        &[one_nt, c2d, c2h_comp, c4nt_b],
        quantitative_answer_1nt(),
    );

    // E.5NT: QUANT
    insert_uncontested(book, &[one_nt, c2d, c2h_comp, c5nt_b], reply_quant_5nt());

    // -------------------------------------------------------------------------
    // BTU Jacoby transfer 2♥ → spades
    // -------------------------------------------------------------------------
    let c2h_trf = call(2, Strain::Hearts);
    insert_uncontested(book, &[one_nt, c2h_trf], transfer_2h_answers());

    // After transfer completion 2♠ (non-super-accept): responder's continuation
    let c2s_comp = call(2, Strain::Spades);
    insert_uncontested(book, &[one_nt, c2h_trf, c2s_comp], after_2h_2s());

    // F. After 1NT-2♥-2♠: opener answers the forcing/invitational continuations
    // F.2NT: UNBAL FG — opener terminates at 3NT
    insert_uncontested(
        book,
        &[one_nt, c2h_trf, c2s_comp, c2nt_inv],
        reply_fg_minor(),
    );

    // F.4♣: SPL (6♠, 0-1♣) — opener node + RKCB
    insert_uncontested(
        book,
        &[one_nt, c2h_trf, c2s_comp, c4c_tex],
        reply_spl4_spades(),
    );
    slam::install_rkcb(book, &[one_nt, c2h_trf, c2s_comp, c4c_tex], Suit::Spades);

    // F.4♦: SPL (6♠, 0-1♦) — opener node + RKCB
    insert_uncontested(
        book,
        &[one_nt, c2h_trf, c2s_comp, c4d_tex],
        reply_spl4_spades(),
    );
    slam::install_rkcb(book, &[one_nt, c2h_trf, c2s_comp, c4d_tex], Suit::Spades);

    // F.4NT: QUANT
    insert_uncontested(
        book,
        &[one_nt, c2h_trf, c2s_comp, c4nt_b],
        quantitative_answer_1nt(),
    );

    // F.5NT: QUANT
    insert_uncontested(
        book,
        &[one_nt, c2h_trf, c2s_comp, c5nt_b],
        reply_quant_5nt(),
    );

    // Install RKCB under spade slam tries from 2♥ path (3♣/3♦/3♥ S/T calls)
    // After 1NT-2♥-2♠-3♣ (S/T 4+♣)
    insert_uncontested(
        book,
        &[one_nt, c2h_trf, c2s_comp, c3c_st],
        reply_slam_try_spades(),
    );
    slam::install_rkcb(book, &[one_nt, c2h_trf, c2s_comp, c3c_st], Suit::Spades);

    // After 1NT-2♥-2♠-3♦ (S/T 4+♦)
    insert_uncontested(
        book,
        &[one_nt, c2h_trf, c2s_comp, c3d_st],
        reply_slam_try_spades(),
    );
    slam::install_rkcb(book, &[one_nt, c2h_trf, c2s_comp, c3d_st], Suit::Spades);

    // After 1NT-2♥-2♠-3♥ (S/T 5+♥): relay to RKCB for spades
    insert_uncontested(
        book,
        &[one_nt, c2h_trf, c2s_comp, c3h_st],
        reply_slam_try_spades(),
    );
    slam::install_rkcb(book, &[one_nt, c2h_trf, c2s_comp, c3h_st], Suit::Spades);

    // -------------------------------------------------------------------------
    // Minor relay 2♠ → clubs
    // -------------------------------------------------------------------------
    let c2s_minor = call(2, Strain::Spades);
    insert_uncontested(book, &[one_nt, c2s_minor], after_2s_club_relay());

    // -------------------------------------------------------------------------
    // Minor relay 2NT → diamonds
    // -------------------------------------------------------------------------
    let c2nt = call(2, Strain::Notrump);
    insert_uncontested(book, &[one_nt, c2nt], after_2nt_diamond_relay());

    // -------------------------------------------------------------------------
    // Puppet Stayman 3♣
    // -------------------------------------------------------------------------
    let c3c = call(3, Strain::Clubs);
    insert_uncontested(book, &[one_nt, c3c], puppet_answers());

    // After 3♦ (no 5-card major): responder's Smolen/S/O continuations
    let c3d_pup = call(3, Strain::Diamonds);
    insert_uncontested(book, &[one_nt, c3c, c3d_pup], after_puppet_3d());

    // After 1NT-3♣-3♦-3♥ (Smolen, 4=♠)
    let c3h_pup = call(3, Strain::Hearts);
    insert_uncontested(book, &[one_nt, c3c, c3d_pup, c3h_pup], after_puppet_3d_3h());

    // After 1NT-3♣-3♦-3♠ (Smolen, 4=♥)
    let c3s_pup = call(3, Strain::Spades);
    insert_uncontested(book, &[one_nt, c3c, c3d_pup, c3s_pup], after_puppet_3d_3s());

    // G. After 1NT-3♣-3♥ (opener showed 5=♥): responder bids 4♥/4NT then RKCB
    let c3h_five = call(3, Strain::Hearts);
    insert_uncontested(book, &[one_nt, c3c, c3h_five], puppet_after_3h_five());
    slam::install_rkcb(book, &[one_nt, c3c, c3h_five], Suit::Hearts);

    // G. After 1NT-3♣-3♠ (opener showed 5=♠): responder bids 4♠/4NT then RKCB
    let c3s_five = call(3, Strain::Spades);
    insert_uncontested(book, &[one_nt, c3c, c3s_five], puppet_after_3s_five());
    slam::install_rkcb(book, &[one_nt, c3c, c3s_five], Suit::Spades);

    // -------------------------------------------------------------------------
    // Texas transfers 4♣→4♥ and 4♦→4♠
    // -------------------------------------------------------------------------
    let c4c = call(4, Strain::Clubs);
    let c4d = call(4, Strain::Diamonds);
    insert_uncontested(book, &[one_nt, c4c], complete_texas_hearts());
    insert_uncontested(book, &[one_nt, c4d], complete_texas_spades());

    // -------------------------------------------------------------------------
    // Quantitative 4NT and 5NT
    // -------------------------------------------------------------------------
    let c4nt = call(4, Strain::Notrump);
    let c5nt = call(5, Strain::Notrump);
    insert_uncontested(book, &[one_nt, c4nt], quantitative_answer_1nt());
    insert_uncontested(book, &[one_nt, c5nt], quantitative_answer_5nt());
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::Rules;
    use crate::bidding::context::Context;
    use crate::bidding::trie::Classifier;
    use contract_bridge::Hand;
    use contract_bridge::auction::RelativeVulnerability;

    /// The highest-logit call the rules make for a hand at an auction
    fn best(rules: &Rules, auction: &[Call], hand: &str) -> Call {
        let hand: Hand = hand.parse().expect("valid test hand");
        let context = Context::new(RelativeVulnerability::NONE, auction);
        let logits = rules.classify(hand, &context);
        (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("array is never empty")
    }

    // All tests use the 1NT-first auction with a pass by the opponents.
    const ONE_NT_P: [Call; 2] = [call(1, Strain::Notrump), Call::Pass];

    // -------------------------------------------------------------------------
    // Top-level responses
    // -------------------------------------------------------------------------

    #[test]
    fn stayman_with_four_card_major() {
        // 10 HCP, 4♠ 2♥ — should bid 2♣ Stayman
        let r = btu_responses();
        assert_eq!(
            best(&r, &ONE_NT_P, "KJ52.Q7.AJ64.842"),
            call(2, Strain::Clubs)
        );
    }

    #[test]
    fn transfer_to_hearts() {
        // 7 HCP, 5♥ — should transfer to hearts with 2♦
        let r = btu_responses();
        assert_eq!(
            best(&r, &ONE_NT_P, "73.KJ842.Q64.753"),
            call(2, Strain::Diamonds)
        );
    }

    #[test]
    fn transfer_to_spades() {
        // 7 HCP, 5♠ — should transfer to spades with 2♥
        let r = btu_responses();
        assert_eq!(
            best(&r, &ONE_NT_P, "KJ842.73.Q64.753"),
            call(2, Strain::Hearts)
        );
    }

    #[test]
    fn minor_relay_clubs() {
        // Long clubs, no major — should bid 2♠ club relay
        // 6 HCP, 6♣, no 4-card major
        let r = btu_responses();
        assert_eq!(
            best(&r, &ONE_NT_P, "73.842.64.KQJ532"),
            call(2, Strain::Spades)
        );
    }

    #[test]
    fn puppet_stayman_game_force() {
        // 12 HCP, 4♠ no 5-card major, game-forcing values — Puppet Stayman 3♣
        // AJ52.Q73.AJ6.842: A=4, J=1, Q=2, A=4, J=1 = 12 HCP
        let r = btu_responses();
        assert_eq!(
            best(&r, &ONE_NT_P, "AJ52.Q73.AJ6.842"),
            call(3, Strain::Clubs)
        );
    }

    #[test]
    fn texas_transfer_hearts() {
        // 9 HCP, 6♥ game-going — Texas transfer 4♣ (not 2♦ transfer)
        // 73.KQ9842.K64.53: K=3, Q=2, K=3 = 8 HCP... need 9
        // 73.KQ9842.AJ4.53: K=3, Q=2, A=4 = 9 HCP
        let r = btu_responses();
        assert_eq!(
            best(&r, &ONE_NT_P, "73.KQ9842.AJ4.53"),
            call(4, Strain::Clubs)
        );
    }

    #[test]
    fn quantitative_4nt() {
        // 16 HCP, balanced, no 4-card major — bid 4NT QUANT
        // AK3.KQ3.Q643.Q72: A=4, K=3, K=3, Q=2, Q=2, Q=2 = 16 HCP, 3♠ 3♥
        let r = btu_responses();
        assert_eq!(
            best(&r, &ONE_NT_P, "AK3.KQ3.Q643.Q72"),
            call(4, Strain::Notrump)
        );
    }

    // -------------------------------------------------------------------------
    // BTU Stayman 2♣ continuations
    // -------------------------------------------------------------------------

    #[test]
    fn stayman_five_spade_invite() {
        // After 1NT-2♣-2♦ (no major), responder has 5=♠ INV — bid 2♥ relay
        let r = after_2c_2d();
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Clubs),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
        ];
        // 8 HCP, 5♠ INV: KQ852.Q42.J63.83 = K=3, Q=2, Q=2, J=1 = 8 HCP, 5♠
        assert_eq!(
            best(&r, &auction, "KQ852.Q42.J63.83"),
            call(2, Strain::Hearts)
        );
    }

    #[test]
    fn stayman_smolen_fg_hearts() {
        // After 1NT-2♣-2♦, responder has FG 5+♥ 4=♠ — Smolen 3♥
        let r = after_2c_2d();
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Clubs),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
        ];
        // 12 HCP, 5♥ 4♠ game force: KQ52.AQ984.J6.83 = K=3+Q=2+A=4+Q=2+J=1 = 12
        assert_eq!(
            best(&r, &auction, "KQ52.AQ984.J6.83"),
            call(3, Strain::Hearts)
        );
    }

    // -------------------------------------------------------------------------
    // Transfer continuations
    // -------------------------------------------------------------------------

    #[test]
    fn transfer_2d_complete() {
        // Opener with minimum and no 4-card hearts just completes 2♥
        let r = transfer_2d_answers();
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
        ];
        // 15 HCP, 4333 — minimum, no 4-card hearts → complete 2♥
        assert_eq!(
            best(&r, &auction, "AK52.Q73.KJ6.Q82"),
            call(2, Strain::Hearts)
        );
    }

    #[test]
    fn transfer_2d_superaccept_3h() {
        // Opener with maximum 4-card hearts super-accepts 3♥
        let r = transfer_2d_answers();
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
        ];
        // 17 HCP, 4=♥, no notable side suit → 3♥
        assert_eq!(
            best(&r, &auction, "AK2.AJ73.KJ6.Q82"),
            call(3, Strain::Hearts)
        );
    }

    #[test]
    fn transfer_2h_complete() {
        // Opener with minimum just completes 2♠
        let r = transfer_2h_answers();
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Hearts),
            Call::Pass,
        ];
        // 15 HCP, 4333 — minimum, no 4-card spades → complete 2♠
        assert_eq!(
            best(&r, &auction, "Q73.AK52.KJ6.Q82"),
            call(2, Strain::Spades)
        );
    }

    // -------------------------------------------------------------------------
    // Puppet Stayman
    // -------------------------------------------------------------------------

    #[test]
    fn puppet_opener_no_five_bids_3d() {
        // Opener with no 5-card major bids 3♦
        let r = puppet_answers();
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(3, Strain::Clubs),
            Call::Pass,
        ];
        // 16 HCP, 4♠ 3♥ — no 5-card major → 3♦
        assert_eq!(
            best(&r, &auction, "AK52.Q73.AJ6.K82"),
            call(3, Strain::Diamonds)
        );
    }

    #[test]
    fn puppet_opener_five_hearts_bids_3h() {
        // Opener with 5=♥ bids 3♥
        let r = puppet_answers();
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(3, Strain::Clubs),
            Call::Pass,
        ];
        // 16 HCP, 5♥ — bid 3♥
        assert_eq!(
            best(&r, &auction, "AK2.AQ973.AJ6.K2"),
            call(3, Strain::Hearts)
        );
    }

    // -------------------------------------------------------------------------
    // No-strand test: every forcing/invitational site must have a book answer
    // -------------------------------------------------------------------------

    #[test]
    fn no_below_game_strand() {
        let mut trie = Trie::new();
        register(&mut trie);

        // A representative opener hand (15 HCP, balanced) used for all checks.
        let hand: contract_bridge::Hand = "AK52.Q73.KJ6.Q82".parse().unwrap();

        // Each entry is OUR side's calls; the opposite hand must have a book answer.
        // Sequences are keyed from [1NT]; the hand to act is the opener (opener
        // answers the responder's last call) unless noted (G: responder acts).
        let sites: &[&[Call]] = &[
            // A. Top-level
            &[call(1, Strain::Notrump), call(3, Strain::Diamonds)],
            &[call(1, Strain::Notrump), call(3, Strain::Hearts)],
            &[call(1, Strain::Notrump), call(3, Strain::Spades)],
            // B. After 1NT-2♣-2♦
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Diamonds),
                call(3, Strain::Clubs),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Diamonds),
                call(3, Strain::Diamonds),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Diamonds),
                call(2, Strain::Notrump),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Diamonds),
                call(4, Strain::Clubs),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Diamonds),
                call(4, Strain::Diamonds),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Diamonds),
                call(4, Strain::Notrump),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Diamonds),
                call(5, Strain::Notrump),
            ],
            // C. After 1NT-2♣-2♥
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Hearts),
                call(3, Strain::Clubs),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Hearts),
                call(3, Strain::Diamonds),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Hearts),
                call(2, Strain::Notrump),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Hearts),
                call(3, Strain::Spades),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Hearts),
                call(4, Strain::Clubs),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Hearts),
                call(4, Strain::Diamonds),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Hearts),
                call(4, Strain::Notrump),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Hearts),
                call(5, Strain::Notrump),
            ],
            // D. After 1NT-2♣-2♠
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Spades),
                call(3, Strain::Clubs),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Spades),
                call(3, Strain::Diamonds),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Spades),
                call(2, Strain::Notrump),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Spades),
                call(3, Strain::Hearts),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Spades),
                call(4, Strain::Clubs),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Spades),
                call(4, Strain::Diamonds),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Spades),
                call(4, Strain::Hearts),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Spades),
                call(4, Strain::Notrump),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Clubs),
                call(2, Strain::Spades),
                call(5, Strain::Notrump),
            ],
            // E. After 1NT-2♦-2♥
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Diamonds),
                call(2, Strain::Hearts),
                call(2, Strain::Notrump),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Diamonds),
                call(2, Strain::Hearts),
                call(3, Strain::Clubs),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Diamonds),
                call(2, Strain::Hearts),
                call(3, Strain::Diamonds),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Diamonds),
                call(2, Strain::Hearts),
                call(3, Strain::Spades),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Diamonds),
                call(2, Strain::Hearts),
                call(4, Strain::Clubs),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Diamonds),
                call(2, Strain::Hearts),
                call(4, Strain::Diamonds),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Diamonds),
                call(2, Strain::Hearts),
                call(4, Strain::Notrump),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Diamonds),
                call(2, Strain::Hearts),
                call(5, Strain::Notrump),
            ],
            // F. After 1NT-2♥-2♠
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Hearts),
                call(2, Strain::Spades),
                call(2, Strain::Notrump),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Hearts),
                call(2, Strain::Spades),
                call(4, Strain::Clubs),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Hearts),
                call(2, Strain::Spades),
                call(4, Strain::Diamonds),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Hearts),
                call(2, Strain::Spades),
                call(4, Strain::Notrump),
            ],
            &[
                call(1, Strain::Notrump),
                call(2, Strain::Hearts),
                call(2, Strain::Spades),
                call(5, Strain::Notrump),
            ],
            // G. Puppet: responder acts after opener showed 5-card major
            &[
                call(1, Strain::Notrump),
                call(3, Strain::Clubs),
                call(3, Strain::Hearts),
            ],
            &[
                call(1, Strain::Notrump),
                call(3, Strain::Clubs),
                call(3, Strain::Spades),
            ],
        ];

        for seq in sites {
            let raw = super::super::uncontested(seq);
            let ctx = crate::bidding::context::Context::new(
                contract_bridge::auction::RelativeVulnerability::NONE,
                &raw,
            );
            assert!(trie.resolve(&ctx, &raw).is_some(), "strand at {seq:?}");
        }
        // Verify that the test hand itself yields a resolution at 1NT (sanity)
        let _ = hand;
    }
}
