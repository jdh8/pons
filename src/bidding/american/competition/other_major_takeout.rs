//! Responder's takeout double when the overcall is the other major —
//! `1♥ (1♠) X`, `1♥ (2♠) X`, `1♠ (2♥) X`
//!
//! Gated on `agreements.competition.other_major_takeout_double` (default on
//! since 2026-11-09).
//! The shipped double reads the "other major" off the opening alone, so here
//! it promises four-plus of *their* suit: a values double whose mass is 5–6
//! card trump stacks, losing to BBA's pass (anchor `ce114214`, per 409,600
//! boards: −1,167 plain / −1,963 PD).  On, the double is BBA's book meaning —
//! 3+♣, 3+♦, ≤2 in opener's suit, ≤4 of theirs, 7+ HCP over `(1♠)` and 10+ at
//! the two level; five-plus of their suit falls to the trap pass and opener's
//! reopening double.  The rule itself lives in
//! [`over_their_overcall`][super::over_overcall]; this module is the rest of
//! the convention.
//!
//! The doubler's other second turns stay
//! [`negative_doubler_rebid`][super::negative_double::negative_doubler_rebid]'s.

use super::*;

/// The auctions this agreement covers, as (opening, overcall)
fn slots() -> [(Suit, Bid); 3] {
    [
        (Suit::Hearts, Bid::new(1, Strain::Spades)),
        (Suit::Hearts, Bid::new(2, Strain::Spades)),
        (Suit::Spades, Bid::new(2, Strain::Hearts)),
    ]
}

/// Their suit: the overcall of a [`slots`] row is always a suit
fn their_suit(overcall: Bid) -> Suit {
    Suit::try_from(overcall.strain).expect("slots only hold suit overcalls")
}

/// The cheapest level for opener's own suit over `overcall`
fn own_level(opening: Suit, overcall: Bid) -> u8 {
    overcall.level.get() + u8::from(Strain::from(opening) <= overcall.strain)
}

/// Opener's answer after `1o (t) X -`, partner showing both minors
///
/// A longer minor first, a six-card suit above a four-card minor, notrump by
/// range with their suit stopped and no four-card minor.  The rebid of the
/// opening suit is the finite catch-all: no pass, the rule the Modern answer
/// measured, since opener's trumps sit in front of the overcaller's.  BBA does
/// pass here on some 5-card hands; unauthored.
fn opener_answer(opening: Suit, overcall: Bid) -> Rules {
    let o = Strain::from(opening);
    let their = their_suit(overcall);
    let one_level = overcall.level.get() == 1;
    let (c, d) = (Suit::Clubs, Suit::Diamonds);
    let (cs, ds) = (Strain::Clubs, Strain::Diamonds);
    let own = own_level(opening, overcall);
    let cheap_nt = overcall.level.get();
    let minor = if one_level { 2 } else { 3 };
    let no_minor = len(c, ..=3) & len(d, ..=3);
    // One notrump rung above the cheapest shows the balanced extras.
    let high_nt = if one_level { 18 } else { 15 };

    let mut rules = Rules::new()
        .rule(
            Bid::new(cheap_nt, Strain::Notrump),
            120,
            no_minor.clone() & stopper_in(their) & hcp(12..=14),
        )
        .rule(
            Bid::new(cheap_nt + 1, Strain::Notrump),
            120,
            no_minor & stopper_in(their) & hcp(high_nt..=19),
        )
        .rule(Bid::new(own + 1, o), 110, len(opening, 6..) & hcp(16..=18));
    // The jump in a minor on extras needs a level to jump to below game.
    if one_level {
        rules = rules
            .rule(Bid::new(3, ds), 106, len(d, 5..) & len(c, ..=4) & hcp(17..))
            .rule(Bid::new(3, cs), 105, len(c, 4..) & hcp(17..))
            .rule(Bid::new(3, ds), 104, len(d, 4..) & hcp(17..));
    }
    rules
        .rule(Bid::new(own, o), 103, len(opening, 6..))
        .rule(Bid::new(minor, ds), 101, len(d, 5..) & len(c, ..=4))
        .rule(Bid::new(minor, cs), 100, len(c, 4..))
        .rule(Bid::new(minor, ds), 99, len(d, 4..))
        .rule(Bid::new(own, o), 0, hcp(0..))
}

/// The doubler's second turn over opener's common answers
///
/// The double is unlimited, so game has to be reachable here; the shared
/// rebid table has no `3NT`.  `answer` is a [`common_answers`] row.
fn doubler_rebid(opening: Suit, overcall: Bid, answer: Bid) -> Rules {
    let o = Strain::from(opening);
    let their = their_suit(overcall);
    let nt = |level| Bid::new(level, Strain::Notrump);
    let rules = if answer.strain == Strain::Notrump {
        // `2NT` over a one-level overcall is the 18–19 hand; every other
        // notrump answer is 12–14.
        if overcall.level.get() == 1 && answer.level.get() == 2 {
            Rules::new().rule(nt(3), 100, hcp(6..))
        } else {
            Rules::new()
                .rule(nt(3), 100, hcp(13..))
                .rule(nt(2), 100, hcp(11..=12))
        }
    } else if answer.strain == o {
        // Opener's own suit: at most two trumps here.
        let mut rules = Rules::new()
            .rule(nt(3), 110, stopper_in(their) & hcp(12..))
            .rule(Bid::new(4, o), 100, len(opening, 2..) & hcp(12..));
        if answer.level.get() == 2 {
            rules = rules.rule(nt(2), 105, stopper_in(their) & hcp(10..=11));
        }
        rules
    } else {
        // A natural minor, four-plus opposite the double's three-plus.
        let m = answer.strain;
        let suit = Suit::try_from(m).expect("a minor answer");
        let rules = Rules::new().rule(Bid::new(5, m), 100, len(suit, 4..) & hcp(13..));
        if answer.level.get() == 2 {
            rules
                .rule(nt(3), 110, stopper_in(their) & hcp(12..))
                .rule(nt(2), 105, stopper_in(their) & hcp(10..=11))
                .rule(Bid::new(3, m), 90, len(suit, 4..) & hcp(10..=12))
        } else {
            rules.rule(nt(3), 110, stopper_in(their) & hcp(10..)).rule(
                Bid::new(4, m),
                90,
                len(suit, 4..) & hcp(10..=12),
            )
        }
    };
    rules.rule(Call::Pass, 0, hcp(0..))
}

/// The answers [`doubler_rebid`] takes over; rarer ones keep the shared table
fn common_answers(opening: Suit, overcall: Bid) -> Vec<Bid> {
    let o = Strain::from(opening);
    let own = own_level(opening, overcall);
    let minor = if overcall.level.get() == 1 { 2 } else { 3 };
    let cheap_nt = overcall.level.get();
    let mut answers = vec![
        Bid::new(own, o),
        Bid::new(minor, Strain::Clubs),
        Bid::new(minor, Strain::Diamonds),
        Bid::new(cheap_nt, Strain::Notrump),
    ];
    if overcall.level.get() == 1 {
        answers.push(Bid::new(2, Strain::Notrump));
    }
    answers
}

/// Opener after they bid over the double, `1o (t) X (x)`
///
/// Compete only with extra shape: the six-card suit, or five of a minor
/// opposite the double's three.  Pass is the catch-all and leaves the doubler
/// room.  A penalty or values `X` here is unauthored.
fn opener_over_interference(opening: Suit, x: Bid) -> Rules {
    let o = Strain::from(opening);
    let cheapest = |s: Strain| x.level.get() + u8::from(s <= x.strain);
    let mut rules = Rules::new();
    if o != x.strain && cheapest(o) <= 4 {
        rules = rules.rule(Bid::new(cheapest(o), o), 100, len(opening, 6..));
    }
    for (weight, m) in [(90, Suit::Clubs), (91, Suit::Diamonds)] {
        let s = Strain::from(m);
        if s != x.strain && cheapest(s) <= 3 {
            rules = rules.rule(Bid::new(cheapest(s), s), weight, len(m, 5..));
        }
    }
    rules.rule(Call::Pass, 0, hcp(0..))
}

/// The package: opener's answers, the doubler's second turn and the contested
/// tails for all three [`slots`]
pub(super) fn other_major_takeout_package() -> Package {
    Package {
        name: "other-major-takeout-double",
        gate: |agreements| agreements.competition.other_major_takeout_double,
        entries: |_| {
            let mut entries = Vec::new();
            for (opening, overcall) in slots() {
                let key = format!("P* 1{} ({overcall})", Strain::from(opening));
                entries.extend(rows_of(
                    Pattern::node(&format!("{key} X -")),
                    opener_answer(opening, overcall),
                ));
                // A redouble changes nothing about opener's hand.
                entries.extend(rows_of(
                    Pattern::node(&format!("{key} X (XX)")),
                    opener_answer(opening, overcall),
                ));
                for answer in common_answers(opening, overcall) {
                    entries.extend(rows_of(
                        Pattern::node(&format!("{key} X - {answer} -")),
                        doubler_rebid(opening, overcall, answer),
                    ));
                }
                // ponytail: suits through 3♠ only; their notrump stays the
                // floor's until a trace shows it matters.
                entries.extend(expand(
                    &format!("{key} X (.x)"),
                    |bindings| bindings.bid('x') <= Bid::new(3, Strain::Spades),
                    move |bindings| opener_over_interference(opening, bindings.bid('x')),
                ));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
