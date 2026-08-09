use super::super::tests::{P, best, best_with, bid};
use crate::bidding::agreements::Agreements;
use contract_bridge::Strain;

/// The opt-in GF-majors structure after the spade transfer: a 5-5 slam try
/// reroutes off the capped both-majors `3♦` onto a natural `3♥`, and the
/// single-suiter relocates from the old artificial `3♥` to a quantitative `4NT`.
#[test]
fn transfer_gf_majors_five_five_and_quantitative() {
    use crate::bidding::american::set_transfer_gf_majors;

    let one_nt = [bid(1, Strain::Notrump), P];
    let after = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(2, Strain::Spades),
        P,
    ];
    // 5♠5♥, ♠AKQ + ♥AK = 16 HCP, clean 5-5-2-1 → point_count 18 (slam).
    let slam_55 = "AKQ52.AK432.32.4";
    // 5♠5♥, ♠KQ + ♥KQ = 10 HCP → point_count 12 (minimum game force).
    let min_55 = "KQ542.KQ432.32.4";
    // Balanced single-suited 5♠, 18 HCP, ≤3 hearts (the old single-suited try).
    let single = "AKQ52.A32.K32.Q2";

    // --- Baseline (gate off): unchanged ------------------------------
    set_transfer_gf_majors(false);
    // The slam 5-5 shows both suits with the direct 3♦ jump.
    assert_eq!(best(&one_nt, slam_55), bid(3, Strain::Diamonds));
    // The single-suiter bids the artificial 3♥ slam try after transferring.
    assert_eq!(best(&after, single), bid(3, Strain::Hearts));

    // --- Gate on -----------------------------------------------------
    set_transfer_gf_majors(true);
    // The slam 5-5 is capped off 3♦ and transfers instead...
    assert_eq!(best(&one_nt, slam_55), bid(2, Strain::Hearts));
    // ...then rebids a natural 3♥ (5-5 slam try).
    assert_eq!(best(&after, slam_55), bid(3, Strain::Hearts));
    // The minimum 5-5 keeps the direct 3♦ — the cap still admits it.
    assert_eq!(best(&one_nt, min_55), bid(3, Strain::Diamonds));
    // The single-suiter relocates to a quantitative 4NT (no longer 3♥).
    assert_eq!(best(&after, single), bid(4, Strain::Notrump));

    // Opener's reply to the quantitative 4NT: a maximum accepts (6♠ on the
    // known eight-card fit), a minimum declines.
    let over_quant = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(2, Strain::Spades),
        P,
        bid(4, Strain::Notrump),
        P,
    ];
    assert_eq!(
        best(&over_quant, "AQ4.KQ3.KQJ2.Q32"),
        bid(6, Strain::Spades)
    );
    assert_eq!(best(&over_quant, "AQ4.K83.KJ72.Q83"), P);

    // Opener's reply to the 5-5 slam try (spade-agreed, like the single-suited
    // try): a maximum launches RKCB, a minimum signs off in 4♠.
    let over_55 = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(2, Strain::Spades),
        P,
        bid(3, Strain::Hearts),
        P,
    ];
    assert_eq!(best(&over_55, "AQ4.KQ3.KQJ2.Q32"), bid(4, Strain::Notrump));
    assert_eq!(best(&over_55, "AQ43.K83.KJ7.Q83"), bid(4, Strain::Spades));

    set_transfer_gf_majors(true); // restore the default
}

/// The GF-majors minor side-suits: `3♣`/`3♦` show five spades and a four-card
/// minor.  Arm A shows them on any game force; Arm B (`minor_min_to_3nt`)
/// reserves them for slam tries, the minimums resting in the floor's `3NT`.
#[test]
fn transfer_gf_majors_minor_side_suits() {
    use crate::bidding::american::set_transfer_gf_majors;

    let after = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(2, Strain::Spades),
        P,
    ];
    // 5♠4♣, ♠KJ + ♣AK = 11 HCP, 5-2-2-4 → point_count 12 (minimum game force).
    let min_club = "KJ542.32.32.AK32";
    // 5♠4♦, the diamond mirror.
    let min_diamond = "KJ542.32.AK32.32";
    // 5♠4♣, ♠AKQ + ♣AKQ = 18 HCP → point_count 19 (slam).
    let slam_club = "AKQ52.32.32.AKQ2";

    set_transfer_gf_majors(true);

    // --- Arm A (default): the minor shows on any game force ------------
    let arm_a = Agreements::current();
    assert_eq!(best_with(&arm_a, &after, min_club), bid(3, Strain::Clubs));
    assert_eq!(
        best_with(&arm_a, &after, min_diamond),
        bid(3, Strain::Diamonds)
    );

    // --- Arm B: minimums lump into the floor's 3NT, slam shows the minor
    let mut arm_b = arm_a;
    arm_b.notrump.minor_min_to_3nt = true;
    assert_eq!(best_with(&arm_b, &after, min_club), bid(3, Strain::Notrump));
    assert_eq!(best_with(&arm_b, &after, slam_club), bid(3, Strain::Clubs));

    // Opener's reply to the minor places game on the 5-3 spade fit: with support
    // 4♠ (the ruffing value beats an un-pulled 3NT), without support 3NT. No RKCB
    // — the minor is undifferentiated min-through-slam, so opener never blasts.
    let over_minor = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(2, Strain::Spades),
        P,
        bid(3, Strain::Clubs),
        P,
    ];
    assert_eq!(
        best_with(&arm_a, &over_minor, "AQ4.KQ3.KQJ2.Q32"),
        bid(4, Strain::Spades)
    );
    assert_eq!(
        best_with(&arm_a, &over_minor, "A4.KQ32.KQJ2.Q32"),
        bid(3, Strain::Notrump)
    );

    set_transfer_gf_majors(true); // restore the default
}

/// Choice of games: a balanced exactly-five-spade game force offers `3NT` (the
/// transfer pinned the five spades).  The 5-4, 5-5 and six-card hands take their
/// own slots, so a bare `3NT` reads as *balanced* — the inference opener's
/// ruff-gated correction relies on.
#[test]
fn transfer_gf_majors_choice_of_games_3nt() {
    use crate::bidding::american::set_transfer_gf_majors;

    let after = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(2, Strain::Spades),
        P,
    ];
    set_transfer_gf_majors(true);
    // 5-3-3-2, 12 HCP, no four-card minor, no second five-card suit → 3NT.
    assert_eq!(best(&after, "AQ654.K72.Q83.J4"), bid(3, Strain::Notrump));
    // A six-card suit is not balanced — it keeps its natural spade route.
    assert_ne!(best(&after, "AQ6543.K72.Q8.J4"), bid(3, Strain::Notrump));
    // A four-card minor shows the minor (3♣), not the balanced 3NT.
    assert_eq!(best(&after, "AQ654.K7.Q8.J432"), bid(3, Strain::Clubs));
}

/// The GF-majors spade splinters: a 6+♠ slam hand with a side-suit splinter is
/// carved off the direct Texas `4♦`, transfers, and splinters at the four level.
/// A singleton ace or king is a working honor, not a splinter.
#[test]
fn transfer_gf_majors_spade_splinters() {
    use crate::bidding::american::set_transfer_gf_majors;

    let one_nt = [bid(1, Strain::Notrump), P];
    let after = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(2, Strain::Spades),
        P,
    ];
    // 6♠, ♠AKQ + ♥AK + ♦Q = 18 HCP, 6-3-3-1 with a low singleton club (splinter).
    let splinter = "AKQ432.AK2.Q43.2";
    // The same shape but a singleton ♣A — a working honor, not a splinter.
    let stiff_ace = "AKQ432.AK2.Q43.A";

    // --- Baseline (gate off): the 16+ six-spader Texas-transfers (4♦) ---
    set_transfer_gf_majors(false);
    assert_eq!(best(&one_nt, splinter), bid(4, Strain::Diamonds));

    // --- Gate on: carved off Texas, it transfers and splinters ---------
    set_transfer_gf_majors(true);
    assert_eq!(best(&one_nt, splinter), bid(2, Strain::Hearts));
    assert_eq!(best(&after, splinter), bid(4, Strain::Clubs));
    // The stiff ace is no splinter — it keeps the Texas route even on the gate.
    assert_eq!(best(&one_nt, stiff_ace), bid(4, Strain::Diamonds));

    // Opener's reply to the splinter: a maximum RKCBs spades, a minimum signs
    // off in 4♠.
    let over_splinter = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(2, Strain::Spades),
        P,
        bid(4, Strain::Clubs),
        P,
    ];
    assert_eq!(
        best(&over_splinter, "AQ3.KJ32.KQ32.Q3"),
        bid(4, Strain::Notrump)
    );
    assert_eq!(
        best(&over_splinter, "KQ3.KJ32.KJ32.Q3"),
        bid(4, Strain::Spades)
    );

    set_transfer_gf_majors(true); // restore the default
}

/// The heart mirror (`set_transfer_gf_hearts`): a five-heart-plus-minor game force
/// shows the minor (`3♣`/`3♦`), and a single-suited 16+ hand invites slam
/// quantitatively (`4NT`); opener places game on the 5-3 heart fit or accepts slam.
#[test]
fn transfer_gf_hearts_minors_and_quant() {
    use crate::bidding::american::{set_transfer_gf_hearts, set_transfer_gf_majors};

    let after = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Diamonds),
        P,
        bid(2, Strain::Hearts),
        P,
    ];
    // 5♥4♣, ♥KJ + ♣AK = 11 HCP → point_count 12 (minimum game force).
    let min_club = "32.KJ542.32.AK32";
    // 5♥4♦, the diamond mirror.
    let min_diamond = "32.KJ542.AK32.32";
    // 5♥, 16 HCP, no four-card side suit — the single-suited quantitative raise.
    let quant = "Q32.AKJ42.KJ2.Q2";

    set_transfer_gf_majors(true);
    set_transfer_gf_hearts(true);
    assert_eq!(best(&after, min_club), bid(3, Strain::Clubs));
    assert_eq!(best(&after, min_diamond), bid(3, Strain::Diamonds));
    assert_eq!(best(&after, quant), bid(4, Strain::Notrump));

    // Opener over the minor (`…3♣`): place game on the 5-3 heart fit — 4♥ with
    // three-card support (its ruffing value beats an un-pulled 3NT), else 3NT.
    let over_minor = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Diamonds),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(3, Strain::Clubs),
        P,
    ];
    assert_eq!(
        best(&over_minor, "AQ4.KQ3.KQJ2.Q32"),
        bid(4, Strain::Hearts)
    );
    assert_eq!(
        best(&over_minor, "AQ32.K3.KQJ2.Q32"),
        bid(3, Strain::Notrump)
    );

    // Opener over the quantitative 4NT: 6♥ with a maximum and support, else pass.
    let over_quant = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Diamonds),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(4, Strain::Notrump),
        P,
    ];
    assert_eq!(
        best(&over_quant, "AQ42.KQ3.KJ5.Q32"),
        bid(6, Strain::Hearts)
    );
    assert_eq!(best(&over_quant, "KJ42.KQ3.KJ5.Q32"), P);

    set_transfer_gf_hearts(true); // restore the default
}

/// The heart mirror's cheap spade splinter: a six-heart slam hand short in spades
/// splinters at `3♠` (below `4♥`), freed by evicting the single-suited slam try;
/// a minor shortness splinters at `4♣`/`4♦`.  A singleton ace is no splinter.
#[test]
fn transfer_gf_hearts_spade_splinter() {
    use crate::bidding::american::{set_transfer_gf_hearts, set_transfer_gf_majors};

    let one_nt = [bid(1, Strain::Notrump), P];
    let after = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Diamonds),
        P,
        bid(2, Strain::Hearts),
        P,
    ];
    // 6♥, ♥AKQ + ♦Q + ♣AK = 18 HCP, 1-6-3-3 with a low singleton spade (splinter).
    let spade_short = "2.AKQ432.Q43.AK2";
    // 6♥ short a diamond instead → the 4♦ splinter (3-6-1-3).
    let diamond_short = "AK2.AKQ432.2.Q43";
    // The same six-heart slam but a singleton ♠A — a working honor, not a splinter.
    let stiff_ace = "A.AKQ432.Q43.Q42";

    set_transfer_gf_majors(true);
    set_transfer_gf_hearts(true);
    // Carved off the direct Texas `4♣`, it transfers (2♦) and splinters at 3♠.
    assert_eq!(best(&one_nt, spade_short), bid(2, Strain::Diamonds));
    assert_eq!(best(&after, spade_short), bid(3, Strain::Spades));
    assert_eq!(best(&after, diamond_short), bid(4, Strain::Diamonds));
    // The stiff ace is no splinter — it keeps the direct Texas route (`4♣`).
    assert_eq!(best(&one_nt, stiff_ace), bid(4, Strain::Clubs));

    // Opener's reply to the 3♠ splinter (agreeing hearts): a maximum RKCBs (4NT),
    // a minimum signs off in 4♥.
    let over_splinter = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Diamonds),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(3, Strain::Spades),
        P,
    ];
    assert_eq!(
        best(&over_splinter, "KJ32.AQ3.KQ32.Q3"),
        bid(4, Strain::Notrump)
    );
    assert_eq!(
        best(&over_splinter, "KJ32.KQ3.KJ32.Q3"),
        bid(4, Strain::Hearts)
    );

    set_transfer_gf_hearts(true); // restore the default
}
