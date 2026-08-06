use crate::american;
use crate::bidding::System;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Strain};

const P: Call = Call::Pass;

fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// The highest-logit call `american()` assigns the hand at the auction
fn best(auction: &[Call], hand: &str) -> Call {
    let hand = hand.parse().expect("valid test hand");
    let logits = american()
        .against()
        .classify(hand, RelativeVulnerability::NONE, auction)
        .expect("a decision");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("the logits array is never empty")
}

/// The longer-major transfer discipline (default on): a two-suiter
/// transfers to its longer major, and equal lengths split by strength —
/// weak to hearts, invitational/minimum-game-force to the both-majors 3♦,
/// slam tries to spades for the `1NT–2♥–2♠–3♥` structure.
#[test]
fn transfers_prefer_the_longer_major() {
    let one_nt = [bid(1, Strain::Notrump), P];

    // 6♠5♥ transfers to spades whatever the strength (the legacy guards
    // tied on the weak hand, and 3♦ grabbed the strong one, losing the
    // sixth spade).
    assert_eq!(best(&one_nt, "QJ9642.98763.4.3"), bid(2, Strain::Hearts));
    assert_eq!(best(&one_nt, "KJ9642.AKJ63.J.3"), bid(2, Strain::Hearts));
    // 6♥5♠ transfers to hearts.
    assert_eq!(best(&one_nt, "98763.QJ9642.4.3"), bid(2, Strain::Diamonds));

    // Equal 5-5: weak prefers hearts for safety...
    assert_eq!(best(&one_nt, "J9863.J9642.4.3"), bid(2, Strain::Diamonds));
    // ...invitational / minimum game force shows both at once via 3♦...
    assert_eq!(best(&one_nt, "KJ863.KJ642.4.3"), bid(3, Strain::Diamonds));
    // ...and a slam try transfers to spades, then bids the natural
    // game-forcing 3♥ — the 5-5 slam-try structure.
    assert_eq!(best(&one_nt, "AKJ63.AKJ42.4.3"), bid(2, Strain::Hearts));
    let over_completion = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(2, Strain::Spades),
        P,
    ];
    assert_eq!(
        best(&over_completion, "AKJ63.AKJ42.4.3"),
        bid(3, Strain::Hearts)
    );

    // The 2NT-strength table follows the same discipline: longer major,
    // hearts on every tie (no both-majors bid at this level).
    let two_nt = [bid(2, Strain::Notrump), P];
    assert_eq!(best(&two_nt, "QJ9642.98763.4.3"), bid(3, Strain::Hearts));
    assert_eq!(best(&two_nt, "J9863.J9642.4.3"), bid(3, Strain::Diamonds));
}

/// The revised South African Texas with the slam-drive reroute (default on):
/// a 16+ six-card major Texas-transfers (4♣/4♦) and drives its own RKCB, while
/// the bare-15 cusp keeps the opener-decides direct 4♥; end to end through
/// `american()`.
#[test]
fn south_african_texas_slam_try() {
    let one_nt = [bid(1, Strain::Notrump), P];

    // Responder, 6 hearts: a 16-count (slam) and a 10-count (game) both take the
    // 4♣ Texas transfer; only the bare-15 invitational cusp keeps the direct 4♥.
    assert_eq!(best(&one_nt, "42.AKJ872.KQ4.K2"), bid(4, Strain::Clubs));
    assert_eq!(best(&one_nt, "42.AKJ872.Q43.32"), bid(4, Strain::Clubs));
    assert_eq!(best(&one_nt, "42.AKJ872.KQ4.Q2"), bid(4, Strain::Hearts));

    // Opener over the bare-15 direct invite (1NT–P–4♥–P): a maximum (17) launches
    // RKCB, a minimum (15) signs off by passing the major game.
    let over_try = [bid(1, Strain::Notrump), P, bid(4, Strain::Hearts), P];
    assert_eq!(best(&over_try, "KQ3.K53.AQ54.K92"), bid(4, Strain::Notrump));
    assert_eq!(best(&over_try, "KQ3.K53.KQ54.Q92"), P);

    // Opener completes the 4♣ transfer (1NT–P–4♣–P) → 4♥.
    let over_transfer = [bid(1, Strain::Notrump), P, bid(4, Strain::Clubs), P];
    assert_eq!(
        best(&over_transfer, "KQ3.K53.KQ54.Q92"),
        bid(4, Strain::Hearts)
    );

    // Responder's drive over the completion (1NT–P–4♣–P–4♥–P): the 16-count
    // keycards (4NT), the 10-count passes the game.
    let over_completion = [
        bid(1, Strain::Notrump),
        P,
        bid(4, Strain::Clubs),
        P,
        bid(4, Strain::Hearts),
        P,
    ];
    assert_eq!(
        best(&over_completion, "42.AKJ872.KQ4.K2"),
        bid(4, Strain::Notrump)
    );
    assert_eq!(best(&over_completion, "42.AKJ872.Q43.32"), P);

    // RKCB is wired on the drive: opener answers keycards over responder's 4NT
    // (♥K + ♦A = 2 keycards, no ♥Q → 5♥), proving the ladder is rooted here.
    let over_ask = [
        bid(1, Strain::Notrump),
        P,
        bid(4, Strain::Clubs),
        P,
        bid(4, Strain::Hearts),
        P,
        bid(4, Strain::Notrump),
        P,
    ];
    assert_eq!(best(&over_ask, "KQ3.K53.AQ54.K92"), bid(5, Strain::Hearts));
}

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
/// minor.  Arm A shows them on any game force; Arm B (`set_minor_min_to_3nt`)
/// reserves them for slam tries, the minimums resting in the floor's `3NT`.
#[test]
fn transfer_gf_majors_minor_side_suits() {
    use crate::bidding::american::{set_minor_min_to_3nt, set_transfer_gf_majors};

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
    set_minor_min_to_3nt(false);
    assert_eq!(best(&after, min_club), bid(3, Strain::Clubs));
    assert_eq!(best(&after, min_diamond), bid(3, Strain::Diamonds));

    // --- Arm B: minimums lump into the floor's 3NT, slam shows the minor
    set_minor_min_to_3nt(true);
    assert_eq!(best(&after, min_club), bid(3, Strain::Notrump));
    assert_eq!(best(&after, slam_club), bid(3, Strain::Clubs));
    set_minor_min_to_3nt(false);

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
        best(&over_minor, "AQ4.KQ3.KQJ2.Q32"),
        bid(4, Strain::Spades)
    );
    assert_eq!(
        best(&over_minor, "A4.KQ32.KQJ2.Q32"),
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

/// The opt-in six-card-major game invite: just below the Texas blast floor,
/// responder transfers and jumps to `3M`; opener accepts game or passes `3M`
/// on `point_count + trump length`.
#[test]
fn sixcard_major_invite() {
    use crate::bidding::american::set_sixcard_invite_floor;
    use crate::bidding::constraint::set_support_points;

    // This exercises the invite *mechanism* (transfer → 3M invite → accept
    // ladder), whose hands are calibrated to legacy `point_count` arithmetic
    // in the comments below.  The shipped `support_points` scale reads these
    // shaped six-card hands ~1 hotter, tipping some across the blast/accept
    // boundaries — that shift is measured by the A/B and `test_support_points`,
    // so pin the legacy scale here to test the ladder in isolation.
    set_support_points(false);

    let one_nt = [bid(1, Strain::Notrump), P];
    // 6 hearts, ♥KQ + ♠J = 6 HCP, 6-3-2-2: point_count 7 (+1 unbalanced),
    // point_count + length = 13 — one below the blast floor (14), so it invites.
    let inv = "J43.KQ8765.32.32";
    // 6 hearts, ♥KQ only = 5 HCP, point_count 6, sum 12 — too weak to invite.
    let weak = "543.KQ8765.32.32";

    // Turned off (floor 14 == blast floor): the invite hand transfers and the
    // floor handles the rebid — no authored 3♥ invite.
    set_sixcard_invite_floor(14);
    let after_transfer = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Diamonds),
        P,
        bid(2, Strain::Hearts),
        P,
    ];
    assert_ne!(best(&after_transfer, inv), bid(3, Strain::Hearts));

    // On by default (floor 13): the invite hand transfers (2♦) then jumps to 3♥;
    // the weak hand stays out of the invite.
    set_sixcard_invite_floor(13);
    assert_eq!(best(&one_nt, inv), bid(2, Strain::Diamonds));
    assert_eq!(best(&after_transfer, inv), bid(3, Strain::Hearts));
    assert_ne!(best(&after_transfer, weak), bid(3, Strain::Hearts));

    // Opener over 1NT–2♦–2♥–3♥: accept (4♥) on point_count + trump length ≥ 18,
    // else pass.  16 with a doubleton (16+2) accepts; a flat 15 with a doubleton
    // (15+2 = 17) passes; a 15 with three-card support (15+3) accepts.
    let over_invite = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Diamonds),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(3, Strain::Hearts),
        P,
    ];
    assert_eq!(
        best(&over_invite, "AK5.32.AQ74.K963"),
        bid(4, Strain::Hearts)
    ); // 16, ♥xx
    assert_eq!(best(&over_invite, "AK5.32.AQ74.Q963"), P); // 15, ♥xx
    assert_eq!(
        best(&over_invite, "AK52.432.AQ74.Q9"),
        bid(4, Strain::Hearts)
    ); // 15, ♥xxx (4-3-4-2 — a flat 4333 would read 14 and rightly pass)

    // Spade side: 6 spades, ♠KQ + ♥J = 6 HCP transfers (2♥) then jumps to 3♠.
    let spade_inv = "KQ8765.J43.32.32";
    assert_eq!(best(&one_nt, spade_inv), bid(2, Strain::Hearts));
    let after_spade = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(2, Strain::Spades),
        P,
    ];
    assert_eq!(best(&after_spade, spade_inv), bid(3, Strain::Spades));

    set_sixcard_invite_floor(13); // restore the default (on)
    set_support_points(true); // restore the shipped default
}

/// Over a natural (2♣) overcall of our 1NT we play *systems on*, not
/// Lebensohl: 2♣ steals no room, so responder keeps the uncontested Jacoby
/// transfers, shows the stolen 2♣ Stayman with a Double, and opener answers in
/// the uncontested tree (the systems-on rebase in `competition.rs`). There is
/// no natural 2♦ escape — 2♦ is a transfer.
#[test]
fn systems_on_over_two_clubs() {
    use contract_bridge::auction::Auction;
    // The highest-logit *legal* call (what the real bidder picks; the bare
    // `best` helper ignores legality, so it can't drop the now-illegal 2♣).
    let best_legal = |auction: &[Call], hand: &str| -> Call {
        let hand = hand.parse().expect("valid test hand");
        let logits = american()
            .against()
            .classify(hand, RelativeVulnerability::NONE, auction)
            .expect("a decision");
        let mut played = Auction::new();
        for &c in auction {
            played.push(c);
        }
        let mut scored: Vec<_> = (&logits.0)
            .into_iter()
            .filter(|(_, l)| l.is_finite())
            .collect();
        scored.sort_by(|x, y| y.1.partial_cmp(x.1).expect("no NaN"));
        scored
            .into_iter()
            .map(|(c, _)| c)
            .find(|&c| played.can_push(c).is_ok())
            .unwrap_or(Call::Pass)
    };

    let over_2c = [bid(1, Strain::Notrump), bid(2, Strain::Clubs)];
    // 5 hearts → 2♦ transfer; 5 spades → 2♥ transfer (systems on, not natural).
    assert_eq!(
        best_legal(&over_2c, "2.KJ876.5432.432"),
        bid(2, Strain::Diamonds)
    );
    assert_eq!(
        best_legal(&over_2c, "KJ876.2.5432.432"),
        bid(2, Strain::Hearts)
    );
    // 4-4 majors, invitational: the stolen 2♣ Stayman is shown by Double.
    assert_eq!(best_legal(&over_2c, "KJ32.KQ43.432.43"), Call::Double);

    // Opener completes the transfer: 1NT–(2♣)–2♦–(P) → 2♥, via the rebase.
    let over_xfer = [
        bid(1, Strain::Notrump),
        bid(2, Strain::Clubs),
        bid(2, Strain::Diamonds),
        P,
    ];
    assert_eq!(
        best_legal(&over_xfer, "KQ3.A53.KQ54.K92"),
        bid(2, Strain::Hearts)
    );

    // Opener answers the stolen Stayman: 1NT–(2♣)–X–(P) → 2♥ with four hearts.
    let over_dbl = [
        bid(1, Strain::Notrump),
        bid(2, Strain::Clubs),
        Call::Double,
        P,
    ];
    assert_eq!(
        best_legal(&over_dbl, "AQ3.KJ54.KQ4.92"),
        bid(2, Strain::Hearts)
    );
}

/// Opener converts the stolen-Stayman Double to penalty with good clubs, and
/// *only* in the contested context — uncontested forcing Stayman never passes.
#[test]
fn penalty_pass_over_two_clubs() {
    use crate::bidding::american::set_penalty_pass;

    // 16 HCP, 5332 with AK-fifth of clubs (5 clubs, 7 club HCP), no 4-card major.
    let opener = "A2.K3.Q42.AK432";
    let over_dbl = [
        bid(1, Strain::Notrump),
        bid(2, Strain::Clubs),
        Call::Double,
        P,
    ];
    let uncontested_stayman = [bid(1, Strain::Notrump), P, bid(2, Strain::Clubs), P];

    // With the penalty pass enabled, opener sits to defend 2♣ doubled.
    set_penalty_pass(Some((4, 4, true)));
    assert_eq!(best(&over_dbl, opener), Call::Pass);
    // Context-specific: the same hand still answers forcing Stayman (2♦) in the
    // *uncontested* auction — the conversion must not leak onto that shared node.
    assert_eq!(best(&uncontested_stayman, opener), bid(2, Strain::Diamonds));

    // With it off (the default), opener can never convert: answers Stayman 2♦.
    set_penalty_pass(None);
    assert_eq!(best(&over_dbl, opener), bid(2, Strain::Diamonds));
}

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

    // A: 1NT–2♣–2♦–2♠, non-forcing (opener denied a major).
    let stayman_no_major = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Diamonds),
        P,
    ];
    assert_eq!(best(&stayman_no_major, s5h4), bid(2, Strain::Spades));

    // B: 1NT–2♣–2♥–2♠, forcing (opener showed hearts); opener with a maximum and
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

/// The single-suited 5-spade invite: `1NT–2♥–2♠–2NT` (the spade mirror of the
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

    // Opener over 1NT–2♥–2♠–2NT, by strength and spade support:
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

/// Crawling Stayman: 4-4 majors *short in diamonds* (4414/4405) Stayman and,
/// over opener's 2♦ denial, crawl to 2♥ — opener passes (heart fit), corrects
/// to 2♠ (spade fit), or flees to 3♣ (no major fit, a 5-card-minor 1NT).
#[test]
fn crawling_stayman_escape() {
    use crate::bidding::american::set_crawling_stayman;

    let one_nt = [bid(1, Strain::Notrump), P];
    // 4414, a weak 5-count (♠QJ + ♥Q): garbage cannot escape it (one diamond).
    let h4414 = "QJ32.Q1043.4.T543";
    // 4405, a weak 5-count, void diamonds.
    let h4405 = "QJ32.Q1043..T9432";

    set_crawling_stayman(true);

    // Both short-diamond 4-4 hands bid 2♣ (crawling), unlike garbage Stayman.
    assert_eq!(best(&one_nt, h4414), bid(2, Strain::Clubs));
    assert_eq!(best(&one_nt, h4405), bid(2, Strain::Clubs));

    // Over opener's 2♦ denial, crawl to 2♥ (both majors, pass-or-correct).
    let two_d = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Diamonds),
        P,
    ];
    assert_eq!(best(&two_d, h4414), bid(2, Strain::Hearts));
    assert_eq!(best(&two_d, h4405), bid(2, Strain::Hearts));

    // Opener's reply to the crawl (1NT–2♣–2♦–2♥): three hearts pass the 4-3
    // fit; two hearts/three spades correct to 2♠; short in both majors with a
    // five-card minor flee to 3♣ (an 8-9 card club fit — responder is short
    // diamonds, hence long clubs).
    let crawl = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Diamonds),
        P,
        bid(2, Strain::Hearts),
        P,
    ];
    assert_eq!(best(&crawl, "A32.K43.KQ32.A52"), P); // 3-3 majors → pass 2♥
    assert_eq!(best(&crawl, "K43.A2.KQ32.A432"), bid(2, Strain::Spades)); // 3-2 → 2♠
    assert_eq!(best(&crawl, "K2.A2.KJ43.AJ432"), bid(3, Strain::Clubs)); // 2-2-4-5 → 3♣

    // Doubled tail (1NT–2♣–2♦–(X)–2♥) is systems-on via the competition rebase:
    // responder still crawls to 2♥, and opener still corrects (2♠ shown here).
    let two_d_doubled = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Diamonds),
        Call::Double,
    ];
    assert_eq!(best(&two_d_doubled, h4414), bid(2, Strain::Hearts));
    let crawl_doubled = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Diamonds),
        Call::Double,
        bid(2, Strain::Hearts),
        P,
    ];
    assert_eq!(
        best(&crawl_doubled, "K43.A2.KQ32.A432"),
        bid(2, Strain::Spades)
    );

    // With crawling off, the weak short-diamond 4-4 has no escape and passes.
    set_crawling_stayman(false);
    assert_eq!(best(&one_nt, h4414), P);
    set_crawling_stayman(true); // restore the default
}

#[test]
fn stayman_minor_slam_try() {
    use crate::bidding::american::set_stayman_minor_slam_try;
    set_stayman_minor_slam_try(true);

    // Responder: 4♠ 5♣, ≤3 hearts, 14 HCP — a slam-oriented two-suiter that
    // Staymaned, found no heart fit, and shows its longer minor.
    let after_2h = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Hearts),
        P,
    ];
    assert_eq!(best(&after_2h, "AJ54.32.32.AKQ32"), bid(3, Strain::Clubs));

    // Opener over the 3♣ slam try.
    let after_3c = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(3, Strain::Clubs),
        P,
    ];
    // Fit (4♣) + maximum (16): cooperate by raising the minor.
    assert_eq!(best(&after_3c, "A2.AQJ2.K32.Q543"), bid(4, Strain::Clubs));
    // No club fit (3♣): sign off in 3NT even with a maximum.
    assert_eq!(best(&after_3c, "A2.AQJ2.K432.Q54"), bid(3, Strain::Notrump));

    // Responder keycards over opener's minor raise (1430 RKCB).
    let after_4c = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(3, Strain::Clubs),
        P,
        bid(4, Strain::Clubs),
        P,
    ];
    assert_eq!(best(&after_4c, "AJ54.32.32.AKQ32"), bid(4, Strain::Notrump));

    // Off the gate the sequence is unauthored — responder does not bid 3♣.
    set_stayman_minor_slam_try(false);
    assert_ne!(best(&after_2h, "AJ54.32.32.AKQ32"), bid(3, Strain::Clubs));
    set_stayman_minor_slam_try(true);
}

#[test]
fn both_majors_relay_game_placement() {
    // 1NT–2♣–2NT (max, both majors) –3♣ (responder names hearts) –3♥: responder
    // places game on `point_count + extra trumps + a fit in the other major`.
    let relay = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Notrump),
        P,
        bid(3, Strain::Clubs),
        P,
        bid(3, Strain::Hearts),
        P,
    ];

    // Double 4-4 fit: a flat 7 reaches game (7 + 0 + 1 = 8) — the second major
    // fit is knowable because opener showed both majors.
    assert_eq!(best(&relay, "KQ54.J932.654.J2"), bid(4, Strain::Hearts));
    // Single 8-card fit, 8 HCP: the pre-accepted invite bids game (8 + 0 + 0).
    assert_eq!(best(&relay, "K32.A654.J432.32"), bid(4, Strain::Hearts));
    // Below the authored `fit_value >= 8` gate the floor's fit-sum (default 31,
    // a measured default-on win) takes over, counting the full trump length
    // opposite opener's 16-point max: a 6-count with a nine-card fit
    // (6 + 16 + 5 + 4 = 31) and a 7-count with an eight-card fit
    // (7 + 16 + 4 + 4 = 31) both clear it and bid game.
    assert_eq!(best(&relay, "Q32.KJ954.762.32"), bid(4, Strain::Hearts));
    assert_eq!(best(&relay, "K32.QJ54.J432.32"), bid(4, Strain::Hearts));
}

#[test]
fn stayman_fit_raise_by_value() {
    // 1NT–2♣–2♥ (opener's four-card major): responder raises on `fit_value`,
    // not raw HCP — any upgrade past a flat eight reaches game.
    let stayman = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Hearts),
        P,
    ];

    // Flat 4-3-3-3 eight, four-card fit, no upgrade: invitational raise (value 8).
    assert_eq!(best(&stayman, "K32.Q654.K32.432"), bid(3, Strain::Hearts));
    // 4-4-4-1 eight with a working singleton: the shape upgrades to value 9, so
    // the same eight now bids game instead of merely inviting.
    assert_eq!(best(&stayman, "Q543.K654.K432.2"), bid(4, Strain::Hearts));
    // Flat 4-3-3-3 seven: value 7, below the invite — passes the partscore.
    assert_eq!(best(&stayman, "K32.Q654.Q32.432"), P);
}
