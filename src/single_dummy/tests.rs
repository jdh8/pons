use super::*;
use contract_bridge::{Bid, Level, Strain};
use rand::SeedableRng;
use rand::rngs::StdRng;

const fn four_spades() -> Bid {
    Bid {
        level: Level::new(4),
        strain: Strain::Spades,
    }
}

/// North + South hold every spade, ace, king, and top honor: no defender can
/// ever win a trick, so North takes all thirteen in spades in *every* layout.
/// The spade game is therefore a double-dummy lock and the defender who holds
/// no trump can never make it.
fn unbeatable_spade_fit() -> (Hand, Hand) {
    // North: all top spades + AK of every side suit.
    let north: Hand = "AKQJT98.AK.AK.AK".parse().expect("valid test hand");
    // South: the rest of the spades + QJ(T) of the side suits.
    let south: Hand = "765432.QJT.QJ.QJ".parse().expect("valid test hand");
    (north, south)
}

/// The lock makes on every deal: `make_probability` is exactly 1 and the mean
/// trick count is a full thirteen, regardless of how the defenders' cards lie.
#[test]
fn unbeatable_fit_always_makes() {
    let (north, south) = unbeatable_spade_fit();
    let mut rng = StdRng::seed_from_u64(1);
    let hist = single_dummy(north, south, Seat::North, &mut rng, 16);

    assert_eq!(hist.expected_tricks(Seat::North, Strain::Spades), 13.0);
    assert_eq!(hist.make_probability(Seat::North, four_spades()), 1.0);
    // A defender holding no trump can never bring home a spade game.
    assert_eq!(hist.make_probability(Seat::East, four_spades()), 0.0);
}

/// Same seed and inputs reproduce the same histogram exactly (the solver never
/// samples its own RNG).
#[test]
fn deterministic_given_a_seed() {
    let (north, south) = unbeatable_spade_fit();
    let mut rng_a = StdRng::seed_from_u64(7);
    let a = single_dummy(north, south, Seat::North, &mut rng_a, 12);
    let mut rng_b = StdRng::seed_from_u64(7);
    let b = single_dummy(north, south, Seat::North, &mut rng_b, 12);
    assert_eq!(a, b);
}

/// With no layouts the histogram is empty and the readers report `NaN`.
#[test]
fn empty_is_no_signal() {
    let (north, south) = unbeatable_spade_fit();
    let mut rng = StdRng::seed_from_u64(0);
    let hist = single_dummy(north, south, Seat::North, &mut rng, 0);
    assert!(hist.expected_tricks(Seat::North, Strain::Spades).is_nan());
    assert!(hist.make_probability(Seat::North, four_spades()).is_nan());
}

/// The unbeatable-fit deal with the remaining cards split between the
/// defenders — the actual layout the lead scorer plays out.
fn unbeatable_deal() -> FullDeal {
    let (north, south) = unbeatable_spade_fit();
    let east: Hand = ".987654.T9876.T9".parse().expect("valid test hand");
    let west: Hand = ".32.5432.8765432".parse().expect("valid test hand");
    let mut builder = Builder::new();
    builder[Seat::North] = north;
    builder[Seat::South] = south;
    builder[Seat::East] = east;
    builder[Seat::West] = west;
    builder.build_full().expect("52 disjoint cards")
}

/// A silent reading — nothing shown by anyone.
fn no_inferences() -> Inferences {
    use crate::bidding::Context;
    use contract_bridge::auction::RelativeVulnerability;
    Inferences::read(&Context::new(RelativeVulnerability::NONE, &[]))
}

/// Against the lock no lead matters: declarer takes all thirteen whatever
/// East chooses, and the chosen card really is East's.
#[test]
fn lead_cannot_beat_the_lock() {
    let deal = unbeatable_deal();
    let mut rng = StdRng::seed_from_u64(3);
    let (lead, tricks) = single_dummy_lead_tricks(
        &deal,
        Strain::Spades,
        Seat::North,
        &no_inferences(),
        &mut rng,
        8,
    );
    assert!(deal[Seat::East][lead.suit].contains(lead.rank));
    assert_eq!(u8::from(tricks), 13);
}

/// Against the lock even a fallible declarer takes all thirteen: every
/// line wins in every world, so the playout cannot lose a trick.
#[test]
fn playout_cannot_misplay_the_lock() {
    let deal = unbeatable_deal();
    let inferences = no_inferences();
    let mut rng = StdRng::seed_from_u64(5);
    let (lead, lead_tricks, tricks) = single_dummy_declarer_tricks(
        &deal,
        Strain::Spades,
        Seat::North,
        &inferences,
        &inferences,
        &mut rng,
        8,
        8,
    );
    assert!(deal[Seat::East][lead.suit].contains(lead.rank));
    assert_eq!(u8::from(lead_tricks), 13);
    assert_eq!(u8::from(tricks), 13);
}

/// A grand slam hinging on a two-way trump-queen guess: North-South hold
/// every side winner, and the spade suit (AJT9 opposite K876, missing
/// Q5432) picks up West's ♠Q54 double-dummy by finessing through West —
/// but a declarer who cannot see the queen must guess.
fn two_way_guess_deal() -> FullDeal {
    let mut builder = Builder::new();
    builder[Seat::North] = "AJT9.AKQ.AKQ2.AK".parse().expect("valid test hand");
    builder[Seat::South] = "K876.JT9.JT9.QJ4".parse().expect("valid test hand");
    builder[Seat::West] = "Q54.8765.876.T98".parse().expect("valid test hand");
    builder[Seat::East] = "32.432.543.76532".parse().expect("valid test hand");
    builder.build_full().expect("52 disjoint cards")
}

/// Double-dummy the guess deal is a cold grand (the finesse is always
/// "found"), but the single-dummy playout must guess blind: over many
/// seeds it sometimes misguesses (no peeking at the actual layout) and
/// sometimes guesses right — and never loses more than the guess.
#[test]
fn playout_guesses_where_double_dummy_peeks() {
    let deal = two_way_guess_deal();
    // Fixture validity: DD says North makes 7♠ on the actual layout.
    let table = crate::dd::solver().solve_deal(deal);
    assert_eq!(u8::from(table[Strain::Spades].get(Seat::North)), 13);

    let inferences = no_inferences();
    let results: Vec<u8> = (0..12)
        .map(|seed| {
            let mut rng = StdRng::seed_from_u64(seed);
            let (_, lead_tricks, tricks) = single_dummy_declarer_tricks(
                &deal,
                Strain::Spades,
                Seat::North,
                &inferences,
                &inferences,
                &mut rng,
                8,
                8,
            );
            // The lead endpoint plays double-dummy after the lead, so it
            // still picks up the queen every time — the whole gap between
            // the two endpoints on this deal *is* the third-eye finesse.
            assert_eq!(u8::from(lead_tricks), 13);
            u8::from(tricks)
        })
        .collect();
    assert!(
        results.iter().all(|&tricks| (11..=13).contains(&tricks)),
        "only the guess (and rare mean-max risk) may cost tricks: {results:?}"
    );
    assert!(
        results.iter().any(|&tricks| tricks < 13),
        "a blind declarer must sometimes misguess: {results:?}"
    );
    assert!(
        results.contains(&13),
        "a blind declarer must sometimes guess right: {results:?}"
    );
}

/// Same seed and inputs reproduce the same playout exactly.
#[test]
fn playout_is_deterministic() {
    let deal = two_way_guess_deal();
    let inferences = no_inferences();
    let mut rng_a = StdRng::seed_from_u64(17);
    let a = single_dummy_declarer_tricks(
        &deal,
        Strain::Spades,
        Seat::North,
        &inferences,
        &inferences,
        &mut rng_a,
        6,
        6,
    );
    let mut rng_b = StdRng::seed_from_u64(17);
    let b = single_dummy_declarer_tricks(
        &deal,
        Strain::Spades,
        Seat::North,
        &inferences,
        &inferences,
        &mut rng_b,
        6,
        6,
    );
    assert_eq!(a, b);
}

/// Same seed and inputs reproduce the same lead and trick count.
#[test]
fn lead_choice_is_deterministic() {
    let deal = unbeatable_deal();
    let inferences = no_inferences();
    let mut rng_a = StdRng::seed_from_u64(11);
    let a = single_dummy_lead_tricks(
        &deal,
        Strain::Spades,
        Seat::North,
        &inferences,
        &mut rng_a,
        6,
    );
    let mut rng_b = StdRng::seed_from_u64(11);
    let b = single_dummy_lead_tricks(
        &deal,
        Strain::Spades,
        Seat::North,
        &inferences,
        &mut rng_b,
        6,
    );
    assert_eq!(a, b);
}

/// The lead scorer's stats are a pure tally: the same answers and RNG
/// position as the plain call, with every world from the reading when
/// nothing was shown.
#[test]
fn lead_stats_leave_the_answer_alone() {
    let question = LeadQuestion {
        deal: unbeatable_deal(),
        strain: Strain::Spades,
        declarer: Seat::North,
        inferences: no_inferences(),
    };
    let mut rng_a = StdRng::seed_from_u64(11);
    let mut rng_b = StdRng::seed_from_u64(11);
    let plain = single_dummy_leads(std::slice::from_ref(&question), &mut rng_a, 6);
    let (answers, stats) = single_dummy_leads_with_stats(&[question], &mut rng_b, 6);
    assert_eq!(plain, answers);
    assert_eq!(rng_a.next_u64(), rng_b.next_u64());
    assert_eq!(stats.len(), 1);
    assert_eq!(
        (stats[0].requested, stats[0].accepted, stats[0].fallback),
        (6, 6, 0)
    );
}

/// The batch overlaps sampling with solving, yet answers exactly as one
/// question at a time on the same RNG, and leaves the RNG at the same place.
#[test]
fn batched_leads_match_serial_leads() {
    let mut deal_rng = StdRng::seed_from_u64(23);
    let empty = Builder::new()
        .build_partial()
        .expect("an empty partial deal is valid");
    let questions: Vec<_> = fill_deals(&mut deal_rng, empty)
        .zip(Strain::ASC.into_iter().cycle())
        .zip(Seat::ALL.into_iter().cycle())
        .take(10)
        .map(|((deal, strain), declarer)| LeadQuestion {
            deal,
            strain,
            declarer,
            inferences: no_inferences(),
        })
        .collect();
    let mut rng_a = StdRng::seed_from_u64(29);
    let mut rng_b = StdRng::seed_from_u64(29);
    let batched = single_dummy_leads(&questions, &mut rng_a, 4);
    let serial: Vec<_> = questions
        .iter()
        .flat_map(|question| single_dummy_leads(std::slice::from_ref(question), &mut rng_b, 4))
        .collect();
    assert_eq!(batched, serial);
    assert_eq!(rng_a.next_u64(), rng_b.next_u64());
}

/// A reading the leader's own hand contradicts — they hold nine hearts, and
/// declarer (the leader's RHO) opened `1♥` — is topped up unconstrained, and
/// the stats say so.
#[test]
fn lead_stats_count_the_unconstrained_fill() {
    use crate::bidding::Context;
    use contract_bridge::auction::{Call, RelativeVulnerability};
    let east: Hand = "32.AKQJT9876.2.2".parse().expect("valid test hand");
    let mut builder = Builder::new();
    builder[Seat::East] = east;
    let partial = builder
        .build_partial()
        .expect("one hand is a valid partial deal");
    let deal = fill_deals(&mut StdRng::seed_from_u64(0), partial)
        .next()
        .expect("fill_deals is infinite");
    let one_heart = [Call::Bid(Bid {
        level: Level::new(1),
        strain: Strain::Hearts,
    })];
    let question = LeadQuestion {
        deal,
        strain: Strain::Hearts,
        declarer: Seat::North,
        inferences: Inferences::read(&Context::new(RelativeVulnerability::NONE, &one_heart)),
    };
    let mut rng = StdRng::seed_from_u64(1);
    let (_, stats) = single_dummy_leads_with_stats(&[question], &mut rng, 4);
    assert_eq!(stats[0].accepted, 0);
    assert_eq!((stats[0].fallback, stats[0].fallback_draws), (4, 4));
    assert_eq!(stats[0].returned(), 4);
    assert!(stats[0].degraded());
}

/// The playout's stats sum its declarer decisions: `k` worlds each, all from
/// the reading when nothing was shown — and the composed call returns the
/// same lead and tricks as the plain one.
#[test]
fn playout_stats_sum_the_decisions() {
    let deal = two_way_guess_deal();
    let inferences = no_inferences();
    let mut rng_a = StdRng::seed_from_u64(17);
    let mut rng_b = StdRng::seed_from_u64(17);
    let plain = single_dummy_declarer_tricks(
        &deal,
        Strain::Spades,
        Seat::North,
        &inferences,
        &inferences,
        &mut rng_a,
        6,
        4,
    );
    let (lead, lead_tricks, tricks, [lead_stats, playout_stats]) =
        single_dummy_declarer_tricks_with_stats(
            &deal,
            Strain::Spades,
            Seat::North,
            &inferences,
            &inferences,
            &mut rng_b,
            6,
            4,
        );
    assert_eq!(plain, (lead, lead_tricks, tricks));
    assert_eq!(rng_a.next_u64(), rng_b.next_u64());

    assert_eq!((lead_stats.requested, lead_stats.fallback), (6, 0));
    assert!(playout_stats.requested > 0, "the guess is a real decision");
    assert_eq!(playout_stats.requested % 4, 0, "k worlds per decision");
    assert_eq!(playout_stats.accepted, playout_stats.requested);
    assert_eq!(playout_stats.fallback, 0);
}

/// Golden pin for the default (mean-tricks) objective: the composed playout
/// on the two-way guess deal, seeds 0..8 at six worlds, recorded before the
/// contract-aware objective existed.  Any drift here means the default path
/// stopped being byte-identical.
#[test]
fn mean_tricks_playout_is_pinned() {
    let deal = two_way_guess_deal();
    let inferences = no_inferences();
    let got: Vec<(Card, u8, u8)> = (0..8)
        .map(|seed| {
            let mut rng = StdRng::seed_from_u64(seed);
            let (lead, lead_tricks, tricks) = single_dummy_declarer_tricks(
                &deal,
                Strain::Spades,
                Seat::North,
                &inferences,
                &inferences,
                &mut rng,
                6,
                6,
            );
            (lead, u8::from(lead_tricks), u8::from(tricks))
        })
        .collect();
    let c2 = Card {
        suit: Suit::Clubs,
        rank: Rank::new(2),
    };
    let d3 = Card {
        suit: Suit::Diamonds,
        rank: Rank::new(3),
    };
    let want = vec![
        (c2, 13, 13),
        (d3, 13, 13),
        (c2, 13, 12),
        (c2, 13, 13),
        (c2, 13, 13),
        (d3, 13, 12),
        (d3, 13, 13),
        (d3, 13, 13),
    ];
    assert_eq!(got, want);
}

const fn card(suit: Suit, rank: u8) -> Card {
    Card {
        suit,
        rank: Rank::new(rank),
    }
}

/// A four-card notrump ending, North to play third to the trick: South led
/// ♠3 towards dummy's ♠AQ and West followed ♠4.  Declarer's side has nine
/// tricks in and ♠A + ♥AK on top for twelve.  Rising with the ace is twelve
/// in every layout; the queen finesse is thirteen when West holds the ♠K,
/// but when East wins it he cashes ♦KQ and declarer takes one more trick in
/// all — ten.  Four worlds put the king onside and one offside, so the
/// finesse averages 12.4 tricks against the ace's 12.0 — the illustrative
/// "sure twelve vs. 80% of thirteen" case, played out double-dummy.
///
/// Returns North's candidates and the five worlds' `Target::Legal` solves.
fn safety_play_ending() -> (Vec<Card>, Vec<FoundPlays>) {
    let north: Hand = "AQ.AK..".parse().expect("valid test hand");
    let south: Hand = "2.2..2".parse().expect("valid test hand");
    // (West's three remaining cards, East's four)
    let worlds = [
        ("K.QJ..", "J..KQ.A"),
        ("KJ...A", ".QJ.KQ."),
        ("K..KQ.", "J.QJ..A"),
        ("K..Q.A", "J.QJ.K."),
        (".QJ..A", "KJ..KQ."),
    ];
    let trick = [card(Suit::Spades, 3), card(Suit::Spades, 4)];
    let objectives: Vec<Objective> = worlds
        .iter()
        .map(|(west, east)| {
            let mut builder = Builder::default();
            builder[Seat::North] = north;
            builder[Seat::South] = south;
            builder[Seat::West] = west.parse().expect("valid test hand");
            builder[Seat::East] = east.parse().expect("valid test hand");
            Objective {
                board: Board::try_new(
                    builder.build_partial().expect("disjoint ending"),
                    CurrentTrick::from_slice(Strain::Notrump, Seat::South, &trick)
                        .expect("a two-card trick"),
                )
                .expect("a valid ending"),
                target: Target::Legal,
            }
        })
        .collect();
    // The ♠K is unseen, so it still separates the ace from the queen.
    let candidates = distinct_plays(north, Some(Suit::Spades), Hand::EMPTY);
    (candidates, crate::dd::solver().solve_boards(&objectives))
}

/// Same layout, same strain, three objectives: the mean-tricks declarer
/// finesses, the 6NT declarer (twelve needed) takes the safety play, and the
/// 7NT declarer (thirteen needed) finesses because only the finesse can make.
#[test]
fn make_objective_takes_the_safety_play() {
    let (candidates, solved) = safety_play_ending();
    let ace = card(Suit::Spades, 14);
    let queen = card(Suit::Spades, 12);
    let mut sorted = candidates.clone();
    sorted.sort_by_key(|card| card.rank);
    assert_eq!(sorted, [queen, ace], "North's two distinct plays");

    // Premises, re-proved double-dummy: the ace is three of the last four
    // tricks everywhere; the queen is four onside and one offside.
    let tricks = |card| -> Vec<u64> { solved.iter().map(|w| score_of(&w.plays, card)).collect() };
    assert_eq!(tricks(ace), [3, 3, 3, 3, 3]);
    assert_eq!(tricks(queen), [4, 4, 4, 4, 1]);

    let won = 9;
    for order in [
        candidates.clone(),
        candidates.iter().rev().copied().collect(),
    ] {
        let pick = |objective| pick_declarer_card(&order, &solved, objective, won);
        assert_eq!(pick(DeclarerObjective::MeanTricks), queen, "12.4 > 12.0");
        assert_eq!(pick(DeclarerObjective::Make(12)), ace, "5/5 > 4/5 make");
        assert_eq!(pick(DeclarerObjective::Make(13)), queen, "4/5 > 0/5 make");
    }
}

/// When every candidate makes in every world, the make objective falls back
/// to mean tricks: the 3NT declarer (nine needed, nine already won) still
/// takes the overtrick line.
#[test]
fn make_objective_breaks_ties_by_mean_tricks() {
    let (candidates, solved) = safety_play_ending();
    let queen = card(Suit::Spades, 12);
    for order in [
        candidates.clone(),
        candidates.iter().rev().copied().collect(),
    ] {
        assert_eq!(
            pick_declarer_card(&order, &solved, DeclarerObjective::Make(9), 9),
            queen,
        );
    }
}
