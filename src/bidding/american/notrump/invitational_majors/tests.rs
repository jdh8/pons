use super::super::tests::{P, best, bid};
use contract_bridge::Strain;
use contract_bridge::auction::Call;

/// The gated invitational 5-4-majors structure, end to end: 5♠4♥ Staymans and
/// rebids 2♠; 5♥4♠ transfers and rebids 2NT (with spades) or 2♠ (without).
#[test]
fn invitational_five_four_majors() {
    use crate::bidding::american::set_invitational_5card_majors;

    let one_nt = [bid(1, Strain::Notrump), P];
    // 5♠4♥, a bare 8 (♠KQ + ♥Q + ♦J).
    let s5h4 = "KQ864.Q1043.J2.32";
    // 6♠4♥, a bare 8 — a six-card major, so it blasts game via Texas (4♦), not
    // caught by the 5-4 Stayman reroute (which is scoped to five-card majors).
    let s6h4 = "KQ8642.QJ43.32.2";
    // 5♥4♠, a bare 8.
    let h5s4 = "Q1043.KQ864.J2.32";
    // 5 hearts, no four-card spade suit, a bare 8 (the single-suited invite).
    let h5 = "Q3.KQ864.J32.432";

    set_invitational_5card_majors(true);

    // Routing: 5♠4♥/8 now Staymans; 6♠4♥/8 blasts game via Texas (4♦, a six-card
    // major); 5♥4♠/8 still takes the heart transfer (2♦).
    assert_eq!(best(&one_nt, s5h4), bid(2, Strain::Clubs));
    assert_eq!(best(&one_nt, s6h4), bid(4, Strain::Diamonds));
    assert_eq!(best(&one_nt, h5s4), bid(2, Strain::Diamonds));

    // A: 1NT - 2♣ - 2♦ - 2♠, non-forcing (opener denied a major).
    let stayman_no_major = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Diamonds),
        P,
    ];
    assert_eq!(best(&stayman_no_major, s5h4), bid(2, Strain::Spades));

    // B: 1NT - 2♣ - 2♥ - 2♠, forcing (opener showed hearts); opener with a maximum and
    // three spades accepts in 4♠.
    let stayman_hearts = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Hearts),
        P,
    ];
    assert_eq!(best(&stayman_hearts, s5h4), bid(2, Strain::Spades));
    let over_two_s = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(2, Strain::Spades),
        P,
    ];
    assert_eq!(
        best(&over_two_s, "AK4.KQ32.A65.J32"),
        bid(4, Strain::Spades)
    );

    // C/D: after the heart transfer completes, 5♥4♠ rebids 2NT; single-suited
    // five hearts rebids the artificial 2♠.
    let after_transfer = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Diamonds),
        P,
        bid(2, Strain::Hearts),
        P,
    ];
    assert_eq!(best(&after_transfer, h5s4), bid(2, Strain::Notrump));
    assert_eq!(best(&after_transfer, h5), bid(2, Strain::Spades));

    // D opener: a maximum with three hearts accepts the 5♥4♠ invite in 4♥.
    let over_two_nt = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Diamonds),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(2, Strain::Notrump),
        P,
    ];
    assert_eq!(
        best(&over_two_nt, "AK2.A104.KQ32.J2"),
        bid(4, Strain::Hearts)
    );

    // Doubled-2♦ escape: when an opponent doubles opener's artificial 2♦, the
    // 5♠4♥ runs to its real 2♠ (systems on) instead of passing it out doubled.
    let two_d_doubled = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Diamonds),
        Call::Double,
    ];
    assert_eq!(best(&two_d_doubled, s5h4), bid(2, Strain::Spades));

    // With the structure off, the same 5♠4♥/8 takes the spade transfer instead.
    set_invitational_5card_majors(false);
    assert_eq!(best(&one_nt, s5h4), bid(2, Strain::Hearts));
    // The doubled-2♦ escape is general (competition-over-Stayman, not the flag):
    // a 4-4 invite runs to 2NT rather than passing the artificial 2♦ doubled.
    assert_eq!(
        best(&two_d_doubled, "KQ32.Q943.J32.43"),
        bid(2, Strain::Notrump)
    );
    set_invitational_5card_majors(true); // restore the default
}

/// The single-suited 5-spade invite: `1NT - 2♥ - 2♠ - 2NT` (the spade mirror of the
/// heart `2♠` relay — `2NT` is free here since 5♠4♥ Staymans), with opener's
/// strength-and-fit placement (4♠ / 3NT / 3♠ / pass-2NT).
#[test]
fn single_suited_spade_invite() {
    // 5 spades, no four-card heart, a bare 8 (♠KQ + ♥Q + ♦J): single-suited invite.
    let s5 = "KQ864.Q3.J32.432";
    let one_nt = [bid(1, Strain::Notrump), P];

    // Transfers to spades (2♥), then rebids the 2NT invite over 2♠.
    assert_eq!(best(&one_nt, s5), bid(2, Strain::Hearts));
    let after_transfer = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(2, Strain::Spades),
        P,
    ];
    assert_eq!(best(&after_transfer, s5), bid(2, Strain::Notrump));
    // A weak five-spade hand transfers and passes — it never invites with 2NT.
    assert_ne!(
        best(&after_transfer, "Q9864.32.J32.432"),
        bid(2, Strain::Notrump)
    );

    // Opener over 1NT - 2♥ - 2♠ - 2NT, by strength and spade support:
    let over_invite = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(2, Strain::Spades),
        P,
        bid(2, Strain::Notrump),
        P,
    ];
    // max (17) + three spades → 4♠; max + doubleton → 3NT.
    assert_eq!(
        best(&over_invite, "AK3.K32.KQ32.Q32"),
        bid(4, Strain::Spades)
    );
    assert_eq!(
        best(&over_invite, "KQ.AK42.KQ32.432"),
        bid(3, Strain::Notrump)
    );
    // min (16) + three spades → 3♠; min + doubleton → pass (rest in 2NT).
    assert_eq!(
        best(&over_invite, "AK3.Q32.KQ32.Q32"),
        bid(3, Strain::Spades)
    );
    assert_eq!(best(&over_invite, "KQ.Q432.KQ32.A32"), P);
}
