use super::*;

/// The queen cannot change what the asker bids: we hold it, or the fit alone
/// carries the small slam
///
/// The asker's test, and deliberately a rung below the answerer's
/// [`has_trump_queen`]: a nine-card fit is not a queen, but hearing "no queen"
/// over one changes nothing — six is bid anyway — so the round is not worth
/// spending.  Threshold `QUEEN_BUFF_FIT`.
fn queen_moot(
    trump: Suit,
) -> crate::bidding::constraint::Cons<impl crate::bidding::constraint::Constraint + Clone> {
    let threshold = usize::from(crate::bidding::instinct::queen_buff_fit());
    described(
        format!("the {trump} queen cannot change the call"),
        move |hand: Hand, context: &crate::bidding::context::Context<'_>| {
            hand[trump].contains(Rank::Q)
                || hand[trump].len() + usize::from(context.inferences().partner().length(trump).min)
                    >= threshold
        },
    )
}

/// A ninth trump or a side-suit void — the values RKCB has no rung for
///
/// Paired with `!has_trump_queen` at the buff jump: partner asked for the queen
/// holding four keycards and will pass five over a denial, never learning that
/// the fit is a card longer than promised or that a suit is stopped by a void.
/// The threshold is `QUEEN_BUFF_FIT`.
fn trump_buff(
    trump: Suit,
) -> crate::bidding::constraint::Cons<impl crate::bidding::constraint::Constraint + Clone> {
    let threshold = usize::from(crate::bidding::instinct::queen_buff_fit());
    described(
        format!("a ninth {trump} or a void"),
        move |hand: Hand, context: &crate::bidding::context::Context<'_>| {
            hand[trump].len() + usize::from(context.inferences().partner().length(trump).min)
                >= threshold
                || Suit::ASC
                    .into_iter()
                    .any(|suit| suit != trump && hand[suit].is_empty())
        },
    )
}

// ---------------------------------------------------------------------------
// The queen relay
// ---------------------------------------------------------------------------
//
// The book's 5♣ and 5♦ answers say nothing about the trump queen, so the asker
// has been betting six on four keycards blind.  The relay is one step above the
// answer, partner's two replies the next two rungs, and — on the queen-shown
// branch only — a king ask above that.  Geometry is shared with the floor
// ([`queen_ask_room`], [`relay_map`], [`king_relay`]) so the two ladders
// cannot drift.

/// Open an asker table with the queen relay, when the lane has room and
/// `interested` says the queen is what the placement turns on
///
/// Weighted above every placement in the table it opens, so a queenless asker
/// relays instead of guessing; holding the queen (or the ten-card fit) the rule
/// is dead and the table below is exactly what shipped.
pub(super) fn relay_first(
    trump: Suit,
    answer: Bid,
    interested: crate::bidding::constraint::Cons<
        impl crate::bidding::constraint::Constraint + Clone + 'static,
    >,
) -> Rules {
    let rules = Rules::new();
    match queen_ask_room(answer, trump) {
        Some(relay) => rules
            .rule(relay, 160, interested & !queen_moot(trump))
            .alert(RKCB),
        None => rules,
    }
}

/// Holds the king of `suit` and of none of the suits on `cheaper` rungs — the
/// "skipped steps deny" half of a king rung
fn cheapest_king(
    suit: Suit,
    cheaper: Vec<Suit>,
) -> crate::bidding::constraint::Cons<impl crate::bidding::constraint::Constraint + Clone> {
    described(
        format!("holds the {suit} king and no cheaper side king"),
        move |hand: Hand, _: &crate::bidding::context::Context<'_>| {
            hand[suit].contains(Rank::K) && cheaper.iter().all(|&s| !hand[s].contains(Rank::K))
        },
    )
}

/// Partner's merged reply to the queen relay
///
/// One round carries the queen and a king both: each side suit's rung shows the
/// queen plus that king and denies every king on a cheaper rung, 5NT shows the
/// queen with no side king at all, and the two denials are five and six of the
/// agreed trump — contracts rather than codes, so neither is alerted.  Six is
/// the *stronger* denial, the ninth trump or the void the ladder has no rung
/// for.
pub(super) fn queen_replies(trump: Suit, map: &RelayMap) -> Rules {
    let mut rules = Rules::new();
    for (index, &(suit, call)) in map.kings.iter().enumerate() {
        let cheaper = map.kings[..index].iter().map(|&(s, _)| s).collect();
        rules = rules
            .rule(
                call,
                100,
                has_trump_queen(trump) & cheapest_king(suit, cheaper),
            )
            .alert(RKCB);
    }
    rules = rules
        .rule(
            map.no_king,
            100,
            has_trump_queen(trump) & kings_outside(trump, 0..=0),
        )
        .alert(RKCB)
        .rule(map.deny, 60, !has_trump_queen(trump) & trump_buff(trump));
    rules.rule(map.weak, 50, hcp(0..))
}

/// Asker's placement over a denied queen
///
/// Both denials are the agreed trump, so both are already a contract: five of
/// it stops there unless all five keycards are on the table, and six of it —
/// the ninth trump or the void — is passed.
///
/// `five` is the **combined**-count decode, which the answer lane owns: over a
/// one-or-four answer four of our own is all five, while over a none-or-three
/// answer four of our own is exactly four — the caller passes the right test
/// because only it knows which answer was heard.
pub(super) fn asker_after_denial(
    trump: Suit,
    denial: Bid,
    five: crate::bidding::constraint::Cons<
        impl crate::bidding::constraint::Constraint + Clone + 'static,
    >,
) -> Rules {
    let t = Strain::from(trump);
    if denial == Bid::new(6, t) {
        return Rules::new().rule(Call::Pass, 50, hcp(0..));
    }
    Rules::new()
        .rule(Bid::new(6, t), 100, five)
        .rule(Call::Pass, 50, hcp(0..))
}

/// Asker's placement over a queen-and-king reply
///
/// Seven needs two side kings between the hands.  Partner named its cheapest,
/// so one of our own already makes two; with none, the **second relay** asks
/// for one more — and that is where kickback pays a second time, because the
/// relay is a step above partner's reply rather than an absolute 5NT.
///
/// Both ride the grand-zone **strength** gate on top of the combined count —
/// RKCB is a slam veto, not a slam seeker, so a partnership short of the grand
/// zone never spends the round.  `hcp(19..)` is the book's available proxy at
/// this node, and `five` is the lane's combined-count decode (see
/// [`asker_after_denial`]): a grand is never touched with a keycard out.
///
/// ponytail: raw HCP, because the book carries no combined-point machinery
/// here; the upgrade path is the floor's `points_and_net(combined_points(37))`
/// once the book's asker tables can see partner's shown strength.
pub(super) fn asker_after_queen(
    trump: Suit,
    partner_king: bool,
    relay: Option<KingRelay>,
    five: crate::bidding::constraint::Cons<
        impl crate::bidding::constraint::Constraint + Clone + 'static,
    >,
) -> Rules {
    let t = Strain::from(trump);
    let mut rules = Rules::new();
    if partner_king {
        rules = rules.rule(
            Bid::new(7, t),
            150,
            five.clone() & kings_outside(trump, 1..) & hcp(19..),
        );
        if let Some(relay) = relay {
            rules = rules
                .rule(
                    relay.ask,
                    140,
                    five & kings_outside(trump, 0..=0) & hcp(19..),
                )
                .alert(RKCB);
        }
    } else {
        rules = rules.rule(
            Bid::new(7, t),
            150,
            five & kings_outside(trump, 2..) & hcp(19..),
        );
    }
    rules.rule(Bid::new(6, t), 100, hcp(0..))
}

/// Partner's reply to the second relay: one more king, or six of trumps
pub(super) fn king_replies(trump: Suit, relay: KingRelay) -> Rules {
    Rules::new()
        .rule(relay.more, 100, kings_outside(trump, 2..))
        .alert(RKCB)
        // Six of the agreed trump is a contract, not a code.
        .rule(relay.none, 50, hcp(0..))
}

/// Asker's placement over the second relay's reply: seven on the second king,
/// and passing partner's six otherwise
pub(super) fn asker_after_relay_kings(trump: Suit, more: bool) -> Rules {
    let rules = Rules::new();
    if more {
        rules.rule(Bid::new(7, Strain::from(trump)), 100, hcp(0..))
    } else {
        rules.rule(Call::Pass, 50, hcp(0..))
    }
}
