//! Representative auctions for the basic 2/1 game-forcing system

mod common;
use common::*;

// --- Openings ---------------------------------------------------------------

#[test]
fn test_openings() {
    let system = stance();
    let open = &[][..];

    // 16 HCP balanced -> 1NT, even though 2♣ exists for the very strong.
    assert_eq!(
        best_call(&system, open, "AQ32.K53.QJ4.A92"),
        call(1, Strain::Notrump)
    );
    // 20 HCP balanced (20.4 Fifths) -> 2NT.
    assert_eq!(
        best_call(&system, open, "AJT2.KQT.KJT.AQ9"),
        call(2, Strain::Notrump)
    );
    // 20 HCP balanced -> 2NT.  This queen-heavy 20-count is only 18.8 Fifths, so
    // the old Fifths gauge downgraded it to a 1♣-planning-2NT-rebid; raw HCP is
    // the default now (Fifths lost the A6 audit), so it opens 2NT directly.
    assert_eq!(
        best_call(&system, open, "AKQ2.KQ5.KJ4.Q92"),
        call(2, Strain::Notrump)
    );
    // 22 HCP -> the strong, artificial 2♣.
    assert_eq!(
        best_call(&system, open, "AKQ2.AKJ.KQ4.932"),
        call(2, Strain::Clubs)
    );
    // 13 HCP, five spades -> 1♠.
    assert_eq!(
        best_call(&system, open, "AQJ32.K53.Q42.J2"),
        call(1, Strain::Spades)
    );
    // 13 HCP, five hearts -> 1♥.
    assert_eq!(
        best_call(&system, open, "A2.KQJ53.Q42.J92"),
        call(1, Strain::Hearts)
    );
    // 13 HCP, 4-4 minors -> 1♦ (better minor).
    assert_eq!(
        best_call(&system, open, "K2.A53.KJ42.Q982"),
        call(1, Strain::Diamonds)
    );
    // 14 HCP, 4-3-3-3 with 3-3 minors -> 1♣.
    assert_eq!(
        best_call(&system, open, "KQ52.A53.Q43.K92"),
        call(1, Strain::Clubs)
    );
    // 6 HCP, six spades -> a weak two.
    assert_eq!(
        best_call(&system, open, "KQJ732.53.842.92"),
        call(2, Strain::Spades)
    );
    // 9 HCP, too light to open in first seat.
    assert_eq!(best_call(&system, open, "AQJ32.853.Q42.92"), Call::Pass);
}

#[test]
fn test_light_third_seat_major() {
    let system = stance();
    // The same 9-count that passes in first seat opens 1♠ in third.
    assert_eq!(
        best_call(&system, &[Call::Pass, Call::Pass], "AQJ32.853.Q42.92"),
        call(1, Strain::Spades),
    );
}

// --- Major responses --------------------------------------------------------

#[test]
fn test_major_responses() {
    let system = stance();
    let after_1h = &[call(1, Strain::Hearts), Call::Pass][..];

    // 9 HCP, three-card support -> single raise.
    assert_eq!(
        best_call(&system, after_1h, "Q32.J53.A964.Q92"),
        call(2, Strain::Hearts)
    );
    // 14 HCP, four-card support -> Jacoby 2NT (game-forcing raise).
    assert_eq!(
        best_call(&system, after_1h, "K2.KQ54.A964.Q92"),
        call(2, Strain::Notrump)
    );
    // 10 HCP, four spades, no heart fit -> 1♠ (a new suit at the one level).
    assert_eq!(
        best_call(&system, after_1h, "KQ32.J5.A964.982"),
        call(1, Strain::Spades)
    );
    // 13 HCP, four clubs, no fit, no spades -> a 2/1 game force.
    assert_eq!(
        best_call(&system, after_1h, "A2.K3.Q543.KJ85"),
        call(2, Strain::Clubs)
    );
    // 8 HCP, no fit, no four-card spade suit -> the forcing 1NT.
    assert_eq!(
        best_call(&system, after_1h, "Q2.J3.K9643.Q982"),
        call(1, Strain::Notrump)
    );
}

// --- Minor responses --------------------------------------------------------

#[test]
fn test_minor_responses() {
    let system = stance();

    // 1♣ - four hearts up the line -> 1♥.
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Clubs), Call::Pass],
            "K32.KQ54.J643.92"
        ),
        call(1, Strain::Hearts),
    );
    // 1♦ - 13 HCP, five clubs, no major -> a 2/1 game force.
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Diamonds), Call::Pass],
            "A32.K3.Q43.KJ842"
        ),
        call(2, Strain::Clubs),
    );
}

// --- 1NT structure ----------------------------------------------------------

#[test]
fn test_notrump_responses_and_completions() {
    let system = stance();
    let p = Call::Pass;
    let one_nt = call(1, Strain::Notrump);

    // Flat 4-3-3-3 with a four-card major, invitational (8): it neither Staymans
    // (no ruff — plays 3NT, not the 4-4 fit) nor invites.  The flat shape is its
    // high cards and nothing more, so it plays a level too high opposite a 15-17;
    // a double-dummy probe scores passing over the 2♠ invite at +0.64 IMPs/board
    // (`examples/probe-uninvite-4333`).  So it passes 1NT.
    assert_eq!(best_call(&system, &[one_nt, p], "KJ54.Q32.J43.J92"), p);
    // Transfer to spades on a five-card suit.
    assert_eq!(
        best_call(&system, &[one_nt, p], "KJ542.Q32.K43.92"),
        call(2, Strain::Hearts)
    );

    // Opener completes the spade transfer.
    assert_eq!(
        best_call(
            &system,
            &[one_nt, p, call(2, Strain::Hearts), p],
            "AQ32.K53.QJ4.A92"
        ),
        call(2, Strain::Spades),
    );
    // Opener answers Stayman with four hearts.
    assert_eq!(
        best_call(
            &system,
            &[one_nt, p, call(2, Strain::Clubs), p],
            "A32.KJ54.KQ4.A92"
        ),
        call(2, Strain::Hearts),
    );
}

// --- Opener's rebid ---------------------------------------------------------

#[test]
fn test_opener_rebid_raises_spades() {
    let system = stance();
    let p = Call::Pass;
    // 1♥ - 1♠ - ?: 14 HCP with four spades raises to 2♠.
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Hearts), p, call(1, Strain::Spades), p],
            "KQ32.AQ542.K43.2"
        ),
        call(2, Strain::Spades),
    );
}

// --- Competition over our opening ------------------------------------------

#[test]
fn test_negative_double_and_system_on() {
    let system = stance();
    let one_h = call(1, Strain::Hearts);

    // 1♥ - (2♣) - ?: 10 HCP with four spades makes a negative double.
    assert_eq!(
        best_call(
            &system,
            &[one_h, call(2, Strain::Clubs)],
            "KQ32.J5.A964.982"
        ),
        Call::Double,
    );
    // 1♥ - (X) - ?: system on, the responses apply through the double.
    assert_eq!(
        best_call(&system, &[one_h, Call::Double], "Q32.J53.A964.Q92"),
        call(2, Strain::Hearts),
    );
}

// --- Defense ----------------------------------------------------------------

#[test]
fn test_defense() {
    let system = stance();

    // (1♣) - ?: 9 HCP with five spades overcalls 1♠.
    assert_eq!(
        best_call(&system, &[call(1, Strain::Clubs)], "AQJ32.853.Q42.92"),
        call(1, Strain::Spades),
    );
    // (1♠) - ?: 15 HCP short in spades makes a takeout double.
    assert_eq!(
        best_call(&system, &[call(1, Strain::Spades)], "2.KQ54.AJ43.KQ92"),
        Call::Double,
    );
    // (1♣) - 1♠ - (P) - ?: advancing partner's overcall is the instinct floor's
    // Rubens job now; a weak three-card raise still takes the simple 2♠ (a limit
    // raise would transfer — see the Rubens rails in american_instinct).
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Clubs), call(1, Strain::Spades), Call::Pass],
            "Q32.J54.A432.J92",
        ),
        call(2, Strain::Spades),
    );
}

// --- More openings ----------------------------------------------------------

#[test]
fn test_more_openings() {
    let system = stance();
    let open = &[][..];

    // 20 HCP balanced (20.4 Fifths) -> 2NT.
    assert_eq!(
        best_call(&system, open, "AJT2.KQT.KJT.AQ9"),
        call(2, Strain::Notrump)
    );
    // Nine-count with six hearts -> a weak two.
    assert_eq!(
        best_call(&system, open, "53.KQJ732.K42.92"),
        call(2, Strain::Hearts)
    );
    // Seven-card spade suit, weak -> a three-level preempt.
    assert_eq!(
        best_call(&system, open, "KQJ8732.5.842.92"),
        call(3, Strain::Spades)
    );
    // A weak-two shape passes in fourth seat (no preempts there).
    assert_eq!(
        best_call(&system, &[Call::Pass; 3], "KQJ732.53.842.92"),
        Call::Pass,
    );
}

// --- Response grades --------------------------------------------------------

#[test]
fn test_major_raise_grades() {
    let system = stance();
    let after_1h = &[call(1, Strain::Hearts), Call::Pass][..];

    // 12 HCP, four-card support -> limit raise (limit raises promise four
    // trumps).  Not flat (4333): that hand now offers the choice-of-games
    // 3NT instead (`set_major_choice_of_games`).
    assert_eq!(
        best_call(&system, after_1h, "K32.K653.A962.Q9"),
        call(3, Strain::Hearts)
    );
}

#[test]
fn test_minor_raise() {
    let system = stance();
    // 1♦ - eight-count with five-card support -> inverted minors flip the raise
    // meanings: 3♦ is the weak preemptive raise.
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Diamonds), Call::Pass],
            "T32.J53.KQ942.Q2"
        ),
        call(3, Strain::Diamonds),
    );
}

#[test]
fn test_notrump_ladder() {
    let system = stance();
    let after_1nt = &[call(1, Strain::Notrump), Call::Pass][..];

    // 11 HCP, flat 4-3-3-3 (four clubs): no Puppet — a flat hand plays 3NT, not
    // the 5-3 major fit it might dig up.
    assert_eq!(
        best_call(&system, after_1nt, "K32.Q43.KJ4.Q932"),
        call(3, Strain::Notrump)
    );
    // Five hearts -> transfer (2♦).
    assert_eq!(
        best_call(&system, after_1nt, "K3.KJ542.Q43.J92"),
        call(2, Strain::Diamonds)
    );
}

// --- More defense -----------------------------------------------------------

#[test]
fn test_defense_extras() {
    let system = stance();

    // (1♦) - 18 HCP with length in diamonds: double first, plan to bid again.
    assert_eq!(
        best_call(&system, &[call(1, Strain::Diamonds)], "A.Q6.KJ852.AKJ42"),
        Call::Double,
    );
    // (1♣) - 17 HCP balanced with a club stopper -> 1NT overcall.
    assert_eq!(
        best_call(&system, &[call(1, Strain::Clubs)], "AQ2.KJ3.KQ54.Q92"),
        call(1, Strain::Notrump),
    );
}

// --- Full table -------------------------------------------------------------

#[test]
fn test_full_board_smoke() {
    // Two bound copies paired into a table: the dealer's side opens, the
    // other defends.
    let table = stance().vs(stance());

    assert_eq!(
        best_call(&table, &[], "AQ32.K53.QJ4.A92"),
        call(1, Strain::Notrump)
    );
    // After the opening, the opposing side's defensive book answers.
    assert!(
        table
            .classify(
                "AQJ32.853.Q42.92".parse().unwrap(),
                RelativeVulnerability::NONE,
                &[call(1, Strain::Clubs)]
            )
            .is_some()
    );
}

// --- End-to-end auctions across conventions ---------------------------------

#[test]
fn test_jacoby_into_keycards() {
    // 1♠ - 2NT (Jacoby) - 3♣ (shortness) - 4NT (RKCB): opener answers.
    let system = stance();
    let p = Call::Pass;
    let auction = [
        call(1, Strain::Spades),
        p,
        call(2, Strain::Notrump),
        p,
        call(3, Strain::Clubs),
        p,
        call(4, Strain::Notrump),
        p,
    ];
    // ♠A plus the trump king = 2 keycards without the trump queen -> 5♥.
    assert_eq!(
        best_call(&system, &auction, "AKJ52.K765.Q72.9"),
        call(5, Strain::Hearts),
    );
}

#[test]
fn test_game_force_into_keycards() {
    // 1♠ - 2♣ - 2♦ - 3♠ (sets trump) - 4NT: responder answers 1430.
    let system = stance();
    let p = Call::Pass;
    let auction = [
        call(1, Strain::Spades),
        p,
        call(2, Strain::Clubs),
        p,
        call(2, Strain::Diamonds),
        p,
        call(3, Strain::Spades),
        p,
        call(4, Strain::Notrump),
        p,
    ];
    // ♥A + ♣A + ♠K = 3 keycards -> 5♦ (0 or 3).
    assert_eq!(
        best_call(&system, &auction, "K32.A2.Q54.AKJ92"),
        call(5, Strain::Diamonds),
    );
}

#[test]
fn test_strong_two_system_on_transfer() {
    // 2♣ - 2♥ (double negative) - 2NT (22-24): transfers stay on.
    let system = stance();
    let p = Call::Pass;
    let auction = [
        call(2, Strain::Clubs),
        p,
        call(2, Strain::Hearts),
        p,
        call(2, Strain::Notrump),
        p,
    ];
    // A bust with five spades transfers at the three level.
    assert_eq!(
        best_call(&system, &auction, "J8542.T32.943.92"),
        call(3, Strain::Hearts),
    );

    // ... and opener completes the transfer.
    let completed = [
        call(2, Strain::Clubs),
        p,
        call(2, Strain::Hearts),
        p,
        call(2, Strain::Notrump),
        p,
        call(3, Strain::Hearts),
        p,
    ];
    assert_eq!(
        best_call(&system, &completed, "AKQ2.AKJ.KQ4.932"),
        call(3, Strain::Spades),
    );
}

// --- Binding ----------------------------------------------------------------

#[test]
fn test_competition_book_needs_binding() {
    // The unbound competitive book answers the negative double directly...
    let book = pons::bidding::american::competition();
    let one_h = call(1, Strain::Hearts);

    assert_eq!(
        best_call(&book, &[one_h, call(2, Strain::Clubs)], "KQ32.J5.A964.982"),
        Call::Double,
    );
    // ...but its system-on rebase lands in the uncontested core, which only
    // the stance bound by `Pair::against` contains. Responder's *first* call
    // over the double is answered directly (the shipped Jordan/Truscott node),
    // so probe one step deeper: opener's rebid after a forcing 1♠ still rides
    // the rebase onto the uncontested tree.
    assert!(
        book.classify(
            "Q32.J53.A964.Q92".parse().unwrap(),
            RelativeVulnerability::NONE,
            &[one_h, Call::Double, call(1, Strain::Spades), Call::Pass]
        )
        .is_none()
    );
}
